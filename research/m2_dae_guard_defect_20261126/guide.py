from pathlib import Path
import hashlib,json,zipfile,sys,itertools,random,gzip,statistics
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
parents={'SchurRefine1':'Schur','SchurRefine2':'Schur','PreparedSchurRefine2':'PreparedSchur','ConservedSchurRefine2':'ConservedSchur'}
depth={'SchurRefine1':1,'SchurRefine2':2,'PreparedSchurRefine2':2,'ConservedSchurRefine2':2}
axes=[
 {'Mass':['native mass RODAS','source-owned L1 residual'],'Reduced5':['scalar Reuse fixed-size certified callback','RETRO Lagrange5'],'Prepared5':['PreparedRhs','Schur','RETRO Lagrange5'],'Scalar5':['DictionaryCompanion','RETRO Lagrange5'],'Affine5':['exact affine singleton','RETRO Lagrange5']},
 {'Max':['whole-slab directed Bernstein supremum'],'Integral4':['four-cell directed Bernstein integral']},
 {'Tangent':['physical tangent'],'Schur':['reduced Schur'],'PreparedSchur':['PreparedRhs inverse candidate','directed defect'],'ConservedSchur':['source-owned L1 derivative box','exact affine lift'],'SchurRefine1':['Schur','one directed fixed-point intersection'],'SchurRefine2':['Schur','two directed fixed-point intersections'],'PreparedSchurRefine2':['PreparedRhs','two directed fixed-point intersections'],'ConservedSchurRefine2':['source-owned L1 derivative box','exact affine lift','two directed fixed-point intersections']},
 {'Box':['original interval box'],'Transport':['reduced-Lipschitz transport','RETRO polynomial composition']}]
fields=['solver','cert','derivative','value']
keys=list(itertools.product(*axes))
def read_log(path,marker,fields):
 data=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 rows={}
 for line in data.splitlines():
  if marker+' {' not in line:continue
  d=json.loads(line.split(marker+' ',1)[1]); k=tuple(d[f]for f in fields)
  assert k not in rows;rows[k]=d
 return rows
def req(d):return sum(d['allocation'][k] for k in ['alloc','zeroed','realloc'])
def sweep(d):return d['audit'].get('refinement_sweeps',{}).get('started',0)
def wd(v):return v[1]-v[0]
def inside(a,b):return b[0]<=a[0] and a[1]<=b[1]
def canonical(d):return json.dumps(d,sort_keys=True,allow_nan=False,separators=(',',':'))
def obj(d):
 r=d['result'];v=r['evidence']['velocity'];q=r['mid_value']
 return (req(d),d['allocation']['requested_bytes'],sweep(d),wd(v),wd(q))
def eligible(d):return 'evidence'in d['result'] and d['result']['evidence']['velocity'][0]>0
def dom(a,b):return all(x<=y for x,y in zip(a,b)) and a!=b
def front(rows,obj,eligible):
 ks=[k for k in rows if eligible(rows[k])]
 return [k for k in ks if not any(dom(obj(rows[q]),obj(rows[k])) for q in ks)]
def verify_mutation(d,p):
 a,b=d['result'],p['result']; assert 'evidence'in a and 'evidence'in b
 e,f=a['evidence'],b['evidence']
 assert all(inside(x,y) for x,y in zip(e['reduced_gradient'],f['reduced_gradient']))
 for k in ['reduced_clock','velocity']:assert inside(e[k],f[k]),(k,e[k],f[k])
 assert e['reduced_lip']<=f['reduced_lip'] and e['inverse_defect']==f['inverse_defect']
 for k in ('mid_value','left_value','right_value'):assert inside(a[k],b[k]),(k,a[k],b[k])
 assert {k:v for k,v in d['audit'].items() if k!='refinement_sweeps'}==p['audit']
 s=d['audit']['refinement_sweeps']
 assert s['started']==s['returned']==depth[d['derivative']]*p['audit']['reduced_columns']['returned']
 return any(wd(e[k])<wd(f[k]) for k in ('velocity','reduced_clock')) or any(wd(x)<wd(y) for x,y in zip(e['reduced_gradient'],f['reduced_gradient']))
if sys.argv[1]=='generate':
 r=explore('Modify certified interval Schur derivative algorithm, not just compose existing ODE solvers: directed Neumann norm enclosure V0; mutate by one or two synchronous fixed-point intersection sweeps V <- V intersect (W+E V); PreparedRhs and source-owned conserved L1 variant; bounded source/defect workspace and no heap in isolated kernels. Compare physical enclosure width, extra work, and heap allocation Pareto.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 # MathHarness UCT emits ordered, distinct candidate proposals before outcomes are known.
 tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
 rng=random.Random(1729);seen=[]
 for _ in range(640):
  k=tuple(tree.propose(rng));fresh=k not in seen
  if fresh:seen.append(k)
  tree.backup(k,float(fresh))
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'catalog_count':r['route_count'],'mutation_edges':{k:{'parent':v,'operation':'append synchronous interval intersection sweep(s)','sweeps':depth[k]} for k,v in parents.items()},'premeasurement_uct_proposals':[dict(zip(fields,k)) for k in seen],'fixed_measured_design':[dict(zip(fields,k),modules=sum((d[v]for d,v in zip(axes,k)),[])+['native phase atlas','directed branch defect','bounded source scratch'],mutation=parents.get(k[2]))for k in keys],'objectives':['allocation requests','requested bytes','extra refinement sweeps','velocity width','midpoint guard width'],'seed':1729})
 print('MathHarness manifest',len(m['files']),'abstract routes',r['route_count'],'premeasurement UCT unique',len(seen),'actual design',len(keys))
elif sys.argv[1]=='select':
 old=read_log(P.parent/'m2_dae_guard_workspace_20261126/after.log.gz','GUARD',fields)
 rows=read_log(P/'portfolio.log','GUARD',fields)
 assert set(rows)==set(keys)
 assert len(old)==80
 saved=[]
 for k,p in old.items():
  d=rows[k]
  assert canonical({x:d['result'][x]for x in p['result']})==canonical(p['result'])
  assert canonical(d['audit'])==canonical(p['audit'])
  assert req(p)-req(d)>=6 and d['allocation']['requested_bytes']<=p['allocation']['requested_bytes']
  saved.append((req(p),req(d),p['allocation']['requested_bytes'],d['allocation']['requested_bytes']))
 mutations={}
 for route in parents:
  ks=[k for k in keys if k[2]==route];strict=0
  for k in ks:
   p=rows[(k[0],k[1],parents[route],k[3])]
   strict+=verify_mutation(rows[k],p)
  mutations[route]={'count':len(ks),'strict_derivative_improvements':strict,'transverse':sum(eligible(rows[k])for k in ks)}
 pf=front(rows,obj,eligible)
 # Measured-table replay: reward based only on observed front; final full-front coverage is evaluated afterward.
 tree=UCT(lambda path:list(axes[len(path)])if len(path)<len(axes)else[])
 bandit=ProposalBandit(['uct_structure','mutate_algorithm'])
 archive=MapElites();rng=random.Random(271828);visited={};used={'uct_structure':0,'mutate_algorithm':0}
 for _ in range(512):
  arm=bandit.pick(rng);used[arm]+=1
  if arm=='mutate_algorithm' and visited:
   k0=rng.choice(list(visited))
   deriv=k0[2];root=parents.get(deriv,deriv)
   choices=[v for v in [root,*[m for m,p in parents.items() if p==root]] if v!=deriv]
   k=(k0[0],k0[1],rng.choice(choices),k0[3]) if choices else tuple(tree.propose(rng))
  else:k=tuple(tree.propose(rng))
  fresh=k not in visited;visited[k]=rows[k]
  good=eligible(rows[k])
  nondom=good and not any(eligible(x) and dom(obj(x),obj(rows[k]))for x in visited.values())
  bandit.update(arm,fresh and nondom)
  tree.backup(k,float(nondom) if fresh else 0.)
  if good:archive.insert(k[0]+'/'+k[1]+'/'+k[2],float(nondom),k)
 # Vector: distinct manufactured exact case and guard; compare parent and child within each stratum.
 vf=['case','guard','derivative','value']
 vr=read_log(P/'vector.log','VECTOR',vf)
 assert len(vr)==72,len(vr)
 vec=[]
 def vobj(r):return (req(r),r['allocation']['requested_bytes'],sweep(r),wd(r['evidence']['velocity']),max(map(wd,r['values'])))
 def veligible(r):return r['evidence']['velocity'][0]>0
 for case,guard in sorted({k[:2]for k in vr}):
  sub={k:v for k,v in vr.items()if k[:2]==(case,guard)}
  assert len(sub)==12
  counts={};improved=0
  for k,d in sub.items():
   if k[2] not in parents:continue
   p=sub[(case,guard,parents[k[2]],k[3])]
   ee,ep=d['evidence'],p['evidence']
   assert all(inside(x,y)for x,y in zip(ee['reduced_gradient'],ep['reduced_gradient']))
   assert inside(ee['reduced_clock'],ep['reduced_clock']) and inside(ee['velocity'],ep['velocity'])
   assert all(inside(x,y)for x,y in zip(d['values'],p['values']))
   assert {x:y for x,y in d['audit'].items()if x!='refinement_sweeps'}==p['audit']
   assert d['audit']['refinement_sweeps']['started']==d['audit']['refinement_sweeps']['returned']==depth[k[2]]*p['audit']['reduced_columns']['returned']
   improved+=wd(ee['velocity'])<wd(ep['velocity'])
   counts[k[2]+'/'+k[3]]={'parent_velocity':ep['velocity'],'velocity':ee['velocity'],'parent_width':wd(ep['velocity']),'width':wd(ee['velocity']),'sweeps':sweep(d),'parent_transverse':veligible(p),'transverse':veligible(d)}
  f=front(sub,vobj,veligible)
  vec.append({'case':case,'guard':guard,'strict_velocity_improvements':improved,'transverse':sum(map(veligible,sub.values())),'pareto':[dict(zip(vf,k),objectives=vobj(sub[k])) for k in f],'comparisons':counts})
 result={'native_rows':len(rows),'legacy_exact':len(old),'native_successful':sum('evidence'in x['result']for x in rows.values()),'native_transverse':sum(map(eligible,rows.values())),'legacy_saved_requests_range':[min(x[0]-x[1]for x in saved),max(x[0]-x[1]for x in saved)],'legacy_medians':[statistics.median([x[0]for x in saved]),statistics.median([x[1]for x in saved])],'legacy_total_requests':[sum(x[0]for x in saved),sum(x[1]for x in saved)],'legacy_total_bytes':[sum(x[2]for x in saved),sum(x[3]for x in saved)],'mutations':mutations,'native_pareto':[dict(zip(fields,k),objectives=obj(rows[k])) for k in pf],'replay':{'unique':len(visited),'pareto_coverage':len(set(pf)&visited.keys()),'pareto_size':len(pf),'proposal_counts':used,'bandit_posterior':bandit.ab},'vector_rows':len(vr),'vector_results':vec}
 dump('SELECTION.json',result)
 print(json.dumps({k:v for k,v in result.items()if k!='vector_results'},indent=2))
 for x in vec:print(x['case'],x['guard'],'strict velocity',x['strict_velocity_improvements'],'transverse',x['transverse'],'pareto',[(p['derivative'],p['value'])for p in x['pareto']])
