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
def model(r,t):
 x=list(map(Q,r['initial']));w=list(map(Q,r['weights']));n=len(x);t,s=Q(t),Q(r['start']);coupled=r['case']=='coupled_clock'
 moments=[(t**(k+1)-s**(k+1))/(k+1)for k in range(3)]
 for j,(aa,bb) in enumerate(zip(r['a'],r['b'])):
  a,b=list(map(Q,aa)),list(map(Q,bb));i=2*j
  pivot=next(k for k in range(3)if a[k]>0);ratio=b[pivot]/a[pivot];assert all(y==ratio*z for y,z in zip(b,a))
  m=sum((a[k]*moments[k]for k in range(3)),Q(0));p=x[i]-x[i+1] if coupled else x[i];q=x[i+1];d=1+p*m
  wp=w[i]+w[n+i];wq=w[i+1]+w[n+i+1]
  if coupled:wq+=wp
  pe=p/d;l,h=log_bounds(d);qe=sorted((q+ratio*l,q+ratio*h));ue=(pe+qe[0],pe+qe[1]) if coupled else (pe,pe)
  slope=-wp*pe*pe+wq*ratio*pe
  params=[pt(v*slope)for v in moments]+[tuple(sorted((wq*l,wq*h)))]
  yield (ue,tuple(qe)),params
def state(r,t):
 x=[v for pair,_ in model(r,t)for v in pair];return x+x
def poly_eval(row,theta):
 lo=hi=Q(0)
 for a,b in reversed(row):lo=lo*theta+Q(a);hi=hi*theta+Q(b)
 return lo,hi
def err_box(a,b):return max(abs(a[0]-b[1]),abs(a[1]-b[0]))
runs=read(P/'clock.log','BLOCK_RUN');slabs=read(P/'clock.log','BLOCK_SLAB');params=read(P/'clock.log','PARAM_COV');policies=read(P/'clock.log','BLOCK_POLICY')
assert (len(runs),len(params),len(policies))==(44,48,64),(len(runs),len(params),len(policies))
models={r['case']:r for r in runs if r['candidate']=='Flat'}
checks=collections.Counter();tube=collections.defaultdict(lambda:dict(rows=0,accepted=0,slabs=0,max_bound=0.,max_weight=0.))
for r in runs:
 s=tube[r['candidate']];s['rows']+=1;s['accepted']+=r['qualified'];s['slabs']+=r.get('qualified_slabs',0)
 assert r['rhs']==models[r['case']]['rhs']
 if r['qualified']:
  ref=state(r,Q(r['end']));assert len(ref)==len(r['terminal'])
  assert max(err_box(pt(Q(v)),x)for v,x in zip(r['terminal'],ref))<=Q(r['terminal_bound'])
  s['max_bound']=max(s['max_bound'],r['terminal_bound'])
assert len(slabs)==sum(r.get('qualified_slabs',0)for r in runs)
@lru_cache(None)
def sample(case,t):return state(models[case],t)
worst=0.
for r in slabs:
 assert req(r['allocation'])==0
 s=tube[r['candidate']];s['max_weight']=max(s['max_weight'],r['work']['weight'])
 n=len(r['power'])//2;t0,t1=map(Q,r['times'])
 for theta in [Q(0),Q(1,4),Q(1,2),Q(3,4),Q(1)]:
  exact=sample(r['case'],t0+(t1-t0)*theta)
  for i,row in enumerate(r['power']):
   bound=Q(r['x_error'] if i<n else r['z_error'])
   error=err_box(poly_eval(row,theta),exact[i])
   assert error<=bound,(r['case'],r['candidate'],r['index'],i,theta,float(error),float(bound))
   if bound:worst=max(worst,float(error/bound))
   checks['slab_coordinate_samples']+=1
ps=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,max_width=0.))
for r in params:
 assert req(r['allocation'])==0
 truth=[v for _,params in model(r,Q(r['end']))for v in params]
 assert len(truth)==len(r['gradient'])
 for iv,t in zip(r['gradient'],truth):contains(iv,t,(r['case'],r['candidate']));checks['parameter_coordinates']+=1
 s=ps[r['candidate']+'/'+r['measure']];s['rows']+=1;s['zeroheap']+=1;s['max_width']=max(s['max_width'],r['work']['max_width'])
# Central difference of the analytic family, independent of derivative
# algebra; smooth extension at coefficient-boundary points.
h=Q(1,2**18);fdmax=Q(0)
for r in models.values():
 x=list(map(Q,r['initial']));w=list(map(Q,r['weights']));n=len(x);t,s=Q(r['end']),Q(r['start'])
 moments=[(t**(k+1)-s**(k+1))/(k+1)for k in range(3)]
 for j,(aa,bb) in enumerate(zip(r['a'],r['b'])):
  a,b=list(map(Q,aa)),list(map(Q,bb));i=2*j;p0=x[i]-x[i+1] if r['case']=='coupled_clock' else x[i];q0=x[i+1]
  pivot=next(k for k in range(3)if a[k]>0);ratio=b[pivot]/a[pivot];wp=w[i]+w[n+i];wq=w[i+1]+w[n+i+1]+(wp if r['case']=='coupled_clock' else 0)
  def goal_at(aa,rr):
   d=1+p0*sum((u*v for u,v in zip(aa,moments)),Q(0));l,u=log_bounds(d);lo,hi=sorted((wq*rr*l,wq*rr*u));base=wp*p0/d+wq*q0
   return base+lo,base+hi
  exact=list(model(r,t))[j][1]
  for k in range(4):
   ap=a.copy();am=a.copy();rp=rm=ratio
   if k<3:ap[k]+=h;am[k]-=h
   else:rp+=h;rm-=h
   plus,minus=goal_at(ap,rp),goal_at(am,rm);fd=((plus[0]-minus[1])/(2*h),(plus[1]-minus[0])/(2*h))
   e=err_box(fd,exact[k]);assert e<=Q(1,10**8);fdmax=max(fdmax,e);checks['analytic_central_difference_diagnostic']+=1
gs=collections.defaultdict(lambda:dict(rows=0,accepted=0,physical=0))
base={(r['case'],r['absolute']):r for r in policies if r['candidate']=='PolynomialFlatTermwise'}
for r in policies:
 a=r['attempt'];s=gs[r['candidate']];s['rows']+=1;s['accepted']+=a['accepted'];s['physical']+=a['physical_qualified']
 old=base[r['case'],r['absolute']];assert r['rhs']==old['rhs'] and r['steps']==old['steps']
 if not a['physical_qualified']:assert not a['accepted'];continue
 u=state(r,Q(r['end']));w=list(map(Q,r['weights']));lo,hi=dot_box(u,w);num=sum((Q(v)*x for v,x in zip(a['terminal_state'],w)),Q(0))
 if a['enclosure'] is not None:contains(a['enclosure'],(lo-num,hi-num),(r['case'],r['candidate']));checks['goal_enclosures']+=1
 if a['accepted']:assert max(map(abs,a['enclosure']))<=r['absolute']
assert read(P/'refusal.log','PARAM_REFUSAL')[0]['late_block']
# Independent fixed-point Taylor reference reused from prior spatial work:
# 256-bit directed integer grid, exact geometric full-horizon tail.
S=1<<256
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
 edges=[(i,j,a.numerator,a.denominator)for i,row in enumerate(rows)for j,a in row if j!=i and a]
 # Exact invariant if represented input is in the null space.
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
grids=read(P/'spatial.log','BATCH_GRID');native=read(P/'spatial.log','BATCH_NATIVE');batch=read(P/'spatial.log','BATCH_ADJOINT')
assert (len(grids),len(native),len(batch))==(5,10,180)
grid={r['case']:r for r in grids};references={};max_native=Q(0)
for r in native:
 g=grid[r['case']];n=g['n'];rows=[[(j,Q(a))for j,a in row]for row in g['rows']];c=list(map(Q,r['weights']))
 ref,order=taylor(rows,c,Q(g['duration']),True);references[r['case'],r['goal']]=(ref,c,order)
 assert req(r['allocation'])==0
 e=max(err_box(pt(Q(v)),x)for v,x in zip(r['values'],ref));max_native=max(max_native,e);assert e<=Q(1,10**6)
bs=collections.defaultdict(lambda:dict(rows=0,actions=0,zeroheap=0,max_width=0.))
old={(r['case'],r['goal'],r['route'],r['pairs']):r for r in batch if r['candidate']=='Repeated'}
for r in batch:
 g=grid[r['case']];n=g['n'];ref,c,_=references[r['case'],r['goal']];k=r['pairs'];key=r['case'],r['goal'],r['route'],k
 assert req(r['allocation'])==0
 for a,b in zip(r['gradient'],ref):contains(a,b,key);checks['spatial_gradient_coordinates']+=1
 for i,v in enumerate(r['receipts']):
  u=list(map(Q,g['initial'][i*n:(i+1)*n]));y=list(map(Q,g['endpoint'][i*n:(i+1)*n]))
  l,h=dot_box(ref,u);j=sum((x*z for x,z in zip(c,y)),Q(0))
  contains(v['exact_goal'],(l,h),key);contains(v['signed_goal_error'],(l-j,h-j),key);checks['spatial_goal_pairs']+=1
 if r['candidate']=='SharedL1':assert r['receipts']==old[key]['receipts']
 s=bs[r['candidate']];s['rows']+=1;s['zeroheap']+=1;s['actions']+=r['work']['actions'];s['max_width']=max(s['max_width'],r['work']['max_goal_width'])
summary={'rows':{'tube':len(runs),'slabs':len(slabs),'parameter':len(params),'policy':len(policies),'spatial':len(batch),'native':len(native)},'checks':checks,'tube':tube,'max_sample_error_ratio':worst,'parameter':ps,'policy':gs,'spatial':bs,'max_native_adjoint_error':float(max_native),'max_analytic_central_difference_error':float(fdmax),'reference':'Exact binary64 rational polynomial clock, 40-term scaled atanh enclosure, independent 256-bit fixed-point directed direct Taylor spatial semigroup and rational tail; finite samples not continuum proof.'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
