#!/usr/bin/env python3
"""Verify only the bounded successor delta after the frozen semantic-control baseline.

This replaces the defective c6..package exhaustive positive allowlist.  The
3f2f771 semantic package is the finite trusted control baseline; only the
small transition-policy repair after that baseline is classified here.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess

ALLOWED_REPAIR_PATHS = {
    "START_CONTINUATION_R2.sh",
    "HOST_CODEX_CONTINUE_R2.md",
    "docs/exec-plans/k0-stage-telemetry-integration-20260827/"
    "WU05_SEMANTIC_CONTINUATION_AUTHORITY.json",
    "tools/verify-k0-semantic-transition-scope.py",
    "tools/test_k0_semantic_transition_scope.py",
}


def stop(message: str) -> "NoReturn":
    print(json.dumps({
        "status": "BLOCKED_BY_AUTHORITY_DRIFT",
        "disposition": "PROVENANCE_REBIND_REQUIRED",
        "scientific_failure": False,
        "error": message,
    }, indent=2))
    raise SystemExit(2)


def git(repo: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        ["git", *args],
        cwd=repo,
        text=True,
        capture_output=True,
        check=False,
    )
    if check and proc.returncode != 0:
        stop(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--trusted-base", required=True)
    parser.add_argument("--package-sha", required=True)
    args = parser.parse_args()

    repo = pathlib.Path(args.repo_root).resolve()
    for label, value in (("trusted-base", args.trusted_base), ("package-sha", args.package_sha)):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            stop(f"{label} must be exact lowercase 40-hex")

    git(repo, "cat-file", "-e", f"{args.trusted_base}^{{commit}}")
    git(repo, "cat-file", "-e", f"{args.package_sha}^{{commit}}")
    if git(
        repo,
        "merge-base",
        "--is-ancestor",
        args.trusted_base,
        args.package_sha,
        check=False,
    ).returncode != 0:
        stop("package does not descend from the frozen semantic-control baseline")

    raw = git(
        repo,
        "diff",
        "--name-only",
        "-z",
        args.trusted_base,
        args.package_sha,
    ).stdout
    changed = {item for item in raw.split("\0") if item}
    forbidden = sorted(changed - ALLOWED_REPAIR_PATHS)
    missing = sorted(ALLOWED_REPAIR_PATHS - changed)

    if forbidden:
        stop(f"successor repair changed non-authorized path(s): {forbidden}")
    if missing:
        stop(f"successor repair is incomplete; missing path(s): {missing}")

    print(json.dumps({
        "status": "PASS",
        "marker": "SEMANTIC_TRANSITION_SCOPE_PASS",
        "trusted_base": args.trusted_base,
        "package_sha": args.package_sha,
        "changed_paths": sorted(changed),
        "identity_disposition": {
            "result_validity": "NOT_EVALUATED",
            "provenance_validity": "REBIND_READY",
            "packaging_validity": "STRUCTURALLY_BOUND",
            "scientific_failure": False,
        },
    }, indent=2))


if __name__ == "__main__":
    main()
