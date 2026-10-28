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
import math
from decimal import Decimal, localcontext


CAP = Decimal("1e-6")
U_REF_CAP = Decimal("1e-7")
SOURCE_RE = re.compile(r"^[0-9a-fA-F]{40}$")
ARMS = ("rhs-only", "pr91")
FROZEN_SPAN = [0, 1]
FROZEN_OPTIONS = {
    "rtol": 1e-7,
    "atol": {"kind": "scalar", "value": 1e-10},
    "h0": 1e-4,
    "max_steps": 100000,
    "dense_output": False,
    "interior_error": "w-filtered-enforce",
    "tier": "protected",
}
ATTEMPT_TIMEOUT = 1800


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
        # Match the production Rust binary64 initial-value expression exactly.
        e0 = Decimal.from_float(0.25 + 0.01 * component)
        decay = (-Decimal(1000) * t).exp()
        denominator = (Decimal(1) + Decimal("0.5") * e0 * e0 *
                       (Decimal(1) - (-Decimal(2000) * t).exp())).sqrt()
        return e0 * decay / denominator


def validate_payload(payload: object, expected_policy: str) -> dict:
    if (not isinstance(payload, dict) or payload.get("dimension") != 16 or
            payload.get("span") != FROZEN_SPAN or payload.get("options") != FROZEN_OPTIONS):
        raise ValueError("benchmark output lacks the required 16D declaration")
    rows = payload.get("rows")
    row = next((r for r in rows if isinstance(r, dict) and
                r.get("case") == "stiff-cubic" and r.get("policy") == expected_policy), None) if isinstance(rows, list) else None
    if row is None or not isinstance(row.get("times"), list) or not isinstance(row.get("states"), list):
        raise ValueError("benchmark output lacks the exact stiff-cubic case/policy and times/states")
    if row.get("success") is not True or row.get("pass") is not True or row.get("accounting") is not True:
        raise ValueError("solver success, pass, and accounting must all be true")
    seconds = row.get("seconds")
    if not isinstance(seconds, (int, float)) or not math.isfinite(seconds) or seconds <= 0:
        raise ValueError("solver-reported timing must be finite and positive")
    times, states = row["times"], row["states"]
    if not times or len(times) != len(states):
        raise ValueError("returned time/state arrays are empty or mismatched")
    errors = []
    uncertainty = []
    for ti, state in zip(times, states):
        if not isinstance(ti, (int, float)) or not math.isfinite(ti):
            raise ValueError("all returned times must be finite numbers")
        if ti < 0.0 or ti > 1.0:
            raise ValueError("all returned times must lie within the frozen [0, 1] span")
        if not isinstance(state, list) or len(state) != 16:
            raise ValueError("every returned state must contain all 16 components")
        if any(not isinstance(value, (int, float)) or not math.isfinite(value) for value in state):
            raise ValueError("all returned state components must be finite numbers")
        td = Decimal.from_float(float(ti))
        for component, value in enumerate(state):
            actual = Decimal.from_float(float(value))
            ref80 = oracle(td, component, 80)
            ref120 = oracle(td, component, 120)
            errors.append(abs(actual - ref120))
            uncertainty.append(abs(ref80 - ref120))
    return {
        "final_time": times[-1],
        "endpoint_matches_request": Decimal.from_float(float(times[-1])) == Decimal(1),
        "full_state": all(isinstance(s, list) and len(s) == 16 for s in states),
        "returned_times": len(times),
        "max_abs_error": str(max(errors)),
        "reference_uncertainty_estimate_max_decimal80_vs120": str(max(uncertainty)),
        "reference_uncertainty_estimate_cap": str(U_REF_CAP),
        "reference_uncertainty_estimate_within_cap": max(uncertainty) <= U_REF_CAP,
        "accuracy_cap": str(CAP),
        "accuracy_pass": max(errors) + max(uncertainty) <= CAP and max(uncertainty) <= U_REF_CAP,
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

    stop_after_timeout = False
    for index, (arm, kind, repetition) in enumerate(sequence, start=1):
        stem = f"{index:02d}_{kind}_{arm}"
        command = [str(args.binary), "stiff-cubic", arm, "1"]
        start_receipt = {"index": index, "arm": arm, "kind": kind,
                         "repetition": repetition, "command": command,
                         "started_unix_ns": time.time_ns(), "timeout_seconds": ATTEMPT_TIMEOUT}
        start_path = args.out / f"{stem}.start.json"
        with start_path.open("w") as handle:
            json.dump(start_receipt, handle, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        started = time.monotonic()
        stdout_path = args.out / f"{stem}.stdout"
        stderr_path = args.out / f"{stem}.stderr"
        end_path = args.out / f"{stem}.end.json"
        try:
            with stdout_path.open("wb") as stdout_handle, stderr_path.open("wb") as stderr_handle:
                proc = subprocess.Popen(
                    command, stdout=stdout_handle, stderr=stderr_handle, env=env,
                    preexec_fn=lambda: os.sched_setaffinity(0, {args.cpu}),
                )
                try:
                    exit_code = proc.wait(timeout=ATTEMPT_TIMEOUT)
                    timed_out = False
                except subprocess.TimeoutExpired:
                    proc.kill()
                    exit_code = proc.wait()
                    timed_out = True
                stdout_handle.flush()
                stderr_handle.flush()
                os.fsync(stdout_handle.fileno())
                os.fsync(stderr_handle.fileno())
            wall = time.monotonic() - started
            record = {
                "index": index, "arm": arm, "kind": kind,
                "repetition": repetition, "command": command,
                "exit_code": exit_code, "subprocess_wall_seconds": wall,
                "stdout_file": stdout_path.name, "stderr_file": stderr_path.name,
                "binary_reported_seconds_separate_from_wall": True,
                "timed_out": timed_out,
            }
            if timed_out:
                record["failure_type"] = "TIMEOUT"
                record["timeout_seconds"] = ATTEMPT_TIMEOUT
                stop_after_timeout = True
            elif exit_code == 0:
                try:
                    record["result"] = validate_payload(json.loads(stdout_path.read_bytes()), arm)
                except (ValueError, UnicodeDecodeError, json.JSONDecodeError, KeyError, TypeError) as exc:
                    record["parse_error"] = str(exc)
            calls.append(record)
        except OSError as exc:
            record = {"index": index, "arm": arm, "kind": kind,
                      "repetition": repetition, "command": command,
                      "launch_error": str(exc), "failure_type": "LAUNCH_ERROR",
                      "subprocess_wall_seconds": time.monotonic() - started}
            calls.append(record)
        end_receipt = {**record, "ended_unix_ns": time.time_ns()}
        with end_path.open("w") as handle:
            json.dump(end_receipt, handle, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        if stop_after_timeout:
            break

    by_arm = {arm: [c for c in calls if c.get("arm") == arm] for arm in ARMS}
    summaries = {}
    for arm, records in by_arm.items():
        good = [c for c in records if c.get("exit_code") == 0 and
                "result" in c and "parse_error" not in c]
        timed_good = [c for c in good if c.get("kind") == "timed"]
        seconds = [c["result"]["solver_seconds"] for c in timed_good]
        costs = [c.get("subprocess_wall_seconds") for c in records]
        complete = len(records) == 6 and len(good) == 6 and all(c.get("exit_code") == 0 for c in records)
        summaries[arm] = {
            "valid_subprocesses_including_warmup": len(good),
            "valid_timed_repetitions": len(timed_good),
            "subprocess_wall_seconds_including_warmup": costs,
            "complete_subprocess_wall_cost": sum(costs) if len(costs) == 6 and all(x is not None for x in costs) else None,
            "complete_arm_record": complete,
            "solver_seconds": seconds,
            "solver_seconds_median_min_max": ({"median": statistics.median(seconds),
                                                "min": min(seconds), "max": max(seconds)}
                                               if len(seconds) == 5 else None),
            "all_accuracy_pass": len(good) == 6 and all(c["result"]["accuracy_pass"] for c in good),
            "all_matched_output": len(good) == 6 and all(
                c["result"]["endpoint_matches_request"] and c["result"]["full_state"] for c in good),
        }
    eligible = len(calls) == 12 and all(summaries[a]["complete_arm_record"] and
                   summaries[a]["valid_timed_repetitions"] == 5 and
                   summaries[a]["all_accuracy_pass"] and summaries[a]["all_matched_output"]
                   for a in ARMS)
    ratio = None
    if eligible:
        costs = {a: summaries[a]["complete_subprocess_wall_cost"] for a in ARMS}
        if costs["rhs-only"] > 0 and costs["pr91"] > 0:
            ratio = costs["rhs-only"] / costs["pr91"]
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
            "all_warmup_and_timed_subprocesses_successful": eligible,
            "ratio_uses_complete_arm_subprocess_wall_costs_including_warmup": True,
        },
        "timing_authority": "diagnostic_only",
        "timed_out": stop_after_timeout,
        "cost_scope": "subprocess wall includes launch/setup and solver cost; warmup and timed calls included by arm; failures retained; no retries; solver-reported seconds are diagnostic only",
    }
    (args.out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    all_valid = (eligible and ratio is not None and all(
        c.get("exit_code") == 0 and "result" in c and "parse_error" not in c and
        c["result"]["accuracy_pass"] and c["result"]["endpoint_matches_request"]
        for c in calls))
    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
