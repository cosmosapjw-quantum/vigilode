#!/usr/bin/env python3
"""Independent evidence review only: no native run or historical campaign.

Uses exact rational elimination and checks every exported intermediate, not the
native interval implementation nor the study's check.py. Fixture identity is
reconstructed from the preregistration; this is not a new discovery campaign.
"""
import hashlib
import json
import math
from fractions import Fraction as Q
from pathlib import Path

HERE = Path(__file__).resolve().parent
NODE = HERE.parent
ROOT = NODE.parents[1]


def q(value):
    assert not isinstance(value, bool) and isinstance(value, (int, float))
    assert math.isfinite(value)
    return Q(value)


def contains(interval, exact):
    assert len(interval) == 2
    lo, hi = map(q, interval)
    return lo <= exact <= hi


def review_cell(cell):
    assert cell['status'] == 'admitted'
    w = [[q(v) for v in row] for row in cell['W']]
    b, x, s = ([q(v) for v in cell[key]] for key in ('b', 'x', 'scales'))
    assert len(w) == 2 and all(len(row) == 2 for row in w)
    assert len(b) == len(x) == len(s) == 2 and min(s) > 0
    det = w[0][0]*w[1][1]-w[0][1]*w[1][0]
    assert det != 0
    inv = [[w[1][1]/det, -w[0][1]/det], [-w[1][0]/det, w[0][0]/det]]
    residual = [b[i]-sum(w[i][j]*x[j] for j in range(2)) for i in range(2)]
    correction = [sum(inv[i][j]*residual[j] for j in range(2)) for i in range(2)]
    weighted = [correction[i]/s[i] for i in range(2)]
    cert = cell['certificate']
    checks = {
        'determinant': contains(cell['determinant_interval'], det),
        'inverse': all(contains(cell['inverse_intervals'][i][j], inv[i][j]) for i in range(2) for j in range(2)),
        'residual': all(contains(cert['residual_intervals'][i], residual[i]) for i in range(2)),
        'correction': all(contains(cert['correction_intervals'][i], correction[i]) for i in range(2)),
        'weighted_correction': all(contains(cert['weighted_correction_intervals'][i], weighted[i]) for i in range(2)),
        'infinity_bound': q(cert['weighted_inf_upper']) >= max(map(abs, weighted)),
        'wrms_bound': q(cert['wrms_upper']) >= 0 and q(cert['wrms_upper'])**2 >= sum(v*v for v in weighted)/2,
        'gain_bound': q(cert['scaled_residual_gain_inf_upper']) >= max(sum(abs(inv[i][j])*s[j]/s[i] for j in range(2)) for i in range(2)),
    }
    assert all(checks.values()), (cell['id'], checks)
    return {'id': cell['id'], 'checks': checks, 'zero_exact_error': all(v == 0 for v in weighted)}


def expected_cells():
    expected = {}
    perturbation = 2.0**-40
    for k in (0, 1, 1024, 1048576):
        for si, scales in enumerate(([1.0, 1.0], [2.0**-20, 2.0**20], [2.0**20, 2.0**-20])):
            for coordinate in range(2):
                x = [1.0-k, 1.0]
                x[coordinate] += perturbation
                expected[f'grid_k{k}_s{si}_p{coordinate}'] = ('discovery', [[1.0, k], [0.0, 1.0]], [1.0, 1.0], x, scales)
    expected['control_symmetric'] = ('control', [[3.0, 1.0], [1.0, 2.0]], [1.0, 1.0], [0.2+perturbation, 0.4], [1.0, 1.0])
    expected['control_near_singular'] = ('control', [[1.0, 1.0], [1.0, 1.0+perturbation]], [1.0, 1.0], [1.0, perturbation], [1.0, 1.0])
    for k in (7, 65536):
        for coordinate in range(2):
            x = [3.0+2*k, -2.0]
            x[coordinate] += 2.0**-30
            expected[f'holdout_k{k}_p{coordinate}'] = ('fixed_holdout', [[1.0, k], [0.0, 1.0]], [3.0, -2.0], x, [2.0**-7, 2.0**9])
    return expected


def main():
    raw = json.loads((NODE/'NATIVE.json').read_text())
    first = json.loads((NODE/'NATIVE_FIRST.json').read_text())
    cells = raw['gain_study']['cells']
    expected = expected_cells()
    assert len(cells) == 30 and {c['id'] for c in cells} == set(expected)
    for cell in cells:
        assert tuple(cell[key] for key in ('partition', 'W', 'b', 'x', 'scales')) == expected[cell['id']], cell['id']
    checks = [review_cell(cell) for cell in cells]
    assert first['gain_study']['cells'] == cells
    assert first['implementation_adversaries'] == raw['implementation_adversaries']
    old_invalid = {c['id']: c for c in first['gain_study']['invalid_controls']}
    invalid = {c['id']: c for c in raw['gain_study']['invalid_controls']}
    assert len(invalid) == len(old_invalid) == 12
    changed = [key for key in invalid if invalid[key] != old_invalid[key]]
    assert changed == ['invalid_scale_gain_overflow']
    assert all(c['status'] == 'rejected' for c in invalid.values())
    old = old_invalid[changed[0]]
    old_check = review_cell(old)
    assert old['W'] == [[1.0, 0.0], [0.0, 1.0]] and old_check['zero_exact_error']
    new = invalid[changed[0]]
    assert new['W'] == [[1.0, 1.0], [0.0, 1.0]]
    for key in ('b', 'x', 'scales', 'id', 'partition'):
        assert new[key] == old[key]
    exact_new_gain = 1+1/q(new['scales'][0])
    assert exact_new_gain > q(float.fromhex('0x1.fffffffffffffp+1023'))
    assert json.loads((NODE/'RESULTS_FIRST.json').read_text())['verdict'] == 'FAIL'
    assert json.loads((NODE/'RESULTS.json').read_text())['verdict'] == 'PASS'
    adversaries = raw['implementation_adversaries']
    underflow = adversaries['controller'][0]
    h, error = map(q, (underflow['trial']['h'], underflow['trial']['error']))
    # Exact positive fifth powers decide clamping without any logarithm/powf.
    raw_factor_fifth = q(0.9)**5*h**5*q(0.5)/(error*error)
    assert raw_factor_fifth < q(0.2)**5
    assert q(0.9)**5/error > q(5.0)**5
    assert underflow['observed']['factor'] == 5.0
    overflow = adversaries['fallback_seam'][0]
    residual_sq = sum(q(v)**2 for v in overflow['callback']['scaled_true_residual'])
    rhs_sq = 2*q(1.5)**2
    assert residual_sq/rhs_sq > q(overflow['production_rtol'])**2
    assert overflow['callback']['literal_driver_predicate_accepts']
    assert not overflow['direct_driver_reproduction'] and not overflow['full_ode_trajectory_reproduction']
    above_target = [c['id'] for c in adversaries['stage_acceptance'] if c['observed']['accepted_above_requested_target']]
    assert above_target == ['triangular2_stall', 'triangular2_floor', 'triangular4_floor']
    probes = json.loads((NODE/'CHECKER_PROBES.json').read_text())
    assert all(hashlib.sha256((ROOT/path).read_bytes()).hexdigest() == digest for path, digest in probes['source_hashes'].items())
    assert probes['source_hashes_unchanged'] and len(probes['helper_probes']) == 15 and len(probes['end_to_end_probes']) == 4
    assert all(p['malformed_copy_passed'] for p in probes['end_to_end_probes'])
    result = {
        'schema_version': '1.0',
        'decision': 'PASS_BOUNDED_EVIDENCE_REVIEW',
        'execution_scope': 'exact checks of existing new-study raw evidence only; no native rerun or old campaign',
        'native_sha256': hashlib.sha256((NODE/'NATIVE.json').read_bytes()).hexdigest(),
        'checks': checks,
        'positive_fixture_identity': '30/30 exact registered inputs matched',
        'positive_cells_unchanged_after_fixture_correction': True,
        'implementation_adversaries_unchanged_after_fixture_correction': True,
        'zero_error_positive_cells': sum(c['zero_exact_error'] for c in checks),
        'surviving_nonzero_error_positive_cells': sum(not c['zero_exact_error'] for c in checks),
        'invalid_controls_rejected': 12,
        'old_fixture_admission_sound': old_check,
        'new_fixture_scaled_gain_exceeds_binary64_max': True,
        'first_fail_preserved': True,
        'predictive_factor_exact_positive_fifth_power_check': 'min_factor=0.2 mathematically; observed factor=5.0',
        'overflow_callback_exact_scaled_relative_error_exceeds_target': True,
        'overflow_scope_kept_at_callback_seam': True,
        'public_staged_solver_above_target_cases': above_target,
        'historical_probe_source_hashes_match': True,
        'limitations': ['No theorem about unrepresented W or approximate JVP.', 'No whole-integrator, global-accuracy, speed, or default-dispatch evidence.', 'Exact fixture checks here supplement check.py, which does not itself bind the complete input manifest.'],
    }
    (HERE/'independent_exact_checks.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    print(json.dumps({k: v for k, v in result.items() if k not in ('checks', 'old_fixture_admission_sound', 'limitations')}, indent=2))


if __name__ == '__main__':
    main()
