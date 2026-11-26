from pathlib import Path
from collections import Counter,defaultdict
import json,gzip,hashlib,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_phase_atlas_20261126'
def load(f,prefix):
 text=gzip.decompress(f.read_bytes()).decode()if f.suffix=='.gz'else f.read_text()
 return [json.loads(l.split(prefix+' ',1)[1])for l in text.splitlines()if prefix+' {'in l]
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
def requests(a):return a['alloc']+a['zeroed']+a['realloc']
def total_alloc(a,b):return {k:a[k]+b[k]for k in a}
def size(a):return a['requested_bytes']
def digest(f):return hashlib.sha256(f.read_bytes()).hexdigest()
old=load(O/'training.log.gz','CELL');new=load(P/'training.log','CELL');rep=load(P/'replay.log','CELL')
resources=load(P/'training.log','RESOURCE');repeat=load(P/'replay.log','RESOURCE')
assert len(old)==len(new)==len(rep)==len(resources)==len(repeat)==144
assert old==new==rep
rs={key(r):r for r in resources};re={key(r):r for r in repeat};assert len(rs)==len(re)==144
prev=json.load(open(O/'AUDIT.json'));old_a={key(r):r for r in prev['rows']};assert len(old_a)==144
out=[];counts=Counter();vary=[]
for r in new:
 k=key(r);a=rs[k];b=re[k];v=old_a[k];run=r.get('atlas')or r['joint'];atlas='atlas'in r
 w=run['work']['phase']if atlas else run['work']['common']
 assert v['qualified']==(run['stop']=='Horizon')
 assert v['attempts']==w['native_attempts'];assert w['native_attempts']<=w['charged_native_attempts']
 assert w['charged_native_attempts']<=4096 and w['charged_scan_slabs']<=512
 assert a['driver_stats']==b['driver_stats']
 if a==b:counts['resource_identical']+=1
 else:
  vary.append({'key':list(k),'first':{q:a[q]for q in a if q not in fields+['driver_stats']},'repeat':{q:b[q]for q in b if q not in fields+['driver_stats']}})
  counts['resource_different']+=1
 known=[s for s in a['driver_stats']if s is not None]
 assert len(known)<=w['native_runs'];unknown=w['native_runs']-len(known)
 cnt=Counter()
 for s in known:
  assert s['attempts']==s['accepted_steps']+s['rejected_steps']
  cnt['attempts']+=s['attempts'];cnt['accepted_steps']+=s['accepted_steps'];cnt['rejected_steps']+=s['rejected_steps']
  for n,x in s['counters'].items():
   if n in ('accepted_steps','rejected_steps'):
    assert x==s[n],(n,x,s[n])
    continue
   if isinstance(x,int)and not isinstance(x,bool):cnt[n]+=x
 assert cnt['attempts']==cnt['accepted_steps']+cnt['rejected_steps']
 assert cnt['attempts']<=w['native_attempts'] and cnt['rhs_calls']<=w['driver_rhs']
 if unknown==0:
  assert cnt['attempts']==w['native_attempts']and cnt['rhs_calls']==w['driver_rhs']
 counts['known_native_runs']+=len(known);counts['unavailable_native_stats']+=unknown
 alloc=a['solve_alloc'];qa=a['centered']if atlas else a['point'];qalloc=qa['allocation']
 assert requests(alloc)>0 and size(alloc)>0
 if atlas:
  assert a['centered']['calls']==a['horner']['calls']==len(run['queries'])
  assert a['centered']['refused']==a['horner']['refused']==0
  assert a['centered']['cells']==sum(len(q['centered']['cells'])for q in run['queries'])
  assert a['horner']['cells']==sum(len(q['horner']['cells'])for q in run['queries'])
  pwork=Counter()
  for p in run['phases']:
   ww=p['work']
   pwork['shared_reconstruction_rhs']+=ww['shared_projection']['rhs_count']
   pwork['replay_aggregate_rhs']+=ww['replay_projection_rhs']
   pwork['guard_prepared_rhs']+=ww['guard_prepared_rhs']
   pwork['difference_prepared_rhs']+=ww['difference_prepared_rhs']
  assert sum(pwork.values())==w['projection_rhs']
  counts['horner_queries']+=a['horner']['calls'];counts['centered_queries']+=qa['calls']
  slabs=[s for p in run['phases']for s in p['slabs']]
 else:
  assert qa['calls']-qa['refused']==sum(len(e['queries'])for e in run['epochs'])
  pwork=None
  slabs=[s for e in run['epochs']for s in e['slabs']]
  counts['point_queries']+=qa['calls']
  counts['point_refused']+=qa['refused']
 cells=sum(len(s['certificate']['conservation']['cells'])for s in slabs if s['certificate']and s['certificate'].get('conservation'))
 combined=total_alloc(alloc,qalloc)
 v0={**v,'driver_rhs':w['driver_rhs'],'projection_rhs':w['projection_rhs'],'guard_evals':w['guard_evals'],'comparison_attempts':w['comparison_attempts'],'dictionary_uses':w['dictionary_uses'],'reported_scan_slabs':w['scan_slabs'],'returned_residual_cells':cells,'projection_breakdown':None if pwork is None else dict(pwork),'native_stats_known':len(known),'native_stats_missing':unknown,'known_native_counter_sum':dict(cnt),'solve_alloc':alloc,'solve_requests':requests(alloc),'solve_bytes':size(alloc),'query_calls':qa['calls'],'query_refused':qa['refused'],'query_alloc':qalloc,'query_requests':requests(qalloc),'query_bytes':size(qalloc),'total_requests':requests(combined),'total_bytes':size(combined),'horner':a.get('horner'),'centered':a.get('centered'),'reset_rhs':None if atlas else w['reset_rhs'],'gap_rhs':None if atlas else w['gap_rhs'],'reset_checks':w['reset_checks']if atlas else w['gap_checks'],'green_calls':None if atlas else w['green_calls']}
 out.append(v0)
comp=load(P/'components.log','COMPONENT')
ref=load(P/'refusals.log','REFUSAL')
assert len(ref)==11 and len({r['case']for r in ref})==11
for r in ref:
 assert requests(r['alloc'])>0
 if r['work']is not None:
  w=r['work']['phase'];assert w['committed']==sum(e['event']['committed']for e in r['events'])
# A directly observed hot callback allocation is decisive failure of noalloc.
for op in ['mass_rhs_callback','mass_jacobian_callback','project','residual','guard_build','guard_value','reconstruct','picard_tube','synchronize','joint_transport','gap_guard']:
 a=[x for x in comp if x['operation']==op];assert a and all(requests(x['allocation'])>0 for x in a),op
groups=[]
for k,items in itertools.groupby(sorted(comp,key=lambda r:(r['operation'],r['solver'],r['cert'],str(r['detail'].get('sync','')))),key=lambda r:(r['operation'],r['solver'],r['cert'],str(r['detail'].get('sync','')))):
 a=list(items);q=[requests(v['allocation'])for v in a];b=[size(v['allocation'])for v in a]
 groups.append({'operation':k[0],'solver':k[1],'cert':k[2],'sync':k[3],'n':len(a),'requests_min':min(q),'requests_median':statistics.median(q),'requests_max':max(q),'bytes_min':min(b),'bytes_median':statistics.median(b),'bytes_max':max(b)})
summary={'status':'RESOURCE_BASELINE_MEASURED_NOALLOC_BLOCKED','physical_receipts_identical':len(new),'resource_receipt_matches':counts['resource_identical'],'resource_variations':vary,'counts':dict(counts),'component_replays':len(comp),'component_groups':groups,'refusals':ref,'rows':out,'input_legacy_sha256':digest(O/'training.log.gz'),'raw_sha256':digest(P/'training.log'),'replay_sha256':digest(P/'replay.log'),'scope':'thread-local requested allocation events during API call; result destruction/setup/serialization excluded; independent component replays overlap; no peak/RSS/timing/FLOP or complete failure debit claim'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k not in ['rows','component_groups','refusals','resource_variations']})
for r in out:
 if r['case']=='alternating'and r['cert']=='Integral4'and r['order']=='Correlated'and r['method']in ['AtlasFresh','AtlasReuse']:
  print('representative',{k:r[k]for k in ['solver','method','attempts','driver_rhs','projection_rhs','solve_requests','solve_bytes','query_requests','query_bytes','last_width']})
for r in groups:
 if r['cert']=='Integral4'and r['operation']in ['mass_rhs_callback','mass_jacobian_callback','project','residual','reconstruct','guard_build','guard_value','picard_tube','synchronize']:print('component',r)
for r in ref:
 w=r.get('work');print('refusal',r['case'],r['stop'],requests(r['alloc']),size(r['alloc']),None if w is None else w['phase'])
