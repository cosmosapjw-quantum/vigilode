#!/usr/bin/env python3
"""Synthetic corruption tests of the benchmark replay filter, not solver evidence."""
import copy
import unittest
from analyze_bench import exact, replay

SOURCE = "0" * 40
CFG = {"id": 0, "case": "decay", "mode": "dense", "dimension": 1,
       "rtol": 1e-6, "atol": 1e-9, "observations": 5}

def fixture():
    times = [float(i) for i in range(5)]
    states = [[exact("decay", t, 0)] for t in times]
    return {"schema": "vigilode.production-hardening-run.v1", "source_commit": SOURCE,
            **CFG, "success": True, "end": 4.0, "requested_times": times, "times": times,
            "states": states, "elapsed_seconds": 0.1, "max_abs_error": 0.0,
            "max_normalized_error": 0.0, "final_time": 4.0, "final_state": states[-1],
            "work": {"attempts": 2, "accepted_steps": 2, "rejected_steps": 0,
                     "rhs_completed": 13, "fsal_reuses": 1, "counters": {"rhs_calls": 13, "jvp_calls": 0}}}

class ReplayProtocol(unittest.TestCase):
    def test_well_formed_synthetic_fixture(self):
        self.assertTrue(replay(fixture(), CFG, SOURCE)["passed"])

    def test_state_corruption_rejected(self):
        run = fixture(); run["states"][2][0] += 0.1
        self.assertFalse(replay(run, CFG, SOURCE)["passed"])

    def test_schedule_corruption_rejected(self):
        run = copy.deepcopy(fixture()); run["times"][2] = 1.5
        self.assertFalse(replay(run, CFG, SOURCE)["passed"])

    def test_work_corruption_rejected(self):
        run = fixture(); run["work"]["counters"]["rhs_calls"] = 12
        self.assertFalse(replay(run, CFG, SOURCE)["passed"])

    def test_identity_corruption_rejected(self):
        run = fixture(); run["source_commit"] = "1" * 40
        self.assertFalse(replay(run, CFG, SOURCE)["passed"])

    def test_nan_state_rejected(self):
        run = fixture(); run["states"][1][0] = float("nan")
        self.assertFalse(replay(run, CFG, SOURCE)["passed"])

    def test_large_honestly_reported_error_is_replay_not_accuracy_failure(self):
        run = fixture(); run["states"][2][0] += 0.1
        run["max_abs_error"] = run["max_normalized_error"] = 0.1
        result = replay(run, CFG, SOURCE)
        self.assertTrue(result["passed"])
        self.assertGreater(result["max_normalized_error"], CFG["rtol"])

if __name__ == "__main__":
    unittest.main()
