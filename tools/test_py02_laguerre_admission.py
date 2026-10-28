#!/usr/bin/env python3
"""Unit tests for tools/py02_laguerre_admission.py (PY02).

Tests the 3-gate Stage-1 admission checker:
- PASS/ABSTAIN paths (exit code 0):
  - Normal run: A1 and A2 hold, A3 fails -> verdict ABSTAIN (A3).
  - Hypothetical run: A1, A2, and A3 all hold -> verdict PASS (ADMIT).
  - A1 failure: recurrence_adjoint share < 0.9 -> verdict ABSTAIN (A1).
  - A2 failure: closability floor F > budget everywhere -> verdict ABSTAIN (A2).
  - Missing component evidence: PY01 != PASS or gate 1_completeness is False -> verdict ABSTAIN (no component evidence).
- INVALID paths (exit code 2):
  - Missing input files.
  - Recorded tree dirty (tree_clean=False) in either PY01 or PLANS.
  - Schema mismatch in PY01 or PLANS.
  - Case count != 32 in PLANS.
  - Reference mismatch or budget formula tampering.
"""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
PY01_PATH = ROOT / "research/py01_laguerre_component_export_20261011/RESULTS.json"
PLANS_PATH = ROOT / "research/py02_laguerre_adjoint_bound_20261011/PLANS.json"

import sys
sys.path.insert(0, str(ROOT / "tools"))
import py02_laguerre_admission as admission


class TestPy02LaguerreAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.py01_doc = json.loads(PY01_PATH.read_text(encoding="utf-8"))
        if PLANS_PATH.exists():
            cls.plans_doc = json.loads(PLANS_PATH.read_text(encoding="utf-8"))
        else:
            with tempfile.NamedTemporaryFile("w+", suffix=".json") as f:
                env = {**os.environ, "PY02_PLANS": f.name}
                subprocess.run(
                    [
                        "cargo",
                        "test",
                        "--offline",
                        "--locked",
                        "--release",
                        "-p",
                        "rodas5p-core",
                        "--test",
                        "laguerre_adjoint_holdout",
                        "--",
                        "--ignored",
                        "plan_only",
                    ],
                    cwd=ROOT,
                    env=env,
                    check=True,
                    capture_output=True,
                )
                cls.plans_doc = json.loads(Path(f.name).read_text(encoding="utf-8"))

    def run_with_docs(
        self,
        py01_doc: dict,
        plans_doc: dict,
        verify_references: bool = False,
        verify_checker_blob: bool = False,
    ) -> tuple[int, dict | None]:
        with tempfile.NamedTemporaryFile("w+", suffix=".json") as f_py01, \
             tempfile.NamedTemporaryFile("w+", suffix=".json") as f_plans, \
             tempfile.NamedTemporaryFile("w+", suffix=".json") as f_out:
            json.dump(py01_doc, f_py01, indent=2)
            f_py01.flush()
            json.dump(plans_doc, f_plans, indent=2)
            f_plans.flush()

            code = admission.run_check(
                py01_path=Path(f_py01.name),
                plans_path=Path(f_plans.name),
                output_path=Path(f_out.name),
                verify_references=verify_references,
                verify_checker_blob=verify_checker_blob,
            )

            out_doc = None
            out_path = Path(f_out.name)
            if out_path.exists() and out_path.stat().st_size > 0:
                try:
                    out_doc = json.loads(out_path.read_text(encoding="utf-8"))
                except Exception:
                    pass
            return code, out_doc

    def test_abstain_a3_on_valid_inputs(self):
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, doc = self.run_with_docs(self.py01_doc, plans_clean)
        self.assertEqual(code, 0, "Valid stage-1 inputs must exit with code 0")
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "ABSTAIN")
        self.assertEqual(doc["decision"], "ABSTAIN (A3)")
        self.assertFalse(doc["stage2_admitted"])
        self.assertEqual(doc["first_failed_gate"], "A3")
        self.assertTrue(doc["gate"]["A1_dominance"])
        self.assertTrue(doc["gate"]["A2_closability"])
        self.assertFalse(doc["gate"]["A3_work_feasibility"])
        self.assertEqual(doc["a3_work_feasibility"]["laguerre_fewer_count"], 0)
        self.assertEqual(doc["a3_work_feasibility"]["total_cases"], 32)

    def test_admit_when_all_pass(self):
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        # Synthesize 8 cases with fewer vector products for Laguerre
        for i in range(8):
            plans_clean["cases"][i]["laguerre_plan"]["vector_products"] = (
                plans_clean["cases"][i]["chebyshev_plan"]["vector_products"] - 5
            )
        code, doc = self.run_with_docs(self.py01_doc, plans_clean)
        self.assertEqual(code, 0)
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "PASS")
        self.assertEqual(doc["decision"], "ADMIT")
        self.assertTrue(doc["stage2_admitted"])
        self.assertIsNone(doc["first_failed_gate"])
        self.assertTrue(doc["gate"]["A1_dominance"])
        self.assertTrue(doc["gate"]["A2_closability"])
        self.assertTrue(doc["gate"]["A3_work_feasibility"])
        self.assertEqual(doc["a3_work_feasibility"]["laguerre_fewer_count"], 8)

    def test_abstain_a1_when_dominance_fails(self):
        py01_mutated = copy.deepcopy(self.py01_doc)
        # Mutate first gated case recurrence_adjoint share to 0.85 < 0.9
        for c in py01_mutated["reported"]["cases"]:
            if c["label"] == "n6-rho400-h0.1-distinct":
                c["shares"]["recurrence_adjoint"] = "8.500000e-01"
                break
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, doc = self.run_with_docs(py01_mutated, plans_clean)
        self.assertEqual(code, 0)
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "ABSTAIN")
        self.assertEqual(doc["decision"], "ABSTAIN (A1)")
        self.assertEqual(doc["first_failed_gate"], "A1")
        self.assertFalse(doc["gate"]["A1_dominance"])

    def test_abstain_a2_when_closability_fails(self):
        py01_mutated = copy.deepcopy(self.py01_doc)
        # Mutate closability floors to exceed budget everywhere
        for c in py01_mutated["reported"]["cases"]:
            c["closability_floor_F"] = "1.000000e+02"
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, doc = self.run_with_docs(py01_mutated, plans_clean)
        self.assertEqual(code, 0)
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "ABSTAIN")
        self.assertEqual(doc["decision"], "ABSTAIN (A2)")
        self.assertEqual(doc["first_failed_gate"], "A2")
        self.assertTrue(doc["gate"]["A1_dominance"])
        self.assertFalse(doc["gate"]["A2_closability"])

    def test_abstain_no_component_evidence_when_py01_not_pass(self):
        py01_mutated = copy.deepcopy(self.py01_doc)
        py01_mutated["verdict"] = "FAIL"
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, doc = self.run_with_docs(py01_mutated, plans_clean)
        self.assertEqual(code, 0)
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "ABSTAIN")
        self.assertEqual(doc["decision"], "ABSTAIN (no component evidence)")
        self.assertFalse(doc["stage2_admitted"])
        self.assertEqual(doc["first_failed_gate"], "no component evidence")

    def test_abstain_no_component_evidence_when_py01_gate1_false(self):
        py01_mutated = copy.deepcopy(self.py01_doc)
        py01_mutated["gate"]["1_completeness"] = False
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, doc = self.run_with_docs(py01_mutated, plans_clean)
        self.assertEqual(code, 0)
        self.assertIsNotNone(doc)
        self.assertEqual(doc["verdict"], "ABSTAIN")
        self.assertEqual(doc["decision"], "ABSTAIN (no component evidence)")
        self.assertFalse(doc["stage2_admitted"])

    def test_invalid_on_missing_file(self):
        code = admission.run_check(
            py01_path=Path("nonexistent/py01.json"),
            plans_path=PLANS_PATH,
        )
        self.assertEqual(code, 2)
        code2 = admission.run_check(
            py01_path=PY01_PATH,
            plans_path=Path("nonexistent/plans.json"),
        )
        self.assertEqual(code2, 2)

    def test_invalid_on_dirty_tree_plans(self):
        plans_dirty = copy.deepcopy(self.plans_doc)
        plans_dirty["tree_clean"] = False
        code, _ = self.run_with_docs(self.py01_doc, plans_dirty)
        self.assertEqual(code, 2)

    def test_invalid_on_dirty_tree_py01(self):
        py01_dirty = copy.deepcopy(self.py01_doc)
        py01_dirty["tree_clean"] = False
        plans_clean = copy.deepcopy(self.plans_doc)
        plans_clean["tree_clean"] = True
        code, _ = self.run_with_docs(py01_dirty, plans_clean)
        self.assertEqual(code, 2)

    def test_invalid_on_wrong_schema(self):
        plans_bad = copy.deepcopy(self.plans_doc)
        plans_bad["tree_clean"] = True
        plans_bad["schema"] = "wrong-schema-v1"
        code, _ = self.run_with_docs(self.py01_doc, plans_bad)
        self.assertEqual(code, 2)

    def test_invalid_on_wrong_case_count(self):
        plans_bad = copy.deepcopy(self.plans_doc)
        plans_bad["tree_clean"] = True
        plans_bad["cases"].pop()
        code, _ = self.run_with_docs(self.py01_doc, plans_bad)
        self.assertEqual(code, 2)

    def test_invalid_on_reference_mismatch(self):
        plans_bad = copy.deepcopy(self.plans_doc)
        plans_bad["tree_clean"] = True
        plans_bad["cases"][0]["norm_f"] *= 2.0
        code, _ = self.run_with_docs(self.py01_doc, plans_bad, verify_references=True)
        self.assertEqual(code, 2)

    def test_invalid_on_budget_mismatch(self):
        plans_bad = copy.deepcopy(self.plans_doc)
        plans_bad["tree_clean"] = True
        plans_bad["cases"][0]["total_budget"] *= 2.0
        code, _ = self.run_with_docs(self.py01_doc, plans_bad, verify_references=True)
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
