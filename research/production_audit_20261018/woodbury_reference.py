#!/usr/bin/env python3
"""Exact Fraction reference for collective cubic rank-2 Rosenbrock stage solves.

No runtime/native speed claim. Counts explicit scalar rational operations; Fraction
bit cost, input parsing, allocation, branches and loop overhead are not time units.
Run: python3 research/production_audit_20261018/woodbury_reference.py --output ...
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path


@dataclass
class Ops:
    additions: int = 0
    multiplications: int = 0
    divisions: int = 0

    def add(self, a, b):
        self.additions += 1
        return a + b

    def sub(self, a, b):
        self.additions += 1
        return a - b

    def mul(self, a, b):
        self.multiplications += 1
        return a * b

    def div(self, a, b):
        self.divisions += 1
        return a / b

    def summary(self):
        return dict(additions=self.additions, multiplications=self.multiplications,
                    divisions=self.divisions,
                    total=self.additions + self.multiplications + self.divisions)


def matvec(a, b, op):
    return tuple(op.add(op.mul(row[0], b[0]), op.mul(row[1], b[1])) for row in a)


def matmul(a, b, op):
    return tuple(tuple(op.add(op.mul(a[i][0], b[0][j]), op.mul(a[i][1], b[1][j]))
                       for j in range(2)) for i in range(2))


def inv2(a, op):
    det = op.sub(op.mul(a[0][0], a[1][1]), op.mul(a[0][1], a[1][0]))
    if not det:
        raise ValueError("singular Schur matrix")
    return ((op.div(a[1][1], det), op.div(-a[0][1], det)),
            (op.div(-a[1][0], det), op.div(a[0][0], det))), det


def prepare(xi, weights, state, width, carrier, kappa, chi, shift):
    """Compile one frozen Jacobian at state; all setup arithmetic is charged."""
    if not xi or len(xi) != len(weights) or len(xi) != len(state):
        raise ValueError("nonempty equal input lengths required")
    if any(w <= 0 for w in weights) or sum(weights) != 1:
        raise ValueError("positive normalized weights required")
    if kappa < 0 or shift < 0:
        raise ValueError("nonnegative damping and stage shift required")
    op = Ops()
    mx = my = F(0)
    for w, (x, y) in zip(weights, state):
        mx = op.add(mx, op.mul(w, x))
        my = op.add(my, op.mul(w, y))
    xx, yy, xy = op.mul(mx, mx), op.mul(my, my), op.mul(mx, my)
    c = ((op.mul(-2 * chi, xy), op.mul(-chi, op.add(xx, op.mul(F(3), yy)))),
         (op.mul(chi, op.add(op.mul(F(3), xx), yy)), op.mul(2 * chi, xy)))
    # The two signed constant multipliers 2*chi are also setup arithmetic.
    op.multiplications += 2
    a = op.add(F(1), op.mul(shift, kappa))
    aa = op.mul(a, a)
    p, h = [], [[F(0), F(0)], [F(0), F(0)]]
    for freq, w in zip(xi, weights):
        omega = op.add(carrier, op.mul(width, freq))
        b = op.mul(shift, omega)
        den = op.add(aa, op.mul(b, b))
        pa, pb = op.div(a, den), op.div(b, den)
        pj = ((pa, -pb), (pb, pa))
        p.append(pj)
        for i in range(2):
            for j in range(2):
                h[i][j] = op.add(h[i][j], op.mul(w, pj[i][j]))
    hc = matmul(h, c, op)
    s = tuple(tuple(op.sub(F(i == j), op.mul(shift, hc[i][j]))
                    for j in range(2)) for i in range(2))
    si, det = inv2(s, op)
    # Certificate calculation is charged in setup as well.
    mean_square = op.add(xx, yy)
    q = op.div(op.mul(op.mul(op.mul(F(3), abs(chi)), shift), mean_square), a)
    inverse_bound = op.div(F(1), op.sub(a, op.mul(a, q))) if q < 1 else None
    return dict(block_inverses=p, schur_inverse=si, schur_det=det, c=c,
                weights=tuple(weights), shift=shift, small_gain_q=q,
                weighted_inverse_norm_upper=inverse_bound, setup_ops=op.summary())


def solve(prepared, rhs):
    if len(rhs) != len(prepared['weights']):
        raise ValueError("rhs mode count mismatch")
    op = Ops()
    z = [matvec(p, b, op) for p, b in zip(prepared['block_inverses'], rhs)]
    v = [F(0), F(0)]
    for w, zj in zip(prepared['weights'], z):
        for i in range(2):
            v[i] = op.add(v[i], op.mul(w, zj[i]))
    q = matvec(prepared['schur_inverse'], v, op)
    cq = matvec(prepared['c'], q, op)
    correction = tuple(op.mul(prepared['shift'], x) for x in cq)
    out = []
    for p, zj in zip(prepared['block_inverses'], z):
        delta = matvec(p, correction, op)
        out.append(tuple(op.add(zj[i], delta[i]) for i in range(2)))
    return out, op.summary()


def dense_matrix(xi, weights, state, width, carrier, kappa, chi, shift):
    """Independent dense Jacobian from coordinate partial derivatives."""
    mx = sum(w*z[0] for w, z in zip(weights, state))
    my = sum(w*z[1] for w, z in zip(weights, state))
    c = [[-2*chi*mx*my, -chi*(mx*mx+3*my*my)],
         [chi*(3*mx*mx+my*my), 2*chi*mx*my]]
    n = len(xi)
    a = [[F(i == j) for j in range(2*n)] for i in range(2*n)]
    for j in range(n):
        omega = carrier + width*xi[j]
        d = [[-kappa, -omega], [omega, -kappa]]
        for r in range(2):
            for k in range(n):
                for s in range(2):
                    jac = c[r][s]*weights[k] + (d[r][s] if j == k else F(0))
                    a[2*j+r][2*k+s] -= shift*jac
    return a


def dense_solve(a, rhs):
    """Exact Gaussian elimination, independent of the rank-2 algorithm."""
    n = len(a)
    b = [list(row)+[val] for row, val in zip(a, rhs)]
    for k in range(n):
        pivot = next(i for i in range(k, n) if b[i][k])
        b[k], b[pivot] = b[pivot], b[k]
        for i in range(k+1, n):
            factor = b[i][k]/b[k][k]
            for j in range(k, n+1):
                b[i][j] -= factor*b[k][j]
    x = [F(0)]*n
    for i in reversed(range(n)):
        x[i] = (b[i][n]-sum(b[i][j]*x[j] for j in range(i+1,n)))/b[i][i]
    return x


def checks():
    cases=[]
    for n in (1, 2, 4, 8):
        xi = [F(2*j-n+1, n) for j in range(n)]
        weights = [F(j+1, n*(n+1)//2) for j in range(n)]
        state = [(F(j+1, 4*n), F((-1)**j, 5)) for j in range(n)]
        for kind, width, carrier, kappa, chi, shift, rhs_count in [
            ('native_rotating_frame',F(1),F(0),F(0),F(1,8),F(1,3),4),
            ('undamped',F(1),F(3),F(0),F(1,8),F(1,3),8),
            ('damped_negative_chi',F(2),F(-4),F(2),F(-1,5),F(2,5),3),
            ('zero_width_linear',F(0),F(0),F(0),F(0),F(1),1),
        ]:
            prepared=prepare(xi,weights,state,width,carrier,kappa,chi,shift)
            dense=dense_matrix(xi,weights,state,width,carrier,kappa,chi,shift)
            total_rhs=0
            for r in range(rhs_count):
                rhs=[(F(j+r+1,n+1),F(2*j-r,3*n+1)) for j in range(n)]
                answer,ops=solve(prepared,rhs)
                flat=[v for mode in answer for v in mode]
                flat_rhs=[v for mode in rhs for v in mode]
                assert flat == dense_solve(dense,flat_rhs)
                assert all(sum(a*x for a,x in zip(row,flat)) == b
                           for row,b in zip(dense,flat_rhs))
                assert ops['total'] == 18*n+14
                total_rhs+=ops['total']
            cases.append(dict(kind=kind,n=n,rhs_count=rhs_count,
                setup_ops=prepared['setup_ops'],all_rhs_and_reconstruction_ops=total_rhs,
                total_ops=prepared['setup_ops']['total']+total_rhs,
                schur_determinant=str(prepared['schur_det']),
                small_gain_q=str(prepared['small_gain_q']),
                weighted_inverse_norm_upper=str(prepared['weighted_inverse_norm_upper']),
                exact_dense_equal=True,exact_residual_zero=True))
    # Counting law: for fixed certificate branch, setup is affine in N.
    for kind in {x['kind'] for x in cases}:
        rows=sorted((x for x in cases if x['kind']==kind),key=lambda x:x['n'])
        slope=F(rows[1]['setup_ops']['total']-rows[0]['setup_ops']['total'],rows[1]['n']-rows[0]['n'])
        intercept=rows[0]['setup_ops']['total']-slope*rows[0]['n']
        assert all(x['setup_ops']['total']==slope*x['n']+intercept for x in rows)
    # The norm test is sufficient, not necessary: det(S)=4 despite q=3.
    outside=prepare([F(0)],[F(1)],[(F(1),F(0))],F(0),F(0),F(0),F(1),F(1))
    outside_answer,_=solve(outside,[(F(2),F(1))])
    assert outside['small_gain_q']==3 and outside['schur_det']==4
    assert outside['weighted_inverse_norm_upper'] is None
    assert outside_answer==[(F(1,4),F(7,4))]
    # A valid model state can give a singular Rosenbrock stage; reject it exactly.
    try:
        prepare([F(0)],[F(1)],[(F(1),F(0))],F(0),F(-2),F(0),F(1),F(1))
    except ValueError as error:
        assert str(error)=='singular Schur matrix'
    else:
        raise AssertionError('singular Schur matrix must be rejected')
    return dict(schema='vigilode-woodbury-exact-checks-v1',status='PASS_SCOPED',
        source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        arithmetic='fractions.Fraction, exact rational operations',
        cases=cases,exact_solve_count=sum(x['rhs_count'] for x in cases),
        additional_invertible_outside_small_gain_check=True,
        singular_schur_refusal_check=True,
        counts_scope='all mean/Jacobian and block/Schur setup + inverse bound + every RHS and all-mode reconstruction; unary sign, input parsing and loop/allocation overhead excluded',
        rhs_scalar_operation_formula='18*N + 14',
        native_source_mapping='CollectiveModel::rhs/jvp uses omega=width*xi; pass carrier=0. Nonzero-carrier cases validate the mathematical physical-frame extension only.',
        complexity='O(N + R*N) rational operations and O(N) storage excluding retained R outputs',
        native_wall_time='NOT_RUN',native_integration='NOT_IMPLEMENTED',
        relative_to_packet_speed='NOT_ESTABLISHED',
        proof='WOODBURY_THEORY.md')


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    result=checks()
    out=json.dumps(result,indent=2)+'\n'
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(out)
    else:
        print(out,end='')


if __name__=='__main__':
    main()
