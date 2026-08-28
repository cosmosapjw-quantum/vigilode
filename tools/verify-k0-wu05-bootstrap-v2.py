#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fcntl
import os
import sys
import tempfile
import uuid
import hashlib
import json
import pathlib
import re
import subprocess

BRANCH = "research/k0-stage-telemetry-integration-20260827"
REVIEW = "e95ce1e58a603306cb665a6ab91cfe02d279972f"
REVIEW_TREE = "e3621a370297a76907e97730ebd18c5c1e0fb83e"
MIN_PACKAGE = "cbdb597fdd58fd1f08a104b4ea8b662dac1f8ba1"
BASE_PACKAGE = "13aed8dabfbb5da4381d9d73d3cb0c0403ad5354"
PACKAGE_REF = "origin/docs/k0-codex-execution-package-20260827"
ROOT = "docs/exec-plans/k0-stage-telemetry-integration-20260827"
BOOTSTRAP_PATHS = [
    f"{ROOT}/WU05_BOOTSTRAP_V2_AUTHORITY.json",
    f"{ROOT}/WU05_BOOTSTRAP_V2_HANDOFF.md",
    f"{ROOT}/WU05_BOOTSTRAP_V2_CODEX_PROMPT.md",
    "tools/verify-k0-wu05-bootstrap-v2.py",
    "tools/k0-wu05-bootstrap-v2.sh",
]
REQUIRED_PACKAGE_PATHS = [
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
    "tools/verify-k0-stage-telemetry-plan.py",
]


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


def check_package(repo: pathlib.Path, package: str) -> dict[str, object]:
    if not re.fullmatch(r"[0-9a-f]{40}", package):
        stop("package SHA must be exact lowercase 40-hex")
    git(repo, "cat-file", "-e", f"{package}^{{commit}}")
    if git(repo, "merge-base", "--is-ancestor", MIN_PACKAGE, package, check=False).returncode != 0:
        stop("package does not descend from the bound WU-05 v3 supplement")
    payload_hash = hashlib.sha256()
    for path in sorted(BOOTSTRAP_PATHS + REQUIRED_PACKAGE_PATHS):
        raw = show(repo, package, path)
        payload_hash.update(path.encode("utf-8"))
        payload_hash.update(b"\0")
        payload_hash.update(len(raw).to_bytes(8, "big"))
        payload_hash.update(raw)
    authority = json.loads(show(repo, package, f"{ROOT}/WU05_BOOTSTRAP_V2_AUTHORITY.json"))
    if authority.get("schema") != "vigilode-k0-wu05-bootstrap-authority/v2" or authority.get("status") != "BOUND":
        stop("bootstrap v2 authority is not bound")
    return {
        "package": package,
        "bootstrap_paths": len(BOOTSTRAP_PATHS),
        "required_package_paths": len(REQUIRED_PACKAGE_PATHS),
        "aggregate_payload_sha256": payload_hash.hexdigest(),
    }


def check_premerge(repo: pathlib.Path) -> dict[str, str]:
    if git(repo, "branch", "--show-current").stdout.strip() != BRANCH:
        stop("wrong implementation branch", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", "HEAD").stdout.strip() != REVIEW:
        stop("fresh-review HEAD drift", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", "HEAD^{tree}").stdout.strip() != REVIEW_TREE:
        stop("fresh-review tree drift", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "status", "--porcelain=v1").stdout:
        stop("preserved worktree is dirty", "BLOCKED_BY_AUTHORITY_DRIFT")
    return {"branch": BRANCH, "head": REVIEW, "tree": REVIEW_TREE}


def check_postmerge(repo: pathlib.Path, package: str) -> dict[str, object]:
    if git(repo, "branch", "--show-current").stdout.strip() != BRANCH:
        stop("wrong implementation branch after merge", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "status", "--porcelain=v1").stdout:
        stop("worktree dirty after merge", "BLOCKED_BY_AUTHORITY_DRIFT")
    parents = git(repo, "show", "-s", "--format=%P", "HEAD").stdout.strip().split()
    if parents != [REVIEW, package]:
        stop(f"merge parents differ from bound order: {parents}", "BLOCKED_BY_AUTHORITY_DRIFT")
    for path in BOOTSTRAP_PATHS + REQUIRED_PACKAGE_PATHS:
        if git(repo, "diff", "--quiet", package, "HEAD", "--", path, check=False).returncode != 0:
            stop(f"merged package path differs from exact package: {path}", "BLOCKED_BY_AUTHORITY_DRIFT")
    return {"head": git(repo, "rev-parse", "HEAD").stdout.strip(), "parents": parents}


def check_only_main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--package-sha", required=True)
    parser.add_argument("--check-premerge", action="store_true")
    parser.add_argument("--check-postmerge", action="store_true")
    args = parser.parse_args()
    repo = pathlib.Path(args.repo_root).resolve()
    results: dict[str, object] = {"package": check_package(repo, args.package_sha)}
    marker = "WU05_BOOTSTRAP_V2_PACKAGE_PASS"
    if args.check_premerge:
        results["local"] = check_premerge(repo)
        marker = "WU05_BOOTSTRAP_V2_PREMERGE_PASS"
    if args.check_postmerge:
        results["local"] = check_postmerge(repo, args.package_sha)
        marker = "WU05_BOOTSTRAP_V2_POSTMERGE_PASS"
    print(json.dumps({"status": "PASS", "marker": marker, "results": results}, indent=2))


# Bootstrap preparation is a bounded HOST_CODEX_ORCHESTRATOR capability.
# Dependency validators remain authoritative and are never replaced by stubs here.
BASE_MARKERS = {"PACKAGE_CONTRACT_PASS"}
SUPPLEMENT_MARKERS = {
    "WU05_SUPPLEMENT_MANIFEST_PASS", "LEGACY_REPAIR_BLOBS_PASS",
    "EXTERNAL_PACKAGE_PIN_PASS", "WU05_SUPPLEMENT_AUTHORITY_PASS",
    "HOSTILE_FIXTURES_PASS",
}
ALLOWED_CONTROL_FILES = {
    "AGENTS.md", "PACKAGE_MANIFEST.sha256",
    "docs/invariants/K0_STAGE_TELEMETRY.md", "docs/quality/P0_P1_POLICY.md",
}


def control_path(path: str) -> bool:
    return (path in ALLOWED_CONTROL_FILES or path.startswith(ROOT + "/")
            or (path.startswith("tools/") and "/" not in path[6:] and
                path[6:].startswith(("verify-k0-", "k0-wu05-", "test_k0_bootstrap"))))


def check_control_delta(repo: pathlib.Path, start: str, end: str) -> list[str]:
    raw = git(repo, "diff", "--name-only", "-z", start, end).stdout
    paths = [p for p in raw.split("\0") if p]
    forbidden = [p for p in paths if not control_path(p)]
    if forbidden:
        stop(f"package preparation changes non-control paths: {forbidden}",
             "BLOCKED_BY_AUTHORITY_DRIFT")
    return paths


def markers_in(text: str) -> set[str]:
    """Accept structured PASS markers, not substring matches or mere exit zero."""
    found: set[str] = set()
    decoder = json.JSONDecoder()

    def visit(value: object) -> None:
        if isinstance(value, dict):
            if value.get("status") == "PASS" and isinstance(value.get("marker"), str):
                found.add(value["marker"])
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    offset = 0
    while offset < len(text):
        if text[offset].isspace():
            offset += 1
            continue
        try:
            value, end = decoder.raw_decode(text, offset)
        except json.JSONDecodeError:
            offset += 1
        else:
            visit(value)
            offset = end
    return found


def bootstrap(repo: pathlib.Path, package: str) -> None:
    """Prepare exactly REVIEW + package, or revalidate that exact existing merge.

    Logs live under the Git common directory, never in the tracked working tree.
    No reset, stash, force push, source repair, campaign, or PR merge is performed.
    """
    if not re.fullmatch(r"[0-9a-f]{40}", package):
        stop("package SHA must be exact lowercase 40-hex")
    if git(repo, "branch", "--show-current").stdout.strip() != BRANCH:
        stop("wrong implementation branch", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "status", "--porcelain=v1").stdout:
        stop("preserved worktree is dirty", "BLOCKED_BY_AUTHORITY_DRIFT")
    start = git(repo, "rev-parse", "HEAD").stdout.strip()
    parents = git(repo, "show", "-s", "--format=%P", start).stdout.strip().split()
    already_merged = parents == [REVIEW, package]
    if start != REVIEW and not already_merged:
        stop("expected preserved review or its exact package merge; no history was changed",
             "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", f"{REVIEW}^{{tree}}").stdout.strip() != REVIEW_TREE:
        stop("preserved review tree differs", "BLOCKED_BY_AUTHORITY_DRIFT")
    if git(repo, "rev-parse", "--verify", "MERGE_HEAD", check=False).returncode == 0:
        stop("an unrelated merge is already in progress", "BLOCKED_BY_AUTHORITY_DRIFT")

    common = pathlib.Path(git(repo, "rev-parse", "--git-common-dir").stdout.strip())
    if not common.is_absolute():
        common = (repo / common).resolve()
    log_root = common / "k0-bootstrap"
    log_root.mkdir(exist_ok=True)
    with (log_root / "prepare.lock").open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            stop("another bootstrap owns this repository", "STOP_INVALID")
        journal = log_root / uuid.uuid4().hex
        journal.mkdir()
        receipt: dict[str, object] = {
            "schema": "vigilode-k0-bootstrap-execution/v2",
            "package_sha": package, "preserved_review": REVIEW,
            "preserved_tree": REVIEW_TREE, "start_head": start,
            "already_merged": already_merged, "status": "ACTIVE", "commands": [],
            "scope": "PACKAGE_PREPARATION_ONLY", "solver_tests_executed": False,
        }
        temp_parent: pathlib.Path | None = None
        package_wt: pathlib.Path | None = None
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GIT_TERMINAL_PROMPT="0",
                   GIT_EDITOR="true", GIT_MERGE_AUTOEDIT="no")

        def save() -> None:
            (journal / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")

        def run(label: str, argv: list[str], cwd: pathlib.Path,
                required: set[str] | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
            receipt["phase"] = label
            save()
            index = len(receipt["commands"])
            try:
                proc = subprocess.run(argv, cwd=cwd, env=env, text=True, capture_output=True,
                                      timeout=180, check=False)
            except subprocess.TimeoutExpired as exc:
                def decoded(value):
                    return value.decode(errors="replace") if isinstance(value, bytes) else (value or "")
                proc = subprocess.CompletedProcess(argv, 124, decoded(exc.stdout), decoded(exc.stderr))
            stdout_name, stderr_name = f"{index:02}-{label}.stdout", f"{index:02}-{label}.stderr"
            (journal / stdout_name).write_text(proc.stdout)
            (journal / stderr_name).write_text(proc.stderr)
            seen = markers_in(proc.stdout)
            receipt["commands"].append({
                "label": label, "argv": argv, "cwd": str(cwd), "exit_code": proc.returncode,
                "stdout": stdout_name, "stderr": stderr_name,
                "stdout_sha256": hashlib.sha256(proc.stdout.encode()).hexdigest(),
                "stderr_sha256": hashlib.sha256(proc.stderr.encode()).hexdigest(),
                "required_markers": sorted(required or []), "observed_markers": sorted(seen),
            })
            save()
            sys.stdout.write(proc.stdout)
            sys.stderr.write(proc.stderr)
            if check and (proc.returncode != 0 or (required and not required <= seen)):
                stop(f"{label} failed (exit={proc.returncode}); missing markers={sorted((required or set())-seen)}; logs={journal}")
            return proc

        def validate(at: pathlib.Path, postmerge: bool) -> None:
            run("post-package" if postmerge else "pre-package",
                [sys.executable, "-B", str(at / "tools/verify-k0-stage-telemetry-plan.py"),
                 "--repo-root", str(at), "--check-package"], at, BASE_MARKERS)
            args = [sys.executable, "-B", str(at / "tools/verify-k0-wu05-supplement.py"),
                    "--repo-root", str(at), "--expected-package-sha", package,
                    "--check-supplement-manifest", "--check-authority", "--self-test"]
            required = set(SUPPLEMENT_MARKERS)
            if postmerge:
                args.append("--check-repair-merge")
                required.add("WU05_REPAIR_MERGE_PASS")
            run("post-supplement" if postmerge else "pre-supplement", args, at, required)

        try:
            run("fetch", ["git", "fetch", "--prune", "origin",
                          "docs/k0-codex-execution-package-20260827"], repo)
            if git(repo, "rev-parse", PACKAGE_REF).stdout.strip() != package:
                stop("package ref differs from external pin; classify as package metadata drift, not scientific failure",
                     "BLOCKED_BY_AUTHORITY_DRIFT")
            receipt["package"] = check_package(repo, package)
            if git(repo, "merge-base", "--is-ancestor", BASE_PACKAGE, package, check=False).returncode:
                stop("package does not descend from inspected bootstrap baseline", "BLOCKED_BY_AUTHORITY_DRIFT")
            receipt["new_package_paths"] = check_control_delta(repo, BASE_PACKAGE, package)
            if already_merged:
                check_postmerge(repo, package)
                check_control_delta(repo, REVIEW, start)
            else:
                check_premerge(repo)
            print(json.dumps({"status": "PASS", "marker": "WU05_BOOTSTRAP_V2_PREMERGE_PASS",
                              "mode": "REVALIDATE" if already_merged else "PREPARE"}))
            temp_parent = pathlib.Path(tempfile.mkdtemp(prefix="k0-package-validation-"))
            package_wt = temp_parent / "worktree"
            run("worktree-add", ["git", "worktree", "add", "--detach", str(package_wt), package], repo)
            validate(package_wt, False)
            run("worktree-remove", ["git", "worktree", "remove", str(package_wt)], repo)
            package_wt = None
            if git(repo, "rev-parse", "HEAD").stdout.strip() != start or git(repo, "status", "--porcelain=v1").stdout:
                stop("implementation moved while package was being validated", "BLOCKED_BY_AUTHORITY_DRIFT")
            if not already_merged:
                merged = run("merge", ["git", "merge", "--no-ff", "--no-edit", package], repo, check=False)
                if merged.returncode:
                    if git(repo, "rev-parse", "--verify", "MERGE_HEAD", check=False).returncode == 0:
                        run("merge-abort", ["git", "merge", "--abort"], repo)
                    if git(repo, "rev-parse", "HEAD").stdout.strip() != REVIEW or git(repo, "status", "--porcelain=v1").stdout:
                        stop("merge failed; state needs inspection and was preserved", "BLOCKED_BY_AUTHORITY_DRIFT")
                    stop("exact package merge failed and was aborted; preserved review unchanged",
                         "BLOCKED_BY_AUTHORITY_DRIFT")
            post = check_postmerge(repo, package)
            receipt["overlay_paths"] = check_control_delta(repo, REVIEW, post["head"])
            print(json.dumps({"status": "PASS", "marker": "WU05_BOOTSTRAP_V2_POSTMERGE_PASS", **post}))
            validate(repo, True)
            if git(repo, "status", "--porcelain=v1").stdout:
                stop("validation left working-tree changes; preserved for inspection")
            receipt["status"] = "LOCAL_WU05_AUTHORITY_READY"
            receipt["prepared_head"] = post["head"]
            receipt["prepared_tree"] = git(repo, "rev-parse", "HEAD^{tree}").stdout.strip()
            print("LOCAL_WU05_AUTHORITY_READY")
        except BaseException:
            receipt["status"] = "STOP_INVALID"
            raise
        finally:
            receipt["end_head"] = git(repo, "rev-parse", "HEAD", check=False).stdout.strip()
            receipt["git_status_porcelain"] = git(repo, "status", "--porcelain=v1", check=False).stdout
            if package_wt is not None and package_wt.exists():
                removal = git(repo, "worktree", "remove", str(package_wt), check=False)
                if removal.returncode:
                    receipt["preserved_package_worktree"] = str(package_wt)
            if temp_parent is not None and temp_parent.exists() and not list(temp_parent.iterdir()):
                temp_parent.rmdir()
            save()
            print(f"BOOTSTRAP_RECEIPT={journal / 'receipt.json'}")


def main() -> None:
    if "--apply" not in sys.argv:
        check_only_main()
        return
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--package-sha", required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    bootstrap(pathlib.Path(args.repo_root).resolve(), args.package_sha)


if __name__ == "__main__":
    main()
