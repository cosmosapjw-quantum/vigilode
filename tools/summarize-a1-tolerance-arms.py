#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import sys
from typing import Any


SCHEMA = "vigilode-a1-two-arm-atomic-cell-v2"
AGGREGATE_SCHEMA = "vigilode-a1-two-arm-authority-receipt-v2"
PROFILE = "enforced-budget-holdout-320"
TAU = 13.39706618860016
PHI_RTOL = 3.0e-2 * 1.0e-5
PHI_ATOL = 3.0e-4 * 1.0e-5
ARMS = ("legacy-fixed", "outer-scaled-numeric-parity")
FAMILIES = (
    "robertson-ramped",
    "hires-ramped",
    "van-der-pol-ramped",
    "rotating-nonnormal",
    "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
)
IDENTITY_FIELDS = (
    "repository",
    "pull_request",
    "scientific_execution_head_sha",
    "scientific_execution_head_tree",
    "base_sha",
    "base_tree",
    "tested_execution_merge_sha",
    "tested_execution_merge_tree",
    "execution_workflow_run_id",
    "execution_workflow_run_attempt",
    "rust_version",
    "cargo_version",
)
REQUIRED_FIELDS = set(IDENTITY_FIELDS) | {
    "schema",
    "profile",
    "family",
    "arm",
    "outer_rtol",
    "linear_rtol",
    "linear_atol",
    "phi_relative_tolerance",
    "phi_absolute_tolerance",
    "attempts",
    "accepted_steps",
    "rejected_steps",
    "rhs_evaluations",
    "jvp_vectors",
    "linear_matvecs",
    "trace_digest",
    "switching_active",
    "frozen_zeta34_tau",
    "event_rows",
    "recommendation_rows",
    "hard_gates",
    "limitations",
}
FORBIDDEN_TRACKED_FIELDS = {
    "receipt_commit_sha",
    "receipt_commit_tree",
    "external_verification_run_id",
    "external_verification_run_attempt",
}
BASE_HARD_GATES = (
    "all_rjf_trajectories_successful",
    "rjf_trace_exact_excluding_wall",
    "zero_budget_breaches",
    "prefix_transactions_resolved",
    "zero_continuation_failures",
    "work_ledgers_exact",
    "realized_work_ratios_finite",
    "resume_cardinality_exact",
    "shadow_implicit_expensive_work_zero",
    "active_switching_false",
)
GIT_OBJECT = re.compile(r"^[0-9a-f]{40}$")
TRACE_DIGEST = re.compile(r"^[0-9a-f]{64}$")
INVALIDATED_EXECUTION_WORKFLOW_RUN_ID = 32906175896
INVALIDATED_AGGREGATE_SCIENTIFIC_DIGEST = (
    "7665718c60ff9c1e0d1e86d1ff4464e8eb71d806dd0e6ce5c4f6ac0501f027a1"
)
EVENT_REQUIRED_FIELDS = {
    "event_key",
    "trajectory_id",
    "decision_accepted_step",
    "target_attempt_index",
    "target_accepted_steps_before",
    "t_start",
    "h",
    "quadratic_drift_zeta34",
    "zeta34_signed_margin",
    "recommended",
    "shadow_full_e_completed",
    "shadow_full_e_locally_admissible",
    "audit_arm",
    "audit_family",
    "audit_event_key",
    "audit_full_e_eligible",
    "audit_full_e_attempted",
    "audit_full_e_completed",
    "audit_full_e_total_error",
    "audit_full_e_locally_admissible",
    "audit_full_e_failure",
    "audit_full_e_work",
    "audit_unsafe",
    "audit_evidence_status",
}


class ReceiptValidationError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ReceiptValidationError(message)


def canonical_bytes(value: object) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def validate_cell(cell: dict[str, Any], source: Path) -> None:
    missing = sorted(REQUIRED_FIELDS - set(cell))
    require(not missing, f"{source}: missing required fields: {', '.join(missing)}")
    forbidden = sorted(FORBIDDEN_TRACKED_FIELDS & set(cell))
    require(not forbidden, f"{source}: forbidden late-bound fields: {', '.join(forbidden)}")
    require(cell["schema"] == SCHEMA, f"{source}: wrong schema")
    require(cell["profile"] == PROFILE, f"{source}: wrong profile")
    require(cell["arm"] in ARMS, f"{source}: unknown arm {cell['arm']!r}")
    require(cell["family"] in FAMILIES, f"{source}: unknown family {cell['family']!r}")
    require(cell["switching_active"] is False, f"{source}: switching must be false")
    require(cell["frozen_zeta34_tau"] == TAU, f"{source}: frozen tau mismatch")
    for field in (
        "scientific_execution_head_sha",
        "scientific_execution_head_tree",
        "base_sha",
        "base_tree",
        "tested_execution_merge_sha",
        "tested_execution_merge_tree",
    ):
        require(
            isinstance(cell[field], str) and GIT_OBJECT.fullmatch(cell[field]) is not None,
            f"{source}: malformed {field}",
        )
    require(
        isinstance(cell["trace_digest"], str)
        and TRACE_DIGEST.fullmatch(cell["trace_digest"]) is not None,
        f"{source}: malformed trace_digest",
    )
    require(cell["pull_request"] == 18, f"{source}: wrong pull request")
    require(
        isinstance(cell["execution_workflow_run_id"], int)
        and cell["execution_workflow_run_id"] > 0,
        f"{source}: invalid workflow run ID",
    )
    require(
        cell["execution_workflow_run_id"] != INVALIDATED_EXECUTION_WORKFLOW_RUN_ID,
        f"{source}: workflow run 32906175896 is diagnostic-only and invalid for authority",
    )
    require(
        isinstance(cell["execution_workflow_run_attempt"], int)
        and cell["execution_workflow_run_attempt"] > 0,
        f"{source}: invalid workflow run attempt",
    )
    require(cell["outer_rtol"] == 1.0e-5, f"{source}: wrong outer rtol")
    require(
        cell["phi_relative_tolerance"] == PHI_RTOL
        and cell["phi_absolute_tolerance"] == PHI_ATOL,
        f"{source}: preserved phi tolerance mismatch",
    )
    expected_linear = (
        (1.0e-10, 1.0e-12)
        if cell["arm"] == "legacy-fixed"
        else (PHI_RTOL, PHI_ATOL)
    )
    require(
        (cell["linear_rtol"], cell["linear_atol"]) == expected_linear,
        f"{source}: linear tolerance mismatch",
    )
    for field in (
        "attempts",
        "accepted_steps",
        "rejected_steps",
        "rhs_evaluations",
        "jvp_vectors",
        "linear_matvecs",
    ):
        require(
            isinstance(cell[field], int) and cell[field] >= 0,
            f"{source}: invalid nonnegative count {field}",
        )
    require(
        cell["attempts"] == cell["accepted_steps"] + cell["rejected_steps"],
        f"{source}: attempt accounting mismatch",
    )

    events = cell["event_rows"]
    require(isinstance(events, list), f"{source}: event_rows must be a list")
    event_keys: set[str] = set()
    recommended_keys: set[str] = set()
    for event in events:
        require(isinstance(event, dict), f"{source}: event row must be an object")
        missing_event = sorted(EVENT_REQUIRED_FIELDS - set(event))
        require(
            not missing_event,
            f"{source}: event row missing audit fields: {', '.join(missing_event)}",
        )
        key = event.get("event_key")
        require(isinstance(key, str) and key, f"{source}: invalid event key")
        require(key not in event_keys, f"{source}: duplicate event key {key}")
        event_keys.add(key)
        zeta = event.get("quadratic_drift_zeta34")
        margin = event.get("zeta34_signed_margin")
        if zeta is None:
            require(margin is None, f"{source}: margin exists without finite zeta34")
        else:
            require(isinstance(zeta, (int, float)) and math.isfinite(zeta), f"{source}: nonfinite zeta34")
            require(
                isinstance(margin, (int, float))
                and math.isfinite(margin)
                and math.isclose(margin, zeta - TAU, rel_tol=0.0, abs_tol=1.0e-14),
                f"{source}: zeta34 signed margin mismatch",
            )
        require(type(event["recommended"]) is bool, f"{source}: recommended must be Boolean")
        require(
            type(event["shadow_full_e_completed"]) is bool
            and type(event["shadow_full_e_locally_admissible"]) is bool,
            f"{source}: runtime shadow completion/admissibility must be Boolean",
        )
        require(event["audit_arm"] == cell["arm"], f"{source}: audit arm identity mismatch")
        require(event["audit_family"] == cell["family"], f"{source}: audit family identity mismatch")
        require(event["audit_event_key"] == key, f"{source}: audit event identity mismatch")
        require(
            isinstance(event["t_start"], (int, float))
            and math.isfinite(event["t_start"])
            and isinstance(event["h"], (int, float))
            and math.isfinite(event["h"])
            and event["h"] > 0.0,
            f"{source}: invalid audit event state or trial h",
        )
        for field in (
            "audit_full_e_eligible",
            "audit_full_e_attempted",
            "audit_full_e_completed",
        ):
            require(type(event[field]) is bool, f"{source}: {field} must be Boolean")

        eligible = event["audit_full_e_eligible"]
        attempted = event["audit_full_e_attempted"]
        completed = event["audit_full_e_completed"]
        status = event["audit_evidence_status"]
        require(isinstance(status, str) and status, f"{source}: missing audit evidence status")
        if eligible:
            require(
                attempted and completed and status == "complete",
                f"{source}: STOP_INVALID incomplete eligible audit evidence for {key}",
            )
            total_error = event["audit_full_e_total_error"]
            require(
                isinstance(total_error, (int, float))
                and math.isfinite(total_error)
                and total_error >= 0.0,
                f"{source}: invalid completed audit total error for {key}",
            )
            admissible = event["audit_full_e_locally_admissible"]
            require(type(admissible) is bool, f"{source}: missing completed audit admissibility for {key}")
            require(event["audit_full_e_failure"] is None, f"{source}: completed audit has failure for {key}")
            work = event["audit_full_e_work"]
            require(isinstance(work, dict) and work, f"{source}: missing completed audit work for {key}")
            require(
                all(type(value) is int and value >= 0 for value in work.values()),
                f"{source}: invalid completed audit work for {key}",
            )
            derived_unsafe = not admissible
            require(
                event["audit_unsafe"] is derived_unsafe,
                f"{source}: audit unsafe mismatch for {key}",
            )
        else:
            require(not attempted and not completed, f"{source}: inconsistent ineligible audit state for {key}")
            require(status == "ineligible", f"{source}: invalid ineligible audit status for {key}")
            require(
                isinstance(event["audit_full_e_failure"], str)
                and bool(event["audit_full_e_failure"].strip()),
                f"{source}: ineligible audit requires an explicit reason for {key}",
            )
            for field in (
                "audit_full_e_total_error",
                "audit_full_e_locally_admissible",
                "audit_full_e_work",
                "audit_unsafe",
            ):
                require(event[field] is None, f"{source}: ineligible audit must keep {field} unknown for {key}")
        if event.get("recommended") is True:
            recommended_keys.add(key)

    recommendations = cell["recommendation_rows"]
    require(isinstance(recommendations, list), f"{source}: recommendation_rows must be a list")
    listed_recommendations = {
        row.get("event_key") for row in recommendations if isinstance(row, dict)
    }
    require(
        None not in listed_recommendations and listed_recommendations == recommended_keys,
        f"{source}: recommendation rows do not match atomic events",
    )

    hard_gates = cell["hard_gates"]
    require(isinstance(hard_gates, dict), f"{source}: hard_gates must be an object")
    for gate in BASE_HARD_GATES:
        require(type(hard_gates.get(gate)) is bool, f"{source}: missing Boolean hard gate {gate}")
    require(type(hard_gates.get("passed")) is bool, f"{source}: missing hard gate passed")
    require(isinstance(cell["limitations"], list), f"{source}: limitations must be a list")


def load_cells(cells_dir: Path) -> list[tuple[Path, dict[str, Any], bytes]]:
    paths = sorted(cells_dir.rglob("*.json"))
    require(bool(paths), f"no JSON cells found under {cells_dir}")
    loaded = []
    for path in paths:
        raw = path.read_bytes()
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as error:
            raise ReceiptValidationError(f"{path}: invalid JSON: {error}") from error
        require(isinstance(value, dict), f"{path}: cell must be a JSON object")
        validate_cell(value, path)
        loaded.append((path, value, raw))
    return loaded


def summarize(cells_dir: Path) -> dict[str, Any]:
    loaded = load_cells(cells_dir)
    expected_keys = {(arm, family) for arm in ARMS for family in FAMILIES}
    observed_keys = [(cell["arm"], cell["family"]) for _, cell, _ in loaded]
    require(len(observed_keys) == 12, f"expected exactly 12 cells, observed {len(observed_keys)}")
    require(len(set(observed_keys)) == 12, "duplicate arm/family cell")
    require(set(observed_keys) == expected_keys, "missing or extra arm/family cell")

    identity = {field: loaded[0][1][field] for field in IDENTITY_FIELDS}
    for path, cell, _ in loaded[1:]:
        for field, expected in identity.items():
            require(cell[field] == expected, f"{path}: scientific execution identity mismatch for {field}")

    ordered = sorted(loaded, key=lambda item: (ARMS.index(item[1]["arm"]), FAMILIES.index(item[1]["family"])))
    complete_cell_keys = [f"{cell['arm']}/{cell['family']}" for _, cell, _ in ordered]
    manifest = [
        {
            "cell_key": f"{cell['arm']}/{cell['family']}",
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
        for _, cell, raw in ordered
    ]

    per_arm_totals: dict[str, dict[str, int]] = {}
    event_key_sets: dict[str, list[str]] = {}
    recommendation_key_sets: dict[str, list[str]] = {}
    zeta_values: list[dict[str, Any]] = []
    unsafe_recommendation_keys: list[str] = []
    audit_unsafe_event_keys: list[str] = []
    hires_positive_control: dict[str, bool] = {}
    audit_evidence_counts: dict[str, dict[str, dict[str, int]]] = {}
    audit_failures: list[dict[str, str]] = []
    base_hard_gates_pass = True

    for arm in ARMS:
        arm_cells = [cell for _, cell, _ in ordered if cell["arm"] == arm]
        per_arm_totals[arm] = {
            field: sum(cell[field] for cell in arm_cells)
            for field in (
                "attempts",
                "accepted_steps",
                "rejected_steps",
                "rhs_evaluations",
                "jvp_vectors",
                "linear_matvecs",
            )
        }
        arm_event_keys: list[str] = []
        arm_recommendations: list[str] = []
        arm_hires_positive = False
        arm_hires_eligible = False
        audit_evidence_counts[arm] = {}
        for cell in arm_cells:
            family_counts = {"eligible": 0, "completed": 0, "ineligible": 0}
            base_hard_gates_pass &= all(cell["hard_gates"][gate] for gate in BASE_HARD_GATES)
            for event in cell["event_rows"]:
                qualified_key = f"{arm}/{cell['family']}/{event['event_key']}"
                arm_event_keys.append(qualified_key)
                if event["quadratic_drift_zeta34"] is not None:
                    zeta_values.append(
                        {
                            "event_key": qualified_key,
                            "quadratic_drift_zeta34": event["quadratic_drift_zeta34"],
                            "zeta34_signed_margin": event["zeta34_signed_margin"],
                        }
                    )
                if event["recommended"]:
                    arm_recommendations.append(qualified_key)
                    if event["audit_unsafe"] is True:
                        unsafe_recommendation_keys.append(qualified_key)
                if event["audit_full_e_eligible"]:
                    family_counts["eligible"] += 1
                    family_counts["completed"] += int(event["audit_full_e_completed"])
                    if cell["family"] == "hires-ramped":
                        arm_hires_eligible = True
                else:
                    family_counts["ineligible"] += 1
                if event["audit_unsafe"] is True:
                    audit_unsafe_event_keys.append(qualified_key)
                    if (
                        cell["family"] == "hires-ramped"
                        and not event["recommended"]
                        and event["quadratic_drift_zeta34"] is not None
                        and event["quadratic_drift_zeta34"] > TAU
                        and event["audit_full_e_completed"]
                        and event["audit_full_e_locally_admissible"] is False
                    ):
                        arm_hires_positive = True
            audit_evidence_counts[arm][cell["family"]] = family_counts
        event_key_sets[arm] = sorted(arm_event_keys)
        recommendation_key_sets[arm] = sorted(arm_recommendations)
        require(
            arm_hires_eligible,
            f"STOP_INVALID: no eligible completed Hires audit population for arm {arm}",
        )
        hires_positive_control[arm] = arm_hires_positive

    unsafe_recommendation_keys.sort()
    audit_unsafe_event_keys.sort()
    zeta_values.sort(key=lambda row: row["event_key"])
    all_hard_gates_pass = base_hard_gates_pass and not unsafe_recommendation_keys
    if not all_hard_gates_pass:
        decision = "NOT_ADMISSIBLE"
    elif all(hires_positive_control.values()):
        decision = "ADMISSIBLE_AND_DISCRIMINATING"
    else:
        decision = "ADMISSIBLE_BUT_NONDISCRIMINATING"

    limitations = sorted(
        {
            limitation
            for _, cell, _ in ordered
            for limitation in cell["limitations"]
            if isinstance(limitation, str)
        }
    )
    scientific_payload = {
        "schema": AGGREGATE_SCHEMA,
        "scientific_execution_identity": identity,
        "receipt_parent_expected": identity["scientific_execution_head_sha"],
        "profile": PROFILE,
        "complete_cell_keys": complete_cell_keys,
        "per_arm_totals": per_arm_totals,
        "event_key_sets": event_key_sets,
        "zeta34_values_and_signed_margins": zeta_values,
        "recommendation_key_sets": recommendation_key_sets,
        "unsafe_recommendation_keys": unsafe_recommendation_keys,
        "audit_unsafe_event_keys": audit_unsafe_event_keys,
        "hires_positive_control": hires_positive_control,
        "hard_gates": {
            "base_scientific_gates_pass": base_hard_gates_pass,
            "zero_unsafe_recommendations": not unsafe_recommendation_keys,
            "complete_two_by_six_matrix": True,
            "single_scientific_execution_identity": True,
            "frozen_tau_exact": True,
            "active_switching_false": True,
            "audit_evidence_complete": True,
            "passed": all_hard_gates_pass,
        },
        "audit_evidence_counts": audit_evidence_counts,
        "audit_failures": audit_failures,
        "predeclared_decision": decision,
        "limitations": limitations,
    }
    scientific_digest = hashlib.sha256(canonical_bytes(scientific_payload)).hexdigest()
    require(
        scientific_digest != INVALIDATED_AGGREGATE_SCIENTIFIC_DIGEST,
        "STOP_INVALID: invalidated aggregate digest cannot support authority",
    )
    return {
        **scientific_payload,
        "artifact_content_manifest": manifest,
        "scientific_digest": scientific_digest,
    }


def markdown(receipt: dict[str, Any]) -> str:
    identity = receipt["scientific_execution_identity"]
    lines = [
        "# A1 Two-Arm Authority Receipt",
        "",
        f"- Decision: `{receipt['predeclared_decision']}`",
        f"- Profile: `{receipt['profile']}`",
        f"- Scientific execution head: `{identity['scientific_execution_head_sha']}`",
        f"- Scientific execution tree: `{identity['scientific_execution_head_tree']}`",
        f"- Tested execution merge: `{identity['tested_execution_merge_sha']}`",
        f"- Tested execution merge tree: `{identity['tested_execution_merge_tree']}`",
        f"- Execution workflow: `{identity['execution_workflow_run_id']}` attempt `{identity['execution_workflow_run_attempt']}`",
        f"- Scientific digest: `{receipt['scientific_digest']}`",
        "",
        "The ordinary committed arm remains `legacy-fixed`. This receipt does not activate the candidate and makes no timing, ranking, speedup, or equal-error-contribution claim.",
        "",
        "## Arm totals",
        "",
        "| Arm | Attempts | Accepted | Rejected | RHS | JVP | Linear matvecs | Hires positive control |",
        "|---|---:|---:|---:|---:|---:|---:|---|",
    ]
    for arm in ARMS:
        totals = receipt["per_arm_totals"][arm]
        lines.append(
            f"| `{arm}` | {totals['attempts']} | {totals['accepted_steps']} | {totals['rejected_steps']} | "
            f"{totals['rhs_evaluations']} | {totals['jvp_vectors']} | {totals['linear_matvecs']} | "
            f"{str(receipt['hires_positive_control'][arm]).lower()} |"
        )
    lines.extend(
        [
            "",
            "## Safety and provenance",
            "",
            f"- Complete cells: {len(receipt['complete_cell_keys'])}",
            f"- Unsafe recommendations: {len(receipt['unsafe_recommendation_keys'])}",
            f"- Audit unsafe events: {len(receipt['audit_unsafe_event_keys'])}",
            f"- Artifact manifest entries: {len(receipt['artifact_content_manifest'])}",
            "- Receipt commit/tree and post-receipt workflow IDs are intentionally external late-bound evidence.",
            "",
            "## Limitations",
            "",
        ]
    )
    lines.extend(f"- {item}" for item in receipt["limitations"])
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cells-dir", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-markdown", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        receipt = summarize(args.cells_dir)
    except (OSError, ReceiptValidationError) as error:
        print(f"STOP_INVALID: A1 receipt validation failed: {error}", file=sys.stderr)
        return 1
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(
        json.dumps(receipt, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    args.output_markdown.write_text(markdown(receipt), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
