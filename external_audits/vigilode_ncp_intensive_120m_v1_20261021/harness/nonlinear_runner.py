import json,os,sys,time,signal,subprocess,pathlib,random
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
cases=json.loads((JOB/'harness/cases.json').read_text())
case_ids=['B-robertson-short','B-vdp-mu20']
plans=[(cid,s) for cid in case_ids for s in ['vigilode','scipy_vode_adams','scipy_vode_bdf','julia','diffsol']]
random.Random(21021).shuffle(plans)
def append(row):
 with open(JOB/'NONLINEAR_CONFIRMATION_RESULTS.jsonl','a') as f:f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\n');f.flush();os.fsync(f.fileno())
workers=[]
for cid,solver in plans:
 c=next(x for x in cases if x['case_id']==cid);tag=f'{cid}-{solver}';out=JOB/'logs'/f'nonlinear-{tag}.stdout.log';err=JOB/'logs'/f'nonlinear-{tag}.stderr.log';gt=JOB/'logs'/f'nonlinear-{tag}.gnu_time.txt';start=time.monotonic()
 if solver in ('vigilode','scipy_vode_adams','scipy_vode_bdf'):
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(gt),'python3',str(JOB/'harness/nonlinear_python_worker.py'),cid,solver]
 elif solver=='julia':
  alg='Rodas5P'; req=JOB/'harness'/f'nonlinear-julia-{cid}.json'
  req.write_text(json.dumps({'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':alg,'warmups':2,'measurements':7},allow_nan=False))
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(gt),'/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia','--project='+str(JOB/'harness/julia-env'),str(JOB/'harness/julia_solver_v2.jl'),str(req)]
 else:
  req=JOB/'harness'/f'nonlinear-diffsol-{cid}.json';req.write_text(json.dumps({'case':{k:v for k,v in c.items() if k!='exact_kind'},'algorithm':'BDF'},allow_nan=False))
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(gt),str(JOB/'harness/bin/diffsol-adapter-v2'),str(req)]
 env={**os.environ,'JULIA_DEPOT_PATH':str(JOB/'julia-depot'),'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'}
 p=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True,env=env)
 try:so,se=p.communicate(timeout=max(1,min(180,279613.26-time.monotonic())));rc=p.returncode;status='EXITED' if rc==0 else 'FAILED'
 except subprocess.TimeoutExpired:
  os.killpg(p.pid,signal.SIGTERM)
  try:so,se=p.communicate(timeout=5)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);so,se=p.communicate()
  rc=None;status='TIMEOUT'
 out.write_text(so);err.write_text(se);nrows=0
 try:
  if status=='EXITED' and solver in ('vigilode','scipy_vode_adams','scipy_vode_bdf'):
   for line in so.splitlines():
    r=json.loads(line)
    if r['warmup']:r['gnu_time_path']=str(gt);append(r);continue
    r['gnu_time_path']=str(gt);append(r);nrows+=1
  elif status=='EXITED' and solver=='julia':
   x=json.loads(so);alg=x['algorithm']
   for i,w in enumerate(x['warmup_seconds'],1):append({'protocol':'vigilode_ncp_intensive_120m_v1','block':'stiff_nonlinear_confirmation','case_id':cid,'solver':'Julia OrdinaryDiffEq','backend':alg,'repetition':i,'warmup':True,'accuracy':None,'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE','elapsed_seconds':w,'thread_count':1,'cpu_affinity':'8'})
   for r in x['measurements']:
    states=r['states'];row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'stiff_nonlinear_confirmation','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':'Julia OrdinaryDiffEq','backend':alg,'repetition':r['repetition'],'warmup':False,'status':'OK' if r['retcode']=='Success' else 'NUMERICAL_FAILURE','retcode':r['retcode'],'accuracy':None,'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE','elapsed_seconds':r['elapsed_seconds'],'states':states,'states_final':states[-1] if states else None,'solver_counters':r['counters'],'solver_stats':r['solver_stats'],'gnu_time_path':str(gt),'thread_count':1,'cpu_affinity':'8'}
    if c['model']=='robertson' and states:row['invariants']={'minimum_component':min(min(y) for y in states),'max_abs_mass_residual':max(abs(sum(y)-sum(c['initial_state'])) for y in states),'positivity_pass_nonnegative':min(min(y) for y in states)>=0}
    if c['model']=='vanderpol' and states:row['trajectory_descriptors']={'max_abs_x':max(abs(y[0]) for y in states),'max_abs_v':max(abs(y[1]) for y in states)}
    append(row);nrows+=1
  elif status=='EXITED' and solver=='diffsol':
   x=json.loads(so)
   for r in x['warmup']:append({'protocol':'vigilode_ncp_intensive_120m_v1','block':'stiff_nonlinear_confirmation','case_id':cid,'solver':'diffsol','backend':x['algorithm'],'repetition':r['repetition'],'warmup':True,'accuracy':None,'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE','elapsed_seconds':r['elapsed_seconds'],'setup_seconds':r['setup_seconds'],'thread_count':1,'cpu_affinity':'8'})
   for r in x['measurements']:
    states=r['states'];row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'stiff_nonlinear_confirmation','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':'diffsol','backend':x['algorithm'],'repetition':r['repetition'],'warmup':False,'status':'OK' if 'TstopReached' in r['stop_reason'] else 'NUMERICAL_FAILURE','stop_reason':r['stop_reason'],'accuracy':None,'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE','elapsed_seconds':r['elapsed_seconds'],'setup_seconds':r['setup_seconds'],'states':states,'states_final':states[-1] if states else None,'solver_counters':None,'gnu_time_path':str(gt),'thread_count':1,'cpu_affinity':'8'}
    if c['model']=='robertson' and states:row['invariants']={'minimum_component':min(min(y) for y in states),'max_abs_mass_residual':max(abs(sum(y)-sum(c['initial_state'])) for y in states),'positivity_pass_nonnegative':min(min(y) for y in states)>=0}
    if c['model']=='vanderpol' and states:row['trajectory_descriptors']={'max_abs_x':max(abs(y[0]) for y in states),'max_abs_v':max(abs(y[1]) for y in states)}
    append(row);nrows+=1
 except Exception as e:status='POSTPROCESS_FAILURE';err.write_text(err.read_text()+repr(e)+'\n')
 rec={'case_id':cid,'solver':solver,'status':status,'exit_code':rc,'pid':p.pid,'process_group':p.pid,'wall_seconds':time.monotonic()-start,'measurement_rows':nrows,'stdout_path':str(out),'stderr_path':str(err),'gnu_time_path':str(gt)}
 with open(JOB/'NONLINEAR_CONFIRMATION_WORKERS.jsonl','a') as f:f.write(json.dumps(rec,separators=(',',':'))+'\n');f.flush();os.fsync(f.fileno())
 workers.append(rec);print(json.dumps(rec),flush=True)
 if status!='EXITED':break
rows=[json.loads(x) for x in (JOB/'NONLINEAR_CONFIRMATION_RESULTS.jsonl').read_text().splitlines()] if (JOB/'NONLINEAR_CONFIRMATION_RESULTS.jsonl').exists() else []
summary={'planned_workers':len(plans),'workers_started':len(workers),'workers_completed':sum(x['status']=='EXITED' and x['exit_code']==0 for x in workers),'measurement_rows':sum(not x.get('warmup',False) for x in rows),'warmup_rows':sum(x.get('warmup',False) for x in rows),'accuracy_reference':'UNKNOWN_NO_INDEPENDENT_REFERENCE','broad_production':'HOLD'}
(JOB/'NONLINEAR_CONFIRMATION_SUMMARY.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary))
