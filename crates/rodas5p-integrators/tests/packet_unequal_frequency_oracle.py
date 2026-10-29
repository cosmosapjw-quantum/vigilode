#!/usr/bin/env python3
"""Exact-rational independent Taylor solution and analytic Cauchy remainder.

Real/imaginary coordinates are analytically continued as independent complex
variables. In max_j(|u_j|+|v_j|), B=max initial L1 norm, Omega=max |omega_j|,
K=|chi| S^3. On the ball of radius 2B, |f|<=2 Omega B+8 K B^3 and
Lip(f)<=Omega+12 K B^2. R=1/(4 Omega+32 K B^2) makes Picard map the ball
into itself and its contraction factor <=3/8. Hence the analytic solution on
|t|<=R has norm<=2B. Cauchy gives a degree-p remainder at t<R bounded by
2B (t/R)^(p+1)/(1-t/R), in every real and imaginary coordinate.

This is independent of packet kernels, candidate polynomials, T1/log norms,
and binary64 interval arithmetic. Input binary64 values are exact rationals.
"""
from fractions import Fraction as F
import json
import sys


def q(x): return F.from_float(float(x))


def coefficients(case, degree=28):
    z = [(q(x), q(y)) for x, y in case['initial']]
    w = list(map(q, case['weights']))
    omega = [q(case['width']) * q(x) for x in case['xi']]
    chi = q(case['chi'])
    B = max(abs(u)+abs(v) for u, v in z)
    K = abs(chi) * sum(w)**3
    denominator = 4*max(map(abs, omega)) + 32*K*B*B
    R = None if denominator == 0 else 1/denominator
    u = [[a] for a, _ in z]; v = [[b] for _, b in z]
    U, V, square = [], [], []
    for k in range(degree):
        U.append(sum((wj*uj[k] for wj, uj in zip(w, u)), F(0)))
        V.append(sum((wj*vj[k] for wj, vj in zip(w, v)), F(0)))
        square.append(sum((U[j]*U[k-j]+V[j]*V[k-j] for j in range(k+1)), F(0)))
        cu = sum((square[j]*U[k-j] for j in range(k+1)), F(0))
        cv = sum((square[j]*V[k-j] for j in range(k+1)), F(0))
        for j in range(len(w)):
            u[j].append((-omega[j]*v[j][k]-chi*cv)/(k+1))
            v[j].append((omega[j]*u[j][k]+chi*cu)/(k+1))
    return w, u, v, B, R


def polynomial(cs, t):
    out = F(0)
    for c in reversed(cs): out = out*t+c
    return out


def contained(center, radius, actual, remainder):
    dr = abs(q(center[0])-actual[0])+remainder
    di = abs(q(center[1])-actual[1])+remainder
    assert dr*dr+di*di <= q(radius)**2, {
        'error_upper': float(dr+di), 'radius': radius, 'reference_remainder': float(remainder)}


def verify(case):
    w, u, v, B, R = coefficients(case)
    checks = 0; max_tail = F(0)
    for output in case['outputs']:
        t = q(output['time'])-q(case.get('initial_time', 0.0))
        assert t >= 0
        if R is None or B == 0:
            tail = F(0)
        else:
            assert t < R, 'oracle analytic disk exceeded'
            ratio = t/R
            tail = 2*B*ratio**len(u[0])/(1-ratio)
        max_tail = max(max_tail, tail)
        exact = [(polynomial(uj, t), polynomial(vj, t)) for uj, vj in zip(u, v)]
        for center, radius, value in zip(output['modes'], output['mode_radii'], exact):
            contained(center, radius, value, tail); checks += 1
        mean = (sum((wj*z[0] for wj,z in zip(w,exact)), F(0)),
                sum((wj*z[1] for wj,z in zip(w,exact)), F(0)))
        contained(output['mean'], output['mean_radius'], mean, sum(w)*tail)
        checks += 1
        weighted_square = sum((wj*((abs(q(c[0])-z[0])+tail)**2+
                                  (abs(q(c[1])-z[1])+tail)**2)
                               for wj,c,z in zip(w,output['modes'],exact)), F(0))
        assert weighted_square <= q(output['weighted_radius'])**2
        checks += 1
    return {'containments': checks, 'max_reference_coordinate_remainder': float(max_tail),
            'degree': len(u[0])-1}


if __name__ == '__main__':
    results = [verify(case) for case in json.load(sys.stdin)['cases']]
    print(json.dumps({'method': 'Fraction Taylor + analytic Cauchy remainder', 'results': results}))
