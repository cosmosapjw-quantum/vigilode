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
grids={r['case']:r for r in read(P/'portfolio.log','SUBSET_GRID')}
rows=read(P/'portfolio.log','SUBSET_RUN')
timings=read(P/'portfolio.log','SUBSET_TIME')
setups=read(P/'portfolio.log','SUBSET_SETUP')
ode=read(P/'ode.log','SPARSE_ODE')
assert (len(grids),len(rows),len(timings),len(setups),len(ode))==(6,432,60,54,12)
prev=P.parent/'m2_defect_budget_checkpoint_economy_20261128'
prior={r['case']:r for r in read(prev/'portfolio.log','BUDGET_GRID')}
prior_runs={(r['case'],r['goal'],r['input'],r['budget'],r['candidate']):r for r in read(prev/'portfolio.log','MODEL_BUDGET')}
for case,g in grids.items():
 assert g['rows']==prior[case]['rows'] and g['duration']==prior[case]['duration'];checks['source_rows']+=1
@lru_cache(None)
def matrix(case):return [[(j,Q(a))for j,a in row]for row in grids[case]['rows']]
@lru_cache(None)
def covector(case,c,t):return taylor(matrix(case),list(map(Q,c)),t,True)[0]
def project(case,c,u,dt,s=0):
 u=[Q(x)-Q(s)for x in u];c=tuple(c);dt=Q(dt)
 if dt==0:return pt(sum((Q(x)*y for x,y in zip(c,u)),Q(0)))
 if all(x==u[0]for x in u):return pt(u[0]*sum(map(Q,c)))
 return dot_box(covector(case,c,dt),u)
def diff(a,b):return a[0]-b[1],a[1]-b[0]
def key(r):return (r['case'],r['goal'],r['input'],r['budget'],r['candidate'])
ix={key(r):r for r in rows};assert len(ix)==432
stats=collections.defaultdict(lambda:dict(rows=0,cold_pass=0,warm_pass=0,cold_rhs=0,warm_rhs=0,cold_actions=0,warm_actions=0,cold_reused=0,cold_built=0,cold_evictions=0,warm_reused=0,warm_built=0,warm_evictions=0,retained_rows=0,pilot_steps=0,splits=0,merges=0,true_failure=0,conservative=0,unresolved=0,zeroheap=0))
focus=[]
for r in rows:
 g=grids[r['case']];n=g['n'];T=Q(g['duration']);c=r['weights'];stride=r['stride'];s=stats[r['candidate']];s['rows']+=1
 assert r['cold']['pilot_times']==r['warm']['pilot_times']
 for phase in ['cold','warm']:
  v=r[phase];w=v['work'];m=w['cells'];a=Q(v['anchor'])
  assert req(v['allocation'])==0 and w['completed'] and w['accepted']==v['qualified']
  assert w['solves']==w['initial_solves']+w['replayed_cells'] and w['initial_solves']==1
  assert len(v['times'])==m+1 and len(v['states'])==(m+1)*n and v['states'][:n]==r['initial']
  assert v['times']==[v['pilot_times'][i]for i in v['pilot_indices']]
  assert v['pilot_indices'][0]==0 and v['pilot_indices'][-1]==w['pilot_packets']
  assert all(x<y for x,y in zip(v['pilot_indices'],v['pilot_indices'][1:]))
  assert all(i%stride==0 or i==w['pilot_packets'] for i in v['pilot_indices'])
  assert w['pilot_retained']==1+(w['pilot_packets']+stride-1)//stride
  checks['actual_partition_identity']+=1
  ps=[project(r['case'],c,v['states'][i*n:(i+1)*n],T-Q(t),a)for i,t in enumerate(v['times'])]
  for p,q in zip(v['projections'],ps):contains(p,q,(key(r),phase));checks['centered_projections']+=1
  for i,p in enumerate(v['defects']):contains(p,diff(ps[i],ps[i+1]),(key(r),phase));checks['integrated_defects']+=1
  truth=project(r['case'],c,r['initial'],T)
  error=diff(truth,project(r['case'],c,v['states'][m*n:],0))
  iv=w['signed_goal_error'];contains(v['receipt']['exact_goal'],truth);contains(iv,error,(key(r),phase))
  assert iv==v['receipt']['signed_goal_error'];checks['terminal_goal_pairs']+=1
  ok=max(map(abs,iv))<=Q(r['budget']);assert ok==v['qualified']
  if not ok:
   if error[0]>Q(r['budget']) or error[1]<-Q(r['budget']):s['true_failure']+=1
   elif max(map(abs,error))<=Q(r['budget']):s['conservative']+=1
   else:s['unresolved']+=1
  s[phase+'_pass']+=ok;s[phase+'_rhs']+=w['counters']['rhs_calls'];s[phase+'_actions']+=w['adjoint_actions']
  for k,k2 in [('reused','boundary_reused'),('built','boundary_built'),('evictions','boundary_evicted')]:s[phase+'_'+k]+=w[k2]
  s['zeroheap']+=1
  if r['slots']>=16 and phase=='warm':assert w['adjoint_actions']==0 and w['boundary_built']==0 and w['boundary_evicted']==0;checks['full_subset_warm_hits']+=1
  if phase=='cold':
   assert v['times']==r['pilot_times'] and v['pilot_indices']==r['pilot_indices']
   first=next((i for i,x in enumerate(w['selected'][:m])if x),m)
   assert v['states'][:(first+1)*n]==r['pilot_states'][:(first+1)*n];checks['preserved_prefix']+=1
   s['retained_rows']+=w['pilot_retained'];s['pilot_steps']+=w['pilot_packets'];s['splits']+=w['splits'];s['merges']+=w['merges']
   if r['candidate'] in ['OldCostFull','AdaptiveFull']:
    old=prior_runs[(*key(r)[:-1],'OldCost' if r['candidate']=='OldCostFull' else 'AdaptiveModel4')]
    assert old['times']==v['times'] and old['states']==v['states'] and old['work']['counters']==w['counters'] and old['work']['signed_goal_error']==iv
    checks['prior_primal_and_receipt_identity']+=1
   if r['goal']=='local' and r['input']=='offset' and r['budget']==1e-11 and r['case'] in ['sphere128','sphere512','holdout_sphere153']:
    focus.append({'case':r['case'],'candidate':r['candidate'],'accepted':ok,'reference':[float(x)for x in error],'signed':iv,'cold_actions':w['adjoint_actions'],'warm_actions':r['warm']['work']['adjoint_actions'],'rhs':w['counters']['rhs_calls'],'cells':m,'retained':w['pilot_retained']})
for r in ode:
 assert req(r['allocation'])==0 and r['qualified'] and r['retained']==1+(r['packets']+r['stride']-1)//r['stride'] and r['max_error']<=1e-8
 checks['ode_sparse_identity_cases']+=1
edge=read(P/'partial.log','SUBSET_PARTIAL');assert len(edge)==1 and req(edge[0]['allocation'])==0 and edge[0]['work']['initial_solves']==4 and edge[0]['work']['replayed_cells']==1 and edge[0]['work']['primal_stops']==1 and not edge[0]['work']['completed']
mem={}
for r in setups:
 mem.setdefault(r['case'],{})[r['candidate']]={'bytes':r['allocation']['requested_bytes'],'requests':req(r['allocation'])}
ti={(r['case'],r['goal'],r['candidate']):r for r in timings};time={}
for candidate in ['OldCostFull','AdaptiveFull','SubsetFull','SubsetStride2','SubsetStride4']:
 rr={'cold':[],'warm':[]};vs={'cold':[],'warm':[]};passed=0;miss=0
 for case in grids:
  for goal in ['mass','local']:
   r=ti[case,goal,candidate];b=ti[case,goal,'OldCostFull'];a=ti[case,goal,'AdaptiveFull']
   assert r['qualified']==ix[case,goal,'offset',1e-11,candidate]['cold']['qualified']
   passed+=r['qualified'];miss+=r['warm_actions']>0
   for t in rr:
    assert len(r[t+'_ns'])==len(b[t+'_ns'])==5 and min(r[t+'_ns'])>0
    if r['qualified'] and b['qualified']:rr[t].extend(x/y for x,y in zip(b[t+'_ns'],r[t+'_ns']))
    if r['qualified'] and a['qualified']:vs[t].extend(x/y for x,y in zip(a[t+'_ns'],r[t+'_ns']))
 time[candidate]={'passed':passed,'repeat_nonzero_actions':miss,**{t:{'pairs':len(v),'geomean_speedup_vs_oldcost':math.exp(statistics.mean(map(math.log,v))),'geomean_speedup_vs_adaptive':math.exp(statistics.mean(map(math.log,vs[t]))),'min':min(v),'max':max(v)}for t,v in rr.items()}}
summary={'rows':{'replay':len(rows),'ode':len(ode),'timing':len(timings),'setup':len(setups)},'checks':checks,'stats':stats,'memory':mem,'timing':time,'focus':focus,'reference':'Independent 1024-bit fixed-point directed Taylor semigroup and rational tail; cold and warm checked separately; no continuum/discrete adjoint claim; measured CPU0 time descriptive.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
