#!/usr/bin/env python3
"""Four bounded safety tests using tiny temporary Git repositories, no solver work."""

import argparse
import contextlib
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import prepare_validation as helper


class PreparationSafetyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="vigilode-prepare-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repository"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Preparation fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.repo / "crates").mkdir()
        self.solver = self.repo / "crates/solver.rs"
        self.solver.write_text("observed main\n")
        observed_main = self.commit("observed main")
        self.solver.write_text("conversation baseline\n")
        baseline = self.commit("conversation baseline")
        prs = {}
        for number in (90, 91, 92, 93):
            self.solver.write_text(f"PR{number}\n")
            prs[number] = self.commit(f"PR{number}")
        (self.repo / "integrated.txt").write_text("registered exact source fixture\n")
        integrated = self.commit("integrated source")
        self.required = helper.RequiredSources(prs, integrated, baseline, observed_main)
        scope = {"schema_version": 1, "source_head": prs[93], "pre_conversation_commit": baseline,
                 "observed_main_commit": observed_main,
                 "conversation_prs": [{"number": n, "head_commit": sha} for n, sha in prs.items()]}
        coverage = {"schema_version": 1, "source_commit": prs[93], "conversation_baseline": baseline,
                    "observed_main": observed_main, "cumulative_changed_path_count": 1,
                    "changed_scope": ["crates"], "gate_groups": {"G-FIXTURE": {}},
                    "paths": [{"path": "crates/solver.rs", "gate_ids": ["G-FIXTURE"],
                               "source_sha256": hashlib.sha256(self.solver.read_bytes()).hexdigest()}]}
        commands = {"schema_version": 1, "commands": [{"id": "never-executed-fixture"}]}
        for name, value in (("scope", scope), ("coverage", coverage), ("commands", commands)):
            path = self.repo / helper.MANIFESTS[name]
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value))
        self.source = self.commit("delivery with committed manifests")
        self.destination = self.root / "independent"
        self.branch = "validation/fresh"

    def git(self, *args, input=None):
        return subprocess.run(["git", "-C", str(self.repo), *args], input=input, text=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout.strip()

    def commit(self, message):
        self.git("add", ".")
        self.git("commit", "-m", message)
        return self.git("rev-parse", "HEAD")

    def plan(self, **kwargs):
        values = {"repository": self.repo, "source": self.source, "destination": self.destination,
                  "branch": self.branch, "required": self.required}
        values.update(kwargs)
        return helper.build_plan(**values)

    def invoke(self, *extra):
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = helper.main(["--source-sha", self.source, "--destination", str(self.destination),
                                "--branch", self.branch, *extra],
                               required=self.required, invocation_dir=self.repo / "crates")
        return code, stdout.getvalue(), stderr.getvalue()

    def test_default_dry_run_uses_committed_manifests_and_cwd(self):
        (self.repo / helper.MANIFESTS["scope"]).write_text("invalid live checkout JSON")
        code, stdout, stderr = self.invoke()
        self.assertEqual((code, stderr), (0, ""))
        plan = json.loads(stdout)
        self.assertEqual(plan["status"], "DRY_RUN")
        self.assertEqual(plan["native_validation_status"], "NOT_RUN")
        self.assertTrue(plan["caller_checkout"]["dirty"])
        self.assertEqual(len(plan["source_inclusion"]), 7)
        self.assertFalse(self.destination.exists())
        self.assertNotIn("refs/heads/" + self.branch, self.git("show-ref"))
        self.assertTrue(all(m["source_commit"] == self.source for m in plan["plan_manifests"]))

    def test_refuses_bad_source_existing_targets_and_branch_aliases(self):
        with self.assertRaises(argparse.ArgumentTypeError):
            helper.full_sha("HEAD")
        unrelated = self.git("commit-tree", self.git("rev-parse", "HEAD^{tree}"), input="unrelated\n")
        with self.assertRaises(helper.PreparationError):
            self.plan(source=unrelated)
        with self.assertRaises(helper.PreparationError):
            self.plan(source=self.required.integrated)  # Required manifest blobs are absent.
        self.destination.mkdir()
        with self.assertRaises(helper.PreparationError):
            self.plan()
        self.destination.rmdir()
        self.git("branch", self.branch)
        with self.assertRaises(helper.PreparationError):
            self.plan()
        for branch in ("main", "bad branch", "@{-1}"):
            with self.subTest(branch=branch), self.assertRaises(helper.PreparationError):
                self.plan(branch=branch)
        with self.assertRaises(helper.PreparationError):
            self.plan(branch="validation/nested", destination=self.repo / "nested")

    def test_descendant_hash_changes_and_new_unmapped_paths_are_recorded(self):
        self.solver.write_text("future native implementation; not executed\n")
        (self.repo / "crates/new.rs").write_text("new native path; not executed\n")
        self.source = self.commit("future native changes")
        plan = self.plan()
        coverage = plan["coverage"]
        self.assertEqual(coverage["changed_or_missing_covered_files"], 1)
        self.assertEqual(coverage["files"][0]["status"], "changed")
        changed = {c["path"]: c for c in coverage["scope_changes_since_coverage_baseline"]}
        self.assertEqual(changed["crates/solver.rs"]["gate_ids"], ["G-FIXTURE"])
        self.assertTrue(changed["crates/new.rs"]["coverage_mapping_required"])
        self.assertEqual(coverage["qualification_status"], "NOT_RUN")
        self.assertFalse(self.destination.exists())

    def test_create_is_clean_pinned_hook_free_and_refuses_reuse(self):
        divergent_main = self.git("commit-tree", self.git("rev-parse", "HEAD^{tree}"),
                                  "-p", self.required.observed_main, input="new divergent main\n")
        self.git("update-ref", "refs/remotes/origin/main", divergent_main)
        marker = self.root / "hook-ran"
        hook = self.repo / ".git/hooks/post-checkout"
        hook.write_text(f"#!/bin/sh\ntouch '{marker}'\n")
        hook.chmod(0o755)
        self.solver.write_text("uncommitted caller changes must remain here\n")
        code, stdout, stderr = self.invoke("--create")
        self.assertEqual((code, stderr), (0, ""))
        plan = json.loads(stdout)
        self.assertEqual(plan["status"], "CREATED")
        self.assertEqual(plan["target"]["checked_local_main_commit"], divergent_main)
        self.assertEqual(plan["target"]["current_main_relation"], "diverged")
        self.assertTrue(plan["target"]["reconciliation_required"])
        self.assertEqual(plan["target"]["common_merge_bases"], [self.required.observed_main])
        self.assertEqual(plan["target"]["cleanliness"], "VERIFIED_CLEAN_AT_SELECTED_SOURCE")
        self.assertEqual(plan["native_validation_status"], "NOT_RUN")
        self.assertFalse(marker.exists())
        self.assertEqual((self.destination / "crates/solver.rs").read_text(), "PR93\n")
        self.assertIn("uncommitted caller", self.solver.read_text())
        created = helper.Git(self.destination)
        self.assertEqual(created.text("rev-parse", "HEAD"), self.source)
        self.assertEqual(created.run("status", "--porcelain=v1"), b"")
        code, stdout, stderr = self.invoke("--create")
        self.assertEqual((code, stdout), (2, ""))
        self.assertEqual(json.loads(stderr)["status"], "REFUSED")
        self.assertEqual(created.text("rev-parse", "HEAD"), self.source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
