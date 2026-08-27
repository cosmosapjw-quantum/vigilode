import copy
import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[3]
RECEIPT = ROOT / "research/k0_stage_telemetry_20260827/intake/authority_receipt.json"

EXPECTED = {
    "schema": "vigilode-k0-authority-intake/v1",
    "status": "READY",
    "repository": "cosmosapjw-quantum/vigilode",
    "implementation_branch": "research/k0-stage-telemetry-integration-20260827",
    "canonical_main_sha": "8d0c79184e09efb5bdadc24a6315c60a71a44264",
    "canonical_main_tree": "acd94364cf69f19d782619fc6c75554cb0754208",
    "source_parent_sha": "e1124586a4029f86669e7489278c61ef676d61aa",
    "source_parent_tree": "adbb933cf3bf3d401d652c8a6d9df661d8500a2b",
    "package_parent_sha": "c224e0d108677755b75a8b9cef074c554647a9b4",
    "package_parent_tree": "7aeb27394c3f22b38ca8c374578925f4b5476084",
    "prepared_start_sha": "2cf820e2ae657f194593dd468efe6fc2798c14f0",
    "prepared_start_tree": "5e3bfc1186d329a91c0d446facf4225999810422",
    "prepared_parent_order": [
        "e1124586a4029f86669e7489278c61ef676d61aa",
        "c224e0d108677755b75a8b9cef074c554647a9b4",
    ],
    "rust_version": "rustc 1.94.1 (e408947bf 2026-03-25)",
    "cargo_version": "cargo 1.94.1 (29ea6fb6a 2026-03-24)",
    "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
    "worktree_dirty": False,
    "control_pr": 21,
    "implementation_base_pr": 20,
    "jira_issue": "PM-7",
    "jira_status": "In Progress",
    "confluence_page_id": "15499267",
    "confluence_version": 2,
}


def validate_receipt(receipt):
    mismatches = {
        key: (receipt.get(key), expected)
        for key, expected in EXPECTED.items()
        if receipt.get(key) != expected
    }
    if mismatches:
        raise ValueError(mismatches)


class AuthorityReceiptTests(unittest.TestCase):
    def test_exact_prepared_authority_is_ready(self):
        receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
        validate_receipt(receipt)

    def test_mutated_source_sha_is_rejected(self):
        receipt = json.loads(RECEIPT.read_text(encoding="utf-8"))
        mutated = copy.deepcopy(receipt)
        mutated["source_parent_sha"] = "0" * 40
        with self.assertRaises(ValueError):
            validate_receipt(mutated)


if __name__ == "__main__":
    unittest.main()
