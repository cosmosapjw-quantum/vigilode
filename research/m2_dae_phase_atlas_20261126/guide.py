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
 {'Mass':['native mass RODAS','source-owned L1 residual'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5']},
 {'Max':['whole-slab Bernstein supremum'],'Integral4':['directed four-cell Bernstein integral']},
 {'Shared':['shared native causal scan','DictionaryHint/PiecewiseProbe'],'Correlated':['shared native causal scan','Schur guard difference','DictionaryHint/PiecewiseProbe']},
 {'JointCentered':['strict Picard tube','centered signed JointTransport','AffineDual gap'],'JointGreen':['strict Picard tube','RETRO GreenMoment','JointTransport','AffineDual gap'],'AtlasFresh':['ConservedSchur','exact stochastic reset','fresh relative native phase','directed clock sum','RETRO centered dense query'],'AtlasReuse':['ConservedSchur','exact stochastic reset','per-invocation mode and exact reset-center keyed atlas','directed clock sum','RETRO centered dense query']}
]
fields=['solver','cert','order','method']
if sys.argv[1]=='generate':
 r=explore('finite autonomous multimode stiff hybrid phase atlas, exact rank-zero reset, mode and reset-center identity, directed absolute clocks, cached first-event receipt, compare fresh and reused phase versus joint L1 Picard/RETRO Green synchronization; Mass PreparedRhs Scalar5 DictionaryCompanion ConservedSchur and Correlated ordering',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 dump('CANDIDATES.json',{'routes':[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['directed L1 residual','positive dwell'])for k in itertools.product(*axes)],'manifest_files':len(m['files']),'harness_catalog_count':r['route_count'],'query_routes':['Horner','Centered']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('CLOCK-WIDTH','(b+n*u)-(a+n*l)','b-a+n*(u-l)'),('RELATIVE-QUERY','(t-a)-(t-b)','b-a'),('RANKZERO','c*u+c*v+c*z','c*(u+v+z)'),('LIFT','u+v+(1-u-v)','1'),('PERIOD','(a+p)+q','a+(p+q)')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite exact algebraic identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(list(itertools.product(*axes))),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case']for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case];lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==48
  def obj(v):return (v['attempts'],v['rhs_proxy'],v['last_width'])
  def key(v):return (*obj(v),tuple(v[f]for f in fields))
  good=[v for v in rows if v['qualified']];best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(384):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts']+v['rhs_proxy']/1000.)if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['solver']+'/'+v['cert']+'/'+v['method'],reward,v)
  uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
  out.append({'case':case,'qualified':len(good),'selected':best,'uct_best':uct,'uct_unique':len(visited),'objective_agrees':obj(best)==obj(uct)if best and uct else best==uct,'pareto':pareto,'map_elites':{k:v[1]for k,v in archive.cells.items()}})
  print(case,len(good),len(visited),out[-1]['objective_agrees'],best)
 dump('SELECTION.json',{'groups':out,'method':'certified horizon then native attempts, incomplete RHS proxy, final component diameter; measured UCT/exhaustive, no total-cost claim'})
