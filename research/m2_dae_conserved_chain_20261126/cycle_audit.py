from pathlib import Path
from fractions import Fraction as Q
import json,collections
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
P=Path(__file__).parent
def row(prefix):
 s=[json.loads(l.split(prefix+' ',1)[1])for l in (P/'focused3.log').read_text().splitlines()if prefix+' {'in l];assert len(s)==1;return s[0]
old=row('CYCLE_OLD');wide=row('CYCLE_DIAGNOSTIC');narrow=row('CYCLE_NARROW_DIAGNOSTIC')
assert old['stop']=='OrderUnresolved'and wide['stop']=='OrderUnresolved'
assert wide['work']['common']['transitions_committed']==1 and narrow['work']['common']['transitions_committed']>=2
assert old['guards'][0]['slabs'][0]['velocity'][0]<=0
assert wide['epochs'][0]['guards'][0]['slabs'][0]['velocity'][0]>0
v=Q(1,8192);threshold=float(Q(1,16384));end=.001
def mul(a,b):
 z=[x*y for x in a for y in b];return min(z),max(z)
def add(*x):return sum(a[0]for a in x),sum(a[1]for a in x)
def scale(a,k):return mul(a,[k,k])
def contain(v,b,tol):return b[0]-tol<=v<=b[1]+tol
def period(initial,method):
 sol=solve(3e7,(0,end),initial if method=='Radau'else initial[:2],method,3e-13,1e-16)
 rt=brentq(lambda t:evaluate(sol,t,method)[1]-threshold,0,end,xtol=5e-324,rtol=4*np.finfo(float).eps)
 return sol,rt
base=[float(1-v),float(v),0.]
nom={m:period(base,m)for m in ['Radau','BDF']}
cnt=collections.Counter()
for rad,r in [(Q(1,2**26),wide),(Q(1,2**30),narrow)]:
 seeds=[]
 for i in range(5):
  for j in range(5-i):
   z=rad*i/4;dv=rad*j/8;s=[float(1-v-z-dv),float(v+dv),float(z)]
   assert sum(map(Q,s))==1 and abs(Q(s[0])-(1-v))+abs(Q(s[1])-v)<=rad
   seeds.append(s)
 for e in r['epochs']:
  for g in e['guards']:
   for s in g['slabs']:
    if 'conserved_derivative_box'not in s:continue
    u,w,z=[tuple(map(Q,b))for b in s['conserved_derivative_box']]
    rng=add(scale(u,-Q(.04)),scale(mul(w,z),Q(10000)),scale(mul(w,w),Q(30000000)))
    assert Q(s['velocity'][0])<=rng[0]<=rng[1]<=Q(s['velocity'][1])
    cnt['exact_velocity_box_checks']+=1
 for seed in seeds:
  first={m:period(seed,m)[1]for m in ['Radau','BDF']}
  for i,e in enumerate(r['events']):
   if not e['committed']:continue
   a=first['Radau']+i*nom['Radau'][1];b=first['BDF']+i*nom['BDF'][1];tol=abs(a-b)+2e-14
   assert contain(a,e['event_time'],tol);cnt['event_checks']+=1
   if i:assert e['dwell_lower']<=nom['Radau'][1]+tol;cnt['dwell_checks']+=1
   s=e['event_time'][1]
   xa=evaluate(nom['Radau'][0],s-a,'Radau');xb=evaluate(nom['BDF'][0],s-b,'BDF')
   er=sum(abs(xa[:2]-e['transport']['center']))
   assert er<=e['transport']['selected_l1']+2*max(abs(xa-xb))+4e-13
   cnt['reset_radius_checks']+=1
# Independent true-root ordering for the hidden gap negative control.
m=4293935398*2**-40;eps=1024*2**-40
hidden={}
for method in ['Radau','BDF']:
 roots=[]
 for initial in [[1.-2**-20,0.,2**-20],[1.,0.,0.]]:
  a=solve(3e7,(0,1/32),initial if method=='Radau'else initial[:2],method,3e-13,1e-16)
  rt=brentq(lambda t:t+evaluate(a,t,method)[2]/128-1/256,0,1/32,xtol=5e-324,rtol=4*np.finfo(float).eps)
  roots.append(rt)
 assert roots[0]<m-eps<m+eps<roots[1]
 hidden[method]=roots
out={'counts':dict(cnt),'old_first_velocity':old['guards'][0]['slabs'][0]['velocity'],'tight_first_velocity':wide['epochs'][0]['guards'][0]['slabs'][0]['velocity'],'nominal_period':{m:v[1]for m,v in nom.items()},'hidden_center':m,'hidden_half_width':eps,'hidden_first_roots':hidden,'wide':{'stop':wide['stop'],'radii':[e['radius']for e in wide['epochs']],'events':[e['event_time']for e in wide['events']],'guard_failure':[g['reason']for g in wide['epochs'][-1]['guards']]},'narrow':{'stop':narrow['stop'],'radii':[e['radius']for e in narrow['epochs']],'events':[e['event_time']for e in narrow['events']],'dwell':[e['dwell_lower']for e in narrow['events']]}}
(P/'CYCLE_AUDIT.json').write_text(json.dumps(out,indent=2)+'\n');print(out)
