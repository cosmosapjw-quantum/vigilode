#!/usr/bin/env python3
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from typing import Any

TOOLS = Path(__file__).resolve().parent
AGGREGATE_PATH = TOOLS / "summarize-a1-post-a2a3-kernel.py"
RUNNER_PATH = TOOLS / "run-a1-post-a2a3-kernel-cell.py"

spec = importlib.util.spec_from_file_location("a1_kernel_aggregate", AGGREGATE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load aggregate module")
aggregate_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(aggregate_module)


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def sha256(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def hard_gates() -> dict[str, bool]:
    return {key: True for key in aggregate_module.REQUIRED_HARD_GATES}


def identity() -> dict[str, Any]:
    return {
        "repository": "cosmosapjw-quantum/vigilode",
        "pull_request": 20,
        "scientific_execution_head_sha": "1" * 40,
        "scientific_execution_head_tree": "2" * 40,
        "base_sha": "3" * 40,
        "base_tree": "4" * 40,
        "tested_execution_merge_sha": "5" * 40,
        "tested_execution_merge_tree": "6" * 40,
        "execution_workflow_run_id": 123456,
        "execution_workflow_run_attempt": 1,
        "rust_version": "rustc 1.94.1 (test)",
        "cargo_version": "cargo 1.94.1 (test)",
    }


def complete_event(family: str, unsafe_positive_control: bool) -> dict[str, Any]:
    return {
        "event_key": f"{family}:2:3:2",
        "trajectory_id": f"{family}-n320",
        "decision_accepted_step": 2,
        "target_attempt_index": 3,
        "target_accepted_steps_before": 2,
        "t_start": 0.1,
        "h": 0.01,
        "quadratic_drift_zeta34": aggregate_module.FROZEN_TAU + 1.0,
        "zeta34_signed_margin": 1.0,
        "recommended": False,
        "shadow_full_e_completed": False,
        "shadow_full_e_locally_admissible": False,
        "audit_arm": aggregate_module.TOLERANCE_ARM,
        "audit_family": family,
        "audit_event_key": f"{family}:2:3:2",
        "audit_full_e_eligible": True,
        "audit_full_e_attempted": True,
        "audit_full_e_completed": True,
        "audit_full_e_total_error": 1.25 if unsafe_positive_control else 0.25,
        "audit_full_e_locally_admissible": not unsafe_positive_control,
        "audit_full_e_failure": None,
        "audit_full_e_work": {"jvp_vectors": 7},
        "audit_unsafe": unsafe_positive_control,
        "audit_evidence_status": "complete",
    }


def payload(family: str, kernel: str, ordinal: int) -> dict[str, Any]:
    events = [complete_event(family, family == "hires-ramped")]
    receipt = {
        "schema": aggregate_module.RECEIPT_SCHEMA,
        **identity(),
        "profile": aggregate_module.PROFILE,
        "family": family,
        "arm": aggregate_module.TOLERANCE_ARM,
        "outer_rtol": aggregate_module.OUTER_RTOL,
        "linear_rtol": aggregate_module.LINEAR_RTOL,
        "linear_atol": aggregate_module.LINEAR_ATOL,
        "phi_relative_tolerance": aggregate_module.PHI_RELATIVE_TOLERANCE,
        "phi_absolute_tolerance": aggregate_module.PHI_ABSOLUTE_TOLERANCE,
        "attempts": 10 + ordinal,
        "accepted_steps": 8 + ordinal,
        "rejected_steps": 2,
        "rhs_evaluations": 100 + ordinal,
        "jvp_vectors": 200 + ordinal,
        "linear_matvecs": 180 + ordinal,
        "trace_digest": f"{ordinal + 1:064x}"[-64:],
        "switching_active": False,
        "frozen_zeta34_tau": aggregate_module.FROZEN_TAU,
        "event_rows": events,
        "recommendation_rows": [],
        "hard_gates": hard_gates(),
        "limitations": [],
    }
    return {
        "schema": aggregate_module.PAYLOAD_SCHEMA,
        "status": aggregate_module.PAYLOAD_STATUS,
        "kernel_arm": kernel,
        "tolerance_arm": aggregate_module.TOLERANCE_ARM,
        "receipt": receipt,
    }


def envelope(family: str, kernel: str, ordinal: int) -> dict[str, Any]:
    scientific_payload = payload(family, kernel, ordinal)
    digest = sha256(scientific_payload)
    return {
        "schema": aggregate_module.ENVELOPE_SCHEMA,
        "status": aggregate_module.PAYLOAD_STATUS,
        "execution_state": "COMPLETE",
        "family": family,
        "kernel_arm": kernel,
        "deterministic_replay": True,
        "payload_sha256_first": digest,
        "payload_sha256_second": digest,
        "payload": scientific_payload,
        "errors": [],
    }


def write_matrix(root: Path) -> list[Path]:
    paths: list[Path] = []
    ordinal = 0
    for family in aggregate_module.FAMILIES:
        for kernel in aggregate_module.KERNELS:
            path = root / f"{kernel}--{family}.json"
            path.write_text(
                json.dumps(envelope(family, kernel, ordinal), indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            paths.append(path)
            ordinal += 1
    return paths


class AggregateContracts(unittest.TestCase):
    def test_complete_matrix_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_matrix(root)
            report = aggregate_module.aggregate(root)
            self.assertEqual(report["verdict"], "COMPLETE")
            self.assertEqual(report["observed_envelope_count"], 12)
            self.assertEqual(report["observed_cell_count"], 12)
            self.assertEqual(report["validation_errors"], [])

    def test_missing_cell_stops_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = write_matrix(root)
            paths[0].unlink()
            report = aggregate_module.aggregate(root)
            self.assertEqual(report["verdict"], "STOP_INVALID")
            self.assertTrue(any("missing matrix cells" in error for error in report["validation_errors"]))

    def test_tolerance_drift_stops_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = write_matrix(root)
            value = json.loads(paths[0].read_text(encoding="utf-8"))
            value["payload"]["receipt"]["linear_atol"] = 1.0e-11
            # Preserve the outer envelope digests to ensure payload binding is also tested.
            paths[0].write_text(json.dumps(value), encoding="utf-8")
            report = aggregate_module.aggregate(root)
            self.assertEqual(report["verdict"], "STOP_INVALID")
            errors = "\n".join(report["validation_errors"])
            self.assertIn("linear atol drift", errors)
            self.assertIn("does not bind the scientific payload", errors)

    def test_unsafe_recommendation_stops_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = write_matrix(root)
            value = json.loads(paths[0].read_text(encoding="utf-8"))
            event = value["payload"]["receipt"]["event_rows"][0]
            event["recommended"] = True
            event["audit_unsafe"] = True
            value["payload"]["receipt"]["recommendation_rows"] = [
                {"event_key": event["event_key"]}
            ]
            digest = sha256(value["payload"])
            value["payload_sha256_first"] = digest
            value["payload_sha256_second"] = digest
            paths[0].write_text(json.dumps(value), encoding="utf-8")
            report = aggregate_module.aggregate(root)
            self.assertEqual(report["verdict"], "STOP_INVALID")
            self.assertTrue(any("unsafe recommendation" in error for error in report["validation_errors"]))

    def test_error_envelope_is_preserved_and_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = write_matrix(root)
            value = json.loads(paths[0].read_text(encoding="utf-8"))
            value["execution_state"] = "ERROR"
            value["deterministic_replay"] = False
            value["errors"] = ["scientific solver failed"]
            paths[0].write_text(json.dumps(value), encoding="utf-8")
            report = aggregate_module.aggregate(root)
            self.assertEqual(report["verdict"], "STOP_INVALID")
            self.assertTrue(any("execution_state=ERROR" in error for error in report["validation_errors"]))


class RunnerContracts(unittest.TestCase):
    def run_wrapper(self, helper: Path, output: Path, state: Path | None = None) -> dict[str, Any]:
        command = [
            sys.executable,
            str(RUNNER_PATH),
            "--family",
            "robertson-ramped",
            "--kernel-arm",
            "legacy-restarted-gmres",
            "--output",
            str(output),
            "--",
            sys.executable,
            str(helper),
        ]
        if state is not None:
            command.append(str(state))
        completed = subprocess.run(command, capture_output=True, text=True, check=False)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        return json.loads(output.read_text(encoding="utf-8"))

    def test_runner_marks_deterministic_payload_complete(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            helper = root / "stable.py"
            helper.write_text("import json; print(json.dumps({'value': 1}))\n", encoding="utf-8")
            result = self.run_wrapper(helper, root / "cell.json")
            self.assertEqual(result["execution_state"], "COMPLETE")
            self.assertTrue(result["deterministic_replay"])

    def test_runner_preserves_nondeterminism_as_stop_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            helper = root / "counter.py"
            helper.write_text(
                textwrap.dedent(
                    """
                    import json, pathlib, sys
                    path = pathlib.Path(sys.argv[1])
                    value = int(path.read_text()) if path.exists() else 0
                    path.write_text(str(value + 1))
                    print(json.dumps({"value": value}))
                    """
                ),
                encoding="utf-8",
            )
            result = self.run_wrapper(helper, root / "cell.json", root / "state.txt")
            self.assertEqual(result["execution_state"], "STOP_INVALID")
            self.assertFalse(result["deterministic_replay"])

    def test_runner_preserves_command_failure_as_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            helper = root / "fail.py"
            helper.write_text("import sys; print('boom', file=sys.stderr); raise SystemExit(7)\n", encoding="utf-8")
            result = self.run_wrapper(helper, root / "cell.json")
            self.assertEqual(result["execution_state"], "ERROR")
            self.assertTrue(any("returned 7" in error for error in result["errors"]))


if __name__ == "__main__":
    unittest.main()
