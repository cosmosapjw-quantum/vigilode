from pathlib import Path
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_reset_accounting_20261126'
def text(f):return gzip.decompress(f.read_bytes()).decode()if f.suffix=='.gz'else f.read_text()
def load(f,prefix):return [json.loads(l.split(prefix+' ',1)[1])for l in text(f).splitlines()if prefix+' {'in l]
def raw_cells(f):return [l.split('CELL ',1)[1]for l in text(f).splitlines()if 'CELL {'in l]
def req(a):return a['alloc']+a['zeroed']+a['realloc']
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
def index(rows):
 d={key(r):r for r in rows};assert len(d)==len(rows);return d
def rawindex(rows):
 d={key(json.loads(r)):r for r in rows};assert len(d)==len(rows);return d
old=rawindex(raw_cells(O/'training.log.gz')+raw_cells(P/'baseline_training.log'))
new=rawindex(raw_cells(P/'training.log')+raw_cells(P/'reduced_training.log'))
assert len(old)==len(new)==240 and old==new
prev_audit=rawindex(raw_cells(P/'baseline_audited.log'))
after_audit=rawindex(raw_cells(P/'reduced_audited.log'))
assert len(prev_audit)==len(after_audit)==48 and prev_audit==after_audit
def audits(f):
 a=load(f,'ATLAS_AUDIT')+load(f,'CHAIN_AUDIT');return index(a)
assert audits(P/'baseline_audited.log')==audits(P/'reduced_audited.log')
cells={k:json.loads(v)for k,v in new.items()}
ao=index(load(O/'training.log.gz','RESOURCE')+load(P/'baseline_training.log','RESOURCE'))
bo=index(load(P/'training.log','RESOURCE')+load(P/'reduced_training.log','RESOURCE'))
assert set(ao)==set(bo)==set(cells)
prev=index(json.load(open(O/'AUDIT.json'))['rows'])
def without_alloc(r):
 r=dict(r);r.pop('solve_alloc')
 for n in ['horner','centered','point']:
  if n in r:r[n]={k:v for k,v in r[n].items()if k!='allocation'}
 return r
rows=[]
for k,n in bo.items():
 o=ao[k];assert without_alloc(o)==without_alloc(n),k
 cell=cells[k];a=cell.get('atlas');v=a or cell['joint'];w=v['work']['phase'if a else'common']
 av=n['solve_alloc'];qa=n['centered']if 'centered'in n else n['point'];qalloc=qa['allocation']
 last_width=None
 if a:
  last=next((q['centered']for q in v['queries']if q['centered']['t']==0.002),None)
  if last:last_width=max(b-a for a,b in last['enclosure'])
 else:
  last=next((q for e in v['epochs']for q in e['queries']if q['t']==0.002),None)
  if last:last_width=2*last['physical_error_upper']
 r={**{f:k[i]for i,f in enumerate(fields)},'stop':v['stop'],'qualified':v['stop']=='Horizon','attempts':w['native_attempts'],'driver_rhs':w['driver_rhs'],'projection_rhs':w['projection_rhs'],'cache_hits':w.get('cache_hits',0),'last_width':last_width,'solve_requests':req(av),'solve_bytes':av['requested_bytes'],'query_requests':req(qalloc),'query_bytes':qalloc['requested_bytes'],'total_requests':req(av)+req(qalloc),'total_bytes':av['requested_bytes']+qalloc['requested_bytes'],'before_solve_requests':req(o['solve_alloc']),'before_solve_bytes':o['solve_alloc']['requested_bytes'],'before_query_requests':req(o.get('centered',o.get('point'))['allocation'])}
 if k in prev:
  for f in ['qualified','attempts','driver_rhs','projection_rhs','last_width','cache_hits']:assert r[f]==prev[k][f],(k,f,r[f],prev[k][f])
 r['requests_saved']=r['before_solve_requests']-r['solve_requests'];r['bytes_saved']=r['before_solve_bytes']-r['solve_bytes']
 rows.append(r)
# New route is replayed against independently frozen actual old callback path;
# all old source/physical receipts remain identical.
red=load(P/'red.log','CALLBACK_RED_GREEN');green=load(P/'green.log','CALLBACK_RED_GREEN')
assert len(red)==len(green)==1
a,b=red[0],green[0]
assert all(a[k]==b[k]for k in ['rhs','jac','time'])
assert [req(a[k+'_alloc'])for k in ['rhs','jac','time']]==[278,278,316]
assert [req(b[k+'_alloc'])for k in ['rhs','jac','time']]==[0,0,0]
local=load(P/'focused.log','CALLBACK')
assert len(local)==5 and all(req(v[k+'_alloc'])==0 for v in local for k in ['rhs','jac','time'])
eq=load(P/'focused.log','SCALAR_EQUIVALENCE');assert eq==[{'qualified':311,'refused':787,'extra':5}]
a=load(O/'components.log.gz','COMPONENT');b=load(P/'components.log','COMPONENT')
assert len(a)==len(b)==360
for x,y in zip(a,b):
 assert {k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}
 if y['operation'] in ['mass_rhs_callback','mass_jacobian_callback','guard_value']:assert req(y['allocation'])==0
a=load(O/'refusals.log.gz','REFUSAL');b=load(P/'refusals.log','REFUSAL')
assert len(a)==len(b)==11
for x,y in zip(a,b):assert {k:v for k,v in x.items()if k!='alloc'}=={k:v for k,v in y.items()if k!='alloc'}
audit=audits(P/'reduced_audited.log')
stats=collections.Counter()
for k,r in audit.items():
 for n in r['audit']['native']:
  v=n['native'];s=v['stats'];assert s is not None and s['attempts']==s['accepted_steps']+s['rejected_steps']
  stats['native_runs']+=1;stats['attempts']+=s['attempts'];stats['accepted']+=s['accepted_steps'];stats['rejected']+=s['rejected_steps']
  stats['point_projection_calls']+=v['point_projection']['calls']['started'];stats['point_projection_rhs']+=v['point_projection']['rhs_requested']
  # Not a claim that every point projection is a hot callback.
  for f in ['rhs','jacobian','time_derivative']:pass
s={'physical_receipts_identical':240,'legacy_physical_receipts_identical':192,'reduced_physical_receipts_identical':48,'reduced_audited_receipts_identical':48,'component_receipts_identical':360,'refusal_receipts_identical':11,'callback_before':red[0],'callback_after':green[0],'additional_noalloc_callback_clients':len(local),'equivalence':eq[0],'reduced_audit_counts':dict(stats),'rows':rows,'scope':'successful scalar Reuse Reduced3/Reduced5 callbacks only; error diagnostics allocate; no full VG1 claim; exact old receipts and measured allocation comparison, development only'}
(P/'AUDIT.json').write_text(json.dumps(s,indent=2,allow_nan=False)+'\n')
print('physical receipts',240,'audited',48,'component',360,'refusals',11)
print('audit counts',dict(stats))
for sol in ['Mass','Reduced5','Prepared5','Scalar5','Affine5']:
 r=[v for v in rows if v['solver']==sol]
 print(sol,'rows',len(r),'Horizon',sum(v['qualified']for v in r),'before',sum(v['before_solve_requests']for v in r),'after',sum(v['solve_requests']for v in r),'before_bytes',sum(v['before_solve_bytes']for v in r),'after_bytes',sum(v['solve_bytes']for v in r))
print('reduced representative',next(r for r in rows if r['case']=='alternating'and r['solver']=='Reduced5'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']=='AtlasReuse'))
