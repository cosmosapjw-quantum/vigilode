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
cases=['clock2','clock4','clock8','mixed_inverse','quadratic']
spatial=['box2','box3','sphere128','sphere512','sphere_odd']
domains={'goal':(['case','budget','candidate'],[cases,['loose','middle','tight'],['Fixed','StateReplay','SecantReplay','Predictive','CostAware']]),
 'shift':(['case','shift','candidate'],[spatial,[4.,32.,256.,4096.],['Identity','Longitude','Meridional','ShortIdentity','EarlyIdentity','EarlyLongitude','FourierTranspose','Fourier']])}
def read(f,marker):
 path=P/f
 if not path.exists():path=Path(str(path)+'.gz')
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 return [json.loads(l.split(marker+' ',1)[1])for l in t.splitlines()if l.startswith(marker+' {')or l.startswith('test ')and ' ... '+marker+' {'in l]
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def obj(name,r):
 a=r['allocation'];o=(req(a),a['requested_bytes'])
 if name=='goal':return (*o,r['median_ns'],r['total_rhs'],r['total_steps'],r['certificate_actions'],max(map(abs,r['attempts'][-1]['enclosure'])))
 w=r['report'];return (*o,r['median_ns'],w['iterations'],w['error_l1'])
def good(name,r):return r['qualified']
def dom(a,b):return a!=b and all(x<=y for x,y in zip(a,b))
def front(name,rows,g):
 ks=[k for k,r in rows.items()if k[:-1]==g and good(name,r)]
 return [k for k in ks if not any(dom(obj(name,rows[q]),obj(name,rows[k]))for q in ks)]
def key(fields,r):return tuple(r[f]for f in fields)
def generate():
 r=explore('Cost-aware initial tolerance and cheap-to-tight goal replay, terminal signed exclusion early jump; compare old fixed and predictive. Actual original spatial transpose shifted resolvent: fixed32 versus restart8, right monitored prefix acceptance using original directed residual and inverse-shift seed, existing longitude, direct modal LU transpose vs weighted Fourier, odd spherical grid and weak shift. Select measured total work time allocations and directed error. Native protected stage integration separately evaluated, no continuum claim.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r);out={}
 for name,(fields,axes)in domains.items():
  rng=random.Random(1729);tree=UCT(lambda p:axes[len(p)]if len(p)<len(axes)else []);seen=set();order=[];target=len(list(itertools.product(*axes)))
  for _ in range(8192):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.add(k);order.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==target:break
  assert len(seen)==target
  out[name]={'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in order],'fixed_design':[dict(zip(fields,k))for k in itertools.product(*axes)]}
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':out,'mutation_edges':{'StateReplay':'Fixed','SecantReplay':'StateReplay','Predictive':'SecantReplay','Longitude':'Identity','Meridional':'Longitude','Fourier':'Identity','CostAware':'Predictive','ShortIdentity':'Identity','EarlyIdentity':'Identity','EarlyLongitude':'Longitude','FourierTranspose':'Fourier'},'native_design':{'grids':spatial+['box2_strong'],'all':['Dopri','Paired','Complex','Identity','ShortIdentity','EarlyIdentity','FourierTranspose'],'sphere_only':['Longitude','Meridional','EarlyLongitude']},'scope':'finite measured design, structural refusals and failed solves excluded; native full IVP separate'})
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
 seq={'goal':read('cost.log','COST_BUDGET'),'shift':read('early.log','EARLY_RUN')}
 out={name:replay(name,rows)for name,rows in seq.items()}
 out['time_first']={name:[{'group':list(g),'candidate':min([r for r in rows if tuple(r[f]for f in domains[name][0][:-1])==g and good(name,r)],key=lambda r:r['median_ns'])['candidate']}for g in sorted({tuple(r[f]for f in domains[name][0][:-1])for r in rows if good(name,r)})]for name,rows in seq.items()}
 out['scope']='finite measured replay; total replay cost not selected-run-only; no promotion'
 dump('SELECTION.json',out)
 for k in domains:print(k,{a:b for a,b in out[k].items()if a!='strata'})
 print('TIME',out['time_first'])
