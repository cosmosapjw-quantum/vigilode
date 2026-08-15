import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RESULT = json.loads((ROOT / 'results/G4_S5B2_ANALYSIS.json').read_text())

class PreflightTests(unittest.TestCase):
    def test_startup_only_oracle_fails(self):
        self.assertFalse(RESULT['gates']['startup_only_profile_holdout_median_regret_le_1_15'])
        self.assertFalse(RESULT['gates']['startup_only_profile_holdout_p95_regret_le_1_5'])

    def test_no_prefix_implementation_authorized(self):
        self.assertFalse(RESULT['gates']['actual_prefix_implementation_authorized'])
        self.assertFalse(RESULT['gates']['active_switching_authorized'])

    def test_reentry_upper_bound_locates_next_architecture(self):
        hold = RESULT['optimistic_reentry_upper_bound']['holdout']
        self.assertLessEqual(hold['median_regret'], 1.15)
        self.assertLessEqual(hold['p95_regret'], 1.5)
        self.assertGreater(hold['p95_startup_overhead'], 0.05)

    def test_candidate_source_unmodified(self):
        self.assertFalse(RESULT['candidate_source_modified'])
        self.assertFalse(RESULT['active_switching'])

if __name__ == '__main__':
    unittest.main()
