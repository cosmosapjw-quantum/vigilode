#!/usr/bin/env python3
"""Independent exact-binary-rational oracle for *new native outputs* only.

No archived code is imported or executed. Gaussian elimination over Fraction
solves the current target; squared norms avoid irrational comparison ambiguity.
"""
import argparse
from fractions import Fraction as F
from pathlib import Path
import json
import math
import struct


def exact(x):
    if isinstance(x, int):
        return F(x)
    return F.from_float(x)


def solve(a, b):
    n = len(b)
    m = [a[i][:] + [b[i]] for i in range(n)]
    for k in range(n):
        pivot = next((i for i in range(k, n) if m[i][k]), None)
        if pivot is None:
            raise ValueError('singular exact oracle target')
        m[k], m[pivot] = m[pivot], m[k]
        v = m[k][k]
        m[k] = [x / v for x in m[k]]
        for i in range(n):
            if i != k:
                c = m[i][k]
                m[i] = [x - c*y for x, y in zip(m[i], m[k])]
    return [row[-1] for row in m]


def ieee(value):
    if isinstance(value, list):
        return [ieee(v) for v in value]
    return struct.pack('>d', float(value)).hex()


def verify(case):
    bits = case['exact_binary_hex']
    for key in ('h', 'gamma0', 'gammas', 'rhs_columns'):
        assert ieee(case[key]) == bits[key], 'input binary identity mismatch'
    assert ieee(case['j']['row_major']) == bits['j_row_major']
    assert len(bits['candidates']) == len(case['report']['candidates'])
    for candidate, bit in zip(case['report']['candidates'], bits['candidates']):
        for key in ('gamma','rhs_columns'):
            assert ieee(candidate[key]) == bit[key], 'output binary identity mismatch'
        assert ieee(candidate['certificate']['rhs_error_upper']) == bit['rhs_error_upper']
    jraw = case['j']
    n = jraw['nrows']
    assert jraw['ncols'] == n
    assert n > 0 and len(jraw['row_major']) == n*n
    j = [[exact(x) for x in jraw['row_major'][i*n:(i+1)*n]] for i in range(n)]
    h = exact(case['h'])
    rhs = [[exact(x) for x in b] for b in case['rhs_columns']]
    assert rhs and all(len(b)==n for b in rhs)
    assert h >= 0
    assert len(case['report']['candidates']) == len(case['gammas'])
    for i in range(n):
        assert j[i][i] + sum(abs((j[i][k]+j[k][i])/2) for k in range(n) if k != i) <= 0
    rows = []
    for requested_gamma, target in zip(case['gammas'], case['report']['candidates']):
        g = exact(target['gamma'])
        assert g == exact(requested_gamma), 'target/request identity mismatch'
        a = [[F(i == k)-g*h*j[i][k] for k in range(n)] for i in range(n)]
        cert = target['certificate']
        assert cert['status'] in ('Certified','Rejected')
        assert len(target['rhs_columns']) == len(rhs) == len(cert['rhs_error_upper'])
        assert all(len(u)==n for u in target['rhs_columns'])
        assert all(math.isfinite(x) and x>=0 for x in cert['rhs_error_upper'])
        assert cert['absolute_tolerance'] == case['config']['absolute_tolerance']
        assert g > 0
        for c, (b, uraw, upper) in enumerate(zip(rhs, target['rhs_columns'], cert['rhs_error_upper'])):
            u = [exact(x) for x in uraw]
            true = solve(a, b)
            error_sq = sum((x-y)**2 for x,y in zip(true, u))
            residual = [b[i]-sum(a[i][k]*u[k] for k in range(n)) for i in range(n)]
            residual_l1 = sum(abs(x) for x in residual)
            q = exact(upper)
            tol = exact(cert['absolute_tolerance'])
            enclosed = q >= 0 and error_sq <= q*q and residual_l1 <= q
            accepted = cert['status'].lower() == 'certified'
            safe_accept = not accepted or (q <= tol and error_sq <= tol*tol)
            rows.append({'case':case['id'],'gamma':float(g),'rhs_column':c,
                         'status':cert['status'],'exact_error_sq':str(error_sq),
                         'exact_residual_l1':str(residual_l1),
                         'reported_upper':upper,'enclosed':enclosed,
                         'safe_acceptance':safe_accept})
    return rows


def main():
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser()
    ap.add_argument('--input', type=Path, default=here/'evidence/native_study.json')
    ap.add_argument('--output', type=Path, default=here/'evidence/exact_oracle.json')
    args = ap.parse_args()
    data = json.loads(args.input.read_text())
    rows = [row for case in data['cases'] for row in verify(case)]
    failures = [r for r in rows if not r['enclosed'] or not r['safe_acceptance']]
    negatives = data.get('negative_controls', [])
    negative_failures = [r for r in negatives if not r['rejected']]
    summary = {'schema':'rvj-native-exact-oracle-v1',
        'reference':'independent stdlib Fraction Gauss-Jordan; exact binary f64 inputs',
        'historical_tests_rerun':False,'case_count':len(data['cases']),
        'rhs_target_rows':len(rows),'certified_rows':sum(r['status'].lower()=='certified' for r in rows),
        'rejected_rows':sum(r['status'].lower()=='rejected' for r in rows),
        'enclosure_failures':len(failures),'negative_control_count':len(negatives),
        'negative_control_failures':len(negative_failures),
        'pass':bool(rows) and not failures and not negative_failures,
        'rows':rows,'negative_controls':negatives}
    args.output.write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps({k:v for k,v in summary.items() if k not in ('rows','negative_controls')}))
    raise SystemExit(0 if summary['pass'] else 1)

if __name__ == '__main__':
    main()
