import json,math,os,sys,time,signal,subprocess,pathlib,random
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
cid='C-rotation-w10';tolerances=[1e-4,1e-6,1e-8,1e-10]
solvers=['vigilode','scipy_vode_adams','scipy_vode_bdf','julia','diffsol']
plans=[(tol,s) for tol in tolerances for s in solvers]
random.Random(42021).shuffle(plans)
cases=json.loads((JOB/'harness/cases.json').read_text());base=next(x for x in cases if x['case_id']==cid)
def exact(t):
 y=base['initial_state'];w=base['parameters']['omega'];return [y[0]*math.cos(w*t)-y[1]*math.sin(w*t),y[0]*math.sin(w*t)+y[1]*math.cos(w*t)]
def metrics(ts,ys,rtol):
 ae=se=re=0.0;end=(None,None);phase=[];amp=[]
 for t,row in zip(ts,ys):
  ref=exact(t);e=ss=0.0
  for a,b,tol in zip(row,ref,base['atol']):
   d=abs(a-b);ae=max(ae,d);z=d/(tol+rtol*abs(b));se=max(se,z);re=max(re,d/max(abs(b),tol));e=max(e,d);ss=max(ss,z)
  end=(e,ss)
  phase.append(abs((math.atan2(row[1],row[0])-base['parameters']['omega']*t+math.pi)%(2*math.pi)-math.pi))
  amp.append(abs(math.hypot(row[0],row[1])-math.hypot(*base['initial_state'])))
 return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'endpoint_abs_error':end[0],'endpoint_max_scaled_error':end[1],'accuracy_pass_scaled_le_1':se<=1,'max_abs_phase_error_rad':max(phase),'max_amplitude_error':max(amp),'reference':'analytic exact sampled output grid'}
def append(row):
 with open(JOB/'ROTATION_TOLERANCE_RESULTS.jsonl','a') as f:f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\n');f.flush();os.fsync(f.fileno())
workers=[]
for idx,(rtol,solver) in enumerate(plans):
 if time.monotonic()>=279613.26:break
 tag=f'{rtol:g}-{solver}';out=JOB/'logs'/f'rotation-tolerance-{tag}.stdout.log';err=JOB/'logs'/f'rotation-tolerance-{tag}.stderr.log';tim=JOB/'logs'/f'rotation-tolerance-{tag}.gnu_time.txt';start=time.monotonic()
 req_case={k:v for k,v in base.items() if k!='exact_kind'};req_case['rtol']=rtol
 if solver in ('vigilode','scipy_vode_adams','scipy_vode_bdf'):
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(tim),'python3',str(JOB/'harness/rotation_tolerance_python_worker.py'),cid,solver,str(rtol)]
 elif solver=='julia':
  request=JOB/'harness'/f'rotation-tolerance-julia-{rtol:g}.json';request.write_text(json.dumps({'case':req_case,'solver_id':'Tsit5','warmups':2,'measurements':7},allow_nan=False))
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(tim),'/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia','--project='+str(JOB/'harness/julia-env'),str(JOB/'harness/julia_solver_v2.jl'),str(request)]
 else:
  request=JOB/'harness'/f'rotation-tolerance-diffsol-{rtol:g}.json';request.write_text(json.dumps({'case':req_case,'algorithm':'Tsit45'},allow_nan=False))
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(tim),str(JOB/'harness/bin/diffsol-adapter-v2'),str(request)]
 env={**os.environ,'JULIA_DEPOT_PATH':str(JOB/'julia-depot'),'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'}
 proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env,start_new_session=True)
 try:
  so,se=proc.communicate(timeout=max(1,min(180,279613.26-time.monotonic())));rc=proc.returncode;status='EXITED' if rc==0 else 'FAILED'
 except subprocess.TimeoutExpired:
  os.killpg(proc.pid,signal.SIGTERM)
  try:so,se=proc.communicate(timeout=5)
  except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);so,se=proc.communicate()
  rc=None;status='TIMEOUT'
 out.write_text(so);err.write_text(se);nrows=0
 try:
  if status=='EXITED' and solver in ('vigilode','scipy_vode_adams','scipy_vode_bdf'):
   before=sum(1 for _ in (JOB/'ROTATION_TOLERANCE_PY_RAW.jsonl').open()) if (JOB/'ROTATION_TOLERANCE_PY_RAW.jsonl').exists() else 0
   pyrows=[json.loads(x) for x in (JOB/'ROTATION_TOLERANCE_PY_RAW.jsonl').read_text().splitlines()[before:]]
   for row in pyrows:
    row.update({'worker_process_wall_seconds':time.monotonic()-start,'gnu_time_path':str(tim)});append(row);nrows+=1
  elif status=='EXITED' and solver=='julia':
   result=json.loads(so);alg=result['algorithm']
   for j,w in enumerate(result['warmup_seconds'],1):append({'protocol':'vigilode_ncp_intensive_120m_v1','block':'rotation_work_precision_serial','case_id':cid,'solver':'Julia OrdinaryDiffEq','backend':alg,'repetition':j,'warmup':True,'rtol':rtol,'atol':base['atol'],'elapsed_seconds':w,'accuracy':None,'thread_count':1,'cpu_affinity':'8'})
   for r in result['measurements']:
    row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'rotation_work_precision_serial','case_id':cid,'family':base['family'],'dimension':2,'solver':'Julia OrdinaryDiffEq','backend':alg,'repetition':r['repetition'],'warmup':False,'rtol':rtol,'atol':base['atol'],'thread_count':1,'cpu_affinity':'8','status':'OK' if r['retcode']=='Success' else 'NUMERICAL_FAILURE','elapsed_seconds':r['elapsed_seconds'],'states':r['states'],'accuracy':metrics(base['output_times'],r['states'],rtol),'solver_counters':r['counters'],'solver_stats':r['solver_stats'],'warmup_seconds':result['warmup_seconds'],'gnu_time_path':str(tim)}
    append(row);nrows+=1
  elif status=='EXITED' and solver=='diffsol':
   result=json.loads(so)
   for r in result['warmup']:append({'protocol':'vigilode_ncp_intensive_120m_v1','block':'rotation_work_precision_serial','case_id':cid,'solver':'diffsol','backend':result['algorithm'],'repetition':r['repetition'],'warmup':True,'rtol':rtol,'atol':base['atol'],'elapsed_seconds':r['elapsed_seconds'],'setup_seconds':r['setup_seconds'],'accuracy':metrics(base['output_times'],r['states'],rtol),'thread_count':1,'cpu_affinity':'8'})
   for r in result['measurements']:
    row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'rotation_work_precision_serial','case_id':cid,'family':base['family'],'dimension':2,'solver':'diffsol','backend':result['algorithm'],'repetition':r['repetition'],'warmup':False,'rtol':rtol,'atol':base['atol'],'elapsed_seconds':r['elapsed_seconds'],'setup_seconds':r['setup_seconds'],'states':r['states'],'accuracy':metrics(base['output_times'],r['states'],rtol),'solver_counters':None,'thread_count':1,'cpu_affinity':'8','gnu_time_path':str(tim)}
    append(row);nrows+=1
 except Exception as e:status='POSTPROCESS_FAILURE';err.write_text(err.read_text()+repr(e)+'\n')
 rec={'case_id':cid,'solver':solver,'rtol':rtol,'status':status,'exit_code':rc,'pid':proc.pid,'process_group':proc.pid,'wall_seconds':time.monotonic()-start,'measurement_rows':nrows,'stdout_path':str(out),'stderr_path':str(err),'gnu_time_path':str(tim)}
 with open(JOB/'ROTATION_TOLERANCE_WORKERS.jsonl','a') as f:f.write(json.dumps(rec,separators=(',',':'))+'\n');f.flush();os.fsync(f.fileno())
 workers.append(rec);print(json.dumps(rec),flush=True)
 if status!='EXITED':break
rows=[json.loads(x) for x in (JOB/'ROTATION_TOLERANCE_RESULTS.jsonl').read_text().splitlines()] if (JOB/'ROTATION_TOLERANCE_RESULTS.jsonl').exists() else []
summary={'protocol':'vigilode_ncp_intensive_120m_v1','case_id':cid,'planned_workers':len(plans),'workers_started':len(workers),'workers_completed':sum(x['status']=='EXITED' and x['exit_code']==0 for x in workers),'measurement_rows':sum(not x.get('warmup',False) for x in rows),'warmup_rows':sum(x.get('warmup',False) for x in rows),'accuracy_pass':sum((x.get('accuracy') or {}).get('accuracy_pass_scaled_le_1') is True for x in rows if not x.get('warmup',False)),'accuracy_fail':sum((x.get('accuracy') or {}).get('accuracy_pass_scaled_le_1') is False for x in rows if not x.get('warmup',False)),'broad_production':'HOLD'}
tmp=JOB/'ROTATION_TOLERANCE_SUMMARY.json.tmp';tmp.write_text(json.dumps(summary,indent=2));os.replace(tmp,JOB/'ROTATION_TOLERANCE_SUMMARY.json');print(json.dumps(summary))
