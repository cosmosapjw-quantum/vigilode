from __future__ import annotations

import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md" if (ROOT / "README.md").is_file() else ROOT / "README_FIRST.md"
TEXT_FILES = [
    ROOT / "AGENTS.md",
    README,
    ROOT / "AUDIT_COMPILED_EXEC_PLAN.yaml",
    ROOT / "IMPLEMENTER_PROMPT.md",
    ROOT / "FRESH_REVIEW_PROMPT.md",
    ROOT / "acceptance" / "README.md",
]
REFERENCE_RE = re.compile(r"`((?:templates|acceptance)/[A-Za-z0-9_./-]+)`")


class HandoffCompletenessContractTests(unittest.TestCase):
    def test_all_repo_local_contract_references_exist(self):
        references = set()
        for path in TEXT_FILES:
            self.assertTrue(path.is_file(), f"missing handoff file: {path}")
            references.update(REFERENCE_RE.findall(path.read_text(encoding="utf-8")))
        self.assertIn("templates/COMPLETION_EVIDENCE_SCHEMA.json", references)
        missing = sorted(ref for ref in references if not (ROOT / ref).is_file())
        self.assertEqual(missing, [], f"missing referenced handoff files: {missing}")

    def test_all_json_contracts_parse(self):
        for path in sorted(ROOT.rglob("*.json")):
            with self.subTest(path=path.relative_to(ROOT)):
                json.loads(path.read_text(encoding="utf-8"))

    def test_schema_named_files_are_actual_json_schemas(self):
        for name in ("COMPLETION_EVIDENCE_SCHEMA.json", "VENDOR_VALIDATION_SCHEMA.json"):
            schema = json.loads((ROOT / "templates" / name).read_text(encoding="utf-8"))
            self.assertEqual(schema.get("$schema"), "https://json-schema.org/draft/2020-12/schema")
            self.assertEqual(schema.get("type"), "object")
            self.assertIn("required", schema)
            self.assertIn("properties", schema)


if __name__ == "__main__":
    unittest.main()
