from pathlib import Path
import hashlib,itertools,json,random,sys,zipfile
P=Path(__file__).parent
H=Path.home()/'vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0'
Z=Path.home()/'Dropbox/mathharness-general-3.1.0.zip'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for n,d in m['files'].items():
  f=(H/n).resolve();assert H in f.parents and sha(f)==d,n
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites,ProposalBandit
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(n,marker):
 import gzip
 p=P/n
 if not p.exists(): p=Path(str(p)+'.gz')
 s=gzip.open(p,'rt') if p.suffix=='.gz' else p.open()
 with s:return [json.loads(l.split(marker+' ',1)[1])for l in s if marker+' {' in l]
grids=['box2_small','box3_small','sphere_odd','sphere128','sphere512','holdout_sphere153']
plans=['Fixed4IndependentUniform','Fixed4ChainMulti','Accepted2ChainMulti','Accepted4IndependentMulti','Accepted4ChainMulti','Accepted8ChainMulti','Accepted4StationaryMulti','Accepted4StationaryCost','Accepted4StationarySingle','Accepted4Pilot','Fixed4Pilot','Accepted2Pilot','Accepted8Pilot']
domains={
 'compact':(['case','goal','input','budget','candidate'],[grids,['mass','local'],['offset','signed'],[1e-10,1e-11],plans]),
 'ode':(['case','candidate'],[[1,3,32],['Legacy','Prevalidated']])}
def valid(name,k):return True
def axes_for(name):
 fields,axes=domains[name]; keys=[k for k in itertools.product(*axes) if valid(name,k)]
 return fields,keys
def obj(name,r):
 a=r['allocation'];alloc=a['alloc']+a['zeroed']+a['realloc']
 if name=='ode':return (alloc,r['counters']['rhs_calls'],r['max_error'])
 w=r['work']
 return (alloc,w['counters']['rhs_calls'],w['stage_solves'],r['cold_actions'],w['projections'],w['max_signed_error_upper'])
def admitted(name,r):return r['qualified']
def generate():
 r=explore('Mutate fixed four-cell local replay into actual accepted-step partition from one uninterrupted pilot with 2/4/8 macro cells, compare single vs multi magnitude and suffix-cost selection with original signed goal gate, preserve prefix and recompute suffix. Remove common constant mode from every directed boundary projection to avoid mass-goal cancellation. Compare independent Poisson actions, shared backward recurrence carrying scalar l1 truncation tail, and near-stationary original matrix residual with rigorous contraction bound, fallback. Reuse compact accepted checkpoints without stage row storage; actual allocation, native work, cold boundary work and exact reference selection.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name in domains:
  fields,keys=axes_for(name)
  tree=UCT(lambda p:sorted(set(k[len(p)] for k in keys if k[:len(p)]==tuple(p))) if len(p)<len(fields) else [])
  rng=random.Random(1729);seen=set();order=[]
  for _ in range(8192):
   k=tuple(tree.propose(rng));fresh=k not in seen
   assert k in keys
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==len(keys):break
  assert len(seen)==len(keys),(name,len(seen),len(keys))
  out[name]=[dict(zip(fields,k)) for k in order]
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'routes':r['route_count'],'actual_designs':out,'mutations':{'Fixed4ChainMulti':'Fixed4IndependentUniform','Accepted2ChainMulti':'Fixed4ChainMulti','Accepted4IndependentMulti':'Fixed4IndependentUniform','Accepted4ChainMulti':'Accepted4IndependentMulti','Accepted8ChainMulti':'Accepted4ChainMulti','Accepted4StationaryMulti':'Accepted4ChainMulti','Accepted4StationaryCost':'Accepted4StationaryMulti','Accepted4StationarySingle':'Accepted4StationaryMulti','Accepted4Pilot':'Accepted4StationaryMulti','Fixed4Pilot':'Fixed4ChainMulti','Accepted2Pilot':'Accepted2ChainMulti','Accepted8Pilot':'Accepted8ChainMulti','Prevalidated':'Legacy'}})
 print('GENERATED',len(m['files']),r['route_count'],{k:len(v)for k,v in out.items()})
def select():
 seq=read('portfolio.log','COMPACT_RUN')
 cold={}
 def key(r):return (r['case'],r['dual'],tuple(r['weights']),tuple(r['times']))
 for r in seq:cold[key(r)]=max(cold.get(key(r),0),r['work']['adjoint_actions'])
 for r in seq:r['cold_actions']=cold[key(r)]
 data={'compact':seq,'ode':read('ode.log','CHECKPOINT_ODE')}
 out={}
 for name,seq in data.items():
  fields,keys=axes_for(name);rows={tuple(r[f]for f in fields):r for r in seq}
  assert len(seq)==len(rows)==len(keys) and set(rows)==set(keys),(name,len(seq),len(keys),set(keys)-set(rows))
  tree=UCT(lambda p:sorted(set(k[len(p)]for k in keys if k[:len(p)]==tuple(p)))if len(p)<len(fields) else [])
  bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen=set();rng=random.Random(97531);mut=0
  def front(g,subset):
   ks=[k for k in subset if k[:-1]==g and admitted(name,rows[k])]
   return [k for k in ks if not any(obj(name,rows[q])!=obj(name,rows[k]) and all(x<=y for x,y in zip(obj(name,rows[q]),obj(name,rows[k])))for q in ks)]
  for _ in range(2048):
   arm=bandit.pick(rng)
   mutable=sorted(p for p in seen if any(q[:-1]==p[:-1] and q!=p for q in keys))
   if arm=='mutation' and mutable:
    p=rng.choice(mutable);k=rng.choice([q for q in keys if q[:-1]==p[:-1]and q!=p]);mut+=1
   else:k=tuple(tree.propose(rng))
   fresh=k not in seen;seen.add(k)
   ok=admitted(name,rows[k]);reward=float(fresh and ok and k in front(k[:-1],seen))
   bandit.update(arm,reward);tree.backup(k,reward)
   if ok:archive.insert('/'.join(map(str,k[:-1])),reward,k)
  strata=[]
  for g in sorted({k[:-1]for k in keys}):
   f=front(g,keys)
   strata.append({'group':g,'pareto':[{'candidate':k[-1],'objectives':obj(name,rows[k])}for k in f],'covered':len(set(f)&seen),'qualified':sum(admitted(name,rows[k]) for k in keys if k[:-1]==g)})
  out[name]={'rows':len(rows),'unique':len(seen),'mutations':mut,'pareto_count':sum(len(s['pareto'])for s in strata),'pareto_covered':sum(s['covered']for s in strata),'strata':strata}
 dump('SELECTION.json',out)
 print('SELECTED',{k:{a:b for a,b in v.items()if a!='strata'}for k,v in out.items()})
if sys.argv[1]=='generate':generate()
else:select()
