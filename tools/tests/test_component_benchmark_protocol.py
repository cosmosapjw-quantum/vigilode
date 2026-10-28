import unittest
from component_benchmark.protocol import validate_case, validate_result

CASE = {"case_id":"x", "family":"test", "model":"x", "parameters":{}, "initial_state":[1.0], "t_span":[0.0,1.0], "output_times":[0.0,1.0], "rtol":1e-6, "atol":[1e-9], "equation_form":"ODE", "mass_matrix":None, "reference_policy":{}}

class ProtocolTest(unittest.TestCase):
    def test_case_and_result(self):
        self.assertEqual(validate_case(CASE), [])
        result={"status":"OK", "times":[0.0,1.0], "states":[[1.0],[2.0]], "counters":None, "timing":{"timed_seconds":.1,"warmup_seconds":.01}, "allocations":None,"measurement_mode":"timing","timing_scope":"warmed_solver_setup_integration_output_certificate"}
        self.assertEqual(validate_result(result, CASE), [])
        result["times"]=[0.0,.9]
        self.assertIn("output_mesh_mismatch",validate_result(result,CASE))

    def test_timing_scope_mismatch_fails(self):
        result={"status":"OK", "times":[0.0,1.0], "states":[[1.0],[2.0]], "counters":None, "timing":{"timed_seconds":.1,"warmup_seconds":.01}, "allocations":None,"measurement_mode":"timing","timing_scope":"cold_process_including_imports"}
        self.assertIn("incomparable_timing_scope",validate_result(result,CASE))

    def test_nan_rejected(self):
        case={**CASE,"initial_state":[float("nan")]}
        self.assertIn("invalid_initial_state",validate_case(case))
