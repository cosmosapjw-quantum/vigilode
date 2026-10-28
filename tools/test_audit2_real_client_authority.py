#!/usr/bin/env python3
"""Exact, candidate-free checks for the frozen Bateman authority artifact."""

from __future__ import annotations

import importlib.util
import json
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
VERIFIER_PATH = (
    ROOT
    / "research"
    / "audit2_real_client_authority_construction_20260830"
    / "verify_authority_manifest.py"
)
MANIFEST_PATH = VERIFIER_PATH.with_name("authority_manifest.json")
RECEIPT_PATH = VERIFIER_PATH.with_name("evidence") / "AUTHORITY_VERIFICATION_RECEIPT.json"

spec = importlib.util.spec_from_file_location("audit2_bateman_authority", VERIFIER_PATH)
assert spec and spec.loader
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class ExactAuthorityTests(unittest.TestCase):
    def test_checked_in_proof_receipt_binds_manifest_verifier_and_candidate_free_summary(self):
        summary = verifier.verify_receipt(MANIFEST_PATH, RECEIPT_PATH)
        self.assertEqual(summary["status"], "AUTHORITY_CONSTRUCTION_VERIFIED")
        self.assertEqual(summary["candidate_executions"], 0)
        self.assertEqual(summary["local_six_case_status"], "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION")
        self.assertEqual(len(summary["receipt_sha256"]), 64)

    def test_canonical_manifest_has_exact_reference_digest_and_pc_authority(self):
        summary = verifier.verify_manifest(MANIFEST_PATH)
        self.assertEqual(summary["status"], "AUTHORITY_CONSTRUCTION_VERIFIED")
        self.assertEqual(summary["candidate_executions"], 0)
        self.assertEqual(summary["verified_operator_cases"], 2)
        self.assertEqual(summary["execution_scenarios"], 6)
        self.assertLess(summary["max_reference_l2_bound"], 1.0e-15)
        self.assertTrue(summary["fast_exponent_exceeds_one"])

    def test_understated_reference_uncertainty_fails_closed(self):
        manifest = json.loads(MANIFEST_PATH.read_text())
        manifest["operator_cases"][0]["reference"]["uncertainty_l2"] = 0.0
        with tempfile.TemporaryDirectory(prefix="audit2-authority-") as directory:
            path = pathlib.Path(directory) / "manifest.json"
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "reference uncertainty"):
                verifier.verify_manifest(path)

    def test_frozen_w_digest_tamper_fails_closed(self):
        manifest = json.loads(MANIFEST_PATH.read_text())
        manifest["operator_cases"][1]["frozen_w_semantic"]["sha256"] = "0" * 64
        with tempfile.TemporaryDirectory(prefix="audit2-authority-") as directory:
            path = pathlib.Path(directory) / "manifest.json"
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "frozen-W digest"):
                verifier.verify_manifest(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
