from pathlib import Path
import hashlib,json,zipfile,sys,random,itertools
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(name,obj):(P/name).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for name,d in m['files'].items():
  f=(H/name).resolve();assert H in f.parents and sha(f)==d,name
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites
from mathharness_general.ledger import Ledger,fresh_verify
from mathharness_general.poly_exact import parse_polynomial
axes=[
 {'Mass':['native mass RODAS','original residual'],'Reduced3':['Schur','RETRO Lagrange3'],'Reduced5':['Schur','RETRO Lagrange5'],'Prepared5':['PreparedRhs','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'WarpFolded':['WARP TensorPencil Folded','conditioned Horner']},
 {False:['generic exponential residual propagation'],True:['RETRO causal Green residual authority']},
 {'Native':['generic original DAE interval bootstrap'],'StructMax':['exact source reduction','native dense packet','exact Bernstein sup','conservation L1 norm'],'StructL1_4':['exact source reduction','native dense packet','four-cell Bernstein integral','conservation L1 norm']}
]
fields=['route','green','method']
if sys.argv[1]=='generate':
 r=explore('stiff Robertson mass conservation, positive simplex invariant, exact Schur elimination, signed column logarithmic L1 norm with nonphysical polynomial negative part, Bernstein defect integration and exact rational dense packet, RETRO causal Green, PreparedRhs, DictionaryCompanion, WARP stiff time pencil; select native plus offline certificate combination',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 candidates=[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[]))for k in itertools.product(*axes)]
 dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(m['files']),'harness_catalog_count':r['route_count']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('CONSERVATION','(-a*u+b*v*z)+(a*u-b*v*z-c*v*v)+c*v*v','0'),('COLUMN1','(-a-b*v)+(a+b*v)','0'),('COLUMN2','b*(z-v)-b*(z-v)-2*c*v','-2*c*v'),('RESET','2*u+v+2*z+v','2*(u+v+z)'),('BRANCH','-2048*z*(z-1)','2*z*(-1024*(z-1))')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite exact polynomial identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case,rt in sorted({(r['case'],r['rtol'])for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case and r['rtol']==rt]
  lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==36
  def obj(v):return (v['attempts'],v['rhs_proxy'],v['proof_cells'],v['bound'])
  def key(v):return (*obj(v),tuple(v[f]for f in fields))
  good=[v for v in rows if v['qualified']]
  best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(256):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts']+v['rhs_proxy']/1000.+v['proof_cells']/100.) if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['route']+'/'+v['method'],reward,v)
  uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
  out.append({'case':case,'rtol':rt,'selected':best,'qualified':len(good),'uct_best':uct,'uct_unique':len(visited),'uct_agrees':uct==best,'objective_agrees':(None if uct is None else obj(uct))==(None if best is None else obj(best)),'map_elites':{k:v[1]for k,v in archive.cells.items()},'pareto':pareto})
 dump('SELECTION.json',{'groups':out,'method':'fixed 1e-3 physical bound and empirical 2*rtol cap; native attempts, incomplete RHS proxy, proof cells, bound; measured lookup UCT, MAP-Elites, exhaustive; no total-cost equivalence'})
 for g in out:
  v=g['selected'];print(g['case'],g['rtol'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['objective_agrees'],None if v is None else {k:v[k]for k in fields+['attempts','rhs_proxy','proof_cells','bound','observed_error']})
