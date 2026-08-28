#!/usr/bin/env python3
"""Focused regression for the observed permanent-one-blocker transition defect."""
from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve()
HELPER = HERE.with_name("verify-k0-semantic-transition-scope.py")
ROOT = "docs/exec-plans/k0-stage-telemetry-integration-20260827"
ALLOWED_SUCCESSOR = {
    "START_CONTINUATION_R2.sh": "# successor runner\n",
    "HOST_CODEX_CONTINUE_R2.md": "# successor prompt\n",
    f"{ROOT}/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json": "{}\n",
    "tools/verify-k0-semantic-transition-scope.py": None,
    "tools/test_k0_semantic_transition_scope.py": None,
}


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        ["git", *args], cwd=repo, text=True, capture_output=True, check=False
    )
    if check and proc.returncode != 0:
        raise AssertionError(f"git {' '.join(args)} failed: {proc.stderr}")
    return proc


class Fixture:
    def __init__(self, directory: str, forbidden_successor: bool = False):
        self.repo = Path(directory) / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-b", "main")
        git(self.repo, "config", "user.name", "K0 transition fixture")
        git(self.repo, "config", "user.email", "fixture@example.invalid")

        self.write("README.md", "base\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "base")
        self.old_base = git(self.repo, "rev-parse", "HEAD").stdout.strip()

        # Frozen semantic baseline deliberately includes the exact path that the
        # defective c6..package positive allowlist rejected.
        self.write(f"{ROOT}/START_CONTINUATION.sh", "# trusted legacy semantic entry\n")
        self.write(f"{ROOT}/schemas/stage-receipt-v3.schema.json", "{}\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "frozen semantic-control baseline")
        self.trusted = git(self.repo, "rev-parse", "HEAD").stdout.strip()

        for path, content in ALLOWED_SUCCESSOR.items():
            if content is None:
                content = (
                    HELPER
                    if path.endswith("verify-k0-semantic-transition-scope.py")
                    else HERE
                ).read_text()
            self.write(path, content)
        if forbidden_successor:
            self.write("crates/solver.rs", "// unauthorized source mutation\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "bounded transition repair")
        self.package = git(self.repo, "rev-parse", "HEAD").stdout.strip()

    def write(self, relative: str, content: str) -> None:
        target = self.repo / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def verify(self, base: str | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                "-B",
                str(HELPER),
                "--repo-root",
                str(self.repo),
                "--trusted-base",
                base or self.trusted,
                "--package-sha",
                self.package,
            ],
            text=True,
            capture_output=True,
            check=False,
        )


class TransitionScopeTests(unittest.TestCase):
    def fixture(self, **kwargs) -> Fixture:
        tmp = tempfile.TemporaryDirectory(prefix="k0-transition-")
        self.addCleanup(tmp.cleanup)
        return Fixture(tmp.name, **kwargs)

    def test_trusted_baseline_absorbs_the_historical_28_path_semantic_package(self) -> None:
        f = self.fixture()
        historical = git(
            f.repo, "diff", "--name-only", f.old_base, f.trusted
        ).stdout.splitlines()
        self.assertIn(f"{ROOT}/START_CONTINUATION.sh", historical)
        proc = f.verify()
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("SEMANTIC_TRANSITION_SCOPE_PASS", proc.stdout)

    def test_rechecking_from_the_old_base_reproduces_the_false_fail(self) -> None:
        f = self.fixture()
        proc = f.verify(base=f.old_base)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("PROVENANCE_REBIND_REQUIRED", proc.stdout)
        self.assertIn(f"{ROOT}/START_CONTINUATION.sh", proc.stdout)

    def test_real_source_change_after_trusted_baseline_still_blocks(self) -> None:
        f = self.fixture(forbidden_successor=True)
        proc = f.verify()
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("crates/solver.rs", proc.stdout)
        self.assertIn('"scientific_failure": false', proc.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
