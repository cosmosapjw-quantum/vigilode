#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "e4-fresh-clone-build.yml"


class WorkflowWrapperInvocationTests(unittest.TestCase):
    def test_offline_wrapper_calls_use_explicit_bash_interpreter(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        calls = [
            line.strip()
            for line in text.splitlines()
            if "tools/cargo-offline.sh" in line
        ]
        self.assertEqual(len(calls), 3)
        self.assertTrue(
            all(call.startswith("bash ./tools/cargo-offline.sh") for call in calls),
            calls,
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
