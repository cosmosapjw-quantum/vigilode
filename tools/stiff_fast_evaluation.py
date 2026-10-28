#!/usr/bin/env python3
"""Evaluation of the lean RODAS5P driver (research node research/stiff_rodas5p_fast_20261002;
see its PREREGISTRATION.md).

Inputs: the release `rodas5p` binary and the ANALYSIS.json that tools/stiff_benchmark_native.py
wrote for a Rust run with the arms `rodas5p` and `rodas5p-fast`.  The script

1. measures instructions per attempted step of both arms with callgrind (one integration as the
   difference of 2 and 1 repetitions of `rodas5p stiff-profile-run`) on the profile workloads;
2. pairs the final-time errors of both arms at every problem and tolerance;
3. restates the matched-error wall times relative to `rodas5p-fast`;
4. applies the preregistered gate and writes EVALUATION.json.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = "vigilode-stiff-rodas5p-fast-evaluation-v1"
RTOL = 1.0e-6
WORKLOADS = ("hires", "van-der-pol-mu1000", "brusselator-1d-200")
SMALL = ("hires", "van-der-pol-mu1000")
SMALL_LIMIT = 0.5
LARGE_LIMIT = 0.8
ERROR_RATIO_LIMIT = 3.0


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", required=True)
    parser.add_argument("--analysis", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--scratch", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    args.scratch.mkdir(parents=True, exist_ok=True)
    profile = load("stiff_profile")
    analysis = json.loads(args.analysis.read_text())

    instructions = []
    for problem in WORKLOADS:
        entry = {"problem": problem}
        for arm in ("rodas5p", "rodas5p-fast"):
            cmd = lambda k: [args.rodas5p, "stiff-profile-run", "--problem", problem, "--arm", arm,
                             "--rtol", repr(RTOL), "--repetitions", str(k)]
            work = profile.run_json(cmd(1))
            runs = {k: profile.callgrind(cmd(k), args.scratch / f"callgrind.{arm}.{problem}.{k}") for k in (1, 2)}
            run = profile.difference(runs[2], runs[1])
            entry[arm] = {
                "attempts": work["attempts"],
                "accepted_steps": work["accepted_steps"],
                "counters": work["counters"],
                "ir_per_run": run["total"],
                "ir_per_attempt": run["total"] / work["attempts"],
                "top_functions": [{"function": f, "share": v / run["total"]}
                                  for f, v in run["by_fn"].most_common(10)],
            }
        entry["ir_per_attempt_ratio"] = entry["rodas5p-fast"]["ir_per_attempt"] / entry["rodas5p"]["ir_per_attempt"]
        entry["ir_per_run_ratio"] = entry["rodas5p-fast"]["ir_per_run"] / entry["rodas5p"]["ir_per_run"]
        print(problem, f"Ir/attempt ratio {entry['ir_per_attempt_ratio']:.3f}")
        instructions.append(entry)

    rows = analysis["rows"]
    pairs = []
    for old in (r for r in rows if r["arm"] == "rodas5p"):
        new = next(r for r in rows if r["arm"] == "rodas5p-fast" and r["problem"] == old["problem"]
                   and r["rtol"] == old["rtol"])
        pair = {"problem": old["problem"], "rtol": old["rtol"], "rodas5p_error": old.get("error"),
                "fast_error": new.get("error"), "rodas5p_steps": old.get("accepted_steps"),
                "fast_steps": new.get("accepted_steps"),
                "wall_ratio": new["wall_median"] / old["wall_median"]}
        if pair["rodas5p_error"] is not None and pair["fast_error"] is not None:
            a, b = pair["rodas5p_error"], pair["fast_error"]
            pair["error_ratio"] = max(a, b) / max(min(a, b), 1e-300)
        pairs.append(pair)

    matched = {}
    for pid, targets in analysis["matched_error"].items():
        matched[pid] = {}
        for target, entry in targets.items():
            if not isinstance(entry, dict) or not entry.get("rodas5p-fast"):
                matched[pid][target] = entry if isinstance(entry, str) else None
                continue
            base = entry["rodas5p-fast"]["wall_median"]
            matched[pid][target] = {arm: (None if v is None else v["wall_median"] / base)
                                    for arm, v in entry.items()}

    gate = {
        "benchmark_valid": analysis["verdict"] == "PASS",
        "small_problems_at_most_half_the_instructions": all(
            e["ir_per_attempt_ratio"] <= SMALL_LIMIT for e in instructions if e["problem"] in SMALL),
        "brusselator_400_at_most_0_8_of_the_instructions": all(
            e["ir_per_attempt_ratio"] <= LARGE_LIMIT for e in instructions if e["problem"] not in SMALL),
        "same_accuracy": all(p.get("error_ratio") is not None and p["error_ratio"] <= ERROR_RATIO_LIMIT
                             for p in pairs),
    }
    report = {"schema": SCHEMA, "rtol": RTOL, "instructions": instructions, "accuracy_pairs": pairs,
              "matched_error_wall_ratio_to_fast": matched, "validity_gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"verdict": report["verdict"], "gate": gate}))


if __name__ == "__main__":
    main()
