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
cases=['stiff','mixed_rates','diag4','diag8','chain_weak','chain_stiff']
candidates=['Legacy','ReuseLu','Triangular','Reciprocal','GoalUniform','GoalAdjoint']
domains={'stage':(['case','candidate'],[cases,candidates]),
 'forward':(['case','candidate'],[cases,['Rodas5p','DopriLanding','DopriDense']])}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='stage':return (*o,r['point_calls'],r['factorizations'],r['rhs'],r['linear_solves'],r['max_physical_error'])
 return (*o,r['rhs'],r['steps'],r['error_upper'])
def good(name,r):return r['qualified'] and (r['max_physical_error']<=.001 if name=='stage' else r['error_upper']<=1e-6)
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Native eight-stage RODAS5P with dense output on source-proved homogeneous autonomous constant triangular reduced index-1 DAE. Keep original RHS, original tableau, original adaptive controller, physical slab proof. Compare protected LU, existing reusable LU stage client, direct triangular substitution, diagonal reciprocal mutation, diagonal/Jacobi/direct proposals gated by exact original stage residual and full state bound plus either uniform goal bound or signed backward block adjoint. Compare actual DOPRI5 landing versus dense observations for same source, no physical authority transfer. Measure actual callback work, factor/solve counts, allocation, whole epoch timing, independent exact stage error and analytic trajectory. No online/default promotion.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=set();order=[];goal=len(list(itertools.product(*axes)))
  for _ in range(4096):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in order],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'ReuseLu':'Legacy','Triangular':'ReuseLu','Reciprocal':'Triangular','GoalUniform':'Reciprocal','GoalAdjoint':'GoalUniform','DopriDense':'DopriLanding'},'scope':'finite measured design, refused candidate retained as refusal'})
 print('MathHarness',len(m['files']),r['route_count'],[(k,len(v['uct_proposals']))for k,v in out.items()])
def replay(name,seq):
 fields,axes=domains[name];rows={key(fields,r):r for r in seq}
 assert len(rows)==len(seq)==len(list(itertools.product(*axes))) and set(rows)==set(itertools.product(*axes)),(name,len(rows),len(seq))
 rng=random.Random(97531);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};proposals={'structure':0,'mutation':0}
 for _ in range(2048):
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
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'])for r in rs),'bytes':sum((r['allocation'])['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 forward=read('portfolio.log','NATIVE_ODE')
 er={(r['case'],r['candidate']):r['error_upper']for r in json.loads((P/'ODE_ACCURACY.json').read_text())}
 for r in forward:r['error_upper']=er[r['case'],r['candidate']]
 seq={'stage':read('portfolio.log','NATIVE_STAGE'),'forward':forward}
 out={name:replay(name,rows)for name,rows in seq.items()}
 ts={(r['case'],r['candidate']):r for r in read('portfolio.log','NATIVE_TIMING')}
 groups=[]
 for case in cases:
  a=[r for r in seq['stage']if r['case']==case and good('stage',r)]
  objective=lambda r:(*obj('stage',r),ts[(case,r['candidate'])]['median_ns'])
  groups.append({'case':case,'pareto':[{'candidate':r['candidate'],'objectives':objective(r)}for r in a if not any(dom(objective(q),objective(r))for q in a)]})
 dump('TIMING_COMPARE.json',{'rows':list(ts.values()),'pareto':groups,'scope':'whole actual certified compact epoch + drop, restoration excluded'})
 out['scope']='finite measured replay; stage-target certificate separate from LTE and physical IVP; no default promotion'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
