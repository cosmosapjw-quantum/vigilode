#!/usr/bin/env python3
"""Same-binary instruction profile of research node SP01 (research/sp01_dupfix_adoption_20261010).

Protocol (SPD01-SPD09, tools/speed_profile.py): callgrind on one process with 1 and with 2 integrations of one cell;
the difference is one trajectory, with process start-up and problem construction cancelled. The 1-repetition run is
repeated once as a determinism check. Both accountings run in the same release binary, the example
`crates/rodas5p-integrators/examples/sp01_dupfix_profile.rs`, which this tool builds first with line tables
(`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked -p rodas5p-integrators --example
sp01_dupfix_profile`; the binary is taken from `$CARGO_TARGET_DIR`, default `target/`). No wall clock: counted
instructions only (Ir).

Profiled cells (registered): van der Pol, HIRES and Brusselator-50 at rtol 1e-6 on the U-form GMRES-into path, and
HIRES at 1e-6 on the sequential K-form, profiled on both K-form entry points of the export (`kform_integrate`, the
protected library driver, and `kform_step`, the adaptive loop over `sequential_matrix_free_step`).

For every cell and accounting the entry records the native 1-repetition work (attempts, counters, final state), the
three callgrind totals, Ir per trajectory (2 minus 1), whether the repeated 1-repetition total is identical, and
whether every callgrind run printed the native record. The output file is immutable.
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

SCHEMA = "vigilode-sp01-dupfix-profile-v1"
ROOT = Path(__file__).resolve().parents[1]
EXAMPLE = "sp01_dupfix_profile"
CELLS = [
    ("uform_into", "van-der-pol-mu1000", 1.0e-6),
    ("uform_into", "hires", 1.0e-6),
    ("uform_into", "brusselator-1d-50", 1.0e-6),
    ("kform_integrate", "hires", 1.0e-6),
    ("kform_step", "hires", 1.0e-6),
]
ACCOUNTINGS = ("recompute_final", "reuse_confirmed")
ENV = {"RAYON_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1"}


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


def command(binary: Path, path: str, problem: str, rtol: float, accounting: str, repetitions: int) -> list[str]:
    return [str(binary), "--path", path, "--problem", problem, "--rtol", repr(rtol), "--accounting", accounting,
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


def profile(binary: Path, scratch: Path, path: str, problem: str, rtol: float, accounting: str, env: dict) -> dict:
    native = run_json(command(binary, path, problem, rtol, accounting, 1), env)
    totals, printed = {}, {}
    for tag, k in (("1", 1), ("2", 2), ("1b", 1)):
        out = scratch / f"callgrind.{path}.{problem}.{accounting}.{tag}"
        cmd = ["valgrind", "--tool=callgrind", f"--callgrind-out-file={out}", *command(binary, path, problem, rtol,
                                                                                         accounting, k)]
        result = subprocess.run(cmd, env=env, check=True, capture_output=True, text=True)
        record = json.loads(result.stdout.strip().splitlines()[-1])
        record.pop("repetitions")
        printed[tag] = record
        totals[tag] = ir_total(out)
    reference = dict(native)
    reference.pop("repetitions")
    ir = totals["2"] - totals["1"]
    return {
        "path": path, "problem": problem, "rtol": rtol, "accounting": accounting,
        "native": native,
        "ir_run1": totals["1"], "ir_run2": totals["2"], "ir_run1_repeat": totals["1b"],
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
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    commit = git("rev-parse", "HEAD")
    dirty = git("status", "--porcelain", "--untracked-files=no")
    binary = args.binary or build()
    scratch = args.scratch or Path(tempfile.mkdtemp(prefix="sp01_callgrind_"))
    scratch.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, **ENV)
    valgrind = subprocess.run(["valgrind", "--version"], capture_output=True, text=True, check=True).stdout.strip()
    entries = []
    for path, problem, rtol in CELLS:
        for accounting in ACCOUNTINGS:
            entry = profile(binary, scratch, path, problem, rtol, accounting, env)
            entries.append(entry)
            print(f"{path} {problem} {rtol:g} {accounting}: Ir/trajectory {entry['ir_per_trajectory']}, "
                  f"deterministic {entry['callgrind_deterministic']}, "
                  f"records match {entry['callgrind_records_match_native']}", flush=True)
    report = {
        "schema": SCHEMA,
        "node": "sp01_dupfix_adoption_20261010",
        "commit": commit,
        "tree_clean": dirty == "",
        "valgrind": valgrind,
        "binary": EXAMPLE,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "build": "CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked -p rodas5p-integrators "
                 f"--example {EXAMPLE}",
        "environment": ENV,
        "protocol": "callgrind Ir of one process with 2 minus 1 integrations of the cell (one trajectory); the "
                    "1-integration run repeated as a determinism check; both accountings in the same binary; no "
                    "wall clock",
        "cells": [{"path": p, "problem": q, "rtol": r} for p, q, r in CELLS],
        "entries": entries,
    }
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
