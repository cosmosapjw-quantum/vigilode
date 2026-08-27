#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess

AUTH = pathlib.Path("docs/exec-plans/k0-stage-telemetry-integration-20260827/FRESH_REVIEW_REPAIR_AUTHORITY.json")
SOURCE_BASE = "e1124586a4029f86669e7489278c61ef676d61aa"
REVIEW_HEAD = "e95ce1e58a603306cb665a6ab91cfe02d279972f"
PACKAGE_REF = "origin/docs/k0-codex-execution-package-20260827"
BRANCH = "research/k0-stage-telemetry-integration-20260827"
ARMS = {"legacy-restarted-gmres", "incremental-givens-candidate"}
FAMILIES = {
    "robertson-ramped", "hires-ramped", "van-der-pol-ramped",
    "rotating-nonnormal", "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
}
CALLSITE_PREFIXES = (
    "crates/rodas5p-integrators/src/k0_stage_telemetry.rs",
    "crates/rodas5p-integrators/src/sequential.rs",
    "crates/rodas5p-integrators/src/a1_two_arm_receipt.rs",
    "crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs",
    "crates/rodas5p-integrators/tests/a1_two_arm_receipt_contracts.rs",
    "crates/rodas5p-cli/src/bin/a1_post_a2a3_kernel_cell.rs",
)


def fail(message: str) -> None:
    raise ValueError(message)


def load(path: pathlib.Path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def digest(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def git(root: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True)
    if check and proc.returncode != 0:
        fail(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc


def nonempty(value) -> bool:
    return isinstance(value, str) and bool(value.strip())


def hex_id(value, length: int) -> bool:
    return isinstance(value, str) and len(value) == length and all(c in "0123456789abcdef" for c in value)


def validate_work(work, where: str) -> None:
    keys = {"linear_matvecs", "diagnostic_matvecs", "operator_applies_total", "telemetry_jvp_overhead", "preserved"}
    if not isinstance(work, dict) or not keys <= set(work):
        fail(f"{where}: incomplete work object")
    for key in keys - {"preserved"}:
        if not isinstance(work[key], int) or work[key] < 0:
            fail(f"{where}: invalid work field {key}")
    if work["operator_applies_total"] != work["linear_matvecs"] + work["diagnostic_matvecs"]:
        fail(f"{where}: operator accounting mismatch")
    if work["preserved"] is not True:
        fail(f"{where}: work not preserved")


def validate_failure(value, where: str, cell: bool) -> None:
    required = {"code", "message", "phase", "command", "exit_code", "log_sha256"}
    if cell:
        required.add("work_preserved")
    if not isinstance(value, dict) or not required <= set(value):
        fail(f"{where}: information-free failure")
    for key in ("code", "message", "phase", "command"):
        if not nonempty(value[key]):
            fail(f"{where}: empty failure field {key}")
    if not hex_id(value["log_sha256"], 64):
        fail(f"{where}: invalid failure log digest")
    if cell and value["work_preserved"] is not True:
        fail(f"{where}: failure work not preserved")


def validate_stage(stage, family: str, arm: str, where: str) -> None:
    if stage.get("schema") != "vigilode-k0-stage-receipt/v2":
        fail(f"{where}: wrong stage schema")
    if stage.get("family") != family or stage.get("kernel_arm") != arm:
        fail(f"{where}: stage identity drift")
    if not isinstance(stage.get("attempt_id"), int) or stage["attempt_id"] < 0:
        fail(f"{where}: bad attempt id")
    if not isinstance(stage.get("stage_index"), int) or not 0 <= stage["stage_index"] <= 7:
        fail(f"{where}: bad stage index")
    if stage.get("claim_class") != "EXPLORATORY_NONAUTHORITATIVE":
        fail(f"{where}: claim drift")
    validate_work(stage.get("work"), where)
    state = stage.get("execution_state")
    if state == "COMPLETE":
        if stage.get("failure") is not None:
            fail(f"{where}: COMPLETE carries failure")
        if not nonempty(stage.get("solver_method")) or stage.get("initial_guess_source") is None:
            fail(f"{where}: incomplete solver identity")
        for key in ("initial_true_residual", "final_true_residual"):
            value = stage.get(key)
            if not isinstance(value, (int, float)) or value < 0:
                fail(f"{where}: missing residual {key}")
        if not hex_id(stage.get("signed_residual_digest"), 64):
            fail(f"{where}: missing signed residual digest")
        if not isinstance(stage.get("geometry"), dict):
            fail(f"{where}: missing geometry")
    elif state in {"ERROR", "STOP_INVALID"}:
        validate_failure(stage.get("failure"), where, cell=False)
    else:
        fail(f"{where}: invalid terminal state")


def validate_cell(cell, root: pathlib.Path, where: str) -> tuple[str, str]:
    if cell.get("schema") != "vigilode-k0-cell-receipt/v2":
        fail(f"{where}: wrong cell schema")
    family, arm = cell.get("family"), cell.get("kernel_arm")
    if family not in FAMILIES or arm not in ARMS:
        fail(f"{where}: invalid identity")
    if cell.get("profile") != "enforced-budget-holdout-320":
        fail(f"{where}: profile drift")
    if cell.get("tolerance") != {"kind": "legacy-fixed", "rtol": 1e-10, "atol": 1e-12}:
        fail(f"{where}: tolerance drift")
    if cell.get("frozen_zeta34_tau") != 13.39706618860016:
        fail(f"{where}: tau drift")
    if cell.get("claim_class") != "EXPLORATORY_NONAUTHORITATIVE":
        fail(f"{where}: claim drift")
    provenance = cell.get("provenance")
    if not isinstance(provenance, dict) or not hex_id(provenance.get("source_head"), 40) or not hex_id(provenance.get("source_tree"), 40):
        fail(f"{where}: source provenance missing")
    stages = cell.get("stages")
    if not isinstance(stages, list):
        fail(f"{where}: stages not an array")
    for index, stage in enumerate(stages):
        validate_stage(stage, family, arm, f"{where}/stage[{index}]")
    state = cell.get("execution_state")
    if state == "COMPLETE":
        raw_rel = provenance.get("raw_receipt_path")
        raw_hash = provenance.get("raw_receipt_sha256")
        if not nonempty(raw_rel) or not hex_id(raw_hash, 64):
            fail(f"{where}: COMPLETE lacks raw receipt provenance")
        raw_path = root / raw_rel
        if not raw_path.is_file() or digest(raw_path) != raw_hash:
            fail(f"{where}: raw receipt missing or digest mismatch")
        if cell.get("failure") is not None:
            fail(f"{where}: COMPLETE carries failure")
        campaign = cell.get("campaign")
        if not isinstance(campaign, dict):
            fail(f"{where}: invented empty COMPLETE cell")
        if campaign.get("raw_schema") != "vigilode-a1-post-a2a3-kernel-atomic-cell-v1":
            fail(f"{where}: raw schema drift")
        if campaign.get("raw_status") != "EXPLORATORY/NONAUTHORITATIVE" or campaign.get("tolerance_arm") != "legacy-fixed":
            fail(f"{where}: raw campaign drift")
        attempts = campaign.get("attempts")
        if not isinstance(attempts, int) or attempts < 1:
            fail(f"{where}: empty campaign")
        if campaign.get("accepted_steps", -1) + campaign.get("rejected_steps", -1) != attempts:
            fail(f"{where}: attempt accounting mismatch")
        gates = campaign.get("hard_gates")
        if not isinstance(gates, dict) or not gates or not all(value is True for value in gates.values()):
            fail(f"{where}: hard gates incomplete")
        if campaign.get("audit_full_e_complete") is not True or campaign.get("unsafe_recommendations") != 0:
            fail(f"{where}: audit/safety gate failed")
        if not hex_id(campaign.get("numerical_payload_sha256"), 64):
            fail(f"{where}: missing numerical payload digest")
        if len(stages) != attempts * 8:
            fail(f"{where}: incomplete stage coverage")
        got = {(row["attempt_id"], row["stage_index"]) for row in stages}
        expected = {(attempt, stage) for attempt in range(attempts) for stage in range(8)}
        if got != expected:
            fail(f"{where}: missing/duplicate stage identity")
    elif state in {"ERROR", "STOP_INVALID"}:
        validate_failure(cell.get("failure"), where, cell=True)
        raw_rel, raw_hash = provenance.get("raw_receipt_path"), provenance.get("raw_receipt_sha256")
        if raw_rel is not None or raw_hash is not None:
            if not nonempty(raw_rel) or not hex_id(raw_hash, 64):
                fail(f"{where}: partial raw provenance")
            raw_path = root / raw_rel
            if not raw_path.is_file() or digest(raw_path) != raw_hash:
                fail(f"{where}: supplied raw receipt mismatch")
    else:
        fail(f"{where}: invalid cell terminal state")
    return arm, family


def check_authority(root: pathlib.Path):
    value = load(root / AUTH)
    ids = {row.get("id") for row in value.get("findings", [])}
    expected = {"FR-K0-P0-001", "FR-K0-P0-002", "FR-K0-P1-001", "FR-K0-P1-002", "FR-K0-P1-003"}
    if value.get("status") != "BOUND" or ids != expected:
        fail("repair authority incomplete")
    if value["api_decision"]["selected"] != "narrow_research_only_doc_hidden_public_bridge":
        fail("API decision drift")
    return {"marker": "FRESH_REVIEW_REPAIR_AUTHORITY_PASS", "findings": 5}


def check_merge(root: pathlib.Path):
    if git(root, "branch", "--show-current").stdout.strip() != BRANCH or git(root, "status", "--porcelain=v1").stdout:
        fail("wrong branch or dirty repair worktree")
    package = git(root, "rev-parse", PACKAGE_REF).stdout.strip()
    parents = git(root, "show", "-s", "--format=%P", "HEAD").stdout.strip().split()
    if parents != [REVIEW_HEAD, package]:
        fail(f"repair parent order mismatch: {parents}")
    return {"marker": "FRESH_REVIEW_REPAIR_MERGE_PASS", "parents": parents}


def check_evidence(root: pathlib.Path, directory: pathlib.Path):
    paths = sorted((root / directory).glob("*.json"))
    if len(paths) != 12:
        fail(f"expected 12 cells, got {len(paths)}")
    identities = set()
    for path in paths:
        identity = validate_cell(load(path), root, str(path))
        if identity in identities:
            fail(f"duplicate cell {identity}")
        identities.add(identity)
    if identities != {(arm, family) for arm in ARMS for family in FAMILIES}:
        fail("2x6 matrix incomplete")
    return {"marker": "EVIDENCE_V2_PASS", "cells": 12}


def check_bridge(root: pathlib.Path):
    for path in ("Cargo.toml", "Cargo.lock", "crates/rodas5p-krylov/Cargo.toml", "crates/rodas5p-integrators/Cargo.toml", "crates/rodas5p-cli/Cargo.toml"):
        if git(root, "diff", "--quiet", SOURCE_BASE, "HEAD", "--", path, check=False).returncode:
            fail(f"forbidden Cargo graph change: {path}")
    receipt = root / "research/k0_stage_telemetry_20260827/review/public_bridge_surface.json"
    if not receipt.is_file():
        fail("missing public bridge surface receipt")
    value = load(receipt)
    if value.get("modules") != ["rodas5p_krylov::k0_research_bridge", "rodas5p_integrators::k0_research_bridge"]:
        fail("bridge module set drift")
    items = value.get("items")
    if not isinstance(items, list) or not items:
        fail("empty bridge surface")
    for item in items:
        if item.get("doc_hidden") is not True or not nonempty(item.get("symbol")):
            fail("non-hidden or unnamed bridge item")
        for site in item.get("call_sites", []):
            if not any(site == prefix or site.startswith(prefix + ":") for prefix in CALLSITE_PREFIXES):
                fail(f"forbidden bridge callsite: {site}")
    return {"marker": "PUBLIC_BRIDGE_PASS", "items": len(items)}


def check_signed_guard(root: pathlib.Path):
    path = root / "crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs"
    if not path.is_file() or "signed_residual_mutation_is_detected" not in path.read_text(encoding="utf-8"):
        fail("missing signed-residual mutation guard")
    return {"marker": "SIGNED_RESIDUAL_GUARD_PASS"}


def self_test():
    common = {
        "schema": "vigilode-k0-cell-receipt/v2",
        "cell_id": "hostile",
        "family": "robertson-ramped",
        "kernel_arm": "legacy-restarted-gmres",
        "profile": "enforced-budget-holdout-320",
        "tolerance": {"kind": "legacy-fixed", "rtol": 1e-10, "atol": 1e-12},
        "frozen_zeta34_tau": 13.39706618860016,
        "provenance": {"source_head": "0" * 40, "source_tree": "0" * 40, "raw_receipt_path": None, "raw_receipt_sha256": None},
        "campaign": None,
        "stages": [],
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE"
    }
    rejected = 0
    for fixture in (
        dict(common, execution_state="COMPLETE", failure=None),
        dict(common, execution_state="ERROR", failure=None),
    ):
        try:
            validate_cell(fixture, pathlib.Path("."), "hostile")
        except ValueError:
            rejected += 1
    detailed = dict(common, execution_state="ERROR", failure={
        "code": "RUNNER_ERROR", "message": "preserved", "phase": "aggregate",
        "command": "atomic-cell", "exit_code": 2, "log_sha256": "1" * 64,
        "work_preserved": True,
    })
    validate_cell(detailed, pathlib.Path("."), "detailed-error")
    if rejected != 2:
        fail("hostile self-test did not reject both weak fixtures")
    return {"marker": "HOSTILE_FIXTURES_PASS", "rejected": 2, "detailed_error_accepted": True}


def main():
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
    try:
        if args.check_authority or not any((args.check_repair_merge, args.evidence_dir, args.check_public_bridge, args.check_signed_residual_guard, args.self_test)):
            results["authority"] = check_authority(root)
        if args.check_repair_merge:
            results["merge"] = check_merge(root)
        if args.evidence_dir:
            results["evidence"] = check_evidence(root, pathlib.Path(args.evidence_dir))
        if args.check_public_bridge:
            results["bridge"] = check_bridge(root)
        if args.check_signed_residual_guard:
            results["signed_guard"] = check_signed_guard(root)
        if args.self_test:
            results["self_test"] = self_test()
    except ValueError as error:
        print(json.dumps({"status": "FAIL", "error": str(error)}, indent=2))
        raise SystemExit(1)
    print(json.dumps({"status": "PASS", "results": results}, indent=2))


if __name__ == "__main__":
    main()
