import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from analyze_full_e_shadow_ledger import LedgerError, analyze_profiles


def work(*, jvp_vectors: int, rhs_evaluations: int = 0) -> dict[str, int]:
    return {
        "rhs_evaluations": rhs_evaluations,
        "jvp_vectors": jvp_vectors,
        "phi_actions": jvp_vectors,
    }


def report(*, full_jvp_vectors: int = 8) -> dict:
    return {
        "schema": "g4-s5b0-enforced-prefix-budget-v1",
        "status": "complete",
        "switching_active": False,
        "runtime_full_e_continuations": 0,
        "attempt_rows": [
            {
                "trajectory_id": "toy-n8",
                "attempt_index": 5,
                "accepted_steps_before": 4,
                "t_start": 0.25,
                "h": 0.125,
                "accepted": True,
                "jvp_vectors": 10,
                "wall_seconds": 0.01,
            },
            {
                "trajectory_id": "toy-n8",
                "attempt_index": 6,
                "accepted_steps_before": 5,
                "t_start": 0.375,
                "h": 0.125,
                "accepted": True,
                "jvp_vectors": 20,
                "wall_seconds": 0.02,
            },
        ],
        "rows": [
            {
                "trajectory_id": "toy-n8",
                "family": "toy",
                "dimension": 8,
                "decision_accepted_step": 3,
                "target_attempt_index": 5,
                "target_accepted_steps_before": 4,
                "t_start": 0.25,
                "h": 0.125,
                "target_r_attempt_accepted": True,
                "prefix_succeeded": True,
                "budget_exhausted": False,
                "quadratic_drift_zeta34": 2.0,
                "prefix_work": work(jvp_vectors=3, rhs_evaluations=1),
                "audit_full_e_completed": True,
                "audit_full_e_total_error": 0.5,
                "audit_full_e_locally_admissible": True,
                "audit_full_e_work": work(
                    jvp_vectors=full_jvp_vectors, rhs_evaluations=1
                ),
            },
            {
                "trajectory_id": "toy-n8",
                "family": "toy",
                "dimension": 8,
                "decision_accepted_step": 4,
                "target_attempt_index": 6,
                "target_accepted_steps_before": 5,
                "t_start": 0.375,
                "h": 0.125,
                "target_r_attempt_accepted": True,
                "prefix_succeeded": True,
                "budget_exhausted": False,
                "quadratic_drift_zeta34": 20.0,
                "prefix_work": work(jvp_vectors=4),
                "audit_full_e_completed": True,
                "audit_full_e_total_error": 2.0,
                "audit_full_e_locally_admissible": False,
                "audit_full_e_work": work(jvp_vectors=12),
            },
        ],
    }


class LedgerAnalysisTests(unittest.TestCase):
    def write_profile(self, payload: dict) -> tuple[tempfile.TemporaryDirectory, Path]:
        temporary = tempfile.TemporaryDirectory()
        profile = Path(temporary.name) / "profile"
        profile.mkdir()
        (profile / "toy.json").write_text(json.dumps(payload), encoding="utf-8")
        return temporary, profile

    def test_reconstructs_only_frozen_recommendations_and_joins_target_attempt(self):
        temporary, profile = self.write_profile(report())
        self.addCleanup(temporary.cleanup)

        result = analyze_profiles(
            {"N8": profile}, source_root=Path(temporary.name)
        )

        self.assertEqual(result["schema"], "vigilode-v36-full-e-ledger-preflight-v1")
        self.assertEqual(result["verdict"], "PASS_TO_RUNTIME_SHADOW_MEASUREMENT")
        self.assertEqual(len(result["events"]), 1)
        event = result["events"][0]
        self.assertEqual(event["source_file"], "profile/toy.json")
        self.assertEqual(event["prefix_jvp_vectors"], 3)
        self.assertEqual(event["continuation_jvp_vectors"], 5)
        self.assertEqual(event["full_e_jvp_vectors"], 8)
        self.assertEqual(event["target_rjf_jvp_vectors"], 10)
        self.assertTrue(math.isclose(event["continuation_over_target_rjf_jvp"], 0.5))
        self.assertTrue(math.isclose(event["full_e_over_target_rjf_jvp"], 0.8))
        self.assertEqual(
            event["continuation_work"],
            {"rhs_evaluations": 0, "jvp_vectors": 5, "phi_actions": 5},
        )
        overall = result["overall"]
        self.assertEqual(overall["recommendations"], 1)
        self.assertEqual(overall["unsafe_recommendations"], 0)
        self.assertEqual(overall["continuation_work"]["jvp_vectors"], 5)
        self.assertEqual(overall["full_e_work"]["jvp_vectors"], 8)
        self.assertTrue(math.isclose(overall["cumulative_continuation_over_target_rjf_jvp"], 0.5))

    def test_rejects_componentwise_negative_work_delta(self):
        temporary, profile = self.write_profile(report(full_jvp_vectors=2))
        self.addCleanup(temporary.cleanup)

        with self.assertRaisesRegex(LedgerError, "negative work delta"):
            analyze_profiles({"N8": profile})

    def test_rejects_missing_target_attempt_join(self):
        payload = report()
        payload["attempt_rows"] = []
        temporary, profile = self.write_profile(payload)
        self.addCleanup(temporary.cleanup)

        with self.assertRaisesRegex(LedgerError, "target R-JF attempt"):
            analyze_profiles({"N8": profile})


if __name__ == "__main__":
    unittest.main()
