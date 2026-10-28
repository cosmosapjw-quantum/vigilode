#!/usr/bin/env python3
"""Exact illustrative mathematics checks; no solver or historical campaign run.

The generic theorems are proved in the accompanying text. Finite examples here
are diagnostics, not proofs of their universal quantifiers or native speed.
"""
from fractions import Fraction as F
from pathlib import Path
import json


def add(a, b):
    return [[x + y for x, y in zip(u, v)] for u, v in zip(a, b)]


def scale(a, c):
    return [[c * x for x in r] for r in a]


def mul(a, b):
    return [[sum((a[i][k] * b[k][j] for k in range(len(b))), F(0))
             for j in range(len(b[0]))] for i in range(len(a))]


def ident(n):
    return [[F(i == j) for j in range(n)] for i in range(n)]


def norminf(a):
    return max(sum(abs(x) for x in r) for r in a)


def minimum_steps(weight, budget):
    k = 0
    while weight * F(1, 2) ** k > budget:
        k += 1
    return k


def run():
    checks = []
    identity = ident(2)
    zero = scale(identity, F(0))
    p = [[F(4, 5), F(0)], [F(0), F(2, 3)]]
    for s in [F(0), F(1, 7), F(1), F(2)]:
        b = [[F(5, 4), -s / 4], [F(0), F(3, 2)]]
        e = add(identity, scale(mul(p, b), -1))
        assert e == [[F(0), s / 5], [F(0), F(0)]]
        assert mul(e, e) == zero
        inverse = mul(add(identity, e), p)
        assert mul(inverse, b) == identity
        assert norminf(inverse) == F(4, 5) + 2 * s / 15
        assert norminf(inverse) <= F(4, 3)
        checks.append({"id": f"atlas_s_{s}", "status": "PASS",
                       "exact_inverse_norm": str(norminf(inverse))})

    # Degree-64 Laguerre recurrence modulo q(x)=x^2-1, tested at q(X)=0.
    x = [[F(0), F(1)], [F(1), F(0)]]
    l0 = identity
    l1 = add(identity, scale(x, -1))
    r0, r1 = [F(1), F(0)], [F(1), F(-1)]
    for j in range(1, 64):
        l2 = scale(add(add(scale(l1, 2 * j + 1), scale(mul(x, l1), -1)),
                       scale(l0, -j)), F(1, j + 1))
        # x*(a+b*x) == b+a*x modulo x^2-1.
        r2 = [((2 * j + 1) * r1[i] - r1[1 - i] - j * r0[i]) / (j + 1)
              for i in range(2)]
        l0, l1 = l1, l2
        r0, r1 = r1, r2
    reduced = add(scale(identity, r1[0]), scale(x, r1[1]))
    assert reduced == l1
    checks.append({"id": "laguerre64_quotient_identity", "status": "PASS",
                   "original_operator_applications_per_rhs": 64,
                   "reduced_operator_applications_per_rhs": 1,
                   "scope": "exact arithmetic; coefficient compilation and witness cost excluded from these counts but required by total-cost gate",
                   "remainder_coefficients_exact": list(map(str, r1))})

    weights = [F(4096), F(1), F(1), F(1)]
    budget = F(1, 16)
    uniform_k = minimum_steps(sum(weights), budget)
    ks = [minimum_steps(4 * a, budget) for a in weights]
    weighted_error = sum((a * F(1, 2) ** k for a, k in zip(weights, ks)), F(0))
    assert uniform_k == 17 and ks == [18, 6, 6, 6]
    assert weighted_error == budget
    checks.append({"id": "influence_budget_discrete_work", "status": "PASS",
                   "uniform_iterations": 4 * uniform_k,
                   "weighted_iterations": sum(ks), "per_target_iterations": ks,
                   "error_bound": str(weighted_error), "budget": str(budget),
                   "scope": "fixed certified weights and q=1/2, equal unit correction costs; not a VigilODE trajectory"})

    # An eight-stage scalar bidiagonal causal map with Lipschitz 1/4.
    s = 8
    btri = [[F(1, 4) if i == j + 1 else F(0) for j in range(s)] for i in range(s)]
    power = ident(s)
    for _ in range(s):
        power = mul(power, btri)
    assert power == [[F(0)] * s for _ in range(s)]
    e0 = [[F(1, 4096)] for _ in range(s)]
    e2 = mul(mul(btri, btri), e0)
    assert norminf(e2) == F(1, 65536)
    checks.append({"id": "causal_two_round_error", "status": "PASS",
                   "stages": s, "nilpotence_index_upper": s,
                   "predictor_error": "1/4096", "rounds": 2,
                   "remaining_error_upper": str(norminf(e2)),
                   "scope": "same target; predictor construction, roundoff and cost are separate obligations"})

    compile_cost, full, member, n = 10000, 1000, 10, 100
    old, new = n * full, compile_cost + n * member
    assert old == 100000 and new == 11000 and new < old
    assert compile_cost + 10 * member >= 10 * full
    assert compile_cost + 11 * member < 11 * full
    checks.append({"id": "cell_amortization", "status": "PASS",
                   "first_profitable_reuse_count": 11,
                   "old_gain_verification_work": old, "new_gain_verification_work": new,
                   "scope": "explicit arithmetic cost model; common solve/residual work cancels; no wall-time observation"})

    # Rational exact inverse polynomial on a Jordan block, arbitrary nonnormality.
    a, kappa, large = F(1, 4), F(2), F(1024)
    nilp = [[F(0), large], [F(0), F(0)]]
    aa = add(scale(identity, -kappa), nilp)
    ww = add(identity, scale(aa, -a))
    rr = scale(add(identity, scale(nilp, a / (1 + a * kappa))), 1 / (1 + a * kappa))
    assert mul(rr, ww) == identity
    checks.append({"id": "nonnormal_nilpotent_inverse", "status": "PASS",
                   "nilpotent_norm": str(norminf(nilp)),
                   "inverse_norm": str(norminf(rr)),
                   "scope": "annihilator, not spectrum-only or norm(N)<1, controls exact termination"})

    return {"schema_version": "1.0", "status": "PASS", "arithmetic": "fractions.Fraction",
            "test_type": "new_theory_illustrative_exact_checks",
            "historical_campaigns_rerun": 0, "native_production_execution": False,
            "generic_theorems_proved_by_this_script": False, "checks": checks}


if __name__ == "__main__":
    result = run()
    path = Path(__file__).with_name("EXACT_RESULTS.json")
    path.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"status": result["status"], "checks": len(result["checks"]), "output": str(path)}))
