#!/usr/bin/env python3
"""End-to-end harness test: play the executor for G0 and F1 in a throwaway clone.

usage: python3 e2e_f1.py CLONE_DIR PHASE   (PHASE: setup | g0finish | red | edit | finish | forge)
Each phase prints what it checked; any unexpected gate result raises SystemExit.
"""
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import tasks_data as D  # noqa: E402

CLONE = Path(sys.argv[1]).resolve()
PHASE = sys.argv[2]
GATE = ["python3", "tools/agent_gate.py"]


def run(args, stdin=None, expect=None):
    p = subprocess.run(args, cwd=CLONE, input=stdin, capture_output=True, text=True)
    out = (p.stdout + p.stderr).strip()
    print(f"$ {' '.join(args)} -> {p.returncode}\n  {out[:700]}")
    if expect is not None and p.returncode != expect:
        raise SystemExit(f"unexpected exit {p.returncode} (expected {expect})")
    return p


def hook(path, expect):
    ev = json.dumps({"hook_event_name": "PreToolUse", "tool_name": "Edit",
                     "tool_input": {"file_path": str(CLONE / path)}})
    run(GATE + ["pre-edit"], stdin=ev, expect=expect)


def line_of(path, sym):
    return next(i for i, line in enumerate((CLONE / path).read_text().splitlines(), 1) if sym in line)


if PHASE == "setup":
    run(GATE + ["validate"], expect=0)
    hook(D.SW, 2)                            # no current task yet
    run(GATE + ["start", "F1"], expect=1)    # G0 not done
    run(GATE + ["start", "G1"], expect=1)    # tier H without approval
    run(GATE + ["start", "G0"], expect=0)
    print("setup ok")

if PHASE == "g0finish":
    st_path = CLONE / ".agent/STATE.json"
    st = json.loads(st_path.read_text())
    st["tasks"]["G0"]["self_audit"].update({
        "restated_goal": "Verify that the design commit builds and the skeleton helpers reproduce the registered counters.",
        "spec_steps_done": ["1", "2", "3", "4", "5"],
        "commands_run": [{"cmd": c["cmd"], "exit_code": 0} for c in D.STD_CMDS],
        "invariants_checked": {"INV-01": "held: skeleton test green", "INV-15": "held: PROGRESS entry written"},
        "no_unrequested_changes": True,
    })
    st_path.write_text(json.dumps(st, indent=2))
    prog = CLONE / D.DOC_DIR / "PROGRESS.md"
    prog.write_text(prog.read_text() + "\n## G0 — baseline — 2026-11-03\n- result: done\n")
    run(GATE + ["check", "G0"], expect=0)
    run(GATE + ["finish", "G0"], expect=0)
    run(["git", "add", "-A"])
    run(["git", "commit", "-qm", "chore(agent): G0 baseline verified at the design commit"], expect=0)
    print("g0 ok")

if PHASE == "red":
    run(GATE + ["start", "F1"], expect=0)
    hook(D.RG, 2)                            # outside F1 scope
    hook(D.DOC_DIR + "/DESIGN.md", 2)        # forbidden path
    hook(D.SW, 0)                            # inside scope
    hook(".agent/STATE.json", 0)             # always allowed
    ct = CLONE / D.CT
    text = ct.read_text()
    i = text.index("// ---- T-F1 ")
    j = text.index("\n", i) + 1
    ct.write_text(text[:j] + "\n" + D.TESTS["F1"].strip("\n") + "\n" + text[j:])
    run(GATE + ["red", "F1"], expect=0)
    print("red ok")

if PHASE == "edit":
    run(GATE + ["stop"], stdin="{}", expect=2)   # self-audit still empty
    for path, old, new, count in D.EDITS["F1"]:
        p = CLONE / path
        t = p.read_text()
        assert t.count(old) == count
        p.write_text(t.replace(old, new))
    sw = CLONE / D.SW
    orig = sw.read_text()
    sw.write_text(orig.replace("    pub fn adaptive_evidence(", "    // dbg!(1);\n    pub fn adaptive_evidence(", 1))
    p = run(GATE + ["check", "F1"], expect=1)
    assert "forbidden pattern" in p.stdout
    sw.write_text(orig)
    st_path = CLONE / ".agent/STATE.json"
    st = json.loads(st_path.read_text())
    st["tasks"]["F1"]["self_audit"].update({
        "restated_goal": "Keep loss-margin backoff out of the adaptive_evidence factory.",
        "symbols_verified": [{"symbol": "pub fn adaptive_evidence_v2(", "file": D.SW, "line": 10}],
        "spec_steps_done": ["tests inserted", "red passed", "edit 1", "existing test edit 1", "commands run"],
        "tests_added": ["f1_adaptive_evidence_factory_has_no_loss_margin_backoff"],
        "commands_run": [{"cmd": c["cmd"], "exit_code": 0} for c in D.STD_CMDS],
        "invariants_checked": {"INV-01": "held: skeleton test green", "INV-07": "held: check passes",
                               "INV-10": "held: red passed", "INV-11": "held: only the listed assertion changed"},
        "no_unrequested_changes": True,
        "open_questions": [],
    })
    st_path.write_text(json.dumps(st, indent=2))
    p = run(GATE + ["stop"], stdin="{}", expect=2)
    assert "hallucinated" in p.stderr
    st["tasks"]["F1"]["self_audit"]["symbols_verified"] = [
        {"symbol": "pub fn adaptive_evidence(", "file": D.SW, "line": line_of(D.SW, "pub fn adaptive_evidence(")},
        {"symbol": "pub fn route_registry(", "file": D.SW, "line": line_of(D.SW, "pub fn route_registry(")},
    ]
    st_path.write_text(json.dumps(st, indent=2))
    run(GATE + ["check", "F1"], expect=0)
    run(GATE + ["stop"], stdin="{}", expect=0)
    print("edit ok")

if PHASE == "finish":
    prog = CLONE / D.DOC_DIR / "PROGRESS.md"
    prog.write_text(prog.read_text() + "\n## F1 — adaptive_evidence — 2026-11-03\n- result: done\n")
    p = run(GATE + ["finish", "F1"])
    print(p.stdout[-3000:])
    st = json.loads((CLONE / ".agent/STATE.json").read_text())
    print(json.dumps(st["tasks"]["F1"].get("mutation"), indent=1))
    print(json.dumps(st["tasks"]["F1"].get("red"), indent=1))

if PHASE == "forge":
    st_path = CLONE / ".agent/STATE.json"
    st = json.loads(st_path.read_text())
    st["tasks"]["F2"]["status"] = "done"     # a hallucinated completion
    st_path.write_text(json.dumps(st, indent=2))
    p = run(GATE + ["status"], expect=0)
    assert "done?NO-RECEIPT" in p.stdout
    st["tasks"]["F2"]["status"] = "todo"
    st_path.write_text(json.dumps(st, indent=2))
    print("forge ok")
