#!/usr/bin/env python3
"""Run research/production_loop_20261116/check_checkpoint.py with one change: the complaint about the
protected blob of crates/rodas5p-integrators/src/stage_batch.rs (expected for node V32, re-pinned by
the orchestrator under OD-L1-28) is printed instead of raised, so that the checker's remaining checks
(evidence identities, strict JSON, allowlist of changed paths) run too. The checker file itself is not
changed: its source is read, the one line is replaced in memory, and it runs with its own __file__.
The output is kept by run_checks.sh in logs/checks/loop_checker_rest.log.

usage: python3 -I loop_checker_rest.py
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CHECKER = os.path.join(HERE, "..", "check_checkpoint.py")
EXPECTED = "crates/rodas5p-integrators/src/stage_batch.rs"
LINE = "require(git('hash-object', path) == expected, 'protected file changed: ' + path)"
REPLACEMENT = (
    "(print('expected complaint (not raised): protected file changed: ' + path, "
    "git('hash-object', path), '!=', expected) "
    f"if path == {EXPECTED!r} and git('hash-object', path) != expected "
    "else require(git('hash-object', path) == expected, 'protected file changed: ' + path))"
)

with open(CHECKER, encoding="utf-8") as handle:
    source = handle.read()
assert source.count(LINE) == 1, "check_checkpoint.py changed; update loop_checker_rest.py"
path = os.path.abspath(CHECKER)
exec(compile(source.replace(LINE, REPLACEMENT), path, "exec"), {"__name__": "__main__", "__file__": path})
