#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import mpmath as mp

mp.mp.dps = 100

MATRICES = {
    "nonnormal-upper-chain": (
        mp.matrix([
            [-5, 8, 0, 0],
            [0, -6, 7, 0],
            [0, 0, -7, 6],
            [0, 0, 0, -8],
        ]),
        mp.matrix([1, mp.mpf('-0.25'), mp.mpf('0.5'), mp.mpf('-0.75')]),
    ),
    "oscillatory-damped-block": (
        mp.matrix([[-3, -12], [12, -3]]),
        mp.matrix([1, mp.mpf('0.25')]),
    ),
}


def phi_action(A, scale, k, v):
    n = A.rows
    if k == 0:
        return mp.expm(scale * A) * v
    aug = mp.matrix(n + k, n + k)
    for i in range(n):
        for j in range(n):
            aug[i, j] = scale * A[i, j]
        aug[i, n] = v[i]
    for j in range(k - 1):
        aug[n + j, n + j + 1] = 1
    exp_aug = mp.expm(aug)
    return mp.matrix([exp_aug[i, n + k - 1] for i in range(n)])


def phi_scalar(z, k):
    return phi_action(mp.matrix([[mp.mpf(z)]]), mp.mpf(1), k, mp.matrix([1]))[0]


def coefficient_residuals():
    c2 = mp.mpf(1) / 4
    c3 = mp.mpf(1) / 2
    c4 = mp.mpf(9) / 10
    a32 = mp.mpf(4)
    a42 = mp.mpf(2916) / 125
    rows = []
    for z in [-5, -1, mp.mpf('-0.1'), 0, mp.mpf('0.5'), 2]:
        p3 = phi_scalar(z, 3)
        p4 = phi_scalar(z, 4)
        b3 = 18 * p3 - 60 * p4
        b4 = -mp.mpf(250) / 81 * p3 + mp.mpf(500) / 27 * p4
        main1 = b3*c3**2 + b4*c4**2 - 2*p3
        main2 = b3*c3**3 + b4*c4**3 - 6*p4
        p3c3 = phi_scalar(c3*z, 3)
        p3c4 = phi_scalar(c4*z, 3)
        psi3 = a32*p3c3*c2**2/2 - c3**3*p3c3
        psi4 = a42*p3c4*c2**2/2 - c4**3*p3c4
        eb2 = 64*p3 - 60*p4
        eb3 = -8*p3 - mp.mpf(285)/8*p4
        eb4 = mp.mpf(125)/8*p4
        emb1 = eb2*c2**2 + eb3*c3**2 + eb4*c4**2 - 2*p3
        emb2 = eb2*c2**3 + eb3*c3**3 + eb4*c4**3 - 6*p4
        rows.extend([main1, main2, psi3, psi4, emb1, emb2])
    p30 = phi_scalar(0,3); p40 = phi_scalar(0,4)
    b30 = 18*p30-60*p40
    b40 = -mp.mpf(250)/81*p30 + mp.mpf(500)/27*p40
    rows.append(b30*c3**4+b40*c4**4-mp.mpf(1)/5)
    return max(abs(x) for x in rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('rust_json', type=Path)
    ap.add_argument('output', type=Path)
    ns = ap.parse_args()
    data = json.loads(ns.rust_json.read_text())
    max_dense_abs = mp.mpf(0)
    max_krylov_abs = mp.mpf(0)
    comparisons = 0
    for row in data['phi_oracle']:
        A, v = MATRICES[row['case_id']]
        ref = phi_action(A, mp.mpf(str(row['scale'])), int(row['phi_index']), v)
        for rust_dense, rust_krylov, ref_value in zip(row['dense_value'], row['krylov_value'], ref):
            max_dense_abs = max(max_dense_abs, abs(mp.mpf(str(rust_dense)) - ref_value))
            max_krylov_abs = max(max_krylov_abs, abs(mp.mpf(str(rust_krylov)) - ref_value))
            comparisons += 1
    coeff = coefficient_residuals()
    result = {
        'schema': 'generic-g2-independent-high-precision-v1',
        'mpmath_decimal_digits': mp.mp.dps,
        'comparisons': comparisons,
        'maximum_rust_dense_phi_absolute_error': float(max_dense_abs),
        'maximum_rust_krylov_phi_absolute_error': float(max_krylov_abs),
        'maximum_exact_coefficient_condition_residual': float(coeff),
        'dense_phi_pass': bool(max_dense_abs < mp.mpf('5e-13')),
        'krylov_phi_pass': bool(max_krylov_abs < mp.mpf('5e-13')),
        'coefficient_pass': bool(coeff < mp.mpf('1e-80')),
    }
    result['overall_pass'] = all(result[k] for k in ['dense_phi_pass','krylov_phi_pass','coefficient_pass'])
    ns.output.parent.mkdir(parents=True, exist_ok=True)
    ns.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(result, indent=2, sort_keys=True))

if __name__ == '__main__':
    main()
