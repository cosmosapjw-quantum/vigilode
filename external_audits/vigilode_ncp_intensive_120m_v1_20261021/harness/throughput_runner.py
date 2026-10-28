import os,sys,time,signal,subprocess,pathlib,json,random
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
cases=['A-decay-n8','B-stiffdiag-n32-L1000','C-rotation-w10','D-damping-z1','E-nonnormal-transient','F-heat-n128','G-mixed-nearzero']
solvers=['vigilode','scipy_vode_adams','scipy_vode_bdf']
plans=[(c,s) for c in cases for s in solvers]+[(c,'julia') for c in cases]
random.Random(21021).shuffle(plans)
cpus=list(range(12,40)) # 28 independent one-thread workers; CPUs 0-3 reserved for OS/monitor
workers=[]; start_wall=time.monotonic(); deadline=279613.26
for i,(cid,solver) in enumerate(plans):
 if time.monotonic()>=deadline: break
 cpu=cpus[i%len(cpus)]; key=f'{cid}__{solver}'; stdout=JOB/'logs'/f'throughput-{key}.stdout.log'; stderr=JOB/'logs'/f'throughput-{key}.stderr.log'; tim=JOB/'logs'/f'throughput-{key}.gnu_time.txt'
 if solver=='julia':
  alg='Tsit5' if cid in ('A-decay-n8','C-rotation-w10') else 'Rodas5P'
  script=JOB/'harness/julia_solver_v2.jl'; request=JOB/'harness'/f'throughput-request-{cid}-{alg}.json'
  c=next(x for x in json.load(open(JOB/'harness/cases.json')) if x['case_id']==cid)
  request.write_text(json.dumps({'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':alg,'warmups':2,'measurements':7},allow_nan=False))
  cmd=['taskset','-c',str(cpu),'/usr/bin/time','-v','-o',str(tim),'/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia','--project='+str(JOB/'harness/julia-env'),str(script),str(request)]
  env={**os.environ,'JULIA_DEPOT_PATH':str(JOB/'julia-depot'),'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'}
 else:
  script=JOB/'harness/throughput_worker.py'
  cmd=['taskset','-c',str(cpu),'/usr/bin/time','-v','-o',str(tim),'python3',str(script),cid,solver]
  env={**os.environ,'VIGILODE_CPU':str(cpu),'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'}
 proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True,env=env)
 workers.append({'case_id':cid,'solver':solver,'cpu':cpu,'pid':proc.pid,'process_group':proc.pid,'process':proc,'stdout':stdout,'stderr':stderr,'time':tim,'started':time.monotonic()})

records=[]
for w in workers:
 p=w['process']; remaining=max(1,min(180,deadline-time.monotonic()))
 try: so,se=p.communicate(timeout=remaining); status='EXITED'
 except subprocess.TimeoutExpired:
  status='TIMEOUT'; os.killpg(p.pid,signal.SIGTERM)
  try: so,se=p.communicate(timeout=5)
  except subprocess.TimeoutExpired: os.killpg(p.pid,signal.SIGKILL); so,se=p.communicate()
 w['stdout'].write_text(so); w['stderr'].write_text(se)
 rec={k:v for k,v in w.items() if k not in ('process','stdout','stderr','time','started')}
 rec.update({'status':status,'exit_code':p.returncode,'wall_seconds':time.monotonic()-w['started'],'stdout_path':str(w['stdout']),'stderr_path':str(w['stderr']),'gnu_time_path':str(w['time']),'recorded_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())})
 records.append(rec)
 with open(JOB/'THROUGHPUT_WORKERS.jsonl','a') as f: f.write(json.dumps(rec,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())
 print(json.dumps(rec),flush=True)
end=time.monotonic(); rows=[]
for p in (JOB/'throughput_raw').glob('*.jsonl'):
 rows.extend(json.loads(x) for x in p.read_text().splitlines())
for w in records:
 if w['solver']!='julia' or w['exit_code']!=0: continue
 try: result=json.loads(w['stdout_path'] and pathlib.Path(w['stdout_path']).read_text().strip().splitlines()[-1])
 except Exception: continue
 # Julia rows are also materialized into THROUGHPUT_RESULTS.jsonl for unified analysis.
 case=next(x for x in json.load(open(JOB/'harness/cases.json')) if x['case_id']==w['case_id'])
 for r in result['measurements']:
  row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'throughput_concurrent_28_workers','case_id':w['case_id'],'family':case['family'],'dimension':len(case['initial_state']),'solver':'Julia OrdinaryDiffEq','backend':result['algorithm'],'repetition':r['repetition'],'warmup':False,'rtol':case['rtol'],'atol':case['atol'],'cpu_affinity':str(w['cpu']),'thread_count':1,'status':'OK' if r['retcode']=='Success' else 'NUMERICAL_FAILURE','retcode':r['retcode'],'elapsed_seconds':r['elapsed_seconds'],'states':r['states'],'states_final':r['states'][-1] if r['states'] else None,'solver_counters':r['counters'],'solver_stats':r['solver_stats'],'warmup_seconds':result['warmup_seconds'],'gnu_time_path':str(w['gnu_time_path'])}
  with open(JOB/'THROUGHPUT_RESULTS.jsonl','a') as f: f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())
summary={'protocol':'vigilode_ncp_intensive_120m_v1','mode':'throughput_concurrent','workers_planned':len(plans),'workers_started':len(workers),'workers_completed':sum(x['status']=='EXITED' and x['exit_code']==0 for x in records),'workers_timed_out':sum(x['status']=='TIMEOUT' for x in records),'workers_failed':sum(x['exit_code'] not in (0,None) for x in records),'parallel_workers':len(workers),'cpu_affinity_list':[x['cpu'] for x in records],'solver_threads_per_worker':1,'elapsed_seconds':end-start_wall,'measured_rows_nonjulia':len(rows),'broad_production':'HOLD'}
(JOB/'THROUGHPUT_SUMMARY.json.tmp').write_text(json.dumps(summary,indent=2)); os.replace(JOB/'THROUGHPUT_SUMMARY.json.tmp',JOB/'THROUGHPUT_SUMMARY.json'); print(json.dumps(summary))
