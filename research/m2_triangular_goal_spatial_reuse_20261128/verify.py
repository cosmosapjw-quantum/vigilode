from pathlib import Path
from fractions import Fraction as Q
from functools import lru_cache
import gzip,json,collections
P=Path(__file__).parent
PREV=P.parent/'m2_weighted_clock_adaptive_20261127'
SPREV=P.parent/'m2_ring_local_signed_20261127'
def read(p,marker):
 if not p.exists():p=Path(str(p)+'.gz')
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1])for l in f if marker+' {' in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def contains(iv,bounds):
 lo,hi=map(Q,iv)
 assert lo<=bounds[0]<=bounds[1]<=hi,(iv,tuple(map(float,bounds)))
def unit_log(q):
 s=(q-1)/(q+1);assert 0<=s<=Q(1,3)
 v=2*sum((s**(2*k+1)/(2*k+1) for k in range(80)),Q(0))
 return v,v+2*s**161/(161*(1-s*s))
@lru_cache(None)
def log_bounds(q):
 assert q>=1
 x=q-1
 if x<=Q(1,2**40):return x-x*x/2,x
 k=0
 while q>2:q/=2;k+=1
 l,h=unit_log(q);a,b=unit_log(Q(2))
 return l+k*a,h+k*b
def ref(case,start,end,initial,terminal,weights):
 n=len(initial); t=(Q(end)-Q(start))*(Q(end)+Q(start))/2
 vals=[]
 for i in range(0,n,2):
  a=Q(1 if case!='clock8' else i//2+1)
  q0=Q(initial[i+1]);p0=Q(initial[i])-q0 if case=='coupled_clock' else Q(initial[i])
  d=1+a*p0*t;p=p0/d;l,h=log_bounds(d);q=(q0+l/a,q0+h/a)
  u=(p+q[0],p+q[1])if case=='coupled_clock' else (p,p)
  vals.extend((u,q))
 lo=hi=Q(0)
 for (a,b),x,c in zip(vals*2,terminal,weights):
  c=Q(c);x=Q(x);z=(c*(a-x),c*(b-x))
  lo+=min(z);hi+=max(z)
 return lo,hi
def initial(case):return [0.3125,0.0625] if case=='coupled_clock' else [0.25,0.0625]*(4 if case=='clock8' else 1)
goals=read(P/'goal.log','TRI_GOAL');direct=read(P/'goal.log','TRI_DIRECT');arith=read(P/'arithmetic.log','TRI_ARITH');spatial=read(P/'spatial.log','REUSE_IVP')
assert (len(goals),len(direct),len(arith),len(spatial))==(99,20,20,84),(len(goals),len(direct),len(arith),len(spatial))
gs=collections.defaultdict(lambda:dict(rows=0,physical=0,accepted=0,actions=0,terms=0,roots=0,direct=0,refusals=0))
fixed={(r['case'],r['end'],r['absolute']):r for r in goals if r['plan']=='Fixed'}
prior={(r['case'],r['end'],r['absolute'],r['plan']):r for r in read(PREV/'goal.log','ADAPTIVE_GOAL')}
refusals=[]
for r in goals:
 k=(r['case'],r['end'],r['absolute']);a=r['attempt'];old=fixed[k];s=gs[r['plan']]
 s['rows']+=1;s['physical']+=a['physical_qualified'];s['accepted']+=a['accepted']
 assert r['rhs']==old['rhs'] and r['steps']==old['steps']
 if (o:=prior.get((*k,r['plan']))) is not None:
  assert o['attempt']==a and o['qualified']==r['qualified'] and o['rhs']==r['rhs']
 if old['attempt']['physical_qualified']:assert a['terminal_state']==old['attempt']['terminal_state']
 if not a['physical_qualified']:assert not a['accepted'] and a['enclosure'] is None;continue
 truth=ref(r['case'],0.,r['end'],initial(r['case']),a['terminal_state'],r['weights'])
 if a['enclosure'] is not None:contains(a['enclosure'],truth)
 if a['accepted']:assert max(map(abs,a['enclosure']))<=r['absolute']
 else:refusals.append({'case':r['case'],'end':r['end'],'absolute':r['absolute'],'plan':r['plan'],'true_error':list(map(float,truth)),'enclosure':a['enclosure']})
 for c in a['certificates']:
  if c.get('work'):
   contains(c['work']['signed_error'],truth);s['actions']+=c['work']['matrix_actions']
 if (w:=a.get('triangular_work')) is not None:
  contains(w['signed_error'],truth);s['direct']+=1;s['terms']+=w['series_terms'];s['roots']+=w['square_roots']
 s['refusals']+=a.get('triangular_refusal') is not None
ds=collections.defaultdict(lambda:dict(rows=0,zeroheap=0,terms=0,roots=0))
for r in direct+arith:
 s=ds[r['candidate']];w=r['work'];assert req(r['allocation'])==0
 truth=ref(r['case'],r.get('start',0.),r['end'],r.get('initial',initial(r['case'])),r['terminal'],r['weights'])
 contains(w['signed_error'],truth)
 s['rows']+=1;s['zeroheap']+=1;s['terms']+=w['series_terms'];s['roots']+=w['square_roots']
sp={(r['case'],r['trial'],r['plan'],r['candidate']):r for r in spatial};assert len(sp)==84
prev={(r['case'],r['candidate']):r for r in read(SPREV/'native.log','COMPACT_IVP')}
ss=collections.defaultdict(lambda:dict(rows=0,requests=0,requested_bytes=0))
for r in spatial:
 k=(r['case'],r['trial'],r['plan']);o=sp[(*k,'Fresh')]
 assert r['qualified'] and r['run']==o['run']
 if r['trial']==0:assert r['run']==prev[(r['case'],r['plan'])]['run']
 if r['candidate']=='Reusable':assert r['allocation']['requested_bytes']<o['allocation']['requested_bytes']
 s=ss[r['candidate']];s['rows']+=1;s['requests']+=req(r['allocation']);s['requested_bytes']+=r['allocation']['requested_bytes']
assert len(read(P/'arithmetic.log','TRI_NEGATIVE'))==1
dump('INDEPENDENT_CHECKS.json',{'goal_rows':len(goals),'goals':gs,'direct_and_arithmetic':ds,'spatial':ss,'spatial_equal_pairs':42,'physical_goal_refusals':refusals,'reference':'exact binary64 fractions; rational range scaling by powers of two and positive atanh series with geometric tail (small x alternating bounds); same physical state, spatial deterministic comparison with M2.26 independent source baseline for trial0'})
print('VERIFIED',len(goals),len(direct),len(arith),len(spatial))
print('GOAL',dict(gs));print('DIRECT/ARITH',dict(ds));print('SPATIAL',dict(ss))
