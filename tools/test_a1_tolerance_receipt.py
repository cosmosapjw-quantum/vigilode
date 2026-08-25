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


def cell(arm: str, family: str) -> dict[str, object]:
    unsafe_control = family == "hires-ramped"
    key = f"{arm}:{family}:event-0"
    event = {
        "event_key": key,
        "trajectory_id": f"{family}-n320-rtol1e-5",
        "decision_accepted_step": 4,
        "target_attempt_index": 5,
        "target_accepted_steps_before": 5,
        "quadratic_drift_zeta34": TAU + 1.0 if unsafe_control else TAU - 1.0,
        "zeta34_signed_margin": 1.0 if unsafe_control else -1.0,
        "recommended": not unsafe_control,
        "shadow_full_e_completed": True,
        "shadow_full_e_locally_admissible": not unsafe_control,
        "audit_unsafe": unsafe_control,
    }
    return {
        "schema": "vigilode-a1-two-arm-atomic-cell-v1",
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
        event["shadow_full_e_locally_admissible"] = False
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
            event["shadow_full_e_locally_admissible"] = True
            event["audit_unsafe"] = False
        result = self.run_aggregate(cells)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.aggregate()["predeclared_decision"],
            "ADMISSIBLE_BUT_NONDISCRIMINATING",
        )


if __name__ == "__main__":
    unittest.main()
