#!/usr/bin/env python3
"""Run matched fresh-process A07 calls, preserving every child receipt."""
import argparse
import hashlib
import json
import os
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--grid", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--pairs", type=int, default=10)
    parser.add_argument("--timeout-seconds", type=float, default=300.0)
    parser.add_argument("--logs", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    os.makedirs(args.logs, exist_ok=True)
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    grid_sha = hashlib.sha256(open(args.grid, "rb").read()).hexdigest()
    manifest_sha = hashlib.sha256(open(args.manifest, "rb").read()).hexdigest()
    binary_sha = hashlib.sha256(open(args.binary, "rb").read()).hexdigest()
    arms = [("native", "diffsol"), ("diffsol", "native")]
    with open(args.out, "w", encoding="utf-8") as output:
        for pair in range(args.pairs):
            for position, arm in enumerate(arms[pair % 2]):
                command = [args.binary, "--single", arm, "--grid", args.grid, "--manifest", args.manifest]
                try:
                    result = subprocess.run(command, text=True, capture_output=True, check=False, timeout=args.timeout_seconds)
                except subprocess.TimeoutExpired as exc:
                    result = None
                    stdout = exc.stdout.decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
                    stderr = exc.stderr.decode() if isinstance(exc.stderr, bytes) else (exc.stderr or "")
                    stderr += f"\nchild timeout after {args.timeout_seconds} seconds"
                    returncode = 124
                except OSError as exc:
                    result = None
                    stdout, stderr, returncode = "", str(exc), 127
                else:
                    stdout, stderr, returncode = result.stdout, result.stderr, result.returncode
                prefix = os.path.join(args.logs, f"cold_pair_{pair:03}_{position}_{arm}")
                with open(prefix + ".stdout", "w", encoding="utf-8") as stream:
                    stream.write(stdout)
                with open(prefix + ".stderr", "w", encoding="utf-8") as stream:
                    stream.write(stderr)
                try:
                    row = json.loads(stdout.strip().splitlines()[-1])
                except (IndexError, json.JSONDecodeError):
                    row = {
                        "schema": "vigilode-a07-run-v1", "arm": arm,
                        "protocol": "single_cold_process", "status": "process_failed",
                        "elapsed_ns": None, "allocations": None, "allocated_bytes": None,
                        "output_count": 0, "times": [], "states": [],
                        "work_counters": None, "solver_counters": None,
                        "callback_counters": None,
                        "unavailable_counters_reason": "child produced no valid JSON receipt; callback counters disabled to preserve timing path",
                        "partial_solver_counters_reason": "child produced no valid JSON receipt",
                        "error": stderr or stdout,
                        "timing_authority": "diagnostic_only",
                        "grid_sha256": grid_sha, "manifest_sha256": manifest_sha,
                        "binary_sha256": binary_sha,
                    }
                row["pair_id"] = pair
                row["within_pair_order"] = position
                row["process_exit_code"] = returncode
                output.write(json.dumps(row, allow_nan=False, separators=(",", ":")) + "\n")
                output.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
