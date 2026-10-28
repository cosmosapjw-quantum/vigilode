import json,math,os,sys,time,subprocess,resource
JOB='/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z'
cid,solver=sys.argv[1],sys.argv[2]
c=next(x for x in json.load(open(JOB+'/harness/cases.json')) if x['case_id']==cid)
def rhs(t,y):
 p=c['parameters'];m=c['model']
 if m=='robertson':
  a=p['k1']*y[0];b=p['k2']*y[1]*y[2];z=p['k3']*y[1]*y[1];return [-a+b,a-b-z,z]
 if m=='vanderpol':return [y[1],p['mu']*(1-y[0]**2)*y[1]-y[0]]
 raise ValueError(m)
for rep in range(9):
 warm=rep<2;row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'stiff_nonlinear_confirmation','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':solver,'repetition':rep+1 if warm else rep-1,'warmup':warm,'rtol':c['rtol'],'atol':c['atol'],'thread_count':1,'cpu_affinity':'8','accuracy':None,'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE'}
 try:
  if solver=='vigilode':
   req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':'native-rodas5p-fast','solver_config':{}}
   rp=JOB+f'/harness/nonlinear-request-{cid}-{rep}.json';open(rp,'w').write(json.dumps(req,allow_nan=False))
   start=time.monotonic();q=subprocess.run([JOB+'/target/measurement/examples/component_benchmark_native','--request',rp],capture_output=True,text=True,timeout=120,env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
   row['process_wall_seconds']=time.monotonic()-start;row['process_exit']=q.returncode
   if q.returncode:row.update(status='PROCESS_FAILURE',stderr=q.stderr[-1000:])
   else:
    o=json.loads(q.stdout);row.update({'status':o.get('status'),'timing':o.get('timing'),'solver_counters':o.get('counters'),'solver_details':o.get('solver_details'),'states':o.get('states'),'times':o.get('times')})
  else:
   from scipy.integrate import ode
   method=solver.rsplit('_',1)[-1];r=ode(rhs).set_integrator('vode',method=method,rtol=c['rtol'],atol=c['atol'][0],nsteps=100000);r.set_initial_value(c['initial_state'],c['t_span'][0]);states=[];times=[];start=time.monotonic();cpu0=time.process_time()
   for t in c['output_times'][1:]:
    y=r.integrate(t)
    if not r.successful():break
    states.append(y.tolist());times.append(t)
   row.update({'status':'OK' if r.successful() and len(states)==len(c['output_times'])-1 else 'NUMERICAL_FAILURE','process_wall_seconds':time.monotonic()-start,'worker_cpu_seconds':time.process_time()-cpu0,'states':states,'times':times,'method':method,'solver_counters':{'nfev':None,'njev':None,'nlu':None}})
  states=row.get('states') or []
  if c['model']=='robertson' and states:
   row['invariants']={'minimum_component':min(min(y) for y in states),'max_abs_mass_residual':max(abs(sum(y)-sum(c['initial_state'])) for y in states),'positivity_pass_nonnegative':min(min(y) for y in states)>=0}
  if c['model']=='vanderpol' and states: row['trajectory_descriptors']={'max_abs_x':max(abs(y[0]) for y in states),'max_abs_v':max(abs(y[1]) for y in states)}
 except subprocess.TimeoutExpired:row.update(status='TIMEOUT',censored=True)
 except Exception as e:row.update(status='ENVIRONMENT_OR_ADAPTER_FAILURE',error=repr(e))
 print(json.dumps(row,allow_nan=False,separators=(',',':')),flush=True)
