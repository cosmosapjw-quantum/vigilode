#!/usr/bin/env python3
"""Semantic evidence-v3 validator for historical K0 WU-04 raw receipts.

This tool deliberately separates:
- exact raw-file SHA-256 provenance; and
- canonical numerical/scientific content identity.

It never invents absent historical fields.  In particular, missing legacy
signed-residual digests remain null and are covered by the current-source
vector mutation test rather than fabricated receipt bytes.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from typing import Any, Iterable

RTOL = 1.0e-10
ATOL = 1.0e-12
TAU = 13.39706618860016
MARKER = "EVIDENCE_V3_PASS"
RECEIPT_KEYS = {
    "profile", "family", "attempts", "accepted_steps", "rejected_steps",
    "rhs_evaluations", "jvp_vectors", "linear_matvecs", "trace_digest",
    "switching_active", "frozen_zeta34_tau", "event_rows",
    "recommendation_rows", "hard_gates",
}
STAGE_KEYS = {"attempt_id", "stage_index", "execution_state", "work"}
HARD_GATES = {
    "all_rjf_trajectories_successful", "rjf_trace_exact_excluding_wall",
    "zero_budget_breaches", "prefix_transactions_resolved",
    "zero_continuation_failures", "zero_unsafe_recommendations",
    "work_ledgers_exact", "realized_work_ratios_finite",
    "resume_cardinality_exact", "shadow_implicit_expensive_work_zero",
    "active_switching_false", "passed",
}
ARMS = {"legacy-restarted-gmres", "incremental-givens-candidate"}
FAMILIES = {
    "robertson-ramped", "hires-ramped", "van-der-pol-ramped",
    "rotating-nonnormal", "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
}


class EvidenceError(RuntimeError):
    pass


def fail(message: str) -> None:
    raise EvidenceError(message)


def finite_number(value: Any, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{name} must be a finite number")
    value = float(value)
    if not math.isfinite(value):
        fail(f"{name} must be finite")
    return value


def nonnegative_int(value: Any, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        fail(f"{name} must be a nonnegative integer")
    return value


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def walk(value: Any) -> Iterable[Any]:
    yield value
    if isinstance(value, dict):
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


def mappings_with_keys(document: Any, required: set[str]) -> list[dict[str, Any]]:
    return [value for value in walk(document)
            if isinstance(value, dict) and required <= set(value)]


def stage_arrays(document: Any) -> list[list[dict[str, Any]]]:
    answer: list[list[dict[str, Any]]] = []
    for value in walk(document):
        if (isinstance(value, list) and value
                and all(isinstance(item, dict) and STAGE_KEYS <= set(item)
                        for item in value)):
            answer.append(value)
    return answer


def exactly_one(values: list[Any], name: str) -> Any:
    if len(values) != 1:
        fail(f"expected exactly one {name}, found {len(values)}")
    return values[0]


def collect_key(document: Any, key: str) -> list[Any]:
    return [value[key] for value in walk(document)
            if isinstance(value, dict) and key in value and value[key] is not None]


def unique_scalar(document: Any, key: str, pattern_len: int | None = None) -> Any:
    values = collect_key(document, key)
    unique = []
    for value in values:
        if value not in unique:
            unique.append(value)
    if len(unique) != 1:
        fail(f"expected one unique {key}, found {unique!r}")
    value = unique[0]
    if pattern_len is not None:
        if (not isinstance(value, str) or len(value) != pattern_len
                or any(ch not in "0123456789abcdef" for ch in value)):
            fail(f"{key} is not lowercase {pattern_len}-hex")
    return value


def tolerance_pairs(document: Any) -> list[tuple[float, float]]:
    pairs: list[tuple[float, float]] = []
    for value in walk(document):
        if not isinstance(value, dict):
            continue
        if "linear_rtol" in value and "linear_atol" in value:
            pairs.append((finite_number(value["linear_rtol"], "linear_rtol"),
                          finite_number(value["linear_atol"], "linear_atol")))
        elif "rtol" in value and "atol" in value:
            pairs.append((finite_number(value["rtol"], "rtol"),
                          finite_number(value["atol"], "atol")))
    return pairs


def derive_tolerance(document: Any) -> tuple[float, float]:
    pairs = tolerance_pairs(document)
    if not pairs:
        fail("raw evidence contains no numerical tolerance pair")
    if any(pair != pairs[0] for pair in pairs):
        fail(f"raw evidence has inconsistent numerical tolerances: {pairs}")
    if pairs[0] != (RTOL, ATOL):
        fail(f"LegacyFixed numerical tolerance drift: {pairs[0]}")
    return pairs[0]


def normalize_profile(value: Any) -> str:
    token = str(value).strip().lower().replace("_", "-")
    token = token.replace("enforcedbudgetholdout320", "enforced-budget-holdout-320")
    if token != "enforced-budget-holdout-320":
        fail(f"unexpected profile {value!r}")
    return token


def normalize_arm(document: Any) -> str:
    values = {str(value) for value in collect_key(document, "kernel_arm")}
    if len(values) != 1 or not values <= ARMS:
        fail(f"invalid or ambiguous kernel_arm values: {sorted(values)}")
    return values.pop()


def normalize_family(receipt: dict[str, Any]) -> str:
    value = str(receipt["family"])
    if value not in FAMILIES:
        fail(f"unexpected family {value!r}")
    return value


def valid_digest_or_none(value: Any) -> bool:
    return (value is None or (isinstance(value, str) and len(value) == 64
                              and all(ch in "0123456789abcdef" for ch in value)))


def signed_coverage(stages: list[dict[str, Any]]) -> str:
    values = [stage.get("signed_residual_digest") for stage in stages
              if stage.get("execution_state") == "COMPLETE"]
    if any(not valid_digest_or_none(value) for value in values):
        fail("invalid signed-residual digest")
    present = [value is not None for value in values]
    if not present or not any(present):
        return "LEGACY_NOT_RECORDED"
    if all(present):
        return "RECORDED"
    fail("mixed historical signed-residual coverage is ambiguous")


def stage_semantic(stage: dict[str, Any]) -> dict[str, Any]:
    work = stage.get("work")
    if not isinstance(work, dict):
        fail("stage work ledger is missing")
    for field in ("linear_matvecs", "diagnostic_matvecs",
                  "operator_applies_total", "telemetry_jvp_overhead"):
        nonnegative_int(work.get(field), f"work.{field}")
    if work["operator_applies_total"] != work["linear_matvecs"] + work["diagnostic_matvecs"]:
        fail("operator apply accounting does not close")
    projected = {
        "execution_state": stage.get("execution_state"),
        "attempt_id": nonnegative_int(stage.get("attempt_id"), "attempt_id"),
        "stage_index": nonnegative_int(stage.get("stage_index"), "stage_index"),
        "solver_method": stage.get("solver_method"),
        "initial_guess_source": stage.get("initial_guess_source"),
        "initial_true_residual": stage.get("initial_true_residual"),
        "final_true_residual": stage.get("final_true_residual"),
        "work": work,
        "geometry": stage.get("geometry"),
        "failure": stage.get("failure"),
    }
    for name in ("initial_true_residual", "final_true_residual"):
        if projected[name] is not None:
            if finite_number(projected[name], name) < 0:
                fail(f"{name} must be nonnegative")
    return projected


def audit_complete(events: Any) -> bool:
    if not isinstance(events, list):
        fail("event_rows must be an array")
    for event in events:
        if not isinstance(event, dict):
            fail("event row is not an object")
        if event.get("audit_full_e_eligible") is True:
            if event.get("audit_full_e_completed") is not True:
                return False
            if event.get("audit_full_e_failure") is not None:
                return False
    return True


def unsafe_count(events: list[dict[str, Any]]) -> int:
    return sum(event.get("recommended") is True and event.get("audit_unsafe") is True
               for event in events)


def validate_hard_gates(value: Any) -> dict[str, bool]:
    if not isinstance(value, dict) or set(value) != HARD_GATES:
        fail(f"hard-gate set drift: {sorted(set(value) if isinstance(value, dict) else [])}")
    if any(value[name] is not True for name in HARD_GATES):
        fail("one or more hard gates are false")
    return {name: True for name in sorted(HARD_GATES)}


def derive(raw_document: dict[str, Any], raw_bytes: bytes, raw_path: str) -> dict[str, Any]:
    if not isinstance(raw_document, dict):
        fail("raw document must be an object")
    if raw_document.get("error") is not None:
        fail("non-null outer error cannot be migrated as COMPLETE")
    receipt = exactly_one(mappings_with_keys(raw_document, RECEIPT_KEYS), "receipt object")
    stages = exactly_one(stage_arrays(raw_document), "stage array")
    arm = normalize_arm(raw_document)
    family = normalize_family(receipt)
    rtol, atol = derive_tolerance(raw_document)
    head = unique_scalar(raw_document, "scientific_execution_head_sha", 40)
    tree = unique_scalar(raw_document, "scientific_execution_head_tree", 40)
    profile = normalize_profile(receipt["profile"])
    if finite_number(receipt["frozen_zeta34_tau"], "frozen_zeta34_tau") != TAU:
        fail("frozen zeta34 threshold drift")
    gates = validate_hard_gates(receipt["hard_gates"])
    events = receipt["event_rows"]
    recommendations = receipt["recommendation_rows"]
    if not isinstance(recommendations, list):
        fail("recommendation_rows must be an array")
    if not audit_complete(events):
        fail("audit full-E evidence is incomplete")
    unsafe = unsafe_count(events)
    if unsafe:
        fail(f"unsafe recommendation count is {unsafe}")
    coverage = signed_coverage(stages)
    stage_projection = [stage_semantic(stage) for stage in stages]
    numerical = {
        "kernel_arm": arm,
        "linear_rtol": rtol,
        "linear_atol": atol,
        "profile": profile,
        "family": family,
        "frozen_zeta34_tau": TAU,
        "attempts": nonnegative_int(receipt["attempts"], "attempts"),
        "accepted_steps": nonnegative_int(receipt["accepted_steps"], "accepted_steps"),
        "rejected_steps": nonnegative_int(receipt["rejected_steps"], "rejected_steps"),
        "rhs_evaluations": nonnegative_int(receipt["rhs_evaluations"], "rhs_evaluations"),
        "jvp_vectors": nonnegative_int(receipt["jvp_vectors"], "jvp_vectors"),
        "linear_matvecs": nonnegative_int(receipt["linear_matvecs"], "linear_matvecs"),
        "trace_digest": receipt["trace_digest"],
        "switching_active": receipt["switching_active"],
        "event_rows": events,
        "recommendation_rows": recommendations,
        "hard_gates": gates,
        "audit_full_e_complete": True,
        "unsafe_recommendations": 0,
        "stages": stage_projection,
    }
    campaign = {
        "linear_rtol": rtol,
        "linear_atol": atol,
        "signed_residual_coverage": coverage,
        "attempts": numerical["attempts"],
        "accepted_steps": numerical["accepted_steps"],
        "rejected_steps": numerical["rejected_steps"],
        "rhs_evaluations": numerical["rhs_evaluations"],
        "jvp_vectors": numerical["jvp_vectors"],
        "linear_matvecs": numerical["linear_matvecs"],
        "trace_digest": numerical["trace_digest"],
        "event_count": len(events),
        "recommendation_count": len(recommendations),
        "hard_gates": gates,
        "audit_full_e_complete": True,
        "unsafe_recommendations": 0,
        "numerical_payload_sha256": digest(numerical),
        "raw_stage_payload_sha256": digest(stage_projection),
    }
    normalized_stages = []
    for stage in stages:
        item = copy.deepcopy(stage)
        item["signed_residual_digest"] = stage.get("signed_residual_digest")
        normalized_stages.append(item)
    return {
        "schema": "vigilode-k0-cell-receipt/v3",
        "execution_state": "COMPLETE",
        "cell_id": f"{arm}::{family}",
        "family": family,
        "kernel_arm": arm,
        "profile": profile,
        "tolerance": {"kind": None, "rtol": rtol, "atol": atol},
        "frozen_zeta34_tau": TAU,
        "provenance": {
            "raw_receipt_path": raw_path,
            "raw_receipt_sha256": sha256_bytes(raw_bytes),
            "source_head": head,
            "source_tree": tree,
        },
        "campaign": campaign,
        "stages": normalized_stages,
        "failure": None,
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
    }


def validate_wrapper(raw: dict[str, Any], raw_bytes: bytes, raw_path: str,
                     wrapper: dict[str, Any]) -> dict[str, Any]:
    expected = derive(raw, raw_bytes, raw_path)
    if wrapper != expected:
        # Report semantic fields rather than treating a wrapper byte mismatch as science.
        differing = sorted(key for key in expected if wrapper.get(key) != expected[key])
        fail(f"wrapper differs from raw-derived semantic content: {differing}")
    return expected


def synthetic_raw() -> dict[str, Any]:
    work = {
        "linear_matvecs": 2,
        "diagnostic_matvecs": 1,
        "operator_applies_total": 3,
        "telemetry_jvp_overhead": 0,
        "preserved": True,
    }
    stages = [{
        "execution_state": "COMPLETE",
        "attempt_id": 0,
        "stage_index": i,
        "solver_method": "gmres",
        "initial_guess_source": "ZERO",
        "initial_true_residual": 1.0,
        "final_true_residual": 1.0e-12,
        "work": work,
        "geometry": {
            "scaled_nonlinear_remainder": 0.0,
            "rhs_to_retained_angle_rad": None,
            "rhs_singular_values": [1.0],
        },
        "failure": None,
    } for i in range(8)]
    receipt = {
        "profile": "EnforcedBudgetHoldout320",
        "family": "robertson-ramped",
        "linear_rtol": RTOL,
        "linear_atol": ATOL,
        "attempts": 1,
        "accepted_steps": 1,
        "rejected_steps": 0,
        "rhs_evaluations": 8,
        "jvp_vectors": 16,
        "linear_matvecs": 16,
        "trace_digest": "1" * 64,
        "switching_active": False,
        "frozen_zeta34_tau": TAU,
        "event_rows": [],
        "recommendation_rows": [],
        "hard_gates": {name: True for name in HARD_GATES},
        "stages": stages,
    }
    return {
        "schema": "historical-outer-envelope/v1",
        "kernel_arm": "legacy-restarted-gmres",
        "scientific_execution_head_sha": "2" * 40,
        "scientific_execution_head_tree": "3" * 40,
        "error": None,
        "payload": {"receipt": receipt},
        "transport": {"archive": "ignored.zip"},
    }


def self_test() -> dict[str, Any]:
    raw = synthetic_raw()
    raw_bytes = canonical_bytes(raw)
    first = derive(raw, raw_bytes, "raw.json")
    assert first["campaign"]["signed_residual_coverage"] == "LEGACY_NOT_RECORDED"
    assert all(stage["signed_residual_digest"] is None for stage in first["stages"])
    assert first["provenance"]["source_head"] == "2" * 40
    assert first["campaign"]["linear_rtol"] == RTOL
    # Representation-only metadata changes raw bytes, but not numerical content.
    changed = copy.deepcopy(raw)
    changed["transport"] = {"archive": "different.zip", "key_order": "irrelevant"}
    second = derive(changed, canonical_bytes(changed), "raw2.json")
    assert first["campaign"]["numerical_payload_sha256"] == second["campaign"]["numerical_payload_sha256"]
    assert first["provenance"]["raw_receipt_sha256"] != second["provenance"]["raw_receipt_sha256"]
    # Fabricating a historical signed digest is observable and forbidden by migration.
    fabricated = copy.deepcopy(first)
    fabricated["stages"][0]["signed_residual_digest"] = "a" * 64
    try:
        validate_wrapper(raw, raw_bytes, "raw.json", fabricated)
    except EvidenceError:
        pass
    else:
        fail("fabricated historical signed digest was accepted")
    # Genuine numerical drift remains fatal.
    drift = copy.deepcopy(raw)
    drift["payload"]["receipt"]["linear_rtol"] = 2.0e-10
    try:
        derive(drift, canonical_bytes(drift), "drift.json")
    except EvidenceError:
        pass
    else:
        fail("numerical tolerance drift was accepted")
    # Non-null outer error cannot become COMPLETE.
    errored = copy.deepcopy(raw)
    errored["error"] = {"message": "failed"}
    try:
        derive(errored, canonical_bytes(errored), "error.json")
    except EvidenceError:
        pass
    else:
        fail("non-null raw error was accepted as COMPLETE")
    return {
        "historical_missing_labels": "PASS",
        "outer_source_identity": "PASS",
        "null_error_semantics": "PASS",
        "legacy_signed_residual": "PASS",
        "representation_digest_invariance": "PASS",
        "genuine_numerical_drift_rejected": "PASS",
    }


def raw_candidates(root: Path) -> list[Path]:
    answer = []
    for path in root.rglob("*.json"):
        if any(part in {"evidence_v2", "evidence_v3", "review", "reviews"}
               for part in path.parts):
            continue
        try:
            document = json.loads(path.read_text())
            if (len(mappings_with_keys(document, RECEIPT_KEYS)) == 1
                    and len(stage_arrays(document)) == 1):
                answer.append(path)
        except (OSError, json.JSONDecodeError, EvidenceError):
            continue
    return sorted(answer)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--raw-dir")
    parser.add_argument("--output-dir")
    parser.add_argument("--require-cells", type=int, default=12)
    parser.add_argument("--report")
    args = parser.parse_args()
    results: dict[str, Any] = {}
    if args.self_test:
        results["self_test"] = self_test()
    if args.raw_dir:
        root = Path(args.raw_dir).resolve()
        files = raw_candidates(root)
        if len(files) != args.require_cells:
            fail(f"expected {args.require_cells} raw cells, found {len(files)}")
        output = Path(args.output_dir).resolve() if args.output_dir else None
        if output:
            output.mkdir(parents=True, exist_ok=True)
        receipts = []
        for path in files:
            raw_bytes = path.read_bytes()
            raw = json.loads(raw_bytes)
            wrapper = derive(raw, raw_bytes, str(path.relative_to(root)))
            receipts.append({
                "path": str(path),
                "raw_sha256": wrapper["provenance"]["raw_receipt_sha256"],
                "numerical_payload_sha256": wrapper["campaign"]["numerical_payload_sha256"],
                "raw_stage_payload_sha256": wrapper["campaign"]["raw_stage_payload_sha256"],
                "signed_residual_coverage": wrapper["campaign"]["signed_residual_coverage"],
            })
            if output:
                name = wrapper["cell_id"].replace("::", "__") + ".json"
                (output / name).write_bytes(canonical_bytes(wrapper) + b"\n")
        results["cells"] = receipts
    if not results:
        parser.error("select --self-test and/or --raw-dir")
    payload = {"status": "PASS", "marker": MARKER, "results": results}
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.report:
        Path(args.report).write_text(text)
    print(text, end="")


if __name__ == "__main__":
    try:
        main()
    except (EvidenceError, OSError, json.JSONDecodeError) as exc:
        print(json.dumps({"status": "STOP_INVALID", "error": str(exc)}, indent=2))
        raise SystemExit(2)
