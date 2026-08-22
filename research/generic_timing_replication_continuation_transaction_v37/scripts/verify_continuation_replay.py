#!/usr/bin/env python3
"""Fail-closed verifier for the VigilODE v3.7 bounded continuation replay."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path
from typing import Any


EXPECTED_CONTRACT_SHA256 = "66f082aeec8c70e0ef23926d2c6f7057fb40fe280c45fd02c200be8778a6e659"
FROZEN_TAU = 13.39706618860016
PREFIX_JVP_CAP = 80
CONTINUATION_JVP_CAP = 80
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
EXPECTED_EXHAUSTED_KEYS = {
    ("N192", "semilinear-advection-diffusion-ramped", 12),
    ("N384", "semilinear-advection-diffusion-ramped", 23),
}
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
    "target_r_recoverable_failure",
    "committed_rjf_jvp_before_target",
    "prefix_speculative_jvp_before_target",
    "prefix_speculative_jvp_after_target",
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
    "frozen_zeta34_tau",
    "recommended",
    "retained_level2_resumed",
    "target_rjf_jvp_vectors",
    "prefix_over_target_rjf_jvp",
)
HARD_GATE_FIELDS = (
    "all_rjf_trajectories_successful",
    "rjf_trace_exact_excluding_wall",
    "zero_prefix_budget_breaches",
    "prefix_transactions_resolved",
    "continuation_transactions_resolved",
    "zero_continuation_budget_breaches",
    "zero_continuation_numerical_failures",
    "zero_unsafe_recommendations",
    "exhausted_rows_emit_no_endpoint_or_labels",
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
IMPLICIT_EXPENSIVE_FIELDS = (
    "jacobian_builds",
    "direct_factorizations",
    "nonlinear_solves",
    "nonlinear_iterations",
    "nonlinear_residual_evaluations",
    "nonlinear_jacobian_evaluations",
)


class VerificationError(RuntimeError):
    """The replay violates a sealed contract or durable parity invariant."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)


def float_bits(value: float) -> bytes:
    return struct.pack(">d", value)


def exact_equal(left: Any, right: Any) -> bool:
    if isinstance(left, float) or isinstance(right, float):
        return (
            isinstance(left, (int, float))
            and not isinstance(left, bool)
            and isinstance(right, (int, float))
            and not isinstance(right, bool)
            and float_bits(float(left)) == float_bits(float(right))
        )
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(
            exact_equal(left[key], right[key]) for key in left
        )
    if isinstance(left, list):
        return len(left) == len(right) and all(
            exact_equal(a, b) for a, b in zip(left, right)
        )
    return left == right


def require_exact(left: Any, right: Any, label: str) -> None:
    require(exact_equal(left, right), f"exact mismatch at {label}: {left!r} != {right!r}")


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
    try:
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(block)
    except OSError as error:
        raise VerificationError(f"cannot hash {path}: {error}") from error
    return digest.hexdigest()


def validate_work(work: dict[str, Any], label: str) -> None:
    require(isinstance(work, dict) and work, f"WorkCounters object required: {label}")
    for key, value in work.items():
        require(isinstance(key, str), f"invalid WorkCounters key: {label}")
        require(
            isinstance(value, int) and not isinstance(value, bool) and value >= 0,
            f"invalid WorkCounters value: {label}.{key}",
        )


def recompose_work(prefix: dict[str, int], continuation: dict[str, int]) -> dict[str, int]:
    require(prefix.keys() == continuation.keys(), "prefix/continuation WorkCounters shape mismatch")
    validate_work(prefix, "prefix")
    validate_work(continuation, "continuation")
    return {key: prefix[key] + continuation[key] for key in prefix}


def add_work(total: dict[str, int], work: dict[str, Any] | None, label: str) -> None:
    if work is None:
        return
    require(work.keys() == total.keys(), f"WorkCounters shape mismatch at {label}")
    validate_work(work, label)
    for key, value in work.items():
        total[key] += value


def rows_without_wall(rows: list[dict[str, Any]], wall_fields: set[str]) -> list[dict[str, Any]]:
    return [{key: value for key, value in row.items() if key not in wall_fields} for row in rows]


def expected_recommendation(row: dict[str, Any]) -> bool:
    zeta = row.get("quadratic_drift_zeta34")
    return bool(
        row.get("prefix_succeeded")
        and not row.get("budget_exhausted")
        and not row.get("budget_breached")
        and isinstance(zeta, (int, float))
        and not isinstance(zeta, bool)
        and math.isfinite(float(zeta))
        and float(zeta) <= FROZEN_TAU
    )


def work_is_implicit_expensive_free(work: dict[str, Any]) -> bool:
    return all(work.get(field, 0) == 0 for field in IMPLICIT_EXPENSIVE_FIELDS)


def validate_committed_method(
    report: dict[str, Any], baseline: dict[str, Any], label: str
) -> None:
    expected = baseline.get("committed_method")
    require(
        expected == "protected-sequential-matrix-free-rodas5p",
        f"unexpected v3.6 committed-method schema label: {label}",
    )
    require_exact(report.get("committed_method"), expected, f"committed method: {label}")


def validate_committed_method(
    report: dict[str, Any], baseline: dict[str, Any], label: str
) -> None:
    expected = "protected-sequential-matrix-free-rodas5p"
    require(
        baseline.get("committed_method") == expected,
        f"v3.6 committed method label drift: {label}",
    )
    require_exact(
        report.get("committed_method"),
        baseline.get("committed_method"),
        f"committed method v3.6/v3.7: {label}",
    )


def validate_continuation_row(row: dict[str, Any], label: str) -> None:
    outcome = row.get("continuation_outcome")
    recommended = row.get("recommended") is True
    resumed = row.get("retained_level2_resumed") is True
    exhausted = row.get("continuation_budget_exhausted") is True
    completed = row.get("shadow_full_e_completed") is True
    failure = row.get("shadow_full_e_failure")
    used = row.get("continuation_used_jvp_vectors")
    continuation = row.get("continuation_work")
    full = row.get("shadow_full_e_work")
    cap = row.get("continuation_jvp_cap")

    require(cap == CONTINUATION_JVP_CAP, f"continuation cap drift: {label}")
    if outcome == "not-recommended":
        require(not recommended and not resumed and not exhausted and not completed, f"invalid abstention flags: {label}")
        require(used is None and continuation is None and full is None, f"abstention emitted work: {label}")
        require(row.get("shadow_full_e_total_error") is None, f"abstention emitted endpoint error: {label}")
        require(row.get("shadow_full_e_locally_admissible") is None, f"abstention emitted label: {label}")
        require(failure is None, f"abstention emitted failure: {label}")
        require(row.get("work_roundtrip_exact") is False, f"abstention claimed work roundtrip: {label}")
        return

    require(recommended and resumed, f"continuation outcome without frozen recommendation: {label}")
    require(isinstance(used, int) and not isinstance(used, bool), f"missing continuation usage: {label}")
    require(0 <= used <= CONTINUATION_JVP_CAP, f"continuation usage breach: {label}")
    require(isinstance(continuation, dict), f"continuation work missing: {label}")
    require(isinstance(full, dict), f"full shadow work missing: {label}")
    validate_work(continuation, f"{label}.continuation_work")
    validate_work(full, f"{label}.shadow_full_e_work")
    require(continuation.get("jvp_vectors") == used, f"continuation work/usage mismatch: {label}")
    require(row.get("work_roundtrip_exact") is True, f"work roundtrip false: {label}")
    require(work_is_implicit_expensive_free(continuation), f"implicit expensive continuation work: {label}")
    require(work_is_implicit_expensive_free(full), f"implicit expensive full-E work: {label}")

    if outcome == "complete":
        require(not exhausted and completed, f"invalid complete flags: {label}")
        error = row.get("shadow_full_e_total_error")
        require(
            isinstance(error, (int, float))
            and not isinstance(error, bool)
            and math.isfinite(float(error)),
            f"complete endpoint error missing/non-finite: {label}",
        )
        require(row.get("shadow_full_e_locally_admissible") is True, f"complete admissibility false: {label}")
        require(failure is None, f"complete row emitted failure: {label}")
    elif outcome == "budget-exhausted":
        require(exhausted and not completed, f"invalid exhaustion flags: {label}")
        require(used == CONTINUATION_JVP_CAP, f"exhaustion did not charge exact cap: {label}")
        require(row.get("shadow_full_e_total_error") is None, f"exhaustion emitted endpoint: {label}")
        require(row.get("shadow_full_e_locally_admissible") is None, f"exhaustion emitted label: {label}")
        require(failure is None, f"exhaustion misclassified as numerical failure: {label}")
    elif outcome == "failed":
        require(not exhausted and not completed, f"invalid hard-failure flags: {label}")
        require(row.get("shadow_full_e_total_error") is None, f"failure emitted endpoint: {label}")
        require(row.get("shadow_full_e_locally_admissible") is None, f"failure emitted label: {label}")
        require(isinstance(failure, str) and failure, f"hard failure label missing: {label}")
    else:
        raise VerificationError(f"unknown continuation outcome {outcome!r}: {label}")


def verify_contract(contract_path: Path) -> dict[str, Any]:
    actual_hash = sha256(contract_path)
    require(actual_hash == EXPECTED_CONTRACT_SHA256, f"sealed contract hash mismatch: {actual_hash}")
    contract = load_json(contract_path)
    require(
        contract.get("schema") == "vigilode-v37-timing-replication-continuation-transaction-contract-v1",
        "contract schema mismatch",
    )
    require(contract.get("pre_output_contract") is True, "contract is not pre-output authority")
    require(contract.get("pre_runtime_shadow_mutation_contract") is True, "contract is not pre-mutation authority")
    frozen = contract.get("frozen_policy", {})
    require(
        frozen.get("committed_trajectory") == "protected-sequential-matrix-free-rjf",
        "contract committed-trajectory drift",
    )
    require(frozen.get("persistence_k") == 3, "contract k drift")
    require(frozen.get("absolute_prefix_jvp_cap") == PREFIX_JVP_CAP, "contract prefix cap drift")
    require_exact(frozen.get("cumulative_prefix_budget_fraction"), 0.25, "contract delta")
    require_exact(frozen.get("zeta34_tau"), FROZEN_TAU, "contract tau")
    require(frozen.get("active_switching") is False, "contract activated switching")
    require(frozen.get("n2048_sealed") is True, "contract unsealed N=2048")
    continuation = contract.get("continuation_transaction", {})
    require(continuation.get("absolute_jvp_cap") == CONTINUATION_JVP_CAP, "contract continuation cap drift")
    require(continuation.get("recommendation_rule_unchanged") is True, "contract recommendation rule drift")
    require(continuation.get("prefix_recomputation_allowed") is False, "contract allows prefix recomputation")
    require(continuation.get("rjf_mutation_allowed") is False, "contract allows R-JF mutation")
    oracle = contract.get("consumed_replay_oracle", {})
    require(oracle.get("recommendations") == 64, "contract recommendation oracle drift")
    require(oracle.get("predicted_completions") == 62, "contract completion oracle drift")
    require(oracle.get("predicted_budget_exhaustions") == 2, "contract exhaustion oracle drift")
    return contract


def verify_report(
    report: dict[str, Any],
    baseline: dict[str, Any],
    profile_key: str,
    family_key: str,
    work_keys: tuple[str, ...],
) -> dict[str, Any]:
    label = f"{profile_key}/{family_key}"
    require(report.get("schema") == "g4-s5b0-v37-continuation-transaction-v1", f"schema: {label}")
    require(report.get("status") == "complete", f"status: {label}")
    require(report.get("profile") == PROFILES[profile_key], f"profile: {label}")
    require(report.get("switching_active") is False, f"switching active: {label}")
    validate_committed_method(report, baseline, label)
    require(report.get("persistence_k") == 3, f"k drift: {label}")
    require(report.get("absolute_prefix_jvp_cap") == PREFIX_JVP_CAP, f"prefix cap drift: {label}")
    require(report.get("absolute_continuation_jvp_cap") == CONTINUATION_JVP_CAP, f"continuation cap drift: {label}")
    require_exact(report.get("frozen_cumulative_prefix_budget_fraction"), 0.25, f"delta: {label}")
    require_exact(report.get("frozen_zeta34_tau"), FROZEN_TAU, f"tau: {label}")
    hard = report.get("hard_gates", {})
    require(all(hard.get(field) is True for field in HARD_GATE_FIELDS), f"hard gate failed: {label}")
    parity = report.get("rjf_parity", {})
    require(all(parity.get(field) is True for field in PARITY_FIELDS), f"R-JF parity failed: {label}")
    require(report.get("prefix_budget_breaches") == 0, f"prefix breach: {label}")
    require(report.get("continuation_budget_breaches") == 0, f"continuation breach: {label}")
    require(report.get("unsafe_recommendations") == 0, f"unsafe recommendation: {label}")
    require(report.get("shadow_full_e_failures") == 0, f"numerical continuation failure: {label}")

    require(baseline.get("schema") == "g4-s5b0-frozen-full-e-shadow-v1", f"v3.6 schema: {label}")
    require(baseline.get("status") == "complete", f"v3.6 status: {label}")
    require_exact(
        rows_without_wall(baseline.get("attempt_rows", []), {"wall_seconds"}),
        rows_without_wall(report.get("attempt_rows", []), {"wall_seconds"}),
        f"R-JF attempts v3.6/v3.7: {label}",
    )
    require_exact(
        rows_without_wall(baseline.get("accepted_rows", []), {"rodas_wall_seconds"}),
        rows_without_wall(report.get("accepted_rows", []), {"rodas_wall_seconds"}),
        f"R-JF accepted v3.6/v3.7: {label}",
    )
    require_exact(baseline.get("trajectories"), report.get("trajectories"), f"trajectories: {label}")

    rows = report.get("rows")
    old_rows = baseline.get("rows")
    require(isinstance(rows, list) and isinstance(old_rows, list), f"rows missing: {label}")
    require(len(rows) == len(old_rows), f"row count drift: {label}")
    runtime_by_attempt = {row.get("target_attempt_index"): row for row in rows}
    old_by_attempt = {row.get("target_attempt_index"): row for row in old_rows}
    require(len(runtime_by_attempt) == len(rows), f"duplicate v3.7 target: {label}")
    require(len(old_by_attempt) == len(old_rows), f"duplicate v3.6 target: {label}")
    require(runtime_by_attempt.keys() == old_by_attempt.keys(), f"target set drift: {label}")

    recommendations = completions = exhaustions = failures = 0
    exhausted_keys: set[tuple[str, str, int]] = set()
    completed_rows: list[dict[str, Any]] = []
    recommended_rows: list[dict[str, Any]] = []
    expected_prefix_before = 0
    expected_total_before = 0
    for attempt in sorted(runtime_by_attempt):
        row = runtime_by_attempt[attempt]
        old = old_by_attempt[attempt]
        row_label = f"{label}/attempt-{attempt}"
        for field in PREFIX_FIELDS:
            require_exact(row.get(field), old.get(field), f"{row_label}/{field}")
        expected = expected_recommendation(row)
        require(row.get("recommended") is expected, f"frozen recommendation recomputation mismatch: {row_label}")
        require_exact(row.get("recommended"), old.get("recommended"), f"recommendation parity: {row_label}")
        validate_continuation_row(row, row_label)
        require(
            row.get("prefix_speculative_jvp_before_target") == expected_prefix_before,
            f"prefix causal continuity: {row_label}",
        )
        require(
            row.get("total_speculative_jvp_before_target") == expected_total_before,
            f"total causal continuity: {row_label}",
        )
        prefix_jvp = 0 if row.get("prefix_work") is None else row["prefix_work"]["jvp_vectors"]
        continuation_jvp = (
            0 if row.get("continuation_work") is None else row["continuation_work"]["jvp_vectors"]
        )
        require(
            row.get("prefix_speculative_jvp_after_target")
            == expected_prefix_before + prefix_jvp,
            f"prefix causal update: {row_label}",
        )
        require(
            row.get("total_speculative_jvp_after_target")
            == expected_total_before + prefix_jvp + continuation_jvp,
            f"total causal update: {row_label}",
        )
        expected_prefix_before = row["prefix_speculative_jvp_after_target"]
        expected_total_before = row["total_speculative_jvp_after_target"]

        if row.get("prefix_work") is not None:
            validate_work(row["prefix_work"], f"{row_label}.prefix_work")
            require(row["prefix_work"].keys() == set(work_keys) or tuple(row["prefix_work"].keys()) == work_keys, f"prefix work shape: {row_label}")
        if row.get("recommended"):
            recommendations += 1
            recommended_rows.append(row)
            prefix = row.get("prefix_work")
            continuation = row.get("continuation_work")
            full = row.get("shadow_full_e_work")
            require(isinstance(prefix, dict) and isinstance(continuation, dict) and isinstance(full, dict), f"recommended work missing: {row_label}")
            require_exact(recompose_work(prefix, continuation), full, f"work recomposition: {row_label}")
            outcome = row.get("continuation_outcome")
            if outcome == "complete":
                completions += 1
                completed_rows.append(row)
                for field in (
                    "shadow_full_e_completed",
                    "shadow_full_e_total_error",
                    "shadow_full_e_locally_admissible",
                    "shadow_full_e_failure",
                    "continuation_work",
                    "shadow_full_e_work",
                    "work_roundtrip_exact",
                    "target_rjf_jvp_vectors",
                    "prefix_over_target_rjf_jvp",
                    "continuation_over_target_rjf_jvp",
                    "full_e_over_target_rjf_jvp",
                ):
                    require_exact(row.get(field), old.get(field), f"completed v3.6/v3.7 parity: {row_label}/{field}")
            elif outcome == "budget-exhausted":
                exhaustions += 1
                key = (f"N{row['dimension']}", row["family"], row["target_attempt_index"])
                exhausted_keys.add(key)
                require(old.get("shadow_full_e_completed") is True, f"v3.6 oracle did not complete: {row_label}")
                require(old.get("continuation_work", {}).get("jvp_vectors") == 140, f"v3.6 outlier is not 140 JVP: {row_label}")
            elif outcome == "failed":
                failures += 1

    prefix_total = {key: 0 for key in work_keys}
    continuation_total = {key: 0 for key in work_keys}
    for index, row in enumerate(rows):
        add_work(prefix_total, row.get("prefix_work"), f"{label}/row-{index}/prefix")
        add_work(continuation_total, row.get("continuation_work"), f"{label}/row-{index}/continuation")
    total = recompose_work(prefix_total, continuation_total)
    require_exact(prefix_total, report.get("prefix_speculative_work"), f"prefix aggregate: {label}")
    require_exact(continuation_total, report.get("continuation_work"), f"continuation aggregate: {label}")
    require_exact(total, report.get("total_speculative_work"), f"total aggregate: {label}")
    require(
        expected_prefix_before == prefix_total["jvp_vectors"],
        f"terminal prefix causal ledger: {label}",
    )
    require(
        expected_total_before == total["jvp_vectors"],
        f"terminal total causal ledger: {label}",
    )

    committed_jvp = sum(row.get("jvp_vectors", 0) for row in report.get("attempt_rows", []))
    require(committed_jvp > 0, f"zero committed R-JF JVP: {label}")
    require(report.get("committed_rjf_jvp_vectors") == committed_jvp, f"committed R-JF aggregate: {label}")
    require_exact(report.get("realized_prefix_over_committed_rjf_jvp"), prefix_total["jvp_vectors"] / committed_jvp, f"prefix ratio: {label}")
    require_exact(report.get("realized_continuation_over_committed_rjf_jvp"), continuation_total["jvp_vectors"] / committed_jvp, f"continuation ratio: {label}")
    require_exact(report.get("realized_total_speculative_over_committed_rjf_jvp"), total["jvp_vectors"] / committed_jvp, f"total ratio: {label}")
    require(report.get("recommendations") == recommendations, f"recommendation count: {label}")
    require(report.get("retained_level2_resumptions") == recommendations, f"resume count: {label}")
    require(report.get("shadow_full_e_completions") == completions, f"completion count: {label}")
    require(report.get("continuation_budget_exhaustions") == exhaustions, f"exhaustion count: {label}")
    require(report.get("shadow_full_e_failures") == failures, f"failure count: {label}")
    return {
        "rows": len(rows),
        "recommendations": recommendations,
        "completions": completions,
        "exhaustions": exhaustions,
        "failures": failures,
        "exhausted_keys": exhausted_keys,
        "prefix_work": prefix_total,
        "continuation_work": continuation_total,
        "total_work": total,
        "committed_rjf_jvp": committed_jvp,
        "recommended_rows": recommended_rows,
        "completed_rows": completed_rows,
    }


def verify_runtime(
    contract_path: Path,
    v36_root: Path,
    runtime_root: Path,
    output_path: Path,
    binary_path: Path | None = None,
) -> dict[str, Any]:
    verify_contract(contract_path)
    expected_paths = {Path(profile) / f"{family}.json" for profile in PROFILES for family in FAMILIES}
    found_paths = {path.relative_to(runtime_root) for path in runtime_root.rglob("*.json")}
    require(found_paths == expected_paths, "runtime shard set is not exactly 5 profiles x 6 families")

    first = load_json(runtime_root / "calibration96" / "robertson.json")
    work = first.get("prefix_speculative_work")
    require(isinstance(work, dict) and work, "runtime WorkCounters schema missing")
    work_keys = tuple(work.keys())
    all_prefix = {key: 0 for key in work_keys}
    all_continuation = {key: 0 for key in work_keys}
    recommended_prefix = {key: 0 for key in work_keys}
    recommended_continuation = {key: 0 for key in work_keys}
    target_rjf_jvp = 0
    total_rows = recommendations = completions = exhaustions = failures = committed_rjf_jvp = 0
    exhausted_keys: set[tuple[str, str, int]] = set()
    runtime_hashes: dict[str, str] = {}
    v36_hashes: dict[str, str] = {}

    for profile in PROFILES:
        for family in FAMILIES:
            relative = Path(profile) / f"{family}.json"
            runtime_path = runtime_root / relative
            baseline_path = v36_root / relative
            report = load_json(runtime_path)
            baseline = load_json(baseline_path)
            summary = verify_report(report, baseline, profile, family, work_keys)
            total_rows += summary["rows"]
            recommendations += summary["recommendations"]
            completions += summary["completions"]
            exhaustions += summary["exhaustions"]
            failures += summary["failures"]
            committed_rjf_jvp += summary["committed_rjf_jvp"]
            exhausted_keys.update(summary["exhausted_keys"])
            add_work(all_prefix, summary["prefix_work"], f"{relative}/prefix")
            add_work(all_continuation, summary["continuation_work"], f"{relative}/continuation")
            for row in summary["recommended_rows"]:
                add_work(recommended_prefix, row["prefix_work"], f"{relative}/recommended-prefix")
                add_work(recommended_continuation, row["continuation_work"], f"{relative}/recommended-continuation")
                target = row.get("target_rjf_jvp_vectors")
                require(isinstance(target, int) and target > 0, f"target R-JF work missing: {relative}")
                target_rjf_jvp += target
            runtime_hashes[str(relative)] = sha256(runtime_path)
            v36_hashes[str(relative)] = sha256(baseline_path)

    all_total = recompose_work(all_prefix, all_continuation)
    recommended_total = recompose_work(recommended_prefix, recommended_continuation)
    require(total_rows == 127, f"prefix row oracle: {total_rows}")
    require(recommendations == 64, f"recommendation oracle: {recommendations}")
    require(completions == 62, f"completion oracle: {completions}")
    require(exhaustions == 2, f"exhaustion oracle: {exhaustions}")
    require(failures == 0, f"numerical failure oracle: {failures}")
    require(exhausted_keys == EXPECTED_EXHAUSTED_KEYS, f"exhausted event set mismatch: {sorted(exhausted_keys)}")
    require(committed_rjf_jvp == 388999, f"committed R-JF JVP oracle: {committed_rjf_jvp}")
    require(all_prefix["jvp_vectors"] == 2669, f"all-event prefix JVP oracle: {all_prefix['jvp_vectors']}")
    require(all_continuation["jvp_vectors"] == 1010, f"all-event continuation JVP oracle: {all_continuation['jvp_vectors']}")
    require(all_total["jvp_vectors"] == 3679, f"all-event total JVP oracle: {all_total['jvp_vectors']}")
    require(recommended_prefix["jvp_vectors"] == 1456, f"recommended prefix JVP oracle: {recommended_prefix['jvp_vectors']}")
    require(recommended_continuation["jvp_vectors"] == 1010, f"recommended continuation JVP oracle: {recommended_continuation['jvp_vectors']}")
    require(recommended_total["jvp_vectors"] == 2466, f"recommended total JVP oracle: {recommended_total['jvp_vectors']}")
    require(target_rjf_jvp == 13043, f"target R-JF JVP oracle: {target_rjf_jvp}")

    input_hashes: dict[str, Any] = {
        "contract": sha256(contract_path),
        "runtime": runtime_hashes,
        "v36_runtime": v36_hashes,
    }
    if binary_path is not None:
        require(binary_path.is_file(), f"measurement binary missing: {binary_path}")
        input_hashes["binary"] = sha256(binary_path)

    result = {
        "schema": "vigilode-v37-continuation-replay-verification-v1",
        "verdict": "PASS",
        "runtime_shards": 30,
        "prefix_policy_rows_exact_v36_to_v37": total_rows,
        "recommendation_rows_exact_v36_to_v37": recommendations,
        "continuation_outcomes": {
            "recommendations": recommendations,
            "completions": completions,
            "budget_exhaustions": exhaustions,
            "numerical_failures": failures,
            "exhausted_keys": [
                {"profile": profile, "family": family, "target_attempt_index": attempt}
                for profile, family, attempt in sorted(exhausted_keys)
            ],
        },
        "frozen_policy": {
            "persistence_k": 3,
            "absolute_prefix_jvp_cap": PREFIX_JVP_CAP,
            "absolute_continuation_jvp_cap": CONTINUATION_JVP_CAP,
            "cumulative_prefix_budget_fraction": 0.25,
            "zeta34_tau": FROZEN_TAU,
            "active_switching": False,
        },
        "all_event_ledger": {
            "committed_rjf_jvp_vectors": committed_rjf_jvp,
            "prefix_work": all_prefix,
            "continuation_work": all_continuation,
            "total_speculative_work": all_total,
            "realized_prefix_over_committed_rjf_jvp": all_prefix["jvp_vectors"] / committed_rjf_jvp,
            "realized_continuation_over_committed_rjf_jvp": all_continuation["jvp_vectors"] / committed_rjf_jvp,
            "realized_total_speculative_over_committed_rjf_jvp": all_total["jvp_vectors"] / committed_rjf_jvp,
        },
        "recommended_event_ledger": {
            "target_rjf_jvp_vectors": target_rjf_jvp,
            "prefix_work": recommended_prefix,
            "continuation_work": recommended_continuation,
            "total_speculative_work": recommended_total,
            "continuation_over_target_rjf_jvp": recommended_continuation["jvp_vectors"] / target_rjf_jvp,
            "total_speculative_over_target_rjf_jvp": recommended_total["jvp_vectors"] / target_rjf_jvp,
        },
        "hard_evidence": {
            "all_runtime_reports_complete": True,
            "all_runtime_hard_gates_passed": True,
            "all_rjf_traces_exact_excluding_wall": True,
            "all_frozen_recommendations_exact": True,
            "all_62_completed_endpoints_exact_v36_to_v37_excluding_wall": True,
            "two_cap_exhaustions_charged_without_endpoint_or_label": True,
            "zero_prefix_budget_breaches": True,
            "zero_continuation_budget_breaches": True,
            "zero_continuation_numerical_failures": True,
            "zero_unsafe_recommendations": True,
            "zero_implicit_expensive_shadow_work": True,
            "active_switching": False,
            "n2048_executed": False,
        },
        "input_sha256": input_hashes,
        "limitations": [
            "This is a consumed semantic replay and not fresh shadow-safety evidence.",
            "No timing replication or speedup claim is authorized by this artifact.",
            "R-JF parity excludes wall fields; un-emitted state/controller/output parity is not inferred.",
            "Active switching remains disabled and N=2048 remains sealed.",
        ],
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--v36-root", type=Path, required=True)
    parser.add_argument("--runtime-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    verify_runtime(args.contract, args.v36_root, args.runtime_root, args.output, args.binary)
    print("V37_CONTINUATION_REPLAY_VERIFICATION_PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VerificationError as error:
        print(f"V37_CONTINUATION_REPLAY_VERIFICATION_FAIL: {error}")
        raise SystemExit(1) from error
