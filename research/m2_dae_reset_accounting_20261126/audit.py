from pathlib import Path
import json,gzip,hashlib,collections,statistics,itertools
P=Path(__file__).parent
O=P.parent/'m2_dae_failure_accounting_20261126'
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
old=raw_index(raw_cells(O/'training.log.gz'));new=raw_index(raw_cells(P/'training.log'))
assert len(old)==len(new)==192 and old==new
cells={k:json.loads(r)for k,r in new.items()}
audited=raw_index(raw_cells(P/'audited_training.log'));assert len(audited)==192 and all(old[k]==r for k,r in audited.items())
ao=indexed(load(O/'training.log.gz','RESOURCE'));bo=indexed(load(P/'training.log','RESOURCE'));co=indexed(load(P/'audited_training.log','RESOURCE'))
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
stats=collections.Counter();local=[]
def count(p):assert 0<=p['returned']<=p['started'];return p['started']-p['returned']
def projection(p):
 for k in ['calls','evaluations','frozen','roots','linear']:count(p[k])
 assert p['admitted']<=p['calls']['started'] and p['rhs_returned']<=p['rhs_requested']
 assert p['rhs_requested']==p['successful_rhs_requested']+p['refused_rhs_requested']
 return p
def guard(g):
 for k in ['builds','evaluations','preconditioners','reduced_columns']:count(g[k])
 assert g['rhs_returned']<=g['rhs_requested']
def kernel(w):
 for k in ['components','branches','branch_evaluations','branch_preconditioners','branch_columns','tubes','picard_images','synchronizations','sync_components','green']:count(w[k])
 guard(w['component_guards']);projection(w['branch_initialization'])
 assert w['branch_rhs_returned']<=w['branch_rhs_requested'] and w['strict_images']<=w['picard_images']['returned']
def transaction(q):
 kernel(q['kernel'])
 for k in ['preparation','table_setup','cells','restart_branch','transport','transport_cells','gap_checks']:count(q[k])
 assert q['negative_gaps']<=q['gap_checks']['returned'] and q['transport_qualified']<=q['transport']['returned']
 if q['committed']:assert q['restart_branch']['returned']==1 and q['cells']['started']==q['cells']['returned'] and q['gap_checks']['returned']==q['negative_gaps']
def native(n):
 guard(n['guard_kernel']);kernel(n['difference_kernel']);count(n['difference_comparisons'])
 assert n['guard_kernel']['builds']==n['guard_build'] and n['positive_differences']<=n['difference_comparisons']['returned']
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
def chain(a,r):
 w=r['work']['common'];assert a['admitted']
 assert sum(x['charged_native_attempts']for x in a['native'])==w['charged_native_attempts']
 assert sum(x['charged_scan_slabs']for x in a['native'])==w['charged_scan_slabs']
 assert len(a['transitions'])==w['transitions_attempted']
 assert sum(t['charged_cells']for t in a['transitions'])==w['charged_reset_cells']
 assert sum(t['charged_picard_attempts']for t in a['transitions'])==r['work']['charged_picard_attempts']
 assert sum(t['work']['gap_checks']['started']for t in a['transitions'])==w['gap_checks']
 assert sum(t['work']['committed']for t in a['transitions'])==w['transitions_committed']
 for x in a['native']:
  assert x['measured_attempts']==native(x['native'])
  if x['returned']:assert x['measured_attempts']==x['charged_native_attempts'] and x['native']['stats']['accepted_steps']==x['charged_scan_slabs']
  else:assert x['charged_native_attempts']==x['native_quota'] and x['charged_scan_slabs']==x['slab_quota']
 for t,e in zip(a['transitions'],r.get('events',[])):
  q=t['work'];transaction(q);assert t['root']==e['event_time'] and t['guard']==e['guard'] and t['from']==e['from'] and t['target']==e['target']
  assert q['committed']==e['committed']
  k=q['kernel'];assert k['branches']['started']<=t['charged_picard_attempts']
  if e['reset'] is not None:
   p=e['reset'];assert q['cells']['started']==len(p['cells'])
   proofs=[b for c in p['cells'] if c['tube'] is not None for b in c['tube']['attempts']]
   if p['restart_branch']is not None:proofs.append(p['restart_branch'])
   assert k['branches']['returned']>=len(proofs)
   assert k['branch_initialization']['rhs_returned']>=sum(b['initial']['work']['rhs_count']for b in proofs)
   assert k['branch_rhs_returned']>=sum(b['work']['rhs_count']for b in proofs)
   assert k['green']['returned']>=sum(c['sync']['green_calls']for c in p['cells']if c['sync']is not None)
   assert k['component_guards']['rhs_returned']>=sum(c['prepared_work']['rhs_count']for r in p['cells']for c in r['reset'])
  assert q['gap_checks']['returned']==len(e['gap']) and q['negative_gaps']==sum(g['enclosure'][1]<0 for g in e['gap'])
  if e['transport'] is not None:assert q['transport_cells']['returned']==len(e['transport']['cells'])
 return w
ar=indexed(load(P/'audited_training.log','ATLAS_AUDIT'));cr=indexed(load(P/'audited_training.log','CHAIN_AUDIT'));assert len(ar)==len(cr)==96 and set(ar)|set(cr)==set(audited) and not set(ar)&set(cr)
# Separate ledgers: the same operation appears in a parent and child receipt,
# so report typed counters, never sum them as a single operation count.
stats=collections.Counter();reset=collections.Counter();local=[]
for k,r in (ar|cr).items():
 a=r['audit'];joint=k in cr;v=cells[k]['joint' if joint else 'atlas'];w=chain(a,v)if joint else atlas(a,v)
 assert without_alloc(bo[k])==without_alloc(co[k]),k
 assert len(a['native'])==w['native_runs'] and all(x['returned']for x in a['native'])
 assert sum(x['measured_attempts']for x in a['native'])==w['native_attempts']
 ns=[x['native']['stats']for x in a['native']]
 assert sum(x['counters']['rhs_calls']for x in ns)==w['driver_rhs']
 hidden=0
 for x,y in zip(ns,bo[k]['driver_stats']):
  if y is None:hidden+=1
  else:assert x==y
 label='joint_' if joint else 'atlas_'
 for n in a['native']:
  q=n['native'];g=q['guard_kernel'];d=q['difference_kernel']
  for tag,obj in [('guard_build',g['builds']),('guard_eval',g['evaluations']),('guard_columns',g['reduced_columns']),('guard_preconditioners',g['preconditioners']),('difference_comparison',q['difference_comparisons']),('difference_cell',d['components'])]:
   for x in ['started','returned']:stats[label+tag+'_'+x]+=obj[x]
  stats[label+'positive_differences']+=q['positive_differences']
  stats[label+'guard_prepared_rhs_returned']+=g['rhs_returned']
  stats[label+'difference_prepared_rhs_returned']+=d['component_guards']['rhs_returned']
 assert sum(n['native']['difference_comparisons']['started']for n in a['native'])==w['comparison_attempts']
 stats[label+'native_runs']+=len(ns);stats[label+'hidden_stats_recovered']+=hidden
 stats[label+'attempts']+=w['native_attempts'];stats[label+'accepted']+=sum(x['accepted_steps']for x in ns);stats[label+'rejected']+=sum(x['rejected_steps']for x in ns)
 if joint:
  for t in a['transitions']:
   q=t['work'];z=q['kernel']
   reset['transitions_attempted']+=1;reset['commits']+=int(q['committed'])
   reset['charged_cells']+=t['charged_cells'];reset['charged_picard']+=t['charged_picard_attempts']
   for tag,obj in [('preparation',q['preparation']),('table',q['table_setup']),('cell',q['cells']),('component',z['components']),('tube',z['tubes']),('branch',z['branches']),('picard_image',z['picard_images']),('restart',q['restart_branch']),('sync',z['synchronizations']),('sync_component',z['sync_components']),('green',z['green']),('transport',q['transport']),('transport_cell',q['transport_cells']),('gap',q['gap_checks'])]:
    for x in ['started','returned']:reset[tag+'_'+x]+=obj[x]
   reset['strict_images']+=z['strict_images'];reset['negative_gaps']+=q['negative_gaps']
   reset['init_rhs_requested']+=z['branch_initialization']['rhs_requested'];reset['init_rhs_returned']+=z['branch_initialization']['rhs_returned'];reset['branch_rhs_returned']+=z['branch_rhs_returned'];reset['component_rhs_returned']+=z['component_guards']['rhs_returned']
 else:stats['atlas_cache_hits']+=w['cache_hits'];stats['atlas_commits']+=w['committed']
 x,y=bo[k]['solve_alloc'],co[k]['solve_alloc'];local.append({**{f:k[i]for i,f in enumerate(fields)},'qualified':v['stop']=='Horizon','attempts':w['native_attempts'],'driver_rhs':w['driver_rhs'],'projection_rhs':w['projection_rhs'],'native_runs':len(ns),'hidden_stats_recovered':hidden,'before_requests':req(x),'audit_requests':req(y),'before_bytes':x['requested_bytes'],'audit_bytes':y['requested_bytes'],'audit':a})
for label in ['joint_','atlas_']:assert stats[label+'attempts']==stats[label+'accepted']+stats[label+'rejected']
probes=load(P/'focused.log','CHAIN_REFUSAL');assert len(probes)==12
for r in probes:chain(r['audit'],r)
p0=load(P/'focused.log','PICARD_RED_GREEN');p1=load(P/'focused.log','BRANCH_FAILURE');p2=load(P/'focused.log','GREEN_FAILURE');p3=load(P/'focused.log','GUARD_FAILURE')
assert all(len(v)==1 for v in (p0,p1,p2,p3))
for p in p0+p1+p2:kernel(p['work'])
guard(p3[0]['work'])
assert p0[0]['work']['branches']=={'started':2,'returned':1}
assert p2[0]['work']['green']=={'started':4,'returned':4} and p2[0]['control']['green']=={'started':8,'returned':8}
assert p3[0]['work']['rhs_returned']==1 and p3[0]['work']['builds']['returned']==0
# Direct component probes in actual production paths, not full-call heap
# counts misrepresented as exclusive computational scratch.
boundary=[]
for solver in ['Mass','Prepared5','Scalar5']:
 for op in ['mass_rhs_callback','mass_jacobian_callback','mass_time_values','project','reconstruct','residual','guard_build','guard_value','picard_tube','reset_component','synchronize','joint_transport','gap_guard']:
  x=[r for r in comp if r['solver']==solver and r['cert']=='Integral4' and r['operation']==op]
  if x:boundary.append({'solver':solver,'operation':op,'cases':len(x),'median_requests':statistics.median(req(r['after'])for r in x),'min_requests':min(req(r['after'])for r in x),'max_requests':max(req(r['after'])for r in x),'median_bytes':statistics.median(r['after']['requested_bytes']for r in x)})
assert any(r['operation']=='project' and r['solver']=='Prepared5' and r['min_requests']>0 for r in boundary)
assert any(r['operation']=='guard_build' and r['min_requests']>0 for r in boundary)
vg={'status':'BLOCKED_NONZERO_REDUCED_CALLBACK_AND_GUARD_BUILD','authority':'AGENTS.md VG1 prohibits heap allocation in actual callback/event loops; output-owned report allocations not exempt inside those boundaries','measurements':boundary,'remaining_unknown':['driver-only hotloop allocations versus owning output','exclusive inner failed kernel work','unreturned native Stats'],'noalloc_scope':'prior actual Mass RHS/Jacobian/time callback and guard value; latest replay confirms RHS/Jacobian/guard values, not entire solver'}
(P/'RESOURCE_BOUNDARY.json').write_text(json.dumps(vg,indent=2)+'\n')
summary={'physical_receipts_identical':192,'audited_physical_receipts_identical':192,'component_receipts_identical':360,'atlas_refusal_receipts_identical':11,'chain_refusal_pairs':len(probes),'counts':dict(stats),'reset_counts':dict(reset),'rows':rows,'local_rows':local,'components':comp,'failure_probes':{'picard':p0,'branch':p1,'green':p2,'guard':p3,'chain':probes},'resource_boundary':vg,'scope':'actual typed boundary calls, returned reports and partial successes; no invented inner failed work; physical authority and conservative debit unchanged; VG1 blocked'}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print('native',dict(stats));print('reset',dict(reset))
print('replay',len(rows),len(local),'horizon',sum(v['qualified']for v in rows))
for method in ['JointCentered','JointGreen','AtlasFresh','AtlasReuse']:
 x=[r for r in local if r['method']==method];print(method,len(x),sum(r['audit_requests']-r['before_requests']for r in x),sum(r['audit_bytes']-r['before_bytes']for r in x))
for r in boundary:print('resource',r)
for v in probes:print('refusal',v['case'],v['stop'],[(n['measured_attempts'],n['charged_native_attempts'])for n in v['audit']['native']],[(t['work']['kernel']['branches'],t['work']['committed'])for t in v['audit']['transitions']])
