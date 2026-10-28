#!/usr/bin/env python3
"""Exact rational supporting checks for the homotopy theory supplement.

This program does not call VigilODE, compile Rust, benchmark a solver, or
certify floating-point interval code. Run only from the committed source
snapshot named by the root campaign ledger. It writes JSON to stdout.

All PASS/FAIL comparisons below use fractions.Fraction. Decimal values are
display diagnostics only. Proofs and full assumptions are in THEORY.md.
"""
from decimal import Decimal, localcontext
from fractions import Fraction as Q
import json


def decimal_diagnostic(value):
    with localcontext() as context:
        context.prec = 35
        return str(Decimal(value.numerator) / Decimal(value.denominator))


def phi(x, k, g):
    return k * x + g * x**3


def ell_bound(x, k, g):
    return k + 3 * g * x**2


def geometric_dyadic(q, n):
    assert n >= 1 and n & (n - 1) == 0
    p, total, length = q, Q(1), 1
    while length < n:
        total = total * (1 + p)
        p = p * p
        length *= 2
    if q == 1:
        assert total == n
    else:
        # Independent closed-form identity; exact rationals avoid cancellation.
        assert total == (1 - q**n) / (1 - q)
    return total


def exact_grid_bounds(n, k=Q(1), g=Q(1, 2), initial=Q(1), horizon=Q(1)):
    h = horizon / n
    denominator = 1 + h * k / 2
    l = ell_bound(initial, k, g)
    common = l**2 * phi(initial, k, g)
    original = n * (common / 8) * h**3 / denominator
    constant12 = n * (common / 12) * h**3 / denominator
    q = 1 / (1 + k * h)
    coefficients = (
        (1, k**3 * initial),
        (3, 7 * k**2 * g * initial**3),
        (5, 15 * k * g**2 * initial**5),
        (7, 9 * g**3 * initial**7),
    )
    weighted = sum(
        (coefficient * geometric_dyadic(q**power, n)
         for power, coefficient in coefficients),
        Q(0),
    )
    decay12 = h**3 * weighted / (12 * denominator)
    return {
        "inherited_1over8": original,
        "constant_1over12": constant12,
        "rational_decay_1over12": decay12,
    }


def check_cubic_identities():
    checks = 0
    for k in (Q(1, 4), Q(1), Q(4)):
        for g in (Q(0), Q(1, 2), Q(3)):
            for u in (Q(0), Q(1, 10), Q(1), Q(3)):
                p = phi(u, k, g)
                l = ell_bound(u, k, g)
                expanded = (
                    k**3 * u
                    + 7 * k**2 * g * u**3
                    + 15 * k * g**2 * u**5
                    + 9 * g**3 * u**7
                )
                assert l**2 * p == expanded
                dominance = l**2 - 3 * g * u * p
                assert dominance == k**2 + 3 * k * g * u**2 + 6 * g**2 * u**4
                assert dominance >= 0
                for fraction in (Q(0), Q(1, 3), Q(1)):
                    v = fraction * u
                    h = Q(7, 11)
                    m = (u + v) / 2
                    # Exact integral of phi(linear chord) minus midpoint value.
                    chord_integral = h * (
                        k * (u + v) / 2
                        + g * (u**3 + u**2 * v + u * v**2 + v**3) / 4
                    )
                    b = chord_integral - h * phi(m, k, g)
                    assert b == g * h * m * (u - v)**2 / 4
                    assert b >= 0
                    checks += 1
    # Exact polynomial antiderivative of s(h-s)/2 on [0,h].
    h = Q(7, 11)
    kernel_integral = h * h**2 / 4 - h**3 / 6
    assert kernel_integral == h**3 / 12
    return {"fixture_count": checks, "status": "PASS"}


def check_grids():
    epsilon, k, g, initial = Q(1, 100000), Q(1), Q(1, 2), Q(1)
    names = ("inherited_1over8", "constant_1over12", "rational_decay_1over12")
    first = {name: None for name in names}
    rows, n = [], 1
    while n <= 512:
        h = Q(1, n)
        contraction = h * (k + 3 * g * (initial + epsilon)**2) / 2
        values = exact_grid_bounds(n)
        for name in names:
            if first[name] is None and contraction < 1 and values[name] < epsilon:
                first[name] = n
        if n in (64, 128, 256, 512):
            rows.append({
                "n": n,
                "q_B_lt_one_exact": contraction < 1,
                "bounds": {
                    name: {
                        "decimal_diagnostic": decimal_diagnostic(value),
                        "strictly_below_epsilon_exact": value < epsilon,
                    }
                    for name, value in values.items()
                },
            })
        # Refining the rational envelope makes its even terms no larger.
        coarse_q = 1 / (1 + k * h)
        fine_q = 1 / (1 + k * h / 2)
        assert fine_q**2 <= coarse_q
        n *= 2
    assert first == {
        "inherited_1over8": 512,
        "constant_1over12": 512,
        "rational_decay_1over12": 256,
    }
    assert exact_grid_bounds(256)["rational_decay_1over12"] < epsilon
    assert exact_grid_bounds(128)["rational_decay_1over12"] >= epsilon
    return {
        "status": "PASS",
        "fixture": {"k": "1", "g": "1/2", "Y": "1", "T": "1", "epsilon": "1/100000"},
        "minimum_admitted_dyadic_n": first,
        "rows": rows,
        "interpretation": "mathematical planning bound; not measured runtime",
    }


def check_station_certificate():
    # G(x)=x/2+1, anchor=0, lambda=1/2, exact center=2/3.
    q, lam, center, radius = Q(1, 2), Q(1, 2), Q(2, 3), Q(1)
    mapped_center = q * center + 1
    e = abs(lam * mapped_center - center)
    b = abs(mapped_center)
    assert e == 0
    delta = ((1 - lam * q) * radius - e) / (q * radius + b)
    mu = lam + delta
    assert delta == Q(9, 22)
    assert mu == Q(10, 11) and mu * q < 1
    assert mu * q * radius + e + delta * b == radius
    low, high = center - radius, center + radius
    image_low = mu * (q * low + 1)
    image_high = mu * (q * high + 1)
    assert low <= image_low <= image_high <= high
    root = mu / (1 - mu * q)
    assert abs(root - center) == radius
    # Slightly exceeding the derived margin breaks this ball's invariance.
    too_far = mu + Q(1, 100)
    assert too_far < 1
    assert too_far * (q * high + 1) > high
    return {
        "status": "PASS",
        "delta_max": "9/22",
        "next_lambda": "10/11",
        "negative_control": "larger station leaves the declared ball",
    }


def check_branch_counterexample():
    parameter = Q(1)
    positive, negative = Q(1), Q(-1)
    assert positive**2 - parameter == negative**2 - parameter == 0
    low, high = Q(1, 2), Q(3, 2)
    assert low <= positive <= high
    assert not low <= negative <= high
    return {
        "status": "PASS",
        "counterexample": "x^2-1 has zero residual on both branches",
        "positive_branch_bracket": ["1/2", "3/2"],
    }


def check_residence():
    gap, variation, overhead = Q(10), Q(2), Q(10)

    def gain(m):
        return m * gap - variation * m * (m + 1) / 2

    assert gain(1) <= overhead
    assert gain(2) > overhead and 2 * variation < gap
    assert 5 * variation == gap  # strict-cheaper certificate expires.
    increments = [gap - j * variation for j in (1, 2)]
    assert sum(increments, Q(0)) == gain(2) == 14
    # Continuous counterpart: d=4,L=2,H=1,S=1.
    d, l, horizon, cost = Q(4), Q(2), Q(1), Q(1)
    saving = d * horizon - l * horizon**2 / 2
    assert saving == 3 and saving > cost and horizon <= d / l
    return {
        "status": "PASS",
        "discrete_fixture": {
            "d": "10", "V": "2", "S": "10", "paying_m": 2,
            "guaranteed_saving": "14", "strict_certificate_expires_at_m": 5,
        },
        "scope": "requires a certified future cost-variation contract",
    }


def main():
    results = {
        "schema": "vigilode.polyalgorithm.homotopy-check.v1",
        "arithmetic": "fractions.Fraction exact; displayed Decimal values diagnostic",
        "cubic_identities": check_cubic_identities(),
        "grid_bound": check_grids(),
        "continuation_ball": check_station_certificate(),
        "branch_negative_control": check_branch_counterexample(),
        "conditional_residence": check_residence(),
        "native_solver_tests": "NOT_RUN",
        "native_benchmarks": "NOT_RUN",
        "production_interval_validation": "NOT_RUN",
        "session_transfer_validation": "NOT_RUN",
        "status": "PASS",
    }
    print(json.dumps(results, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

