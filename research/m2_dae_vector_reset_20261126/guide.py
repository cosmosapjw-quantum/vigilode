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
 {'BoxPicard':['independent reset interval','strict original-source Picard tube'],'SchurPicard':['Schur Guard Transport','strict Picard tube'],'Centered':['composed P+(s-t)k','Schur transport','centered residual integral'],'GreenMoment':['RETRO phi0/phi1','diagonal unit-damping split','signed Schur forcing remainder','composed Q and Green moment']},
 {1:['actual native slab intersection'],4:['four time subcells per native slab']},
 {'Reuse':['LU reuse','uniform branch contraction'],'PreparedBatch':['PreparedRhs','uniform branch contraction'],'ScalarRoot':['DictionaryCompanion','exact scalar nominal root','uniform branch contraction']},
 {'Mass':['native mass restart'],'Prepared5':['PreparedRhs','Schur','dictionary Lagrange5'],'Scalar5':['ScalarRoot','DictionaryCompanion','dictionary Lagrange5'],'WarpFolded':['TensorPencil Folded','conditioned Horner','original projection','dictionary Lagrange5']}
]
fields=['sync_route','subdivisions','branch_route','post_route']
if sys.argv[1]=='generate':
 r=explore('verified vector polynomial reset and nonlinear source-coupled index-one post DAE, Picard tube with uniform algebraic branch, Schur reduced signed Jacobian, centered time-state correlation, diagonal Green moment split and forcing remainder, time subdivision, RETRO DictionaryCompanion, PreparedRhs and WARP epoch predictor combination guidance',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 candidates=[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['native Mass pre','RETRO causal residual tube','directed::Interval'])for k in itertools.product(*axes)]
 dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(m['files']),'harness_catalog_count':r['route_count']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('CENTERED','P+(s-t)*F','P+(s-t)*k+(s-t)*(F-k)'),('SPLIT','F','-(X-e)+(F+X-e)'),('EXCURSION','(a-q)*(a-q)-(b-q)*(b-q)','(a-b)*(a+b-2*q)')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite polynomial identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case']for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case]
  lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==96
  def obj(v):return (v['attempts'],v['rhs_proxy'],v['picard_attempts'],v['green_calls'],v['width'])
  def key(v):return (*obj(v),tuple(v[f]for f in fields))
  good=[v for v in rows if v['qualified']]
  best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  # Explicit exploration warm-up from random distinct combinations is
  # reported separately from UCT; no pretend expensive solver savings.
  for _ in range(512):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts']+v['rhs_proxy']/1000.+v['picard_attempts']/100.) if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['sync_route']+'/'+str(v['subdivisions'])+'/'+v['post_route'],reward,v)
  uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
  out.append({'case':case,'selected':best,'qualified':len(good),'uct_best':uct,'uct_unique':len(visited),'uct_agrees':uct==best,'objective_agrees':(None if uct is None else obj(uct))==(None if best is None else obj(best)),'map_elites':{k:v[1]for k,v in archive.cells.items()},'pareto':pareto})
 dump('SELECTION.json',{'groups':out,'method':'hard physical contract then attempts/RHS/Picard/Green/width; categorical UCT measured lookup; exhaustive and MAP-Elites comparison'})
 for g in out:print(g['case'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['objective_agrees'],g['selected'])
