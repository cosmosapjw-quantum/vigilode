#!/usr/bin/env python3
"""Reference endpoint solutions and uncertainty for the U03 nonlinear problems.

Independent of the Rust code: scipy Radau (rtol 1e-13 and 1e-12) and a second method (rtol 1e-13), all
with analytic Jacobians.  Per component the reference uncertainty is
  U = |Radau13 - Radau12| + |Radau13 - M2_13|   (Richardson proxy + method disagreement),
with the declared second method M2 = DOP853 for the non-severely-stiff problems (VdP eps 0.1,
Fisher) and LSODA for VdP eps 1e-3 and Robertson.  Declared after a reference-only diagnostic
(before any Rust solver run): on Fisher LSODA and BDF at rtol 1e-13 differ from both Radau and
DOP853 by ~5e-8 WRMS while Radau and DOP853 agree to 1e-10.  All disagreements are recorded.
The Rust test reads nonlinear_reference.json.  Problem formulas must match
ppsw1_independent_qualification.rs (n-series tests) exactly.
"""
import json
import math
import sys
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp


def vdp(eps):
    def f(t, y):
        return [y[1], ((1 - y[0] ** 2) * y[1] - y[0]) / eps]

    def jac(t, y):
        return [[0.0, 1.0], [(-2 * y[0] * y[1] - 1) / eps, (1 - y[0] ** 2) / eps]]
    return f, jac, [2.0, -0.66], 0.5, 1e-6


def robertson():
    def f(t, y):
        return [-0.04 * y[0] + 1e4 * y[1] * y[2],
                0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] ** 2,
                3e7 * y[1] ** 2]

    def jac(t, y):
        return [[-0.04, 1e4 * y[2], 1e4 * y[1]],
                [0.04, -1e4 * y[2] - 6e7 * y[1], -1e4 * y[1]],
                [0.0, 6e7 * y[1], 0.0]]
    return f, jac, [1.0, 0.0, 0.0], 40.0, 1e-10


def fisher(n=20):
    dx = 1.0 / (n + 1)
    inv = 1.0 / (dx * dx)

    def f(t, u):
        out = []
        for i in range(n):
            left = u[i - 1] if i > 0 else 0.0
            right = u[i + 1] if i < n - 1 else 0.0
            out.append((left - 2 * u[i] + right) * inv + u[i] * (1 - u[i]))
        return out

    def jac(t, u):
        j = np.zeros((n, n))
        for i in range(n):
            j[i, i] = -2 * inv + 1 - 2 * u[i]
            if i > 0:
                j[i, i - 1] = inv
            if i < n - 1:
                j[i, i + 1] = inv
        return j
    y0 = [math.sin(math.pi * (i + 1) * dx) for i in range(n)]
    return f, jac, y0, 1.0, 1e-6


SECOND = {"vdp_eps_1e-1": "DOP853", "vdp_eps_1e-3": "LSODA", "robertson": "LSODA",
          "fisher_mol_n20": "DOP853"}
PROBLEMS = {"vdp_eps_1e-1": vdp(1e-1), "vdp_eps_1e-3": vdp(1e-3), "robertson": robertson(),
            "fisher_mol_n20": fisher()}


def solve(f, jac, y0, t_end, method, rtol, floor):
    kw = {"jac": jac} if method in ("Radau", "LSODA", "BDF") else {}
    r = solve_ivp(f, (0.0, t_end), y0, method=method, rtol=rtol, atol=floor * 1e-6, **kw)
    if not r.success:
        raise RuntimeError(f"{method} {rtol}: {r.message}")
    return np.array(r.y[:, -1]), int(r.nfev)


def main():
    out = {"schema": "vigilode.u03-nonlinear-reference.v1", "scipy": scipy.__version__,
           "uncertainty": "|Radau(1e-13)-Radau(1e-12)| + |Radau(1e-13)-M2(1e-13)| per component",
           "second_method": SECOND,
           "problems": {}}
    for name, (f, jac, y0, t_end, floor) in PROBLEMS.items():
        r13, n13 = solve(f, jac, y0, t_end, "Radau", 1e-13, floor)
        r12, _ = solve(f, jac, y0, t_end, "Radau", 1e-12, floor)
        w = np.abs(r13) + floor
        others = {}
        for m in ("LSODA", "DOP853", "BDF"):
            try:
                ym, _ = solve(f, jac, y0, t_end, m, 1e-13, floor)
                others[m] = ym
            except RuntimeError as e:
                others[m] = None
                print(name, m, "failed", e)
        m2 = others[SECOND[name]]
        unc = np.abs(r13 - r12) + np.abs(r13 - m2)
        disagreement = {m: (None if v is None else float(np.sqrt(np.mean(((v - r13) / w) ** 2))))
                        for m, v in others.items()}
        disagreement["Radau1e-12"] = float(np.sqrt(np.mean(((r12 - r13) / w) ** 2)))
        out["problems"][name] = {"t_end": t_end, "y0": y0, "weight_floor": floor,
                                 "y_ref": r13.tolist(), "u_ref": unc.tolist(),
                                 "u_ref_wrms": float(np.sqrt(np.mean((unc / w) ** 2))),
                                 "radau13_nfev": n13, "second_method": SECOND[name],
                                 "wrms_disagreement_vs_radau13": disagreement}
        print(name, "u_ref_wrms=%.3e" % out["problems"][name]["u_ref_wrms"])
    dest = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
    (dest / "nonlinear_reference.json").write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")


if __name__ == "__main__":
    main()
