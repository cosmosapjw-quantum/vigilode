#!/usr/bin/env python3
"""Fail-closed verifier for the v3.6 frozen full-E runtime shadow campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path
from typing import Any


FROZEN_TAU = 13.39706618860016
PROFILES = {
    "calibration96": "stage-growth-calibration-96",
    "calibration192": "stage-growth-calibration-192",
    "calibration256": "stage-growth-calibration-256",
    "holdout320": "enforced-budget-holdout-320",
    "holdout384": "stage-growth-holdout-384",
}
FAMILIES = (
    "robertson",
    "hires",
    "van-der-pol",
    "rotating-nonnormal",
    "nonautonomous-forcing",
    "semilinear",
)
PREFIX_FIELDS = (
    "trajectory_id",
    "family",
    "dimension",
    "rtol",
    "decision_accepted_step",
    "feature_value",
    "target_attempt_index",
    "target_accepted_steps_before",
    "t_start",
    "h",
    "target_r_attempt_accepted",
    "target_r_error_norm",
    "committed_rjf_jvp_before_target",
    "budget_reserve_jvp",
    "budget_cap_jvp",
    "budget_fraction",
    "budget_admitted",
    "budget_exhausted",
    "budget_breached",
    "prefix_succeeded",
    "prefix_failure",
    "actual_prefix_jvp_vectors",
    "prefix_work",
    "normalized_stage_growth_a34",
    "rho2",
    "rho3",
    "rho4",
    "stage_log_slope_s23",
    "stage_log_slope_s34",
    "stage_log_curvature_kappa234",
    "remainder_chi23",
    "remainder_chi34",
    "remainder_chi24",
    "remainder_q34_perp",
    "remainder_delta_chi",
    "quadratic_drift_zeta23",
    "quadratic_drift_zeta34",
    "quadratic_drift_relative",
)
PREFLIGHT_FIELD_MAP = {
    "profile": lambda row: f"N{row['dimension']}",
    "trajectory_id": "trajectory_id",
    "family": "family",
    "dimension": "dimension",
    "rtol": "rtol",
    "decision_accepted_step": "decision_accepted_step",
    "target_attempt_index": "target_attempt_index",
    "t_start": "t_start",
    "h": "h",
    "zeta34": "quadratic_drift_zeta34",
    "prefix_work": "prefix_work",
    "continuation_work": "continuation_work",
    "full_e_work": "shadow_full_e_work",
    "full_e_total_error": "shadow_full_e_total_error",
    "full_e_locally_admissible": "shadow_full_e_locally_admissible",
    "target_r_attempt_accepted": "target_r_attempt_accepted",
    "target_rjf_jvp_vectors": "target_rjf_jvp_vectors",
    "prefix_over_target_rjf_jvp": "prefix_over_target_rjf_jvp",
    "continuation_over_target_rjf_jvp": "continuation_over_target_rjf_jvp",
    "full_e_over_target_rjf_jvp": "full_e_over_target_rjf_jvp",
}
HARD_GATE_FIELDS = (
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
PARITY_FIELDS = (
    "attempt_rows_exact_excluding_wall",
    "accepted_rows_exact_excluding_wall",
    "trajectories_exact",
    "passed",
)


class VerificationError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)


def float_bits(value: float) -> bytes:
    return struct.pack(">d", value)


def exact_equal(left: Any, right: Any) -> bool:
    if isinstance(left, float) or isinstance(right, float):
        return isinstance(left, (int, float)) and not isinstance(left, bool) and isinstance(
            right, (int, float)
        ) and not isinstance(right, bool) and float_bits(float(left)) == float_bits(float(right))
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(exact_equal(left[key], right[key]) for key in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(exact_equal(a, b) for a, b in zip(left, right))
    return left == right


def require_exact(left: Any, right: Any, label: str) -> None:
    require(exact_equal(left, right), f"exact mismatch at {label}: {left!r} != {right!r}")


def rows_without_wall(rows: list[dict[str, Any]], wall_field: str) -> list[dict[str, Any]]:
    return [{key: value for key, value in row.items() if key != wall_field} for row in rows]


def load_json(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise VerificationError(f"cannot load {path}: {error}") from error
    require(isinstance(value, dict), f"top-level JSON object required: {path}")
    return value


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def add_work(total: dict[str, int], work: dict[str, Any] | None, label: str) -> None:
    if work is None:
        return
    require(work.keys() == total.keys(), f"WorkCounters shape mismatch at {label}")
    for key, value in work.items():
        require(isinstance(value, int) and value >= 0, f"invalid counter {label}.{key}")
        total[key] += value


def sum_work(rows: list[dict[str, Any]], field: str, work_keys: tuple[str, ...]) -> dict[str, int]:
    total = {key: 0 for key in work_keys}
    for index, row in enumerate(rows):
        add_work(total, row.get(field), f"rows[{index}].{field}")
    return total


def recompose_work(prefix: dict[str, Any], continuation: dict[str, Any]) -> dict[str, int]:
    require(prefix.keys() == continuation.keys(), "prefix/continuation WorkCounters shape mismatch")
    return {key: prefix[key] + continuation[key] for key in prefix}


def expected_recommendation(row: dict[str, Any]) -> bool:
    zeta = row.get("quadratic_drift_zeta34")
    return bool(
        row.get("prefix_succeeded")
        and not row.get("budget_exhausted")
        and not row.get("budget_breached")
        and isinstance(zeta, (int, float))
        and not isinstance(zeta, bool)
        and math.isfinite(zeta)
        and zeta <= FROZEN_TAU
    )


def v35_path(root: Path, profile: str, family: str) -> Path:
    if profile == "holdout320":
        return root / "fresh_holdout320" / f"{family}.json"
    return root / "consumed_replay" / profile / f"{family}.json"


def verify_report(
    report: dict[str, Any],
    v35: dict[str, Any],
    profile: str,
    family: str,
    work_keys: tuple[str, ...],
) -> tuple[list[dict[str, Any]], int]:
    label = f"{profile}/{family}"
    require(report.get("schema") == "g4-s5b0-frozen-full-e-shadow-v1", f"schema: {label}")
    require(report.get("status") == "complete", f"status: {label}")
    require(report.get("profile") == PROFILES[profile], f"profile label: {label}")
    require(report.get("switching_active") is False, f"switching active: {label}")
    require(report.get("persistence_k") == 3, f"k changed: {label}")
    require(report.get("absolute_prefix_jvp_cap") == 80, f"B_abs changed: {label}")
    require_exact(
        report.get("frozen_cumulative_prefix_budget_fraction"), 0.25, f"delta: {label}"
    )
    require_exact(report.get("frozen_zeta34_tau"), FROZEN_TAU, f"tau: {label}")
    require(
        all(report.get("hard_gates", {}).get(key) is True for key in HARD_GATE_FIELDS),
        f"hard gate failed: {label}",
    )
    require(
        all(report.get("rjf_parity", {}).get(key) is True for key in PARITY_FIELDS),
        f"R-JF parity failed: {label}",
    )
    require(v35.get("schema") == "g4-s5b0-enforced-prefix-budget-v1", f"v3.5 schema: {label}")
    require(v35.get("status") == "complete", f"v3.5 status: {label}")
    require(v35.get("switching_active") is False, f"v3.5 switching active: {label}")
    require(v35.get("runtime_full_e_continuations") == 0, f"v3.5 runtime full-E: {label}")
    require_exact(
        rows_without_wall(v35.get("attempt_rows", []), "wall_seconds"),
        rows_without_wall(report.get("attempt_rows", []), "wall_seconds"),
        f"durable v3.5/v3.6 R-JF attempts: {label}",
    )
    require_exact(
        rows_without_wall(v35.get("accepted_rows", []), "rodas_wall_seconds"),
        rows_without_wall(report.get("accepted_rows", []), "rodas_wall_seconds"),
        f"durable v3.5/v3.6 R-JF accepted rows: {label}",
    )
    require_exact(
        v35.get("trajectories"), report.get("trajectories"), f"durable v3.5/v3.6 trajectories: {label}"
    )

    rows = report.get("rows")
    old_rows = v35.get("rows")
    require(isinstance(rows, list) and isinstance(old_rows, list), f"rows missing: {label}")
    require(len(rows) == len(old_rows), f"v3.5/v3.6 row count mismatch: {label}")
    runtime_by_attempt = {row["target_attempt_index"]: row for row in rows}
    old_by_attempt = {row["target_attempt_index"]: row for row in old_rows}
    require(len(runtime_by_attempt) == len(rows), f"duplicate runtime target: {label}")
    require(len(old_by_attempt) == len(old_rows), f"duplicate v3.5 target: {label}")
    require(runtime_by_attempt.keys() == old_by_attempt.keys(), f"target set mismatch: {label}")

    attempt_rows = report.get("attempt_rows")
    require(isinstance(attempt_rows, list), f"attempt trace missing: {label}")
    attempt_by_key = {
        (row["trajectory_id"], row["attempt_index"]): row for row in attempt_rows
    }
    require(len(attempt_by_key) == len(attempt_rows), f"duplicate attempt trace row: {label}")

    recommended: list[dict[str, Any]] = []
    expected_prefix_before = 0
    expected_total_before = 0
    for attempt_index in sorted(runtime_by_attempt):
        row = runtime_by_attempt[attempt_index]
        old = old_by_attempt[attempt_index]
        for field in PREFIX_FIELDS:
            require_exact(old.get(field), row.get(field), f"{label}/target-{attempt_index}/{field}")
        require_exact(
            old.get("speculative_jvp_before_target"),
            row.get("prefix_speculative_jvp_before_target"),
            f"{label}/target-{attempt_index}/prefix-ledger-before",
        )
        expected = expected_recommendation(row)
        require(row.get("recommended") is expected, f"recommendation mismatch: {label}/{attempt_index}")
        require(
            row.get("retained_level2_resumed") is expected,
            f"resume cardinality mismatch: {label}/{attempt_index}",
        )
        require(
            row["prefix_speculative_jvp_before_target"] == expected_prefix_before,
            f"prefix causal continuity mismatch: {label}/{attempt_index}",
        )
        require(
            row["total_speculative_jvp_before_target"] == expected_total_before,
            f"total causal continuity mismatch: {label}/{attempt_index}",
        )
        prefix = row.get("prefix_work")
        continuation = row.get("continuation_work")
        full = row.get("shadow_full_e_work")
        prefix_jvp = 0 if prefix is None else prefix["jvp_vectors"]
        continuation_jvp = 0 if continuation is None else continuation["jvp_vectors"]
        require(
            row["prefix_speculative_jvp_after_target"]
            == row["prefix_speculative_jvp_before_target"] + prefix_jvp,
            f"prefix ledger mismatch: {label}/{attempt_index}",
        )
        require(
            row["total_speculative_jvp_after_target"]
            == row["total_speculative_jvp_before_target"] + prefix_jvp + continuation_jvp,
            f"total ledger mismatch: {label}/{attempt_index}",
        )
        if expected:
            require(row.get("shadow_full_e_completed") is True, f"full-E incomplete: {label}/{attempt_index}")
            require(row.get("shadow_full_e_failure") is None, f"full-E failure: {label}/{attempt_index}")
            require(row.get("work_roundtrip_exact") is True, f"work round-trip: {label}/{attempt_index}")
            require(prefix is not None and continuation is not None and full is not None, f"ledger absent: {label}/{attempt_index}")
            require_exact(recompose_work(prefix, continuation), full, f"full work: {label}/{attempt_index}")
            recommended.append(row)
        else:
            require(continuation is None and full is None, f"unexpected continuation: {label}/{attempt_index}")

        target = attempt_by_key.get((row["trajectory_id"], row["target_attempt_index"]))
        require(target is not None, f"target R-JF attempt missing: {label}/{attempt_index}")
        require_exact(target["accepted"], row["target_r_attempt_accepted"], f"target accepted: {label}/{attempt_index}")
        require_exact(target["error_norm"], row["target_r_error_norm"], f"target error: {label}/{attempt_index}")
        require_exact(
            target["recoverable_failure"],
            row["target_r_recoverable_failure"],
            f"target recoverable failure: {label}/{attempt_index}",
        )
        require_exact(target["jvp_vectors"], row["target_rjf_jvp_vectors"], f"target JVP: {label}/{attempt_index}")
        expected_prefix_before = row["prefix_speculative_jvp_after_target"]
        expected_total_before = row["total_speculative_jvp_after_target"]

    prefix_total = sum_work(rows, "prefix_work", work_keys)
    continuation_total = sum_work(rows, "continuation_work", work_keys)
    full_total = recompose_work(prefix_total, continuation_total)
    require_exact(prefix_total, report.get("prefix_speculative_work"), f"prefix aggregate: {label}")
    require_exact(continuation_total, report.get("continuation_work"), f"continuation aggregate: {label}")
    require_exact(full_total, report.get("total_speculative_work"), f"total aggregate: {label}")
    require(
        expected_prefix_before == prefix_total["jvp_vectors"], f"terminal prefix ledger mismatch: {label}"
    )
    require(
        expected_total_before == full_total["jvp_vectors"], f"terminal total ledger mismatch: {label}"
    )
    committed_jvp = sum(row["jvp_vectors"] for row in attempt_rows)
    require(committed_jvp > 0, f"zero committed R-JF denominator: {label}")
    require(report.get("committed_rjf_jvp_vectors") == committed_jvp, f"R-JF aggregate: {label}")
    ratios = {
        "realized_prefix_over_committed_rjf_jvp": prefix_total["jvp_vectors"] / committed_jvp,
        "realized_continuation_over_committed_rjf_jvp": continuation_total["jvp_vectors"] / committed_jvp,
        "realized_total_speculative_over_committed_rjf_jvp": full_total["jvp_vectors"] / committed_jvp,
    }
    for field, expected_ratio in ratios.items():
        require_exact(report.get(field), expected_ratio, f"{field}: {label}")
    require(report.get("recommendations") == len(recommended), f"recommendation count: {label}")
    require(report.get("retained_level2_resumptions") == len(recommended), f"resume count: {label}")
    require(report.get("shadow_full_e_completions") == len(recommended), f"completion count: {label}")
    require(report.get("shadow_full_e_failures") == 0, f"continuation failure: {label}")
    require(report.get("unsafe_recommendations") == 0, f"unsafe recommendation: {label}")
    require(report.get("budget_breaches") == 0, f"budget breach: {label}")
    return recommended, committed_jvp


def verify_preflight(
    recommended: list[dict[str, Any]],
    preflight: dict[str, Any],
    work_keys: tuple[str, ...],
) -> dict[str, Any]:
    events = preflight.get("events")
    require(isinstance(events, list), "preflight events missing")
    runtime_by_key = {
        (f"N{row['dimension']}", row["family"], row["target_attempt_index"]): row
        for row in recommended
    }
    preflight_by_key = {
        (row["profile"], row["family"], row["target_attempt_index"]): row for row in events
    }
    require(len(runtime_by_key) == len(recommended), "duplicate runtime recommendation")
    require(len(preflight_by_key) == len(events), "duplicate preflight event")
    require(runtime_by_key.keys() == preflight_by_key.keys(), "preflight/runtime event set mismatch")
    for key in sorted(runtime_by_key):
        runtime = runtime_by_key[key]
        event = preflight_by_key[key]
        for preflight_field, runtime_field in PREFLIGHT_FIELD_MAP.items():
            runtime_value = runtime_field(runtime) if callable(runtime_field) else runtime.get(runtime_field)
            require_exact(event.get(preflight_field), runtime_value, f"preflight/{key}/{preflight_field}")

    prefix = sum_work(recommended, "prefix_work", work_keys)
    continuation = sum_work(recommended, "continuation_work", work_keys)
    full = sum_work(recommended, "shadow_full_e_work", work_keys)
    target_jvp = sum(row["target_rjf_jvp_vectors"] for row in recommended)
    overall = preflight.get("overall", {})
    require_exact(prefix, overall.get("prefix_work"), "preflight aggregate prefix work")
    require_exact(continuation, overall.get("continuation_work"), "preflight aggregate continuation work")
    require_exact(full, overall.get("full_e_work"), "preflight aggregate full-E work")
    require(target_jvp == overall.get("target_rjf_jvp_vectors"), "preflight target-R aggregate")
    require(len(recommended) == overall.get("recommendations"), "preflight recommendation count")
    require(max(row["continuation_work"]["jvp_vectors"] for row in recommended) == 140, "maximum continuation JVP oracle")
    require(prefix["jvp_vectors"] == 1456, "prefix JVP oracle")
    require(continuation["jvp_vectors"] == 1130, "continuation JVP oracle")
    require(full["jvp_vectors"] == 2586, "full-E JVP oracle")
    require(target_jvp == 13043, "target-R JVP oracle")
    require_exact(
        continuation["jvp_vectors"] / target_jvp,
        overall.get("cumulative_continuation_over_target_rjf_jvp"),
        "preflight continuation/target-R ratio",
    )
    require_exact(
        full["jvp_vectors"] / target_jvp,
        overall.get("cumulative_full_e_over_target_rjf_jvp"),
        "preflight full-E/target-R ratio",
    )
    return {
        "recommendations": len(recommended),
        "prefix_work": prefix,
        "continuation_work": continuation,
        "full_e_work": full,
        "target_rjf_jvp_vectors": target_jvp,
        "maximum_continuation_jvp_vectors": 140,
        "continuation_over_target_rjf_jvp": continuation["jvp_vectors"] / target_jvp,
        "full_e_over_target_rjf_jvp": full["jvp_vectors"] / target_jvp,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--preflight", type=Path, required=True)
    parser.add_argument("--v35-root", type=Path, required=True)
    parser.add_argument("--runtime-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    preflight = load_json(args.preflight)
    require(
        preflight.get("schema") == "vigilode-v36-full-e-ledger-preflight-v1",
        "preflight schema mismatch",
    )
    require(preflight.get("verdict") == "PASS_TO_RUNTIME_SHADOW_MEASUREMENT", "preflight verdict")
    work = preflight.get("overall", {}).get("prefix_work")
    require(isinstance(work, dict) and work, "preflight WorkCounters schema missing")
    work_keys = tuple(sorted(work))
    expected_runtime = {
        Path(profile) / f"{family}.json" for profile in PROFILES for family in FAMILIES
    }
    found_runtime = {
        path.relative_to(args.runtime_root) for path in args.runtime_root.rglob("*.json")
    }
    require(found_runtime == expected_runtime, "runtime shard set is not exactly 5 profiles x 6 families")

    recommended: list[dict[str, Any]] = []
    prefix_row_count = 0
    committed_rjf_jvp = 0
    runtime_hashes: dict[str, str] = {}
    v35_hashes: dict[str, str] = {}
    for profile in PROFILES:
        for family in FAMILIES:
            runtime_path = args.runtime_root / profile / f"{family}.json"
            old_path = v35_path(args.v35_root, profile, family)
            report = load_json(runtime_path)
            old = load_json(old_path)
            shard_recommended, shard_committed_jvp = verify_report(
                report, old, profile, family, work_keys
            )
            recommended.extend(shard_recommended)
            prefix_row_count += len(report["rows"])
            committed_rjf_jvp += shard_committed_jvp
            runtime_hashes[str(runtime_path.relative_to(args.runtime_root))] = sha256(runtime_path)
            v35_hashes[str(old_path.relative_to(args.v35_root))] = sha256(old_path)

    require(prefix_row_count == 127, "full prefix-policy row oracle")
    require(committed_rjf_jvp == 388999, "committed R-JF JVP oracle")
    preflight_summary = verify_preflight(recommended, preflight, work_keys)
    all_prefix_work = {key: 0 for key in work_keys}
    all_continuation_work = {key: 0 for key in work_keys}
    for profile in PROFILES:
        for family in FAMILIES:
            report = load_json(args.runtime_root / profile / f"{family}.json")
            add_work(all_prefix_work, report["prefix_speculative_work"], f"{profile}/{family}/prefix")
            add_work(all_continuation_work, report["continuation_work"], f"{profile}/{family}/continuation")
    all_total_work = recompose_work(all_prefix_work, all_continuation_work)
    require(all_prefix_work["jvp_vectors"] == 2669, "all-event prefix JVP oracle")
    require(all_continuation_work["jvp_vectors"] == 1130, "all-event continuation JVP oracle")
    require(all_total_work["jvp_vectors"] == 3799, "all-event total JVP oracle")

    output = {
        "schema": "vigilode-v36-runtime-shadow-verification-v1",
        "verdict": "PASS",
        "runtime_shards": 30,
        "prefix_policy_rows_exact_v35_to_v36": prefix_row_count,
        "recommendation_rows_exact_preflight_to_runtime": len(recommended),
        "frozen_policy": {
            "persistence_k": 3,
            "absolute_prefix_jvp_cap": 80,
            "cumulative_prefix_budget_fraction": 0.25,
            "zeta34_tau": FROZEN_TAU,
        },
        "all_event_ledger": {
            "committed_rjf_jvp_vectors": committed_rjf_jvp,
            "prefix_work": all_prefix_work,
            "continuation_work": all_continuation_work,
            "total_speculative_work": all_total_work,
            "realized_prefix_over_committed_rjf_jvp": all_prefix_work["jvp_vectors"]
            / committed_rjf_jvp,
            "realized_continuation_over_committed_rjf_jvp": all_continuation_work[
                "jvp_vectors"
            ]
            / committed_rjf_jvp,
            "realized_total_speculative_over_committed_rjf_jvp": all_total_work["jvp_vectors"]
            / committed_rjf_jvp,
        },
        "recommended_event_ledger": preflight_summary,
        "hard_evidence": {
            "all_runtime_reports_complete": True,
            "all_runtime_hard_gates_passed": True,
            "all_rjf_traces_exact_excluding_wall": True,
            "zero_budget_breaches": True,
            "zero_continuation_failures": True,
            "zero_unsafe_recommendations": True,
            "active_switching": False,
        },
        "input_sha256": {
            "preflight": sha256(args.preflight),
            "runtime": runtime_hashes,
            "v35_prefix_policy": v35_hashes,
        },
        "limitations": [
            "The five profiles are consumed descriptive evidence, not a fresh safety holdout.",
            "Exact R-JF parity excludes wall-clock fields and is not promoted to un-emitted state/output parity.",
            "No active switching or speedup claim is authorized; N=2048 remains sealed.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("RUNTIME_SHADOW_VERIFICATION_PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VerificationError as error:
        print(f"RUNTIME_SHADOW_VERIFICATION_FAIL: {error}")
        raise SystemExit(1) from error
