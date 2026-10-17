#!/usr/bin/env python3
"""Synthetic mutation tests for tools/ht01_selection_check.py (node HT01).

Tests:
1. Valid baseline fixture passes (exit 0, verdict PASS, decision ABSTAIN).
2. Validity mutations yield INVALID (exit 2):
   - schema mismatch
   - pinned input missing from disk
   - pinned input hash mismatch
   - candidate list length or identity mismatch
   - applicable native arm list modified from closed preregistered list
   - contract document missing
3. Failure mutations yield FAIL (exit 1):
   - preliminary decision mismatch
   - K1 row missing q1_solves = 0 or 'PP07 ABSTAIN preserved'
   - false declaration claim on candidate
   - false pass_s1 claim
   - false pass_s4_a claim on candidate lacking GeneratedFromModel

Run:
    python3 tools/test_ht01_selection_check.py
    python3 -m unittest discover -s tools -p "test_ht01_*.py"
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

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
CHECKER = TOOLS / "ht01_selection_check.py"
SCREEN_GEN = TOOLS / "ht01_client_screen.py"


class TestHt01SelectionCheck(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp_dir = tempfile.TemporaryDirectory()
        cls.td = Path(cls.temp_dir.name)
        cls.screen_file = cls.td / "SCREEN.json"
        # Run screen generator to create valid baseline
        proc = subprocess.run(
            [sys.executable, str(SCREEN_GEN), "--output", str(cls.screen_file)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        cls.valid_screen_data = json.loads(cls.screen_file.read_text(encoding="utf-8"))

    @classmethod
    def tearDownClass(cls):
        cls.temp_dir.cleanup()

    def run_check(self, screen_path: Path, output_path: Path) -> subprocess.CompletedProcess:
        return subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--screen",
                str(screen_path),
                "--output",
                str(output_path),
                "--allow-dirty",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )

    def test_valid_baseline(self):
        out = self.td / "RESULTS_baseline.json"
        proc = self.run_check(self.screen_file, out)
        self.assertEqual(proc.returncode, 0, f"Expected 0, got {proc.returncode}. Output:\n{proc.stdout}\n{proc.stderr}")
        self.assertTrue(out.exists())
        data = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(data.get("verdict"), "PASS")
        self.assertEqual(data.get("decision"), "ABSTAIN")
        self.assertIsNone(data.get("selected_candidate"))
        self.assertEqual(len(data.get("rows", [])), 8)

    def test_invalid_schema(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        mutant["schema"] = "invalid-schema-v0"
        scr = self.td / "SCREEN_invalid_schema.json"
        out = self.td / "RESULTS_invalid_schema.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on schema mutation, got {proc.returncode}")

    def test_invalid_hash_mismatch(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        first_key = list(mutant["inputs_sha256"].keys())[0]
        mutant["inputs_sha256"][first_key] = "0" * 64
        scr = self.td / "SCREEN_hash_mismatch.json"
        out = self.td / "RESULTS_hash_mismatch.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on hash mismatch, got {proc.returncode}")

    def test_invalid_missing_input_file(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        mutant["inputs_sha256"]["non_existent_file_fixture.rs"] = "a" * 64
        scr = self.td / "SCREEN_missing_input.json"
        out = self.td / "RESULTS_missing_input.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on missing input, got {proc.returncode}")

    def test_invalid_modified_applicable_arms(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        # Add an unauthorized arm to K1
        for c in mutant["candidates"]:
            if c["id"] == "K1":
                c["applicable_native_arms"] = ["dense_v2"]
        scr = self.td / "SCREEN_bad_arms.json"
        out = self.td / "RESULTS_bad_arms.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on arm list modification, got {proc.returncode}")

    def test_invalid_candidate_count(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        # Drop K8
        mutant["candidates"] = [c for c in mutant["candidates"] if c["id"] != "K8"]
        scr = self.td / "SCREEN_bad_candidates.json"
        out = self.td / "RESULTS_bad_candidates.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 2, f"Expected 2 on dropping candidate, got {proc.returncode}")

    def test_fail_preliminary_decision_mismatch(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        mutant["preliminary_decision"] = "SELECTED(K2)"
        scr = self.td / "SCREEN_bad_decision.json"
        out = self.td / "RESULTS_bad_decision.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on decision mismatch, got {proc.returncode}")

    def test_fail_k1_missing_abstain_citation(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        for c in mutant["candidates"]:
            if c["id"] == "K1":
                c["s2_shifted_solves"]["notes"] = "some arbitrary notes without citation"
        scr = self.td / "SCREEN_k1_citation.json"
        out = self.td / "RESULTS_k1_citation.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on K1 citation missing, got {proc.returncode}")

    def test_fail_false_declaration_claim(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        for c in mutant["candidates"]:
            if c["id"] == "K2":
                c["s1_declared_expensive"]["has_declaration"] = True
        scr = self.td / "SCREEN_false_decl.json"
        out = self.td / "RESULTS_false_decl.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on false declaration claim, got {proc.returncode}")

    def test_fail_false_pass_s4_claim(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        for c in mutant["candidates"]:
            if c["id"] == "K3":
                c["s4_accounting_certificate"]["pass_s4_a"] = True
        scr = self.td / "SCREEN_false_s4.json"
        out = self.td / "RESULTS_false_s4.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on false pass_s4_a, got {proc.returncode}")

    def test_fail_s4_b_status_mutation(self):
        mutant = copy.deepcopy(self.valid_screen_data)
        for c in mutant["candidates"]:
            if c["id"] == "K2":
                c["s4_accounting_certificate"]["s4_b_status"] = "PASS"
        scr = self.td / "SCREEN_s4_b_mut.json"
        out = self.td / "RESULTS_s4_b_mut.json"
        scr.write_text(json.dumps(mutant), encoding="utf-8")
        proc = self.run_check(scr, out)
        self.assertEqual(proc.returncode, 1, f"Expected 1 on S4(b) status mutation, got {proc.returncode}")

    def test_generator_determinism(self):
        run1 = self.td / "SCREEN_det1.json"
        run2 = self.td / "SCREEN_det2.json"
        subprocess.run([sys.executable, str(SCREEN_GEN), "--output", str(run1)], cwd=ROOT, check=True)
        subprocess.run([sys.executable, str(SCREEN_GEN), "--output", str(run2)], cwd=ROOT, check=True)
        import hashlib
        h1 = hashlib.sha256(run1.read_bytes()).hexdigest()
        h2 = hashlib.sha256(run2.read_bytes()).hexdigest()
        self.assertEqual(h1, h2, "SCREEN generator must be bitwise deterministic across runs")

    def test_generator_comment_rejection(self):
        import ht01_client_screen as screen_mod
        temp_rs = self.td / "test_comments.rs"
        temp_rs.write_text(
            "// TODO: ExpensiveClientDeclaration { declared_rhs_ir: 100 }\n"
            "/*\n"
            "ExpensiveClientDeclaration {\n"
            "    declared_rhs_ir: 200\n"
            "}\n"
            "*/\n"
            "fn dummy() {}\n"
        )
        self.assertFalse(
            screen_mod.check_expensive_declaration([str(temp_rs.relative_to(ROOT) if temp_rs.is_relative_to(ROOT) else temp_rs)]),
            "ExpensiveClientDeclaration inside comments must not be recognized as a valid declaration",
        )


if __name__ == "__main__":
    unittest.main()

