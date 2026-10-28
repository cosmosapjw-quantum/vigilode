#!/usr/bin/env python3
"""Regenerate every number in ADDENDUM_20260928_OUTPUT_POLICY_AND_GLOBAL_ERROR.md.

Inputs are the adversarial audit's committed experiment records only:
  research/adversarial_audit_20260927/experiments/E-02/results.json
  research/adversarial_audit_20260927/experiments/E-03/results.json
E-02 re-ran the 18 n=96 v2 rows and reproduced the committed ab8fbcd arms
bitwise (36/36 checksums).  E-03 is the SciPy Radau control on the same
problems, at the same (rtol, atol), with an analytic Jacobian.

The campaign's tight basis is (atol 1e-10, rtol 1e-8) anchored on the
reference and the case basis is (0.01 rtol, rtol), so the two weight tables
are proportional and case-tolerance WRMS = tight WRMS * 1e-8 / rtol.
Standard library only; prints one KEY=VALUE line per number.
"""

import json
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXP = ROOT / "research/adversarial_audit_20260927/experiments"


def rtol_of(case_id: str) -> float:
    return float(case_id.split("-rtol-")[1].split("-v2")[0])


def main() -> int:
    e02 = json.loads((EXP / "E-02/results.json").read_text())
    e03 = json.loads((EXP / "E-03/results.json").read_text())
    rows = e02["rows"]
    out = {}

    out["e02_rows"] = len(rows)
    out["e02_arm_checksums_bitwise"] = e02["summary_metrics"]["arm_checksum_bitwise_equal"]
    dense_case = {}
    for row in rows:
        scale = 1.0e-8 / row["rtol"]
        dense_case[row["case_id"]] = row["dense"]["max_grid_wrms"]["committed"] * scale
    out["dense_case_wrms_gt_1"] = sum(v > 1.0 for v in dense_case.values())
    worst = max(dense_case, key=dense_case.get)
    out["dense_case_wrms_max"] = f"{dense_case[worst]:.4g}"
    out["dense_case_wrms_max_case"] = worst
    semilinear = sorted(
        (rtol_of(k), v) for k, v in dense_case.items() if k.startswith("semilinear")
    )
    out["dense_case_wrms_semilinear_by_rtol"] = ",".join(
        f"{r:g}:{v:.3g}" for r, v in semilinear
    )
    accepted = [
        row[arm]["solver_max_accepted_error_norm"] for row in rows for arm in ("clipped", "dense")
    ]
    out["arms_with_max_accepted_embedded_norm_le_1"] = f"{sum(a <= 1.0 for a in accepted)}/{len(accepted)}"

    scipy = {r["case_id"]: r for r in e03["e03c_scipy_rows"]}
    ratios = []
    for row in rows:
        s = scipy[row["case_id"]]
        ratios.append(row["dense"]["max_grid_wrms"]["committed"] / s["max_grid_wrms_tight"])
    out["vigilode_over_scipy_rows"] = len(ratios)
    out["vigilode_worse_than_scipy"] = sum(r > 1.0 for r in ratios)
    out["vigilode_over_scipy_median"] = f"{statistics.median(ratios):.3g}"
    out["vigilode_over_scipy_max"] = f"{max(ratios):.4g}"
    scipy_case = [s["max_grid_wrms_tight"] * 1.0e-8 / rtol_of(k) for k, s in scipy.items()]
    out["scipy_case_wrms_max"] = f"{max(scipy_case):.3g}"
    out["scipy_case_wrms_gt_1"] = sum(v > 1.0 for v in scipy_case)

    for key in ("A_vs_B_first_step", "A_vs_D_maxstep_grid"):
        m = e03["summary_metrics"]["E03b_scipy_radau_control"][key]
        out[f"scipy_{key}_0p1_violations"] = (
            f"{m['violations_among_both_correct']}/{m['both_reference_admissible']}"
        )
        out[f"scipy_{key}_median_ratio"] = f"{m['median_ratio']:.3g}"
        # Recount from the per-pair rows rather than trusting the summary.
        pairs = [
            p for p in e03["e03b_pairs"][key] if p["both_reference_admissible"]
        ]
        recount = sum(p["gap_wrms_tight"] > 0.1 * p["denominator_max_grid_wrms"] for p in pairs)
        out[f"scipy_{key}_0p1_violations_recount"] = f"{recount}/{len(pairs)}"

    for key, value in out.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
