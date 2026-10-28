#!/usr/bin/env python3
"""Analyze held-out actual dispatch measurements; no accuracy admission from timing."""
import json, statistics, sys
from pathlib import Path
source = Path(sys.argv[1])
rows = [json.loads(line) for line in source.read_text().splitlines() if line.startswith('{')]
report = []
for row in rows:
    groups = {arm: [r['seconds'] for r in row['rows'] if r['arm'] == arm]
              for arm in ('serial','parallel','adaptive')}
    med = {arm: statistics.median(values) for arm,values in groups.items()}
    faster = min(('serial','parallel'), key=med.__getitem__)
    adaptive_rows = [r for r in row['rows'] if r['arm']=='adaptive']
    selected = [r['decision']['mode'].lower() for r in adaptive_rows]
    reasons = {}
    for r in adaptive_rows:
        reason = r['decision']['reason']; reasons[reason] = reasons.get(reason,0)+1
    all_checksums = {r['output_checksum'] for r in row['rows']}
    assert len(all_checksums)==1, 'selected/counterfactual output mismatch'
    report.append({'batch_len':row['workload']['batch_len'], 'workers':row['workers'],
       'heldout_median_seconds':med, 'heldout_faster_arm':faster,
       'adaptive_time_over_best_median':med['adaptive']/med[faster],
       'selected_parallel_fraction':selected.count('parallel')/len(selected),
       'choice_disagreement_fraction':sum(s!=faster for s in selected)/len(selected),
       'reasons':reasons,'calibration_seconds':row['calibration_seconds'],
       'startup_seconds':row['startup_seconds'],
       'warmup_seconds':row.get('warmup_seconds'),
       'calibration_samples':row.get('calibration_samples'),
       'first_selection':adaptive_rows[0]['decision'],
       'adaptive_total_with_calibration_and_startup':sum(groups['adaptive'])+row['calibration_seconds']+row['startup_seconds'],
       'serial_total':sum(groups['serial']),
       'parallel_total_excluding_pool_startup':sum(groups['parallel']),
       'checksum_equal':True})
print(json.dumps({'schema':'vigilode.execution-selection-pilot.v1','source':str(source),
 'timing_status':'EXPLORATORY_SHARED_HOST','results':report,
 'interpretation':'Single-process paired calibration and heldout cyclic timing; choice disagreement is a diagnostic, not statistical proof of misclassification. Empirical timing envelopes are not certified future costs.'}, indent=2))
