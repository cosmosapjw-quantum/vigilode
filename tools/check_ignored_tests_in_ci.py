#!/usr/bin/env python3
"""Fail if an #[ignore]d Rust test is not run by any workflow (audit F-003).

A test counts as reachable when some workflow step runs
`cargo test --workspace ... -- --ignored` (which runs every ignored test),
or runs `-- --ignored` with `--test <file stem>` naming the test's file.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


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


def main():
    commands = workflow_commands()
    whole_workspace = any("--workspace" in c for c in commands)
    missing = []
    for path, name in ignored_tests():
        stem = path.stem
        if whole_workspace or any(f"--test {stem}" in c for c in commands):
            continue
        missing.append(f"{path}::{name}")
    if missing:
        print("ignored tests not run by any workflow:\n  " + "\n  ".join(missing))
        return 1
    print(f"{len(ignored_tests())} ignored tests, all reachable from CI")
    return 0


if __name__ == "__main__":
    sys.exit(main())
