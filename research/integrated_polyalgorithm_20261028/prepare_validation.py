#!/usr/bin/env python3
"""Prepare a fresh cumulative-validation worktree; never run solver validation.

The default is a read-only JSON plan. All manifests and covered file bytes come
from --source-sha, so this script may be extracted into a temporary directory
and invoked from an older checkout. Missing local objects fail without fetching.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass


PACKAGE = "research/integrated_polyalgorithm_20261028"
MANIFESTS = {
    "scope": f"{PACKAGE}/PR_SCOPE.json",
    "coverage": f"{PACKAGE}/validation_design/COVERAGE.json",
    "commands": f"{PACKAGE}/validation_design/COMMANDS.json",
}
SAFE_ENV = {
    "GIT_NO_LAZY_FETCH": "1",
    "GIT_ALLOW_PROTOCOL": "",  # Also deny transports if Git lacks the first knob.
    "GIT_TERMINAL_PROMPT": "0",
    "GIT_OPTIONAL_LOCKS": "0",
    "GIT_LFS_SKIP_SMUDGE": "1",
}


@dataclass(frozen=True)
class RequiredSources:
    prs: dict[int, str]
    integrated: str
    baseline: str
    observed_main: str


REQUIRED = RequiredSources(
    prs={
        90: "68a28357d960c4c2743c299770f0a33ef63245b2",
        91: "206298b4068d923bb47c7160b111b7439098660c",
        92: "7eb2dd0ffda39afb2871d76700a720986ff09fe0",
        93: "384dba091a3d9e1aac0ecddec264958298239253",
    },
    integrated="af7f3e9f55d652554042d2386bf07abb7d857d75",
    baseline="3064d453091af1dec7f99f0c8afabd98b447ea34",
    observed_main="8d0c79184e09efb5bdadc24a6315c60a71a44264",
)


class PreparationError(Exception):
    """A refusal that must not be promoted to validation evidence."""


def full_sha(value: str) -> str:
    if not re.fullmatch(r"[0-9a-fA-F]{40}", value):
        raise argparse.ArgumentTypeError("supply a full 40-hex commit SHA, not a ref")
    return value.lower()


class Git:
    def __init__(self, repository: Path):
        self.repository = repository

    def argv(self, *args: str) -> list[str]:
        return [
            "git", "--literal-pathspecs", "-c", f"core.hooksPath={os.devnull}",
            "-c", "core.fsmonitor=false", "-c", "submodule.recurse=false",
            "-C", str(self.repository), *args,
        ]

    def result(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            self.argv(*args), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env={**os.environ, **SAFE_ENV}, check=False,
        )

    def run(self, *args: str, allowed: tuple[int, ...] = (0,)) -> bytes:
        result = self.result(*args)
        if result.returncode not in allowed:
            detail = result.stderr.decode("utf-8", "replace").strip()
            raise PreparationError(f"Git {' '.join(args)} failed: {detail}")
        return result.stdout

    def text(self, *args: str) -> str:
        return self.run(*args).decode("utf-8", "surrogateescape").strip()

    def require_commit(self, sha: str) -> None:
        if self.text("cat-file", "-t", sha) != "commit":
            raise PreparationError(f"Expected a commit object, not a tag/tree/blob: {sha}")

    def require_ancestor(self, ancestor: str, source: str) -> None:
        self.require_commit(ancestor)
        self.run("merge-base", "--is-ancestor", ancestor, source)

    def is_ancestor(self, ancestor: str, descendant: str) -> bool:
        result = self.result("merge-base", "--is-ancestor", ancestor, descendant)
        if result.returncode not in (0, 1):
            raise PreparationError("Cannot determine local ancestry: " + result.stderr.decode("utf-8", "replace"))
        return result.returncode == 0

    def blob(self, source: str, path: str) -> bytes:
        return self.run("show", "--no-ext-diff", "--no-textconv", f"{source}:{path}")


def committed_manifests(git: Git, source: str, required: RequiredSources) -> tuple[dict, list]:
    data, identities = {}, []
    for key, path in MANIFESTS.items():
        raw = git.blob(source, path)
        try:
            data[key] = json.loads(raw)
        except (ValueError, UnicodeError) as exc:
            raise PreparationError(f"Invalid committed manifest {path}: {exc}") from exc
        if data[key].get("schema_version") != 1:
            raise PreparationError(f"Unsupported manifest schema: {path}")
        identities.append({"path": path, "source_commit": source,
                           "sha256": hashlib.sha256(raw).hexdigest()})
    scope, coverage, commands = data["scope"], data["coverage"], data["commands"]
    prs = scope["conversation_prs"]
    if len(prs) != len(required.prs) or {p["number"]: p["head_commit"] for p in prs} != required.prs:
        raise PreparationError("PR_SCOPE does not match the pinned PR90--93 heads")
    expected = [
        (scope["source_head"], required.prs[93]),
        (scope["pre_conversation_commit"], required.baseline),
        (scope["observed_main_commit"], required.observed_main),
        (coverage["source_commit"], required.prs[93]),
        (coverage["conversation_baseline"], required.baseline),
        (coverage["observed_main"], required.observed_main),
    ]
    if any(actual != pin for actual, pin in expected):
        raise PreparationError("Manifest source/baseline/main pins disagree with this helper")
    paths = coverage["paths"]
    if len(paths) != coverage["cumulative_changed_path_count"]:
        raise PreparationError("COVERAGE path count is inconsistent")
    names = [p["path"] for p in paths]
    if len(names) != len(set(names)):
        raise PreparationError("COVERAGE contains duplicate paths")
    for entry in paths:
        path = PurePosixPath(entry["path"])
        if path.is_absolute() or ".." in path.parts or str(path) != entry["path"]:
            raise PreparationError(f"Invalid repository-relative coverage path: {path}")
        if not re.fullmatch(r"[0-9a-f]{64}", entry["source_sha256"]):
            raise PreparationError(f"Invalid coverage SHA256: {path}")
        if not entry["gate_ids"] or any(g not in coverage["gate_groups"] for g in entry["gate_ids"]):
            raise PreparationError(f"Missing coverage gate definition: {path}")
    command_ids = [c["id"] for c in commands["commands"]]
    if len(command_ids) != len(set(command_ids)):
        raise PreparationError("COMMANDS contains duplicate IDs")
    return data, identities


def fresh_target(git: Git, destination: Path, branch: str) -> Path:
    if git.text("check-ref-format", "--branch", branch) != branch or branch == "main":
        raise PreparationError("Use a literal new validation branch, distinct from main")
    refs = git.text("for-each-ref", "--format=%(refname)", "refs/heads", "refs/remotes").splitlines()
    proposed = f"refs/heads/{branch}"
    for ref in refs:
        if ref.startswith("refs/heads/") and (
            ref == proposed or ref.startswith(proposed + "/") or proposed.startswith(ref + "/")
        ):
            raise PreparationError(f"Branch is not fresh or conflicts with an existing ref: {ref}")
        if ref.startswith("refs/remotes/") and ref.split("/", 3)[-1] == branch:
            raise PreparationError(f"A cached remote branch already has this name: {ref}")
    candidate = destination.expanduser().absolute()
    if os.path.lexists(candidate):
        raise PreparationError(f"Destination must not exist, even as an empty directory: {candidate}")
    if not candidate.parent.is_dir():
        raise PreparationError(f"Destination parent must already be a directory: {candidate.parent}")
    candidate = candidate.parent.resolve() / candidate.name
    for field in git.run("worktree", "list", "--porcelain", "-z").split(b"\0"):
        if field.startswith(b"worktree "):
            registered = Path(os.fsdecode(field[len(b"worktree "):])).resolve()
            if candidate == registered or registered in candidate.parents:
                raise PreparationError(f"Destination overlaps a registered worktree: {registered}")
    return candidate


def coverage_drift(git: Git, source: str, coverage: dict) -> dict:
    entries = {p["path"]: p for p in coverage["paths"]}
    tree = {}
    for record in git.run("ls-tree", "-r", "-z", source, "--", *entries).split(b"\0"):
        if record:
            metadata, path = record.split(b"\t", 1)
            mode, kind, oid = metadata.decode("ascii").split()
            tree[os.fsdecode(path)] = (mode, kind, oid)
    files = []
    for path, entry in entries.items():
        actual, mode, kind = None, None, "missing"
        if path in tree:
            mode, kind, oid = tree[path]
            if kind == "blob":
                actual = hashlib.sha256(git.run("cat-file", "blob", oid)).hexdigest()
        status = "unchanged" if actual == entry["source_sha256"] else "changed" if actual else "missing_or_nonblob"
        files.append({"path": path, "gate_ids": entry["gate_ids"], "status": status,
                      "expected_sha256": entry["source_sha256"], "selected_sha256": actual,
                      "selected_mode": mode, "selected_object_kind": kind})
    raw = git.run("diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--name-status", "-z",
                  coverage["source_commit"], source, "--", *coverage["changed_scope"])
    fields, changes = raw.split(b"\0"), []
    for index in range(0, len(fields) - 1, 2):
        status, path = os.fsdecode(fields[index]), os.fsdecode(fields[index + 1])
        changes.append({"git_status": status, "path": path,
                        "gate_ids": entries.get(path, {}).get("gate_ids", []),
                        "coverage_mapping_required": path not in entries})
    return {"baseline_commit": coverage["source_commit"], "files": files,
            "changed_or_missing_covered_files": sum(f["status"] != "unchanged" for f in files),
            "scope_changes_since_coverage_baseline": changes,
            "qualification_status": "NOT_RUN",
            "meaning": "Hash differences are recorded, not rejected or counted as test results."}


def build_plan(repository: Path, source: str, destination: Path | None, branch: str | None,
               main_ref: str | None = None, *, required: RequiredSources = REQUIRED) -> dict:
    source = full_sha(source)
    git = Git(repository)
    git.require_commit(source)
    requirements = {"observed_main": required.observed_main, "conversation_baseline": required.baseline,
                    **{f"PR{n}": sha for n, sha in required.prs.items()}, "integrated_theory": required.integrated}
    ancestry = []
    for label, ancestor in requirements.items():
        git.require_ancestor(ancestor, source)
        ancestry.append({"label": label, "ancestor": ancestor, "descendant": source, "verified": True})
    manifests, identities = committed_manifests(git, source, required)
    refs = git.text("for-each-ref", "--format=%(refname)", "refs/remotes/origin/main").splitlines()
    checked_main_ref = main_ref or ("refs/remotes/origin/main" if "refs/remotes/origin/main" in refs else None)
    checked_main = None
    main_relation, reconciliation_required, merge_bases = "NOT_CHECKED", None, []
    if checked_main_ref:
        checked_main = git.text("rev-parse", "--verify", "--end-of-options", f"{checked_main_ref}^{{commit}}")
        if git.is_ancestor(checked_main, source):
            main_relation, reconciliation_required = "main_is_ancestor", False
        elif git.is_ancestor(source, checked_main):
            main_relation, reconciliation_required = "source_is_ancestor", True
        else:
            main_relation, reconciliation_required = "diverged", True
        merge_bases = git.run("merge-base", "--all", checked_main, source, allowed=(0, 1)).decode("ascii").splitlines()
    branch = branch or f"validation/integrated-{source[:12]}"
    destination = destination or repository.parent / f"{repository.name}-validation-{source[:12]}"
    destination = fresh_target(git, destination, branch)
    argv = git.argv("worktree", "add", "-b", branch, str(destination), source)
    return {
        "schema_version": 1, "kind": "independent_validation_worktree_preparation",
        "status": "DRY_RUN", "source_commit": source, "source_inclusion": ancestry,
        "native_validation_status": "NOT_RUN",
        "independence": "Fresh worktree/branch isolation does not establish an independent numerical oracle.",
        "target": {"branch": branch, "destination": str(destination), "pull_request_base": "main",
                   "observed_main_commit": required.observed_main, "checked_local_main_ref": checked_main_ref,
                   "checked_local_main_commit": checked_main, "remote_main_freshness": "NOT_CHECKED_NO_FETCH",
                   "current_main_relation": main_relation, "reconciliation_required": reconciliation_required,
                   "common_merge_bases": merge_bases,
                   "cleanliness": "DESTINATION_ABSENT; verify after creation"},
        "caller_checkout": {"repository": str(repository), "head": git.text("rev-parse", "HEAD"),
                            "dirty": bool(git.run("status", "--porcelain=v1", "--untracked-files=all")),
                            "uncommitted_changes_copied": False},
        "plan_manifests": identities,
        "planned_command_ids": [c["id"] for c in manifests["commands"]["commands"]],
        "coverage": coverage_drift(git, source, manifests["coverage"]),
        "command": {"argv": argv, "environment": SAFE_ENV,
                    "shell": shlex.join(["env", *(f"{k}={v}" for k, v in SAFE_ENV.items()), *argv])},
        "limitations": ["This helper schedules Git inspection/worktree commands, not fetch, push, merge, cherry-pick, reset, force options or solver validation.",
                        "Git hooks are suppressed for these commands; configured checkout filters may still run.",
                        "Advanced/diverged current main requires separate reconciliation in the new worktree.",
                        "Ancestry proves inclusion in history; descendant edits may supersede earlier behavior.",
                        "The preparation receipt does not replay historical evidence or pass qualification gates."],
    }


def create_worktree(plan: dict) -> None:
    git = Git(Path(plan["caller_checkout"]["repository"]))
    target, source = plan["target"], plan["source_commit"]
    destination = fresh_target(git, Path(target["destination"]), target["branch"])
    try:
        git.run("worktree", "add", "-b", target["branch"], str(destination), source)
        created = Git(destination)
        if created.text("rev-parse", "HEAD") != source:
            raise PreparationError("Created worktree HEAD differs from selected source")
        if created.text("symbolic-ref", "--short", "HEAD") != target["branch"]:
            raise PreparationError("Created worktree branch differs from the requested branch")
        if created.run("status", "--porcelain=v1", "--untracked-files=all"):
            raise PreparationError("Created worktree is not clean")
    except PreparationError as exc:
        raise PreparationError(f"Creation did not qualify: {exc}. Inspect {destination} and branch "
                               f"{target['branch']}; no automatic cleanup/reset is performed.") from exc
    plan["status"] = "CREATED"
    target["cleanliness"] = "VERIFIED_CLEAN_AT_SELECTED_SOURCE"


def main(argv: list[str] | None = None, *, required: RequiredSources = REQUIRED,
         invocation_dir: Path | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha", type=full_sha, required=True,
                        help="Full delivered/descendant commit containing all three committed manifests")
    parser.add_argument("--destination", type=Path, help="Absent worktree path; default is a sibling of this checkout")
    parser.add_argument("--branch", help="Fresh local branch; defaults to validation/integrated-<source-prefix>")
    parser.add_argument("--main-ref", help="Optional locally available main ref; cached origin/main is checked when present")
    parser.add_argument("--create", action="store_true", help="Create the new branch/worktree after all preflight checks")
    args = parser.parse_args(argv)
    try:
        repository = Path(Git(invocation_dir or Path.cwd()).text("rev-parse", "--show-toplevel"))
        plan = build_plan(repository, args.source_sha, args.destination, args.branch,
                          args.main_ref, required=required)
        if args.create:
            create_worktree(plan)
        print(json.dumps(plan, indent=2, sort_keys=True))
        return 0
    except (PreparationError, OSError, KeyError, TypeError, ValueError) as exc:
        print(json.dumps({"status": "REFUSED", "error": str(exc), "native_validation_status": "NOT_RUN"}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
