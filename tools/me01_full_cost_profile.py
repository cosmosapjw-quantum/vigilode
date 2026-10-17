#!/usr/bin/env python3
"""Instruction and work profiling for research node ME01 (full-cost candidate judgement).

Runs Callgrind on target/release/rodas5p stiff-profile-run with repetitions 0, 1, 2, and 1b (repeat).
Records:
- rep0_ir, rep1_ir, rep2_ir, rep1b_ir
- warm_ir (rep2_ir - rep1_ir), cold_ir (rep1_ir - rep0_ir)
- determinism check
- attempts, accepted, rejected, counters, final_state, final_state_sha256
- grid_error and endpoint_error for CT01 arms
Writes research/me01_full_cost_judgement_20261011/PROFILE.json (schema vigilode-me01-profile-v1).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

SCHEMA = "vigilode-me01-profile-v1"
NODE = "me01"

PROBLEMS = ["robertson", "hires", "van-der-pol-mu1000", "brusselator-1d-50", "brusselator-1d-160", "brusselator-1d-200", "brusselator-1d-500"]

def get_git_info() -> tuple[str, bool]:
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    status = subprocess.check_output(["git", "status", "--porcelain=v1"], text=True).strip()
    return commit, len(status) == 0

def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def parse_callgrind_ir(path: Path) -> int:
    totals = None
    with open(path) as f:
        for line in f:
            if line.startswith(("summary:", "totals:")):
                totals = [int(x) for x in line.split()[1:]]
                break
    if totals is None or not totals:
        raise RuntimeError(f"no totals in callgrind output {path}")
    return totals[0]

def all_profile_cells() -> list[tuple[str, str, float]]:
    cells = []
    # Small problems at 1e-6, 1e-8
    for prob in ["robertson", "hires", "van-der-pol-mu1000"]:
        for rtol in [1.0e-6, 1.0e-8]:
            cells.append((prob, "rodas5p-fast-small", rtol))
            cells.append((prob, "rodas5p-fast-small-ovh", rtol))
            cells.append((prob, "rodas5p-fast-small-static-stages", rtol))
            cells.append((prob, "rodas5p-fast", rtol))
    # Brusselator-200 for E3
    for rtol in [1.0e-6, 1.0e-8]:
        cells.append(("brusselator-1d-200", "rodas5p-fast-ovh", rtol))
    # Brusselators for E4, E6 at 1e-6, 1e-8
    for prob in ["brusselator-1d-50", "brusselator-1d-200", "brusselator-1d-500"]:
        for rtol in [1.0e-6, 1.0e-8]:
            cells.append((prob, "rodas5p-fast-banded", rtol))
            cells.append((prob, "rodas5p-fast-colext64", rtol))
            cells.append((prob, "rodas5p-fast", rtol))
    # Matrix-free candidates (E1, E7, E8) on JVP-only Brusselator at 1e-6, 1e-8
    for prob in ["brusselator-1d-50", "brusselator-1d-160"]:
        for rtol in [1.0e-6, 1.0e-8]:
            cells.append((prob, "rodas5p-mf-legacy-v2", rtol))
            cells.append((prob, "rodas5p-mf-legacy", rtol))
            cells.append((prob, "rodas5p-mf-ls-ws", rtol))
            cells.append((prob, "rodas5p-mf-into", rtol))
            cells.append((prob, "rodas5p-mf", rtol))
    # E2: CT01 half-decade rungs:
    # van der Pol and Brusselator-50: 9 rungs 1e-3 to 1e-7 (q = 6..=14)
    for prob in ["van-der-pol-mu1000", "brusselator-1d-50"]:
        for q in range(6, 15):
            rtol = 10.0 ** (-q / 2.0)
            cells.append((prob, "rodas5p-fast-pred2", rtol))
            cells.append((prob, "rodas5p-fast-int-grid", rtol))
    # HIRES and Robertson: 15 rungs 1e-3 to 1e-10 (q = 6..=20)
    for prob in ["hires", "robertson"]:
        for q in range(6, 21):
            rtol = 10.0 ** (-q / 2.0)
            cells.append((prob, "rodas5p-fast-pred2", rtol))
            cells.append((prob, "rodas5p-fast-int-grid", rtol))

    # Deduplicate preserving order
    deduped = []
    for c in cells:
        if not any(c[0] == d[0] and c[1] == d[1] and abs(c[2] - d[2]) < 1e-15 * c[2] for d in deduped):
            deduped.append(c)
    return deduped

def load_ct01_references() -> dict[str, list[list[float]]]:
    ct01_path = Path("research/ct01_controller_grid_holdout_20261010/RUNS.json")
    if not ct01_path.is_file():
        raise RuntimeError(f"CT01 RUNS.json not found at {ct01_path}")
    data = json.loads(ct01_path.read_text())
    refs = {}
    for prob, pdata in data.get("references", {}).items():
        ref_hex = pdata["reference"]["grid_states"]
        ref_floats = [[struct.unpack(">d", bytes.fromhex(x))[0] for x in row] for row in ref_hex]
        refs[prob] = ref_floats
    return refs

def point_error(y: list[float], r: list[float]) -> float:
    return max(abs(a - b) / max(abs(b), 1e-10) for a, b in zip(y, r))

def compute_grid_error(states: list[list[float]], reference: list[list[float]]) -> float:
    # states has 41 elements (index 0 is t0, index 1..40 are the 40 points)
    use = states[1:] if len(states) == len(reference) + 1 else states
    if len(use) != len(reference):
        raise ValueError(f"grid states length {len(use)} != reference length {len(reference)}")
    return max(point_error(y, r) for y, r in zip(use, reference))

def run_callgrind(
    rodas5p: Path, scratch: Path, problem: str, arm: str, rtol: float, repetitions: int, tag: str
) -> tuple[int, dict]:
    cg_out = scratch / f"cg_{problem}_{arm}_{rtol}_{repetitions}_{tag}.out"
    cmd = [
        "valgrind",
        "--enable-debuginfod=no",
        "--tool=callgrind",
        f"--callgrind-out-file={cg_out}",
        str(rodas5p),
        "stiff-profile-run",
        "--problem",
        problem,
        "--arm",
        arm,
        "--rtol",
        str(rtol),
        "--repetitions",
        str(repetitions),
    ]
    env = os.environ.copy()
    env["DEBUGINFOD_URLS"] = ""
    env["RAYON_NUM_THREADS"] = "1"
    env["OPENBLAS_NUM_THREADS"] = "1"
    proc = subprocess.run(cmd, env=env, capture_output=True, text=True, check=True)
    ir = parse_callgrind_ir(cg_out)
    cg_out.unlink(missing_ok=True)
    
    # Parse json line from stdout
    run_json = None
    for line in proc.stdout.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}"):
            try:
                run_json = json.loads(line)
                break
            except Exception:
                pass
    if run_json is None:
        raise RuntimeError(f"failed to parse json output from rodas5p: {proc.stdout}")
    return ir, run_json

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", type=Path, default=Path("target/release/rodas5p"))
    parser.add_argument("--scratch", type=Path, default=Path("/tmp/me01_scratch"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    args.scratch.mkdir(parents=True, exist_ok=True)
    if args.output.is_file():
        print(f"refusing to overwrite existing {args.output}")
        sys.exit(1)

    binary_sha256 = sha256_file(args.rodas5p)
    commit, tree_clean = get_git_info()
    ct01_refs = load_ct01_references()

    cells = all_profile_cells()
    print(f"Profiling {len(cells)} cells with {args.rodas5p}...")

    rows = []
    for i, (prob, arm, rtol) in enumerate(cells):
        is_ct01 = arm in ("rodas5p-fast-pred2", "rodas5p-fast-int-grid")
        print(f"[{i+1}/{len(cells)}] {prob} {arm} rtol={rtol:.2e}...")

        ir0, _ = run_callgrind(args.rodas5p, args.scratch, prob, arm, rtol, 0, "0")
        ir1, run1 = run_callgrind(args.rodas5p, args.scratch, prob, arm, rtol, 1, "1")
        ir2, _ = run_callgrind(args.rodas5p, args.scratch, prob, arm, rtol, 2, "2")
        ir1b, run1b = run_callgrind(args.rodas5p, args.scratch, prob, arm, rtol, 1, "1b")

        deterministic = (ir1 == ir1b) and (run1.get("final_state") == run1b.get("final_state"))
        warm_ir = ir2 - ir1
        cold_ir = ir1 - ir0

        final_state = run1.get("final_state")
        fs_sha256 = hashlib.sha256(json.dumps(final_state).encode()).hexdigest() if final_state is not None else None

        row = {
            "problem": prob,
            "arm": arm,
            "rtol": rtol,
            "rep0_ir": ir0,
            "rep1_ir": ir1,
            "rep2_ir": ir2,
            "rep1b_ir": ir1b,
            "warm_ir": warm_ir,
            "cold_ir": cold_ir,
            "callgrind_deterministic": deterministic,
            "attempts": run1.get("attempts"),
            "accepted_steps": run1.get("accepted_steps"),
            "rejected_steps": run1.get("rejected_steps"),
            "counters": run1.get("counters"),
            "final_state": final_state,
            "final_state_sha256": fs_sha256,
        }
        if "counters_full" in run1:
            row["counters_full"] = run1["counters_full"]
        if is_ct01 and prob in ct01_refs and "grid_states" in run1:
            grid_states = run1["grid_states"]
            row["grid_states"] = grid_states
            row["grid_error"] = compute_grid_error(grid_states, ct01_refs[prob])
            row["endpoint_error"] = point_error(grid_states[-1], ct01_refs[prob][-1])

        rows.append(row)

    doc = {
        "schema": SCHEMA,
        "node": NODE,
        "source_commit": commit,
        "tree_clean": tree_clean,
        "binary_sha256": binary_sha256,
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(doc, indent=2, sort_keys=True) + "\n")
    print(f"Wrote profile with {len(rows)} rows to {args.output}")

if __name__ == "__main__":
    main()
