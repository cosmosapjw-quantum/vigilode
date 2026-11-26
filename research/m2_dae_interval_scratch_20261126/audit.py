from pathlib import Path
from fractions import Fraction as Q
from math import prod
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_source_into_20261126'
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
native=load(P/'interval_portfolio.log','INTERVAL');generic=load(P/'focused2.log','GENERIC')+load(P/'focused2.log','TINY')
assert len(native)==36 and len(generic)==16
assert {(r['detail']['solver'],r['detail']['cert'],r['detail']['mode'],r['detail']['position'])for r in native}==set(itertools.product(['Mass','Prepared5','Scalar5'],['Max','Integral4'],[0,1],['first','middle','last']))
for r in native:
 d=r['detail'];p=cells['alternating',d['solver'],d['cert'],'Correlated','AtlasReuse']['atlas']['phases'][d['mode']]
 assert p['mode']==d['mode'];s=p['slabs'][d['slab']];c=s['certificate']
 assert c['global_qualified'] and r['t']==s['times']==d['native_times']
 assert r['y']==c['x_range']+c['branch_root_box']
# Independent exact rational point evaluation and derivative formula,
# not merely comparing two implementations of interval operations.
def evaluate_exact(source,t,y):
 n=len(y);allrows=source['f']+source['g'];v=[];j=[]
 for eq in allrows:
  value=Q(0);row=[Q(0)]*(n+1)
  for term in eq['terms']:
   c=Q(term['coefficient']);factors=term['factors']
   vs=[Q(f['constant'])+Q(f['time'])*t+sum(Q(w)*y[i]for i,w in f['state'])for f in factors]
   value+=c*prod(vs)
   # exact arithmetic permits grouping repeated derivative entries;
   # independently computed prefix/suffix products, no interval code.
   pre=[Q(1)]
   for x in vs:pre.append(pre[-1]*x)
   suf=[Q(1)]*(len(vs)+1)
   for i in reversed(range(len(vs))):suf[i]=suf[i+1]*vs[i]
   for i,f in enumerate(factors):
    d=c*pre[i]*suf[i+1]
    row[0]+=d*Q(f['time'])
    for k,w in f['state']:row[k+1]+=d*Q(w)
  v.append(value);j.append(row)
 return v,j
exact_count=0;sample_count=0;bit_count=0
for r in native+generic:
 assert req(r['into'])==0 and req(r['owned'])<req(r['legacy'])
 bb=[list(map(Q,r['t']))]+[list(map(Q,v))for v in r['y']]
 n=len(r['y'])
 assert r['bit_checks']==4*(n+n*(n+1));bit_count+=r['bit_checks']
 samples=[[b[k]for b in bb]for k in (0,1)]
 samples += [[sum(b)/2 for b in bb],[b[i%2]for i,b in enumerate(bb)],[b[1-i%2]for i,b in enumerate(bb)]]
 for t,*y in samples:
  v,j=evaluate_exact(r['source'],t,y)
  for x,b in zip(v,r['values']):assert Q(b[0])<=x<=Q(b[1]),(r['detail'],x,b);exact_count+=1
  for row,br in zip(j,r['jacobian']):
   for x,b in zip(row,br):assert Q(b[0])<=x<=Q(b[1]),(r['detail'],x,b);exact_count+=1
  sample_count+=1
summary={'physical_receipts_identical':144,'work_and_query_metadata_identical':144,'component_receipts_identical':360,'refusal_receipts_identical':11,'native_interval_boxes':len(native),'generic_interval_boxes':len(generic),'interval_endpoint_bit_checks':bit_count,'exact_rational_sample_points':sample_count,'exact_rational_component_checks':exact_count,'rows':rows,'components':comp,'refusals':refusals,'interval_rows':native,'generic_interval_rows':generic,'logical_sha256':hashlib.sha256(('\n'.join(old)+'\n').encode()).hexdigest(),'baseline_log_sha256':sha(O/'training.log.gz'),'raw_log_sha256':sha(P/'training.log'),'scope':'same-thread allocation requests, no peak/RSS/time; full CELL text identity; exact rational finite samples not proof of entire enclosure; interval endpoint identity against frozen old evaluator; proof outputs remain owned'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ['rows','components','refusals','interval_rows','generic_interval_rows']})
for k in ['legacy','owned','into']:
 print('interval',k,len(native),sum(req(x[k])for x in native),sum(x[k]['requested_bytes']for x in native))
for s in ['Mass','Prepared5','Scalar5']:
 a=[r for r in rows if r['solver']==s];print('portfolio',s,len(a),sum(r['before_solve_requests']for r in a),sum(r['solve_requests']for r in a),sum(r['bytes_saved']for r in a),sum(r['before_query_requests']for r in a),sum(r['query_requests']for r in a))
for s in ['Mass','Prepared5','Scalar5']:
 for op in ['project','reconstruct','residual','guard_build','picard_tube']:
  a=[r for r in comp if r['solver']==s and r['operation']==op and r['cert']=='Integral4']
  if a:print('component',s,op,len(a),statistics.median(req(r['before'])for r in a),statistics.median(req(r['after'])for r in a))
for r in rows:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse':print('representative',{k:r[k]for k in ['solver','before_solve_requests','solve_requests','before_solve_bytes','solve_bytes','query_requests','last_width','attempts']})
