#!/usr/bin/env python3
"""Same-author adversarial science checks of original R6 source stage study."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

import numpy as np

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("vigilode_r6_causal_frontier", HERE/"experiment.py")
m=importlib.util.module_from_spec(spec)
sys.modules[spec.name]=m
spec.loader.exec_module(m)


class HostileR6(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.results=m.main()
        cls.data=json.loads((HERE/"RESULTS.json").read_text())
        cls.models=m.load_cases()
        cls.a,cls.C,cls.b,cls.gamma=m.tableau()

    def test_original_exact_source_bytes_locked(self):
        self.assertTrue(m.check_locks())
        self.assertEqual(m.BLOBS[str(m.NATIVE.relative_to(m.ROOT))],
                         "d68239d53977e36f98b186b627aa0c4c5c7de7cb")
        self.assertEqual(hashlib.sha256(m.TABLEAU.read_bytes()).hexdigest(),m.TAB_SHA256)

    def test_native_eight_stage_tableau_shape_and_strict_causality(self):
        self.assertEqual(self.a.shape,(8,8))
        self.assertEqual(self.C.shape,(8,8))
        self.assertTrue(np.all(np.triu(self.a,0)==0))
        self.assertTrue(np.all(np.triu(self.C,0)==0))
        self.assertGreater(self.gamma,0)

    def test_distinct_original_source_models_not_inflated(self):
        self.assertEqual(self.data["distinct_models"],3)
        self.assertEqual(self.data["tested_original_fixture_rows"],4)
        self.assertEqual(
            self.models["asymmetric_2"],
            self.models["asymmetric_2_refined"])
        self.assertFalse(next(r for r in self.data["records"]
                              if r["case"]=="asymmetric_2_refined")["independent_physical_model"])

    def test_causal_prefix_is_fixed_exact_with_source_solver(self):
        for name,steps in m.ORIGINAL_STEPS.items():
            k,g,c,y=self.models[name]
            h,tau,W=m.build_case(k,g,c,y,steps,self.gamma)
            seq,_=m.protected_sequential(y,k,g,c,self.a,self.C,self.b,tau,self.gamma,W)
            for sweep,frontier,charged,total in m.frontier_sweeps(
                    y,k,g,c,self.a,self.C,self.b,tau,self.gamma,W):
                self.assertLess(float(np.max(np.abs(frontier[:sweep]-seq[:sweep]))),2e-12)
                self.assertEqual(total,sum(9-q for q in range(1,sweep+1)))

    def test_full_eight_sweep_agrees_with_90digit_oracle(self):
        for row in self.data["records"]:
            final=row["frontier_q0_to_q7"][-1]
            self.assertLessEqual(final["actual_high_precision_stage_linf_error"],2e-12)
            self.assertLessEqual(final["actual_high_precision_endpoint_linf_error"],2e-12)
            self.assertLess(final["all_original_stages_residual_max"],1e-13)
            self.assertEqual(final["cumulative_original_W_solves"],36)

    def test_q2_false_acceptance_excluded(self):
        for row in self.data["records"]:
            early=row["frontier_q0_to_q7"][2]
            self.assertEqual(early["serial_stage_depth"],3)
            self.assertEqual(early["cumulative_original_W_solves"],21)
            self.assertGreater(early["actual_high_precision_endpoint_linf_error"],1e-3)
            self.assertEqual(early["native_full_step_admission"],
                             "NOT_ADMITTED_NO_ORIGINAL_EMBEDDED_ERROR_ALLOWANCE")

    def test_outer_error_is_not_monotone(self):
        for row in self.data["records"]:
            q2=row["frontier_q0_to_q7"][2]
            q3=row["frontier_q0_to_q7"][3]
            self.assertGreater(q3["actual_high_precision_endpoint_linf_error"],
                               q2["actual_high_precision_endpoint_linf_error"])

    def test_backward_exact_rational_certificate_encloses_reference(self):
        for row in self.data["records"]:
            for k in row["frontier_q0_to_q7"]:
                self.assertGreaterEqual(k["rigorous_source_scoped_endpoint_bound_inf"],
                    k["actual_high_precision_endpoint_linf_error"]-5e-14)
                self.assertGreaterEqual(k["rigorous_source_scoped_stage_bound_inf"],
                    k["actual_high_precision_stage_linf_error"]-5e-14)
                self.assertGreaterEqual(k["all_original_stages_residual_max"],0)

    def test_wrong_root_trial_not_accidentally_certified_as_exact(self):
        k,g,c,y=self.models["asymmetric_2"]
        h,tau,W=m.build_case(k,g,c,y,64,self.gamma)
        seq,_=m.protected_sequential(y,k,g,c,self.a,self.C,self.b,tau,self.gamma,W)
        corrupt=seq.copy()
        corrupt[0,0]+=0.001
        cert=m.bound_by_lower_triangle(corrupt,y,k,g,c,self.a,self.C,self.b,tau,self.gamma)
        self.assertGreater(cert["max_residual_upper"],1e-4)
        self.assertGreater(cert["endpoint_bound_upper"],1e-4)

    def test_newton_free_source_owned_W_solver_not_generic_derivative_free(self):
        for row in self.data["records"]:
            inner=row["source_owned_newton_free_first_W_Jacobi"]
            self.assertTrue(inner["no_newton"] and inner["no_jacobian_callback"])
            self.assertLess(inner["exact_original_W_residual_inf"],2e-13)
            self.assertLess(inner["contraction_ratio_bound"],1.0)
            self.assertGreater(inner["component_arithmetic_work_proxy"],0)

    def test_nonnormal_oversized_strict_lower_coupling_finite(self):
        nc=self.data["nonnormal_negative_control"]
        self.assertFalse(nc["global_contraction"])
        self.assertGreater(nc["at_three_sweeps_genuine_error"],1)
        self.assertEqual(nc["history"][-1]["max_true_error"],0.0)

    def test_lambda_polynomial_degree_and_no_fold_theorem(self):
        degrees=[0]
        for i in range(1,8):
            degrees.append(3*(1+degrees[-1]))
        self.assertEqual(degrees,[0,3,12,39,120,363,1092,3279])
        # The stage Jacobian is block-lower-triangular diag W, so determinant
        # det(W)^8 is independent of parameter lambda and stage values.
        k,g,c,y=self.models["coupled_3_cubic"]
        _,tau,W=m.build_case(k,g,c,y,128,self.gamma)
        self.assertGreater(float(np.linalg.det(W)),0)

    def test_output_identity_no_production_claim(self):
        self.assertEqual(self.data["official_dag"],"7/23")
        self.assertEqual(self.data["successor_admitted"],"0/12")
        self.assertEqual(self.data["production"],"HOLD")
        self.assertEqual(self.data["full_native_Rust_8_stage_parity"],"NOT_RUN")
        self.assertEqual(self.data["full_IVP_matched_achieved_global_error"],"NOT_RUN")
        self.assertEqual(self.data["protected_serial_full_depth_W_work"],8)
        self.assertEqual(self.data["causal_frontier_full_depth_W_work"],36)

    def test_tampered_original_staged_driver_refused(self):
        with patch.dict(m.BLOBS,{str(m.NATIVE.relative_to(m.ROOT)):"0"*40}):
            with self.assertRaises(AssertionError):
                m.check_locks()


if __name__=="__main__":
    unittest.main(verbosity=2)
