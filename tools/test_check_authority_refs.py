#!/usr/bin/env python3
"""Tests for tools/check-authority-refs.py using throwaway Git repositories."""
from __future__ import annotations

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOOL = ROOT / "tools" / "check-authority-refs.py"
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
ABSENT = "1234567890abcdef1234567890abcdef12345678"


def git(cwd: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, env=ENV, check=True, capture_output=True, text=True
    ).stdout.strip()


class AuthorityRefsTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        tmp = pathlib.Path(self._tmp.name)
        self.origin = tmp / "origin.git"
        self.repo = tmp / "clone"
        seed = tmp / "seed"
        seed.mkdir()
        git(seed, "init", "-q", "-b", "main")
        (seed / "README.md").write_text("seed\n")
        git(seed, "add", "-A")
        git(seed, "commit", "-q", "-m", "published commit")
        self.published = git(seed, "rev-parse", "HEAD")
        git(tmp, "clone", "-q", "--bare", str(seed), str(self.origin))
        git(tmp, "clone", "-q", str(self.origin), str(self.repo))
        (self.repo / "local.txt").write_text("never pushed\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "local-only commit")
        self.local_only = git(self.repo, "rev-parse", "HEAD")
        self.tree = git(self.repo, "rev-parse", "HEAD^{tree}")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def write(self, rel: str, text: str) -> None:
        path = self.repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def check(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(TOOL), *args], cwd=self.repo, env=ENV, capture_output=True, text=True
        )

    def commit_all(self) -> None:
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "docs")

    def test_reachable_sha_passes(self):
        self.write("docs/NOTE.md", f"Authority commit: `{self.published}`\n")
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertIn(f"reachable                {self.published} docs/NOTE.md:1", proc.stdout)

    def test_unreachable_local_only_sha_fails(self):
        self.write("research/node/README.md", f"Sealed head: {self.local_only}\n")
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"research/node/README.md:1: {self.local_only} is unreachable", proc.stdout)

    def test_missing_sha_in_json_key_fails(self):
        self.write("research/node/receipt.json", json.dumps({"provenance": {"source_commit": ABSENT}}))
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"{ABSENT} is missing", proc.stdout)
        self.assertIn("provenance.source_commit", proc.stdout)

    def test_allowlisted_sha_passes(self):
        self.write("research/node/README.md", f"Sealed head: {self.local_only}\nparent commit {ABSENT}\n")
        self.write(
            "tools/authority_refs_allowlist.txt",
            f"# comment\n{self.local_only} local-only worktree commit, bytes not published\n"
            f"{ABSENT} pre-repository history\n",
        )
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertIn("unreachable+allowlisted", proc.stdout)
        self.assertIn("missing+allowlisted", proc.stdout)

    def test_allowlist_entry_without_reason_fails(self):
        self.write("research/node/README.md", f"Sealed head: {self.local_only}\n")
        self.write("tools/authority_refs_allowlist.txt", f"{self.local_only}\n")
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("expected '<40-hex sha> <reason>'", proc.stdout)

    def test_non_authority_context_and_sha256_and_trees_are_ignored(self):
        digest = "ab" * 32
        self.write(
            "docs/NOTE.md",
            f"Random identifier {ABSENT} in prose.\n\nArtifact sha256: `{digest}`\n\n"
            f"Base tree: `{self.tree}`\n",
        )
        self.write("research/n/data.json", json.dumps({"label": ABSENT}))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertNotIn(ABSENT, proc.stdout)
        self.assertNotIn(digest, proc.stdout)
        self.assertIn(f"non-commit               {self.tree}", proc.stdout)

    def test_paths_limits_scope(self):
        self.write("docs/NOTE.md", f"Authority commit: {ABSENT}\n")
        self.write("research/node/README.md", f"Authority commit: {self.published}\n")
        self.commit_all()
        self.assertEqual(self.check().returncode, 1)
        proc = self.check("--paths", "research")
        self.assertEqual(proc.returncode, 0, proc.stdout)

    def test_untracked_files_are_not_scanned(self):
        self.write("docs/NOTE.md", f"Authority commit: {ABSENT}\n")
        proc = self.check()
        self.assertEqual(proc.returncode, 0, proc.stdout)


if __name__ == "__main__":
    unittest.main()
