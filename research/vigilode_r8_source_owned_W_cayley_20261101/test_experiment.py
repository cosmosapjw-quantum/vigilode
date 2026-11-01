#!/usr/bin/env python3
"""Adversarial same-author R8 tests. No independent C9 claim."""
from __future__ import annotations
from fractions import Fraction as F
import hashlib, importlib.util, json, math, unittest
from pathlib import Path
from unittest.mock import patch
import numpy as np

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("vigilode_r8_algebraic", HERE/"experiment.py")
import sys
m=importlib.util.module_from_spec(spec);sys.modules[spec.name]=m;spec.loader.exec_module(m)


class R8Hostile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result=m.main()
        cls.mod=m.source()
        cls.cases=cls.mod.load_cases()

    def test_original_blob_and_r7_native_science_identity(self):
        self.assertTrue(self.mod.check_locks())
        self.assertEqual(hashlib.sha256(m.R7.read_bytes()).hexdigest(),m.SOURCE_LOCK["R7_result"])
        self.assertEqual(self.result["original_R7_transactional_decision"],"HOLD_DEFAULT_NATIVE_Q2_FOR_THESE_SOURCE_CLASSES")

    def test_no_model_duplication_in_science_claim(self):
        self.assertEqual(self.result["source_physics_family_count"],3)
        self.assertEqual(self.result["total_full_original_first_step_rows"],4)
        self.assertEqual(self.result["total_full_original_RAW_U_stage_solves"],32)
        self.assertEqual(self.cases["asymmetric_2"],self.cases["asymmetric_2_refined"])

    def test_exact_Cayley_Hamilton_inverse_and_determinant_positive(self):
        for row in self.result["total_source_family_rows"]:
            k,g,c,y=self.cases[row["source"]]
            A=m.exact_inverse(k,g,c,y,row["tau"])
            self.assertGreater(A["det"],0)
            n=row["n"]
            self.assertEqual(m.matmul(A["W"],A["inv"]),[[F(int(i==j)) for j in range(n)] for i in range(n)])
            self.assertEqual(len(A["charpoly"]),n+1)

    def test_positive_M_matrix_and_strict_contraction(self):
        for row in self.result["total_source_family_rows"]:
            self.assertGreater(row["q_infty"],0)
            self.assertLess(row["q_infty"],1)
            self.assertGreater(row["M_inverse_inf_bound"],0)

    def test_all_original_eight_stage_residuals_bounded(self):
        for row in self.result["total_source_family_rows"]:
            self.assertEqual(len(row["CH_stage_details"]),8)
            self.assertLess(row["whole_eight_stage_rational_residual_bound"],1e-12)
            self.assertLessEqual(row["CH_against_reference_endpoint_linf"],
                                 row["whole_eight_stage_rational_residual_bound"]+5e-14)

    def test_neumann_and_residual_bound_with_exact_arithmetic(self):
        for row in self.result["total_source_family_rows"]:
            self.assertEqual(len(row["first_Raw_U_neumann"]),10)
            for r in row["first_Raw_U_neumann"]:
                self.assertGreaterEqual(r["residual_bound"],r["error"]-1e-17)
                self.assertGreaterEqual(r["geometric_bound"],r["error"]-1e-17)
            self.assertLessEqual(row["first_K_meeting_1e_minus_12"],3)

    def test_signed_H_matrix_counterexample_is_not_M_matrix(self):
        control=self.result["non_Metzler_signed_H_matrix_branch"]
        self.assertFalse(control["is_M_matrix"])
        self.assertLessEqual(F(control["actual_inverse_infty"]),F(control["bound"]))
        self.assertFalse(control["nonnegative_inverse_claim"])

    def test_negative_cubic_coefficient_refused(self):
        k,g,c,y=self.cases["asymmetric_2"]
        g=list(g);g[0]=-1.
        with self.assertRaises(AssertionError):
            m.exact_inverse(k,g,c,y,0.01)

    def test_negative_coupling_refused_not_scoped_certification(self):
        k,g,c,y=self.cases["asymmetric_2"]
        cc=[row[:] for row in c];cc[0][1]=-1.
        with self.assertRaises(AssertionError):
            m.exact_inverse(k,g,cc,y,.01)

    def test_dimension_four_not_promoted(self):
        with self.assertRaisesRegex(AssertionError,"SOURCE_DIMENSION_NOT_SUPPORTED"):
            m.exact_inverse([1.]*4,[1.]*4,[[0.]*4 for _ in range(4)],[.1]*4,.01)

    def test_tampered_R7_source_refused(self):
        with patch.dict(m.SOURCE_LOCK,{"R7_result":"0"*64}):
            with self.assertRaises(AssertionError):
                m.source()

    def test_conclusions_and_no_false_production(self):
        self.assertEqual(self.result["actual_native_full_IVP_CH"],"NOT_RUN")
        self.assertEqual(self.result["production"],"HOLD")
        self.assertEqual(self.result["frozen_original_DAG"],"7/23")
        self.assertFalse(self.result["default_or_tolerance_changed"])
        self.assertEqual(self.result["first_stage_Neumann_probes"],40)


if __name__=="__main__": unittest.main(verbosity=2)
