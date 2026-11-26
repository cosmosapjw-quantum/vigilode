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
 {'Mass':['native mass RODAS','source-owned L1 residual'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'Affine5':['exact frozen affine singleton','original-source rank/Schur proof','RETRO Lagrange5']},
 {'Max':['whole-slab Bernstein supremum'],'Integral4':['directed four-cell Bernstein integral']},
 {'Shared':['shared native causal scan','DictionaryHint/PiecewiseProbe'],'Correlated':['shared native causal scan','Schur guard difference','DictionaryHint/PiecewiseProbe']},
 {'JointCentered':['strict Picard tube','centered signed JointTransport','AffineDual gap'],'JointGreen':['strict Picard tube','RETRO GreenMoment','JointTransport','AffineDual gap'],'AtlasFresh':['ConservedSchur','exact stochastic reset','fresh relative native phase','directed clock sum','RETRO centered dense query'],'AtlasReuse':['ConservedSchur','exact stochastic reset','per-invocation mode and exact reset-center keyed atlas','directed clock sum','RETRO centered dense query']}
]
fields=['solver','cert','order','method']
if sys.argv[1]=='generate':
 r=explore('independent one-shot falsification of frozen certified hybrid DAE portfolio, exact manufactured solutions, native Robertson full/reduced reference, rank-zero atlas versus nonconstant reset, exact tie, PreparedRhs DictionaryCompanion Affine5 RETRO Green and conservation; compare fixed development choices without tuning',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 dump('CANDIDATES.json',{'routes':[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['directed L1 residual','bounded projection interval scratch'])for k in itertools.product(*axes)],'manifest_files':len(m['files']),'harness_catalog_count':r['route_count'],'audit_candidates':['legacy receipts','explicit audited atlas','explicit audited conserved chain'],'local_scope':'new fixed192 hybrid and32 manufactured challenge cases; prior development selection frozen, not tuned on outcome','query_routes':['Horner','Centered'],'objectives':['native attempts','driver RHS calls','reported projection RHS','allocation requests','requested bytes','physical width']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('ATTEMPTS','(a+r)+(b+s)','(a+b)+(r+s)'),('RHS-PARTITION','(a+b)+(c+d)','(a+c)+(b+d)'),('DEBIT','m+(q-m)','q')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite bookkeeping identities only; measured costs independent'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(list(itertools.product(*axes))),'exact',v)
else: raise SystemExit("new challenge is not a selector training corpus")
