#!/usr/bin/env python3
"""Regression tests for frozen experiment scope, not numerical admission."""
from __future__ import annotations
import importlib.util
import pathlib
import re
import json
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("scope", ROOT / "tools/a1_receipt_ci_scope.py")
assert spec and spec.loader
scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scope)


class ScopeTests(unittest.TestCase):
    def test_audit2_shared_source_is_not_a_new_a1_receipt(self):
        active, reason = scope.classify("research/audit2-output-policy-20260829", [
            "crates/rodas5p-integrators/src/lib.rs",
            "crates/rodas5p-integrators/src/audit2_research.rs",
            "crates/rodas5p-fair-ab/src/output_accuracy.rs",
        ])
        self.assertFalse(active, reason)

    def test_explicit_a1_branch_still_runs_frozen_authority_checks(self):
        self.assertTrue(scope.classify("research/a1-inner-tolerance-parity", [
            "crates/rodas5p-integrators/src/sequential.rs"])[0])

    def test_changed_a1_evidence_cannot_hide_on_another_branch(self):
        for name in ("A1_TWO_ARM_AUTHORITY_RECEIPT.json", "A1_TWO_ARM_AUTHORITY_RECEIPT.md", "new-cell.json"):
            self.assertTrue(scope.classify("unrelated-branch", [
                "research/a1_inner_tolerance_audit_20260825/" + name])[0])

    def test_workflow_only_change_is_control_not_a_scientific_reexecution(self):
        self.assertFalse(scope.classify("ci/repair", [
            ".github/workflows/a1-two-arm-receipt.yml", "tools/a1_receipt_ci_scope.py"])[0])

    def test_similar_names_do_not_claim_a1_authority(self):
        self.assertFalse(scope.classify("research/a10-other", [
            "research/a1_inner_tolerance_audit_20260825-copy/file.json"])[0])

    def test_empty_unrelated_delta_is_not_a1(self):
        self.assertFalse(scope.classify("docs/help", [])[0])

    def test_workflow_gates_both_frozen_guard_and_execution_selection(self):
        text = (ROOT / ".github/workflows/a1-two-arm-receipt.yml").read_text()
        for title in ("Enforce forbidden-diff and ordinary-arm guards", "Select execution or receipt-validation mode"):
            match = re.search(r"      - name: " + re.escape(title) + r"\n(.*?)(?=\n      - name:|\n  [a-z]|\Z)", text, re.S)
            self.assertIsNotNone(match, title)
            self.assertIn("if: steps.scope.outputs.applicable == 'true'", match.group(1), title)
        self.assertIn("python3 tools/a1_receipt_ci_scope.py", text)
        # Preserve the original scientific guard; scope repair is not its removal.
        self.assertIn('git diff --exit-code "$STARTING_A1_HEAD" "$FEATURE_HEAD"', text)
        self.assertIn('crates/rodas5p-krylov crates/rodas5p-core crates/rodas5p-fair-ab', text)


class GitBoundaryTests(unittest.TestCase):
    def git(self, repo, *args):
        return subprocess.run(["git", *args], cwd=repo, check=True, text=True, capture_output=True).stdout.strip()

    def fixture(self):
        directory = tempfile.TemporaryDirectory(prefix="a1-scope-")
        self.addCleanup(directory.cleanup)
        root = pathlib.Path(directory.name)
        self.git(root, "init", "-q")
        self.git(root, "config", "user.name", "scope test")
        self.git(root, "config", "user.email", "fixture@example.invalid")
        receipt = root / scope.A1_RESEARCH / "old.json"
        receipt.parent.mkdir(parents=True)
        receipt.write_text("{}\n")
        self.git(root, "add", ".")
        self.git(root, "commit", "-qm", "fixture")
        base = self.git(root, "rev-parse", "HEAD")
        return root, receipt, base

    def test_deleted_evidence_activates_the_original_frozen_checks(self):
        root, receipt, base = self.fixture()
        receipt.unlink()
        self.git(root, "add", "-A")
        self.git(root, "commit", "-qm", "delete")
        paths = scope.changed_paths(root, base, self.git(root, "rev-parse", "HEAD"))
        self.assertTrue(scope.classify("unrelated", paths)[0])

    def test_renaming_evidence_away_does_not_evade_a1_scope(self):
        root, receipt, base = self.fixture()
        receipt.rename(root / "not-a1.json")
        self.git(root, "add", "-A")
        self.git(root, "commit", "-qm", "rename")
        paths = scope.changed_paths(root, base, self.git(root, "rev-parse", "HEAD"))
        self.assertTrue(scope.classify("unrelated", paths)[0])

    def test_invalid_identity_is_not_silently_not_applicable(self):
        result = subprocess.run([sys.executable, str(ROOT / "tools/a1_receipt_ci_scope.py"),
            "--base", "not-a-sha", "--head", "0" * 40, "--head-ref", "other"],
            text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)["scope"], "UNRESOLVED")

    def test_cli_reports_scope_without_claiming_receipt_validation(self):
        root, _, base = self.fixture()
        output = root / "github-output"
        result = subprocess.run([sys.executable, str(ROOT / "tools/a1_receipt_ci_scope.py"),
            "--repo-root", str(root), "--base", base, "--head", base, "--head-ref", "docs/help",
            "--github-output", str(output)], text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output.read_text(), "applicable=false\n")
        self.assertFalse(json.loads(result.stdout)["scientific_receipt_validated"])


class ResearchCoverageTests(unittest.TestCase):
    def test_ci_runs_actual_opt_in_contracts_and_the_default_solver_example(self):
        workflow = ROOT / ".github/workflows/audit2-research.yml"
        runner = ROOT / "tools/check-audit2-readiness.sh"
        self.assertTrue(workflow.is_file(), "audit2 feature is not covered by CI")
        self.assertTrue(runner.is_file())
        self.assertIn("bash tools/check-audit2-readiness.sh", workflow.read_text())
        self.assertIn("actions/setup-python@", workflow.read_text())
        self.assertIn("numpy==2.3.5 mpmath==1.3.0", workflow.read_text())
        text = runner.read_text()
        self.assertIn("--features audit2-research --test audit2_structured_correction_contracts", text)
        self.assertIn("--test audit2_matrix_free_common_w_contracts", text)
        self.assertIn("--no-default-features --example solve_stiff", text)
        self.assertIn("output_accuracy_assessment_contracts", text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
