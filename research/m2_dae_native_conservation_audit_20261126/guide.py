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
solvers={'Mass':['native mass RODAS'],'Reduced3':['Schur reduction','RETRO Lagrange3'],'Reduced5':['Schur reduction','RETRO Lagrange5'],'Prepared5':['PreparedRhs','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'WarpFolded':['WARP TensorPencil Folded','conditioned Horner']}
certs={'Max':['affine lift','directed Bernstein sup','L1 lognorm'],'Integral4':['affine lift','four-cell directed Bernstein integral','L1 lognorm'],'Integral16':['affine lift','16-cell directed Bernstein integral','L1 lognorm']}
orders={'Shared':['RETRO Causal shared native slab','DictionaryHint/PiecewiseProbe'],'Correlated':['shared native slab','Schur guard difference','DictionaryHint/PiecewiseProbe']}
r=explore('source-owned stiff conserved simplex DAE, signed reduced L1 logarithmic norm and interval Bernstein integral, negative polynomial overshoot, native mass or Schur PreparedRhs ScalarRoot DictionaryCompanion and WARP pencil, shared or correlated exact-clock hybrid reset with L1 transport',domain='numerical_analysis',max_routes=64)
dump('HARNESS_ROUTES.json',r)
dump('CANDIDATES.json',{'manifest_files':len(m['files']),'harness_catalog_count':r['route_count'],'native':[{'solver':s,'cert':c,'modules':solvers[s]+certs[c]}for s,c in itertools.product(solvers,certs)],'hybrid':[{'solver':s,'cert':c,'order':o,'modules':solvers[s]+certs[c]+orders[o]}for s,c,o in itertools.product(list(solvers)[:5],certs,orders)]})
l=Ledger(P/'exact_ledger.json')
for cid,lhs,rhs in [('LIFT','u+v+(1-u-v)','1'),('COLUMN1','(-a-b*v)+(a+b*v)','0'),('COLUMN2','b*(z-v)-b*(z-v)-2*c*v','-2*c*v'),('RESET','2*u+v+2*z+v','2*(u+v+z)'),('DEFECT-SUM','(du-h*f)+(dv+h*f+h*c*v*v)','du+dv+h*c*v*v')]:
 l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite polynomial identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
 assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
data=json.load(open(P/'AUDIT.json'));out=[]
def choose(name,rows,axes,fields,obj):
 lookup={tuple(r[f]for f in fields):r for r in rows}
 assert len(lookup)==len(list(itertools.product(*axes)))
 good=[r for r in rows if r['qualified']]
 def key(r):return (*obj(r),tuple(r[f]for f in fields))
 best=min(good,key=key,default=None);tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
 archive=MapElites();rng=random.Random(1729);visited=set()
 for _ in range(256):
  path=tree.propose(rng);r=lookup[path];visited.add(path)
  reward=1./(1.+r['attempts']+sum(obj(r)[1:])/1000.)if r['qualified']else -1.
  tree.backup(path,reward)
  if r['qualified']:archive.insert('/'.join(str(r[f])for f in fields),reward,r)
 uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
 pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
 ans={'group':name,'qualified':len(good),'selected':best,'uct_best':uct,'uct_unique':len(visited),'objective_agrees':obj(best)==obj(uct) if best and uct else best==uct,'pareto':pareto,'map_elites':{k:v[1]for k,v in archive.cells.items()}}
 out.append(ans);print(name,len(good),len(visited),ans['objective_agrees'],best)
for rt,rad in sorted({(r['rtol'],r['radius'])for r in data['rows']}):
 choose({'rtol':rt,'radius':rad},[r for r in data['rows']if r['rtol']==rt and r['radius']==rad],[solvers,certs],['solver','cert'],lambda v:(v['attempts'],v['rhs_proxy'],v['proof_cells'],v['bound']))
choose('hybrid',data['chains'],[list(solvers)[:5],certs,orders],['solver','cert','order'],lambda v:(v['attempts'],v['post_bound']))
dump('SELECTION.json',{'groups':out,'note':'finite measured lookup, exact proof identity separate; incomplete RHS proxy; hybrid objective attempts then bound, no wall-time or online savings claim'})
print('manifest',len(m['files']),'catalog',r['route_count'],'exact',v)
