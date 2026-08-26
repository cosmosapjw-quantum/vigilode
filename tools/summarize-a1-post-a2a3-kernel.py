#!/usr/bin/env python3
"""Validate and summarize the frozen 2x6 post-A2/A3 kernel experiment."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable

ENVELOPE_SCHEMA = "vigilode-a1-post-a2a3-kernel-execution-cell-v1"
PAYLOAD_SCHEMA = "vigilode-a1-post-a2a3-kernel-atomic-cell-v1"
PAYLOAD_STATUS = "EXPLORATORY/NONAUTHORITATIVE"
RECEIPT_SCHEMA = "vigilode-a1-two-arm-atomic-cell-v2"
AGGREGATE_SCHEMA = "vigilode-a1-post-a2a3-kernel-aggregate-v1"
PROFILE = "enforced-budget-holdout-320"
TOLERANCE_ARM = "legacy-fixed"
OUTER_RTOL = 1.0e-5
LINEAR_RTOL = 1.0e-10
LINEAR_ATOL = 1.0e-12
PHI_RELATIVE_TOLERANCE = 3.0e-7
PHI_ABSOLUTE_TOLERANCE = 3.0e-9
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
IDENTITY_KEYS = (
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
WORK_KEYS = (
    "attempts",
    "accepted_steps",
    "rejected_steps",
    "rhs_evaluations",
    "jvp_vectors",
    "linear_matvecs",
)
REQUIRED_HARD_GATES = (
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


def bits_equal(left: Any, right: Any) -> bool:
    try:
        return float(left).hex() == float(right).hex()
    except (TypeError, ValueError, OverflowError):
        return False


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def payload_digest(value: Any) -> str:
    import hashlib

    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def load_envelopes(cells_dir: Path) -> list[tuple[Path, dict[str, Any]]]:
    loaded: list[tuple[Path, dict[str, Any]]] = []
    for path in sorted(cells_dir.rglob("*.json")):
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(value, dict) and value.get("schema") == ENVELOPE_SCHEMA:
            loaded.append((path, value))
    return loaded


def integer_nonnegative(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def validate_event(event: dict[str, Any], label: str, errors: list[str]) -> None:
    eligible = event.get("audit_full_e_eligible") is True
    attempted = event.get("audit_full_e_attempted") is True
    completed = event.get("audit_full_e_completed") is True
    status = event.get("audit_evidence_status")
    if eligible:
        if not attempted or not completed or status != "complete":
            errors.append(f"{label}: eligible audit event is not complete")
        if event.get("audit_full_e_total_error") is None:
            errors.append(f"{label}: completed audit event lacks total error")
        if not isinstance(event.get("audit_full_e_locally_admissible"), bool):
            errors.append(f"{label}: completed audit event lacks admissibility boolean")
        if not isinstance(event.get("audit_full_e_work"), dict):
            errors.append(f"{label}: completed audit event lacks work ledger")
    else:
        if attempted or completed or status != "ineligible":
            errors.append(f"{label}: ineligible audit event has inconsistent state")
        if not event.get("audit_full_e_failure"):
            errors.append(f"{label}: ineligible audit event lacks explicit reason")
    if event.get("recommended") is True and event.get("audit_unsafe") is True:
        errors.append(f"{label}: unsafe recommendation")


def validate_payload(
    envelope: dict[str, Any], expected_pair: tuple[str, str], label: str, errors: list[str]
) -> dict[str, Any] | None:
    family, kernel = expected_pair
    if envelope.get("status") != PAYLOAD_STATUS:
        errors.append(f"{label}: envelope status drift")
    if envelope.get("execution_state") != "COMPLETE":
        errors.append(
            f"{label}: execution_state={envelope.get('execution_state')} errors={envelope.get('errors')}"
        )
    if envelope.get("deterministic_replay") is not True:
        errors.append(f"{label}: deterministic replay not established")
    if envelope.get("family") != family or envelope.get("kernel_arm") != kernel:
        errors.append(f"{label}: envelope identity mismatch")
    if envelope.get("payload_sha256_first") != envelope.get("payload_sha256_second"):
        errors.append(f"{label}: deterministic payload digest mismatch")
    if envelope.get("errors") not in ([], None):
        errors.append(f"{label}: COMPLETE envelope retains errors")
    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        errors.append(f"{label}: missing scientific payload")
        return None
    actual_payload_digest = payload_digest(payload)
    if envelope.get("payload_sha256_first") != actual_payload_digest:
        errors.append(f"{label}: envelope digest does not bind the scientific payload")
    if payload.get("schema") != PAYLOAD_SCHEMA or payload.get("status") != PAYLOAD_STATUS:
        errors.append(f"{label}: scientific payload schema/status drift")
    if payload.get("kernel_arm") != kernel or payload.get("tolerance_arm") != TOLERANCE_ARM:
        errors.append(f"{label}: kernel/tolerance identity drift")
    receipt = payload.get("receipt")
    if not isinstance(receipt, dict):
        errors.append(f"{label}: missing nested A1 receipt")
        return None
    if receipt.get("schema") != RECEIPT_SCHEMA:
        errors.append(f"{label}: nested receipt schema drift")
    if receipt.get("profile") != PROFILE or receipt.get("family") != family:
        errors.append(f"{label}: profile/family drift")
    if receipt.get("arm") != TOLERANCE_ARM:
        errors.append(f"{label}: nested tolerance arm is not LegacyFixed")
    if not bits_equal(receipt.get("outer_rtol"), OUTER_RTOL):
        errors.append(f"{label}: outer rtol drift")
    if not bits_equal(receipt.get("linear_rtol"), LINEAR_RTOL):
        errors.append(f"{label}: linear rtol drift")
    if not bits_equal(receipt.get("linear_atol"), LINEAR_ATOL):
        errors.append(f"{label}: linear atol drift")
    if not bits_equal(receipt.get("phi_relative_tolerance"), PHI_RELATIVE_TOLERANCE):
        errors.append(f"{label}: phi relative tolerance drift")
    if not bits_equal(receipt.get("phi_absolute_tolerance"), PHI_ABSOLUTE_TOLERANCE):
        errors.append(f"{label}: phi absolute tolerance drift")
    if receipt.get("switching_active") is not False:
        errors.append(f"{label}: active switching was enabled")
    if not bits_equal(receipt.get("frozen_zeta34_tau", float("nan")), FROZEN_TAU):
        errors.append(f"{label}: frozen zeta34 threshold drift")
    for key in WORK_KEYS:
        if not integer_nonnegative(receipt.get(key)):
            errors.append(f"{label}: invalid work counter {key}")
    if integer_nonnegative(receipt.get("attempts")) and integer_nonnegative(
        receipt.get("accepted_steps")
    ) and integer_nonnegative(receipt.get("rejected_steps")):
        if receipt["accepted_steps"] + receipt["rejected_steps"] != receipt["attempts"]:
            errors.append(f"{label}: accepted+rejected != attempts")
    trace_digest = receipt.get("trace_digest")
    if not isinstance(trace_digest, str) or len(trace_digest) != 64:
        errors.append(f"{label}: invalid trace digest")
    gates = receipt.get("hard_gates")
    if not isinstance(gates, dict):
        errors.append(f"{label}: missing hard-gate ledger")
    else:
        for gate in REQUIRED_HARD_GATES:
            if gates.get(gate) is not True:
                errors.append(f"{label}: hard gate {gate} is not true")
    events = receipt.get("event_rows")
    if not isinstance(events, list):
        errors.append(f"{label}: event_rows missing")
        events = []
    event_keys: set[str] = set()
    for index, event in enumerate(events):
        if not isinstance(event, dict):
            errors.append(f"{label}: event row {index} is not an object")
            continue
        key = event.get("event_key")
        if not isinstance(key, str) or not key:
            errors.append(f"{label}: event row {index} lacks key")
        elif key in event_keys:
            errors.append(f"{label}: duplicate event key {key}")
        else:
            event_keys.add(key)
        validate_event(event, f"{label}:{key or index}", errors)
    recommendations = receipt.get("recommendation_rows")
    if not isinstance(recommendations, list):
        errors.append(f"{label}: recommendation_rows missing")
        recommendations = []
    recommendation_keys = [row.get("event_key") for row in recommendations if isinstance(row, dict)]
    expected_recommendations = sorted(
        event.get("event_key")
        for event in events
        if isinstance(event, dict) and event.get("recommended") is True
    )
    if sorted(recommendation_keys) != expected_recommendations:
        errors.append(f"{label}: recommendation ledger does not match event rows")
    return receipt


def sum_work(receipts: Iterable[dict[str, Any]]) -> dict[str, int]:
    return {key: sum(int(receipt[key]) for receipt in receipts) for key in WORK_KEYS}


def aggregate(cells_dir: Path) -> dict[str, Any]:
    errors: list[str] = []
    loaded = load_envelopes(cells_dir)
    by_pair: dict[tuple[str, str], tuple[Path, dict[str, Any]]] = {}
    for path, envelope in loaded:
        pair = (str(envelope.get("family")), str(envelope.get("kernel_arm")))
        if pair in by_pair:
            errors.append(f"duplicate matrix cell {pair}: {by_pair[pair][0]} and {path}")
        else:
            by_pair[pair] = (path, envelope)
    expected = {(family, kernel) for family in FAMILIES for kernel in KERNELS}
    missing = sorted(expected - set(by_pair))
    extra = sorted(set(by_pair) - expected)
    if missing:
        errors.append(f"missing matrix cells: {missing}")
    if extra:
        errors.append(f"unexpected matrix cells: {extra}")

    receipts: dict[tuple[str, str], dict[str, Any]] = {}
    for pair in sorted(expected & set(by_pair)):
        path, envelope = by_pair[pair]
        receipt = validate_payload(envelope, pair, str(path), errors)
        if receipt is not None:
            receipts[pair] = receipt

    identities: list[dict[str, Any]] = []
    for receipt in receipts.values():
        identities.append({key: receipt.get(key) for key in IDENTITY_KEYS})
    scientific_identity = identities[0] if identities else None
    if scientific_identity is not None and any(item != scientific_identity for item in identities[1:]):
        errors.append("scientific execution identity differs across cells")

    for kernel in KERNELS:
        hires = receipts.get(("hires-ramped", kernel))
        if hires is None:
            continue
        positive_control = any(
            isinstance(event, dict)
            and event.get("recommended") is False
            and isinstance(event.get("zeta34_signed_margin"), (int, float))
            and event["zeta34_signed_margin"] > 0.0
            and event.get("audit_unsafe") is True
            and event.get("audit_evidence_status") == "complete"
            for event in hires.get("event_rows", [])
        )
        if not positive_control:
            errors.append(f"{kernel}: Hires unrecommended unsafe positive control missing")

    totals_by_kernel: dict[str, dict[str, int]] = {}
    for kernel in KERNELS:
        kernel_receipts = [receipts[(family, kernel)] for family in FAMILIES if (family, kernel) in receipts]
        if len(kernel_receipts) == len(FAMILIES):
            totals_by_kernel[kernel] = sum_work(kernel_receipts)

    family_rows: list[dict[str, Any]] = []
    for family in FAMILIES:
        legacy = receipts.get((family, KERNELS[0]))
        candidate = receipts.get((family, KERNELS[1]))
        if legacy is None or candidate is None:
            continue
        legacy_events = {event["event_key"] for event in legacy.get("event_rows", [])}
        candidate_events = {event["event_key"] for event in candidate.get("event_rows", [])}
        row: dict[str, Any] = {
            "family": family,
            "legacy": {key: legacy[key] for key in WORK_KEYS},
            "candidate": {key: candidate[key] for key in WORK_KEYS},
            "candidate_minus_legacy": {
                key: int(candidate[key]) - int(legacy[key]) for key in WORK_KEYS
            },
            "legacy_event_count": len(legacy_events),
            "candidate_event_count": len(candidate_events),
            "event_key_overlap": len(legacy_events & candidate_events),
            "legacy_recommendation_count": len(legacy.get("recommendation_rows", [])),
            "candidate_recommendation_count": len(candidate.get("recommendation_rows", [])),
            "legacy_trace_digest": legacy.get("trace_digest"),
            "candidate_trace_digest": candidate.get("trace_digest"),
        }
        family_rows.append(row)

    aggregate_delta: dict[str, int] = {}
    if all(kernel in totals_by_kernel for kernel in KERNELS):
        aggregate_delta = {
            key: totals_by_kernel[KERNELS[1]][key] - totals_by_kernel[KERNELS[0]][key]
            for key in WORK_KEYS
        }

    cell_summaries: list[dict[str, Any]] = []
    for pair in sorted(receipts):
        family, kernel = pair
        receipt = receipts[pair]
        cell_summaries.append(
            {
                "family": family,
                "kernel_arm": kernel,
                **{key: receipt[key] for key in WORK_KEYS},
                "event_count": len(receipt.get("event_rows", [])),
                "recommendation_count": len(receipt.get("recommendation_rows", [])),
                "trace_digest": receipt.get("trace_digest"),
                "hard_gates_passed": receipt.get("hard_gates", {}).get("passed") is True,
            }
        )

    verdict = "COMPLETE" if not errors and len(receipts) == 12 else "STOP_INVALID"
    return {
        "schema": AGGREGATE_SCHEMA,
        "status": PAYLOAD_STATUS,
        "verdict": verdict,
        "claim_ceiling": (
            "Raw same-physics/same-tolerance/same-output kernel work and trajectory comparison only; "
            "no activation, timing, ranking, speedup, STDB-Lite, tag, or release claim."
        ),
        "profile": PROFILE,
        "tolerance_arm": TOLERANCE_ARM,
        "linear_rtol": LINEAR_RTOL,
        "linear_atol": LINEAR_ATOL,
        "frozen_zeta34_tau": FROZEN_TAU,
        "kernel_arms": list(KERNELS),
        "families": list(FAMILIES),
        "expected_cell_count": 12,
        "observed_envelope_count": len(loaded),
        "observed_cell_count": len(receipts),
        "scientific_execution_identity": scientific_identity,
        "totals_by_kernel": totals_by_kernel,
        "candidate_minus_legacy_total": aggregate_delta,
        "family_comparison": family_rows,
        "cells": cell_summaries,
        "validation_errors": errors,
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# A1 post-A2/A3 kernel-isolation receipt",
        "",
        f"- Status: `{report['status']}`",
        f"- Verdict: `{report['verdict']}`",
        f"- Profile: `{report['profile']}`",
        f"- Tolerance: `{report['tolerance_arm']}` (`rtol=1e-10`, `atol=1e-12`)",
        f"- Frozen tau: `{report['frozen_zeta34_tau']}`",
        f"- Matrix: `{report['observed_cell_count']}/{report['expected_cell_count']}` validated cells "
        f"(`{report['observed_envelope_count']}` envelopes found)",
        "- Claim ceiling: raw work/trajectory comparison only; no activation, timing, ranking, or speedup claim.",
        "",
        "## Per-family raw work",
        "",
        "| Family | Kernel | Attempts | Accepted | Rejected | RHS | JVP | Linear matvecs | Events | Recommendations |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for cell in report["cells"]:
        lines.append(
            "| {family} | {kernel_arm} | {attempts} | {accepted_steps} | {rejected_steps} | "
            "{rhs_evaluations} | {jvp_vectors} | {linear_matvecs} | {event_count} | "
            "{recommendation_count} |".format(**cell)
        )
    lines.extend(
        [
            "",
            "## Candidate minus legacy (signed raw deltas)",
            "",
            "| Family | Attempts | Accepted | Rejected | RHS | JVP | Linear matvecs | Legacy events | Candidate events | Event-key overlap |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for row in report["family_comparison"]:
        delta = row["candidate_minus_legacy"]
        lines.append(
            f"| {row['family']} | {delta['attempts']} | {delta['accepted_steps']} | "
            f"{delta['rejected_steps']} | {delta['rhs_evaluations']} | {delta['jvp_vectors']} | "
            f"{delta['linear_matvecs']} | {row['legacy_event_count']} | "
            f"{row['candidate_event_count']} | {row['event_key_overlap']} |"
        )
    lines.extend(["", "## Validation"])
    if report["validation_errors"]:
        lines.append("")
        lines.extend(f"- STOP_INVALID: {error}" for error in report["validation_errors"])
    else:
        lines.extend(
            [
                "",
                "- Exactly 12 unique cells were present.",
                "- Both kernels used LegacyFixed and the frozen profile/tau.",
                "- Every cell reproduced deterministically in a second execution.",
                "- Independent full-E audit evidence was complete or explicitly ineligible.",
                "- Hires positive control was present for both kernels.",
                "- No unsafe recommendation or hard-gate failure was observed.",
            ]
        )
    lines.extend(
        [
            "",
            "## Non-claims",
            "",
            "This receipt does not activate a kernel, rank either arm, establish wall-time performance, "
            "claim speedup, introduce STDB-Lite, or authorize a tag or release.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cells-dir", required=True, type=Path)
    parser.add_argument("--output-json", required=True, type=Path)
    parser.add_argument("--output-markdown", required=True, type=Path)
    args = parser.parse_args()
    report = aggregate(args.cells_dir)
    args.output_json.write_text(
        json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    args.output_markdown.write_text(render_markdown(report), encoding="utf-8")
    print(f"{report['verdict']}: {report['observed_cell_count']}/12 cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
