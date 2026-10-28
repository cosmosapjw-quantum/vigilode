#!/usr/bin/env python3
"""Matched full-state stiff-cubic benchmark with an independent Decimal oracle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import time
from decimal import Decimal, localcontext


CAP = Decimal("1e-6")
SOURCE_RE = re.compile(r"^[0-9a-fA-F]{40}$")
ARMS = ("rhs-only", "pr91")


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--cpu", required=True, type=int)
    args = parser.parse_args()
    if not SOURCE_RE.fullmatch(args.source_commit):
        parser.error("--source-commit must be a 40-character Git SHA")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("--binary must name an executable file")
    if args.out.exists():
        parser.error("--out must be a new, nonexistent directory")
    if args.cpu < 0:
        parser.error("--cpu must be nonnegative")
    if not hasattr(os, "sched_getaffinity") or args.cpu not in os.sched_getaffinity(0):
        parser.error("--cpu must be in the current process affinity set")
    args.binary = binary
    return args


def oracle(t: Decimal, component: int, precision: int) -> Decimal:
    if component == 0:
        return t
    with localcontext() as ctx:
        ctx.prec = precision
        j = Decimal(component)
        e0 = Decimal("0.25") + Decimal("0.01") * j
        decay = (-Decimal(1000) * t).exp()
        denominator = (Decimal(1) + Decimal("0.5") * e0 * e0 *
                       (Decimal(1) - (-Decimal(2000) * t).exp())).sqrt()
        return e0 * decay / denominator


def validate_payload(payload: object) -> dict:
    if not isinstance(payload, dict) or payload.get("dimension") != 16:
        raise ValueError("benchmark output lacks the required 16D declaration")
    rows = payload.get("rows")
    row = next((r for r in rows if isinstance(r, dict) and
                r.get("case") == "stiff-cubic"), None) if isinstance(rows, list) else None
    if row is None or not isinstance(row.get("times"), list) or not isinstance(row.get("states"), list):
        raise ValueError("benchmark output lacks stiff-cubic times/states")
    times, states = row["times"], row["states"]
    if not times or len(times) != len(states):
        raise ValueError("returned time/state arrays are empty or mismatched")
    errors = []
    uncertainty = []
    for ti, state in zip(times, states):
        if not isinstance(state, list) or len(state) != 16:
            raise ValueError("every returned state must contain all 16 components")
        td = Decimal(str(ti))
        for component, value in enumerate(state):
            actual = Decimal(str(value))
            ref80 = oracle(td, component, 80)
            ref120 = oracle(td, component, 120)
            errors.append(abs(actual - ref120))
            uncertainty.append(abs(ref80 - ref120))
    return {
        "final_time": times[-1],
        "endpoint_matches_request": Decimal(str(times[-1])) == Decimal(1),
        "full_state": all(isinstance(s, list) and len(s) == 16 for s in states),
        "returned_times": len(times),
        "max_abs_error": str(max(errors)),
        "reference_uncertainty_estimate_max_decimal80_vs120": str(max(uncertainty)),
        "accuracy_cap": str(CAP),
        "accuracy_pass": max(errors) + max(uncertainty) <= CAP,
        "reference_uncertainty_certified": False,
        "solver_reported_pass": row.get("pass"),
        "solver_seconds": row.get("seconds"),
        "work": row.get("work"),
        "rhs": row.get("rhs"),
        "jvp": row.get("jvp"),
        "attempts": row.get("attempts"),
        "switches": row.get("switches"),
        "accounting": row.get("accounting"),
    }


def main() -> int:
    args = arguments()
    args.out.mkdir(parents=True)
    binary_hash = hashlib.sha256(args.binary.read_bytes()).hexdigest()
    env = os.environ.copy()
    env.update({"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1",
                "OPENBLAS_NUM_THREADS": "1"})
    calls = []
    sequence = [(arm, "warmup", 1) for arm in ARMS]
    for repetition in range(5):
        order = ARMS if repetition % 2 == 0 else tuple(reversed(ARMS))
        sequence.extend((arm, "timed", repetition + 1) for arm in order)

    for index, (arm, kind, repetition) in enumerate(sequence, start=1):
        stem = f"{index:02d}_{kind}_{arm}"
        command = [str(args.binary), "stiff-cubic", arm, "1"]
        started = time.monotonic()
        try:
            proc = subprocess.run(
                command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                env=env, check=False,
                preexec_fn=lambda: os.sched_setaffinity(0, {args.cpu}),
            )
            wall = time.monotonic() - started
            stdout_path = args.out / f"{stem}.stdout"
            stderr_path = args.out / f"{stem}.stderr"
            stdout_path.write_bytes(proc.stdout)
            stderr_path.write_bytes(proc.stderr)
            record = {
                "index": index, "arm": arm, "kind": kind,
                "repetition": repetition, "command": command,
                "exit_code": proc.returncode, "subprocess_wall_seconds": wall,
                "stdout_file": stdout_path.name, "stderr_file": stderr_path.name,
                "binary_reported_seconds_separate_from_wall": True,
            }
            if proc.returncode == 0:
                try:
                    record["result"] = validate_payload(json.loads(proc.stdout))
                except (ValueError, json.JSONDecodeError, KeyError, TypeError) as exc:
                    record["parse_error"] = str(exc)
            calls.append(record)
        except OSError as exc:
            calls.append({"index": index, "arm": arm, "kind": kind,
                          "repetition": repetition, "command": command,
                          "launch_error": str(exc), "subprocess_wall_seconds": time.monotonic() - started})

    timed = {arm: [c for c in calls if c.get("arm") == arm and c.get("kind") == "timed"]
             for arm in ARMS}
    summaries = {}
    for arm, records in timed.items():
        good = [c for c in records if c.get("exit_code") == 0 and
                "result" in c and "parse_error" not in c]
        seconds = [c["result"]["solver_seconds"] for c in good
                   if isinstance(c["result"].get("solver_seconds"), (int, float))]
        summaries[arm] = {
            "valid_timed_repetitions": len(good),
            "subprocess_wall_seconds": [c.get("subprocess_wall_seconds") for c in records],
            "solver_seconds": seconds,
            "solver_seconds_median_min_max": ({"median": statistics.median(seconds),
                                                "min": min(seconds), "max": max(seconds)}
                                               if len(seconds) == 5 else None),
            "all_accuracy_pass": len(good) == 5 and all(c["result"]["accuracy_pass"] for c in good),
            "all_matched_output": len(good) == 5 and all(
                c["result"]["endpoint_matches_request"] and c["result"]["full_state"] for c in good),
        }
    eligible = all(summaries[a]["valid_timed_repetitions"] == 5 and
                   summaries[a]["all_accuracy_pass"] and summaries[a]["all_matched_output"]
                   for a in ARMS)
    ratio = None
    if eligible:
        medians = {a: summaries[a]["solver_seconds_median_min_max"]["median"] for a in ARMS}
        if medians["rhs-only"] > 0:
            ratio = medians["rhs-only"] / medians["pr91"] if medians["pr91"] > 0 else None
    summary = {
        "schema": "vigilode.matched-stiff-cubic.v1",
        "source_commit": args.source_commit.lower(),
        "binary": str(args.binary), "binary_sha256": binary_hash,
        "cpu": args.cpu, "thread_environment": {k: env[k] for k in
            ("RAYON_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS")},
        "requested_endpoint": 1, "required_dimension": 16,
        "warmups_per_arm": 1, "timed_repetitions_per_arm": 5,
        "execution_order": [{"index": c["index"], "arm": c["arm"], "kind": c["kind"]} for c in calls],
        "calls": calls, "arm_summaries": summaries,
        "ratio_rhs_only_over_pr91": ratio,
        "ratio_eligible": ratio is not None,
        "ratio_conditions": {
            "same_binary": True, "matched_physical_endpoint_full_state": eligible,
            "output_requirement_satisfied": eligible,
            "both_arms_error_plus_reference_uncertainty_within_1e-6": eligible,
        },
        "timing_authority": "diagnostic_only",
        "cost_scope": "all subprocess setup, warmup, timed, launch failures and retries are preserved; no costs combined into profile costs",
    }
    (args.out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return 0 if all(len([c for c in timed[a] if c.get("exit_code") == 0 and "result" in c]) == 5
                    for a in ARMS) else 1


if __name__ == "__main__":
    sys.exit(main())
