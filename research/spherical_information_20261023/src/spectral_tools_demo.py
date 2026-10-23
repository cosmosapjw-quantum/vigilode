"""Small finite-polynomial/Laguerre checks; no PDE or production speed claim."""
from __future__ import annotations

import hashlib
import json
import math
import platform
import time
from pathlib import Path

import numpy as np
import scipy
from scipy.special import eval_genlaguerre, gammaln, roots_genlaguerre


def poisson_cutoff(v: float, coefficient_l2: float, tolerance: float, lmax: int) -> int:
    if not 0 <= v <= 1 or coefficient_l2 < 0 or tolerance <= 0 or lmax < 0:
        raise ValueError("invalid cutoff contract")
    if v == 0 or coefficient_l2 <= tolerance:
        return 0
    if v == 1:
        return lmax  # No rank suppression for a massless shell.
    # v**(L+1) * K <= tolerance. Full known finite field is always exact.
    return min(lmax, max(0, math.ceil(math.log(coefficient_l2 / tolerance) / -math.log(v)) - 1))


def poisson_linf_tail_bound(v: float, cutoff: int, tail_l2: float) -> float:
    if tail_l2 == 0 or v == 0:
        return 0.0
    if not 0 < v < 1:
        raise ValueError("infinite-series Linfinity bound needs 0<v<1 or empty tail")
    s, n = v * v, cutoff + 1
    series = s**n * ((2 * n + 1) / (1 - s) + 2 * s / (1 - s)**2)
    return tail_l2 * math.sqrt(series)


def run() -> dict:
    started = time.perf_counter()
    # Weight x^2 exp(-x); order16 integrates polynomial products through degree31.
    nodes, weights = roots_genlaguerre(16, 2.0)
    polynomials = np.stack([
        (-1)**n * np.exp(0.5 * (gammaln(n + 1) - gammaln(n + 3))) * eval_genlaguerre(n, 2.0, nodes)
        for n in range(8)
    ], axis=1)
    gram = polynomials.T @ (weights[:, None] * polynomials)
    laguerre = {
        "count": 8, "quadrature_order": 16, "alpha": 2,
        "weight": "x^2 exp(-x) dx on [0,infinity)",
        "quadrature_weights_positive": bool(np.all(weights > 0)),
        "weight_total_error": abs(float(np.sum(weights)) - 2),
        "gram_max_absolute_error": float(np.max(np.abs(gram - np.eye(8)))),
        "p0_max_error": float(np.max(np.abs(polynomials[:, 0] - 1 / np.sqrt(2)))),
        "p1_max_error": float(np.max(np.abs(polynomials[:, 1] - (nodes - 3) / np.sqrt(6)))),
    }
    assert laguerre["gram_max_absolute_error"] < 2e-12
    assert laguerre["p0_max_error"] < 2e-14 and laguerre["p1_max_error"] < 2e-13

    lmax = 24
    ranks = np.concatenate([np.full(2 * ell + 1, ell) for ell in range(lmax + 1)])
    rng = np.random.default_rng(281023)
    coefficients = rng.normal(size=len(ranks)) / (1 + ranks)**1.4
    coefficients[0] = 0  # Nontrivial field; thermal positive baseline is separate.
    coefficients /= np.linalg.norm(coefficients)
    K = float(np.linalg.norm(coefficients))
    tolerance = 0.02
    # Addition theorem gives |A| <= sum_l sqrt(2l+1)*||a_l||_2 everywhere.
    uniform_majorant = sum(np.sqrt(2 * ell + 1) * np.linalg.norm(coefficients[ranks == ell]) for ell in range(lmax + 1))
    amplitude = 0.25 / uniform_majorant
    # B=1+amplitude*A, hence exact full B and every Poisson average are >=0.75.
    full_positive_floor = 0.75
    cases = []
    for v in (0.0, 0.3, 0.7, 0.9, 0.95, 1.0):
        cutoff = poisson_cutoff(v, K, tolerance, lmax)
        omitted = ranks > cutoff
        actual_l2 = float(np.linalg.norm(coefficients[omitted] * v**ranks[omitted]))
        l2_bound = 0.0 if cutoff == lmax else v**(cutoff + 1) * K
        tail_norm = float(np.linalg.norm(coefficients[omitted]))
        linf_bound = amplitude * poisson_linf_tail_bound(v, cutoff, tail_norm)
        # Independently bound this *finite known* omitted polynomial by rank sums.
        finite_linf_bound = amplitude * sum(
            v**ell * np.sqrt(2 * ell + 1) * np.linalg.norm(coefficients[ranks == ell])
            for ell in range(cutoff + 1, lmax + 1)
        )
        positive_guard = full_positive_floor - linf_bound
        cases.append({
            "v": v, "lcut": cutoff, "retained_basis_count": (cutoff + 1)**2,
            "full_basis_count": (lmax + 1)**2, "exact_coefficient_l2_error": actual_l2,
            "sufficient_l2_bound": l2_bound,
            "requested_l2_tolerance": tolerance,
            "thermal_field_linf_error_bound": linf_bound,
            "finite_rank_triangle_linf_bound": finite_linf_bound,
            "certified_thermal_positive_lower_bound": positive_guard,
            "l2_bound_pass": bool(actual_l2 <= l2_bound + 3e-15),
            "tolerance_pass": bool(actual_l2 <= tolerance + 3e-15),
            "positive_guard_pass": bool(positive_guard > 0),
            "massless_rank_suppression_used": False if v == 1 else None,
        })
    assert all(c["l2_bound_pass"] and c["tolerance_pass"] and c["positive_guard_pass"] for c in cases)
    assert cases[0]["lcut"] == 0 and cases[-1]["lcut"] == lmax
    try:
        poisson_cutoff(1.1, K, tolerance, lmax)
    except ValueError:
        invalid_domain_rejected = True
    else:
        invalid_domain_rejected = False
    assert invalid_domain_rejected
    total_basis = sum(c["retained_basis_count"] for c in cases)
    full_basis = len(cases) * (lmax + 1)**2
    return {
        "schema": "spherical-information.spectral-tools-verification.v1",
        "status": "PASS_SCOPED", "scope": "Known finite field through L=24 and MB reference tangent polynomials; no PDE benchmark",
        "command": "OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 src/spectral_tools_demo.py",
        "elapsed_seconds": time.perf_counter() - started,
        "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "environment": {"python": platform.python_version(), "numpy": np.__version__, "scipy": scipy.__version__},
        "laguerre": laguerre,
        "finite_field": {"lmax": lmax, "seed": 281023, "coefficients": coefficients.tolist(),
                         "coefficient_l2": K, "field_uniform_majorant": float(uniform_majorant),
                         "thermal_amplitude": float(amplitude), "thermal_field": "B=1+amplitude*A",
                         "global_thermal_positive_floor": full_positive_floor},
        "shell_cases": cases,
        "total_retained_basis": total_basis, "total_full_basis": full_basis,
        "retained_basis_fraction": total_basis / full_basis,
        "evaluation_count_reduction_factor": full_basis / total_basis,
        "invalid_domain_rejected": invalid_domain_rejected,
        "limitations": ["Known full finite coefficient tail; no unknown continuum tail inferred.",
                        "Operation count is basis terms evaluated, not solver wall time.",
                        "v=1 has no automatic rank suppression and retains all known ranks.",
                        "Floating-point checks use stated margins, not outward-rounded interval arithmetic."]
    }


if __name__ == "__main__":
    result = run()
    out = Path(__file__).resolve().parents[1] / "results" / "spectral_tools_results.json"
    out.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"status": result["status"], "gram_error": result["laguerre"]["gram_max_absolute_error"],
                      "basis_counts": [c["retained_basis_count"] for c in result["shell_cases"]],
                      "basis_fraction": result["retained_basis_fraction"]}))
