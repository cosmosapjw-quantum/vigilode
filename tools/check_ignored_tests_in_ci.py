#!/usr/bin/env python3
"""Fail if an #[ignore]d Rust test is not run by any workflow (audit F-003).

A test counts as reachable when some workflow step runs
`cargo test --workspace ... -- --ignored` (which runs every ignored test),
or runs `-- --ignored` with `--test <file stem>` naming the test's file,
and that step does not leave the test out: `--skip <text>` leaves out every
test whose name contains <text>, and `--exact <name>` runs only that test.

A test that no step runs must be listed in DOCUMENTED_EXCLUSIONS. The script
also fails when a listed test does not exist or is run by a workflow.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Ignored tests that no workflow runs, each with the reason. Empty: the
# recorded kill gate, which the whole-workspace step skips, runs in a step of
# its own as a negative control (ignored-tests.yml).
DOCUMENTED_EXCLUSIONS = {}


def ignored_tests():
    found = []
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if line.strip().startswith("#[ignore"):
                name = next(
                    (m.group(1) for l in lines[index + 1 : index + 4] if (m := re.search(r"fn (\w+)", l))),
                    "?",
                )
                found.append((path.relative_to(ROOT), name))
    return found


def workflow_commands():
    commands = []
    for path in sorted((ROOT / ".github/workflows").glob("*.yml")):
        text = path.read_text().replace("\\\n", " ")
        commands += [line for line in text.splitlines() if "cargo test" in line and "--ignored" in line]
    return commands


def skips(command, name):
    """True when the test step `command` does not run the test `name`."""
    if any(text in name for text in re.findall(r"--skip[ =](\S+)", command)):
        return True
    exact = re.findall(r"--exact[ =](\S+)", command)
    return bool(exact) and name not in exact


def main():
    commands = workflow_commands()
    tests = ignored_tests()
    missing, excluded, problems = [], [], []
    for path, name in tests:
        covering = [c for c in commands if "--workspace" in c or f"--test {path.stem}" in c]
        reached = any(not skips(c, name) for c in covering)
        if name in DOCUMENTED_EXCLUSIONS:
            if reached:
                problems.append(f"{path}::{name} is a documented exclusion but a workflow runs it")
            else:
                excluded.append(f"{path}::{name} ({DOCUMENTED_EXCLUSIONS[name]})")
        elif not reached:
            missing.append(f"{path}::{name}")
    names = {name for _, name in tests}
    problems += [
        f"documented exclusion names no ignored test: {name}"
        for name in sorted(set(DOCUMENTED_EXCLUSIONS) - names)
    ]
    if missing:
        print("ignored tests not run by any workflow:\n  " + "\n  ".join(missing))
    if problems:
        print("exclusion list out of date:\n  " + "\n  ".join(problems))
    if missing or problems:
        return 1
    print(
        f"{len(tests)} ignored tests: {len(tests) - len(excluded)} reachable from CI, "
        f"{len(excluded)} documented exclusion(s)"
    )
    for line in excluded:
        print("  excluded: " + line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
