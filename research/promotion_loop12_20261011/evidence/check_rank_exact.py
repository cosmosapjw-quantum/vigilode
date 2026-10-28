#!/usr/bin/env python3
"""New rank-2 / two-cluster exact algebra checks; no historical campaign replay."""
from fractions import Fraction as F
from math import factorial
import json
from pathlib import Path

def z(a=0, b=0):
    return (F(a), F(b))

def add(a, b):
    return (a[0] + b[0], a[1] + b[1])

def neg(a):
    return (-a[0], -a[1])

def mul(a, b):
    return (a[0]*b[0] - a[1]*b[1], a[0]*b[1] + a[1]*b[0])

def scale(a, b):
    return (a[0]*b, a[1]*b)

def zp(a, n):
    out = z(1)
    for _ in range(n):
        out = mul(out, a)
    return out

def zeros(q):
    return [z() for _ in range(q+1)]

def padd(a, b):
    return [add(x, y) for x, y in zip(a, b)]

def pscale(a, b):
    return [scale(x, b) for x in a]

def pmul(a, b, q):
    out = zeros(q)
    for i, ai in enumerate(a):
        for j, bj in enumerate(b[:q+1-i]):
            out[i+j] = add(out[i+j], mul(ai, bj))
    return out

def conv(a, b, q):
    out = zeros(q)
    for i, ai in enumerate(a):
        for j, bj in enumerate(b):
            k = i+j+1
            if k <= q:
                w = F(factorial(i)*factorial(j), factorial(k))
                out[k] = add(out[k], scale(mul(ai, bj), w))
    return out

def row_apply(row, polys):
    out = zeros(len(polys[0])-1)
    for a, poly in zip(row, polys):
        out = padd(out, pscale(poly, a))
    return out

def aggregate(polys, v):
    return [row_apply(row, polys) for row in v]

def forcing(m, q):
    """Same deterministic degree-3 forcing projection on both paths."""
    a = pmul(m[0], m[1], q)
    b = padd(pmul(m[0], m[0], q), pscale(m[1], -1))
    return [poly[:4]+[z()]*(q-3) for poly in (a, b)]

def check():
    q = 8
    u = [[F(x) for x in row] for row in
         [[1,2],[2,-1],[3,1],[-1,4],[2,3],[4,-2]]]
    v = [[F(x,7) for x in [1,2,-1,3,1,-2]],
         [F(x,11) for x in [2,-1,3,1,-2,4]]]
    y0 = [F(1,4), F(-1,8), F(3,16), F(1,3), F(-1,5), F(1,7)]
    groups = [[0,1,2], [3,4,5]]
    centers = [F(-1), F(-2)]
    detuning = [F(-1,2), F(1,3), F(1,2), F(-1,4), F(0), F(1,4)]
    orders = [2,3]
    assert u[0][0]*u[1][1]-u[0][1]*u[1][0] != 0
    assert v[0][0]*v[1][1]-v[0][1]*v[1][0] != 0
    kernels = [zeros(q) for _ in y0]
    kmat = [[zeros(q) for _ in range(2)] for _ in range(2)]
    free_compiled = [zeros(q) for _ in range(2)]
    for group, center, p in zip(groups, centers, orders):
        for j in group:
            for k in range(p+1):
                for h in range(q+1-k):
                    term = scale(zp(z(0,detuning[j]), k),
                                 center**h/F(factorial(k)*factorial(h)))
                    kernels[j][k+h] = add(kernels[j][k+h], term)
        for k in range(p+1):
            moments = [[sum(v[a][j]*u[j][b]*detuning[j]**k for j in group)
                        for b in range(2)] for a in range(2)]
            initials = [sum(v[a][j]*y0[j]*detuning[j]**k for j in group)
                        for a in range(2)]
            for h in range(q+1-k):
                basis = scale(zp(z(0,1), k),
                              center**h/F(factorial(k)*factorial(h)))
                for a in range(2):
                    free_compiled[a][k+h] = add(free_compiled[a][k+h],
                                                scale(basis, initials[a]))
                    for b in range(2):
                        kmat[a][b][k+h] = add(kmat[a][b][k+h],
                                               scale(basis, moments[a][b]))
    free_full = [pscale(k, a) for k, a in zip(kernels, y0)]
    assert aggregate(free_full, v) == free_compiled
    for a in range(2):
        for b in range(2):
            direct = row_apply([v[a][j]*u[j][b] for j in range(6)], kernels)
            assert direct == kmat[a][b]
    full = free_full
    reduced = free_compiled
    equalities = 0
    for _ in range(4):
        f = forcing(reduced, q)
        ff = forcing(aggregate(full, v), q)
        assert ff == f
        full = [padd(free_full[j], conv(kernels[j], row_apply(u[j], ff), q))
                for j in range(6)]
        nxt = []
        for a in range(2):
            row = free_compiled[a]
            for b in range(2):
                row = padd(row, conv(kmat[a][b], f[b], q))
            nxt.append(row)
        reduced = nxt
        assert aggregate(full, v) == reduced
        equalities += 2*(q+1)
    n, r, nf, sweeps = 1024, 2, 4, 16
    packet_products = n*r*r+n*r+sweeps*(r*r*nf+r*nf)+r*nf+n*r
    full_integration_lower = sweeps*n*(nf-1)
    margin = full_integration_lower-packet_products
    assert (packet_products, full_integration_lower, margin) == (8584,49152,40568)
    packet_additions = (n-1)*r*r+(n-1)*r+sweeps*nf*r*(r-1)+r*(nf-1)+n*(r-1)+n
    packet_ops = packet_products+packet_additions
    assert (packet_additions, packet_ops, full_integration_lower-packet_ops) == (8320,16904,32248)
    x = F(4)
    esum = sum(x**k/F(factorial(k)) for k in range(21))
    exp_upper = esum+(x**21/F(factorial(21)))/(1-F(4,22))
    assert exp_upper < 55
    s = 32
    factorial_bound = F(55)*x**s/F(factorial(s))
    assert factorial_bound < F(1,10**12)
    return {
        "status":"PASS", "evidence_status":"exact-rational-checked",
        "scope":"new algebra fixtures only; no physical oracle/native runtime/history replay",
        "rank":2, "clusters":2, "n":6, "cluster_orders":orders,
        "projection_degree":q, "forcing_degree":3, "sweeps":4,
        "kernel_complex_coefficient_equalities":4*(q+1),
        "free_complex_coefficient_equalities":2*(q+1),
        "same_map_complex_coefficient_equalities":equalities,
        "centers":[str(c) for c in centers],
        "physical_certificate":"NOT_RUN; full continuous residual not certified by Taylor-prefix parity",
        "cost_example":{"n":n,"rank":r,"forcing_coefficients":nf,"sweeps":sweeps,
                        "packet_extra_scalar_multiplications_upper":packet_products,
                        "full_scalar_integration_multiplications_lower":full_integration_lower,
                        "strict_extra_multiplication_cost_margin":margin,
                        "packet_extra_additions_upper":packet_additions,
                        "packet_extra_scalar_ops_upper":packet_ops,
                        "strict_extra_scalar_ops_margin":full_integration_lower-packet_ops,
                        "elapsed_speed_claim":False},
        "factorial_noncontracting_example":{"aT":"4","D0":"1","sweeps":s,
                        "exp4_upper_rational":str(exp_upper),
                        "error_upper":str(factorial_bound),"threshold":"1/1000000000000",
                        "meaning":"checks only explicit arithmetic in theorem bound"}
    }

if __name__ == '__main__':
    output = Path(__file__).with_name('RANK_EXACT_CHECKS.json')
    result = check()
    output.write_text(json.dumps(result, indent=2, ensure_ascii=False)+'\n')
    print(json.dumps({"status":result["status"],"output":str(output)}, ensure_ascii=False))
