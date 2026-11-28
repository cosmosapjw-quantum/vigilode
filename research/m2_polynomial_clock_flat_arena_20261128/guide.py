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
grids=['sphere_odd','box2','box3','sphere128','sphere512','box2_strong','sphere1024','holdout_box3','holdout_sphere2048']
clocks=['affine','quadratic','ratio_third','long','block4','zero_q','coupled_clock']
tubes=['Flat','Mean4','Mean8','Hybrid4']
policy=['Fixed','EconomySigned','PolynomialFlatTermwise','PolynomialFlatFactored','PolynomialMean4','PolynomialMean8','PolynomialHybrid4']
domains={
 'spatial':(['case','schedule','partial','plan','candidate'],[grids,[0,1],[False,True],['Hybrid','RingLocalHybrid'],['RowLegacy','RowPrevalidated','FlatLegacy','FlatPrevalidated']]),
 'ode':(['case','candidate'],[[1,3,32],['Legacy','Prevalidated']]),
 'tube':(['case','candidate'],[clocks,tubes]),
 'direct':(['case','log','candidate'],[clocks,['TinyAdaptive','TinyRootReduced'],['Termwise','Factored','Intersection']]),
 'gradient':(['case','candidate'],[clocks,['Whole','Sub4','Contracted16']]),
 'policy':(['case','absolute','candidate'],[clocks,[1e-8,1e-10],policy])}
def valid(name,k):
 if name=='tube':return k[0]!='block4' or k[-1]=='Flat'
 if name=='policy':return k[0]!='block4' or k[-1] not in ['PolynomialMean4','PolynomialMean8','PolynomialHybrid4']
 return True
def axes_for(name):
 fields,axes=domains[name]; keys=[k for k in itertools.product(*axes) if valid(name,k)]
 return fields,keys
def obj(name,r):
 a=r['allocation'];alloc=a['alloc']+a['zeroed']+a['realloc']
 if name=='spatial':return (alloc,a['requested_bytes'],r['setup']['requested_bytes'],r['setup']['alloc']+r['setup']['zeroed']+r['setup']['realloc'],r['median_ns'])
 if name=='ode':return (alloc,a['requested_bytes'],r['counters']['rhs_calls'],r['max_error'])
 if name=='gradient':return (alloc,r['actions'],r['width'])
 if name=='tube':return (alloc,a['requested_bytes'],r['rhs'],r['bound'])
 if name=='policy':
  q=r['attempt'];w=q.get('polynomial_work') or {};g=w.get('goal') or {}
  return (alloc,a['requested_bytes'],r['rhs'],sum((c.get('work') or {}).get('matrix_actions',0) for c in q['certificates']),g.get('series_terms',0),g.get('square_roots',0),max(map(abs,q['enclosure'])))
 w=r['work'];g=w['goal'];return (alloc,g['series_terms'],g['square_roots'],w['gradient_width'],g['width_upper'],w['measure_width'])
def admitted(name,r):return r['qualified']
def generate():
 r=explore('Generalize source-proved triangular clock to degree-two nonnegative polynomial rate and exact binary64 proportional drift via integer significand product equality; mutate independent moment sum to factored and intersected measure, fuse terminal goal and exact gradient. Reuse local branch certificate and weighted residual tube, vary weights and Bernstein/Legendre bound. Mutate row output arena into bounded contiguous observation arena using native driver; compare legacy/prevalidated controller, packed and ring-local residuals, balanced paired timing, zeroheap, holdout grids and partial prefixes.',domain='numerical_analysis',max_routes=64)
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
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'routes':r['route_count'],'actual_designs':out,'mutations':{'Factored':'Termwise','Intersection':'Factored','Mean8':'Mean4','Hybrid4':'Mean4','PolynomialFlatFactored':'PolynomialFlatTermwise','PolynomialMean4':'PolynomialFlatFactored','PolynomialMean8':'PolynomialMean4','PolynomialHybrid4':'PolynomialMean4','FlatLegacy':'RowLegacy','FlatPrevalidated':'RowPrevalidated'}})
 print('GENERATED',len(m['files']),r['route_count'],{k:len(v)for k,v in out.items()})
def select():
 data={'spatial':read('arena.log','FLAT_IVP'),'ode':read('ode.log','FLAT_ODE'),'tube':read('clock.log','POLY_TUBE'),'direct':read('clock.log','POLY_DIRECT'),'gradient':read('clock.log','POLY_GRAD'),'policy':read('clock.log','POLY_POLICY')}
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
