import numpy as np
from scipy.integrate import solve_ivp
from pathlib import Path
import scipy,json
P=Path(__file__).parent
a=.04;b=1e4
def rhs3(c):
 def f(t,y):
  u,v,z=y;k=a*u-b*v*z;l=c*v*v
  return [-k,k-l,l]
 def j(t,y):
  u,v,z=y;return [[-a,b*z,b*v],[a,-b*z-2*c*v,-b*v],[0,2*c*v,0]]
 return f,j
def rhs2(c):
 def f(t,y):
  u,v=y;z=1-u-v;k=a*u-b*v*z;l=c*v*v
  return [-k,k-l]
 def j(t,y):
  u,v=y;z=1-u-v;return [[-a-b*v,b*(z-v)],[a+b*v,-b*(z-v)-2*c*v]]
 return f,j
def solve(c,span,y,method,rtol,atol):
 f,j=rhs3(c)if method=='Radau'else rhs2(c)
 r=solve_ivp(f,span,y,method=method,jac=j,dense_output=True,rtol=rtol,atol=atol)
 assert r.success,(method,r.message)
 return r
def evaluate(r,t,method):
 y=np.asarray(r.sol(t))
 if method=='BDF':y=np.vstack((y,1-y[0]-y[1])) if y.ndim==2 else np.array([*y,1-y[0]-y[1]])
 return y
def references(t,hybrid=False):
 out={};stats={}
 for method in ['Radau','BDF']:
  for level,rtol,atol in [('coarse',3e-11,1e-14),('fine',3e-13,1e-16)]:
   key=method+'-'+level
   y=[1.,0.,0.]if method=='Radau'else[1.,0.]
   if hybrid:
    s=1/256;end=1/32;r0=solve(3e7,(0,s),y,method,rtol,atol);v=r0.y[:,-1].copy()
    if method=='Radau':v[2]+=v[1]/2
    v[1]/=2
    r1=solve(6e7,(s,end),v,method,rtol,atol)
    # At the exact clock distinguish both one-sided values.
    out[key]={'pre':evaluate(r0,t['pre'],method).T,'post':evaluate(r1,t['post'],method).T}
    stats[key]=[{'nfev':r.nfev,'njev':r.njev,'nlu':r.nlu,'steps':len(r.t)-1}for r in [r0,r1]]
   else:
    r=solve(3e7,(0,1),y,method,rtol,atol);out[key]=evaluate(r,t,method).T
    stats[key]={'nfev':r.nfev,'njev':r.njev,'nlu':r.nlu,'steps':len(r.t)-1}
 return out,stats
if __name__=='__main__':
 def load(n,prefix):return [json.loads(l.split(prefix,1)[1])for l in (P/n).read_text().splitlines()if prefix+'{'in l]
 rows=load('training.log','CELL ');hy=load('hybrid.log','HYBRID ')
 ts=sorted({x['t'] for r in rows if r['case']=='robertson'and 'run'in r for k in ['accepted','queries']for x in r['run'][k]})
 ht={k:sorted({x['t']for r in hy if k in r for j in ['accepted','queries']for x in r[k][j]})for k in ['pre','post']}
 ref,stats=references(ts);href,hstats=references(ht,True)
 def spread(a):return float(max(np.max(np.abs(a[k]-a['Radau-fine']))for k in a))
 s={'scipy':scipy.__version__,'times':ts,'values':{k:v.tolist()for k,v in ref.items()},'stats':stats,'spread':spread(ref),'hybrid_times':ht,'hybrid_values':{k:{j:v.tolist()for j,v in vs.items()}for k,vs in href.items()},'hybrid_stats':hstats,'hybrid_spread':{j:spread({k:v[j]for k,v in href.items()})for j in ht}}
 (P/'REFERENCE.json').write_text(json.dumps(s,indent=2)+'\n')
 print('reference',len(ts),{k:len(v)for k,v in ht.items()},s['spread'],s['hybrid_spread'])
 print('fine Robertson endpoint',ref['Radau-fine'][-1].tolist(),ref['BDF-fine'][-1].tolist())
