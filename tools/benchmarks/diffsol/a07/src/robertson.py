#!/usr/bin/env python3
"""Independent Robertson reference and canonical A07 output grid."""

import argparse
import hashlib
import json
import struct
import sys

import numpy as np
import scipy
from scipy.integrate import solve_ivp

START = 0.0
END = 40.0
GRID_POINTS = 101
RTOL = 1.0e-8
ATOL = [1.0e-11, 1.0e-11, 1.0e-11]
Y0 = [1.0, 0.0, 0.0]
REF_ATOL = [1.0e-14, 1.0e-24, 1.0e-14]
RADAU_RTOLS = [1.0e-10, 1.0e-12, 1.0e-13]
LSODA_RTOL = 1.0e-12


def rhs(_t, y):
    y0, y1, y2 = y
    transfer = 1.0e4 * y1 * y2
    quadratic = 3.0e7 * y1 * y1
    return [-0.04 * y0 + transfer, 0.04 * y0 - transfer - quadratic, quadratic]


def jac(_t, y):
    y0, y1, y2 = y
    del y0
    return [
        [-0.04, 1.0e4 * y2, 1.0e4 * y1],
        [0.04, -1.0e4 * y2 - 6.0e7 * y1, -1.0e4 * y1],
        [0.0, 6.0e7 * y1, 0.0],
    ]


def grid_values():
    values = [END * i / (GRID_POINTS - 1) for i in range(GRID_POINTS)]
    values[0] = START
    values[-1] = END
    return values


def bits(value):
    return struct.pack(">d", float(value)).hex()


def metric(left, right):
    scales = np.asarray(ATOL) + RTOL * np.abs(np.asarray(right))
    delta = (np.asarray(left) - np.asarray(right)) / scales
    per_time = np.sqrt(np.mean(delta * delta, axis=1))
    return float(np.max(per_time))


def run(method, rtol, grid):
    result = solve_ivp(
        rhs,
        (START, END),
        Y0,
        method=method,
        t_eval=grid,
        rtol=rtol,
        atol=REF_ATOL,
        jac=jac,
    )
    return {
        "method": method,
        "rtol": rtol,
        "atol": REF_ATOL,
        "success": bool(result.success),
        "message": str(result.message),
        "nfev": int(result.nfev),
        "njev": int(result.njev or 0),
        "nlu": int(result.nlu or 0),
        "times": [float(x) for x in result.t],
        "states": np.asarray(result.y.T, dtype=float).tolist(),
        "last_time": float(result.t[-1]) if result.t.size else None,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-grid", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    grid = grid_values()
    grid_bytes = json.dumps(grid, separators=(",", ":"), allow_nan=False).encode()
    grid_obj = {
        "schema": "vigilode-a07-grid-v1",
        "problem": "Robertson",
        "start": START,
        "end": END,
        "points": GRID_POINTS,
        "formula": "40.0 * i / 100; endpoints forced to 0.0 and 40.0",
        "times": grid,
        "binary64_be_hex": [bits(t) for t in grid],
        "sha256_json_times": hashlib.sha256(grid_bytes).hexdigest(),
    }
    with open(args.write_grid, "w", encoding="utf-8") as stream:
        json.dump(grid_obj, stream, indent=2, allow_nan=False)
        stream.write("\n")

    refs = [run("Radau", tolerance, grid) for tolerance in RADAU_RTOLS]
    lsoda = run("LSODA", LSODA_RTOL, grid)
    if not all(row["success"] and len(row["states"]) == GRID_POINTS for row in refs + [lsoda]):
        document = {
            "schema": "vigilode-a07-reference-v1",
            "status": "failed",
            "scipy_version": scipy.__version__,
            "problem": "Robertson",
            "rtol": RTOL,
            "atol": ATOL,
            "reference_atol": REF_ATOL,
            "grid_sha256": grid_obj["sha256_json_times"],
            "radau_ladder": refs,
            "lsoda_check": lsoda,
        }
        with open(args.out, "w", encoding="utf-8") as stream:
            json.dump(document, stream, indent=2, allow_nan=False)
            stream.write("\n")
        return 2

    fine = refs[-1]["states"]
    spreads = {
        "fine_vs_previous_radau_max_grid_wrms": metric(fine, refs[-2]["states"]),
        "fine_vs_lsoda_max_grid_wrms": metric(fine, lsoda["states"]),
    }
    uncertainty = max(spreads.values())
    document = {
        "schema": "vigilode-a07-reference-v1",
        "status": "ok",
        "problem": "Robertson",
        "equation": "y0'=-0.04*y0+1e4*y1*y2; y1'=0.04*y0-1e4*y1*y2-3e7*y1^2; y2'=3e7*y1^2",
        "initial_state": Y0,
        "span": [START, END],
        "solver_rtol": RTOL,
        "solver_atol": ATOL,
        "reference_atol": REF_ATOL,
        "scipy_version": scipy.__version__,
        "grid_sha256": grid_obj["sha256_json_times"],
        "radau_ladder": refs,
        "lsoda_check": lsoda,
        "reference_center": fine,
        "reference_spreads_empirical_not_certified": spreads,
        "reference_uncertainty_empirical_not_certified": uncertainty,
        "reference_stability_threshold": 0.05,
    }
    with open(args.out, "w", encoding="utf-8") as stream:
        json.dump(document, stream, indent=2, allow_nan=False)
        stream.write("\n")
    return 0 if uncertainty <= 0.05 else 3


if __name__ == "__main__":
    sys.exit(main())
