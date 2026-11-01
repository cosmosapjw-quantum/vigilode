#!/usr/bin/env python3
"""Adversarial WP9B same-author scientific and execution tests; not independent C9."""
import hashlib
import importlib.util
import json
import math
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("wp9b_science", HERE/"experiment.py")
wp9b = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(wp9b)


class WP9BReview(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases = wp9b.source_cases()

    def test_original_git_blobs_exact(self):
        wp9b.verify_locks()
        self.assertEqual(wp9b.git_blob(wp9b.SOURCE_RHS.read_bytes()),
                         wp9b.SOURCE_LOCKS["rhs"])

    def test_rust_fixture_parsing(self):
        self.assertEqual(len(self.cases), 4)
        a = self.cases["asymmetric_2"]
        self.assertEqual(a["c"].tolist(), [[0.0,12.0],[8.0,0.0]])
        self.assertEqual(a["y0"].tolist(), [0.2,0.12])
        self.assertTrue(wp9b.np.array_equal(a["c"],self.cases["asymmetric_2_refined"]["c"]))

    def test_all_weights_annihilate_exchange(self):
        for name,p in self.cases.items():
            for axis in wp9b.AXES:
                for theta in (-0.03,0,0.03):
                    k,g,c,y0,q,Q = wp9b.matrices(p,axis,theta)
                    pi = wp9b.stationary(Q)
                    self.assertGreater(min(pi),0)
                    self.assertLess(max(abs(pi @ Q)), 2e-12)
                    self.assertLess(max(abs(Q.sum(axis=1))), 1e-12)

    def test_stationary_weights_must_vary_on_one_edge(self):
        p = self.cases["asymmetric_2"]
        zero = wp9b.stationary(wp9b.matrices(p,"EXCHANGE_C01_REL",0.0)[-1])
        pert = wp9b.stationary(wp9b.matrices(p,"EXCHANGE_C01_REL",0.03)[-1])
        self.assertGreater(max(abs(pert-zero)),0.001)
        derivative = wp9b.sensitivity(p,"EXCHANGE_C01_REL",0.1)
        self.assertGreater(abs(derivative["pi_term"]),1e-4)
        self.assertAlmostEqual(derivative["first_derivative"],-0.0169628408664,delta=3e-8)

    def test_linear_null_cubic_and_uniform_exchange(self):
        p = self.cases["linear_exchange_control"]
        for axis in ("CUBIC_ALL_REL","EXCHANGE_ALL_REL"):
            r = wp9b.evaluate(p,"linear_exchange_control",axis)
            self.assertTrue(r["valid"])
            self.assertLess(abs(r["Wp"]-r["Wm"]),1e-10)
            self.assertLess(abs(r["Wp"]+r["Wm"]-2*r["W0"]),1e-10)

    def test_tangent_source_second_order_nonconservation(self):
        p = self.cases["coupled_3_cubic"]
        x = wp9b.sensitivity(p,"DECAY_ALL_REL",0.1)
        self.assertAlmostEqual(x["first_derivative"],-0.014955924658,delta=3e-8)

    def test_positive_priors_and_mutation_changes_only_corrected(self):
        cases = wp9b.candidate_set(self.cases)
        obs = {(cases[0][0],cases[0][1]): 0.9}
        corr, mut = wp9b.priors(cases,obs,True)
        static, static_mut = wp9b.priors(cases,obs,False)
        self.assertAlmostEqual(sum(corr),1)
        self.assertAlmostEqual(sum(mut),1)
        self.assertTrue(all(x >= wp9b.EPSILON/len(cases)-1e-14 for x in corr))
        self.assertNotEqual(corr,static)
        self.assertNotEqual(mut,static_mut)
        self.assertEqual(len(cases),19)

    def test_refined_source_cannot_double_count_physics(self):
        candidates=wp9b.candidate_set(self.cases)
        a=("asymmetric_2","DECAY_ALL_REL")
        b=("asymmetric_2_refined","DECAY_ALL_REL")
        one=wp9b.compute_scores(candidates,{a:0.4})
        two=wp9b.compute_scores(candidates,{a:0.4,b:0.4})
        self.assertEqual(one[0],two[0])
        self.assertEqual(one[1],two[1])
        self.assertEqual(wp9b.canonical_candidate(b),a)
        self.assertEqual(len({wp9b.canonical_candidate(c) for c in candidates}),14)

    def test_output_integrity_and_holdout_after_frozen(self):
        result_file=HERE/"RESULTS.json"
        if not result_file.exists():
            self.skipTest("Result not yet generated")
        data=json.loads(result_file.read_text())
        self.assertEqual(data["parent_wp9_head"],
                         "1e4acb486fe5f497594b9cf040e9b791aca1e39b")
        self.assertEqual(data["pre_simulation_prereg_commit"],
                         "3734004ebaaa5b5ce65f692791faac9a25bb828b")
        self.assertTrue(data["measurement"]["all_physical_consensus_pass"])
        self.assertEqual(len(data["arms"]),2)
        for row in data["arms"]:
            self.assertEqual(row["waves"],16)
            self.assertEqual(row["observed_pairs"],16)
            self.assertTrue(row["ProgramDatabase_real_vendor"])
        self.assertEqual(len(data["holdout"]),2)
        for key in ("SEARCH_TRACE.jsonl","TRAJECTORIES.jsonl","PHYSICAL_EXPERIMENTS.jsonl"):
            self.assertEqual(hashlib.sha256((HERE/key).read_bytes()).hexdigest(),
                             data[key+"_sha256"])
        trace=[json.loads(line) for line in (HERE/"SEARCH_TRACE.jsonl").read_text().splitlines()]
        self.assertEqual(len(trace),32)
        for arm in ("corrected","uniform_control"):
            waves=[t for t in trace if t["arm"]==arm]
            self.assertEqual(len(waves),16)
            self.assertEqual([x["wave"] for x in waves],list(range(1,17)))
            self.assertEqual([x["operator"] for x in waves[::2]],["OpenEvolve_mutation"]*8)
            self.assertEqual([x["operator"] for x in waves[1::2]],["MCTS_PUCT"]*8)
            self.assertTrue(all(x["n_programs"]>=1 for x in waves))
        adap=[t for t in trace if t["arm"]=="corrected"]
        null=[t for t in trace if t["arm"]=="uniform_control"]
        self.assertTrue(any(max(abs(a-b) for a,b in zip(t["prior_before"],t["prior_after"]))>1e-7 for t in adap))
        self.assertTrue(any(max(abs(a-b) for a,b in zip(t["mutation_before"],t["mutation_after"]))>1e-7 for t in adap))
        self.assertTrue(all(t["prior_before"]==t["prior_after"] for t in null))
        self.assertTrue(all(t["mutation_before"]==t["mutation_after"] for t in null))

    def test_vendor_is_source_authentic(self):
        # A source ZIP is verified in production run; direct import must identify actual module.
        result_file=HERE/"RESULTS.json"
        if not result_file.exists():
            self.skipTest("Result not yet generated")
        data=json.loads(result_file.read_text())
        self.assertEqual(data["openevolve_source_zip_sha256"],wp9b.VENDOR_ZIP_LOCK)
        self.assertEqual(data["external_llm_calls"],0)
        self.assertEqual(data["official_vigilode_dag"],"7/23")
        self.assertEqual(data["production"],"HOLD")


if __name__ == "__main__":
    unittest.main(verbosity=2)
