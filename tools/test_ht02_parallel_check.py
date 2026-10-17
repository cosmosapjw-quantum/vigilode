#!/usr/bin/env python3
"""Unit and synthetic mutation tests for tools/ht02_parallel_check.py (node HT02).

Tests:
1. Valid Branch A execution passes (exit 0, verdict INCONCLUSIVE, decision ABSTAIN_NOT_RUN).
2. Missing HT01 results yields INVALID (exit 2).
3. Schema mismatch yields INVALID (exit 2).
4. HT01 decision mismatch (e.g. SELECTED(K2) under --abstain-only) yields INVALID (exit 2).
5. HT01 verdict FAIL yields FAIL (exit 1).

Run:
    python3 tools/test_ht02_parallel_check.py
    python3 -m unittest discover -s tools -p "test_ht02_*.py"
"""

from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
CHECKER = TOOLS / "ht02_parallel_check.py"
HT01_RESULTS = ROOT / "research/ht01_homotopy_client_contract_20261011/RESULTS.json"


class TestHt02ParallelCheck(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp_dir = tempfile.TemporaryDirectory()
        cls.td = Path(cls.temp_dir.name)
        cls.valid_ht01_data = json.loads(HT01_RESULTS.read_text(encoding="utf-8"))

    @classmethod
    def tearDownClass(cls):
        cls.temp_dir.cleanup()

    def run_check(self, ht01_path: Path, output_path: Path, extra_args: list[str] | None = None) -> subprocess.CompletedProcess:
        cmd = [
            sys.executable,
            str(CHECKER),
            "--ht01",
            str(ht01_path),
            "--abstain-only",
            "--output",
            str(output_path),
        ]
        if extra_args:
            cmd.extend(extra_args)
        return subprocess.run(
            cmd,
            cwd=ROOT,
            capture_output=True,
            text=True,
        )

    def test_valid_branch_a(self):
        out = self.td / "RESULTS_branch_a.json"
        proc = self.run_check(HT01_RESULTS, out)
        self.assertEqual(proc.returncode, 0, f"Expected 0, got {proc.returncode}. Output:\n{proc.stdout}\n{proc.stderr}")
        self.assertTrue(out.exists())
        data = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(data.get("schema"), "vigilode-ht02-results-v1")
        self.assertEqual(data.get("node"), "HT02")
        self.assertEqual(data.get("verdict"), "INCONCLUSIVE")
        self.assertEqual(data.get("decision"), "ABSTAIN_NOT_RUN")
        self.assertEqual(data.get("runs"), 0)
        self.assertIsNone(data.get("wall_time_speedup"))
        self.assertIn("K1", data.get("candidate_failing_criteria", {}))

    def test_missing_ht01_file(self):
        non_existent = self.td / "non_existent_results.json"
        out = self.td / "RESULTS_missing.json"
        proc = self.run_check(non_existent, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on missing file, got {proc.returncode}")

    def test_schema_mismatch(self):
        mutant = copy.deepcopy(self.valid_ht01_data)
        mutant["schema"] = "invalid-schema-v0"
        ht01_file = self.td / "ht01_bad_schema.json"
        out = self.td / "RESULTS_bad_schema.json"
        ht01_file.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(ht01_file, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on schema mismatch, got {proc.returncode}")

    def test_ht01_decision_mismatch(self):
        mutant = copy.deepcopy(self.valid_ht01_data)
        mutant["decision"] = "SELECTED(K2)"
        ht01_file = self.td / "ht01_selected.json"
        out = self.td / "RESULTS_selected.json"
        ht01_file.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(ht01_file, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on decision mismatch with --abstain-only, got {proc.returncode}")

    def test_ht01_verdict_fail(self):
        mutant = copy.deepcopy(self.valid_ht01_data)
        mutant["verdict"] = "FAIL"
        ht01_file = self.td / "ht01_fail.json"
        out = self.td / "RESULTS_fail.json"
        ht01_file.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(ht01_file, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on HT01 verdict FAIL, got {proc.returncode}")


if __name__ == "__main__":
    unittest.main()
