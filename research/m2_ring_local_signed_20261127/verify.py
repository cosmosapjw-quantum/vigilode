from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections
P=Path(__file__).parent
PREV=P.parent/'m2_structured_affine_compact_20261127'
def read(p,marker):
 if not p.exists():p=Path(str(p)+'.gz')
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def contains(iv,bounds):
 lo,hi=map(Q,iv)
 assert lo<=bounds[0]<=bounds[1]<=hi,(iv,tuple(map(float,bounds)))
def log_bounds(q):
 s=(q-1)/(q+1);assert 0<=s<1
 v=2*sum((s**(2*k+1)/ (2*k+1) for k in range(60)),Q(0))
 return v,v+2*s**121/(121*(1-s*s))
def goal_reference(case,t):
 t=Q(t);count=4 if case=='clock8' else 1
 lo=hi=Q(0)
 for j in range(count):
  a=Q(j+1);q=1+a*t*t/8
  x=Q(1,4)/q; l,h=log_bounds(q)
  y0=Q(1,16);coef=Q(3,2) if case=='coupled_clock' else Q(1)
  const=(Q(1,2) if j%2==0 else -Q(1,2))*x
  lo+=const+coef*(y0+l/a);hi+=const+coef*(y0+h/a)
 return lo,hi
def error_bounds(case,t,y,c):
 lo,hi=goal_reference(case,t);n=sum((Q(a)*Q(b) for a,b in zip(y,c)),Q(0))
 return lo-n,hi-n
def base_stats():return dict(rows=0,admitted=0,fallbacks=0,zeroheap=0)
grids={r['case']:r['rows'] for r in read(P/'residual.log','COMPACT_GRID')}
inputs={(r['case'],r['scenario']):r for r in read(P/'residual.log','COMPACT_INPUT')}
rows=read(P/'residual.log','COMPACT_CERT')
assert len(grids)==8 and len(inputs)==40 and len(rows)==240
exact={}
for k,r in inputs.items():
 x=list(map(Q,r['x']));b=list(map(Q,r['b']));z=Q(r['shift'])
 d=[v-z*u for v,u in zip(b,x)]
 for i,rr in enumerate(grids[k[0]]):
  for j,a in rr:
   if j!=i and a!=0:
    p=Q(a)*x[i];d[i]-=p;d[j]+=p
 exact[k]=(sum(map(abs,d)),z,sum(map(abs,b)))
stats=collections.defaultdict(base_stats)
indexed={(r['case'],r['scenario'],r['candidate']):r for r in rows}
assert len(indexed)==240
for r in rows:
 k=(r['case'],r['scenario']);rho,z,b=exact[k];a=r['receipt']
 assert rho<=Q(a['residual_l1']) and rho/z<=Q(a['error_l1']) and b<=Q(a['rhs_l1']),(k,r['candidate'])
 assert req(r['allocation'])==req(r['prepare'])==0
 if r['candidate'].startswith('RingLocal'):
  orig=indexed[(*k,r['candidate'].replace('RingLocal','Compact'))]
  assert a==orig['receipt'],(k,r['candidate'],a,orig['receipt'])
 s=stats[r['candidate']];s['rows']+=1;s['admitted']+=a['admitted'];s['fallbacks']+=a['fallbacks'];s['zeroheap']+=1
native=read(P/'native.log','COMPACT_IVP')
prior={r['case']:r for r in read(PREV/'native.log','COMPACT_IVP') if r['candidate']=='Hybrid'}
assert len(native)==28
for r in native:
 assert r['qualified'] and r['error_upper']<=1e-6
 p=prior[r['case']]
 assert p['qualified'] and p['run']==r['run'],(r['case'],r['candidate'])
goals=read(P/'goal.log','SIGNED_GOAL')
physical=read(P/'goal.log','SIGNED_PHYSICAL')
direct=read(P/'goal.log','SIGNED_DIRECT')
assert len(goals)==36 and len(physical)==4
old={(r['case'],r['end'],r['absolute']):r for r in goals if r['plan']=='Fixed'}
gstats=collections.defaultdict(lambda:dict(rows=0,physical=0,accepted=0,actions=0,certificates=0,certified_true_error=0))
for r in goals:
 a=r['attempt'];o=old[(r['case'],r['end'],r['absolute'])];b=o['attempt'];s=gstats[r['plan']]
 s['rows']+=1;s['accepted']+=r['qualified'];s['physical']+=a['physical_qualified']
 assert r['rhs']==o['rhs'] and r['steps']==o['steps']
 assert a['physical_qualified']==b['physical_qualified'] and a['terminal_state']==b['terminal_state']
 if not a['physical_qualified']:
  assert not r['qualified'] and not a['certificates'] and a['enclosure'] is None
  continue
 ref=error_bounds(r['case'],r['end'],a['terminal_state'],r['weights'])
 if a['enclosure'] is not None:
  contains(a['enclosure'],ref);s['certified_true_error']+=1
 for c in a['certificates']:
  if c.get('work') is not None:
   contains(c['work']['signed_error'],ref);s['actions']+=c['work']['matrix_actions'];s['certificates']+=1
 if r['qualified']:assert max(map(abs,a['enclosure']))<=r['absolute']
assert len(direct)==8*sum(x['qualified'] for x in physical)
dstats=collections.defaultdict(lambda:dict(rows=0,qualified=0,zeroheap=0,actions=0))
for r in direct:
 s=dstats[r['candidate']];s['rows']+=1
 if r['qualified']:
  contains(r['work']['signed_error'],error_bounds(r['case'],r['end'],r['terminal'],r['weights']))
  assert req(r['allocation'])==0
  s['qualified']+=1;s['zeroheap']+=1;s['actions']+=r['work']['matrix_actions']
dump('INDEPENDENT_CHECKS.json',{'residual_rows':len(rows),'residual':stats,'native_replay_same_run':len(native),'goal':gstats,'direct':dstats,'physical':physical,'native_reference':'M2.25 identical deterministic full run; M2.24 independent represented-source reference'})
print('VERIFIED',len(rows),len(native),len(goals),len(direct),dict(stats),dict(gstats),dict(dstats))
