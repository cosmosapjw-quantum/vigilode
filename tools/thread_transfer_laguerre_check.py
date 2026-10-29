#!/usr/bin/env python3
"""Exact checks of the signed output-adjoint Laguerre bound (research node
research/thread_transfer_laguerre_adjoint_20261002, thread-transfer DAG node P1-LAGUERRE-CERT).

Reads fixtures/thread_transfer_laguerre_cases.json, written by the native test: for each case the
operator A = -X (beta = 1), the source w, the stored coefficients c_n, the native recurrence vectors
t_hat_n, the native local residual bounds eps_n, the native envelopes beta_j and the native bound
sum_j beta_j eps_(j-1). Every binary64 value is taken as the exact rational it encodes.

1. Identity: with delta_j = t_hat_j - (a_(j-1)(X) t_hat_(j-1) - (j-1)/j t_hat_(j-2)) exactly,
   sum_n c_n (t_hat_n - t_n) == sum_j z_j(X) delta_j, with t_n the exact recurrence and z_j the
   backward adjoint polynomials. Also ||delta_j|| <= eps_(j-1) (the native residual bounds).
2. Bernstein validity: beta_j >= the exact Bernstein-subdivision bound at the native depth, and
   beta_j >= |z_j(x)| at 257 equally spaced rational points of [0, L]. A deliberately lowered beta
   must fail the same check (the checker is not vacuous).
3. Enclosure: ||sum_n c_n (t_hat_n - t_n)||_2 <= the native bound (compared squared, exactly).
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from fractions import Fraction as F
from math import comb
from pathlib import Path

SCHEMA = "vigilode-thread-transfer-laguerre-exact-check-v1"


def val(h: str) -> F:
    x = struct.unpack(">d", bytes.fromhex(h))[0]
    if not math.isfinite(x):
        raise ValueError(h)
    return F(x)


# Polynomials in x as coefficient lists (lowest first).
def padd(p, q):
    r = [F(0)] * max(len(p), len(q))
    for i, v in enumerate(p):
        r[i] += v
    for i, v in enumerate(q):
        r[i] += v
    return r


def pscale(p, a):
    return [a * x for x in p]


def adjoints(c):
    m = len(c) - 1
    z = [[F(0)] for _ in range(m + 3)]
    for j in range(m, -1, -1):
        a = F(2 * j + 1, j + 1)
        g = F(-1, j + 1)
        lin = padd(pscale(z[j + 1], a), [F(0)] + pscale(z[j + 1], g))
        z[j] = padd(padd([c[j]], lin), pscale(z[j + 2], -F(j + 1, j + 2)))
    return z[: m + 1]


def peval(p, x):
    r = F(0)
    for v in reversed(p):
        r = r * x + v
    return r


def bernstein(p, L):
    d = len(p) - 1
    powers = [p[i] * L**i for i in range(d + 1)]
    return [sum((powers[i] * F(comb(k, i), comb(d, i)) for i in range(k + 1)), F(0)) for k in range(d + 1)]


def split(b):
    r = b[:]
    left, right = [r[0]], [r[-1]]
    for _ in range(len(b) - 1):
        r = [(r[i] + r[i + 1]) / 2 for i in range(len(r) - 1)]
        left.append(r[0])
        right.append(r[-1])
    return left, list(reversed(right))


def bern_bound(b, depth):
    if depth == 0:
        return max(abs(x) for x in b)
    a, c = split(b)
    return max(bern_bound(a, depth - 1), bern_bound(c, depth - 1))


def check_case(case):
    A = [[val(x) for x in row] for row in case["matrix"]]
    X = [[-x for x in row] for row in A]  # beta = 1
    n = len(X)
    L = val(case["extent"])
    w = [val(x) for x in case["source"]]
    c = [val(x) for x in case["stored"]]
    that = [[val(x) for x in v] for v in case["vectors"]]
    eps = [val(x) for x in case["local"]]
    beta = [val(x) for x in case["envelopes"]]
    native_bound = val(case["adjoint_bound"])
    m = case["degree"]
    depth = case["depth"]

    def mv(v):
        return [sum((X[i][k] * v[k] for k in range(n)), F(0)) for i in range(n)]

    # Exact recurrence and exact local defects of the computed vectors.
    exact = [w]
    delta = [None]
    residual_ok = True
    for j in range(1, m + 1):
        k = j - 1
        xt = mv(exact[k])
        prev = exact[k - 1] if k > 0 else [F(0)] * n
        exact.append([(F(2 * k + 1) * exact[k][i] - xt[i] - F(k) * prev[i]) / F(k + 1) for i in range(n)])
        xs = mv(that[k])
        sprev = that[k - 1] if k > 0 else [F(0)] * n
        predicted = [(F(2 * k + 1) * that[k][i] - xs[i] - F(k) * sprev[i]) / F(k + 1) for i in range(n)]
        d = [that[j][i] - predicted[i] for i in range(n)]
        delta.append(d)
        residual_ok &= sum((x * x for x in d), F(0)) <= eps[k] * eps[k]
    error = [sum((c[q] * (that[q][i] - exact[q][i]) for q in range(m + 1)), F(0)) for i in range(n)]
    z = adjoints(c)

    def poly_mv(p, v):
        out = [F(0)] * n
        for coef in reversed(p):
            out = [a + coef * b for a, b in zip(mv(out), v)]
        return out

    transported = [F(0)] * n
    for j in range(1, m + 1):
        transported = [a + b for a, b in zip(transported, poly_mv(z[j], delta[j]))]
    identity = transported == error
    exact_bounds = [bern_bound(bernstein(p, L), depth) for p in z]
    grid = [L * F(i, 256) for i in range(257)]
    def valid_envelopes(b):
        return all(b[j] >= exact_bounds[j] for j in range(m + 1)) and all(
            b[j] >= abs(peval(z[j], x)) for j in range(m + 1) for x in grid)

    valid = valid_envelopes(beta)
    # The check is not vacuous: the same check rejects envelopes with one
    # entry lowered to half the exact Bernstein bound.
    jmax = max(range(m + 1), key=lambda j: exact_bounds[j])
    lowered = list(beta)
    lowered[jmax] = exact_bounds[jmax] / 2
    lowered_rejected = not valid_envelopes(lowered)
    error2 = sum((x * x for x in error), F(0))
    enclosed = error2 <= native_bound * native_bound
    tight = [float(beta[j] / exact_bounds[j]) for j in range(1, m + 1) if exact_bounds[j] > 0]
    return {
        "label": case["label"], "degree": m,
        "identity_exact": identity,
        "native_residual_bounds_hold": residual_ok,
        "bernstein_valid": valid,
        "lowered_beta_rejected": lowered_rejected,
        "enclosed": enclosed,
        "recurrence_error_2norm": math.sqrt(float(error2)),
        "native_adjoint_bound": float(native_bound),
        "native_over_exact_beta_max": max(tight) if tight else None,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.cases.read_text())
    rows = [check_case(c) for c in data["cases"]]
    for r in rows:
        print(r["label"], {k: r[k] for k in ("identity_exact", "bernstein_valid", "enclosed")})
    gate = {
        "backward_identity": all(r["identity_exact"] and r["native_residual_bounds_hold"] for r in rows),
        "bernstein_bounds_valid": all(r["bernstein_valid"] and r["lowered_beta_rejected"] for r in rows),
        "enclosure": all(r["enclosed"] for r in rows),
    }
    report = {"schema": SCHEMA, "cases": rows, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
