#!/usr/bin/env python3
"""Acceptance contract for the durable VigilODE A1 handoff.

Run this before any implementation-branch mutation. Standard library only.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REQUIRED_HANDOFF_FILES = (
    "AGENTS.md",
    "README.md",
    "CURRENT_STATE.json",
    "AUDIT_COMPILED_EXEC_PLAN.yaml",
    "P0_P1_THREAT_CATALOG.yaml",
    "INVARIANT_TEST_MATRIX.yaml",
    "IMPLEMENTER_PROMPT.md",
    "FRESH_REVIEW_PROMPT.md",
    "CODEX_LAUNCHER.md",
    "tools/discover_a1_tolerance_sites.py",
    "acceptance/test_handoff_contract.py",
)


def run_git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, help="A1 implementation worktree")
    parser.add_argument("--handoff", required=True, help="detached handoff worktree")
    args = parser.parse_args()

    repo = Path(args.repo).expanduser().resolve()
    handoff = Path(args.handoff).expanduser().resolve()
    checks: list[dict[str, Any]] = []

    def check(name: str, condition: bool, detail: Any) -> None:
        checks.append({"name": name, "pass": bool(condition), "detail": detail})

    for rel in REQUIRED_HANDOFF_FILES:
        path = handoff / rel
        check(f"handoff file exists: {rel}", path.is_file(), str(path))
        if path.is_file():
            check(f"handoff file nonempty: {rel}", path.stat().st_size > 0, path.stat().st_size)

    state_path = handoff / "CURRENT_STATE.json"
    try:
        state = json.loads(state_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(json.dumps({"pass": False, "error": f"invalid CURRENT_STATE.json: {exc}"}, indent=2))
        return 2

    base = state["canonical_main"]["commit"]
    expected_head = state["implementation"]["head"]
    expected_tree = state["implementation"]["tree"]
    expected_paths = set(state["implementation"]["expected_changed_paths"])

    check(
        "handoff branch name is exact",
        state["handoff"]["branch"] == "handoff/a1-inner-tolerance-parity-20260825",
        state["handoff"]["branch"],
    )
    check("handoff merge is forbidden", state["handoff"]["merge_forbidden"] is True, state["handoff"])
    check("handoff is read only", state["handoff"]["read_only"] is True, state["handoff"])
    check("PR #18 is pinned", state["pull_request"]["number"] == 18, state["pull_request"])
    check("PR is draft", state["pull_request"]["draft"] is True, state["pull_request"])
    check("PR is unmerged", state["pull_request"]["merged"] is False, state["pull_request"])

    try:
        head = run_git(repo, "rev-parse", "HEAD")
        tree = run_git(repo, "rev-parse", "HEAD^{tree}")
        merge_base = run_git(repo, "merge-base", "HEAD", base)
        ahead_behind = run_git(repo, "rev-list", "--left-right", "--count", f"{base}...HEAD")
        changed = set(
            line
            for line in run_git(repo, "diff", "--name-only", f"{base}...HEAD").splitlines()
            if line
        )
        tracked_status = run_git(repo, "status", "--porcelain", "--untracked-files=no")
    except subprocess.CalledProcessError as exc:
        print(
            json.dumps(
                {
                    "pass": False,
                    "error": "git intake command failed",
                    "command": exc.cmd,
                    "stdout": exc.stdout,
                    "stderr": exc.stderr,
                },
                indent=2,
            )
        )
        return 2

    check("implementation HEAD matches intake", head == expected_head, {"expected": expected_head, "actual": head})
    check("implementation tree matches intake", tree == expected_tree, {"expected": expected_tree, "actual": tree})
    check("canonical merge base matches", merge_base == base, {"expected": base, "actual": merge_base})
    check("implementation is 0 behind and 8 ahead", ahead_behind == "0\t8" or ahead_behind == "0 8", ahead_behind)
    check(
        "implementation diff surface matches exactly",
        changed == expected_paths,
        {"expected": sorted(expected_paths), "actual": sorted(changed)},
    )
    check("tracked implementation worktree is clean", tracked_status == "", tracked_status)
    check(
        "handoff artifacts are not in implementation diff",
        not any(path in changed for path in REQUIRED_HANDOFF_FILES),
        sorted(changed),
    )

    for rel, marker in (
        ("AUDIT_COMPILED_EXEC_PLAN.yaml", "schema: vigilode-a1-audit-compiled-exec-plan-v1"),
        ("P0_P1_THREAT_CATALOG.yaml", "schema: vigilode-a1-threat-catalog-v1"),
        ("INVARIANT_TEST_MATRIX.yaml", "schema: vigilode-a1-invariant-test-matrix-v1"),
    ):
        text = (handoff / rel).read_text(encoding="utf-8")
        check(f"schema marker: {rel}", marker in text, marker)

    discovery = subprocess.run(
        [
            sys.executable,
            str(handoff / "tools/discover_a1_tolerance_sites.py"),
            "--repo",
            str(repo),
        ],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    check(
        "callsite discovery audit passes",
        discovery.returncode == 0,
        {
            "returncode": discovery.returncode,
            "stdout": discovery.stdout,
            "stderr": discovery.stderr,
        },
    )

    passed = all(item["pass"] for item in checks)
    payload = {
        "schema": "vigilode-a1-handoff-acceptance-result-v1",
        "pass": passed,
        "repo": str(repo),
        "handoff": str(handoff),
        "base": base,
        "head": head,
        "tree": tree,
        "checks": checks,
    }
    print(json.dumps(payload, indent=2, sort_keys=True))
    if not passed:
        failed = [item["name"] for item in checks if not item["pass"]]
        print("FAILED CHECKS: " + "; ".join(failed), file=sys.stderr)
        return 1

    print("PASS: vigilode A1 handoff contract")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
