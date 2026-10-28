import json,math,os,sys,time,subprocess,resource,pathlib
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
BIN=JOB/'target/measurement/examples/component_benchmark_native'
cases={c['case_id']:c for c in json.load(open(JOB/'harness/cases.json'))}
cid,solver=sys.argv[1],sys.argv[2]; c=cases[cid]
raw=JOB/'CONFIRMATION_RESULTS.jsonl'

def exact(t):
 y=c['initial_state']; p=c['parameters']; m=c['exact_kind']
 if m=='diagonal_exponential': return [x*math.exp(r*t) for x,r in zip(y,p['rates'])]
 if m=='logistic':
  r=p['rate']; k=p['capacity']; q=y[0]; return [q*math.exp(r*t)/(1+q/k*(math.exp(r*t)-1))]
 if m=='damped_oscillator':
  w=p['omega']; z=p['zeta']; al=z*w
  if z<1:
   wd=w*math.sqrt(1-z*z); a=y[0]; b=(y[1]+al*a)/wd
   x=math.exp(-al*t)*(a*math.cos(wd*t)+b*math.sin(wd*t)); v=math.exp(-al*t)*((-al*a+wd*b)*math.cos(wd*t)+(-al*b-wd*a)*math.sin(wd*t))
  elif z==1:
   a=y[0]; b=y[1]+w*a; x=(a+b*t)*math.exp(-w*t); v=(b-w*(a+b*t))*math.exp(-w*t)
  else:
   s=w*math.sqrt(z*z-1); r1=-al+s; r2=-al-s; c1=(y[1]-r2*y[0])/(r1-r2); c2=y[0]-c1; x=c1*math.exp(r1*t)+c2*math.exp(r2*t); v=r1*c1*math.exp(r1*t)+r2*c2*math.exp(r2*t)
  return [x,v]
 if m=='nonnormal_triangular':
  a=p['a']; d=p['d']; b=p['b']; return [y[0]*math.exp(a*t)+b*y[1]*(math.exp(d*t)-math.exp(a*t))/(d-a),y[1]*math.exp(d*t)]
 if m=='periodic_heat_fourier':
  n=len(y); q=math.exp(-4*p['diffusion']*n*n*math.sin(math.pi/n)**2*t); return [x*q for x in y]
 return None

def metrics(times,states):
 if c['exact_kind']=='numerical_reference': return None
 ae=se=re=0.0; endpoint=(None,None)
 for t,yy in zip(times,states):
  rr=exact(t); ee=ss=0.0
  for a,b,tol in zip(yy,rr,c['atol']):
   d=abs(a-b); z=d/(tol+c['rtol']*abs(b)); ae=max(ae,d); se=max(se,z); re=max(re,d/max(abs(b),tol)); ee=max(ee,d); ss=max(ss,z)
  endpoint=(ee,ss)
 return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'endpoint_abs_error':endpoint[0],'endpoint_max_scaled_error':endpoint[1],'accuracy_pass_scaled_le_1':se<=1.0,'reference':'analytic exact, common sampled output grid'}

def rhs(t,y):
 m=c['model']; p=c['parameters']
 if m=='diagonal_stiff': return [a*b for a,b in zip(p['rates'],y)]
 if m=='damping': return [y[1],-p['omega']**2*y[0]-2*p['zeta']*p['omega']*y[1]]
 if m=='nonnormal': return [p['a']*y[0]+p['b']*y[1],p['d']*y[1]]
 if m=='periodic_heat':
  n=len(y); d=p['diffusion']*n*n; return [d*(y[(i-1)%n]-2*y[i]+y[(i+1)%n]) for i in range(n)]
 raise ValueError('unsupported '+m)

def append(x):
 with open(raw,'a') as f: f.write(json.dumps(x,allow_nan=False,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())

for rep in range(9):
 warm=rep<2; row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'confirmation','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':solver,'repetition':rep-2 if not warm else rep+1,'warmup':warm,'rtol':c['rtol'],'atol':c['atol'],'cpu_affinity':'8','thread_count':1,'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_commit':'de4c10a9f41321beef2313a3d7fb503bab35d714'}
 try:
  if solver=='vigilode':
   req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':'native-rodas5p-fast','solver_config':{}}
   rp=JOB/'harness/confirm-request.json'; rp.write_text(json.dumps(req,allow_nan=False))
   before=resource.getrusage(resource.RUSAGE_CHILDREN); start=time.monotonic(); cpu0=time.process_time()
   q=subprocess.run([str(BIN),'--request',str(rp)],capture_output=True,text=True,timeout=120,env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
   row['process_wall_seconds']=time.monotonic()-start; row['worker_cpu_seconds']=time.process_time()-cpu0
   after=resource.getrusage(resource.RUSAGE_CHILDREN); row['child_cpu_seconds']=(after.ru_utime+after.ru_stime)-(before.ru_utime+before.ru_stime); row['child_peak_rss_kb']=after.ru_maxrss
   row['process_exit']=q.returncode
   if q.returncode: row.update(status='PROCESS_FAILURE',stderr=q.stderr[-1000:])
   else:
    out=json.loads(q.stdout); row['status']=out.get('status'); row['solver_counters']=out.get('counters'); row['solver_details']=out.get('solver_details'); row['timing']=out.get('timing'); row['derivative_work']=out.get('derivative_work'); row['states_final']=out.get('states',[])[-1] if out.get('states') else None; row['accuracy']=metrics(out.get('times',[]),out.get('states',[])) if out.get('status')=='OK' else None
  else:
   from scipy.integrate import ode
   method='adams' if solver.endswith('adams') else 'bdf'; start=time.monotonic(); cpu0=time.process_time()
   r=ode(rhs).set_integrator('vode',method=method,rtol=c['rtol'],atol=c['atol'][0],nsteps=100000); r.set_initial_value(c['initial_state'],c['t_span'][0]); states=[]; times=[]
   for ti in c['output_times'][1:]:
    yy=r.integrate(ti)
    if not r.successful(): break
    states.append(yy.tolist()); times.append(ti)
   row['process_wall_seconds']=time.monotonic()-start; row['worker_cpu_seconds']=time.process_time()-cpu0; row['status']='OK' if r.successful() and len(states)==len(c['output_times'])-1 else 'NUMERICAL_FAILURE'; row['states_final']=states[-1] if states else None; row['accuracy']=metrics(times,states); row['method']=method; row['jacobian']='VODE default finite difference for implicit mode'; row['solver_counters']={'nfev':None,'njev':None,'nlu':None}
 except subprocess.TimeoutExpired:
  row.update(status='TIMEOUT',censored=True)
 except Exception as e: row.update(status='ENVIRONMENT_OR_ADAPTER_FAILURE',error=repr(e))
 row['worker_peak_rss_kb']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
 row['finished_monotonic']=time.monotonic(); append(row)
print(json.dumps({'case_id':cid,'solver':solver,'rows':9,'status':'WORKER_COMPLETED'}))
