#!/usr/bin/env python3
"""Test receipts of research node AS05 (research/as05_residual_gain_core_20261011/PREREGISTRATION.md).

Runs, at the current commit and on a clean tree, registered commands 1 and 2 and the pre-existing rodas5p-core suite:

    cargo test --offline --locked -p rodas5p-core --test residual_gain_contract
    cargo test --offline --locked -p rodas5p-core --doc residual_gain
    cargo test --offline --locked -p rodas5p-core --all-targets

and writes RECEIPTS.json (schema vigilode-as05-receipts-v1): the commit, the ``git status --porcelain`` before and
after, and per command the argv, exit code and the ``Running``/``Doc-tests``/``test`` lines of the combined output.
Write it outside the tree (the tree must stay clean for the export) and copy it next to RUNS.json afterwards; the
checker reads it there. The cargo environment (CARGO_TARGET_DIR etc.) is inherited.

Usage: python3 -B tools/as05_receipts.py --output /path/outside/the/tree/RECEIPTS.json
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parent.parent
COMMANDS = (
    ("contract_tests", ["cargo", "test", "--offline", "--locked", "-p", "rodas5p-core", "--test",
                        "residual_gain_contract"]),
    ("doc_residual_gain", ["cargo", "test", "--offline", "--locked", "-p", "rodas5p-core", "--doc", "residual_gain"]),
    ("rodas5p_core_all_targets", ["cargo", "test", "--offline", "--locked", "-p", "rodas5p-core", "--all-targets"]),
)
KEEP = re.compile(r"^\s*(Running |Doc-tests |test |error)")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=REPO, capture_output=True, text=True, check=True).stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    if out.exists():
        raise SystemExit(f"immutable output exists: {out}")
    if REPO in out.parents:
        raise SystemExit("write the receipts outside the tree, then copy them next to RUNS.json")
    commit = git("rev-parse", "HEAD").strip()
    before = git("status", "--porcelain")
    commands = []
    for name, argv in COMMANDS:
        run = subprocess.run(argv, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        lines = [ln.strip() for ln in run.stdout.splitlines() if KEEP.match(ln)]
        commands.append({"name": name, "argv": argv, "exit_code": run.returncode, "lines": lines})
        print(f"{name}: exit {run.returncode}, {len(lines)} lines")
    after = git("status", "--porcelain")
    doc = {"schema": "vigilode-as05-receipts-v1", "commit": commit, "tree_status_before": before,
           "tree_status_after": after, "commands": commands}
    out.write_text(json.dumps(doc, indent=1) + "\n")
    return 0 if all(c["exit_code"] == 0 for c in commands) and not before and not after else 1


if __name__ == "__main__":
    sys.exit(main())
