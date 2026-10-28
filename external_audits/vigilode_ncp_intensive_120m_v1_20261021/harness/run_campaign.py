import json, math, os, subprocess, time, pathlib, random
JOB=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
SRC=pathlib.Path('/root/vigilode-reaudit-20261021T0549Z/checkout')
BIN=JOB/'target/measurement/examples/component_benchmark_native'
CASES=json.load(open(JOB/'harness/cases.json'))
RAW=JOB/'RAW_RESULTS.jsonl'
T0=time.monotonic(); DEADLINE=T0+7200

def exact(c,t):
    y=c['initial_state']; p=c['parameters']; m=c['exact_kind']
    if m=='diagonal_exponential': return [x*math.exp(r*t) for x,r in zip(y,p['rates'])]
    if m=='logistic':
        r=p['rate']; k=p['capacity']; q=y[0]; z=q*math.exp(r*t)/(1+q/k*(math.exp(r*t)-1)); return [z]
    if m=='rotation':
        w=p['omega']; return [y[0]*math.cos(w*t)-y[1]*math.sin(w*t), y[0]*math.sin(w*t)+y[1]*math.cos(w*t)]
    if m=='damped_oscillator':
        w=p['omega']; z=p['zeta']; alpha=z*w
        if z<1:
            wd=w*math.sqrt(1-z*z); a=y[0]; b=(y[1]+alpha*a)/wd
            x=math.exp(-alpha*t)*(a*math.cos(wd*t)+b*math.sin(wd*t))
            v=math.exp(-alpha*t)*((-alpha*a+wd*b)*math.cos(wd*t)+(-alpha*b-wd*a)*math.sin(wd*t))
        elif z==1:
            a=y[0]; b=y[1]+w*a; x=(a+b*t)*math.exp(-w*t); v=(b-w*(a+b*t))*math.exp(-w*t)
        else:
            s=w*math.sqrt(z*z-1); r1=-alpha+s; r2=-alpha-s; c1=(y[1]-r2*y[0])/(r1-r2); c2=y[0]-c1
            x=c1*math.exp(r1*t)+c2*math.exp(r2*t); v=r1*c1*math.exp(r1*t)+r2*c2*math.exp(r2*t)
        return [x,v]
    if m=='nonnormal_triangular':
        a=p['a']; d=p['d']; b=p['b']; x=y[0]*math.exp(a*t)+b*y[1]*(math.exp(d*t)-math.exp(a*t))/(d-a); return [x,y[1]*math.exp(d*t)]
    if m=='periodic_heat_fourier':
        n=len(y); decay=math.exp(-4*p['diffusion']*n*n*math.sin(math.pi/n)**2*t); return [x*decay for x in y]
    return None

def errors(c,times,states):
    if not times or c['exact_kind']=='numerical_reference': return None
    refs=[exact(c,t) for t in times]; ae=se=re=0.0
    for yy,rr in zip(states,refs):
        for a,b,tol in zip(yy,rr,c['atol']):
            d=abs(a-b); ae=max(ae,d); se=max(se,d/(tol+c['rtol']*abs(b))); re=max(re,d/max(abs(b),tol))
    return {'max_abs_error':ae,'max_scaled_error':se,'max_relative_error_atol_floor':re,'accuracy_pass_scaled_le_1':se<=1.0,'reference':'analytic exact; sampled grid'}

def append(row):
    with open(RAW,'a') as f: f.write(json.dumps(row,allow_nan=False,separators=(',',':'))+'\n'); f.flush(); os.fsync(f.fileno())

random.seed(21021)
solvers=['vigilode','scipy_vode_adams','scipy_vode_bdf']
if not BIN.exists():
    while time.monotonic()<DEADLINE and not BIN.exists(): time.sleep(2)
for ci,c in enumerate(CASES):
    if time.monotonic()>=DEADLINE or not BIN.exists(): break
    for solver in solvers:
        order=list(range(4)); random.Random(21021+ci*31+len(solver)).shuffle(order)
        for rep in order:
            if time.monotonic()>=DEADLINE: break
            warm=rep==0; row={'protocol':'vigilode_ncp_intensive_120m_v1','case_id':c['case_id'],'family':c['family'],'dimension':len(c['initial_state']),'solver':solver,'repetition':rep,'warmup':warm,'rtol':c['rtol'],'atol':c['atol'],'thread_count':1,'mode':'latency_serial','source_commit':'de4c10a9f41321beef2313a3d7fb503bab35d714','started_monotonic':time.monotonic()}; start=time.monotonic()
            try:
                if solver=='vigilode':
                    req={'case':{k:v for k,v in c.items() if k!='exact_kind'},'solver_id':'native-rodas5p-fast','solver_config':{}}
                    rp=JOB/'harness/request.json'; rp.write_text(json.dumps(req,allow_nan=False))
                    q=subprocess.run([str(BIN),'--request',str(rp)],capture_output=True,text=True,timeout=min(30,max(1,DEADLINE-time.monotonic())),env={**os.environ,'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1'})
                    row['process_exit']=q.returncode
                    if q.returncode: row.update(status='PROCESS_FAILURE',stderr=q.stderr[-1000:])
                    else:
                        out=json.loads(q.stdout); row['status']=out.get('status'); row['solver_counters']=out.get('counters'); row['solver_details']=out.get('solver_details'); row['derivative_work']=out.get('derivative_work'); row['accuracy']=errors(c,out.get('times',[]),out.get('states',[])) if out.get('status')=='OK' else None; row['states_final']=out.get('states',[])[-1] if out.get('states') else None
                else:
                    from scipy.integrate import ode
                    def f(t,y):
                        m=c['model']; p=c['parameters']
                        if m=='diagonal_stiff': return [a*b for a,b in zip(p['rates'],y)]
                        if m=='logistic': return [p['rate']*y[0]*(1-y[0]/p['capacity'])]
                        if m=='rotation': return [-p['omega']*y[1],p['omega']*y[0]]
                        if m=='damping': return [y[1],-p['omega']**2*y[0]-2*p['zeta']*p['omega']*y[1]]
                        if m=='nonnormal': return [p['a']*y[0]+p['b']*y[1],p['d']*y[1]]
                        if m=='robertson':
                            a=p['k1']*y[0]; b=p['k2']*y[1]*y[2]; z=p['k3']*y[1]**2; return [-a+b,a-b-z,z]
                        if m=='vanderpol': return [y[1],p['mu']*(1-y[0]**2)*y[1]-y[0]]
                        if m=='periodic_heat':
                            n=len(y); d=p['diffusion']*n*n; return [d*(y[(i-1)%n]-2*y[i]+y[(i+1)%n]) for i in range(n)]
                        return [0.0]*len(y)
                    method='adams' if solver.endswith('adams') else 'bdf'; r=ode(f).set_integrator('vode',method=method,rtol=c['rtol'],atol=c['atol'][0],nsteps=100000); r.set_initial_value(c['initial_state'],0.0); states=[]; times=[]
                    for ti in c['output_times'][1:]:
                        yy=r.integrate(ti)
                        if not r.successful(): break
                        states.append(yy.tolist()); times.append(ti)
                    row['status']='OK' if r.successful() and len(states)==len(c['output_times'])-1 else 'NUMERICAL_FAILURE'; row['states_final']=states[-1] if states else None; row['accuracy']=errors(c,times,states); row['available_counters']={'nfev':None,'njev':None,'nlu':None,'reason':'SciPy VODE wrapper exposes no stable counters'}; row['method']=method; row['jacobian']='internal finite differences/default VODE'
            except subprocess.TimeoutExpired:
                row.update(status='TIMEOUT',censored=True)
            except Exception as e: row.update(status='ENVIRONMENT_OR_ADAPTER_FAILURE',error=repr(e))
            row['wall_seconds']=time.monotonic()-start; row['finished_monotonic']=time.monotonic(); append(row)
rows=[json.loads(x) for x in RAW.read_text().splitlines()] if RAW.exists() else []; meas=[x for x in rows if not x.get('warmup')]
coverage={'planned':len(CASES)*len(solvers)*3,'raw_rows':len(rows),'completed':sum(x['status']=='OK' for x in meas),'failed':sum(x['status'] not in ('OK','TIMEOUT') for x in meas),'timed_out':sum(x['status']=='TIMEOUT' for x in meas),'not_run':max(0,len(CASES)*len(solvers)*3-len(meas)),'warmups':sum(x.get('warmup',False) for x in rows)}
(JOB/'COVERAGE.json.tmp').write_text(json.dumps(coverage,indent=2)); os.replace(JOB/'COVERAGE.json.tmp',JOB/'COVERAGE.json'); print(json.dumps(coverage))
