import json,math,pathlib,subprocess,time,os,sys,hashlib
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
BIN=JOB/'diffsol-target/release/vigilode-diffsol-adapter'
cases=json.loads((JOB/'harness/cases.json').read_text())
plan=[
 ('A-decay-n8','Tsit45'),('B-stiffdiag-n32-L1000','BDF'),('C-rotation-w10','Tsit45'),
 ('D-damping-z1','Tsit45'),('E-nonnormal-transient','Tsit45'),('F-heat-n128','BDF'),
 ('G-mixed-nearzero','BDF'),('A-logistic-r1e-06','Tsit45'),
 ('B-robertson-short','BDF'),('B-vdp-mu20','BDF')]
def exact(c,t):
 y=c['initial_state']; p=c['parameters']; m=c.get('exact_kind')
 if m=='diagonal_exponential': return [x*math.exp(r*t) for x,r in zip(y,p['rates'])]
 if m=='logistic':
  r=p['rate']; k=p['capacity']; q=y[0]; return [q*math.exp(r*t)/(1+q/k*(math.exp(r*t)-1))]
 if m=='rotation':
  w=p['omega']; return [y[0]*math.cos(w*t)-y[1]*math.sin(w*t), y[0]*math.sin(w*t)+y[1]*math.cos(w*t)]
 if m=='damped_oscillator':
  w=p['omega']; z=p['zeta']; al=z*w
  if z<1:
   wd=w*math.sqrt(1-z*z); a=y[0]; b=(y[1]+al*a)/wd
   return [math.exp(-al*t)*(a*math.cos(wd*t)+b*math.sin(wd*t)), math.exp(-al*t)*((-al*a+wd*b)*math.cos(wd*t)+(-al*b-wd*a)*math.sin(wd*t))]
  if z==1:
   a=y[0]; b=y[1]+w*a; return [(a+b*t)*math.exp(-w*t),(b-w*(a+b*t))*math.exp(-w*t)]
  s=w*math.sqrt(z*z-1); r1=-al+s; r2=-al-s; c1=(y[1]-r2*y[0])/(r1-r2); c2=y[0]-c1
  return [c1*math.exp(r1*t)+c2*math.exp(r2*t),r1*c1*math.exp(r1*t)+r2*c2*math.exp(r2*t)]
 if m=='nonnormal_triangular':
  a=p['a']; d=p['d']; b=p['b']; return [y[0]*math.exp(a*t)+b*y[1]*(math.exp(d*t)-math.exp(a*t))/(d-a),y[1]*math.exp(d*t)]
 if m=='periodic_heat_fourier':
  n=len(y); q=math.exp(-4*p['diffusion']*n*n*math.sin(math.pi/n)**2*t); return [x*q for x in y]
 return None
def err(c,ts,ys):
 if c.get('exact_kind')=='numerical_reference': return None
 ae=se=re=0.0; end=(None,None)
 for t,row in zip(ts,ys):
  ref=exact(c,t); e=ss=0.0
  for a,b,tol in zip(row,ref,c['atol']):
   d=abs(a-b); ae=max(ae,d); se=max(se,d/(tol+c['rtol']*abs(b))); re=max(re,d/max(abs(b),tol)); e=max(e,d); ss=max(ss,d/(tol+c['rtol']*abs(b)))
  end=(e,ss)
 return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'endpoint_abs_error':end[0],'endpoint_max_scaled_error':end[1],'accuracy_pass_scaled_le_1':se<=1.0,'reference':'analytic exact sampled output grid'}
def append(path,row):
 with open(path,'a') as f:f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\\n');f.flush();os.fsync(f.fileno())
for i,(cid,alg) in enumerate(plan):
 c=next((z for z in cases if z['case_id']==cid),None)
 if c is None:
  print('CASE_NOT_FOUND',cid,flush=True);continue
 request=JOB/'harness'/f'diffsol-request-{cid}.json'
 req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'algorithm':alg}
 request.write_text(json.dumps(req,allow_nan=False))
 sout=JOB/'logs'/f'diffsol-{cid}-{alg}.stdout.log';serr=JOB/'logs'/f'diffsol-{cid}-{alg}.stderr.log';gt=JOB/'logs'/f'diffsol-{cid}-{alg}.gnu_time.txt'
 cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(gt),str(BIN),str(request)]
 start=time.monotonic()
 proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'},start_new_session=True)
 try:
  so,se=proc.communicate(timeout=max(1,min(180,279613.26-time.monotonic())))
  rc=proc.returncode
  status='EXITED' if rc==0 else 'FAILED'
  sout.write_text(so);serr.write_text(se)
  q=type('RunResult',(),{'returncode':rc,'stdout':so,'stderr':se})()
  if q.returncode==0:
   result=json.loads(q.stdout); measurements=result['measurements']; warmups=result['warmup']
   for r in measurements:
    states=r['states']; ts=c['output_times']; valid=(len(states)==len(ts) and 'TstopReached' in r['stop_reason'])
    row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'diffsol_latency_serial','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':'diffsol','backend':alg,'diffsol_version':'0.17.1','linear_solver':result['linear_solver'],'jacobian':result['jacobian'],'repetition':r['repetition'],'warmup':False,'rtol':c['rtol'],'atol':c['atol'],'thread_count':1,'cpu_affinity':'8','status':'OK' if valid else 'NUMERICAL_FAILURE','stop_reason':r['stop_reason'],'elapsed_seconds':r['elapsed_seconds'],'setup_seconds':r['setup_seconds'],'states_final':states[-1] if states else None,'states':states,'accuracy':err(c,ts,states) if valid else None,'solver_counters':None,'gnu_time_path':str(gt),'worker_process_wall_seconds':time.monotonic()-start}
    append(JOB/'DIFFSOL_RESULTS.jsonl',row)
   rec={'case_id':cid,'algorithm':alg,'status':'EXITED','exit_code':q.returncode,'wall_seconds':time.monotonic()-start,'warmups':len(warmups),'measurements':len(measurements),'return_grid_points':len(measurements[0]['states']) if measurements else None,'stdout_path':str(sout),'stderr_path':str(serr),'gnu_time_path':str(gt)}
  else:
   rec={'case_id':cid,'algorithm':alg,'status':'FAILED','exit_code':q.returncode,'wall_seconds':time.monotonic()-start,'stdout_path':str(sout),'stderr_path':str(serr),'gnu_time_path':str(gt)}
 except subprocess.TimeoutExpired:
  import signal
  os.killpg(proc.pid,signal.SIGTERM)
  try: so,se=proc.communicate(timeout=5)
  except subprocess.TimeoutExpired:
   os.killpg(proc.pid,signal.SIGKILL);so,se=proc.communicate()
  sout.write_text(so);serr.write_text(se)
  rec={'case_id':cid,'algorithm':alg,'status':'TIMEOUT','exit_code':None,'wall_seconds':time.monotonic()-start,'stdout_path':str(sout),'stderr_path':str(serr),'gnu_time_path':str(gt),'killed_process_group':proc.pid}
 append(JOB/'DIFFSOL_WORKERS.jsonl',rec)
 print(json.dumps(rec),flush=True)
 if rec['status']!='EXITED': break
rows=[json.loads(x) for x in (JOB/'DIFFSOL_RESULTS.jsonl').read_text().splitlines()] if (JOB/'DIFFSOL_RESULTS.jsonl').exists() else []
summary={'planned_cases':len(plan),'worker_records':sum(1 for _ in (JOB/'DIFFSOL_WORKERS.jsonl').open()) if (JOB/'DIFFSOL_WORKERS.jsonl').exists() else 0,'measurement_rows':len(rows),'completed_rows':sum(x['status']=='OK' for x in rows),'failed_rows':sum(x['status']!='OK' for x in rows),'broad_production':'HOLD'}
tmp=JOB/'DIFFSOL_SUMMARY.json.tmp';tmp.write_text(json.dumps(summary,indent=2));os.replace(tmp,JOB/'DIFFSOL_SUMMARY.json')
print(json.dumps(summary))
