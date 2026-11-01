#!/usr/bin/env python3
"""R10 same-author hostile test: original ROW source, scan math and evidence."""
from __future__ import annotations
import hashlib, importlib.util, json, math, sys, unittest
from pathlib import Path
from fractions import Fraction as F
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("vigilode_r10_scan",HERE/"experiment.py")
m=importlib.util.module_from_spec(spec)
sys.modules[spec.name]=m
spec.loader.exec_module(m)


class R10Hostile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.results=m.main()
        cls.r9=m.load()
        cls.r6=cls.r9.load_original()

    def test_r9_source_exact_provenance(self):
        self.assertEqual(hashlib.sha256(m.R9_DATA.read_bytes()).hexdigest(),m.R9_SHA)
        self.assertTrue(self.r6.check_locks())
        self.assertEqual(self.results["previous_R9_exact_source_sha256"],m.R9_SHA)

    def test_n_stage_counts_and_physics_scope(self):
        self.assertEqual(self.results["actual_original_raw_U_mathematical_stage_comparisons"],64)
        self.assertEqual(self.results["original_source_cases"],4)
        self.assertEqual(self.results["original_physical_independent"],3)
        self.assertEqual(self.results["synthetic_eight_stage_controls"],4)
        self.assertEqual(self.results["additional_synthetic_one_stage_exact_checks"],2)

    def test_exact_stage_source_and_R9_graph_serial_agreement(self):
        self.assertTrue(all(r["mathematically_exact_same_as_R9_serial"]
                            for r in self.results["verified_rows"]))
        parent=json.loads(m.R9_DATA.read_text())
        for r in self.results["verified_rows"]:
            p=next(v for v in parent["rows"] if v["name"]==r["name"])
            self.assertEqual(r["max_first_step_original_endpoint_error_upper"],
                             p["exact_rational_first_step_endpoint_bound"])

    def test_exact_associativity_and_noncommutative_adversary(self):
        pairs=[(F(2),F(1)),(F(3),F(4)),(F(5),F(7))]
        a,b,c=pairs
        self.assertEqual(m.compose(m.compose(a,b),c),m.compose(a,m.compose(b,c)))
        self.assertNotEqual(m.compose(a,b),m.compose(b,a))

    def test_dag8_full_path_depth_and_operations(self):
        v=next(r for r in self.results["verified_rows"] if r["name"]=="dag8")
        self.assertEqual((v["path_serial_depth"],v["path_parallel_scan_layers"]),(7,3))
        self.assertEqual(v["parallel_affine_pairs_per_apply"],17)

    def test_ring24_parallel_depth_and_work_are_separate(self):
        v=next(r for r in self.results["verified_rows"] if r["name"]=="ring24")
        self.assertEqual((v["path_serial_depth"],v["path_parallel_scan_layers"]),(22,5))
        self.assertEqual((v["serial_edges_per_path_apply"],v["parallel_affine_pairs_per_apply"]),(22,84))
        self.assertLess(v["path_parallel_scan_layers"],v["path_serial_depth"])
        self.assertGreater(v["parallel_affine_pairs_per_apply"],v["serial_edges_per_path_apply"])

    def test_two_component_cycle_core(self):
        v=next(r for r in self.results["verified_rows"] if r["name"]=="two_cycles14")
        self.assertEqual(v["path_count"],2)
        self.assertEqual(v["B_size"],2)
        self.assertEqual(v["path_serial_depth"],10)

    def test_extra_ring96_source_model_is_synthetic_only(self):
        v=next(r for r in self.results["extended_ring_controls"] if r["name"]=="ring96")
        self.assertEqual((v["serial_depth"],v["scan_layers"]),(94,7))
        self.assertEqual((v["sequential_edge_operations"],v["affine_scan_pairs"]),(94,538))
        self.assertTrue(v["exact_original_W_residual_zero"])
        self.assertEqual(v["whole_eight_stage_comparison"],"NOT_RUN")
        self.assertEqual(v["type"],"SYNTHETIC_ONE_LINEAR_STAGE_ONLY")

    def test_branching_DAG_refused_not_falsely_promoted(self):
        ref=self.results["rejected_controls"][0]
        self.assertEqual(ref["category"],"REJECTED_PATH_ONLY")
        self.assertTrue(ref["general_DAG_R9_serial_still_valid"])
        self.assertTrue(ref["reason"].startswith("NOT_PATH_FOREST_"))

    def test_signed_noncooperative_negative_source_refusal(self):
        k,g,c,y,B=self.r9.synthetic("ring12",12)
        cc=[row[:] for row in c];cc[0][1]=-1.
        with self.assertRaises(AssertionError):
            m.ScanProvider(self.r9,k,g,cc,y,.001,B)

    def test_tampered_prior_data_refused(self):
        with patch.object(m,"R9_SHA","0"*64):
            with self.assertRaises(AssertionError):
                m.load()

    def test_G0_stop_gate_no_native_gain_claim(self):
        self.assertEqual(self.results["official_DAG"],"7/23")
        self.assertEqual(self.results["successor_admitted"],"0/12")
        self.assertEqual(self.results["production"],"HOLD")
        self.assertEqual(self.results["scan_actual_native_parallel_speed"],"NOT_RUN")
        self.assertFalse(not self.results["no_production_mutation"])

    def test_all_dag_paths_scanned_in_log_rounds(self):
        for r in self.results["verified_rows"]:
            maxlen=max(r["path_lengths"],default=0)
            rounds=(maxlen-1).bit_length() if maxlen else 0
            self.assertEqual(r["path_parallel_scan_layers"],rounds)
            self.assertLessEqual(r["path_parallel_scan_layers"],r["path_serial_depth"])

    def test_signed_affine_map_mathematics_not_source_positivity(self):
        a=(F(-2),F(3));b=(F(5),F(-7))
        self.assertEqual(m.compose(a,b),(F(-10),F(17)))
        # This arithmetic is valid for signed inputs, but not a nonnegative
        # M matrix inverse or a standalone unknown-RHS stage certificate.

if __name__=="__main__":unittest.main(verbosity=2)
