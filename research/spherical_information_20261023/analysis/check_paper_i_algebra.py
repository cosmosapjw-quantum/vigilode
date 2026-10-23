"""Standard-library exact-rational checks for Paper-I-derived formulas.

These checks corroborate the displayed elementary algebra, not a general
formal proof checker or an ODE/PDE validation.
"""
from fractions import Fraction as Q
import json
from pathlib import Path
import platform


def choose(a, n):
    out = Q(1)
    for j in range(n):
        out *= (a-j)/(j+1)
    return out


def main():
    coeff = {str(j): -choose(Q(3, 4), j) for j in range(2, 7)}
    checks = {
        "r2": coeff["2"] == Q(3, 32),
        "r3": coeff["3"] == -Q(5, 128),
        "r4": coeff["4"] == Q(45, 2048),
        "r5": coeff["5"] == -Q(117, 8192),
        "joint_hessian_determinant_numerator": 4*3-3*3 == 3,
        "joint_metric_dn2_coefficient": 4-6+3 == 1,
        "joint_metric_cross_coefficient": -6+6 == 0,
        "scalar_metric_power": 2*(Q(3, 8)-1) == -Q(5, 4),
        "scalar_metric_coefficient_over_kappa":
            (Q(2, 3)*Q(3, 8))**2 * 3 == Q(3, 16),
    }
    out = {
        "schema": "vigilode.spherical-information.paper-i-algebra.v1",
        "tool": "Python standard-library fractions",
        "python": platform.python_version(),
        "command": "python3 analysis/check_paper_i_algebra.py",
        "stable_bregman_coefficients": {j: str(c) for j, c in coeff.items()},
        "checks": checks,
        "status": "pass_scoped" if all(checks.values()) else "fail",
        "scope": "Exact rational coefficients in the displayed T6-T8 derivation; not a solver validation or formal proof checker.",
        "environment_failure": {
            "attempt": "import sympy for equivalent algebra verification",
            "observed": "ModuleNotFoundError: No module named 'sympy'",
            "class": "environment",
            "resolution": "No installation; use exact fractions and preserve explicit analytic derivation."
        },
    }
    target = Path(__file__).with_name("PAPER_I_SYMBOLIC_CHECK.json")
    target.write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))
    assert all(checks.values())


if __name__ == "__main__":
    main()
