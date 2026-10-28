import os,sys,time,signal,subprocess,pathlib,json,random
job=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
cases=['A-decay-n8','B-stiffdiag-n32-L1000','C-rotation-w10','D-damping-z1','E-nonnormal-transient','F-heat-n128','G-mixed-nearzero']
solvers=['vigilode','scipy_vode_adams','scipy_vode_bdf']
summary=job/'CONFIRMATION_WORKERS.jsonl'
deadline=279613.26
for cid in cases:
 arms=solvers[:]; random.Random(21021+cases.index(cid)).shuffle(arms)
 for solver in arms:
  if time.monotonic()>=deadline:
   print('GLOBAL_DEADLINE_REACHED',flush=True); raise SystemExit(2)
  name=f'{cid}__{solver}'; out=job/'logs'/f'confirm-{name}.stdout.log'; err=job/'logs'/f'confirm-{name}.stderr.log'; tim=job/'logs'/f'confirm-{name}.gnu_time.txt'
  cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(tim),'python3',str(job/'harness/confirmation_worker.py'),cid,solver]
  start=time.monotonic(); p=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True,env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
  try:
   stdout,stderr=p.communicate(timeout=max(1,min(180,deadline-time.monotonic()))); status='EXITED'
  except subprocess.TimeoutExpired:
   status='TIMEOUT'; os.killpg(p.pid,signal.SIGTERM)
   try: stdout,stderr=p.communicate(timeout=5)
   except subprocess.TimeoutExpired:
    os.killpg(p.pid,signal.SIGKILL); stdout,stderr=p.communicate()
  out.write_text(stdout); err.write_text(stderr)
  rec={'case_id':cid,'solver':solver,'pid':p.pid,'process_group':p.pid,'status':status,'exit_code':p.returncode,'worker_wall_seconds':time.monotonic()-start,'stdout_path':str(out),'stderr_path':str(err),'gnu_time_path':str(tim),'recorded_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
  with open(summary,'a') as f: f.write(json.dumps(rec,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())
  print(json.dumps(rec),flush=True)
print('confirmation_workers_complete')
