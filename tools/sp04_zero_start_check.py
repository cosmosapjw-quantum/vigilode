#!/usr/bin/env python3
"""Gate checker for research node research/sp04_zero_start_recycling_20261011 (SP04).

Inputs:
- --base: BASE.json (research/sp04_zero_start_recycling_20261011/BASE.json)
- --runs: RUNS.json (export_runs from zero_start_policy_holdout.rs)
- --profile: PROFILE.json (tools/sp04_zero_start_profile.py)
- --output: RESULTS.json

Exit codes:
- 0: PASS (all gates pass on every feasible cell)
- 1: FAIL (scientifically valid evidence, but one or more gate thresholds fail)
- 2: INVALID (malformed evidence, schema mismatch, violated premises, diff leaks, dirty tree)
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

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence

SCHEMA = "vigilode-sp04-check-v1"
NODE = "sp04_zero_start_recycling_20261011"
BASE_SCHEMA = "vigilode-sp04-base-v1"
RUNS_SCHEMA = "vigilode-sp04-runs-v1"
PROFILE_SCHEMA = "vigilode-sp04-profile-v1"

BASE_SHA256 = "e4afecdab62a83e194da3b52499bcfc5e23c293cc274132c67023f5d757985b5"

PROBLEMS = ["E-normal", "E-nonnormal", "E-nonnormal-strong"]
RTOLS = [1.0e-6, 1.0e-8]
COMPARATOR_ARMS = ["G-Z", "G-P", "G-Z2", "R-P", "C-P", "C-Z", "Lg-P", "L-P"]
ALL_ARMS = ["G-Z", "G-P", "G-Z2", "R-P", "C-P", "C-Z", "Lg-P", "L-P", "R-Z", "R-Zw", "L-Z"]
ALLOWED_DIFF_FILES = {
    "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs",
    "crates/rodas5p-krylov/src/gmres_into.rs",
}

DIMENSION = 256
F_JVP = 2 * DIMENSION * DIMENSION + 7 * DIMENSION
F_RHS = 2 * DIMENSION * DIMENSION + 8 * DIMENSION
TWO_N = 2 * DIMENSION


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()


def run_command_checked(cmd: list[str]) -> tuple[bool, str]:
    try:
        proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
        return True, proc.stdout
    except subprocess.CalledProcessError as e:
        return False, e.stderr + "\n" + e.stdout


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


def hex_to_float(hex_str: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_str))[0]


def recompute_endpoint_error(y_last_hex: list[str], y_exact_hex: list[str]) -> float:
    max_exact = 0.0
    max_diff = 0.0
    for y_h, ye_h in zip(y_last_hex, y_exact_hex):
        y_val = hex_to_float(y_h)
        ye_val = hex_to_float(ye_h)
        max_exact = max(max_exact, abs(ye_val))
        max_diff = max(max_diff, abs(y_val - ye_val))
    return max_diff / max_exact if max_exact > 0.0 else 0.0


def compute_cost(counters: dict) -> float:
    jvp = float(counters.get("jvp_vectors", 0))
    rhs = float(counters.get("rhs_evaluations", 0))
    orth = float(
        counters.get("orthogonalization_inner_products", 0)
        + counters.get("orthogonalization_vector_updates", 0)
    )
    return F_JVP * jvp + F_RHS * rhs + TWO_N * orth


def check_jvp_identity(counters: dict) -> bool:
    jvp = counters.get("jvp_vectors", 0)
    linear = counters.get("linear_matvecs", 0)
    diag = counters.get("diagnostic_matvecs", 0)
    refresh = counters.get("recycle_refresh_matvecs", 0)
    return jvp == (linear + diag + refresh)


def validate_evidence(
    base_raw: bytes,
    runs_raw: bytes,
    profile_raw: bytes,
    base_path: Path,
) -> tuple[bool, list[str], dict, dict, dict]:
    errors = []

    # 1. Base SHA-256 check
    base_sha = hashlib.sha256(base_raw).hexdigest()
    if base_sha != BASE_SHA256:
        errors.append(f"BASE.json SHA-256 mismatch: {base_sha} != {BASE_SHA256}")

    # 2. Strict JSON parsing
    try:
        base_data = evidence.loads_strict(base_raw)
    except Exception as exc:
        errors.append(f"BASE.json strict JSON error: {exc}")
        base_data = {}

    try:
        runs_data = evidence.loads_strict(runs_raw)
    except Exception as exc:
        errors.append(f"RUNS.json strict JSON error: {exc}")
        runs_data = {}

    try:
        profile_data = evidence.loads_strict(profile_raw)
    except Exception as exc:
        errors.append(f"PROFILE.json strict JSON error: {exc}")
        profile_data = {}

    if errors:
        return False, errors, base_data, runs_data, profile_data

    # 3. Schemas and nodes
    if base_data.get("schema") != BASE_SCHEMA:
        errors.append(f"invalid base schema: {base_data.get('schema')}")
    if base_data.get("node") != NODE:
        errors.append(f"invalid base node: {base_data.get('node')}")
    if base_data.get("tree_clean") is not True:
        errors.append(f"base tree_clean must be True, got {base_data.get('tree_clean')}")

    if runs_data.get("schema") != RUNS_SCHEMA:
        errors.append(f"invalid runs schema: {runs_data.get('schema')}")
    if runs_data.get("node") != NODE:
        errors.append(f"invalid runs node: {runs_data.get('node')}")
    if runs_data.get("tree_clean") is not True:
        errors.append(f"runs tree_clean must be True, got {runs_data.get('tree_clean')}")

    if profile_data.get("schema") != PROFILE_SCHEMA:
        errors.append(f"invalid profile schema: {profile_data.get('schema')}")
    if profile_data.get("node") != NODE:
        errors.append(f"invalid profile node: {profile_data.get('node')}")
    if profile_data.get("tree_clean") is not True:
        errors.append(f"profile tree_clean must be True, got {profile_data.get('tree_clean')}")

    # 4. Faer premise
    faer_ok, faer_msg = check_faer_pinned()
    if not faer_ok:
        errors.append(f"faer premise check failed: {faer_msg}")

    # 5. Source commits and git diff BASE..RUNS
    base_commit = base_data.get("source_commit")
    if not base_commit:
        errors.append("BASE.json lacks source_commit")

    runs_commit = runs_data.get("source_commit")
    if not runs_commit:
        errors.append("RUNS.json lacks source_commit")

    if base_commit and runs_commit:
        try:
            diff_proc = subprocess.run(
                ["git", "diff", "--name-only", f"{base_commit}..{runs_commit}", "--", "crates/*/src"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=True,
            )
            touched_files = [line.strip() for line in diff_proc.stdout.splitlines() if line.strip()]
            for f in touched_files:
                if f not in ALLOWED_DIFF_FILES:
                    errors.append(f"git diff BASE..RUNS touched unauthorized file in crates/*/src: {f}")
        except subprocess.CalledProcessError as exc:
            errors.append(f"git diff BASE..RUNS failed: {exc}")

    # 6. Check checker blob at RUNS source commit
    if runs_commit:
        try:
            this_checker = Path(__file__).resolve().read_text()
            repo_checker_proc = subprocess.run(
                ["git", "cat-file", "-p", f"{runs_commit}:tools/sp04_zero_start_check.py"],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            if repo_checker_proc.returncode == 0:
                if repo_checker_proc.stdout.strip() != this_checker.strip():
                    errors.append("checker blob at RUNS commit differs from executed checker")
        except Exception as exc:
            errors.append(f"checker blob verification failed: {exc}")

    # 7. Check rows in BASE.json (exactly 48 rows: 3 problems x 2 rtols x 8 arms)
    base_rows = base_data.get("rows", [])
    expected_base_keys = set()
    for p in PROBLEMS:
        for r in RTOLS:
            for a in COMPARATOR_ARMS:
                expected_base_keys.add((p, r, a))

    actual_base_keys = set()
    for r in base_rows:
        key = (r.get("problem"), r.get("rtol"), r.get("arm"))
        if key in actual_base_keys:
            errors.append(f"duplicate row in BASE: {key}")
        actual_base_keys.add(key)
        # JVP identity
        if not check_jvp_identity(r.get("counters", {})):
            errors.append(f"JVP identity failed in BASE row: {key}")
        # Endpoint error
        if r.get("ok"):
            recomputed = recompute_endpoint_error(r.get("y_last", []), r.get("y_exact", []))
            reported = r.get("error", 0.0)
            if abs(recomputed - reported) > 1e-10:
                errors.append(f"BASE row {key} error mismatch: recomputed={recomputed:.6e} reported={reported:.6e}")

    if actual_base_keys != expected_base_keys:
        missing = expected_base_keys - actual_base_keys
        extra = actual_base_keys - expected_base_keys
        errors.append(f"BASE rows mismatch: missing {len(missing)}, extra {len(extra)}")

    # 8. Check rows in RUNS.json (exactly 66 rows: 3 problems x 2 rtols x 11 arms)
    runs_rows = runs_data.get("rows", [])
    expected_runs_keys = set()
    for p in PROBLEMS:
        for r in RTOLS:
            for a in ALL_ARMS:
                expected_runs_keys.add((p, r, a))

    actual_runs_keys = set()
    for r in runs_rows:
        key = (r.get("problem"), r.get("rtol"), r.get("arm"))
        if key in actual_runs_keys:
            errors.append(f"duplicate row in RUNS: {key}")
        actual_runs_keys.add(key)
        # JVP identity
        if not check_jvp_identity(r.get("counters", {})):
            errors.append(f"JVP identity failed in RUNS row: {key}")
        # Endpoint error
        if r.get("ok"):
            recomputed = recompute_endpoint_error(r.get("y_last", []), r.get("y_exact", []))
            reported = r.get("error", 0.0)
            if abs(recomputed - reported) > 1e-10:
                errors.append(f"RUNS row {key} error mismatch: recomputed={recomputed:.6e} reported={reported:.6e}")

    if actual_runs_keys != expected_runs_keys:
        missing = expected_runs_keys - actual_runs_keys
        extra = actual_runs_keys - expected_runs_keys
        errors.append(f"RUNS rows mismatch: missing {len(missing)}, extra {len(extra)}")

    # 9. Profile entries check
    profile_entries = profile_data.get("entries", [])
    if len(profile_entries) != 6:
        errors.append(f"PROFILE.json entries count mismatch: expected 6, got {len(profile_entries)}")
    for entry in profile_entries:
        key = (entry.get("problem"), entry.get("rtol"), entry.get("arm"))
        if not entry.get("callgrind_deterministic"):
            errors.append(f"callgrind repeat not deterministic for {key}")
        if not entry.get("callgrind_records_match_native"):
            errors.append(f"callgrind records do not match native for {key}")

    return len(errors) == 0, errors, base_data, runs_data, profile_data


def evaluate_gate(base_data: dict, runs_data: dict, profile_data: dict) -> dict:
    base_rows = {(r["problem"], r["rtol"], r["arm"]): r for r in base_data.get("rows", [])}
    runs_rows = {(r["problem"], r["rtol"], r["arm"]): r for r in runs_data.get("rows", [])}
    profiles = {(e["problem"], e["rtol"], e["arm"]): e for e in profile_data.get("entries", [])}

    # Feasibility check: a cell is feasible if at least one comparator arm succeeds with zero linear solve failures
    feasible_cells = []
    infeasible_cells = []
    for p in PROBLEMS:
        for r in RTOLS:
            cell_ok = False
            for a in COMPARATOR_ARMS:
                row = base_rows.get((p, r, a))
                if row and row.get("success") is True and row.get("counters", {}).get("linear_solve_failures", 0) == 0:
                    cell_ok = True
                    break
            if cell_ok:
                feasible_cells.append((p, r))
            else:
                infeasible_cells.append((p, r))

    if len(infeasible_cells) > 2:
        return {
            "verdict": "HOLD",
            "claim": f"HOLD: {len(infeasible_cells)}/6 cells infeasible",
            "feasible_cells": feasible_cells,
            "infeasible_cells": infeasible_cells,
        }

    # Item 1: Existing arms unchanged
    item1_errors = []
    for p, r in feasible_cells:
        for a in COMPARATOR_ARMS:
            b = base_rows[(p, r, a)]
            u = runs_rows[(p, r, a)]
            for field in ["success", "message", "attempts", "accepted", "rejected", "t", "y_last", "counters"]:
                if b.get(field) != u.get(field):
                    item1_errors.append(f"comparator arm {a} mismatch for ({p}, {r:g}) in {field}")

    # Item 2: Separation contract (run contract tests)
    print("Running separation contract and telemetry parity test...", flush=True)
    item2_pass, out2 = run_command_checked([
        "cargo",
        "test",
        "--offline",
        "--locked",
        "-p",
        "rodas5p-integrators",
        "--test",
        "zero_start_policy_holdout",
        "--",
        "separation_contract_and_telemetry_parity",
    ])
    item2_errors = []
    if not item2_pass:
        item2_errors.append(f"separation_contract_and_telemetry_parity test failed: {out2}")

    # Item 3: Current-W contract
    print("Running current-W contract test...", flush=True)
    item3_pass_cmd, out3 = run_command_checked([
        "cargo",
        "test",
        "--offline",
        "--locked",
        "-p",
        "rodas5p-integrators",
        "--test",
        "zero_start_policy_holdout",
        "--",
        "current_w_contract",
    ])
    item3_errors = []
    if not item3_pass_cmd:
        item3_errors.append(f"current_w_contract test failed: {out3}")

    # Verify recycle_update_refreshes == recycle_updates in every R-Z row
    for p, r in feasible_cells:
        rz = runs_rows[(p, r, "R-Z")]
        c = rz.get("counters", {})
        refreshes = c.get("recycle_update_refreshes", 0)
        updates = c.get("recycle_updates", 0)
        if refreshes != updates:
            item3_errors.append(f"R-Z ({p}, {r:g}) recycle_update_refreshes ({refreshes}) != recycle_updates ({updates})")

    # Determine S* for each feasible cell
    s_star_map = {}
    for p, r in feasible_cells:
        best_arm = None
        best_cost = float("inf")
        for a in COMPARATOR_ARMS:
            row = runs_rows[(p, r, a)]
            if row.get("success") is True and row.get("counters", {}).get("linear_solve_failures", 0) == 0:
                cost = row.get("cost", float("inf"))
                if cost < best_cost:
                    best_cost = cost
                    best_arm = a
        s_star_map[(p, r)] = best_arm

    # Item 4: Equal accuracy and failures
    item4_errors = []
    item4_details = {}
    for p, r in feasible_cells:
        s_star = s_star_map[(p, r)]
        row_s = runs_rows[(p, r, s_star)]
        row_rz = runs_rows[(p, r, "R-Z")]

        if row_rz.get("success") is not True:
            item4_errors.append(f"R-Z did not succeed for ({p}, {r:g})")
        if row_rz.get("counters", {}).get("linear_solve_failures", 0) != 0:
            item4_errors.append(f"R-Z had linear solve failures for ({p}, {r:g})")

        err_rz = row_rz.get("error", float("inf"))
        err_s = row_s.get("error", float("inf"))
        err_ratio = err_rz / err_s if err_s > 0.0 else (1.0 if err_rz == 0.0 else float("inf"))
        if err_ratio > 1.500000001:
            item4_errors.append(f"R-Z error exceeds 1.5x S* for ({p}, {r:g}): {err_ratio:.4f}")

        att_rz = row_rz.get("attempts", 0)
        att_s = row_s.get("attempts", 0)
        allowed_att_diff = max(1, math.ceil(0.02 * att_s))
        if abs(att_rz - att_s) > allowed_att_diff:
            item4_errors.append(
                f"R-Z attempts difference {abs(att_rz - att_s)} exceeds allowed {allowed_att_diff} for ({p}, {r:g})"
            )

        item4_details[f"{p}_{r:g}"] = {
            "s_star": s_star,
            "error_s_star": err_s,
            "error_rz": err_rz,
            "error_ratio": err_ratio,
            "attempts_s_star": att_s,
            "attempts_rz": att_rz,
            "allowed_attempts_diff": allowed_att_diff,
        }

    # Item 5: Total work gain C(R-Z) <= 0.90 C(S*)
    item5_errors = []
    item5_details = {}
    all_cells_rz_worse_or_equal = True
    for p, r in feasible_cells:
        s_star = s_star_map[(p, r)]
        c_s = runs_rows[(p, r, s_star)].get("cost", float("inf"))
        c_rz = runs_rows[(p, r, "R-Z")].get("cost", float("inf"))
        ratio = c_rz / c_s if c_s > 0.0 else float("inf")
        if ratio < 1.0:
            all_cells_rz_worse_or_equal = False
        if ratio > 0.900000001:
            item5_errors.append(f"C(R-Z)/C(S*) {ratio:.4f} > 0.90 for ({p}, {r:g}) with S*={s_star}")

        item5_details[f"{p}_{r:g}"] = {
            "s_star": s_star,
            "cost_s_star": c_s,
            "cost_rz": c_rz,
            "cost_ratio": ratio,
        }

    # Item 6: No hidden cost Ir(R-Z) <= 1.000x Ir(S*) on the 3 profiled cells
    item6_errors = []
    item6_details = {}
    for p in PROBLEMS:
        s_star = s_star_map.get((p, 1.0e-6))
        prof_s = profiles.get((p, 1.0e-6, s_star))
        prof_rz = profiles.get((p, 1.0e-6, "R-Z"))
        if not prof_s or not prof_rz:
            item6_errors.append(f"missing profile for ({p}, 1e-6)")
            continue
        ir_s = prof_s.get("ir_per_trajectory", 0)
        ir_rz = prof_rz.get("ir_per_trajectory", 0)
        ratio = ir_rz / ir_s if ir_s > 0 else float("inf")
        if ir_rz > ir_s:
            item6_errors.append(f"Ir(R-Z) {ir_rz} > Ir(S*) {ir_s} (ratio={ratio:.4f}) for {p}")

        item6_details[p] = {
            "s_star": s_star,
            "ir_s_star": ir_s,
            "ir_rz": ir_rz,
            "ir_ratio": ratio,
        }

    item1_pass = len(item1_errors) == 0
    item3_pass = len(item3_errors) == 0
    item4_pass = len(item4_errors) == 0
    item5_pass = len(item5_errors) == 0
    item6_pass = len(item6_errors) == 0

    all_pass = item1_pass and item2_pass and item3_pass and item4_pass and item5_pass and item6_pass
    verdict = "PASS" if all_pass else "FAIL"

    return {
        "verdict": verdict,
        "feasible_cells": feasible_cells,
        "infeasible_cells": infeasible_cells,
        "s_star_by_cell": {f"{p}_{r:g}": arm for (p, r), arm in s_star_map.items()},
        "hypothesis_killed": all_cells_rz_worse_or_equal,
        "items": {
            "1_existing_arms_unchanged": {"pass": item1_pass, "errors": item1_errors},
            "2_separation_contract": {"pass": item2_pass, "errors": item2_errors},
            "3_current_w_contract": {"pass": item3_pass, "errors": item3_errors},
            "4_equal_accuracy_and_failures": {"pass": item4_pass, "errors": item4_errors, "details": item4_details},
            "5_total_work_gain": {"pass": item5_pass, "errors": item5_errors, "details": item5_details},
            "6_no_hidden_cost": {"pass": item6_pass, "errors": item6_errors, "details": item6_details},
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--force", action="store_true", help="overwrite output file")
    args = parser.parse_args()

    if args.output.exists() and not args.force:
        raise SystemExit(f"output file exists: {args.output}")

    if not args.base.is_file():
        raise SystemExit(f"base file not found: {args.base}")
    if not args.runs.is_file():
        raise SystemExit(f"runs file not found: {args.runs}")
    if not args.profile.is_file():
        raise SystemExit(f"profile file not found: {args.profile}")

    base_raw = args.base.read_bytes()
    runs_raw = args.runs.read_bytes()
    profile_raw = args.profile.read_bytes()

    is_valid, validation_errors, base_data, runs_data, profile_data = validate_evidence(
        base_raw, runs_raw, profile_raw, args.base
    )
    if not is_valid:
        report = {
            "schema": SCHEMA,
            "node": NODE,
            "verdict": "INVALID",
            "validation_errors": validation_errors,
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        print("Verdict: INVALID")
        for err in validation_errors:
            print(f"  - {err}")
        return 2

    gate_result = evaluate_gate(base_data, runs_data, profile_data)
    head = git("rev-parse", "HEAD")
    clean = git("status", "--porcelain", "--untracked-files=no") == ""

    report = {
        "schema": SCHEMA,
        "node": NODE,
        "head": head,
        "tree_clean": clean,
        "base_sha256": hashlib.sha256(base_raw).hexdigest(),
        "runs_sha256": hashlib.sha256(runs_raw).hexdigest(),
        "profile_sha256": hashlib.sha256(profile_raw).hexdigest(),
        **gate_result,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")

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
