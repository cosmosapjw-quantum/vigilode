#!/usr/bin/env python3
"""Independent analytic/VODE replay and common achieved-error selection."""
import argparse,gzip,hashlib,json,math,pathlib,statistics
import numpy as np
from scipy.integrate import ode
import scipy

def exact(name,t,n):
 if name=='decay':return [math.exp(-t)]
 if name=='logistic':return [1/(1+19*math.exp(-t))]
 if name=='prothero':return [math.cos(t)]
 if name=='transition':
  def lc(x):return abs(x)+math.log1p(math.exp(-2*abs(x)))-math.log(2)
  inte=t+4999.5/40*(lc(40*(t-.3))-lc(40*(t-.7))-lc(-12)+lc(-28))
  return [math.cos(t)+math.exp(-inte)]
 if name=='rapid':return [math.cos(200*t),math.sin(200*t)]
 if name=='damped':return [math.exp(-100*t)*math.cos(1000*t),math.exp(-100*t)*math.sin(1000*t)]
 if name=='nonnormal':return [100*(math.exp(-t)-math.exp(-2*t)),math.exp(-2*t)]
 if name=='vector':return [math.exp(-[1,10,1000][i%3]*t)for i in range(n)]
 raise ValueError(name)
def robertson_ref(times,tol):
 def f(t,y):
  a=-.04*y[0]+1e4*y[1]*y[2];b=3e7*y[1]*y[1];return [a,-a-b,b]
 def j(t,y):return [[-.04,1e4*y[2],1e4*y[1]],[.04,-1e4*y[2]-6e7*y[1],-1e4*y[1]],[0,6e7*y[1],0]]
 s=ode(f,j).set_integrator('vode',method='bdf',rtol=tol,atol=tol*.001,nsteps=200000).set_initial_value([1.,0.,0.],0.);out={0.:[1.,0.,0.]}
 for t in sorted(times):
  if t==0:continue
  s.integrate(t)
  if not s.successful():raise RuntimeError('VODE reference failed')
  out[t]=list(s.y)
 return out

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder');a=p.parse_args();folder=pathlib.Path(a.folder);plan=json.loads((folder/'PLAN.json').read_text());rows=[json.loads(l)for l in gzip.decompress((folder/'RAW_RUNS.jsonl.gz').read_bytes()).decode().splitlines()]
 parsed=[]
 for row in rows:
  assert hashlib.sha256(row['stdout'].encode()).hexdigest()==row['stdout_sha256']
  try:d=json.loads(row['stdout'])
  except (ValueError,TypeError):d=None
  parsed.append((row,d))
 rt={t for row,d in parsed if d and d['case']=='robertson'for t in d['times']}
 ref=robertson_ref(rt,1e-12);ref_coarse=robertson_ref(rt,1e-11);refdelta=max((abs(ref[t][i]-ref_coarse[t][i])for t in rt for i in range(3)),default=0.)
 replay=[]
 for row,d in parsed:
  rec={k:row[k]for k in ['id','phase','repeat','status','exit_code']}
  if d is None:rec.update(valid=False,reason='no parsed result');replay.append(rec);continue
  shape=len(d['times'])==len(d['states']) and len(d['times'])>0 and all(len(y)==d['dimension']and all(math.isfinite(v)for v in y)for y in d['states']) and all(math.isfinite(t)for t in d['times']) and all(x<y for x,y in zip(d['times'],d['times'][1:]))
  identity=d.get('source_commit')==plan['source_commit'] and all(d.get(k)==row[k]for k in ['case','dimension','mode','rtol','windows'])
  if not shape or not identity:rec.update(valid=False,reason='shape/time/source/config mismatch');replay.append(rec);continue
  rr=[ref[t]if d['case']=='robertson'else exact(d['case'],t,d['dimension'])for t in d['times']]
  init=rr[0];err=max((abs(y[i]-r[i])/max(1,abs(init[i]),abs(r[i]))for y,r in zip(d['states'],rr)for i in range(d['dimension'])),default=math.inf)
  abs_err=max((abs(y[i]-r[i])for y,r in zip(d['states'],rr)for i in range(d['dimension'])),default=math.inf)
  reported=d.get('max_normalized_error');matches=reported is None or abs(err-reported)<=1e-12*max(1,err)
  reached=d['times'][-1]==d['end'];full_schedule=all(t in d['times']for t in d['requested_times'])
  if d['mode'].startswith('dopri'):matches=matches and all(d['work'][k]==0 for k in ['jacobian_builds','jvp_vectors','linear_solves'])
  phase=0.;amplitude=0.
  if d['case']in ['rapid','damped']:
   for y,r in zip(d['states'],rr):
    ra=math.hypot(*r);ya=math.hypot(*y);amplitude=max(amplitude,abs(ya-ra))
    if ra>1e-10 and ya>1e-10:phase=max(phase,abs(math.atan2(y[1]*r[0]-y[0]*r[1],y[0]*r[0]+y[1]*r[1])))
   matches=matches and abs(phase-d['max_phase_error_radians'])<1e-11 and abs(amplitude-d['max_amplitude_error'])<1e-11
  replay.append(dict(**rec,valid=shape and matches,solver_success=d['success']and row['exit_code']==0 and row['status']=='COMPLETED',reached=reached,full_schedule=full_schedule,normalized_error=err,absolute_error=abs_err,elapsed_seconds=d['elapsed_seconds'],phase_error=d.get('max_phase_error_radians'),amplitude_error=d.get('max_amplitude_error'),mass_drift=d.get('max_mass_drift'),work=d['work'],message=d['message']))
 configs=[]
 for cfg in plan['configurations']:
  rr=[r for r in replay if r['id']==cfg['id']and r['phase']=='timed'];valid=[r for r in rr if r.get('valid')];eligible=len(valid)==plan['repeats']and all(r['solver_success']and r['reached']and r['full_schedule']for r in valid)
  configs.append(dict(**cfg,eligible=eligible,completed_replays=len(valid),median_seconds=statistics.median([r['elapsed_seconds']for r in valid])if valid else None,min_seconds=min([r['elapsed_seconds']for r in valid],default=None),max_seconds=max([r['elapsed_seconds']for r in valid],default=None),max_error=max([r['normalized_error']for r in valid],default=None),max_phase_error=max([r['phase_error']for r in valid if r['phase_error']is not None],default=None),max_amplitude_error=max([r['amplitude_error']for r in valid if r['amplitude_error']is not None],default=None),work=valid[0]['work']if valid else None,failures=[r.get('message',r.get('reason'))for r in rr if not r.get('solver_success',False)]))
 comparisons=[]
 for case,n in sorted({(c['case'],c['dimension'])for c in configs}):
  for cap in plan['accuracy_caps']:
   best={}
   for mode in sorted({c['mode']for c in configs}):
    candidates=[c for c in configs if c['case']==case and c['dimension']==n and c['mode']==mode and c['eligible']and c['max_error']<=cap]
    if candidates:best[mode]=min(candidates,key=lambda c:c['median_seconds'])
   comparisons.append(dict(case=case,dimension=n,error_cap=cap,best={k:{x:v[x]for x in ['id','rtol','median_seconds','max_error']}for k,v in best.items()},ratios={f'{base}_over_{candidate}':best[base]['median_seconds']/best[candidate]['median_seconds']for base,candidate in [('dopri5','switch-fd'),('rodas-fd','switch-fd'),('dopri5','switch-jvp'),('rodas-jvp','switch-jvp'),('dopri-whole','switch-fd'),('dopri-whole-dense','switch-fd'),('rodas-whole-fd','switch-fd'),('dopri-whole','switch-jvp'),('rodas-whole-fd','switch-jvp')]if base in best and candidate in best}))
 summary=dict(schema='vigilode.adaptive-causal-summary.v1',source_commit=plan['source_commit'],records=len(rows),valid_replays=sum(r.get('valid',False)for r in replay),solver_successes=sum(r.get('solver_success',False)for r in replay),configurations=configs,comparisons=comparisons,reference=dict(robertson='SciPy VODE BDF analytic Jacobian, rtol1e-12 atol1e-15 versus1e-11/1e-14 refinement',scipy_version=scipy.__version__,max_refinement_difference=refdelta,points=len(rt),analytic_other_cases=True),timing_scope=plan['comparison'],limitations=['Three timed repeats on shared host; median/min/max diagnostic only','Window controls include identical restarts; whole-call controlsretainworkspaces andcomparetheircompleteoutputcontract atsameactualerrorcap; wholeRODASusesWFilteredEnforcewhileendpointdriverdoesnot','Sampled errors are not uniform-time guarantees','Successful local error tests do not establish global error','JVP and finite differences explicitly labeled; neither called fully derivative-free'])
 for name,obj in [('SUMMARY.json',summary),('REPLAY.json',replay),('ROBERTSON_REFERENCE.json',dict(refined={str(k):v for k,v in ref.items()},coarse={str(k):v for k,v in ref_coarse.items()},difference=refdelta))]:
  (folder/name).write_text(json.dumps(obj,indent=2,allow_nan=False)+'\n')
 print(json.dumps(dict(records=summary['records'],valid=summary['valid_replays'],successes=summary['solver_successes'],robertson_refinement=refdelta)))
 try:
  import matplotlib;matplotlib.use('Agg');import matplotlib.pyplot as plt
  names=[x for x in ['decay','logistic','prothero','robertson','rapid','damped','nonnormal','transition','vector']if any(c['case']==x for c in configs)]
  fig,axes=plt.subplots(3,3,figsize=(13,10),squeeze=False);colors={'dopri5':'#1f77b4','rodas-fd':'#d62728','rodas-jvp':'#9467bd','switch-fd':'#ff7f0e','switch-jvp':'#2ca02c','dopri-whole':'#17becf','dopri-whole-dense':'#444444','rodas-whole-fd':'#8c564b'}
  for ax,case in zip(axes.flat,names):
   for mode in colors:
    cc=sorted([c for c in configs if c['case']==case and c['mode']==mode and c['eligible']],key=lambda c:c['max_error'])
    if cc:ax.loglog([max(c['max_error'],1e-17)for c in cc],[c['median_seconds']for c in cc],'-o',label=mode,color=colors[mode],markersize=4)
   ax.set_title(case);ax.grid(True,which='both',alpha=.25);ax.set_xlabel('max sampled normalized error');ax.set_ylabel('seconds, median of 3')
  handles,labels=axes.flat[0].get_legend_handles_labels();fig.legend(handles,labels,loc='lower center',ncol=4);fig.suptitle('Single-thread cost: window and retained-workspace controls, all setup included');fig.tight_layout(rect=[0,.04,1,.96]);fig.savefig(folder/'WORK_PRECISION.png',dpi=160);fig.savefig(folder/'WORK_PRECISION.pdf');plt.close(fig)
 except ImportError:pass
if __name__=='__main__':main()
