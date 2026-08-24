from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = [
    "AGENTS.md",
    "README.md",
    "IDENTITY_POLICY.json",
    "HASH_IDENTITY_POLICY.md",
    "AUDIT_COMPILED_EXEC_PLAN.yaml",
    "P0_P1_THREAT_CATALOG.yaml",
    "INVARIANT_TEST_MATRIX.yaml",
    "IMPLEMENTER_PROMPT.md",
    "FRESH_REVIEW_PROMPT.md",
    "CODEX_LAUNCHER.md",
    "payload/PM4_TASK1_SCHEMA_BOUNDARY.patch",
    "acceptance/README.md",
    "acceptance/run_preflight.sh",
    "acceptance/test_identity_policy_contract.py",
    "acceptance/test_payload_contract.py",
]


class HandoffCompletenessTests(unittest.TestCase):
    def test_required_files_exist_and_are_nonempty(self) -> None:
        missing = []
        for rel in REQUIRED:
            path = ROOT / rel
            if not path.is_file() or path.stat().st_size == 0:
                missing.append(rel)
        self.assertEqual(missing, [])

    def test_machine_readable_policy_parses(self) -> None:
        json.loads((ROOT / "IDENTITY_POLICY.json").read_text(encoding="utf-8"))

    def test_no_archive_transport_parts_or_sidecars(self) -> None:
        forbidden = []
        for path in ROOT.rglob("*"):
            if path.is_file() and (
                path.name.endswith((".tar.gz", ".zip", ".whl", ".sha256"))
                or ".b64.part-" in path.name
            ):
                forbidden.append(str(path.relative_to(ROOT)))
        self.assertEqual(forbidden, [])


if __name__ == "__main__":
    unittest.main()
