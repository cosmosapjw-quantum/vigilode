#!/usr/bin/env python3
"""Validate the candidate-free local-Codex handoff without running science."""

from __future__ import annotations

import json
import pathlib
import sys

DIRECTORY = pathlib.Path("research/audit2_stage_certificate_telemetry_20260831")
REQUIRED_FILES = {
    "CLAIM_LEDGER.md",
    "CODEX_START_HERE.md",
    "EXECUTION_CONTRACT.json",
    "FORMAL_SCOPE.md",
    "PUBLICATION_SCHEMA.json",
    "RAW_DATA_POLICY.md",
    "README.md",
    "handoff.json",
}
CEILING = (
    "EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_"
    "TRANSACTIONAL_STEP_SUBSTRATE"
)
FORBIDDEN_RUNTIME_TOKENS = {
    "run_audit2_bateman_local_validation.py",
    "audit2_bateman_local_six_case",
    "run_audit2_bateman_local_six_case_suite",
    "scientific_validity_v2/external_evidence.py",
}
PROMPT_MARKERS = {
    "LOCAL_CODEX_JOB_ONLY",
    "FRESH_LOCAL_GENERATION",
    "candidate_executions: 0",
    "raw_data_committed: false",
    "FORMAL_BACKEND_UNAVAILABLE",
}


def _load(path: pathlib.Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def validate(root: pathlib.Path) -> dict:
    research = root / DIRECTORY
    if not research.is_dir():
        raise ValueError(f"missing handoff directory: {DIRECTORY}")

    files = {path.relative_to(research).as_posix() for path in research.rglob("*") if path.is_file()}
    missing = sorted(REQUIRED_FILES - files)
    if missing:
        raise ValueError(f"missing required handoff files: {', '.join(missing)}")

    contract = _load(research / "EXECUTION_CONTRACT.json")
    handoff = _load(research / "handoff.json")
    schema = _load(research / "PUBLICATION_SCHEMA.json")

    authority = contract.get("authority", {})
    policy = contract.get("execution_policy", {})
    implementation = contract.get("implementation", {})
    publication = contract.get("publication", {})
    formal = contract.get("formal", {})

    if authority.get("repository") != "cosmosapjw-quantum/vigilode":
        raise ValueError("repository authority mismatch")
    if authority.get("stack_base_pr") != 40:
        raise ValueError("stack base PR mismatch")
    if authority.get("stack_base_head") != "426d37ce3c0f4e5b7843b163eaf772b8e55bfa87":
        raise ValueError("stack base head mismatch")
    if authority.get("recovery_policy") != "FRESH_LOCAL_GENERATION_NOT_UNPUBLISHED_COMMIT_RECOVERY":
        raise ValueError("fresh-generation policy mismatch")

    if policy.get("executor") != "LOCAL_CODEX_JOB_ONLY":
        raise ValueError("executor policy mismatch")
    if policy.get("local_llm_allowed") is not False:
        raise ValueError("local LLM must be forbidden")
    if policy.get("candidate_executions_required") != 0:
        raise ValueError("candidate executions must remain zero")
    if policy.get("holdout_access") != "NOT_OPENED_OR_EXECUTED":
        raise ValueError("holdout boundary mismatch")

    harnesses = contract.get("harness_inputs")
    expected_harnesses = {
        "physmath-research-harness-gpt56.zip":
            "9adde688f8020e7feb2c1c0304b3204dbe70dd01e2d87e64a5c4eb357c019934",
        "physmath-coding-harness-gpt56.zip":
            "6e67e999a0c19f6ed9de7c339067cc11691d5cf5cb662a11756d8fc393c849b4",
    }
    if not isinstance(harnesses, list) or {
        row.get("name"): row.get("sha256") for row in harnesses if isinstance(row, dict)
    } != expected_harnesses:
        raise ValueError("process harness identity mismatch")

    if implementation.get("cargo_feature") != "audit2-stage-certificate":
        raise ValueError("stage-certificate feature mismatch")
    if implementation.get("required_feature_dependency") != "audit2-research":
        raise ValueError("research feature dependency mismatch")
    if implementation.get("forbidden_feature_dependency") != "audit2-bateman-authority":
        raise ValueError("candidate-authority feature boundary mismatch")
    if implementation.get("norm_schema") != "dimensionless-synthetic-l2/v1":
        raise ValueError("synthetic norm schema mismatch")
    if implementation.get("norm_scale_bits") != 0x3FF0000000000000:
        raise ValueError("synthetic norm scale is not exact unity")
    if implementation.get("receipt_authority") != "SyntheticSchemaConsistencyOnly":
        raise ValueError("receipt authority exceeds the synthetic ceiling")

    if formal.get("required_obligations") != ["F01", "F02", "F03", "F04", "F05"]:
        raise ValueError("formal obligation list mismatch")
    if formal.get("pass_requires_every_backend") is not True:
        raise ValueError("formal PASS must require every backend")
    if formal.get("raw_backend_output_in_git") is not False:
        raise ValueError("raw formal output must remain outside Git")

    if contract.get("claim_ceiling") != CEILING or handoff.get("claim_ceiling") != CEILING:
        raise ValueError("claim ceiling mismatch")
    if handoff.get("execution", {}).get("candidate_executions") != 0:
        raise ValueError("handoff candidate count mismatch")
    if handoff.get("publication", {}).get("merge_tag_release_authorized") is not False:
        raise ValueError("merge/tag/release must remain unauthorized")

    properties = schema.get("properties", {})
    if properties.get("claim_ceiling", {}).get("const") != CEILING:
        raise ValueError("publication schema claim ceiling mismatch")
    if (
        properties.get("policy", {})
        .get("properties", {})
        .get("candidate_executions", {})
        .get("const")
        != 0
    ):
        raise ValueError("publication schema does not freeze candidate count")

    max_single = publication.get("max_single_research_file_bytes")
    max_total = publication.get("max_total_research_directory_bytes")
    suffixes = tuple(publication.get("forbidden_suffixes", []))
    if type(max_single) is not int or type(max_total) is not int:
        raise ValueError("publication size limits must be integers")
    total = 0
    for relative in sorted(files):
        path = research / relative
        size = path.stat().st_size
        total += size
        if size > max_single:
            raise ValueError(f"handoff file exceeds size limit: {relative}")
        if suffixes and relative.endswith(suffixes):
            raise ValueError(f"forbidden checked-in suffix: {relative}")
    if total > max_total:
        raise ValueError("handoff directory exceeds total size limit")

    prompt = (research / "CODEX_START_HERE.md").read_text(encoding="utf-8")
    missing_markers = sorted(marker for marker in PROMPT_MARKERS if marker not in prompt)
    if missing_markers:
        raise ValueError(f"prompt markers missing: {', '.join(missing_markers)}")

    all_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(research.rglob("*"))
        if path.is_file() and path.suffix in {".md", ".json", ".py"}
    )
    present_tokens = sorted(token for token in FORBIDDEN_RUNTIME_TOKENS if token in all_text)
    if present_tokens:
        raise ValueError(f"historic candidate runtime surface named: {', '.join(present_tokens)}")

    return {
        "status": "CANDIDATE_FREE_LOCAL_CODEX_HANDOFF_VALID",
        "candidate_executions": 0,
        "required_files": len(REQUIRED_FILES),
        "checked_files": len(files),
        "checked_bytes": total,
        "claim_ceiling": CEILING,
    }


def main() -> int:
    root = pathlib.Path(__file__).resolve().parents[1]
    try:
        result = validate(root)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"HANDOFF_INVALID: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
