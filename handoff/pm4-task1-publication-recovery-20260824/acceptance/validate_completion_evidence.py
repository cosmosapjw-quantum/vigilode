from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA64 = re.compile(r"^[0-9a-f]{64}$")
MAIN = "140f6b5c078c3d8fcd5b6c52310c063ee233dc12"
PREVIOUS = "b2d5ec41cb147e01aadbc9c42928da8abfa75c58"
R4 = "6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333"
PATCH = "705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3"
SCRIPT = "63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7"
EXPECTED_STATUS = [
    {"status": "M", "path": "crates/rodas5p-integrators/src/lib.rs"},
    {"status": "A", "path": "crates/rodas5p-integrators/src/v38d_performance_tournament.rs"},
    {"status": "A", "path": "crates/rodas5p-integrators/tests/v38d_performance_probe_contracts.rs"},
    {"status": "D", "path": "research/generic_v38d_high_entropy_performance_tournament/RECOVERY_START.md"},
]
REQUIRED_COMMAND_IDS = {
    "r4_sidecar", "archive_authority_contract", "handoff_completeness_contract",
    "completion_evidence_contract", "vendor_helper_unit", "vendor_acceptance_contract",
    "publication_script_contract", "cargo_metadata_frozen", "task1_focused_tests",
    "all_targets_no_run", "clippy", "rustfmt", "git_diff_check", "git_status_clean",
    "transaction_rehearsal", "deterministic_repack", "remote_ref_recheck",
    "remote_verification",
}


class EvidenceError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def validate(e: dict) -> None:
    require(e.get("schema") == "vigilode-pm4-task1-r5-completion-evidence-v1", "schema mismatch")
    require(e.get("repository") == "cosmosapjw-quantum/vigilode", "repository mismatch")
    require(e.get("target_pull_request") == 11, "PR mismatch")
    require(e.get("r4_archive_sha256") == R4, "R4 hash mismatch")
    require(e.get("base_sha") == MAIN, "main mismatch")
    require(e.get("previous_feature_sha") == PREVIOUS, "previous feature mismatch")
    require(SHA40.fullmatch(str(e.get("published_feature_sha", ""))) is not None, "published feature SHA invalid")
    require(SHA40.fullmatch(str(e.get("published_tree_sha", ""))) is not None, "published tree SHA invalid")
    require(e.get("git_status_porcelain") == "", "worktree not clean")

    authority = e.get("r4_archive_authority_check", {})
    for key in ("passed", "sidecar_verified", "internal_manifest_verified", "withdrawn_hash_rejected"):
        require(authority.get(key) is True, f"archive authority {key} not true")
    require(authority.get("task1_patch_sha256") == PATCH, "authority patch mismatch")
    require(authority.get("r4_publication_script_sha256") == SCRIPT, "authority script mismatch")
    require(e.get("task1_patch_sha256") == PATCH, "Task-1 patch mismatch")

    statuses = e.get("staged_and_final_name_status", {})
    require(statuses.get("staged") == EXPECTED_STATUS, "staged surface mismatch")
    require(statuses.get("final") == EXPECTED_STATUS, "final surface mismatch")

    hashes = e.get("final_task1_file_sha256_results", {})
    expected_hash_paths = {item["path"] for item in EXPECTED_STATUS if item["status"] != "D"}
    require(set(hashes) == expected_hash_paths, "final file hash path set mismatch")
    require(all(SHA64.fullmatch(str(value)) for value in hashes.values()), "final file SHA invalid")

    vendor = e.get("vendor_validation_json", {})
    require(vendor.get("schema") == "vigilode-cargo-directory-source-validation-v1", "vendor schema mismatch")
    require(vendor.get("exact_package_count_enforced") is False, "exact vendor count was enforced")
    require(vendor.get("missing_checksum_packages") == [], "vendor missing checksums")
    require("faer-0.24.4" in vendor.get("required_packages_present", []), "faer missing")
    require(vendor.get("package_directory_count") == vendor.get("checksum_record_count"), "package/checksum count mismatch")
    require(SHA64.fullmatch(str(e.get("cargo_metadata_json_sha256", ""))) is not None, "metadata SHA invalid")

    logs = e.get("all_command_logs_and_exit_codes", [])
    require(isinstance(logs, list), "command logs must be a list")
    ids = set()
    for row in logs:
        require(row.get("exit_code") == 0, f"command failed: {row.get('command_id')}")
        require(SHA64.fullmatch(str(row.get("log_sha256", ""))) is not None, "log SHA invalid")
        ids.add(row.get("command_id"))
    require(REQUIRED_COMMAND_IDS <= ids, f"missing command evidence: {sorted(REQUIRED_COMMAND_IDS - ids)}")

    artifact = e.get("r5_artifact", {})
    require(artifact.get("deterministic_repack_byte_identical") is True, "R5 repack not byte-identical")
    require(SHA64.fullmatch(str(artifact.get("archive_sha256", ""))) is not None, "R5 archive SHA invalid")
    require(SHA64.fullmatch(str(artifact.get("internal_manifest_sha256", ""))) is not None, "manifest SHA invalid")

    publication = e.get("publication_receipt", {})
    require(publication.get("branch") == "research/v38d-exploratory-benchmark-substrate", "publication branch mismatch")
    require(publication.get("remote_update") == "NORMAL_NON_FORCE_FAST_FORWARD", "remote update mismatch")
    require(publication.get("push_performed") is True, "push not performed")
    for key in ("merge_performed", "real_wall_campaign_created", "candidate_ranking_performed", "task2_started"):
        require(publication.get(key) is False, f"forbidden publication flag true: {key}")
    require(publication.get("pr_state_after") == "OPEN_DRAFT_UNMERGED", "PR state mismatch")

    remote = e.get("remote_verification_receipt", {})
    require(remote.get("main_sha") == MAIN and remote.get("main_unchanged") is True, "main changed")
    for key in ("fast_forward_verified", "exact_four_file_surface", "pr_open", "pr_draft", "recovery_marker_absent", "zero_wall_campaigns"):
        require(remote.get(key) is True, f"remote check not true: {key}")
    require(remote.get("pr_merged") is False, "PR merged")

    ledger = e.get("p0_p1_ledger", {})
    require(ledger.get("P0") == [] and ledger.get("P1") == [], "P0/P1 ledger not empty")
    require(e.get("unresolved_blockers") == [], "unresolved blockers remain")

    boundary = e.get("statement_that_merge_task2_and_wall_timing_were_not_performed", {})
    require(all(boundary.get(key) is False for key in ("merge_performed", "wall_timing_performed", "candidate_ranking_performed", "task2_started")), "claim boundary violated")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", required=True)
    args = parser.parse_args()
    evidence = json.loads(Path(args.evidence).read_text(encoding="utf-8"))
    validate(evidence)
    print("PASS: completion evidence satisfies the executable PM-4 Task-1 R5 contract")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except EvidenceError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
