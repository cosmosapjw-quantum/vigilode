#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess

BRANCH = "research/k0-stage-telemetry-integration-20260827"
REVIEW = "e95ce1e58a603306cb665a6ab91cfe02d279972f"
REVIEW_TREE = "e3621a370297a76907e97730ebd18c5c1e0fb83e"
MIN_PACKAGE = "cbdb597fdd58fd1f08a104b4ea8b662dac1f8ba1"
ROOT = "docs/exec-plans/k0-stage-telemetry-integration-20260827"
MANIFEST = f"{ROOT}/WU05_PREMERGE_BOOTSTRAP_MANIFEST.sha256"
BOOTSTRAP_FILES = {
    f"{ROOT}/WU05_PREMERGE_BOOTSTRAP_AUTHORITY.json",
    f"{ROOT}/WU05_PREMERGE_BOOTSTRAP_HANDOFF.md",
    f"{ROOT}/WU05_PREMERGE_CODEX_PROMPT.md",
    "tools/verify-k0-wu05-premerge-bootstrap.py",
    "tools/k0-wu05-premerge-bootstrap.sh",
}
REQUIRED_PACKAGE_FILES = {
    f"{ROOT}/WU05_LOCAL_REPAIR_SUPPLEMENT.json",
    f"{ROOT}/WU05_LOCAL_REPAIR_HANDOFF.md",
    f"{ROOT}/WU05_LOCAL_CODEX_PROMPT.md",
    f"{ROOT}/PUBLIC_BRIDGE_CONTRACT_V2.md",
    f"{ROOT}/evidence/EVIDENCE_V3_CANONICALIZATION.json",
    f"{ROOT}/schemas/stage-receipt-v3.schema.json",
    f"{ROOT}/schemas/cell-receipt-v3.schema.json",
    f"{ROOT}/reviews/FRESH_REPAIR_SUPPLEMENT_REVIEW_PROMPT.md",
    f"{ROOT}/WU05_REPAIR_SUPPLEMENT_MANIFEST.sha256",
    "tools/verify-k0-wu05-supplement.py",
    "tools/verify-k0-wu05-supplement.payload.b64",
}


def stop(message: str, state: str = "STOP_INVALID") -> None:
    print(json.dumps({"status": state, "error": message}, indent=2))
    raise SystemExit(2)


def git(repo: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(["git", *args], cwd=repo, text=True, capture_output=True)
    if check and proc.returncode != 0:
        stop(f"git {' '.join(args)} failed: {proc.stderr.strip()}", "BLOCKED_BY_AUTHORITY_DRIFT")
    return proc


def show(repo: pathlib.Path, package: str, path: str) -> bytes:
    proc = subprocess.run(["git", "show", f"{package}:{path}"], cwd=repo, capture_output=True)
    if proc.returncode != 0:
        stop(f"package {package} lacks {path}: {proc.stderr.decode(errors='replace').strip()}")
    return proc.stdout


def parse_manifest(raw: bytes) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in raw.decode("utf-8").splitlines():
        if not line.strip():
            continue
        try:
            digest, path = line.split("  ", 1)
        except ValueError:
            stop(f"malformed bootstrap manifest line: {line!r}")
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or path in result:
            stop(f"invalid or duplicate bootstrap manifest entry: {line!r}")
        result[path] = digest
    return result


def check_package_bytes(repo: pathlib.Path, package: str) -> dict[str, object]:
    if not re.fullmatch(r"[0-9a-f]{40}", package):
        stop("package SHA is not exact lowercase 40-hex")
    git(repo, "cat-file", "-e", f"{package}^{{commit}}")
    if git(repo, "merge-base", "--is-ancestor", MIN_PACKAGE, package, check=False).returncode != 0:
        stop("package does not descend from the bound WU-05 v3 supplement")
    manifest = parse_manifest(show(repo, package, MANIFEST))
    if set(manifest) != BOOTSTRAP_FILES:
        stop(f"bootstrap manifest file set mismatch: {sorted(manifest)}")
    for path, expected in manifest.items():
        got = hashlib.sha256(show(repo, package, path)).hexdigest()
        if got != expected:
            stop(f"bootstrap byte mismatch for {path}")
    missing = []
    for path in sorted(REQUIRED_PACKAGE_FILES):
        proc = subprocess.run(["git", "cat-file", "-e", f"{package}:{path}"], cwd=repo)
        if proc.returncode != 0:
            missing.append(path)
    if missing:
        stop(f"package lacks required WU-05 files: {missing}")
    authority = json.loads(show(repo, package, f"{ROOT}/WU05_PREMERGE_BOOTSTRAP_AUTHORITY.json"))
    if authority.get("schema") != "vigilode-k0-wu05-premerge-bootstrap-authority/v1" or authority.get("status") != "BOUND":
        stop("bootstrap authority is not bound")
    return {"package": package, "bootstrap_files": len(manifest), "required_files": len(REQUIRED_PACKAGE_FILES)}


def check_local_premerge(repo: pathlib.Path) -> dict[str, str]:
    if git(repo, "branch", "--show-current").stdout.strip() != BRANCH:
        stop("wrong implementation branch", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", "HEAD").stdout.strip() != REVIEW:
        stop("preserved fresh-review HEAD drift", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", "HEAD^{tree}").stdout.strip() != REVIEW_TREE:
        stop("preserved fresh-review tree drift", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "status", "--porcelain=v1").stdout:
        stop("preserved worktree is dirty", "BLOCKED_BY_AUTHORITY_DRIFT")
    return {"branch": BRANCH, "head": REVIEW, "tree": REVIEW_TREE}


def check_local_postmerge(repo: pathlib.Path, package: str) -> dict[str, object]:
    if git(repo, "branch", "--show-current").stdout.strip() != BRANCH:
        stop("wrong implementation branch after merge", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "status", "--porcelain=v1").stdout:
        stop("worktree dirty after package merge", "BLOCKED_BY_AUTHORITY_DRIFT")
    parents = git(repo, "show", "-s", "--format=%P", "HEAD").stdout.strip().split()
    if parents != [REVIEW, package]:
        stop(f"repair merge parents differ: {parents}", "BLOCKED_BY_AUTHORITY_DRIFT")
    for path in sorted(BOOTSTRAP_FILES | REQUIRED_PACKAGE_FILES | {MANIFEST}):
        if git(repo, "diff", "--quiet", package, "HEAD", "--", path, check=False).returncode != 0:
            stop(f"merged package path differs from exact package: {path}", "BLOCKED_BY_AUTHORITY_DRIFT")
    return {"head": git(repo, "rev-parse", "HEAD").stdout.strip(), "parents": parents}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--package-sha", required=True)
    parser.add_argument("--check-premerge", action="store_true")
    parser.add_argument("--check-postmerge", action="store_true")
    args = parser.parse_args()
    repo = pathlib.Path(args.repo_root).resolve()
    package_result = check_package_bytes(repo, args.package_sha)
    results: dict[str, object] = {"package": package_result}
    if args.check_premerge:
        results["local"] = check_local_premerge(repo)
    if args.check_postmerge:
        results["local"] = check_local_postmerge(repo, args.package_sha)
    print(json.dumps({"status": "PASS", "marker": "WU05_PREMERGE_BOOTSTRAP_PASS", "results": results}, indent=2))


if __name__ == "__main__":
    main()
