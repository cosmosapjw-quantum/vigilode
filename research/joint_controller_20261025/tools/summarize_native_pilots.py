#!/usr/bin/env python3
"""Summarize this research unit's actual pilot outputs; never runs a solver.

Input binaries and source hashes belong to the parent run receipts. This reader
checks recorded numerical outputs and within-file pair identities, not the
truth of an external raw-data attestation or statistical coverage.
"""
import argparse
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import statistics


def read(path):
    if path is None:
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def finite(x):
    return type(x) in (float, int) and math.isfinite(x)


def med(values):
    return statistics.median(values) if values else None


def switch_summary(rows):
    if not rows:
        return {'status': 'NOT_RUN'}
    groups = defaultdict(list)
    for r in rows:
        assert r['schema'] == 'vigilode.adaptive-entry-pilot.v1'
        assert finite(r['inclusive_seconds']) and r['inclusive_seconds'] > 0
        y, ref = r['physical_state'], r['reference']
        valid = bool(y) and len(y) == len(ref) and all(finite(x) for x in y + ref)
        recomputed = max(abs(a - b) for a, b in zip(y, ref)) if valid else None
        error, u, cap = (r[k] for k in ['physical_endpoint_max_abs_error', 'reference_empirical_uncertainty', 'accuracy_cap'])
        eligible = (valid and all(finite(x) for x in [error, u, cap]) and error >= 0 and u >= 0 and cap > 0
                    and r['success'] and r['reached_time'] == r['target']
                    and math.isclose(recomputed, error, rel_tol=1e-13, abs_tol=1e-30)
                    and Fraction(error) + Fraction(u) <= Fraction(cap))
        r = dict(r, independently_checked_eligible=eligible)
        if not r['warmup']:
            groups[r['case']].append(r)
    cases = []
    for name, rr in sorted(groups.items()):
        by_policy = defaultdict(list)
        by_repeat = defaultdict(dict)
        for r in rr:
            assert r['policy'] not in by_repeat[r['repetition']], 'duplicate case/policy/repetition'
            by_policy[r['policy']].append(r)
            by_repeat[r['repetition']][r['policy']] = r
        policies = {}
        for policy, pp in by_policy.items():
            policies[policy] = {
                'runs': len(pp), 'eligible_runs': sum(r['independently_checked_eligible'] for r in pp),
                'median_inclusive_seconds': med([r['inclusive_seconds'] for r in pp]),
                'median_rhs': med([r['rhs'] for r in pp]),
                'median_attempts': med([r['attempts'] for r in pp]),
                'median_rodas_probe_batches': med([r['detail']['pilot_count'] for r in pp]),
                'probe_count_semantics': 'RODAS entry trials and same-window return comparators, including productive committed comparators; discarded work is counted separately',
                'median_discarded_probe_rhs': med([r['detail']['discarded_pilot_rhs'] for r in pp]),
                'maximum_endpoint_error': max(r['physical_endpoint_max_abs_error'] for r in pp),
                'physical_cap': pp[0]['accuracy_cap'],
                'median_implicit_committed_time': med([r['detail']['implicit_committed_time'] for r in pp]),
                'max_switches': max(r['detail']['switches'] for r in pp)}
        comparisons = []
        for rep, pair in sorted(by_repeat.items()):
            for other in ['evidence', 'dopri', 'rodas']:
                if 'adaptive' not in pair or other not in pair:
                    continue
                a, b = pair['adaptive'], pair[other]
                identity = all(a[k] == b[k] for k in ['case', 'target', 'reference', 'accuracy_cap', 'reference_empirical_uncertainty', 'timing_scope'])
                eligible = identity and a['independently_checked_eligible'] and b['independently_checked_eligible']
                comparisons.append({'repetition': rep, 'alternative': other,
                                    'status': 'COMPARABLE' if eligible else 'EXCLUDED_ACCURACY_OR_IDENTITY',
                                    'adaptive_over_alternative_seconds': a['inclusive_seconds'] / b['inclusive_seconds'] if eligible else None,
                                    'adaptive_over_alternative_rhs': a['rhs'] / b['rhs'] if eligible and b['rhs'] else None})
        cases.append({'case': name, 'policies': policies, 'paired_comparisons': comparisons})
    return {'status': 'MEASURED_SCOPED', 'all_rows_including_warmup': len(rows),
            'timed_rows': sum(len(x) for x in groups.values()), 'cases': cases,
            'comparison_unit': 'whole trajectory, same physical endpoint cap and supplied fixture',
            'per_decision_false_positive_rate': None,
            'rate_limitation': 'pilot segments identify trigger and commitment, but complete-trajectory alternatives do not supply same-checkpoint truth labels for every decision'}


def execution_summary(rows):
    if not rows:
        return {'status': 'NOT_RUN'}
    phases = []
    previous_startup = 0.0
    previous_direct_startup = 0.0
    for phase in rows:
        assert phase['schema'] == 'vigilode.joint-execution-pilot.v1'
        by_arm = defaultdict(list)
        pairs = defaultdict(dict)
        for r in phase['rows']:
            assert finite(r['inclusive_seconds']) and r['inclusive_seconds'] > 0
            assert r['output_parity'] and finite(r['maximum_endpoint_error'])
            assert 0 <= r['maximum_endpoint_error'] <= r['physical_error_cap']
            assert r['arm'] not in pairs[r['repeat']], 'duplicate phase/arm/repeat'
            by_arm[r['arm']].append(r)
            pairs[r['repeat']][r['arm']] = r
        observed = []
        for repeat, pp in sorted(pairs.items()):
            assert set(pp) == {'serial', 'parallel', 'adaptive'}
            serial, parallel, adaptive = [pp[a]['inclusive_seconds'] for a in ['serial', 'parallel', 'adaptive']]
            observed.append({'repeat': repeat, 'adaptive_over_best_direct_seconds': adaptive / min(serial, parallel),
                             'best_observed_direct': 'serial' if serial <= parallel else 'parallel',
                             'decision': pp['adaptive']['decision']})
        startup = phase['startup_seconds_cumulative'] - previous_startup
        assert startup >= 0
        previous_startup = phase['startup_seconds_cumulative']
        direct_cumulative = phase.get('direct_parallel_startup_seconds_cumulative')
        direct_startup = None if direct_cumulative is None else direct_cumulative - previous_direct_startup
        if direct_startup is not None:
            assert direct_startup >= 0
            previous_direct_startup = direct_cumulative
        totals = {a: math.fsum(r['inclusive_seconds'] for r in rr) for a, rr in by_arm.items()}
        charged = totals['adaptive'] + phase['calibration_seconds'] + startup
        phases.append({'phase': phase['phase'], 'physical_span': phase['physical_span'],
                       'deliberately_stale_window_epoch': phase['deliberately_stale_window_epoch'],
                       'evaluated_batches_per_arm': len(pairs),
                       'median_dispatch_seconds': {a: med([r['inclusive_seconds'] for r in rr]) for a, rr in by_arm.items()},
                       'sum_dispatch_seconds': totals,
                       'calibration_seconds_including_warmup': phase['calibration_seconds'],
                       'adaptive_pool_startup_delta_seconds': startup,
                       'adaptive_total_including_calibration_and_pool': charged,
                       'direct_parallel_pool_startup_delta_seconds': direct_startup,
                       'direct_parallel_total_including_pool': None if direct_startup is None else totals['parallel'] + direct_startup,
                       'cost_scope': 'dispatch sums are warmed backend comparisons; adaptive full total separately charges calibration and lazy-pool startup once',
                       'maximum_endpoint_error': max(r['maximum_endpoint_error'] for r in phase['rows']),
                       'all_ordered_outputs_equal': True,
                       'paired_dispatches': observed,
                       'calibration_samples_last': phase['calibration_samples_last'],
                       'telemetry_cumulative': phase['telemetry_cumulative']})
    return {'status': 'MEASURED_SCOPED', 'phases': phases,
            'comparison_unit': 'same-horizon 64-item logistic ODE batches within each phase',
            'future_cost_coverage': 'unmeasured; empirical min/max timing envelope is not a confidence interval',
            'drift_stress_semantics': 'the intentionally unchanged window epoch is an invalid evidence-reuse case used to test observed-cost invalidation'}


def causal_summary(rows):
    if not rows:
        return {'status': 'NOT_RUN'}
    assert len(rows) == 1 and rows[0]['schema'] == 'vigilode.causal-budget-pilot.v1'
    p = rows[0]
    a, b = p['selected'], p['blanket']
    for r in [a, b]:
        assert r['fallback_steps'] == 0 and finite(r['endpoint_absolute_error'])
        assert r['endpoint_absolute_error'] <= 1e-6
        assert r['combined_rhs_and_verifier_evaluations'] == r['candidate_rhs'] + r['interval_verifier_rhs']
        assert r['maximum_output_error_upper'] <= r['inner_output_budget']
    assert all(a[k] == b[k] for k in ['equation', 'steps', 'h', 'inner_output_budget', 'embedded_plus_inner_limit'])
    return {'status': 'MEASURED_SCOPED', 'selected': {k: v for k, v in a.items() if k != 'steps_detail'},
            'blanket': {k: v for k, v in b.items() if k != 'steps_detail'},
            'combined_work_ratio': a['combined_rhs_and_verifier_evaluations'] / b['combined_rhs_and_verifier_evaluations'],
            'correction_rounds_ratio': a['correction_rounds'] / b['correction_rounds'],
            'scope': 'same fixed mesh derivative-free stage-root correction with original-target output-budget admission; no measured parallel speedup or global-error certificate'}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--switch-jsonl', type=Path)
    p.add_argument('--execution-jsonl', type=Path)
    p.add_argument('--causal-jsonl', type=Path)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    paths = {'switching': args.switch_jsonl, 'execution': args.execution_jsonl, 'causal': args.causal_jsonl}
    result = {'schema': 'vigilode.joint-controller.native-pilot-summary.v1',
              'inputs': {k: {'path': str(v), 'sha256': hashlib.sha256(v.read_bytes()).hexdigest()} for k, v in paths.items() if v},
              'identity_scope': 'input source/binary identity is supplied by the parent execution receipts; this summary validates reported numbers and within-fixture comparisons',
              'switching': switch_summary(read(args.switch_jsonl)),
              'execution': execution_summary(read(args.execution_jsonl)),
              'causal': causal_summary(read(args.causal_jsonl))}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as f:
        json.dump(result, f, ensure_ascii=False, indent=2, allow_nan=False)
        f.write('\n')
    print(json.dumps({k: result[k]['status'] for k in paths}))


if __name__ == '__main__':
    main()
