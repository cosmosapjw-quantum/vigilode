from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
from typing import Any

DRAFT_2020_12 = "https://json-schema.org/draft/2020-12/schema"
REQUIRED_KEYS = {
    "schema", "repository", "target_pull_request", "r4_archive_sha256",
    "base_sha", "previous_feature_sha", "published_feature_sha",
    "published_tree_sha", "git_status_porcelain", "r4_archive_authority_check",
    "task1_patch_sha256", "staged_and_final_name_status",
    "final_task1_file_sha256_results", "vendor_validation_json",
    "cargo_metadata_json_sha256", "all_command_logs_and_exit_codes",
    "r5_artifact", "publication_receipt", "remote_verification_receipt",
    "p0_p1_ledger", "unresolved_blockers",
    "statement_that_merge_task2_and_wall_timing_were_not_performed",
}


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object: {path}")
    return value


def validate_schema(schema: dict[str, Any]) -> None:
    if schema.get("$schema") != DRAFT_2020_12:
        raise AssertionError("completion evidence must use JSON Schema Draft 2020-12")
    if schema.get("type") != "object":
        raise AssertionError("completion evidence schema root type must be object")
    if schema.get("additionalProperties") is not False:
        raise AssertionError("completion evidence must reject undeclared keys")
    required = schema.get("required")
    properties = schema.get("properties")
    if not isinstance(required, list) or set(required) != REQUIRED_KEYS:
        raise AssertionError("completion evidence required-key set mismatch")
    if not isinstance(properties, dict) or set(properties) != REQUIRED_KEYS:
        raise AssertionError("completion evidence property set mismatch")
    if properties["unresolved_blockers"].get("maxItems") != 0:
        raise AssertionError("successful evidence must require unresolved_blockers=[]")
    if properties["publication_receipt"]["properties"]["merge_performed"].get("const") is not False:
        raise AssertionError("merge boundary is not fixed false")
    if properties["publication_receipt"]["properties"]["task2_started"].get("const") is not False:
        raise AssertionError("Task-2 boundary is not fixed false")


def load_executable_validator(schema_path: Path):
    validator_path = schema_path.parent.parent / "acceptance" / "validate_completion_evidence.py"
    if not validator_path.is_file():
        raise AssertionError(f"completion-evidence validator missing: {validator_path}")
    spec = importlib.util.spec_from_file_location("completion_evidence_validator", validator_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate the PM-4 Task-1 completion-evidence JSON Schema and optional instance."
    )
    parser.add_argument("--schema", required=True)
    parser.add_argument("--instance")
    args = parser.parse_args()

    schema_path = Path(args.schema).resolve()
    if not schema_path.is_file():
        raise AssertionError(f"completion-evidence schema missing: {schema_path}")
    validate_schema(load_json(schema_path))

    if args.instance:
        instance_path = Path(args.instance).resolve()
        if not instance_path.is_file():
            raise AssertionError(f"completion-evidence instance missing: {instance_path}")
        module = load_executable_validator(schema_path)
        module.validate(load_json(instance_path))

    print("PASS: completion-evidence JSON Schema exists and the requested instance conforms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
