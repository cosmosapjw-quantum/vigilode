#!/usr/bin/env python3
"""Independent reference solutions for loop node V06 (shared Krylov basis).

Each family of PREDECLARED.json is implemented here a second time, in numpy,
and integrated to its final time with two unrelated scipy integrators:

  reference    DOP853 (explicit, no linear algebra), rtol 1e-13, atol 1e-16
  cross-check  Radau (implicit, sparse finite-difference Jacobian), rtol 1e-10, atol 1e-13

The stated uncertainty of the reference is the largest componentwise
difference of the two final states. Nothing here reads a Rust result.

usage: python3 -I make_reference.py --family bruss1d --out reference/bruss1d.json
       [--reference-method DOP853|Radau] [--cross-method Radau|BDF]
"""
import argparse
import json
import struct
import sys
import time

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import lil_matrix


def bruss1d(cells, length):
    n = 2 * cells
    dx = length / (cells + 1)
    c = 0.02 / (dx * dx)
    x = (np.arange(cells) + 1.0) * dx
    y0 = np.empty(n)
    y0[0::2] = 1.0 + np.sin(2.0 * np.pi * x)
    y0[1::2] = 3.0

    def rhs(_t, y):
        u, v = y[0::2], y[1::2]
        ul = np.concatenate(([1.0], u[:-1]))
        ur = np.concatenate((u[1:], [1.0]))
        vl = np.concatenate(([3.0], v[:-1]))
        vr = np.concatenate((v[1:], [3.0]))
        out = np.empty_like(y)
        out[0::2] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur)
        out[1::2] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr)
        return out

    pattern = lil_matrix((n, n))
    for i in range(n):
        for j in range(max(0, i - 2), min(n, i + 3)):
            pattern[i, j] = 1
    return rhs, y0, (0.0, 10.0), pattern, 1.0


def grayscott2d(grid):
    cells = grid * grid
    n = 2 * cells
    inv_dx2 = float(grid * grid)
    centres = (np.arange(grid) + 0.5) / grid
    xx, yy = np.meshgrid(centres, centres)  # [j, i]
    bump = 0.25 * np.exp(-100.0 * ((xx - 0.5) ** 2 + (yy - 0.5) ** 2))
    y0 = np.concatenate(((1.0 - bump).ravel(), bump.ravel()))

    def laplace(w):
        return (np.roll(w, 1, 0) + np.roll(w, -1, 0) + np.roll(w, 1, 1) + np.roll(w, -1, 1) - 4.0 * w) * inv_dx2

    def rhs(_t, y):
        u = y[:cells].reshape(grid, grid)
        v = y[cells:].reshape(grid, grid)
        uv2 = u * v * v
        du = 2.0e-3 * laplace(u) - uv2 + 0.04 * (1.0 - u)
        dv = 1.0e-3 * laplace(v) + uv2 - (0.04 + 0.06) * v
        return np.concatenate((du.ravel(), dv.ravel()))

    pattern = lil_matrix((n, n))
    for j in range(grid):
        for i in range(grid):
            k = i + grid * j
            neighbours = [
                k,
                (i - 1) % grid + grid * j,
                (i + 1) % grid + grid * j,
                i + grid * ((j - 1) % grid),
                i + grid * ((j + 1) % grid),
            ]
            for m in neighbours:
                pattern[k, m] = 1
                pattern[cells + k, cells + m] = 1
            pattern[k, cells + k] = 1
            pattern[cells + k, k] = 1
    return rhs, y0, (0.0, 20.0), pattern, 1.0


def adr2d(grid):
    n = grid * grid
    dx = 1.0 / grid
    diffusion = 0.01 / (dx * dx)
    advection = 10.0 / dx
    centres = (np.arange(grid) + 0.5) * dx
    xx, yy = np.meshgrid(centres, centres)  # [j, i]
    y0 = (256.0 * (xx * yy * (1.0 - xx) * (1.0 - yy)) ** 2 + 0.3).ravel()

    def rhs(_t, y):
        u = y.reshape(grid, grid)
        p = np.pad(u, 1, mode="edge")
        left, right = p[1:-1, :-2], p[1:-1, 2:]
        down, up = p[:-2, 1:-1], p[2:, 1:-1]
        out = (
            diffusion * (left + right + down + up - 4.0 * u)
            - advection * ((u - left) + (u - down))
            + 100.0 * u * (u - 0.5) * (1.0 - u)
        )
        return out.ravel()

    pattern = lil_matrix((n, n))
    for j in range(grid):
        for i in range(grid):
            k = i + grid * j
            pattern[k, k] = 1
            if i > 0:
                pattern[k, k - 1] = 1
            if i + 1 < grid:
                pattern[k, k + 1] = 1
            if j > 0:
                pattern[k, k - grid] = 1
            if j + 1 < grid:
                pattern[k, k + grid] = 1
    return rhs, y0, (0.0, 0.1), pattern, 1.0


def robertson_rd1d(cells):
    n = 3 * cells
    dx = 1.0 / cells
    d = 1.0e-5 / (dx * dx)
    x = (np.arange(cells) + 0.5) * dx
    s = 0.5 * np.sin(np.pi * x) ** 2
    y0 = np.stack((1.0 - s, np.zeros(cells), s), axis=1).ravel()

    def rhs(_t, y):
        w = y.reshape(cells, 3)
        p = np.pad(w, ((1, 1), (0, 0)), mode="edge")
        lap = d * (p[:-2] + p[2:] - 2.0 * w)
        y1, y2, y3 = w[:, 0], w[:, 1], w[:, 2]
        out = np.empty_like(w)
        out[:, 0] = -0.04 * y1 + 1.0e4 * y2 * y3
        out[:, 1] = 0.04 * y1 - 1.0e4 * y2 * y3 - 3.0e7 * y2 * y2
        out[:, 2] = 3.0e7 * y2 * y2
        return (out + lap).ravel()

    pattern = lil_matrix((n, n))
    for i in range(n):
        cell = i // 3
        for j in range(3 * cell, 3 * cell + 3):
            pattern[i, j] = 1
        if cell > 0:
            pattern[i, i - 3] = 1
        if cell + 1 < cells:
            pattern[i, i + 3] = 1
    return rhs, y0, (0.0, 40.0), pattern, 1.0e-4


FAMILIES = {
    "bruss1d": lambda: bruss1d(512, 8.0),
    "grayscott2d": lambda: grayscott2d(32),
    "adr2d": lambda: adr2d(32),
    "robertson_rd1d": lambda: robertson_rd1d(342),
    "bruss1d_stiff": lambda: bruss1d(512, 1.0),
    "dev_bruss1d": lambda: bruss1d(32, 1.0),
    "dev_adr2d": lambda: adr2d(8),
    "dev_grayscott2d": lambda: grayscott2d(8),
    "dev_robertson_rd1d": lambda: robertson_rd1d(8),
}

TOLERANCES = {
    "DOP853": (1.0e-13, 1.0e-16),
    "Radau": (1.0e-10, 1.0e-13),
    "Radau-tight": (1.0e-12, 1.0e-15),
    "BDF": (1.0e-10, 1.0e-13),
}


def integrate(method, rhs, y0, span, pattern):
    rtol, atol = TOLERANCES[method]
    name = "Radau" if method == "Radau-tight" else method
    options = {"method": name, "rtol": rtol, "atol": atol, "t_eval": [span[1]]}
    if name in ("Radau", "BDF"):
        options["jac_sparsity"] = pattern.tocsc()
    start = time.time()
    solution = solve_ivp(rhs, span, y0, **options)
    elapsed = time.time() - start
    if not solution.success or solution.t[-1] != span[1]:
        raise RuntimeError(f"{method} failed: {solution.message}")
    info = {
        "method": name,
        "rtol": rtol,
        "atol": atol,
        "nfev": int(solution.nfev),
        "njev": int(solution.njev),
        "nlu": int(solution.nlu),
        "wall_seconds": elapsed,
    }
    return solution.y[:, -1].copy(), info


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--family", required=True, choices=sorted(FAMILIES))
    parser.add_argument("--out", required=True)
    parser.add_argument("--reference-method", default="DOP853", choices=["DOP853", "Radau-tight"])
    parser.add_argument("--cross-method", default="Radau", choices=["Radau", "BDF"])
    args = parser.parse_args()

    rhs, y0, span, pattern, atol_over_rtol = FAMILIES[args.family]()
    print(f"V06_REFERENCE family={args.family} n={y0.size} span={span} numpy={np.__version__} scipy={scipy.__version__}")
    sys.stdout.flush()
    reference, reference_info = integrate(args.reference_method, rhs, y0, span, pattern)
    print(f"V06_REFERENCE reference {json.dumps(reference_info)}")
    sys.stdout.flush()
    cross, cross_info = integrate(args.cross_method, rhs, y0, span, pattern)
    print(f"V06_REFERENCE cross_check {json.dumps(cross_info)}")

    difference = np.abs(reference - cross)
    uncertainty = {
        "max_abs": float(difference.max()),
        "rms": float(np.sqrt(np.mean(difference**2))),
        "max_tolerance_units": {},
    }
    for rtol in (1.0e-4, 1.0e-6):
        atol = rtol * atol_over_rtol
        units = float((difference / (atol + rtol * np.abs(reference))).max())
        uncertainty["max_tolerance_units"][f"{rtol:g}"] = units
    if not np.all(np.isfinite(reference)):
        raise RuntimeError("non-finite reference")
    record = {
        "schema": "vigilode.loop-l1.v06.reference.v1",
        "family": args.family,
        "n": int(y0.size),
        "t_span": [float(span[0]), float(span[1])],
        "atol_over_rtol": atol_over_rtol,
        "reference": reference_info,
        "cross_check": cross_info,
        "uncertainty": uncertainty,
        "state_min": float(reference.min()),
        "state_max": float(reference.max()),
        "numpy": np.__version__,
        "scipy": scipy.__version__,
        "state_hex": "".join(struct.pack(">d", value).hex() for value in reference),
    }
    with open(args.out, "w") as handle:
        json.dump(record, handle, allow_nan=False, indent=1)
        handle.write("\n")
    print(f"V06_REFERENCE uncertainty {json.dumps(uncertainty)}")
    print(f"V06_REFERENCE wrote {args.out}")


if __name__ == "__main__":
    main()
