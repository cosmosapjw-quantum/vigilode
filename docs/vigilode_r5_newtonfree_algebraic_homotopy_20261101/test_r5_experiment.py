#!/usr/bin/env python3
"""Single-author hostile R5 unit tests; distinct author C9 is not implied."""
from __future__ import annotations
import copy
import hashlib
import importlib.util
import math
import unittest
from fractions import Fraction as F
from pathlib import Path
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("vigilode_r5_experiment",HERE/"r5_experiment.py")
m=importlib.util.module_from_spec(spec)
import sys
sys.modules[spec.name]=m
spec.loader.exec_module(m)


class R5Hostile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases=m.locked_cases()
        cls.result=m.run()

    def test_source_hard_locked(self):
        for key,digest in m.LOCKS.items():
            self.assertEqual(m.blob((m.ROOT/key).read_bytes()),digest)
        self.assertEqual(hashlib.sha256(m.TABLEAU.read_bytes()).hexdigest(),m.LOCK_TABLEAU_SHA256)

    def test_source_alias_not_physical_replication(self):
        self.assertEqual(self.result["source_families_distinct"],3)
        self.assertEqual(self.result["cohorts_with_resolution_control"],4)
        a=m.params(self.cases["asymmetric_2"])
        b=m.params(self.cases["asymmetric_2_refined"])
        self.assertEqual(a,b)

    def test_fixed_point_strict_contraction_in_actual_cohorts(self):
        for name,model in self.cases.items():
            k,g,c,b=m.params(model)
            steps=64 if name=="asymmetric_2" else 128
            for lam in m.LAMBDAS:
                q=m.q_contraction(k,c,1/steps,lam)
                self.assertGreaterEqual(q,0)
                self.assertLess(q,1)

    def test_global_nonlinear_source_residual(self):
        for item in self.result["records"]:
            n=item["source"];k,g,c,b=m.params(self.cases[n])
            for arm in ("newton_free_homotopy","baseline_value_only_picard"):
                for row in item[arm]["path"]:
                    rr=m.nonlinear_residual_exact(row["state"],k,g,c,b,item["tau_stage_only"],row["lambda"])
                    assert all(isinstance(x,F) for x in rr)
                    exact_bound=max(abs(v) for v in rr)
                    self.assertLessEqual(exact_bound,F.from_float(row["residual_inf_upper"]))
                    m_exact=min(
                        1+F.from_float(item["tau_stage_only"])*
                        (F.from_float(k[i])+(1-F.from_float(row["lambda"]))*sum(map(F.from_float,c[i])))
                        for i in range(len(k))
                    )
                    self.assertLessEqual(exact_bound/m_exact,F.from_float(row["error_bound_inf_upper"]))
                    self.assertTrue(row["residual_gate"])

    def test_homotopy_real_branch_monotone(self):
        for item in self.result["records"]:
            k,g,c,b=m.params(self.cases[item["source"]])
            self.assertTrue(m.assert_path_and_margins(
                item["newton_free_homotopy"]["path"],k,g,c,b,item["tau_stage_only"]))

    def test_no_newton_or_callback(self):
        self.assertEqual(self.result["newton_iterations_runtime"],0)
        self.assertEqual(self.result["jacobian_callbacks_runtime"],0)
        for r in self.result["records"]:
            self.assertEqual(r["newton_free_homotopy"]["counters"]["rejected"],0)
            self.assertFalse(r["rodas_original_first_U_stage"]["requires_callback_JVP"])

    def test_original_first_Rodas_U_residual_exact(self):
        for x in self.result["records"]:
            r=x["rodas_original_first_U_stage"]
            self.assertLess(r["max_original_raw_U_residual_exact_upper"],1e-13)
            self.assertLess(r["max_stage_error_bound_exact_upper"],1e-13)
            self.assertTrue(r["not_full_8stage_trajectory"])
            self.assertGreater(r["Jacobi_component_operations"],0)

    def test_negative_cubic_has_three_real_roots(self):
        neg=self.result["negative_control"]
        self.assertFalse(neg["source_admissible"])
        for z in (-math.sqrt(2),0.0,math.sqrt(2)):
            self.assertAlmostEqual(2*z-z**3,0,delta=2e-15)
        self.assertEqual(len(neg["roots_exact"]),3)

    def test_negative_coupling_must_refuse(self):
        model=copy.deepcopy(self.cases["asymmetric_2"])
        model["c"][0,1]=-1.0
        with self.assertRaises(AssertionError):
            m.params(model)

    def test_bad_frozen_source_object_fails_closed(self):
        with patch.dict(m.LOCKS,{str(m.RHS.relative_to(m.ROOT)):"0"*40}):
            with self.assertRaisesRegex(AssertionError,"SOURCE_IDENTITY"):
                m.locked_cases()

    def test_homotopy_not_a_speed_claim(self):
        self.assertEqual(self.result["full_native_8stage_and_same_error"],"NOT_RUN")
        self.assertEqual(self.result["production"],"HOLD")
        self.assertTrue(all(
            r["newton_free_homotopy"]["counters"]["scalar_cubic_evals"]>=
            r["baseline_value_only_picard"]["counters"]["scalar_cubic_evals"]
            for r in self.result["records"]
        ))
        self.assertEqual(self.result["original_dag"],"7/23")

    def test_exact_discriminant_structure(self):
        # For a*x^3+d*x-b, discriminant=-a*(4*d^3+27*a*b^2).
        # If a>0 and d>0 this is strictly negative, hence exactly one real root.
        for n,p in self.cases.items():
            k,g,c,b=m.params(p)
            for i in range(len(k)):
                a=F.from_float(g[i])*F(1,128)
                dd=1+F(1,128)*(F.from_float(k[i])+sum(map(F.from_float,c[i])))
                bb=F.from_float(b[i])
                disc=-a*(4*dd**3+27*a*bb**2)
                if a>0:self.assertLess(disc,0)
                else:self.assertEqual(disc,0)

if __name__=="__main__":
    unittest.main(verbosity=2)
