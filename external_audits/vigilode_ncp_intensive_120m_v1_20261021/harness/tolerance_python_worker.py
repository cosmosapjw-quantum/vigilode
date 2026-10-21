import json,math,os,sys,time,subprocess,resource
JOB='/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z'
cid,solver,rtol=sys.argv[1],sys.argv[2],float(sys.argv[3])
c=next(x for x in json.load(open(JOB+'/harness/cases.json')) if x['case_id']==cid)
c=dict(c);c['rtol']=rtol
def exact(t):
 y=c['initial_state'];p=c['parameters'];return [x*math.exp(r*t) for x,r in zip(y,p['rates'])]
def metrics(ts,ys):
 ae=se=re=0.0;end=(None,None)
 for t,row in zip(ts,ys):
  ref=exact(t);e=ss=0.0
  for a,b,tol in zip(row,ref,c['atol']):
   d=abs(a-b);ae=max(ae,d);z=d/(tol+rtol*abs(b));se=max(se,z);re=max(re,d/max(abs(b),tol));e=max(e,d);ss=max(ss,z)
  end=(e,ss)
 return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'endpoint_abs_error':end[0],'endpoint_max_scaled_error':end[1],'accuracy_pass_scaled_le_1':se<=1,'reference':'analytic exact; common sampled output grid'}
for rep in range(9):
 warm=rep<2
 row={'protocol':'vigilode_ncp_intensive_120m_v1','block':'work_precision_serial','case_id':cid,'family':c['family'],'dimension':len(c['initial_state']),'solver':solver,'backend':'native-rodas5p-fast' if solver=='vigilode' else 'VODE '+solver.rsplit('_',1)[-1],'repetition':rep+1 if warm else rep-1,'warmup':warm,'rtol':rtol,'atol':c['atol'],'thread_count':1,'cpu_affinity':'8'}
 try:
  if solver=='vigilode':
   req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':'native-rodas5p-fast','solver_config':{}}
   rp=JOB+f'/harness/tolerance-request-{cid}-{rtol}-{rep}.json'
   open(rp,'w').write(json.dumps(req,allow_nan=False))
   q=subprocess.run([JOB+'/target/measurement/examples/component_benchmark_native','--request',rp],capture_output=True,text=True,timeout=120,env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
   row['process_exit']=q.returncode
   if q.returncode:
    row.update(status='PROCESS_FAILURE',stderr=q.stderr[-1000:])
   else:
    out=json.loads(q.stdout);row['status']=out.get('status');row['solver_counters']=out.get('counters');row['solver_details']=out.get('solver_details');row['timing']=out.get('timing');row['derivative_work']=out.get('derivative_work');row['states']=out.get('states');row['times']=out.get('times');row['accuracy']=metrics(row['times'],row['states']) if row['status']=='OK' else None
  else:
   from scipy.integrate import ode
   start=time.monotonic();cpu0=time.process_time()
   r=ode(lambda t,y:[a*b for a,b in zip(c['parameters']['rates'],y)]).set_integrator('vode',method=solver.rsplit('_',1)[-1],rtol=rtol,atol=c['atol'][0],nsteps=100000)
   r.set_initial_value(c['initial_state'],c['t_span'][0]);states=[];times=[]
   for ti in c['output_times'][1:]:
    yy=r.integrate(ti)
    if not r.successful():break
    states.append(yy.tolist());times.append(ti)
   row.update({'process_wall_seconds':time.monotonic()-start,'worker_cpu_seconds':time.process_time()-cpu0,'status':'OK' if r.successful() and len(states)==len(c['output_times'])-1 else 'NUMERICAL_FAILURE','states':states,'times':times,'method':solver.rsplit('_',1)[-1],'jacobian':'VODE default finite difference for BDF','solver_counters':{'nfev':None,'njev':None,'nlu':None},'accuracy':metrics(times,states) if states else None})
 except subprocess.TimeoutExpired:row.update(status='TIMEOUT',censored=True)
 except Exception as e:row.update(status='ENVIRONMENT_OR_ADAPTER_FAILURE',error=repr(e))
 print(json.dumps(row,allow_nan=False,separators=(',',':')),flush=True)
