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
grids=['box2_small','box3_small','sphere_odd','sphere128','sphere512']
clocks=['affine','quadratic','ratio_third','long','block4','zero_q','coupled_clock','holdout_block6','zero_p','fast_rate']
plans=['SharedL1','SharedCentered','SharedCenteredIntersect']
domains={
 'policy':(['case','absolute','candidate'],[['long','block4','zero_p','fast_rate'],[1e-8,1e-10],['PolynomialBlockSplit','PolynomialBlockDecay1','PolynomialBlockDecay4','PolynomialBlockRational4']]),
 'tube':(['case','candidate'],[clocks,['Flat','BlockSplit','BlockSplitDecay1','BlockSplitDecay4','BlockSplitRational4']]),
 'dense':(['case','log','candidate'],[clocks,['TinyAdaptive','TinyRootReduced'],['Termwise','Factored','Intersection']]),
 'spatial':(['case','goal','route','pairs','candidate'],[grids,['mass','local'],['ModeResidual','PoissonWideDecay'],[2,4],plans]),
 'budget':(['case','goal','route','pairs','budget','candidate'],[grids,['mass','local'],['ModeResidual','PoissonWideDecay'],[2,4],[1e-8,1e-10],plans])}
def valid(name,k):return True
def axes_for(name):
 fields,axes=domains[name]; keys=[k for k in itertools.product(*axes) if valid(name,k)]
 return fields,keys
def obj(name,r):
 a=r['allocation'];alloc=a['alloc']+a['zeroed']+a['realloc']
 if name=='policy':
  q=r['attempt'];w=q.get('polynomial_work') or {};g=w.get('goal') or {}
  return (alloc,a['requested_bytes'],r['rhs'],sum((c.get('work') or {}).get('matrix_actions',0) for c in q['certificates']),g.get('series_terms',0),max(map(abs,q['enclosure'])))
 if name=='tube':return (alloc,a['requested_bytes'],r['rhs'],r['terminal_bound'],r['whole_bound'])
 if name=='dense':return (alloc,r['work']['log_terms'],r['work']['square_roots'],r['work']['max_radius'])
 if name=='spatial':return (alloc,a['requested_bytes'],r['work']['actions'],r['work']['max_goal_width'],r['work']['max_gradient_width'])
 w=r['work'];return (alloc,w['work']['actions'],w['max_signed_error_upper'])
def admitted(name,r):return r['qualified']
def generate():
 r=explore('Improve split triangular residual carry by damping incoming p error and integrating its response against nonnegative clock mass, distinguish whole slab peak from endpoint radius. Compare whole exponential, four dyadic subcell exponential, four subcell rational bound; reuse exact source and Bernstein integrals. Activate existing verified dense transaction staging with actual retained centers and exact source state at each represented request time, independent of rounded theta. Mutate shared spatial adjoint certificate by separating exact constant mode and centering initial data; intersect with old bound and add signed budget gate. Use actual native spatial outputs, zeroheap, independent rational references.',domain='numerical_analysis',max_routes=64)
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
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'routes':r['route_count'],'actual_designs':out,'mutations':{'BlockSplitDecay1':'BlockSplit','BlockSplitDecay4':'BlockSplitDecay1','BlockSplitRational4':'BlockSplitDecay4','Factored':'Termwise','Intersection':'Factored','SharedCentered':'SharedL1','SharedCenteredIntersect':'SharedCentered'}})
 print('GENERATED',len(m['files']),r['route_count'],{k:len(v)for k,v in out.items()})
def select():
 data={'policy':read('policy.log','DECAY_POLICY'),'tube':read('clock.log','DECAY_RUN'),'dense':read('clock.log','CLOCK_DENSE'),'spatial':read('spatial.log','CENTER_BATCH'),'budget':read('spatial.log','CENTER_BUDGET')}
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
