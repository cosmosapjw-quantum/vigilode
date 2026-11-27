from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,math
P=Path(__file__).parent
PREV=P.parent/'m2_fast_residual_goal_escalation_20261127'
def read(p,marker):
 if not p.exists():p=Path(str(p)+'.gz')
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def exp_bounds(t):
 assert 0<=t<=8
 term=s=Q(1)
 for k in range(1,151):term*=t/k;s+=term
 tail=term*t/151/(1-t/152)
 return s,s+tail
def contains(iv,bounds):
 lo,hi=map(Q,iv)
 assert lo<=bounds[0]<=bounds[1]<=hi,(iv,tuple(map(float,bounds)))
# Independently evaluate exact rational original directed-edge residual.
grids={r['case']:r['rows'] for r in read(P/'residual.log','COMPACT_GRID')}
inputs={(r['case'],r['scenario']):r for r in read(P/'residual.log','COMPACT_INPUT')}
rows=read(P/'residual.log','COMPACT_CERT')
assert len(grids)==8 and len(inputs)==40 and len(rows)==160
exact={}
for k,r in inputs.items():
 x=list(map(Q,r['x']));b=list(map(Q,r['b']));z=Q(r['shift'])
 d=[v-z*u for v,u in zip(b,x)]
 for i,rr in enumerate(grids[k[0]]):
  for j,a in rr:
   if j!=i and a!=0:
    p=Q(a)*x[i];d[i]-=p;d[j]+=p
 rho=sum(map(abs,d));exact[k]=(rho,z,sum(map(abs,b)))
summary=collections.defaultdict(lambda:dict(rows=0,admitted=0,fallbacks=0,zeroheap=0))
for r in rows:
 k=(r['case'],r['scenario']);rho,z,b=exact[k];a=r['receipt']
 assert rho<=Q(a['residual_l1']) and rho/z<=Q(a['error_l1']) and b<=Q(a['rhs_l1']),(k,r['candidate'],float(rho),a)
 assert req(r['allocation'])==req(r['prepare'])==0
 s=summary[r['candidate']];s['rows']+=1;s['admitted']+=a['admitted'];s['fallbacks']+=a['fallbacks'];s['zeroheap']+=1
# Same exact full driver rows previously independently bounded in .24.
native=read(P/'native.log','COMPACT_IVP')
prior={r['case']:r for r in read(PREV/'native.log','FAST_IVP') if r['candidate']=='Hybrid'}
assert len(native)==21
for r in native:
 assert r['qualified'] and r['error_upper']<=1e-6
 p=prior[r['case']]
 assert p['qualified'] and p['run']==r['run'],(r['case'],r['candidate'])
# Exact reference for growth; non-growth must preserve the previous fixed policy.
goals=read(P/'goal.log','STRUCT_GOAL')
assert len(goals)==48
old={(r['case'],r['end'],r['absolute']):r for r in read(PREV/'goal.log','ESC_GOAL') if r['candidate']=='Fixed'}
stats=collections.defaultdict(lambda:dict(rows=0,accepted=0,growth_accepted=0,structured=0,actions=0,refusals=0))
growth=[]
for r in goals:
 a=r['attempt'];key=(r['case'],r['end'],r['absolute']);o=old[key]['attempts'][-1];s=stats[r['plan']]
 s['rows']+=1;s['accepted']+=r['qualified'];s['structured']+=a.get('structured_work') is not None;s['refusals']+=a.get('structured_refusal') is not None
 s['actions']+=sum((c.get('work') or {}).get('matrix_actions',0)for c in a['certificates'])+(a.get('structured_work') or {}).get('matrix_actions',0)
 assert r['rhs']==old[key]['total_rhs'] and r['steps']==old[key]['total_steps']
 assert a['physical_qualified']==o['physical_qualified']
 if not a['physical_qualified']:
  assert not r['qualified'] and a.get('structured_work') is None
  continue
 assert a['terminal_state']==o['terminal_state']
 if r['case']=='growth2':
  lo,hi=exp_bounds(8*Q(r['end']));v=Q(a['terminal_state'][0]);bounds=(lo/1024-v,hi/1024-v)
  contains(a['enclosure'],bounds)
  s['growth_accepted']+=r['qualified']
  growth.append({'plan':r['plan'],'end':r['end'],'budget':r['absolute'],'accepted':r['qualified'],'true_error':list(map(float,bounds)),'enclosure':a['enclosure'],'actions':(a.get('structured_work')or{}).get('matrix_actions',0),'secant_calls':len(a['certificates'])})
 else:
  assert a['enclosure']==o['enclosure'] and a['accepted']==o['accepted'] and a['certificates']==o['certificates']
 if r['qualified']:assert max(map(abs,a['enclosure']))<=r['absolute']
dump('INDEPENDENT_CHECKS.json',{'exact_rational_residual_bounds':len(rows),'zeroheap_prepare_evaluate':len(rows),'residual':summary,'native_replay_same_run':len(native),'native_reference':'previous .24 exact independent source reference via identical full deterministic run','growth_exact_enclosures':len(growth),'goals':stats,'growth':growth})
print('VERIFIED',len(rows),len(native),len(growth),dict(summary),dict(stats))
