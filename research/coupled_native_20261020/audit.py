#!/usr/bin/env python3
"""Read retained native-step data without rerunning or replacing a solver.

The scalar fsum metric path differs from the campaign's NumPy norm. This is
computational cross-checking by the same author, not a CodeRabbit review.
"""
import csv,gzip,hashlib,json,math,statistics,struct,sys
from collections import Counter
from pathlib import Path
out=Path(sys.argv[1]);load=lambda p:json.loads((out/p).read_text());refs=load('REFERENCES.json');manifest=load('MANIFEST.json');summary=load('SUMMARY.json')
cache={};cal={};rawtim={};checks=Counter();failures=Counter()
def state(h):
 if h not in cache:
  with gzip.open(out/'states'/(h+'.json.gz'),'rb')as f:b=f.read()
  assert hashlib.sha256(b).hexdigest()==h
  cache[h]=json.loads(b)
 return cache[h]
def bits(v):return b''.join(struct.pack('>d',x)for x in v)
def wrms(v,s):return math.sqrt(math.fsum((a/b)**2 for a,b in zip(v,s))/len(v))
def compatible(a,b):return math.isclose(a,b,rel_tol=3e-12,abs_tol=1e-22)
for filename,phase in [('accuracy.jsonl.gz','accuracy'),('timing.jsonl.gz','timing')]:
 with gzip.open(out/filename,'rt')as f:
  for line in f:
   r=json.loads(line);row=r['row'];obj=state(row['state_object_sha256']);ref=refs[row['cell_id']];m=r['metrics'];checks[phase]+=1
   if row.get('root_completed'):
    y=obj['y_new'];e=obj['error_vector'];v=ref['python']['y_new'];e0=ref['python']['error_vector'];y0=ref['python']['y0'];rho=ref['radius'];n=row['n']
    assert len(y)==len(e)==len(v)==len(e0)==len(y0)==len(rho)==n
    assert all(math.isfinite(x)for x in y+e)
    scale=[1e-10+1e-6*max(abs(a),abs(b))for a,b in zip(y0,v)]
    error=wrms([a-b for a,b in zip(y,v)],scale)+wrms(rho,scale)
    embedded=wrms([a-b for a,b in zip(e,e0)],scale)
    assert compatible(error,m['output_wrms_up']) and compatible(embedded,m['embedded_difference_wrms'])
    assert m['valid']==(error<=.1 and embedded<=.1 and row['deterministic'])
    checks['recomputed_metrics']+=1
   else:
    assert not m['valid'] and m['output_wrms_up'] is None and m['embedded_difference_wrms'] is None
    checks['declined_preserved']+=1
   result=obj.get('result')or{}
   if result.get('kind')=='coupled':
    attempt=result['attempt'];w=attempt['work'];failure=attempt.get('failure');candidate=attempt.get('candidate')
    assert w['rhs_completed']<=w['rhs_attempted'] and w['jvp_completed']<=w['jvp_attempted']
    assert w['jvp_attempted']==w['basis_jvp']+w['stage_jvp']+w['validation_jvp']
    assert w['stage_tasks_scheduled']==8*(w['stage_batches']+w['validation_batches'])
    assert w['stage_tasks_scheduled']==w['stage_tasks_completed']
    if candidate:
     assert failure is None and candidate['target_id']=='rodas5p-raw-u-coupled-operational-v1'
     assert candidate['full_residual_wrms']<=row['config']['full_residual_wrms_limit']
     assert len(candidate['stages_u'])==8 and all(len(x)==row['n']for x in candidate['stages_u'])
     assert bits(candidate['y_new'])==bits(obj['y_new'])
     assert bits(candidate['error_vector'])==bits(candidate['stages_u'][-1])
     assert candidate['accuracy_authority'].startswith('operational')
    else:
     assert failure is not None
     if phase=='accuracy':failures[failure['kind']]+=1
    checks['work_accounting']+=1
   if phase=='accuracy':
    key=(row['cell_id'],row.get('arm')or row['command'][6]);assert key not in cal;cal[key]=(row,obj,m)
   else:
    u=r['unit'];key=(u['cell'],u['arm'],u['block']);assert key not in rawtim;rawtim[key]=(row,obj,m,u)
    if u['admitted']:
     assert m['valid'] and row['warmups']==1 and row['deterministic']
     assert len(row['elapsed_ns'])==row['repetitions'] and all(x>0 for x in row['elapsed_ns'])
     assert u['median_ns']==statistics.median(row['elapsed_ns'])
     old=cal[(u['cell'],u['arm'])][1]
     assert bits(old['y_new'])==bits(obj['y_new']) and bits(old['error_vector'])==bits(obj['error_vector'])
     checks['timing_calibration_identity']+=1
# Only claim worker-count bit identity after checking all corresponding outputs.
for cell in {k[0]for k in cal}:
 base=cal[(cell,'coupled-1')]
 for arm in ['coupled-2','coupled-4']:
  other=cal[(cell,arm)]
  if base[0].get('root_completed')and other[0].get('root_completed'):
   assert bits(base[1]['y_new'])==bits(other[1]['y_new'])
   assert bits(base[1]['error_vector'])==bits(other[1]['error_vector'])
   checks['worker_count_bit_parity']+=1
for u in load('TIMING_UNITS.json'):
 assert u==rawtim[(u['cell'],u['arm'],u['block'])][3]
for p in load('PAIRED.json'):
 ratios=[]
 for block in range(10):
  a=rawtim[(p['cell'],p['baseline'],block)][3];b=rawtim[(p['cell'],p['candidate'],block)][3]
  if a['admitted']and b['admitted']:ratios.append(a['median_ns']/b['median_ns'])
 assert ratios==p['ratios'] and len(ratios)==p['blocks']
 if p['status']=='DIAGNOSTIC_COMPLETE':
  assert len(ratios)==10 and statistics.median(ratios)==p['ratio_median'];checks['paired_medians']+=1
assert hashlib.sha256((out/'FROZEN_COMPARISONS.json').read_bytes()).hexdigest()==manifest['frozen_sha256']
assert checks['accuracy']==summary['accuracy_rows'] and checks['timing']==summary['timing_processes']
audit={'status':'PASS_SCOPED_RAW_READBACK','checks':dict(checks),'verified_state_objects':len(cache),'accuracy_declines_by_kind':dict(failures),
 'independence':'different metric implementation; same author, NOT a CodeRabbit or independent personnel review',
 'not_verified':['global trajectory error','certified residual-to-output transport','adaptive switching','cross-platform speed','bootstrap coverage under correlated host noise']}
(out/'AUDIT.json').write_text(json.dumps(audit,indent=2)+'\n')
cost=list(csv.DictReader((out/'COST.csv').open()));pairs=load('PAIRED.json')
text=['# Native coupled-step benchmark','', '## Measured source','',f"`{manifest['source_commit']}`",'',
 'Fixed-h complete-step root latency. All candidate/basis/corrector/full-residual work is timed; pool setup, references, JSON and result destruction are outside the timer. This is not a full-trajectory benchmark.','',
 '```json',json.dumps(summary,indent=2),'```','',
 '|Case|Arm|Root complete|Reference gate|ms|Basis|Peak concurrent stage tasks|','|---|---|---|---|---:|---:|---:|']
for r in cost:text.append('|'+ '|'.join(r.get(k,'')for k in ['cell','arm','root_completed','valid','median_ms','basis','peak_parallel_tasks'])+'|')
text+=['','## Paired cost ratios','', 'Baseline/candidate; >1 means lower candidate time. Worker-count comparisons have checked identical output bits. Sequential comparisons pass the same raw-reference threshold but the native-K and raw-U targets are not declared identical. Neither guarantees matched continuous-time accuracy.','',
 '|Case|Baseline|Candidate|Ratio|Family-adjusted interval|','|---|---|---|---:|---|']
for p in pairs:
 if 'ratio_median'in p:text.append(f"|{p['cell']}|{p['baseline']}|{p['candidate']}|{p['ratio_median']:.4f}|{p['ci95_family'][0]:.4f}, {p['ci95_family'][1]:.4f}|")
text+=['','## Independent computational readback','', '```json',json.dumps(audit,indent=2),'```','',
 'Small residual is not a nonlinear output certificate. References here solve the frozen step target, not the full physical trajectory. Matrix-free state operators can still lead to expensive reduced dense systems and repeated stage JVP work. The negative broad/stencil cases and failed speculative work are retained rather than excluded from cost accounting.',
 'Tests and review availability are separate: a successful numerical readback is not a CodeRabbit review, and the standalone companion is not covered by a root-workspace test unless explicitly invoked.']
(out/'REPORT.md').write_text('\n'.join(text)+'\n');print(json.dumps(audit))
