#!/usr/bin/env python3
"""Approximate-J (theta) axis study for PPSW1-6b with an independent backend (U03).

T = theta * J + DeltaT.  Coefficients come from oracle_coefficients.oracle (route R1), never
from the Rust implementation; the production exact-J code is untouched.  This axis is separate
from the step-ratio axis sigma: (a) uses constant sigma, (b) reports uniform and variable sigma.

(a) scalar linear stability: y' = lam y, T = theta lam, z = h lam:
      M(z, theta) = (1 - gamma theta z)^-1 (B + z (theta Gamma + A)),
    spectral radius scan over z in -[1e-3, 1e8] (log grid) and theta in [0.8, 1.1].
(b) nonnormal stiff Prothero-Robinson system y' = L (y - phi) + phi', y(0) = phi(0), with an
    upper-triangular nonnormal L (eigenvalues -1, -50, -1000; off-diagonals 1e2, 1e3) and
    noncommuting DeltaT = eps * ||L||_2 * R (R fixed random, ||R||_2 = 1).  Exact start data on
    phi at t0 + h c_j (c_1 = -1).  Global endpoint error at t = 1 vs. phi(1).
Output: theta_oracle_results.json.
"""
import json
import sys
from pathlib import Path

import mpmath as mp
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import oracle_coefficients as oc  # noqa: E402

mp.mp.dps = 40
GAMMA, _ = oc.gamma_hat()
G = float(GAMMA)
C = np.array([-1.0, -0.5, 0.0, 0.4, 0.8, 1.0])
_cache = {}


def coeffs(sigma):
    key = float(sigma)
    if key not in _cache:
        m, _ = oc.oracle(key, GAMMA)
        _cache[key] = {k: np.array([[float(v[i, j]) for j in range(6)] for i in range(6)])
                       for k, v in m.items()}
    return _cache[key]


NS = (12, 24, 48, 96)
THETAS = [0.8, 0.85, 0.9, 0.95, 1.0, 1.05, 1.1]


def scalar_scan():
    zs = -np.logspace(-3, 8, 600)
    out = []
    for sigma in (0.5, 1.0, 1.1):
        k = coeffs(sigma)
        for th in THETAS:
            rho = []
            for z in zs:
                M = (k["B"] + z * (th * k["Gamma"] + k["A"])) / (1.0 - G * th * z)
                rho.append(max(abs(np.linalg.eigvals(M))))
            rho = np.array(rho)
            bad = zs[rho > 1.0 + 1e-9]
            out.append({"sigma": sigma, "theta": th, "max_rho": float(rho.max()),
                        "z_at_max": float(zs[rho.argmax()]),
                        "rho_at_z_minus_1e8": float(rho[-1]),
                        "unstable_z_range": [float(bad.max()), float(bad.min())] if bad.size else None})
    return out


L = np.array([[-1.0, 1e2, 0.0], [0.0, -50.0, 1e3], [0.0, 0.0, -1000.0]])
rng = np.random.default_rng(20261115)
R = rng.standard_normal((3, 3))
R /= np.linalg.norm(R, 2)
NL = np.linalg.norm(L, 2)


def phi(t):
    return np.array([np.sin(t), np.cos(2 * t), np.exp(-0.5 * t)])


def dphi(t):
    return np.array([np.cos(t), -2 * np.sin(2 * t), -0.5 * np.exp(-0.5 * t)])


def f(t, y):
    return L @ (y - phi(t)) + dphi(t)


def run(theta, eps, hs):
    T = theta * L + eps * NL * R
    h_prev = hs[0]
    t = 0.0  # reference t_m for the first computed step; previous nodes at t + h_prev (c - 1)
    Y = [phi(t + h_prev * (c - 1)) for c in C]
    F = [dphi(t + h_prev * (c - 1)) for c in C]
    for h in hs:
        sigma = h / h_prev
        k = coeffs(sigma)
        W = np.eye(3) - G * h * T
        Yn, Fn = [], []
        for i in range(6):
            rhs = sum(k["B"][i, j] * Y[j] + h * k["A"][i, j] * F[j] + h * (T @ (k["Gamma"][i, j] * Y[j]))
                      for j in range(6))
            yi = np.linalg.solve(W, rhs)
            Yn.append(yi)
            Fn.append(f(t + C[i] * h, yi))
        Y, F, h_prev, t = Yn, Fn, h, t + h
        if not np.all(np.isfinite(Y[5])) or np.max(np.abs(Y[5])) > 1e12:
            return float("inf"), t
    # Y_{m,6} sits at t_m + h_m c_6 = t_m + h_m, i.e. the last accepted time t.
    return float(np.max(np.abs(Y[5] - phi(t)))), t


def schedules(n):
    uni = [1.0 / n] * n
    # cycle product is 1, so the schedule refines uniformly with n (sigma in {1.1, 1.05, 0.6505})
    cyc = [1.1, 1.1, 1.1, 1.1, 1.05, 1.0 / (1.1 ** 4 * 1.05)]
    r, raw = 1.0, []
    for m in range(n):
        raw.append(r)
        r *= cyc[m % len(cyc)]
    scale = 1.0 / sum(raw)
    var = [x * scale for x in raw]
    return {"uniform": uni, "variable": var}


def system_scan():
    rows = []
    for th in THETAS:
        for eps in (0.0, 1e-3, 1e-2, 1e-1):
            for kind in ("uniform", "variable"):
                errs = []
                for n in NS:
                    e, tend = run(th, eps, schedules(n)[kind])
                    errs.append(e)
                orders = [float(np.log2(errs[i] / errs[i + 1]))
                          if np.isfinite(errs[i]) and np.isfinite(errs[i + 1]) and errs[i + 1] > 0 else None
                          for i in range(3)]
                # strict JSON: a diverged (non-finite) run is recorded as null with diverged=true
                rows.append({"theta": th, "eps": eps, "schedule": kind, "N": list(NS),
                             "endpoint_error": [e if np.isfinite(e) else None for e in errs], "observed_orders": orders,
                             "diverged": any(not np.isfinite(e) for e in errs)})
    return rows


def main():
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
    res = {"schema": "vigilode.ppsw1-theta-oracle.v1", "gamma": G, "thetas": THETAS,
           "backend": "numpy float64 stepping with mpmath R1 coefficients",
           "L": L.tolist(), "R_seed": 20261115, "R": R.tolist(), "norm_L_2": NL,
           "scalar_stability": scalar_scan(), "nonnormal_pr_system": system_scan()}
    (out_dir / "theta_oracle_results.json").write_text(json.dumps(res, indent=1, allow_nan=False) + "\n")
    for r in res["scalar_stability"]:
        print("SCALAR", r)
    for r in res["nonnormal_pr_system"]:
        print("SYSTEM", r["theta"], r["eps"], r["schedule"],
              ["inf" if e is None else "%.2e" % e for e in r["endpoint_error"]],
              [None if o is None else round(o, 2) for o in r["observed_orders"]])


if __name__ == "__main__":
    main()
