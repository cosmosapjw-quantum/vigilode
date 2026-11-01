#!/usr/bin/env python3
"""One bounded hostile same-author R9 review of graph/Schur raw-U science."""
from __future__ import annotations
import hashlib, importlib.util, json, sys, unittest
from fractions import Fraction as F
from unittest.mock import patch
from pathlib import Path

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("vigilode_r9_cycle_core_science",HERE/"experiment.py")
m=importlib.util.module_from_spec(spec)
sys.modules[spec.name]=m
spec.loader.exec_module(m)


class R9Hostile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data=m.main()
        cls.older=m.load_original()
        cls.cases=cls.older.load_cases()

    def test_original_source_identity_pinned(self):
        self.assertTrue(self.older.check_locks())
        self.assertEqual(hashlib.sha256((m.R8.parent/"RESULTS.json").read_bytes()).hexdigest(),
                         m.SOURCE_R8_SHA)
        self.assertEqual(hashlib.sha256(m.R7.read_bytes()).hexdigest(),m.SOURCE_R7_SHA)

    def test_original_and_synthetic_classification_precise(self):
        self.assertEqual(self.data["source_original_cases"],4)
        self.assertEqual(self.data["source_original_physical_independent"],3)
        self.assertEqual(self.data["synthetic_algorithmic_controls"],4)
        self.assertEqual(len(self.data["rows"]),8)
        self.assertEqual(self.cases["asymmetric_2"],self.cases["asymmetric_2_refined"])

    def test_true_original_eight_stage_source_replay(self):
        self.assertEqual(self.data["first_step_total_8stage_raw_U_solves"],64)
        for r in self.data["rows"]:
            self.assertEqual(r["original_RAW_U_stages"],8)
            self.assertTrue(r["exact_Cayley_Schur"])
            self.assertLess(r["full_first_step_endpoint_linf_vs_protected"],1e-11)
            if r["kind"]=="ORIGINAL_FROZEN":
                self.assertIsNotNone(r["full_first_step_endpoint_linf_vs_90digit"])
                self.assertLessEqual(
                    r["full_first_step_endpoint_linf_vs_90digit"],
                    r["exact_rational_first_step_endpoint_bound"]+5e-14)
            else:
                self.assertIsNone(r["full_first_step_endpoint_linf_vs_90digit"])

    def test_graph_fvs_always_verified(self):
        for r in self.data["rows"]:
            self.assertLessEqual(r["B_size"],3)
            self.assertEqual(r["A_size"]+r["B_size"],r["n"])
            self.assertLessEqual(r["dag_longest_path_depth"],max(0,r["A_size"]-1))

    def test_exact_schur_core_ch_inverse(self):
        for row in self.data["rows"]:
            if row["name"] in m.synthetic.__name__:continue
            if row["kind"]=="ORIGINAL_FROZEN":
                k,g,c,y=self.cases[row["name"]]
            else:k,g,c,y,_=m.synthetic(row["name"],row["n"])
            P=m.create_provider(k,g,c,y,row["tau"],row["B"])
            S=P["core_matrix"]; inv=P["core_inverse"];n=len(S)
            for i in range(n):
                for j in range(n):
                    actual=sum(S[i][a]*inv[a][j] for a in range(n))
                    self.assertEqual(actual,F(int(i==j)))

    def test_mmatrix_source_row_contraction(self):
        for row in self.data["rows"]:
            self.assertGreaterEqual(row["M_q"],0)
            self.assertLess(row["M_q"],1)
            self.assertGreater(row["M_residual_margin"],1)

    def test_nontrivial_arbitrary_n_with_one_cycle(self):
        for n in (12,24):
            r=next(x for x in self.data["rows"] if x["name"]==f"ring{n}")
            self.assertEqual(r["B_size"],1)
            self.assertEqual(r["dag_longest_path_depth"],n-2)
            self.assertGreater(r["M_q"],.2)

    def test_nontrivial_two_cycle_core_and_dag(self):
        r=next(x for x in self.data["rows"] if x["name"]=="two_cycles14")
        self.assertEqual(r["B_size"],2)
        self.assertEqual(r["dag_longest_path_depth"],10)
        self.assertGreater(r["Schur_nonzero_entries"],1)

    def test_acyclic_special_case_no_feedback_vars(self):
        r=next(x for x in self.data["rows"] if x["name"]=="dag8")
        self.assertEqual(r["B"],[])
        self.assertEqual(r["Schur_nonzero_entries"],0)
        self.assertEqual(r["dag_longest_path_depth"],7)

    def test_invalid_declared_feedback_set_refused(self):
        k,g,c,y,B=m.synthetic("dense8_refuse",8)
        with self.assertRaisesRegex(ValueError,"A_GRAPH_HAS_CYCLE"):
            m.create_provider(k,g,c,y,.02,B)

    def test_core_too_large_refused_even_if_small_cycles(self):
        k,g,c,y,B=m.synthetic("four_cycles_refuse",8)
        with self.assertRaisesRegex(ValueError,"CORE_TOO_LARGE"):
            m.create_provider(k,g,c,y,.02,B)

    def test_negative_coupling_refused(self):
        k,g,c,y=m.synthetic("ring12",12)[:4]
        c[0][1]=-3.0
        with self.assertRaises(AssertionError):
            m.create_provider(k,g,c,y,.02,[0])

    def test_zero_or_negative_tau_refused(self):
        k,g,c,y,B=m.synthetic("ring12",12)
        for tau in (0,-.001):
            with self.assertRaises(AssertionError):
                m.create_provider(k,g,c,y,tau,B)

    def test_false_source_identity_refused(self):
        with patch.object(m,"SOURCE_R8_SHA","0"*64):
            with self.assertRaises(AssertionError):
                m.load_original()

    def test_no_universal_no_go_or_native_full_IVP_claim(self):
        self.assertEqual(self.data["full_native_production_speed"],"NOT_RUN")
        self.assertEqual(self.data["entire_ODE_global_achieved_error"],"NOT_RUN")
        self.assertEqual(self.data["official_DAG"],"7/23")
        self.assertEqual(self.data["successor_admitted"],"0/12")
        self.assertEqual(self.data["production"],"HOLD")
        self.assertTrue(self.data["no_existing_solver_mutation"])
        self.assertEqual(len(self.data["negative_controls"]),3)

if __name__=="__main__":
    unittest.main(verbosity=2)
