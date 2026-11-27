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
cases=['stable','stiff','nonlinear','vector4','vector8','mixed_rates','quadratic_flow']
adj=['stiff','mixed_rates','diag4','diag8']
domains={
 'engine':(['case','candidate'],[cases,['Legacy','Shared','SharedBounded','SharedFused']]),
 'taylor':(['case','candidate'],[adj,['Fixed12','Fixed20','Budgeted']]),
 'adjoint_ode':(['case','candidate'],[adj,['RodasAdjoint','DopriAdjoint','DopriForwardBlock','RodasForwardColumns']]),
 'forward_ode':(['case','candidate'],[['stiff','mixed_rates','quadratic_flow','diag4','diag8'],['Rodas5p','DopriLanding','DopriDense']])
}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='engine':return (*o,r['heap']['peak_delta'],r['heap']['end_delta'],req(r['setup']),r['max_physical_error'])
 if name=='taylor':return (*o,r['work']['matrix_actions'],r['work']['scalar_products'],r['work']['width_upper'])
 if name=='adjoint_ode':return (*o,r['gradient']['scalar_rhs_items'],r['gradient']['linear_solves'],r['error_upper'])
 return (*o,r['rhs'],r['steps'],r['error_upper'])
def good(name,r):
 if not r['qualified']:return False
 return {'engine':lambda:r['max_physical_error']<=.001,'taylor':lambda:r['work']['width_upper']<=1e-8,'adjoint_ode':lambda:r['error_upper']<=1e-8,'forward_ode':lambda:r['error_upper']<=1e-6}[name]()
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Actual reduced index-one DAE Engine ownership optimization, shared immutable callback captures, bounded initialization and fused source time Jacobian jet. Existing Rodas5p and Dopri5 on actual reduced-source problems, exact source constant Schur derivative and terminal physical goal. Reverse clock adjoint versus forward variational block/columns. Directed Taylor matrix action fixed12/fixed20 and existing causal adjoint budget adaptive order mutation, independently certified continuous gradient. Separate physical state authority and sensitivity scope. Finite measured selection, no online promotion.',domain='numerical_analysis',max_routes=64)
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
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'Shared':'Legacy','SharedBounded':'Shared','SharedFused':'SharedBounded','Fixed20':'Fixed12','Budgeted':'Fixed12','RodasAdjoint':'RodasForwardColumns','DopriAdjoint':'DopriForwardBlock','DopriDense':'DopriLanding'},'scope':'finite measured candidate design, no synthetic speed evidence'})
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
 ode=read('ode.log','ODE_COMPARE')
 er={(r['case'],r['candidate']):r['error_upper']for r in json.loads((P/'ODE_ACCURACY.json').read_text())}
 for r in ode:r['error_upper']=er[r['case'],r['candidate']]
 seq={'engine':read('engine.log','ENGINE_EPOCH'),'taylor':read('adjoint.log','ADJOINT_TAYLOR'),'adjoint_ode':read('adjoint.log','ADJOINT_ODE'),'forward_ode':ode}
 out={name:replay(name,rows)for name,rows in seq.items()}
 out['scope']='finite measured replay; numerical adjoint solver output checked by independent directed Taylor enclosure; forward ODE sampled error not slab qualification'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
