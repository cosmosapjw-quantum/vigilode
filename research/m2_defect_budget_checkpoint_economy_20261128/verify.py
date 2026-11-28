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
grids={r['case']:r for r in read(P/'portfolio.log','BUDGET_GRID')}
rows=read(P/'portfolio.log','MODEL_BUDGET')
glob=read(P/'portfolio.log','GLOBAL_BUDGET')
times=read(P/'portfolio.log','BUDGET_TIME')
setup=read(P/'portfolio.log','SLIM_SETUP')
allocs=read(P/'allocator.log','SMALL_ALLOC')
assert (len(grids),len(rows),len(glob),len(times),len(setup),len(allocs))==(6,480,48,48,6,96)
prev_dir=P.parent/'m2_adaptive_partition_dual_precision_20261128'
prev_grids={r['case']:r for r in read(prev_dir/'portfolio.log','COMPACT_GRID')}
previous={ (r['case'],r['goal'],r['input'],r['budget'],r['candidate']):r for r in read(prev_dir/'portfolio.log','COMPACT_RUN')}
for case,g in grids.items():assert g['rows']==prev_grids[case]['rows'] and g['duration']==prev_grids[case]['duration'];checks['source_identity']+=1
def model_bound(A,c,n,p,error,work,budget):
 e=sum((Q(a)/(int(k)**p)for a,k in zip(A,n)),Q(0))
 w=sum((Q(x)*int(k)for x,k in zip(c,n)),Q(0))
 assert all(k>=1 for k in n) and e<=Q(error)<=Q(budget) and w<=Q(work)
for r in allocs:
 a=r['allocation'];w=r['work'];assert req(a)==0
 model_bound([b['error_coefficient']for b in r['blocks']],[b['step_cost_upper']for b in r['blocks']],r['counts'],r['order'],w['error_upper'],w['work_upper'],r['budget'])
 checks['allocator_exact_models']+=1
@lru_cache(None)
def row_matrix(case):return [[(j,Q(a))for j,a in row]for row in grids[case]['rows']]
@lru_cache(None)
def covector(case,c,dt):return taylor(row_matrix(case),list(map(Q,c)),dt,True)[0]
def projection(case,c,u,dt,anchor=0):
 u=[Q(x)-Q(anchor)for x in u];c=tuple(c);dt=Q(dt)
 if dt==0:return pt(sum((Q(v)*x for v,x in zip(c,u)),Q(0)))
 if all(x==u[0]for x in u):return pt(u[0]*sum(map(Q,c)))
 return dot_box(covector(case,c,dt),u)
def diff(a,b):return a[0]-b[1],a[1]-b[0]
def key(r):return (r['case'],r['goal'],r['input'],r['budget'],r['candidate'])
ix={key(r):r for r in rows+glob};assert len(ix)==528
stats=collections.defaultdict(lambda:dict(rows=0,accepted=0,zeroheap=0,warm_zeroheap=0,rhs=0,actions=0,warm_actions=0,solves=0,model_calls=0,model_refusals=0,splits=0,merges=0,fallbacks=0,prefix=0,true_failure=0,conservative=0,unresolved=0))
focus=[]
for r in rows+glob:
 w=r['work'];warm=r['warm_work'];g=grids[r['case']];n=g['n'];c=r['weights'];T=Q(g['duration'])
 assert req(r['allocation'])==req(r['warm_allocation'])==0 and w['accepted']==warm['accepted']==r['qualified'] and w['counters']==warm['counters']
 s=stats[r['candidate']];s['rows']+=1;s['accepted']+=r['qualified'];s['zeroheap']+=1;s['warm_zeroheap']+=1;s['rhs']+=w['counters']['rhs_calls'];s['actions']+=w['adjoint_actions'];s['warm_actions']+=warm['adjoint_actions'];s['solves']+=w['solves']
 if r['candidate']=='GlobalGuided':
  end=r['endpoint'];iv=r['receipt']['signed_goal_error'];assert warm['adjoint_actions']==0
 else:
  m=w['cells'];a=Q(r['anchor']);end=r['states'][m*n:(m+1)*n];iv=w['signed_goal_error']
  assert w['completed'] and len(r['times'])==m+1 and r['states'][:n]==r['initial']
  assert w['solves']==w['initial_solves']+w['replayed_cells'] and w['initial_solves']==1
  assert w['signed_goal_error']==warm['signed_goal_error']==r['receipt']['signed_goal_error']
  assert r['times']==[r['pilot_times'][i]for i in r['pilot_indices']]
  assert all(x<y for x,y in zip(r['pilot_indices'],r['pilot_indices'][1:])) and r['pilot_indices'][0]==0 and r['pilot_indices'][-1]==w['pilot_packets']
  checks['accepted_partition_identity']+=1
  if not w['global_fallbacks']:
   pilot=ix[(*key(r)[:-1],'AdaptivePilot' if r['partition']=='DefectAdaptive' else 'Pilot')]
   first=next((i for i,x in enumerate(w['selected'][:m])if x),m)
   assert r['times']==pilot['times'] and r['states'][:(first+1)*n]==pilot['states'][:(first+1)*n]
   checks['preserved_prefix']+=1
  else:
   assert m==1 and w['global_fallbacks']==1
   checks['global_fallback_identity']+=1
  ps=[projection(r['case'],c,r['states'][i*n:(i+1)*n],T-Q(t),a)for i,t in enumerate(r['times'])]
  for p,q in zip(r['projections'],ps):contains(p,q,key(r));checks['centered_projections']+=1
  for i,p in enumerate(r['defects']):contains(p,diff(ps[i],ps[i+1]),key(r));checks['defects']+=1
  contains(iv,diff(ps[0],ps[-1]),key(r))
  if r['candidate']=='OldCost':
   old=previous[(*key(r)[:-1],'Accepted4StationaryCost')]
   assert old['states']==r['states'] and old['times']==r['times'] and old['work']['counters']==w['counters'] and old['work']['signed_goal_error']==iv
   checks['prior_oldcost_identity']+=1
  if w['model_order']:
   k=w['model_cells'];model_bound(w['model_coeff'][:k],w['model_cost'][:k],w['model_counts'][:k],w['model_order'],w['model_error_upper'],w['model_work_upper'],w['model_budget'])
   checks['last_model_receipts']+=1
  for k in ('model_calls','model_refusals','splits','merges'):s[k]+=w[k]
  s['fallbacks']+=w['global_fallbacks'];s['prefix']+=w['prefix_cells_reused']
 truth=projection(r['case'],c,r['initial'],T);num=projection(r['case'],c,end,0);e=diff(truth,num)
 contains(r['receipt']['exact_goal'],truth,key(r));contains(iv,e,key(r));checks['terminal_signed_and_absolute']+=1
 b=Q(r['budget']);ok=max(map(abs,iv))<=b;assert ok==r['qualified']
 if not ok:
  if e[0]>b or e[1]<-b:s['true_failure']+=1
  elif max(map(abs,e))<=b:s['conservative']+=1
  else:s['unresolved']+=1
 if r['budget']==1e-11 and r['input']=='offset' and (r['goal']=='local' and r['case'] in ['sphere128','sphere512','holdout_sphere153'] or r['goal']=='mass' and r['case']=='sphere512'):
  focus.append({'case':r['case'],'goal':r['goal'],'candidate':r['candidate'],'accepted':ok,'reference':[float(v)for v in e],'signed':iv,'rhs':w['counters']['rhs_calls'],'actions':w['adjoint_actions'],'warm_actions':warm['adjoint_actions']})
timing={}
ti={(r['case'],r['goal'],r['candidate']):r for r in times}
for candidate in ['GlobalGuided','OldCost','Model2','AdaptiveModel4']:
 ratios={'cold':[],'warm':[]};qualified=0;warm_nonzero=0
 for case in grids:
  for goal in ['mass','local']:
   r=ti[case,goal,candidate];base=ti[case,goal,'OldCost']
   assert r['qualified']==ix[case,goal,'offset',1e-11,candidate]['qualified']
   qualified+=r['qualified'];warm_nonzero+=r['warm_actions']>0
   for typ in ratios:
    assert len(r[typ+'_ns'])==len(base[typ+'_ns'])==4 and min(r[typ+'_ns'])>0
    # Only compare two admitted results; failed runs are separately counted.
    if r['qualified'] and base['qualified']:
     ratios[typ].extend(a/b for a,b in zip(base[typ+'_ns'],r[typ+'_ns']))
 timing[candidate]={'qualified_strata':qualified,'warm_nonzero_actions_strata':warm_nonzero,**{k:{'pairs':len(v),'geomean_speedup_vs_oldcost':math.exp(statistics.mean(map(math.log,v))),'min':min(v),'max':max(v)}for k,v in ratios.items()}}
old_setup={v['case']:v for v in json.loads((prev_dir/'INDEPENDENT_CHECKS.json').read_text())['memory']}
memory=[{'case':r['case'],'new_bytes':r['allocation']['requested_bytes'],'previous_bytes':old_setup[r['case']]['compact_bytes'],'new_requests':req(r['allocation']),'previous_requests':old_setup[r['case']]['compact_requests']}for r in setup]
edge=read(P/'fallback.log','FALLBACK_EDGE');assert len(edge)==1 and req(edge[0]['allocation'])==0 and edge[0]['work']['global_fallbacks']==edge[0]['work']['replayed_cells']==edge[0]['work']['primal_stops']==1 and not edge[0]['work']['completed']
summary={'rows':{'replay':len(rows)+len(glob),'allocator':len(allocs),'timing':len(times)},'checks':checks,'stats':stats,'timing':timing,'memory':memory,'focus':focus,'reference':'Exact rational model allocation and independent 1024-bit direct Taylor semigroup with geometric tail; finite represented source, not continuous spatial error. Timing descriptive, paired CPU0 admitted strata.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
