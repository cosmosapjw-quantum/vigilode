#!/usr/bin/env python3
"""Exact-rational supporting checks; no native solver or performance claims."""

from fractions import Fraction as F
from pathlib import Path
import argparse
import datetime
import hashlib
import json


def zeros(m, n):
    return [[F(0) for _ in range(n)] for _ in range(m)]


def eye(n):
    return [[F(i == j) for j in range(n)] for i in range(n)]


def add(a, b):
    return [[x + y for x, y in zip(ar, br)] for ar, br in zip(a, b)]


def scale(c, a):
    return [[c * x for x in row] for row in a]


def transpose(a):
    return [list(row) for row in zip(*a)]


def mul(a, b):
    bt = transpose(b)
    return [[sum(x * y for x, y in zip(row, col)) for col in bt] for row in a]


def mv(a, x):
    return [sum(u * v for u, v in zip(row, x)) for row in a]


def inverse(a):
    """Independent rational Gauss-Jordan inversion for small full matrices."""
    n = len(a)
    aug = [row[:] + unit for row, unit in zip(a, eye(n))]
    for k in range(n):
        pivot = next(i for i in range(k, n) if aug[i][k])
        aug[k], aug[pivot] = aug[pivot], aug[k]
        value = aug[k][k]
        aug[k] = [x / value for x in aug[k]]
        for i in range(n):
            if i != k:
                value = aug[i][k]
                aug[i] = [x - value * y for x, y in zip(aug[i], aug[k])]
    return [row[n:] for row in aug]


def norm_inf(a):
    return max(sum(abs(x) for x in row) for row in a)


def dot(x, y):
    return sum(u * v for u, v in zip(x, y))


def quadratic(c, x):
    return dot(x, mv(c, x))


def vector_add(x, y):
    return [u + v for u, v in zip(x, y)]


def vector_scale(c, x):
    return [c * u for u in x]


def nu(n, t, c):
    off = (c - t) / n
    return abs(t + off) + (n - 1) * abs(off)


def check_variable_step_transport():
    n, s = 3, 3
    gamma = F(1, 2)
    decay, coupling = F(3, 2), F(2, 3)
    rates = [decay, decay - coupling]
    hs = [F(1, 7), F(2, 9), F(5, 3)]
    a = [[F(0), F(0), F(0)], [F(1, 3), F(0), F(0)],
         [F(-1, 4), F(2, 5), F(0)]]
    c = [[F(0), F(0), F(0)], [F(-2, 5), F(0), F(0)],
         [F(1, 6), F(-1, 3), F(0)]]
    beta = [F(3, 7), F(2, 7), F(2, 7)]
    p = [[F(1, n) for _ in range(n)] for _ in range(n)]
    q = add(eye(n), scale(F(-1), p))
    j = add(scale(-decay, q), scale(-(decay - coupling), p))
    endpoint = [[beta[k] if r == component else F(0)
                 for k in range(s) for component in range(n)] for r in range(n)]
    full_step, full_transfer, weights, sigmas = [], [], [], []
    for h in hs:
        full = zeros(s * n, s * n)
        for i in range(s):
            for k in range(i + 1):
                if i == k:
                    block = add(eye(n), scale(-h * gamma, j))
                else:
                    block = add(scale(-gamma * c[i][k], eye(n)),
                                scale(-h * gamma * a[i][k], j))
                for r in range(n):
                    for col in range(n):
                        full[i * n + r][k * n + col] = block[r][col]
        full_inv = inverse(full)
        transfer = mul(endpoint, full_inv)
        rhs_derivative = [row[:] for _ in range(s) for row in scale(h * gamma, j)]
        step = add(eye(n), mul(transfer, rhs_derivative))
        step_weights, step_sigmas = [], []
        for rate in rates:
            scalar = [[F(0) for _ in range(s)] for _ in range(s)]
            for i in range(s):
                scalar[i][i] = 1 + h * gamma * rate
                for k in range(i):
                    scalar[i][k] = h * gamma * rate * a[i][k] - gamma * c[i][k]
            scalar_inverse = inverse(scalar)
            w = mv(transpose(scalar_inverse), beta)
            theta = mv(scalar_inverse, [h * gamma] * s)
            step_weights.append(w)
            step_sigmas.append(1 - rate * dot(beta, theta))
        expected = zeros(n, s * n)
        for i in range(s):
            block = add(scale(step_weights[0][i], q), scale(step_weights[1][i], p))
            for r in range(n):
                expected[r][i * n:(i + 1) * n] = block[r]
        assert transfer == expected
        assert step == add(scale(step_sigmas[0], q), scale(step_sigmas[1], p))
        full_step.append(step)
        full_transfer.append(transfer)
        weights.append(step_weights)
        sigmas.append(step_sigmas)
    propagated = []
    sharp = F(0)
    for k in range(len(hs)):
        after = eye(n)
        pt = pc = F(1)
        for ell in range(k + 1, len(hs)):
            after = mul(full_step[ell], after)
            pt *= sigmas[ell][0]
            pc *= sigmas[ell][1]
        assert after == add(scale(pt, q), scale(pc, p))
        propagated.append(mul(after, full_transfer[k]))
        sharp += sum(nu(n, pt * weights[k][0][i], pc * weights[k][1][i])
                     for i in range(s))
    global_matrix = [sum((block[r] for block in propagated), []) for r in range(n)]
    assert norm_inf(global_matrix) == sharp
    # Each independent residual component chooses the maximizing first-row sign.
    signs = [F(1) if value >= 0 else F(-1) for value in global_matrix[0]]
    assert abs(mv(global_matrix, signs)[0]) == sharp
    return {"id": "G1-G6", "status": "pass", "arithmetic": "exact Fraction",
            "n": n, "stages": s, "steps": len(hs),
            "step_sizes": [str(h) for h in hs],
            "sharp_accumulated_norm": str(sharp),
            "scope": "General causal coefficients; no RODAS5P order assertion."}


def check_nonlinear_majorant():
    a = [[F(2), F(0), F(0)], [F(-3), F(2), F(0)],
         [F(1), F(-4), F(2)]]
    d = [[abs(x) for x in row] for row in inverse(a)]
    # N(U)=(0,u0^2,(u0+2*u1)^2) on [-1,1]^3.
    # Derivative bounds are 2, 6, 12 from that declared box.
    n = [[F(0), F(0), F(0)], [F(2), F(0), F(0)],
         [F(6), F(12), F(0)]]
    b = mul(d, n)
    assert norm_inf(b) > 1
    assert mul(mul(b, b), b) == zeros(3, 3)
    finite_inverse = add(add(eye(3), b), mul(b, b))
    assert finite_inverse == inverse(add(eye(3), scale(F(-1), b)))
    ustar = [F(1, 5), F(2, 5), F(-1, 3)]
    perturbation = [F(1, 100), F(-1, 150), F(1, 200)]
    candidate = vector_add(ustar, perturbation)
    assert all(abs(x) <= 1 for x in ustar + candidate)

    def nonlinear(u):
        return [F(0), u[0] ** 2, (u[0] + 2 * u[1]) ** 2]

    forcing = vector_add(mv(a, ustar), vector_scale(F(-1), nonlinear(ustar)))
    residual = vector_add(vector_add(mv(a, candidate), vector_scale(F(-1), forcing)),
                          vector_scale(F(-1), nonlinear(candidate)))
    error_bound = mv(mul(finite_inverse, d), [abs(x) for x in residual])
    assert all(abs(e) <= bound for e, bound in zip(perturbation, error_bound))
    beta = [F(1, 3), F(-2, 5), F(1, 2)]
    endpoint_error = abs(dot(beta, perturbation))
    endpoint_bound = dot([abs(x) for x in beta], error_bound)
    assert endpoint_error <= endpoint_bound
    return {"id": "G13", "status": "pass", "arithmetic": "exact Fraction",
            "unweighted_majorant_norm": str(norm_inf(b)),
            "endpoint_error": str(endpoint_error),
            "endpoint_bound": str(endpoint_bound),
            "scope": "One manufactured nonlinear causal root with a valid domain bound."}


def check_invariant_drift():
    c = [[F(2), F(0)], [F(0), F(3)]]
    a = [[F(0), F(3)], [F(-2), F(0)]]
    assert add(mul(transpose(a), c), mul(c, a)) == zeros(2, 2)
    y, stage = [F(1), F(2)], [F(1, 3), F(-2, 5)]
    h = F(3, 7)
    eta = [F(1, 7), F(-1, 8)]
    slope = vector_add(mv(a, stage), eta)
    delta = vector_add(vector_add(stage, vector_scale(F(-1), y)),
                       vector_scale(-h / 2, slope))
    z = vector_add(y, vector_scale(h, slope))
    predicted = -2 * h * dot(delta, mv(c, slope)) + 2 * h * dot(stage, mv(c, eta))
    assert quadratic(c, z) - quadratic(c, y) == predicted
    xi = [F(1, 101), F(-2, 103)]
    represented = vector_add(z, xi)
    predicted += 2 * dot(z, mv(c, xi)) + quadratic(c, xi)
    actual = quadratic(c, represented) - quadratic(c, y)
    assert actual == predicted
    left = add(eye(2), scale(-h / 2, a))
    right = add(eye(2), scale(h / 2, a))
    residual = vector_add(mv(left, represented), vector_scale(F(-1), mv(right, y)))
    midpoint = vector_scale(F(1, 2), vector_add(represented, y))
    assert actual == 2 * dot(midpoint, mv(c, residual))
    return {"id": "G20-G22", "status": "pass", "arithmetic": "exact Fraction",
            "invariant_drift": str(actual),
            "scope": "Inexact midpoint, including stage, RHS and endpoint defects."}


def check_verlet():
    j = [[F(0), F(1)], [F(-1), F(0)]]

    def verlet(h):
        return [[1 - h * h / 2, h],
                [-h * (1 - h * h / 4), 1 - h * h / 2]]

    def det(a):
        return a[0][0] * a[1][1] - a[0][1] * a[1][0]

    h1, h2 = F(1), F(19, 10)
    m1, m2 = verlet(h1), verlet(h2)
    for m in (m1, m2):
        assert det(m) == 1
        assert mul(mul(transpose(m), j), m) == j
        assert abs(m[0][0] + m[1][1]) < 2
    product = mul(m2, m1)
    trace = product[0][0] + product[1][1]
    assert trace == F(-9661, 4000)
    assert det(product) == 1 and trace < -2
    boundary = verlet(F(2))
    nilpotent = add(boundary, eye(2))
    assert nilpotent != zeros(2, 2) and mul(nilpotent, nilpotent) == zeros(2, 2)
    epsilon, q, p = F(1, 10), F(1), F(2)
    variable_step_jacobian = [[1 + epsilon * p, epsilon * q], [F(0), F(1)]]
    assert det(variable_step_jacobian) == F(6, 5)
    return {"id": "G17-G18", "status": "pass", "arithmetic": "exact Fraction",
            "omega": "1", "step_sizes": [str(h1), str(h2)],
            "two_step_trace": str(trace), "two_step_determinant": "1",
            "state_dependent_exact_flow_determinant": "6/5",
            "scope": "Specific stability/symplecticity counterexamples, not a universal no-go."}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--preregistration-sha", required=True)
    args = parser.parse_args()
    if len(args.preregistration_sha) != 40 or any(
            c not in "0123456789abcdef" for c in args.preregistration_sha):
        parser.error("Use the full 40-character lowercase preregistration commit.")
    checks = [check_variable_step_transport(), check_nonlinear_majorant(),
              check_invariant_drift(), check_verlet()]
    payload = {
        "schema": "vigilode.geometry.rational_checks.v1",
        "executed_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "baseline_source_commit": "7eb2dd0ffda39afb2871d76700a720986ff09fe0",
        "preregistration_commit": args.preregistration_sha,
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "native_solver_executed": False,
        "performance_claim": False,
        "checks": checks,
        "interpretation": "Exact-rational supporting identities only; no production enclosure or runtime-speed conclusion.",
    }
    print(json.dumps(payload, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
