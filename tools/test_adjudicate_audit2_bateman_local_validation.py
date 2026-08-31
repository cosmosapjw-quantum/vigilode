#!/usr/bin/env python3
"""Candidate-free contracts for independent Bateman package adjudication.

The tests keep package parsing and sealing real.  Only the two boundaries that
cannot be reproduced safely in a unit test are replaced: Git object identity
queries and execution of the already-frozen receipt validator.  No test starts
Cargo or a scientific candidate.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
ADJUDICATOR_PATH = ROOT / "tools" / "adjudicate_audit2_bateman_local_validation.py"

spec = importlib.util.spec_from_file_location("audit2_bateman_adjudicator", ADJUDICATOR_PATH)
assert spec and spec.loader
adjudicator = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = adjudicator
spec.loader.exec_module(adjudicator)

RUNNER_TEST_PATH = ROOT / "tools" / "test_audit2_bateman_local_validation_runner.py"
runner_test_spec = importlib.util.spec_from_file_location(
    "audit2_bateman_runner_contract_fixture", RUNNER_TEST_PATH
)
assert runner_test_spec and runner_test_spec.loader
runner_contracts = importlib.util.module_from_spec(runner_test_spec)
sys.modules[runner_test_spec.name] = runner_contracts
runner_test_spec.loader.exec_module(runner_contracts)
runner = runner_contracts.runner


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class FrozenBoundary:
    """Fake read-only Git and validator boundary; it never invokes a candidate."""

    def __init__(
        self,
        *,
        validator_exit: int = 0,
        validator_stderr: bytes = b"",
        validator_error: OSError | None = None,
        real_validator: bool = False,
    ) -> None:
        self.validator_exit = validator_exit
        self.validator_stderr = validator_stderr
        self.validator_error = validator_error
        self.real_validator = real_validator
        self.validator_calls = 0
        self.git_calls = 0
        self.forbidden_executable_paths: set[str] = set()
        self.forbidden_executable_calls = 0

    def __call__(self, argv, *, cwd, env=None):
        executable = str(pathlib.Path(argv[0]).resolve())
        if executable in self.forbidden_executable_paths:
            self.forbidden_executable_calls += 1
            return subprocess.CompletedProcess(
                argv, 126, b"", b"package-selected executable blocked\n"
            )
        if pathlib.Path(argv[0]).name == "git":
            self.git_calls += 1
            query = tuple(argv[1:])
            if query == ("rev-parse", "HEAD^{tree}"):
                stdout = adjudicator.EXECUTION_SOURCE_TREE + "\n"
            elif query == ("rev-parse", "HEAD"):
                stdout = adjudicator.EXECUTION_SOURCE_HEAD + "\n"
            elif query == ("rev-parse", "HEAD^"):
                stdout = adjudicator.IMPLEMENTATION_HEAD + "\n"
            elif query == (
                "rev-parse",
                f"{adjudicator.IMPLEMENTATION_HEAD}^{{tree}}",
            ):
                stdout = adjudicator.IMPLEMENTATION_TREE + "\n"
            elif query == (
                "rev-parse",
                f"{adjudicator.IMPLEMENTATION_HEAD}^",
            ):
                stdout = adjudicator.BASE_HEAD + "\n"
            elif query == ("rev-parse", f"{adjudicator.BASE_HEAD}^{{tree}}"):
                stdout = adjudicator.BASE_TREE + "\n"
            elif query == ("rev-parse", "--show-toplevel"):
                stdout = str(pathlib.Path(cwd).resolve()) + "\n"
            elif query in {
                (
                    "merge-base",
                    "--is-ancestor",
                    adjudicator.IMPLEMENTATION_HEAD,
                    adjudicator.EXECUTION_SOURCE_HEAD,
                ),
                (
                    "merge-base",
                    "--is-ancestor",
                    adjudicator.BASE_HEAD,
                    adjudicator.IMPLEMENTATION_HEAD,
                ),
                ("status", "--porcelain=v1", "--untracked-files=all"),
            }:
                stdout = ""
            else:  # pragma: no cover - a new Git query must be added explicitly.
                raise AssertionError(f"unexpected Git query: {argv!r}")
            return subprocess.CompletedProcess(argv, 0, stdout.encode(), b"")

        self.validator_calls += 1
        if self.validator_error is not None:
            raise self.validator_error
        if self.real_validator:
            return subprocess.run(
                list(argv),
                cwd=cwd,
                env=None if env is None else dict(env),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
        report = pathlib.Path(argv[-1]).read_bytes()
        payload = {
            "status": "LOCAL_SIX_CASE_RECEIPT_VERIFIED",
            "scenario_count": 6,
            "report_sha256": sha256_bytes(report),
            "claim_scope": adjudicator.REPORT_CLAIM_SCOPE,
        }
        stdout = json.dumps(payload, sort_keys=True).encode() + b"\n"
        return subprocess.CompletedProcess(
            argv, self.validator_exit, stdout if self.validator_exit == 0 else b"", self.validator_stderr
        )


class AdjudicatorContracts(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.source = self.root / "source"
        self.package = self.root / "package"
        self.source.mkdir()
        self.package.mkdir()
        self.synthetic_hashes: dict[str, str] = {}
        for index, relative in enumerate(adjudicator.EXPECTED_SOURCE_HASHES, 1):
            payload = f"frozen-source-{index}\n".encode()
            path = self.source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
            self.synthetic_hashes[relative] = sha256_bytes(payload)
        self.tool_root = self.root / "tools"
        self.tool_root.mkdir()
        self.executables = {"python": str(pathlib.Path(sys.executable).resolve())}
        for name in ("git", "cargo", "rustc", "bash"):
            path = self.tool_root / name
            path.write_bytes(f"synthetic-{name}\n".encode())
            self.executables[name] = str(path)
        self.controller = self.tool_root / "run_audit2_bateman_local_validation.py"
        self.controller.write_bytes(b"synthetic-frozen-controller\n")
        self.controller_hash = sha256_bytes(self.controller.read_bytes())
        self.run_id = "synthetic-run"
        self.cargo_target_dir = (
            self.root
            / "state"
            / "build-cache"
            / adjudicator.EXECUTION_SOURCE_HEAD
            / self.run_id
        )
        self.candidate_binary = (
            self.cargo_target_dir
            / "debug"
            / "examples"
            / "audit2_bateman_local_six_case"
        )
        self.candidate_binary.parent.mkdir(parents=True)
        self.candidate_binary.write_bytes(b"synthetic-frozen-candidate-binary\n")
        self.environment = {
            **adjudicator.FIXED_ENVIRONMENT,
            "HOME": str(self.root),
            "CARGO_HOME": str(self.root / ".cargo"),
            "PATH": ":".join(
                dict.fromkeys(
                    [
                        str(pathlib.Path(value).parent)
                        for value in self.executables.values()
                    ]
                    + ["/usr/bin", "/bin"]
                )
            ),
            "CARGO_TARGET_DIR": str(self.cargo_target_dir),
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def accepted_report(self) -> bytes:
        report = {
            "schema": adjudicator.REPORT_SCHEMA,
            "claim_scope": adjudicator.REPORT_CLAIM_SCOPE,
            "client_id": adjudicator.CLIENT_ID,
            "authority_manifest_sha256": self.synthetic_hashes[
                "research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
            ],
            "exact_verifier_sha256": self.synthetic_hashes[
                "research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
            ],
            "authority_proof_sha256": self.synthetic_hashes[
                "research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
            ],
            "scenario_plan": [
                {
                    "ordinal": index,
                    "scenario_id": scenario,
                    "operator_case_id": operator_case,
                    "kind": kind,
                }
                for index, (scenario, operator_case, kind, _) in enumerate(
                    adjudicator.EXPECTED_SCENARIOS, 1
                )
            ],
            "scenario_receipts": [
                {
                    "ordinal": index,
                    "scenario_id": scenario,
                    "operator_case_id": operator_case,
                    "kind": kind,
                    "disposition": disposition,
                    "contract_satisfied": True,
                }
                for index, (scenario, operator_case, kind, disposition) in enumerate(
                    adjudicator.EXPECTED_SCENARIOS, 1
                )
            ],
            "all_six_executed": True,
            "all_contracts_satisfied": True,
            "terminal_failure": None,
        }
        return json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n"

    def accepted_authority_verification(self) -> dict:
        return {
            "candidate_executions": 0,
            "declared_reference_l2_uncertainty": 1e-15,
            "execution_scenarios": 6,
            "fast_exponent_exceeds_one": True,
            "holdout_access": "NOT_OPENED_OR_EXECUTED",
            "local_six_case_status": "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION",
            "max_reference_l2_bound": 2.075243427511439e-17,
            "receipt_sha256": self.synthetic_hashes[
                "research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
            ],
            "status": "AUTHORITY_CONSTRUCTION_VERIFIED",
            "verified_operator_cases": 2,
        }

    def frozen_validator_canonical_report(self) -> bytes:
        """Build the validator's complete candidate-free canonical fixture."""

        for relative in tuple(self.synthetic_hashes):
            frozen = ROOT / relative
            if frozen.is_file():
                payload = frozen.read_bytes()
                (self.source / relative).write_bytes(payload)
                self.synthetic_hashes[relative] = sha256_bytes(payload)

        fixture_path = ROOT / "tools" / "test_audit2_bateman_local_receipt.py"
        fixture_spec = importlib.util.spec_from_file_location(
            "audit2_bateman_receipt_fixture_for_adjudication", fixture_path
        )
        assert fixture_spec and fixture_spec.loader
        fixture = importlib.util.module_from_spec(fixture_spec)
        fixture_spec.loader.exec_module(fixture)
        return (
            json.dumps(
                fixture.canonical_report(), sort_keys=True, separators=(",", ":")
            ).encode()
            + b"\n"
        )

    def _command_rows(self, report_hash: str) -> list[dict]:
        del report_hash
        rows = adjudicator.expected_command_contract(
            self.source, self.package, self.executables, self.environment
        )
        for ordinal, row in enumerate(rows, 1):
            row.update(
                {
                    "ordinal": ordinal,
                    "started_at_utc": f"2026-08-31T00:00:{ordinal:02d}Z",
                    "finished_at_utc": f"2026-08-31T00:01:{ordinal:02d}Z",
                    "launch_status": "LAUNCHED",
                    "exit_code": 0,
                }
            )
        return rows

    def build_package(
        self,
        *,
        report: bytes | None = None,
        candidate_exit: int = 0,
        runner_validator_exit: int = 0,
    ) -> None:
        report = self.accepted_report() if report is None else report
        report_hash = sha256_bytes(report)
        commands = self._command_rows(report_hash)
        candidate_row = next(row for row in commands if row["name"] == "candidate")
        validator_row = next(row for row in commands if row["name"] == "validator")
        candidate_row["exit_code"] = candidate_exit
        validator_row["exit_code"] = runner_validator_exit

        source_identity = adjudicator.expected_source_identity()
        for row in commands:
            stdout = b"ok\n"
            stderr = b""
            if row["phase"] == "source":
                prefix = next(
                    prefix
                    for prefix in ("source_prelaunch", "source_pre", "source_post")
                    if row["name"].startswith(prefix + "_")
                )
                key = row["name"].removeprefix(prefix + "_")
                value = source_identity[key]
                stdout = (
                    b""
                    if key.endswith("ancestry") or key == "status"
                    else f"{value}\n".encode()
                )
            elif row["name"] == "candidate":
                stdout = report
                stderr = b"candidate stderr\n"
            elif row["name"] == "authority_verification":
                stdout = (
                    json.dumps(
                        self.accepted_authority_verification(), sort_keys=True
                    ).encode()
                    + b"\n"
                )
            elif row["name"] == "validator":
                stdout = (
                    json.dumps(
                        {
                            "status": "LOCAL_SIX_CASE_RECEIPT_VERIFIED",
                            "scenario_count": 6,
                            "report_sha256": report_hash,
                            "claim_scope": adjudicator.REPORT_CLAIM_SCOPE,
                        },
                        sort_keys=True,
                    ).encode()
                    + b"\n"
                    if runner_validator_exit == 0
                    else b""
                )
                stderr = b"" if runner_validator_exit == 0 else b"receipt rejected\n"
            for key, value in (("stdout_path", stdout), ("stderr_path", stderr)):
                path = self.package / row[key]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(value)
                row[f"{key.removesuffix('_path')}_sha256"] = sha256_bytes(value)
        for relative in (
            "readiness-output/solve-stiff.json",
            "readiness-output/solve-stiff-budget-exhausted.json",
        ):
            path = self.package / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"{}\n")

        key_material = {
                "implementation_head": adjudicator.IMPLEMENTATION_HEAD,
                "authority_manifest_sha256": self.synthetic_hashes[
                    "research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
                ],
                "example_target": "audit2_bateman_local_six_case",
                "protocol_version": adjudicator.PROTOCOL_VERSION,
        }
        guard_hash = sha256_bytes(
            json.dumps(key_material, sort_keys=True, separators=(",", ":")).encode()
        )
        attempt = {
            "schema": "vigilode-audit2-bateman-one-shot-lock/v1",
            "protocol_version": adjudicator.PROTOCOL_VERSION,
            "key_sha256": guard_hash,
            "key_material": key_material,
            "created_at_utc": "2026-08-31T00:00:00Z",
            "run_id": self.run_id,
        }
        (self.package / "attempt_lock.json").write_text(
            json.dumps(attempt, sort_keys=True) + "\n", encoding="utf-8"
        )
        (self.package / "candidate_launch.json").write_text(
            json.dumps(
                {**attempt, "state": "CANDIDATE_LAUNCH_COMMITTED"}, sort_keys=True
            )
            + "\n",
            encoding="utf-8",
        )

        validator_stdout = (self.package / "local_receipt_validation.json").read_bytes()
        validator_stderr = (self.package / "logs/validator.stderr.log").read_bytes()
        validator_attempt = {
            "schema": "vigilode-audit2-bateman-validator-attempt/v1",
            "attempted": True,
            "attempt_count": 1,
            "launch_status": "LAUNCHED",
            "exit_code": runner_validator_exit,
            "argv": validator_row["argv"],
            "stdout_path": validator_row["stdout_path"],
            "stderr_path": validator_row["stderr_path"],
            "stdout_sha256": sha256_bytes(validator_stdout),
            "stderr_sha256": sha256_bytes(validator_stderr),
        }
        (self.package / "validator_attempt.json").write_text(
            json.dumps(validator_attempt, sort_keys=True) + "\n", encoding="utf-8"
        )

        try:
            report_object = json.loads(report)
        except (json.JSONDecodeError, UnicodeDecodeError):
            report_object = None
        if (
            isinstance(report_object, dict)
            and report_object.get("all_six_executed") is True
            and (
                report_object.get("all_contracts_satisfied") is False
                or report_object.get("terminal_failure") is not None
            )
            and runner_validator_exit == 1
        ):
            runner_verdict = adjudicator.REJECT_VERDICT
        elif runner_validator_exit == 127:
            runner_verdict = "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        elif runner_validator_exit == 0 and candidate_exit == 0:
            runner_verdict = adjudicator.RUNNER_ACCEPT_VERDICT
        else:
            runner_verdict = "INCONCLUSIVE_LAUNCH_OR_REPORT"
        binary_hash = sha256_bytes(self.candidate_binary.read_bytes())
        runner_path = self.controller
        manifest = {
            "schema": "vigilode-audit2-bateman-local-execution-manifest/v1",
            "verdict": runner_verdict,
            "protocol_version": adjudicator.PROTOCOL_VERSION,
            "run_id": self.run_id,
            "utc_start": "2026-08-31T00:00:00Z",
            "utc_end": "2026-08-31T00:02:00Z",
            "source_pre": source_identity,
            "source_prelaunch": source_identity,
            "source_post": source_identity,
            "scientific_implementation_provenance": (
                adjudicator.SCIENTIFIC_IMPLEMENTATION_PROVENANCE
            ),
            "source_hashes": {
                relative: {"expected": digest, "observed": digest, "match": True}
                for relative, digest in self.synthetic_hashes.items()
            },
            "tool_versions": {
                "executables": {
                    name: {
                        "path": path,
                        "realpath": str(pathlib.Path(path).resolve()),
                        "sha256": sha256_bytes(pathlib.Path(path).resolve().read_bytes()),
                        "size_bytes": pathlib.Path(path).resolve().stat().st_size,
                    }
                    for name, path in sorted(self.executables.items())
                },
                "platform": {
                    "system": "Synthetic",
                    "release": "test",
                    "machine": "test",
                    "python_runtime": adjudicator.EXPECTED_PYTHON_VERSION,
                },
                "controller": {
                    "path": str(runner_path),
                    "sha256_pre": self.controller_hash,
                    "sha256_post": self.controller_hash,
                },
                "environment_policy": {
                    "mode": "SANITIZED_ALLOWLIST_PLUS_FIXED_VALUES",
                    "forbidden_ambient_detected": [],
                    "cargo_config_files_present": [],
                    "effective": self.environment,
                },
                "python": {
                    "python": adjudicator.EXPECTED_PYTHON_VERSION,
                    "numpy": adjudicator.EXPECTED_NUMPY_VERSION,
                    "mpmath": adjudicator.EXPECTED_MPMATH_VERSION,
                },
                "rustc_version": "rustc 1.94.1 (synthetic 2026-08-31)",
                "cargo_version": "cargo 1.94.1 (synthetic 2026-08-31)",
                "candidate_binary": {
                    "path": str(self.candidate_binary),
                    "sha256_prelaunch": binary_hash,
                    "sha256_post": binary_hash,
                    "size_bytes": self.candidate_binary.stat().st_size,
                },
            },
            "authority_preflight": self.accepted_authority_verification(),
            "candidate": {
                "invocation_count": 1,
                "launch_status": "LAUNCHED",
                "exit_code": candidate_exit,
                "argv": candidate_row["argv"],
                "result_summary_path": "result_summary.json",
                "result_summary_sha256": report_hash,
                "stderr_path": "logs/candidate.stderr.log",
                "guard_key_sha256": guard_hash,
                "binary_sha256_prelaunch": binary_hash,
                "binary_sha256_post": binary_hash,
            },
            "validator": {
                "attempt_count": 1,
                "launch_status": "LAUNCHED",
                "exit_code": runner_validator_exit,
                "argv": validator_row["argv"],
                "stdout_path": "local_receipt_validation.json",
                "stdout_sha256": sha256_bytes(validator_stdout),
                "stderr_path": "logs/validator.stderr.log",
                "stderr_sha256": sha256_bytes(validator_stderr),
                "parsed_status": (
                    "LOCAL_SIX_CASE_RECEIPT_VERIFIED" if runner_validator_exit == 0 else None
                ),
            },
            "pre_candidate_stop": None,
            "commands": commands,
            "declarations": {
                "execution_policy": "HOST_CODEX_ONLY",
                "local_llm_used": False,
                "local_llm_used_attestation": "SELF_DECLARED_NOT_HOST_ATTESTED",
                "holdout_access": "NOT_OPENED_OR_EXECUTED",
                "holdout_access_attestation": (
                    "SELF_DECLARED_AND_ABSENT_FROM_FIXED_COMMAND_PLAN"
                ),
                "remote_write": "NOT_PERFORMED_BY_RUNNER_COMMAND_PLAN",
                "remote_write_attestation": (
                    "FIXED_COMMAND_PLAN_ONLY_NOT_HOST_WIDE_ATTESTATION"
                ),
                "claim_ceiling": adjudicator.GLOBAL_CLAIM_CEILING,
            },
            "math_blocker_coverage": adjudicator.expected_math_blocker_coverage(
                runner_verdict
                in {adjudicator.RUNNER_ACCEPT_VERDICT, adjudicator.REJECT_VERDICT}
            ),
            "package": {"sha256sums_path": "SHA256SUMS"},
        }
        (self.package / "execution_manifest.json").write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        (self.package / "authority_bundle_sha256.txt").write_text(
            "".join(
                f"{digest}  {relative}\n"
                for relative, digest in sorted(self.synthetic_hashes.items())
            ),
            encoding="utf-8",
        )
        (self.package / "authority_verification.json").write_text(
            json.dumps(self.accepted_authority_verification(), sort_keys=True)
            + "\n",
            encoding="utf-8",
        )
        events = [
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 1,
                "event": "run_created",
                "utc": "2026-08-31T00:00:00Z",
                "run_id": "synthetic-run",
            }
        ]
        for row in commands:
            events.extend(
                [
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 1,
                        "event": "command_started",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["started_at_utc"],
                        "argv": row["argv"],
                    },
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 2,
                        "event": "command_finished",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["finished_at_utc"],
                        "launch_status": row["launch_status"],
                        "exit_code": row["exit_code"],
                    },
                ]
            )
        events.append(
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": len(events) + 1,
                "event": "sealing_started",
                "utc": "2026-08-31T00:02:00Z",
                "verdict": manifest["verdict"],
            }
        )
        (self.package / "events.jsonl").write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in events),
            encoding="utf-8",
        )
        self.seal()

    def build_not_run_preflight_package(self, failed_name: str) -> None:
        """Convert the full fixture into a sealed, genuine executed prefix."""
        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        full_commands = manifest["commands"]
        failed_index = next(
            index for index, row in enumerate(full_commands) if row["name"] == failed_name
        )
        retained = full_commands[: failed_index + 1]
        retained[-1]["exit_code"] = 101
        for row in full_commands[failed_index + 1 :]:
            for key in ("stdout_path", "stderr_path"):
                path = self.package / row[key]
                if path.exists():
                    path.unlink()
        for relative in ("attempt_lock.json", "candidate_launch.json"):
            path = self.package / relative
            if path.exists():
                path.unlink()

        manifest["verdict"] = "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED"
        manifest["source_prelaunch"] = None
        manifest["source_post"] = None
        manifest["candidate"] = {
            "invocation_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
            "argv": list(adjudicator.CANDIDATE_ARGV),
        }
        manifest["validator"] = {
            "attempt_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
        }
        manifest["commands"] = retained
        failed = retained[-1]
        manifest["pre_candidate_stop"] = {
            "runner_verdict": manifest["verdict"],
            "reached_ordinal": len(retained),
            "terminal_command": failed["name"],
            "terminal_phase": failed["phase"],
            "terminal_launch_status": failed["launch_status"],
            "terminal_exit_code": failed["exit_code"],
            "failed_commands": [
                {
                    "ordinal": failed["ordinal"],
                    "name": failed["name"],
                    "launch_status": failed["launch_status"],
                    "exit_code": failed["exit_code"],
                }
            ],
            "kind": "COMMAND_FAILURE",
        }
        manifest["tool_versions"].pop("candidate_binary", None)
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        validator_attempt = {
            "schema": "vigilode-audit2-bateman-validator-attempt/v1",
            "attempted": False,
            "attempt_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
            "argv": None,
            "stdout_path": None,
            "stderr_path": None,
            "stdout_sha256": None,
            "stderr_sha256": None,
        }
        (self.package / "validator_attempt.json").write_text(
            json.dumps(validator_attempt, sort_keys=True) + "\n", encoding="utf-8"
        )
        events = [
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 1,
                "event": "run_created",
                "utc": manifest["utc_start"],
                "run_id": manifest["run_id"],
            }
        ]
        for row in retained:
            events.extend(
                [
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 1,
                        "event": "command_started",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["started_at_utc"],
                        "argv": row["argv"],
                    },
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 2,
                        "event": "command_finished",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["finished_at_utc"],
                        "launch_status": row["launch_status"],
                        "exit_code": row["exit_code"],
                    },
                ]
            )
        events.append(
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": len(events) + 1,
                "event": "sealing_started",
                "utc": manifest["utc_end"],
                "verdict": manifest["verdict"],
            }
        )
        (self.package / "events.jsonl").write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in events),
            encoding="utf-8",
        )
        self.seal()

    def seal(self) -> None:
        sums = self.package / "SHA256SUMS"
        if sums.exists():
            sums.unlink()
        files = sorted(
            path.relative_to(self.package).as_posix()
            for path in self.package.rglob("*")
            if path.is_file()
        )
        sums.write_text(
            "".join(
                f"{sha256_bytes((self.package / relative).read_bytes())}  {relative}\n"
                for relative in files
            ),
            encoding="ascii",
        )

    def reseal(self, package: pathlib.Path) -> None:
        previous = self.package
        self.package = package
        try:
            self.seal()
        finally:
            self.package = previous

    def mutate_manifest(self, mutation) -> None:
        path = self.package / "execution_manifest.json"
        manifest = json.loads(path.read_bytes())
        mutation(manifest)
        path.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
        self.seal()

    def command(self, manifest: dict, name: str) -> dict:
        return next(row for row in manifest["commands"] if row["name"] == name)

    def adjudicate(self, boundary: FrozenBoundary | None = None):
        boundary = boundary or FrozenBoundary()
        out = self.root / f"sidecar-{len(list(self.root.glob('sidecar-*')))}"
        with (
            mock.patch.dict(
                adjudicator.EXPECTED_SOURCE_HASHES, self.synthetic_hashes, clear=True
            ),
            mock.patch.object(
                adjudicator, "EXPECTED_RUNNER_SHA256", self.controller_hash
            ),
            mock.patch.object(
                adjudicator, "CANONICAL_STATE_ROOT", self.root / "state"
            ),
            mock.patch.object(
                adjudicator.platform,
                "python_version",
                return_value=adjudicator.EXPECTED_PYTHON_VERSION,
            ),
        ):
            result = adjudicator.adjudicate_package(
                self.package,
                self.source,
                out,
                command_runner=boundary,
            )
        return result, out, boundary

    def run_candidate_free_prefix(
        self,
        overrides: dict[str, tuple[int, bytes, bytes] | BaseException],
        *,
        ambient_environment: dict[str, str] | None = None,
        create_candidate_binary: bool = True,
    ):
        executor = runner_contracts.FakeExecutor()
        executor.create_candidate_binary = create_candidate_binary
        executor.overrides.update(overrides)
        expected_authority = adjudicator.expected_authority_verification(
            self.synthetic_hashes
        )
        full_authority = (
            json.dumps(expected_authority, sort_keys=True).encode()
            + b"\n"
        )
        executor.overrides.setdefault(
            "authority_verification", (0, full_authority, b"")
        )
        with (
            mock.patch.object(runner, "CANONICAL_STATE_ROOT", self.root / "state"),
            mock.patch.dict(
                os.environ, ambient_environment or {}, clear=True
            ),
            mock.patch.dict(
                runner.EXPECTED_FILE_HASHES, self.synthetic_hashes, clear=True
            ),
            mock.patch.object(
                runner,
                "MANIFEST_SHA256",
                self.synthetic_hashes[adjudicator.MANIFEST_PATH],
            ),
            mock.patch.object(
                runner,
                "EXPECTED_AUTHORITY_VERIFICATION",
                expected_authority,
            ),
            mock.patch.object(
                runner, "EXPECTED_PYTHON_VERSION", adjudicator.EXPECTED_PYTHON_VERSION
            ),
        ):
            outcome = runner.run_protocol(
                self.source,
                executor=executor,
                executables=self.executables,
            )
        return outcome, executor

    def adjudicate_runner_package(self, package: pathlib.Path):
        previous_package = self.package
        previous_controller_hash = self.controller_hash
        self.package = package
        self.controller_hash = sha256_bytes(runner_contracts.RUNNER_PATH.read_bytes())
        try:
            return self.adjudicate()
        finally:
            self.package = previous_package
            self.controller_hash = previous_controller_hash

    def test_synthetic_fixture_is_ci_python_version_independent(self) -> None:
        """A synthetic sealed package reaches its fake validator under CI Python."""

        self.build_package()
        with mock.patch.object(
            adjudicator.platform, "python_version", return_value="3.13.15"
        ):
            result, _, boundary = self.adjudicate(FrozenBoundary())

        self.assertEqual(result["verdict"], adjudicator.ACCEPT_VERDICT)
        self.assertEqual(boundary.validator_calls, 1)

    def test_real_adjudicator_python_guard_rejects_nonfrozen_runtime(self) -> None:
        """The test fixture never relaxes production's exact local runtime gate."""

        with mock.patch.object(
            adjudicator.platform, "python_version", return_value="3.13.15"
        ):
            with self.assertRaisesRegex(
                adjudicator.AdjudicationInputError,
                "independent Python version mismatch",
            ):
                adjudicator.trusted_adjudicator_python(self.package, self.source)

    def test_accepts_only_fully_sealed_validator_consistent_package(self) -> None:
        self.build_package(report=self.frozen_validator_canonical_report())
        result, out, boundary = self.adjudicate(
            FrozenBoundary(real_validator=True)
        )

        self.assertEqual(result["verdict"], adjudicator.ACCEPT_VERDICT)
        self.assertEqual(result["evidence_eligibility"], "ELIGIBLE")
        self.assertEqual(result["candidate_outcome"], "SIX_SCENARIO_RECEIPT_VERIFIED")
        self.assertEqual(boundary.validator_calls, 1)
        self.assertTrue((out / "adjudication.json").is_file())
        self.assertTrue((out / "validator.stdout.log").is_file())
        self.assertTrue((out / "validator.stderr.log").is_file())
        self.assertFalse((self.package / "adjudication.json").exists())
        self.assertEqual(result["claim_ceiling_after_review"], adjudicator.GLOBAL_CLAIM_CEILING)
        self.assertIn("changed-W cache probe", " ".join(result["limitations"]))

    def test_full_candidate_manifest_rejects_unknown_top_level_key(self) -> None:
        """A resealed extension must not silently widen the manifest contract."""

        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest.__setitem__("unrecognized_extension", True)
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_command_rejects_unknown_key(self) -> None:
        """A command row cannot carry an unverified field under a valid seal."""

        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["commands"][0].__setitem__(
                "unrecognized_extension", True
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_source_hash_row_rejects_unknown_key(self) -> None:
        """Source identity rows have exactly expected/observed/match fields."""

        self.build_package()
        relative = next(iter(self.synthetic_hashes))
        self.mutate_manifest(
            lambda manifest: manifest["source_hashes"][relative].__setitem__(
                "unrecognized_extension", True
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_summary_rejects_unknown_key(self) -> None:
        """The candidate summary cannot self-extend its schema."""

        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["candidate"].__setitem__(
                "unrecognized_extension", True
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_validator_summary_rejects_unknown_key(self) -> None:
        """The validator summary cannot self-extend its schema."""

        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["validator"].__setitem__(
                "unrecognized_extension", True
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_tool_shape_rejects_unknown_key(self) -> None:
        """Tool attestation metadata is a closed, runner-defined object."""

        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["tool_versions"]["platform"].__setitem__(
                "unrecognized_extension", True
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_rejects_resealed_unknown_regular_file(self) -> None:
        """A valid SHA256SUMS cannot authorize a file outside the stage allowlist."""

        self.build_package()
        (self.package / "unrecognized-artifact.bin").write_bytes(b"sealed extra\n")
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PACKAGE_INTEGRITY")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_rejects_noncanonical_cargo_target_root(self) -> None:
        """The run-scoped target must remain under the canonical state root."""

        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        noncanonical_target = (
            self.root
            / "noncanonical-state"
            / "build-cache"
            / adjudicator.EXECUTION_SOURCE_HEAD
            / self.run_id
        )
        noncanonical_binary = (
            noncanonical_target
            / "debug"
            / "examples"
            / "audit2_bateman_local_six_case"
        )
        noncanonical_binary.parent.mkdir(parents=True)
        noncanonical_binary.write_bytes(self.candidate_binary.read_bytes())
        target_text = str(noncanonical_target)
        manifest["tool_versions"]["environment_policy"]["effective"][
            "CARGO_TARGET_DIR"
        ] = target_text
        for row in manifest["commands"]:
            row["environment_overrides"]["CARGO_TARGET_DIR"] = target_text
        binary = manifest["tool_versions"]["candidate_binary"]
        binary["path"] = str(noncanonical_binary)
        binary["sha256_prelaunch"] = sha256_bytes(noncanonical_binary.read_bytes())
        binary["sha256_post"] = sha256_bytes(noncanonical_binary.read_bytes())
        binary["size_bytes"] = noncanonical_binary.stat().st_size
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_full_candidate_never_executes_package_selected_host_tools(self) -> None:
        """Recorded tool paths are evidence, never adjudicator execution authority."""

        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        replacements: dict[str, str] = {}
        forbidden_paths: set[str] = set()
        for name in ("git", "python"):
            previous = manifest["tool_versions"]["executables"][name]["path"]
            selected = self.root / f"package-selected-{name}"
            selected.write_bytes(f"attacker-selected-{name}\n".encode())
            selected_text = str(selected)
            replacements[previous] = selected_text
            forbidden_paths.add(str(selected.resolve()))
            identity = manifest["tool_versions"]["executables"][name]
            identity["path"] = selected_text
            identity["realpath"] = str(selected.resolve())
            identity["sha256"] = sha256_bytes(selected.read_bytes())
            identity["size_bytes"] = selected.stat().st_size
        for row in manifest["commands"]:
            row["argv"] = [replacements.get(part, part) for part in row["argv"]]
        manifest["validator"]["argv"] = [
            replacements.get(part, part) for part in manifest["validator"]["argv"]
        ]
        validator_attempt_path = self.package / "validator_attempt.json"
        validator_attempt = json.loads(validator_attempt_path.read_bytes())
        validator_attempt["argv"] = [
            replacements.get(part, part) for part in validator_attempt["argv"]
        ]
        events_path = self.package / "events.jsonl"
        events = [json.loads(line) for line in events_path.read_text().splitlines()]
        rows_by_name = {row["name"]: row for row in manifest["commands"]}
        for event in events:
            if event["event"] == "command_started":
                event["argv"] = rows_by_name[event["name"]]["argv"]
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        validator_attempt_path.write_text(
            json.dumps(validator_attempt, sort_keys=True) + "\n", encoding="utf-8"
        )
        events_path.write_text(
            "".join(json.dumps(event, sort_keys=True) + "\n" for event in events),
            encoding="utf-8",
        )
        self.seal()
        boundary = FrozenBoundary()
        boundary.forbidden_executable_paths = forbidden_paths

        result, _, boundary = self.adjudicate(boundary)

        self.assertEqual(result["verdict"], adjudicator.ACCEPT_VERDICT)
        self.assertEqual(boundary.forbidden_executable_calls, 0)
        self.assertEqual(boundary.git_calls, 10)
        self.assertEqual(boundary.validator_calls, 1)

    def test_full_candidate_one_shot_artifacts_reject_unknown_keys(self) -> None:
        """Matching lock/launch extensions cannot widen the one-shot schema."""

        self.build_package()
        for relative in ("attempt_lock.json", "candidate_launch.json"):
            path = self.package / relative
            payload = json.loads(path.read_bytes())
            payload["unrecognized_extension"] = True
            path.write_text(
                json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8"
            )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_unlisted_extra_file_is_inconclusive_and_validator_is_not_run(self) -> None:
        self.build_package()
        (self.package / "unsealed-extra").write_bytes(b"tamper")

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PACKAGE_INTEGRITY")
        self.assertEqual(boundary.validator_calls, 0)

    def test_duplicate_json_key_is_never_accepted(self) -> None:
        self.build_package()
        report = self.accepted_report().rstrip()[:-1] + b',"all_six_executed":true}\n'
        (self.package / "result_summary.json").write_bytes(report)
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        digest = sha256_bytes(report)
        manifest["candidate"]["result_summary_sha256"] = digest
        manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_parseable_recorded_contract_failure_is_bounded_reject(self) -> None:
        failure_object = json.loads(self.accepted_report())
        failure_object["all_contracts_satisfied"] = False
        failure_object["terminal_failure"] = {
            "phase": "solve",
            "message": "frozen structured failure",
        }
        failure = (
            json.dumps(failure_object, sort_keys=True, separators=(",", ":")).encode()
            + b"\n"
        )
        self.build_package(report=failure, candidate_exit=7, runner_validator_exit=1)
        boundary = FrozenBoundary(validator_exit=1, validator_stderr=b"receipt rejected\n")

        result, _, _ = self.adjudicate(boundary)

        self.assertEqual(result["verdict"], adjudicator.REJECT_VERDICT)
        self.assertEqual(result["candidate_outcome"], "RECORDED_SCIENTIFIC_OR_STRUCTURAL_FAILURE")

    def test_nonzero_without_parseable_rust_report_is_inconclusive(self) -> None:
        self.build_package(report=b"cargo: process killed\n", candidate_exit=137, runner_validator_exit=1)
        boundary = FrozenBoundary(validator_exit=1, validator_stderr=b"parse error\n")

        result, _, _ = self.adjudicate(boundary)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_LAUNCH_OR_REPORT")

    def test_duplicate_candidate_ledger_entry_is_inconclusive(self) -> None:
        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        duplicate = dict(next(row for row in manifest["commands"] if row["name"] == "candidate"))
        duplicate["ordinal"] = len(manifest["commands"]) + 1
        manifest["commands"].append(duplicate)
        manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_malformed_command_row_is_inconclusive_without_crashing(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["commands"].__setitem__(0, "malformed")
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_complete_candidate_package_requires_full_command_ledger(self) -> None:
        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        retained = manifest["commands"][: -9]
        for row in manifest["commands"][-9:]:
            for key in ("stdout_path", "stderr_path"):
                (self.package / row[key]).unlink()
        manifest["commands"] = retained
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )

        events = [
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 1,
                "event": "run_created",
                "utc": manifest["utc_start"],
                "run_id": manifest["run_id"],
            }
        ]
        for row in retained:
            events.extend(
                [
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 1,
                        "event": "command_started",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["started_at_utc"],
                        "argv": row["argv"],
                    },
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(events) + 2,
                        "event": "command_finished",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["finished_at_utc"],
                        "launch_status": row["launch_status"],
                        "exit_code": row["exit_code"],
                    },
                ]
            )
        events.append(
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": len(events) + 1,
                "event": "sealing_started",
                "utc": manifest["utc_end"],
                "verdict": manifest["verdict"],
            }
        )
        (self.package / "events.jsonl").write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in events),
            encoding="utf-8",
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_command_argv_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: self.command(manifest, "authority_verification")[
                "argv"
            ].append("--scientific-override")
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_command_cwd_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: self.command(manifest, "readiness").__setitem__(
                "cwd", str(self.root)
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_environment_overrides_are_inconclusive_before_validator(
        self,
    ) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: self.command(manifest, "cargo_build_candidate")[
                "environment_overrides"
            ].__setitem__("RUSTFLAGS", "--cfg outcome_dependent")
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_stream_path_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: self.command(manifest, "authority_verification").__setitem__(
                "stdout_path", "logs/python_dependencies.stdout.log"
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_stream_hash_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: self.command(manifest, "authority_verification").__setitem__(
                "stdout_sha256", "0" * 64
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_authority_verification_is_inconclusive_before_validator(
        self,
    ) -> None:
        self.build_package()
        authority_path = self.package / "authority_verification.json"
        authority = json.loads(authority_path.read_bytes())
        authority["candidate_executions"] = 1
        authority_path.write_text(
            json.dumps(authority, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_missing_event_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        events_path = self.package / "events.jsonl"
        events = [json.loads(line) for line in events_path.read_text().splitlines()]
        del events[2]
        events_path.write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in events),
            encoding="utf-8",
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_event_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        events_path = self.package / "events.jsonl"
        events = [json.loads(line) for line in events_path.read_text().splitlines()]
        candidate_start = next(
            row
            for row in events
            if row.get("event") == "command_started" and row.get("name") == "candidate"
        )
        candidate_start["argv"].append("--unfrozen")
        events_path.write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in events),
            encoding="utf-8",
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_source_snapshot_is_reconstructed_from_streams(self) -> None:
        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        row = self.command(manifest, "source_prelaunch_head")
        stream = self.package / row["stdout_path"]
        stream.write_text("0" * 40 + "\n", encoding="utf-8")
        row["stdout_sha256"] = sha256_bytes(stream.read_bytes())
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_declarations_are_exact_and_cannot_self_promote(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["declarations"].update(
                {
                    "local_llm_used": True,
                    "holdout_access": "OPENED",
                    "remote_write": "PERFORMED",
                    "claim_ceiling": "AUTHORITATIVE",
                }
            )
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_missing_candidate_launch_is_inconclusive_before_validator(self) -> None:
        self.build_package()
        (self.package / "candidate_launch.json").unlink()
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PACKAGE_INTEGRITY")
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_package_missing_authority_artifact_never_runs_validator(
        self,
    ) -> None:
        self.build_package()
        (self.package / "authority_verification.json").unlink()
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PACKAGE_INTEGRITY")
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_package_missing_tool_stage_never_runs_validator(self) -> None:
        self.build_package()
        self.mutate_manifest(
            lambda manifest: manifest["tool_versions"].pop("python")
        )

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_runner_validator_exit_127_is_infrastructure_inconclusive(self) -> None:
        self.build_package(runner_validator_exit=127)

        result, _, boundary = self.adjudicate()

        self.assertEqual(
            result["verdict"], "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )
        self.assertEqual(boundary.validator_calls, 0)

    def test_independent_validator_exit_127_is_infrastructure_inconclusive(
        self,
    ) -> None:
        self.build_package()
        boundary = FrozenBoundary(
            validator_exit=127, validator_stderr=b"validator unavailable\n"
        )

        result, _, boundary = self.adjudicate(boundary)

        self.assertEqual(
            result["verdict"], "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )
        self.assertEqual(boundary.validator_calls, 1)

    def test_independent_validator_spawn_error_is_infrastructure_inconclusive(
        self,
    ) -> None:
        self.build_package()
        boundary = FrozenBoundary(validator_error=OSError("synthetic spawn failure"))

        result, _, boundary = self.adjudicate(boundary)

        self.assertEqual(
            result["verdict"], "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )
        self.assertEqual(boundary.validator_calls, 1)

    def test_independent_validator_nonempty_stderr_is_infrastructure_inconclusive(
        self,
    ) -> None:
        self.build_package()
        boundary = FrozenBoundary(validator_stderr=b"unexpected diagnostic\n")

        result, _, boundary = self.adjudicate(boundary)

        self.assertEqual(
            result["verdict"], "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
        )
        self.assertEqual(boundary.validator_calls, 1)

    def test_wrong_scenario_disposition_is_never_accepted(self) -> None:
        report = json.loads(self.accepted_report())
        report["scenario_receipts"][0]["disposition"] = "candidate"
        mutated = json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        self.build_package(report=mutated)

        result, _, _ = self.adjudicate()

        self.assertNotEqual(result["verdict"], adjudicator.ACCEPT_VERDICT)

    def test_preflight_failure_cannot_be_scientific_reject(self) -> None:
        self.build_package()
        manifest_path = self.package / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        next(row for row in manifest["commands"] if row["name"] == "readiness")[
            "exit_code"
        ] = 1
        manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
        self.seal()

        result, _, boundary = self.adjudicate()

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.validator_calls, 0)

    def test_sealed_failed_prefix_without_candidate_is_not_run_preflight(self) -> None:
        outcome, executor = self.run_candidate_free_prefix(
            {"rust_authority_contracts": (101, b"", b"contracts failed\n")}
        )

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertIn("rust_authority_contracts", " ".join(result["errors"]))
        self.assertEqual(boundary.validator_calls, 0)
        self.assertEqual(boundary.git_calls, 0)
        self.assertNotIn("candidate", executor.calls)
        self.assertFalse((outcome.package_dir / "attempt_lock.json").exists())
        self.assertFalse((outcome.package_dir / "result_summary.json").exists())

    def test_candidate_free_resealed_unknown_file_is_package_integrity_failure(self) -> None:
        """A candidate-free seal is closed just like a full candidate seal."""

        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        (outcome.package_dir / "unexpected-sealed-note.txt").write_bytes(b"tamper\n")
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PACKAGE_INTEGRITY")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_manifest_key_injection_is_protocol_violation(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["unexpected"] = "resealed"
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_command_key_injection_is_protocol_violation(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["commands"][-1]["unexpected"] = "resealed"
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_runner_python_failure_package_is_not_run_preflight(self) -> None:
        outcome, executor = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED")
        self.assertNotIn("candidate", executor.calls)
        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertEqual(boundary.validator_calls, 0)

    def test_runner_rust_version_mismatch_package_is_not_run_preflight(self) -> None:
        outcome, executor = self.run_candidate_free_prefix(
            {"rustc_version": (0, b"rustc 1.93.0 (wrong)\n", b"")}
        )

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED")
        self.assertNotIn("candidate", executor.calls)
        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertEqual(boundary.validator_calls, 0)

    def test_all_runner_command_failures_are_candidate_free_and_ineligible(self) -> None:
        command_names = [
            template.name
            for template in runner.source_templates("source_pre", self.executables)
        ] + [template.name for template in runner.PREFLIGHT_COMMANDS]
        self.assertEqual(len(command_names), 21)

        for name in command_names:
            with self.subTest(command=name):
                outcome, executor = self.run_candidate_free_prefix(
                    {name: (1, b"", f"{name} failed\n".encode())}
                )
                result, _, boundary = self.adjudicate_runner_package(
                    outcome.package_dir
                )

                self.assertNotIn("candidate", executor.calls)
                self.assertEqual(
                    result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT"
                )
                self.assertEqual(boundary.git_calls, 0)
                self.assertEqual(boundary.validator_calls, 0)
                self.assertFalse(
                    (outcome.package_dir / "candidate_launch.json").exists()
                )
                self.assertFalse(
                    (outcome.package_dir / "result_summary.json").exists()
                )

    def test_semantic_environment_and_authority_stops_are_not_run(self) -> None:
        wrong_python = (
            json.dumps(
                {
                    "python": adjudicator.EXPECTED_PYTHON_VERSION,
                    "numpy": "0.0.0",
                    "mpmath": adjudicator.EXPECTED_MPMATH_VERSION,
                },
                sort_keys=True,
            ).encode()
            + b"\n"
        )
        wrong_authority = b'{"status":"WRONG"}\n'
        cases = {
            "python_dependencies": (0, wrong_python, b""),
            "rustc_version": (0, b"rustc 1.93.0 (wrong)\n", b""),
            "cargo_version": (0, b"cargo 1.93.0 (wrong)\n", b""),
            "authority_verification": (0, wrong_authority, b""),
        }
        for name, override in cases.items():
            with self.subTest(command=name):
                outcome, executor = self.run_candidate_free_prefix(
                    {name: override}
                )
                result, _, boundary = self.adjudicate_runner_package(
                    outcome.package_dir
                )

                self.assertNotIn("candidate", executor.calls)
                self.assertEqual(
                    result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT"
                )
                self.assertEqual(boundary.git_calls, 0)
                self.assertEqual(boundary.validator_calls, 0)

    def test_invalid_authority_json_is_distinct_runner_internal_not_run(self) -> None:
        outcome, executor = self.run_candidate_free_prefix(
            {"authority_verification": (0, b"{not-json\n", b"")}
        )

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE")
        self.assertNotIn("candidate", executor.calls)
        self.assertEqual(
            result["verdict"], "INCONCLUSIVE_NOT_RUN_RUNNER_INTERNAL_FAILURE"
        )
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_runner_internal_artifact_has_closed_schema(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"authority_verification": (0, b"{not-json\n", b"")}
        )
        failure_path = outcome.package_dir / "runner_internal_failure.json"
        failure = json.loads(failure_path.read_bytes())
        failure["unexpected"] = "resealed"
        failure_path.write_text(
            json.dumps(failure, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_forbidden_environment_is_not_run_without_any_subprocess(self) -> None:
        outcome, executor = self.run_candidate_free_prefix(
            {}, ambient_environment={"RUSTFLAGS": "--cfg outcome_dependent"}
        )

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED")
        self.assertEqual(executor.calls, [])
        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_forbidden_environment_artifact_binds_exact_keys(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {}, ambient_environment={"RUSTFLAGS": "--cfg forged"}
        )
        artifact_path = outcome.package_dir / "forbidden_environment.json"
        artifact_path.write_text(
            json.dumps({"keys": ["forged"]}, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_malformed_candidate_free_terminal_row_never_executes_subprocess(
        self,
    ) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["commands"][-1] = "malformed"
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_row_missing_name_never_raises_or_executes(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["commands"][-1].pop("name")
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_stage_key_injection_is_protocol_violation(self) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"python_dependencies": (1, b"", b"python unavailable\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["tool_versions"]["rustc_version"] = (
            f"rustc {adjudicator.EXPECTED_RUST_VERSION} (injected)"
        )
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_tampered_candidate_summary_cannot_escape_subprocess_free_path(
        self,
    ) -> None:
        outcome, _ = self.run_candidate_free_prefix(
            {"rust_authority_contracts": (101, b"", b"contracts failed\n")}
        )
        manifest_path = outcome.package_dir / "execution_manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["candidate"]["invocation_count"] = 1
        manifest["pre_candidate_stop"] = None
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(outcome.package_dir)

        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_all_prelaunch_source_failures_are_candidate_free(self) -> None:
        names = [
            template.name
            for template in runner.source_templates(
                "source_prelaunch", self.executables
            )
        ]
        self.assertEqual(len(names), 9)
        for name in names:
            with self.subTest(command=name):
                outcome, executor = self.run_candidate_free_prefix(
                    {name: (1, b"", f"{name} failed\n".encode())}
                )
                result, _, boundary = self.adjudicate_runner_package(
                    outcome.package_dir
                )

                self.assertEqual(
                    outcome.verdict,
                    "INCONCLUSIVE_SOURCE_CHANGED_BEFORE_CANDIDATE",
                )
                self.assertNotIn("candidate", executor.calls)
                self.assertEqual(
                    result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT"
                )
                self.assertEqual(boundary.git_calls, 0)
                self.assertEqual(boundary.validator_calls, 0)

    def test_source_hash_mismatch_is_candidate_free(self) -> None:
        target = self.source / next(iter(self.synthetic_hashes))
        target.write_bytes(b"mismatched source bytes\n")

        outcome, executor = self.run_candidate_free_prefix({})
        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(outcome.verdict, "INCONCLUSIVE_AUTHORITY_BUNDLE_MISMATCH")
        self.assertNotIn("candidate", executor.calls)
        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_missing_candidate_binary_is_candidate_free(self) -> None:
        self.candidate_binary.unlink()

        outcome, executor = self.run_candidate_free_prefix(
            {}, create_candidate_binary=False
        )
        result, _, boundary = self.adjudicate_runner_package(outcome.package_dir)

        self.assertEqual(
            outcome.verdict, "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
        )
        self.assertNotIn("candidate", executor.calls)
        self.assertEqual(result["verdict"], "INCONCLUSIVE_NOT_RUN_PREFLIGHT")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_one_shot_contention_is_distinct_candidate_free_outcome(self) -> None:
        first, first_executor = self.run_candidate_free_prefix({})
        self.assertEqual(first.verdict, runner.ACCEPT_VERDICT)
        self.assertEqual(first_executor.calls.count("candidate"), 1)

        second, second_executor = self.run_candidate_free_prefix({})
        result, _, boundary = self.adjudicate_runner_package(second.package_dir)

        self.assertEqual(
            second.verdict, "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED"
        )
        self.assertNotIn("candidate", second_executor.calls)
        self.assertEqual(
            result["verdict"],
            "INCONCLUSIVE_NOT_RUN_ONE_SHOT_ALREADY_CONSUMED",
        )
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_candidate_free_one_shot_lock_has_closed_schema(self) -> None:
        first, _ = self.run_candidate_free_prefix({})
        self.assertEqual(first.verdict, runner.ACCEPT_VERDICT)
        second, _ = self.run_candidate_free_prefix({})
        lock_path = second.package_dir / "attempt_lock.json"
        lock = json.loads(lock_path.read_bytes())
        lock["unexpected"] = "resealed"
        lock_path.write_text(
            json.dumps(lock, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.reseal(second.package_dir)

        result, _, boundary = self.adjudicate_runner_package(second.package_dir)

        self.assertEqual(result["verdict"], "INCONCLUSIVE_PROTOCOL_VIOLATION")
        self.assertEqual(boundary.git_calls, 0)
        self.assertEqual(boundary.validator_calls, 0)

    def test_cli_rejects_scientific_override_and_inside_package_sidecar(self) -> None:
        self.build_package()
        unsupported = subprocess.run(
            [
                sys.executable,
                str(ADJUDICATOR_PATH),
                "--package",
                str(self.package),
                "--source-worktree",
                str(self.source),
                "--out",
                str(self.root / "sidecar"),
                "--output-atol",
                "1",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(unsupported.returncode, 2)
        self.assertIn(b"unrecognized arguments", unsupported.stderr)

        with mock.patch.dict(
            adjudicator.EXPECTED_SOURCE_HASHES, self.synthetic_hashes, clear=True
        ):
            with self.assertRaises(adjudicator.AdjudicationInputError):
                adjudicator.adjudicate_package(
                    self.package,
                    self.source,
                    self.package / "sidecar",
                    command_runner=FrozenBoundary(),
                )


if __name__ == "__main__":
    unittest.main()
