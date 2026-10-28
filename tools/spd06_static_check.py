#!/usr/bin/env python3
"""Gate of research node research/spd06_small_static_stages_20261007 (speed node SPD06).

G1 identity: the gated arm `rodas5p-fast-small-static` (fixed stages and index-loop solves) equals the
   L-0041 driver bitwise on all 21 identity points (van der Pol, Robertson, HIRES x rtol 1e-3..1e-9:
   output states, step counts, reuses, clipped steps, counters) and on the 64-member van der Pol
   ensemble (`rodas5p stiff-ensemble-run` of `rodas5p-fast-small` and `rodas5p-fast-small-static`:
   the same attempts and the same checksum bit for bit).
G2 tables: the fixed tables equal rodas5p_coefficients() bit for bit (recorded in IDENTITY.json by the
   export, which runs the construction-time check; plus the outcome of the non-ignored contract tests
   `cargo test --test spd06_small_static_stages`, given on the command line).
G3 instructions (same binary, rtol 1e-6, Ir per attempt gated arm / rodas5p-fast-small): <= 0.90 on
   van der Pol, <= 0.93 on Robertson, <= 0.97 on HIRES. Each threshold is the PASS and the kill line.
G4 legacy reproduction: rodas5p-fast-small of the new binary reproduces SPD01's BASE_PROFILE.json in
   work (attempts, accepted/rejected, counters) and final state (hash); its Ir per attempt drift
   against the base is reported, not gated.
Reported, not gated: the secondary arms (-static-stages, -static-ovh), the solves-only effect, the
Robertson profile at rtol 1e-8, the predicted ranges, the by-line attribution of
rodas5p_fast_small.rs, and the callgrind determinism flags.
Counted instructions only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_BASE = HERE.parent / "research/spd01_fast_driver_overhead_20261005/BASE_PROFILE.json"

LEGACY = "rodas5p-fast-small"
GATED = "rodas5p-fast-small-static"
STAGES_ONLY = "rodas5p-fast-small-static-stages"
STATIC_OVH = "rodas5p-fast-small-static-ovh"
LEGACY_OVH = "rodas5p-fast-small-ovh"
PROBLEMS = ["van-der-pol-mu1000", "robertson", "hires"]
TOLERANCES = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9]
THRESHOLDS = {"van-der-pol-mu1000": 0.90, "robertson": 0.93, "hires": 0.97}
PREDICTED = {"van-der-pol-mu1000": (0.85, 0.90), "robertson": (0.88, 0.93), "hires": (0.95, 0.96)}
ENSEMBLE_MEMBERS = 64


def entries(profile):
    return {(e["arm"], e["problem"]): e for e in profile["entries"] if e["success"]}


def bits(x: float) -> str:
    return struct.pack(">d", float(x)).hex()


def same_work(a, b):
    return (a["attempts"] == b["attempts"] and a["accepted_steps"] == b["accepted_steps"]
            and a["rejected_steps"] == b["rejected_steps"] and a["counters"] == b["counters"]
            and a["final_state_sha256"] == b["final_state_sha256"])


def ratio_row(new, arm, reference, problem):
    a = new.get((arm, problem))
    r = new.get((reference, problem))
    if a is None or r is None:
        return {"arm": arm, "reference": reference, "problem": problem, "missing": True}
    return {"arm": arm, "reference": reference, "problem": problem, "attempts": r["attempts"],
            "arm_ir_per_attempt": a["ir_per_attempt"], "reference_ir_per_attempt": r["ir_per_attempt"],
            "ratio": a["ir_per_attempt"] / r["ir_per_attempt"],
            "difference_per_attempt": a["ir_per_attempt"] - r["ir_per_attempt"],
            "same_work": same_work(a, r),
            "deterministic": a["callgrind_deterministic"] and r["callgrind_deterministic"]}


def lines_of(entry, file_suffix, limit=25):
    return [l for l in entry["top_lines"] if l["file"].endswith(file_suffix)][:limit]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", type=Path, default=DEFAULT_BASE, help="SPD01 BASE_PROFILE.json")
    parser.add_argument("--profile", required=True, type=Path, help="PROFILE.json (rtol 1e-6, three problems)")
    parser.add_argument("--profile-rob8", required=True, type=Path, help="PROFILE_ROB8.json (Robertson, rtol 1e-8)")
    parser.add_argument("--identity", required=True, type=Path, help="IDENTITY.json of the SPD06 export")
    parser.add_argument("--ensemble", required=True, type=Path, nargs=2, metavar=("JSON", "JSON"),
                        help="stiff-ensemble-run outputs of rodas5p-fast-small and rodas5p-fast-small-static")
    parser.add_argument("--contract-passed", required=True, choices=["true", "false"],
                        help="outcome of `cargo test --release -p rodas5p-integrators --locked --test spd06_small_static_stages`")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base_profile = json.loads(args.base.read_text())
    profile = json.loads(args.profile.read_text())
    rob8 = json.loads(args.profile_rob8.read_text())
    identity = json.loads(args.identity.read_text())
    ensembles = {}
    for path in args.ensemble:
        e = json.loads(path.read_text())
        if e["arm"] in ensembles:
            raise SystemExit(f"two ensemble files for arm {e['arm']}")
        ensembles[e["arm"]] = e
    base = entries(base_profile)
    new = entries(profile)
    new8 = entries(rob8)

    # G1: identity of the gated arm on the 21 points and the ensemble.
    g1_rows = []
    for r in identity["rows"]:
        g1_rows.append({"problem": r["problem"], "rtol": r["rtol"],
                        **{arm: v["identical"] for arm, v in r["arms"].items()}})
    points = sorted((r["problem"], r["rtol"]) for r in identity["rows"])
    expected_points = sorted((p, t) for p in PROBLEMS for t in TOLERANCES)
    points_ok = points == expected_points
    g1_points = points_ok and len(g1_rows) == 21 and all(r.get(GATED) is True for r in g1_rows)
    leg_e, sta_e = ensembles.get(LEGACY), ensembles.get(GATED)
    ensemble = {"present": leg_e is not None and sta_e is not None}
    if ensemble["present"]:
        ensemble.update({
            "members": [leg_e["members"], sta_e["members"]], "rtol": [leg_e["rtol"], sta_e["rtol"]],
            "attempts": [leg_e["attempts"], sta_e["attempts"]],
            "checksum": [leg_e["checksum"], sta_e["checksum"]],
            "checksum_bits": [bits(leg_e["checksum"]), bits(sta_e["checksum"])],
        })
        ensemble["identical"] = (leg_e["members"] == sta_e["members"] == ENSEMBLE_MEMBERS
                                 and leg_e["rtol"] == sta_e["rtol"]
                                 and leg_e["attempts"] == sta_e["attempts"]
                                 and bits(leg_e["checksum"]) == bits(sta_e["checksum"]))
    else:
        ensemble["identical"] = False
    g1 = g1_points and ensemble["identical"]
    per_arm_identity = identity.get("all_identical", {})
    solves_dropped = (not per_arm_identity.get(GATED, False)) and per_arm_identity.get(STAGES_ONLY, False)

    # G2: tables.
    tables = identity["tables"]
    g2_tables = (tables["bitwise_equal"] is True and not tables["mismatches"]
                 and tables["zero_strictly_lower_entries"] == 0 and tables["zero_b_code_entries"] == 0
                 and tables["stages"] == 8)
    g2 = g2_tables and args.contract_passed == "true"

    # G3: instructions of the gated arm against the L-0041 arm, same binary, rtol 1e-6.
    g3_rows = []
    g3 = profile["rtol"] == 1.0e-6
    for problem in PROBLEMS:
        row = ratio_row(new, GATED, LEGACY, problem)
        if row.get("missing"):
            row["ok"] = False
        else:
            row["threshold"] = THRESHOLDS[problem]
            row["ok"] = row["ratio"] <= THRESHOLDS[problem]
            lo, hi = PREDICTED[problem]
            row["predicted"] = [lo, hi]
            row["in_predicted_range"] = lo <= row["ratio"] <= hi
        g3 &= row["ok"]
        g3_rows.append(row)

    # G4: legacy reproduction of the base export (work and final state); drift reported.
    g4_rows = []
    g4 = True
    for problem in PROBLEMS:
        b = base.get((LEGACY, problem))
        n = new.get((LEGACY, problem))
        if b is None or n is None:
            g4 = False
            g4_rows.append({"problem": problem, "missing": True})
            continue
        same = same_work(n, b)
        g4 &= same
        g4_rows.append({"problem": problem, "same_work_and_final_state": same,
                        "base_ir_per_attempt": b["ir_per_attempt"], "new_ir_per_attempt": n["ir_per_attempt"],
                        "drift_ratio_reported": n["ir_per_attempt"] / b["ir_per_attempt"]})

    # Reported: the secondary arms, the solves-only effect and the Robertson profile at 1e-8.
    reported = []
    for problem in PROBLEMS:
        for arm, reference in ((STAGES_ONLY, LEGACY), (STATIC_OVH, LEGACY), (STATIC_OVH, LEGACY_OVH),
                               (GATED, STAGES_ONLY)):
            reported.append(ratio_row(new, arm, reference, problem))
    rob8_rows = [ratio_row(new8, arm, reference, "robertson")
                 for arm, reference in ((GATED, LEGACY), (STAGES_ONLY, LEGACY), (STATIC_OVH, LEGACY),
                                        (STATIC_OVH, LEGACY_OVH), (LEGACY_OVH, LEGACY), (GATED, STAGES_ONLY))]
    ir_table = {problem: {arm: new[(arm, problem)]["ir_per_attempt"] for arm in
                          (LEGACY, GATED, STAGES_ONLY, LEGACY_OVH, STATIC_OVH) if (arm, problem) in new}
                for problem in PROBLEMS}
    deterministic = all(e["callgrind_deterministic"] for e in profile["entries"] + rob8["entries"] if e["success"])
    attribution = {problem: {arm: lines_of(new[(arm, problem)], "rodas5p_fast_small.rs")
                             for arm in (LEGACY, GATED, STAGES_ONLY) if (arm, problem) in new}
                   for problem in PROBLEMS}
    rob8_attempts = new8[(LEGACY, "robertson")]["attempts"] if (LEGACY, "robertson") in new8 else None

    gate = {"g1_identity": g1, "g2_tables": g2, "g3_instructions": g3, "g4_legacy_reproduction": g4}
    result = {
        "schema": "vigilode-spd06-check-v1",
        "gate": gate,
        "g1": {"points_as_registered": points_ok, "points_identical": g1_points, "rows": g1_rows,
               "ensemble": ensemble, "all_identical_by_arm": per_arm_identity,
               "static_solves_dropped": solves_dropped},
        "g2": {"tables_bitwise_equal": g2_tables, "contract_tests_passed": args.contract_passed == "true",
               "mismatches": tables["mismatches"]},
        "g3_rows": g3_rows,
        "g4_rows": g4_rows,
        "reported_secondary": reported,
        "reported_robertson_rtol_1e_8": {"attempts": rob8_attempts, "rtol": rob8["rtol"], "rows": rob8_rows},
        "ir_per_attempt": ir_table,
        "line_attribution_rodas5p_fast_small_rs": attribution,
        "validity": {"all_callgrind_deterministic": deterministic,
                     "same_binary": profile["binary_sha256"] == rob8["binary_sha256"],
                     "profile_rtol": profile["rtol"], "rob8_rtol": rob8["rtol"]},
        "binary_sha256": profile["binary_sha256"], "valgrind": profile["valgrind"],
        "base_binary_sha256": base_profile["binary_sha256"],
    }
    result["verdict"] = "PASS" if all(gate.values()) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(gate), result["verdict"])
    for r in g3_rows:
        if not r.get("missing"):
            print(f"{r['problem']:20s} {LEGACY} {r['reference_ir_per_attempt']:9.1f} {GATED} "
                  f"{r['arm_ir_per_attempt']:9.1f} ratio {r['ratio']:.4f} (<= {r['threshold']})")
    for r in g4_rows:
        if not r.get("missing"):
            print(f"{r['problem']:20s} legacy drift {r['drift_ratio_reported']:.4f} same work {r['same_work_and_final_state']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
