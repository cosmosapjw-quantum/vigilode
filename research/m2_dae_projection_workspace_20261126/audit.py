from pathlib import Path
from fractions import Fraction as Q
from math import prod

import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_scalar_affine_20261126'
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
init=load(P/'projection_portfolio.log','INIT');batch=load(P/'projection_portfolio.log','BATCH')
assert len(init)==144 and len(batch)==108
old_init=load(O/'scalar_portfolio.log.gz','INIT');assert len(old_init)==108
def local_key(r):
 d=r['detail'];return (d['solver'],d['cert'],d['mode'],d['position'])
expected=set(itertools.product(['Mass','Prepared5','Scalar5'],['Max','Integral4'],[0,1],['first','middle','last']))
previnit={(local_key(r),r['route']):r for r in old_init}
groups={};local=[];checks=collections.Counter()
for r in init:groups.setdefault(local_key(r),{})[r['route']]=r
assert set(groups)==expected
def enclosed(pair,x):assert Q(pair[0])<=x<=Q(pair[1]),(pair,x)
def compare(p,actual,zd,zt,zx,f,jac,root,ginv,gcond,tangent,residual):
 assert p['qualified'] and p['rank']==1
 enclosed(p['root_box'][0],root)
 assert abs(Q(p['z'][0])-root)<=Q(p['state_error_upper'])
 assert abs(residual)<=Q(p['residual_upper'])
 assert abs(tangent)<=Q(p['tangent_residual_upper'])
 assert Q(p['inverse_norm_upper'])>=ginv and Q(p['condition_upper'])>=gcond
 E=Q(p['derivative_error_upper'])
 arr=[(p['xdot'],f),(p['zdot'],[zd]),(p['zt'],[zt])]
 for k,v in enumerate(zx):arr.append((p['zx_columns'][k],[v]))
 arr.extend(zip(p['reduced_jacobian'],jac))
 for a,b in arr:
  assert len(a)==len(b)
  for u,v in zip(a,b):
   assert abs(Q(u)-v)<=E,(p['route'],u,v,float(E))
   checks['exact_derivative_components']+=1
 assert p['state_error_upper']<=1e-11 and p['residual_upper']<=1e-11 and p['derivative_error_upper']<=1e-6
 checks['exact_init_reports']+=1
for k,d in groups.items():
 assert set(d)=={'Reuse','PreparedBatch','ScalarRoot','AffineExact'}
 for route,r in d.items():
  detail=r['detail'];s=cells['alternating',k[0],k[1],'Correlated','AtlasReuse']['atlas']['phases'][k[2]]['slabs'][detail['slab']]
  assert s['certificate']['global_qualified'] and detail['time']==s['times'][0] and detail['x']==s['native']['y0'][:2]
  p=r['report'];u,v=map(Q,detail['x']);z=1-u-v;c=Q(detail['rate']);a=Q(.04)
  f1=-a*u+10000*v*z;f2=-f1-c*v*v
  j1=-a-10000*v;j2=10000*(z-v)
  actual=Q(p['z'][0])
  compare(p,actual,c*v*v,Q(0),[-Q(1),-Q(1)],[f1,f2],[[j1,j2],[-j1,-j2-2*c*v]],z,Q(1),Q(1),sum(map(Q,p['xdot']))+Q(p['zdot'][0]),u+v+actual-1)
  if route!='AffineExact':
   prev=previnit[k,route];assert p==prev['report'];checks['legacy_local_init_identical']+=1
  else:
   a=p['affine_root'];theta=(z+Q(1,4))/Q(3,2)
   assert a['multiplicity']==1 and a['kind']=='Up' and Q(a['exact_theta'][0])==Q(a['exact_theta'][1])==theta
   enclosed(a['theta'],theta);enclosed(a['time'],z);assert a['time']==p['root_box'][0]
   assert p['branch_authority']=='exact affine scalar root' and p['work']['dictionary_uses']==0 and p['work']['root_calls']==1
   checks['affine_root_receipts']+=1
  local.append({'detail':detail,'method':route,'allocation':r['allocation'],'reused_allocation':r['reused_allocation'],'requests':req(r['allocation']),'bytes':r['allocation']['requested_bytes'],'width':p['root_box'][0][1]-p['root_box'][0][0],'state_error':p['state_error_upper'],'derivative_error':p['derivative_error_upper'],'authority':p['branch_authority'],'work':p['work']})
general=load(P/'focused2.log','GENERAL');assert len(general)==8
for r in general:
 p=r['report'];sg=Q(r['sign']);x=Q(1,8);t=Q(1,4);a=1+t+x;z=x/a
 f=2*z-x;zx=(1+t)/a**2;zt=-x/a**2;zd=zt+zx*f
 v=Q(p['z'][0]);td=sg*(v+(v-1)*Q(p['xdot'][0])+a*Q(p['zdot'][0]))
 compare(p,v,zd,zt,[zx],[f],[[2*zx-1]],z,1/a,Q(1),td,sg*(a*v-x))
 if r['route']=='AffineExact':
  rr=p['affine_root'];theta=(z+Q(1,4))/Q(3,2)
  assert Q(rr['exact_theta'][0])==Q(rr['exact_theta'][1])==theta
  assert rr['kind']==('Up' if sg>0 else 'Down')
  checks['affine_root_receipts']+=1
for r in batch:
 assert r['matrix']==[1.] and r['values']==r['rhs'] and r['residual']==0.
 checks['prepared_scalar_batches']+=1
vbatch=load(P/'focused2.log','VECTOR_BATCH');assert len(vbatch)==3
for r in vbatch:
 a,b,c,d=map(Q,r['matrix']);det=a*d-b*c
 for rhs,sol in zip(r['rhs'],r['values']):
  e,f=map(Q,rhs);x,y=map(Q,sol)
  exact=((d*e-b*f)/det,(a*f-c*e)/det)
  assert max(abs(x-exact[0]),abs(y-exact[1]))<=Q(r['error_upper'])
  assert max(abs(a*x+b*y-e),abs(c*x+d*y-f))<=Q(r['residual'])
  checks['prepared_vector_rhs']+=1
red=load(P/'red.log','PROJ_RED_GREEN');green=load(P/'green.log','PROJ_RED_GREEN')
assert len(red)==len(green)==1
a,b=red[0],green[0];assert a['report']==b['report'] and req(a['allocation'])==158 and req(b['allocation'])<=145
# New affine driver receipts are independently audited before combining
# portfolios. New semantic receipts are NOT asserted equal to Scalar5.
aff=json.load(open(P/'AFFINE_AUDIT.json'));ar=load(P/'affine_training.log','RESOURCE');assert len(aff['rows'])==len(ar)==48
av={key(r):r for r in ar}
newcells=load(P/'affine_training.log','CELL');assert len(newcells)==48
for v in aff['rows']:
 k=key(v);r=av[k];cell=next(x for x in newcells if key(x)==k);assert v['solver']=='Affine5'
 w=cell.get('atlas',cell.get('joint'))['work'];w=w.get('phase',w.get('common'))
 solve=r['solve_alloc'];qa=r.get('centered',r.get('point'))['allocation']
 rows.append({**v,'driver_rhs':w['driver_rhs'],'projection_rhs':w['projection_rhs'],'solve_alloc':solve,'solve_requests':req(solve),'solve_bytes':solve['requested_bytes'],'query_alloc':qa,'query_requests':req(qa),'query_bytes':qa['requested_bytes'],'total_requests':req(solve)+req(qa),'total_bytes':solve['requested_bytes']+qa['requested_bytes'],'before_solve_requests':None,'before_solve_bytes':None,'requests_saved':None,'bytes_saved':None})
summary={'physical_receipts_identical':144,'work_and_query_metadata_identical':144,'component_receipts_identical':360,'refusal_receipts_identical':11,'new_affine_native_candidates':48,'local_init_reports':len(init),'workspace_comparisons':len(init),'checks':dict(checks),'red_green':{'before':red[0],'after':green[0]},'rows':rows,'components':comp,'refusals':refusals,'local_rows':local,'batch_rows':batch,'vector_batch_rows':vbatch,'general_rows':general,'affine_audit_counts':aff['counts'],'logical_sha256':hashlib.sha256(('\n'.join(old)+'\n').encode()).hexdigest(),'baseline_log_sha256':sha(O/'training.log.gz'),'raw_log_sha256':sha(P/'training.log'),'scope':'same-thread requested allocation counts; original 144 exact serialized CELL identity, new 48 independently audited native Affine5, exact local derivatives; no promotion or runtime/peak claim'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ['rows','components','refusals','local_rows','batch_rows','vector_batch_rows','general_rows','red_green']})
for m in ['Reuse','PreparedBatch','ScalarRoot','AffineExact']:
 a=[r for r in local if r['method']==m];print('local init',m,len(a),statistics.median(r['requests']for r in a),statistics.median(r['bytes']for r in a),statistics.median(r['width']for r in a),statistics.median(r['derivative_error']for r in a))
for mode in ['FreshFactor','SerialReuse','FaerBatch']:
 a=[r for r in batch if r['mode']==mode];print('scalar batch',mode,len(a),statistics.median(req(r['allocation'])for r in a),statistics.median(r['allocation']['requested_bytes']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 a=[r for r in rows if r['solver']==s];print('portfolio',s,len(a),sum(r['before_solve_requests']for r in a),sum(r['solve_requests']for r in a),sum(r['bytes_saved']for r in a),sum(r['before_query_requests']for r in a),sum(r['query_requests']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 for op in ['project','reconstruct','residual','guard_build','picard_tube']:
  a=[r for r in comp if r['solver']==s and r['operation']==op and r['cert']=='Integral4']
  if a:print('component',s,op,len(a),statistics.median(req(r['before'])for r in a),statistics.median(req(r['after'])for r in a))
for r in rows:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse':print('representative',{k:r[k]for k in ['solver','before_solve_requests','solve_requests','before_solve_bytes','solve_bytes','query_requests','last_width','attempts']})
