#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("validate_k0_cells", HERE / "validate_k0_cells.py")
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)

KERNELS = ("legacy-restarted-gmres", "incremental-givens-candidate")
FAMILIES = (
    "robertson-ramped",
    "hires-ramped",
    "van-der-pol-ramped",
    "rotating-nonnormal",
    "nonautonomous-stiff-forcing",
    "semilinear-advection-diffusion-ramped",
)
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


def stage(family: str, kernel: str, index: int) -> dict[str, object]:
    return {
        "schema": "vigilode-k0-stage-receipt/v1",
        "execution_state": "COMPLETE",
        "case_id": f"{family}-n320",
        "family": family,
        "kernel_arm": kernel,
        "attempt_id": 0,
        "stage_index": index,
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
        "solver_method": "gmres",
        "initial_guess_source": "ZERO",
        "initial_true_residual": 1.0,
        "final_true_residual": 1.0e-12,
        "work": {
            "linear_matvecs": 2,
            "diagnostic_matvecs": 1,
            "operator_applies_total": 3,
            "telemetry_jvp_overhead": int(index > 0),
        },
        "geometry": {
            "scaled_nonlinear_remainder": 2.0,
            "rhs_to_retained_angle_rad": None if index == 0 else 0.5,
            "rhs_singular_values": [1.0],
        },
        "error": None,
    }


def audit_event(family: str) -> dict[str, object]:
    return {
        "event_key": "trajectory:1:2:1",
        "recommended": False,
        "zeta34_signed_margin": 1.0,
        "audit_arm": "legacy-fixed",
        "audit_family": family,
        "audit_event_key": "trajectory:1:2:1",
        "audit_full_e_eligible": True,
        "audit_full_e_attempted": True,
        "audit_full_e_completed": True,
        "audit_full_e_total_error": 1.5,
        "audit_full_e_locally_admissible": False,
        "audit_full_e_failure": None,
        "audit_full_e_work": {"jvp_vectors": 1},
        "audit_unsafe": True,
        "audit_evidence_status": "complete",
    }


def cell(family: str, kernel: str) -> dict[str, object]:
    events = [audit_event(family)] if family == "hires-ramped" else []
    return {
        "schema": "vigilode-k0-stage-telemetry-cell/v1",
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
        "execution_state": "COMPLETE",
        "repository": "cosmosapjw-quantum/vigilode",
        "source_pull_request": 20,
        "source_base_sha": "e1124586a4029f86669e7489278c61ef676d61aa",
        "source_base_tree": "adbb933cf3bf3d401d652c8a6d9df661d8500a2b",
        "execution_head_sha": "7" * 40,
        "execution_head_tree": "8" * 40,
        "rust_version": "rustc 1.94.1",
        "cargo_version": "cargo 1.94.1",
        "case_id": f"{family}-n320",
        "family": family,
        "kernel_arm": kernel,
        "tolerance": {"kind": "LegacyFixed", "rtol": 1.0e-10, "atol": 1.0e-12},
        "solver_configuration_tokens": ["a" * 64],
        "stages": [stage(family, kernel, index) for index in range(8)],
        "receipt": {
            "schema": "vigilode-a1-two-arm-atomic-cell-v2",
            "profile": "enforced-budget-holdout-320",
            "family": family,
            "tolerance_arm": "legacy-fixed",
            "outer_rtol": 1.0e-5,
            "linear_rtol": 1.0e-10,
            "linear_atol": 1.0e-12,
            "phi_relative_tolerance": 3.0e-2 * 1.0e-5,
            "phi_absolute_tolerance": 3.0e-9,
            "attempts": 1,
            "accepted_steps": 1,
            "rejected_steps": 0,
            "rhs_evaluations": 1,
            "jvp_vectors": 1,
            "linear_matvecs": 1,
            "trace_digest": "b" * 64,
            "switching_active": False,
            "frozen_zeta34_tau": 13.39706618860016,
            "event_rows": events,
            "recommendation_rows": [],
            "hard_gates": {key: True for key in HARD_GATES},
            "limitations": [],
        },
        "error": None,
        "limitations": [],
    }


class K0CellValidatorContracts(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.directory = pathlib.Path(self.temp.name)
        self.cells: dict[tuple[str, str], dict[str, object]] = {}
        for kernel in KERNELS:
            for family in FAMILIES:
                payload = cell(family, kernel)
                self.cells[(kernel, family)] = payload
                self.write(kernel, family, payload)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, kernel: str, family: str, payload: dict[str, object]) -> None:
        (self.directory / f"{kernel}--{family}.json").write_text(
            json.dumps(payload), encoding="utf-8"
        )

    def assert_invalid(self, fragment: str) -> None:
        with self.assertRaises(VALIDATOR.CellValidationError) as caught:
            VALIDATOR.validate_evidence_dir(self.directory)
        self.assertIn(fragment, str(caught.exception))

    def test_exact_two_by_six_matrix_is_complete(self) -> None:
        report = VALIDATOR.validate_evidence_dir(self.directory)
        self.assertEqual(report["verdict"], "COMPLETE")
        self.assertEqual(report["observed_cell_count"], 12)

    def test_missing_cell_is_rejected(self) -> None:
        next(self.directory.glob("*.json")).unlink()
        self.assert_invalid("expected 12")

    def test_tolerance_family_audit_claim_and_accounting_mutations_are_rejected(self) -> None:
        mutations = (
            ("legacy-restarted-gmres", "robertson-ramped", ("tolerance", "rtol"), 2.0e-10, "tolerance"),
            ("legacy-restarted-gmres", "robertson-ramped", ("family",), "hires-ramped", "identity"),
            ("legacy-restarted-gmres", "hires-ramped", ("receipt", "event_rows", 0, "audit_full_e_completed"), None, "audit"),
            ("legacy-restarted-gmres", "robertson-ramped", ("claim_class",), "AUTHORITATIVE", "claim"),
            ("legacy-restarted-gmres", "robertson-ramped", ("stages", 0, "work", "operator_applies_total"), 2, "accounting"),
        )
        for kernel, family, path, value, fragment in mutations:
            with self.subTest(path=path):
                payload = copy.deepcopy(self.cells[(kernel, family)])
                target = payload
                for key in path[:-1]:
                    target = target[key]  # type: ignore[index]
                if value is None:
                    del target[path[-1]]  # type: ignore[index]
                else:
                    target[path[-1]] = value  # type: ignore[index]
                self.write(kernel, family, payload)
                self.assert_invalid(fragment)
                self.write(kernel, family, self.cells[(kernel, family)])

    def test_explicit_error_cell_is_preserved_as_stop_invalid_matrix(self) -> None:
        payload = copy.deepcopy(self.cells[(KERNELS[0], FAMILIES[0])])
        payload["execution_state"] = "ERROR"
        payload["receipt"] = None
        payload["stages"] = []
        payload["solver_configuration_tokens"] = []
        payload["error"] = {"kind": "EXECUTION_ERROR", "message": "preserved"}
        self.write(KERNELS[0], FAMILIES[0], payload)
        report = VALIDATOR.validate_evidence_dir(self.directory)
        self.assertEqual(report["verdict"], "STOP_INVALID")
        self.assertEqual(report["cell_states"][f"{KERNELS[0]}--{FAMILIES[0]}"], "ERROR")


if __name__ == "__main__":
    unittest.main()
