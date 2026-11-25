#!/usr/bin/env python3
"""Run the V30 E7 diagnosis binary (node V30_ROUTER_BREAKEVEN_V27B, phase 1) on P_scalar_fd.

One process per tolerance pair (A, L, S) and one for the one-step grid (--tau), pinned to cores 0-5
and 12-17, one at a time. Raw stdout, stderr and the exit code of every process are kept under
logs/e7/raw_<first 12 hex of the binary's sha256>/; logs/e7/RUNS_INDEX.jsonl gets
one row per process (UTC start, command, exit code, line count, sha256 of stdout). Nothing is
filtered: a non-zero exit or a malformed line is a result and is kept.

Usage: python3 -I run_e7.py
"""
import datetime
import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BINARY = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v30/release/v30-e7-diag"
RAW = os.path.join(HERE, "logs", "e7", "raw")
INDEX = os.path.join(HERE, "logs", "e7", "RUNS_INDEX.jsonl")


def main():
    binary_sha = hashlib.sha256(open(BINARY, "rb").read()).hexdigest()
    # one directory per binary (the first run, binary 01a78af8..., wrote logs/e7/raw/ directly)
    raw = os.path.join(RAW + "_" + binary_sha[:12])
    os.makedirs(raw, exist_ok=True)
    rows = []
    for tol in ("A", "L", "S", "tau"):
        flag = ["--tau"] if tol == "tau" else ["--tol", tol]
        command = ["taskset", "-c", "0-5,12-17", BINARY, *flag]
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        done = subprocess.run(command, capture_output=True, text=True)
        base = os.path.join(raw, f"P_scalar_fd__{tol}")
        with open(base + ".stdout.jsonl", "w") as out:
            out.write(done.stdout)
        with open(base + ".stderr.log", "w") as err:
            err.write(done.stderr)
        lines = [line for line in done.stdout.splitlines() if line.strip()]
        malformed = 0
        for line in lines:
            try:
                json.loads(line)
            except ValueError:
                malformed += 1
        rows.append({
            "utc_start": started,
            "command": command,
            "binary_sha256": binary_sha,
            "exit_code": done.returncode,
            "lines": len(lines),
            "malformed_lines": malformed,
            "stdout_sha256": hashlib.sha256(done.stdout.encode()).hexdigest(),
            "stdout": os.path.relpath(base + ".stdout.jsonl", HERE),
            "stderr": os.path.relpath(base + ".stderr.log", HERE),
        })
    with open(INDEX, "a") as index:
        for row in rows:
            index.write(json.dumps(row, sort_keys=True) + "\n")
    for row in rows:
        print(row["command"][-1], "exit", row["exit_code"], "lines", row["lines"],
              "malformed", row["malformed_lines"])
    return 0 if all(r["exit_code"] == 0 and r["malformed_lines"] == 0 for r in rows) else 1


if __name__ == "__main__":
    sys.exit(main())
