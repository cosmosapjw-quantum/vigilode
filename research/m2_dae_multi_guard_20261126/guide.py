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
 {'Mass':['native mass RODAS'],'Reduced5':['Schur reduced','RETRO Lagrange5'],'Prepared5':['PreparedRhs','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'WarpFolded':['WARP TensorPencil Folded','conditioned Horner','original-source projection']},
 {'Independent':['independent native runs','single global attempt cap','interval root order'],'Shared':['one native accepted-step driver','one RETRO physical slab proof','interval root order'],'Correlated':['shared physical slab proof','exact polynomial difference','Schur Transport competitor-root test']},
 {'IntervalNewton':['directed Newton','Piecewise pending'],'DictionaryHint':['DictionaryCompanion hint','PiecewiseProbe pending']},
 {'Schur':['original slab preconditioner'],'PreparedSchur':['PreparedRhs','signed original inverse-defect check']},
 {'Box':['independent interval guard'],'Transport':['composed polynomial','Schur physical error transport']}
]
fields=['solver','method','root_route','derivative','value']
if sys.argv[1]=='generate':
 r=explore('certified first-event ordering of multiple increasing polynomial guards on one semi-explicit index-one DAE, shared accepted-step certificate and independent baseline, overlapping marginal root intervals versus same-trajectory guard difference, Schur transport, RETRO Causal Green, PreparedRhs, DictionaryCompanion pending isolation, WARP time pencil',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 candidates=[dict(zip(fields,k),pending={'IntervalNewton':'Piecewise','DictionaryHint':'PiecewiseProbe'}[k[2]],modules=sum((d[v]for d,v in zip(axes,k)),[])+['directed::Interval','RETRO causal Green residual authority'])for k in itertools.product(*axes)]
 dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(m['files']),'harness_catalog_count':r['route_count']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [
 ('SHARED-STATE','(x-q)-(x-q-d)','d'),
 ('CONSTRAINT','(x+t-2*q)-(z-2*q-d)','d-(z-x-t)'),
 ('SWITCH','(q-delta)-q','-delta'),
 ('PENCIL','t+t*t/2+delta-q*(1+t/8)','t*t/2+(1-q/8)*t+delta-q')]:
  if cid=='PENCIL':lhs,rhs='8*t+4*t*t+8*delta-q*(8+t)','4*t*t+(8-q)*t+8*delta-8*q'
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite exact algebraic identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case']for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case]
  lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==120
  def obj(v):return (v['attempts'],v['rhs_proxy'],v['certificates'],v['guard_evals'],v['comparisons'],v['width'])
  def key(v):return (*obj(v),tuple(v[f]for f in fields))
  good=[v for v in rows if v['qualified']]
  best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(768):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts']+v['rhs_proxy']/1000.+v['certificates']/100.) if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['method']+'/'+v['solver']+'/'+v['value'],reward,v)
  uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
  out.append({'case':case,'selected':best,'qualified':len(good),'uct_best':uct,'uct_unique':len(visited),'uct_agrees':uct==best,'objective_agrees':(None if uct is None else obj(uct))==(None if best is None else obj(best)),'map_elites':{k:v[1]for k,v in archive.cells.items()},'pareto':pareto})
 dump('SELECTION.json',{'groups':out,'method':'hard physical order/absence then native attempts, recorded RHS, certificates, guard evaluations, comparisons, width; categorical UCT measured lookup + MAP-Elites + exhaustive'})
 for g in out:print(g['case'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['objective_agrees'],g['selected'])
