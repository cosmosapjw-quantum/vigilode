import importlib.util
import math
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_continuation_replay.py")
SPEC = importlib.util.spec_from_file_location("v37_continuation_verifier", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ContinuationReplayVerifierTests(unittest.TestCase):
    def test_float_comparison_is_bit_exact(self) -> None:
        self.assertTrue(MODULE.exact_equal(1.0, 1.0))
        self.assertFalse(MODULE.exact_equal(1.0, math.nextafter(1.0, math.inf)))

    def test_work_recomposition_is_componentwise(self) -> None:
        self.assertEqual(
            MODULE.recompose_work({"a": 2, "b": 5}, {"a": 3, "b": 7}),
            {"a": 5, "b": 12},
        )

    def test_recommendation_reuses_the_frozen_prefix_witness_only(self) -> None:
        row = {
            "prefix_succeeded": True,
            "budget_exhausted": False,
            "budget_breached": False,
            "quadratic_drift_zeta34": MODULE.FROZEN_TAU,
        }
        self.assertTrue(MODULE.expected_recommendation(row))
        row["quadratic_drift_zeta34"] = math.nextafter(MODULE.FROZEN_TAU, math.inf)
        self.assertFalse(MODULE.expected_recommendation(row))

    def test_exhausted_row_requires_charged_work_and_no_endpoint_labels(self) -> None:
        zero_expensive = {
            "jacobian_builds": 0,
            "direct_factorizations": 0,
            "nonlinear_solves": 0,
            "nonlinear_iterations": 0,
            "nonlinear_residual_evaluations": 0,
            "nonlinear_jacobian_evaluations": 0,
        }
        row = {
            "recommended": True,
            "retained_level2_resumed": True,
            "continuation_outcome": "budget-exhausted",
            "continuation_budget_exhausted": True,
            "continuation_jvp_cap": 80,
            "continuation_used_jvp_vectors": 80,
            "prefix_work": {"jvp_vectors": 21, **zero_expensive},
            "continuation_work": {"jvp_vectors": 80, **zero_expensive},
            "shadow_full_e_work": {"jvp_vectors": 101, **zero_expensive},
            "shadow_full_e_completed": False,
            "shadow_full_e_total_error": None,
            "shadow_full_e_locally_admissible": None,
            "shadow_full_e_failure": None,
            "work_roundtrip_exact": True,
        }
        MODULE.validate_continuation_row(row, "fixture")
        row["shadow_full_e_locally_admissible"] = False
        with self.assertRaises(MODULE.VerificationError):
            MODULE.validate_continuation_row(row, "fixture")

    def test_committed_method_preserves_the_v36_schema_label_for_rjf(self) -> None:
        report = {"committed_method": "protected-sequential-matrix-free-rodas5p"}
        baseline = {"committed_method": "protected-sequential-matrix-free-rodas5p"}
        MODULE.validate_committed_method(report, baseline, "fixture")
        report["committed_method"] = "protected-sequential-matrix-free-rjf"
        with self.assertRaises(MODULE.VerificationError):
            MODULE.validate_committed_method(report, baseline, "fixture")

    def test_sealed_contract_hash_is_explicit(self) -> None:
        self.assertEqual(
            MODULE.EXPECTED_CONTRACT_SHA256,
            "66f082aeec8c70e0ef23926d2c6f7057fb40fe280c45fd02c200be8778a6e659",
        )


if __name__ == "__main__":
    unittest.main()
