from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,math
P=Path(__file__).parent
def read(path,marker):
 if not path.exists():path=Path(str(path)+'.gz')
 with (gzip.open(path,'rt') if path.suffix=='.gz' else path.open()) as f:return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,sort_keys=True,indent=2,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def contains(iv,truth,label=None):
 lo,hi=map(Q,iv);assert lo<=truth[0]<=truth[1]<=hi,(label,iv,list(map(float,truth)))
def pt(x):return (x,x)
def dot_box(box,v):
 lo=hi=Q(0)
 for (a,b),c in zip(box,v):lo+=min(a*c,b*c);hi+=max(a*c,b*c)
 return lo,hi
# Independent direct Taylor on a 1024-bit fixed-point integer grid, not the
# production Poisson uniformization or source mode residual.
S=1<<1024
def floor(x):return x.numerator//x.denominator
def ceil(x):return -floor(-x)
def fixed(x):return floor(Q(x)*S),ceil(Q(x)*S)
def scale(a,num,den):
 lo,hi=a
 if num<0:lo,hi=-hi,-lo;num=-num
 return lo*num//den,-((-hi*num)//den)
def mv(edges,v,trans):
 n=len(v);lo=[0]*n;hi=[0]*n
 if trans:
  for i,j,a,b in edges:
   l,h=scale(v[i],a,b);lo[i]-=h;hi[i]-=l;lo[j]+=l;hi[j]+=h
 else:
  for i,j,a,b in edges:
   l,h=scale((v[j][0]-v[i][1],v[j][1]-v[i][0]),a,b);lo[i]+=l;hi[i]+=h
 return list(zip(lo,hi))
def taylor(rows,initial,T,trans):
 n=len(rows);v=[fixed(x)for x in initial]
 if T==0:return [pt(x)for x in initial],0
 edges=[(i,j,a.numerator,a.denominator)for i,row in enumerate(rows)for j,a in row if j!=i and a]
 exact=[Q(0)]*n
 for i,row in enumerate(rows):
  for j,a in row:
   if i==j:continue
   if trans:exact[i]-=a*initial[i];exact[j]+=a*initial[i]
   else:exact[i]+=a*(initial[j]-initial[i])
 if all(x==0 for x in exact):return [pt(x)for x in initial],0
 M=2*T*max(sum((a for j,a in row if j!=i),Q(0))for i,row in enumerate(rows));weight=Q(1);order=0
 while True:
  if order+2>M:
   tail=weight*M/(order+1)/(1-M/(order+2))
   if tail<=Q(1,1<<180):break
  order+=1;assert order<=2048;weight*=M/order
 norm=sum(max(abs(l),abs(h))for l,h in v)if trans else max(max(abs(l),abs(h))for l,h in v)
 radius=ceil(norm*tail);term=v.copy();res=v.copy()
 for k in range(1,order+1):
  h=T/k;term=[scale(x,h.numerator,h.denominator)for x in mv(edges,term,trans)]
  res=[(a+c,b+d)for (a,b),(c,d)in zip(res,term)]
 box=[(Q(a-radius,S),Q(b+radius,S))for a,b in res]
 assert max(b-a for a,b in box)<=Q(1,10**25)
 return box,order

import statistics
checks=collections.Counter()
grids={r['case']:r for r in read(P/'portfolio.log','COMPACT_GRID')}
runs=read(P/'portfolio.log','COMPACT_RUN');setup=read(P/'portfolio.log','COMPACT_SETUP')
ode=read(P/'ode.log','CHECKPOINT_ODE')
assert (len(grids),len(runs),len(setup),len(ode))==(6,624,6,6)
previous={r['case']:r for r in read(P.parent/'m2_local_adjoint_defect_cache_20261128/portfolio.log','LOCAL_GRID')}
prev_runs=read(P.parent/'m2_local_adjoint_defect_cache_20261128/portfolio.log','LOCAL_REPLAY')
prev_global=read(P.parent/'m2_local_adjoint_defect_cache_20261128/portfolio.log','GLOBAL_LOCAL')
prev_ix={(r['case'],r['goal'],r['input'],r['budget'],r['candidate']):r for r in prev_runs+prev_global}
for k,g in grids.items():assert g['rows']==previous[k]['rows'] and g['duration']==previous[k]['duration'];checks['prior_source_rows']+=1
@lru_cache(None)
def row_matrix(case):
 return [[(j,Q(a))for j,a in row]for row in grids[case]['rows']]
@lru_cache(None)
def covector(case,weights,duration):
 return taylor(row_matrix(case),list(map(Q,weights)),duration,True)[0]
def projection(case,c,u,dt,anchor=0):
 u=[Q(x)-Q(anchor)for x in u];c=tuple(c);dt=Q(dt)
 if dt==0:return pt(sum((Q(v)*x for v,x in zip(c,u)),Q(0)))
 if all(x==u[0]for x in u):return pt(u[0]*sum(map(Q,c)))
 return dot_box(covector(case,c,dt),u)
def diff(a,b):return (a[0]-b[1],a[1]-b[0])
def status(iv,t,b):
 if max(map(abs,iv))<=b:return 'pass'
 if t[0]>b or t[1]<-b:return 'true_failure'
 if max(map(abs,t))<=b:return 'conservative'
 return 'unresolved'
def key(r):return (r['case'],r['goal'],r['input'],r['budget'],r['candidate'])
ix={key(r):r for r in runs};assert len(ix)==624
def ckey(r):return (r['case'],r['dual'],tuple(r['weights']),tuple(r['times']))
cold={}
for r in runs:cold[ckey(r)]=max(cold.get(ckey(r),0),r['work']['adjoint_actions'])
stats=collections.defaultdict(lambda:dict(rows=0,accepted=0,zeroheap=0,warm_zeroheap=0,rhs=0,stage_solves=0,solves=0,replays=0,prefix_reused=0,actions=0,cold_actions=0,stationary=0,projections=0,true_failure=0,conservative=0,unresolved=0))
recover=[]
for r in runs:
 w=r['work'];warm=r['warm_work'];n=grids[r['case']]['n'];T=Q(grids[r['case']]['duration']);m=w['cells'];a=Q(r['anchor'])
 assert req(r['allocation'])==req(r['warm_allocation'])==0
 assert w['completed'] and warm['completed'] and w['accepted']==warm['accepted']==r['qualified']
 assert warm['dual_cache_hit'] and warm['adjoint_actions']==0 and w['counters']==warm['counters'] and w['signed_goal_error']==warm['signed_goal_error']
 assert len(r['times'])==m+1 and len(r['states'])==(m+1)*n and r['states'][:n]==r['initial']
 assert w['solves']==w['initial_solves']+w['replayed_cells'] and w['refined_cells']==sum(w['selected'])
 assert r['times'][0]==0 and Q(r['times'][-1])==T and all(a<b for a,b in zip(r['times'],r['times'][1:]))
 if r['partition']=='Fixed4':
  assert m==4 and w['initial_solves']==4 and not r['pilot_times']
 else:
  assert w['initial_solves']==1 and len(r['pilot_times'])==w['pilot_packets']+1
  assert [r['pilot_times'][i] for i in r['pilot_indices']]==r['times']
  assert all(a<b for a,b in zip(r['pilot_indices'],r['pilot_indices'][1:]))
  assert r['pilot_indices'][0]==0 and r['pilot_indices'][-1]==w['pilot_packets']
  checks['accepted_partition_identity']+=1
 pilot=ix[(*key(r)[:-1],r['partition']+'Pilot')]
 first=next((i for i,x in enumerate(w['selected'][:m])if x),m)
 assert r['times']==pilot['times'] and r['states'][:(first+1)*n]==pilot['states'][:(first+1)*n]
 checks['prefix_identity']+=1
 old=prev_ix[(*key(r)[:-1],'Segmented')]
 assert old['initial']==r['initial'] and old['weights']==r['weights']
 if r['candidate']=='Fixed4Pilot':
  assert r['states']==old['states'];checks['old_fixed_primal_identity']+=1
 ps=[projection(r['case'],r['weights'],r['states'][i*n:(i+1)*n],T-Q(t),a)for i,t in enumerate(r['times'])]
 for p,q in zip(r['projections'],ps):contains(p,q,key(r));checks['centered_projection']+=1
 for i,p in enumerate(r['defects']):contains(p,diff(ps[i],ps[i+1]),key(r));checks['integrated_defect']+=1
 truth=diff(ps[0],ps[-1]);iv=w['signed_goal_error'];contains(iv,truth,key(r));checks['terminal_signed_goal']+=1
 exact=projection(r['case'],r['weights'],r['initial'],T)
 contains(r['receipt']['exact_goal'],exact,key(r));checks['absolute_goal_receipt']+=1
 assert r['receipt']['signed_goal_error']==iv
 st=status(iv,truth,Q(r['budget']));assert r['qualified']==(st=='pass')
 s=stats[r['candidate']];s['rows']+=1;s['accepted']+=w['accepted'];s['zeroheap']+=1;s['warm_zeroheap']+=1;s['rhs']+=w['counters']['rhs_calls']
 for k in ['stage_solves','solves','projections','adjoint_actions']:s['actions' if k=='adjoint_actions' else k]+=w[k]
 s['replays']+=w['replay_rounds'];s['prefix_reused']+=w['prefix_cells_reused'];s['cold_actions']+=cold[ckey(r)];s['stationary']+=w['stationary_used']
 if st!='pass':s[st]+=1
 if w['stationary_used']:
  c=list(map(Q,r['weights']));res=[Q(0)]*n
  for i,row in enumerate(row_matrix(r['case'])):
   for j,z in row:
    if j!=i:res[i]-=z*c[i];res[j]+=z*c[i]
  assert sum(map(abs,res))<=Q(w['stationary_residual'])
  assert r['receipt']['l1_tail_upper']>=0 and w['adjoint_actions'] in (0,1)
  checks['stationary_residual_bound']+=1
 if r['budget']==1e-11 and r['input']=='offset' and (r['goal']=='local' and r['case'] in ['sphere128','sphere512','holdout_sphere153'] or r['goal']=='mass' and r['case']=='sphere512'):
  recover.append({'case':r['case'],'goal':r['goal'],'candidate':r['candidate'],'status':st,'signed':iv,'reference':[float(v)for v in truth],'rhs':w['counters']['rhs_calls'],'cold_actions':cold[ckey(r)],'scales':r['scales'],'selected':w['selected'],'prefix':w['prefix_cells_reused'],'stationary':w['stationary_used']})
assert len(recover)==52
assert all(req(r['allocation'])==0 and r['qualified'] and r['max_error']<=1e-8 for r in ode)
memory=[]
for r in setup:
 a,b=r['allocation'],r['legacy_allocation']
 memory.append({'case':r['case'],'compact_bytes':a['requested_bytes'],'old_bytes':b['requested_bytes'],'ratio':a['requested_bytes']/b['requested_bytes'],'compact_requests':req(a),'old_requests':req(b)})
edge=read(P/'partial.log','PARTIAL_SUFFIX');assert len(edge)==1
assert edge[0]['work']['solves']==5 and edge[0]['work']['replayed_cells']==1 and req(edge[0]['allocation'])==0 and not edge[0]['work']['completed']
summary={'rows':{'compact':len(runs),'ode':len(ode)},'checks':checks,'compact':stats,'memory':memory,'recovery':recover,'reference':'Independent 1024-bit fixed-point directed direct Taylor semigroup, exact rational geometric tail, common-anchor projection and exact represented stationary residual, accepted-step partition and prefix identity. No continuum claim.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
