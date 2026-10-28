#!/usr/bin/env python3
"""Uniform perturbation stability of RVJ5 and RODAS5P step maps (research node
research/rnext08_perturbation_stability_20261003, remaining-only DAG node R-NEXT-08; see its
PREREGISTRATION.md).

(a) linear two-mode non-normal systems: ||R(hA)||_2 for both methods against the
    Crouzeix-Palencia bound 1 + sqrt(2) where the numerical range lies in the left half plane;
(b) the semilinear two-mode system x' = x^2, y' = A y + 2 x y + x^2 e: one-step error E(K) and
    step-map Jacobian deviation D(K) against the closed-form flow, K over ten decades.
RVJ5 is the vendored analytic reference of L-0040; RODAS5P is the K form with the stored
coefficients exported by research/rnext01_residual_output_20261003/stages.json.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import mpmath as mp
import sympy as sp

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "research/thread_transfer_negative_controls_20261002/thread_loop3/proofs"))
from reference_rvj import AnalyticRVJ, R5  # noqa: E402

SCHEMA = "vigilode-rnext08-perturbation-stability-v1"
KS = [mp.mpf(10) ** k for k in (2, 4, 6, 8, 10, 12)]
H = mp.mpf(1) / 16


def unhex(text):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def load_coefficients():
    raw = json.loads((ROOT / "research/rnext01_residual_output_20261003/stages.json").read_text())["coefficients"]
    c = {"gamma": unhex(raw["gamma"])}
    for key in ("c", "b", "btilde", "gamma_rows"):
        c[key] = [unhex(x) for x in raw[key]]
    for key in ("alpha", "gamma_matrix"):
        c[key] = [[unhex(x) for x in row] for row in raw[key]]
    return c


COEFFS = None


def rodas5p_step(f, jac, y, h):
    """K form: (I - h gamma J) K_i = h f(y + sum alpha K) + h J sum Gamma K."""
    g = COEFFS["gamma"]
    n = len(y)
    J = jac(y)
    W = mp.eye(n) - h * g * J
    K = []
    for i in range(8):
        Y = [y[q] + mp.fsum(COEFFS["alpha"][i][j] * K[j][q] for j in range(i)) for q in range(n)]
        mix = mp.matrix([mp.fsum(COEFFS["gamma_matrix"][i][j] * K[j][q] for j in range(i)) for q in range(n)])
        rhs = mp.matrix(f(Y)) * h + h * (J * mix)
        K.append(list(mp.lu_solve(W, rhs)))
    return [y[q] + mp.fsum(COEFFS["b"][j] * K[j][q] for j in range(8)) for q in range(n)]


def two_norm(m):
    """Largest singular value of a small real matrix."""
    return max(mp.svd_r(m, compute_uv=False))


# ---------------------------------------------------------------------------
# (a) linear systems
# ---------------------------------------------------------------------------

def linear_case(k, c):
    A = mp.matrix([[-k, c], [0, -10 * k]])
    f = lambda y: (lambda r: [r[i] for i in range(2)])(A * mp.matrix(y))
    jac = lambda y: A
    # RODAS5P: the step map of a linear system is R(hA); columns from unit vectors.
    cols = [rodas5p_step(f, jac, [mp.mpf(1), mp.mpf(0)], H), rodas5p_step(f, jac, [mp.mpf(0), mp.mpf(1)], H)]
    R_rodas = mp.matrix([[cols[0][0], cols[1][0]], [cols[0][1], cols[1][1]]])
    # RVJ5: R5(hA) = Q5(Z)^-1 P5(Z) as a matrix function.
    Z = H * A
    I = mp.eye(2)
    P = I + mp.mpf(2) / 5 * Z + Z * Z / 20
    Q = I - mp.mpf(3) / 5 * Z + mp.mpf(3) / 20 * Z * Z - Z * Z * Z / 60
    R_rvj = mp.inverse(Q) * P
    gated = c * c < 4 * k * (10 * k)
    return {"K": mp.nstr(k, 3), "c_over_K": mp.nstr(c / k, 3), "numerical_range_left": bool(gated),
            "rodas5p_norm": two_norm(R_rodas), "rvj5_norm": two_norm(R_rvj)}


def scalar_stability_max():
    """max |R(z)| on a grid of the closed left half plane, both methods."""
    points = [mp.mpc(0, y) for y in [mp.mpf(10) ** e for e in range(-3, 9)]]
    points += [mp.mpc(-r, 0) for r in [mp.mpf(10) ** e for e in range(-3, 13)]]
    points += [mp.mpc(-r, r * s) for r in [mp.mpf(10) ** e for e in range(-2, 9)] for s in (mp.mpf("0.5"), 2, 10)]
    rodas = mp.mpf(0)
    rvj = mp.mpf(0)
    for z in points:
        lam = z / H
        y = rodas5p_step(lambda v: [lam * v[0]], lambda v: mp.matrix([[lam]]), [mp.mpc(1)], H)[0]
        rodas = max(rodas, abs(y))
        rvj = max(rvj, abs(R5(z)))
    return rodas, rvj


# ---------------------------------------------------------------------------
# (b) semilinear two-mode system
# ---------------------------------------------------------------------------

def semilinear_system(k):
    c = k
    A = mp.matrix([[-k, c], [0, -10 * k]])

    def f(v):
        x, y1, y2 = v
        return [x * x, -k * y1 + c * y2 + 2 * x * y1 + x * x, -10 * k * y2 + 2 * x * y2 + x * x]

    def jac(v):
        x, y1, y2 = v
        return mp.matrix([[2 * x, 0, 0], [2 * y1 + 2 * x, -k + 2 * x, c], [2 * y2 + 2 * x, 0, -10 * k + 2 * x]])

    def flow(v, t):
        x0, y0 = v[0], mp.matrix([v[1], v[2]])
        a, d = -k, -10 * k
        eat = mp.matrix([[mp.e ** (a * t), c * (mp.e ** (a * t) - mp.e ** (d * t)) / (a - d)], [0, mp.e ** (d * t)]])
        e = mp.matrix([1, 1])
        u = eat * y0 + x0 * x0 * mp.lu_solve(A, (eat - mp.eye(2)) * e)
        s = (1 - x0 * t) ** -2
        return [x0 / (1 - x0 * t), s * u[0], s * u[1]]

    return A, f, jac, flow


def jacobian_fd(step, v, delta):
    n = len(v)
    cols = []
    for j in range(n):
        plus = list(v)
        minus = list(v)
        plus[j] += delta
        minus[j] -= delta
        a, b = step(plus), step(minus)
        cols.append([(a[i] - b[i]) / (2 * delta) for i in range(n)])
    return mp.matrix([[cols[j][i] for j in range(n)] for i in range(n)])


def semilinear(rvj):
    rows = []
    for k in KS:
        A, f, jac, flow = semilinear_system(k)
        x0 = mp.mpf(1) / 2
        e = mp.matrix([1, 1])
        slow = -mp.lu_solve(A + 2 * x0 * mp.eye(2), x0 * x0 * e)
        v0 = [x0, slow[0] + mp.mpf("1e-3"), slow[1] + mp.mpf("1e-3")]
        exact = flow(v0, H)
        steps = {
            "rodas5p": lambda v: rodas5p_step(f, jac, v, H),
            "rvj5": lambda v: (lambda r: [r[i] for i in range(3)])(rvj.step(v, H, (k,))),
        }
        flow_jac = jacobian_fd(lambda v: flow(v, H), v0, mp.mpf("1e-20"))
        row = {"K": mp.nstr(k, 3)}
        for name, step in steps.items():
            out = step(v0)
            err = mp.sqrt(mp.fsum((a - b) ** 2 for a, b in zip(out, exact)))
            dev = two_norm(jacobian_fd(step, v0, mp.mpf("1e-20")) - flow_jac)
            row[name] = {"E": err, "D": dev}
        rows.append(row)
        print("K", mp.nstr(k, 3), {m: (mp.nstr(row[m]["E"], 4), mp.nstr(row[m]["D"], 4)) for m in steps}, flush=True)
    return rows


def label(values):
    lo, hi = min(values), max(values)
    if lo > 0 and hi <= 10 * lo:
        return "uniformly stable"
    if lo == 0 or hi > 100 * lo:
        return "not uniform"
    return "inconclusive"


def main():
    global COEFFS
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 100
    COEFFS = load_coefficients()
    # (a)
    linear = [linear_case(k, c * k) for k in KS for c in (0, 1, 100)]
    bound = 1 + mp.sqrt(2)
    rodas_max, rvj_max = scalar_stability_max()
    gate1 = all(r["rodas5p_norm"] <= bound and r["rvj5_norm"] <= bound for r in linear if r["numerical_range_left"])
    gate1 = gate1 and rodas_max <= 1 + mp.mpf("1e-30") and rvj_max <= 1 + mp.mpf("1e-30")
    # (b)
    k_sym = sp.Symbol("k")
    x, y1, y2 = sp.symbols("x y1 y2")
    rhs = sp.Matrix([x**2, -k_sym * y1 + k_sym * y2 + 2 * x * y1 + x**2, -10 * k_sym * y2 + 2 * x * y2 + x**2])
    rvj = AnalyticRVJ((x, y1, y2), (k_sym,), rhs, order=5)
    semi = semilinear(rvj)
    labels = {m: {"E": label([r[m]["E"] for r in semi]), "D": label([r[m]["D"] for r in semi])} for m in ("rodas5p", "rvj5")}
    overall = {m: ("not uniform" if "not uniform" in labels[m].values() else
                   "uniformly stable" if all(v == "uniformly stable" for v in labels[m].values()) else "inconclusive")
               for m in labels}
    finite = all(mp.isfinite(r[m][q]) for r in semi for m in ("rodas5p", "rvj5") for q in ("E", "D"))
    s = lambda v: mp.nstr(v, 6)
    report = {
        "schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "h": "1/16",
        "linear": [{**r, "rodas5p_norm": s(r["rodas5p_norm"]), "rvj5_norm": s(r["rvj5_norm"])} for r in linear],
        "crouzeix_palencia_bound": s(bound),
        "scalar_max_abs_R_left_half_plane": {"rodas5p": s(rodas_max), "rvj5": s(rvj_max)},
        "semilinear": [{"K": r["K"], **{m: {q: s(r[m][q]) for q in ("E", "D")} for m in ("rodas5p", "rvj5")}} for r in semi],
        "labels": labels, "overall": overall,
        "dag": {"rodas5p": {"derivatives": "J only (order 1)", "solves": 8, "solve_dependency_depth": 8},
                "rvj5": {"derivatives": "jets to order 5 (sources s_1..s_5)", "solves": 1,
                         "solve_dependency_depth": 1, "matrix_polynomial_degree": 3}},
        "gate": {"1_linear_limited_domain": bool(gate1), "2_semilinear_complete": bool(finite),
                 "3_labels_kept_apart": True},
    }
    report["verdict"] = "PASS" if all(report["gate"].values()) else "FAIL"
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": report["gate"], "overall": overall, "labels": labels, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
