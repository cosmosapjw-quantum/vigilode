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
positions=['first','middle','last']
routes=['Pivot','Equilibrated','Refined','PackedGreen']
epoch_candidates=['legacy/legacy']+[r+'/'+s for r in routes for s in ['Local','Reusable']]
domains={
 'point':(['case','position','candidate'],[cases,positions,['legacy']+routes]),
 'epoch':(['case','variant'],[cases,epoch_candidates]),
}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='point':
  c=r['certificate'];return (*o,c['whole_x_error'],c['whole_z_error'],c['outgoing'])
 return (*o,r['slabs'],r['max_physical_error'],r['outgoing'])
def good(name,r):
 if not r['qualified']:return False
 if name=='point':
  c=r['certificate'];return c['whole_x_error']<=.001 and c['whole_z_error']<=.001
 return r['max_physical_error']<=.001
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Native source-owned index-1 DAE slab certificate workspace, pivoted and equilibrated original-source inverse, bounded Bernstein residual, interval derivative Gauss-Seidel refinement, existing packed phi response for dissipative Green propagation, local versus reused scratch with retained owning output; exact native packet and projection result identity, allocation positive control, measured bound and time tradeoffs, do not change physical authority.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=[];goal=len(list(itertools.product(*axes)))
  for _ in range(4096):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal,(name,len(seen),goal)
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in seen],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'Pivot':'legacy','Equilibrated':'Pivot','Refined':'Pivot','PackedGreen':'Pivot','Reusable':'Local'},'scope':'finite candidate designs; measured replay; no online promotion'})
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
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'])for r in rs),'bytes':sum(r['allocation']['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 pp=read('portfolio.log','CERT_POINT');ee=read('portfolio.log','CERT_EPOCH')
 for r in ee:r['variant']=r['candidate']+'/'+r['storage']
 seq={'point':pp,'epoch':ee}
 out={name:replay(name,rows)for name,rows in seq.items()}
 # Timing is a separate diagnostic Pareto objective; output reuse and
 # fresh ownership are named explicitly in the measurement records.
 ts={(r['case'],r['position'],r['candidate']):r for r in read('portfolio.log','CERT_TIMING')}
 assert len(ts)==len(pp)
 for r in pp:r['median_ns']=ts[(r['case'],r['position'],r['candidate'])]['median_ns']
 timefront=[]
 for case,pos in itertools.product(cases,positions):
  rs=[r for r in pp if r['case']==case and r['position']==pos and good('point',r)]
  objective=lambda r:(*obj('point',r),r['median_ns'])
  front=[{'candidate':r['candidate'],'objectives':objective(r)}for r in rs if not any(dom(objective(q),objective(r))for q in rs)]
  timefront.append({'case':case,'position':pos,'pareto':front})
 dump('TIMING_COMPARE.json',{'rows':list(ts.values()),'pareto':timefront,'scope':'pinned caller thread, interleaved 7 samples x8 calls, legacy fresh owner vs warmed output; not whole-driver benchmark'})
 out['scope']='finite measured replay, timing separate; allocation requests/bytes are not peak memory; no promotion'
 dump('SELECTION.json',out)
 for k,v in out.items():print(k,{a:b for a,b in v.items()if a!='strata'}if isinstance(v,dict)else v)
