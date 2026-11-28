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

import statistics,struct
checks=collections.Counter()
grids={r['case']:r for r in read(P/'portfolio.log','LOCAL_GRID')}
cache=read(P/'portfolio.log','CACHE_RUN');timing=read(P/'portfolio.log','CACHE_TIME')
local=read(P/'portfolio.log','LOCAL_REPLAY');global_rows=read(P/'portfolio.log','GLOBAL_LOCAL')
ode=read(P/'ode.log','DENSE_CAPTURE_ODE')
assert (len(grids),len(cache),len(timing),len(local),len(global_rows),len(ode))==(6,108,18,240,48,6)
prior={r['case']:r for r in read(P.parent/'m2_spatial_primal_replay_decay_cost_20261128/replay.log','REPLAY_GRID')}
for k,g in grids.items():
 assert g['rows']==prior[k]['rows'] and g['duration']==prior[k]['duration'];checks['prior_source_rows']+=1
@lru_cache(None)
def covector(case,weights,duration):
 g=grids[case]
 rows=[[(j,Q(a))for j,a in row]for row in g['rows']]
 return taylor(rows,list(map(Q,weights)),duration,True)[0]
def projection(case,c,u,dt):
 u=list(map(Q,u));c=tuple(c);dt=Q(dt)
 if dt==0:return pt(sum((Q(v)*x for v,x in zip(c,u)),Q(0)))
 if all(x==u[0]for x in u):return pt(u[0]*sum(map(Q,c)))
 return dot_box(covector(case,c,dt),u)
def difference(x,y):return x[0]-y[1],x[1]-y[0]
def decision(iv,truth,budget):
 if max(map(abs,iv))<=budget:return 'pass'
 if truth[0]>budget or truth[1]<-budget:return 'true_failure'
 if max(map(abs,truth))<=budget:return 'conservative'
 return 'unresolved'
cache_stats=collections.defaultdict(lambda:dict(rows=0,accepted=0,hits=0,actions=0,projections=0,rhs=0,zeroheap=0))
ci={(r['case'],r['seq'],r['candidate']):r for r in cache};assert len(ci)==108
for r in cache:
 w=r['work'];g=grids[r['case']];n=g['n'];c=r['weights'];key=(r['case'],r['seq'],r['candidate'])
 expected=(r['seq'] in [1,2] and r['candidate']=='ExactBatch') or (r['seq'] in [1,2,3] and r['candidate']=='GoalOnly')
 assert w['cache_hit']==expected and (w['adjoint_actions']==0)==expected
 assert w['goal_projections']==(0 if expected and r['candidate']=='ExactBatch' else 4)
 assert req(r['allocation'])==0 and w['accepted']==r['qualified']
 old=ci[(*key[:-1],'Off')]
 assert r['pairs']==old['pairs'] and w['counters']==old['work']['counters']
 if r['seq']==0:assert r['initial']==prior[r['case']]['initial']
 for i,p in enumerate(r['pairs']):
  u=r['initial'][i*n:(i+1)*n];v=p['endpoint'];truth=projection(r['case'],c,u,g['duration']);num=projection(r['case'],c,v,0)
  contains(p['receipt']['exact_goal'],truth,key);contains(p['receipt']['signed_goal_error'],difference(truth,num),key)
  assert p['accepted']==(max(map(abs,p['receipt']['signed_goal_error']))<=r['budget'])
  checks['cache_goal_pairs']+=1
 s=cache_stats[r['candidate']];s['rows']+=1;s['accepted']+=w['accepted'];s['hits']+=w['cache_hit'];s['actions']+=w['adjoint_actions'];s['projections']+=w['goal_projections'];s['rhs']+=w['counters']['rhs_calls'];s['zeroheap']+=1
# Bitwise one-ULP changes, exact original recovery and timing.
for case,g in grids.items():
 c=[ci[case,i,'Off']for i in range(6)]
 assert c[3]['initial']!=c[0]['initial'] and sum(a!=b for a,b in zip(c[3]['initial'],c[0]['initial']))==1
 assert c[4]['weights']!=c[0]['weights'] and sum(a!=b for a,b in zip(c[4]['weights'],c[0]['weights']))==1
 assert c[5]['initial']==c[0]['initial'] and c[5]['weights']==c[0]['weights']
def ratio_stats():
 ix={(r['case'],r['candidate']):r for r in timing};out={}
 for k in ['Off','ExactBatch','GoalOnly']:
  ratios=[]
  for case in grids:
   a,b=ix[case,k],ix[case,'Off']
   assert len(a['samples_ns'])==len(b['samples_ns'])==6 and min(a['samples_ns'])>0
   ratios.extend(x/y for x,y in zip(b['samples_ns'],a['samples_ns']))
  out[k]={'samples':len(ratios),'geomean_speedup_off_over_candidate':math.exp(statistics.mean(map(math.log,ratios))),'min':min(ratios),'max':max(ratios)}
 return out
li={(r['case'],r['goal'],r['input'],r['budget'],r['candidate']):r for r in local+global_rows};assert len(li)==288
cold={}
for r in local:
 k=r['case'],r['goal'];cold[k]=max(cold.get(k,0),r['work']['adjoint_actions'])
ls=collections.defaultdict(lambda:dict(rows=0,accepted=0,zeroheap=0,rhs=0,actions=0,cold_actions=0,solves=0,replays=0,prefix_reused=0,packets=0,projection_checks=0,true_failure=0,conservative=0,unresolved=0,max_error=0.))
recovery=[]
for key,r in li.items():
 g=grids[r['case']];n=g['n'];w=r['work'];c=r['weights'];T=Q(g['duration'])
 assert req(r['allocation'])==0 and w['accepted']==r['qualified']
 s=ls[r['candidate']];s['rows']+=1;s['accepted']+=w['accepted'];s['zeroheap']+=1;s['rhs']+=w['counters']['rhs_calls'];s['actions']+=w['adjoint_actions'];s['solves']+=w['solves'];s['max_error']=max(s['max_error'],w['max_signed_error_upper'])
 start=projection(r['case'],c,r['initial'],T)
 if r['candidate']=='GlobalGuided':
  num=projection(r['case'],c,r['endpoint'],0);truth=difference(start,num)
  iv=r['signed_goal_error'];s['cold_actions']+=w['adjoint_actions'];s['replays']+=w['replays']
 else:
  assert w['completed'] and len(r['times'])==5 and len(r['states'])==5*n and r['states'][:n]==r['initial']
  assert w['solves']==4+w['replayed_cells'] and w['refined_cells']==sum(w['selected'])
  assert r['packet_count']<=w['dense_packets'] and len(r['defects'])==4
  s['cold_actions']+=cold[r['case'],r['goal']];s['replays']+=w['replay_rounds'];s['prefix_reused']+=w['prefix_cells_reused'];s['packets']+=w['dense_packets']
  ps=[projection(r['case'],c,r['states'][i*n:(i+1)*n],T-Q(t))for i,t in enumerate(r['times'])]
  assert ps[0]==start
  for a,b in zip(r['projections'],ps):contains(a,b,key);checks['local_dual_projections']+=1;s['projection_checks']+=1
  for i,a in enumerate(r['defects']):contains(a,difference(ps[i],ps[i+1]),key);checks['local_integrated_defects']+=1
  truth=difference(ps[0],ps[-1]);iv=w['signed_goal_error']
  sum_lo=sum((Q(v[0])for v in r['defects']),Q(0));sum_hi=sum((Q(v[1])for v in r['defects']),Q(0))
  assert sum_lo<=truth[0]<=truth[1]<=sum_hi
  # Unmodified prefix agrees byte-for-byte with initial segmented run.
  old=li[(*key[:-1],'Segmented')];first=next((i for i,v in enumerate(w['selected'])if v),4)
  assert r['states'][:(first+1)*n]==old['states'][:(first+1)*n];checks['prefix_identity']+=1
  # Independently reconstruct actual binary64 native quartic.
  H=[list(map(Q,row))for row in r['dense_h']]
  for a in r['samples']:
   stages=list(map(Q,a['stage']));d=[sum((x*y for x,y in zip(row,stages)),Q(0))for row in H]
   y0,y1=Q(a['y0']),Q(a['y1']);p=[y0,y1-y0+d[0],d[1]-d[0],d[2]-d[1],-d[2]]
   assert sum(p,Q(0))==y1
   for v,t in zip(a['power'],p):contains(v,pt(t),key);checks['native_power_coordinates']+=1
   checks['native_packet_samples']+=1
 contains(iv,truth,key);checks['local_or_global_terminal_goals']+=1
 status=decision(iv,truth,Q(r['budget']))
 assert r['qualified']==(status=='pass')
 if status!='pass':s[status]+=1
 if r['case'] in ['sphere128','sphere512']and r['goal']=='local'and r['input']=='offset':
  recovery.append({'case':r['case'],'budget':r['budget'],'candidate':r['candidate'],'accepted':r['qualified'],'signed':iv,'reference':[float(v)for v in truth],'rhs':w['counters']['rhs_calls'],'actions':w['adjoint_actions'],'cold_actions':w['adjoint_actions'] if r['candidate']=='GlobalGuided' else cold[r['case'],r['goal']],'prefix':w.get('prefix_cells_reused',0),'selected':w.get('selected',[]),'status':status})
assert all(req(r['allocation'])==0 and r['qualified'] and r['max_error']<=1e-8 for r in ode)
edge=read(P/'edge.log','LOCAL_EDGE');assert len(edge)==1 and edge[0]['case']=='capture_cap' and req(edge[0]['allocation'])==0 and not edge[0]['work']['completed']
summary={'rows':{'cache':len(cache),'local':len(local),'global':len(global_rows),'ode':len(ode),'timing':len(timing)},'checks':checks,'cache':cache_stats,'local':ls,'timing':ratio_stats(),'recovery':recovery,'reference':'Independent 1024-bit directed fixed-point Taylor semigroup with exact rational tail; exact constant forward invariant and exact native quartic coefficient reconstruction. Integrated macro defect by proved endpoint identity, not separate residual quadrature. Timing paired cyclic descriptive only.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
