from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections
P=Path(__file__).parent
PREV=P.parent/'m2_ring_local_signed_20261127'
def read(p,marker):
 if not p.exists():p=Path(str(p)+'.gz')
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def contains(iv,bounds):
 lo,hi=map(Q,iv)
 assert lo<=bounds[0]<=bounds[1]<=hi,(iv,tuple(map(float,bounds)))
@lru_cache(None)
def log_bounds(q):
 s=(q-1)/(q+1);assert 0<=s<1
 v=2*sum((s**(2*k+1)/(2*k+1) for k in range(60)),Q(0))
 return v,v+2*s**121/(121*(1-s*s))
def goal_reference(case,t):
 t=Q(t);count=4 if case=='clock8' else 1
 lo=hi=Q(0)
 for j in range(count):
  a=Q(j+1);q=1+a*t*t/8
  x=Q(1,4)/q;l,h=log_bounds(q)
  y0=Q(1,16);coef=Q(3,2) if case=='coupled_clock' else Q(1)
  const=(Q(1,2) if j%2==0 else -Q(1,2))*x
  lo+=const+coef*(y0+l/a);hi+=const+coef*(y0+h/a)
 return lo,hi
def error_bounds(case,t,y,c):
 lo,hi=goal_reference(case,t);n=sum((Q(a)*Q(b) for a,b in zip(y,c)),Q(0))
 return lo-n,hi-n
@lru_cache(None)
def state_bounds(case,t):
 t=Q(t);p=Q(1,4)/(1+t*t/8);l,h=log_bounds(1+t*t/8)
 q=(Q(1,16)+l,Q(1,16)+h)
 u=(p+q[0],p+q[1]) if case=='coupled_clock' else (p,p)
 return (u,q,u,q)
def poly(row,theta):
 return tuple(sum((Q(c[i])*theta**j for j,c in enumerate(row)),Q(0))for i in [0,1])
def maxdiff(a,b):return max(abs(a[0]-b[1]),abs(a[1]-b[0]))
runs=read(P/'tube.log','TUBE_RUN')
slabs=read(P/'tube.log','TUBE_SLAB')
refusals=read(P/'tube.log','TUBE_REFUSAL')
direct=read(P/'tube.log','TUBE_ADJOINT')
goals=read(P/'goal.log','ADAPTIVE_GOAL')
assert len(runs)==35 and len(direct)==125 and len(goals)==57,(len(runs),len(direct),len(goals))
by={(r['case'],r['end'],r['candidate']):r for r in runs};assert len(by)==35
counts=collections.Counter();samples=0
maxratio=0.
for r in slabs:
 k=(r['case'],r['end'],r['candidate']);counts[k]+=1
 w=r['work'];assert req(r['allocation'])==0
 assert Q(w['p_lower'])>=1/(2*Q(w['weight']))
 assert Q(w['outgoing'])>=sum(map(Q,[w['incoming'],w['seam'],w['residual_integral_upper']]))
 assert w['outgoing']==r['x_error'] and r['x_error']<1e-3
 assert r['z_error']>=r['x_error']
 a,b=map(Q,r['times'])
 for theta in [Q(0),Q(1,2),Q(1)]:
  t=a+(b-a)*theta
  exact=state_bounds(r['case'],t)
  for i,(row,truth) in enumerate(zip(r['power'],exact)):
   iv=poly(row,theta);bound=Q(r['x_error'] if i<2 else r['z_error'])
   e=maxdiff(iv,truth)
   assert e<=bound,(k,r['index'],i,float(e),float(bound))
   if bound:maxratio=max(maxratio,float(e/bound))
   samples+=1
stats=collections.defaultdict(lambda:dict(rows=0,qualified=0,slabs=0,warm_zero=0))
for r in runs:
 k=(r['case'],r['end'],r['candidate']);s=stats[r['candidate']]
 s['rows']+=1;s['qualified']+=r['qualified']
 baseline=by[(r['case'],r['end'],'Flat')]
 assert r['rhs']==baseline['rhs'] and r['slabs']==baseline['slabs']
 if r['candidate']=='Flat':continue
 assert counts[k]==r['qualified_slabs'];s['slabs']+=counts[k]
 assert r['warm_allocations']==0;s['warm_zero']+=counts[k]
 if r['qualified']:
  assert r['qualified_slabs']==r['slabs'] and r['bound']<=1e-3
  exact=state_bounds(r['case'],Q(r['end']))
  for v,b in zip(r['terminal'],exact):
   assert maxdiff((Q(v),Q(v)),b)<=Q(r['terminal_bound'])
assert len(refusals)==3
for r in refusals:
 assert r['case']=='coupled_clock' and r['end']==3 and r['candidate']!='Mean8'
 assert 'positivity/contraction' in r['reason']
dstats=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,actions=0,cells=0,adaptive=[0,0,0]))
for r in direct:
 w=r['work'];s=dstats[r['route']]
 assert req(r['allocation'])==0
 contains(w['signed_error'],error_bounds(r['case'],r['end'],r['terminal'],r['weights']))
 s['rows']+=1;s['zeroheap']+=1;s['actions']+=w['matrix_actions'];s['cells']+=w['cells']
 if (a:=w.get('adaptive_cells')) is not None:
  assert sum(a)==w['slabs'] and sum(v*n for v,n in zip(a,[4,8,16]))==w['cells']
  s['adaptive']=[x+y for x,y in zip(s['adaptive'],a)]
gstats=collections.defaultdict(lambda:dict(rows=0,physical=0,accepted=0,actions=0,certificates=0))
old={(r['case'],r['end'],r['absolute']):r for r in goals if r['plan']=='Fixed'}
for r in goals:
 a=r['attempt'];o=old[(r['case'],r['end'],r['absolute'])];s=gstats[r['plan']]
 s['rows']+=1;s['physical']+=a['physical_qualified'];s['accepted']+=r['qualified']
 assert r['rhs']==o['rhs'] and r['steps']==o['steps']
 if r['plan']!='WeightedAdaptiveSigned':
  assert a['physical_qualified']==o['attempt']['physical_qualified'] and a['terminal_state']==o['attempt']['terminal_state']
 elif o['attempt']['physical_qualified']:
  assert a['terminal_state']==o['attempt']['terminal_state']
 if not a['physical_qualified']:
  assert not r['qualified'] and not a['certificates'] and a['enclosure'] is None
  continue
 ref=error_bounds(r['case'],r['end'],a['terminal_state'],r['weights'])
 if a['enclosure'] is not None:contains(a['enclosure'],ref)
 for c in a['certificates']:
  if c.get('work') is not None:
   contains(c['work']['signed_error'],ref);s['actions']+=c['work']['matrix_actions'];s['certificates']+=1
 if r['qualified']:assert max(map(abs,a['enclosure']))<=r['absolute']
# Replay unchanged spatial integrators against prior exact-rational checked receipts.
spatial=read(P/'residual.log','COMPACT_CERT');native=read(P/'native.log','COMPACT_IVP')
prior_sp={(r['case'],r['scenario'],r['candidate']):r for r in read(PREV/'residual.log','COMPACT_CERT')}
prior_iv={(r['case'],r['candidate']):r for r in read(PREV/'native.log','COMPACT_IVP')}
assert len(spatial)==240 and len(native)==28
for r in spatial:
 o=prior_sp[(r['case'],r['scenario'],r['candidate'])]
 assert r['receipt']==o['receipt'] and req(r['allocation'])==req(r['prepare'])==0
for r in native:
 o=prior_iv[(r['case'],r['candidate'])]
 assert r['qualified'] and r['run']==o['run'] and r['error_upper']==o['error_upper']
dump('INDEPENDENT_CHECKS.json',{'tube_runs':len(runs),'weighted_slabs':len(slabs),'sample_coordinate_bounds':samples,'max_sample_error_over_bound':maxratio,'tube':stats,'refusals':refusals,'direct':dstats,'goal':gstats,'spatial_replay':len(spatial),'native_replay':len(native),'method':'Exact binary64 inputs as Fractions, rational positive atanh series and geometric tail, 3 points per slab; sample checks do not prove continuous tube beyond analytic bound.'})
print('VERIFIED',len(runs),len(slabs),samples,len(direct),len(goals),len(spatial),len(native))
print('TUBE',dict(stats));print('GOAL',dict(gstats));print('DIRECT',dict(dstats));print('MAX SAMPLE ERROR / BOUND',maxratio)
