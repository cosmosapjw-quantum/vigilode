#!/usr/bin/env python3
"""The checks of ../check_checkpoint.py that it does not reach on the branch of node V18.

`check_checkpoint.py` stops at its first failure. On this branch that is the expected one: the
pre-seeded stub `crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse.rs`, whose digest
is recorded in CHECKPOINT.json, is the file this node implements. This script imports the
checker unchanged and runs every one of its identity, strict-JSON and allowlist checks, listing
all failures instead of stopping at the first. It edits nothing.

Usage (from the worktree root):

    python3 -I research/production_loop_20261116/dense_lu_client/check_remaining.py
"""

import fnmatch
import hashlib
import importlib.util
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOOP = HERE.parent
spec = importlib.util.spec_from_file_location("check_checkpoint", LOOP / "check_checkpoint.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)

# The commit the builder worktree of node V18 started from.
NODE_START = "587a139501729bbe886bb2b8c49b0ef893705d08"
# The paths the task of node V18 allows it to write.
ALLOWED_BY_THE_TASK = (
    "crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse.rs",
    "crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse/*",
    "crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_*.rs",
    "research/production_loop_20261116/dense_lu_client/*",
)


def allowed_by_the_task(path):
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in ALLOWED_BY_THE_TASK)


def main():
    root, loop, program = checker.ROOT, checker.HERE, checker.PROGRAM
    checkpoint = checker.strict(loop / "CHECKPOINT.json")
    base = checkpoint["base_commit"]
    failures = []
    protected = 0
    for path, expected in checkpoint["protected_blob_sha1"].items():
        protected += 1
        if checker.git("hash-object", path) != expected:
            failures.append("protected file changed: " + path)
    evidence = 0
    for path, expected in checkpoint["evidence_sha256"].items():
        evidence += 1
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != expected:
            failures.append("evidence bytes changed: " + path)
    implemented, implemented_changed = 0, []
    for path, expected in checkpoint["implemented_source_sha256"].items():
        implemented += 1
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != expected:
            implemented_changed.append(path)
    strict_files = 0
    for path in [*program.glob("*.json"), *loop.rglob("*.json")]:
        strict_files += 1
        try:
            checker.strict(path)
        except ValueError as error:
            failures.append(f"not strict JSON: {path.relative_to(root)}: {error}")
    allowlist = checker.strict(program / "MUTATION_GATES.json")["active_allowlist"]
    changed = checker.git("diff", "--name-only", base).splitlines()
    outside = [
        path for path in changed if not any(fnmatch.fnmatchcase(path, pattern) for pattern in allowlist)
    ]
    print(f"base commit: {base}")
    print(f"protected blobs: {protected} checked")
    print(f"evidence digests: {evidence} checked")
    print(f"implemented sources: {implemented} checked, changed: {implemented_changed}")
    print(f"strict JSON files: {strict_files} checked")
    print(f"changed paths against the checkpoint base: {len(changed)}")
    print(f"changed paths outside the programme allowlist: {outside}")
    mine = checker.git("diff", "--name-only", NODE_START).splitlines()
    print(f"changed paths against the start of the node ({NODE_START[:8]}): {len(mine)}")
    for path in mine:
        print(f"  {path}")
    for path in implemented_changed:
        if not allowed_by_the_task(path):
            failures.append("implementation changed outside the task's paths: " + path)
    for path in mine:
        if not allowed_by_the_task(path):
            failures.append("path changed by the node outside the task's paths: " + path)
    for path in outside:
        if not allowed_by_the_task(path):
            failures.append("changed path outside the programme allowlist and the task's paths: " + path)
    if failures:
        print("FAIL")
        for failure in failures:
            print("  " + failure)
        return 1
    print("PASS: every failure of check_checkpoint.py on this branch names a path the task of V18 allows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
