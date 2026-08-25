#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

REQUIRED = [
    "AGENTS.md",
    "README.md",
    "CURRENT_STATE.json",
    "SUPERSEDING_REPAIR_32906175896.md",
    "WORKFLOW_PROVENANCE.md",
    "AUDIT_COMPILED_EXEC_PLAN.yaml",
    "P0_P1_THREAT_CATALOG.yaml",
    "INVARIANT_TEST_MATRIX.yaml",
    "IMPLEMENTER_PROMPT.md",
    "FRESH_REVIEW_PROMPT.md",
    "CODEX_LAUNCHER.md",
]


def corpus() -> str:
    return "\n".join(
        (ROOT / relative).read_text(encoding="utf-8") for relative in REQUIRED
    )


class HandoffContractTests(unittest.TestCase):
    def test_required_files_exist_and_have_no_pending_placeholder(self) -> None:
        for relative in REQUIRED:
            path = ROOT / relative
            self.assertTrue(path.is_file(), relative)
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("PENDING_", text, relative)

    def test_state_is_bound_to_exact_stop_invalid_checkpoint(self) -> None:
        state = json.loads((ROOT / "CURRENT_STATE.json").read_text(encoding="utf-8"))
        self.assertEqual(
            state["canonical_main"]["commit"],
            "4e3a75e5b2843dc1e135dcadba72edb1d09be94c",
        )
        self.assertEqual(
            state["implementation"]["head"],
            "755b31750c1f0e026bbe11aca24efb71e6242624",
        )
        self.assertEqual(
            state["implementation"]["tree"],
            "abbeed3aa1e8ac5d8b00f8173d67f560a914a087",
        )
        self.assertEqual(
            state["implementation"]["tested_merge_sha"],
            "31b0e52a0ebe025db99a299c38a47c88517c88c8",
        )
        self.assertEqual(
            state["current_node"],
            "A1-AUDIT-FULL-E-EVIDENCE-CLOSURE",
        )
        self.assertFalse(state["merge_authorized"])
        self.assertTrue(state["handoff_branch_must_not_be_merged"])

    def test_invalidated_execution_is_exact_and_non_authoritative(self) -> None:
        state = json.loads((ROOT / "CURRENT_STATE.json").read_text(encoding="utf-8"))
        invalid = state["invalidated_execution"]
        self.assertEqual(invalid["workflow_run_id"], 32906175896)
        self.assertEqual(
            invalid["aggregate_scientific_digest"],
            "7665718c60ff9c1e0d1e86d1ff4464e8eb71d806dd0e6ce5c4f6ac0501f027a1",
        )
        self.assertEqual(
            invalid["authority_status"],
            "STOP_INVALID_NON_AUTHORITY",
        )
        self.assertFalse(state["required_next_execution"]["old_execution_reusable"])

    def test_runtime_shadow_and_audit_channels_are_load_bearing(self) -> None:
        text = corpus()
        for token in [
            "shadow_full_e_completed",
            "audit_full_e_completed",
            "audit_full_e_locally_admissible",
            "audit_evidence_status",
            "Missing audit execution",
            "STOP_INVALID",
            "runtime shadow",
            "independent audit full-E",
        ]:
            self.assertIn(token, text, token)

    def test_positive_control_and_decision_rules_are_explicit(self) -> None:
        text = corpus()
        for token in [
            "ADMISSIBLE_AND_DISCRIMINATING",
            "ADMISSIBLE_BUT_NONDISCRIMINATING",
            "NOT_ADMISSIBLE",
            "Hires positive control",
            "above-tau unrecommended",
            "completed audit full-E",
        ]:
            self.assertIn(token, text, token)

    def test_new_execution_is_required_after_load_bearing_repair(self) -> None:
        text = corpus()
        for token in [
            "publish a new scientific execution head",
            "new H_exec",
            "rerun all twelve cells",
            "Do not create a receipt commit from run `32906175896`",
        ]:
            self.assertIn(token, text, token)

    def test_cycle_free_provenance_remains_load_bearing(self) -> None:
        text = corpus()
        for token in [
            "H_exec",
            "R_exec",
            "H_receipt",
            "external R_verify",
            "receipt_commit_sha",
            "external_verification_run_id",
            "final merged main tree == reviewed final PR merge tree",
        ]:
            self.assertIn(token, text, token)

    def test_forbidden_nodes_are_explicit(self) -> None:
        prompt = (ROOT / "IMPLEMENTER_PROMPT.md").read_text(encoding="utf-8")
        for token in [
            "Do not perform A2/A3",
            "Do not change",
            "ordinary committed arm `legacy-fixed`",
            "stop with PR #18 OPEN / DRAFT / UNMERGED",
            "Do not ask user questions",
        ]:
            self.assertIn(token, prompt, token)


if __name__ == "__main__":
    unittest.main(verbosity=2)
