#!/usr/bin/env python3
"""Gate of research node research/spd09_colext_threshold_20261007 (speed node SPD09).

The new arm `rodas5p-fast-colext64` (column-extent LU iff n > RODAS5P_FAST_SMALL_LU_MAX = 64, the legacy
LU otherwise) against `rodas5p-fast`, same binary, rtol 1e-6.
G1 identity: bitwise on all 37 points (the five benchmark problems x rtol 1e-3..1e-9, and
   brusselator-1d-30 and brusselator-1d-40 at rtol 1e-6; output states, step counts, reuses, clipped
   steps, counters).
G2 no loss at or below the threshold: Ir per attempt ratio <= 1.005 on van der Pol, Robertson, HIRES and
   brusselator-1d-30 (n = 60).
G3 gain above it: ratio <= 0.85 on brusselator-1d-40 (n = 80), <= 0.80 on brusselator-1d-50 (n = 100)
   and <= 0.60 on brusselator-1d-200 (n = 400).
Reported, not gated: the always-on `rodas5p-fast-colext` arm (especially on the two new sizes, where
the crossover lies), the legacy arm's drift against SPD01's BASE_PROFILE.json, and the callgrind
determinism flags.
Counted instructions only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_BASE = HERE.parent / "research/spd01_fast_driver_overhead_20261005/BASE_PROFILE.json"

LEGACY = "rodas5p-fast"
NEW = "rodas5p-fast-colext64"
ALWAYS = "rodas5p-fast-colext"
BENCHMARK = ["robertson", "hires", "van-der-pol-mu1000", "brusselator-1d-50", "brusselator-1d-200"]
NEW_SIZES = ["brusselator-1d-30", "brusselator-1d-40"]
TOLERANCES = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9]
NO_LOSS = {"van-der-pol-mu1000": 1.005, "robertson": 1.005, "hires": 1.005, "brusselator-1d-30": 1.005}
GAIN = {"brusselator-1d-40": 0.85, "brusselator-1d-50": 0.80, "brusselator-1d-200": 0.60}
PROFILED = ["van-der-pol-mu1000", "robertson", "hires", "brusselator-1d-30", "brusselator-1d-40",
            "brusselator-1d-50", "brusselator-1d-200"]


def entries(profile):
    return {(e["arm"], e["problem"]): e for e in profile["entries"] if e["success"]}


def same_work(a, b):
    return (a["attempts"] == b["attempts"] and a["accepted_steps"] == b["accepted_steps"]
            and a["rejected_steps"] == b["rejected_steps"] and a["counters"] == b["counters"]
            and a["final_state_sha256"] == b["final_state_sha256"])


def ratio_row(new, arm, problem):
    a = new.get((arm, problem))
    r = new.get((LEGACY, problem))
    if a is None or r is None:
        return {"arm": arm, "problem": problem, "missing": True}
    return {"arm": arm, "problem": problem, "attempts": r["attempts"],
            "legacy_ir_per_attempt": r["ir_per_attempt"], "arm_ir_per_attempt": a["ir_per_attempt"],
            "ratio": a["ir_per_attempt"] / r["ir_per_attempt"], "same_work": same_work(a, r),
            "deterministic": a["callgrind_deterministic"] and r["callgrind_deterministic"]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", type=Path, default=DEFAULT_BASE, help="SPD01 BASE_PROFILE.json (drift, reported)")
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--identity", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base_profile = json.loads(args.base.read_text())
    base = entries(base_profile)
    profile = json.loads(args.profile.read_text())
    new = entries(profile)
    identity = json.loads(args.identity.read_text())

    # G1.
    g1_rows = [{"problem": r["problem"], "dimension": r["dimension"], "rtol": r["rtol"],
                "identical": r["identical"], "legacy_lu": r["legacy_lu"], "colext64_lu": r["colext64_lu"]}
               for r in identity["rows"]]
    points = sorted((r["problem"], r["rtol"]) for r in g1_rows)
    expected = sorted([(p, t) for p in BENCHMARK for t in TOLERANCES] + [(p, 1.0e-6) for p in NEW_SIZES])
    lu_as_designed = all((r["colext64_lu"] == "InPlaceColumnExtents") == (r["dimension"] > identity["threshold"])
                         for r in g1_rows)
    g1 = points == expected and len(g1_rows) == 37 and all(r["identical"] for r in g1_rows)

    # G2 and G3.
    g2_rows, g3_rows = [], []
    g2 = g3 = profile["rtol"] == 1.0e-6
    for problem, limit in NO_LOSS.items():
        row = ratio_row(new, NEW, problem)
        row["threshold"] = limit
        row["ok"] = not row.get("missing") and row["ratio"] <= limit
        g2 &= row["ok"]
        g2_rows.append(row)
    for problem, limit in GAIN.items():
        row = ratio_row(new, NEW, problem)
        row["threshold"] = limit
        row["ok"] = not row.get("missing") and row["ratio"] <= limit
        g3 &= row["ok"]
        g3_rows.append(row)

    # Reported: always-on colext, legacy drift against the base export, determinism.
    always = [ratio_row(new, ALWAYS, p) for p in PROFILED]
    drift = []
    for problem in PROFILED:
        b = base.get((LEGACY, problem))
        n = new.get((LEGACY, problem))
        if b is None or n is None:
            drift.append({"problem": problem, "in_base": b is not None, "in_profile": n is not None})
            continue
        drift.append({"problem": problem, "same_work_and_final_state": same_work(n, b),
                      "base_ir_per_attempt": b["ir_per_attempt"], "new_ir_per_attempt": n["ir_per_attempt"],
                      "drift_ratio": n["ir_per_attempt"] / b["ir_per_attempt"]})
    deterministic = all(e["callgrind_deterministic"] for e in profile["entries"] if e["success"])
    ir_table = {p: {arm: new[(arm, p)]["ir_per_attempt"] for arm in (LEGACY, ALWAYS, NEW) if (arm, p) in new}
                for p in PROFILED}

    gate = {"g1_identity": g1, "g2_no_loss_at_or_below_64": g2, "g3_gain_above_64": g3}
    result = {
        "schema": "vigilode-spd09-check-v1",
        "gate": gate,
        "g1": {"points_as_registered": points == expected, "lu_choice_as_designed": lu_as_designed,
               "rows": g1_rows},
        "g2_rows": g2_rows, "g3_rows": g3_rows,
        "reported_always_on_colext": always,
        "reported_legacy_drift_against_base": drift,
        "ir_per_attempt": ir_table,
        "validity": {"all_callgrind_deterministic": deterministic, "profile_rtol": profile["rtol"]},
        "binary_sha256": profile["binary_sha256"], "valgrind": profile["valgrind"],
        "base_binary_sha256": base_profile["binary_sha256"],
    }
    result["verdict"] = "PASS" if all(gate.values()) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(gate), result["verdict"])
    for r in g2_rows + g3_rows:
        if not r.get("missing"):
            print(f"{r['problem']:20s} legacy {r['legacy_ir_per_attempt']:12.1f} colext64 {r['arm_ir_per_attempt']:12.1f} "
                  f"ratio {r['ratio']:.4f} (<= {r['threshold']})")
        else:
            print(f"{r['problem']:20s} MISSING")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
