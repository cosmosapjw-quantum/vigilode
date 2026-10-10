#!/usr/bin/env python3
"""Independent exact-binary oracle; deliberately does not import native interval code."""
import json
import math
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent


def number(x):
    if isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x):
        raise ValueError(f"not a finite JSON number: {x!r}")
    return F(x)


def verify_cell(row):
    w = row["W"]
    if len(w) != 2 or any(len(r) != 2 for r in w):
        raise ValueError("W shape")
    if any(len(row[k]) != 2 for k in ("b", "x", "scales")):
        raise ValueError("vector shape")
    a, b, c, d = [number(v) for r in w for v in r]
    rhs = list(map(number, row["b"]))
    x = list(map(number, row["x"]))
    s = list(map(number, row["scales"]))
    if min(s) <= 0:
        raise ValueError("nonpositive scale")
    det = a*d-b*c
    if not det:
        raise ValueError("singular positive cell")
    inv = [[d/det, -b/det], [-c/det, a/det]]
    exact = [sum(inv[i][j]*rhs[j] for j in range(2)) for i in range(2)]
    correction = [(exact[i]-x[i])/s[i] for i in range(2)]
    error_sq = sum(v*v for v in correction)/2
    cert = row.get("certificate", {})
    upper = number(cert["wrms_upper"])
    inf_upper = number(cert["weighted_inf_upper"])
    intervals = cert["weighted_correction_intervals"]
    if len(intervals) != 2 or any(len(z) != 2 for z in intervals):
        raise ValueError("interval shape")
    enclosed_components = all(number(z[0]) <= e <= number(z[1]) for z, e in zip(intervals, correction))
    inverse_norm = max(sum(abs(inv[i][j])*s[j]/s[i] for j in range(2)) for i in range(2))
    gain = number(cert["scaled_residual_gain_inf_upper"])
    passed = (row["status"] == "admitted" and upper >= 0 and inf_upper >= 0
              and upper*upper >= error_sq and inf_upper >= max(map(abs, correction))
              and enclosed_components and gain >= inverse_norm)
    return {"id": row["id"], "partition": row["partition"], "pass": passed,
            "component_enclosure": enclosed_components,
            "exact_wrms_error_squared": str(error_sq),
            "wrms_upper": float(upper), "inverse_gain_exact": str(inverse_norm),
            "inverse_gain_enclosed": gain >= inverse_norm,
            "bound_over_error": float(upper)/math.sqrt(float(error_sq)) if error_sq else None,
            "zero_exact_error": error_sq == 0}


def main():
    raw = json.loads((HERE/"NATIVE.json").read_text())
    study = raw["gain_study"]
    cells = study["cells"]
    if len(cells) != 30 or len({r["id"] for r in cells}) != len(cells):
        raise ValueError("expected 30 unique registered cells")
    checks = [verify_cell(r) for r in cells]
    invalid = study["invalid_controls"]
    invalid_pass = len(invalid) >= 8 and all(r["status"] == "rejected" for r in invalid)
    partitions = {p: sum(r["partition"] == p for r in cells) for p in {r["partition"] for r in cells}}
    if partitions != {"discovery": 24, "control": 2, "fixed_holdout": 4}:
        raise ValueError("registered partition mismatch")
    all_pass = all(r["pass"] for r in checks) and invalid_pass
    result = {"schema_version": "1.0", "verdict": "PASS" if all_pass else "FAIL",
              "claim_ceiling": "represented-real-2x2-linear-solve-research-prototype-only",
              "production_promotion": "HOLD", "global_accuracy": "NOT_TESTED",
              "speed": "NOT_MEASURED", "oracle": "Python Fraction exact binary64 rational 2x2 inversion",
              "numeric_summary": {"cells": len(cells), "passed_cells": sum(r["pass"] for r in checks),
                                  "invalid_controls": len(invalid),
                                  "rejected_invalid_controls": sum(r["status"] == "rejected" for r in invalid),
                                  "partitions": partitions},
              "checks": checks, "invalid_controls_pass": invalid_pass}
    (HERE/"RESULTS.json").write_text(json.dumps(result, indent=2, allow_nan=False)+"\n")
    print(json.dumps({"verdict": result["verdict"], **result["numeric_summary"]}))
    return 0 if all_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
