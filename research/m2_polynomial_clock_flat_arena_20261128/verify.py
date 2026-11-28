from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections,statistics,math
P=Path(__file__).parent
def read(path,marker):
 if not path.exists():path=Path(str(path)+'.gz')
 with (gzip.open(path,'rt') if path.suffix=='.gz' else path.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,sort_keys=True,indent=2,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
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
 x=list(map(Q,r['initial']));w=list(map(Q,r['weights']));n=len(x);t,s=Q(r['end']),Q(r['start'])
 coupled=r['case']=='coupled_clock'
 for j,(aa,bb) in enumerate(zip(r['a'],r['b'])):
  a,b=list(map(Q,aa)),list(map(Q,bb));i=2*j
  pivot=next(k for k in range(3)if a[k]>0);ratio=b[pivot]/a[pivot];assert all(y==ratio*z for y,z in zip(b,a))
  m=sum((a[k]*(t**(k+1)-s**(k+1))/(k+1)for k in range(3)),Q(0))
  p=x[i]-x[i+1] if coupled else x[i];q=x[i+1];d=1+p*m
  wp=w[i]+w[n+i];wq=w[i+1]+w[n+i+1]
  if coupled:wq+=wp
  lp=wp/d**2+wq*ratio*m/d
  lq=wq-lp if coupled else wq
  pe=p/d;l,h=log_bounds(d);qe=sorted((q+ratio*l,q+ratio*h));ue=(pe+qe[0],pe+qe[1]) if coupled else (pe,pe)
  yield i,(lp,lq),(ue,qe),m
def ref(r):
 n=len(r['initial']);y=list(map(Q,r['terminal']));w=list(map(Q,r['weights']));lo=hi=Q(0);err=Q(0)
 for i,_,(u,v),_ in model(r):
  for j,(a,b) in [(i,u),(i+1,v),(n+i,u),(n+i+1,v)]:
   err=max(err,abs(a-y[j]),abs(b-y[j]));z=sorted(((a-y[j])*w[j],(b-y[j])*w[j]));lo+=z[0];hi+=z[1]
 return (lo,hi),err
arena=read(P/'arena.log','FLAT_IVP');ode=read(P/'ode.log','FLAT_ODE');tube=read(P/'clock.log','POLY_TUBE');direct=read(P/'clock.log','POLY_DIRECT');grad=read(P/'clock.log','POLY_GRAD');policy=read(P/'clock.log','POLY_POLICY')
assert (len(arena),len(ode),len(tube),len(direct),len(grad),len(policy))==(288,6,25,42,21,92),(len(arena),len(ode),len(tube),len(direct),len(grad),len(policy))
sp={(r['case'],r['schedule'],r['partial'],r['plan'],r['candidate']):r for r in arena};assert len(sp)==288
prev=read(P.parent/'m2_output_arena_clock_adjoint_20261128/arena.log','ARENA_IVP')
prior={(r['case'],r['partial'],r['plan']):r for r in prev if r['schedule']==0 and r['candidate']=='Legacy'}
stats=collections.defaultdict(lambda:dict(rows=0,requests=0,bytes=0,setup_requests=0,setup_bytes=0))
timings=collections.defaultdict(list);comparisons=0
for k,r in sp.items():
 old=sp[(*k[:-1],'RowLegacy')];assert r['run']==old['run']
 assert req(r['allocation'])==0 and r['allocation']['requested_bytes']==0 and len(r['samples_ns'])==8
 if r['schedule']==0 and (r['case'],r['partial'],r['plan']) in prior:
  assert r['run']==prior[(r['case'],r['partial'],r['plan'])]['run'];comparisons+=1
 s=stats[r['candidate']];s['rows']+=1;s['requests']+=req(r['allocation']);s['bytes']+=r['allocation']['requested_bytes'];s['setup_requests']+=req(r['setup']);s['setup_bytes']+=r['setup']['requested_bytes']
 if r['candidate'].startswith('Flat'):
  row=sp[(*k[:-1],r['candidate'].replace('Flat','Row'))];timings[(r['candidate'],r['partial'])].extend(a/b for a,b in zip(row['samples_ns'],r['samples_ns']))
for r in ode:assert req(r['allocation'])==0 and r['qualified'] and r['max_error']<=1e-8
tstats=collections.defaultdict(lambda:dict(rows=0,accepted=0,max_bound=0.))
by={}
for r in tube:
 s=tstats[r['candidate']];s['rows']+=1;s['accepted']+=r['qualified']
 key=r['case'];z=(r['driver_last'],r['rhs'])
 if key in by:assert by[key]==z
 else:by[key]=z
 if r['qualified']:
  _,err=ref(r);assert Q(r['bound'])>=err,(r['case'],r['candidate'],r['bound'],float(err))
  s['max_bound']=max(s['max_bound'],r['bound'])
ds=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,max_gradient_width=0.,max_goal_width=0.,max_measure_width=0.))
for r in direct:
 assert req(r['allocation'])==0 and r['qualified'];truth=[]
 for _,pair,_,m in model(r):truth.extend(pair)
 for iv,t in zip(r['gradient'],truth):contains(iv,(t,t))
 contains(r['work']['goal']['signed_error'],ref(r)[0])
 s=ds[r['candidate']+'/'+r['log']];s['rows']+=1;s['zeroheap']+=1
 for k,w in [('max_gradient_width',r['work']['gradient_width']),('max_goal_width',r['work']['goal']['width_upper']),('max_measure_width',r['work']['measure_width'])]:s[k]=max(s[k],w)
gs=collections.defaultdict(lambda:dict(rows=0,qualified=0,actions=0,max_width=0.))
for r in grad:
 s=gs[r['candidate']];s['rows']+=1
 if not r['qualified']:continue
 truth=[]
 for _,pair,_,_ in model(r):truth.extend(pair)
 for iv,t in zip(r['gradient'],truth):contains(iv,(t,t))
 s['qualified']+=1;s['actions']+=r['actions'];s['max_width']=max(s['max_width'],r['width'])
ps=collections.defaultdict(lambda:dict(rows=0,accepted=0,physical=0,fused=0,actions=0,series=0,roots=0))
fixed={(r['case'],r['absolute']):r for r in policy if r['candidate']=='Fixed'}
for r in policy:
 a=r['attempt'];s=ps[r['candidate']];s['rows']+=1;s['accepted']+=a['accepted'];s['physical']+=a['physical_qualified']
 old=fixed[(r['case'],r['absolute'])];assert r['rhs']==old['rhs'] and r['steps']==old['steps']
 if a['physical_qualified'] and old['attempt']['physical_qualified']:assert a['terminal_state']==old['attempt']['terminal_state']
 if not a['physical_qualified']:assert not a['accepted'] and a['enclosure'] is None;continue
 rr=dict(r,terminal=a['terminal_state']);truth,_=ref(rr)
 if a['enclosure'] is not None:contains(a['enclosure'],truth)
 if a['accepted']:assert max(map(abs,a['enclosure']))<=r['absolute']
 for c in a['certificates']:
  if c.get('work'):
   contains(c['work']['signed_error'],truth);s['actions']+=c['work']['matrix_actions']
 if (w:=a.get('polynomial_work')):
  contains(w['goal']['signed_error'],truth);s['fused']+=1;s['series']+=w['goal']['series_terms'];s['roots']+=w['goal']['square_roots']
assert len(read(P/'refusal.log','POLY_REFUSAL'))==1
summary={'rows':{'arena':len(arena),'ode':len(ode),'tube':len(tube),'direct':len(direct),'gradient':len(grad),'policy':len(policy)},'spatial':stats,'prior_spatial_receipts':comparisons,'timing':{'/'.join(map(str,k)):{'paired_samples':len(v),'geomean_speedup_row_over_flat':math.exp(statistics.mean(map(math.log,v))),'median_ratio':statistics.median(v),'min':min(v),'max':max(v)}for k,v in timings.items()},'tube':tstats,'direct':ds,'general_gradient':gs,'policy':ps,'reference':'exact binary64 Fractions, exact polynomial integration and ratio, rational gradient and dyadically scaled positive atanh log bounds; spatial verified equality and prior M2.29 same endpoint receipts, new holdout not continuum verification'}
dump('INDEPENDENT_CHECKS.json',summary);print('VERIFIED',json.dumps(summary,sort_keys=True))
