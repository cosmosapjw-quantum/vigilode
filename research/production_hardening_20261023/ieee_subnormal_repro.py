#!/usr/bin/env python3
"""Independent binary64 transcription; does not execute the Rust solver.

Reproduce the old DOPRI5 multiply order on a constant RHS, then the new
small-step multiply order. The result is numeric diagnostic evidence only.
"""
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys

BASELINE = 'fbc3c272fa06b443de148631ea78bfa6a34f1b2b'
SOURCE = 'crates/rodas5p-integrators/src/dopri5.rs'
A = [
    [], [1/5], [3/40, 9/40], [44/45, -56/15, 32/9],
    [19372/6561, -25360/2187, 64448/6561, -212/729],
    [9017/3168, -355/33, 46732/5247, 49/176, -5103/18656],
    [35/384, 0, 500/1113, 125/192, -2187/6784, 11/84],
]
E = [71/57600, 0, -71/16695, 71/1920, -17253/339200, 22/525, -1/40]

def trial(h, derivative, revised):
    states = [0.0]
    for row in A[1:]:
        y = 0.0
        for a in row:
            if revised:
                if a != 0.0:
                    y += a * (h * derivative)
            else:
                weight = h * a
                if weight != 0.0:
                    y += weight * derivative
        states.append(y)
    error = 0.0
    for a in E:
        if revised:
            if a != 0.0:
                error += a * (h * derivative)
        else:
            weight = h * a
            if weight != 0.0:
                error += weight * derivative
    return states[-1], error, states


def main():
    eta = math.ulp(0.0)
    h = 2.0 * eta
    original = subprocess.check_output(['git', 'show', f'{BASELINE}:{SOURCE}'])
    rows = []
    for derivative in [1e308, -1e308]:
        exact = h * derivative
        for name, revised in [('baseline_h_times_coefficient_first', False),
                              ('revised_h_times_derivative_first', True)]:
            endpoint, embedded, stages = trial(h, derivative, revised)
            error = abs(endpoint - exact)
            rows.append({
                'algorithm': name, 'h': h, 'derivative': derivative,
                'exact_constant_rhs_increment': exact,
                'computed_endpoint': endpoint, 'absolute_error': error,
                'relative_error': error / abs(exact),
                'embedded_difference': embedded,
                'stage_states': stages,
                'constant_rhs_consistency_within_32_eps':
                    error <= 32 * sys.float_info.epsilon * abs(exact),
            })
    result = {
        'schema': 'vigilode.independent-ieee-reproduction.v1',
        'evidence_kind': 'python_binary64_transcription_not_rust_solver_execution',
        'baseline_commit': BASELINE, 'baseline_source': SOURCE,
        'baseline_source_sha256': hashlib.sha256(original).hexdigest(),
        'python_version': sys.version,
        'float_radix': sys.float_info.radix, 'float_mantissa_bits': sys.float_info.mant_dig,
        'minimum_positive_subnormal': eta,
        'endpoint_weight_units_of_minimum_subnormal': [h * a / eta for a in A[-1]],
        'embedded_weight_units_of_minimum_subnormal': [h * a / eta for a in E],
        'scope': 'Constant finite RHS and finite endpoint increment on h=2*minimum subnormal; not a general below-resolution convergence guarantee.',
        'rows': rows,
    }
    output = Path(__file__).parent / 'evidence' / 'IEEE_SUBNORMAL_REPRO.json'
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    print(json.dumps({'output': str(output), 'baseline_relative_errors': [r['relative_error'] for r in rows if r['algorithm'].startswith('baseline')], 'revised_relative_errors': [r['relative_error'] for r in rows if r['algorithm'].startswith('revised')]}))

if __name__ == '__main__':
    main()
