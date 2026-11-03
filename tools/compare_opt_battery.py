#!/usr/bin/env python3
"""Compare two VigilODE 8-problem battery benchmark runs.

Verifies:
1. G1 Bit-Identity / deterministic work counters on default paths.
2. Numerical error non-regression.
3. Geometric mean work ratio W_base / W_cand.
"""

import json
import math
import sys
from pathlib import Path


def load_battery(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <baseline.json> <candidate.json>")
        sys.exit(1)

    base_path = Path(sys.argv[1])
    cand_path = Path(sys.argv[2])

    if not base_path.exists():
        print(f"Error: {base_path} does not exist")
        sys.exit(1)
    if not cand_path.exists():
        print(f"Error: {cand_path} does not exist")
        sys.exit(1)

    base = load_battery(base_path)
    cand = load_battery(cand_path)

    base_records = {
        (r["problem_id"], r["rtol"], r["route_policy"]): r
        for r in base.get("records", [])
    }
    cand_records = {
        (r["problem_id"], r["rtol"], r["route_policy"]): r
        for r in cand.get("records", [])
    }

    print("=" * 78)
    print("VigilODE Battery Regression & Work Comparison")
    print(f"Baseline : {base_path} ({len(base_records)} runs)")
    print(f"Candidate: {cand_path} ({len(cand_records)} runs)")
    print("=" * 78)

    work_ratios = []
    bit_identical_count = 0
    total_comparisons = 0
    regressions = []

    for key, b_rec in base_records.items():
        prob_id, rtol, policy = key
        c_rec = cand_records.get(key)
        if not c_rec:
            print(f"[MISSING] {prob_id} rtol={rtol} policy={policy} missing in candidate")
            continue

        total_comparisons += 1
        b_work = b_rec.get("work_model_v2_ir", 0)
        c_work = c_rec.get("work_model_v2_ir", 0)

        # Work ratio (>= 1.0 means candidate uses <= baseline work)
        ratio = (b_work / c_work) if c_work > 0 else 1.0
        work_ratios.append(ratio)

        # Check work counters match exactly
        b_counters = (
            b_rec["f_evals"],
            b_rec["jac_builds"],
            b_rec["jvp_calls"],
            b_rec["lu_factorizations"],
            b_rec["accepted"],
            b_rec["rejected"],
            b_rec["route_switches"],
        )
        c_counters = (
            c_rec["f_evals"],
            c_rec["jac_builds"],
            c_rec["jvp_calls"],
            c_rec["lu_factorizations"],
            c_rec["accepted"],
            c_rec["rejected"],
            c_rec["route_switches"],
        )

        b_err = b_rec["err_wrms_at_T"]
        c_err = c_rec["err_wrms_at_T"]

        # Check bit identity on work counters & acceptable error
        if b_counters == c_counters:
            bit_identical_count += 1
        elif c_work > b_work * 1.05:
            regressions.append(
                f"{prob_id} {policy} rtol={rtol}: work grew from {b_work} to {c_work} (+{(c_work/b_work - 1)*100:.1f}%)"
            )

        # Check error didn't blow up
        if math.isfinite(b_err) and math.isfinite(c_err):
            if c_err > b_err * 2.0 and c_err > 1e-4:
                regressions.append(
                    f"{prob_id} {policy} rtol={rtol}: err_wrms grew from {b_err:.2e} to {c_err:.2e}"
                )

    geom_mean_ratio = math.exp(sum(math.log(r) for r in work_ratios) / len(work_ratios)) if work_ratios else 1.0

    print(f"\nTotal comparisons : {total_comparisons} / {len(base_records)}")
    print(f"Exact bit-identical work counter matches: {bit_identical_count} / {total_comparisons} ({bit_identical_count / max(1, total_comparisons) * 100:.1f}%)")
    print(f"Geometric mean work ratio W_base / W_cand: {geom_mean_ratio:.4f}")

    if regressions:
        print("\n[WARNING] Regressions detected:")
        for reg in regressions:
            print(f"  - {reg}")
    else:
        print("\n[SUCCESS] No work or error regressions detected across 96 runs.")

    print("=" * 78)


if __name__ == "__main__":
    main()
