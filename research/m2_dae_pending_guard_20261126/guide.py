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
solvers={'Mass':['native mass Rosenbrock','native dense D'],'Reduced5':['Schur reduction','RETRO dictionary Lagrange5'],'Prepared5':['PreparedRhs','dictionary Lagrange5'],'Scalar5':['ScalarRoot','DictionaryCompanion'],'WarpFolded':['TensorPencil Folded','conditioned Horner','dictionary Lagrange5']}
roots={'IntervalNewton':['directed interval Newton'],'DictionaryHint':['DictionaryCompanion hint','physical interval Newton']}
guards={('Schur','Transport'):['reduced guard','correlated residual transport'],('Schur','Box'):['reduced guard','independent state interval'],('PreparedSchur','Transport'):['PreparedRhs','recomputed inverse defect','correlated residual transport']}
modes={'Legacy':['fail closed at uncertain endpoint'],'Hull':['pending derivative hull','all endpoint interval Newton'],'Piecewise':['per-slab derivative transport','hull of surviving root intervals'],'PiecewiseProbe':['per-slab transport','local midpoint or DictionaryCompanion hint'],'Checkpoint':['Piecewise','physical checkpoint','RETRO Green epoch restart','shared attempt cap']}
axes=[solvers,roots,guards,modes]
fields=['solver','root_route','derivative','value','mode']
if sys.argv[1]=='generate':
 r=explore('bounded multi-slab physical guard uncertainty; strict monotone guard; global derivative hull versus local piecewise Newton and dictionary probe; exact initial set root diameter floor; checkpoint retry with carried certified radius; Schur Guard Transport; RETRO Green, PreparedRhs and WARP candidate combination guidance',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 candidates=[dict(zip(fields,(s,k,*g,a)),modules=solvers[s]+roots[k]+guards[g]+modes[a]+['retro_dae_evolution','retro_causal','retro_green','directed::Interval']) for s,k,g,a in itertools.product(*axes)]
 dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(m['files']),'harness_catalog_count':r['route_count']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('ROOT-DIAMETER','(q+r)-(q-r)','2*r'),('SLOPE-SECANT','(b+4*b*b)-(a+4*a*a)','(b-a)*(1+4*(a+b))'),('CHECKPOINT-CLOCK','(x+d)+(t-s)+(s-a)','x+d+t-a')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite algebraic identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case'] for r in data['rows']}):
  rows=[r for r in data['rows'] if r['case']==case]
  lookup={(r['solver'],r['root_route'],(r['derivative'],r['value']),r['mode']):r for r in rows}
  assert len(lookup)==150
  def key(v):return (v['attempts_used'],v['rhs_proxy'],v['guard_evals'],v['refinements'],v['hint_calls'],v['root_width'],tuple(v[f] for f in fields))
  good=[r for r in rows if r['qualified']]
  best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)]) if len(path)<len(axes) else [])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(768):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts_used']+v['rhs_proxy']/10000.) if v['qualified'] else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['solver']+'/'+v['mode']+'/'+v['value'],reward,v)
  uct=min((lookup[p] for p in visited if lookup[p]['qualified']),key=key,default=None)
  def obj(v):return key(v)[:-1]
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v))) and obj(w)!=obj(v) for w in good)]
  out.append({'case':case,'selected':best,'qualified':len(good),'uct_best':uct,'uct_unique':len(visited),'uct_agrees':uct==best,'map_elites':{k:v[1] for k,v in archive.cells.items()},'pareto':pareto})
 dump('SELECTION.json',{'groups':out,'method':'hard physical contract; lexicographic attempts/RHS/guard/refinement/hint/width; categorical UCT over measured lookup, MAP-Elites and exhaustive comparison'})
 for g in out:print(g['case'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['selected'])
