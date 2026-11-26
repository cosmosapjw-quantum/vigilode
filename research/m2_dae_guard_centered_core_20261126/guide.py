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
native=['Tangent','Schur','PreparedSchur','ConservedSchur','SchurRefine1','SchurRefine2','PreparedSchurRefine2','ConservedSchurRefine2','SchurGs1','SchurGs2','SchurGsAdaptive2','PreparedSchurGs2','ConservedSchurGsAdaptive2','SchurCentered','SchurCenteredGrouped','SchurCenteredGs2','PreparedSchurCenteredGs2','ConservedSchurCenteredGs2']
vector=[r for r in native if not r.startswith('Conserved')]
parents={'SchurCentered':'Schur','SchurCenteredGrouped':'Schur','SchurCenteredGs2':'Schur','PreparedSchurCenteredGs2':'PreparedSchur','ConservedSchurCenteredGs2':'ConservedSchur'}
comparators={'SchurCentered':'SchurGs1','SchurCenteredGrouped':'SchurGs1','SchurCenteredGs2':'SchurGs2','PreparedSchurCenteredGs2':'PreparedSchurGs2','ConservedSchurCenteredGs2':'ConservedSchurGsAdaptive2'}
nf=['solver','cert','derivative','value'];vf=['case','guard','derivative','value']
na=[['Mass','Reduced5','Prepared5','Scalar5','Affine5'],['Max','Integral4'],native,['Box','Transport']]
va=[['weak','moderate','coupled'],['algebraic','cancellation'],vector,['Box','Transport']]
def read(path,marker,fields):
 t=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
 out={}
 for l in t.splitlines():
  if not (l.startswith(marker+' {') or l.startswith('test ') and (' ... '+marker+' {') in l):continue
  v=json.loads(l.split(marker+' ',1)[1]);k=tuple(v[f]for f in fields);assert k not in out;out[k]=v
 return out
def req(r):return sum(r['allocation'][k]for k in ('alloc','zeroed','realloc'))
def sweeps(r):return r['audit'].get('refinement_sweeps',{}).get('started',0)
def evidence(r):return r['result']['evidence'] if 'result'in r else r['evidence']
def vals(r):return [r['result'][k]for k in ['left_value','mid_value','right_value']] if 'result'in r else r['values']
def wd(v):return v[1]-v[0]
def inside(a,b):return b[0]<=a[0] and a[1]<=b[1]
def eligible(r):return evidence(r)['velocity'][0]>0
def work(r):return r['audit'].get('centered_residuals',{}).get('started',0)+r['audit'].get('centered_corrections',{}).get('started',0)
def obj(r):return (req(r),r['allocation']['requested_bytes'],work(r),sweeps(r),wd(evidence(r)['velocity']),max(map(wd,vals(r))))
def dom(a,b):return all(x<=y for x,y in zip(a,b)) and a!=b
def front(rows,group=None):
 ks=[k for k,r in rows.items()if eligible(r) and (group is None or k[:2]==group)]
 return [k for k in ks if not any(dom(obj(rows[q]),obj(rows[k]))for q in ks)]
def canonical(r):return json.dumps(r,sort_keys=True,separators=(',',':'),allow_nan=False)
def compare_old(old,new):
 assert set(old)<=set(new)
 for k,a in old.items():
  b=new[k];assert canonical({x:y for x,y in a.items()if x!='allocation'})==canonical({x:y for x,y in b.items()if x!='allocation'}),k
  assert req(b)<=req(a) and b['allocation']['requested_bytes']<=a['allocation']['requested_bytes'],k
 return {'count':len(old),'requests':[sum(req(r)for r in old.values()),sum(req(new[k])for k in old)],'bytes':[sum(r['allocation']['requested_bytes']for r in old.values()),sum(new[k]['allocation']['requested_bytes']for k in old)],'medians':[statistics.median(map(req,old.values())),statistics.median(req(new[k])for k in old)],'saved_range':[min(req(r)-req(new[k])for k,r in old.items()),max(req(r)-req(new[k])for k,r in old.items())]}
def inspect(rows):
 summary={}
 for name,parent in parents.items():
  ks=[k for k in rows if k[2]==name]
  if not ks:continue
  counts={'count':len(ks),'strict_velocity_narrowing':0,'transverse':0,'new_transverse_vs_parent':0,'narrower_than_gs':0,'wider_than_gs':0,'new_transverse_vs_gs':0}
  for k in ks:
   r=rows[k];b=rows[k[:2]+(parent,k[3])];j=rows[k[:2]+(comparators[name],k[3])]
   e,f=evidence(r),evidence(b)
   assert all(inside(x,y)for x,y in zip(e['reduced_gradient'],f['reduced_gradient'])),k
   assert inside(e['reduced_clock'],f['reduced_clock']) and inside(e['velocity'],f['velocity']),k
   assert all(inside(x,y)for x,y in zip(vals(r),vals(b))),k
   assert e['inverse_defect']==f['inverse_defect'] and e['reduced_lip']<=f['reduced_lip'],k
   assert {x:y for x,y in r['audit'].items()if x not in ('refinement_sweeps','centered_residuals','centered_corrections')}==b['audit'],k
   columns=b['audit']['reduced_columns']['returned']
   for field in ('centered_residuals','centered_corrections'):
    assert r['audit'][field]=={'started':columns,'returned':columns},(k,field)
   sw=r['audit'].get('refinement_sweeps',{'started':0,'returned':0})
   assert sw['started']==sw['returned']==(2*columns if name.endswith('Gs2') else 0),k
   counts['strict_velocity_narrowing']+=wd(e['velocity'])<wd(f['velocity'])
   counts['transverse']+=eligible(r)
   counts['new_transverse_vs_parent']+=eligible(r) and not eligible(b)
   counts['narrower_than_gs']+=wd(e['velocity'])<wd(evidence(j)['velocity'])
   counts['wider_than_gs']+=wd(e['velocity'])>wd(evidence(j)['velocity'])
   counts['new_transverse_vs_gs']+=eligible(r) and not eligible(j)
  summary[name]=counts
 return summary
def check_core(rows,cores):
 assert set(rows)==set(cores)
 zero=0;reports=[]
 for k,r in rows.items():
  c=cores[k];a=c['core'];o=c['report'];n=lambda x:sum(x[z]for z in ('alloc','zeroed','realloc'))
  assert c['queries']==vals(r),k
  assert n(a)+n(o)==req(r),k
  assert a['requested_bytes']+o['requested_bytes']==r['allocation']['requested_bytes'],k
  if not k[2].startswith('Prepared'):
   assert n(a)==a['requested_bytes']==0,k;zero+=1
  reports.append(n(o))
 return {'count':len(cores),'nonprepared_zero':zero,'report_requests_range':[min(reports),max(reports)]}
def record(k,r,fields):return dict(zip(fields,k),objectives=obj(r),velocity=evidence(r)['velocity'])
def replay(rows,axes,fields,grouped):
 rng=random.Random(271828 if not grouped else 123407)
 tree=UCT(lambda p:axes[len(p)] if len(p)<4 else [])
 bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};proposals={'structure':0,'mutation':0}
 lineage={**parents,'SchurRefine1':'Schur','SchurRefine2':'Schur','PreparedSchurRefine2':'PreparedSchur','ConservedSchurRefine2':'ConservedSchur','SchurGs1':'Schur','SchurGs2':'Schur','SchurGsAdaptive2':'Schur','PreparedSchurGs2':'PreparedSchur','ConservedSchurGsAdaptive2':'ConservedSchur'}
 for _ in range(1024):
  arm=bandit.pick(rng);proposals[arm]+=1
  if arm=='mutation' and seen:
   p=rng.choice(list(seen));root=lineage.get(p[2],p[2])
   n=[v for v in [root,*[m for m,b in lineage.items()if b==root]]if v!=p[2] and v in axes[2]]
   k=p[:2]+(rng.choice(n),p[3]) if n else tuple(tree.propose(rng))
  else:k=tuple(tree.propose(rng))
  fresh=k not in seen;seen[k]=rows[k];g=k[:2] if grouped else None
  good=eligible(rows[k]); nondom=good and k in front(seen,g)
  reward=fresh and nondom;bandit.update(arm,reward);tree.backup(k,float(reward))
  if good:archive.insert('/'.join(k[:3]),float(nondom),k)
 groups=sorted({k[:2]for k in rows}) if grouped else [None]
 strata=[]
 for g in groups:
  f=front(rows,g)
  choices={}
  if f:
   choices['resource_first']=record(min(f,key=lambda k:(obj(rows[k])[:4],obj(rows[k])[4:],k)),rows[min(f,key=lambda k:(obj(rows[k])[:4],obj(rows[k])[4:],k))],fields)
   tight=min(f,key=lambda k:(obj(rows[k])[4:],obj(rows[k])[:4],k));choices['width_first']=record(tight,rows[tight],fields)
  strata.append({'group':g,'qualified':sum(eligible(v) for k,v in rows.items()if g is None or k[:2]==g),'pareto_count':len(f),'covered':len(set(f)&seen.keys()),'pareto':[record(k,rows[k],fields)for k in f],'recommendations':choices})
 return {'unique':len(seen),'proposals':proposals,'posterior':bandit.ab,'pareto_count':sum(x['pareto_count']for x in strata),'pareto_covered':sum(x['covered']for x in strata),'strata':strata}
if sys.argv[1]=='generate':
 r=explore('Source-centered residual of implicit DAE guard derivative on full certified physical box, grouping identical remaining affine factors and collecting affine coefficients before directed interval evaluation, zero heap borrowed preconditioner and inline guard proof, centered correction with or without GS, compare Jacobi, original Schur, PreparedRhs, conservation-based Schur, RETRO polynomial transport. Preserve exact physical admission.',domain='numerical_analysis',max_routes=64)
 dump('HARNESS_ROUTES.json',r)
 designs=[]
 for axes,fields in [(na,nf),(va,vf)]:
  tree=UCT(lambda p:axes[len(p)]if len(p)<4 else [])
  rng=random.Random(1729);seen=[]
  for _ in range(4096):
   k=tuple(tree.propose(rng));fresh=k not in seen
   if fresh:seen.append(k)
   tree.backup(k,float(fresh))
   if len(seen)==len(list(itertools.product(*axes))):break
  designs.append({'fields':fields,'uct_proposals':[dict(zip(fields,k))for k in seen],'fixed_design':[dict(zip(fields,k),parent=parents.get(k[2]))for k in itertools.product(*axes)]})
 dump('CANDIDATES.json',{'manifest_files':len(m['files']),'abstract_routes':r['route_count'],'designs':designs,'mutation_edges':parents,'gs_comparators':comparators,'native_modules':['Mass RODAS','Reduced5 Reuse','PreparedRhs','DictionaryCompanion','Affine5','RETRO Lagrange5','Integral4 Bernstein','ConservedSchur','native phase atlas'],'hard_gate':'strict positive velocity and authentic certified slab','objectives':['requests','bytes','centered residual/correction boundaries','actual sweeps','velocity width','max endpoint/mid value width']})
 print('MathHarness',len(m['files']),r['route_count'],[(len(d['uct_proposals']),len(d['fixed_design'])) for d in designs])
else:
 prior=P.parent/'m2_dae_guard_poly_gs_20261126'
 n=read(P/'native.log','GUARD',nf);v=read(P/'vector.log','VECTOR',vf)
 assert set(n)==set(itertools.product(*na)) and set(v)==set(itertools.product(*va))
 n0=read(prior/'native.log.gz','GUARD',nf);v0=read(prior/'vector.log.gz','VECTOR',vf)
 result={'native_core':check_core(n,read(P/'native.log','CORE',nf)),'vector_core':check_core(v,read(P/'vector.log','VECTOR_CORE',vf)),'native_old':compare_old(n0,n),'vector_old':compare_old(v0,v),'native_rows':len(n),'vector_rows':len(v),'native_mutations':inspect(n),'vector_mutations':inspect(v),'native_search':replay(n,na,nf,False),'vector_search':replay(v,va,vf,True),'scope':'offline measured-table selection; no held-out generalization or default promotion'}
 dump('SELECTION.json',result)
 print(json.dumps({k:x for k,x in result.items() if k not in ['native_search','vector_search']},indent=2))
 for k in ['native_search','vector_search']:
  x=result[k];print(k,{a:b for a,b in x.items()if a!='strata'})
  for g in x['strata']:print(g['group'],g['qualified'],g['recommendations'])
