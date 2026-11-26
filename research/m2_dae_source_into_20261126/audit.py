from pathlib import Path
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_resource_baseline_20261126'
def load(f,prefix):
 text=gzip.decompress(f.read_bytes()).decode()if f.suffix=='.gz'else f.read_text()
 return [json.loads(l.split(prefix+' ',1)[1])for l in text.splitlines()if prefix+' {'in l]
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
old=load(O/'training.log.gz','CELL');new=load(P/'training.log','CELL')
assert len(old)==len(new)==144 and old==new
a=load(O/'training.log.gz','RESOURCE');b=load(P/'training.log','RESOURCE')
assert len(a)==len(b)==144
ao={key(r):r for r in a};bo={key(r):r for r in b};assert set(ao)==set(bo)
base=json.load(open(O/'AUDIT.json'));prev={key(r):r for r in base['rows']};rows=[]
for k,o in ao.items():
 n=bo[k];v=prev[k]
 assert {j:x for j,x in o.items()if j!='solve_alloc'}=={j:x for j,x in n.items()if j!='solve_alloc'},k
 av=n['solve_alloc'];q=v['query_alloc']
 r={**v,'solve_alloc':av,'solve_requests':req(av),'solve_bytes':av['requested_bytes'],'total_requests':req(av)+req(q),'total_bytes':av['requested_bytes']+q['requested_bytes'],'before_solve_requests':req(o['solve_alloc']),'before_solve_bytes':o['solve_alloc']['requested_bytes']}
 r['requests_saved']=r['before_solve_requests']-r['solve_requests']
 r['bytes_saved']=r['before_solve_bytes']-r['solve_bytes']
 rows.append(r)
# Authenticate unchanged source and proof authority through matching actual component receipt metadata.
a=load(O/'components.log.gz','COMPONENT');b=load(P/'components.log','COMPONENT')
assert len(a)==len(b)==360
comp=[]
for x,y in zip(a,b):
 assert {k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}
 if y['operation'] in ['mass_rhs_callback','mass_jacobian_callback']:assert req(y['allocation'])==0
 comp.append({'operation':y['operation'],'solver':y['solver'],'cert':y['cert'],'detail':y['detail'],'before':x['allocation'],'after':y['allocation']})
a=load(O/'refusals.log.gz','REFUSAL');b=load(P/'refusals.log','REFUSAL')
assert len(a)==len(b)==11
for x,y in zip(a,b):
 assert {k:v for k,v in x.items()if k!='alloc'}=={k:v for k,v in y.items()if k!='alloc'}
source=load(P/'source_portfolio.log','SOURCE');assert len(source)==36
groups={}
for r in source:
 groups.setdefault((r['case'],r['rate'],r['rtol']),{})[r['method']]=r
assert len(groups)==12
for k,d in groups.items():
 assert set(d)=={'LegacyVec','StackInto','ReusedInto'}
 a=d['LegacyVec'];assert all(req(c)>0 for c in a['callbacks'])
 for m in ['StackInto','ReusedInto']:
  b=d[m];assert all(req(c)==0 for c in b['callbacks'])
  assert all(a[x]==b[x]for x in ['stats','final_state','dense','callback_points','bit_checks'])
  assert req(b['alloc'])<req(a['alloc'])
summary={'physical_receipts_identical':144,'resource_counter_and_query_receipts_identical':144,'component_receipts_identical':360,'refusal_receipts_identical':11,'source_driver_groups':12,'source_driver_calls':36,'source_callback_evaluations':sum(r['callback_points']*3 for r in source),'zeroalloc_callback_evaluations':sum(r['callback_points']*3 for r in source if r['method']!='LegacyVec'),'source_callback_bit_checks':sum(r['bit_checks']for r in source),'rows':rows,'components':comp,'refusals':load(P/'refusals.log','REFUSAL'),'source_rows':source,'baseline_source':sha(O/'training.log.gz'),'raw_source':sha(P/'training.log'),'scope':'same-thread requested allocation counts, no timing or peak memory; identical physical receipts, old Vec reference retained; mass time actual measured in source_portfolio, prior isolated mass_time_values is unchanged Vec reference'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ('rows','components','refusals','source_rows')})
for method in ['LegacyVec','StackInto','ReusedInto']:
 r=[v for v in source if v['method']==method];print(method,'calls',len(r),'callback points',sum(v['callback_points']for v in r),'alloc requests total',sum(req(v['alloc'])for v in r),'bytes',sum(v['alloc']['requested_bytes']for v in r),'callback requests',sum(req(c)for v in r for c in v['callbacks']))
for s in ['Mass','Prepared5','Scalar5']:
 r=[x for x in rows if x['solver']==s];print('portfolio',s,len(r),sum(x['before_solve_requests']for x in r),sum(x['solve_requests']for x in r),sum(x['bytes_saved']for x in r))
for r in rows:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse':
  print('representative',{k:r[k]for k in ['solver','before_solve_requests','solve_requests','before_solve_bytes','solve_bytes','attempts','last_width']})
