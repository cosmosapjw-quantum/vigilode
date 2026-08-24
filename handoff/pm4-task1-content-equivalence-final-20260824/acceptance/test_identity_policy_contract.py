from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class IdentityPolicyContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = json.loads((ROOT / "IDENTITY_POLICY.json").read_text(encoding="utf-8"))

    def test_required_identity_classes_and_gate_strengths(self) -> None:
        classes = self.policy["classes"]
        self.assertEqual(classes["git_source_authority"]["default_gate"], "HARD")
        self.assertEqual(classes["immutable_external_input"]["default_gate"], "HARD")
        self.assertEqual(classes["deterministic_generated_evidence"]["default_gate"], "CONDITIONAL_HARD")
        self.assertEqual(classes["numerical_generated_output"]["default_gate"], "NUMERICAL")
        self.assertEqual(classes["packaging_transport"]["default_gate"], "SOFT")
        self.assertEqual(classes["working_tree_materialization"]["default_gate"], "DIAGNOSTIC")

    def test_packaging_sha_mismatch_cannot_block_task1(self) -> None:
        packaging = self.policy["classes"]["packaging_transport"]
        task = self.policy["task1_policy"]
        self.assertFalse(packaging["outer_sha_mismatch_alone_blocks"])
        self.assertFalse(task["uses_outer_archive"])
        self.assertFalse(task["packaging_sha_mismatch_is_blocker"])
        self.assertTrue(task["tracked_patch_is_primary_payload"])

    def test_numerical_output_is_not_byte_hash_authority_by_default(self) -> None:
        numerical = self.policy["classes"]["numerical_generated_output"]
        self.assertFalse(numerical["byte_hash_default_authority"])
        self.assertIn("residuals", numerical["primary_evidence"])
        self.assertIn("tolerances", numerical["primary_evidence"])

    def test_load_bearing_prompts_encode_the_policy(self) -> None:
        combined = "\n".join(
            (ROOT / name).read_text(encoding="utf-8")
            for name in [
                "AGENTS.md",
                "README.md",
                "HASH_IDENTITY_POLICY.md",
                "IMPLEMENTER_PROMPT.md",
                "FRESH_REVIEW_PROMPT.md",
                "CODEX_LAUNCHER.md",
            ]
        )
        self.assertIn("packaging SHA mismatch", combined)
        self.assertIn("Git", combined)
        self.assertNotIn("sha256sum -c", combined)


if __name__ == "__main__":
    unittest.main()
