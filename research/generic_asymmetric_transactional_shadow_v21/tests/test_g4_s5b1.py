import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = json.loads((ROOT / "results/G4_S5B1_ANALYSIS.json").read_text())


class G4S5B1Tests(unittest.TestCase):
    def test_scientific_identity(self):
        self.assertEqual(len(set(DATA["scientific_hashes"].values())), 1)

    def test_metric_correction_is_material(self):
        stats = DATA["metric_correction"]["statistics"]
        self.assertEqual(stats["winner_flips"], 24)
        self.assertGreater(stats["winner_flip_fraction"], 0.05)

    def test_contract_policies_not_activated(self):
        for policy in ("r-default", "e-first"):
            self.assertFalse(DATA["contract_gates"][policy]["activation_gate_pass"])
            self.assertTrue(DATA["contract_gates"][policy]["unsafe_zero"])

    def test_safe_e_is_diagnostic_not_activation(self):
        result = DATA["results"]["safe-e-diagnostic"]
        self.assertLessEqual(result["holdout"]["p95_regret"], 1.5)
        self.assertGreater(result["calibration"]["p95_regret"], 1.5)
        self.assertEqual(DATA["closeout"]["safe_e_diagnostic"], "PROMOTE_DIAGNOSTIC_ONLY")

    def test_no_active_switching(self):
        self.assertFalse(DATA["active_switching"])
        self.assertFalse(DATA["candidate_source_modified"])


if __name__ == "__main__":
    unittest.main()
