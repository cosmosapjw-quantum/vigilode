#!/usr/bin/env python3
"""Route frozen A1 receipt checks without claiming that unrelated code is A1 evidence.

The old starting-head/diff/cell checks remain in the workflow and still run for
explicit research/a1-* work or any change (including deletion) to the frozen A1
research directory. Shared solver edits get ordinary regression CI instead.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

A1_RESEARCH = "research/a1_inner_tolerance_audit_20260825/"


def classify(head_ref: str, changed_paths: list[str]) -> tuple[bool, str]:
    if any(path.startswith(A1_RESEARCH) for path in changed_paths):
        return True, "frozen-a1-evidence-changed"
    if head_ref.startswith("research/a1-"):
        return True, "explicit-a1-research-branch"
    return False, "historical-a1-evidence-unchanged; ordinary regression CI applies"


def changed_paths(repo: pathlib.Path, base: str, head: str) -> list[str]:
    for revision in (base, head):
        if not re.fullmatch(r"[0-9a-f]{40}", revision):
            raise ValueError("base/head must be exact commit SHAs, not shell text or moving refs")
        subprocess.run(["git", "cat-file", "-e", f"{revision}^{{commit}}"],
                       cwd=repo, check=True, capture_output=True)
    # No rename detection: moving/deleting frozen evidence must still activate A1.
    result = subprocess.run(["git", "diff", "--no-renames", "--name-only", "-z", base, head, "--"],
                            cwd=repo, check=True, capture_output=True)
    return [p.decode("utf-8", errors="surrogateescape") for p in result.stdout.split(b"\0") if p]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", default=".", type=pathlib.Path)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--head-ref", required=True)
    parser.add_argument("--github-output", type=pathlib.Path)
    args = parser.parse_args()
    try:
        paths = changed_paths(args.repo_root, args.base, args.head)
        applicable, reason = classify(args.head_ref, paths)
        result = {
            "scope": "FROZEN_A1_RECEIPT" if applicable else "A1_RECEIPT_NOT_APPLICABLE",
            "applicable": applicable, "reason": reason,
            "base": args.base, "head": args.head, "changed_paths": paths,
            "scientific_receipt_validated": False,
        }
        if args.github_output:
            with args.github_output.open("a", encoding="utf-8") as stream:
                stream.write(f"applicable={str(applicable).lower()}\n")
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(json.dumps({"scope": "UNRESOLVED", "error": str(error),
                          "scientific_receipt_validated": False}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
