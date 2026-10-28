#!/usr/bin/env python3
"""Candidate-free contracts for the Bateman local validation orchestrator."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import platform
import subprocess
import sys
import tempfile
import unittest
import datetime as dt
from concurrent.futures import ThreadPoolExecutor
from unittest import mock

REAL_DATETIME = dt.datetime

ROOT = pathlib.Path(__file__).resolve().parents[1]
RUNNER_PATH = ROOT / "tools" / "run_audit2_bateman_local_validation.py"

spec = importlib.util.spec_from_file_location("audit2_bateman_runner", RUNNER_PATH)
assert spec and spec.loader
runner = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = runner
spec.loader.exec_module(runner)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def accepted_report() -> bytes:
    plan = [
        {
            "ordinal": index,
            "scenario_id": scenario,
            "operator_case_id": operator,
            "kind": kind,
        }
        for index, (scenario, operator, kind, _) in enumerate(
            runner.EXPECTED_SCENARIOS, 1
        )
    ]
    receipts = [
        {
            "ordinal": index,
            "scenario_id": scenario,
            "operator_case_id": operator,
            "kind": kind,
            "disposition": disposition,
            "contract_satisfied": True,
        }
        for index, (scenario, operator, kind, disposition) in enumerate(
            runner.EXPECTED_SCENARIOS, 1
        )
    ]
    return (
        json.dumps(
            {
                "schema": runner.REPORT_SCHEMA,
                "claim_scope": runner.REPORT_CLAIM_SCOPE,
                "client_id": runner.CLIENT_ID,
                "authority_manifest_sha256": runner.MANIFEST_SHA256,
                "exact_verifier_sha256": runner.VERIFIER_SHA256,
                "authority_proof_sha256": runner.PROOF_SHA256,
                "scenario_plan": plan,
                "scenario_receipts": receipts,
                "all_six_executed": True,
                "all_contracts_satisfied": True,
                "terminal_failure": None,
            },
            sort_keys=True,
        )
        + "\n"
    ).encode()


def semantic_rejection_report() -> bytes:
    """A complete frozen-shape report whose candidate-authored claim fails."""
    report = json.loads(accepted_report())
    report.update(
        {
            "all_six_executed": True,
            "all_contracts_satisfied": False,
            "terminal_failure": {
                "phase": "candidate",
                "message": "frozen structured semantic rejection",
            },
        }
    )
    return (json.dumps(report, sort_keys=True) + "\n").encode()


def verified_validator_output(report: bytes | None = None) -> bytes:
    report_bytes = accepted_report() if report is None else report
    return (
        json.dumps(
            {
                "status": "LOCAL_SIX_CASE_RECEIPT_VERIFIED",
                "scenario_count": 6,
                "report_sha256": sha256_bytes(report_bytes),
                "claim_scope": runner.REPORT_CLAIM_SCOPE,
            },
            sort_keys=True,
        )
        + "\n"
    ).encode()


class FakeExecutor:
    """Writes command streams without starting Cargo or a scientific binary."""

    def __init__(self) -> None:
        self.calls: list[str] = []
        self.candidate_target_dirs: list[str] = []
        self.overrides: dict[str, tuple[int, bytes, bytes] | BaseException] = {}
        self.after_candidate = None
        self.marker_seen_before_candidate = False
        self.create_candidate_binary = True
        self.create_candidate_binary_symlink = False

    @staticmethod
    def _write(path: pathlib.Path | None, value: bytes) -> None:
        if path is None:
            return
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("xb") as stream:
            stream.write(value)
            stream.flush()

    def _default(self, command) -> tuple[int, bytes, bytes]:
        name = command.name
        if name.endswith("_head"):
            return 0, (runner.EXECUTION_SOURCE_HEAD + "\n").encode(), b""
        if name.endswith("_implementation_tree"):
            return 0, (runner.IMPLEMENTATION_TREE + "\n").encode(), b""
        if name.endswith("_tree") and "base" not in name:
            return 0, (runner.EXECUTION_SOURCE_TREE + "\n").encode(), b""
        if name.endswith("_implementation_parent"):
            return 0, (runner.IMPLEMENTATION_PARENT + "\n").encode(), b""
        if name.endswith("_parent"):
            return 0, (runner.IMPLEMENTATION_HEAD + "\n").encode(), b""
        if name.endswith("_base_tree"):
            return 0, (runner.BASE_TREE + "\n").encode(), b""
        if name.endswith("_ancestry") or name.endswith("_status"):
            return 0, b"", b""
        if name == "python_dependencies":
            return (
                0,
                json.dumps(
                    {
                        "python": runner.EXPECTED_PYTHON_VERSION,
                        "numpy": runner.EXPECTED_NUMPY_VERSION,
                        "mpmath": runner.EXPECTED_MPMATH_VERSION,
                    },
                    sort_keys=True,
                ).encode()
                + b"\n",
                b"",
            )
        if name == "rustc_version":
            return 0, b"rustc 1.94.1 (mock 2026-08-01)\n", b""
        if name == "cargo_version":
            return 0, b"cargo 1.94.1 (mock 2026-08-01)\n", b""
        if name == "authority_verification":
            return (
                0,
                json.dumps(runner.EXPECTED_AUTHORITY_VERIFICATION, sort_keys=True).encode()
                + b"\n",
                b"",
            )
        if name == "candidate":
            return 0, accepted_report(), b"candidate stderr\n"
        if name == "validator":
            report = command.argv[-1]
            report_hash = hashlib.sha256(pathlib.Path(report).read_bytes()).hexdigest()
            return (
                0,
                json.dumps(
                    {
                        "status": "LOCAL_SIX_CASE_RECEIPT_VERIFIED",
                        "scenario_count": 6,
                        "report_sha256": report_hash,
                        "claim_scope": runner.REPORT_CLAIM_SCOPE,
                    },
                    sort_keys=True,
                ).encode()
                + b"\n",
                b"",
            )
        return 0, b"ok\n", b""

    def execute(self, command) -> int:
        self.calls.append(command.name)
        if command.name == "cargo_build_candidate":
            self.candidate_target_dirs.append(command.environment["CARGO_TARGET_DIR"])
        if command.name == "candidate":
            marker = command.package_dir / "attempt_lock.json"
            self.marker_seen_before_candidate = marker.is_file()
        result = (
            self.overrides[command.name]
            if command.name in self.overrides
            else self._default(command)
        )
        if isinstance(result, BaseException):
            raise result
        exit_code, stdout, stderr = result
        self._write(command.stdout_path, stdout)
        self._write(command.stderr_path, stderr)
        if command.name == "readiness" and exit_code == 0:
            output_dir = pathlib.Path(command.environment["AUDIT2_OUTPUT_DIR"])
            for relative in (
                "solve-stiff.json",
                "solve-stiff-budget-exhausted.json",
            ):
                self._write(output_dir / relative, b"{}\n")
        if (
            command.name == "cargo_build_candidate"
            and exit_code == 0
            and self.create_candidate_binary
        ):
            binary = (
                pathlib.Path(command.environment["CARGO_TARGET_DIR"])
                / "debug/examples/audit2_bateman_local_six_case"
            )
            binary.parent.mkdir(parents=True, exist_ok=True)
            if self.create_candidate_binary_symlink:
                target = command.package_dir / "synthetic-attacker-candidate"
                target.write_bytes(b"synthetic-attacker-candidate-binary\n")
                binary.symlink_to(target)
            elif not binary.exists():
                binary.write_bytes(b"synthetic-frozen-candidate-binary\n")
        if command.name == "candidate" and self.after_candidate is not None:
            self.after_candidate()
        return exit_code


class RunnerContracts(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.source = self.root / "source"
        self.state = self.root / "state"
        self.source.mkdir()
        self.file_hashes: dict[str, str] = {}
        for index, relative in enumerate(runner.EXPECTED_FILE_HASHES, 1):
            payload = f"frozen-file-{index}\n".encode()
            path = self.source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
            self.file_hashes[relative] = sha256_bytes(payload)
        self.executables = {
            "git": "/mock/git",
            "python": "/mock/python3",
            "cargo": "/mock/cargo",
            "rustc": "/mock/rustc",
            "bash": "/mock/bash",
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_protocol(self, executor: FakeExecutor):
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.state),
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch.dict(
                runner.EXPECTED_FILE_HASHES, self.file_hashes, clear=True
            ),
            mock.patch.object(
                runner, "EXPECTED_PYTHON_VERSION", platform.python_version()
            ),
        ):
            return runner.run_protocol(
                self.source,
                executor=executor,
                executables=self.executables,
            )

    def test_success_marks_before_candidate_and_seals_every_regular_file(self) -> None:
        executor = FakeExecutor()
        outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, runner.ACCEPT_VERDICT)
        self.assertTrue(executor.marker_seen_before_candidate)
        self.assertEqual(executor.calls.count("candidate"), 1)
        self.assertEqual(executor.calls.count("validator"), 1)
        self.assertEqual(
            (outcome.package_dir / "result_summary.json").read_bytes(),
            accepted_report(),
        )
        manifest = json.loads((outcome.package_dir / "execution_manifest.json").read_bytes())
        self.assertEqual(manifest["source_pre"]["head"], runner.EXECUTION_SOURCE_HEAD)
        self.assertEqual(manifest["source_pre"]["parent"], runner.IMPLEMENTATION_HEAD)
        self.assertEqual(
            manifest["scientific_implementation_provenance"]["implementation_parent"],
            runner.IMPLEMENTATION_PARENT,
        )
        attempt = json.loads((outcome.package_dir / "attempt_lock.json").read_bytes())
        self.assertEqual(attempt["key_material"]["implementation_head"], runner.IMPLEMENTATION_HEAD)
        self.assertNotIn("execution_source_head", attempt["key_material"])

        sums_path = outcome.package_dir / "SHA256SUMS"
        lines = sums_path.read_text().splitlines()
        sealed = {line.split("  ", 1)[1] for line in lines}
        regular = {
            path.relative_to(outcome.package_dir).as_posix()
            for path in outcome.package_dir.rglob("*")
            if path.is_file() and path != sums_path
        }
        self.assertEqual(sealed, regular)
        self.assertEqual(lines, sorted(lines, key=lambda line: line.split("  ", 1)[1]))

    def test_fixed_hash_mismatch_stops_before_one_shot_marker(self) -> None:
        executor = FakeExecutor()
        first = next(iter(self.file_hashes))
        self.file_hashes[first] = "0" * 64

        outcome = self.run_protocol(executor)

        self.assertTrue(outcome.verdict.startswith("INCONCLUSIVE_"))
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_preflight_failure_stops_before_marker(self) -> None:
        executor = FakeExecutor()
        executor.overrides["rust_authority_contracts"] = (101, b"", b"failed\n")

        outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED")
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_authority_preflight_rejects_noncanonical_extra_fields(self) -> None:
        executor = FakeExecutor()
        noncanonical = dict(runner.EXPECTED_AUTHORITY_VERIFICATION)
        noncanonical["max_reference_l2_bound"] = 999.0
        executor.overrides["authority_verification"] = (
            0,
            (json.dumps(noncanonical, sort_keys=True) + "\n").encode(),
            b"",
        )

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED"
        )
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_preexisting_candidate_symlink_is_removed_before_build(self) -> None:
        run_id = "20260831T000000Z-deadbeef"
        binary = (
            self.state
            / "build-cache"
            / runner.EXECUTION_SOURCE_HEAD
            / run_id
            / "debug/examples/audit2_bateman_local_six_case"
        )
        binary.parent.mkdir(parents=True)
        stale = self.root / "stale-candidate"
        stale.write_bytes(b"stale-candidate-binary\n")
        binary.symlink_to(stale)

        class FrozenClock:
            @classmethod
            def now(cls, timezone):
                return REAL_DATETIME(2026, 8, 31, tzinfo=timezone)

        executor = FakeExecutor()
        with (
            mock.patch.object(runner.dt, "datetime", FrozenClock),
            mock.patch.object(runner.secrets, "token_hex", return_value="deadbeef"),
        ):
            outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, runner.ACCEPT_VERDICT)
        self.assertFalse(binary.is_symlink())
        self.assertEqual(binary.read_bytes(), b"synthetic-frozen-candidate-binary\n")
        self.assertEqual(executor.candidate_target_dirs, [str(binary.parents[2])])

    def test_preexisting_candidate_directory_stops_before_build_and_marker(self) -> None:
        run_id = "20260831T000000Z-deadbeef"
        binary = (
            self.state
            / "build-cache"
            / runner.EXECUTION_SOURCE_HEAD
            / run_id
            / "debug/examples/audit2_bateman_local_six_case"
        )
        binary.mkdir(parents=True)
        executor = FakeExecutor()

        class FrozenClock:
            @classmethod
            def now(cls, timezone):
                return REAL_DATETIME(2026, 8, 31, tzinfo=timezone)

        with (
            mock.patch.object(runner.dt, "datetime", FrozenClock),
            mock.patch.object(runner.secrets, "token_hex", return_value="deadbeef"),
        ):
            outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
        )
        self.assertNotIn("cargo_build_candidate", executor.calls)
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_candidate_build_symlink_output_stops_before_marker(self) -> None:
        executor = FakeExecutor()
        executor.create_candidate_binary_symlink = True

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
        )
        self.assertIn("cargo_build_candidate", executor.calls)
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_failed_source_status_query_stops_before_marker(self) -> None:
        executor = FakeExecutor()
        executor.overrides["source_pre_status"] = (1, b"", b"git status failed\n")

        outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_SOURCE_IDENTITY_UNRESOLVED")
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_validator_exit_one_rejects_only_eligible_structured_candidate_failure(self) -> None:
        executor = FakeExecutor()
        rejected = semantic_rejection_report()
        executor.overrides["candidate"] = (7, rejected, b"candidate failed\n")
        executor.overrides["validator"] = (1, b"", b"receipt rejected\n")

        outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, runner.REJECT_VERDICT)
        self.assertEqual(executor.calls.count("candidate"), 1)
        self.assertEqual(executor.calls.count("validator"), 1)
        self.assertTrue(runner.validate_report_shape(json.loads(rejected)))
        self.assertEqual((outcome.package_dir / "result_summary.json").read_bytes(), rejected)
        self.assertEqual(
            (outcome.package_dir / "logs" / "validator.stderr.log").read_bytes(),
            b"receipt rejected\n",
        )

    def test_validator_exit_one_cannot_reject_an_unparseable_candidate_report(self) -> None:
        executor = FakeExecutor()
        executor.overrides["candidate"] = (7, b"{not-json\n", b"candidate failed\n")
        executor.overrides["validator"] = (1, b"", b"receipt rejected\n")

        outcome = self.run_protocol(executor)

        self.assertTrue(outcome.verdict.startswith("INCONCLUSIVE_"))
        self.assertNotEqual(outcome.verdict, runner.REJECT_VERDICT)

    def test_validator_spawn_oserror_is_infrastructure_inconclusive(self) -> None:
        executor = FakeExecutor()
        executor.overrides["validator"] = OSError("frozen validator unavailable")

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )

    def test_validator_exit_127_is_infrastructure_inconclusive(self) -> None:
        executor = FakeExecutor()
        executor.overrides["validator"] = (127, b"", b"spawn failure\n")

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )

    def test_validator_exit_zero_empty_stdout_is_infrastructure_inconclusive(self) -> None:
        executor = FakeExecutor()
        executor.overrides["validator"] = (0, b"", b"")

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )

    def test_validator_exit_zero_malformed_stdout_is_infrastructure_inconclusive(self) -> None:
        executor = FakeExecutor()
        executor.overrides["validator"] = (0, b"{not-json\n", b"")

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )

    def test_validator_exit_zero_nonempty_stderr_is_infrastructure_inconclusive(self) -> None:
        executor = FakeExecutor()
        executor.overrides["validator"] = (
            0,
            verified_validator_output(),
            b"unexpected validator diagnostic\n",
        )

        outcome = self.run_protocol(executor)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )

    def test_source_mutation_after_candidate_downgrades_to_inconclusive(self) -> None:
        executor = FakeExecutor()
        target = self.source / next(iter(self.file_hashes))
        executor.after_candidate = lambda: target.write_bytes(b"mutated\n")

        outcome = self.run_protocol(executor)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_SOURCE_CHANGED_DURING_EXECUTION")
        self.assertEqual(executor.calls.count("candidate"), 1)
        self.assertEqual(executor.calls.count("validator"), 1)

    def test_second_run_cannot_invoke_candidate_again(self) -> None:
        first_executor = FakeExecutor()
        first = self.run_protocol(first_executor)
        self.assertEqual(first.verdict, runner.ACCEPT_VERDICT)

        second_executor = FakeExecutor()
        second = self.run_protocol(second_executor)

        self.assertEqual(second.verdict, "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED")
        self.assertNotIn("candidate", second_executor.calls)

    def test_candidate_spawn_failure_still_consumes_one_shot_guard(self) -> None:
        first_executor = FakeExecutor()
        first_executor.overrides["candidate"] = OSError("synthetic spawn failure")
        first = self.run_protocol(first_executor)
        self.assertTrue(first.verdict.startswith("INCONCLUSIVE_"))
        self.assertEqual(first_executor.calls.count("candidate"), 1)

        second_executor = FakeExecutor()
        second = self.run_protocol(second_executor)
        self.assertEqual(second.verdict, "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED")
        self.assertNotIn("candidate", second_executor.calls)

    def test_concurrent_runners_contend_on_one_canonical_guard(self) -> None:
        executors = (FakeExecutor(), FakeExecutor())
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.state),
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch.dict(runner.EXPECTED_FILE_HASHES, self.file_hashes, clear=True),
            mock.patch.object(runner, "EXPECTED_PYTHON_VERSION", platform.python_version()),
        ):
            with ThreadPoolExecutor(max_workers=2) as pool:
                outcomes = list(
                    pool.map(
                        lambda executor: runner.run_protocol(
                            self.source,
                            executor=executor,
                            executables=self.executables,
                        ),
                        executors,
                    )
                )
        self.assertEqual(sum(item.calls.count("candidate") for item in executors), 1)
        self.assertEqual(
            sorted(outcome.verdict for outcome in outcomes),
            sorted([runner.ACCEPT_VERDICT, "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED"]),
        )

    def test_concurrent_runners_use_distinct_build_targets(self) -> None:
        executors = (FakeExecutor(), FakeExecutor())
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.state),
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch.dict(runner.EXPECTED_FILE_HASHES, self.file_hashes, clear=True),
            mock.patch.object(runner, "EXPECTED_PYTHON_VERSION", platform.python_version()),
        ):
            with ThreadPoolExecutor(max_workers=2) as pool:
                list(
                    pool.map(
                        lambda executor: runner.run_protocol(
                            self.source,
                            executor=executor,
                            executables=self.executables,
                        ),
                        executors,
                    )
                )
        self.assertEqual(
            len({directory for executor in executors for directory in executor.candidate_target_dirs}),
            2,
        )

    def test_symlinked_canonical_state_component_is_rejected(self) -> None:
        target = self.root / "real-state"
        target.mkdir()
        self.state.symlink_to(target, target_is_directory=True)
        executor = FakeExecutor()
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.state),
            mock.patch.dict(runner.EXPECTED_FILE_HASHES, self.file_hashes, clear=True),
        ):
            with self.assertRaises(ValueError):
                runner.run_protocol(
                    self.source,
                    executor=executor,
                    executables=self.executables,
                )
        self.assertEqual(executor.calls, [])

    def test_ambient_build_override_stops_before_any_candidate(self) -> None:
        executor = FakeExecutor()
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.state),
            mock.patch.dict(
                runner.EXPECTED_FILE_HASHES, self.file_hashes, clear=True
            ),
            mock.patch.dict(
                os.environ,
                {"RUSTFLAGS": "--cfg outcome_dependent"},
                clear=True,
            ),
        ):
            outcome = runner.run_protocol(
                self.source,
                executor=executor,
                executables=self.executables,
            )
        self.assertEqual(outcome.verdict, "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED")
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((self.state / "one-shot").exists())

    def test_plan_has_no_remote_write_holdout_or_formal_tool_commands(self) -> None:
        text = "\n".join(
            " ".join(template.argv) for template in runner.PREFLIGHT_COMMANDS
        ).lower()
        for forbidden in (
            "push",
            "fetch",
            "pull",
            "clone",
            "holdout",
            "wolfram",
            "xact",
            "sage",
            "singular",
            "lean",
            "lake",
            "rocq",
            "coqc",
        ):
            self.assertNotIn(forbidden, text)

    def test_cli_rejects_scientific_override_flags(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(RUNNER_PATH),
                "--source-worktree",
                str(self.source),
                "--output-atol",
                "1",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn(b"unrecognized arguments", completed.stderr)

    def test_canonical_state_root_cannot_be_overridden_by_api_or_cli(self) -> None:
        with self.assertRaises(TypeError):
            runner.run_protocol(
                self.source,
                state_root=self.root / "attacker-selected-state",
                executor=FakeExecutor(),
                executables=self.executables,
            )

        completed = subprocess.run(
            [
                sys.executable,
                str(RUNNER_PATH),
                "--source-worktree",
                str(self.source),
                "--state-root",
                str(self.root / "attacker-selected-state"),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn(b"unrecognized arguments", completed.stderr)


if __name__ == "__main__":
    unittest.main()
