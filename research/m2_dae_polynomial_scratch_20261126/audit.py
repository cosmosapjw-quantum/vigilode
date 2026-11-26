from pathlib import Path
from fractions import Fraction as Q
from math import prod
from structural import native,ev
from exact_native import exact_cells,counts
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_interval_scratch_20261126'
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
 if y['operation'] in ['mass_rhs_callback','mass_jacobian_callback']:assert req(y['allocation'])==0
 comp.append({'operation':y['operation'],'solver':y['solver'],'cert':y['cert'],'detail':y['detail'],'before':x['allocation'],'after':y['allocation']})
a=load(O/'refusals.log.gz','REFUSAL');b=load(P/'refusals.log','REFUSAL')
assert len(a)==len(b)==11
for x,y in zip(a,b):assert {k:v for k,v in x.items()if k!='alloc'}=={k:v for k,v in y.items()if k!='alloc'}
refusals=[{'case':x['case'],'stop':x['stop'],'before':x['alloc'],'after':y['alloc']}for x,y in zip(a,b)]
res=load(P/'polynomial_portfolio.log','RESIDUAL');guards=load(P/'polynomial_portfolio.log','GUARD');degree=load(P/'polynomial_portfolio.log','DEGREE')
assert len(res)==108 and len(guards)==216 and len(degree)==36
def local_key(r):
 d=r['detail'];return (d['solver'],d['original'],d['mode'],d['position'])
groups={};gs={}
for r in res:groups.setdefault(local_key(r),{})[r['route']]=r
for r in guards:gs.setdefault(local_key(r),[]).append(r)
expected=set(itertools.product(['Mass','Prepared5','Scalar5'],['Max','Integral4'],[0,1],['first','middle','last']))
assert set(groups)==set(gs)=={local_key(r)for r in degree}==expected
local=[];guard_count=0;matching=0
for k,d in groups.items():
 assert set(d)=={'Max','Integral4','Integral16'}
 detail=d['Max']['detail'];s=cells['alternating',k[0],k[1],'Correlated','AtlasReuse']['atlas']['phases'][k[2]]['slabs'][detail['slab']]
 assert s['times']==detail['times']
 original=s['certificate'];assert original['global_qualified']
 p0=native(s)[:2]
 for route,r in d.items():
  c=r['certificate'];assert c['global_qualified']and c['incoming']==original['incoming']
  if route==k[1]:assert c==original;matching+=1
  exact_cells({**s,'certificate':c},p0,int(detail['rate']),route)
  local.append({'detail':detail,'route':route,'allocation':r['allocation'],'requests':req(r['allocation']),'bytes':r['allocation']['requested_bytes'],'outgoing':c['outgoing'],'defect_integral':sum(x['residual_integral']for x in c['conservation']['cells'])})
 assert len(gs[k])==6
 for g in gs[k]:
  assert req(g['query'])==0 and g['radius']==original['whole_x_error']
  t=Q(g['time']);a,b=map(Q,s['times']);assert a<=t<=b
  v=ev(p0[1],(t-a)/(b-a));e=Q(g['radius']);q=Q(1,16384)
  lo,hi=v-e,v+e
  if g['route']=='Box':
   lo=max(lo,Q(original['x_range'][1][0]));hi=min(hi,Q(original['x_range'][1][1]))
  assert lo<=hi and Q(g['value'][0])<=q-hi<=q-lo<=Q(g['value'][1]),(k,g)
  guard_count+=1
assert matching==36 and all(req(r['allocation'])==0 for r in degree)
red=load(P/'red.log','GUARD_RED_GREEN');green=load(P/'green.log','GUARD_RED_GREEN')
assert len(red)==len(green)==2
assert [{k:v for k,v in r.items()if k!='allocation'}for r in red]==[{k:v for k,v in r.items()if k!='allocation'}for r in green]
assert all(req(r['allocation'])==0 for r in green)
for r in comp:
 if r['operation']=='guard_value':assert req(r['after'])==0
summary={'physical_receipts_identical':144,'work_and_query_metadata_identical':144,'component_receipts_identical':360,'refusal_receipts_identical':11,'local_certificate_candidates':len(local),'matching_local_certificates':matching,'exact_cell_checks':counts['exact_cell_checks'],'guard_query_checks':guard_count,'degree16_noalloc_queries':len(degree),'red_green_same_values':2,'rows':rows,'components':comp,'refusals':refusals,'local_rows':local,'guard_rows':guards,'logical_sha256':hashlib.sha256(('\n'.join(old)+'\n').encode()).hexdigest(),'baseline_log_sha256':sha(O/'training.log.gz'),'raw_log_sha256':sha(P/'training.log'),'scope':'same-thread requested heap counts, no peak/time; exact rational native residual and guard comparator bounds; local candidates not new full chain authority'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ['rows','components','refusals','local_rows','guard_rows']})
for route in ['Max','Integral4','Integral16']:
 a=[r for r in local if r['route']==route];print('local',route,len(a),'requests',statistics.median(r['requests']for r in a),'bytes',statistics.median(r['bytes']for r in a),'outgoing',statistics.median(r['outgoing']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 a=[r for r in rows if r['solver']==s];print('portfolio',s,len(a),sum(r['before_solve_requests']for r in a),sum(r['solve_requests']for r in a),sum(r['bytes_saved']for r in a),sum(r['before_query_requests']for r in a),sum(r['query_requests']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 for op in ['project','reconstruct','residual','guard_build','picard_tube']:
  a=[r for r in comp if r['solver']==s and r['operation']==op and r['cert']=='Integral4']
  if a:print('component',s,op,len(a),statistics.median(req(r['before'])for r in a),statistics.median(req(r['after'])for r in a))
for r in rows:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse':print('representative',{k:r[k]for k in ['solver','before_solve_requests','solve_requests','before_solve_bytes','solve_bytes','query_requests','last_width','attempts']})
