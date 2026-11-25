"""Independent external solver event/reset comparison, not matched-error timing."""
from pathlib import Path
import json,math
import scipy
from scipy.integrate import solve_ivp
from audit import read,oracle,m
P=Path(__file__).resolve().parent
for src in read(P/'training.log','SOURCE '):
 exact,final_mode,final=oracle(src)
 for method in ['DOP853','Radau','BDF']:
  for rt in [1e-6,1e-8]:
   modes=src['system']['modes'];t=0.;y=src['y'];mode=0;events=[];rhs=0;ec=0;ok=True
   for _ in range(65):
    mm=modes[mode];guard=mm['guard'];a,b=mm['rate'],mm['forcing']
    def f(t,y):return [(-a)*y[0]+b]
    def g(t,y):
     global ec
     ec+=1
     return y[0]-guard['threshold']
    g.terminal=True;g.direction=0
    sol=solve_ivp(f,(t,src['end']),[y],method=method,rtol=rt,atol=rt*.01,events=g if guard else None)
    rhs+=sol.nfev
    if not sol.success:ok=False;break
    if not guard or not len(sol.t_events[0]):
     t=float(sol.t[-1]);y=float(sol.y[0,-1]);break
    new=float(sol.t_events[0][0]);pre=float(sol.y_events[0][0][0])
    if new<=t:ok=False;break
    z=guard['reset'];y=z['state_scale']*pre+z['offset']+z['time_scale']*new
    events.append(new);t=new;mode=guard['next']
    if t>=src['end']:break
   ok=bool(ok and t==src['end'] and mode==final_mode and len(events)==len(exact))
   print(json.dumps({'case':src['case'],'method':method,'scipy':scipy.__version__,'rtol':rt,'ok':ok,'events':events,'rhs_nfev':rhs,'event_calls':ec,'root_error':float(max((abs(m(t)-e['time']) for t,e in zip(events,exact)),default=0)) if ok else None,'state_error':float(abs(m(y)-final)) if ok else None}))
