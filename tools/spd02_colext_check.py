#!/usr/bin/env python3
"""Gate of research node research/spd02_lu_column_extents_20261005 (speed node SPD02).

G1 identity: the column-extent LU equals the legacy driver bitwise on all 35 identity points.
G2 LU contract: recorded from the cargo test run (passed/failed flag given on the command line).
G3 instructions: Ir per attempt colext / legacy <= 0.60 on brusselator-1d-200 and <= 0.80 on
   brusselator-1d-50.
G4 no small-problem regression: ratio <= 1.03 on hires, van-der-pol-mu1000 and robertson.
G5 legacy reproduction: the legacy arm reproduces the base export (work and final-state hash) and
   its Ir per attempt within +-2 % on all five problems.
G6 allocations: recorded from the cargo test run (flag on the command line).
Kill: n = 400 ratio >= 0.90, any identity or contract difference, small-problem ratio > 1.03.
Counted instructions only; no wall-time claim.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

SMALL = ["hires", "van-der-pol-mu1000", "robertson"]


def entries(profile):
    return {(e["arm"], e["problem"]): e for e in profile["entries"] if e["success"]}


def lines_of(entry, file_suffix):
    return [l for l in entry["top_lines"] if l["file"].endswith(file_suffix)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--identity", required=True, type=Path)
    parser.add_argument("--contract-passed", required=True, choices=["true", "false"])
    parser.add_argument("--allocations-passed", required=True, choices=["true", "false"])
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = entries(json.loads(args.base.read_text()))
    profile = json.loads(args.profile.read_text())
    new = entries(profile)
    identity = json.loads(args.identity.read_text())

    g1_rows = [{"problem": r["problem"], "rtol": r["rtol"], "identical": r["identical"]} for r in identity["rows"]]
    g1 = len(g1_rows) == 35 and all(r["identical"] for r in g1_rows)
    g2 = args.contract_passed == "true"
    ratios = {}
    for problem in ["brusselator-1d-200", "brusselator-1d-50", *SMALL]:
        leg = new[("rodas5p-fast", problem)]
        col = new[("rodas5p-fast-colext", problem)]
        ratios[problem] = {"legacy_ir_per_attempt": leg["ir_per_attempt"], "colext_ir_per_attempt": col["ir_per_attempt"],
                           "ratio": col["ir_per_attempt"] / leg["ir_per_attempt"], "attempts": leg["attempts"],
                           "same_work": (leg["attempts"] == col["attempts"] and leg["counters"] == col["counters"]
                                         and leg["final_state_sha256"] == col["final_state_sha256"]),
                           "deterministic": leg["callgrind_deterministic"] and col["callgrind_deterministic"]}
    g3 = ratios["brusselator-1d-200"]["ratio"] <= 0.60 and ratios["brusselator-1d-50"]["ratio"] <= 0.80
    g4 = all(ratios[p]["ratio"] <= 1.03 for p in SMALL)
    g5_rows = []
    g5 = True
    for problem in ["brusselator-1d-200", "brusselator-1d-50", *SMALL]:
        b = base[("rodas5p-fast", problem)]
        n = new[("rodas5p-fast", problem)]
        same = (n["attempts"] == b["attempts"] and n["accepted_steps"] == b["accepted_steps"]
                and n["rejected_steps"] == b["rejected_steps"] and n["counters"] == b["counters"]
                and n["final_state_sha256"] == b["final_state_sha256"])
        ratio = n["ir_per_attempt"] / b["ir_per_attempt"]
        ok = same and abs(ratio - 1.0) <= 0.02
        g5 &= ok
        g5_rows.append({"problem": problem, "same_work": same, "base_ir_per_attempt": b["ir_per_attempt"],
                        "new_ir_per_attempt": n["ir_per_attempt"], "ratio": ratio, "ok": ok})
    g6 = args.allocations_passed == "true"
    kill = ratios["brusselator-1d-200"]["ratio"] >= 0.90 or not g1 or not g2 or not g4
    attribution = {problem: {arm: lines_of(new[(arm, problem)], "rodas5p_fast.rs")[:20]
                             for arm in ("rodas5p-fast", "rodas5p-fast-colext")}
                   for problem in ("brusselator-1d-200", "brusselator-1d-50")}
    categories = {problem: {arm: new[(arm, problem)]["categories_by_file"]
                            for arm in ("rodas5p-fast", "rodas5p-fast-colext")}
                  for problem in ("brusselator-1d-200", "brusselator-1d-50")}
    result = {"schema": "vigilode-spd02-check-v1",
              "gate": {"g1_identity": g1, "g2_lu_contract": g2, "g3_instructions": g3, "g4_small_problems": g4,
                       "g5_legacy_reproduction": g5, "g6_allocations": g6, "kill": kill},
              "ratios": ratios, "g1_rows": g1_rows, "g5_rows": g5_rows,
              "line_attribution_rodas5p_fast_rs": attribution, "categories_by_file": categories,
              "binary_sha256": profile["binary_sha256"], "valgrind": profile["valgrind"]}
    result["verdict"] = "PASS" if all([g1, g2, g3, g4, g5, g6]) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result["gate"]), result["verdict"])
    for p, r in ratios.items():
        print(f"{p:20s} legacy {r['legacy_ir_per_attempt']:12.1f} colext {r['colext_ir_per_attempt']:12.1f} ratio {r['ratio']:.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
