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
runs=read(P/'clock.log','DECAY_RUN');slabs=read(P/'clock.log','DECAY_SLAB');dense=read(P/'clock.log','CLOCK_DENSE')
assert (len(runs),len(dense))==(50,60),(len(runs),len(dense))
models={r['case']:r for r in runs if r['candidate']=='Flat'}
assert len(models)==10
checks=collections.Counter();tube=collections.defaultdict(lambda:dict(rows=0,accepted=0,slabs=0,max_bound=0.,max_peak=0.))
for r in runs:
 s=tube[r['candidate']];s['rows']+=1;s['accepted']+=r['qualified'];s['slabs']+=r.get('qualified_slabs',0)
 assert r['rhs']==models[r['case']]['rhs']
 if r['qualified']:
  ref=state(r,Q(r['end']));assert len(ref)==len(r['terminal'])
  assert max(err_box(pt(Q(v)),x)for v,x in zip(r['terminal'],ref))<=Q(r['terminal_bound'])
  s['max_bound']=max(s['max_bound'],r['terminal_bound']);s['max_peak']=max(s['max_peak'],r['whole_bound'])
assert len(slabs)==sum(r.get('qualified_slabs',0)for r in runs)
@lru_cache(None)
def sample(case,t):return state(models[case],t)
worst=0.;endpoint_worst=0.
for r in slabs:
 assert req(r['allocation'])==0
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
    endpoint=Q(r['outgoing']);assert error<=endpoint,(r['case'],r['candidate'],r['index'],i,float(error),float(endpoint))
    if endpoint:endpoint_worst=max(endpoint_worst,float(error/endpoint))
    checks['endpoint_carry_coordinates']+=1
ds=collections.defaultdict(lambda:dict(rows=0,requests=0,coordinates=0,zeroheap=0,max_radius=0.,terms=0,roots=0))
centers={}
for r in dense:
 assert req(r['allocation'])==0
 key=r['case'];t=r['times'];y=r['centers'];rad=r['radii'];w=r['work']
 assert len(t)==len(y)==len(rad)==w['requests']+1
 assert t[0]==models[key]['start'] and t[-1]==models[key]['end']
 assert all(a<b for a,b in zip(t,t[1:]))
 if key in centers:assert (t,y)==centers[key]
 else:centers[key]=(t,y)
 s=ds[r['candidate']+'/'+r['log']];s['rows']+=1;s['requests']+=w['requests'];s['zeroheap']+=1;s['max_radius']=max(s['max_radius'],w['max_radius']);s['terms']+=w['log_terms'];s['roots']+=w['square_roots']
 for time,center,radius in zip(t,y,rad):
  exact=sample(key,Q(time));assert len(center)==len(radius)==len(exact)
  for v,e,truth in zip(center,radius,exact):
   assert err_box(pt(Q(v)),truth)<=Q(e),(key,time,r['candidate'],v,e,tuple(map(float,truth)))
   assert e<=w['max_radius'];checks['dense_coordinate_samples']+=1;s['coordinates']+=1
assert read(P/'refusal.log','DENSE_REFUSAL')==[{'late_overflow':True,'unchanged_prefix':True,'retry_zeroheap':True,'reset':True}]
policies=read(P/'policy.log','DECAY_POLICY')
assert len(policies)==32
ps=collections.defaultdict(lambda:dict(rows=0,accepted=0,physical=0))
for r in policies:
 a=r['attempt'];s=ps[r['candidate']];s['rows']+=1;s['accepted']+=a['accepted'];s['physical']+=a['physical_qualified']
 if not a['physical_qualified']:assert not a['accepted'];continue
 m=models[r['case']];ref=state(m,Q(m['end']));w=list(map(Q,m['weights']));lo,hi=dot_box(ref,w);num=sum((Q(v)*x for v,x in zip(a['terminal_state'],w)),Q(0))
 if a['enclosure'] is not None:contains(a['enclosure'],(lo-num,hi-num),(r['case'],r['candidate']));checks['dae_goal_enclosures']+=1
 if a['accepted']:assert max(map(abs,a['enclosure']))<=r['absolute']
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
grids=read(P/'spatial.log','CENTER_GRID');native=read(P/'spatial.log','CENTER_NATIVE');batch=read(P/'spatial.log','CENTER_BATCH');budget=read(P/'spatial.log','CENTER_BUDGET')
assert (len(grids),len(native),len(batch),len(budget))==(5,10,120,240),(len(grids),len(native),len(batch),len(budget))
grid={r['case']:r for r in grids};references={};max_native=Q(0)
for r in native:
 g=grid[r['case']];n=g['n'];rows=[[(j,Q(a))for j,a in row]for row in g['rows']];c=list(map(Q,r['weights']))
 assert req(r['allocation'])==0 and len(r['values'])==len(r['times'])*n
 for k,t in enumerate(r['times']):
  ref,order=taylor(rows,c,Q(t),True)
  e=max(err_box(pt(Q(v)),x)for v,x in zip(r['values'][k*n:(k+1)*n],ref));max_native=max(max_native,e);assert e<=Q(1,10**6)
  checks['native_spatial_coordinates']+=n
  if k==len(r['times'])-1:references[r['case'],r['goal']]=(ref,c,order)
bs=collections.defaultdict(lambda:dict(rows=0,actions=0,zeroheap=0,max_width=0.))
ix={(r['case'],r['goal'],r['route'],r['pairs'],r['candidate']):r for r in batch};assert len(ix)==len(batch)
truths={}
for key,r in ix.items():
 g=grid[r['case']];n=g['n'];ref,c,_=references[r['case'],r['goal']];k=r['pairs']
 assert req(r['allocation'])==0
 for a,b in zip(r['gradient'],ref):contains(a,b,key);checks['spatial_gradient_coordinates']+=1
 goal_ref=[]
 for i,v in enumerate(r['receipts']):
  u=list(map(Q,g['initial'][i*n:(i+1)*n]));y=list(map(Q,g['endpoint'][i*n:(i+1)*n]))
  # Exact constant initial state is an invariant of the represented
  # difference generator. Componentwise Taylor boxes do not preserve
  # its exact dual dot product, so use the independent exact forward
  # identity (not any production result) on this input stratum.
  if all(x==u[0] for x in u):
   l=h=u[0]*sum(c,Q(0));checks['exact_constant_forward_goals']+=1
  else:l,h=dot_box(ref,u)
  j=sum((x*z for x,z in zip(c,y)),Q(0))
  contains(v['exact_goal'],(l,h),key);contains(v['signed_goal_error'],(l-j,h-j),key);checks['spatial_goal_pairs']+=1;goal_ref.append((l-j,h-j))
 truths[key]=goal_ref
 old=ix[(*key[:-1],'SharedL1')]
 assert r['gradient']==old['gradient'] and r['work']['actions']==old['work']['actions']
 if r['candidate']=='SharedCenteredIntersect':
  for a,b in zip(r['receipts'],old['receipts']):assert b['exact_goal'][0]<=a['exact_goal'][0]<=a['exact_goal'][1]<=b['exact_goal'][1]
 s=bs[r['route']+'/'+r['candidate']];s['rows']+=1;s['zeroheap']+=1;s['actions']+=r['work']['actions'];s['max_width']=max(s['max_width'],r['work']['max_goal_width'])
gs=collections.defaultdict(lambda:dict(rows=0,accepted=0,pairs=0,accepted_pairs=0,true_failure_pairs=0,conservative_refusal_pairs=0))
for r in budget:
 key=(r['case'],r['goal'],r['route'],r['pairs'],r['candidate']);old=ix[key];w=r['work'];b=Q(r['budget'])
 assert req(r['allocation'])==0
 admitted=[max(map(abs,v['signed_goal_error']))<=r['budget']for v in old['receipts']]
 assert w['accepted_pairs']==sum(admitted) and w['accepted']==all(admitted)==r['qualified'] and w['work']==old['work']
 s=gs[r['route']+'/'+r['candidate']+'/'+str(r['budget'])];s['rows']+=1;s['accepted']+=w['accepted'];s['pairs']+=r['pairs'];s['accepted_pairs']+=w['accepted_pairs']
 for ok,truth in zip(admitted,truths[key]):
  if ok:assert max(map(abs,truth))<=b
  elif truth[0]>b or truth[1]<-b:s['true_failure_pairs']+=1
  elif max(map(abs,truth))<=b:s['conservative_refusal_pairs']+=1
summary={'rows':{'tube':len(runs),'slabs':len(slabs),'dense':len(dense),'spatial':len(batch),'budget':len(budget),'native':len(native),'policy':len(policies)},'checks':checks,'policy':ps,'tube':tube,'max_sample_error_ratio':worst,'max_endpoint_error_ratio':endpoint_worst,'dense':ds,'spatial':bs,'budget':gs,'max_native_adjoint_error':float(max_native),'reference':'Exact represented binary64 rational polynomial clock and scaled positive atanh log enclosure; 1024-bit fixed-point direct Taylor semigroup with rigorous rational geometric tail, exact constant-state forward invariant; finite samples, not continuum discretization error.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
