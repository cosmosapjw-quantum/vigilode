#!/usr/bin/env python3
"""Exact arithmetic checks for the continuation/cost note; not solver timings."""
from fractions import Fraction as F
from pathlib import Path
import json


def mm(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(2)) for j in range(2)] for i in range(2)]


def add(a, b):
    return [[a[i][j] + b[i][j] for j in range(2)] for i in range(2)]


def sub(a, b):
    return [[a[i][j] - b[i][j] for j in range(2)] for i in range(2)]


def inv(a):
    d = a[0][0] * a[1][1] - a[0][1] * a[1][0]
    return [[a[1][1] / d, -a[0][1] / d], [-a[1][0] / d, a[0][0] / d]]


def norm(a):
    return max(sum(abs(v) for v in row) for row in a)


def growth_count(length, h0, g):
    n, covered, h = 0, F(0), h0
    while covered < length:
        covered += h
        h *= g
        n += 1
    exact_sum = h0 * (g**n - 1) / (g - 1)
    previous = h0 * (g**(n - 1) - 1) / (g - 1)
    assert covered == exact_sum
    assert previous < length <= exact_sum
    return n


def main():
    rows = []
    for windows in (1, 4, 16, 64):
        cold = windows * growth_count(F(1, 100), F(1, 10**6), F(5))
        free = growth_count(F(windows, 100), F(1, 10**6), F(5))
        rows.append({"windows": windows, "cold_restarts": cold, "no_interior_barriers": free})
    assert rows[-1]["cold_restarts"] == 448
    assert rows[-1]["no_interior_barriers"] == 10
    geometric_cases = 0
    for g in (F(3, 2), F(2), F(5)):
        for n in range(1, 25):
            h0 = F(1, 10**6)
            endpoint = h0 * (g**n - 1) / (g - 1)
            assert growth_count(endpoint, h0, g) == n
            assert growth_count(endpoint + h0 / 3, h0, g) == n + 1
            geometric_cases += 2

    identity = [[F(1), F(0)], [F(0), F(1)]]
    w = [[F(2), F(1, 4)], [F(0), F(3)]]
    wp = [[F(21, 10), F(1, 5)], [F(1, 20), F(29, 10)]]
    wi = inv(w)
    q = mm(wi, sub(w, wp))
    assert mm(w, sub(identity, q)) == wp
    assert mm(w, q) != mm(q, w), "fixture must not silently assume commutation"
    eta = norm(q)
    assert eta < 1
    series, power = [[F(0), F(0)], [F(0), F(0)]], identity
    inverse_rows = []
    for m in range(7):
        series = add(series, power)
        approx = mm(series, wi)
        actual = norm(sub(inv(wp), approx))
        bound = eta ** (m + 1) * norm(wi) / (1 - eta)
        assert actual <= bound
        inverse_rows.append({"degree": m, "error_inf_exact": str(actual), "bound_exact": str(bound)})
        power = mm(power, q)

    # An exact rational instance of kappa^(3/10), avoiding floating roots.
    kappa, ratio = F(1, 1024), F(1, 8)
    assert ratio**10 == kappa**3

    overshoot_cases = 0
    for cap in range(1, 21):
        for weights in ([1] * 30, [3, 1, 4, 1, 5, 9] * 3, [10, 10, 10]):
            charged = 0
            for weight in weights:
                charged += weight
                if charged > cap:
                    assert charged <= cap + max(weights)
                    break
            overshoot_cases += 1

    result = {
        "schema_version": 1,
        "kind": "exact_arithmetic_theory_checks_not_solver_benchmarks",
        "status": "PASS",
        "geometric_minimal_count_cases": geometric_cases,
        "restart_count_examples": rows,
        "noncommuting_reuse_fixture": {"eta_inf_exact": str(eta), "bounds": inverse_rows},
        "pi_clipping_rational_fixture": {"kappa": str(kappa), "ratio": str(ratio)},
        "post_work_overshoot_cases": overshoot_cases,
        "limitations": [
            "These exact checks instantiate algebraic proofs, not floating-point solver performance.",
            "No-interior-barriers counts do not describe the current exact-landing RODAS path.",
            "Cost ceilings count declared work units, not callback wall time.",
        ],
    }
    target = Path(__file__).with_name("THEORY_CHECKS.json")
    target.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"status": result["status"], "output": str(target), "geometric_cases": geometric_cases}))


if __name__ == "__main__":
    main()
