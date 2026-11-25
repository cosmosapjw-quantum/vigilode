#!/usr/bin/env python3
"""Fail if an #[ignore]d Rust test is not run by any workflow (audit F-003).

A test counts as reachable when some workflow step runs
`cargo test --workspace ... -- --ignored` (which runs every ignored test of
the workspace members; a test of a crate outside the members, such as
crates/vigilode-bench, is not reached by it), or runs `-- --ignored` with
`--test <file stem>` naming the test's file, and that step does not leave the
test out: `--skip <text>` leaves out every test whose name contains <text>,
and `--exact <name>` runs only that test.
A step `tools/ci/select_tests.py run --mode ignored` counts as a
whole-workspace run (with its `--skip` filters) when the same workflow
selects with `--scope` naming every workspace package: in full mode it runs
the ignored tests of every package (owner decision OD-L1-19: otherwise it
runs the affected ones). A step `tools/ci/select_tests.py control --target
<package>:<target> --test <name>` runs `--test <target> -- --ignored --exact
<name>` (a negative control, OD-L1-21).

A test that no step runs must be listed in DOCUMENTED_EXCLUSIONS. The script
also fails when a listed test does not exist or is run by a workflow.
"""

import re
import sys
import tomllib
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


def workspace_members():
    """The member directories of the root workspace, relative to the repository root."""
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    return [Path(member) for member in manifest["workspace"]["members"]]


def workspace_packages():
    names = set()
    for member in workspace_members():
        package = tomllib.loads((ROOT / member / "Cargo.toml").read_text())["package"]["name"]
        names.add(package)
    return names


def in_workspace_member(path):
    """True when the repository-relative path lies under a workspace member directory."""
    return any(path.is_relative_to(member) for member in workspace_members())


def selector_scope_covers_workspace(lines):
    """True if a select command of the change-based selector names every workspace package."""
    packages = workspace_packages()
    for line in lines:
        if "select_tests.py select" in line and "--scope" in line:
            scope = line.split("--scope", 1)[1].split(" --", 1)[0].split()
            if packages <= set(scope):
                return True
    return False


def workflow_commands():
    commands = []
    for path in sorted((ROOT / ".github/workflows").glob("*.yml")):
        text = path.read_text().replace("\\\n", " ")
        lines = text.splitlines()
        commands += [line for line in lines if "cargo test" in line and "--ignored" in line]
        for line in lines:
            if "select_tests.py control" in line:
                target = re.search(r"--target[ =]\S+?:(\S+)", line)
                test = re.search(r"--test[ =](\S+)", line)
                if target and test:
                    commands.append(f"cargo test --test {target.group(1)} -- --ignored --exact {test.group(1)}")
        if selector_scope_covers_workspace(lines):
            for line in lines:
                if "select_tests.py run" in line and "--mode ignored" in line:
                    skips = " ".join(f"--skip {s}" for s in re.findall(r"--skip[ =](\S+)", line))
                    commands.append(f"cargo test --workspace -- --ignored {skips}")
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
        member = in_workspace_member(path)
        covering = [c for c in commands if ("--workspace" in c and member) or f"--test {path.stem}" in c]
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
