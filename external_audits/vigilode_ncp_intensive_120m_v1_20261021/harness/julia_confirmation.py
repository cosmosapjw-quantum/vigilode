import json,math,os,signal,subprocess,time,pathlib,random,re
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
JUL='/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia'
ENV=JOB/'harness/julia-env'; SCRIPT=JOB/'harness/julia_solver_v2.jl'
CASES={c['case_id']:c for c in json.load(open(JOB/'harness/cases.json'))}
plans=[('A-decay-n8','Tsit5'),('B-stiffdiag-n32-L1000','Rodas5P'),('C-rotation-w10','Tsit5'),('D-damping-z1','Rodas5P'),('E-nonnormal-transient','Rodas5P'),('F-heat-n128','Rodas5P'),('G-mixed-nearzero','Rodas5P')]
DEADLINE=279613.26
def exact(c,t):
 y=c['initial_state']; p=c['parameters']; m=c['exact_kind']
 if m=='diagonal_exponential': return [x*math.exp(r*t) for x,r in zip(y,p['rates'])]
 if m=='logistic':
  r=p['rate']; k=p['capacity']; q=y[0]; return [q*math.exp(r*t)/(1+q/k*(math.exp(r*t)-1))]
 if m=='rotation':
  w=p['omega']; return [y[0]*math.cos(w*t)-y[1]*math.sin(w*t),y[0]*math.sin(w*t)+y[1]*math.cos(w*t)]
 if m=='damped_oscillator':
  w=p['omega']; z=p['zeta']; a0=y[0]; al=z*w
  if z<1:
   wd=w*math.sqrt(1-z*z); b=(y[1]+al*a0)/wd; x=math.exp(-al*t)*(a0*math.cos(wd*t)+b*math.sin(wd*t)); v=math.exp(-al*t)*((-al*a0+wd*b)*math.cos(wd*t)+(-al*b-wd*a0)*math.sin(wd*t))
  elif z==1:
   b=y[1]+w*a0; x=(a0+b*t)*math.exp(-w*t); v=(b-w*(a0+b*t))*math.exp(-w*t)
  else:
   s=w*math.sqrt(z*z-1); r1=-al+s; r2=-al-s; c1=(y[1]-r2*a0)/(r1-r2); c2=a0-c1; x=c1*math.exp(r1*t)+c2*math.exp(r2*t); v=r1*c1*math.exp(r1*t)+r2*c2*math.exp(r2*t)
  return [x,v]
 if m=='nonnormal_triangular':
  a=p['a']; d=p['d']; b=p['b']; return [y[0]*math.exp(a*t)+b*y[1]*(math.exp(d*t)-math.exp(a*t))/(d-a),y[1]*math.exp(d*t)]
 if m=='periodic_heat_fourier':
  n=len(y); q=math.exp(-4*p['diffusion']*n*n*math.sin(math.pi/n)**2*t); return [x*q for x in y]
 return None

def metrics(c,times,states):
 ae=se=re=0.; end_abs=end_scale=None
 for t,yy in zip(times,states):
  rr=exact(c,t); ee=ss=0.
  for a,b,tol in zip(yy,rr,c['atol']):
   d=abs(a-b); z=d/(tol+c['rtol']*abs(b)); ae=max(ae,d); se=max(se,z); re=max(re,d/max(abs(b),tol)); ee=max(ee,d); ss=max(ss,z)
  end_abs,end_scale=ee,ss
 return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'endpoint_abs_error':end_abs,'endpoint_max_scaled_error':end_scale,'accuracy_pass_scaled_le_1':se<=1.0,'reference':'analytic exact, common sampled output grid'}

def append(path,row):
 with open(path,'a') as f: f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())

for cid,alg in plans:
 if time.monotonic()>=DEADLINE: raise SystemExit(2)
 c=CASES[cid]; req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':alg,'warmups':2,'measurements':7}
 reqpath=JOB/'harness'/f'julia-request-{cid}-{alg}.json'; reqpath.write_text(json.dumps(req,allow_nan=False))
 logbase=f'julia-confirm-{cid}-{alg}'; out=JOB/'logs'/f'{logbase}.stdout.log'; err=JOB/'logs'/f'{logbase}.stderr.log'; tim=JOB/'logs'/f'{logbase}.gnu_time.txt'; start=time.monotonic()
 cmd=['taskset','-c','8','/usr/bin/time','-v','-o',str(tim),JUL,'--project='+str(ENV),str(SCRIPT),str(reqpath)]
 p=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True,env={**os.environ,'JULIA_DEPOT_PATH':str(JOB/'julia-depot'),'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
 try: stdout,stderr=p.communicate(timeout=max(1,min(180,DEADLINE-time.monotonic()))); status='EXITED'
 except subprocess.TimeoutExpired:
  status='TIMEOUT'; os.killpg(p.pid,signal.SIGTERM)
  try: stdout,stderr=p.communicate(timeout=5)
  except subprocess.TimeoutExpired: os.killpg(p.pid,signal.SIGKILL); stdout,stderr=p.communicate()
 out.write_text(stdout); err.write_text(stderr)
 worker={'case_id':cid,'solver':'Julia OrdinaryDiffEq','algorithm':alg,'pid':p.pid,'process_group':p.pid,'status':status,'exit_code':p.returncode,'process_wall_seconds':time.monotonic()-start,'stdout_path':str(out),'stderr_path':str(err),'gnu_time_path':str(tim),'request_path':str(reqpath),'recorded_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
 if status!='EXITED' or p.returncode!=0:
  append(JOB/'JULIA_WORKERS.jsonl',worker); print(json.dumps(worker),flush=True); continue
 try: result=json.loads(stdout.strip().splitlines()[-1])
 except Exception as e:
  worker.update(status='OUTPUT_PARSE_FAILURE',error=repr(e)); append(JOB/'JULIA_WORKERS.jsonl',worker); continue
 worker.update(julia_version=result['julia_version'],warmup_seconds=result['warmup_seconds'],peak_rss_kb=None)
 try:
  for line in tim.read_text().splitlines():
   if 'Maximum resident set size' in line: worker['peak_rss_kb']=int(line.split(':')[-1].strip())
 except OSError: pass
 append(JOB/'JULIA_WORKERS.jsonl',worker)
 for r in result['measurements']:
  row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'confirmation','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':'Julia OrdinaryDiffEq','backend':alg,'repetition':r['repetition'],'warmup':False,'rtol':c['rtol'],'atol':c['atol'],'cpu_affinity':'8','thread_count':1,'source_commit':'de4c10a9f41321beef2313a3d7fb503bab35d714','status':'OK' if r['retcode'].endswith('.T') or r['retcode']=='Success' else 'NUMERICAL_FAILURE','retcode':r['retcode'],'elapsed_seconds':r['elapsed_seconds'],'states':r['states'],'states_final':r['states'][-1] if r['states'] else None,'accuracy':metrics(c,c['output_times'][:len(r['states'])],r['states']) if r['states'] else None,'solver_counters':r['counters'],'solver_stats':r['solver_stats'],'warmup_seconds':result['warmup_seconds'],'process_wall_seconds':worker['process_wall_seconds'],'peak_rss_kb':worker['peak_rss_kb'],'gnu_time_path':str(tim)}
  append(JOB/'JULIA_RESULTS.jsonl',row)
 print(json.dumps(worker),flush=True)
