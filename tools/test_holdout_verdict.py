#!/usr/bin/env python3
"""Tests for tools/holdout_verdict.py (audit F-023)."""

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("holdout_verdict", HERE / "holdout_verdict.py")
hv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hv)


def events(safe_scores, unsafe_scores):
    return [{"score": s, "positive": False} for s in safe_scores] + [
        {"score": s, "positive": True} for s in unsafe_scores
    ]


class HoldoutVerdictTests(unittest.TestCase):
    def test_exact_zero_miss_bound(self):
        self.assertAlmostEqual(hv.miss_rate_upper_95(5), 0.45072, places=5)
        self.assertAlmostEqual(hv.miss_rate_upper_95(59), 0.04951, places=5)
        self.assertAlmostEqual(hv.miss_rate_upper_95(299), 0.00997, places=5)
        self.assertIsNone(hv.miss_rate_upper_95(0))
        self.assertEqual(hv.positives_needed(0.05), 59)
        self.assertEqual(hv.positives_needed(0.01), 299)

    def test_one_positive_is_inconclusive(self):
        report = hv.holdout_verdict(events([1, 2, 3, 4], [5]), 4.5)
        self.assertEqual(report["raw_verdict_at_tau"], "PASS")
        self.assertEqual(report["verdict"], "INCONCLUSIVE_INSUFFICIENT_POSITIVES")

    def test_six_positives_and_a_rank_stable_threshold_give_a_verdict(self):
        # Safe scores 1..5 and 20, unsafe 10..15: tau = 3 keeps PASS one rank
        # up (4) and down (2).
        report = hv.holdout_verdict(events([1, 2, 3, 4, 5, 20], [10, 11, 12, 13, 14, 15]), 3.0)
        self.assertTrue(report["rank_stable"])
        self.assertEqual(report["verdict"], "PASS")
        failing = hv.holdout_verdict(events([1, 2, 3], [0.5, 0.6, 0.7, 0.8, 0.9, 10]), 3.0)
        self.assertEqual(failing["verdict"], "FAIL")

    def test_rank_unstable_verdict_is_inconclusive(self):
        report = hv.holdout_verdict(events([1, 2, 3], [4, 10, 11, 12, 13]), 3.0)
        self.assertEqual(report["verdict_one_rank_up"], "FAIL")
        self.assertEqual(report["verdict"], "INCONCLUSIVE_RANK_UNSTABLE")

    def test_v35_holdout320_replay(self):
        out = subprocess.run(
            [sys.executable, str(HERE / "holdout_verdict.py"), "--v35-holdout320"],
            check=True,
            capture_output=True,
            text=True,
        )
        report = json.loads(out.stdout)
        self.assertEqual(report["n_events"], 28)
        self.assertEqual(report["n_positives"], 1)
        self.assertEqual(report["positives"][0]["rank"], 14)
        self.assertEqual(report["raw_verdict_at_tau"], "PASS")
        self.assertEqual(report["raw_verdict_at_tau_selected"], "FAIL")
        self.assertEqual(report["verdict"], "INCONCLUSIVE_INSUFFICIENT_POSITIVES")


if __name__ == "__main__":
    unittest.main()
