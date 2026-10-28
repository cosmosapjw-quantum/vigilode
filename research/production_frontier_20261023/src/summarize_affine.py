#!/usr/bin/env python3
"""Deterministic analysis only: never executes native code or benchmarks."""
import collections, hashlib, json, statistics
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'results/AFFINE_CAMPAIGN.json'
RECEIPT=ROOT/'evidence/affine_campaign_receipt.json'
CAP=1e-5
raw=json.loads(INPUT.read_text()); receipt=json.loads(RECEIPT.read_text()); rows=raw['rows']
assert hashlib.sha256(INPUT.read_bytes()).hexdigest()==receipt['output_sha256']
assert receipt['exit_code']==0 and receipt['threads']==1 and receipt['cpu_affinity']==[0]
assert len(rows)==192 and raw['observation_count']==17
keys=[(r['case'],r['method'],r['tol'],r['repeat']) for r in rows]
assert len(set(keys))==len(rows)
assert all(r['warmup']==(r['repeat']==0) for r in rows)
methods=['full-rhs-dopri5','original-lawson','affine-lawson','exact-declared-forcing']
cases=list(dict.fromkeys(r['case'] for r in rows))
assert len(cases)==6 and set(r['method'] for r in rows)==set(methods)
assert all(r['seconds']>=0 and r['maximum_sampled_absolute_error']>=0 for r in rows)
timed=[r for r in rows if not r['warmup']]
assert len(timed)==144

def metrics(group):
 times=[r['seconds'] for r in group]; errors=[r['maximum_sampled_absolute_error'] for r in group]
 return {'timing_samples':len(group),'seconds_median':statistics.median(times),
   'seconds_min':min(times),'seconds_max':max(times),'maximum_sampled_absolute_error':max(errors),
   'all_success':all(r['success'] for r in group),'eligible_at_error_cap':all(r['success'] and r['maximum_sampled_absolute_error']<=CAP for r in group),
   'rhs_calls_values':sorted(set(r['rhs_calls'] for r in group)),
   'accepted_steps_values':sorted(set(r['accepted'] for r in group),key=lambda x:-1 if x is None else x),
   'rejected_steps_values':sorted(set(r['rejected'] for r in group))}

results=[]
for case in cases:
 selected={}; groups={}
 for method in methods:
  source=[r for r in timed if r['case']==case and r['method']==method]
  assert len(source)==6
  if method=='exact-declared-forcing':
   assert all(r['tolerance_used']==False and r['direct_output_actions']==17 and r['rhs_calls']==0 for r in source)
   assert len(set(r['maximum_sampled_absolute_error'] for r in source))==1
   stat=metrics(source);stat.update({'selected_tolerance':None,'selection':'pooled_all_six_timing_repeats_of_same_workload; tolerance_labels_are_duplicates','direct_output_actions':17,'adaptive_trajectory_solver':False})
   groups[method]=[stat];selected[method]=stat
  else:
   candidates=[]
   for tol in sorted(set(r['tol'] for r in source)):
    group=[r for r in source if r['tol']==tol];assert len(group)==3
    stat=metrics(group);stat['tolerance']=tol;candidates.append(stat)
   groups[method]=candidates
   eligible=[r for r in candidates if r['eligible_at_error_cap']]
   best=min(eligible,key=lambda r:r['seconds_median']) if eligible else None
   selected[method]=None if best is None else dict(best,selected_tolerance=best['tolerance'],selection='lowest_median_of_two_eligible_tolerance_groups')
 def ratio(numerator,denominator):
  a,b=selected[numerator],selected[denominator]
  return None if a is None or b is None else a['seconds_median']/b['seconds_median']
 first=next(r for r in rows if r['case']==case)
 results.append({'case':case,'dimension':first['dimension'],'damping':first['damping'],'frequency':first['frequency'],'constant_forcing':first['constant_forcing'],
   'all_tolerance_groups':groups,'selected':selected,
   'median_speed_ratios':{'original_over_affine':ratio('original-lawson','affine-lawson'),'full_rhs_over_affine':ratio('full-rhs-dopri5','affine-lawson'),'original_over_exact':ratio('original-lawson','exact-declared-forcing'),'full_rhs_over_exact':ratio('full-rhs-dopri5','exact-declared-forcing')},
   'affine_rhs_reduction_from_original':selected['original-lawson']['rhs_calls_values'][0]/selected['affine-lawson']['rhs_calls_values'][0]})
summary={'schema':'vigilode.frontier-affine-summary.v1','source_commit':receipt['source_commit'],
 'campaign_sha256':receipt['output_sha256'],'receipt_sha256':hashlib.sha256(RECEIPT.read_bytes()).hexdigest(),
 'host':{'cpu_model':receipt['cpu_model'],'cpu_affinity':receipt['cpu_affinity'],'threads':1,'compiler':receipt['compiler'],'profile':receipt['build_profile']},
 'common_error_cap':{'metric':'maximum absolute component error across17requested observation times','value':CAP,'continuous_global_bound':False},
 'counts':{'rows':len(rows),'success':sum(bool(r['success']) for r in rows),'failure':sum(not r['success'] for r in rows),'warmup_excluded':48,'timed_rows':144,'cases':6,'methods':4,'nominal_tolerances':2,'labeled_configurations':48,'distinct_algorithm_configurations_excluding_direct_tolerance_duplicates':42,'adaptive_timing_samples_per_tolerance':3,'direct_timing_samples_pooled':6},
 'selection_policy':{'adaptive':'select lowest median among the two tolerance groups with all timed runs successful and sampled error<=cap','exact':'pool all6timed observations; tol labels do not define independent configurations','uncertainty':'exploratory microsecond timings, no confidence interval or statistical significance claim; min/max retained; min-of-two selection can be optimistic'},
 'all_rows_pass_error_cap':all(r['success'] and r['maximum_sampled_absolute_error']<=CAP for r in rows),
 'cases':results,
 'scope':{'direct_method':'exact declared constant or single harmonic forcing;17direct output evaluations from initial state, no adaptive trajectory integration','general_nonlinear_time_forcing_resolved':False,'old_case_equation_same':True,'old_case_parameters_same':{'damping':500,'frequency':500,'dimension':2,'time_span':[0,1]},'old_and_new_protocol_identical':False,'protocol_differences':['old11observations,new17','old initialstep0.001,new0.0625','source commits differ; old timing receipt is not used for ratios'],'cross_campaign_speed_comparison':False},
 'production_decision':'specialized exact kernels and affine residual adapter only; broad production HOLD'}
assert summary['all_rows_pass_error_cap']
assert summary['counts']['success']==192 and summary['counts']['failure']==0
(ROOT/'results/AFFINE_SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
for r in results:
 print(r['case'],{k:round(v,4) for k,v in r['median_speed_ratios'].items()},'rhs reduction',r['affine_rhs_reduction_from_original'])
print('PASS:192unique rows,hash/receipt,144timed rows,all row success and common error cap:',summary['all_rows_pass_error_cap'])
