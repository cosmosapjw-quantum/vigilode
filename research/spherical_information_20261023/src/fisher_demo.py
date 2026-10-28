"""Finite-grid Fisher whitening for entropy moment inversion, not an ODE solver.

Dimensionless x=E/E_* and z=mc^2/E_*; angular harmonics have integral Y^2 dOmega=1.
The omitted common positive dimensional measure factor cancels from conditioning.
Run with BLAS threads=1 to make these small timing probes reproducible.
"""
from __future__ import annotations

import json
import os
import platform
import sys
import time
from pathlib import Path

import numpy as np
import scipy
from numpy.polynomial.legendre import leggauss
from scipy.special import expit, sph_harm_y


def occupation_and_weight(psi, statistics="MB"):
    psi = np.asarray(psi, dtype=float)
    if not np.all(np.isfinite(psi)):
        raise ValueError("nonfinite natural score")
    if statistics == "MB":
        f = np.exp(psi)
        w = f
    elif statistics == "FD":
        f = expit(psi)
        w = f * expit(-psi)
    elif statistics == "BE":
        if np.any(psi >= 0):
            raise ValueError("regular BE branch requires psi < 0")
        e = np.exp(psi)
        d = -np.expm1(psi)
        f, w = e / d, e / d**2
    else:
        raise ValueError("unknown statistics")
    if not np.all(np.isfinite(w)) or np.any(w <= 0):
        raise ValueError("positive representable susceptibility required")
    return f, w


def sphere_rule(L):
    if L < 0:
        raise ValueError("negative cutoff")
    mu, wm = leggauss(L + 2)
    phi = 2 * np.pi * np.arange(2 * L + 3) / (2 * L + 3)
    mm, pp = np.meshgrid(mu, phi, indexing="ij")
    theta, phi = np.arccos(mm.ravel()), pp.ravel()
    weights = np.repeat(wm, 2 * L + 3) * (2 * np.pi / (2 * L + 3))
    harmonics, ranks = [], []
    for ell in range(L + 1):
        for m in range(-ell, ell + 1):
            yy = sph_harm_y(ell, abs(m), theta, phi)
            if m < 0:
                yy = np.sqrt(2) * yy.imag
            elif m > 0:
                yy = np.sqrt(2) * yy.real
            else:
                yy = yy.real
            harmonics.append(yy)
            ranks.append(ell)
    return mm.ravel(), phi, weights, np.stack(harmonics, axis=1), np.array(ranks)


def block_parameters(x, radial_weights, velocity, reference_weight, ranks):
    """2x2 inverse Fisher blocks; central variance avoids moment subtraction."""
    params = {}
    for ell in np.unique(ranks):
        w = radial_weights * reference_weight * velocity ** (2 * ell)
        m0 = float(np.sum(w))
        mu = float(np.sum(w * x) / m0)
        variance = float(np.sum(w * (x - mu) ** 2) / m0)
        if not np.isfinite(m0 * variance) or m0 <= 0 or variance <= 0:
            raise ValueError("nondegenerate positive radial variance required")
        params[int(ell)] = (m0, mu, variance)
    # H0^-1 for score pair (eta,b), psi=eta-x*b.
    blocks = np.array([
        np.array([[1 + params[int(l)][1] ** 2 / params[int(l)][2],
                   params[int(l)][1] / params[int(l)][2]],
                  [params[int(l)][1] / params[int(l)][2],
                   1 / params[int(l)][2]]]) / params[int(l)][0]
        for l in ranks
    ])
    return params, blocks


def apply_blocks(blocks, vector):
    return np.einsum("nij,nj->ni", blocks, vector.reshape(-1, 2)).reshape(-1)


def build_problem(L=3, z=2.0, anisotropy=0.5, statistics="MB", narrow=False):
    mu, phi, wa, Y, ranks = sphere_rule(L)
    nr = 20
    nodes, wg = leggauss(nr)
    lo, hi = ((4.99, 5.01) if narrow else (z + 0.1, z + 12.0))
    if lo <= z:
        raise ValueError("energy grid must be above mass shell")
    x = 0.5 * (hi - lo) * nodes + 0.5 * (hi + lo)
    velocity = np.sqrt(1 - (z / x) ** 2)
    wr = 0.5 * (hi - lo) * wg * x**2 * velocity
    # Isotropic reference is deliberately not fitted to the test anisotropy.
    psi0 = -(x - z) - 1.0
    _, W0 = occupation_and_weight(psi0, statistics)
    # Each term is in the retained natural-score family v^ell*Y_lm*(eta-x*b).
    # Avoid using a generic anisotropic weight that is not a fitted-family Hessian.
    pattern = np.zeros((nr, len(wa)))
    if L >= 1:
        pattern += 0.55 * velocity[:, None] * mu[None, :]
    if L >= 2:
        angular2 = 0.3 * (1 - mu**2) * np.cos(2 * phi) + 0.15 * (3 * mu**2 - 1) / 2
        pattern += velocity[:, None]**2 * angular2[None, :]
    displacement = anisotropy * (0.8 + 0.02 * (x - z))[:, None] * pattern
    psi = psi0[:, None] + displacement
    _, W = occupation_and_weight(psi, statistics)
    ratio = W / W0[:, None]
    start = time.perf_counter()
    params, blocks = block_parameters(x, wr, velocity, W0, ranks)
    setup_seconds = time.perf_counter() - start
    angular_scores = velocity[:, None, None] ** ranks[None, None, :] * Y[None, :, :]
    A = np.stack((angular_scores, -x[:, None, None] * angular_scores), axis=-1)
    A = A.reshape(nr * len(wa), -1)
    weights = (wr[:, None] * wa[None, :] * W).ravel()
    started = time.perf_counter()
    H = A.T @ (weights[:, None] * A)
    assembly_seconds = time.perf_counter() - started
    # Build centered scores directly, avoiding subtracting nearly equal second moments.
    Q = np.empty((nr, len(wa), len(ranks), 2))
    for j, ell in enumerate(ranks):
        m0, center, variance = params[int(ell)]
        Q[:, :, j, 0] = angular_scores[:, :, j] / np.sqrt(m0)
        Q[:, :, j, 1] = angular_scores[:, :, j] * (center - x[:, None]) / np.sqrt(m0 * variance)
    Q = Q.reshape(len(weights), -1)
    B = Q.T @ (weights[:, None] * Q)
    ref_weights = (wr[:, None] * wa[None, :] * W0[:, None]).ravel()
    B0 = Q.T @ (ref_weights[:, None] * Q)
    rng = np.random.default_rng(9127 + L)
    x_true = rng.normal(size=len(H))
    rhs = H @ x_true
    # Check that the actual score displacement is in the declared finite family.
    natural_fit = np.linalg.lstsq(A, displacement.ravel(), rcond=None)[0]
    natural_fit_error = float(np.max(np.abs(A @ natural_fit - displacement.ravel())))
    return dict(H=H, B=B, B0=B0, rhs=rhs, x_true=x_true, blocks=blocks,
                lower=float(ratio.min()), upper=float(ratio.max()),
                delta=float(np.abs(displacement).max()),
                gamma=float(-max(psi.max(), psi0.max())),
                setup_seconds=setup_seconds, assembly_seconds=assembly_seconds,
                points=len(weights), dimension=len(H),
                natural_family_max_residual=natural_fit_error,
                x=x, wr=wr, velocity=velocity, W0=W0, ranks=ranks)


def cg_solve(H, rhs, inverse_blocks=None, rtol=1e-9, max_iterations=None):
    """CG/PCG with an actual natural-coordinate residual checked each iteration."""
    n = len(rhs)
    max_iterations = max_iterations or 20 * n
    x = np.zeros(n)
    norm_rhs = np.linalg.norm(rhs)
    if norm_rhs == 0:
        return x, {"iterations": 0, "relative_physical_residual": 0.0, "converged": True}
    r = rhs.copy()
    z = r.copy() if inverse_blocks is None else apply_blocks(inverse_blocks, r)
    p = z.copy()
    rz = float(r @ z)
    ratio = 1.0
    for k in range(1, max_iterations + 1):
        hp = H @ p
        denominator = float(p @ hp)
        if denominator <= 0 or not np.isfinite(denominator):
            break
        alpha = rz / denominator
        x += alpha * p
        r -= alpha * hp
        ratio = float(np.linalg.norm(H @ x - rhs) / norm_rhs)
        if ratio <= rtol:
            return x, {"iterations": k, "relative_physical_residual": ratio, "converged": True}
        z = r.copy() if inverse_blocks is None else apply_blocks(inverse_blocks, r)
        rz_new = float(r @ z)
        if rz_new <= 0 or not np.isfinite(rz_new):
            break
        p = z + (rz_new / rz) * p
        rz = rz_new
    return x, {"iterations": k, "relative_physical_residual": ratio, "converged": False}


def one_case(L, z, anisotropy, statistics="MB", narrow=False, repeats=7):
    problem = build_problem(L, z, anisotropy, statistics, narrow)
    H, B = problem["H"], problem["B"]
    ev0, ev1 = np.linalg.eigvalsh(H), np.linalg.eigvalsh(B)
    outputs = {}
    for label in ("plain_cg", "fisher_pcg"):
        times, result = [], None
        def solve_once():
            start = time.perf_counter()
            if label == "fisher_pcg":
                _, blocks = block_parameters(problem["x"], problem["wr"], problem["velocity"], problem["W0"], problem["ranks"])
            else:
                blocks = None
            solution, result = cg_solve(H, problem["rhs"], blocks)
            elapsed = time.perf_counter() - start
            return solution, result, elapsed
        # One warmup/calibration; each timed block targets about 5 ms.
        _, _, warmup_seconds = solve_once()
        batch_count = max(1, min(200, int(np.ceil(0.005 / max(warmup_seconds, 1e-9)))))
        for _ in range(repeats):
            start = time.perf_counter()
            for _ in range(batch_count):
                solution, result, _ = solve_once()
            times.append((time.perf_counter() - start) / batch_count)
        error = solution - problem["x_true"]
        relative_energy_error = np.sqrt(max(0, float(error @ H @ error)) /
                                        float(problem["x_true"] @ H @ problem["x_true"]))
        outputs[label] = dict(result, seconds=times, median_seconds=float(np.median(times)),
                              solves_per_timing_block=batch_count,
                              relative_solution_l2_error=float(np.linalg.norm(error) / np.linalg.norm(problem["x_true"])),
                              relative_solution_energy_error=float(relative_energy_error))
    a, b = problem["lower"], problem["upper"]
    delta = problem["delta"]
    log_lipschitz = 1.0 if statistics != "BE" else 1 / np.tanh(problem["gamma"] / 2)
    return dict(L=L, z=z, anisotropy=anisotropy, statistics=statistics, narrow=narrow,
                points=problem["points"], dimension=problem["dimension"],
                reference_whitening_max_abs=float(np.max(np.abs(problem["B0"] - np.eye(len(B))))),
                natural_family_max_residual=problem["natural_family_max_residual"],
                raw_condition=float(ev0[-1] / ev0[0]), whitened_condition=float(ev1[-1] / ev1[0]),
                weight_ratio_lower=a, weight_ratio_upper=b, condition_upper=b / a,
                whitened_eigen_min=float(ev1[0]), whitened_eigen_max=float(ev1[-1]),
                eigen_enclosure_pass=bool(ev1[0] >= a - 2e-10 and ev1[-1] <= b + 2e-10),
                natural_displacement_delta=delta, be_margin_gamma=problem["gamma"] if statistics == "BE" else None,
                theoretical_ratio_lower=float(np.exp(-delta * log_lipschitz)),
                theoretical_ratio_upper=float(np.exp(delta * log_lipschitz)),
                susceptibility_enclosure_pass=bool(a >= np.exp(-delta * log_lipschitz) - 1e-12 and b <= np.exp(delta * log_lipschitz) + 1e-12),
                shared_hessian_assembly_seconds=problem["assembly_seconds"],
                initial_preconditioner_setup_seconds=problem["setup_seconds"],
                runs=outputs,
                median_solve_speed_ratio=outputs["plain_cg"]["median_seconds"] / outputs["fisher_pcg"]["median_seconds"])


def main():
    started = time.perf_counter()
    cases = []
    for z in (0.0, 2.0, 8.0):
        for L in (1, 3, 5):
            cases.append(one_case(L, z, 0.6))
    cases += [one_case(3, 2.0, 0.6, "FD"), one_case(3, 2.0, 0.6, "BE"),
              one_case(3, 2.0, 0.6, "MB", narrow=True), one_case(3, 2.0, 0.0)]
    all_pass = all(c["eigen_enclosure_pass"] and c["susceptibility_enclosure_pass"]
                   and c["reference_whitening_max_abs"] < 2e-10
                   and c["natural_family_max_residual"] < 2e-12
                   and all(v["converged"] for v in c["runs"].values()) for c in cases)
    result = dict(schema="spherical-information.fisher-verification.v1", status="PASS_SCOPED" if all_pass else "FAIL",
                  scope="Finite positive quadrature entropy moment-inversion Hessian; does not measure VigilODE trajectory performance.",
                  command="OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 src/fisher_demo.py",
                  environment=dict(python=sys.version, numpy=np.__version__, scipy=scipy.__version__, platform=platform.platform(),
                                   openblas_threads=os.environ.get("OPENBLAS_NUM_THREADS"), omp_threads=os.environ.get("OMP_NUM_THREADS")),
                  definitions=dict(relative_residual="||H*x-b||_2/||b||_2 in natural eta,b coordinates", tolerance=1e-9,
                                   samples="one warmup/calibration then seven timing blocks per method; 1..200 repeated solves per block targeting 5 ms",
                                   timing="each solve; PCG includes rebuilding all radial 2x2 inverse blocks and all applications; common assembled H, quadrature, and spectral audit excluded",
                                   complexity="one inverse-block application O((L+1)^2), setup O(n_rad*(L+1)+(L+1)^2); dense H matvec used only in this small diagnostic",
                                   numerical_certification="floating-point inequalities with stated margins; not directed-rounding interval certification"),
                  elapsed_seconds=time.perf_counter() - started, cases=cases)
    out = Path(__file__).resolve().parents[1] / "results" / "fisher_results.json"
    out.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"status":result["status"], "cases":len(cases), "seconds":result["elapsed_seconds"],
                      "file":str(out), "max_raw_condition":max(c["raw_condition"] for c in cases),
                      "max_whitened_condition":max(c["whitened_condition"] for c in cases)}, indent=2))
    return 0 if all_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
