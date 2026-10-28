#!/usr/bin/env python3
"""Gate checker for research node research/sp02_krylov_workspace_caller_20261011 (SP02).

Inputs: RUNS.json (export_runs from krylov_workspace_driver_contract) and PROFILE.json (sp02_workspace_profile.py).
Evaluates the gate conditions specified in PREREGISTRATION.md:
1. Parity across all 56 gated runs (bitwise identical states, times, counters, and solve logs).
2. Caller contracts (all contract tests and charge contract tests pass).
3. Full-trajectory allocations (<= 0.10x control on non-HIRES, strictly fewer on all including HIRES).
4. Peak memory is charged (candidate peak <= control peak + added charged bytes).
5. Instructions (Ir per trajectory <= 1.000x control on all 12 profiles).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence

SCHEMA = "vigilode-sp02-workspace-check-v1"
NODE = "sp02_krylov_workspace_caller_20261011"

PROBLEMS = [
    "robertson",
    "van-der-pol-mu1000",
    "hires",
    "prothero-robinson-forced",
    "quadratic-4",
    "brusselator-1d-50",
    "brusselator-1d-160",
]
NON_HIRES_PROBLEMS = [
    "robertson",
    "van-der-pol-mu1000",
    "prothero-robinson-forced",
    "quadratic-4",
    "brusselator-1d-50",
    "brusselator-1d-160",
]
RTOLS = [1.0e-6, 1.0e-8]
METHODS = ["gmres", "lgmres"]
PROFILED_PROBLEMS = ["van-der-pol-mu1000", "hires", "brusselator-1d-50"]


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()


def check_faer_pinned() -> tuple[bool, str]:
    cargo_lock = ROOT / "Cargo.lock"
    if not cargo_lock.exists():
        return False, "Cargo.lock does not exist"
    content = cargo_lock.read_text()
    match = re.search(r'\[\[package\]\]\nname = "faer"\nversion = "([^"]+)"', content)
    if not match:
        return False, "faer package not found in Cargo.lock"
    version = match.group(1)
    if version != "0.24.4":
        return False, f"faer version is {version}, expected 0.24.4"
    return True, version


def validate_evidence(runs_data: dict, profile_data: dict) -> tuple[bool, list[str]]:
    errors = []
    if runs_data.get("schema") != "vigilode-sp02-workspace-runs-v1":
        errors.append(f"invalid runs schema: {runs_data.get('schema')}")
    if runs_data.get("node") != NODE:
        errors.append(f"invalid runs node: {runs_data.get('node')}")
    if runs_data.get("faer_pinned") != "0.24.4":
        errors.append(f"invalid runs faer_pinned: {runs_data.get('faer_pinned')}")
    if runs_data.get("test_threads") != 1:
        errors.append(f"runs test_threads must be 1, got {runs_data.get('test_threads')}")
    if runs_data.get("rayon_num_threads") != "1":
        errors.append(f"runs rayon_num_threads must be '1', got {runs_data.get('rayon_num_threads')}")
    if runs_data.get("openblas_num_threads") != "1":
        errors.append(f"runs openblas_num_threads must be '1', got {runs_data.get('openblas_num_threads')}")

    faer_ok, faer_msg = check_faer_pinned()
    if not faer_ok:
        errors.append(f"faer premise check failed: {faer_msg}")

    # Check rows in runs_data
    rows = runs_data.get("rows", [])
    expected_rows = set()
    for p in PROBLEMS:
        for r in RTOLS:
            expected_rows.add((p, r, "gmres", "control"))
            expected_rows.add((p, r, "gmres", "candidate"))
            expected_rows.add((p, r, "lgmres", "control"))
            expected_rows.add((p, r, "lgmres", "candidate"))
            expected_rows.add((p, r, "lgmres", "attribution"))

    actual_rows = set()
    for row in rows:
        key = (row.get("problem"), row.get("rtol"), row.get("method"), row.get("arm"))
        if key in actual_rows:
            errors.append(f"duplicate row in runs: {key}")
        actual_rows.add(key)

    missing_rows = expected_rows - actual_rows
    if missing_rows:
        errors.append(f"missing {len(missing_rows)} rows in runs: {sorted(list(missing_rows))}")

    # Check profile_data
    if profile_data.get("schema") != "vigilode-sp02-workspace-profile-v1":
        errors.append(f"invalid profile schema: {profile_data.get('schema')}")
    if profile_data.get("node") != NODE:
        errors.append(f"invalid profile node: {profile_data.get('node')}")
    if profile_data.get("tree_clean") is not True:
        errors.append(f"profile tree_clean must be True, got {profile_data.get('tree_clean')}")

    entries = profile_data.get("entries", [])
    expected_profiles = set()
    for p in PROFILED_PROBLEMS:
        for m in METHODS:
            for arm in ["control", "candidate"]:
                expected_profiles.add((p, 1.0e-6, m, arm))

    actual_profiles = set()
    runs_by_key = {}
    for row in rows:
        key = (row.get("problem"), row.get("rtol"), row.get("method"), row.get("arm"))
        runs_by_key[key] = row

    for entry in entries:
        key = (entry.get("problem"), entry.get("rtol"), entry.get("method"), entry.get("arm"))
        if key in actual_profiles:
            errors.append(f"duplicate entry in profile: {key}")
        actual_profiles.add(key)

        if not entry.get("callgrind_deterministic"):
            errors.append(f"profile entry not deterministic: {key}")
        if not entry.get("callgrind_records_match_native"):
            errors.append(f"profile entry callgrind records do not match native: {key}")

        # Check native matches RUNS record
        runs_rec = runs_by_key.get(key)
        if runs_rec is not None:
            native = entry.get("native", {})
            if native.get("attempts") != runs_rec.get("attempts"):
                errors.append(f"profile native attempts mismatch for {key}: {native.get('attempts')} vs {runs_rec.get('attempts')}")
            if native.get("accepted_steps") != runs_rec.get("accepted_steps"):
                errors.append(f"profile native accepted_steps mismatch for {key}")
            if native.get("rejected_steps") != runs_rec.get("rejected_steps"):
                errors.append(f"profile native rejected_steps mismatch for {key}")
            if native.get("final_state") != runs_rec.get("final_state"):
                errors.append(f"profile native final_state mismatch for {key}")
            if native.get("counters") != runs_rec.get("counters"):
                errors.append(f"profile native counters mismatch for {key}")

    missing_profiles = expected_profiles - actual_profiles
    if missing_profiles:
        errors.append(f"missing {len(missing_profiles)} profiles: {sorted(list(missing_profiles))}")

    return len(errors) == 0, errors


def run_command_checked(cmd: list[str]) -> tuple[bool, str]:
    try:
        proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
        return True, proc.stdout
    except subprocess.CalledProcessError as e:
        return False, e.stderr + "\n" + e.stdout


def evaluate_gate(runs_data: dict, profile_data: dict) -> dict:
    rows = runs_data.get("rows", [])
    runs_map = {}
    for row in rows:
        key = (row["problem"], row["rtol"], row["method"], row["arm"])
        runs_map[key] = row

    profiles_map = {}
    for entry in profile_data.get("entries", []):
        key = (entry["problem"], entry["rtol"], entry["method"], entry["arm"])
        profiles_map[key] = entry

    # Check controls success count
    gmres_ctrl_success = sum(1 for p in PROBLEMS for r in RTOLS if runs_map.get((p, r, "gmres", "control"), {}).get("success"))
    lgmres_ctrl_success = sum(1 for p in PROBLEMS for r in RTOLS if runs_map.get((p, r, "lgmres", "control"), {}).get("success"))

    if gmres_ctrl_success < 10 or lgmres_ctrl_success < 10:
        smaller = min(gmres_ctrl_success, lgmres_ctrl_success)
        return {
            "verdict": "HOLD",
            "claim": f"HOLD: {smaller}/14 controls",
            "gmres_controls_success": gmres_ctrl_success,
            "lgmres_controls_success": lgmres_ctrl_success,
        }

    # 1. Parity Check
    parity_errors = []
    for p in PROBLEMS:
        for r in RTOLS:
            for m in METHODS:
                ctrl = runs_map[(p, r, m, "control")]
                cand = runs_map[(p, r, m, "candidate")]
                for field in ["success", "message", "attempts", "accepted_steps", "rejected_steps",
                              "output_times", "final_state", "counters", "stage_solve_log_sha256"]:
                    if ctrl.get(field) != cand.get(field):
                        parity_errors.append(f"parity mismatch for ({p}, {r}, {m}) in {field}")

    # 2. Caller Contracts
    print("Running krylov_workspace_driver_contract tests...", flush=True)
    contract_ok, contract_out = run_command_checked([
        "cargo", "test", "--offline", "--locked", "-p", "rodas5p-integrators", "--test", "krylov_workspace_driver_contract"
    ])
    print("Running sp02_workspace_charge tests...", flush=True)
    charge_ok, charge_out = run_command_checked([
        "cargo", "test", "--offline", "--locked", "-p", "rodas5p-krylov", "--test", "sp02_workspace_charge"
    ])
    contracts_pass = contract_ok and charge_ok
    contract_errors = []
    if not contract_ok:
        contract_errors.append("krylov_workspace_driver_contract failed")
    if not charge_ok:
        contract_errors.append("sp02_workspace_charge failed")

    # 3. Full-trajectory Allocations
    alloc_errors = []
    alloc_ratios = {}
    for p in PROBLEMS:
        for r in RTOLS:
            for m in METHODS:
                ctrl = runs_map[(p, r, m, "control")]
                cand = runs_map[(p, r, m, "candidate")]
                if not ctrl.get("success"):
                    continue
                ctrl_alloc = ctrl.get("allocations", 0)
                cand_alloc = cand.get("allocations", 0)
                ratio = cand_alloc / ctrl_alloc if ctrl_alloc > 0 else float("inf")
                alloc_ratios[f"{p}_{r:g}_{m}"] = {
                    "control": ctrl_alloc,
                    "candidate": cand_alloc,
                    "ratio": ratio,
                }
                # Must be strictly fewer everywhere
                if cand_alloc >= ctrl_alloc:
                    alloc_errors.append(f"allocations not strictly fewer for ({p}, {r}, {m}): cand={cand_alloc} >= ctrl={ctrl_alloc}")
                # Must be <= 0.10x on non-HIRES
                if p in NON_HIRES_PROBLEMS and ratio > 0.10:
                    alloc_errors.append(f"allocation ratio > 0.10 for ({p}, {r}, {m}): {ratio:.4f}")

    # 4. Peak Memory Charged
    peak_errors = []
    peak_comparisons = {}
    for p in PROBLEMS:
        for r in RTOLS:
            for m in METHODS:
                ctrl = runs_map[(p, r, m, "control")]
                cand = runs_map[(p, r, m, "candidate")]
                ctrl_peak = ctrl.get("peak_live_bytes", 0)
                cand_peak = cand.get("peak_live_bytes", 0)
                cand_bytes = cand.get("krylov_workspace_bytes", {})
                if m == "gmres":
                    added_charged = cand_bytes.get("least_squares_bytes", 0)
                else:
                    added_charged = (
                        cand_bytes.get("least_squares_bytes", 0)
                        + cand_bytes.get("lgmres_into_bytes", 0)
                        + cand_bytes.get("attempt_snapshot_bytes", 0)
                    )
                peak_comparisons[f"{p}_{r:g}_{m}"] = {
                    "control_peak": ctrl_peak,
                    "candidate_peak": cand_peak,
                    "added_charged": added_charged,
                    "allowed_ceiling": ctrl_peak + added_charged,
                }
                if cand_peak > ctrl_peak + added_charged:
                    peak_errors.append(
                        f"peak memory exceeded for ({p}, {r}, {m}): cand_peak={cand_peak} > ctrl_peak={ctrl_peak} + charged={added_charged}"
                    )

    # 5. Instructions (Ir)
    ir_errors = []
    ir_ratios = {}
    for p in PROFILED_PROBLEMS:
        for m in METHODS:
            prof_ctrl = profiles_map[(p, 1.0e-6, m, "control")]
            prof_cand = profiles_map[(p, 1.0e-6, m, "candidate")]
            ctrl_ir = prof_ctrl["ir_per_trajectory"]
            cand_ir = prof_cand["ir_per_trajectory"]
            ratio = cand_ir / ctrl_ir if ctrl_ir > 0 else float("inf")
            ir_ratios[f"{p}_{m}"] = {
                "control_ir": ctrl_ir,
                "candidate_ir": cand_ir,
                "ratio": ratio,
            }
            if cand_ir > ctrl_ir:
                ir_errors.append(f"instruction regression for ({p}, {m}): cand_ir={cand_ir} > ctrl_ir={ctrl_ir} (ratio={ratio:.4f})")

    item1_pass = len(parity_errors) == 0
    item2_pass = contracts_pass
    item3_pass = len(alloc_errors) == 0
    item4_pass = len(peak_errors) == 0
    item5_pass = len(ir_errors) == 0

    all_pass = item1_pass and item2_pass and item3_pass and item4_pass and item5_pass
    verdict = "PASS" if all_pass else "FAIL"

    return {
        "verdict": verdict,
        "items": {
            "1_parity": {"pass": item1_pass, "errors": parity_errors},
            "2_caller_contracts": {"pass": item2_pass, "errors": contract_errors},
            "3_full_trajectory_allocations": {"pass": item3_pass, "errors": alloc_errors, "ratios": alloc_ratios},
            "4_peak_memory_charged": {"pass": item4_pass, "errors": peak_errors, "peaks": peak_comparisons},
            "5_instructions": {"pass": item5_pass, "errors": ir_errors, "ratios": ir_ratios},
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--force", action="store_true", help="overwrite output")
    args = parser.parse_args()

    if args.output.exists() and not args.force:
        raise SystemExit(f"output file exists: {args.output}")

    if not args.runs.is_file():
        raise SystemExit(f"runs file not found: {args.runs}")
    if not args.profile.is_file():
        raise SystemExit(f"profile file not found: {args.profile}")

    runs_data = json.loads(args.runs.read_text())
    profile_data = json.loads(args.profile.read_text())

    is_valid, validation_errors = validate_evidence(runs_data, profile_data)
    if not is_valid:
        report = {
            "schema": SCHEMA,
            "node": NODE,
            "verdict": "INVALID",
            "validation_errors": validation_errors,
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=1) + "\n")
        print("Verdict: INVALID")
        for err in validation_errors:
            print(f"  - {err}")
        return 2

    gate_result = evaluate_gate(runs_data, profile_data)
    head = git("rev-parse", "HEAD")
    clean = git("status", "--porcelain", "--untracked-files=no") == ""

    report = {
        "schema": SCHEMA,
        "node": NODE,
        "verdict": gate_result["verdict"],
        "head": head,
        "tree_clean": clean,
        "runs_file": str(args.runs),
        "profile_file": str(args.profile),
        "runs_sha256": hashlib.sha256(args.runs.read_bytes()).hexdigest(),
        "profile_sha256": hashlib.sha256(args.profile.read_bytes()).hexdigest(),
        **gate_result,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=1) + "\n")

    print(f"Verdict: {gate_result['verdict']}")
    if "items" in gate_result:
        for k, v in gate_result["items"].items():
            print(f"  {k}: {'PASS' if v['pass'] else 'FAIL'}")
            if not v["pass"]:
                for err in v.get("errors", []):
                    print(f"    - {err}")

    return 0 if gate_result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
