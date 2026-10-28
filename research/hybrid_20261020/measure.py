#!/usr/bin/env python3
"""Accuracy-only selection followed by fresh native full-trajectory timings."""
import argparse,gzip,hashlib,json,csv,math,statistics,subprocess,random
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
import scipy
NATIVE=['auto','structured','eager-full','lazy-full','lazy-local','sequential','cubic-auto']
EXTERNAL=['fast','diffsol','dvode']
CASES=[(16,0),(16,1),(64,0),(64,1)]
LADDER=[1e-4,1e-5,1e-6,1e-7,1e-8,1e-9]
def dump(p,x):Path(p).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def bits(v):return np.asarray(v,dtype='<f8').tobytes()
def ref_model(n,eps):
 d=.01*n*n;i=np.arange(n);initial=.45+.12*np.cos(np.pi*(i+.5)/n)+.03*np.cos(7*np.pi*(i+.5)/n)
 def f(t,y):
  v=-2*y+.4*np.mean(y)*(1-y)-eps*y*y;flux=d*(y[1:]-y[:-1]);v[:-1]+=flux;v[1:]-=flux;return v
 def jac(t,y):
  j=np.repeat((.4*(1-y)/n)[:,None],n,axis=1);q=-2-.4*np.mean(y)-2*eps*y;q[0]-=d;q[-1]-=d;q[1:-1]-=2*d;j[np.arange(n),np.arange(n)]+=q;j[np.arange(n-1),np.arange(1,n)]+=d;j[np.arange(1,n),np.arange(n-1)]+=d;return j
 return initial,f,jac

def main(a):
 out=a.out;out.mkdir(parents=True,exist_ok=True);(out/'objects').mkdir(exist_ok=True);refs={};times=[.2*i/20 for i in range(21)]
 for n,eps in CASES+[(256,0)]:
  y,f,j=ref_model(n,eps);sol=[]
  for tol in [1e-11,1e-12]:
   s=solve_ivp(f,[0,.2],y,method='Radau',jac=j,t_eval=times,rtol=tol,atol=tol*.01);assert s.success and s.y.shape==(n,21);sol.append(s.y.T)
  refs[f'{n}:{eps}']={'initial':y.tolist(),'times':times,'states':sol[-1].tolist(),'radius':(2*np.abs(sol[-1]-sol[0])+2e-13).tolist()}
 dump(out/'REFERENCES.json',refs)
 manifest={'source':a.source,'binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'cases':CASES,'native':NATIVE,'external':EXTERNAL,'native_budget':1e-6,'external_tolerance_ladder':LADDER,'blocks':4,'reps':3,'numpy':np.__version__,'scipy':scipy.__version__,'reference':'independent Python Radau ladder, empirical radius; NOT native admission authority','timing':'whole trajectory including certificate where provided, setup and pools','matched_error_ratio_limit':2.0,'shared_runner_diagnostic':True};dump(out/'MANIFEST.json',manifest)
 def call(n,eps,arm,tol,reps=1,workers=1):
  cmd=[str(a.binary),str(n),str(eps),arm,str(tol),str(reps),str(workers)]
  try:
   p=subprocess.run(cmd,capture_output=True,text=True,timeout=45);r=json.loads(p.stdout)if p.returncode==0 else{'success':False,'failure':p.stderr[-4000:],'exit':p.returncode}
  except subprocess.TimeoutExpired:r={'success':False,'failure':'PROCESS_TIMEOUT_45S'}
  except ValueError:r={'success':False,'failure':'MALFORMED_STDOUT'}
  r.update(n=n,epsilon=eps,arm=arm,tol=tol,workers=workers,command=cmd);ref=refs[f'{n}:{eps}'];v=np.array(ref['states']);m={'valid':False,'absolute_error_upper':None,'empirical_reference_radius':None,'certified_radius':None};y=np.array(r.get('states'))
  if r.get('success')and r.get('deterministic')and r.get('times')==times and y.shape==v.shape and np.isfinite(y).all():
   rad=float(np.max(ref['radius']));error=float(np.max(np.abs(y-v)))+rad;integ=(r.get('details')or{}).get('integration');bound=max(integ['radii'])if integ else None;m.update(valid=bool(error<=1e-6),absolute_error_upper=error,empirical_reference_radius=rad,certified_radius=bound)
  return r,m
 def retain(r,m,fp,**metadata):
  q=dict(r);obj={k:q.pop(k,None)for k in ['states','details']};b=json.dumps(obj,sort_keys=True,separators=(',',':'),allow_nan=False).encode();h=hashlib.sha256(b).hexdigest();path=out/'objects'/(h+'.json.gz')
  if not path.exists():
   with gzip.open(path,'wb',compresslevel=9)as f:f.write(b)
  q['object_sha256']=h;fp.write(json.dumps({'row':q,'metrics':m,**metadata},separators=(',',':'),allow_nan=False)+'\n');fp.flush();return h
 selected={};cal=[]
 with gzip.open(out/'CALIBRATION.jsonl.gz','wt')as fp:
  for n,eps in CASES:
   target=None
   for arm in NATIVE:
    r,m=call(n,eps,arm,1e-6);h=retain(r,m,fp);selected[(n,eps,arm)]={'tol':1e-6,'metrics':m,'hash':h,'row':r};cal.append((r,m))
    if arm=='auto':target=m['absolute_error_upper']
   for arm in EXTERNAL:
    attempts=[]
    for tol in LADDER:
     r,m=call(n,eps,arm,tol);h=retain(r,m,fp);cal.append((r,m));attempts.append({'tol':tol,'metrics':m,'hash':h,'row':r})
    good=[x for x in attempts if x['metrics']['valid']];selected[(n,eps,arm)]=min(good,key=lambda x:abs(math.log(max(x['metrics']['absolute_error_upper'],1e-300)/max(target or 1e-7,1e-300))))if good else attempts[-1]
   print('calibration',n,eps,flush=True)
  for n,eps,arm,workers in [(256,0,'auto',1),(64,1,'lazy-local',4)]:
   r,m=call(n,eps,arm,1e-6,workers=workers);retain(r,m,fp,control_only=True);cal.append((r,m))
 frozen=[{'n':n,'epsilon':eps,'arm':arm,'tol':v['tol'],'valid':v['metrics']['valid'],'calibration_object':v['hash']}for(n,eps,arm),v in selected.items()];dump(out/'FROZEN_SELECTION.json',frozen)
 units=[];rng=random.Random(20261020)
 with gzip.open(out/'TIMING.jsonl.gz','wt')as fp:
  for block in range(4):
   keys=list(selected);rng.shuffle(keys)
   for key in keys:
    n,eps,arm=key;s=selected[key]
    if not s['metrics']['valid']:
     units.append({'n':n,'epsilon':eps,'arm':arm,'block':block,'valid':False,'median_ns':None,'timed_attempts':0,'status':'NOT_RERUN_FAILED_CALIBRATION'});continue
    r,m=call(n,eps,arm,s['tol'],reps=3);ns=r.get('elapsed_ns',[]);valid=m['valid']and len(ns)==3 and all(isinstance(x,int)and x>0 for x in ns)and bits(r['states'])==bits(s['row']['states']);h=retain(r,m,fp,block=block)
    units.append({'n':n,'epsilon':eps,'arm':arm,'block':block,'valid':bool(valid),'median_ns':statistics.median(ns)if valid else None,'object_sha256':h,'timed_attempts':len(ns)})
   print('timing block',block+1,flush=True)
 dump(out/'UNITS.json',units);rows=[]
 for(n,eps,arm),s in selected.items():
  u=[x for x in units if(x['n'],x['epsilon'],x['arm'])==(n,eps,arm)];integ=(s['row'].get('details')or{}).get('integration')or{};w=integ.get('work')or{}
  rows.append({'n':n,'epsilon':eps,'arm':arm,'tol':s['tol'],'complete_blocks':sum(x['valid']for x in u),'median_ms':statistics.median(x['median_ns']for x in u)/1e6 if all(x['valid']for x in u)else None,**s['metrics'],'accepted_windows':integ.get('accepted'),'rejected_windows':integ.get('rejected'),'rhs_vectors':w.get('rhs_vectors'),'jvp_vectors':w.get('jvp_vectors'),'local_dual_cells':w.get('local_dual_cells'),'full_dual_cells':w.get('full_dual_cells'),'partial_ad_cells':w.get('partial_ad_cells'),'band_factorizations':w.get('band_factorizations'),'certificate_components':w.get('certificate_components')})
 with(out/'RESULTS.csv').open('w',newline='')as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
 pairs=[]
 for n,eps in CASES:
  for base,cand in [('eager-full','lazy-local'),('lazy-full','lazy-local'),('structured','auto'),('cubic-auto','auto'),('sequential','structured'),('diffsol','auto'),('dvode','auto'),('fast','auto')]:
   a0=selected[(n,eps,base)]['metrics'];b0=selected[(n,eps,cand)]['metrics'];rat=[]
   for b in range(4):
    group={x['arm']:x for x in units if x['n']==n and x['epsilon']==eps and x['block']==b}
    if group[base]['valid']and group[cand]['valid']:rat.append(group[base]['median_ns']/group[cand]['median_ns'])
   errors=[a0['absolute_error_upper'],b0['absolute_error_upper']];ratio=max(errors)/min(errors)if all(x is not None and x>0 for x in errors)else None
   pairs.append({'n':n,'epsilon':eps,'baseline':base,'candidate':cand,'ratios':rat,'median_ratio':statistics.median(rat)if len(rat)==4 else None,'achieved_error_ratio':ratio,'matched_achieved_error':bool(len(rat)==4 and ratio is not None and ratio<=2),'min_observed_ratio':min(rat)if rat else None,'max_observed_ratio':max(rat)if rat else None,'authority':'FOUR_BLOCK_DIAGNOSTIC_NOT_PRODUCTION','certificate_contracts_same':base not in EXTERNAL})
 dump(out/'PAIRS.json',pairs)
 summary={'status':'COMPLETED_BOUNDED','calibration_and_control_solves':len(cal),'calibration_valid':sum(m['valid']for r,m in cal),'selected_configurations':len(rows),'timing_units':len(units),'valid_timing_units':sum(u['valid']for u in units),'timed_trajectory_attempts':sum(u['timed_attempts']for u in units),'complete_comparisons':sum(p['median_ratio']is not None for p in pairs),'matched_achieved_comparisons':sum(p['matched_achieved_error']for p in pairs),'objects':len(list((out/'objects').glob('*.gz'))),'global_optimality_claim':False,'external_continuous_certificates':False};dump(out/'SUMMARY.json',summary);print(json.dumps(summary),flush=True)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--source',required=True);a=p.parse_args()
 try:main(a)
 except Exception as e:a.out.mkdir(parents=True,exist_ok=True);dump(a.out/'BLOCKER.json',{'type':type(e).__name__,'error':str(e)});raise
