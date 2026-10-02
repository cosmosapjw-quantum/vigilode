#!/usr/bin/env python3
"""Evaluation of the small-n specialization of the fast driver (research node
research/thread_transfer_smalln_cost_20261002, thread-transfer DAG node P1-SMALLN-COST; see its
PREREGISTRATION.md).

1. Identity: for van der Pol, Robertson and HIRES at rtol 1e-4, 1e-6 and 1e-8, `rodas5p
   stiff-profile-run` with the arms `rodas5p-fast` (v2) and `rodas5p-fast-small` must report the same
   final state (as values), accepted and rejected steps, and RHS, Jacobian and factorization counts.
2. Instructions per attempted step at rtol 1e-6 with callgrind, one integration as the difference of
   2 and 1 repetitions, for both arms; the gate asks for at most 0.8 on van der Pol and HIRES.
3. An ensemble of 64 van der Pol trajectories (`rodas5p stiff-ensemble-run`), instructions per
   trajectory as the difference of 64 and 32 members, against one trajectory: reported only.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = "vigilode-thread-transfer-smalln-evaluation-v1"
PROBLEMS = ("van-der-pol-mu1000", "robertson", "hires")
GATED = ("van-der-pol-mu1000", "hires")
TOLERANCES = (1.0e-4, 1.0e-6, 1.0e-8)
LIMIT = 0.8
COUNTERS = ("rhs_evaluations", "jacobian_builds", "direct_factorizations")
ARMS = ("rodas5p-fast", "rodas5p-fast-small")


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--scratch", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    args.scratch.mkdir(parents=True, exist_ok=True)
    profile = load("stiff_profile")

    def run(problem, arm, rtol, k=1):
        return [args.rodas5p, "stiff-profile-run", "--problem", problem, "--arm", arm,
                "--rtol", repr(rtol), "--repetitions", str(k)]

    identity = []
    for problem in PROBLEMS:
        for rtol in TOLERANCES:
            v2, small = (profile.run_json(run(problem, arm, rtol)) for arm in ARMS)
            same = (v2["success"] and small["success"]
                    and v2["final_state"] == small["final_state"]
                    and v2["accepted_steps"] == small["accepted_steps"]
                    and v2["rejected_steps"] == small["rejected_steps"]
                    and all(v2["counters"][c] == small["counters"][c] for c in COUNTERS))
            identity.append({"problem": problem, "rtol": rtol, "identical": same,
                             "attempts": v2["attempts"], "small_attempts": small["attempts"]})
            print(problem, rtol, "identical" if same else "DIFFERENT", flush=True)

    instructions = []
    for problem in PROBLEMS:
        entry = {"problem": problem}
        for arm in ARMS:
            work = profile.run_json(run(problem, arm, 1.0e-6))
            runs = {k: profile.callgrind(run(problem, arm, 1.0e-6, k), args.scratch / f"cg.{arm}.{problem}.{k}")
                    for k in (1, 2)}
            one = profile.difference(runs[2], runs[1])
            entry[arm] = {"attempts": work["attempts"], "ir_per_run": one["total"],
                          "ir_per_attempt": one["total"] / work["attempts"],
                          "top_functions": [{"function": f, "share": v / one["total"]}
                                            for f, v in one["by_fn"].most_common(8)]}
        entry["ratio"] = entry[ARMS[1]]["ir_per_attempt"] / entry[ARMS[0]]["ir_per_attempt"]
        print(problem, f"small/v2 Ir per attempt {entry['ratio']:.3f}", flush=True)
        instructions.append(entry)

    ensemble = {}
    for arm in ARMS:
        cmd = lambda members: [args.rodas5p, "stiff-ensemble-run", "--arm", arm, "--members", str(members),
                               "--rtol", "1e-6"]
        full, half = (profile.callgrind(cmd(m), args.scratch / f"cg.ensemble.{arm}.{m}") for m in (64, 32))
        info = profile.run_json(cmd(64))
        per_member = (full["total"] - half["total"]) / 32
        single = next(e[arm]["ir_per_run"] for e in instructions if e["problem"] == "van-der-pol-mu1000")
        ensemble[arm] = {"members": 64, "attempts": info["attempts"], "checksum": info["checksum"],
                         "ir_per_trajectory": per_member, "single_trajectory_mu1000_ir": single}
    ensemble["checksums_equal"] = ensemble[ARMS[0]]["checksum"] == ensemble[ARMS[1]]["checksum"]
    ensemble["per_trajectory_ratio"] = ensemble[ARMS[1]]["ir_per_trajectory"] / ensemble[ARMS[0]]["ir_per_trajectory"]

    gate = {
        "identical_results": all(i["identical"] for i in identity),
        "instructions_at_most_0_8": all(e["ratio"] <= LIMIT for e in instructions if e["problem"] in GATED),
    }
    report = {"schema": SCHEMA, "identity": identity, "instructions": instructions, "ensemble": ensemble,
              "validity_gate": gate, "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"verdict": report["verdict"], "gate": gate}))


if __name__ == "__main__":
    main()
