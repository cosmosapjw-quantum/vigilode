#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SUMMARIZER = ROOT / "tools" / "summarize-a1-tolerance-arms.py"
ARMS = ("legacy-fixed", "outer-scaled-numeric-parity")
FAMILIES = (
    "robertson-ramped",
    "hires-ramped",
    "van-der-pol-ramped",
    "rotating-nonnormal",
    "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
)
TAU = 13.39706618860016
PHI_RTOL = 3.0e-2 * 1.0e-5
PHI_ATOL = 3.0e-4 * 1.0e-5


def audit_work() -> dict[str, int]:
    return {
        "rhs_calls": 0,
        "rhs_batch_calls": 0,
        "rhs_evaluations": 0,
        "ft_calls": 0,
        "jacobian_builds": 0,
        "jvp_calls": 1,
        "jvp_vectors": 1,
        "mass_matvecs": 0,
        "nonlinear_solves": 0,
        "nonlinear_iterations": 0,
        "nonlinear_residual_evaluations": 0,
        "nonlinear_jacobian_evaluations": 0,
        "nonlinear_failures": 0,
        "linear_solves": 0,
        "linear_iterations": 0,
        "linear_matvecs": 0,
        "preconditioner_apps": 0,
        "direct_factorizations": 0,
        "direct_solve_calls": 0,
        "recycle_projection_calls": 0,
        "recycle_same_operator_uses": 0,
        "recycle_cross_operator_refreshes": 0,
        "recycle_refresh_matvecs": 0,
        "recycle_updates": 0,
        "recycle_vectors_selected": 0,
        "recycle_dropped_vectors": 0,
        "harmonic_ritz_solves": 0,
        "orthogonalization_inner_products": 0,
        "orthogonalization_vector_updates": 0,
        "diagnostic_matvecs": 0,
        "phi_actions": 1,
        "phi_krylov_vectors": 1,
        "phi_projected_exponentials": 1,
        "phi_restarts": 0,
        "phi_dense_oracle_calls": 0,
        "block_linear_solves": 0,
        "block_linear_iterations": 0,
        "block_matvecs": 0,
        "block_preconditioner_apps": 0,
        "fast_attempts": 0,
        "fast_accepts": 0,
        "fallback_steps": 0,
        "accepted_steps": 0,
        "rejected_steps": 0,
    }


def cell(arm: str, family: str) -> dict[str, object]:
    unsafe_control = family == "hires-ramped"
    key = f"{arm}:{family}:event-0"
    event = {
        "event_key": key,
        "trajectory_id": f"{family}-n320-rtol1e-5",
        "decision_accepted_step": 4,
        "target_attempt_index": 5,
        "target_accepted_steps_before": 5,
        "t_start": 0.125,
        "h": 0.01,
        "quadratic_drift_zeta34": TAU + 1.0 if unsafe_control else TAU - 1.0,
        "zeta34_signed_margin": 1.0 if unsafe_control else -1.0,
        "recommended": not unsafe_control,
        "shadow_full_e_completed": not unsafe_control,
        "shadow_full_e_locally_admissible": not unsafe_control,
        "audit_arm": arm,
        "audit_family": family,
        "audit_event_key": key,
        "audit_full_e_eligible": True,
        "audit_full_e_attempted": True,
        "audit_full_e_completed": True,
        "audit_full_e_total_error": 2.0 if unsafe_control else 0.5,
        "audit_full_e_locally_admissible": not unsafe_control,
        "audit_full_e_failure": None,
        "audit_full_e_work": audit_work(),
        "audit_unsafe": unsafe_control,
        "audit_evidence_status": "complete",
    }
    return {
        "schema": "vigilode-a1-two-arm-atomic-cell-v2",
        "repository": "cosmosapjw-quantum/vigilode",
        "pull_request": 18,
        "scientific_execution_head_sha": "1" * 40,
        "scientific_execution_head_tree": "2" * 40,
        "base_sha": "3" * 40,
        "base_tree": "4" * 40,
        "tested_execution_merge_sha": "5" * 40,
        "tested_execution_merge_tree": "6" * 40,
        "execution_workflow_run_id": 123,
        "execution_workflow_run_attempt": 1,
        "rust_version": "rustc 1.94.1",
        "cargo_version": "cargo 1.94.1",
        "profile": "enforced-budget-holdout-320",
        "family": family,
        "arm": arm,
        "outer_rtol": 1.0e-5,
        "linear_rtol": 1.0e-10 if arm == "legacy-fixed" else PHI_RTOL,
        "linear_atol": 1.0e-12 if arm == "legacy-fixed" else PHI_ATOL,
        "phi_relative_tolerance": PHI_RTOL,
        "phi_absolute_tolerance": PHI_ATOL,
        "attempts": 10,
        "accepted_steps": 9,
        "rejected_steps": 1,
        "rhs_evaluations": 20,
        "jvp_vectors": 30,
        "linear_matvecs": 40,
        "trace_digest": ("a" if arm == "legacy-fixed" else "b") * 64,
        "switching_active": False,
        "frozen_zeta34_tau": TAU,
        "event_rows": [event],
        "recommendation_rows": [] if unsafe_control else [{"event_key": key}],
        "hard_gates": {
            "all_rjf_trajectories_successful": True,
            "rjf_trace_exact_excluding_wall": True,
            "zero_budget_breaches": True,
            "prefix_transactions_resolved": True,
            "zero_continuation_failures": True,
            "zero_unsafe_recommendations": True,
            "work_ledgers_exact": True,
            "realized_work_ratios_finite": True,
            "resume_cardinality_exact": True,
            "shadow_implicit_expensive_work_zero": True,
            "active_switching_false": True,
            "passed": True,
        },
        "limitations": ["receipt-only synthetic test cell"],
    }


class ReceiptAggregateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.cells = [cell(arm, family) for arm in ARMS for family in FAMILIES]

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_aggregate(self, cells: list[dict[str, object]] | None = None) -> subprocess.CompletedProcess[str]:
        cells = self.cells if cells is None else cells
        cells_dir = self.root / "cells"
        cells_dir.mkdir(exist_ok=True)
        for old in cells_dir.glob("*.json"):
            old.unlink()
        for index, payload in enumerate(cells):
            (cells_dir / f"cell-{index:02d}.json").write_text(
                json.dumps(payload, sort_keys=True), encoding="utf-8"
            )
        return subprocess.run(
            [
                "python3",
                str(SUMMARIZER),
                "--cells-dir",
                str(cells_dir),
                "--output-json",
                str(self.root / "aggregate.json"),
                "--output-markdown",
                str(self.root / "aggregate.md"),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def aggregate(self) -> dict[str, object]:
        return json.loads((self.root / "aggregate.json").read_text(encoding="utf-8"))

    def test_complete_matrix_recomputes_discriminating_decision_and_manifest(self) -> None:
        result = self.run_aggregate()
        self.assertEqual(result.returncode, 0, result.stderr)
        aggregate = self.aggregate()
        self.assertEqual(len(aggregate["complete_cell_keys"]), 12)
        self.assertEqual(aggregate["predeclared_decision"], "ADMISSIBLE_AND_DISCRIMINATING")
        self.assertEqual(len(aggregate["artifact_content_manifest"]), 12)
        self.assertEqual(len(aggregate["audit_unsafe_event_keys"]), 2)
        self.assertEqual(aggregate["unsafe_recommendation_keys"], [])
        self.assertTrue(all(aggregate["hires_positive_control"].values()))

    def test_invalidated_v1_hires_cell_is_rejected_for_authority(self) -> None:
        old = cell("legacy-fixed", "hires-ramped")
        old["schema"] = "vigilode-a1-two-arm-atomic-cell-v1"
        old["execution_workflow_run_id"] = 32906175896
        event = old["event_rows"][0]
        for field in list(event):
            if field.startswith("audit_"):
                del event[field]
        event["shadow_full_e_completed"] = False
        event["shadow_full_e_locally_admissible"] = False
        result = self.run_aggregate([old])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("wrong schema", result.stderr)

    def test_invalidated_workflow_id_is_rejected_even_under_v2_schema(self) -> None:
        cells = copy.deepcopy(self.cells)
        for payload in cells:
            payload["execution_workflow_run_id"] = 32906175896
        result = self.run_aggregate(cells)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("diagnostic-only", result.stderr)

    def test_runtime_shadow_absence_does_not_imply_audit_safety(self) -> None:
        hires = cell("legacy-fixed", "hires-ramped")["event_rows"][0]
        self.assertFalse(hires["shadow_full_e_completed"])
        self.assertTrue(hires["audit_full_e_completed"])
        self.assertTrue(hires["audit_unsafe"])

    def test_unrecommended_event_can_have_completed_independent_audit(self) -> None:
        result = self.run_aggregate()
        self.assertEqual(result.returncode, 0, result.stderr)
        for payload in self.cells:
            if payload["family"] == "hires-ramped":
                event = payload["event_rows"][0]
                self.assertFalse(event["recommended"])
                self.assertFalse(event["shadow_full_e_completed"])
                self.assertTrue(event["audit_full_e_completed"])

    def test_missing_or_incomplete_audit_evidence_stops_before_decision(self) -> None:
        variants = []
        missing = copy.deepcopy(self.cells)
        del missing[0]["event_rows"][0]["audit_full_e_completed"]
        variants.append(missing)
        incomplete = copy.deepcopy(self.cells)
        event = incomplete[0]["event_rows"][0]
        event["audit_full_e_completed"] = False
        event["audit_full_e_total_error"] = None
        event["audit_full_e_locally_admissible"] = None
        event["audit_full_e_work"] = None
        event["audit_unsafe"] = None
        event["audit_evidence_status"] = "failed"
        event["audit_full_e_failure"] = "audit continuation failed"
        variants.append(incomplete)
        for cells in variants:
            result = self.run_aggregate(cells)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((self.root / "aggregate.json").exists())

    def test_audit_identity_mismatch_is_rejected(self) -> None:
        for field, value in (
            ("audit_arm", "outer-scaled-numeric-parity"),
            ("audit_family", "hires-ramped"),
            ("audit_event_key", "different:event"),
        ):
            cells = copy.deepcopy(self.cells)
            event = cells[0]["event_rows"][0]
            if event[field] == value:
                value = "legacy-fixed" if field == "audit_arm" else "robertson-ramped:other"
            event[field] = value
            result = self.run_aggregate(cells)
            self.assertNotEqual(result.returncode, 0)

    def test_audit_failure_and_ineligibility_require_explicit_reason(self) -> None:
        cells = copy.deepcopy(self.cells)
        event = cells[0]["event_rows"][0]
        event["audit_full_e_eligible"] = False
        event["audit_full_e_attempted"] = False
        event["audit_full_e_completed"] = False
        event["audit_full_e_total_error"] = None
        event["audit_full_e_locally_admissible"] = None
        event["audit_full_e_work"] = None
        event["audit_unsafe"] = None
        event["audit_evidence_status"] = "ineligible"
        event["audit_full_e_failure"] = None
        result = self.run_aggregate(cells)
        self.assertNotEqual(result.returncode, 0)

    def test_missing_duplicate_extra_and_unknown_domain_are_rejected(self) -> None:
        variants = [
            self.cells[:-1],
            self.cells + [copy.deepcopy(self.cells[0])],
            self.cells + [cell("legacy-fixed", "unknown-family")],
        ]
        for cells in variants:
            result = self.run_aggregate(cells)
            self.assertNotEqual(result.returncode, 0)

    def test_tau_or_scientific_execution_identity_mismatch_is_rejected(self) -> None:
        for field, value in [
            ("frozen_zeta34_tau", TAU + 1.0),
            ("scientific_execution_head_sha", "9" * 40),
            ("tested_execution_merge_tree", "8" * 40),
        ]:
            cells = copy.deepcopy(self.cells)
            cells[-1][field] = value
            result = self.run_aggregate(cells)
            self.assertNotEqual(result.returncode, 0)

    def test_derived_totals_are_recomputed_from_atomic_rows(self) -> None:
        cells = copy.deepcopy(self.cells)
        cells[0]["recommendations"] = 999999
        cells[0]["unsafe_recommendations"] = 999999
        result = self.run_aggregate(cells)
        self.assertEqual(result.returncode, 0, result.stderr)
        aggregate = self.aggregate()
        self.assertEqual(aggregate["unsafe_recommendation_keys"], [])

    def test_input_permutation_wall_and_archive_metadata_do_not_change_scientific_digest(self) -> None:
        first = self.run_aggregate()
        self.assertEqual(first.returncode, 0, first.stderr)
        digest = self.aggregate()["scientific_digest"]
        cells = list(reversed(copy.deepcopy(self.cells)))
        for payload in cells:
            payload["wall_seconds"] = 999.0
            payload["archive_metadata"] = {"sha256": "packaging-only"}
        second = self.run_aggregate(cells)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(self.aggregate()["scientific_digest"], digest)

    def test_tracked_receipt_rejects_late_bound_identity_fields(self) -> None:
        for field in (
            "receipt_commit_sha",
            "receipt_commit_tree",
            "external_verification_run_id",
            "external_verification_run_attempt",
        ):
            cells = copy.deepcopy(self.cells)
            cells[0][field] = "forbidden"
            result = self.run_aggregate(cells)
            self.assertNotEqual(result.returncode, 0)

    def test_unsafe_recommendation_forces_not_admissible(self) -> None:
        cells = copy.deepcopy(self.cells)
        event = cells[0]["event_rows"][0]
        event["recommended"] = True
        event["audit_full_e_locally_admissible"] = False
        event["audit_full_e_total_error"] = 2.0
        event["audit_unsafe"] = True
        cells[0]["recommendation_rows"] = [{"event_key": event["event_key"]}]
        result = self.run_aggregate(cells)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.aggregate()["predeclared_decision"], "NOT_ADMISSIBLE")

    def test_positive_control_loss_is_admissible_but_nondiscriminating(self) -> None:
        cells = copy.deepcopy(self.cells)
        for payload in cells:
            if payload["family"] != "hires-ramped":
                continue
            event = payload["event_rows"][0]
            event["quadratic_drift_zeta34"] = TAU - 1.0
            event["zeta34_signed_margin"] = -1.0
            event["audit_full_e_locally_admissible"] = True
            event["audit_full_e_total_error"] = 0.5
            event["audit_unsafe"] = False
        result = self.run_aggregate(cells)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.aggregate()["predeclared_decision"],
            "ADMISSIBLE_BUT_NONDISCRIMINATING",
        )

    def test_nondiscriminating_requires_complete_positive_control_absence(self) -> None:
        cells = copy.deepcopy(self.cells)
        for payload in cells:
            if payload["family"] != "hires-ramped":
                continue
            event = payload["event_rows"][0]
            event["audit_full_e_completed"] = False
            event["audit_full_e_total_error"] = None
            event["audit_full_e_locally_admissible"] = None
            event["audit_full_e_work"] = None
            event["audit_unsafe"] = None
            event["audit_evidence_status"] = "failed"
            event["audit_full_e_failure"] = "missing positive-control audit"
        result = self.run_aggregate(cells)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "aggregate.json").exists())


if __name__ == "__main__":
    unittest.main()
