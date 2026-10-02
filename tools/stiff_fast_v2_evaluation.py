#!/usr/bin/env python3
"""Evaluation of the RODAS5P fast driver v2 (research node research/stiff_rodas5p_fast_v2_20261002;
see its PREREGISTRATION.md).

v2 changes only operations on exact zeros (banded LU extents) and where the Jacobian lives (an
in-place callback), so it must reproduce v1 (L-0032) exactly.  The script

1. compares every `rodas5p-fast` row of the new Rust run with the v1 row of L-0032: the final
   state (as values), accepted and rejected steps and the work counters must be identical;
2. measures instructions per attempted step of v2 with callgrind on the profile workloads, as one
   integration (the difference of 2 and 1 repetitions), against v1's recorded numbers;
3. restates the matched-error wall times relative to v2;
4. applies the preregistered gate and writes EVALUATION.json.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = "vigilode-stiff-rodas5p-fast-v2-evaluation-v1"
RTOL = 1.0e-6
SMALL = ("hires", "van-der-pol-mu1000")
LARGE = ("brusselator-1d-200",)
SMALL_LIMIT = 0.95
LARGE_LIMIT = 0.6
COUNTERS = ("rhs_evaluations", "jacobian_builds", "direct_factorizations")


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", required=True)
    parser.add_argument("--rust", required=True, type=Path)
    parser.add_argument("--analysis", required=True, type=Path)
    parser.add_argument("--v1-rust", required=True, type=Path)
    parser.add_argument("--v1-evaluation", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--scratch", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    args.scratch.mkdir(parents=True, exist_ok=True)
    profile = load("stiff_profile")
    rust = json.loads(args.rust.read_text())
    analysis = json.loads(args.analysis.read_text())
    v1_rows = {(r["problem"], r["rtol"]): r for r in json.loads(args.v1_rust.read_text())["rows"]
               if r["arm"] == "rodas5p-fast"}
    v1_ir = {w["problem"]: w["rodas5p-fast"]["ir_per_attempt"]
             for w in json.loads(args.v1_evaluation.read_text())["instructions"]}

    identity = []
    for row in (r for r in rust["rows"] if r["arm"] == "rodas5p-fast"):
        old = v1_rows[(row["problem"], row["rtol"])]
        same = (row["status"] == old["status"] == "completed"
                and row["final_state"] == old["final_state"]
                and row["accepted_steps"] == old["accepted_steps"]
                and row["rejected_steps"] == old["rejected_steps"]
                and all(row["counters"][c] == old["counters"][c] for c in COUNTERS))
        identity.append({"problem": row["problem"], "rtol": row["rtol"], "identical": same,
                         "wall_ratio_to_v1": row["wall_median"] / old["wall_median"]})

    instructions = []
    for problem in SMALL + LARGE:
        cmd = lambda k: [args.rodas5p, "stiff-profile-run", "--problem", problem, "--arm", "rodas5p-fast",
                         "--rtol", repr(RTOL), "--repetitions", str(k)]
        work = profile.run_json(cmd(1))
        runs = {k: profile.callgrind(cmd(k), args.scratch / f"callgrind.v2.{problem}.{k}") for k in (1, 2)}
        run = profile.difference(runs[2], runs[1])
        per_attempt = run["total"] / work["attempts"]
        instructions.append({
            "problem": problem, "attempts": work["attempts"], "ir_per_run": run["total"],
            "ir_per_attempt": per_attempt, "v1_ir_per_attempt": v1_ir[problem],
            "ratio_to_v1": per_attempt / v1_ir[problem],
            "top_functions": [{"function": f, "share": v / run["total"]} for f, v in run["by_fn"].most_common(10)],
        })
        print(problem, f"v2/v1 Ir per attempt {per_attempt / v1_ir[problem]:.3f}")

    matched = {}
    for pid, targets in analysis["matched_error"].items():
        matched[pid] = {}
        for target, entry in targets.items():
            if not isinstance(entry, dict) or not entry.get("rodas5p-fast"):
                matched[pid][target] = entry if isinstance(entry, str) else None
                continue
            base = entry["rodas5p-fast"]["wall_median"]
            matched[pid][target] = {"rodas5p-fast_ms": base * 1e3,
                                    **{arm: (None if v is None else v["wall_median"] / base) for arm, v in entry.items()}}

    gate = {
        "benchmark_valid": analysis["verdict"] == "PASS",
        "reproduces_v1_exactly": len(identity) == 35 and all(i["identical"] for i in identity),
        "small_problems_at_most_0_95_of_v1": all(
            i["ratio_to_v1"] <= SMALL_LIMIT for i in instructions if i["problem"] in SMALL),
        "brusselator_400_at_most_0_6_of_v1": all(
            i["ratio_to_v1"] <= LARGE_LIMIT for i in instructions if i["problem"] in LARGE),
    }
    report = {"schema": SCHEMA, "rtol": RTOL, "identity": identity, "instructions": instructions,
              "matched_error_wall_ratio_to_fast": matched, "validity_gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"verdict": report["verdict"], "gate": gate}))


if __name__ == "__main__":
    main()
