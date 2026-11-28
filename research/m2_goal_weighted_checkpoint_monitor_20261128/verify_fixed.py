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
grids={r['case']:r for r in read(P/'portfolio.log','MONITOR_GRID')}
rows=read(P/'portfolio.log','MONITOR_RUN')
timings=read(P/'portfolio.log','MONITOR_TIME')
setups=read(P/'portfolio.log','MONITOR_SETUP')
ode=read(P/'ode.log','MONITOR_ODE')
responses=read(P/'portfolio.log','RESPONSE_RUN')
assert (len(grids),len(rows),len(timings),len(setups),len(ode),len(responses))==(5,306,40,45,30,12)
prev=P.parent/'m2_cost_aware_density_boundary_pool_20261128'
prior={r['case']:r for r in read(prev/'portfolio.log','DENSITY_GRID')}
prior_runs={(r['case'],r['goal'],r['input'],r['budget'],r['candidate']):r for r in read(prev/'portfolio.log','DENSITY_RUN')}
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
ix={key(r):r for r in rows};assert len(ix)==306
stats=collections.defaultdict(lambda:dict(rows=0,cold_pass=0,warm_pass=0,cold_rhs=0,warm_rhs=0,cold_actions=0,warm_actions=0,monitor_actions=0,retained_rows=0,pilot_steps=0,splits=0,merges=0,true_failure=0,conservative=0,unresolved=0,zeroheap=0,proxy_rank_comparisons=0,proxy_rank_match=0))
focus=[]
for r in rows:
 g=grids[r['case']];n=g['n'];T=Q(g['duration']);c=r['weights'];density=r['density'];s=stats[r['candidate']];s['rows']+=1
 assert r['cold']['pilot_times']==r['warm']['pilot_times']
 assert r['cold']['scores']==r['warm']['scores']
 for phase in ['cold','warm']:
  v=r[phase];w=v['work'];m=w['cells'];a=Q(v['anchor'])
  assert req(v['allocation'])==0 and w['completed'] and w['accepted']==v['qualified']
  assert w['solves']==w['initial_solves']+w['replayed_cells'] and w['initial_solves']==1
  assert len(v['times'])==m+1 and len(v['states'])==(m+1)*n and v['states'][:n]==r['initial']
  assert v['times']==[v['pilot_times'][i]for i in v['pilot_indices']]
  assert v['pilot_indices'][0]==0 and v['pilot_indices'][-1]==w['pilot_packets']
  assert all(x<y for x,y in zip(v['pilot_indices'],v['pilot_indices'][1:]))
  assert w['monitor_adjoint_actions']<=w['adjoint_actions']
  if phase=='warm':assert w['adjoint_actions']==0 and w['monitor_adjoint_actions']==0
  if 'Stride' in density:
   stride=density['Stride'];assert all(i%stride==0 or i==w['pilot_packets'] for i in v['pilot_indices'])
   assert w['pilot_retained']==1+(w['pilot_packets']+stride-1)//stride
  elif 'OnlineGoal' in density:
   cap,mode=density['OnlineGoal'];assert w['pilot_retained']==min(cap,w['pilot_packets']+1)
   assert len(v['scores'])==w['pilot_packets'] and all(math.isfinite(x) and x>=0 for x in v['scores'])
   if mode.startswith('Terminal'):assert w['monitor_adjoint_actions']==0
   checks['monitor_score_rows']+=1
  else:
   cap=next(iter(density.values()));assert w['pilot_retained']==min(cap,w['pilot_packets']+1)
   assert not v['scores'] and w['monitor_adjoint_actions']==0
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
  s[phase+'_pass']+=ok;s[phase+'_rhs']+=w['counters']['rhs_calls'];s[phase+'_actions']+=w['adjoint_actions'];s['zeroheap']+=1
  if phase=='cold':
   s['monitor_actions']+=w['monitor_adjoint_actions']
   assert v['times']==r['pilot_times'] and v['pilot_indices']==r['pilot_indices']
   first=next((i for i,x in enumerate(w['selected'][:m])if x),m)
   assert v['states'][:(first+1)*n]==r['pilot_states'][:(first+1)*n];checks['preserved_prefix']+=1
   s['retained_rows']+=w['pilot_retained'];s['pilot_steps']+=w['pilot_packets'];s['splits']+=w['splits'];s['merges']+=w['merges']
   mapping={'OldCostStride4':'OldCostStride4','SubsetStride4':'SubsetStride4','Time8':'OnlineTime8','Motion8':'OnlineMotion8'}
   if r['candidate'] in mapping and r['goal']!='multimode':
    old=prior_runs[(*key(r)[:-1],mapping[r['candidate']])]['cold']
    assert old['times']==v['times'] and old['states']==v['states'] and old['work']['counters']==w['counters'] and old['work']['signed_goal_error']==iv
    checks['prior_primal_and_receipt_identity']+=1
   # Compare actual pilot macro defects to accepted proxy mass; not an
   # error bound. Only count unique maxima on genuinely refused pilots.
   if v['scores'] and not r['pilot_work']['accepted'] and m>=2:
    p0=[project(r['case'],c,r['pilot_states'][i*n:(i+1)*n],T-Q(t),a)for i,t in enumerate(r['pilot_times'])]
    actual=[max(map(abs,diff(p0[i],p0[i+1])))for i in range(m)]
    proxy=[sum(v['scores'][v['pilot_indices'][i]:v['pilot_indices'][i+1]])for i in range(m)]
    if max(proxy)>0 and proxy.count(max(proxy))==1 and actual.count(max(actual))==1:
     s['proxy_rank_comparisons']+=1;s['proxy_rank_match']+=proxy.index(max(proxy))==actual.index(max(actual))
   if r['goal']=='local' and r['input']=='offset' and r['budget']==1e-11 and r['case'] in ['sphere128','sphere512','holdout_sphere800']:
    focus.append({'case':r['case'],'candidate':r['candidate'],'accepted':ok,'reference':[float(x)for x in error],'signed':iv,'cold_actions':w['adjoint_actions'],'monitor_actions':w['monitor_adjoint_actions'],'rhs':w['counters']['rhs_calls'],'cells':m,'retained':w['pilot_retained']})
# Independent rational projection of actual native dense transformed stage.
for r in ode:
 assert req(r['allocation'])==0 and r['qualified'] and r['retained']==min(r['row_cap'],r['packets']+1) and r['max_error']<=1e-8
 assert r['indices'][0]==0 and r['indices'][-1]==r['packets'] and len(r['indices'])==r['retained']
 assert len(r['scores'])==len(r['errors'])==r['packets']
 for k,(e,score) in enumerate(zip(r['errors'],r['scores'])):
  x=(Q(r['times'][k+1])-Q(r['times'][0]))/(Q(r['times'][-1])-Q(r['times'][0]))
  if r['mode'].startswith('Terminal'):weights=list(map(Q,r['right']))
  else:
   a,b,z=(r['left'],r['mid'],2*x)if x<=Q(1,2)else(r['mid'],r['right'],2*x-1)
   weights=[(1-z)*Q(v)+z*Q(w)for v,w in zip(a,b)]
  terms=[w*Q(v)/len(e)for w,v in zip(weights,e)]
  ref=sum(map(abs,terms))if r['mode'].endswith('Abs')else abs(sum(terms))
  assert abs(Q(score)-ref)<=Q(128,2**52)*sum(map(abs,terms)),(r['mode'],k,float(ref),score)
  checks['embedded_stage_score']+=1
 checks['ode_goal_identity_cases']+=1
partial=read(P/'ode.log','MONITOR_PARTIAL');assert len(partial)==1 and req(partial[0]['allocation'])==0 and partial[0]['packets']==3
mem={}
for r in setups:mem.setdefault(r['case'],{})[r['candidate']]={'bytes':r['allocation']['requested_bytes'],'requests':req(r['allocation'])}
ti={(r['case'],r['goal'],r['candidate']):r for r in timings};time={}
for candidate in ['OldCostStride4','Time8','TerminalSigned8','AdjointSigned8']:
 rr={'cold':[],'warm':[]};passed=0
 for case in grids:
  for goal in ['mass','local']:
   r=ti[case,goal,candidate];b=ti[case,goal,'Time8']
   assert r['qualified']==ix[case,goal,'offset',1e-11,candidate]['cold']['qualified']
   passed+=r['qualified']
   for t in rr:
    assert len(r[t+'_ns'])==len(b[t+'_ns'])==4 and min(r[t+'_ns'])>0
    if r['qualified'] and b['qualified']:rr[t].extend(x/y for x,y in zip(b[t+'_ns'],r[t+'_ns']))
 time[candidate]={'passed':passed,**{t:{'pairs':len(v),'geomean_speedup_vs_time8':math.exp(statistics.mean(map(math.log,v))) if v else None,'min':min(v) if v else None,'max':max(v) if v else None}for t,v in rr.items()}}
resp={}
for r in responses:
 w=r['work'];assert req(r['allocation'])==0 and w['completed'] and w['solves']==1
 T=Q(grids[r['case']]['duration']);c=r['weights']
 exact=project(r['case'],c,r['initial'],T);error=diff(exact,project(r['case'],c,r['endpoint'],0))
 contains(r['receipt']['exact_goal'],exact);contains(w['signed_goal_error'],error)
 resp.setdefault(r['case'],[]).append({'scale':r['scale'],'error':[float(v)for v in error],'error_mag':float(max(map(abs,error))),'bound':w['max_signed_error_upper'],'rhs':w['counters']['rhs_calls'],'accepted':w['accepted']})
 checks['tolerance_response_goal_pairs']+=1
for a in resp.values():
 a.sort(key=lambda r:-r['scale'])
 for k,r in enumerate(a):
  if k and a[k-1]['error_mag']>0 and r['error_mag']>0:r['observed_log_slope']=math.log(r['error_mag']/a[k-1]['error_mag'])/math.log(r['scale']/a[k-1]['scale'])
summary={'rows':{'replay':len(rows),'ode':len(ode),'timing':len(timings),'setup':len(setups),'response':len(responses)},'checks':checks,'stats':stats,'memory':mem,'timing':time,'focus':focus,'response':resp,'reference':'Independent 1024-bit fixed-point Taylor semigroup, rational tail, independent rational embedded stage projection. Proxy rank compares certified actual pilot defects, not physical certificate. Finite represented source only.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
