#!/usr/bin/env python3
from __future__ import annotations

import json
import pathlib
import subprocess
import sys

BASE_SCRIPT = pathlib.Path(__file__).with_name("verify-k0-stage-telemetry-plan-base.py")
EXPECTED_BRANCH = "research/k0-stage-telemetry-integration-20260827"
EXPECTED_SOURCE_BASE = "e1124586a4029f86669e7489278c61ef676d61aa"
EXPECTED_WU02 = "2badcec35b51d23fcd2938d1e15c9e0875a0f9df"
EXPECTED_WU02_TREE = "5df56a846908972ed0159d8fd59aa47934550a3b"
PACKAGE_REMOTE_REF = "origin/docs/k0-codex-execution-package-20260827"
PACKAGE_PATHS = (
    "AGENTS.md",
    "PACKAGE_MANIFEST.sha256",
    "docs/exec-plans/k0-stage-telemetry-integration-20260827/",
    "docs/invariants/K0_STAGE_TELEMETRY.md",
    "docs/quality/P0_P1_POLICY.md",
    "tools/verify-k0-stage-telemetry-plan.py",
    "tools/verify-k0-stage-telemetry-plan-base.py",
)


def fail(message: str) -> None:
    print(json.dumps({"status": "FAIL", "error": message}, indent=2))
    raise SystemExit(1)


def git(root: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", *args], cwd=root, text=True, capture_output=True, check=False
    )
    if check and result.returncode != 0:
        fail(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result


def is_package_path(path: str) -> bool:
    return any(path == prefix or path.startswith(prefix) for prefix in PACKAGE_PATHS)


def spec_repair_check(root: pathlib.Path) -> dict[str, object]:
    if git(root, "rev-parse", "--is-inside-work-tree").stdout.strip() != "true":
        fail("spec-repair authority check requires a Git worktree")
    branch = git(root, "branch", "--show-current").stdout.strip()
    if branch != EXPECTED_BRANCH:
        fail(f"wrong implementation branch {branch}")
    if git(root, "status", "--porcelain=v1").stdout:
        fail("dirty worktree after spec-repair overlay")

    head = git(root, "rev-parse", "HEAD").stdout.strip()
    package_tip = git(root, "rev-parse", PACKAGE_REMOTE_REF).stdout.strip()
    if git(root, "merge-base", "--is-ancestor", EXPECTED_SOURCE_BASE, head, check=False).returncode:
        fail("source base is not an ancestor after spec repair")
    if git(root, "merge-base", "--is-ancestor", package_tip, head, check=False).returncode:
        fail("updated package tip is not an ancestor after spec repair")

    merges: list[list[str]] = []
    for line in git(root, "rev-list", "--first-parent", "--parents", head).stdout.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[2] == package_tip:
            merges.append(fields)
    if len(merges) != 1:
        fail(f"expected one active spec-repair merge with package second parent, got {merges}")
    merge = merges[0]
    if merge[1] != EXPECTED_WU02:
        fail(f"spec-repair first parent {merge[1]} is not preserved WU-02")
    wu02_tree = git(root, "rev-parse", f"{EXPECTED_WU02}^{{tree}}").stdout.strip()
    if wu02_tree != EXPECTED_WU02_TREE:
        fail("preserved WU-02 tree identity drift")

    changed = git(root, "diff", "--name-only", package_tip, head, "--", *PACKAGE_PATHS).stdout.splitlines()
    if changed:
        fail(f"active package paths differ from updated package tip: {changed}")

    required_tokens = {
        "docs/invariants/K0_STAGE_TELEMETRY.md": ["INV-K0-013", "f64::MIN_POSITIVE"],
        "docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/scaled-nonlinear-remainder-oracle.json": [
            "vigilode-k0-scaled-nonlinear-remainder-oracle/v1",
            "upper_bound_one",
        ],
        "docs/exec-plans/k0-stage-telemetry-integration-20260827/work-units/WU-03-stage-receipts-and-aggregate.json": [
            "FM-WU03-005",
            "scaled_nonlinear_remainder_exact_oracle_and_mutations",
        ],
    }
    for relative, tokens in required_tokens.items():
        text = (root / relative).read_text(encoding="utf-8")
        missing = [token for token in tokens if token not in text]
        if missing:
            fail(f"spec-repair authority missing {missing} in {relative}")

    return {
        "status": "PASS",
        "marker": "SPEC_REPAIR_AUTHORITY_PASS",
        "head": head,
        "package_tip": package_tip,
        "repair_merge": merge[0],
        "preserved_wu02": EXPECTED_WU02,
        "preserved_wu02_tree": EXPECTED_WU02_TREE,
    }


def main() -> None:
    args = sys.argv[1:]
    run_spec = "--check-spec-repair-authority" in args
    forwarded = [arg for arg in args if arg != "--check-spec-repair-authority"]
    result = subprocess.run([sys.executable, str(BASE_SCRIPT), *forwarded], check=False)
    if result.returncode != 0:
        raise SystemExit(result.returncode)
    if run_spec:
        root = pathlib.Path(".")
        if "--repo-root" in forwarded:
            root = pathlib.Path(forwarded[forwarded.index("--repo-root") + 1])
        print(json.dumps(spec_repair_check(root.resolve()), indent=2))


if __name__ == "__main__":
    main()
