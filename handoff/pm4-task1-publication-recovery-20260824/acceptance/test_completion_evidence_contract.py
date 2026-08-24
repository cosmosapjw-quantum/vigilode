from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "templates" / "COMPLETION_EVIDENCE_SCHEMA.json"
EXAMPLE = ROOT / "templates" / "COMPLETION_EVIDENCE_EXAMPLE.json"
VALIDATOR = ROOT / "acceptance" / "validate_completion_evidence.py"

spec = importlib.util.spec_from_file_location("completion_validator", VALIDATOR)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CompletionEvidenceContractTests(unittest.TestCase):
    def test_schema_is_real_draft_2020_12_and_closed(self):
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        self.assertEqual(schema["$schema"], "https://json-schema.org/draft/2020-12/schema")
        self.assertEqual(schema["type"], "object")
        self.assertFalse(schema["additionalProperties"])
        required = set(schema["required"])
        for key in (
            "r4_archive_sha256",
            "base_sha",
            "previous_feature_sha",
            "published_feature_sha",
            "published_tree_sha",
            "staged_and_final_name_status",
            "vendor_validation_json",
            "cargo_metadata_json_sha256",
            "all_command_logs_and_exit_codes",
            "publication_receipt",
            "remote_verification_receipt",
            "unresolved_blockers",
        ):
            self.assertIn(key, required)

    def test_positive_example_passes_executable_validator(self):
        evidence = json.loads(EXAMPLE.read_text(encoding="utf-8"))
        module.validate(evidence)

    def test_partial_or_forbidden_example_fails(self):
        evidence = json.loads(EXAMPLE.read_text(encoding="utf-8"))
        evidence["publication_receipt"]["merge_performed"] = True
        with self.assertRaises(module.EvidenceError):
            module.validate(evidence)

    def test_missing_command_evidence_fails(self):
        evidence = json.loads(EXAMPLE.read_text(encoding="utf-8"))
        evidence["all_command_logs_and_exit_codes"] = evidence["all_command_logs_and_exit_codes"][:-1]
        with self.assertRaises(module.EvidenceError):
            module.validate(evidence)


if __name__ == "__main__":
    unittest.main()
