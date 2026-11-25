"""Independent analytic root comparisons; not a matched-error timing campaign."""
import json,math
import scipy
from scipy.integrate import solve_ivp
for method in ['DOP853','Radau','BDF']:
 for rtol in [1e-6,1e-8]:
  for terminal in [True,False]:
   count=[0]
   def g(t,y):
    count[0]+=1
    return y[0]
   g.terminal=terminal;g.direction=0
   s=solve_ivp(lambda t,y:[y[1],-y[0]],(0,8.5),[1.,0.],method=method,events=g,rtol=rtol,atol=rtol*.01)
   expected=[math.pi/2] if terminal else [math.pi/2,3*math.pi/2,5*math.pi/2]
   actual=s.t_events[0].tolist()
   valid=s.success and len(actual)==len(expected)
   print(json.dumps({'method':method,'scipy':scipy.__version__,'rtol':rtol,'terminal':terminal,'ok':bool(valid),'roots':actual,'root_error':max(abs(a-b) for a,b in zip(actual,expected)) if valid else None,'rhs_nfev':s.nfev,'event_calls':count[0],'end':float(s.t[-1]),'status':s.status}))
