#!/usr/bin/env python3
"""Definition checks of the fresh problem set of V30 (phase 1, before PREDECLARED.json is pushed).

No VigilODE driver and no arm of the evaluation runs here. For each problem of fresh_set.py:
* a closed form, where stated: y(t0) = y0 and the residual |y'(t) - f(t, y(t))| at four times
  (mpmath derivative, 50 digits, f in double at the rounded exact state);
* an independent scipy integration (Radau rtol 1e-9 / atol 1e-12 for the stiff group, DOP853
  rtol 1e-10 / atol 1e-12 otherwise) over the span: finite, and against the closed form at t1;
* a stiffness estimate: the spectral radius of a forward-difference Jacobian at 41 points of the
  scipy solution, its maximum, and S = integral of rho(t) dt / 3.3, the attempts DOPRI5 needs at
  its real stability limit (a lower bound when stiffness binds; max_steps is 100000);
* the stated derivative callbacks (dense, band, JVP, time derivative) against finite differences.
Writes logs/fresh_set_checks.json (strict JSON). Usage: python3 -I check_fresh_set.py
"""
import json
import math
import os
import sys

import mpmath
import numpy as np
from scipy.integrate import solve_ivp

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fresh_set as fs  # noqa: E402

OUT = os.path.join(HERE, "logs", "fresh_set_checks.json")


def num(x, digits=6):
    x = float(x)
    if not math.isfinite(x):
        return None
    return float(f"{x:.{digits}e}") if x != 0 else 0.0


def fd_jacobian(f, t, y):
    y = np.asarray(y, dtype=float)
    f0 = np.asarray(f(t, y), dtype=float)
    J = np.zeros((len(y), len(y)))
    for j in range(len(y)):
        yy = y.copy()
        d = math.sqrt(np.finfo(float).eps) * max(abs(y[j]), 1.0)
        yy[j] = y[j] + d
        J[:, j] = (np.asarray(f(t, yy), dtype=float) - f0) / (yy[j] - y[j])
    return J


def exact_of(p):
    if p.get("exact") is not None:
        return p["exact"]
    if p.get("exact_mp") is not None:
        M, y0 = p["exact_mp"]()
        y0 = mpmath.matrix(y0)
        return lambda t: list(mpmath.expm(M * mpmath.mpf(t)) * y0)
    return None


def check_closed_form(p, exact):
    t0, t1 = p["span"]
    y0 = exact(t0)
    out = {"y0_matches": all(abs(float(a) - b) <= 1e-15 * max(1.0, abs(b)) for a, b in zip(y0, p["y0v"]))}
    if p["rhs"] is None:
        return out
    worst = 0.0
    for frac in (0.1, 0.37, 0.73, 1.0):
        t = t0 + frac * (t1 - t0)
        y = [float(v) for v in exact(t)]
        dy = [mpmath.diff(lambda s, k=k: exact(s)[k], mpmath.mpf(t)) for k in range(len(y))]
        f = p["rhs"](t, np.asarray(y))
        scale = max(1.0, max(abs(float(v)) for v in dy))
        worst = max(worst, max(abs(float(a) - float(b)) for a, b in zip(dy, f)) / scale)
    out["max_relative_residual"] = num(worst)
    return out


def check_problem(p):
    row = {"id": p["id"], "group": p["group"], "n": p["n"], "b": fs.break_even_length(p["n"])}
    exact = exact_of(p)
    if exact is not None:
        row["closed_form"] = check_closed_form(p, exact)
    if p["rhs"] is None or p["id"] in ("cb_time",):
        return row
    t0, t1 = p["span"]
    stiff = p["group"] == "stiff_or_late_stiff" or p["id"] in ("cb_column",)
    method, rtol = ("Radau", 1e-9) if stiff else ("DOP853", 1e-10)
    sol = solve_ivp(lambda t, y: np.asarray(p["rhs"](t, y), dtype=float), (t0, t1),
                    np.asarray(p["y0v"], dtype=float), method=method, rtol=rtol, atol=1e-12,
                    dense_output=True)
    row["scipy"] = {"method": method, "rtol": rtol, "success": bool(sol.success),
                    "nfev": int(sol.nfev), "steps": int(len(sol.t) - 1),
                    "final_max_abs": num(np.max(np.abs(sol.y[:, -1]))),
                    "final_finite": bool(np.all(np.isfinite(sol.y[:, -1])))}
    if exact is not None and sol.success:
        ref = np.array([float(v) for v in exact(t1)])
        row["scipy"]["max_abs_diff_to_closed_form_at_t1"] = num(np.max(np.abs(sol.y[:, -1] - ref)))
    if sol.success:
        ts = np.linspace(t0, t1, 41)
        rhos = []
        for t in ts:
            J = fd_jacobian(lambda s, y: p["rhs"](s, y), t, sol.sol(t))
            rhos.append(float(np.max(np.abs(np.linalg.eigvals(J)))))
        rhos = np.array(rhos)
        S = float(np.sum((rhos[1:] + rhos[:-1]) / 2 * np.diff(ts)) / 3.3)
        row["stiffness"] = {"rho_max": num(rhos.max()), "rho_at_t0": num(rhos[0]),
                            "rho_at_t1": num(rhos[-1]),
                            "dopri5_stability_attempts_S": num(S),
                            "S_over_max_steps": num(S / 1e5)}
    return row


def derivative_checks():
    """The stated derivative callbacks against forward differences of the stated f."""
    rows = []
    rng = np.random.default_rng(20261125)
    chem = next(p for p in fs.problems() if p["id"] == "chem_d4")["rhs"]
    for _ in range(3):
        y = rng.uniform(0.1, 1.5, 3) * np.array([1, 1, 1e-3])
        J = np.array([[-0.013 - 1000 * y[2], 0, -1000 * y[0]], [0, -2500 * y[2], -2500 * y[1]],
                      [-0.013 - 1000 * y[2], -2500 * y[2], -1000 * y[0] - 2500 * y[1]]])
        Jfd = fd_jacobian(chem, 0.0, y)
        rows.append({"callback": "sr_chem_dense J", "max_rel_diff": num(np.max(np.abs(J - Jfd)) / np.max(np.abs(J)))})
    pr = next(p for p in fs.problems() if p["id"] == "pr_sin2t")["rhs"]
    for t in (0.3, 2.2, 4.9):
        y = np.array([math.sin(2 * t) + 1e-3])
        ft = 2e4 * math.cos(2 * t) - 4 * math.sin(2 * t)
        e = 1e-6
        ftfd = (pr(t + e, y)[0] - pr(t - e, y)[0]) / (2 * e)
        Jfd = fd_jacobian(pr, t, y)[0, 0]
        rows.append({"callback": "sr_pr_dense_ft J and f_t", "J_rel_diff": num(abs(Jfd + 1e4) / 1e4),
                     "ft_rel_diff": num(abs(ft - ftfd) / max(abs(ft), 1.0))})
    ac = fs.ac_rhs
    u = 0.53 * fs.AC_X + 0.47 * np.sin(-1.5 * np.pi * fs.AC_X)
    Jfd = fd_jacobian(ac, 0.0, u)
    inv = 0.01 / fs.AC_DX ** 2
    band = np.diag(-2 * inv + 1 - 3 * u ** 2) + np.diag(np.full(99, inv), 1) + np.diag(np.full(99, inv), -1)
    v = rng.standard_normal(100)
    vp = np.concatenate(([0.0], v, [0.0]))
    jvp = inv * (vp[:-2] - 2 * vp[1:-1] + vp[2:]) + (1 - 3 * u ** 2) * v
    rows.append({"callback": "sr_ac_band band", "max_rel_diff": num(np.max(np.abs(band - Jfd)) / np.max(np.abs(band)))})
    rows.append({"callback": "sr_ac_jvp JVP", "max_rel_diff": num(np.max(np.abs(jvp - Jfd @ v)) / np.max(np.abs(jvp)))})
    yb = np.array([0.7])
    rows.append({"callback": "cb_blowup J", "max_rel_diff": num(abs(fd_jacobian(lambda t, y: [1 + y[0] ** 2], 0.0, yb)[0, 0] - 1.4) / 1.4)})
    ym = np.array([0.4, 0.3, 0.3])
    g = lambda t, y: [-y[0] + 2 * y[2], y[0] - 3 * y[1], y[0] + y[1] + y[2] - 1]  # noqa: E731
    Jm = np.array([[-1, 0, 2], [1, -3, 0], [1, 1, 1]], dtype=float)
    rows.append({"callback": "cb_mass J", "max_rel_diff": num(np.max(np.abs(fd_jacobian(g, 0.0, ym) - Jm)) / 3)})
    return rows


def mass_check():
    p = next(p for p in fs.problems() if p["id"] == "cb_mass")
    ex = p["exact"]
    worst = 0.0
    for t in (0.0, 0.5, 2.0, 5.0):
        y = ex(t)
        dy = [mpmath.diff(lambda s, k=k: ex(s)[k], mpmath.mpf(t)) for k in range(2)]
        r1 = dy[0] - (-y[0] + 2 * y[2])
        r2 = dy[1] - (y[0] - 3 * y[1])
        r3 = y[0] + y[1] + y[2] - 1
        worst = max(worst, abs(r1), abs(r2), abs(r3))
    return {"max_residual_of_the_dae": num(worst), "y0": [num(v) for v in ex(0.0)]}


def main():
    rows = [check_problem(p) for p in fs.problems()]
    out = {
        "schema": "vigilode.v30.fresh_set_checks.v1",
        "note": "definition checks only: closed forms, an independent scipy integration and a "
                "stiffness estimate per problem; no VigilODE driver and no arm ran",
        "problems": rows,
        "derivative_callbacks": derivative_checks(),
        "cb_mass_closed_form": mass_check(),
        "scipy_version": __import__("scipy").__version__,
        "numpy_version": np.__version__,
        "mpmath_version": mpmath.__version__,
    }
    with open(OUT, "w") as f:
        json.dump(out, f, indent=1, allow_nan=False)
        f.write("\n")
    for r in rows:
        cf = r.get("closed_form", {})
        sc = r.get("scipy", {})
        st = r.get("stiffness", {})
        print("%-16s n=%-3d b=%-4d cf=%s res=%s scipy=%s steps=%s diff=%s rho=%s S=%s" % (
            r["id"], r["n"], r["b"], cf.get("y0_matches"), cf.get("max_relative_residual"),
            sc.get("success"), sc.get("steps"), sc.get("max_abs_diff_to_closed_form_at_t1"),
            st.get("rho_max"), st.get("dopri5_stability_attempts_S")))
    for r in out["derivative_callbacks"]:
        print(r)
    print(out["cb_mass_closed_form"])


if __name__ == "__main__":
    main()
