#!/usr/bin/env python3
"""Independent reference solutions of loop node V25 by the rule of node V17
(mf_stage_solve/PREDECLARED.json, "reference"), unchanged:

  reference     SciPy DOP853 (explicit, no linear algebra), rtol 1e-13, atol 1e-13
  cross-check 1 DOP853, rtol 1e-11, atol 1e-11
  cross-check 2 Radau (implicit, sparse finite-difference Jacobian), rtol 1e-11, atol 1e-11
  fallback      if DOP853 at 1e-13 does not finish within 2 hours: Radau rtol 1e-12
                is the reference and Radau rtol 1e-10 the cross-check

on the numpy right-hand sides of families.py. The stated uncertainty u_ref
is the larger of the cross-check differences max_i |ref_i - other_i| /
(1 + |ref_i|). The final time span of a family is read from PROTOCOL.json.
Nothing here reads a Rust result.

usage: python3 -I make_reference.py --family ac3d --out reference/ac3d.json
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

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
NODE = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import families  # noqa: E402  (the module next to this script)


def final_span(family):
    with open(os.path.join(NODE, "PROTOCOL.json"), encoding="utf-8") as handle:
        protocol = json.load(handle)
    for entry in protocol["families"]:
        if entry["id"] == family:
            return entry["t_span"], "PROTOCOL.json families (final spans after the sizing pilot)"
    raise SystemExit(f"family {family} is not in PROTOCOL.json")


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
    parser.add_argument("--family", required=True, choices=families.DECLARED)
    parser.add_argument("--out", required=True)
    parser.add_argument("--reference-method", choices=["DOP853", "Radau"], default="DOP853",
                        help="Radau: the fallback (reference Radau 1e-12, cross-check Radau 1e-10)")
    args = parser.parse_args()

    rhs, _jvp, y0, pattern = families.BUILDERS[args.family]()
    span, span_source = final_span(args.family)
    print(f"family {args.family} n {y0.size} span {span} ({span_source})", flush=True)
    f0 = rhs(span[0], y0)

    checks = []
    if args.reference_method == "Radau":
        reference, info = integrate(rhs, y0, span, "Radau", 1e-12, 1e-12, pattern)
        print(f"reference {info}", flush=True)
        second, second_info = integrate(rhs, y0, span, "Radau", 1e-10, 1e-10, pattern)
        second_info["difference"] = difference(reference, second)
        print(f"cross-check 1 {second_info}", flush=True)
        checks.append(second_info)
    else:
        reference, info = integrate(rhs, y0, span, "DOP853", 1e-13, 1e-13, pattern)
        print(f"reference {info}", flush=True)
        second, second_info = integrate(rhs, y0, span, "DOP853", 1e-11, 1e-11, pattern)
        second_info["difference"] = difference(reference, second)
        print(f"cross-check 1 {second_info}", flush=True)
        checks.append(second_info)
        third, third_info = integrate(rhs, y0, span, "Radau", 1e-11, 1e-11, pattern)
        third_info["difference"] = difference(reference, third)
        print(f"cross-check 2 {third_info}", flush=True)
        checks.append(third_info)

    uncertainty = max(check["difference"]["max_scaled_1_plus_abs"] for check in checks)
    record = {
        "schema": "vigilode.loop-l1.v25.reference.v1",
        "family": args.family,
        "n": int(y0.size),
        "t_span": [float(span[0]), float(span[1])],
        "t_span_source": span_source,
        "reference": info,
        "cross_checks": checks,
        "uncertainty_max_scaled_1_plus_abs": uncertainty,
        "uncertainty_rule": "max over the cross-checks of max_i |ref_i - other_i| / (1 + |ref_i|)",
        "reference_method_rule": "V17: DOP853 rtol 1e-13 unless it does not finish within 2 hours (then Radau 1e-12, cross-check Radau 1e-10)",
        "versions": {"python": sys.version.split()[0], "numpy": np.__version__, "scipy": scipy.__version__},
        "state_summary": {"min": float(reference.min()), "max": float(reference.max()),
                          "spread_max_minus_min": float(reference.max() - reference.min())},
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
