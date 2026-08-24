from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

SCHEMA_ID = "vigilode-pm4-task1-r5-completion-evidence-v1"
SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA64 = re.compile(r"^[0-9a-f]{64}$")

REQUIRED_KEYS = {
    "all_integrator_targets_compile",
    "canonical_main",
    "cargo_metadata_json_sha256",
    "clippy",
    "exact_pr_surface",
    "focused_tests",
    "merge_performed",
    "observed_vendor_checksum_records",
    "observed_vendor_package_directories",
    "pr_state_after",
    "previous_feature_head",
    "published_feature_head",
    "published_tree",
    "r4_archive_authority_check",
    "r4_archive_sha256",
    "real_wall_campaign_created",
    "remote_update",
    "rustfmt",
    "schema",
    "task1_patch_sha256",
    "task2_started",
    "unresolved_blockers",
    "vendor_validation_json_sha256",
}

FIXED_VALUES = {
    "all_integrator_targets_compile": "PASS",
    "clippy": "PASS",
    "exact_pr_surface": "PASS",
    "focused_tests": "PASS",
    "merge_performed": False,
    "pr_state_after": "OPEN_DRAFT_UNMERGED",
    "r4_archive_authority_check": "PASS",
    "r4_archive_sha256": "6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333",
    "real_wall_campaign_created": False,
    "remote_update": "NORMAL_NON_FORCE_FAST_FORWARD",
    "rustfmt": "PASS",
    "schema": SCHEMA_ID,
    "task2_started": False,
}

SHA40_FIELDS = {
    "canonical_main",
    "previous_feature_head",
    "published_feature_head",
    "published_tree",
}
SHA64_FIELDS = {
    "cargo_metadata_json_sha256",
    "task1_patch_sha256",
    "vendor_validation_json_sha256",
}
INTEGER_FIELDS = {
    "observed_vendor_checksum_records",
    "observed_vendor_package_directories",
}


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object: {path}")
    return value


def validate_shape(value: dict[str, Any], *, allow_placeholders: bool) -> None:
    keys = set(value)
    if keys != REQUIRED_KEYS:
        missing = sorted(REQUIRED_KEYS - keys)
        extra = sorted(keys - REQUIRED_KEYS)
        raise AssertionError(
            f"completion evidence keys mismatch; missing={missing}; extra={extra}"
        )

    for key, expected in FIXED_VALUES.items():
        if value[key] != expected:
            raise AssertionError(f"{key} mismatch: {value[key]!r} != {expected!r}")

    for key in SHA40_FIELDS:
        actual = value[key]
        if allow_placeholders and actual == "40-hex":
            continue
        if not isinstance(actual, str) or not SHA40.fullmatch(actual):
            raise AssertionError(f"{key} must be lowercase 40-hex")

    for key in SHA64_FIELDS:
        actual = value[key]
        if allow_placeholders and actual == "64-hex":
            continue
        if not isinstance(actual, str) or not SHA64.fullmatch(actual):
            raise AssertionError(f"{key} must be lowercase 64-hex")

    for key in INTEGER_FIELDS:
        actual = value[key]
        if type(actual) is not int or actual < 0:
            raise AssertionError(f"{key} must be a nonnegative exact integer")

    blockers = value["unresolved_blockers"]
    if not isinstance(blockers, list) or not all(
        isinstance(item, str) for item in blockers
    ):
        raise AssertionError("unresolved_blockers must be a list of strings")

    if not allow_placeholders and blockers:
        raise AssertionError(
            "successful completion evidence must have unresolved_blockers=[]"
        )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate the canonical PM-4 Task-1 R5 completion-evidence shape."
    )
    parser.add_argument(
        "--schema", required=True, help="Path to the concrete canonical shape template"
    )
    parser.add_argument(
        "--instance", help="Optional produced COMPLETION_EVIDENCE.json to validate"
    )
    args = parser.parse_args()

    schema_path = Path(args.schema).resolve()
    if not schema_path.is_file():
        raise AssertionError(f"completion-evidence schema missing: {schema_path}")
    validate_shape(load_json(schema_path), allow_placeholders=True)

    if args.instance:
        instance_path = Path(args.instance).resolve()
        if not instance_path.is_file():
            raise AssertionError(f"completion-evidence instance missing: {instance_path}")
        validate_shape(load_json(instance_path), allow_placeholders=False)

    print("PASS: completion-evidence schema exists and the requested instance conforms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
