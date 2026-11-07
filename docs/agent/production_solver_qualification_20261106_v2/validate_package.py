#!/usr/bin/env python3
"""Fail-closed validation for package bytes and the exact source snapshot."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]

def fail(message):
    print("FAIL:", message, file=sys.stderr)
    raise SystemExit(1)

def load(name):
    try:
        return json.loads((ROOT / name).read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON {name}: {exc}")

def git(*args):
    try:
        return subprocess.check_output(["git", *args], cwd=REPO, text=True, stderr=subprocess.STDOUT).strip()
    except Exception as exc:
        fail(f"git preflight failed: {exc}")

manifest = load("MANIFEST.json")
repo_id = manifest["repository"]
if git("branch", "--show-current") != repo_id["branch"]:
    fail("current branch differs from pinned branch")
if git("rev-parse", "HEAD") != repo_id["head"]:
    fail("current HEAD differs from pinned HEAD; re-audit before applying tasks")
if git("rev-parse", "HEAD^{tree}") != repo_id["tree"]:
    fail("current tree differs from pinned tree")
if git("rev-parse", "origin/" + repo_id["branch"]) != repo_id["origin_tracking_head"]:
    fail("origin tracking head differs from manifest")
if repo_id["local_remote_head_equal"] is not True or repo_id["head"] != repo_id["origin_tracking_head"]:
    fail("manifest source identity was not synchronized at intake")
if git("diff", "--quiet") != "":
    fail("tracked worktree has unstaged source changes")
if git("diff", "--cached", "--quiet") != "":
    fail("index has staged source changes")

# Source preconditions: a changed source invalidates this handoff's line-level findings.
for rel, item in manifest.get("source_hashes", {}).items():
    p = (REPO / rel).resolve()
    if REPO.resolve() not in p.parents:
        fail(f"source path escapes repository: {rel}")
    if not p.is_file():
        fail(f"pinned source missing: {rel}")
    data = p.read_bytes()
    if len(data) != item["bytes"]:
        fail(f"source byte-count mismatch: {rel}")
    if hashlib.sha256(data).hexdigest() != item["sha256"]:
        fail(f"source SHA256 mismatch: {rel}")

# Package byte identity.
files = manifest.get("files")
if not isinstance(files, dict) or not files:
    fail("MANIFEST.files must be a non-empty path-to-SHA256 mapping")
actual = {
    p.relative_to(ROOT).as_posix()
    for p in ROOT.rglob("*")
    if p.is_file() and p.name not in {"MANIFEST.json", "SHA256SUMS"}
}
if set(files) != actual:
    fail(f"manifest file set mismatch; missing={sorted(set(files)-actual)} extra={sorted(actual-set(files))}")
for rel, expected in files.items():
    p = (ROOT / rel).resolve()
    if ROOT.resolve() not in p.parents:
        fail(f"path escapes package: {rel}")
    if hashlib.sha256(p.read_bytes()).hexdigest() != expected:
        fail(f"manifest SHA256 mismatch: {rel}")

checksums = {}
for line in (ROOT / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
    try:
        expected, rel = line.split("  ", 1)
    except ValueError:
        fail("malformed SHA256SUMS line")
    if rel in checksums:
        fail(f"duplicate SHA256 entry: {rel}")
    checksums[rel] = expected
sum_paths = {
    p.relative_to(ROOT).as_posix()
    for p in ROOT.rglob("*")
    if p.is_file() and p.name != "SHA256SUMS"
}
if set(checksums) != sum_paths:
    fail("SHA256SUMS file set mismatch")
for rel, expected in checksums.items():
    if hashlib.sha256((ROOT / rel).read_bytes()).hexdigest() != expected:
        fail(f"SHA256SUMS mismatch: {rel}")

parsed = {}
for p in ROOT.glob("*.json"):
    try:
        parsed[p.name] = json.loads(p.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON {p.name}: {exc}")

queue = parsed["TASK_QUEUE.json"]
tasks = queue["tasks"]
ids = [task["task_id"] for task in tasks]
if len(ids) != len(set(ids)):
    fail("duplicate task_id")
known = set(ids)
dependencies = {}
required = ["title", "priority", "base_sha", "depends_on", "read_allowlist",
            "write_allowlist", "math_contract", "implementation_contract", "acceptance",
            "requires_isolated_worktree", "requires_owner_approval", "mutation_required"]
for task in tasks:
    missing = [key for key in required if key not in task]
    if missing:
        fail(f"task schema fields missing for {task.get('task_id')}: {missing}")
    if task["status"] != "PROPOSED_NOT_AUTHORIZED":
        fail(f"initial task status is not fail-closed: {task['task_id']}")
    if task["base_sha"] != repo_id["head"]:
        fail(f"task base SHA mismatch: {task['task_id']}")
    if task["requires_isolated_worktree"] is not True or task["requires_owner_approval"] is not True:
        fail(f"mutation gate missing: {task['task_id']}")
    if task["mutation_required"] is not True:
        fail(f"mutation flag missing: {task['task_id']}")
    if not task["read_allowlist"] or not task["write_allowlist"] or not task["acceptance"]:
        fail(f"empty allowlist or acceptance: {task['task_id']}")
    for rel in task["read_allowlist"] + task["write_allowlist"]:
        from pathlib import PurePosixPath
        candidate = PurePosixPath(rel)
        if candidate.is_absolute() or ".." in candidate.parts:
            fail(f"unsafe allowlist path: {task['task_id']} {rel}")
    for dependency in task["depends_on"]:
        if dependency not in known:
            fail(f"dangling dependency: {task['task_id']} -> {dependency}")
    dependencies[task["task_id"]] = task["depends_on"]

temporary, permanent = set(), set()
def visit(node):
    if node in permanent:
        return
    if node in temporary:
        fail(f"dependency cycle at {node}")
    temporary.add(node)
    for dependency in dependencies[node]:
        visit(dependency)
    temporary.remove(node)
    permanent.add(node)

for node in dependencies:
    visit(node)

graph = parsed["TASK_GRAPH.json"]
if graph["dependencies"] != dependencies:
    fail("TASK_GRAPH does not match TASK_QUEUE dependencies")
if parsed["PRODUCTION_VERDICT.json"]["overall"] != "HOLD_NOT_RELEASE_QUALIFIED":
    fail("initial verdict must remain HOLD")
if manifest["task_count"] != len(tasks):
    fail("manifest task_count mismatch")
if queue["repository"]["base_sha"] != repo_id["head"]:
    fail("manifest/queue source identity mismatch")
if manifest["loop"]["native_subagent"] != "NOT_RUN":
    fail("native subagent provenance must be explicit")

print(f"PASS: pinned source, {len(files)} package files, {len(parsed)} JSON documents, "
      f"{len(tasks)} tasks, DAG acyclic, HOLD preserved.")
