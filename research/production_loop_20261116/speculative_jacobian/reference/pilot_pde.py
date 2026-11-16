#!/usr/bin/env python3
"""Problem characterisation for PREDECLARED.json (SciPy only, no VigilODE run).

Purpose: choose the end times of the two method-of-lines problems so that the end
point lies in the active phase (a moving front inside the domain). The script runs
SciPy BDF at a moderate tolerance and prints the front position against time.
It was run before PREDECLARED.json was written; its output is logs/pilot_pde.log.
"""
import sys
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp

# `python3 -I` does not put the script directory on the path.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from models import combustion, nldiff  # noqa: E402


def front(u, level):
    """Position (in cells from the left) where u falls through `level`; -1 if nowhere."""
    for i in range(len(u) - 1):
        if (u[i] - level) * (u[i + 1] - level) <= 0.0 and u[i] != u[i + 1]:
            return i + (u[i] - level) / (u[i] - u[i + 1])
    return -1.0


def main():
    print("PILOT scipy", scipy.__version__, "numpy", np.__version__)
    cells = 100
    model = combustion(cells, 0.32)
    times = [0.20 + 0.005 * k for k in range(25)]
    sol = solve_ivp(model["f"], model["t_span"], model["y0"], method="BDF", jac=model["jac"],
                    rtol=1e-8, atol=1e-10, t_eval=times)
    print("PILOT combustion-100 success", sol.success, "steps", len(sol.t), "nfev", sol.nfev)
    for k, t in enumerate(sol.t):
        u = sol.y[:, k]
        print("PILOT combustion-100 t %.3f u(0) %.6f u(mid) %.6f max %.6f front(1.5) cell %.2f of %d"
              % (t, u[0], u[cells // 2], u.max(), front(u, 1.5), cells))
    model = nldiff(cells, 2.0)
    times = [0.05 * k for k in range(1, 41)]
    sol = solve_ivp(model["f"], model["t_span"], model["y0"], method="BDF", jac=model["jac"],
                    rtol=1e-8, atol=1e-10, t_eval=times)
    print("PILOT nldiff-100 success", sol.success, "steps", len(sol.t), "nfev", sol.nfev)
    for k, t in enumerate(sol.t):
        u = sol.y[:, k]
        print("PILOT nldiff-100 t %.3f u(first) %.6f u(mid) %.6f u(last) %.6f min %.6f "
              "front(0.9) cell %.2f of %d"
              % (t, u[0], u[cells // 2], u[-1], u.min(), front(u, 0.9), cells))
    return 0


if __name__ == "__main__":
    sys.exit(main())
