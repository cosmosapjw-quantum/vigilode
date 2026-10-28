#!/usr/bin/env python3
"""Same-binary instruction profile of research node SP02 (research/sp02_krylov_workspace_caller_20261011).

Protocol (SP01, SPD01-SPD09): callgrind on one process with 1 and with 2 integrations of one cell;
the difference is one trajectory, with process start-up and problem construction cancelled. The 1-repetition run is
repeated once as a determinism check. Both arms run in the same release binary, the example
`crates/rodas5p-integrators/examples/sp02_workspace_profile.rs`, which this tool builds first with line tables
(`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked -p rodas5p-integrators --example
sp02_workspace_profile`; the binary is taken from `$CARGO_TARGET_DIR`, default `target/`). No wall clock: counted
instructions only (Ir).

Profiled cells (registered): van der Pol, HIRES and Brusselator-50 at rtol 1e-6, for both methods (GMRES and LGMRES)
and both gated arms (control and candidate), 12 profiles in total.

For every cell and arm the entry records the native 1-repetition work (attempts, counters, final state), the
three callgrind totals, Ir per trajectory (2 minus 1), whether the repeated 1-repetition total is identical, and
whether every callgrind run printed the native record.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA = "vigilode-sp02-workspace-profile-v1"
NODE = "sp02_krylov_workspace_caller_20261011"
ROOT = Path(__file__).resolve().parents[1]
EXAMPLE = "sp02_workspace_profile"
PROBLEMS = ["van-der-pol-mu1000", "hires", "brusselator-1d-50"]
RTOL = 1.0e-6
METHODS = ["gmres", "lgmres"]
ARMS = ["control", "candidate"]
ENV = {
    "RAYON_NUM_THREADS": "1",
    "OPENBLAS_NUM_THREADS": "1",
    "DEBUGINFOD_URLS": "",
}


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()


def build() -> Path:
    env = dict(os.environ, CARGO_PROFILE_RELEASE_DEBUG="line-tables-only", **ENV)
    subprocess.run(["cargo", "build", "--release", "--locked", "-p", "rodas5p-integrators", "--example", EXAMPLE],
                   cwd=ROOT, env=env, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binary = target / "release" / "examples" / EXAMPLE
    if not binary.is_file():
        raise SystemExit(f"example binary not found: {binary}")
    return binary


def command(binary: Path, method: str, arm: str, problem: str, rtol: float, repetitions: int) -> list[str]:
    return [str(binary), "--method", method, "--arm", arm, "--problem", problem, "--rtol", repr(rtol),
            "--repetitions", str(repetitions)]


def run_json(cmd: list[str], env: dict) -> dict:
    out = subprocess.run(cmd, env=env, check=True, capture_output=True, text=True).stdout.strip().splitlines()
    return json.loads(out[-1])


def ir_total(path: Path) -> int:
    events = totals = None
    with open(path) as handle:
        for line in handle:
            if line.startswith("events:"):
                events = line.split()[1:]
            elif line.startswith(("summary:", "totals:")):
                totals = [int(x) for x in line.split()[1:]]
    if events is None or totals is None or "Ir" not in events:
        raise RuntimeError(f"no Ir total in {path}")
    return totals[events.index("Ir")]


def profile(binary: Path, scratch: Path, method: str, arm: str, problem: str, rtol: float, env: dict) -> dict:
    native = run_json(command(binary, method, arm, problem, rtol, 1), env)
    totals, printed = {}, {}
    for tag, k in (("1", 1), ("2", 2), ("1b", 1)):
        out = scratch / f"callgrind.{method}.{arm}.{problem}.{tag}"
        cmd = [
            "valgrind",
            "--tool=callgrind",
            "--enable-debuginfod=no",
            f"--callgrind-out-file={out}",
            *command(binary, method, arm, problem, rtol, k),
        ]
        result = subprocess.run(cmd, env=env, check=True, capture_output=True, text=True)
        record = json.loads(result.stdout.strip().splitlines()[-1])
        record.pop("repetitions")
        printed[tag] = record
        totals[tag] = ir_total(out)
    reference = dict(native)
    reference.pop("repetitions")
    ir = totals["2"] - totals["1"]
    return {
        "problem": problem,
        "rtol": rtol,
        "method": method,
        "arm": arm,
        "native": native,
        "ir_run1": totals["1"],
        "ir_run2": totals["2"],
        "ir_run1_repeat": totals["1b"],
        "callgrind_deterministic": totals["1"] == totals["1b"],
        "callgrind_records_match_native": all(r == reference for r in printed.values()),
        "ir_per_trajectory": ir,
        "ir_per_attempt": ir / native["attempts"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--scratch", type=Path, default=None, help="callgrind output directory (default: temporary)")
    parser.add_argument("--binary", type=Path, default=None, help="use this example binary instead of building it")
    parser.add_argument("--force", action="store_true", help="overwrite existing output file")
    args = parser.parse_args()
    if args.output.exists() and not args.force:
        raise SystemExit(f"output exists: {args.output}")
    commit = git("rev-parse", "HEAD")
    dirty = git("status", "--porcelain", "--untracked-files=no")
    binary = args.binary or build()
    scratch = args.scratch or Path(tempfile.mkdtemp(prefix="sp02_callgrind_"))
    scratch.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, **ENV)
    valgrind = subprocess.run(["valgrind", "--version"], capture_output=True, text=True, check=True).stdout.strip()
    entries = []
    for problem in PROBLEMS:
        for method in METHODS:
            for arm in ARMS:
                entry = profile(binary, scratch, method, arm, problem, RTOL, env)
                entries.append(entry)
                print(f"{problem} {RTOL:g} {method} {arm}: Ir/trajectory {entry['ir_per_trajectory']}, "
                      f"deterministic {entry['callgrind_deterministic']}, "
                      f"records match {entry['callgrind_records_match_native']}", flush=True)
    report = {
        "schema": SCHEMA,
        "node": NODE,
        "commit": commit,
        "tree_clean": dirty == "",
        "valgrind": valgrind,
        "binary": EXAMPLE,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "build": f"CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked -p rodas5p-integrators "
                 f"--example {EXAMPLE}",
        "environment": ENV,
        "protocol": "callgrind Ir of one process with 2 minus 1 integrations of the cell (one trajectory); the "
                    "1-integration run repeated as a determinism check; both arms in the same binary; no "
                    "wall clock",
        "cells": [{"problem": p, "rtol": RTOL, "method": m} for p in PROBLEMS for m in METHODS],
        "entries": entries,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
