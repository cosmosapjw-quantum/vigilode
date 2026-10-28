#!/usr/bin/env python3
"""Frozen scalar readback with actual derivative-dispatch accounting."""
import csv,gzip,hashlib,json,math,statistics,struct,sys
from collections import Counter
from pathlib import Path
out=Path(sys.argv[1]);load=lambda p:json.loads((out/p).read_text());refs=load('REFERENCES.json');manifest=load('MANIFEST.json')
cache={};cal={};tim={};checks=Counter()
def state(h):
 if h not in cache:
  with gzip.open(out/'states'/(h+'.json.gz'),'rb')as f:b=f.read()
  assert hashlib.sha256(b).hexdigest()==h;cache[h]=json.loads(b)
 return cache[h]
def bits(v):return b''.join(struct.pack('>d',x)for x in v)
def same(a,b):return a['y_new']is not None and b['y_new']is not None and bits(a['y_new'])==bits(b['y_new'])and bits(a['error_vector'])==bits(b['error_vector'])
def rms(v,s):return math.sqrt(math.fsum((a/b)**2 for a,b in zip(v,s))/len(v))
def close(a,b):return math.isclose(a,b,rel_tol=4e-12,abs_tol=1e-22)
for file,phase in [('accuracy.jsonl.gz','accuracy'),('timing.jsonl.gz','timing')]:
 with gzip.open(out/file,'rt')as f:
  for line in f:
   record=json.loads(line);r=record['row'];m=record['metrics'];obj=state(r['state_object_sha256']);ref=refs[r['cell_id']];checks[phase]+=1
   if r.get('root_completed'):
    y=obj['y_new'];e=obj['error_vector'];v=ref['python']['y_new'];e0=ref['python']['error_vector'];y0=ref['python']['y0'];rho=ref['radius'];n=r['n']
    assert len(y)==len(e)==len(v)==len(e0)==len(y0)==len(rho)==n and all(math.isfinite(x)for x in y+e)
    scale=[1e-10+1e-6*max(abs(a),abs(b))for a,b in zip(y0,v)]
    error=rms([a-b for a,b in zip(y,v)],scale)+rms(rho,scale);embedded=rms([a-b for a,b in zip(e,e0)],scale)
    assert close(error,m['output_wrms_up'])and close(embedded,m['embedded_difference_wrms'])
    assert m['valid']==(error<=.1 and embedded<=.1 and r['deterministic']);checks['metrics']+=1
   else:assert not m['valid']and m['output_wrms_up']is None and m['embedded_difference_wrms']is None;checks['failures_preserved']+=1
   result=obj.get('result')or{};tx=result.get('result')or{};attempt=result.get('attempt')or tx.get('candidate_attempt')
   if attempt:
    w=attempt['work'];assert w['jvp_attempted']==w['basis_jvp']+w['stage_jvp']+w['validation_jvp']
    assert w['jvp_completed']<=w['jvp_attempted']and w['rhs_completed']<=w['rhs_attempted']
    assert w['stage_tasks_scheduled']==8*(w['stage_batches']+w['derivative_batches']+w['validation_batches'])
    assert w['stage_tasks_completed']==w['stage_tasks_scheduled']
    if r['arm'].startswith('eager'):assert w['derivative_batches']==0 and w['unused_stage_jvp_avoided']==0
    cap=r.get('candidate_jvp_cap')
    if cap is not None:assert w['jvp_attempted']<=min(cap,r['config']['max_jvp']);checks['candidate_cap']+=1
    candidate=attempt.get('candidate')
    if candidate:
     assert candidate['full_residual_wrms']<=r['config']['full_residual_wrms_limit']
     assert candidate['target_id']=='rodas5p-raw-u-coupled-operational-v1'
     assert bits(candidate['error_vector'])==bits(candidate['stages_u'][-1])
     assert bits(candidate['y_new'])==bits(obj['y_new'])
    else:assert attempt['failure']is not None
    checks['work_ledgers']+=1
   if phase=='accuracy':
    key=(r['cell_id'],r['arm']);assert key not in cal;cal[key]=(r,obj,m)
   else:
    u=record['unit'];key=(u['cell'],u['arm'],u['block']);assert key not in tim;tim[key]=(r,obj,m,u)
    if u['time_valid']:assert len(r['elapsed_ns'])==3 and u['median_ns']==statistics.median(r['elapsed_ns'])
    if u['admitted']:
     assert m['valid']and r['warmups']==1 and same(cal[(u['cell'],u['arm'])][1],obj);checks['timing_identity']+=1
for p in load('PARITY.json'):
 x=cal[(p['cell'],p['left'])][1];y=cal[(p['cell'],p['right'])][1]
 if p['completed_both']:
  assert p['output_and_embedded_bits_equal']and same(x,y)
  ax=x['result']['attempt'];ay=y['result']['attempt']
  assert bits([v for row in ax['candidate']['stages_u']for v in row])==bits([v for row in ay['candidate']['stages_u']for v in row])
  assert ax['candidate']['full_residual_wrms']==ay['candidate']['full_residual_wrms']
  if p['left'].startswith('eager'):
   wx=ax['work'];wy=ay['work'];assert wx['newton_updates']==wy['newton_updates'];assert wx['rhs_attempted']==wy['rhs_attempted']
   assert wx['jvp_attempted']-wy['jvp_attempted']==wy['unused_stage_jvp_avoided'];checks['exact_saved_work']+=1
  checks['full_stage_bit_parity']+=1
for u in load('TIMING_UNITS.json'):assert u==tim[(u['cell'],u['arm'],u['block'])][3]
for p in load('PAIRED.json'):
 ratios=[];failed=p['relation']=='failed_attempt_cost_not_solver_speed'
 for b in range(8):
  x=tim[(p['cell'],p['baseline'],b)][3];y=tim[(p['cell'],p['candidate'],b)][3]
  if (x['admitted']and y['admitted'])or(failed and x['time_valid']and y['time_valid']and not x['root_completed']and not y['root_completed']):ratios.append(x['median_ns']/y['median_ns'])
 assert ratios==p['ratios']and len(ratios)==p['blocks']
 if p['status']=='COMPLETE_DIAGNOSTIC':assert statistics.median(ratios)==p['ratio_median'];checks['paired_medians']+=1
assert hashlib.sha256((out/'FROZEN_TIMING.json').read_bytes()).hexdigest()==manifest['frozen_sha256']
summary=load('SUMMARY.json');assert checks['accuracy']==summary['accuracy_rows']and checks['timing']==summary['timing_processes']
audit={'status':'PASS_SCOPED_READBACK','checks':dict(checks),'state_objects_verified':len(cache),'author_independence':'different metric path, same author; no independent-personnel claim',
 'unclaimed':['continuous/global error certificate','whole-workspace tests','optimal budget or trajectory routing','rigorous bootstrap coverage under shared-host dependence']}
(out/'AUDIT.json').write_text(json.dumps(audit,indent=2)+'\n');print(json.dumps(audit))
cost=list(csv.DictReader((out/'COST.csv').open()));pairs=load('PAIRED.json')
text=['# Residual-first coupled-step economy','',f"Measured source `{manifest['source_commit']}`.",'',
 'This is fixed-h complete-root attempt cost. Original raw-U/full-space and embedded tests are unchanged. Root success is not physical timestep acceptance. All failed work remains visible.','',
 '```json',json.dumps(summary,indent=2),'```','',
 '|Case|Arm|Reference gate|Embedded test|ms|JVP|Avoided JVP|Derivative batches|Fallback|','|---|---|---|---|---:|---:|---:|---:|---|']
for r in cost:text.append('|'+ '|'.join(r.get(k,'')for k in ['cell','arm','reference_admitted','embedded_test_passes','median_ms','jvp','avoided_jvp','derivative_batches','fallback_used'])+'|')
text+=['','## Paired ratios: baseline/candidate','',
 'Intervals allocate Bonferroni budget over50comparisons;8shared-host blocks do not establish universal performance or certified coverage. Failed-attempt cost is explicitly not solver speed. Different-target threshold comparisons are not exact matched-accuracy claims.','',
 '|Case|Baseline|Candidate|Ratio|Family interval|Relation|','|---|---|---|---:|---|---|']
for p in pairs:
 if 'ratio_median'in p:text.append(f"|{p['cell']}|{p['baseline']}|{p['candidate']}|{p['ratio_median']:.4f}|{p['ci95_family'][0]:.4f}, {p['ci95_family'][1]:.4f}|{p['relation']}|")
text+=['','## Readback','', '```json',json.dumps(audit,indent=2),'```','',
 'An explicit JVP cap limits only attempted candidate JVP work, not projected-LU time or total fallback time. It can refuse otherwise successful coupled candidates. The64/256values are predeclared sensitivity examples, not optimal thresholds.','',
 'Default eager API remains unchanged. Residual-first policy adds parallel dispatches, saves only unused derivatives and changes callback ordering. Callbacks must be pure at a frozen epoch. The existing prior19tests are rerun separately alongside new tests; no CodeRabbit or whole-repository PASS is inferred.']
(out/'REPORT.md').write_text('\n'.join(text)+'\n')
