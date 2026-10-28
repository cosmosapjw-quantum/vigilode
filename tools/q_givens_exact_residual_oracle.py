#!/usr/bin/env python3
"""Check exported binary64 GMRES fixtures with exact rational arithmetic.

The comparison uses squared norms of the exact represented A, b, x, rtol and
atol. Decimal120 values are display diagnostics, never the acceptance gate.
The script only reads exports and writes one create-only receipt.
"""

import argparse
from decimal import Decimal, localcontext
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct


def from_bits(word):
    value = struct.unpack('>d', bytes.fromhex(word))[0]
    # Fraction rejects nonfinite values; no nonfinite result can certify.
    return Fraction.from_float(value)


def read_export(path):
    lines = path.read_text().splitlines()
    values = {'schema': lines[0]}
    for line in lines[1:]:
        name, *fields = line.split()
        if name == 'status':
            values[name] = ' '.join(fields)
        elif name in ('dimension', 'restart', 'max_arnoldi', 'iterations'):
            values[name] = int(fields[0])
        else:
            count = int(fields[0])
            assert len(fields[1:]) == count, (path, name, count)
            values[name] = [from_bits(word) for word in fields[1:]]
    return values


def decimal_value(value):
    return Decimal(value.numerator) / Decimal(value.denominator)


def decimal_sqrt(value):
    with localcontext() as context:
        context.prec = 120
        return str(decimal_value(value).sqrt())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_result(fixture, path):
    result = read_export(path)
    assert result['schema'] == 'q-givens-result-bits-v1'
    row = {'file': path.name, 'sha256': digest(path), 'status': result['status']}
    if result['status'] != 'ok':
        row['verdict'] = 'NO_RETURNED_SOLUTION'
        return row

    n = fixture['dimension']
    matrix, rhs, solution = fixture['a_row_major'], result['b'], result['x']
    assert len(matrix) == n * n and len(rhs) == len(solution) == n
    residual = [
        rhs[i] - sum((matrix[i * n + j] * solution[j] for j in range(n)), Fraction())
        for i in range(n)
    ]
    residual_squared = sum((value * value for value in residual), Fraction())
    rhs_squared = sum((value * value for value in rhs), Fraction())
    rtol, atol = fixture['rtol'][0], fixture['atol'][0]
    requested_threshold_squared = max(atol * atol, rtol * rtol * rhs_squared)
    solver_threshold_squared = result['binary64_threshold'][0] ** 2

    # Corrupt one returned component by a finite, material binary64 change.
    x0 = float(solution[0])
    corrupted_x0 = Fraction.from_float(x0 + max(1.0, abs(x0)))
    delta = corrupted_x0 - solution[0]
    assert delta != 0
    corrupted_residual_squared = sum(
        ((residual[i] - matrix[i * n] * delta) ** 2 for i in range(n)), Fraction()
    )
    requested_pass = residual_squared <= requested_threshold_squared
    binary64_pass = residual_squared <= solver_threshold_squared
    corrupted_rejected = corrupted_residual_squared > requested_threshold_squared
    corrupted_binary64_rejected = corrupted_residual_squared > solver_threshold_squared
    row.update({
        'iterations': result['iterations'],
        'acceptance_arithmetic': 'exact_rational_squared_norm',
        'decimal_display_precision': 120,
        'exact_residual_norm_decimal': decimal_sqrt(residual_squared),
        'exact_requested_threshold_decimal': decimal_sqrt(requested_threshold_squared),
        'binary64_solver_threshold_decimal': decimal_sqrt(solver_threshold_squared),
        'exact_residual_le_requested_threshold': requested_pass,
        'exact_residual_le_binary64_solver_threshold': binary64_pass,
        'corrupted_x_rejected': corrupted_rejected,
        'corrupted_x_rejected_at_binary64_threshold': corrupted_binary64_rejected,
        'corrupted_residual_norm_decimal': decimal_sqrt(corrupted_residual_squared),
        'verdict': 'PASS_SCOPED' if all((requested_pass, binary64_pass, corrupted_rejected, corrupted_binary64_rejected)) else 'FAIL',
    })
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fixture', type=Path)
    parser.add_argument('results', nargs='+', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    fixture = read_export(args.fixture)
    assert fixture['schema'] == 'q-givens-exact-bits-v1'
    n = fixture['dimension']
    assert len(fixture['a_row_major']) == n * n
    assert len(fixture['b_unit']) == len(fixture['jacobi_inverse_diagonal']) == n
    # Confirm the exported preconditioner is the one formed for these A bits.
    for i, inverse in enumerate(fixture['jacobi_inverse_diagonal']):
        diagonal = float(fixture['a_row_major'][i * n + i])
        assert inverse == Fraction.from_float(1.0 / diagonal)
    rows = [check_result(fixture, path) for path in args.results]
    checked_ok = sum(row['status'] == 'ok' for row in rows)
    verdict = 'FAIL' if any(row['verdict'] == 'FAIL' for row in rows) else ('PASS_SCOPED' if checked_ok else 'NOT_EVALUABLE')
    report = {
        'schema': 'q-givens-exact-residual-oracle-v1',
        'returned_solutions_checked': checked_ok,
        'oracle_sha256': digest(Path(__file__)),
        'fixture_sha256': digest(args.fixture),
        'fixture_dimension': n,
        'jacobi_inverse_matches_fixture': True,
        'rows': rows,
        'claim_ceiling': 'exact represented linear-system residual only; no physical/global convergence claim',
        'verdict': verdict,
    }
    with args.output.open('x') as output:
        json.dump(report, output, indent=2)
        output.write('\n')
    print(json.dumps({'oracle_receipt': str(args.output), 'verdict': report['verdict']}))
    return 0 if report['verdict'] == 'PASS_SCOPED' else 1


if __name__ == '__main__':
    raise SystemExit(main())
