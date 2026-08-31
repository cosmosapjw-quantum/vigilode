#!/usr/bin/env python3
"""Candidate-free tests for the local-Codex stage-certificate handoff."""

from __future__ import annotations

import importlib.util
import json
import pathlib
import shutil
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
VALIDATOR_PATH = ROOT / "tools" / "validate_audit2_stage_certificate_handoff.py"
SOURCE_DIRECTORY = ROOT / "research" / "audit2_stage_certificate_telemetry_20260831"

spec = importlib.util.spec_from_file_location("stage_certificate_handoff", VALIDATOR_PATH)
assert spec and spec.loader
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class StageCertificateHandoffTests(unittest.TestCase):
    def copied_root(self, temporary: str) -> pathlib.Path:
        root = pathlib.Path(temporary)
        destination = root / validator.DIRECTORY
        destination.parent.mkdir(parents=True)
        shutil.copytree(SOURCE_DIRECTORY, destination)
        return root

    def test_checked_in_handoff_is_candidate_free_and_bounded(self):
        result = validator.validate(ROOT)
        self.assertEqual(result["status"], "CANDIDATE_FREE_LOCAL_CODEX_HANDOFF_VALID")
        self.assertEqual(result["candidate_executions"], 0)
        self.assertLessEqual(result["checked_bytes"], 2_000_000)

    def test_candidate_count_tamper_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            path = root / validator.DIRECTORY / "EXECUTION_CONTRACT.json"
            value = json.loads(path.read_text())
            value["execution_policy"]["candidate_executions_required"] = 1
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "candidate executions"):
                validator.validate(root)

    def test_claim_ceiling_tamper_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            path = root / validator.DIRECTORY / "handoff.json"
            value = json.loads(path.read_text())
            value["claim_ceiling"] = "PROMOTED"
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "claim ceiling"):
                validator.validate(root)

    def test_raw_or_compiled_artifact_suffix_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            (root / validator.DIRECTORY / "raw-output.log").write_text("not admissible")
            with self.assertRaisesRegex(ValueError, "forbidden checked-in suffix"):
                validator.validate(root)

    def test_historic_candidate_runtime_surface_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            path = root / validator.DIRECTORY / "README.md"
            path.write_text(path.read_text() + "\naudit2_bateman_local_six_case\n")
            with self.assertRaisesRegex(ValueError, "historic candidate runtime surface"):
                validator.validate(root)

    def test_formal_partial_pass_policy_tamper_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            path = root / validator.DIRECTORY / "EXECUTION_CONTRACT.json"
            value = json.loads(path.read_text())
            value["formal"]["pass_requires_every_backend"] = False
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "every backend"):
                validator.validate(root)

    def test_process_harness_hash_tamper_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix="audit2-stage-handoff-") as temporary:
            root = self.copied_root(temporary)
            path = root / validator.DIRECTORY / "EXECUTION_CONTRACT.json"
            value = json.loads(path.read_text())
            value["harness_inputs"][0]["sha256"] = "0" * 64
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "harness identity"):
                validator.validate(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
