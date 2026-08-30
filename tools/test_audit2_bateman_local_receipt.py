#!/usr/bin/env python3
"""Candidate-free contracts for the local Bateman report validator."""

from __future__ import annotations

import importlib.util
import hashlib
import json
import pathlib
import struct
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
VALIDATOR_PATH = (
    ROOT
    / "research"
    / "audit2_real_client_authority_construction_20260830"
    / "verify_local_six_case_receipt.py"
)

spec = importlib.util.spec_from_file_location("audit2_bateman_receipt", VALIDATOR_PATH)
assert spec and spec.loader
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


def work_counters(**overrides):
    value = {key: 0 for key in validator.WORK_COUNTER_FIELDS}
    value.update(overrides)
    return value


def state_digest(scenario_id, bits):
    encoded = scenario_id.encode()
    payload = bytearray(b"VIGILODE\0AUDIT2\0BATEMAN_STATE\0")
    payload.extend(struct.pack(">I", len(encoded)))
    payload.extend(encoded)
    payload.extend(struct.pack(">I", len(bits)))
    for value in bits:
        payload.extend(struct.pack(">Q", value))
    return hashlib.sha256(payload).hexdigest()


def binding(case_id):
    case = validator.OPERATOR_CASES[case_id]
    gamma = struct.unpack(
        ">d", struct.pack(">Q", validator.AUTHORITY_MANIFEST["coefficient_gamma_bits"])
    )[0]
    return {
        "dimension": 4,
        "h_gamma_bits": validator.f64_bits(case["h"] * gamma),
        "frozen_w_semantic": case["frozen_w_semantic"],
        "preconditioner": case["preconditioner_identity"],
        "returned_inverse_diagonal_bits": case["preconditioner_identity"][
            "expected_inverse_diagonal_bits"
        ],
    }


def cache(*, committed_case_id=None, **overrides):
    value = {
        "attempts": 1,
        "setup_attempts": 1,
        "setup_completed": 1,
        "setup_failures": 0,
        "same_binding_reuses": 0,
        "changed_operator_invalidations": 0,
        "changed_preconditioner_invalidations": 0,
        "commits": 0,
        "rollbacks": 0,
        "setup_work": work_counters(),
        "last_setup_failure": None,
        "committed_binding": (
            None if committed_case_id is None else binding(committed_case_id)
        ),
        "pending_binding": None,
    }
    value.update(overrides)
    return value


def step(*, accepted, fallback, bits, counters):
    return {
        "method": (
            "RODAS5P-audit2-protected-sequential-JF-fallback"
            if fallback
            else "RODAS5P-audit2-reusable-preconditioner-candidate"
        ),
        "accepted": accepted,
        "used_fallback": fallback,
        "error_norm": 0.0 if accepted else 2.0,
        "y_new_bits": list(bits),
        "counters": counters,
    }


def correction_work(*, late_failure=False):
    session_counters = work_counters(preconditioner_apps=1 if late_failure else 0)
    return {
        "preparation_counters": work_counters(),
        "correction_jvp_attempts": 0,
        "correction_jvp_completed": 0,
        "diagnostic_shifted_apply_attempts": 0,
        "diagnostic_shifted_apply_completed": 0,
        "diagnostic_jvp_attempts": 0,
        "diagnostic_jvp_completed": 0,
        "coupling_counters": work_counters(),
        "session": {
            "preconditioner_apply_attempts": 2 if late_failure else 1,
            "preconditioner_apply_completed": 1,
            "counters": session_counters,
        },
    }


def correction_success():
    return {
        "projection": {},
        "projected_residual": [],
        "correction": [],
        "solve_reports": [],
        "initial_residual_l2": 0.0,
        "linear_residual_l2": 0.0,
        "work": correction_work(),
    }


def correction_failure():
    return {
        "phase": "solve",
        "message": "frozen synthetic second-apply failure",
        "projection": None,
        "projected_residual": None,
        "partial_correction": [],
        "partial_reports": [],
        "work": correction_work(late_failure=True),
    }


def budget(*, scenario_id, candidate_bits, nominal):
    case = validator.OPERATOR_CASES["nominal-h1e-3"]
    uncertainty = case["reference"]["uncertainty_l2"]
    output_error = validator.conservative_l2_difference_upper(
        list(candidate_bits), case["reference"]["state"]
    )
    output_upper = validator.add_up_nonnegative(output_error, uncertainty)
    output_budget = validator.expected_output_budget(case) if nominal else 0.0
    output_accepted = output_upper <= output_budget
    return {
        "identifier": (
            case["budget"]["identifier"]
            if nominal
            else f"frozen-bateman-{scenario_id}-nominal-h1e-3-v1"
        ),
        "reference_source": validator.REFERENCE_SOURCE,
        "output_error_l2": output_error,
        "output_budget_l2": output_budget,
        "reference_uncertainty_l2": uncertainty,
        "output_error_upper_l2": output_upper,
        "uncertainty_treatment": "DECLARED_UPPER_BOUND",
        "embedded_l2": 0.0,
        "original_target_residual_l2": 0.0,
        "original_target_contraction": 0.0,
        "output_accepted": output_accepted,
        "embedded_accepted": True,
        "original_target_accepted": True,
        "accepted": output_accepted,
    }


def canonical_report():
    rows = [
        ("same-live-context-reuse", "nominal-h1e-3", "same-live-context-cache-probe", "cache-reuse-observed"),
        ("changed-w-invalidation", "changed-w-h5e-4", "changed-w-cache-probe", "changed-w-invalidation-observed"),
        ("nominal-independent-budget", "nominal-h1e-3", "transactional-nominal", "candidate"),
        ("over-strict-budget-fallback", "nominal-h1e-3", "transactional-strict-fallback", "protected-fallback"),
        ("late-preconditioner-failure", "nominal-h1e-3", "transactional-late-apply-failure", "protected-fallback"),
        ("terminal-rejection", "nominal-h1e-3", "transactional-terminal-rejection", "rejected"),
    ]
    plan = [
        {"ordinal": i, "scenario_id": scenario, "operator_case_id": case, "kind": kind}
        for i, (scenario, case, kind, _) in enumerate(rows, 1)
    ]
    reference_bits = [
        validator.f64_bits(value)
        for value in validator.OPERATOR_CASES["nominal-h1e-3"]["reference"]["state"]
    ]
    initial_bits = list(validator.INITIAL_STATE_BITS)
    receipts = []
    for i, (scenario, case, kind, disposition) in enumerate(rows, 1):
        committed_bits = None if i <= 2 else (reference_bits if i == 3 else initial_bits)
        receipt = {
            **plan[i - 1],
            "disposition": disposition,
            "contract_satisfied": True,
            "committed": None if i <= 2 else i != 6,
            "committed_state_bits": committed_bits,
            "committed_state_sha256": (
                None if i <= 2 else state_digest(scenario, committed_bits)
            ),
            "candidate_step": None,
            "selected_step": None,
            "fallback_step": None,
            "candidate_budget": None,
            "candidate_correction": None,
            "candidate_failure": None,
            "candidate_failure_phase": None,
            "transaction_failure_phase": None,
            "transaction_failure_message": None,
            "work": work_counters(rhs_calls=1, rhs_evaluations=1, ft_calls=1),
            "cache": cache(),
        }
        receipts.append(receipt)
    receipts[0]["cache"] = cache(
        committed_case_id="nominal-h1e-3",
        attempts=2, same_binding_reuses=1, commits=2
    )
    receipts[1]["cache"] = cache(
        committed_case_id="changed-w-h5e-4",
        attempts=3,
        setup_attempts=2,
        setup_completed=2,
        same_binding_reuses=1,
        changed_operator_invalidations=1,
        changed_preconditioner_invalidations=1,
        commits=3,
    )
    nominal_work = work_counters(
        rhs_calls=1, rhs_evaluations=1, ft_calls=1, accepted_steps=1
    )
    candidate_work = work_counters(rhs_calls=1, rhs_evaluations=1, ft_calls=1)
    receipts[2]["work"] = nominal_work
    receipts[2]["selected_step"] = step(
        accepted=True, fallback=False, bits=reference_bits, counters=nominal_work
    )
    receipts[2]["candidate_step"] = step(
        accepted=True, fallback=False, bits=reference_bits, counters=candidate_work
    )
    receipts[2]["candidate_budget"] = budget(
        scenario_id=rows[2][0], candidate_bits=reference_bits, nominal=True
    )
    receipts[2]["candidate_correction"] = correction_success()
    receipts[2]["cache"] = cache(committed_case_id="nominal-h1e-3", commits=1)

    strict_work = work_counters(
        rhs_calls=1,
        rhs_evaluations=1,
        ft_calls=1,
        fallback_steps=1,
        accepted_steps=1,
    )
    receipts[3]["work"] = strict_work
    receipts[3]["selected_step"] = step(
        accepted=True, fallback=True, bits=initial_bits, counters=strict_work
    )
    receipts[3]["candidate_step"] = step(
        accepted=True, fallback=False, bits=reference_bits, counters=candidate_work
    )
    receipts[3]["fallback_step"] = step(
        accepted=True, fallback=True, bits=initial_bits, counters=strict_work
    )
    receipts[3]["candidate_budget"] = budget(
        scenario_id=rows[3][0], candidate_bits=reference_bits, nominal=False
    )
    receipts[3]["candidate_correction"] = correction_success()
    receipts[3]["cache"] = cache(rollbacks=1)

    late_work = work_counters(
        rhs_calls=1,
        rhs_evaluations=1,
        ft_calls=1,
        fallback_steps=1,
        accepted_steps=1,
        linear_solve_failures=1,
    )
    receipts[4]["work"] = late_work
    receipts[4]["selected_step"] = step(
        accepted=True, fallback=True, bits=initial_bits, counters=late_work
    )
    receipts[4]["fallback_step"] = step(
        accepted=True, fallback=True, bits=initial_bits, counters=late_work
    )
    receipts[4]["candidate_failure"] = correction_failure()
    receipts[4]["candidate_failure_phase"] = "solve"
    receipts[4]["cache"] = cache(rollbacks=1)

    terminal_work = work_counters(
        rhs_calls=1,
        rhs_evaluations=1,
        ft_calls=1,
        fallback_steps=1,
        rejected_steps=1,
    )
    receipts[5]["work"] = terminal_work
    receipts[5]["fallback_step"] = step(
        accepted=False, fallback=True, bits=initial_bits, counters=terminal_work
    )
    receipts[5]["candidate_step"] = step(
        accepted=False, fallback=False, bits=reference_bits, counters=candidate_work
    )
    receipts[5]["candidate_budget"] = budget(
        scenario_id=rows[5][0], candidate_bits=reference_bits, nominal=False
    )
    receipts[5]["candidate_correction"] = correction_success()
    receipts[5]["cache"] = cache(rollbacks=1)
    return {
        "schema": "vigilode-audit2-bateman-local-six-case-report/v1",
        "claim_scope": "LOCAL_ONLY_EXPLORATORY_NONAUTHORITATIVE_REAL_CLIENT_VALIDATION",
        "client_id": "bateman-two-timescale-parent-stable-daughter-v1",
        "authority_manifest_sha256": validator.MANIFEST_SHA256,
        "exact_verifier_sha256": validator.VERIFIER_SHA256,
        "authority_proof_sha256": validator.PROOF_SHA256,
        "scenario_plan": plan,
        "scenario_receipts": receipts,
        "all_six_executed": True,
        "all_contracts_satisfied": True,
        "terminal_failure": None,
    }


class LocalReceiptValidatorTests(unittest.TestCase):
    def write(self, report):
        directory = tempfile.TemporaryDirectory(prefix="audit2-bateman-report-")
        self.addCleanup(directory.cleanup)
        path = pathlib.Path(directory.name) / "result_summary.json"
        path.write_text(json.dumps(report))
        return path

    def test_exact_six_case_shape_and_invariants_pass(self):
        summary = validator.verify_report(self.write(canonical_report()))
        self.assertEqual(summary["status"], "LOCAL_SIX_CASE_RECEIPT_VERIFIED")
        self.assertEqual(summary["scenario_count"], 6)

    def test_reordered_or_self_asserted_receipt_fails_closed(self):
        report = canonical_report()
        report["scenario_receipts"][0], report["scenario_receipts"][1] = (
            report["scenario_receipts"][1],
            report["scenario_receipts"][0],
        )
        with self.assertRaisesRegex(ValueError, "receipt order"):
            validator.verify_report(self.write(report))

    def test_failed_contract_cannot_hide_behind_top_level_true(self):
        report = canonical_report()
        report["scenario_receipts"][3]["contract_satisfied"] = False
        with self.assertRaisesRegex(ValueError, "contract_satisfied"):
            validator.verify_report(self.write(report))

    def test_empty_work_counters_cannot_pass_as_complete_evidence(self):
        report = canonical_report()
        report["scenario_receipts"][2]["work"] = {}
        with self.assertRaisesRegex(ValueError, "work"):
            validator.verify_report(self.write(report))

    def test_state_digest_is_recomputed_from_exact_state_bits(self):
        report = canonical_report()
        report["scenario_receipts"][2]["committed_state_sha256"] = "b" * 64
        with self.assertRaisesRegex(ValueError, "state digest"):
            validator.verify_report(self.write(report))

    def test_budget_cannot_self_assert_acceptance_with_a_failed_subgate(self):
        report = canonical_report()
        report["scenario_receipts"][2]["candidate_budget"].update(
            {
                "embedded_accepted": False,
                "original_target_accepted": True,
                "accepted": True,
            }
        )
        with self.assertRaisesRegex(ValueError, "budget"):
            validator.verify_report(self.write(report))

    def test_budget_uses_the_exact_rust_uncertainty_enum_spelling(self):
        report = canonical_report()
        report["scenario_receipts"][2]["candidate_budget"][
            "uncertainty_treatment"
        ] = "declared-upper-bound"
        with self.assertRaisesRegex(ValueError, "uncertainty treatment"):
            validator.verify_report(self.write(report))

    def test_late_failure_requires_the_second_apply_ledger(self):
        report = canonical_report()
        report["scenario_receipts"][4]["candidate_failure"]["work"]["session"][
            "preconditioner_apply_attempts"
        ] = 1
        with self.assertRaisesRegex(ValueError, "apply ledger"):
            validator.verify_report(self.write(report))

    def test_boolean_cache_counter_is_not_an_integer_receipt(self):
        report = canonical_report()
        report["scenario_receipts"][2]["cache"]["attempts"] = True
        with self.assertRaisesRegex(ValueError, "cache"):
            validator.verify_report(self.write(report))

    def test_transaction_cache_transition_is_exact_not_merely_monotone(self):
        report = canonical_report()
        report["scenario_receipts"][2]["cache"].update(
            {"attempts": 42, "setup_attempts": 41, "setup_completed": 41}
        )
        with self.assertRaisesRegex(ValueError, "cache transition"):
            validator.verify_report(self.write(report))

    def test_selected_fallback_clone_cannot_drift(self):
        report = canonical_report()
        report["scenario_receipts"][3]["selected_step"]["error_norm"] = 0.5
        with self.assertRaisesRegex(ValueError, "selected and fallback"):
            validator.verify_report(self.write(report))

    def test_boolean_ordinal_is_not_an_integer_ordinal(self):
        report = canonical_report()
        report["scenario_plan"][0]["ordinal"] = True
        with self.assertRaisesRegex(ValueError, "plan ordinal"):
            validator.verify_report(self.write(report))


if __name__ == "__main__":
    unittest.main(verbosity=2)
