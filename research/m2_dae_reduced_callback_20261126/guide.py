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
 {'Mass':['native mass RODAS','source-owned L1 residual'],'Reduced5':['scalar Reuse projection','caller-owned fixed-size certified callback','RETRO Lagrange5'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'Affine5':['exact frozen affine singleton','original-source rank/Schur proof','RETRO Lagrange5']},
 {'Max':['whole-slab Bernstein supremum'],'Integral4':['directed four-cell Bernstein integral']},
 {'Shared':['shared native causal scan','DictionaryHint/PiecewiseProbe'],'Correlated':['shared native causal scan','Schur guard difference','DictionaryHint/PiecewiseProbe']},
 {'JointCentered':['strict Picard tube','centered signed JointTransport','AffineDual gap'],'JointGreen':['strict Picard tube','RETRO GreenMoment','JointTransport','AffineDual gap'],'AtlasFresh':['ConservedSchur','exact stochastic reset','fresh relative native phase','directed clock sum','RETRO centered dense query'],'AtlasReuse':['ConservedSchur','exact stochastic reset','per-invocation mode and exact reset-center keyed atlas','directed clock sum','RETRO centered dense query']}
]
fields=['solver','cert','order','method']
if sys.argv[1]=='generate':
 r=explore('certified scalar Reuse reduced callback owned-output separation with unchanged interval proof, compare actual Reduced5 fixed scratch versus Mass PreparedRhs DictionaryCompanion Affine5 RETRO Green and phase atlas, measured resource and physical width Pareto',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 dump('CANDIDATES.json',{'routes':[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['directed L1 residual','bounded projection interval scratch'])for k in itertools.product(*axes)],'manifest_files':len(m['files']),'harness_catalog_count':r['route_count'],'audit_candidates':['legacy receipts','explicit audited atlas','explicit audited conserved chain'],'local_scope':'240 actual fixed development combinations; 48 Reduced5 before/after and audited comparisons; successful callback zero heap requests only','query_routes':['Horner','Centered'],'objectives':['native attempts','driver RHS calls','reported projection RHS','allocation requests','requested bytes','physical width']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('IMPLICIT','a*(b*c)','(a*b)*c'),('SCHUR','a*x+a*y','a*(x+y)'),('TANGENT','a*(b+c)+d','a*b+a*c+d')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite bookkeeping identities only; measured costs independent'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(list(itertools.product(*axes))),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case']for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case];lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==80
  good=[v for v in rows if v['qualified']]
  obj=lambda v:(v['attempts'],v['driver_rhs'],v['projection_rhs'],v['total_requests'],v['total_bytes'],v['last_width'])
  def dominates(w,v):return all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)
  left=good[:];fronts=[];rank={}
  while left:
   f=[v for v in left if not any(dominates(w,v)for w in left)]
   assert f
   for v in f:rank[tuple(v[k]for k in fields)]=len(fronts)
   fronts.append(f);left=[v for v in left if v not in f]
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(384):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+rank[path])if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['solver']+'/'+v['cert']+'/'+v['method'],reward,v)
  def brief(v):return {k:v[k]for k in fields+['attempts','driver_rhs','projection_rhs','solve_requests','solve_bytes','total_requests','total_bytes','last_width','cache_hits']}
  def choose(order):
   return min(good,key=lambda v:tuple(v[k]for k in order)+tuple(v[k]for k in fields))
  choices={'attempt_first':choose(['attempts','driver_rhs','projection_rhs','total_requests','total_bytes','last_width']),'allocation_first':choose(['total_requests','total_bytes','last_width','attempts','driver_rhs','projection_rhs']),'byte_first':choose(['total_bytes','total_requests','last_width','attempts']),'width_first':choose(['last_width','total_bytes','attempts']),'projection_first':choose(['projection_rhs','driver_rhs','total_requests','last_width'])}
  assert all(v in fronts[0]for v in choices.values())
  frontkeys={tuple(v[k]for k in fields)for v in fronts[0]}
  q={'case':case,'qualified':len(good),'pareto':list(map(brief,fronts[0])),'front_sizes':list(map(len,fronts)),'uct_unique':len(visited),'uct_pareto_coverage':len(visited&frontkeys),'map_elites':{k:brief(v[1])for k,v in archive.cells.items()},'choices':{k:brief(v)for k,v in choices.items()}}
  out.append(q);print(case,'qualified',len(good),'front',len(fronts[0]),'uct found',q['uct_pareto_coverage'],'of',len(fronts[0]),'choices',q['choices'])
 dump('SELECTION.json',{'groups':out,'method':'hard certified horizon then six-dimensional Pareto; UCT reward measured nondomination rank, sensitivity lexicographic; no commensurate FLOP/time units or online exploration claim'})
