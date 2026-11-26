from pathlib import Path
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_projection_workspace_20261126'
def text(f):return gzip.decompress(f.read_bytes()).decode()if f.suffix=='.gz'else f.read_text()
def load(f,prefix):return [json.loads(l.split(prefix+' ',1)[1])for l in text(f).splitlines()if prefix+' {'in l]
def raw_cells(f):return [l.split('CELL ',1)[1]for l in text(f).splitlines()if 'CELL {'in l]
def req(a):return a['alloc']+a['zeroed']+a['realloc']
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
def indexed(rows):
 d={key(r):r for r in rows};assert len(d)==len(rows);return d
def raw_index(rows):
 d={key(json.loads(r)):r for r in rows};assert len(d)==len(rows);return d
old=raw_index(raw_cells(O/'training.log.gz')+raw_cells(O/'affine_training.log.gz'));new=raw_index(raw_cells(P/'training.log'))
assert len(old)==len(new)==192 and old==new
cells={k:json.loads(r)for k,r in new.items()}
audited=raw_index(raw_cells(P/'audited_training.log'));assert len(audited)==96 and all(old[k]==r for k,r in audited.items())
ao=indexed(load(O/'training.log.gz','RESOURCE')+load(O/'affine_training.log.gz','RESOURCE'));bo=indexed(load(P/'training.log','RESOURCE'));co=indexed(load(P/'audited_training.log','RESOURCE'))
assert set(ao)==set(bo)==set(cells) and set(co)==set(audited)
prev=indexed(json.load(open(O/'AUDIT.json'))['rows']);rows=[]
def without_alloc(r):
 r=dict(r);r.pop('solve_alloc')
 for n in ['horner','centered','point']:
  if n in r:r[n]={k:v for k,v in r[n].items()if k!='allocation'}
 return r
for k,o in ao.items():
 n=bo[k];v=prev[k];assert without_alloc(o)==without_alloc(n),k
 av=n['solve_alloc'];qa=n['centered']if 'centered'in n else n['point'];qalloc=qa['allocation']
 r={**v,'solve_alloc':av,'solve_requests':req(av),'solve_bytes':av['requested_bytes'],'query_alloc':qalloc,'query_requests':req(qalloc),'query_bytes':qalloc['requested_bytes'],'total_requests':req(av)+req(qalloc),'total_bytes':av['requested_bytes']+qalloc['requested_bytes'],'horner':n.get('horner'),'centered':n.get('centered'),'before_solve_requests':req(o['solve_alloc']),'before_solve_bytes':o['solve_alloc']['requested_bytes'],'before_query_requests':v['query_requests'],'before_query_bytes':v['query_bytes']}
 r['requests_delta']=r['solve_requests']-r['before_solve_requests'];r['bytes_delta']=r['solve_bytes']-r['before_solve_bytes'];rows.append(r)
a=load(O/'components.log.gz','COMPONENT');b=load(P/'components.log','COMPONENT')
assert len(a)==len(b)==360
comp=[]
for x,y in zip(a,b):
 assert {k:v for k,v in x.items()if k!='allocation'}=={k:v for k,v in y.items()if k!='allocation'}
 if y['operation'] in ['mass_rhs_callback','mass_jacobian_callback','guard_value']:assert req(y['allocation'])==0
 comp.append({'operation':y['operation'],'solver':y['solver'],'cert':y['cert'],'before':x['allocation'],'after':y['allocation']})
a=load(O/'refusals.log.gz','REFUSAL');b=load(P/'refusals.log','REFUSAL')
assert len(a)==len(b)==11
for x,y in zip(a,b):assert {k:v for k,v in x.items()if k!='alloc'}=={k:v for k,v in y.items()if k!='alloc'}
ref={r['case']:r for r in b};ra={r['case']:r['audit']for r in load(P/'refusals.log','REFUSAL_AUDIT')}
assert set(ref)==set(ra)
stats=collections.Counter();local=[]
def count(p):assert 0<=p['returned']<=p['started'];return p['started']-p['returned']
def projection(p):
 for k in ['calls','evaluations','frozen','roots','linear']:count(p[k])
 assert p['admitted']<=p['calls']['started'] and p['rhs_returned']<=p['rhs_requested']
 assert p['rhs_requested']==p['successful_rhs_requested']+p['refused_rhs_requested']
 return p
def native(n):
 for k in ['initialization','point_projection','scan_nodes','replay_nodes']:projection(n[k])
 for k in ['scan_reconstruction','replay_reconstruction','scan_certificate','replay_certificate','guard_build','warp_setup']:count(n[k])
 assert n['scan_global_certificates']<=n['scan_certificate']['returned'] and n['replay_global_certificates']<=n['replay_certificate']['returned']
 s=n['stats']
 if s is not None:
  assert n['driver_invoked'] and s['attempts']==s['accepted_steps']+s['rejected_steps']
  assert s['accepted_steps']==s['counters']['accepted_steps'] and s['rejected_steps']==s['counters']['rejected_steps']
  return s['attempts']
 return None if n['driver_invoked'] else 0
def atlas(a,r):
 w=r['work']['phase'];assert a['admitted']
 assert sum(x['charged_native_attempts']for x in a['native'])==w['charged_native_attempts']
 assert sum(x['charged_scan_slabs']for x in a['native'])==w['charged_scan_slabs']
 assert a['committed']==w['committed'] and a['cache_hits']==w['cache_hits'] and a['reset_guards']['started']==w['reset_checks']
 assert a['negative_reset_guards']<=a['reset_guards']['returned']<=a['reset_guards']['started']
 for x in a['native']:
  assert x['measured_attempts']==native(x['native'])
  if x['returned']:assert x['native']['stats'] is not None and x['charged_native_attempts']==x['measured_attempts'] and x['charged_scan_slabs']==x['native']['stats']['accepted_steps']
  else:assert x['charged_native_attempts']==x['native_quota'] and x['charged_scan_slabs']==x['slab_quota']
 return w
ar=indexed(load(P/'audited_training.log','ATLAS_AUDIT'));assert set(ar)==set(audited)
for k,r in ar.items():
 a=r['audit'];v=cells[k]['atlas'];w=atlas(a,v)
 assert without_alloc(bo[k])==without_alloc(co[k]),k
 assert all(x['returned']for x in a['native'])
 assert len(a['native'])==len(v['phases'])==w['native_runs']
 assert sum(x['measured_attempts']for x in a['native'])==w['native_attempts']
 assert sum(x['native']['stats']['counters']['rhs_calls']for x in a['native'])==w['driver_rhs']
 assert sum(x['native']['scan_certificate']['started']for x in a['native'])>=w['scan_slabs']
 new_stats=[x['native']['stats']for x in a['native']];old_stats=bo[k]['driver_stats']
 assert len(new_stats)==len(old_stats)
 recovered=0
 for x,y,p in zip(new_stats,old_stats,v['phases']):
  if y is None:
   assert p['winner'] is None;recovered+=1
  else:assert x==y
 stats['native_runs']+=len(new_stats);stats['previously_hidden_stats']+=recovered
 stats['native_attempts']+=w['native_attempts'];stats['accepted']+=sum(x['accepted_steps']for x in new_stats);stats['rejected']+=sum(x['rejected_steps']for x in new_stats)
 stats['cache_hits']+=w['cache_hits'];stats['committed']+=w['committed']
 stats['guard_build_requests']+=sum(x['native']['guard_build']['started']for x in a['native'])
 for scope in ['initialization','point_projection','scan_nodes','replay_nodes']:
  for fld in ['rhs_requested','rhs_returned','refused_rhs_requested']:
   stats[scope+'_'+fld]+=sum(x['native'][scope][fld]for x in a['native'])
  for fld in ['calls','evaluations','linear','roots','frozen']:
   for count_kind in ['started','returned']:
    stats[scope+'_'+fld+'_'+count_kind]+=sum(x['native'][scope][fld][count_kind]for x in a['native'])
 base_alloc=bo[k]['solve_alloc'];audit_alloc=co[k]['solve_alloc']
 local.append({**{f:k[i]for i,f in enumerate(fields)},'qualified':v['stop']=='Horizon','attempts':w['native_attempts'],'driver_rhs':w['driver_rhs'],'projection_rhs':w['projection_rhs'],'native_runs':len(new_stats),'hidden_stats_recovered':recovered,'old_stats_coverage':len(new_stats)-recovered,'new_stats_coverage':len(new_stats),'before_requests':req(base_alloc),'audit_requests':req(audit_alloc),'before_bytes':base_alloc['requested_bytes'],'audit_bytes':audit_alloc['requested_bytes'],'audit':a})
for k,r in ref.items():
 a=ra[k]
 if r['work']is None:assert not a['admitted'] and not a['native']
 else:atlas(a,r)
warp=ra['warp']['native'][0]
assert warp['measured_attempts']==0 and warp['charged_native_attempts']==4096 and warp['native']['initialization']['calls']['returned']==1
assert not warp['native']['driver_invoked'] and warp['native']['warp_setup']=={'started':1,'returned':0}
f=load(P/'focused2.log','FAILED_INIT');e=load(P/'focused2.log','EPOCH_REFUSAL');rec=load(P/'focused2.log','RECON_REFUSAL');nw=load(P/'focused2.log','NO_WINNER')
assert len(f)==len(rec)==len(nw)==1 and len(e)==5
for v in e:assert v['measured']==native(v['work'])
projection(rec[0]['work']);assert f[0]['work']['evaluations']['returned']==7 and f[0]['work']['rhs_returned']==2
assert nw[0]['native']['stats'] is not None and nw[0]['native']['stats']['attempts']==nw[0]['work']['attempts']
assert stats['native_attempts']==stats['accepted']+stats['rejected']
summary={'physical_receipts_identical':len(new),'audited_physical_receipts_identical':len(audited),'component_receipts_identical':len(comp),'refusal_receipts_identical':len(ref),'counts':dict(stats),'rows':rows,'local_rows':local,'components':comp,'refusals':[{**r,'audit':ra[k]}for k,r in ref.items()],'failure_probes':{'initialization':f,'reconstruction':rec,'epoch':e,'no_winner':nw},'scope':'measured native Stats or explicit unknown; boundary requests/returned counts not failed inner arithmetic. Quota debit unchanged; no physical event authority from audit. Allocations per thread only, no whole-hotloop noalloc, timing or peak claim.'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print('counts',dict(stats))
print('legacy rows',len(rows),'Horizon',sum(r['qualified']for r in rows),'audit rows',len(local),'Horizon',sum(r['qualified']for r in local))
for solver in ['Mass','Prepared5','Scalar5','Affine5']:
 r=[v for v in rows if v['solver']==solver];a=[v for v in local if v['solver']==solver]
 print(solver,'legacy allocations',sum(v['before_solve_requests']for v in r),sum(v['solve_requests']for v in r),'audit requests overhead',sum(v['audit_requests']-v['before_requests']for v in a),'bytes',sum(v['audit_bytes']-v['before_bytes']for v in a),'hidden stats',sum(v['hidden_stats_recovered']for v in a))
print('failure reconstruction',rec[0]['work'])
for v in e:print('epoch',v['case'],v['measured'],v['work']['point_projection']['calls'],v['work']['point_projection']['refused_rhs_requested'])
