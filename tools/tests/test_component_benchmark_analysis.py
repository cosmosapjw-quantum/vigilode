import math
import importlib.util
import tempfile
import unittest
from pathlib import Path
from component_benchmark.analysis import interval_index, paired_log_cost, paired_median_ci, reference_gate, recommendation, same_achieved_accuracy
from component_benchmark.protocol import digest, file_digest, physical_input_digest
from component_benchmark.runner import atomic_json, save_receipt
from test_component_benchmark_protocol import CASE

_script = Path(__file__).resolve().parents[1] / "component_benchmark.py"
_spec = importlib.util.spec_from_file_location("component_cli_for_test",_script)
_cli = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_cli)

class AnalysisTest(unittest.TestCase):
    def test_reference_uses_reference_denominator_and_uncertainty_ceiling(self):
        case={**CASE,"rtol":.1,"atol":[.01]}
        gate=reference_gate(case,[[1.0],[1.05]],[[1.0],[1.0]],[[0.0],[.0]])
        self.assertAlmostEqual(gate["max_error_budget_ratio"],.05/.11)
        self.assertAlmostEqual(gate["achieved_error_lower"],.05/.11)
        self.assertAlmostEqual(gate["achieved_error_upper"],.05/.11)
        self.assertEqual(reference_gate(case,[[1.0],[1.0]],[[1.0],[1.0]],[[.006],[0]])["status"],"HOLD")

    def test_achieved_interval_overlap_and_zero_rule(self):
        b={"status":"PASS","achieved_error_lower":.4,"achieved_error_upper":.5}
        c={"status":"PASS","achieved_error_lower":.4,"achieved_error_upper":.5}
        self.assertEqual(same_achieved_accuracy(b,c)["status"],"HOLD")
        exact={"status":"PASS","achieved_error_lower":0.0,"achieved_error_upper":0.0}
        self.assertEqual(same_achieved_accuracy(exact,exact)["status"],"PASS")
        self.assertEqual(same_achieved_accuracy(exact,b)["status"],"HOLD")

    def test_exact_interval_and_hold(self):
        self.assertEqual(interval_index(17,1),5)
        ci=paired_median_ci([math.log(1.2)]*17,1)
        self.assertEqual(ci["status"],"INTERVAL")
        self.assertEqual(recommendation(True,"PASS",ci,critical_regression=False)["action"],"promote")
        self.assertEqual(paired_median_ci([1.0]*16,1)["status"],"HOLD")
        inconclusive=paired_median_ci([math.log(1.02)]*17,1)
        decision=recommendation(True,"PASS",inconclusive)
        self.assertEqual((decision["action"],decision["evidence_status"]),("need to optimize","HOLD"))

    def test_pair_refuses_mismatched_timing_scope(self):
        base={"measurement_mode":"timing","timing_scope":"warmed_solver_setup_integration_output_certificate","timing":{"timed_seconds":2.0,"warmup_seconds":.1}}
        candidate={"measurement_mode":"timing","timing_scope":"cold_process_including_jit","timing":{"timed_seconds":1.0,"warmup_seconds":.1}}
        self.assertIsNone(paired_log_cost(base,candidate))
        candidate["timing_scope"]=base["timing_scope"]
        self.assertAlmostEqual(paired_log_cost(base,candidate),math.log(2))

    def test_reference_receipt_provenance_rejects_forgery(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); folder=root/"jobs"/"one"; folder.mkdir(parents=True)
            request={"case":CASE,"solver_id":"independent-reference"}
            atomic_json(folder/"request.json",request)
            result={"status":"OK","times":CASE["output_times"],"states":[[1.0],[2.0]],"uncertainty_abs":[[0.0],[0.0]],"reference_converged":True,"measurement_mode":"reference","input_sha256":digest(CASE),"physical_input_sha256":physical_input_digest(CASE),"reference_source_sha256":"source-hash"}
            job={"measurement_mode":"reference","request":request}
            receipt={"status":"OK","job":job,"result":result,"source_binding_sha256":"source-hash","request_sha256":file_digest(folder/"request.json"),"result_sha256":digest(result),"attempts":[]}
            save_receipt(folder,receipt)
            provenance={"receipt_path":"jobs/one/receipt.json","receipt_sha256":file_digest(folder/"receipt.json"),"source_binding_sha256":"source-hash","request_sha256":receipt["request_sha256"],"physical_input_sha256":physical_input_digest(CASE),"input_sha256":digest(CASE)}
            self.assertTrue(_cli.validated_reference(CASE,result,provenance,root,"source-hash"))
            forged=dict(provenance,physical_input_sha256="forged")
            self.assertFalse(_cli.validated_reference(CASE,result,forged,root,"source-hash"))
