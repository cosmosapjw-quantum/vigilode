"""Output-assessment regression tests; no external solver is launched."""
import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("native_compare_under_test", Path(__file__).with_name("compare.py"))
compare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare)


class AssessmentTests(unittest.TestCase):
    def setUp(self):
        self.descriptor = {"model_contract": {"reference_kind": "analytic endpoint"},
                           "initial_state": [1.0, 2.0], "t_end": 1.0}
        self.result = {"method": "dopri5", "solver_success": True,
                       "final_state": [1.0, 2.0], "final_time": 1.0}

    def assess(self, result=None, exact=(1.0, 2.0), uncertainty=0.0, execution="COMPLETED"):
        return compare.assess(self.result if result is None else result, self.descriptor,
                              exact, uncertainty, 1e-4, execution)

    def test_finite_endpoint_accepts_and_preserves_inputs(self):
        before = copy.deepcopy((self.result, self.descriptor))
        out = self.assess()
        self.assertEqual(out["status"], "PASS_SCOPED")
        self.assertEqual(out["actual_error"], 0)
        self.assertEqual((self.result, self.descriptor), before)

    def test_nonfinite_endpoint_rejected_before_accuracy(self):
        for t in (float("nan"), float("inf"), float("-inf")):
            with self.subTest(t=t):
                self.result["final_time"] = t
                out = self.assess()
                self.assertEqual(out["status"], "OUTPUT_CONTRACT_FAILURE")
                self.assertIsNone(out["actual_error"])

    def test_wrong_numeric_endpoint_types_rejected(self):
        for t in (True, False, None, "1.0", 10 ** 400):
            with self.subTest(t=repr(t)[:30]):
                self.result["final_time"] = t
                self.assertEqual(self.assess()["status"], "OUTPUT_CONTRACT_FAILURE")

    def test_endpoint_tolerance_unchanged(self):
        self.result["final_time"] = 1.0 + 5e-11
        self.assertEqual(self.assess()["status"], "PASS_SCOPED")
        self.result["final_time"] = 1.0 + 2e-10
        self.assertEqual(self.assess()["status"], "OUTPUT_CONTRACT_FAILURE")

    def test_invalid_state_rejected(self):
        for state in ([True, 2.0], [float("nan"), 2.0], [float("inf"), 2.0],
                      [10 ** 400, 2.0], [1.0], None):
            with self.subTest(state=repr(state)[:40]):
                self.result["final_state"] = state
                self.assertEqual(self.assess()["status"], "OUTPUT_CONTRACT_FAILURE")

    def test_reference_and_error_cap_semantics_unchanged(self):
        self.assertEqual(self.assess(exact=None)["status"], "UNVALIDATED_REFERENCE")
        self.result["final_state"] = [1.00005, 2.0]
        self.assertEqual(self.assess(uncertainty=1e-6)["status"], "PASS_SCOPED")
        self.assertEqual(self.assess(uncertainty=6e-5)["status"], "ERROR_CAP_FAILURE")

    def test_process_and_solver_failures_preserved(self):
        self.assertEqual(self.assess(execution="TIMEOUT")["status"], "PROCESS_FAILURE")
        self.result["solver_success"] = False
        self.assertEqual(self.assess()["status"], "SOLVER_FAILURE")

    def test_external_model_match_requirement_preserved(self):
        self.result["method"] = "diffsol-bdf"
        self.assertEqual(self.assess()["status"], "MODEL_MISMATCH")
        self.result["model_match"] = {"status": "MATCHED_AT_DECLARED_PROBES"}
        self.assertEqual(self.assess()["status"], "PASS_SCOPED")


if __name__ == "__main__":
    unittest.main()
