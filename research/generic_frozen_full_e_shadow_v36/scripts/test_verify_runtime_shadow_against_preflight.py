import importlib.util
import math
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_runtime_shadow_against_preflight.py")
SPEC = importlib.util.spec_from_file_location("v36_runtime_verifier", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RuntimeVerifierTests(unittest.TestCase):
    def test_float_comparison_is_bit_exact(self) -> None:
        self.assertTrue(MODULE.exact_equal(1.0, 1.0))
        self.assertFalse(MODULE.exact_equal(1.0, math.nextafter(1.0, math.inf)))

    def test_work_recomposition_is_componentwise(self) -> None:
        self.assertEqual(
            MODULE.recompose_work({"a": 2, "b": 5}, {"a": 3, "b": 7}),
            {"a": 5, "b": 12},
        )

    def test_recommendation_is_fail_closed_on_budget_breach(self) -> None:
        row = {
            "prefix_succeeded": True,
            "budget_exhausted": False,
            "budget_breached": False,
            "quadratic_drift_zeta34": MODULE.FROZEN_TAU,
        }
        self.assertTrue(MODULE.expected_recommendation(row))
        row["budget_breached"] = True
        self.assertFalse(MODULE.expected_recommendation(row))


if __name__ == "__main__":
    unittest.main()
