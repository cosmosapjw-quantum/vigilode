#!/usr/bin/env python3
"""Unit tests for tools/py01_laguerre_component_check.py (PY01).

Tests fail-closed behavior on valid and mutated evidence:
- PASS path: valid export produces verdict PASS (exit code 0).
- FAIL path: gate violations (completeness failure, broken enclosure, adjoint mismatch) produce verdict FAIL (exit code 1).
- INVALID path: schema corruption, SHA-256 mismatch, dirty tree, identity stamp mismatch produce verdict INVALID (exit code 2).
"""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import py01_laguerre_component_check as checker

PUBLISHED_PATH = ROOT / "research/rnext04_laguerre_admission_20261003/cases.json"
PP09_PATH = ROOT / "research/pp09_laguerre_envelope_20261004/RESULTS.json"
COMPONENTS_PATH = ROOT / "research/py01_laguerre_component_export_20261011/COMPONENTS.json"


class TestPy01LaguerreComponentCheck(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Ensure COMPONENTS.json exists for base valid tests
        if not COMPONENTS_PATH.exists():
            subprocess.run(
                [
                    "cargo",
                    "test",
                    "--offline",
                    "--locked",
                    "-p",
                    "rodas5p-core",
                    "--test",
                    "laguerre_component_export",
                    "--",
                    "--ignored",
                    "export_components",
                ],
                cwd=ROOT,
                check=True,
                capture_output=True,
            )
        cls.valid_components_bytes = COMPONENTS_PATH.read_bytes()

    def run_checker_on_temp(
        self,
        comp_data: dict,
        published_path: Path = PUBLISHED_PATH,
        pp09_path: Path = PP09_PATH,
        num_workers: int | None = None,
    ) -> tuple[int, dict | None]:
        with tempfile.NamedTemporaryFile("w+", suffix=".json") as comp_file, tempfile.NamedTemporaryFile(
            "w+", suffix=".json"
        ) as out_file:
            json.dump(comp_data, comp_file, indent=2)
            comp_file.flush()
            code = checker.run_check(
                published_path=published_path,
                pp09_path=pp09_path,
                components_path=Path(comp_file.name),
                output_path=Path(out_file.name),
                num_workers=num_workers,
            )
            out_doc = None
            out_path = Path(out_file.name)
            if out_path.exists() and out_path.stat().st_size > 0:
                try:
                    out_doc = json.loads(out_path.read_text())
                except Exception:
                    pass
            return code, out_doc

    def test_pass_on_valid_export(self):
        comp_data = json.loads(self.valid_components_bytes)
        code, out_doc = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 0, "Valid export must exit with code 0")
        self.assertIsNotNone(out_doc)
        self.assertEqual(out_doc["verdict"], "PASS")
        self.assertTrue(out_doc["gate"]["1_completeness"])
        self.assertTrue(out_doc["gate"]["2_enclosure"])
        self.assertTrue(out_doc["gate"]["3_source_binding"])
        self.assertEqual(len(out_doc["reported"]["cases"]), 9)

    def test_invalid_on_missing_published_cases(self):
        comp_data = json.loads(self.valid_components_bytes)
        code, _ = self.run_checker_on_temp(comp_data, published_path=Path("nonexistent/cases.json"))
        self.assertEqual(code, 2, "Missing published file must exit with code 2 (INVALID)")

    def test_invalid_on_corrupted_published_sha(self):
        comp_data = json.loads(self.valid_components_bytes)
        with tempfile.NamedTemporaryFile("w+", suffix=".json") as bad_pub:
            bad_pub.write('{"cases": []}')
            bad_pub.flush()
            code, _ = self.run_checker_on_temp(comp_data, published_path=Path(bad_pub.name))
            self.assertEqual(code, 2, "Tampered published SHA must exit with code 2 (INVALID)")

    def test_invalid_on_dirty_tree(self):
        comp_data = json.loads(self.valid_components_bytes)
        comp_data["tree_clean"] = False
        code, _ = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 2, "tree_clean=False must exit with code 2 (INVALID)")

    def test_invalid_on_tampered_identity_stamp(self):
        comp_data = json.loads(self.valid_components_bytes)
        # Modify identity degree without updating identity_sha256
        comp_data["cases"][0]["export"]["identity"]["degree"] += 1
        code, _ = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 2, "Mismatched identity_sha256 must exit with code 2 (INVALID)")

    def test_invalid_on_tampered_column_stamp(self):
        comp_data = json.loads(self.valid_components_bytes)
        # Tamper stamp on column 0
        comp_data["cases"][0]["export"]["columns"][0]["identity_sha256"] = "0" * 64
        code, _ = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 2, "Tampered column stamp must exit with code 2 (INVALID)")

    def test_invalid_on_duplicate_labels(self):
        comp_data = json.loads(self.valid_components_bytes)
        comp_data["cases"][1]["label"] = comp_data["cases"][0]["label"]
        code, _ = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 2, "Duplicate case labels must exit with code 2 (INVALID)")

    def test_fail_on_negative_or_nan_term(self):
        comp_data = json.loads(self.valid_components_bytes)
        # Make column 0 truncation negative: float -1.0 is bff0000000000000
        comp_data["cases"][0]["export"]["columns"][0]["truncation"] = "bff0000000000000"
        code, out_doc = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 1, "Negative term must exit with code 1 (FAIL)")
        self.assertEqual(out_doc["verdict"], "FAIL")
        self.assertFalse(out_doc["gate"]["1_completeness"])

    def test_fail_on_tampered_recurrence_adjoint(self):
        comp_data = json.loads(self.valid_components_bytes)
        # Modify recurrence_adjoint to break source binding
        col0 = comp_data["cases"][0]["export"]["columns"][0]
        col0["recurrence_adjoint"] = "3ff0000000000000"  # 1.0
        code, out_doc = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 1, "Tampered recurrence_adjoint must exit with code 1 (FAIL)")
        self.assertEqual(out_doc["verdict"], "FAIL")
        self.assertFalse(out_doc["gate"]["3_source_binding"])

    def test_fail_on_tampered_envelope_key_digest(self):
        comp_data = json.loads(self.valid_components_bytes)
        # Mutate envelope key digest
        exp0 = comp_data["cases"][0]["export"]
        exp0["identity"]["envelope_key_digests"][0] = "a" * 64
        # Recompute identity_sha256 so it passes validity
        new_stamp = checker.canonical_identity_sha256(exp0["identity"])
        exp0["identity"]["identity_sha256"] = new_stamp
        for col in exp0["columns"]:
            col["identity_sha256"] = new_stamp
        exp0["assembly"]["identity_sha256"] = new_stamp
        for adj in exp0["adjoint_columns"]:
            adj["identity_sha256"] = new_stamp

        code, out_doc = self.run_checker_on_temp(comp_data)
        self.assertEqual(code, 1, "Mismatched envelope key digest must exit with code 1 (FAIL)")
        self.assertEqual(out_doc["verdict"], "FAIL")
        self.assertFalse(out_doc["gate"]["3_source_binding"])


if __name__ == "__main__":
    unittest.main()
