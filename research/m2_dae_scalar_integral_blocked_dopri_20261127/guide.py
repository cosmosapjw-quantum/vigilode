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
cases=['quadratic','quartic','logistic','affine','clock','equilibrium','shifted']
dopri_cases=['linear1','linear4','linear8','linear16','linear31','linear64','linear127','linear256','linear1024','logistic31','quadratic','coupled','affine']
domains={'scalar':(['case','candidate'],[cases,['Repeated16','IntegralGuard16','Direct16','DirectBare16','Adaptive','FlowRatio','QuadraticSeed16']]),
 'blocked':(['case','candidate'],[dopri_cases,['Rows','Components','Block4','Block8','Block16','Block32']])}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='scalar':
  w=r['work'];return (*o,w['source_rows'],w['exponentials'],w['width_upper'],r['median_ns'])
 w=r['fixed_work'];return (*o,r['fixed_median_ns'],r['median_ns'],w['state_writes']+w['error_writes'],r['error_upper'])
def good(name,r):return r['qualified'] and (r['work']['width_upper']<=.1 if name=='scalar' else r['error_upper']<=1e-6)
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Certified original scalar index-1 DAE continuous adjoint: repeated exponential, interval integral accumulation, direct interval scalar Schur division with/without Newton narrowing, adaptive dyadic partition with interval intersection, source-proved autonomous nonzero flow ratio eliminating integration, existing quadratic homotopy center proposal with original interval Newton proof. Existing DOPRI5 row vs component vs fixed block4/8/16/32 recurrence preserving rounded tableau and controller, pure and actual reduced DAE. Select finite measured resource/width/time objectives, no default.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r);out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=set();order=[];goal=len(list(itertools.product(*axes)))
  for _ in range(8192):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==goal:break
  assert len(seen)==goal
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in order],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'IntegralGuard16':'Repeated16','Direct16':'IntegralGuard16','DirectBare16':'Direct16','Adaptive':'Direct16','FlowRatio':'Adaptive','QuadraticSeed16':'Direct16','Components':'Rows','Block4':'Components','Block8':'Block4','Block16':'Block8','Block32':'Block16'},'scope':'finite measured design, explicit refusals'})
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
  rs=[r for k,r in rows.items()if k[-1]==route];candidates[route]={'rows':len(rs),'qualified':sum(good(name,r)for r in rs),'requests':sum(req(r['allocation'])for r in rs),'bytes':sum((r['allocation'])['requested_bytes']for r in rs)}
 return {'rows':len(rows),'unique':len(seen),'proposals':proposals,'pareto_count':sum(len(g['pareto'])for g in groups),'pareto_covered':sum(g['covered']for g in groups),'candidates':candidates,'strata':groups}
if sys.argv[1]=='generate':generate()
else:
 d=read('blocked.log','BLOCK_KERNEL')
 er={r['case']:r['error_upper']for r in json.loads((P/'ODE_ACCURACY.json').read_text())}
 for r in d:r['error_upper']=er[r['case']]
 seq={'scalar':read('scalar.log','SCALAR_INTEGRAL'),'blocked':d}
 out={name:replay(name,rows)for name,rows in seq.items()}
 out['scalar_width_targets']=[]
 for case in cases:
  for cap in (.1,1e-4,1e-6):
   a=[r for r in seq['scalar']if r['case']==case and good('scalar',r)and r['work']['width_upper']<=cap]
   out['scalar_width_targets'].append({'case':case,'max_width':cap,'fastest':min(a,key=lambda r:r['median_ns'])['candidate']if a else None,'qualified':len(a)})
 out['blocked_time_first']=[{'case':case,'fixed':min([r for r in d if r['case']==case],key=lambda r:r['fixed_median_ns'])['candidate'],'full':min([r for r in d if r['case']==case],key=lambda r:r['median_ns'])['candidate']}for case in dopri_cases]
 out['scope']='finite measured candidates; actual width/time/resources, not policy promotion'
 dump('SELECTION.json',out)
 for k in domains:print(k,{a:b for a,b in out[k].items()if a!='strata'})
 print('WIDTH',out['scalar_width_targets']);print('TIME',out['blocked_time_first'])
