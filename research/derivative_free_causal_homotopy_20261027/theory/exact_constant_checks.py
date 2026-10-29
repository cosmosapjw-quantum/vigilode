#!/usr/bin/env python3
"""Exact rational audit of the derived cubic constants and budget formulas.

This is an algebra/indexing audit, not a replacement for the differential
and quadrature proofs in THEORY.md. It uses no third-party dependencies,
floating tolerance, numerical derivative, or fitted safety margin.
"""

from fractions import Fraction as F
from hashlib import sha256
from itertools import product
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent


def phi(x, k, g):
    return k * x + g * x**3


def stage_residual(increment, y, a, k, g):
    return increment + 2 * a * phi(y + increment / 2, k, g)


def rational(value):
    return f"{value.numerator}/{value.denominator}"


def main():
    ks = [F(1, 2), F(1), F(3)]
    gs = [F(0), F(1, 4), F(2)]
    aa = [F(0), F(1, 100), F(1, 4), F(1)]
    roots = [F(-2), F(-1, 3), F(0), F(1, 3), F(2)]
    trials = [F(-3), F(-1), F(0), F(1), F(3)]
    secant_cells = 0
    nonexpansive_cells = 0
    invariant_cells = 0
    truncation_cells = 0

    for k, g, a, mstar, m in product(ks, gs, aa, roots, trials):
        # Manufacture the exact input whose midpoint root is mstar.
        y = mstar + a * phi(mstar, k, g)
        exact_increment = 2 * (mstar - y)
        increment = 2 * (m - y)
        fstar = stage_residual(exact_increment, y, a, k, g)
        value = stage_residual(increment, y, a, k, g)
        divisor = 1 + a * (k + g * (m*m + m*mstar + mstar*mstar))
        lower = 1 + a * k
        assert fstar == 0
        assert value - fstar == (increment - exact_increment) * divisor
        assert divisor >= lower > 0
        assert abs(increment - exact_increment) <= abs(value) / lower
        assert abs(mstar) <= abs(y)
        assert (mstar >= 0) == (y >= 0)
        secant_cells += 1

        derivative = k + 3 * g * mstar*mstar
        midpoint_derivative = (1 - a * derivative) / (1 + a * derivative)
        assert abs(midpoint_derivative) <= 1
        # Exact square identity behind nonexpansivity.
        assert (1 + a*derivative)**2 - (1 - a*derivative)**2 == 4*a*derivative
        nonexpansive_cells += 1

    for k, g, a, y in product(ks, gs, aa, roots):
        v = abs(y)
        if v == 0:
            assert phi(y, k, g) == 0
        else:
            image_inside = 2*a*phi(v, k, g) <= 2*v
            exact_criterion = a*(k + g*v*v) <= 1
            assert image_inside == exact_criterion
            if a*(k + 3*g*v*v) < 1:
                assert image_inside
        invariant_cells += 1

    for k, g, y in product(ks, gs, [F(0), F(1, 5), F(1), F(3)]):
        ph = phi(y, k, g)
        ell = k + 3*g*y*y
        second_phi = 6*g*y
        m2 = ell * ph
        m3 = second_phi * ph*ph + ell*ell*ph
        identity = 2*k*k + 6*k*g*y*y + 12*g*g*y**4
        assert 2*ell*ell - second_phi*ph == identity >= 0
        assert m3 <= 3*ell*m2
        assert max(ell*m2/F(8), m3/F(24)) == ell*ell*ph/F(8)
        # The derivative extrema on [-Y,Y] occur at 0 and the endpoints.
        for sample in [F(-1), F(-1, 2), F(0), F(1, 2), F(1)]:
            derivative = k + 3*g*(sample*y)**2
            assert k <= derivative <= ell
        truncation_cells += 1

    planner_cases = [
        (F(1), F(0), F(1), F(1), F(1, 100)),
        (F(1), F(0), F(1), F(1), F(1, 10000)),
        (F(1, 2), F(1, 4), F(1), F(2), F(1, 1000)),
        (F(3), F(2), F(1, 3), F(1, 2), F(1, 10000)),
        (F(20), F(1), F(1), F(1, 8), F(1, 1000)),
        (F(1), F(2), F(0), F(1), F(1, 100)),
    ]
    planner_results = []
    for k, g, y, horizon, epsilon in planner_cases:
        ph = phi(y, k, g)
        ell = k + 3*g*y*y
        c = ell*ell*ph/F(8)
        enlarged_l = k + 3*g*(y + epsilon)**2

        def constants(n):
            h = horizon / n
            lower = 1 + h*k/2
            local = c*h**3/lower
            total = n*local
            assert total == c*horizon**3 / (n*(n + k*horizon/2))
            q = h*enlarged_l/2
            return h, lower, local, total, q

        n = 1
        rejected = []
        while True:
            h, lower, local, total, q = constants(n)
            if total < epsilon and q < 1:
                break
            rejected.append(n)
            n *= 2
            assert n <= 2**30, "audit fixture unexpectedly requires a huge grid"
        for earlier in rejected:
            _, _, _, total_earlier, q_earlier = constants(earlier)
            assert total_earlier >= epsilon or q_earlier >= 1
        algebraic = (epsilon - total)/n
        residual = lower*algebraic
        assert h*n == horizon
        assert n*local + n*residual/lower == epsilon
        assert algebraic > 0 and residual > 0 and q < 1
        # All preceding grid bounds fit the enlarged iterative domain.
        for j in range(n + 1):
            prefix_error = j*(local + algebraic)
            assert prefix_error <= epsilon
            assert y + prefix_error <= y + epsilon
        planner_results.append({
            "k": rational(k), "g": rational(g), "Y": rational(y),
            "T": rational(horizon), "epsilon": rational(epsilon),
            "steps": n, "h": rational(h),
            "C": rational(c), "q": rational(q),
            "truncation_bound": rational(total),
            "algebraic_step_budget": rational(algebraic),
            "stage_residual_target": rational(residual),
            "exact_total_budget_identity": True,
            "smallest_admitted_dyadic_grid": True,
        })

    # The scalar coefficient quotient is invariant under a common residual
    # scale. Degenerate denominators are exactly zero, without an epsilon.
    coefficient_cells = 0
    for current, previous in product(
        [F(-3), F(-1, 10), F(0), F(2, 3), F(5)], repeat=2
    ):
        scale = max(abs(current), abs(previous))
        dr = current - previous
        denominator = dr*dr
        if scale == 0 or denominator == 0:
            assert current == previous
        else:
            exact = current*dr/denominator
            scaled_current, scaled_previous = current/scale, previous/scale
            scaled_dr = scaled_current - scaled_previous
            scaled = scaled_current*scaled_dr/(scaled_dr*scaled_dr)
            assert scaled == exact
        coefficient_cells += 1

    result = {
        "status": "PASS",
        "arithmetic": "Python fractions.Fraction; exact rational identities only",
        "scope": "Algebra and finite formula audit; general analytic bounds proved in THEORY.md section 11",
        "script_sha256": sha256(Path(__file__).read_bytes()).hexdigest(),
        "secant_and_global_residual_cells": secant_cells,
        "nonexpansive_derivative_cells": nonexpansive_cells,
        "invariant_bracket_cells": invariant_cells,
        "truncation_polynomial_cells": truncation_cells,
        "scale_invariant_anderson_cells": coefficient_cells,
        "planner_cells": planner_results,
        "structural_exact_audit": "Independent parent audit: evidence/EXACT_STRUCTURAL_CONSTANTS.json",
    }
    (HERE / "EXACT_CONSTANT_RESULTS.json").write_text(
        json.dumps(result, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps({
        "status": result["status"],
        "secant_cells": secant_cells,
        "nonexpansive_cells": nonexpansive_cells,
        "invariant_cells": invariant_cells,
        "truncation_cells": truncation_cells,
        "anderson_cells": coefficient_cells,
        "planner_cells": len(planner_results),
        "planner_steps": [cell["steps"] for cell in planner_results],
    }))


if __name__ == "__main__":
    main()
