#!/usr/bin/env python3
"""Same-author hostile R7 science/lineage audit (not independent C9)."""
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import struct
import sys
import unittest
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
SPEC=importlib.util.spec_from_file_location("native_r7_analysis",HERE/"analyze.py")
m=importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name]=m
SPEC.loader.exec_module(m)


class HostileR7(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        m.main()
        cls.d=json.loads((HERE/"RESULTS.json").read_text())
        cls.reference=json.loads((HERE/"REFERENCE.json").read_text())
        cls.native=[json.loads(x) for x in (HERE/"NATIVE.jsonl").read_text().splitlines()]

    def test_exact_wp3_rust_model_source_is_pinned(self):
        original=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
        self.assertEqual(m.subprocess.check_output(["git","hash-object",str(original)],text=True).strip(),
                         m.EXPECTED_SOURCE_BLOB)
        self.assertEqual(self.d["original_frozen_wp3_git_blob"],m.EXPECTED_SOURCE_BLOB)

    def test_all_native_full_ivps_have_complete_requested_time_bits(self):
        self.assertEqual(self.d["full_native_runs"],24)
        self.assertEqual(self.d["successful_full_native_grid_runs"],24)
        self.assertEqual(len(self.native),28)
        self.assertTrue(all(r["native_result_complete"] for r in self.d["all_source_and_method_records"]))
        self.assertTrue(all(r["output_bit_deterministic"] for r in self.d["all_source_and_method_records"]))

    def test_native_traces_are_true_rust_implementations_not_python_surrogates(self):
        methods={r["method"] for r in self.d["all_source_and_method_records"]}
        self.assertEqual(methods,set(m.ROUTES))
        original=ROOT/"crates/rodas5p-integrators/examples/vigilode_r7_native_comparison.rs"
        text=original.read_text()
        self.assertIn("integrate_sequential_matrix_free_adaptive_dense_observed",text)
        self.assertIn("integrate_transactional_q1_q2_adaptive_dense_observed",text)
        self.assertIn("struct OriginalSource",text)
        self.assertNotIn("def solve_ivp",text)

    def test_original_cases_identical_but_resolution_control_not_new_model(self):
        self.assertEqual(self.d["distinct_physics_models"],3)
        self.assertEqual(self.d["resolution_control_models"],1)
        self.assertEqual({r["physics_family"] for r in self.d["all_source_and_method_records"]},
                         {"asymmetric_2","coupled_3_cubic","linear_exchange_control"})
        controls=[r for r in self.d["all_source_and_method_records"] if r["resolution_control"]]
        self.assertEqual(len(controls),6)

    def test_reference_methods_agree_at_full_requested_grid(self):
        self.assertTrue(self.reference["all_reference_consensus_pass"])
        for r in self.reference["rows"]:
            self.assertLessEqual(r["cross_solver_state_linf"],
                                 self.reference["consensus_absolute_limit"])
            self.assertEqual(len(r["grid_time_bits"]),r["original_steps"]+1)

    def test_whole_ivp_achieved_error_strictly_improves_with_tolerance(self):
        for src in ("asymmetric_2","asymmetric_2_refined",
                    "coupled_3_cubic","linear_exchange_control"):
            for method in m.ROUTES:
                rows=sorted((r for r in self.d["all_source_and_method_records"]
                             if r["source"]==src and r["method"]==method),
                            key=lambda r:r["rtol"],reverse=True)
                self.assertEqual([r["rtol"] for r in rows],list(m.RTOLS))
                errors=[r["global_error_max"] for r in rows]
                self.assertGreater(errors[0],errors[1])
                self.assertGreater(errors[1],errors[2])

    def test_original_native_work_is_not_unaccounted(self):
        for row in self.d["all_source_and_method_records"]:
            w=row["reported_work"]
            self.assertEqual(w["accepted_steps"],row["internal_accepted_steps"])
            self.assertEqual(w["rejected_steps"],row["rejected_steps"])
            self.assertGreater(w["jvp_vectors"],0)
            self.assertEqual(w["jacobian_builds"],0)
            self.assertEqual(w["nonlinear_iterations"],0)
            self.assertGreater(row["median_wall_ns"],0)

    def test_native_transactional_actual_lane_counts_sum_up(self):
        trans=[r for r in self.d["all_source_and_method_records"]
               if r["method"]==m.ROUTES[1]]
        self.assertEqual(len(trans),12)
        for row in trans:
            self.assertEqual(row["native_q1_accepted"]+
                             row["native_q2_accepted"]+
                             row["native_fallback_accepted"],
                             row["internal_accepted_steps"])
            self.assertGreater(row["native_critical_depth_total"],0)
        self.assertEqual(sum(r["native_q1_accepted"] for r in trans),8)
        self.assertEqual(sum(r["native_q2_accepted"] for r in trans),113)
        self.assertEqual(sum(r["native_fallback_accepted"] for r in trans),0)

    def test_no_pretense_native_q2_equals_naive_frontier(self):
        self.assertTrue(all("native_transactional" in r["diagnostics"]
                            for r in self.d["all_source_and_method_records"]
                            if r["method"]==m.ROUTES[1]))
        self.assertEqual(self.d["fused_exponential_comparison_same_dense_grid"],
                         "NOT_RUN_CAPABILITY_DIFFERENCE")

    def test_true_matched_error_interpolation_within_both_measured_domains(self):
        matched=self.d["matched_error_interpolated_comparisons"]
        self.assertEqual(len(matched),4)
        for r in matched:
            target=r["achieved_error_target_interpolated"]
            for route in m.ROUTES:
                err=[row["global_error_max"] for row in self.d["all_source_and_method_records"]
                     if row["source"]==r["source"] and row["method"]==route]
                self.assertGreaterEqual(target,min(err))
                self.assertLessEqual(target,max(err))
            self.assertAlmostEqual(
                r["protected_over_transactional_speed_ratio"],
                r["protected_interpolated_wall_ns"]/r["transactional_interpolated_wall_ns"],
                places=12)

    def test_negative_result_on_three_independent_original_sources(self):
        matched=[r for r in self.d["matched_error_interpolated_comparisons"]
                 if not r["resolution_control"]]
        self.assertEqual(len(matched),3)
        self.assertTrue(all(0<r["protected_over_transactional_speed_ratio"]<1 for r in matched))
        self.assertLess(self.d["median_matched_error_protected_over_transactional"],0.3)

    def test_all_raw_input_hashes_match_frozen_results(self):
        for fname,expected in self.d["input_sha256"].items():
            self.assertEqual(hashlib.sha256((HERE/fname).read_bytes()).hexdigest(),expected)

    def test_fingerprint_matches_original_reproducible_build(self):
        fingerprint=(HERE/"RUN_FINGERPRINT.txt").read_text()
        self.assertIn("SOURCE_WP3_BLOB="+m.EXPECTED_SOURCE_BLOB,fingerprint)
        self.assertIn("BUILD_RUSTC=rustc",fingerprint)
        self.assertIn("BINARY_SHA256=",fingerprint)
        self.assertIn("SOURCE_GIT_HEAD=703fc3d72f42f460cb7b33f72bdb7777816e6b41",fingerprint)

    def test_no_false_interval_certificate_or_production_promotion(self):
        self.assertTrue(self.d["reference_is_numerical_not_interval_certified"])
        self.assertEqual(self.d["official_dag"],"7/23")
        self.assertEqual(self.d["successor_admitted"],"0/12")
        self.assertEqual(self.d["production"],"HOLD")
        self.assertTrue(self.d["no_native_q2_default_winner"])
        self.assertEqual(self.d["stiffness_generalization"],
                         "NOT_ESTABLISHED_THREE_SMALL_CUBIC_MODELS_ONLY")

    def test_tampered_original_source_is_refused_before_analysis(self):
        with patch.object(m,"EXPECTED_SOURCE_BLOB","0"*40):
            with self.assertRaises(AssertionError):
                m.main()


if __name__=="__main__":
    unittest.main(verbosity=2)
