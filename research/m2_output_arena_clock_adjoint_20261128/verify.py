from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections
P=Path(__file__).parent
def read(path,marker):
 if not path.exists():path=Path(str(path)+'.gz')
 with (gzip.open(path,'rt') if path.suffix=='.gz' else path.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(name,obj):(P/name).write_text(json.dumps(obj,sort_keys=True,indent=2,allow_nan=False)+'\n')
def requests(a):return a['alloc']+a['zeroed']+a['realloc']
def contains(iv,truth):
 lo,hi=map(Q,iv);assert lo<=truth[0]<=truth[1]<=hi,(iv,list(map(float,truth)))
def log_unit(d):
 s=(d-1)/(d+1);assert 0<=s<=Q(1,3)
 a=2*sum((s**(2*j+1)/(2*j+1)for j in range(80)),Q(0))
 return a,a+2*s**161/(161*(1-s*s))
@lru_cache(None)
def log_bounds(d):
 x=d-1;assert x>=0
 if x<=Q(1,2**40):return x-x*x/2,x
 k=0
 while d>2:d/=2;k+=1
 a,b=log_unit(d);c,e=log_unit(Q(2));return a+k*c,b+k*e
def model(r):
 n=len(r['initial']);case=r['case'];start=Q(r['start']);end=Q(r['end']);k=r.get('k',1)
 s=(end**(k+1)-start**(k+1))/(k+1)
 x=list(map(Q,r['initial']));w=list(map(Q,r['weights']))
 for i in range(0,n,2):
  a=Q(1.5 if case.startswith('mono') else i//2+1 if case=='clock8' else 1)
  b=Q(r.get('b',1));p=x[i]-x[i+1] if case=='coupled_clock' else x[i];q=x[i+1]
  d=1+a*p*s
  wu=w[i]+w[n+i];wv=w[i+1]+w[n+i+1];wq=wu+wv if case=='coupled_clock' else wv
  lp=wu/d**2+wq*b*s/d
  lq=wq-lp if case=='coupled_clock' else wq
  pe=p/d
  l,h=log_bounds(d)
  qe=sorted((q+b*l/a,q+b*h/a))
  ue=(pe+qe[0],pe+qe[1]) if case=='coupled_clock' else (pe,pe)
  yield i,(lp,lq),(ue,qe)
def goal(r):
 n=len(r['initial']);y=list(map(Q,r['terminal']));w=list(map(Q,r['weights']));lo=hi=Q(0)
 for i,_,(u,v) in model(r):
  for j,(a,b) in [(i,u),(i+1,v),(n+i,u),(n+i+1,v)]:
   q=sorted(((a-y[j])*w[j],(b-y[j])*w[j]));lo+=q[0];hi+=q[1]
 return lo,hi
arena=read(P/'arena.log','ARENA_IVP');ode=read(P/'ode.log','ARENA_ODE');grad=read(P/'clock.log','CLOCK_GRAD');goals=read(P/'clock.log','CLOCK_GOAL');tiny=read(P/'tiny.log','TINY_LOG')
assert (len(arena),len(ode),len(grad),len(goals),len(tiny))==(252,6,24,24,28),(len(arena),len(ode),len(grad),len(goals),len(tiny))
sp={(r['case'],r['schedule'],r['partial'],r['plan'],r['candidate']):r for r in arena}
assert len(sp)==252
prev=read(P.parent/'m2_triangular_goal_spatial_reuse_20261128/spatial.log','REUSE_IVP')
prior={(r['case'],r['plan']):r for r in prev if r['trial']==0 and r['candidate']=='Reusable'}
counts=collections.defaultdict(lambda:dict(rows=0,requests=0,bytes=0,partial=0,success=0))
for k,r in sp.items():
 old=sp[(*k[:-1],'Owning')]
 assert r['run']==old['run'];a=r['allocation'];s=counts[r['candidate']]
 s['rows']+=1;s['requests']+=requests(a);s['bytes']+=a['requested_bytes'];s['partial']+=r['partial'];s['success']+=not r['partial']
 if r['candidate']!='Owning':assert requests(a)==0 and a['requested_bytes']==0
 if r['schedule']==0 and not r['partial']:
  o=prior[(r['case'],r['plan'])]['run'];v=r['run']
  assert v['last_state']==o['values'] and v['counters']==o['counters'] and v['accepted']==o['accepted'] and v['rejected']==o['rejected']
for r in ode:assert requests(r['allocation'])==0 and r['qualified'] and r['max_error']<=1e-8
gcount=collections.defaultdict(lambda:dict(rows=0,qualified=0,zeroheap=0,actions=0,max_width=0.))
for r in grad:
 s=gcount[r['candidate']];s['rows']+=1
 if not r['qualified']:continue
 truth=[]
 for _,pair,_ in model(r):truth.extend(pair)
 assert len(r['gradient'])==len(truth)
 for iv,t in zip(r['gradient'],truth):contains(iv,(t,t))
 s['qualified']+=1;s['zeroheap']+=requests(r['allocation'])==0;s['actions']+=r['actions'];s['max_width']=max(s['max_width'],r['width'])
 if r['candidate']=='Structured':assert requests(r['allocation'])==0
for r in goals+tiny:
 assert requests(r['allocation'])==0
 contains(r['work']['signed_error'],goal(r))
assert gcount['Structured']['qualified']==6
summary={'rows':{'spatial':len(arena),'ode':len(ode),'gradient':len(grad),'goal':len(goals),'tiny':len(tiny)},'spatial':counts,'gradient':gcount,'zeroheap_goal_and_tiny':len(goals)+len(tiny),'zeroheap_ode':len(ode),'reference':'Fraction exact monomial time integration and rational true-flow gradient, scaled rational atanh log enclosure; prior spatial deterministic source oracle receipt on identical endpoint schedule, new multi-output and partial against owning driver; ODE sampled analytic exp check, not rigorous interval'}
dump('INDEPENDENT_CHECKS.json',summary)
print('VERIFIED',json.dumps(summary,sort_keys=True))
