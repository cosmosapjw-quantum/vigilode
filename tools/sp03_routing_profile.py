#!/usr/bin/env python3
"""Instruction profile of research node SP03 (research/sp03_declared_structure_routing_20261010).

Builds the release CLI with line tables (``CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p
rodas5p-cli --locked``, in ``$CARGO_TARGET_DIR`` or ``target/``) unless ``--rodas5p`` names a binary, and runs the
callgrind protocol of ``tools/speed_profile.py`` (``profile_arm``: ``stiff-profile-run`` with 1 and 2 repetitions,
the difference is one trajectory; the 1-repetition run repeated as a determinism check) on the registered entries:

* every Brusselator cell (brusselator-1d-50, -160, -200, -500; rtol 1e-6 and 1e-8): ``rodas5p-routed`` (declared
  band), ``rodas5p-fast-banded``, ``rodas5p-fast`` (dense v2), ``rodas5p-fast-colext64`` and the matrix-free
  ``rodas5p-mf-legacy`` (gate item 3 compares the routed arm with it, so it is profiled too);
* robertson, hires, van-der-pol-mu1000 (rtol 1e-6 and 1e-8): ``rodas5p-routed`` (declared Dense) and
  ``rodas5p-fast``;
* brusselator-1d-160 at rtol 1e-6 with no declaration: ``rodas5p-routed-undeclared`` (the matrix-free route); its
  direct target is the ``rodas5p-mf-legacy`` entry of the same cell.

``RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`` for every process. Entries run in parallel processes (``--jobs``);
callgrind counts are per process and do not depend on that. For every entry one extra native run of the
1-repetition command records the routing record, every counter and the final state (bound to RUNS.json by the
checker). Counted instructions only; no wall clock. The output file is immutable.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA = "vigilode-sp03-routing-profile-v1"
HERE = Path(__file__).resolve().parent
REPO = HERE.parent

RTOLS = (1.0e-6, 1.0e-8)
BRUSSELATORS = ("brusselator-1d-50", "brusselator-1d-160", "brusselator-1d-200", "brusselator-1d-500")
SMALL = ("robertson", "hires", "van-der-pol-mu1000")
BRUSSELATOR_ARMS = ("rodas5p-routed", "rodas5p-fast-banded", "rodas5p-fast", "rodas5p-fast-colext64",
                    "rodas5p-mf-legacy")
SMALL_ARMS = ("rodas5p-routed", "rodas5p-fast")
UNDECLARED = ("rodas5p-routed-undeclared", "brusselator-1d-160", 1.0e-6)


def registered_entries() -> list[tuple[str, str, float]]:
    entries = []
    for problem in BRUSSELATORS:
        for rtol in RTOLS:
            for arm in BRUSSELATOR_ARMS:
                entries.append((arm, problem, rtol))
    for problem in SMALL:
        for rtol in RTOLS:
            for arm in SMALL_ARMS:
                entries.append((arm, problem, rtol))
    entries.append(UNDECLARED)
    return entries


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build(env: dict) -> Path:
    env = dict(env, CARGO_PROFILE_RELEASE_DEBUG="line-tables-only")
    subprocess.run(["cargo", "build", "--release", "-p", "rodas5p-cli", "--locked"], cwd=REPO, env=env, check=True)
    target = Path(env.get("CARGO_TARGET_DIR", REPO / "target"))
    return target / "release" / "rodas5p"


def profile_one(binary: Path, scratch: Path, top: int, keep: bool, entry: tuple[str, str, float]) -> dict:
    arm, problem, rtol = entry
    speed = load("speed_profile")
    stiff = speed.load_stiff_profile()
    args = argparse.Namespace(rodas5p=str(binary), rtol=rtol, scratch=scratch / f"{arm}.{problem}.{rtol!r}",
                              top=top, cache_sim=False, keep_callgrind=False)
    args.scratch.mkdir(parents=True, exist_ok=True)
    result = speed.profile_arm(stiff, args, arm, problem, REPO)
    native = stiff.run_json([str(binary), "stiff-profile-run", "--problem", problem, "--arm", arm,
                             "--rtol", repr(rtol), "--repetitions", "1"])
    result["native_run"] = {
        "success": native["success"], "attempts": native["attempts"],
        "accepted_steps": native["accepted_steps"], "rejected_steps": native["rejected_steps"],
        "deterministic": native["deterministic"], "final_state": native["final_state"],
        "counters_full": native.get("counters_full"), "routing": native.get("routing"),
    }
    if not keep:
        shutil.rmtree(args.scratch, ignore_errors=True)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rodas5p", type=Path, help="release CLI with line tables (default: build it)")
    parser.add_argument("--scratch", type=Path, help="callgrind scratch directory (default: a temporary one)")
    parser.add_argument("--jobs", type=int, default=min(3, os.cpu_count() or 1))
    parser.add_argument("--top", type=int, default=15)
    parser.add_argument("--keep-callgrind", action="store_true")
    parser.add_argument("--label", default="")
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    env = dict(os.environ, RAYON_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
    os.environ.update(RAYON_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
    binary = args.rodas5p or build(env)
    scratch = args.scratch or Path(tempfile.mkdtemp(prefix="sp03-profile-"))
    scratch.mkdir(parents=True, exist_ok=True)
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True).stdout.strip()
    dirty = bool(subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=REPO,
                                capture_output=True, text=True).stdout.strip())
    valgrind = subprocess.run(["valgrind", "--version"], capture_output=True, text=True, check=True).stdout.strip()
    entries = registered_entries()
    # Longest first (the matrix-free arm at n = 1000), for packing.
    order = sorted(entries, key=lambda e: (e[0] != "rodas5p-mf-legacy", -int(e[1].split("-")[-1])
                                           if e[1].startswith("brusselator") else 0))
    results: dict = {}
    with concurrent.futures.ProcessPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        futures = {pool.submit(profile_one, binary, scratch, args.top, args.keep_callgrind, e): e for e in order}
        for future in concurrent.futures.as_completed(futures):
            entry = futures[future]
            result = future.result()
            results[entry] = result
            if result.get("success"):
                print(f"{entry[0]} {entry[1]} {entry[2]:g}: attempts {result['attempts']}, Ir/run "
                      f"{result['ir_per_run']}, deterministic {result['callgrind_deterministic']}", flush=True)
            else:
                print(f"{entry[0]} {entry[1]} {entry[2]:g}: FAILED", flush=True)
    report = {
        "schema": SCHEMA, "label": args.label, "commit": commit, "worktree_dirty": dirty, "valgrind": valgrind,
        "binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
        "environment": {"RAYON_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1"},
        "protocol": "tools/speed_profile.py profile_arm: callgrind Ir of stiff-profile-run with 2 minus 1 "
                    "repetitions (one trajectory); 1-repetition run repeated as determinism check; no wall clock",
        "entries": [results[e] for e in entries],
    }
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
