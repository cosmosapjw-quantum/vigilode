from pathlib import Path
import gzip,hashlib,itertools,json,random,statistics,sys,zipfile
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for n,d in m['files'].items():
  f=(H/n).resolve();assert H in f.parents and sha(f)==d,n
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites,ProposalBandit
# Selection on measured rows only. No protected holdout or online route promotion.
routes=['Bernstein','LegendreInterval','LegendreCentered','HybridInterval','HybridCentered']
solvers=['Reduced5','Prepared5','BoundedPrepared5','Scalar5','Affine5']
certs=['Max','Integral4'];positions=['first','middle','last']
recon=['existing','bounded_fixed','bounded_previous']
epochs=['Reduced5','Prepared5','BoundedPrepared5','Reduced5Legendre','BoundedPreparedLegendre5']
domains={
 'spectral_native':(['case','solver','position','route'],[['quadratic','quartic'],['Mass','Reduced5','Prepared5'],positions,routes]),
 'spectral_manufactured':(['case','solver','position','route'],[['oscillatory'],['manufactured'],['one'],routes]),
 'reconstruction':(['solver','cert','position','candidate'],[solvers,certs,positions,recon]),
 'epoch':(['case','solver'],[['quadratic','quartic'],epochs]),
 'atlas':(['cert','solver'],[certs,solvers])
}
def read(f,marker):
 path=P/f
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1]) for l in t.splitlines() if l.startswith(marker+' {') or (l.startswith('test ') and ' ... '+marker+' {' in l)]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name.startswith('spectral'):
  b=r['bounds'];return (*o,b['spectral_products'],b['linearization_entries'],b['original_g_upper'],b['preconditioned_g_upper'],b['differential_defect'],r['whole_x'],r['whole_z'])
 if name=='reconstruction':
  a=r['audit'];return (*o,a['evaluations']['started'],a['rhs_requested'],a.get('bounded_residual_terms',0)+sum(w['prepared_residual_terms']for w in r['work']),sum(w['newton_steps']for w in r['work']),r['original_g'],r['whole_x'],r['whole_z'])
 if name=='epoch':
  return (*o,r['audit']['point_projection']['rhs_requested'],r['dictionary_center_products'],r['max_physical_error'])
 return (*o,r['slabs'])
def good(name,r):
 # Match actual resource-baseline policy.global_tol=1e-3; no relaxed gate.
 if name=='atlas':return r['stop']=='Horizon'
 if not r['qualified']:return False
 if name=='epoch':return r['max_physical_error'] is not None and r['max_physical_error']<=1e-3
 return r['whole_x'] is not None and r['whole_z'] is not None and r['whole_x']<=1e-3 and r['whole_z']<=1e-3
def dominates(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dominates(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Verified DAE scalar midpoint reciprocal batch with original directed residual and strict row dominance; rounded local self-map regrouping with bounded ULP expansion; existing RETRO shifted Legendre linearization table product with interval coefficient correction and hybrid Bernstein/Legendre full-slab residual; scalar reconstruction node seed continuation and actual native callbacks/epoch/atlas. No altered source/tolerance.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=[];goal=len(list(itertools.product(*axes)))
  for _ in range(4096):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in seen],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'LegendreInterval':'Bernstein','LegendreCentered':'LegendreInterval','HybridInterval':'LegendreInterval','HybridCentered':'LegendreCentered','bounded_fixed':'existing','bounded_previous':'bounded_fixed','BoundedPrepared5':'Prepared5','Reduced5Legendre':'Reduced5','BoundedPreparedLegendre5':'BoundedPrepared5'},'revived_modules':['Dictionary::legendre_product','Dictionary::products','ScalarReuseProjection','IntervalEvalWorkspace','ScalarReconstructionWorkspace'],'scope':'actual measured candidates; per-stratum Pareto; no default promotion'})
 print('MathHarness',len(m['files']),r['route_count'],[(k,len(v['uct_proposals']))for k,v in out.items()])
def replay(name,seq):
 fields,axes=domains[name];rows={key(fields,r):r for r in seq}
 assert len(rows)==len(seq)==len(list(itertools.product(*axes))) and set(rows)==set(itertools.product(*axes)),(name,len(rows),len(seq))
 rng=random.Random(97531);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};proposals={'structure':0,'mutation':0}
 for _ in range(1024):
  arm=bandit.pick(rng);proposals[arm]+=1
  if arm=='mutation'and seen:
   k=rng.choice(list(seen));k=k[:-1]+(rng.choice([v for v in axes[-1]if v!=k[-1]]),)
  else:k=tuple(tree.propose(rng))
  fresh=k not in seen;seen[k]=rows[k];qualified=good(name,rows[k]);nd=qualified and k in front(name,seen,k[:-1]);reward=fresh and nd
  bandit.update(arm,reward);tree.backup(k,float(reward))
  if qualified:archive.insert('/'.join(k[:-1]),float(nd),k)
 groups=[]
 for g in sorted({k[:-1]for k in rows}):
  f=front(name,rows,g)
  groups.append({'group':g,'qualified':sum(good(name,v)for k,v in rows.items()if k[:-1]==g),'pareto':[dict(zip(fields,k),objectives=obj(name,rows[k]))for k in f],'resource_first':min(f,key=lambda k:(obj(name,rows[k]),k))[-1]if f else None,'covered':len(set(f)&seen.keys())})
 candidates={}
 for route in axes[-1]:
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'])for r in rs),'bytes':sum(r['allocation']['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 sp=read('spectral.log','LEGENDRE')
 seq={'spectral_native':[r for r in sp if r['solver']!='manufactured'],'spectral_manufactured':[r for r in sp if r['solver']=='manufactured'],'reconstruction':read('prepared.log','PREPARED_RECON'),'epoch':read('spectral.log','SPECTRAL_EPOCH'),'atlas':read('prepared.log','PREPARED_ATLAS')}
 out={name:replay(name,rows)for name,rows in seq.items()}
 # Verify hybrid bound is componentwise no wider than original Bernstein;
 # preserve refusals, no claim of total arithmetic count from spectral counters.
 b={key(domains['spectral_native'][0],r)[:-1]:r for r in sp if r['route']=='Bernstein'}
 comparisons={}
 for route in routes[1:]:
  rs=[r for r in sp if r['route']==route]
  tighter={k:0 for k in ('original_g_upper','preconditioned_g_upper','differential_defect')}
  wider=dict(tighter)
  for r in rs:
   a=b[key(domains['spectral_native'][0],r)[:-1]]
   for k in tighter:
    tighter[k]+=r['bounds'][k]<a['bounds'][k];wider[k]+=r['bounds'][k]>a['bounds'][k]
   if route.startswith('Hybrid'):assert all(r['bounds'][k]<=a['bounds'][k]for k in tighter)
  comparisons[route]={'count':len(rs),'tighter':tighter,'wider':wider}
 out['spectral_comparison']=comparisons
 out['scope']='finite measured-table selection; product counters not total FLOPs; no speed/default/holdout claim'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'} if isinstance(v,dict) else v)
