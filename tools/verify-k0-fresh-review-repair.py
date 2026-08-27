#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile

AUTH = pathlib.Path("docs/exec-plans/k0-stage-telemetry-integration-20260827/FRESH_REVIEW_REPAIR_AUTHORITY.json")
SOURCE_BASE = "e1124586a4029f86669e7489278c61ef676d61aa"
LOCAL_REVIEW_HEAD = "e95ce1e58a603306cb665a6ab91cfe02d279972f"
PACKAGE_REF = "origin/docs/k0-codex-execution-package-20260827"
BRANCH = "research/k0-stage-telemetry-integration-20260827"
ARMS = {"legacy-restarted-gmres", "incremental-givens-candidate"}
FAMILIES = {
    "robertson-ramped", "hires-ramped", "van-der-pol-ramped",
    "rotating-nonnormal", "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
}
ALLOWED_CALLSITE_PREFIXES = (
    "crates/rodas5p-integrators/src/k0_stage_telemetry.rs",
    "crates/rodas5p-integrators/src/sequential.rs",
    "crates/rodas5p-integrators/src/a1_two_arm_receipt.rs",
    "crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs",
    "crates/rodas5p-integrators/tests/a1_two_arm_receipt_contracts.rs",
    "crates/rodas5p-cli/src/bin/a1_post_a2a3_kernel_cell.rs",
)


def die(message: str) -> None:
    print(json.dumps({"status": "FAIL", "error": message}, indent=2))
    raise SystemExit(1)


def load(path: pathlib.Path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def git(root: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True)
    if check and proc.returncode != 0:
        die(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc


def nonempty(value) -> bool:
    return isinstance(value, str) and bool(value.strip())


def valid_hash(value, length: int) -> bool:
    return isinstance(value, str) and len(value) == length and all(c in "0123456789abcdef" for c in value)


def validate_work(work, where: str) -> None:
    if not isinstance(work, dict):
        die(f"{where}: missing work object")
    required = {"linear_matvecs", "diagnostic_matvecs", "operator_applies_total", "telemetry_jvp_overhead", "preserved"}
    if not required <= set(work):
        die(f"{where}: incomplete work object")
    for key in required - {"preserved"}:
        if not isinstance(work[key], int) or work[key] < 0:
            die(f"{where}: invalid work field {key}")
    if work["operator_applies_total"] != work["linear_matvecs"] + work["diagnostic_matvecs"]:
        die(f"{where}: operator accounting mismatch")
    if work["preserved"] is not True:
        die(f"{where}: work not marked preserved")


def validate_failure(failure, where: str, cell: bool = False) -> None:
    if not isinstance(failure, dict):
        die(f"{where}: failure state lacks structured failure")
    required = {"code", "message", "phase", "command", "exit_code", "log_sha256"}
    if cell:
        required.add("work_preserved")
    if not required <= set(failure):
        die(f"{where}: information-free failure object")
    for key in ("code", "message", "phase", "command"):
        if not nonempty(failure[key]):
            die(f"{where}: empty failure field {key}")
    if not valid_hash(failure["log_sha256"], 64):
        die(f"{where}: invalid log digest")
    if cell and failure["work_preserved"] is not True:
        die(f"{where}: failure did not preserve work")


def validate_stage(stage, family: str, arm: str, where: str) -> None:
    if stage.get("schema") != "vigilode-k0-stage-receipt/v2":
        die(f"{where}: wrong stage schema")
    if stage.get("family") != family or stage.get("kernel_arm") != arm:
        die(f"{where}: stage identity drift")
    if not isinstance(stage.get("attempt_id"), int) or stage["attempt_id"] < 0:
        die(f"{where}: invalid attempt")
    if not isinstance(stage.get("stage_index"), int) or not 0 <= stage["stage_index"] <= 7:
        die(f"{where}: invalid stage index")
    if stage.get("claim_class") != "EXPLORATORY_NONAUTHORITATIVE":
        die(f"{where}: claim drift")
    validate_work(stage.get("work"), where)
    state = stage.get("execution_state")
    if state == "COMPLETE":
        if stage.get("failure") is not None:
            die(f"{where}: COMPLETE carries failure")
        if not nonempty(stage.get("solver_method")):
            die(f"{where}: empty COMPLETE solver method")
        if stage.get("initial_guess_source") is None:
            die(f"{where}: missing initial guess source")
        for key in ("initial_true_residual", "final_true_residual"):
            value = stage.get(key)
            if not isinstance(value, (int, float)) or value < 0:
                die(f"{where}: missing COMPLETE residual {key}")
        if not valid_hash(stage.get("signed_residual_digest"), 64):
            die(f"{where}: missing signed residual digest")
        geometry = stage.get("geometry")
        if not isinstance(geometry, dict) or "scaled_nonlinear_remainder" not in geometry:
            die(f"{where}: missing COMPLETE geometry")
    elif state in {"ERROR", "STOP_INVALID"}:
        validate_failure(stage.get("failure"), where)
    else:
        die(f"{where}: invalid terminal state")


def validate_cell(cell, root: pathlib.Path, where: str) -> tuple[str, str]:
    if cell.get("schema") != "vigilode-k0-cell-receipt/v2":
        die(f"{where}: wrong cell schema")
    family, arm = cell.get("family"), cell.get("kernel_arm")
    if family not in FAMILIES or arm not in ARMS:
        die(f"{where}: invalid arm/family")
    if cell.get("profile") != "enforced-budget-holdout-320":
        die(f"{where}: profile drift")
    tolerance = cell.get("tolerance")
    if tolerance != {"kind": "legacy-fixed", "rtol": 1e-10, "atol": 1e-12}:
        die(f"{where}: tolerance drift")
    if cell.get("frozen_zeta34_tau") != 13.39706618860016:
        die(f"{where}: tau drift")
    if cell.get("claim_class") != "EXPLORATORY_NONAUTHORITATIVE":
        die(f"{where}: claim drift")
    provenance = cell.get("provenance")
    if not isinstance(provenance, dict):
        die(f"{where}: missing provenance")
    raw_path = root / provenance.get("raw_receipt_path", "")
    if not raw_path.is_file() or sha256(raw_path) != provenance.get("raw_receipt_sha256"):
        die(f"{where}: raw receipt missing or digest mismatch")
    state = cell.get("execution_state")
    stages = cell.get("stages")
    if not isinstance(stages, list):
        die(f"{where}: stages is not an array")
    for index, stage in enumerate(stages):
        validate_stage(stage, family, arm, f"{where}/stage[{index}]")
    if state == "COMPLETE":
        if cell.get("failure") is not None:
            die(f"{where}: COMPLETE carries failure")
        campaign = cell.get("campaign")
        if not isinstance(campaign, dict):
            die(f"{where}: invented empty COMPLETE cell")
        if campaign.get("raw_schema") != "vigilode-a1-post-a2a3-kernel-atomic-cell-v1":
            die(f"{where}: raw schema drift")
        if campaign.get("raw_status") != "EXPLORATORY/NONAUTHORITATIVE" or campaign.get("tolerance_arm") != "legacy-fixed":
            die(f"{where}: raw campaign identity drift")
        attempts = campaign.get("attempts")
        if not isinstance(attempts, int) or attempts < 1:
            die(f"{where}: empty campaign attempts")
        if campaign.get("accepted_steps", -1) + campaign.get("rejected_steps", -1) != attempts:
            die(f"{where}: attempt accounting mismatch")
        hard = campaign.get("hard_gates")
        if not isinstance(hard, dict) or not hard or not all(value is True for value in hard.values()):
            die(f"{where}: hard gates incomplete")
        if campaign.get("audit_full_e_complete") is not True or campaign.get("unsafe_recommendations") != 0:
            die(f"{where}: audit completeness/safety failure")
        if not valid_hash(campaign.get("numerical_payload_sha256"), 64):
            die(f"{where}: missing numerical payload digest")
        if len(stages) != attempts * 8:
            die(f"{where}: incomplete stage coverage")
        identities = {(stage["attempt_id"], stage["stage_index"]) for stage in stages}
        expected = {(attempt, stage) for attempt in range(attempts) for stage in range(8)}
        if identities != expected:
            die(f"{where}: missing or duplicate stage identity")
    elif state in {"ERROR", "STOP_INVALID"}:
        validate_failure(cell.get("failure"), where, cell=True)
    else:
        die(f"{where}: invalid cell state")
    return arm, family


def check_authority(root: pathlib.Path) -> dict:
    auth = load(root / AUTH)
    if auth.get("schema") != "vigilode-k0-fresh-review-repair-authority/v1" or auth.get("status") != "BOUND":
        die("repair authority is not bound")
    findings = auth.get("findings", [])
    if {finding.get("id") for finding in findings} != {"FR-K0-P0-001", "FR-K0-P0-002", "FR-K0-P1-001", "FR-K0-P1-002", "FR-K0-P1-003"}:
        die("repair authority does not cover all five findings")
    if auth["api_decision"]["selected"] != "narrow_research_only_doc_hidden_public_bridge":
        die("public bridge decision drift")
    return {"status": "PASS", "marker": "FRESH_REVIEW_REPAIR_AUTHORITY_PASS", "findings": 5}


def check_repair_merge(root: pathlib.Path) -> dict:
    if git(root, "branch", "--show-current").stdout.strip() != BRANCH:
        die("wrong implementation branch")
    if git(root, "status", "--porcelain=v1").stdout:
        die("dirty worktree before repair")
    package = git(root, "rev-parse", PACKAGE_REF).stdout.strip()
    parents = git(root, "show", "-s", "--format=%P", "HEAD").stdout.strip().split()
    if parents != [LOCAL_REVIEW_HEAD, package]:
        die(f"repair merge parents {parents} do not match review/package authority")
    return {"status": "PASS", "marker": "FRESH_REVIEW_REPAIR_MERGE_PASS", "review_parent": LOCAL_REVIEW_HEAD, "package_parent": package}


def check_evidence(root: pathlib.Path, directory: pathlib.Path) -> dict:
    cells = sorted((root / directory).glob("*.json"))
    if len(cells) != 12:
        die(f"expected twelve v2 cell receipts, got {len(cells)}")
    identities = set()
    for path in cells:
        identity = validate_cell(load(path), root, str(path))
        if identity in identities:
            die(f"duplicate cell identity {identity}")
        identities.add(identity)
    expected = {(arm, family) for arm in ARMS for family in FAMILIES}
    if identities != expected:
        die("2x6 identity matrix incomplete")
    return {"status": "PASS", "marker": "EVIDENCE_V2_PASS", "cells": 12}


def check_public_bridge(root: pathlib.Path) -> dict:
    for path in ("Cargo.toml", "Cargo.lock", "crates/rodas5p-krylov/Cargo.toml", "crates/rodas5p-integrators/Cargo.toml", "crates/rodas5p-cli/Cargo.toml"):
        if git(root, "diff", "--quiet", SOURCE_BASE, "HEAD", "--", path, check=False).returncode != 0:
            die(f"forbidden Cargo graph change: {path}")
    surface_path = root / "research/k0_stage_telemetry_20260827/review/public_bridge_surface.json"
    if not surface_path.is_file():
        die("missing public bridge surface receipt")
    surface = load(surface_path)
    if surface.get("modules") != ["rodas5p_krylov::k0_research_bridge", "rodas5p_integrators::k0_research_bridge"]:
        die("bridge module set drift")
    for item in surface.get("items", []):
        if item.get("doc_hidden") is not True or not nonempty(item.get("symbol")):
            die("undocumented/non-hidden bridge symbol")
        for callsite in item.get("call_sites", []):
            if not any(callsite == prefix or callsite.startswith(prefix + ":") for prefix in ALLOWED_CALLSITE_PREFIXES):
                die(f"forbidden bridge callsite {callsite}")
    return {"status": "PASS", "marker": "PUBLIC_BRIDGE_PASS", "symbols": len(surface.get("items", []))}


def check_signed_guard(root: pathlib.Path) -> dict:
    path = root / "crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs"
    if not path.is_file() or "signed_residual_mutation_is_detected" not in path.read_text(encoding="utf-8"):
        die("missing vector-aware signed residual mutation guard")
    return {"status": "PASS", "marker": "SIGNED_RESIDUAL_GUARD_PASS"}


def self_test(root: pathlib.Path) -> dict:
    base = {
        "schema": "vigilode-k0-cell-receipt/v2", "cell_id": "x", "family": "robertson-ramped",
        "kernel_arm": "legacy-restarted-gmres", "profile": "enforced-budget-holdout-320",
        "tolerance": {"kind": "legacy-fixed", "rtol": 1e-10, "atol": 1e-12},
        "frozen_zeta34_tau": 13.39706618860016,
        "provenance": {"raw_receipt_path": "missing", "raw_receipt_sha256": "0" * 64, "source_head": "0" * 40, "source_tree": "0" * 40},
        "campaign": None, "stages": [], "failure": None, "claim_class": "EXPLORATORY_NONAUTHORITATIVE"
    }
    for state in ("COMPLETE", "ERROR"):
        fixture = dict(base, execution_state=state)
        try:
            with tempfile.TemporaryDirectory() as tmp:
                validate_cell(fixture, pathlib.Path(tmp), "hostile")
        except SystemExit:
            continue
        die(f"self-test failed: accepted information-free {state}")
    return {"status": "PASS", "marker": "HOSTILE_FIXTURES_PASS", "rejected": 2}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--check-authority", action="store_true")
    parser.add_argument("--check-repair-merge", action="store_true")
    parser.add_argument("--evidence-dir")
    parser.add_argument("--check-public-bridge", action="store_true")
    parser.add_argument("--check-signed-residual-guard", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = pathlib.Path(args.repo_root).resolve()
    results = {}
    if args.check_authority or not any((args.check_repair_merge, args.evidence_dir, args.check_public_bridge, args.check_signed_residual_guard, args.self_test)):
        results["authority"] = check_authority(root)
    if args.check_repair_merge:
        results["merge"] = check_repair_merge(root)
    if args.evidence_dir:
        results["evidence"] = check_evidence(root, pathlib.Path(args.evidence_dir))
    if args.check_public_bridge:
        results["bridge"] = check_public_bridge(root)
    if args.check_signed_residual_guard:
        results["signed_guard"] = check_signed_guard(root)
    if args.self_test:
        results["self_test"] = self_test(root)
    print(json.dumps({"status": "PASS", "results": results}, indent=2))


if __name__ == "__main__":
    main()
