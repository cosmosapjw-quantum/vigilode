#!/usr/bin/env python3
"""New, bounded theory illustrations; no historical/native campaign rerun."""
from fractions import Fraction as F
from math import factorial
from pathlib import Path
import json


def phi_interval(z, k, even=60):
    assert z <= 0 and even % 2 == 0
    # At these inputs |z|<=2. Tail magnitudes decrease after even>=2.
    upper = sum((z**j / factorial(j + k) for j in range(even + 1)), F(0))
    lower = upper + z**(even + 1) / factorial(even + 1 + k)
    assert lower <= upper
    return lower, upper


def max_abs_interval(lo, hi):
    return max(abs(lo), abs(hi))


checks = []
# P1: X has eigenvalues -1, 15/16, A=-I+X, h=1; q=x^2-1.
x = F(15, 16)
delta = F(31, 256)
weight_left, weight_right = (1 - x) / 2, (1 + x) / 2
for k in range(5):
    node_lo, node_hi = phi_interval(F(-2), k)
    target_lo, target_hi = phi_interval(x - 1, k)
    r_lo = weight_left * node_lo + weight_right / factorial(k)
    r_hi = weight_left * node_hi + weight_right / factorial(k)
    rigorous_actual_upper = max_abs_interval(target_lo - r_hi, target_hi - r_lo)
    theorem_upper = delta / factorial(2 + k)
    assert rigorous_actual_upper <= theorem_upper
    checks.append({
        "id": f"P1_near_annihilator_phi{k}", "status": "PASS",
        "delta_exact": str(delta), "theorem_upper_exact": str(theorem_upper),
        "actual_error_upper_display_only": float(rigorous_actual_upper),
        "comparison_used_exact_rationals": True,
    })

# Exact four-dimensional projection action, with scalar exp(-4) interval.
e1_lo, e1_hi = phi_interval(F(-1), 0, even=30)
e4_lo, e4_hi = e1_lo**4, e1_hi**4
assert 0 < e4_lo < e4_hi
v = [F(1), F(2), F(3), F(4)]
mean = sum(v) / len(v)
pv = [t - mean for t in v]
assert sum(pv) == 0
assert [t - sum(pv) / len(pv) for t in pv] == pv
e4_hat = float((e4_lo + e4_hi) / 2)
e4_hat_exact = F.from_float(e4_hat)
coef_error = max_abs_interval(e4_hat_exact - e4_hi, e4_hat_exact - e4_lo)
candidate = [float(mean) + e4_hat * float(t) for t in pv]
# IEEE callback in this exact fixture has exact mean and Pv; explicitly
# compute assembly error against the represented coefficient polynomial.
local = [abs(F.from_float(y) - (mean + e4_hat_exact * t)) for y, t in zip(candidate, pv)]
upper_inf = max(abs(t) * coef_error + eta for t, eta in zip(pv, local))
for y, t in zip(candidate, pv):
    target = sorted([mean + t * e4_lo, mean + t * e4_hi])
    assert max_abs_interval(F.from_float(y) - target[1], F.from_float(y) - target[0]) <= upper_inf
checks.append({
    "id": "P1_projector_binary64_candidate", "status": "PASS",
    "target": "exp(-4P)*(1,2,3,4), P=I-11^T/4",
    "candidate_hex": [y.hex() for y in candidate],
    "infinity_error_upper_exact": str(upper_inf),
    "infinity_error_upper_display_only": float(upper_inf),
    "operator_products": 1,
    "scope": "exact structural fixture; scalar coefficient and output assembly errors explicitly enclosed; no native implementation",
})

out = {
    "schema_version": "1.0", "status": "PASS", "arithmetic": "fractions.Fraction",
    "test_type": "new_theory_illustrative_exact_checks",
    "historical_campaigns_rerun": 0, "native_production_execution": False,
    "generic_theorems_proved_by_this_script": False,
    "checks": checks,
}
Path(__file__).with_suffix(".json").write_text(json.dumps(out, indent=2) + "\n")
print(json.dumps({"status": out["status"], "checks": len(checks)}))
