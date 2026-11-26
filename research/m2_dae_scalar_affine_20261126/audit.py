from pathlib import Path
from fractions import Fraction as Q
from math import prod

import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_polynomial_scratch_20261126'
def text(f):return gzip.decompress(f.read_bytes()).decode()if f.suffix=='.gz'else f.read_text()
def load(f,prefix):return [json.loads(l.split(prefix+' ',1)[1])for l in text(f).splitlines()if prefix+' {'in l]
def raw_cells(f):return [l.split('CELL ',1)[1]for l in text(f).splitlines()if 'CELL {'in l]
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
old=raw_cells(O/'training.log.gz');new=raw_cells(P/'training.log')
assert len(old)==len(new)==144 and old==new
cells={key(r):r for r in map(json.loads,new)}
a=load(O/'training.log.gz','RESOURCE');b=load(P/'training.log','RESOURCE')
assert len(a)==len(b)==144
ao={key(r):r for r in a};bo={key(r):r for r in b};assert set(ao)==set(bo)
base=json.load(open(O/'AUDIT.json'));prev={key(r):r for r in base['rows']};rows=[]
def without_alloc(r):
 r=dict(r);r.pop('solve_alloc')
 for n in ['horner','centered','point']:
  if n in r:r[n]={k:v for k,v in r[n].items()if k!='allocation'}
 return r
for k,o in ao.items():
 n=bo[k];v=prev[k];assert without_alloc(o)==without_alloc(n),k
 av=n['solve_alloc'];qa=n['centered']if 'centered'in n else n['point'];qalloc=qa['allocation']
 r={**v,'solve_alloc':av,'solve_requests':req(av),'solve_bytes':av['requested_bytes'],'query_alloc':qalloc,'query_requests':req(qalloc),'query_bytes':qalloc['requested_bytes'],'total_requests':req(av)+req(qalloc),'total_bytes':av['requested_bytes']+qalloc['requested_bytes'],'horner':n.get('horner'),'centered':n.get('centered'),'before_solve_requests':req(o['solve_alloc']),'before_solve_bytes':o['solve_alloc']['requested_bytes'],'before_query_requests':v['query_requests'],'before_query_bytes':v['query_bytes']}
 r['requests_saved']=r['before_solve_requests']-r['solve_requests']
 r['bytes_saved']=r['before_solve_bytes']-r['solve_bytes']
 rows.append(r)
a=load(O/'components.log.gz','COMPONENT');b=load(P/'components.log','COMPONENT')
assert len(a)==len(b)==360
comp=[]
for x,y in zip(a,b):
 assert {k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}
 if y['operation'] in ['mass_rhs_callback','mass_jacobian_callback','guard_value']:assert req(y['allocation'])==0
 comp.append({'operation':y['operation'],'solver':y['solver'],'cert':y['cert'],'detail':y['detail'],'before':x['allocation'],'after':y['allocation']})
a=load(O/'refusals.log.gz','REFUSAL');b=load(P/'refusals.log','REFUSAL')
assert len(a)==len(b)==11
for x,y in zip(a,b):assert {k:v for k,v in x.items()if k!='alloc'}=={k:v for k,v in y.items()if k!='alloc'}
refusals=[{'case':x['case'],'stop':x['stop'],'before':x['alloc'],'after':y['alloc']}for x,y in zip(a,b)]
frozen=load(P/'scalar_portfolio.log','FROZEN');root_rows=load(P/'scalar_portfolio.log','ROOT');affine=load(P/'scalar_portfolio.log','AFFINE');init=load(P/'scalar_portfolio.log','INIT')
assert len(frozen)==36 and len(root_rows)==180 and len(affine)==36 and len(init)==108
def local_key(r):
 d=r['detail'];return (d['solver'],d['cert'],d['mode'],d['position'])
expected=set(itertools.product(['Mass','Prepared5','Scalar5'],['Max','Integral4'],[0,1],['first','middle','last']))
ff={local_key(r):r for r in frozen};aa={local_key(r):r for r in affine}
assert set(ff)==set(aa)==expected
roots={};inits={}
for r in root_rows:roots.setdefault(local_key(r),{})[r['method']]=r
for r in init:inits.setdefault(local_key(r),{})[r['route']]=r
assert set(roots)==set(inits)==expected
tol=Q(2)**-48
root_checks=0;init_checks=0;local=[]
def encl(pair,x):assert Q(pair[0])<=x<=Q(pair[1]),(pair,x)
def check_root(r,theta,z,clock,singleton=False):
 l,h=map(Q,r['exact_theta']);a,w=clock
 assert 0<=l<=theta<=h<=1
 assert l<theta or l==h==theta
 if singleton:assert l==h==theta
 assert r['multiplicity']==1 and r['kind']=='Up'
 encl(r['theta'],l);encl(r['theta'],h)
 encl(r['time'],a+w*l);encl(r['time'],a+w*h);encl(r['time'],z)
 assert Q(r['theta'][1])-Q(r['theta'][0])<=tol
for k,f in ff.items():
 d=f['detail'];s=cells['alternating',k[0],k[1],'Correlated','AtlasReuse']['atlas']['phases'][k[2]]['slabs'][d['slab']]
 assert s['certificate']['global_qualified'] and d['time']==s['times'][0] and d['x']==s['native']['y0'][:2]
 x,y=map(Q,d['x']);clock=list(map(Q,f['clock']));assert clock==[-Q(1,4),Q(3,2)]
 c,a=map(Q,f['coefficients']);assert c==x+y-1+clock[0] and a==clock[1]
 theta=-c/a;z=1-x-y;assert 0<theta<1
 assert set(roots[k])=={'Sturm','NativeBernstein','DictionaryBernstein','Companion','DictionaryCompanion'}
 for method,r in roots[k].items():
  p=r['report'];assert p['route']==method and len(p['roots'])==1 and p['work']['companion_attempts']==0
  v=p['roots'][0];check_root(v,theta,z,clock);root_checks+=1
  assert p['work']['sturm_chains']==1
  if method.startswith('Dictionary'):assert p['work']['dictionary_uses']==1
  local.append({'detail':d,'method':method,'allocation':r['allocation'],'requests':req(r['allocation']),'bytes':r['allocation']['requested_bytes'],'width':p['max_theta_width'],'work':p['work'],'singleton':False})
 r=aa[k];v=r['root'];check_root(v,theta,z,clock,True);root_checks+=1
 local.append({'detail':d,'method':'AffineExactSingleton','allocation':r['allocation'],'requests':req(r['allocation']),'bytes':r['allocation']['requested_bytes'],'width':v['theta'][1]-v['theta'][0],'work':None,'singleton':True})
 assert set(inits[k])=={'Reuse','PreparedBatch','ScalarRoot'}
 for route,r in inits[k].items():
  p=r['report'];assert p['qualified'] and p['rank']==1 and p['x']==d['x'] and p['route']==route
  encl(p['root_box'][0],z)
  assert abs(Q(p['z'][0])-z)<=Q(p['state_error_upper'])
  assert abs(x+y+Q(p['z'][0])-1)<=Q(p['residual_upper'])
  assert p['state_error_upper']<=1e-11 and p['residual_upper']<=1e-11
  if route=='ScalarRoot':
   assert p['branch_authority']=='exact scalar root count'
   assert p['root_box'][0]==roots[k]['DictionaryCompanion']['report']['roots'][0]['time']
   assert p['work']['root_calls']==1 and p['work']['dictionary_uses']==1
  init_checks+=1
red=load(P/'genuine_red.log','COST_RED_GREEN');green=load(P/'green.log','COST_RED_GREEN')
assert len(red)==len(green)==1
a,b=red[0],green[0]
assert all(a[k]==b[k]for k in ['coefficients','clock','root','init','composition'])
assert req(a['isolate'])==1852 and req(b['isolate'])<1000 and req(b['project'])<1400
summary={'physical_receipts_identical':144,'work_and_query_metadata_identical':144,'component_receipts_identical':360,'refusal_receipts_identical':11,'frozen_native_states':len(frozen),'root_candidates':len(local),'exact_root_checks':root_checks,'exact_init_checks':init_checks,'red_green_identical_root_init':True,'red_green':{'before':a,'after':b},'rows':rows,'components':comp,'refusals':refusals,'local_rows':local,'frozen_rows':frozen,'init_rows':init,'logical_sha256':hashlib.sha256(('\n'.join(old)+'\n').encode()).hexdigest(),'baseline_log_sha256':sha(O/'training.log.gz'),'raw_log_sha256':sha(P/'training.log'),'scope':'same-thread requested heap counts; exact frozen polynomial and independent Fraction root, multiplicity and enclosure checks; singleton diagnostic not substituted for legacy scalar initialization'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ['rows','components','refusals','local_rows','frozen_rows','init_rows','red_green']})
for m in ['Sturm','NativeBernstein','DictionaryBernstein','Companion','DictionaryCompanion','AffineExactSingleton']:
 a=[r for r in local if r['method']==m];print('local root',m,len(a),'requests',statistics.median(r['requests']for r in a),'bytes',statistics.median(r['bytes']for r in a),'width',statistics.median(r['width']for r in a))
for route in ['Reuse','PreparedBatch','ScalarRoot']:
 a=[r for r in init if r['route']==route];print('local init',route,len(a),'requests',statistics.median(req(r['allocation'])for r in a),'bytes',statistics.median(r['allocation']['requested_bytes']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 a=[r for r in rows if r['solver']==s];print('portfolio',s,len(a),sum(r['before_solve_requests']for r in a),sum(r['solve_requests']for r in a),sum(r['bytes_saved']for r in a),sum(r['before_query_requests']for r in a),sum(r['query_requests']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 for op in ['project','reconstruct','residual','guard_build','picard_tube']:
  a=[r for r in comp if r['solver']==s and r['operation']==op and r['cert']=='Integral4']
  if a:print('component',s,op,len(a),statistics.median(req(r['before'])for r in a),statistics.median(req(r['after'])for r in a))
for r in rows:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse':print('representative',{k:r[k]for k in ['solver','before_solve_requests','solve_requests','before_solve_bytes','solve_bytes','query_requests','last_width','attempts']})
