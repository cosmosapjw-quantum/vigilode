#!/usr/bin/env python3
"""Achieved global error of the common trajectory of each qualifying problem
of loop node V08 against an independent reference (PREDECLARED.json,
matched_error.documentation_of_the_error_level).

The reference is SciPy solve_ivp LSODA (ODEPACK) on an independent NumPy
implementation of the same equations with analytic Jacobians: rtol = 1e-12,
atol = 1e-14; its uncertainty is the max norm of the difference to a second
LSODA solve with rtol = 1e-10, atol = 1e-12. The state compared is the last
accepted state of the sequential arm, dumped by `run_measurements.sh dump`
(little-endian f64: t, then y); every arm ends in that state bit for bit.

Not a discriminator. Usage:  python3 -I reference_error.py <cell> [limit_seconds]
"""

import json
import os
import signal
import struct
import sys
import time

import numpy as np
from scipy.integrate import solve_ivp

HERE = os.path.dirname(os.path.abspath(__file__))
SOLVER_RTOL = 1.0e-6
SOLVER_ATOL = 1.0e-8
H = 0.1


def laplacian(u):
    """Second difference with zero-flux ends, divided by h^2."""
    out = np.empty_like(u)
    out[1:-1] = u[:-2] - 2.0 * u[1:-1] + u[2:]
    out[0] = u[1] - u[0]
    out[-1] = u[-2] - u[-1]
    return out / (H * H)


def nlkpp_start(n, ripple):
    x = (np.arange(n) + 0.5) * H
    length = n * H
    y0 = 0.1 + 0.8 * np.exp(-((x - 0.5 * length) ** 2) / 20.0)
    if ripple:
        y0 = y0 + 0.05 * np.sin(2.0 * np.pi * x / 12.8)
    return y0


def nlkpp_dense(n):
    sigma = 5.0
    index = np.arange(n)
    kernel = np.exp(-np.abs(index[:, None] - index[None, :]) * H / sigma) / (2.0 * sigma)
    stencil = np.zeros((n, n))
    stencil[index, index] = -2.0
    stencil[index[:-1], index[:-1] + 1] = 1.0
    stencil[index[1:], index[1:] - 1] = 1.0
    stencil[0, 0] = -1.0
    stencil[-1, -1] = -1.0
    stencil /= H * H

    def rhs(_t, u):
        return laplacian(u) + u * (1.0 - H * (kernel @ u))

    def jac(_t, u):
        return stencil + np.diag(1.0 - H * (kernel @ u)) - H * u[:, None] * kernel

    return rhs, jac, nlkpp_start(n, False), {}


def nlkpp_band(n, radius):
    rho = (radius + 0.5) * H
    shape = 1.0 - (np.arange(radius + 1) * H / rho) ** 2
    mass = H * (shape[0] + 2.0 * shape[1:].sum())
    phi = shape / mass
    window = np.concatenate([phi[:0:-1], phi])  # offsets -radius .. radius

    def conv(u):
        return np.convolve(u, window, mode="same")

    def rhs(_t, u):
        return laplacian(u) + u * (1.0 - H * conv(u))

    def jac(_t, u):
        # ODEPACK band storage: packed[upper + i - j, j] = dF_i/du_j.
        packed = np.zeros((2 * radius + 1, n))
        for offset in range(-radius, radius + 1):  # j = i + offset
            rows = np.arange(max(0, -offset), min(n, n - offset))
            packed[radius - offset, rows + offset] = -H * u[rows] * phi[abs(offset)]
        packed[radius, :] += 1.0 - H * conv(u)
        packed[radius, :] += -2.0 / (H * H)
        packed[radius, 0] += 1.0 / (H * H)
        packed[radius, -1] += 1.0 / (H * H)
        packed[radius - 1, 1:] += 1.0 / (H * H)  # dF_i/du_{i+1}
        packed[radius + 1, :-1] += 1.0 / (H * H)  # dF_i/du_{i-1}
        return packed

    return rhs, jac, nlkpp_start(n, True), {"lband": radius, "uband": radius}


def brusselator(cells):
    c = (cells + 1.0) ** 2 / 50.0
    n = 2 * cells

    def rhs(_t, y):
        u = y[0::2]
        v = y[1::2]
        ul = np.concatenate([[1.0], u[:-1]])
        ur = np.concatenate([u[1:], [1.0]])
        vl = np.concatenate([[3.0], v[:-1]])
        vr = np.concatenate([v[1:], [3.0]])
        out = np.empty(n)
        out[0::2] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur)
        out[1::2] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr)
        return out

    def jac(_t, y):
        u = y[0::2]
        v = y[1::2]
        packed = np.zeros((5, n))  # packed[2 + i - j, j]
        packed[2, 0::2] = 2.0 * u * v - 4.0 - 2.0 * c  # d f_u / d u
        packed[2, 1::2] = -u * u - 2.0 * c  # d f_v / d v
        packed[1, 1::2] = u * u  # d f_u / d v: i = 2k, j = 2k + 1
        packed[3, 0::2] = 3.0 - 2.0 * u * v  # d f_v / d u: i = 2k + 1, j = 2k
        packed[0, 2:] = c  # i = j - 2
        packed[4, :-2] = c  # i = j + 2
        return packed

    x = (np.arange(cells) + 1.0) / (cells + 1.0)
    y0 = np.empty(n)
    y0[0::2] = 1.0 + np.sin(2.0 * np.pi * x)
    y0[1::2] = 3.0
    return rhs, jac, y0, {"lband": 2, "uband": 2}


PROBLEMS = {
    "nlkpp-dense-n400": lambda: nlkpp_dense(400),
    "nlkpp-band-n4096-r24": lambda: nlkpp_band(4096, 24),
    "bruss1d-n131072": lambda: brusselator(65536),
}


class Limit(Exception):
    pass


def on_alarm(_signum, _frame):
    raise Limit()


def main():
    cell = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 900
    record = {
        "cell": cell,
        "reference": "scipy.integrate.solve_ivp LSODA, analytic Jacobian, NumPy right-hand side",
        "limit_seconds": limit,
    }
    path = os.path.join(HERE, "reference", f"final_state_{cell}.bin")
    with open(path, "rb") as handle:
        raw = handle.read()
    values = struct.unpack(f"<{len(raw) // 8}d", raw)
    t_end, state = values[0], np.array(values[1:])
    rhs, band_jac, y0, band = PROBLEMS[cell]()
    if band:
        # SciPy 1.11 passes the ODEPACK work array to the callback: it wants
        # lband extra rows below the lband + uband + 1 rows of the band
        # (as scipy's own wrapper for VODE adds them).
        padding = np.zeros((band["lband"], y0.size))

        def jac(t, y):
            return np.vstack([band_jac(t, y), padding])

    else:
        jac = band_jac
    record.update({"t_end": t_end, "n": int(state.size), "dump": os.path.relpath(path, HERE)})
    # The NumPy right-hand side against the dumped state's dimension.
    assert y0.size == state.size
    signal.signal(signal.SIGALRM, on_alarm)
    signal.alarm(limit)
    solutions = {}
    try:
        for name, rtol, atol in (("tight", 1.0e-12, 1.0e-14), ("loose", 1.0e-10, 1.0e-12)):
            start = time.monotonic()
            solution = solve_ivp(
                rhs, (0.0, t_end), y0, method="LSODA", rtol=rtol, atol=atol, jac=jac,
                t_eval=[t_end], **band,
            )
            seconds = time.monotonic() - start
            if not solution.success:
                raise RuntimeError(f"LSODA {name}: {solution.message}")
            solutions[name] = solution.y[:, -1]
            record[f"lsoda_{name}"] = {
                "rtol": rtol,
                "atol": atol,
                "rhs_evaluations": int(solution.nfev),
                "jacobian_evaluations": int(solution.njev),
                "seconds": seconds,
            }
        signal.alarm(0)
        reference = solutions["tight"]
        uncertainty = float(np.max(np.abs(reference - solutions["loose"])))
        error = np.abs(state - reference)
        scale = SOLVER_ATOL + SOLVER_RTOL * np.abs(reference)
        record.update(
            {
                "status": "computed",
                "reference_uncertainty_max_norm": uncertainty,
                "achieved_error_max_norm": float(error.max()),
                "achieved_error_in_tolerance_units_max": float((error / scale).max()),
                "achieved_error_in_tolerance_units_rms": float(
                    np.sqrt(np.mean((error / scale) ** 2))
                ),
                "solver_rtol": SOLVER_RTOL,
                "solver_atol": SOLVER_ATOL,
                "state_max_abs": float(np.max(np.abs(reference))),
                "error_over_reference_uncertainty": float(error.max() / uncertainty)
                if uncertainty > 0.0
                else None,
            }
        )
    except Limit:
        record.update(
            {
                "status": "not_computed",
                "obstacle": f"the LSODA reference did not finish within {limit} s of wall time",
                "finished_solves": sorted(solutions),
            }
        )
    except Exception as error:  # the obstacle is the result
        signal.alarm(0)
        record.update({"status": "not_computed", "obstacle": f"{type(error).__name__}: {error}"})
    print("V08_REFERENCE_JSON " + json.dumps(record, allow_nan=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
