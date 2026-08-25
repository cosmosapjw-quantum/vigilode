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
    "AUDIT_COMPILED_EXEC_PLAN.yaml",
    "P0_P1_THREAT_CATALOG.yaml",
    "INVARIANT_TEST_MATRIX.yaml",
    "IMPLEMENTER_PROMPT.md",
    "FRESH_REVIEW_PROMPT.md",
]


class HandoffContractTests(unittest.TestCase):
    def test_required_files_exist_and_have_no_pending_placeholder(self) -> None:
        for relative in REQUIRED:
            path = ROOT / relative
            self.assertTrue(path.is_file(), relative)
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("PENDING_", text, relative)

    def test_state_is_bound_to_exact_green_checkpoint(self) -> None:
        state = json.loads((ROOT / "CURRENT_STATE.json").read_text(encoding="utf-8"))
        self.assertEqual(
            state["canonical_main"]["commit"],
            "4e3a75e5b2843dc1e135dcadba72edb1d09be94c",
        )
        self.assertEqual(
            state["implementation"]["head"],
            "7952bf96bfd9fb604e87bce41bd9b918cc9b93f4",
        )
        self.assertEqual(
            state["implementation"]["tree"],
            "dd32d7bebe50419c510f2e779b43ed6a26f29242",
        )
        self.assertEqual(state["current_node"], "A1-TWO-ARM-AUTHORITY-RECEIPT")
        self.assertEqual(
            state["compile_trace_closure"]["a1_workflow"]["conclusion"],
            "SUCCESS",
        )
        self.assertEqual(
            state["compile_trace_closure"]["e4_workflow"]["conclusion"],
            "SUCCESS",
        )
        self.assertFalse(state["merge_authorized"])
        self.assertTrue(state["handoff_branch_must_not_be_merged"])

    def test_scientific_boundaries_are_load_bearing(self) -> None:
        corpus = "\n".join(
            (ROOT / relative).read_text(encoding="utf-8") for relative in REQUIRED
        )
        for token in [
            "legacy-fixed",
            "outer-scaled-numeric-parity",
            "EnforcedBudgetHoldout320",
            "12 cells",
            "13.39706618860016",
            "ADMISSIBLE_AND_DISCRIMINATING",
            "ADMISSIBLE_BUT_NONDISCRIMINATING",
            "NOT_ADMISSIBLE",
            "BLOCKED_BY_UNRESOLVED_SPEC",
            "OPEN_DRAFT_UNMERGED",
        ]:
            self.assertIn(token, corpus, token)

    def test_forbidden_nodes_are_explicit(self) -> None:
        prompt = (ROOT / "IMPLEMENTER_PROMPT.md").read_text(encoding="utf-8")
        for token in [
            "Do not perform A2/A3",
            "Do not change",
            "Do not switch the committed arm inside the replay runner",
            "Stop with PR #18 OPEN / DRAFT / UNMERGED",
            "Do not ask the user questions",
        ]:
            self.assertIn(token, prompt, token)


if __name__ == "__main__":
    unittest.main(verbosity=2)
