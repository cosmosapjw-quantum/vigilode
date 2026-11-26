from pathlib import Path
import hashlib,json,zipfile,sys,itertools,random
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
from mathharness_general.search.primitives import UCT,MapElites
axes=[
 {'Mass':['native mass RODAS','source-owned L1 residual'],'Reduced5':['scalar Reuse fixed-size certified callback','RETRO Lagrange5'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'Affine5':['exact affine singleton','RETRO Lagrange5']},
 {'Max':['whole-slab directed Bernstein supremum'],'Integral4':['four-cell directed Bernstein integral']},
 {'Tangent':['physical tangent'],'Schur':['reduced Schur'],'PreparedSchur':['PreparedRhs inverse candidate','directed defect'],'ConservedSchur':['source-owned L1 derivative box','exact affine lift']},
 {'Box':['original interval box'],'Transport':['reduced-Lipschitz transport','RETRO polynomial composition']}]
fields=['solver','cert','derivative','value']
keys=list(itertools.product(*axes))
def read(n):
 rows={}
 for line in (P/n).read_text().splitlines():
  if 'GUARD {' not in line:continue
  d=json.loads(line.split('GUARD ',1)[1]); k=tuple(d[f]for f in fields); assert k not in rows;rows[k]=d
 assert set(rows)==set(keys),(len(rows),set(keys)-set(rows));return rows
def req(d):return sum(d['allocation'][k] for k in ['alloc','zeroed','realloc'])
def obj(d):
 r=d['result'];v=r['evidence']['velocity'];q=r['mid_value']
 return (req(d),d['allocation']['requested_bytes'],v[1]-v[0],q[1]-q[0])
def eligible(d):return 'evidence'in d['result'] and d['result']['evidence']['velocity'][0]>0
if sys.argv[1]=='generate':
 r=explore('Native stiff DAE guard construction portfolio: directed interval Schur versus tangent and source-owned conserved Schur; box vs RETRO polynomial transport; bounded temporary solve arrays vs owned storage, preserve physical certificate and work, Pareto allocation and enclosure widths.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'catalog_count':r['route_count'],'measured_design':[dict(zip(fields,k),modules=sum((d[v] for d,v in zip(axes,k)),[])+['native phase atlas','directed branch defect','fixed-size derivative scratch candidate']) for k in keys],'scope':'native first certified development slab; not independent end-to-end route qualification','objectives':['allocation requests','requested bytes','velocity enclosure width','midpoint guard enclosure width']})
 print('MathHarness manifest',len(m['files']),'catalog',r['route_count'],'measured design',len(keys))
else:
 old=read('baseline.log');new=read('after.log')
 assert all(old[k]['result']==new[k]['result'] and old[k]['audit']==new[k]['audit'] for k in keys)
 good=[k for k in keys if eligible(new[k])]
 assert good
 for k in keys:
  if 'evidence'in old[k]['result']:
   assert req(old[k])-req(new[k])>=6,(k,req(old[k]),req(new[k]))
   assert new[k]['allocation']['requested_bytes']<=old[k]['allocation']['requested_bytes'],k
 def dom(a,b):return all(x<=y for x,y in zip(obj(new[a]),obj(new[b]))) and obj(new[a])!=obj(new[b])
 front=[k for k in good if not any(dom(q,k)for q in good)]
 tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
 rng=random.Random(1729);visited=set();archive=MapElites()
 for _ in range(384):
  k=tuple(tree.propose(rng));visited.add(k)
  reward=1 if k in front else (0.25 if k in good else -1)
  tree.backup(k,reward)
  if k in good:archive.insert(k[0]+'/'+k[1]+'/'+k[2]+'/'+k[3],reward,k)
 out={'candidate_count':len(keys),'successful':sum('evidence'in new[k]['result']for k in keys),'transverse':len(good),'identical_result_and_audit':len(keys),'pareto':[dict(zip(fields,k),objectives=obj(new[k])) for k in front],'uct_unique':len(visited),'uct_pareto_coverage':len(set(front)&visited),'rows':[dict(zip(fields,k),old_requests=req(old[k]),new_requests=req(new[k]),old_bytes=old[k]['allocation']['requested_bytes'],new_bytes=new[k]['allocation']['requested_bytes'],eligible=eligible(new[k]),objectives=obj(new[k]) if 'evidence'in new[k]['result'] else None) for k in keys]}
 dump('SELECTION.json',out)
 print({k:v for k,v in out.items()if k!='rows'})
