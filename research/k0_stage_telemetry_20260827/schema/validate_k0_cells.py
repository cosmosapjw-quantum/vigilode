#!/usr/bin/env python3
"""Validate the exact failure-preserving K0 2x6 evidence matrix."""
from __future__ import annotations

import argparse
import json
import math
import pathlib
from typing import Any

CELL_SCHEMA = "vigilode-k0-stage-telemetry-cell/v1"
STAGE_SCHEMA = "vigilode-k0-stage-receipt/v1"
RECEIPT_SCHEMA = "vigilode-a1-two-arm-atomic-cell-v2"
CLAIM_CLASS = "EXPLORATORY_NONAUTHORITATIVE"
PROFILE = "enforced-budget-holdout-320"
TOLERANCE_ARM = "legacy-fixed"
SOURCE_PULL_REQUEST = 20
SOURCE_BASE_SHA = "e1124586a4029f86669e7489278c61ef676d61aa"
SOURCE_BASE_TREE = "adbb933cf3bf3d401d652c8a6d9df661d8500a2b"
FROZEN_TAU = 13.39706618860016
KERNELS = (
    "legacy-restarted-gmres",
    "incremental-givens-candidate",
)
FAMILIES = (
    "robertson-ramped",
    "hires-ramped",
    "van-der-pol-ramped",
    "rotating-nonnormal",
    "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
)
TERMINAL_STATES = {"COMPLETE", "STOP_INVALID", "ERROR"}
HARD_GATES = (
    "all_rjf_trajectories_successful",
    "rjf_trace_exact_excluding_wall",
    "zero_budget_breaches",
    "prefix_transactions_resolved",
    "zero_continuation_failures",
    "zero_unsafe_recommendations",
    "work_ledgers_exact",
    "realized_work_ratios_finite",
    "resume_cardinality_exact",
    "shadow_implicit_expensive_work_zero",
    "active_switching_false",
    "passed",
)
IDENTITY_KEYS = (
    "repository",
    "source_pull_request",
    "source_base_sha",
    "source_base_tree",
    "execution_head_sha",
    "execution_head_tree",
    "rust_version",
    "cargo_version",
)


class CellValidationError(ValueError):
    pass


def fail(message: str) -> None:
    raise CellValidationError(message)


def is_nonnegative_finite(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0.0


def is_hex(value: Any, length: int) -> bool:
    return isinstance(value, str) and len(value) == length and all(char in "0123456789abcdefABCDEF" for char in value)


def validate_work(work: Any, label: str) -> None:
    if not isinstance(work, dict):
        fail(f"{label}: missing work accounting")
    values = []
    for key in ("linear_matvecs", "diagnostic_matvecs", "operator_applies_total", "telemetry_jvp_overhead"):
        value = work.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            fail(f"{label}: invalid work accounting field {key}")
        values.append(value)
    if values[2] != values[0] + values[1]:
        fail(f"{label}: stage accounting mismatch")


def validate_stage_sequence(cell: dict[str, Any], label: str) -> None:
    stages = cell.get("stages")
    if not isinstance(stages, list) or not stages:
        fail(f"{label}: COMPLETE cell has no stage evidence")
    expected_attempt = 0
    expected_stage = 0
    terminal_seen = False
    for position, stage in enumerate(stages):
        stage_label = f"{label}:stage[{position}]"
        if not isinstance(stage, dict):
            fail(f"{stage_label}: stage is not an object")
        if stage.get("schema") != STAGE_SCHEMA or stage.get("claim_class") != CLAIM_CLASS:
            fail(f"{stage_label}: stage schema or claim boundary drift")
        if (
            stage.get("case_id") != cell["case_id"]
            or stage.get("family") != cell["family"]
            or stage.get("kernel_arm") != cell["kernel_arm"]
        ):
            fail(f"{stage_label}: stage identity mismatch")
        attempt = stage.get("attempt_id")
        index = stage.get("stage_index")
        if attempt != expected_attempt:
            if terminal_seen or expected_stage != 8 or attempt != expected_attempt + 1:
                fail(f"{stage_label}: nonexhaustive stage identity sequence")
            expected_attempt += 1
            expected_stage = 0
        if terminal_seen:
            fail(f"{stage_label}: receipt follows a terminal failed stage")
        if index != expected_stage:
            fail(f"{stage_label}: duplicate or missing stage identity")
        expected_stage += 1
        state = stage.get("execution_state")
        if state not in TERMINAL_STATES:
            fail(f"{stage_label}: invalid stage execution state")
        validate_work(stage.get("work"), stage_label)
        for key in ("initial_true_residual", "final_true_residual"):
            value = stage.get(key)
            if value is not None and not is_nonnegative_finite(value):
                fail(f"{stage_label}: invalid true residual {key}")
        geometry = stage.get("geometry")
        if state == "COMPLETE":
            if not isinstance(geometry, dict) or not is_nonnegative_finite(
                geometry.get("scaled_nonlinear_remainder")
            ):
                fail(f"{stage_label}: COMPLETE stage lacks finite exact-rho evidence")
            angle = geometry.get("rhs_to_retained_angle_rad")
            if angle is not None and (
                not is_nonnegative_finite(angle) or angle > math.pi / 2.0
            ):
                fail(f"{stage_label}: invalid RHS novelty angle")
            singular = geometry.get("rhs_singular_values")
            if not isinstance(singular, list) or any(not is_nonnegative_finite(value) for value in singular):
                fail(f"{stage_label}: invalid RHS singular values")
            if stage.get("error") is not None:
                fail(f"{stage_label}: COMPLETE stage retains an error")
        else:
            error = stage.get("error")
            if not isinstance(error, dict) or not error.get("kind") or not error.get("message"):
                fail(f"{stage_label}: failed stage lacks an explicit error")
            terminal_seen = True
    if not terminal_seen and expected_stage != 8:
        fail(f"{label}: final attempt is missing stage identities")


def validate_event(event: Any, family: str, label: str) -> None:
    if not isinstance(event, dict):
        fail(f"{label}: audit event is not an object")
    required = (
        "event_key",
        "audit_arm",
        "audit_family",
        "audit_event_key",
        "audit_full_e_eligible",
        "audit_full_e_attempted",
        "audit_full_e_completed",
        "audit_evidence_status",
        "audit_unsafe",
    )
    if any(key not in event for key in required):
        fail(f"{label}: audit evidence field is missing")
    if (
        not event["event_key"]
        or event["audit_arm"] != TOLERANCE_ARM
        or event["audit_family"] != family
        or event["audit_event_key"] != event["event_key"]
    ):
        fail(f"{label}: audit identity mismatch")
    eligible = event["audit_full_e_eligible"] is True
    if eligible:
        if (
            event["audit_full_e_attempted"] is not True
            or event["audit_full_e_completed"] is not True
            or event["audit_evidence_status"] != "complete"
            or not is_nonnegative_finite(event.get("audit_full_e_total_error"))
            or not isinstance(event.get("audit_full_e_locally_admissible"), bool)
            or event.get("audit_full_e_failure") is not None
            or not isinstance(event.get("audit_full_e_work"), dict)
            or not isinstance(event.get("audit_unsafe"), bool)
        ):
            fail(f"{label}: audit-eligible event lacks complete audit evidence")
    elif (
        event["audit_full_e_attempted"] is not False
        or event["audit_full_e_completed"] is not False
        or event["audit_evidence_status"] != "ineligible"
        or not event.get("audit_full_e_failure")
        or event.get("audit_full_e_total_error") is not None
        or event.get("audit_full_e_locally_admissible") is not None
        or event.get("audit_full_e_work") is not None
        or event.get("audit_unsafe") is not None
    ):
        fail(f"{label}: audit-ineligible event has inconsistent evidence")
    if event.get("recommended") is True and event.get("audit_unsafe") is True:
        fail(f"{label}: audit-unsafe recommendation")


def validate_receipt(receipt: Any, family: str, label: str) -> None:
    if not isinstance(receipt, dict):
        fail(f"{label}: COMPLETE cell lacks A1 receipt evidence")
    if (
        receipt.get("schema") != RECEIPT_SCHEMA
        or receipt.get("profile") != PROFILE
        or receipt.get("family") != family
        or receipt.get("tolerance_arm") != TOLERANCE_ARM
    ):
        fail(f"{label}: receipt identity or LegacyFixed tolerance arm drift")
    expected_floats = {
        "outer_rtol": 1.0e-5,
        "linear_rtol": 1.0e-10,
        "linear_atol": 1.0e-12,
        "phi_relative_tolerance": 3.0e-2 * 1.0e-5,
        "phi_absolute_tolerance": 3.0e-4 * 1.0e-5,
        "frozen_zeta34_tau": FROZEN_TAU,
    }
    for key, expected in expected_floats.items():
        if receipt.get(key) != expected:
            fail(f"{label}: receipt tolerance or frozen tau drift at {key}")
    if receipt.get("switching_active") is not False:
        fail(f"{label}: active switching is not false")
    counts = []
    for key in ("attempts", "accepted_steps", "rejected_steps", "rhs_evaluations", "jvp_vectors", "linear_matvecs"):
        value = receipt.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            fail(f"{label}: invalid receipt work field {key}")
        counts.append(value)
    if counts[1] + counts[2] != counts[0]:
        fail(f"{label}: receipt attempt accounting mismatch")
    if not is_hex(receipt.get("trace_digest"), 64):
        fail(f"{label}: malformed trace digest")
    gates = receipt.get("hard_gates")
    if not isinstance(gates, dict) or any(gates.get(key) is not True for key in HARD_GATES):
        fail(f"{label}: frozen hard-gate evidence is incomplete")
    events = receipt.get("event_rows")
    if not isinstance(events, list):
        fail(f"{label}: audit event ledger is missing")
    event_keys: set[str] = set()
    expected_recommendations: set[str] = set()
    hires_positive = False
    for index, event in enumerate(events):
        validate_event(event, family, f"{label}:event[{index}]")
        key = event["event_key"]
        if key in event_keys:
            fail(f"{label}: duplicate audit event identity")
        event_keys.add(key)
        if event.get("recommended") is True:
            expected_recommendations.add(key)
        if (
            family == "hires-ramped"
            and event.get("recommended") is False
            and is_nonnegative_finite(event.get("zeta34_signed_margin"))
            and event["zeta34_signed_margin"] > 0.0
            and event.get("audit_unsafe") is True
            and event.get("audit_evidence_status") == "complete"
        ):
            hires_positive = True
    recommendations = receipt.get("recommendation_rows")
    if not isinstance(recommendations, list):
        fail(f"{label}: recommendation ledger is missing")
    actual_recommendations = {
        row.get("event_key") for row in recommendations if isinstance(row, dict)
    }
    if len(actual_recommendations) != len(recommendations) or actual_recommendations != expected_recommendations:
        fail(f"{label}: recommendation ledger differs from events")
    if family == "hires-ramped" and not hires_positive:
        fail(f"{label}: Hires positive-control audit evidence is missing")


def validate_cell(cell: Any, expected_pair: tuple[str, str], label: str) -> str:
    if not isinstance(cell, dict):
        fail(f"{label}: cell is not an object")
    kernel, family = expected_pair
    if cell.get("schema") != CELL_SCHEMA:
        fail(f"{label}: cell schema drift")
    if cell.get("claim_class") != CLAIM_CLASS:
        fail(f"{label}: claim boundary drift")
    state = cell.get("execution_state")
    if state not in TERMINAL_STATES:
        fail(f"{label}: invalid cell terminal state")
    if (
        cell.get("kernel_arm") != kernel
        or cell.get("family") != family
        or cell.get("case_id") != f"{family}-n320"
    ):
        fail(f"{label}: cell identity mismatch")
    if (
        cell.get("repository") != "cosmosapjw-quantum/vigilode"
        or cell.get("source_pull_request") != SOURCE_PULL_REQUEST
        or cell.get("source_base_sha") != SOURCE_BASE_SHA
        or cell.get("source_base_tree") != SOURCE_BASE_TREE
        or not is_hex(cell.get("execution_head_sha"), 40)
        or not is_hex(cell.get("execution_head_tree"), 40)
        or not cell.get("rust_version")
        or not cell.get("cargo_version")
    ):
        fail(f"{label}: execution/source identity drift")
    tolerance = cell.get("tolerance")
    if not isinstance(tolerance, dict) or tolerance != {
        "kind": "LegacyFixed",
        "rtol": 1.0e-10,
        "atol": 1.0e-12,
    }:
        fail(f"{label}: LegacyFixed tolerance drift")
    if state == "COMPLETE":
        if cell.get("error") is not None:
            fail(f"{label}: COMPLETE cell retains an error")
        tokens = cell.get("solver_configuration_tokens")
        if (
            not isinstance(tokens, list)
            or not tokens
            or tokens != sorted(set(tokens))
            or any(not is_hex(token, 64) for token in tokens)
        ):
            fail(f"{label}: malformed or nondeterministic solver configuration tokens")
        validate_stage_sequence(cell, label)
        validate_receipt(cell.get("receipt"), family, label)
    else:
        error = cell.get("error")
        if not isinstance(error, dict) or not error.get("kind") or not error.get("message"):
            fail(f"{label}: failed cell lacks an explicit error")
        stages = cell.get("stages")
        if not isinstance(stages, list):
            fail(f"{label}: failed cell does not preserve a stage list")
        if stages:
            validate_stage_sequence(cell, label)
    return state


def validate_evidence_dir(directory: pathlib.Path) -> dict[str, Any]:
    paths = sorted(directory.glob("*.json"))
    if len(paths) != 12:
        fail(f"expected 12 atomic cell JSON files, found {len(paths)}")
    expected = {(kernel, family) for kernel in KERNELS for family in FAMILIES}
    loaded: dict[tuple[str, str], tuple[pathlib.Path, dict[str, Any]]] = {}
    for path in paths:
        try:
            cell = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            fail(f"{path}: unreadable cell JSON: {error}")
        pair = (cell.get("kernel_arm"), cell.get("family"))
        if pair in loaded:
            fail(f"duplicate cell identity {pair}")
        loaded[pair] = (path, cell)
    if set(loaded) != expected:
        fail(f"matrix identity mismatch: missing={sorted(expected - set(loaded))} extra={sorted(set(loaded) - expected)}")

    states: dict[str, str] = {}
    identities: list[dict[str, Any]] = []
    for pair in sorted(expected):
        path, cell = loaded[pair]
        state = validate_cell(cell, pair, str(path))
        states[f"{pair[0]}--{pair[1]}"] = state
        identities.append({key: cell.get(key) for key in IDENTITY_KEYS})
    if any(identity != identities[0] for identity in identities[1:]):
        fail("execution identity differs across the 12 cells")
    verdict = "COMPLETE" if all(state == "COMPLETE" for state in states.values()) else "STOP_INVALID"
    return {
        "schema": "vigilode-k0-stage-telemetry-validation/v1",
        "claim_class": CLAIM_CLASS,
        "verdict": verdict,
        "expected_cell_count": 12,
        "observed_cell_count": len(paths),
        "execution_identity": identities[0],
        "cell_states": states,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", required=True, type=pathlib.Path)
    args = parser.parse_args()
    try:
        report = validate_evidence_dir(args.evidence_dir)
    except CellValidationError as error:
        print(json.dumps({"verdict": "STOP_INVALID", "error": str(error)}, indent=2))
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
