#!/usr/bin/env python3
"""Fail-closed validator for the frozen local Bateman six-case receipt.

This consumes a solver report only after the separately frozen authority
preflight.  It does not execute a candidate and does not relax a failed case.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import pathlib
import struct
from typing import Any

MANIFEST_SHA256 = "673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360"
VERIFIER_SHA256 = "542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d"
PROOF_SHA256 = "057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12"
REFERENCE_SOURCE = "bateman-taylor-lagrange-fraction-v1"
STATE_DOMAIN = b"VIGILODE\0AUDIT2\0BATEMAN_STATE\0"

WORK_COUNTER_FIELDS = (
    "rhs_calls",
    "rhs_batch_calls",
    "rhs_evaluations",
    "ft_calls",
    "jacobian_builds",
    "jvp_calls",
    "jvp_vectors",
    "mass_matvecs",
    "nonlinear_solves",
    "nonlinear_iterations",
    "nonlinear_residual_evaluations",
    "nonlinear_jacobian_evaluations",
    "nonlinear_failures",
    "linear_solves",
    "linear_iterations",
    "linear_matvecs",
    "preconditioner_apps",
    "direct_factorizations",
    "direct_solve_calls",
    "recycle_projection_calls",
    "recycle_same_operator_uses",
    "recycle_cross_operator_refreshes",
    "recycle_refresh_matvecs",
    "recycle_updates",
    "recycle_vectors_selected",
    "recycle_dropped_vectors",
    "harmonic_ritz_solves",
    "orthogonalization_inner_products",
    "orthogonalization_vector_updates",
    "diagnostic_matvecs",
    "phi_actions",
    "phi_krylov_vectors",
    "phi_projected_exponentials",
    "phi_restarts",
    "phi_dense_oracle_calls",
    "block_linear_solves",
    "block_linear_iterations",
    "block_matvecs",
    "block_preconditioner_apps",
    "fast_attempts",
    "fast_accepts",
    "fallback_steps",
    "accepted_steps",
    "rejected_steps",
    "local_error_failures",
    "linear_solve_failures",
    "nonlinear_solve_failures",
    "nonfinite_step_failures",
    "forced_stage_solves",
)

AUTHORITY_MANIFEST_PATH = pathlib.Path(__file__).with_name("authority_manifest.json")
_manifest_bytes = AUTHORITY_MANIFEST_PATH.read_bytes()
if hashlib.sha256(_manifest_bytes).hexdigest() != MANIFEST_SHA256:
    raise RuntimeError("local receipt validator authority manifest hash mismatch")
AUTHORITY_MANIFEST = json.loads(_manifest_bytes)
OPERATOR_CASES = {
    case["case_id"]: case for case in AUTHORITY_MANIFEST["operator_cases"]
}
INITIAL_STATE_BITS = tuple(AUTHORITY_MANIFEST["initial_state_bits"])

EXPECTED = (
    ("same-live-context-reuse", "nominal-h1e-3", "same-live-context-cache-probe", "cache-reuse-observed"),
    ("changed-w-invalidation", "changed-w-h5e-4", "changed-w-cache-probe", "changed-w-invalidation-observed"),
    ("nominal-independent-budget", "nominal-h1e-3", "transactional-nominal", "candidate"),
    ("over-strict-budget-fallback", "nominal-h1e-3", "transactional-strict-fallback", "protected-fallback"),
    ("late-preconditioner-failure", "nominal-h1e-3", "transactional-late-apply-failure", "protected-fallback"),
    ("terminal-rejection", "nominal-h1e-3", "transactional-terminal-rejection", "rejected"),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def lower_hex_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def require_uint(value: Any, field: str) -> int:
    require(
        type(value) is int and 0 <= value < 2**64,
        f"{field} must be a uint64 counter",
    )
    return value


def require_work_counters(value: Any, field: str) -> dict[str, int]:
    require(type(value) is dict, f"{field} work counters must be an object")
    expected = set(WORK_COUNTER_FIELDS)
    actual = set(value)
    require(actual == expected, f"{field} work counter schema mismatch")
    for key in WORK_COUNTER_FIELDS:
        require_uint(value[key], f"{field}.{key}")
    return value


def require_uint64_bits(value: Any, field: str) -> list[int]:
    require(type(value) is list and len(value) == 4, f"{field} must contain four state bits")
    for index, bits in enumerate(value):
        require(
            type(bits) is int and 0 <= bits < 2**64,
            f"{field}[{index}] is not uint64",
        )
        require(
            math.isfinite(struct.unpack(">d", struct.pack(">Q", bits))[0]),
            f"{field}[{index}] is nonfinite",
        )
    return value


def state_digest(scenario_id: str, bits: list[int]) -> str:
    scenario = scenario_id.encode()
    payload = bytearray(STATE_DOMAIN)
    payload.extend(struct.pack(">I", len(scenario)))
    payload.extend(scenario)
    payload.extend(struct.pack(">I", len(bits)))
    for value in bits:
        payload.extend(struct.pack(">Q", value))
    return hashlib.sha256(payload).hexdigest()


def f64_bits(value: float) -> int:
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def add_up_nonnegative(left: float, right: float) -> float:
    if left == 0.0:
        return right
    if right == 0.0:
        return left
    rounded = left + right
    return math.nextafter(rounded, math.inf) if math.isfinite(rounded) else rounded


def add_down_nonnegative(left: float, right: float) -> float:
    if left == 0.0:
        return right
    if right == 0.0:
        return left
    return max(0.0, math.nextafter(left + right, -math.inf))


def multiply_down_nonnegative(left: float, right: float) -> float:
    if left == 0.0 or right == 0.0:
        return 0.0
    return max(0.0, math.nextafter(left * right, -math.inf))


def conservative_l2_lower(values: list[float]) -> float:
    squared = 0.0
    for value in values:
        squared = add_down_nonnegative(
            squared, multiply_down_nonnegative(abs(value), abs(value))
        )
    if squared == 0.0:
        return 0.0
    return max(0.0, math.nextafter(math.sqrt(squared), -math.inf))


def expected_output_budget(case: dict[str, Any]) -> float:
    relative = multiply_down_nonnegative(
        case["budget"]["output_rtol"],
        conservative_l2_lower(case["reference"]["state"]),
    )
    return add_down_nonnegative(case["budget"]["output_atol_l2"], relative)


def multiply_up_nonnegative(left: float, right: float) -> float:
    if left == 0.0 or right == 0.0:
        return 0.0
    rounded = left * right
    return math.nextafter(rounded, math.inf) if math.isfinite(rounded) else rounded


def conservative_l2_difference_upper(
    actual_bits: list[int], expected: list[float]
) -> float:
    actual = [struct.unpack(">d", struct.pack(">Q", bits))[0] for bits in actual_bits]
    squared = 0.0
    for left, right in zip(actual, expected, strict=True):
        if left == right:
            difference = 0.0
        else:
            difference = math.nextafter(abs(left - right), math.inf)
        squared = add_up_nonnegative(
            squared, multiply_up_nonnegative(difference, difference)
        )
    if squared == 0.0:
        return 0.0
    return math.nextafter(math.sqrt(squared), math.inf)


def require_binding(binding: Any, case_id: str, field: str) -> None:
    require(type(binding) is dict, f"{field} committed binding missing")
    require(
        set(binding)
        == {
            "dimension",
            "h_gamma_bits",
            "frozen_w_semantic",
            "preconditioner",
            "returned_inverse_diagonal_bits",
        },
        f"{field} committed binding schema mismatch",
    )
    case = OPERATOR_CASES[case_id]
    gamma = struct.unpack(
        ">d", struct.pack(">Q", AUTHORITY_MANIFEST["coefficient_gamma_bits"])
    )[0]
    require(binding["dimension"] == 4, f"{field} binding dimension mismatch")
    require(
        binding["h_gamma_bits"] == f64_bits(case["h"] * gamma),
        f"{field} h-gamma identity mismatch",
    )
    require(
        binding["frozen_w_semantic"] == case["frozen_w_semantic"],
        f"{field} frozen-W identity mismatch",
    )
    require(
        binding["preconditioner"] == case["preconditioner_identity"],
        f"{field} preconditioner identity mismatch",
    )
    require(
        binding["returned_inverse_diagonal_bits"]
        == case["preconditioner_identity"]["expected_inverse_diagonal_bits"],
        f"{field} returned preconditioner map mismatch",
    )


def require_cache(
    cache: Any, field: str, committed_case_id: str | None
) -> dict[str, Any]:
    require(type(cache) is dict, f"{field} cache must be an object")
    require(
        set(cache)
        == {
            "attempts",
            "setup_attempts",
            "setup_completed",
            "setup_failures",
            "same_binding_reuses",
            "changed_operator_invalidations",
            "changed_preconditioner_invalidations",
            "commits",
            "rollbacks",
            "setup_work",
            "last_setup_failure",
            "committed_binding",
            "pending_binding",
        },
        f"{field} cache schema mismatch",
    )
    for key in (
        "attempts",
        "setup_attempts",
        "setup_completed",
        "setup_failures",
        "same_binding_reuses",
        "changed_operator_invalidations",
        "changed_preconditioner_invalidations",
        "commits",
        "rollbacks",
    ):
        require_uint(cache[key], f"{field}.{key}")
    require_work_counters(cache["setup_work"], f"{field}.setup_work")
    require(cache["last_setup_failure"] is None, f"{field} has a setup failure")
    require(cache.get("pending_binding") is None, f"{field} retains a pending cache lease")
    require(
        cache["setup_completed"] <= cache["setup_attempts"] <= cache["attempts"],
        f"{field} cache setup counters are nonmonotone",
    )
    require(
        cache["setup_failures"] <= cache["setup_attempts"],
        f"{field} cache failure counters are nonmonotone",
    )
    require(
        cache["commits"] + cache["rollbacks"] <= cache["attempts"],
        f"{field} cache disposition counters are nonmonotone",
    )
    if committed_case_id is None:
        require(cache["committed_binding"] is None, f"{field} unexpectedly committed a binding")
    else:
        require_binding(cache["committed_binding"], committed_case_id, field)
    return cache


def require_step(
    step: Any,
    *,
    accepted: bool,
    fallback: bool,
    field: str,
) -> dict[str, Any]:
    require(type(step) is dict, f"{field} missing")
    require(
        set(step)
        == {"method", "accepted", "used_fallback", "error_norm", "y_new_bits", "counters"},
        f"{field} step schema mismatch",
    )
    require(step.get("accepted") is accepted, f"{field}.accepted mismatch")
    require(step.get("used_fallback") is fallback, f"{field}.used_fallback mismatch")
    expected_method = (
        "RODAS5P-audit2-protected-sequential-JF-fallback"
        if fallback
        else "RODAS5P-audit2-reusable-preconditioner-candidate"
    )
    require(step["method"] == expected_method, f"{field}.method mismatch")
    require(
        type(step["error_norm"]) in (int, float)
        and not isinstance(step["error_norm"], bool)
        and math.isfinite(step["error_norm"])
        and step["error_norm"] >= 0.0,
        f"{field}.error_norm invalid",
    )
    require(
        step["accepted"] is (step["error_norm"] <= 1.0),
        f"{field}.accepted is inconsistent with error_norm",
    )
    require_uint64_bits(step["y_new_bits"], f"{field}.y_new_bits")
    require_work_counters(step["counters"], f"{field}.counters")
    return step


def require_counter_prefix(prefix: dict[str, int], total: dict[str, int], field: str) -> None:
    require(
        all(prefix[key] <= total[key] for key in WORK_COUNTER_FIELDS),
        f"{field} counters exceed the scenario work receipt",
    )


def require_budget(
    budget: Any,
    *,
    scenario_id: str,
    candidate_bits: list[int],
    nominal: bool,
    field: str,
) -> dict[str, Any]:
    require(type(budget) is dict, f"{field} budget missing")
    expected_fields = {
        "identifier",
        "reference_source",
        "output_error_l2",
        "output_budget_l2",
        "reference_uncertainty_l2",
        "output_error_upper_l2",
        "uncertainty_treatment",
        "embedded_l2",
        "original_target_residual_l2",
        "original_target_contraction",
        "output_accepted",
        "embedded_accepted",
        "original_target_accepted",
        "accepted",
    }
    require(set(budget) == expected_fields, f"{field} budget schema mismatch")
    case = OPERATOR_CASES["nominal-h1e-3"]
    expected_identifier = (
        case["budget"]["identifier"]
        if nominal
        else f"frozen-bateman-{scenario_id}-nominal-h1e-3-v1"
    )
    require(budget["identifier"] == expected_identifier, f"{field} budget identifier mismatch")
    require(budget["reference_source"] == REFERENCE_SOURCE, f"{field} reference source mismatch")
    for name in (
        "output_error_l2",
        "output_budget_l2",
        "reference_uncertainty_l2",
        "output_error_upper_l2",
        "embedded_l2",
        "original_target_residual_l2",
        "original_target_contraction",
    ):
        value = budget[name]
        require(
            type(value) in (int, float)
            and not isinstance(value, bool)
            and math.isfinite(value)
            and value >= 0.0,
            f"{field}.{name} invalid",
        )
    uncertainty = case["reference"]["uncertainty_l2"]
    require(
        f64_bits(budget["reference_uncertainty_l2"]) == f64_bits(uncertainty),
        f"{field} uncertainty mismatch",
    )
    require(
        budget["uncertainty_treatment"] == "DECLARED_UPPER_BOUND",
        f"{field} uncertainty treatment mismatch",
    )
    expected_error = conservative_l2_difference_upper(candidate_bits, case["reference"]["state"])
    require(
        f64_bits(budget["output_error_l2"]) == f64_bits(expected_error),
        f"{field} output error mismatch",
    )
    expected_upper = add_up_nonnegative(expected_error, uncertainty)
    require(
        f64_bits(budget["output_error_upper_l2"]) == f64_bits(expected_upper),
        f"{field} output error upper bound mismatch",
    )
    expected_budget = expected_output_budget(case) if nominal else 0.0
    require(
        f64_bits(budget["output_budget_l2"]) == f64_bits(expected_budget),
        f"{field} output budget mismatch",
    )
    limits = case["budget"] if nominal else {
        "max_embedded_l2": 0.0,
        "max_original_target_residual_l2": 0.0,
        "max_original_target_contraction": 0.0,
    }
    expected_output = expected_upper <= expected_budget
    expected_embedded = budget["embedded_l2"] <= limits["max_embedded_l2"]
    expected_original = (
        budget["original_target_residual_l2"]
        <= limits["max_original_target_residual_l2"]
        and budget["original_target_contraction"]
        <= limits["max_original_target_contraction"]
    )
    require(budget["output_accepted"] is expected_output, f"{field} output budget boolean mismatch")
    require(budget["embedded_accepted"] is expected_embedded, f"{field} embedded budget boolean mismatch")
    require(
        budget["original_target_accepted"] is expected_original,
        f"{field} original-target budget boolean mismatch",
    )
    require(
        budget["accepted"] is (expected_output and expected_embedded and expected_original),
        f"{field} budget aggregate mismatch",
    )
    return budget


def require_correction_work(value: Any, field: str) -> dict[str, Any]:
    require(type(value) is dict, f"{field} correction work missing")
    for key in (
        "correction_jvp_attempts",
        "correction_jvp_completed",
        "diagnostic_shifted_apply_attempts",
        "diagnostic_shifted_apply_completed",
        "diagnostic_jvp_attempts",
        "diagnostic_jvp_completed",
    ):
        require_uint(value.get(key), f"{field}.{key}")
    require_work_counters(value.get("preparation_counters"), f"{field}.preparation_counters")
    require_work_counters(value.get("coupling_counters"), f"{field}.coupling_counters")
    require(type(value.get("session")) is dict, f"{field}.session missing")
    require_work_counters(value["session"].get("counters"), f"{field}.session.counters")
    attempts = require_uint(
        value["session"].get("preconditioner_apply_attempts"),
        f"{field}.session.preconditioner_apply_attempts",
    )
    completed = require_uint(
        value["session"].get("preconditioner_apply_completed"),
        f"{field}.session.preconditioner_apply_completed",
    )
    require(completed <= attempts, f"{field} preconditioner apply ledger is nonmonotone")
    return value


def require_correction_success(value: Any, field: str) -> dict[str, Any]:
    require(type(value) is dict, f"{field} correction receipt missing")
    require(
        set(value)
        == {
            "projection",
            "projected_residual",
            "correction",
            "solve_reports",
            "initial_residual_l2",
            "linear_residual_l2",
            "work",
        },
        f"{field} correction receipt schema mismatch",
    )
    for key in ("projected_residual", "correction", "solve_reports"):
        require(type(value[key]) is list, f"{field}.{key} must be a list")
    require(type(value["projection"]) is dict, f"{field}.projection missing")
    for key in ("initial_residual_l2", "linear_residual_l2"):
        require(
            type(value[key]) in (int, float)
            and not isinstance(value[key], bool)
            and math.isfinite(value[key])
            and value[key] >= 0.0,
            f"{field}.{key} invalid",
        )
    require_correction_work(value["work"], f"{field}.work")
    return value


def require_correction_failure(value: Any, field: str) -> dict[str, Any]:
    require(type(value) is dict, f"{field} candidate failure missing")
    require(
        set(value)
        == {
            "phase",
            "message",
            "projection",
            "projected_residual",
            "partial_correction",
            "partial_reports",
            "work",
        },
        f"{field} candidate failure schema mismatch",
    )
    require(type(value["message"]) is str and value["message"], f"{field} failure message missing")
    require(type(value["partial_correction"]) is list, f"{field}.partial_correction missing")
    require(type(value["partial_reports"]) is list, f"{field}.partial_reports missing")
    require_correction_work(value["work"], f"{field}.work")
    return value


def verify_report(path: pathlib.Path) -> dict[str, Any]:
    raw = path.read_bytes()
    report = json.loads(raw)
    require(isinstance(report, dict), "report root must be an object")
    require(
        set(report)
        == {
            "schema",
            "claim_scope",
            "client_id",
            "authority_manifest_sha256",
            "exact_verifier_sha256",
            "authority_proof_sha256",
            "scenario_plan",
            "scenario_receipts",
            "all_six_executed",
            "all_contracts_satisfied",
            "terminal_failure",
        },
        "report root schema mismatch",
    )
    require(report.get("schema") == "vigilode-audit2-bateman-local-six-case-report/v1", "report schema mismatch")
    require(
        report.get("claim_scope") == "LOCAL_ONLY_EXPLORATORY_NONAUTHORITATIVE_REAL_CLIENT_VALIDATION",
        "report claim scope mismatch",
    )
    require(report.get("client_id") == "bateman-two-timescale-parent-stable-daughter-v1", "client identity mismatch")
    require(report.get("authority_manifest_sha256") == MANIFEST_SHA256, "manifest hash mismatch")
    require(report.get("exact_verifier_sha256") == VERIFIER_SHA256, "verifier hash mismatch")
    require(report.get("authority_proof_sha256") == PROOF_SHA256, "proof hash mismatch")
    require(report.get("all_six_executed") is True, "all six scenarios were not executed")
    require(report.get("all_contracts_satisfied") is True, "all contracts were not satisfied")
    require(report.get("terminal_failure") is None, "terminal failure is present")

    plan = report.get("scenario_plan")
    receipts = report.get("scenario_receipts")
    require(isinstance(plan, list) and len(plan) == 6, "scenario plan must contain six rows")
    require(isinstance(receipts, list) and len(receipts) == 6, "scenario receipts must contain six rows")

    for index, (expected, plan_row, receipt) in enumerate(zip(EXPECTED, plan, receipts, strict=True), 1):
        scenario_id, operator_case_id, kind, disposition = expected
        require(
            type(plan_row) is dict
            and set(plan_row) == {"ordinal", "scenario_id", "operator_case_id", "kind"},
            f"scenario plan schema mismatch at {index}",
        )
        require(
            type(receipt) is dict
            and set(receipt)
            == {
                "ordinal",
                "scenario_id",
                "operator_case_id",
                "kind",
                "disposition",
                "contract_satisfied",
                "committed",
                "committed_state_bits",
                "committed_state_sha256",
                "candidate_step",
                "selected_step",
                "fallback_step",
                "candidate_budget",
                "candidate_correction",
                "candidate_failure",
                "candidate_failure_phase",
                "transaction_failure_phase",
                "transaction_failure_message",
                "cache",
                "work",
            },
            f"scenario receipt schema mismatch at {index}",
        )
        plan_tuple = (
            plan_row.get("scenario_id"),
            plan_row.get("operator_case_id"),
            plan_row.get("kind"),
        )
        require(
            type(plan_row.get("ordinal")) is int and plan_row["ordinal"] == index,
            f"scenario plan ordinal mismatch at {index}",
        )
        require(plan_tuple == expected[:3], f"scenario plan mismatch at {index}")
        receipt_tuple = (
            receipt.get("scenario_id"),
            receipt.get("operator_case_id"),
            receipt.get("kind"),
        )
        require(
            type(receipt.get("ordinal")) is int and receipt["ordinal"] == index,
            f"scenario receipt order mismatch at {index}",
        )
        require(receipt_tuple == expected[:3], f"scenario receipt order mismatch at {index}")
        require(receipt.get("disposition") == disposition, f"scenario disposition mismatch at {index}")
        require(receipt.get("contract_satisfied") is True, f"scenario contract_satisfied is false at {index}")
        require(receipt["transaction_failure_phase"] is None, f"scenario {index} transaction failed")
        require(receipt["transaction_failure_message"] is None, f"scenario {index} has a failure message")
        work = require_work_counters(receipt.get("work"), f"scenario[{index}].work")
        committed_case_id = {
            1: "nominal-h1e-3",
            2: "changed-w-h5e-4",
            3: "nominal-h1e-3",
        }.get(index)
        cache = require_cache(
            receipt.get("cache"), f"scenario[{index}].cache", committed_case_id
        )

        if index <= 2:
            require(receipt.get("committed") is None, f"cache probe {index} has transaction disposition")
            require(receipt["committed_state_bits"] is None, f"cache probe {index} has state bits")
            require(receipt["committed_state_sha256"] is None, f"cache probe {index} has state hash")
            for key in (
                "candidate_step",
                "selected_step",
                "fallback_step",
                "candidate_budget",
                "candidate_correction",
                "candidate_failure",
                "candidate_failure_phase",
            ):
                require(receipt[key] is None, f"cache probe {index} unexpectedly has {key}")
            require(
                work["rhs_calls"] == work["rhs_evaluations"] == work["ft_calls"] == 1,
                f"cache probe {index} work receipt mismatch",
            )
        else:
            state_bits = require_uint64_bits(
                receipt.get("committed_state_bits"), f"scenario[{index}].committed_state_bits"
            )
            state_sha256 = receipt.get("committed_state_sha256")
            require(lower_hex_sha256(state_sha256), f"scenario {index} state hash invalid")
            require(
                state_sha256 == state_digest(scenario_id, state_bits),
                f"scenario {index} state digest mismatch",
            )

        if index == 1:
            require(cache["attempts"] == 2, "same-W probe attempt count mismatch")
            require(cache["setup_attempts"] == cache["setup_completed"] == 1, "same-W setup count mismatch")
            require(cache["setup_failures"] == 0, "same-W setup failure mismatch")
            require(cache["same_binding_reuses"] == 1 and cache["commits"] == 2, "same-W reuse/commit mismatch")
            require(
                cache["changed_operator_invalidations"]
                == cache["changed_preconditioner_invalidations"]
                == cache["rollbacks"]
                == 0,
                "same-W cache transition mismatch",
            )
        elif index == 2:
            require(cache["attempts"] == 3, "changed-W cumulative attempt count mismatch")
            require(cache["setup_attempts"] == cache["setup_completed"] == 2, "changed-W setup count mismatch")
            require(cache["setup_failures"] == 0, "changed-W setup failure mismatch")
            require(cache["same_binding_reuses"] == 1, "changed-W lost same-binding history")
            require(cache["changed_operator_invalidations"] == 1, "changed-W operator invalidation mismatch")
            require(cache["changed_preconditioner_invalidations"] == 1, "changed-W PC invalidation mismatch")
            require(cache["commits"] == 3, "changed-W commit count mismatch")
            require(cache["rollbacks"] == 0, "changed-W rollback mismatch")
        elif index == 3:
            require(receipt.get("committed") is True, "nominal candidate did not commit")
            candidate = require_step(
                receipt.get("candidate_step"),
                accepted=True,
                fallback=False,
                field="nominal candidate_step",
            )
            selected = require_step(
                receipt.get("selected_step"),
                accepted=True,
                fallback=False,
                field="nominal selected_step",
            )
            require(receipt["fallback_step"] is None, "nominal candidate has a fallback step")
            require(receipt["candidate_failure"] is None, "nominal candidate has a correction failure")
            require(receipt["candidate_failure_phase"] is None, "nominal candidate has a failure phase")
            require_correction_success(receipt["candidate_correction"], "nominal candidate_correction")
            require_counter_prefix(candidate["counters"], work, "nominal candidate")
            require(selected["counters"] == work, "nominal selected work mismatch")
            require(
                receipt["committed_state_bits"]
                == candidate["y_new_bits"]
                == selected["y_new_bits"],
                "nominal committed state does not match the selected candidate",
            )
            budget = require_budget(
                receipt.get("candidate_budget"),
                scenario_id=scenario_id,
                candidate_bits=candidate["y_new_bits"],
                nominal=True,
                field="nominal candidate_budget",
            )
            require(budget["accepted"] is True, "nominal independent budget failed")
            require(cache["commits"] == 1 and cache["rollbacks"] == 0, "nominal cache disposition mismatch")
            require(
                (
                    cache["attempts"],
                    cache["setup_attempts"],
                    cache["setup_completed"],
                    cache["setup_failures"],
                    cache["same_binding_reuses"],
                    cache["changed_operator_invalidations"],
                    cache["changed_preconditioner_invalidations"],
                )
                == (1, 1, 1, 0, 0, 0, 0),
                "nominal cache transition mismatch",
            )
            require(work["accepted_steps"] == 1, "nominal accepted-step accounting mismatch")
            require(work["fallback_steps"] == work["rejected_steps"] == 0, "nominal disposition work mismatch")
        elif index == 4:
            require(receipt.get("committed") is True, "strict-budget fallback did not commit")
            candidate = require_step(
                receipt.get("candidate_step"), accepted=True, fallback=False, field="strict candidate_step"
            )
            selected = require_step(
                receipt.get("selected_step"), accepted=True, fallback=True, field="strict selected_step"
            )
            fallback = require_step(
                receipt.get("fallback_step"), accepted=True, fallback=True, field="strict fallback_step"
            )
            require(receipt["candidate_failure"] is None, "strict case has a correction failure")
            require(receipt["candidate_failure_phase"] is None, "strict case has a failure phase")
            require_correction_success(receipt["candidate_correction"], "strict candidate_correction")
            require_counter_prefix(candidate["counters"], work, "strict candidate")
            require(selected == fallback, "strict selected and fallback receipts differ")
            require(selected["counters"] == work, "strict fallback work mismatch")
            require(
                receipt["committed_state_bits"] == selected["y_new_bits"] == fallback["y_new_bits"],
                "strict committed state does not match fallback",
            )
            budget = require_budget(
                receipt.get("candidate_budget"),
                scenario_id=scenario_id,
                candidate_bits=candidate["y_new_bits"],
                nominal=False,
                field="strict candidate_budget",
            )
            require(budget["accepted"] is False, "strict candidate unexpectedly admitted")
            require(cache["commits"] == 0 and cache["rollbacks"] == 1, "strict cache disposition mismatch")
            require(
                (
                    cache["attempts"],
                    cache["setup_attempts"],
                    cache["setup_completed"],
                    cache["setup_failures"],
                    cache["same_binding_reuses"],
                    cache["changed_operator_invalidations"],
                    cache["changed_preconditioner_invalidations"],
                )
                == (1, 1, 1, 0, 0, 0, 0),
                "strict cache transition mismatch",
            )
            require(
                work["accepted_steps"] == 1
                and work["fallback_steps"] == 1
                and work["rejected_steps"] == 0,
                "strict fallback work disposition mismatch",
            )
        elif index == 5:
            require(receipt.get("committed") is True, "late-failure fallback did not commit")
            require(receipt["candidate_step"] is None, "late failure has a candidate step")
            require(receipt["candidate_budget"] is None, "late failure has a candidate budget")
            require(receipt["candidate_correction"] is None, "late failure has a completed correction")
            selected = require_step(
                receipt.get("selected_step"), accepted=True, fallback=True, field="late selected_step"
            )
            fallback = require_step(
                receipt.get("fallback_step"), accepted=True, fallback=True, field="late fallback_step"
            )
            require(selected == fallback, "late selected and fallback receipts differ")
            require(selected["counters"] == work, "late fallback work mismatch")
            require(
                receipt["committed_state_bits"] == selected["y_new_bits"] == fallback["y_new_bits"],
                "late-failure committed state does not match fallback",
            )
            require(receipt.get("candidate_failure_phase") == "solve", "late failure phase mismatch")
            failure = require_correction_failure(receipt["candidate_failure"], "late candidate_failure")
            require(failure["phase"] == "solve", "late failure receipt phase mismatch")
            session = failure["work"]["session"]
            require(
                session["preconditioner_apply_attempts"] == 2
                and session["preconditioner_apply_completed"] == 1,
                "late failure apply ledger mismatch",
            )
            require(
                session["counters"]["preconditioner_apps"] == 1,
                "late failure apply ledger work mismatch",
            )
            require(cache["commits"] == 0 and cache["rollbacks"] == 1, "late-failure cache disposition mismatch")
            require(
                (
                    cache["attempts"],
                    cache["setup_attempts"],
                    cache["setup_completed"],
                    cache["setup_failures"],
                    cache["same_binding_reuses"],
                    cache["changed_operator_invalidations"],
                    cache["changed_preconditioner_invalidations"],
                )
                == (1, 1, 1, 0, 0, 0, 0),
                "late-failure cache transition mismatch",
            )
            require(
                work["accepted_steps"] == 1
                and work["fallback_steps"] == 1
                and work["rejected_steps"] == 0
                and work["linear_solve_failures"] == 1,
                "late-failure work disposition mismatch",
            )
        else:
            require(receipt.get("committed") is False, "terminal rejection committed state")
            candidate = require_step(
                receipt.get("candidate_step"), accepted=False, fallback=False, field="terminal candidate_step"
            )
            require(receipt.get("selected_step") is None, "terminal rejection exposes selected step")
            fallback = require_step(
                receipt.get("fallback_step"), accepted=False, fallback=True, field="terminal fallback_step"
            )
            require(receipt["candidate_failure"] is None, "terminal case has a correction failure")
            require(receipt["candidate_failure_phase"] is None, "terminal case has a failure phase")
            require_correction_success(receipt["candidate_correction"], "terminal candidate_correction")
            require_counter_prefix(candidate["counters"], work, "terminal candidate")
            require(fallback["counters"] == work, "terminal fallback work mismatch")
            require(
                receipt["committed_state_bits"] == list(INITIAL_STATE_BITS),
                "terminal rejection did not preserve the frozen initial state",
            )
            budget = require_budget(
                receipt.get("candidate_budget"),
                scenario_id=scenario_id,
                candidate_bits=candidate["y_new_bits"],
                nominal=False,
                field="terminal candidate_budget",
            )
            require(budget["accepted"] is False, "terminal candidate unexpectedly admitted")
            require(cache["commits"] == 0 and cache["rollbacks"] == 1, "terminal cache disposition mismatch")
            require(
                (
                    cache["attempts"],
                    cache["setup_attempts"],
                    cache["setup_completed"],
                    cache["setup_failures"],
                    cache["same_binding_reuses"],
                    cache["changed_operator_invalidations"],
                    cache["changed_preconditioner_invalidations"],
                )
                == (1, 1, 1, 0, 0, 0, 0),
                "terminal cache transition mismatch",
            )
            require(
                work["accepted_steps"] == 0
                and work["fallback_steps"] == 1
                and work["rejected_steps"] == 1,
                "terminal rejection work disposition mismatch",
            )

    return {
        "status": "LOCAL_SIX_CASE_RECEIPT_VERIFIED",
        "scenario_count": len(receipts),
        "report_sha256": hashlib.sha256(raw).hexdigest(),
        "claim_scope": report["claim_scope"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=pathlib.Path)
    args = parser.parse_args()
    print(json.dumps(verify_report(args.report), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
