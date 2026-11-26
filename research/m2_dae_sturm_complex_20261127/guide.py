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
positions=['first','middle','last']
domains={
 'point':(['degree','position','route'],[[2,3,4],positions,['legacy','bounded','newton']]),
 'reconstruction':(['degree','position','candidate'],[[2,3,4],positions,['legacy','bounded_fixed','bounded_previous','newton_fixed','newton_previous']]),
 'sturm_epoch':(['degree','solver'],[[2,3,4],['Scalar5','Reduced5','BoundedSturm5','BoundedSturmNewton5']]),
 'complex_bound':(['freq','forced','route'],[[.5,4.,8.,12.],[False,True],['DictionaryDirected','Taylor48','ScaledRecurrence']]),
 'complex_epoch':(['freq','forced','solver'],[[.5,4.,8.],[False,True],['Reduced5','Reduced5PolynomialPicard','Reduced5ComplexDictionary','Reduced5ComplexScaled']])
}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='point':
  w=r['audit'];v=r['report'];return (*o,w['evaluations']['started'],w['rhs_requested'],w['legacy'].get('bounded_sturm_iterations',0),v['state_error_upper'],v['derivative_error_upper'],v['residual_upper'])
 if name=='reconstruction':
  a=r['audit'];return (*o,a['evaluations']['started'],a['rhs_requested'],a.get('bounded_sturm_iterations',0),r['original_g'],r['whole_x'],r['whole_z'])
 if name=='complex_bound':
  b=r['bounds'];return (*o,b['series_terms'],b['squarings'],b['candidate_error'])
 if name=='complex_epoch':return (*o,r['slabs'],r['max_physical_error'],r['outgoing'])
 return (*o,r['slabs'],r['max_physical_error'])
def good(name,r):
 if not r['qualified']:return False
 if name=='point':return r['report']['state_error_upper']<=1e-11 and r['report']['residual_upper']<=1e-11 and r['report']['derivative_error_upper']<=1e-6
 if name=='reconstruction':return r['whole_x']<=1e-3 and r['whole_z']<=1e-3
 if name.endswith('epoch'):return r['max_physical_error']<=1e-3
 return True
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Nonlinear scalar frozen exact dyadic primitive signed Sturm pseudo-remainder root count, bounded capacity, branch simple root, bisection versus interval Newton, caller-owned reconstruction and continuation; real source constant damped rotation polynomial forcing, existing dictionary complex phi numerical proposal independently validated by directed Taylor and scaled recurrence, original full slab certificate and endpoint min; compare actual real Picard. Preserve physical authority and refusal.',domain='numerical_analysis',max_routes=64)
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
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'bounded':'legacy','newton':'bounded','bounded_fixed':'legacy','bounded_previous':'bounded_fixed','newton_fixed':'bounded_fixed','newton_previous':'newton_fixed','BoundedSturmNewton5':'BoundedSturm5','Taylor48':'DictionaryDirected','ScaledRecurrence':'Taylor48','Reduced5ComplexScaled':'Reduced5ComplexDictionary'},'scope':'finite measured designs; no online promotion'})
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
  if qualified:archive.insert('/'.join(map(str,k[:-1])),float(nd),k)
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
 seq={'point':read('sturm.log','STURM_POINT'),'reconstruction':read('sturm.log','STURM_RECON'),'sturm_epoch':read('sturm.log','STURM_EPOCH'),'complex_bound':read('complex.log','COMPLEX_BOUND'),'complex_epoch':read('complex.log','COMPLEX_EPOCH')}
 out={name:replay(name,rows)for name,rows in seq.items()}
 out['scope']='finite measured replay, refused candidates preserved; work counters not comparable walltime; no promotion'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
