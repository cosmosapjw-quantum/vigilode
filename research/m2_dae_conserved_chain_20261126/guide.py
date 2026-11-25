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
 {'Mass':['native mass RODAS','source-owned affine L1 residual'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5','source-owned L1 residual'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5','source-owned L1 residual']},
 {'Centered':['Schur reset transport','centered polynomial synchronization'],'GreenMoment':['RETRO phi0/phi1','diagonal forcing remainder','Schur reset transport']},
 {'BoxLift':['explicit infinity-to-L1 dimension factor'],'JointTransport':['source-owned column-stochastic map','reduced L1 operator','four signed polynomial directions','centered physical field remainder']},
 {'Box':['original affine-lifted state box'],'AffineDual':['signed exact affine lift','centered reduced gradient over tube','L1 dual support over full restart set']},
 {'Shared':['shared native accepted-step certificate','DictionaryHint/PiecewiseProbe'],'Correlated':['shared physical slab','Schur guard difference','DictionaryHint/PiecewiseProbe']}
]
fields=['solver','sync','l1_route','gap_route','order']
if sys.argv[1]=='generate':
 r=explore('conserved stiff hybrid multimode chain, source-owned stochastic reset, actual joint L1 transfer, affine-dual guard negativity over full transition gap and transported L1 restart set, directed dwell and common budgets; native Mass PreparedRhs Scalar5 DictionaryCompanion RETRO Green Picard and Correlated guard proof, conserved narrow L1 derivative box for cyclic modes',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 candidates=[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['directed interval Bernstein Integral4','bounded native causal scan','positive dwell'])for k in itertools.product(*axes)]
 dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(m['files']),'harness_catalog_count':r['route_count'],'focused_cycle_candidates':['Schur','ConservedSchur']})
 l=Ledger(P/'exact_ledger.json')
 for cid,lhs,rhs in [('LIFT','u+v+1-u-v','1'),('RESET-LINEAR','a*(u+du)+b*(1-u-du-v-dv)-a*u-b*(1-u-v)','(a-b)*du-b*dv'),('GAP-CANCEL','u+v+(1-u-v)-1+e*(t-b)+e*k*(1-u-v)','e*(t-b)+e*k*(1-u-v)'),('DWELL','(b-r)-(a+s)','b-a-r-s'),('CHAIN-NORM','a*(b*e+r)+s','a*b*e+a*r+s')]:
  l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite exact algebraic identities only'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
  assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
 v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
 print('manifest',len(m['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
 data=json.loads((P/'AUDIT.json').read_text());out=[]
 for case in sorted({r['case']for r in data['rows']}):
  rows=[r for r in data['rows']if r['case']==case];lookup={tuple(r[f]for f in fields):r for r in rows};assert len(lookup)==48
  def obj(v):return (v['attempts'],v['rhs_proxy'],v['charged_picard'],v['gap_checks'],v['green_calls'],v['bound'],v['incoming_l1'])
  def key(v):return (*obj(v),tuple(v[f]for f in fields))
  good=[v for v in rows if v['qualified']];best=min(good,key=key,default=None)
  tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
  archive=MapElites();rng=random.Random(1729);visited=set()
  for _ in range(384):
   path=tree.propose(rng);v=lookup[path];visited.add(path)
   reward=1./(1.+v['attempts']+v['rhs_proxy']/1000.+v['charged_picard']/100.)if v['qualified']else -1.
   tree.backup(path,reward)
   if v['qualified']:archive.insert(v['solver']+'/'+v['sync']+'/'+v['l1_route']+'/'+v['gap_route'],reward,v)
  uct=min((lookup[p]for p in visited if lookup[p]['qualified']),key=key,default=None)
  pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v)))and obj(w)!=obj(v)for w in good)]
  out.append({'case':case,'qualified':len(good),'selected':best,'uct_best':uct,'uct_unique':len(visited),'objective_agrees':obj(best)==obj(uct)if best and uct else best==uct,'pareto':pareto,'map_elites':{k:v[1]for k,v in archive.cells.items()}})
  print(case,len(good),len(visited),out[-1]['objective_agrees'],best)
 dump('SELECTION.json',{'groups':out,'method':'horizon authority then native attempts/RHS proxy/charged Picard/gap checks/Green calls/bound/incoming L1; measured UCT/exhaustive, incomplete cost accounting'})
