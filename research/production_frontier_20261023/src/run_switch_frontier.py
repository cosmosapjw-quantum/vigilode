#!/usr/bin/env python3
"""Pinned, bounded, same-source switching campaign; preserves every failed run."""
import argparse, hashlib, json, os, pathlib, statistics, subprocess, time
ROOT=pathlib.Path(__file__).resolve().parents[3]
CASES=['decay','rapid','prothero','transition']
MODES=['rhs','legacy-fd','legacy-jvp','screen-fd','screen-jvp']

def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);p.add_argument('--repeats',type=int,default=3);p.add_argument('--budget',type=float,default=600);a=p.parse_args()
 if a.repeats<1 or a.budget<=0:raise SystemExit('positive repeats and budget required')
 out=pathlib.Path(a.output);out.mkdir(parents=True,exist_ok=False)
 binary=pathlib.Path(a.binary).resolve()
 status=subprocess.check_output(['git','status','--porcelain','--','crates','Cargo.toml','Cargo.lock'],cwd=ROOT,text=True)
 if status.strip():raise SystemExit('native source must be committed before timing')
 sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
 sources=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']+sorted((ROOT/'crates').rglob('*.rs'))+sorted((ROOT/'crates').rglob('Cargo.toml'))
 cpus=sorted(os.sched_getaffinity(0));cpu=cpus[0];os.sched_setaffinity(0,{cpu})
 plan={'schema':'vigilode.switch-frontier-campaign.v1','source_commit':sha,'cpu':cpu,'cases':CASES,'modes':MODES,'tolerances':[1e-6,1e-8],'repeats':a.repeats,'warmup_runs':1,'absolute_error_cap':1e-5,'total_budget_seconds':a.budget,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'source_file_sha256':{str(x.relative_to(ROOT)):hashlib.sha256(x.read_bytes()).hexdigest() for x in sources},'selection':'fastest of two prespecified tolerances satisfying all repeat errors at common observations; exploratory same-host only'}
 (out/'PLAN.json').write_text(json.dumps(plan,indent=2)+'\n')
 rows=[];receipts=[];start=time.monotonic()
 env=os.environ.copy();env.update({k:'1' for k in ['RAYON_NUM_THREADS','OPENBLAS_NUM_THREADS','OMP_NUM_THREADS','MKL_NUM_THREADS']})
 for case in CASES:
  for ti,tol in enumerate(plan['tolerances']):
   for rep in range(-1,a.repeats):
    shift=(CASES.index(case)+ti+rep)%len(MODES)
    for mode in MODES[shift:]+MODES[:shift]:
     tag=f'{case}_{mode}_{tol:g}_{rep}'
     cmd=[str(binary),'--case',case,'--mode',mode,'--rtol',str(tol),'--source-commit',sha]
     left=a.budget-(time.monotonic()-start)
     if left<=0:
      receipts.append({'tag':tag,'status':'NOT_RUN_BUDGET','command':cmd});continue
     begin=time.monotonic()
     with (out/(tag+'.json')).open('w') as stdout,(out/(tag+'.stderr.log')).open('w') as stderr:
      try:r=subprocess.run(cmd,stdout=stdout,stderr=stderr,env=env,cwd=ROOT,timeout=min(30,left));code=r.returncode
      except subprocess.TimeoutExpired:code=124
     rec={'tag':tag,'exit_code':code,'command':cmd,'process_seconds':time.monotonic()-begin,'warmup':rep<0};receipts.append(rec)
     if rep<0:continue
     row={'case':case,'mode':mode,'rtol':tol,'repeat':rep,'success':False,'exit_code':code}
     if code==0:
      try:
       d=json.loads((out/(tag+'.json')).read_text());s=d['solution']['stats']
       row.update({'success':d['success'] and d['common_observations_covered'],'seconds':d['elapsed_seconds'],'error':d['common_observation_max_abs_error'],'phase':d['phase_error_radians'],'amplitude':d['amplitude_error'],'rhs':s['rhs_calls'],'jvp':s['jvp_vectors'],'attempts':s['attempts'],'switches':s['switches'],'probes':sum(x['is_probe'] for x in s['segments']),'work_units':sum(x['work_units'] for x in s['segments'])})
      except Exception as e:row['parse_error']=str(e)
     rows.append(row)
 summary=[]
 for case in CASES:
  eligible={}
  for mode in MODES:
   options=[]
   for tol in plan['tolerances']:
    rr=[r for r in rows if r['case']==case and r['mode']==mode and r['rtol']==tol]
    if len(rr)==a.repeats and all(r['success'] and r['error']<=plan['absolute_error_cap'] for r in rr):
     options.append({'rtol':tol,'median_seconds':statistics.median(x['seconds'] for x in rr),'min_seconds':min(x['seconds'] for x in rr),'max_seconds':max(x['seconds'] for x in rr),'worst_error':max(x['error'] for x in rr),'rhs':rr[0]['rhs'],'jvp':rr[0]['jvp'],'attempts':rr[0]['attempts'],'probes':rr[0]['probes'],'switches':rr[0]['switches']})
   if options:eligible[mode]=min(options,key=lambda r:r['median_seconds'])
  summary.append({'case':case,'eligible_best':eligible})
 for name,data in [('ROWS.json',rows),('RECEIPTS.json',receipts),('SUMMARY.json',{'schema':'vigilode.switch-frontier-summary.v1','completed_rows':len(rows),'successes':sum(x['success'] for x in rows),'failures':sum(not x['success'] for x in rows),'comparisons':summary,'budget_exhausted':any(r.get('status')=='NOT_RUN_BUDGET' for r in receipts),'scope':'input tolerance not equal achieved error; all selected settings satisfy same sampled absolute error cap1e-5; no continuous global certificate'})]:
  (out/name).write_text(json.dumps(data,indent=2)+'\n')
 print(json.dumps({'rows':len(rows),'successes':sum(x['success'] for x in rows)}))
if __name__=='__main__':main()
