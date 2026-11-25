"""Pinned MathHarness generates structural routes, filters and ranks actual epoch runs."""
import sys,json,zipfile,hashlib,random,csv
from pathlib import Path
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(n,v): (P/n).write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for k,v in m['files'].items():
  f=(H/k).resolve();assert H in f.parents and sha(f)==v,k
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.ledger import Ledger,fresh_verify
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.search.primitives import UCT, MapElites
if sys.argv[1]=='generate':
 r=explore('absolute physical clock and incoming error for declared index-one DAE, affine pencil reanchoring, Schur projection, directed residual certificate, Green contraction, dense dictionary reconstruction, scalar branch exact roots, prepared RHS selection, missing bound refusal',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 mapped={'Mass':['native mass Rosenbrock','DaeSource::initialize','native dense packet'], 'Reduced3':['Schur ODE','dictionary basis3','Reuse'], 'Reduced5':['Schur ODE','dictionary basis5','Reuse'], 'Prepared5':['Schur ODE','dictionary basis5','PreparedBatch'], 'Scalar5':['Schur ODE','dictionary basis5','DictionaryCompanion'], 'WarpDirect':['TensorPencil DirectBlock','conditioned Horner','Reuse','basis5'], 'WarpFolded':['TensorPencil Folded','conditioned Horner','Reuse','basis5']}
 rows=[{'route':k,'green':g,'modules':v+(['retro_causal','RetroTable Hybrid phi0/phi1'] if g else ['directed exponential norm bound'])} for k,v in mapped.items() for g in [False,True]]
 dump('CANDIDATES.json',{'catalog_count':r['route_count'],'manifest_files':len(m['files']),'candidates':rows,'selection_contract':'full physical global bound <= 0.005; pointwise and branch; minimize proxy RHS count, then bound; no timing claim'})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('CLOCK','a+b*(s+h*u)','(a+b*s)+b*h*u'),('CLOCK-INVERSE','s+(e-s)*u','s+e*u-s*u'),('FORCING','x+(t*t-s*s)/2','x+(t-s)*(t+s)/2')]:
  # parser accepts rational division by 2? use multiplication-only cleared form below if needed
  if cid=='FORCING':lhs,rhs='2*x+t*t-s*s','2*x+(t-s)*(t+s)'
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite clock translation identity'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v.get('ok'),v
 dump('EXACT_CHECKS.json',v)
 print('generated',len(rows),'catalog',r['route_count'],'manifest',len(m['files']),'exact',v)
elif sys.argv[1]=='select':
 a=json.loads((P/'AUDIT.json').read_text())
 rows=a['rows'];out=[]
 for k,s in sorted({(v['kind'],v['start']) for v in rows}):
  z=[v for v in rows if (v['kind'],v['start'])==(k,s)]
  assert len(z)==14
  good=[v for v in z if v['qualified']]
  best=min(good,key=lambda v:(v['rhs_proxy'],v['bound'],v['route'],v['green'])) if good else None
  # UCT over categorical solver family and Green choice, evaluated against
  # already measured cells. MapElites preserves distinct route families.
  lookup={(v['route'],v['green']):v for v in z}
  tree=UCT(lambda path: sorted({v['route'] for v in z}) if not path else ([False,True] if len(path)==1 else []))
  rng=random.Random(1729); visited=set(); archive=MapElites()
  def reward(v):
   return 1./(1.+v['rhs_proxy']) if v['qualified'] else -1.
  for _ in range(64):
   path=tree.propose(rng);v=lookup[path];visited.add(path);tree.backup(path,reward(v))
   if v['qualified']:archive.insert(v['route'],reward(v),v)
  seen=[lookup[p] for p in visited if lookup[p]['qualified']]
  uct_best=min(seen,key=lambda v:(v['rhs_proxy'],v['bound'],v['route'],v['green'])) if seen else None
  pareto=[v for v in good if not any(w['rhs_proxy']<=v['rhs_proxy'] and w['bound']<=v['bound'] and (w['rhs_proxy']<v['rhs_proxy'] or w['bound']<v['bound']) for w in good)]
  out.append({'kind':k,'start':s,'selected':best,'qualified':len(good),'pareto':pareto,'uct_best':uct_best,'uct_unique_cells':len(visited),'uct_agrees':uct_best==best,'map_elites':{k:v[1] for k,v in archive.cells.items()}})
 dump('SELECTION.json',{'groups':out,'method':'hard global qualification then deterministic RHS proxy; UCT measured lookup and MapElites; Pareto preserved; no reduced native-run claim','timing_authority':'none','holdout':'NOT_RUN'})
 print('qualified groups',sum(x['selected'] is not None for x in out),'/',len(out))
 for x in out:
  print(x['kind'],x['start'],x['qualified'], x['selected'] and (x['selected']['route'],x['selected']['green'],x['selected']['rhs_proxy'],x['selected']['bound']))
