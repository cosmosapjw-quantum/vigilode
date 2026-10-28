#!/usr/bin/env python3
"""Produce failure-preserving external-comparator evidence for corpus v2.1.

The SciPy lane is deliberately a correlated cross-check: both it and the
canonical numerical reference use the pinned SciPy 1.17 Radau implementation.
It is therefore never labelled independent ranking evidence.

Calibration execution is partitionable and create-new.  Oregonator execution
has a stricter entry point: the calibration freeze is read and fully verified
before the reference manifest, holdout specification, or holdout artifact is
opened.  SUNDIALS/CVODE is probed rather than substituted; on the current
authority host only the IDA 6.4.1 runtime is present, so CVODE evidence is
typed unavailable and carries neither states nor invented work counters.
"""

from __future__ import annotations

import argparse
import copy
import ctypes.util
import hashlib
import importlib.util
import json
import math
import os
import re
import shutil
import struct
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable


TOOLS_ROOT = Path(__file__).resolve().parents[1]
REFERENCE_TOOLS = TOOLS_ROOT / "reference_v2"
if str(REFERENCE_TOOLS) not in sys.path:
    sys.path.insert(0, str(REFERENCE_TOOLS))

import generate_references_v2 as reference  # noqa: E402


EVIDENCE_SCHEMA = "vigilode-external-comparator-evidence-v1"
EVIDENCE_SET_SCHEMA = "vigilode-external-comparator-evidence-set-v1"
SCIPY_LINEAGE = f"scipy-radau@{reference.RUNTIME['scipy_git_revision']}"
SUNDIALS_LINEAGE = "sundials-cvode@not-installed-host-probe-v1"
SCRIPT_REPOSITORY_PATH = "tools/scientific_validity_v2/external_evidence.py"
SCIPY_SOURCE_REPOSITORY = "https://github.com/scipy/scipy"
SUNDIALS_SOURCE_REPOSITORY = "https://github.com/LLNL/sundials"
SCIPY_BUILD_ID = "cp312-cp312-manylinux_2_27_x86_64+manylinux_2_28_x86_64"
ZERO_SHA256 = "0" * 64
DEPENDENCY_CLOSURE_DOMAIN = b"vigilode-external-runner-dependency-closure-v1\0"
RUNNER_DEPENDENCY_PATHS = (
    "fixtures/scientific_corpus_v2_1_calibration_oracle.json",
    "fixtures/scientific_corpus_v2_1_semilinear_oracle.json",
    "tools/reference_v2/generate_references.py",
    "tools/reference_v2/generate_references_v2.py",
    SCRIPT_REPOSITORY_PATH,
)
CALIBRATION_FAMILIES = (
    "hires-ramped",
    "nonautonomous-stiff-forcing",
    "robertson-ramped",
    "rotating-nonnormal",
    "semilinear-advection-diffusion-ramped",
    "van-der-pol-ramped",
)
CALIBRATION_DIMENSIONS = (96, 384, 1536)
TOLERANCES = ((1.0e-4, "1e-4"), (1.0e-6, "1e-6"), (1.0e-8, "1e-8"))
CANONICAL_CANDIDATE_ID = "sequential-rodas5p-gmres-wrms-forcing-v2"
CANONICAL_RUNNER_SCHEMA = "scientific-validity-v2-campaign-runner-v1"
SOLVER_PROTOCOL_ID = (
    "rodas5p;gmres-restart32-max256;fallback-atol=1e-12;fallback-rtol=1e-10;"
    "inner-m=30;outer-k=8;recycle-dim=8;recycle-rank-tol=1e-12;pc-none;"
    "x0-previous;wrms-stage-residual-heuristic-v2;"
    "endpoint-bound=requires-resolvent-certificate;"
    "cross-step-recycle-images=refresh-per-linearization;"
    "outer=case-spec;initial=span/100;"
    "min=1e-12;max=span;controller=integral;safety=.9;factors=.2,5;reject=.9;"
    "total-attempts=200000"
)
OUTPUT_PROTOCOL_ID = (
    "branch-fixed-controller-krylov-restart; "
    "clipped-and-rodas5p-dense-independent-v1"
)


class ExternalEvidenceError(RuntimeError):
    """A preflight, schema, provenance, or persistence failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ExternalEvidenceError(message)


def canonical_json(value: Any, *, sort_keys: bool = True) -> bytes:
    return (
        json.dumps(
            value,
            allow_nan=False,
            ensure_ascii=False,
            sort_keys=sort_keys,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def serde_json_bytes(value: Any) -> bytes:
    """Match compact serde_json output while preserving insertion order."""

    return json.dumps(
        value,
        allow_nan=False,
        ensure_ascii=False,
        sort_keys=False,
        separators=(",", ":"),
    ).encode("utf-8")


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def valid_revision(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 40
        and all(character in "0123456789abcdef" for character in value)
    )


def atomic_create(path: Path, payload: bytes) -> None:
    """Durably create ``path`` without an overwrite race."""

    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    try:
        with os.fdopen(descriptor, "wb", closefd=True) as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.link(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    except FileExistsError as error:
        raise ExternalEvidenceError(
            f"create-new target already exists: {path}"
        ) from error
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def read_json_with_bytes(path: Path) -> tuple[dict[str, Any], bytes]:
    try:
        pinned_bytes = path.read_bytes()
        value = json.loads(pinned_bytes)
    except (OSError, json.JSONDecodeError, UnicodeDecodeError) as error:
        raise ExternalEvidenceError(f"cannot read {path}: {error}") from error
    require(isinstance(value, dict), f"JSON root is not an object: {path}")
    return value, pinned_bytes


def read_json(path: Path) -> dict[str, Any]:
    return read_json_with_bytes(path)[0]


def f64_bits(value: float) -> int:
    return struct.unpack(">Q", struct.pack(">d", float(value)))[0]


def same_f64(left: float, right: float) -> bool:
    return f64_bits(left) == f64_bits(right)


def push_u64(payload: bytearray, value: int) -> None:
    payload.extend(struct.pack(">Q", value))


def push_string(payload: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    push_u64(payload, len(encoded))
    payload.extend(encoded)


def push_campaign_binding(payload: bytearray, binding: dict[str, Any]) -> None:
    authority = binding.get("authority")
    require(
        authority in ("synthetic-ci-smoke", "canonical-v2-runner"),
        "freeze campaign authority is unknown",
    )
    payload.append(0 if authority == "synthetic-ci-smoke" else 1)
    for field in (
        "runner_schema",
        "candidate_id",
        "code_revision",
        "solver_config_sha256",
        "wrms_scale_sha256",
        "output_policy_protocol_sha256",
    ):
        value = binding.get(field)
        require(isinstance(value, str), f"freeze campaign binding lacks {field}")
        push_string(payload, value)


def push_evidence_binding(payload: bytearray, binding: dict[str, Any]) -> None:
    campaign = binding.get("campaign")
    require(isinstance(campaign, dict), "freeze row lacks campaign binding")
    push_campaign_binding(payload, campaign)
    for field in (
        "reference_checksum_sha256",
        "clipped_output_checksum_sha256",
        "dense_output_checksum_sha256",
    ):
        value = binding.get(field)
        require(isinstance(value, str), f"freeze row binding lacks {field}")
        push_string(payload, value)


def push_freeze_row(payload: bytearray, row: dict[str, Any]) -> None:
    push_string(payload, row["case_id"])
    push_string(payload, row["family"])
    partition = row["partition"]
    require(partition in ("calibration", "holdout"), "freeze row partition is unknown")
    payload.append(0 if partition == "calibration" else 1)
    push_u64(payload, int(row["dimension"]))
    push_u64(payload, f64_bits(row["atol"]))
    push_u64(payload, f64_bits(row["rtol"]))
    status_codes = {
        "pass": 0,
        "fail": 1,
        "reference-dominated": 2,
        "output-policy-dominated": 3,
    }
    require(row["status"] in status_codes, "freeze row status is unknown")
    payload.append(status_codes[row["status"]])
    metric = row.get("conservative_max_wrms")
    if metric is None:
        payload.append(0)
    else:
        payload.append(1)
        push_u64(payload, f64_bits(metric))
    binding = row.get("binding")
    require(isinstance(binding, dict), "freeze row lacks evidence binding")
    push_evidence_binding(payload, binding)
    push_string(payload, row["evidence"])


def calibration_freeze_checksum(payload_value: dict[str, Any]) -> str:
    """Reproduce the Rust calibration checksum; wall time is excluded there."""

    payload = bytearray()
    push_string(payload, payload_value["schema"])
    push_string(payload, payload_value["corpus_version"])
    profile = payload_value["profile"]
    require(profile in ("smoke", "canonical"), "freeze profile is unknown")
    payload.append(0 if profile == "smoke" else 1)
    push_string(payload, payload_value["campaign_label"])
    push_string(payload, payload_value["threshold_derivation_id"])
    push_campaign_binding(payload, payload_value["campaign_binding"])
    push_string(payload, payload_value["predeclared_holdout_family"])
    sealed = payload_value["sealed_remaining_holdout_families"]
    push_u64(payload, len(sealed))
    for family in sealed:
        push_string(payload, family)
    push_u64(payload, int(payload_value["conservative_threshold_bits"]))
    rows = sorted(payload_value["rows"], key=lambda row: row["case_id"])
    push_u64(payload, len(rows))
    for row in rows:
        push_freeze_row(payload, row)
    return sha256_bytes(bytes(payload))


def calibration_case_contracts() -> list[dict[str, Any]]:
    """Return the exact 54-case surface without touching any holdout definition."""

    cases: list[dict[str, Any]] = []
    for family in CALIBRATION_FAMILIES:
        for dimension in CALIBRATION_DIMENSIONS:
            for rtol, label in TOLERANCES:
                if family == "semilinear-advection-diffusion-ramped":
                    nx, ny = {96: (8, 12), 384: (16, 24), 1536: (32, 48)}[dimension]
                    identity = f"{family}-n{dimension}-grid{nx}x{ny}-rtol-{label}-v2.1"
                else:
                    identity = f"{family}-n{dimension}-rtol-{label}-v2.1"
                cases.append(
                    {
                        "case_id": identity,
                        "family": family,
                        "dimension": dimension,
                        "rtol": rtol,
                        "atol": 0.01 * rtol,
                    }
                )
    return sorted(cases, key=lambda case: case["case_id"])


def expected_canonical_campaign_binding(revision: str) -> dict[str, Any]:
    require(valid_revision(revision), "canonical campaign revision is malformed")
    return {
        "authority": "canonical-v2-runner",
        "runner_schema": CANONICAL_RUNNER_SCHEMA,
        "candidate_id": CANONICAL_CANDIDATE_ID,
        "code_revision": revision,
        "solver_config_sha256": sha256_bytes(
            b"vigilode-scientific-v2-solver-protocol-v1\0"
            + SOLVER_PROTOCOL_ID.encode("utf-8")
        ),
        "wrms_scale_sha256": sha256_bytes(
            b"vigilode-scientific-v2-wrms-policy-v1\0"
            b"wrms-tight-radau-l2-anchor-v1; absolute=1e-10; relative=1e-8"
        ),
        "output_policy_protocol_sha256": sha256_bytes(
            b"vigilode-scientific-v2-output-protocol-v1\0"
            + OUTPUT_PROTOCOL_ID.encode("utf-8")
        ),
    }


def verify_calibration_freeze(
    freeze: dict[str, Any], expected_revision: str | None = None
) -> str:
    """Verify canonical calibration authority before any holdout access."""

    require(
        set(freeze) == {"payload", "checksum_sha256"}, "freeze envelope fields mismatch"
    )
    payload = freeze.get("payload")
    require(isinstance(payload, dict), "freeze payload is missing")
    require(
        payload.get("schema") == "scientific-validity-v2-calibration-freeze-v1"
        and payload.get("corpus_version") == reference.CORPUS_VERSION
        and payload.get("profile") == "canonical"
        and payload.get("campaign_label") == "canonical-scientific-campaign"
        and payload.get("threshold_derivation_id")
        == "scientific-validity-v2-conservative-max-wrms-v1"
        and payload.get("predeclared_holdout_family") == "oregonator"
        and payload.get("sealed_remaining_holdout_families")
        == ["pollution", "medical-akzo", "brusselator-2d"],
        "freeze metadata is not the canonical v2.1 protocol",
    )
    campaign = payload.get("campaign_binding")
    require(isinstance(campaign, dict), "freeze lacks a campaign binding")
    revision = expected_revision or campaign.get("code_revision")
    require(valid_revision(revision), "freeze campaign revision is malformed")
    require(
        campaign == expected_canonical_campaign_binding(revision),
        "freeze campaign binding differs from the exact current canonical RODAS5P protocol",
    )

    rows = payload.get("rows")
    expected = calibration_case_contracts()
    require(
        isinstance(rows, list) and len(rows) == len(expected),
        "freeze must contain 54 rows",
    )
    require(
        [row.get("case_id") for row in rows] == [case["case_id"] for case in expected],
        "freeze rows are not in canonical case order",
    )
    metrics: list[float] = []
    for row, case in zip(rows, expected):
        require(
            row.get("family") == case["family"]
            and row.get("partition") == "calibration"
            and row.get("dimension") == case["dimension"]
            and same_f64(row.get("rtol"), case["rtol"])
            and same_f64(row.get("atol"), case["atol"])
            and row.get("status") == "pass",
            f"freeze row metadata/status mismatch: {case['case_id']}",
        )
        metric = row.get("conservative_max_wrms")
        require(
            isinstance(metric, (int, float))
            and math.isfinite(metric)
            and metric >= 0.0,
            f"freeze row lacks a finite passing metric: {case['case_id']}",
        )
        metrics.append(float(metric))
        require(
            isinstance(row.get("evidence"), str) and bool(row["evidence"].strip()),
            f"freeze row lacks evidence: {case['case_id']}",
        )
        binding = row.get("binding")
        require(
            isinstance(binding, dict), f"freeze row lacks binding: {case['case_id']}"
        )
        require(
            binding.get("campaign") == campaign,
            f"freeze row campaign differs: {case['case_id']}",
        )
        for field in (
            "reference_checksum_sha256",
            "clipped_output_checksum_sha256",
            "dense_output_checksum_sha256",
        ):
            require(
                valid_sha256(binding.get(field)),
                f"freeze row {field} is malformed: {case['case_id']}",
            )
        require(
            binding["clipped_output_checksum_sha256"]
            != binding["dense_output_checksum_sha256"],
            f"freeze row clipped/dense evidence aliases: {case['case_id']}",
        )
        wall = row.get("wall_seconds")
        require(
            wall is None
            or (isinstance(wall, (int, float)) and math.isfinite(wall) and wall >= 0.0),
            f"freeze row wall time is invalid: {case['case_id']}",
        )

    threshold = max(metrics)
    require(
        int(payload.get("conservative_threshold_bits", -1)) == f64_bits(threshold)
        and same_f64(payload.get("conservative_threshold_wrms"), threshold),
        "freeze threshold does not equal the maximum passing metric",
    )
    checksum = freeze.get("checksum_sha256")
    require(valid_sha256(checksum), "freeze checksum is malformed")
    require(
        checksum == calibration_freeze_checksum(payload), "freeze checksum mismatch"
    )
    return checksum


def verify_calibration_campaign(
    campaign: dict[str, Any], freeze: dict[str, Any], expected_revision: str
) -> None:
    """Bind holdout admission to the complete source-produced 54-artifact aggregate."""

    expected_fields = {
        "schema",
        "status",
        "corpus_version",
        "code_revision",
        "expected_case_count",
        "attempted_case_count",
        "failure_count",
        "freeze_eligible",
        "freeze_checksum_sha256",
        "freeze_admission_error",
        "record_set_sha256",
        "records",
        "rows",
    }
    require(set(campaign) == expected_fields, "calibration campaign fields mismatch")
    require(
        campaign.get("schema") == "scientific-validity-v2-calibration-campaign-v1"
        and campaign.get("status") == "complete-pass"
        and campaign.get("corpus_version") == reference.CORPUS_VERSION
        and campaign.get("code_revision") == expected_revision
        and campaign.get("expected_case_count") == 54
        and campaign.get("attempted_case_count") == 54
        and campaign.get("failure_count") == 0
        and campaign.get("freeze_eligible") is True
        and campaign.get("freeze_admission_error") is None,
        "calibration campaign is not a complete passing canonical aggregate",
    )
    freeze_checksum = verify_calibration_freeze(freeze, expected_revision)
    require(
        campaign.get("freeze_checksum_sha256") == freeze_checksum,
        "calibration campaign does not bind the supplied freeze",
    )
    rows = campaign.get("rows")
    require(
        isinstance(rows, list)
        and len(rows) == 54
        and all(isinstance(row, dict) for row in rows),
        "campaign needs 54 typed rows",
    )
    rows_by_id = {row.get("case_id"): row for row in rows}
    require(
        len(rows_by_id) == 54
        and [rows_by_id.get(row["case_id"]) for row in freeze["payload"]["rows"]]
        == freeze["payload"]["rows"],
        "campaign rows differ from freeze rows",
    )
    records = campaign.get("records")
    require(isinstance(records, list) and len(records) == 54, "campaign needs 54 records")
    expected_ids = [case["case_id"] for case in calibration_case_contracts()]
    observed_ids: list[str] = []
    ledger: list[dict[str, Any]] = []
    for record in records:
        require(
            isinstance(record, dict)
            and set(record) == {"status", "artifact"}
            and record.get("status") == "complete",
            "calibration campaign contains a failed or malformed record",
        )
        artifact = record.get("artifact")
        require(isinstance(artifact, dict), "complete campaign record lacks artifact")
        spec = artifact.get("spec")
        require(isinstance(spec, dict), "campaign artifact lacks case spec")
        case_id = spec.get("id")
        row = rows_by_id.get(case_id)
        require(isinstance(row, dict), "campaign artifact/row case mismatch")
        require(artifact.get("row") == row, "campaign artifact row differs from aggregate row")
        require(
            artifact.get("code_revision") == expected_revision,
            "campaign artifact revision mismatch",
        )
        artifact_checksum = artifact.get("artifact_checksum_sha256")
        require(valid_sha256(artifact_checksum), "campaign artifact checksum is malformed")
        observed_ids.append(case_id)
        ledger.append(
            {
                "case_id": case_id,
                "status": "complete",
                "artifact_checksum_sha256": artifact_checksum,
            }
        )
    require(
        sorted(observed_ids) == expected_ids,
        "campaign artifacts are not the exact canonical set",
    )
    ledger_bytes = json.dumps(
        ledger, ensure_ascii=False, allow_nan=False, separators=(",", ":")
    ).encode("utf-8")
    expected_record_set = sha256_bytes(
        b"vigilode-scientific-v2-campaign-record-set-v1\0" + ledger_bytes
    )
    require(
        campaign.get("record_set_sha256") == expected_record_set,
        "calibration campaign record-set checksum mismatch",
    )


def load_complete_reference_manifest(
    manifest_path: Path,
    implementation_revision: str,
    pinned_bytes: bytes | None = None,
) -> dict[str, Any]:
    reference.require_exact_runtime()
    if pinned_bytes is None:
        manifest = reference.read_json(manifest_path)
    else:
        try:
            manifest = json.loads(pinned_bytes)
        except (json.JSONDecodeError, UnicodeDecodeError) as error:
            raise ExternalEvidenceError(
                f"reference manifest is invalid JSON: {manifest_path}: {error}"
            ) from error
    reference.validate_manifest_layout(manifest)
    require(
        manifest["generation_status"] == "complete",
        "reference manifest is not complete",
    )
    require(
        manifest["producer"]["implementation_revision"] == implementation_revision,
        "reference manifest revision differs from requested code revision",
    )
    return manifest


def case_tolerance(case_id: str) -> tuple[float, float]:
    for rtol, label in TOLERANCES:
        if case_id.endswith(f"-rtol-{label}-v2.1"):
            return rtol, 0.01 * rtol
    raise ExternalEvidenceError(f"case id has no canonical tolerance suffix: {case_id}")


def selected_case_contexts(
    manifest_path: Path,
    manifest: dict[str, Any],
    mode: str,
) -> list[dict[str, Any]]:
    expected_partition = "calibration" if mode == "calibration" else "holdout"
    expected_family = None if mode == "calibration" else "oregonator"
    entries = {entry["problem"]["problem_id"]: entry for entry in manifest["artifacts"]}
    contexts: list[dict[str, Any]] = []
    validated_artifacts: set[str] = set()
    for binding in manifest["bindings"]:
        entry = entries[binding["problem_id"]]
        problem = entry["problem"]
        if problem["partition"] != expected_partition:
            continue
        if expected_family is not None and problem["family"] != expected_family:
            continue
        rtol, atol = case_tolerance(binding["case_id"])
        expected_checksum = reference.binding_checksum(binding, entry)
        require(
            binding["reference_checksum_sha256"] == expected_checksum,
            f"reference binding checksum mismatch: {binding['case_id']}",
        )
        times = reference.requested_times(problem)
        require(
            reference.grid_checksum(times) == entry["grid_sha256"],
            f"reference grid digest mismatch: {problem['problem_id']}",
        )
        problem_id = problem["problem_id"]
        if problem_id not in validated_artifacts:
            artifact_path = manifest_path.parent / entry["artifact_path"]
            artifact, raw = reference.validate_artifact_file(artifact_path, problem)
            require(
                reference.sha256_bytes(raw) == entry["artifact_sha256"]
                and artifact["checksums"]["grid_sha256"] == entry["grid_sha256"]
                and artifact["checksums"]["state_sha256"] == entry["state_sha256"],
                f"reference artifact authentication failed: {problem_id}",
            )
            validated_artifacts.add(problem_id)
        contexts.append(
            {
                "binding": binding,
                "entry": entry,
                "problem": problem,
                "requested_times": times,
                "rtol": rtol,
                "atol": atol,
            }
        )
    contexts.sort(key=lambda context: context["binding"]["case_id"])
    require(
        len(contexts) == (54 if mode == "calibration" else 3),
        f"{mode} reference case cardinality mismatch",
    )
    return contexts


def external_runtime_identity_checksum(runtime: dict[str, Any]) -> str:
    kind = runtime.get("kind")
    if kind == "scipy-python":
        identity = runtime["identity"]
        libraries = [
            {
                "role": library["role"],
                "basename": library["basename"],
                "version": library["version"],
                "configuration": library["configuration"],
                "sha256": library["sha256"],
            }
            for library in identity["blas_libraries"]
        ]
        environment = identity["thread_environment"]
        payload = {
            "kind": kind,
            "identity": {
                "python_executable": identity["python_executable"],
                "python_version": identity["python_version"],
                "python_sha256": identity["python_sha256"],
                "numpy_record_sha256": identity["numpy_record_sha256"],
                "numpy_record_verified_file_count": identity[
                    "numpy_record_verified_file_count"
                ],
                "numpy_git_revision": identity["numpy_git_revision"],
                "scipy_record_sha256": identity["scipy_record_sha256"],
                "scipy_record_verified_file_count": identity[
                    "scipy_record_verified_file_count"
                ],
                "scipy_git_revision": identity["scipy_git_revision"],
                "scipy_release": identity["scipy_release"],
                "scipy_version_module_sha256": identity["scipy_version_module_sha256"],
                "scipy_radau_module_sha256": identity["scipy_radau_module_sha256"],
                "blas_libraries": libraries,
                # Rust serializes this BTreeMap in lexical key order.
                "thread_environment": {
                    key: environment[key] for key in sorted(environment)
                },
            },
        }
    elif kind == "sundials-host-probe":
        payload = {
            "kind": kind,
            "cvode_available": runtime["cvode_available"],
            "executable_names_checked": runtime["executable_names_checked"],
            "pkg_config_modules_checked": runtime["pkg_config_modules_checked"],
            "header_paths_checked": runtime["header_paths_checked"],
            "library_names_checked": runtime["library_names_checked"],
            "python_modules_checked": runtime["python_modules_checked"],
            "ida_only_version": runtime["ida_only_version"],
            "probe_findings": [
                {
                    "category": finding["category"],
                    "target": finding["target"],
                    "observed": finding["observed"],
                    "detail": finding["detail"],
                }
                for finding in runtime["probe_findings"]
            ],
            "probe_evidence_sha256": runtime["probe_evidence_sha256"],
        }
    else:
        raise ExternalEvidenceError(f"unknown external runtime identity: {kind}")
    return sha256_bytes(
        b"vigilode-external-runtime-identity-v1\0" + serde_json_bytes(payload)
    )


def runner_dependency_closure_entries() -> list[dict[str, str]]:
    entries = [
        {
            "path": path,
            "sha256": sha256_file(reference.REPOSITORY_ROOT / path),
        }
        for path in sorted(RUNNER_DEPENDENCY_PATHS)
    ]
    require(
        len(entries) == len({entry["path"] for entry in entries}),
        "runner dependency closure contains duplicate paths",
    )
    return entries


def runner_dependency_closure_checksum(
    entries: list[dict[str, str]] | None = None,
) -> str:
    closure = runner_dependency_closure_entries() if entries is None else entries
    require(
        closure == sorted(closure, key=lambda entry: entry["path"])
        and all(
            set(entry) == {"path", "sha256"}
            and isinstance(entry["path"], str)
            and bool(entry["path"])
            and valid_sha256(entry["sha256"])
            for entry in closure
        ),
        "runner dependency closure is malformed or unsorted",
    )
    return sha256_bytes(DEPENDENCY_CLOSURE_DOMAIN + serde_json_bytes(closure))


def sundials_probe_evidence_checksum(probe: dict[str, Any]) -> str:
    payload = {
        "cvode_available": probe["cvode_available"],
        "executable_names_checked": probe["executable_names_checked"],
        "pkg_config_modules_checked": probe["pkg_config_modules_checked"],
        "header_paths_checked": probe["header_paths_checked"],
        "library_names_checked": probe["library_names_checked"],
        "python_modules_checked": probe["python_modules_checked"],
        "ida_only_version": probe["ida_only_version"],
        "probe_findings": [
            {
                "category": finding["category"],
                "target": finding["target"],
                "observed": finding["observed"],
                "detail": finding["detail"],
            }
            for finding in probe["probe_findings"]
        ],
    }
    return sha256_bytes(
        b"vigilode-sundials-host-probe-v1\0" + serde_json_bytes(payload)
    )


def scipy_runner_binding(
    dependency_closure_sha256: str | None = None,
) -> dict[str, Any]:
    runtime = {"kind": "scipy-python", "identity": reference.RUNTIME}
    return {
        "runner_id": "scipy-solve-ivp-radau",
        "version": reference.GENERATOR["scipy"],
        "build_id": SCIPY_BUILD_ID,
        "implementation_lineage_id": SCIPY_LINEAGE,
        "script_path": SCRIPT_REPOSITORY_PATH,
        "script_sha256": sha256_file(Path(__file__).resolve()),
        "dependency_closure_sha256": (
            dependency_closure_sha256 or runner_dependency_closure_checksum()
        ),
        "source_repository": SCIPY_SOURCE_REPOSITORY,
        "source_revision": reference.RUNTIME["scipy_git_revision"],
        "source_sha256": reference.RUNTIME["scipy_radau_module_sha256"],
        "observed_upstream_identity": True,
        "runtime": runtime,
        "runtime_identity_sha256": external_runtime_identity_checksum(runtime),
    }


def run_command(arguments: list[str]) -> tuple[int, str, str]:
    try:
        result = subprocess.run(
            arguments,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return 127, "", f"{type(error).__name__}: {error}"
    return result.returncode, result.stdout.strip(), result.stderr.strip()


def actual_sundials_probe() -> dict[str, Any]:
    executable_names = ["cvode", "cvode_serial"]
    pkg_modules = ["sundials-cvode", "sundials_cvode"]
    header_paths = [
        "/usr/include/cvode/cvode.h",
        "/usr/local/include/cvode/cvode.h",
    ]
    library_names = ["sundials_cvode", "libsundials_cvode.so"]
    python_modules = ["scikits.odes", "sundials", "assimulo"]
    findings: list[dict[str, Any]] = []
    cvode_observed = False

    for name in executable_names:
        resolved = shutil.which(name)
        observed = resolved is not None
        cvode_observed |= observed
        findings.append(
            {
                "category": "executable",
                "target": name,
                "observed": observed,
                "detail": resolved or "not found on PATH",
            }
        )
    for module in pkg_modules:
        code, stdout, _ = run_command(["pkg-config", "--modversion", module])
        observed = code == 0 and bool(stdout)
        cvode_observed |= observed
        findings.append(
            {
                "category": "pkg-config",
                "target": module,
                "observed": observed,
                "detail": stdout if observed else "module not found",
            }
        )
    for name in header_paths:
        observed = Path(name).is_file()
        cvode_observed |= observed
        findings.append(
            {
                "category": "header",
                "target": name,
                "observed": observed,
                "detail": "regular file" if observed else "not found",
            }
        )
    for name in library_names:
        lookup = "sundials_cvode" if name.startswith("lib") else name
        resolved = ctypes.util.find_library(lookup)
        observed = resolved is not None
        cvode_observed |= observed
        findings.append(
            {
                "category": "library",
                "target": name,
                "observed": observed,
                "detail": resolved or "not found by dynamic loader",
            }
        )
    for name in python_modules:
        try:
            spec = importlib.util.find_spec(name)
        except (ImportError, ModuleNotFoundError, AttributeError) as error:
            spec = None
            detail = f"{type(error).__name__}: {error}"
        else:
            detail = spec.origin if spec is not None else "module not found"
        observed = spec is not None
        cvode_observed |= observed
        findings.append(
            {
                "category": "python-module",
                "target": name,
                "observed": observed,
                "detail": detail,
            }
        )

    ida_library = ctypes.util.find_library("sundials_ida")
    ida_version: str | None = None
    if ida_library:
        matches = sorted(Path("/lib").glob("**/libsundials_ida.so.*.*.*"))
        for match in matches:
            parsed = re.search(r"\.so\.(\d+\.\d+\.\d+)$", match.name)
            if parsed:
                ida_version = parsed.group(1)
                break
    # IDA is context for the typed `ida_only_version` field, not a positive
    # CVODE finding.  Probe findings cover exactly the declared CVODE targets.
    require(
        not cvode_observed,
        "CVODE is now available; this unavailable-only producer must not fabricate an available runner",
    )
    payload = {
        "cvode_available": False,
        "executable_names_checked": executable_names,
        "pkg_config_modules_checked": pkg_modules,
        "header_paths_checked": header_paths,
        "library_names_checked": library_names,
        "python_modules_checked": python_modules,
        "ida_only_version": ida_version,
        "probe_findings": findings,
    }
    payload["probe_evidence_sha256"] = sundials_probe_evidence_checksum(payload)
    return payload


def sundials_runner_binding(
    probe: dict[str, Any], dependency_closure_sha256: str | None = None
) -> dict[str, Any]:
    runtime = {"kind": "sundials-host-probe", **probe}
    return {
        "runner_id": "sundials-cvode",
        "version": "not-installed",
        "build_id": "not-installed",
        "implementation_lineage_id": SUNDIALS_LINEAGE,
        "script_path": SCRIPT_REPOSITORY_PATH,
        "script_sha256": sha256_file(Path(__file__).resolve()),
        "dependency_closure_sha256": (
            dependency_closure_sha256 or runner_dependency_closure_checksum()
        ),
        "source_repository": SUNDIALS_SOURCE_REPOSITORY,
        "source_revision": "not-observed",
        "source_sha256": ZERO_SHA256,
        "observed_upstream_identity": False,
        "runtime": runtime,
        "runtime_identity_sha256": external_runtime_identity_checksum(runtime),
    }


def problem_binding(
    context: dict[str, Any], manifest: dict[str, Any], revision: str
) -> dict[str, Any]:
    problem = context["problem"]
    times = context["requested_times"]
    grid_sha = reference.grid_checksum(times)
    # The source field intentionally carries the manifest's domain-separated
    # digest of all 22 physical definitions.  The selected identity is then
    # narrowed by case_id, problem_id, exact grid, and reference checksum.
    return {
        "case_id": context["binding"]["case_id"],
        "problem_id": problem["problem_id"],
        "implementation_revision": revision,
        "dimension": problem["dimension"],
        "t_span": problem["t_span"],
        "problem_source_sha256": manifest["producer"]["problem_definition_sha256"],
        "has_mass_matrix": False,
        "requested_times": times,
        "output_grid_id": f"{reference.CORPUS_VERSION}-reference-grid:{grid_sha}",
        "reference_checksum": context["binding"]["reference_checksum_sha256"],
    }


def base_evidence(
    comparator: str,
    runner: dict[str, Any],
    context: dict[str, Any],
    manifest: dict[str, Any],
    revision: str,
) -> dict[str, Any]:
    problem = problem_binding(context, manifest, revision)
    return {
        "schema_version": EVIDENCE_SCHEMA,
        "comparator": comparator,
        "runner": runner,
        "problem": problem,
        "tolerance": {"rtol": context["rtol"], "atol": context["atol"]},
        "dense_output": {
            "interpolation": "scipy-radau-cubic-collocation",
            "solver_dense_output": True,
            "controller_step_clipping": False,
        },
        "mass_treatment": {"kind": "identity"},
        "reference_dependency": {
            "reference_lineage_id": SCIPY_LINEAGE,
            "runner_lineage_id": runner["implementation_lineage_id"],
            "shares_implementation_lineage": comparator == "scipy-radau",
        },
        "status": {"kind": "not-run", "detail": {"reason": "not executed"}},
        "checksums": {
            "grid_sha256": reference.grid_checksum(problem["requested_times"]),
            "committed_grid_sha256": None,
            "state_sha256": None,
        },
        "committed_times": None,
        "states": None,
        "native_work": None,
    }


@dataclass
class ScipySolveOutcome:
    success: bool
    reason: str
    times: list[float]
    states: list[list[float]]
    nfev: int
    njev: int
    nlu: int


def _finite_rows(values: Any, dimension: int) -> list[list[float]]:
    rows = [[float(value) for value in row] for row in values]
    require(
        all(
            len(row) == dimension and all(math.isfinite(value) for value in row)
            for row in rows
        ),
        "SciPy returned non-finite or wrong-sized dense states",
    )
    return rows


def solve_scipy_radau(context: dict[str, Any]) -> ScipySolveOutcome:
    """Run one tolerance case using solver dense output and no grid clipping."""

    problem = context["problem"]
    requested = context["requested_times"]
    rhs, jacobian, initial, _ = reference.problem_runtime(problem)
    bounds = [
        problem["t_span"][0],
        *problem["mandatory_breakpoints"],
        problem["t_span"][1],
    ]
    state = reference.np.asarray(initial, dtype=reference.np.float64)
    committed_times = [float(requested[0])]
    committed_states = [[float(value) for value in state]]
    counters = {"nfev": 0, "njev": 0, "nlu": 0}

    def failure(reason: str) -> ScipySolveOutcome:
        return ScipySolveOutcome(
            False,
            reason,
            committed_times,
            committed_states,
            **counters,
        )

    for segment_index, (start, end) in enumerate(zip(bounds, bounds[1:])):
        segment_times = [
            time_value for time_value in requested if start <= time_value <= end
        ]
        require(
            segment_times
            and reference.legacy.same_bits(segment_times[0], start)
            and reference.legacy.same_bits(segment_times[-1], end),
            "branch-fixed segment lacks exact requested endpoints",
        )
        observed_callbacks = {"nfev": 0, "njev": 0}
        segment_rhs = reference.branch_fixed_rhs(problem, segment_index, rhs)

        def counted_rhs(time_value: float, values: Any) -> Any:
            observed_callbacks["nfev"] += 1
            return segment_rhs(time_value, values)

        def counted_jacobian(time_value: float, values: Any) -> Any:
            observed_callbacks["njev"] += 1
            return jacobian(time_value, values)

        try:
            result = reference.solve_ivp(
                counted_rhs,
                (start, end),
                state,
                method="Radau",
                jac=counted_jacobian,
                rtol=context["rtol"],
                atol=context["atol"],
                dense_output=True,
            )
        except Exception as error:  # SciPy failures must retain the prior prefix.
            counters["nfev"] += observed_callbacks["nfev"]
            counters["njev"] += observed_callbacks["njev"]
            # SciPy exposes nlu only on a returned OdeResult.  A pre-result
            # exception has no native LU counter to inspect, so zero is the
            # only authenticated value; the reason preserves that limitation.
            return failure(
                f"{type(error).__name__}: {error}; "
                f"observed pre-result callbacks nfev={observed_callbacks['nfev']} "
                f"njev={observed_callbacks['njev']}; "
                "native nlu unavailable before OdeResult and recorded as 0"
            )
        for field in counters:
            counters[field] += int(getattr(result, field, 0))
        if result.sol is not None and len(result.t):
            last_time = float(result.t[-1])
            evaluable = [value for value in segment_times[1:] if value <= last_time]
            if evaluable:
                try:
                    dense = result.sol(
                        reference.np.asarray(evaluable, dtype=reference.np.float64)
                    ).T
                    rows = _finite_rows(dense, problem["dimension"])
                except Exception as error:
                    return failure(
                        "dense-output evaluation failed after actual SciPy work: "
                        f"{type(error).__name__}: {error}"
                    )
                if (
                    result.success
                    and evaluable
                    and reference.legacy.same_bits(evaluable[-1], end)
                ):
                    rows[-1] = [float(value) for value in result.y[:, -1]]
                    if not all(math.isfinite(value) for value in rows[-1]):
                        return failure("SciPy endpoint state is non-finite")
                committed_times.extend(float(value) for value in evaluable)
                committed_states.extend(rows)
        if not result.success:
            return failure(str(result.message))
        if result.sol is None or not len(result.t):
            return failure("successful SciPy segment lacks dense-output history")
        if not reference.legacy.same_bits(float(result.t[-1]), end):
            return failure(
                "successful SciPy segment did not land on its exact endpoint"
            )
        if not committed_times or not reference.legacy.same_bits(
            committed_times[-1], end
        ):
            return failure("successful SciPy segment omitted its requested endpoint")
        state = result.y[:, -1].copy()

    if not (
        len(committed_times) == len(requested)
        and len(committed_states) == len(requested)
        and all(
            reference.legacy.same_bits(actual, expected)
            for actual, expected in zip(committed_times, requested)
        )
    ):
        return failure("successful SciPy run changed or omitted the requested grid")
    return ScipySolveOutcome(
        True,
        "success",
        committed_times,
        committed_states,
        **counters,
    )


def scipy_evidence(
    context: dict[str, Any],
    manifest: dict[str, Any],
    revision: str,
    runner: dict[str, Any] | None = None,
) -> tuple[dict[str, Any], float]:
    evidence = base_evidence(
        "scipy-radau", runner or scipy_runner_binding(), context, manifest, revision
    )
    started = time.perf_counter()
    outcome = solve_scipy_radau(context)
    wall_seconds = time.perf_counter() - started
    work = {
        "kind": "scipy-radau",
        "nfev": outcome.nfev,
        "njev": outcome.njev,
        "nlu": outcome.nlu,
    }
    evidence["native_work"] = work
    evidence["committed_times"] = outcome.times
    evidence["states"] = outcome.states
    evidence["checksums"]["committed_grid_sha256"] = reference.grid_checksum(
        outcome.times
    )
    evidence["checksums"]["state_sha256"] = reference.state_checksum(outcome.states)
    if outcome.success:
        evidence["status"] = {"kind": "success"}
    else:
        evidence["status"] = {
            "kind": "solver-failure",
            "detail": {"reason": outcome.reason},
        }
    validate_external_evidence(evidence)
    return evidence, wall_seconds


def sundials_unavailable_evidence(
    context: dict[str, Any],
    manifest: dict[str, Any],
    revision: str,
    probe: dict[str, Any],
    runner: dict[str, Any] | None = None,
) -> tuple[dict[str, Any], float]:
    evidence = base_evidence(
        "sundials-cvode",
        runner or sundials_runner_binding(probe),
        context,
        manifest,
        revision,
    )
    evidence["dense_output"] = {
        "interpolation": "cvode-dense-output-unavailable",
        "solver_dense_output": True,
        "controller_step_clipping": False,
    }
    evidence["status"] = {
        "kind": "unavailable",
        "detail": {
            "reason": (
                "CVODE was not found in executables, pkg-config modules, headers, "
                "dynamic libraries, or Python bindings; IDA-only runtime "
                f"version={probe['ida_only_version'] or 'not-observed'}"
            )
        },
    }
    validate_external_evidence(evidence)
    return evidence, 0.0


def _exact_keys(value: dict[str, Any], expected: Iterable[str], label: str) -> None:
    require(set(value) == set(expected), f"{label} fields mismatch")


def validate_external_evidence(evidence: dict[str, Any]) -> None:
    """Local mirror of the live Rust admission invariants."""

    _exact_keys(
        evidence,
        (
            "schema_version",
            "comparator",
            "runner",
            "problem",
            "tolerance",
            "dense_output",
            "mass_treatment",
            "reference_dependency",
            "status",
            "checksums",
            "committed_times",
            "states",
            "native_work",
        ),
        "external evidence",
    )
    require(evidence["schema_version"] == EVIDENCE_SCHEMA, "evidence schema mismatch")
    comparator = evidence["comparator"]
    require(comparator in ("scipy-radau", "sundials-cvode"), "unknown comparator")
    runner = evidence["runner"]
    require(
        valid_sha256(runner["script_sha256"])
        and valid_sha256(runner["dependency_closure_sha256"])
        and runner["dependency_closure_sha256"] == runner_dependency_closure_checksum()
        and valid_sha256(runner["source_sha256"])
        and valid_sha256(runner["runtime_identity_sha256"])
        and runner["runtime_identity_sha256"]
        == external_runtime_identity_checksum(runner["runtime"]),
        "runner checksum/provenance mismatch",
    )
    if comparator == "scipy-radau":
        require(
            runner["runner_id"] == "scipy-solve-ivp-radau"
            and runner["version"] == "1.17.0"
            and runner["source_repository"] == SCIPY_SOURCE_REPOSITORY
            and runner["source_revision"] == reference.RUNTIME["scipy_git_revision"]
            and runner["source_sha256"]
            == reference.RUNTIME["scipy_radau_module_sha256"]
            and runner["observed_upstream_identity"] is True,
            "SciPy runner is outside the pinned installed source contract",
        )
    else:
        runtime = runner["runtime"]
        expected_probe_targets = {
            (category, target)
            for category, field in (
                ("executable", "executable_names_checked"),
                ("pkg-config", "pkg_config_modules_checked"),
                ("header", "header_paths_checked"),
                ("library", "library_names_checked"),
                ("python-module", "python_modules_checked"),
            )
            for target in runtime.get(field, [])
        }
        findings = runtime.get("probe_findings", [])
        actual_probe_targets = {
            (finding.get("category"), finding.get("target")) for finding in findings
        }
        require(
            runner["runner_id"] == "sundials-cvode"
            and runner["version"] == "not-installed"
            and runner["build_id"] == "not-installed"
            and runner["source_revision"] == "not-observed"
            and runner["source_sha256"] == ZERO_SHA256
            and runner["observed_upstream_identity"] is False
            and runtime.get("kind") == "sundials-host-probe"
            and runtime.get("cvode_available") is False
            and runtime.get("probe_evidence_sha256")
            == sundials_probe_evidence_checksum(runtime)
            and len(findings) == len(actual_probe_targets)
            and actual_probe_targets == expected_probe_targets
            and all(
                finding.get("observed") is False
                and isinstance(finding.get("detail"), str)
                and bool(finding["detail"].strip())
                for finding in findings
            ),
            "unavailable CVODE runner/probe contract mismatch",
        )
    problem = evidence["problem"]
    require(
        valid_revision(problem["implementation_revision"]),
        "bad implementation revision",
    )
    require(valid_sha256(problem["problem_source_sha256"]), "bad problem source digest")
    require(valid_sha256(problem["reference_checksum"]), "bad reference digest")
    times = problem["requested_times"]
    require(
        len(times) > 0
        and all(math.isfinite(value) for value in times)
        and all(right > left for left, right in zip(times, times[1:]))
        and same_f64(times[0], problem["t_span"][0])
        and same_f64(times[-1], problem["t_span"][1]),
        "bad requested grid",
    )
    tolerance = evidence["tolerance"]
    require(
        all(
            isinstance(tolerance.get(field), (int, float))
            and math.isfinite(tolerance[field])
            and tolerance[field] > 0.0
            for field in ("rtol", "atol")
        ),
        "bad external tolerance binding",
    )
    dense_output = evidence["dense_output"]
    require(
        isinstance(dense_output.get("interpolation"), str)
        and bool(dense_output["interpolation"].strip())
        and dense_output.get("solver_dense_output") is True
        and dense_output.get("controller_step_clipping") is False,
        "external output policy is not dense/no-clipping",
    )
    require(
        evidence["mass_treatment"] == {"kind": "identity"}, "mass treatment mismatch"
    )
    require(
        evidence["checksums"]["grid_sha256"] == reference.grid_checksum(times),
        "requested grid checksum mismatch",
    )
    dependency = evidence["reference_dependency"]
    require(
        dependency["reference_lineage_id"] == SCIPY_LINEAGE
        and dependency["runner_lineage_id"] == runner["implementation_lineage_id"]
        and dependency["shares_implementation_lineage"]
        == (dependency["reference_lineage_id"] == dependency["runner_lineage_id"]),
        "reference dependency mismatch",
    )
    if comparator == "scipy-radau":
        require(
            dependency["shares_implementation_lineage"]
            and runner["runtime"]
            == {"kind": "scipy-python", "identity": reference.RUNTIME},
            "SciPy evidence is not explicitly correlated with its reference",
        )
    status = evidence["status"]["kind"]
    if status == "success":
        require(
            set(evidence["status"]) == {"kind"}, "success status carries extra fields"
        )
    else:
        detail = evidence["status"].get("detail")
        require(
            isinstance(detail, dict)
            and set(detail) == {"reason"}
            and isinstance(detail["reason"], str)
            and bool(detail["reason"].strip()),
            "non-success status lacks a reason",
        )
    if status == "success":
        require(
            len(evidence["committed_times"]) == len(times)
            and all(
                same_f64(actual, expected)
                for actual, expected in zip(evidence["committed_times"], times)
            ),
            "success grid differs",
        )
    if status in ("success", "solver-failure"):
        committed = evidence["committed_times"]
        states = evidence["states"]
        require(
            isinstance(committed, list)
            and isinstance(states, list)
            and len(committed) == len(states)
            and len(committed) > 0
            and all(
                same_f64(actual, expected)
                for actual, expected in zip(committed, times[: len(committed)])
            ),
            "run does not preserve an exact requested-grid prefix",
        )
        require(
            all(
                len(row) == problem["dimension"]
                and all(math.isfinite(value) for value in row)
                for row in states
            ),
            "run state prefix is malformed",
        )
        require(
            evidence["checksums"]["committed_grid_sha256"]
            == reference.grid_checksum(committed)
            and evidence["checksums"]["state_sha256"]
            == reference.state_checksum(states),
            "run prefix checksum mismatch",
        )
        work = evidence["native_work"]
        require(
            isinstance(work, dict) and work.get("kind") == comparator,
            "run lacks comparator-native work",
        )
        counter_fields = (
            ("nfev", "njev", "nlu")
            if comparator == "scipy-radau"
            else ("nst", "nfe", "nje", "nni", "ncfn", "netf", "nli", "nsetups")
        )
        require(
            all(
                isinstance(work.get(field), int) and work[field] >= 0
                for field in counter_fields
            ),
            "external native work counter is missing or negative",
        )
    elif status in ("unavailable", "not-run", "non-applicable"):
        require(
            evidence["committed_times"] is None
            and evidence["states"] is None
            and evidence["native_work"] is None
            and evidence["checksums"]["committed_grid_sha256"] is None
            and evidence["checksums"]["state_sha256"] is None,
            "unexecuted evidence contains fabricated payload",
        )
        if comparator == "sundials-cvode":
            require(
                status == "unavailable",
                "CVODE-unavailable host probe requires typed unavailable status",
            )
    else:
        raise ExternalEvidenceError(f"unknown external run status: {status}")


def artifact_relative_path(comparator: str, case_id: str) -> Path:
    return Path(comparator) / f"{case_id}.json"


def evidence_set_checksum(
    mode: str,
    revision: str,
    manifest: dict[str, Any],
    reference_manifest_sha256: str,
    runner_dependency_closure_sha256: str,
    partition_index: int,
    partition_count: int,
    expected_case_count: int,
    expected_record_count: int,
    records: list[dict[str, Any]],
    calibration_authority: dict[str, str] | None = None,
) -> str:
    """Checksum scientific identity only; operational wall time is excluded."""

    payload = bytearray(b"vigilode-external-comparator-evidence-set-v1\0")
    for field in (
        mode,
        revision,
        runner_dependency_closure_sha256,
        reference_manifest_sha256,
        manifest["artifact_set_sha256"],
        manifest["binding_set_sha256"],
    ):
        encoded = field.encode("utf-8")
        payload.extend(struct.pack("<Q", len(encoded)))
        payload.extend(encoded)
    if calibration_authority is not None:
        require(
            set(calibration_authority)
            == {"freeze_checksum_sha256", "campaign_file_sha256"},
            "holdout calibration authority fields mismatch",
        )
        for field in ("freeze_checksum_sha256", "campaign_file_sha256"):
            value = calibration_authority[field]
            require(valid_sha256(value), f"holdout {field} is malformed")
            encoded = value.encode("utf-8")
            payload.extend(struct.pack("<Q", len(encoded)))
            payload.extend(encoded)
    payload.extend(struct.pack("<Q", partition_index))
    payload.extend(struct.pack("<Q", partition_count))
    payload.extend(struct.pack("<Q", expected_case_count))
    payload.extend(struct.pack("<Q", expected_record_count))
    ordered = sorted(
        records, key=lambda record: (record["case_id"], record["comparator"])
    )
    payload.extend(struct.pack("<Q", len(ordered)))
    for record in ordered:
        for field in (
            record["case_id"],
            record["comparator"],
            record["artifact_path"],
            record["artifact_sha256"],
            record["status"],
        ):
            encoded = field.encode("utf-8")
            payload.extend(struct.pack("<Q", len(encoded)))
            payload.extend(encoded)
    return sha256_bytes(bytes(payload))


def aggregate_scientific_projection(aggregate: dict[str, Any]) -> dict[str, Any]:
    projection = copy.deepcopy(aggregate)
    projection.pop("wall_seconds", None)
    for record in projection.get("records", []):
        record.pop("wall_seconds", None)
        record.pop("resumed", None)
    return projection


def validate_resumed_aggregate(
    existing: dict[str, Any],
    expected: dict[str, Any],
    manifest: dict[str, Any],
) -> None:
    partition = expected["partition"]
    expected_existing_checksum = evidence_set_checksum(
        expected["mode"],
        expected["implementation_revision"],
        manifest,
        expected["reference_manifest_sha256"],
        expected["runner_dependency_closure_sha256"],
        partition["index"],
        partition["count"],
        expected["expected_case_count"],
        expected["expected_record_count"],
        existing.get("records", []),
        expected.get("calibration_authority"),
    )
    require(
        existing.get("scientific_set_sha256") == expected_existing_checksum
        and existing["scientific_set_sha256"] == expected["scientific_set_sha256"]
        and aggregate_scientific_projection(existing)
        == aggregate_scientific_projection(expected),
        "resume aggregate scientific checksum/fields differ",
    )
    for wall in [
        existing.get("wall_seconds"),
        *(record.get("wall_seconds") for record in existing.get("records", [])),
    ]:
        require(
            isinstance(wall, (int, float)) and math.isfinite(wall) and wall >= 0.0,
            "resume aggregate operational timing is invalid",
        )


def evidence_set_completion(
    *,
    expected_case_count: int,
    selected_case_count: int,
    comparator_count: int,
    partition_count: int,
    records: list[dict[str, Any]],
) -> dict[str, Any]:
    expected_record_count = expected_case_count * comparator_count
    selected_record_count = selected_case_count * comparator_count
    status_counts = {
        status: sum(record.get("status") == status for record in records)
        for status in (
            "success",
            "solver-failure",
            "unavailable",
            "not-run",
            "non-applicable",
        )
    }
    require(
        sum(status_counts.values()) == len(records),
        "external evidence set contains an unknown record status",
    )
    partition_covered = (
        selected_case_count > 0 and len(records) == selected_record_count
    )
    full_surface_covered = (
        partition_count == 1
        and selected_case_count == expected_case_count
        and len(records) == expected_record_count
    )
    full_surface_complete = (
        full_surface_covered and status_counts["success"] == expected_record_count
    )
    if selected_case_count == 0:
        status = "empty-partition"
    elif status_counts["solver-failure"]:
        status = (
            "full-surface-with-solver-failures"
            if full_surface_covered
            else "partition-with-solver-failures"
        )
    elif status_counts["unavailable"]:
        status = (
            "full-surface-with-unavailable"
            if full_surface_covered
            else "partition-with-unavailable"
        )
    elif status_counts["not-run"] or status_counts["non-applicable"]:
        status = (
            "full-surface-with-unexecuted"
            if full_surface_covered
            else "partition-with-unexecuted"
        )
    elif full_surface_complete:
        status = "full-surface-complete"
    else:
        status = "partition-complete"
    return {
        "status": status,
        "expected_record_count": expected_record_count,
        "selected_record_count": selected_record_count,
        "status_counts": status_counts,
        "partition_covered": partition_covered,
        "full_surface_covered": full_surface_covered,
        "full_surface_complete": full_surface_complete,
    }


def validate_existing_artifact(
    path: Path,
    expected_base: dict[str, Any],
) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    try:
        evidence = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ExternalEvidenceError(
            f"existing artifact is invalid JSON: {path}: {error}"
        ) from error
    validate_external_evidence(evidence)
    require(
        raw == canonical_json(evidence),
        f"existing artifact is not in canonical create-new form: {path}",
    )
    for field in (
        "schema_version",
        "comparator",
        "runner",
        "problem",
        "tolerance",
        "dense_output",
        "mass_treatment",
        "reference_dependency",
    ):
        require(
            evidence[field] == expected_base[field],
            f"existing artifact {field} drift: {path}",
        )
    return evidence, raw


def parse_partition(value: str) -> tuple[int, int]:
    try:
        index_text, count_text = value.split("/", 1)
        index, count = int(index_text), int(count_text)
    except (ValueError, TypeError) as error:
        raise argparse.ArgumentTypeError("partition must be INDEX/COUNT") from error
    if count <= 0 or index < 0 or index >= count:
        raise argparse.ArgumentTypeError("partition must satisfy 0 <= INDEX < COUNT")
    return index, count


def run_evidence_set(
    *,
    mode: str,
    manifest_path: Path,
    output_dir: Path,
    aggregate_path: Path,
    revision: str,
    partition: tuple[int, int],
    comparators: str,
    resume: bool,
    calibration_authority: dict[str, str] | None = None,
) -> dict[str, Any]:
    require(
        valid_revision(revision), "--code-revision must be 40 lowercase hex characters"
    )
    try:
        pinned_manifest_bytes = manifest_path.read_bytes()
    except OSError as error:
        raise ExternalEvidenceError(
            f"cannot read reference manifest: {manifest_path}: {error}"
        ) from error
    reference_manifest_sha256 = sha256_bytes(pinned_manifest_bytes)
    pinned_dependency_entries = runner_dependency_closure_entries()
    pinned_dependency_sha256 = runner_dependency_closure_checksum(
        pinned_dependency_entries
    )
    manifest = load_complete_reference_manifest(
        manifest_path, revision, pinned_manifest_bytes
    )
    contexts = selected_case_contexts(manifest_path, manifest, mode)
    partition_index, partition_count = partition
    selected = [
        context
        for index, context in enumerate(contexts)
        if index % partition_count == partition_index
    ]
    kinds = (
        ("scipy-radau", "sundials-cvode") if comparators == "all" else (comparators,)
    )
    probe = actual_sundials_probe() if "sundials-cvode" in kinds else None
    scipy_runner = (
        scipy_runner_binding(pinned_dependency_sha256)
        if "scipy-radau" in kinds
        else None
    )
    sundials_runner = (
        sundials_runner_binding(probe, pinned_dependency_sha256)
        if probe is not None
        else None
    )
    records: list[dict[str, Any]] = []
    started = time.perf_counter()
    for context in selected:
        for comparator in kinds:
            relative = artifact_relative_path(comparator, context["binding"]["case_id"])
            destination = output_dir / relative
            if comparator == "scipy-radau":
                assert scipy_runner is not None
                expected_base = base_evidence(
                    comparator, scipy_runner, context, manifest, revision
                )
                builder: Callable[[], tuple[dict[str, Any], float]] = (
                    lambda context=context: scipy_evidence(
                        context, manifest, revision, scipy_runner
                    )
                )
            else:
                assert probe is not None and sundials_runner is not None
                expected_base = base_evidence(
                    comparator,
                    sundials_runner,
                    context,
                    manifest,
                    revision,
                )
                expected_base["dense_output"] = {
                    "interpolation": "cvode-dense-output-unavailable",
                    "solver_dense_output": True,
                    "controller_step_clipping": False,
                }
                builder = lambda context=context: sundials_unavailable_evidence(
                    context, manifest, revision, probe, sundials_runner
                )
            if destination.exists():
                require(resume, f"artifact exists without --resume: {destination}")
                evidence, raw = validate_existing_artifact(destination, expected_base)
                wall_seconds = 0.0
                resumed = True
            else:
                evidence, wall_seconds = builder()
                raw = canonical_json(evidence)
                atomic_create(destination, raw)
                resumed = False
            status = evidence["status"]["kind"]
            records.append(
                {
                    "case_id": context["binding"]["case_id"],
                    "problem_id": context["problem"]["problem_id"],
                    "comparator": comparator,
                    "artifact_path": relative.as_posix(),
                    "artifact_sha256": sha256_bytes(raw),
                    "status": status,
                    "resumed": resumed,
                    "wall_seconds": wall_seconds,
                }
            )
    expected_case_count = 54 if mode == "calibration" else 3
    completion = evidence_set_completion(
        expected_case_count=expected_case_count,
        selected_case_count=len(selected),
        comparator_count=len(kinds),
        partition_count=partition_count,
        records=records,
    )
    expected_record_count = completion["expected_record_count"]
    try:
        final_manifest_bytes = manifest_path.read_bytes()
    except OSError as error:
        raise ExternalEvidenceError(
            f"cannot re-read reference manifest: {manifest_path}: {error}"
        ) from error
    require(
        final_manifest_bytes == pinned_manifest_bytes,
        "reference manifest changed during external evidence execution",
    )
    require(
        runner_dependency_closure_entries() == pinned_dependency_entries,
        "runner dependency closure changed during external evidence execution",
    )
    aggregate = {
        "schema_version": EVIDENCE_SET_SCHEMA,
        "status": completion["status"],
        "mode": mode,
        "corpus_version": reference.CORPUS_VERSION,
        "implementation_revision": revision,
        "reference_manifest_sha256": reference_manifest_sha256,
        "reference_artifact_set_sha256": manifest["artifact_set_sha256"],
        "reference_binding_set_sha256": manifest["binding_set_sha256"],
        "runner_dependency_closure_sha256": pinned_dependency_sha256,
        "partition": {"index": partition_index, "count": partition_count},
        "expected_case_count": expected_case_count,
        "selected_case_count": len(selected),
        "comparator_selection": list(kinds),
        "expected_record_count": expected_record_count,
        "selected_record_count": completion["selected_record_count"],
        "record_count": len(records),
        "status_counts": completion["status_counts"],
        "partition_covered": completion["partition_covered"],
        "full_surface_covered": completion["full_surface_covered"],
        "full_surface_complete": completion["full_surface_complete"],
        "records": records,
        "wall_seconds": time.perf_counter() - started,
        "scientific_set_sha256": evidence_set_checksum(
            mode,
            revision,
            manifest,
            reference_manifest_sha256,
            pinned_dependency_sha256,
            partition_index,
            partition_count,
            expected_case_count,
            expected_record_count,
            records,
            calibration_authority,
        ),
    }
    if calibration_authority is not None:
        aggregate["calibration_authority"] = calibration_authority
    if aggregate_path.exists():
        require(resume, f"aggregate exists without --resume: {aggregate_path}")
        existing = read_json(aggregate_path)
        validate_resumed_aggregate(existing, aggregate, manifest)
        return existing
    atomic_create(aggregate_path, canonical_json(aggregate))
    return aggregate


def run_calibration_command(args: argparse.Namespace) -> dict[str, Any]:
    return run_evidence_set(
        mode="calibration",
        manifest_path=args.reference_manifest,
        output_dir=args.output_dir,
        aggregate_path=args.aggregate,
        revision=args.code_revision,
        partition=args.partition,
        comparators=args.comparator,
        resume=args.resume,
    )


def run_oregonator_command(args: argparse.Namespace) -> dict[str, Any]:
    # Access order is a scientific policy: do not call any reference/holdout
    # loader until the immutable canonical calibration freeze and its complete
    # source-produced artifact aggregate both pass.
    freeze = read_json(args.freeze)
    freeze_checksum = verify_calibration_freeze(freeze, args.code_revision)
    campaign, pinned_campaign_bytes = read_json_with_bytes(args.calibration_campaign)
    verify_calibration_campaign(campaign, freeze, args.code_revision)
    campaign_file_sha256 = sha256_bytes(pinned_campaign_bytes)
    return run_evidence_set(
        mode="oregonator",
        manifest_path=args.reference_manifest,
        output_dir=args.output_dir,
        aggregate_path=args.aggregate,
        revision=args.code_revision,
        partition=args.partition,
        comparators=args.comparator,
        resume=args.resume,
        calibration_authority={
            "freeze_checksum_sha256": freeze_checksum,
            "campaign_file_sha256": campaign_file_sha256,
        },
    )


def add_run_arguments(parser: argparse.ArgumentParser, *, holdout: bool) -> None:
    if holdout:
        parser.add_argument("--freeze", required=True, type=Path)
        parser.add_argument("--calibration-campaign", required=True, type=Path)
    parser.add_argument("--reference-manifest", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--aggregate", required=True, type=Path)
    parser.add_argument("--code-revision", required=True)
    parser.add_argument("--partition", type=parse_partition, default=(0, 1))
    parser.add_argument(
        "--comparator",
        choices=("scipy-radau", "sundials-cvode", "all"),
        default="all",
    )
    parser.add_argument("--resume", action="store_true")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="mode", required=True)
    calibration = subparsers.add_parser("calibration")
    add_run_arguments(calibration, holdout=False)
    oregonator = subparsers.add_parser("oregonator")
    add_run_arguments(oregonator, holdout=True)
    runtime = subparsers.add_parser("runtime-check")
    runtime.add_argument("--sundials-probe", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.mode == "calibration":
            aggregate = run_calibration_command(args)
        elif args.mode == "oregonator":
            aggregate = run_oregonator_command(args)
        else:
            reference.require_exact_runtime()
            result: dict[str, Any] = {"scipy_runtime": "exact-match"}
            if args.sundials_probe:
                result["sundials_probe"] = actual_sundials_probe()
            print(json.dumps(result, sort_keys=True))
            return 0
    except (ExternalEvidenceError, reference.V2ReferenceError) as error:
        print(f"EXTERNAL_EVIDENCE_ERROR: {error}", file=sys.stderr)
        return 2
    print(
        json.dumps(
            {
                "status": aggregate["status"],
                "record_count": aggregate["record_count"],
                "status_counts": aggregate["status_counts"],
                "full_surface_complete": aggregate["full_surface_complete"],
                "scientific_set_sha256": aggregate["scientific_set_sha256"],
            },
            sort_keys=True,
        )
    )
    if aggregate["status_counts"]["solver-failure"]:
        return 2
    if aggregate["status_counts"]["unavailable"]:
        return 3
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
