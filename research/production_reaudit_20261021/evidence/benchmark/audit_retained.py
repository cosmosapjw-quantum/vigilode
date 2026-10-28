#!/usr/bin/env python3
"""Read-only replay of retained E1b accounting/interpolation; no solver rerun.

This is an independent consumer, not an accuracy oracle. Cross-code agreement
is empirical and cannot by itself bound distance to the exact solution.
"""
import argparse
import collections
import hashlib
import json
import math
import pathlib
import statistics


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def close(a, b):
    return math.isclose(a, b, rel_tol=3e-13, abs_tol=1e-18)


def interpolate(points, level):
    groups = collections.defaultdict(list)
    for error, cost in points:
        if error > 0 and cost > 0 and math.isfinite(error) and math.isfinite(cost):
            groups[math.log(error)].append(cost)
    points = sorted((x, statistics.mean(map(math.log, cs)), cs) for x, cs in groups.items())
    x = math.log(level)
    if not points or x < points[0][0] or x > points[-1][0]:
        return None
    for error, logcost, costs in points:
        if x == error:
            return costs[0] if len(costs) == 1 else math.exp(logcost)
    for (x0, y0, _), (x1, y1, _) in zip(points, points[1:]):
        if x0 < x < x1:
            return math.exp(y0 + (y1 - y0) * (x - x0) / (x1 - x0))
    raise AssertionError('uncovered bracket')


def audit(path):
    d = json.loads(path.read_text())
    c = d['corpus']
    rows = d['rows']
    banded = {'vigilode-banded', 'vigilode-banded-dense', 'vigilode-banded-dense-off',
              'diffsol-bdf-sparse', 'cvode-bdf-band', 'dvode-mf24', 'radau5-band', 'rodas-band'}
    expected = {(p['id'], a, t) for p in c['problems'] for a in c['arms']
                if p['banded'] or a not in banded for t in c['rtols']}
    actual = [(r['problem'], r['arm'], r['rtol']) for r in rows]
    assert len(actual) == len(set(actual)) and set(actual) == expected
    assert len(rows) == 132
    for r in rows:
        reps = r['repetitions']
        assert len(reps) == c['repetitions'] == 21
        assert close(r['wall_ms_median'], statistics.median(x['wall_ns'] / 1e6 for x in reps))
        assert all(x['callbacks'] == r['callbacks'] for x in reps)
        if r['allocations_stable']:
            assert all(x['allocations'] == r['allocations'] for x in reps)
        if r['status']['kind'] == 'success':
            assert r['grid_points_completed'] == c['grid_points']
            assert all(math.isfinite(r[k]) and r[k] >= 0 for k in ('error_wrms_own', 'error_common'))
            assert close(r['error_wrms_own'], r['error_common'] / r['rtol'])
        else:
            assert r['error_wrms_own'] is None and r['error_common'] is None
    for ref in d['references']:
        p = next(p for p in c['problems'] if p['id'] == ref['problem'])
        assert len(ref['states']) == c['grid_points']
        assert all(len(row) == p['n'] and all(map(math.isfinite, row)) for row in ref['states'])
    checks = 0
    for m in d['matched']:
        curves = []
        for arm in [m['arm'], m['baseline']]:
            here = [r for r in rows if r['problem'] == m['problem'] and r['arm'] == arm
                    and r['status']['kind'] == 'success']
            curves.append((interpolate([(r['error_common'], r['wall_ms_median']) for r in here], m['error_level']),
                           interpolate([(r['error_common'], r['callbacks']['rhs']) for r in here], m['error_level'])))
        (ct, cr), (bt, br) = curves
        for actual, expected_value in [(m['comparator_wall_ms'], ct), (m['baseline_wall_ms'], bt),
                                       (m['comparator_rhs'], cr), (m['baseline_rhs'], br)]:
            assert (actual is None) == (expected_value is None)
            assert actual is None or close(actual, expected_value)
        deterministic = all(r['deterministic'] for r in rows if r['problem'] == m['problem']
                            and r['arm'] in (m['arm'], m['baseline']))
        disputed = any(r['disputed'] for r in d['references'] if r['problem'] == m['problem'])
        matched = ct is not None and bt is not None and deterministic and not disputed
        assert (m['status'] == 'MATCHED') == matched
        if matched:
            assert close(m['time_ratio'], ct / bt) and close(m['rhs_ratio'], cr / br)
        else:
            assert m['time_ratio'] is None and m['rhs_ratio'] is None
        assert m['timing_authority'] == 'diagnostic_only'
        checks += 1
    dense = [r for r in rows if r['arm'].startswith('vigilode') and '-dense' in r['arm']]
    assert len(dense) == 30
    assert all(r['native']['output_clipped_steps'] == 0 for r in dense)
    by_problem = []
    for ref in d['references']:
        spread = max(x['disagreement'] for x in [ref['crosscheck']] + ref['informational'])
        ms = [m for m in d['matched'] if m['basis'] == 'dense' and m['status'] == 'MATCHED' and m['problem'] == ref['problem']]
        by_problem.append({'problem': ref['problem'], 'max_observed_reference_disagreement': spread,
                           'matched_dense_levels': len(ms),
                           'levels_within_10x_observed_disagreement': sum(m['error_level'] <= 10 * spread for m in ms),
                           'max_dense_own_error': max(r['error_wrms_own'] for r in dense if r['problem'] == ref['problem'])})
    return {'schema': 'vigilode.production-reaudit.benchmark-replay.v1',
            'input': str(path), 'input_sha256': digest(path),
            'audit_script_sha256': digest(pathlib.Path(__file__)),
            'status': 'PASS_RETAINED_ACCOUNTING_AND_INTERPOLATION',
            'solver_rerun': False, 'new_accuracy_certificate': False,
            'rows': len(rows), 'status_counts': dict(collections.Counter(r['status']['kind'] for r in rows)),
            'repetitions_per_row': c['repetitions'], 'matched_records_recomputed': checks,
            'dense_rows': len(dense), 'dense_error_own_above_one': sum(r['error_wrms_own'] > 1 for r in dense),
            'max_dense_error_own': max(r['error_wrms_own'] for r in dense),
            'dense_matched': sum(m['basis'] == 'dense' and m['status'] == 'MATCHED' for m in d['matched']),
            'dense_total': sum(m['basis'] == 'dense' for m in d['matched']),
            'per_problem': by_problem,
            'historical_verdict': 'INCONCLUSIVE_UNCHANGED', 'timing_authority': d['timing_authority'],
            'limitations': ['No retained per-repetition output states or statuses in historical wp.v1 rows; determinism is a producer assertion, callbacks are replay-checked.',
                            'Observed cross-code disagreement is not an outward exact-solution enclosure.',
                            'Only input accounting and interpolation are independently replayed; no new timing or solver execution.']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    result = audit(args.input)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(result['status'], result['rows'], 'rows;', result['matched_records_recomputed'], 'matched records')
