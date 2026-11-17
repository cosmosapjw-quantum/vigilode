#!/usr/bin/env python3
"""Independent reference solutions for loop node V17 (matrix-free stage solve).

Each family of PREDECLARED.json is implemented here a second time, in numpy,
and integrated to its final time with scipy:

  reference     DOP853 (explicit, no linear algebra), rtol 1e-13, atol 1e-13
  cross-check 1 DOP853, rtol 1e-11, atol 1e-11
  cross-check 2 Radau (implicit, sparse finite-difference Jacobian), rtol 1e-11, atol 1e-11

The stated uncertainty of the reference is the larger of the two largest
componentwise differences |ref_i - other_i| / (1 + |ref_i|). Nothing here
reads a Rust result. The time span of a family is read from PREDECLARED.json
("families_final", amendment A1).

usage: python3 -I make_reference.py --family fhn3d --out reference/fhn3d.json
"""
import argparse
import json
import os
import struct
import sys
import time

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import lil_matrix

HERE = os.path.dirname(os.path.abspath(__file__))


def fhn3d(m):
    cells = m ** 3
    n = 2 * cells
    du, dv = 0.1, 0.02
    scale = float(m * m)
    centres = (np.arange(m) + 0.5) / m
    zz, yy, xx = np.meshgrid(centres, centres, centres, indexing="ij")  # [k, j, i]
    drive = 0.5 + 0.4 * np.cos(np.pi * xx) * np.cos(2.0 * np.pi * yy) * np.cos(np.pi * zz)
    u0 = -1.2 + 0.6 * np.cos(np.pi * xx) * np.cos(np.pi * yy)
    v0 = -0.6 + 0.3 * np.cos(np.pi * zz)
    y0 = np.concatenate((u0.ravel(), v0.ravel()))

    def laplace(w):
        p = np.pad(w, 1, mode="edge")
        neighbours = (p[1:-1, 1:-1, :-2] + p[1:-1, 1:-1, 2:] + p[1:-1, :-2, 1:-1]
                      + p[1:-1, 2:, 1:-1] + p[:-2, 1:-1, 1:-1] + p[2:, 1:-1, 1:-1])
        return scale * (neighbours - 6.0 * w)

    def rhs(_t, y):
        u = y[:cells].reshape(m, m, m)
        v = y[cells:].reshape(m, m, m)
        out_u = du * laplace(u) + u - u ** 3 / 3.0 - v + drive
        out_v = dv * laplace(v) + 0.08 * (u + 0.7 - 0.8 * v)
        return np.concatenate((out_u.ravel(), out_v.ravel()))

    pattern = lil_matrix((n, n))
    for k in range(m):
        for j in range(m):
            for i in range(m):
                c = i + m * (j + m * k)
                around = [c]
                if i > 0:
                    around.append(c - 1)
                if i + 1 < m:
                    around.append(c + 1)
                if j > 0:
                    around.append(c - m)
                if j + 1 < m:
                    around.append(c + m)
                if k > 0:
                    around.append(c - m * m)
                if k + 1 < m:
                    around.append(c + m * m)
                for block in (0, cells):
                    for q in around:
                        pattern[block + c, block + q] = 1
                    pattern[block + c, (cells - block) + c] = 1
    return rhs, y0, pattern


def cusp1d(cells):
    n = 3 * cells
    inv_eps = 1.0e4
    dc = cells * cells / 144.0
    angle = 2.0 * np.pi * (np.arange(cells) + 1.0) / cells
    y0 = np.empty(n)
    y0[0::3] = 0.0
    y0[1::3] = -2.0 * np.cos(angle)
    y0[2::3] = 2.0 * np.sin(angle)

    def laplace(w):
        return dc * (np.roll(w, 1) - 2.0 * w + np.roll(w, -1))

    def rhs(_t, w):
        y, a, b = w[0::3], w[1::3], w[2::3]
        u = (y - 0.7) * (y - 1.3)
        v = u / (u + 0.1)
        out = np.empty_like(w)
        out[0::3] = -(y ** 3 + a * y + b) * inv_eps + laplace(y)
        out[1::3] = b + 0.07 * v + laplace(a)
        out[2::3] = (1.0 - a * a) * b - a - 0.4 * y + 0.035 * v + laplace(b)
        return out

    pattern = lil_matrix((n, n))
    for i in range(cells):
        for s in range(3):
            row = 3 * i + s
            for t in range(3):
                pattern[row, 3 * i + t] = 1
            pattern[row, 3 * ((i - 1) % cells) + s] = 1
            pattern[row, 3 * ((i + 1) % cells) + s] = 1
    return rhs, y0, pattern


def burgers2d(m, nu=0.05, u_mean=4.0):
    cells = m * m
    n = 2 * cells
    half_inv_dx = 0.5 * m
    diffusion = nu * m * m
    centres = (np.arange(m) + 0.5) / m
    yy, xx = np.meshgrid(centres, centres, indexing="ij")  # [j, i]
    u0 = np.sin(2.0 * np.pi * xx) * np.cos(2.0 * np.pi * yy) + u_mean
    v0 = -np.cos(2.0 * np.pi * xx) * np.sin(2.0 * np.pi * yy) + 0.25 * np.sin(2.0 * np.pi * (xx + yy))
    y0 = np.concatenate((u0.ravel(), v0.ravel()))

    def stencil(w):
        east, west = np.roll(w, -1, 1), np.roll(w, 1, 1)
        north, south = np.roll(w, -1, 0), np.roll(w, 1, 0)
        return (half_inv_dx * (east - west), half_inv_dx * (north - south),
                diffusion * (east + west + north + south - 4.0 * w))

    def rhs(_t, y):
        u = y[:cells].reshape(m, m)
        v = y[cells:].reshape(m, m)
        ux, uy, ulap = stencil(u)
        vx, vy, vlap = stencil(v)
        du = -(u * ux + v * uy) + ulap
        dv = -(u * vx + v * vy) + vlap
        return np.concatenate((du.ravel(), dv.ravel()))

    pattern = lil_matrix((n, n))
    for j in range(m):
        for i in range(m):
            c = i + m * j
            around = [c, (i - 1) % m + m * j, (i + 1) % m + m * j,
                      i + m * ((j - 1) % m), i + m * ((j + 1) % m)]
            for block in (0, cells):
                for q in around:
                    pattern[block + c, block + q] = 1
                # the other velocity component enters without derivatives
                pattern[block + c, (cells - block) + c] = 1
    return rhs, y0, pattern


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
    return rhs, y0, pattern


BUILDERS = {
    "fhn3d": lambda: fhn3d(10),
    "cusp1d": lambda: cusp1d(400),
    "burgers2d": lambda: burgers2d(28),
    "bruss1d": lambda: bruss1d(700, 8.0),
    # exploratory, not a declared family
    "bruss1d_stiff600": lambda: bruss1d(600, 1.0),
    # reduced instances used only to test this script against the Rust code
    "dev_fhn3d": lambda: fhn3d(5),
    "dev_cusp1d": lambda: cusp1d(32),
    "dev_burgers2d": lambda: burgers2d(8),
    "dev_bruss1d": lambda: bruss1d(48, 8.0),
}
EXPLORATORY_SPANS = {"bruss1d_stiff600": [0.0, 10.0]}


def declared_span(family):
    base = family[4:] if family.startswith("dev_") else family
    if base in EXPLORATORY_SPANS:
        return EXPLORATORY_SPANS[base], "exploratory cell"
    with open(os.path.join(HERE, "..", "PREDECLARED.json"), encoding="utf-8") as handle:
        declared = json.load(handle)
    for entry in declared["families_final"]["list"]:
        if entry["id"] == base:
            return entry["t_span"], "PREDECLARED.json families_final (amendment A1)"
    raise SystemExit(f"family {family} is not in PREDECLARED.json")


def hex_bits(values):
    return "".join(format(struct.unpack("<Q", struct.pack("<d", float(v)))[0], "016x") for v in values)


def integrate(rhs, y0, span, method, rtol, atol, pattern):
    calls = [0]

    def counted(t, y):
        calls[0] += 1
        return rhs(t, y)

    options = {"method": method, "rtol": rtol, "atol": atol, "t_eval": [span[1]]}
    if method in ("Radau", "BDF"):
        options["jac_sparsity"] = pattern.tocsc()
    start = time.time()
    solution = solve_ivp(counted, tuple(span), y0, **options)
    wall = time.time() - start
    if not solution.success:
        raise SystemExit(f"{method} failed: {solution.message}")
    return solution.y[:, -1], {
        "method": method, "rtol": rtol, "atol": atol, "rhs_evaluations": calls[0],
        "nfev": int(solution.nfev), "njev": int(solution.njev), "nlu": int(solution.nlu),
        "wall_seconds": wall,
    }


def difference(reference, other):
    absolute = np.abs(reference - other)
    return {
        "max_abs": float(absolute.max()),
        "max_scaled_1_plus_abs": float((absolute / (1.0 + np.abs(reference))).max()),
        "rms": float(np.sqrt(np.mean(absolute ** 2))),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--family", required=True, choices=sorted(BUILDERS))
    parser.add_argument("--out", required=True)
    parser.add_argument("--reference-rtol", type=float, default=1e-13)
    parser.add_argument("--skip-radau", action="store_true")
    args = parser.parse_args()

    rhs, y0, pattern = BUILDERS[args.family]()
    span, span_source = declared_span(args.family)
    print(f"family {args.family} n {y0.size} span {span} ({span_source})", flush=True)
    f0 = rhs(span[0], y0)

    reference, info = integrate(rhs, y0, span, "DOP853", args.reference_rtol, args.reference_rtol, pattern)
    print(f"reference {info}", flush=True)
    checks = []
    second, second_info = integrate(rhs, y0, span, "DOP853", 1e-11, 1e-11, pattern)
    second_info["difference"] = difference(reference, second)
    print(f"cross-check 1 {second_info}", flush=True)
    checks.append(second_info)
    if not args.skip_radau:
        third, third_info = integrate(rhs, y0, span, "Radau", 1e-11, 1e-11, pattern)
        third_info["difference"] = difference(reference, third)
        print(f"cross-check 2 {third_info}", flush=True)
        checks.append(third_info)

    uncertainty = max(check["difference"]["max_scaled_1_plus_abs"] for check in checks)
    record = {
        "schema": "vigilode.loop-l1.v17.reference.v1",
        "family": args.family,
        "n": int(y0.size),
        "t_span": [float(span[0]), float(span[1])],
        "t_span_source": span_source,
        "reference": info,
        "cross_checks": checks,
        "uncertainty_max_scaled_1_plus_abs": uncertainty,
        "uncertainty_rule": "max over the cross-checks of max_i |ref_i - other_i| / (1 + |ref_i|)",
        "radau_skipped": bool(args.skip_radau),
        "versions": {"python": sys.version.split()[0], "numpy": np.__version__, "scipy": scipy.__version__},
        "y0_hex": hex_bits(y0),
        "f0_hex": hex_bits(f0),
        "state_hex": hex_bits(reference),
    }
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=1, allow_nan=False)
        handle.write("\n")
    print(f"wrote {args.out}: uncertainty {uncertainty:.3e}", flush=True)


if __name__ == "__main__":
    main()
