#!/usr/bin/env python3
"""U11 references (pre-registered rule) and SciPy BDF/Radau count-only comparators."""
import json
import math
import sys
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp

HERE = Path(__file__).resolve().parent


def hires():
    def f(t, y):
        return [-1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007, 1.71 * y[0] - 8.75 * y[1],
                -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4], 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
                -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
                -280 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6],
                280 * y[5] * y[7] - 1.81 * y[6], -280 * y[5] * y[7] + 1.81 * y[6]]

    def j(t, y):
        m = np.zeros((8, 8))
        m[0, :3] = [-1.71, 0.43, 8.32]; m[1, :2] = [1.71, -8.75]
        m[2, 2:5] = [-10.03, 0.43, 0.035]; m[3, 1:4] = [8.32, 1.71, -1.12]
        m[4, 4:7] = [-1.745, 0.43, 0.43]
        m[5, 3:8] = [0.69, 1.71, -0.43 - 280 * y[7], 0.69, -280 * y[5]]
        m[6, 5:8] = [280 * y[7], -1.81, 280 * y[5]]; m[7, 5:8] = [-280 * y[7], 1.81, -280 * y[5]]
        return m
    return f, j, [1, 0, 0, 0, 0, 0, 0, 0.0057], 321.8122


def oregonator():
    def f(t, y):
        return [77.27 * (y[1] + y[0] * (1 - 8.375e-6 * y[0] - y[1])),
                (y[2] - (1 + y[0]) * y[1]) / 77.27, 0.161 * (y[0] - y[2])]

    def j(t, y):
        return [[77.27 * (1 - 2 * 8.375e-6 * y[0] - y[1]), 77.27 * (1 - y[0]), 0],
                [-y[1] / 77.27, -(1 + y[0]) / 77.27, 1 / 77.27], [0.161, 0, -0.161]]
    return f, j, [1.0, 2.0, 3.0], 360.0


def brusselator(n=32, a=1.0, b=3.0, alpha=1 / 50):
    dx = 1 / (n + 1)
    c = alpha / dx ** 2

    def f(t, y):
        u, v = y[:n], y[n:]
        ue = np.concatenate([[1.0], u, [1.0]]); ve = np.concatenate([[3.0], v, [3.0]])
        du = a + u * u * v - (b + 1) * u + c * (ue[:-2] - 2 * u + ue[2:])
        dv = b * u - u * u * v + c * (ve[:-2] - 2 * v + ve[2:])
        return np.concatenate([du, dv])

    def j(t, y):
        u, v = y[:n], y[n:]
        m = np.zeros((2 * n, 2 * n))
        for i in range(n):
            m[i, i] = 2 * u[i] * v[i] - (b + 1) - 2 * c; m[i, n + i] = u[i] ** 2
            m[n + i, i] = b - 2 * u[i] * v[i]; m[n + i, n + i] = -u[i] ** 2 - 2 * c
            if i > 0:
                m[i, i - 1] = c; m[n + i, n + i - 1] = c
            if i < n - 1:
                m[i, i + 1] = c; m[n + i, n + i + 1] = c
        return m
    x = np.arange(1, n + 1) * dx
    return f, j, list(np.concatenate([1 + np.sin(2 * math.pi * x), 3 * np.ones(n)])), 10.0


PROBLEMS = {"hires": hires(), "oregonator": oregonator(), "brusselator_mol32": brusselator()}


def solve(f, j, y0, t_end, method, rtol, atol):
    kw = {"jac": j} if method in ("Radau", "BDF", "LSODA") else {}
    r = solve_ivp(f, (0, t_end), y0, method=method, rtol=rtol, atol=atol, **kw)
    return r


def main():
    out = {"schema": "vigilode.u11-reference.v1", "scipy": scipy.__version__, "problems": {}}
    for name, (f, j, y0, t_end) in PROBLEMS.items():
        r13 = solve(f, j, y0, t_end, "Radau", 1e-13, 1e-16)
        r12 = solve(f, j, y0, t_end, "Radau", 1e-12, 1e-15)
        second, m2 = None, None
        for m in ("DOP853", "LSODA"):
            try:
                rr = solve(f, j, y0, t_end, m, 1e-13, 1e-16)
                if rr.success:
                    second, m2 = rr, m
                    break
            except Exception:
                pass
        yref = r13.y[:, -1]
        unc = np.abs(yref - r12.y[:, -1]) + (np.abs(yref - second.y[:, -1]) if second else np.inf)
        w = np.abs(yref) + 1e-6
        comp = []
        for tol in [1e-4, 1e-5, 1e-6, 1e-7, 1e-8]:
            for m in ("BDF", "Radau"):
                r = solve(f, j, y0, t_end, m, tol, tol * 1e-2)
                e = float(np.sqrt(np.mean(((r.y[:, -1] - yref) / w) ** 2))) if r.success else None
                comp.append({"method": "scipy_" + m, "tol": tol, "success": bool(r.success),
                             "error_wrms": e, "nfev": int(r.nfev), "njev": int(r.njev), "nlu": int(r.nlu),
                             "steps": int(len(r.t) - 1)})
        out["problems"][name] = {"t_end": t_end, "y_ref": yref.tolist(), "u_ref": unc.tolist(),
                                 "u_ref_wrms": float(np.sqrt(np.mean((unc / w) ** 2))),
                                 "second_method": m2, "radau13_success": bool(r13.success),
                                 "scipy_comparators": comp}
        print(name, "U_ref %.3e" % out["problems"][name]["u_ref_wrms"], m2, file=sys.stderr)
    (HERE / "reference.json").write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")


if __name__ == "__main__":
    main()
