from pathlib import Path
import gzip,hashlib,itertools,json,random,statistics,sys,zipfile
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for n,d in m['files'].items():
  f=(H/n).resolve();assert H in f.parents and sha(f)==d,n
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites,ProposalBandit
cases=['stable','stiff','nonlinear','vector4','vector8']
query_cases=cases+['mass','conserved']
candidates=['legacy','fresh_fixed','fixed_rows','stream_pairs','cursor_pairs']
domains={
 'point':(['case','index','candidate'],[query_cases,list(range(12)),candidates]),
 'epoch':(['case','grid','candidate'],[cases,['sparse','dense'],['legacy','fixed_rows','stream_pairs','cursor_pairs']]),
}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='point':return (*o,r['query']['physical_error_upper'])
 return (*o,r['slabs'],r['max_physical_error'])
def good(name,r):return r['qualified'] and (r['query']['physical_error_upper']<=.001 if name=='point' else r['max_physical_error']<=.001)
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Original-source DAE dense query, actual native Solution.at_into, full retained InitReport storage reuse, fixed scalar and bounded vector projection; original algebraic branch and physical certificate unchanged. Mutations: fixed copied interval rows versus direct streaming interval-pair Horner, validated local/adjacent slab cursor with fallback binary search, exact retained report reservation and streaming maximum fold; compare fresh owner, warmed output and full epoch sparse/dense query workloads.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=set();order=[];goal=len(list(itertools.product(*axes)))
  for _ in range(32768):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal,(name,len(seen),goal)
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in order],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'fresh_fixed':'legacy','fixed_rows':'fresh_fixed','stream_pairs':'fixed_rows','cursor_pairs':'stream_pairs'},'scope':'finite preregistered candidate set; measured replay only'})
 print('MathHarness',len(m['files']),r['route_count'],[(k,len(v['uct_proposals']))for k,v in out.items()])
def replay(name,seq):
 fields,axes=domains[name];rows={key(fields,r):r for r in seq}
 assert len(rows)==len(seq)==len(list(itertools.product(*axes))) and set(rows)==set(itertools.product(*axes)),(name,len(rows),len(seq))
 rng=random.Random(97531);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};proposals={'structure':0,'mutation':0}
 for _ in range(8192):
  arm=bandit.pick(rng);proposals[arm]+=1
  if arm=='mutation'and seen:
   k=rng.choice(list(seen));k=k[:-1]+(rng.choice([v for v in axes[-1]if v!=k[-1]]),)
  else:k=tuple(tree.propose(rng))
  fresh=k not in seen;seen[k]=rows[k];qualified=good(name,rows[k]);nd=qualified and k in front(name,seen,k[:-1]);reward=fresh and nd
  bandit.update(arm,reward);tree.backup(k,float(reward))
  if qualified:archive.insert('/'.join(map(str,k[:-1])),float(nd),k)
 groups=[]
 for g in sorted({k[:-1]for k in rows}):
  f=front(name,rows,g)
  groups.append({'group':g,'qualified':sum(good(name,v)for k,v in rows.items()if k[:-1]==g),'pareto':[dict(zip(fields,k),objectives=obj(name,rows[k]))for k in f],'resource_first':min(f,key=lambda k:(obj(name,rows[k]),k))[-1]if f else None,'covered':len(set(f)&seen.keys())})
 candidates={}
 for route in axes[-1]:
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'])for r in rs),'bytes':sum(r['allocation']['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 pp=read('portfolio.log','QUERY_POINT');ee=read('portfolio.log','QUERY_EPOCH')
 seq={'point':pp,'epoch':ee}
 out={name:replay(name,rows)for name,rows in seq.items()}
 ts=read('portfolio.log','QUERY_TIMING')
 assert len(ts)==len(query_cases)*len(candidates)
 timing={(r['case'],r['candidate']):r['median_ns']for r in ts}
 cross=[]
 for case in query_cases:
  rows=[r for r in pp if r['case']==case];scores=[]
  for route in candidates:
   a=[r for r in rows if r['candidate']==route]
   assert len(a)==12
   scores.append({'candidate':route,'objectives':[sum(req(r['allocation'])for r in a),sum(r['allocation']['requested_bytes']for r in a),max(r['query']['physical_error_upper']for r in a),timing[(case,route)]],'cursor_hits':sum((r['work']or {}).get('cursor_hits',0)for r in a),'binary_searches':sum((r['work']or {}).get('binary_searches',0)for r in a)})
  cross.append({'case':case,'candidates':scores,'pareto':[r for r in scores if not any(dom(q['objectives'],r['objectives'])for q in scores)]})
 dump('TIMING_COMPARE.json',{'rows':ts,'cross':cross,'scope':'pinned single caller, interleaved seven samples of 48 mixed-order queries, fresh versus reused report explicitly distinguished'})
 out['scope']='finite measured replay, timing separate; equality of bound/retained reports; no promotion'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
