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
def log_unit(d):
 s=(d-1)/(d+1);assert 0<=s<=Q(1,3)
 a=Q(0);power=s
 for j in range(40):a+=2*power/(2*j+1);power*=s*s
 return a,a+2*power/(81*(1-s*s))
@lru_cache(None)
def log_bounds(d):
 x=d-1;assert x>=0
 if x<=Q(1,2**40):return x-x*x/2,x
 k=0
 while d>2:d/=2;k+=1
 a,b=log_unit(d);c,e=log_unit(Q(2));return a+k*c,b+k*e
def state(r,t):
 x=list(map(Q,r['initial']));t,s=Q(t),Q(r['start']);coupled=r['case']=='coupled_clock'
 moments=[(t**(k+1)-s**(k+1))/(k+1)for k in range(3)]
 result=[]
 for j,(aa,bb) in enumerate(zip(r['a'],r['b'])):
  a,b=list(map(Q,aa)),list(map(Q,bb));i=2*j
  pivot=next(k for k in range(3)if a[k]>0);ratio=b[pivot]/a[pivot];assert all(y==ratio*z for y,z in zip(b,a))
  m=sum((a[k]*moments[k]for k in range(3)),Q(0));p=x[i]-x[i+1] if coupled else x[i];q=x[i+1];d=1+p*m
  pe=p/d;l,h=log_bounds(d);qe=sorted((q+ratio*l,q+ratio*h));ue=(pe+qe[0],pe+qe[1]) if coupled else (pe,pe)
  result.extend([ue,tuple(qe)])
 return result+result
def poly_eval(row,theta):
 lo=hi=Q(0)
 for a,b in reversed(row):lo=lo*theta+Q(a);hi=hi*theta+Q(b)
 return lo,hi
def err_box(a,b):return max(abs(a[0]-b[1]),abs(a[1]-b[0]))
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
models={r['case']:r for r in read(P/'decay.log','DECAY_COST_MODEL')}
cost=read(P/'decay.log','DECAY_COST')
slabs=read(P/'decay.log','DECAY_COST_SLAB')
dense=read(P/'decay.log','DENSE_PARAMETER')
assert len(models)==10 and len(cost)==60 and len(dense)==120
prev=read(P.parent/'m2_split_decay_verified_dense_20261128/clock.log','DECAY_RUN')
old={(r['case'],r['candidate']):r for r in prev}
ix={(r['case'],r['candidate']):r for r in cost};assert len(ix)==60
ts=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,slabs=0,ratios=[],bounds=[],median_ns=[]))
for r in cost:
 assert r['qualified'] and req(r['allocation'])==0 and len(r['samples_ns'])==6
 m=models[r['case']];assert m['rhs']==r['rhs'];assert len(r['terminal'])==2*len(m['initial'])
 ref=state(m,Q(m['end']))
 assert max(err_box(pt(Q(v)),x)for v,x in zip(r['terminal'],ref))<=Q(r['terminal_bound'])
 assert r['terminal']==ix[r['case'],'BlockSplit']['terminal']
 if (r['case'],r['candidate']) in old:
  o=old[r['case'],r['candidate']]
  assert r['terminal']==o['terminal'] and r['terminal_bound']==o['terminal_bound'] and r['whole_bound']==o['whole_bound']
  checks['prior_decay_receipts']+=1
 v=ts[r['candidate']];v['rows']+=1;v['zeroheap']+=1;v['slabs']+=r['slabs'];v['median_ns'].append(r['median_ns'])
 base=ix[r['case'],'BlockSplit']
 v['ratios'] += [a/b for a,b in zip(r['samples_ns'],base['samples_ns'])]
 if base['terminal_bound']>0:v['bounds'].append(r['terminal_bound']/base['terminal_bound'])
assert len(slabs)==sum(r['slabs']for r in cost)
@lru_cache(None)
def sample(case,t):return state(models[case],t)
worst=endpoint_worst=0.
for r in slabs:
 n=len(r['power'])//2;t0,t1=map(Q,r['times'])
 assert r['outgoing']<=r['x_error']
 for theta in [Q(0),Q(1,4),Q(1,2),Q(3,4),Q(1)]:
  exact=sample(r['case'],t0+(t1-t0)*theta)
  for i,row in enumerate(r['power']):
   bound=Q(r['x_error'] if i<n else r['z_error']);error=err_box(poly_eval(row,theta),exact[i])
   assert error<=bound,(r['case'],r['candidate'],r['index'],i,theta,float(error),float(bound))
   if bound:worst=max(worst,float(error/bound))
   checks['slab_coordinate_samples']+=1
   if theta==1 and i<n:
    endpoint=Q(r['outgoing']);assert error<=endpoint
    if endpoint:endpoint_worst=max(endpoint_worst,float(error/endpoint))
    checks['endpoint_carry_coordinates']+=1
def params(m,t):
 x=list(map(Q,m['initial']));n=len(x);w=list(map(Q,m['weights']));s=Q(m['start'])
 moments=[(t**(k+1)-s**(k+1))/(k+1)for k in range(3)]
 coupled=m['case']=='coupled_clock';out=[]
 for j,(aa,bb) in enumerate(zip(m['a'],m['b'])):
  a,b=list(map(Q,aa)),list(map(Q,bb));i=2*j
  pivot=next(k for k in range(3)if a[k]>0);ratio=b[pivot]/a[pivot]
  assert all(y==ratio*z for y,z in zip(b,a))
  mass=sum((v*d for v,d in zip(a,moments)),Q(0))
  p=x[i]-x[i+1] if coupled else x[i];d=1+p*mass;pe=p/d
  wp=w[i]+w[n+i];wq=w[i+1]+w[n+i+1]+(wp if coupled else 0)
  slope=-wp*pe*pe+wq*ratio*pe
  lo,hi=log_bounds(d)
  out.extend([pt(v*slope)for v in moments]+[tuple(sorted((wq*lo,wq*hi)))])
 return out
ds=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,max_width=0.))
for r in dense:
 assert req(r['allocation'])==0 and r['qualified']
 m=models[r['case']];t=Q(r['time'])
 exact=sample(r['case'],t)
 for v,rad,truth in zip(r['center'],r['radius'],exact):
  assert err_box(pt(Q(v)),truth)<=Q(rad);checks['dense_state_coordinates']+=1
 truth=params(m,t);assert len(truth)==len(r['gradient'])
 for iv,v in zip(r['gradient'],truth):contains(iv,v,(r['case'],r['request'],r['candidate']));checks['dense_parameter_coordinates']+=1
 s=ds[r['candidate']];s['rows']+=1;s['zeroheap']+=1;s['max_width']=max(s['max_width'],r['work']['max_width'])
# Reproduce old represented inputs and actual numerical baseline.
grids={r['case']:r for r in read(P/'replay.log','REPLAY_GRID')}
replay=read(P/'replay.log','PRIMAL_REPLAY');setup=read(P/'replay.log','REPLAY_SETUP')
assert len(grids)==len(setup)==6 and len(replay)==144
prior={r['case']:r for r in read(P.parent/'m2_split_decay_verified_dense_20261128/spatial.log','CENTER_GRID')}
for case,g in grids.items():
 if case in prior:
  p=prior[case]
  assert g['rows']==p['rows'] and g['initial']==p['initial'] and g['old_endpoint']==p['endpoint']
  checks['prior_spatial_inputs_and_endpoints']+=1
references={}
def ref_for(r):
 key=r['case'],r['goal']
 if key not in references:
  g=grids[r['case']];c=list(map(Q,r['weights']))
  rows=[[(j,Q(a))for j,a in row]for row in g['rows']]
  ref,_=taylor(rows,c,Q(g['duration']),True)
  n=g['n'];goals=[]
  for i in range(4):
   u=list(map(Q,g['initial'][i*n:(i+1)*n]))
   if all(x==u[0]for x in u):goals.append(pt(u[0]*sum(c,Q(0))))
   else:goals.append(dot_box(ref,u))
  references[key]=(c,goals)
 return references[key]
rs=collections.defaultdict(lambda:dict(rows=0,accepted=0,pairs=0,accepted_pairs=0,zeroheap=0,solves=0,replays=0,centered=0,rhs=0,stage_solves=0,adjoint_actions=0,true_failure_pairs=0,conservative_refusal_pairs=0))
ri={(r['case'],r['goal'],r['budget'],r['candidate']):r for r in replay};assert len(ri)==144
recovery=[]
for r in replay:
 assert req(r['allocation'])==0
 g=grids[r['case']];n=g['n'];w=r['work'];c,truth=ref_for(r);budget=Q(r['budget'])
 assert len(r['pairs'])==4 and w['solves']==4+w['replays']
 assert w['adjoint_actions']==ri[r['case'],r['goal'],r['budget'],'Baseline']['work']['adjoint_actions']
 assert w['accepted']==r['qualified']==all(p['accepted']for p in r['pairs'])
 assert w['accepted_pairs']==sum(p['accepted']for p in r['pairs'])
 assert w['replays']==sum(p['retries']for p in r['pairs'])
 assert w['min_scale_used']>=1e-4
 s=rs[r['candidate']];s['rows']+=1;s['accepted']+=w['accepted'];s['pairs']+=4;s['accepted_pairs']+=w['accepted_pairs'];s['zeroheap']+=1
 for k in ['solves','replays','adjoint_actions','stage_solves']:s[k]+=w[k]
 s['centered']+=w['centered_solves'];s['rhs']+=w['counters']['rhs_calls']
 for i,(p,(l,h)) in enumerate(zip(r['pairs'],truth)):
  rec=p['receipt'];y=list(map(Q,p['endpoint']))
  assert len(y)==n
  num=sum((a*b for a,b in zip(c,y)),Q(0));e=(l-num,h-num)
  contains(rec['exact_goal'],(l,h));contains(rec['signed_goal_error'],e)
  assert p['accepted']==(max(map(abs,rec['signed_goal_error']))<=r['budget'])
  if p['accepted']:assert max(map(abs,e))<=budget
  elif e[0]>budget or e[1]<-budget:s['true_failure_pairs']+=1
  elif max(map(abs,e))<=budget:s['conservative_refusal_pairs']+=1
  if r['candidate']=='Baseline':assert p['endpoint']==g['old_endpoint'][i*n:(i+1)*n]
  if r['case'] in ['sphere128','sphere512']and r['goal']=='local'and r['budget']==1e-10 and i==3:
   recovery.append({'case':r['case'],'candidate':r['candidate'],'accepted':p['accepted'],'error':rec['signed_goal_error'],'reference':[float(e[0]),float(e[1])],'scale':p['scale'],'centered':p['centered'],'retries':p['retries']})
  checks['spatial_goal_pairs']+=1
assert len(recovery)==12
for case in ['sphere128','sphere512']:
 b=next(r for r in recovery if r['case']==case and r['candidate']=='Baseline')
 assert not b['accepted'] and b['reference'][0]>1e-10
 assert any(r['case']==case and r['candidate'] in ('CenteredSelective','CenteredFirst') and r['accepted']for r in recovery)
timing={}
for k,v in ts.items():
 timing[k]={'rows':v['rows'],'zeroheap':v['zeroheap'],'slabs':v['slabs'],'geomean_time_ratio_to_split':math.exp(statistics.mean(map(math.log,v['ratios']))),'median_bound_ratio_to_split':statistics.median(v['bounds']),'median_ns_sum':sum(v['median_ns'])}
summary={'rows':{'replay':len(replay),'decay':len(cost),'slabs':len(slabs),'dense':len(dense)},'checks':checks,'max_sample_error_ratio':worst,'max_endpoint_error_ratio':endpoint_worst,'timing':timing,'dense':ds,'replay':rs,'recovery':recovery,'reference':'Exact binary64 rational clock and scaled atanh; independent 1024-bit direct Taylor semigroup and exact constant forward invariant. Timing descriptive cyclic paired order on CPU0, no statistical speedup guarantee.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
