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
        git(self.repo, "remote", "set-url", "origin", "https://github.com/test/local.git")

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

    def publish(self) -> None:
        git(self.repo, "push", "-q", str(self.origin), "HEAD:refs/heads/main")
        git(self.repo, "fetch", "-q", str(self.origin), "refs/heads/main:refs/remotes/origin/main")

    def publication_map(self, source=ABSENT, target=None, tree=None) -> None:
        self.write("research/node/PUBLISH_MAP.json", json.dumps({
            "schema": "vigilode.git-publication-map.v1",
            "repository": "test/local",
            "mappings": [{"local_evidence_commit": source,
                          "remote_equivalent_commit": target or self.published,
                          "identical_tree_sha": tree or git(self.repo, "rev-parse", f"{self.published}^{{tree}}") }],
        }))

    def test_opaque_github_payload_is_not_authority(self):
        self.write("research/node/github.json", json.dumps({
            "pulls": [{"body": f"source commit {ABSENT}"}],
            "structuredContent": {"content": json.dumps({"merge_commit_sha": ABSENT})},
        }))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertNotIn(ABSENT, proc.stdout)
        self.write("research/node/receipt.json", json.dumps({"source_commit": ABSENT}))
        self.commit_all()
        self.assertEqual(self.check().returncode, 1)

    def test_julia_package_tree_does_not_become_local_commit(self):
        self.write("research/node/Manifest.toml", f'[[deps.Example]]\ngit-tree-sha1 = "{ABSENT}"\n')
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertNotIn(ABSENT, proc.stdout)
        self.write("research/node/Manifest.toml", f'source_commit = "{ABSENT}"\n')
        self.commit_all()
        self.assertEqual(self.check().returncode, 1)

    def test_explicit_commit_cannot_be_a_tree(self):
        self.write("research/node/receipt.json", json.dumps({"source_commit": self.tree}))
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("object-kind-mismatch", proc.stdout)

    def test_missing_local_tree_stays_a_failure(self):
        self.write("research/node/receipt.json", json.dumps({"source": {"tree": ABSENT}}))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("tree", proc.stdout)

    def test_external_repository_requires_matching_checkout(self):
        self.write("research/node/receipt.json", json.dumps({
            "repo": "test/external", "source_commit": self.published,
        }))
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("repository-unavailable", proc.stdout)
        proc = self.check("--repository", f"test/external={self.repo}")
        self.assertEqual(proc.returncode, 2, proc.stdout + proc.stderr)
        self.assertIn("repository identity mismatch", proc.stdout + proc.stderr)
        external = self.repo.parent / "external"
        git(self.repo.parent, "clone", "-q", str(self.origin), str(external))
        git(external, "remote", "set-url", "origin", "https://github.com/test/external.git")
        proc = self.check("--repository", f"test/external={external}", "--list")
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("repository=test/external", proc.stdout)

    def test_published_equal_tree_map_preserves_source_identity(self):
        self.publication_map()
        self.write("research/node/receipt.json", json.dumps({"source_commit": ABSENT}))
        self.commit_all()
        self.publish()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertIn(f"tree-equivalent          {ABSENT}", proc.stdout)
        self.assertIn("PUBLISH_MAP.json", proc.stdout)

    def test_mapping_cannot_satisfy_remote_ancestry_relation(self):
        self.publication_map()
        self.write("research/node/receipt.json", json.dumps({
            "repository": "test/local", "object_kind": "commit", "identifier": ABSENT,
            "required_relation": "remote-reachable",
        }))
        self.commit_all()
        self.publish()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"{ABSENT} is missing", proc.stdout)

    def test_mapping_cannot_turn_explicit_tree_or_blob_into_commit(self):
        self.publication_map()
        self.write("research/node/receipt.json", json.dumps({
            "objects": [
                {"object_kind": "tree", "identifier": ABSENT},
                {"object_kind": "blob", "identifier": ABSENT},
                {"object_kind": "tree", "source_commit": ABSENT},
            ]
        }))
        self.commit_all()
        self.publish()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"{ABSENT} is missing", proc.stdout)
        self.assertIn("object_kind=tree", proc.stdout)
        self.assertIn("object_kind=blob", proc.stdout)

    def test_source_repository_binds_revision_and_blob(self):
        self.write("research/node/receipt.json", json.dumps({"source": {
            "source_repository": "test/external", "source_revision": self.published,
            "source_blob": self.tree,
        }}))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("repository=test/external", proc.stdout)
        self.assertIn("source.source_revision", proc.stdout)
        self.assertIn("source.source_blob", proc.stdout)
        self.assertNotIn("reachable", proc.stdout)

    def test_markdown_github_commit_url_binds_external_repository(self):
        self.write("docs/NOTE.md", f"Source https://github.com/test/external/commit/{self.published}\n")
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("repository=test/external", proc.stdout)

    def test_malformed_explicit_typed_records_fail(self):
        self.write("research/node/receipt.json", json.dumps({"objects": [
            {"object_kind": "commit", "identifier": "not-a-git-object"},
            {"object_kind": "treee", "identifier": self.published},
        ]}))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("invalid-identifier", proc.stdout)
        self.assertIn("invalid-kind", proc.stdout)

    def test_scientific_content_is_not_opaque_capture(self):
        self.write("research/node/receipt.json", json.dumps({
            "source": {"content": f"source commit {ABSENT}"}
        }))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"{ABSENT} is missing", proc.stdout)

    def test_tree_commit_and_tree_ref_are_commits_but_source_tree_is_tree(self):
        self.write("research/node/receipt.json", json.dumps({
            "tree_commit": self.published, "tree_ref": self.published,
            "source_tree": self.published,
        }))
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("object-kind-mismatch", proc.stdout)
        self.assertIn("tree_commit;", proc.stdout)
        self.assertIn("tree_ref;", proc.stdout)

    def test_prose_repo_proximity_does_not_rebind_local_commit(self):
        self.write("research/node/receipt.json", json.dumps({
            "repository": "test/external", "source_commit": self.published,
        }))
        self.write("docs/NOTE.md", f"BASS/REI branch {self.published} was consulted.\n")
        self.commit_all()
        proc = self.check("--list")
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"reachable                {self.published} docs/NOTE.md", proc.stdout)
        self.assertIn("repository=test/local", proc.stdout)

    def test_unpublished_or_wrong_tree_mapping_fails(self):
        self.publication_map()
        self.write("research/node/receipt.json", json.dumps({"source_commit": ABSENT}))
        self.commit_all()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("mapping is not published unchanged", proc.stdout)
        self.publication_map(tree=self.tree)
        self.commit_all()
        self.publish()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("mapping target tree mismatch", proc.stdout)

    def test_unknown_mapping_schema_cannot_authorize_missing_source(self):
        self.publication_map()
        path = self.repo / "research/node/PUBLISH_MAP.json"
        value = json.loads(path.read_text())
        value["schema"] = "unrecognized.v1"
        path.write_text(json.dumps(value))
        self.commit_all()
        self.publish()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn(f"{ABSENT} is missing", proc.stdout)

    def test_mapping_detects_existing_local_tree_disagreement(self):
        self.publication_map(source=self.local_only)
        self.commit_all()
        self.publish()
        proc = self.check()
        self.assertEqual(proc.returncode, 1, proc.stdout)
        self.assertIn("mapping local tree mismatch", proc.stdout)

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
