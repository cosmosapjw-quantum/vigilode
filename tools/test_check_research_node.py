#!/usr/bin/env python3
"""Tests for tools/check-research-node.py using throwaway Git repositories."""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOOL = ROOT / "tools" / "check-research-node.py"
LEDGER = "research/LEDGER.jsonl"
ENV = {
    **os.environ,
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "test",
    "GIT_AUTHOR_EMAIL": "test@example.invalid",
    "GIT_COMMITTER_NAME": "test",
    "GIT_COMMITTER_EMAIL": "test@example.invalid",
    "PYTHONDONTWRITEBYTECODE": "1",
}
COMMIT = "0" * 40


class Repo:
    def __init__(self, path: pathlib.Path):
        self.path = path
        self.next_id = 1
        self.git("init", "-q", "-b", "main")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args], cwd=self.path, env=ENV, check=True, capture_output=True, text=True
        ).stdout

    def write(self, rel: str, text: str) -> None:
        target = self.path / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def sha(self, rel: str) -> str:
        return hashlib.sha256((self.path / rel).read_bytes()).hexdigest()

    def row(self, outputs: list[str], verdict: str = "PASS", date: str = "2026-09-01", **extra) -> dict:
        row = {
            "id": f"L-{self.next_id:04d}",
            "date": date,
            "commit": COMMIT,
            "command": "python3 run.py",
            "profile": "test",
            "inputs_sha256": {},
            "outputs_sha256": {p: self.sha(p) for p in outputs},
            "claim": "A test claim.",
            "verdict": verdict,
            "supersedes": None,
        }
        row.update(extra)
        self.next_id += 1
        return row

    def append_rows(self, *rows: dict) -> None:
        with (self.path / LEDGER).open("a", encoding="utf-8") as handle:
            for row in rows:
                handle.write(json.dumps(row) + "\n")

    def node(self, name: str, verdict: str = "PASS", prereg: bool = True, date: str = "2026-09-01") -> dict:
        if prereg:
            self.write(f"research/{name}/PREREGISTRATION.md", "# Plan\n\nHypothesis and gate.\n")
        self.write(f"research/{name}/results.csv", "case,error\na,1.5e-3\n")
        row = self.row([f"research/{name}/results.csv"], verdict=verdict, date=date)
        self.append_rows(row)
        return row

    def commit(self, message: str) -> None:
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)

    def check(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(TOOL), *args], cwd=self.path, env=ENV, capture_output=True, text=True
        )


class ResearchNodeTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Repo(pathlib.Path(self._tmp.name))
        self.repo.write("research/old/README.md", "legacy node without a pre-registration\n")
        self.repo.write("research/old/result.json", json.dumps({"value": 1.0}))
        self.repo.append_rows(self.repo.row(["research/old/result.json"], date="2026-08-01"))
        self.repo.commit("base with ledger")
        self.repo.git("branch", "base")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def assertFails(self, proc: subprocess.CompletedProcess, needle: str) -> None:
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn(needle, proc.stdout)

    def assertPasses(self, proc: subprocess.CompletedProcess) -> None:
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("PASS", proc.stdout)

    def test_valid_new_node_passes(self):
        self.repo.node("new_node")
        self.repo.commit("add node")
        proc = self.repo.check("--base", "base")
        self.assertPasses(proc)
        self.assertIn("new_node", proc.stdout)

    def test_missing_preregistration_fails(self):
        self.repo.node("new_node", prereg=False)
        self.repo.commit("add node without prereg")
        self.assertFails(self.repo.check("--base", "base"), "missing PREREGISTRATION.md")

    def test_new_node_without_ledger_row_fails(self):
        self.repo.write("research/new_node/PREREGISTRATION.md", "# Plan\n")
        self.repo.write("research/new_node/results.csv", "case,error\na,2.0\n")
        self.repo.commit("add node without ledger row")
        self.assertFails(self.repo.check("--base", "base"), "no research/LEDGER.jsonl row")

    def test_new_node_without_numeric_results_fails(self):
        self.repo.write("research/new_node/PREREGISTRATION.md", "# Plan\n")
        self.repo.write("research/new_node/notes.json", json.dumps({"status": "planned"}))
        self.repo.append_rows(self.repo.row(["research/new_node/notes.json"]))
        self.repo.commit("add prose-only node")
        self.assertFails(self.repo.check("--base", "base"), "no results file with numeric rows")

    def test_edited_ledger_row_fails(self):
        text = (self.repo.path / LEDGER).read_text().replace("A test claim.", "A better claim.")
        (self.repo.path / LEDGER).write_text(text)
        self.repo.commit("edit a ledger row")
        self.assertFails(self.repo.check("--base", "base"), "append-only violation")

    def test_removed_ledger_row_fails(self):
        (self.repo.path / LEDGER).write_text("")
        self.repo.commit("remove a ledger row")
        self.assertFails(self.repo.check("--base", "base"), "were removed")

    def test_bad_verdict_fails(self):
        self.repo.node("new_node", verdict="MAYBE")
        self.repo.commit("bad verdict")
        self.assertFails(self.repo.check("--base", "base"), "verdict must be one of")
        self.assertFails(self.repo.check(), "verdict must be one of")

    def test_schema_rejects_unknown_field_and_dangling_supersedes(self):
        self.repo.write("research/old/extra.json", json.dumps({"x": 2}))
        self.repo.append_rows(
            self.repo.row(["research/old/extra.json"], supersedes="L-9999", note="x")
        )
        proc = self.repo.check()
        self.assertFails(proc, "unknown fields")
        self.assertIn("supersedes must be null or the id of an earlier row", proc.stdout)

    def test_unrecorded_command_cannot_pass(self):
        self.repo.write("research/old/extra.json", json.dumps({"x": 2}))
        self.repo.append_rows(self.repo.row(["research/old/extra.json"], command="UNRECORDED: lost"))
        self.assertFails(self.repo.check(), "UNRECORDED command requires")

    def test_output_hash_mismatch_fails(self):
        self.repo.write("research/old/result.json", json.dumps({"value": 2.0}))
        self.assertFails(self.repo.check(), "sha256 mismatch")

    def test_stall_rule_triggers_on_third_node_without_numeric_result(self):
        self.repo.node("a_node", verdict="INCONCLUSIVE", date="2026-09-01")
        self.repo.node("b_node", verdict="INVALID", date="2026-09-02")
        self.repo.node("c_node", verdict="INCONCLUSIVE", date="2026-09-03")
        self.repo.commit("three process-only nodes")
        proc = self.repo.check("--base", "base")
        self.assertFails(proc, "stall rule: node research/c_node")
        self.assertNotIn("stall rule: node research/b_node", proc.stdout)

    def test_stall_rule_allows_two_and_is_broken_by_a_numeric_result(self):
        self.repo.node("a_node", verdict="INCONCLUSIVE", date="2026-09-01")
        self.repo.node("b_node", verdict="INVALID", date="2026-09-02")
        self.repo.commit("two process-only nodes")
        self.assertPasses(self.repo.check("--base", "base"))
        self.repo.node("c_node", verdict="FAIL", date="2026-09-03")
        self.repo.commit("a node with a numeric result")
        self.assertPasses(self.repo.check("--base", "base"))

    def test_nodes_present_when_ledger_was_introduced_are_frozen(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(pathlib.Path(tmp))
            repo.write("README.md", "root\n")
            repo.commit("before research")
            repo.git("branch", "base")
            repo.write("research/legacy/notes.md", "process-only history\n")
            repo.commit("legacy node")
            repo.write("research/old/result.json", json.dumps({"value": 1}))
            repo.append_rows(repo.row(["research/old/result.json"]))
            repo.commit("introduce ledger")
            proc = repo.check("--base", "base")
            self.assertPasses(proc)
            self.assertIn("new nodes=none", proc.stdout)


if __name__ == "__main__":
    unittest.main()
