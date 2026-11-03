#!/usr/bin/env python3
"""Deterministic gate for agent-executed tasks (Python standard library only).

The gate enforces what a prompt cannot: task order, file scope, exact anchors,
append-only files, forbidden diff patterns, red-before-green tests, re-run
acceptance commands, cargo-mutants results and a completed self-audit record.

Task data: docs/agent/switcher_v2_remediation/TASKS.json (read-only for agents).
Run state: .agent/STATE.json (git-ignored; written by this script, plus the
agent's own edits of its self_audit block).

Usage (from the repository root):
  python3 tools/agent_gate.py validate             # TASKS.json schema, DAG, anchors
  python3 tools/agent_gate.py status               # task states and READY tasks
  python3 tools/agent_gate.py next                 # the next READY task and its design section
  python3 tools/agent_gate.py start TASK_ID        # begin a task (deps, approval, clean tree, anchors)
  python3 tools/agent_gate.py red TASK_ID          # prove the new tests fail on the task's base commit
  python3 tools/agent_gate.py check [TASK_ID]      # scope, patterns, append-only files, acceptance greps
  python3 tools/agent_gate.py mutation TASK_ID     # cargo-mutants on the task diff
  python3 tools/agent_gate.py finish TASK_ID       # full gate; marks the task done
  python3 tools/agent_gate.py abort TASK_ID REASON # mark blocked, clear the current task (escalation)
  python3 tools/agent_gate.py approve TASK_ID NAME # HUMAN ONLY (denied to the agent in .claude/settings.json)
  python3 tools/agent_gate.py pre-edit             # Claude Code PreToolUse hook (JSON on stdin)
  python3 tools/agent_gate.py stop                 # Claude Code Stop hook (JSON on stdin)

Exit codes: 0 pass, 1 check failed, 2 hook block (Claude Code convention), 3 usage error.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TASKS = ROOT / "docs/agent/switcher_v2_remediation/TASKS.json"
STATE = ROOT / ".agent/STATE.json"
SCHEMA = "vigilode.agent-tasks.v1"
TIERS = {"S", "M", "L", "H"}
APPROVAL_TIERS = {"L", "H"}
KINDS = {"governance", "bitwise_safe", "opt_in_behaviour", "default_behaviour", "experiment", "docs"}
ACCEPT_KINDS = {"grep_present", "grep_absent", "test_name_present", "mutation"}
REQUIRED_TASK_KEYS = ["id", "title", "phase", "tier", "kind", "depends_on", "files_allowed", "anchors",
                      "design_section", "invariants", "tests_required", "red", "commands", "acceptance",
                      "ledger_required", "escalate_if", "commit_message", "removals_allowed", "spec_only"]
SELF_AUDIT_KEYS = {
    "restated_goal": str,
    "symbols_verified": list,
    "spec_steps_done": list,
    "tests_added": list,
    "commands_run": list,
    "invariants_checked": dict,
    "no_unrequested_changes": bool,
    "open_questions": list,
}
TEST_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", re.M)


# ----------------------------------------------------------------- helpers
def sh(args: list[str], check: bool = True, cwd: Path = ROOT) -> str:
    out = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if check and out.returncode != 0:
        raise RuntimeError(f"{' '.join(args)} failed ({out.returncode}): {out.stderr.strip()[:500]}")
    return out.stdout


def load_json(path: Path) -> dict:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def save_state(state: dict) -> None:
    STATE.parent.mkdir(parents=True, exist_ok=True)
    tmp = STATE.with_suffix(".tmp")
    tmp.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(STATE)


def load_state(spec: dict) -> dict:
    if STATE.exists():
        state = load_json(STATE)
    else:
        state = {"schema": "vigilode.agent-state.v1", "current_task": None, "tasks": {}}
    for t in spec["tasks"]:
        state["tasks"].setdefault(t["id"], {"status": "todo"})
    state.setdefault("approvals", {})
    return state


def match_any(path: str, globs: list[str]) -> bool:
    # fnmatch: `*` also matches `/`, so "crates/*.rs" covers every Rust file under crates/.
    return any(fnmatch.fnmatchcase(path, g) for g in globs)


def read_at(rev: str | None, rel: str) -> str | None:
    if rev is None:
        p = ROOT / rel
        return p.read_text(encoding="utf-8") if p.exists() else None
    out = subprocess.run(["git", "show", f"{rev}:{rel}"], cwd=ROOT, capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else None


def anchor_errors(task: dict, rev: str | None) -> list[str]:
    errs = []
    for anc in task.get("anchors", []):
        where = None if (rev is None or anc.get("at") == "head") else rev
        text = read_at(where, anc["file"])
        if text is None:
            errs.append(f"{task['id']}: anchor file {anc['file']} unreadable at {where or 'working tree'}")
            continue
        n = text.count(anc["text"])
        if n != anc.get("count", 1):
            errs.append(f"{task['id']}: anchor found {n}x in {anc['file']} "
                        f"(expected {anc.get('count', 1)}): {anc['text'][:70]!r}")
    return errs


# ----------------------------------------------------------------- validate
def validate(spec: dict) -> list[str]:
    errors: list[str] = []
    if spec.get("schema") != SCHEMA:
        errors.append(f"schema must be {SCHEMA}")
    base = spec.get("base_commit", "")
    if not re.fullmatch(r"[0-9a-f]{40}", base):
        errors.append("base_commit must be a full 40-hex SHA")
    ids = [t.get("id") for t in spec.get("tasks", [])]
    if len(ids) != len(set(ids)):
        errors.append("duplicate task ids")
    inv_ids = {i["id"] for i in spec.get("global", {}).get("invariants", [])}
    design_path = ROOT / spec.get("design_doc", "")
    design = design_path.read_text(encoding="utf-8") if design_path.is_file() else ""
    if not design:
        errors.append("design_doc missing or empty")
    for t in spec.get("tasks", []):
        tid = t.get("id", "?")
        for k in REQUIRED_TASK_KEYS:
            if k not in t:
                errors.append(f"{tid}: missing key {k}")
        if t.get("tier") not in TIERS:
            errors.append(f"{tid}: tier must be one of {sorted(TIERS)}")
        if t.get("kind") not in KINDS:
            errors.append(f"{tid}: kind must be one of {sorted(KINDS)}")
        for d in t.get("depends_on", []):
            if d not in ids:
                errors.append(f"{tid}: unknown dependency {d}")
        for i in t.get("invariants", []):
            if i not in inv_ids:
                errors.append(f"{tid}: unknown invariant {i}")
        if f'<a id="{t.get("design_section", "")}"></a>' not in design:
            errors.append(f"{tid}: design section anchor {t.get('design_section')!r} not in design doc")
        for a in t.get("acceptance", []):
            if a.get("kind") not in ACCEPT_KINDS:
                errors.append(f"{tid}: unknown acceptance kind {a.get('kind')}")
        names = {x["name"] for x in t.get("tests_required", [])}
        for r in t.get("red", {}).get("tests", []):
            if r not in names:
                errors.append(f"{tid}: red test {r} is not in tests_required")
        if t.get("red", {}).get("tests") and not t["red"].get("cmd"):
            errors.append(f"{tid}: red tests need red.cmd")
        errors += anchor_errors(t, base)
    graph = {t["id"]: t.get("depends_on", []) for t in spec.get("tasks", []) if "id" in t}
    seen: dict[str, int] = {}

    def visit(node: str, stack: list[str]) -> None:
        if seen.get(node) == 1:
            errors.append("dependency cycle: " + " -> ".join(stack + [node]))
            return
        if seen.get(node) == 2:
            return
        seen[node] = 1
        for nxt in graph.get(node, []):
            visit(nxt, stack + [node])
        seen[node] = 2

    for node in graph:
        visit(node, [])
    return errors


# ----------------------------------------------------------------- diff checks
def changed_files(base: str) -> list[str]:
    tracked = sh(["git", "diff", "--name-only", base]).split()
    untracked = sh(["git", "ls-files", "--others", "--exclude-standard"]).split()
    return sorted(set(tracked) | set(untracked))


def added_and_removed_lines(base: str) -> tuple[list[tuple[str, str]], list[tuple[str, str]]]:
    diff = sh(["git", "diff", "--unified=0", base])
    added, removed, cur = [], [], None
    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            cur = line[6:]
        elif line.startswith("+++ /dev/null"):
            cur = None
        elif line.startswith("--- a/"):
            cur = line[6:]
        elif line.startswith("+") and not line.startswith("+++") and cur:
            added.append((cur, line[1:]))
        elif line.startswith("-") and not line.startswith("---") and cur:
            removed.append((cur, line[1:]))
    for path in sh(["git", "ls-files", "--others", "--exclude-standard"]).split():
        p = ROOT / path
        if p.is_file() and p.suffix in {".rs", ".py", ".json", ".md", ".toml", ".jsonl"}:
            for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
                added.append((path, line))
    return added, removed


def ledger_row_errors(old: list[str], new: list[str]) -> list[str]:
    errs = []
    keys = ("id", "date", "commit", "command", "profile", "inputs_sha256", "outputs_sha256",
            "claim", "verdict", "supersedes")
    for i, line in enumerate(new[len(old):], start=len(old) + 1):
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            errs.append(f"LEDGER line {i}: not JSON")
            continue
        if not isinstance(row, dict):
            errs.append(f"LEDGER line {i}: not a JSON object")
            continue
        for k in keys:
            if k not in row:
                errs.append(f"LEDGER line {i}: missing {k}")
        if row.get("verdict") not in {"PASS", "FAIL", "INCONCLUSIVE", "INVALID"}:
            errs.append(f"LEDGER line {i}: bad verdict")
        if str(row.get("command", "")).startswith("UNRECORDED") and row.get("verdict") not in {"INCONCLUSIVE", "INVALID"}:
            errs.append(f"LEDGER line {i}: UNRECORDED command needs verdict INCONCLUSIVE or INVALID")
        if len(row.get("claim", "")) > 600:
            errs.append(f"LEDGER line {i}: claim longer than 600 characters")
        if not re.fullmatch(r"[0-9a-f]{40}", str(row.get("commit", ""))):
            errs.append(f"LEDGER line {i}: commit must be a full 40-hex SHA")
        elif not sh(["git", "branch", "-r", "--contains", row["commit"]], check=False).strip():
            errs.append(f"LEDGER line {i}: commit {row['commit'][:10]} is not reachable from a remote branch (INV-13)")
    return errs


def append_only_errors(spec: dict, base: str) -> list[str]:
    errs = []
    for rel in spec["global"].get("append_only", []):
        old = read_at(base, rel)
        if old is None:
            continue
        new = (ROOT / rel).read_text(encoding="utf-8") if (ROOT / rel).exists() else ""
        if new.splitlines()[: len(old.splitlines())] != old.splitlines():
            errs.append(f"{rel}: existing lines changed, removed or reordered (append-only)")
        elif rel.endswith("LEDGER.jsonl"):
            errs += ledger_row_errors(old.splitlines(), new.splitlines())
    return errs


def fast_checks(spec: dict, task: dict, base: str) -> list[str]:
    g = spec["global"]
    errs: list[str] = []
    always_ok = g.get("always_allowed", []) + ["tools/agent_gate.py"]
    allowed = task["files_allowed"] + always_ok
    for f in changed_files(base):
        if match_any(f, always_ok):
            continue
        if match_any(f, g.get("forbidden_paths", [])):
            errs.append(f"forbidden path changed: {f}")
        elif not match_any(f, allowed):
            errs.append(f"out-of-scope file changed: {f} (task {task['id']} allows {task['files_allowed']})")
    added, removed = added_and_removed_lines(base)
    added = [(p, l) for p, l in added if p != "tools/agent_gate.py"]
    removed = [(p, l) for p, l in removed if p != "tools/agent_gate.py"]
    for rule in g.get("forbidden_added_patterns", []):
        rx = re.compile(rule["regex"])
        for path, line in added:
            if match_any(path, rule.get("paths", ["*"])) and rx.search(line):
                errs.append(f"forbidden pattern ({rule['reason']}) added in {path}: {line.strip()[:100]}")
    exempt = {(r["file"], r["line"].strip()) for r in task.get("removals_allowed", [])}
    for rule in g.get("forbidden_removed_patterns", []):
        rx = re.compile(rule["regex"])
        for path, line in removed:
            if (path, line.strip()) in exempt:
                continue
            if match_any(path, rule.get("paths", ["*"])) and rx.search(line):
                errs.append(f"forbidden removal ({rule['reason']}) in {path}: {line.strip()[:100]}")
    errs += append_only_errors(spec, base)
    for tr in task["tests_required"]:
        p = ROOT / tr["file"]
        text = p.read_text(encoding="utf-8") if p.exists() else ""
        if not re.search(rf"fn\s+{re.escape(tr['name'])}\s*\(", text):
            errs.append(f"required test fn {tr['name']} not found in {tr['file']}")
    for a in task["acceptance"]:
        if a["kind"] in {"grep_present", "grep_absent", "test_name_present"}:
            p = ROOT / a["file"]
            text = p.read_text(encoding="utf-8") if p.exists() else ""
            if a["kind"] == "grep_absent" and a["text"] in text:
                errs.append(f"acceptance grep_absent failed: {a['text'][:80]!r} still in {a['file']}")
            if a["kind"] == "grep_present" and text.count(a["text"]) < a.get("min_count", 1):
                errs.append(f"acceptance grep_present failed: {a['text'][:80]!r} not in {a['file']}")
            if a["kind"] == "test_name_present" and not re.search(rf"fn\s+{re.escape(a['text'])}\s*\(", text):
                errs.append(f"required test fn {a['text']} not found in {a['file']}")
    return errs


def audit_errors(task: dict, record: dict) -> list[str]:
    errs = []
    audit = record.get("self_audit")
    if not isinstance(audit, dict):
        return ["self_audit missing in .agent/STATE.json"]
    for key, typ in SELF_AUDIT_KEYS.items():
        if not isinstance(audit.get(key), typ):
            errs.append(f"self_audit.{key} missing or not {typ.__name__}")
    if errs:
        return errs
    if not audit["restated_goal"].strip():
        errs.append("self_audit.restated_goal is empty")
    if task["kind"] != "governance" and not audit["symbols_verified"]:
        errs.append("self_audit.symbols_verified is empty (record file:line of every symbol you used)")
    for s in audit["symbols_verified"]:
        if not (isinstance(s, dict) and s.get("symbol") and s.get("file") and isinstance(s.get("line"), int)):
            errs.append(f"self_audit.symbols_verified entry malformed: {s}")
            continue
        p = ROOT / s["file"]
        if not p.exists():
            errs.append(f"symbol file does not exist: {s['file']}")
            continue
        lines = p.read_text(encoding="utf-8").splitlines()
        if not (1 <= s["line"] <= len(lines)) or s["symbol"] not in lines[s["line"] - 1]:
            errs.append(f"symbol {s['symbol']!r} not on {s['file']}:{s['line']} (hallucinated or stale reference)")
    for tr in task["tests_required"]:
        before = read_at(record.get("base_sha"), tr["file"]) or ""
        is_new = not re.search(rf"fn\s+{re.escape(tr['name'])}\s*\(", before)
        if is_new and tr["name"] not in audit["tests_added"]:
            errs.append(f"self_audit.tests_added lacks required new test {tr['name']}")
    for inv in task["invariants"]:
        if not str(audit["invariants_checked"].get(inv, "")).startswith("held:"):
            errs.append(f"self_audit.invariants_checked[{inv}] must start with 'held:' plus evidence")
    if audit["no_unrequested_changes"] is not True:
        errs.append("self_audit.no_unrequested_changes must be true")
    if audit["open_questions"]:
        errs.append("open questions remain: escalate with `abort TASK_ID REASON` instead of finishing")
    if not audit["spec_steps_done"]:
        errs.append("self_audit.spec_steps_done is empty (list the DESIGN.md steps you completed)")
    for c in audit["commands_run"]:
        if not (isinstance(c, dict) and c.get("exit_code") == 0 and c.get("cmd")):
            errs.append(f"self_audit.commands_run entry not a passing command: {c}")
    return errs


def run_commands(task: dict) -> list[str]:
    errs = []
    for c in task["commands"]:
        try:
            proc = subprocess.run(c["cmd"], cwd=ROOT, shell=True, capture_output=True, text=True,
                                  timeout=c.get("timeout_s", 1800))
        except subprocess.TimeoutExpired:
            errs.append(f"command timed out: {c['cmd']}")
            continue
        if proc.returncode != 0:
            tail = (proc.stdout + proc.stderr).strip().splitlines()[-15:]
            errs.append(f"command failed ({proc.returncode}): {c['cmd']}\n    " + "\n    ".join(tail))
    return errs


# ----------------------------------------------------------------- red and mutation
def red_check(task: dict, record: dict) -> list[str]:
    red = task.get("red", {})
    if not red.get("tests"):
        return []
    base = record["base_sha"]
    wt = ROOT / ".agent" / "red-worktree"
    subprocess.run(["git", "worktree", "remove", "--force", str(wt)], cwd=ROOT, capture_output=True)
    if wt.exists():
        shutil.rmtree(wt)
    sh(["git", "worktree", "add", "--detach", str(wt), base])
    try:
        for rel in red.get("copy", []):
            src = ROOT / rel
            if not src.exists():
                return [f"red: file to copy does not exist: {rel}"]
            (wt / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, wt / rel)
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target" / "agent-red"))
        try:
            proc = subprocess.run(red["cmd"], cwd=wt, shell=True, capture_output=True, text=True, env=env,
                                  timeout=red.get("timeout_s", 3600))
        except subprocess.TimeoutExpired:
            return [f"red: command timed out: {red['cmd']}"]
        out = proc.stdout + proc.stderr
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(wt)], cwd=ROOT, capture_output=True)
    results = dict(TEST_LINE.findall(out))
    compile_failed = "could not compile" in out or "error[E" in out
    errs, details = [], {}
    for name in red["tests"]:
        status = results.get(name)
        if status == "FAILED":
            details[name] = "red: failed on base"
        elif (status is None and compile_failed and red.get("allow_compile_red")
              and any(sym in out for sym in red.get("new_symbols", []))):
            details[name] = "red: does not compile on base (new symbol absent)"
        else:
            details[name] = f"NOT red on base (status {status or 'missing'})"
            errs.append(f"test {name} is not red on base {base[:10]}: it does not detect the missing change")
    record["red"] = {"base_sha": base, "ok": not errs, "details": details}
    return errs


def mutation(task: dict, record: dict) -> list[str]:
    acc = [a for a in task["acceptance"] if a["kind"] == "mutation"]
    if not acc:
        return []
    a = acc[0]
    out_dir = ROOT / ".agent" / "mutants" / task["id"]
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
    diff_path = out_dir / "task.diff"
    # --unified=0: only changed lines count as "in the diff", so unchanged neighbours generate no mutants.
    diff_path.write_text(sh(["git", "diff", "--unified=0", record["base_sha"]]), encoding="utf-8")
    avail_gib = 4
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                avail_gib = int(line.split()[1]) // (1024 * 1024)
                break
    except OSError:
        pass
    safe_jobs = max(1, min(4, avail_gib // 4))
    jobs = max(int(a.get("jobs", 2)), safe_jobs)
    cmd = ["cargo", "mutants", "-p", a.get("package", "vigilode"), "--in-diff", str(diff_path),
           "-o", str(out_dir), "-j", str(jobs), "--jobserver-tasks", str(min(16, jobs * 4)),
           "--timeout", str(a.get("timeout_s", 600))]
    if a.get("profile"):
        cmd += ["--profile", a["profile"]]
    test_args = list(a.get("cargo_test_args", []))
    idx = 0
    while idx < len(test_args):
        arg = test_args[idx]
        if arg == "--lib":
            cmd.append("--cargo-arg=--lib")
            idx += 1
        elif arg == "--test" and idx + 1 < len(test_args):
            cmd.append(f"--cargo-arg=--test={test_args[idx + 1]}")
            idx += 2
        else:
            cmd.append(f"--cargo-test-arg={arg}")
            idx += 1
    # Debug info off: each cargo-mutants job copies the workspace and builds it; with full debug
    # info one copy reached 14 GB in the dry run.
    env = dict(os.environ, CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0")
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env)
    mo = out_dir / "mutants.out"
    if not mo.exists():
        return [f"cargo mutants produced no output (exit {proc.returncode}); command: {' '.join(cmd)}"]

    def read(name: str) -> list[str]:
        p = mo / name
        return [m for m in p.read_text(encoding="utf-8").splitlines() if m.strip()] if p.exists() else []

    missed, timeouts, caught = read("missed.txt"), read("timeout.txt"), read("caught.txt")
    # Waivers name equivalent mutants without the `path:line:col: ` prefix, so they survive line shifts.
    # Task-level waivers are reviewed in DESIGN.md; record-level waivers are written by a human.
    waived = set(task.get("mutation_waivers", {})) | set(record.get("mutation_waivers", {}))
    strip = re.compile(r"^\S+?:\d+:\d+: ")
    unwaived = [m for m in missed + timeouts if m not in waived and strip.sub("", m) not in waived]
    record["mutation"] = {"missed": missed, "timeout": timeouts, "caught": len(caught),
                          "unviable": len(read("unviable.txt")), "unwaived": unwaived}
    if proc.returncode not in (0, 2, 3) and not missed and not caught:
        return [f"cargo mutants failed (exit {proc.returncode}); the unmutated baseline probably fails: "
                f"{proc.stderr.strip()[-400:]}"]
    if len(unwaived) > a.get("max_missed", 0):
        return ([f"{len(unwaived)} surviving mutant(s) without waiver; add a test that kills each:"]
                + [f"  {m}" for m in unwaived])
    return []


# ----------------------------------------------------------------- commands
def receipt(tid: str, record: dict) -> str:
    # Written only by `finish`. It does not stop deliberate forgery; it makes a hand-edited
    # "status": "done" (a hallucinated completion) visible to `start`, `next` and `status`.
    return hashlib.sha256(f"vigilode-gate-v1|{tid}|{record.get('base_sha')}|{record.get('diff_sha256')}".encode()).hexdigest()


def is_done(state: dict, tid: str) -> bool:
    rec = state["tasks"].get(tid, {})
    return rec.get("status") == "done" and rec.get("receipt") == receipt(tid, rec)


def ready(state: dict, t: dict) -> bool:
    st = state["tasks"][t["id"]].get("status", "todo")
    return st == "todo" and all(is_done(state, d) for d in t["depends_on"])


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 3
    cmd = argv[1]
    spec = load_json(TASKS)
    by_id = {t["id"]: t for t in spec["tasks"]}
    state = load_state(spec)

    if cmd == "validate":
        errs = validate(spec)
        print("\n".join(errs) if errs else f"OK: {len(spec['tasks'])} tasks, anchors verified at {spec['base_commit'][:8]}")
        return 1 if errs else 0

    if cmd == "status":
        for t in spec["tasks"]:
            st = state["tasks"][t["id"]].get("status", "todo")
            if st == "done" and not is_done(state, t["id"]):
                st = "done?NO-RECEIPT"
            flag = "READY " if ready(state, t) else "      "
            print(f"{t['id']:4s} {st:12s} tier={t['tier']} {flag}{t['title']}")
        print(f"current_task: {state.get('current_task')}")
        return 0

    if cmd == "next":
        if state.get("current_task"):
            print(f"current task: {state['current_task']} (finish or abort it first)")
            return 0
        for t in spec["tasks"]:
            if ready(state, t):
                need = (" (needs human approval first)"
                        if t["tier"] in APPROVAL_TIERS and t["id"] not in state["approvals"] else "")
                print(f"next: {t['id']} tier={t['tier']}{need}\nread: {spec['design_doc']} section {t['design_section']}")
                return 0
        print("no READY task")
        return 0

    if cmd == "start":
        tid = argv[2] if len(argv) > 2 else ""
        if tid not in by_id:
            print(f"unknown task {tid}")
            return 3
        task = by_id[tid]
        if state.get("current_task"):
            print(f"task {state['current_task']} is in progress; finish or abort it first")
            return 1
        if state["tasks"][tid].get("status") == "done":
            print(f"{tid} is already done")
            return 1
        missing = [d for d in task["depends_on"] if not is_done(state, d)]
        if missing:
            print(f"dependencies not done: {missing}")
            return 1
        if task["tier"] in APPROVAL_TIERS and tid not in state["approvals"]:
            print(f"{tid} is tier {task['tier']}: a human must run "
                  f"`python3 tools/agent_gate.py approve {tid} NAME` first")
            return 1
        dirty = [line for line in sh(["git", "status", "--porcelain"]).splitlines()
                 if not line[3:].startswith(".agent/")]
        if dirty:
            print("working tree not clean outside .agent/; commit or stash first:\n" + "\n".join(dirty[:10]))
            return 1
        errs = anchor_errors(task, None)
        if errs:
            print("anchors do not match the current tree; do not edit, escalate with abort:\n" + "\n".join(errs))
            return 1
        state["current_task"] = tid
        state["tasks"][tid] = {
            "status": "in_progress",
            "base_sha": sh(["git", "rev-parse", "HEAD"]).strip(),
            "self_audit": {k: ([] if v is list else {} if v is dict else False if v is bool else "")
                           for k, v in SELF_AUDIT_KEYS.items()},
        }
        save_state(state)
        note = (" This task is spec_only: no dry-run-verified code exists; follow the spec literally."
                if task["spec_only"] else "")
        print(f"started {tid}. Read {spec['design_doc']} section {task['design_section']} before editing.{note}")
        return 0

    if cmd == "pre-edit":
        try:
            event = json.load(sys.stdin)
        except (json.JSONDecodeError, ValueError):
            return 0
        tool_input = event.get("tool_input", {}) or {}
        path = tool_input.get("file_path") or tool_input.get("notebook_path")
        if not path:
            return 0
        resolved = Path(path).resolve()
        if ROOT not in resolved.parents:
            return 0  # outside the repository: not governed by the task scope
        rel = resolved.relative_to(ROOT).as_posix()
        g = spec["global"]
        if match_any(rel, g.get("always_allowed", [])):
            return 0
        if match_any(rel, g.get("forbidden_paths", [])):
            print(f"BLOCKED: {rel} is read-only for agents (forbidden_paths). Escalate if it must change.",
                  file=sys.stderr)
            return 2
        tid = state.get("current_task")
        if not tid:
            print("BLOCKED: no current task. Run `python3 tools/agent_gate.py next`, then `start TASK_ID`.",
                  file=sys.stderr)
            return 2
        if not match_any(rel, by_id[tid]["files_allowed"]):
            print(f"BLOCKED: {rel} is outside task {tid} files_allowed {by_id[tid]['files_allowed']}. "
                  "Re-read the task card; if the change is truly needed, run `abort` with the reason.",
                  file=sys.stderr)
            return 2
        return 0

    if cmd in {"stop", "check"}:
        if cmd == "stop":
            try:
                json.load(sys.stdin)
            except (json.JSONDecodeError, ValueError):
                pass
        tid = argv[2] if len(argv) > 2 else state.get("current_task")
        if not tid:
            return 0
        record = state["tasks"].get(tid, {})
        if record.get("status") != "in_progress":
            return 0
        errs = fast_checks(spec, by_id[tid], record["base_sha"])
        if cmd == "stop":
            errs += audit_errors(by_id[tid], record)
        if errs:
            if cmd == "stop":
                errs.append(f"then run `python3 tools/agent_gate.py finish {tid}`, "
                            f"or `abort {tid} REASON` to escalate")
            stream = sys.stderr if cmd == "stop" else sys.stdout
            print(f"Task {tid} is not finished:\n- " + "\n- ".join(errs), file=stream)
            return 2 if cmd == "stop" else 1
        print(f"{tid}: fast checks pass")
        return 0

    if cmd in {"red", "mutation"}:
        tid = argv[2] if len(argv) > 2 else ""
        record = state["tasks"].get(tid, {})
        if record.get("status") != "in_progress":
            print(f"{tid} is not in progress")
            return 1
        errs = red_check(by_id[tid], record) if cmd == "red" else mutation(by_id[tid], record)
        save_state(state)
        print("\n".join(errs) if errs else f"{tid}: {cmd} gate passes")
        return 1 if errs else 0

    if cmd == "finish":
        tid = argv[2] if len(argv) > 2 else ""
        record = state["tasks"].get(tid, {})
        if record.get("status") != "in_progress":
            print(f"{tid} is not in progress")
            return 1
        task = by_id[tid]
        errs = fast_checks(spec, task, record["base_sha"]) + audit_errors(task, record)
        if not errs and task["red"]["tests"] and not record.get("red", {}).get("ok"):
            errs += red_check(task, record)
        if not errs:
            errs += run_commands(task)
        if not errs:
            errs += mutation(task, record)
        if errs:
            save_state(state)
            print(f"{tid} NOT finished:\n- " + "\n- ".join(errs))
            return 1
        record["status"] = "done"
        record["diff_sha256"] = hashlib.sha256(sh(["git", "diff", record["base_sha"]]).encode()).hexdigest()
        record["receipt"] = receipt(tid, record)
        state["current_task"] = None
        save_state(state)
        print(f"{tid} done. Append a PROGRESS.md entry, then commit with: {task['commit_message']}")
        return 0

    if cmd == "abort":
        if len(argv) < 3:
            print("usage: abort TASK_ID REASON")
            return 3
        tid, reason = argv[2], " ".join(argv[3:]) or "unspecified"
        state["tasks"].setdefault(tid, {})["status"] = "blocked"
        state["tasks"][tid]["blocked_reason"] = reason
        state["current_task"] = None
        save_state(state)
        print(f"{tid} blocked: {reason}. Leave the working tree as it is for the human reviewer.")
        return 0

    if cmd == "approve":
        if len(argv) < 4:
            print("usage: approve TASK_ID NAME")
            return 3
        state["approvals"][argv[2]] = argv[3]
        save_state(state)
        print(f"{argv[2]} approved by {argv[3]}")
        return 0

    print(__doc__)
    return 3


if __name__ == "__main__":
    sys.exit(main(sys.argv))
