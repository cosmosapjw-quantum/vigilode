import math
import sys
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))

from analyze_shadow_economics import positive_ulp_distance, validate_pair


def arm(mode: str, wall: float) -> dict:
    return {
        "mode": mode,
        "repetitions": 1,
        "wall_seconds": wall,
        "proposed_interval": 2.0,
        "gamma_seconds_per_interval": wall / 2.0,
        "family_count": 6,
        "all_suite_identities_passed": True,
    }


class EconomicsAnalysisTests(unittest.TestCase):
    def test_positive_ulp_distance_detects_one_step(self) -> None:
        self.assertEqual(positive_ulp_distance(1.0, math.nextafter(1.0, math.inf)), 1)

    def test_pair_contract_accepts_exact_alternating_payload(self) -> None:
        rjf = arm("rjf-only", 4.0)
        shadow = arm("frozen-full-e-shadow", 5.0)
        pair = {
            "pair_index": 0,
            "order": "rjf-first",
            "rjf_only": rjf,
            "frozen_full_e_shadow": shadow,
            "wall_ratio_shadow_over_rjf": 1.25,
            "gamma_ratio_shadow_over_rjf": 1.25,
        }
        self.assertEqual(validate_pair(pair, 0, 1, "test"), 1.25)


if __name__ == "__main__":
    unittest.main()
