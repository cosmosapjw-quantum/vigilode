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
candidates=['legacy','packed_owned','shared_owned','packed_rows','shared_rows','packed_component','shared_component']
domains={
 'epoch':(['case','grid','candidate'],[cases,['sparse','dense'],candidates]),
 'arena':(['case','layout'],[cases+['mass','conserved'],['Packed','Shared']])
}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 if name=='arena':
  return (req(r['append_allocation']),req(r['read_allocation']),r['capacity_bytes'])
 a=r['allocation']
 return (req(a),a['requested_bytes'],r['heap']['peak_delta'],r['heap']['end_delta'],r['query_bytes']+r['native_bytes'],r['max_physical_error'])
def good(name,r):return r['qualified'] and (name=='arena' or r['max_physical_error']<=.001)
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Lossless retained DAE query arena, original source proof and full report preserved. Candidate compact layout all active arrays versus bit checked shared aliases, native stage ownership versus borrowed rows versus original fast physical-event component kernel, separate scalar scratch, explicit materialization with exact legacy identity. Compare allocation requests, bytes, live heap delta and controlled full epoch time. No default promotion.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=set();order=[];goal=len(list(itertools.product(*axes)))
  for _ in range(4096):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in order],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'packed_owned':'legacy','shared_owned':'packed_owned','packed_rows':'packed_owned','shared_rows':'packed_rows','packed_component':'packed_rows','shared_component':'shared_rows','Shared':'Packed'},'scope':'finite measured candidate designs; no online promotion'})
 print('MathHarness',len(m['files']),r['route_count'],[(k,len(v['uct_proposals']))for k,v in out.items()])
def replay(name,seq):
 fields,axes=domains[name];rows={key(fields,r):r for r in seq}
 assert len(rows)==len(seq)==len(list(itertools.product(*axes))) and set(rows)==set(itertools.product(*axes)),(name,len(rows),len(seq))
 rng=random.Random(97531);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};proposals={'structure':0,'mutation':0}
 for _ in range(2048):
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
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'] if name=='epoch' else r['append_allocation'])for r in rs),'bytes':sum((r['allocation'] if name=='epoch' else r['append_allocation'])['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 seq={'epoch':read('portfolio.log','ARENA_EPOCH'),'arena':read('portfolio.log','ARENA_REPLAY')}
 out={name:replay(name,rows)for name,rows in seq.items()}
 ts={(r['case'],r['grid'],r['candidate']):r for r in read('portfolio.log','ARENA_TIMING')}
 assert len(ts)==len(seq['epoch'])
 groups=[]
 for case,grid in itertools.product(cases,['sparse','dense']):
  a=[r for r in seq['epoch']if r['case']==case and r['grid']==grid]
  objective=lambda r:(*obj('epoch',r),ts[(case,grid,r['candidate'])]['median_ns'])
  groups.append({'case':case,'grid':grid,'pareto':[{'candidate':r['candidate'],'objectives':objective(r)}for r in a if not any(dom(objective(q),objective(r))for q in a)]})
 dump('TIMING_COMPARE.json',{'rows':list(ts.values()),'pareto':groups,'scope':'pinned caller, five interleaved whole epoch + drop samples; no restoration cost included'})
 out['scope']='finite measured replay; caller-thread live heap delta is not RSS or peak stack; no promotion'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
