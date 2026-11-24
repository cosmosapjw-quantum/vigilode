"""Independent SciPy BDF/Radau endpoint reference for the Rust manufactured test."""

import json
import math

import numpy as np
import scipy
from scipy.integrate import solve_ivp


def exact(t):
    return 0.25 + math.sin(t)


def rhs(t, y):
    x = exact(t)
    return (-80 * y - y**3) + (math.cos(t) + 80 * x + x**3)


def jacobian(t, y):
    return np.array([[-80 - 3 * y[0] ** 2]])


def main():
    rows = []
    for method in ("BDF", "Radau"):
        for rtol in (1e-4, 1e-6, 1e-8):
            sol = solve_ivp(
                rhs, (0, 1), [0.25], method=method, jac=jacobian, rtol=rtol,
                atol=0.01 * rtol, first_step=0.05,
            )
            if not sol.success:
                raise RuntimeError(sol.message)
            rows.append({
                "method": method, "rtol": rtol, "accepted": len(sol.t) - 1,
                "nfev": sol.nfev, "njev": sol.njev, "nlu": sol.nlu,
                "endpoint_error": abs(sol.y[0, -1] - exact(1.0)),
            })
    print(json.dumps({
        "schema": 1, "scipy": scipy.__version__, "numpy": np.__version__,
        "problem": "x=1/4+sin(t); f(t,y)=-80*y-y^3+cos(t)+80*x+x^3; t=0..1",
        "reference": "analytic x(1)", "config": {
            "first_step": 0.05, "atol_over_rtol": 0.01, "jacobian": "analytic -80-3*y^2",
        }, "scope": "SciPy implementation counts and endpoint errors; no cross-host timing. "
                    "The solver object exposes accepted steps and work, not every rejection.",
        "rows": rows,
    }, indent=2))


if __name__ == "__main__":
    main()
