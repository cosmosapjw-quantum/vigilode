#!/usr/bin/env python3
"""Replay all verified edits on the base commit and render TASKS.json and DESIGN.md.

usage: python3 build.py REPO OUT_DIR
  writes OUT_DIR/replay/<repo paths>  (base files after all F/X edits, in task order)
         OUT_DIR/docs/agent/switcher_v2_remediation/{DESIGN.md,TASKS.json,PROGRESS.md}
         OUT_DIR/CLAUDE.md, OUT_DIR/.claude/settings.json
Fails loudly when any anchor count differs during the replay.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import task_prose as P  # noqa: E402
import tasks_data as D  # noqa: E402
import tasks_meta as M  # noqa: E402

REPO = Path(sys.argv[1]).resolve()
OUT = Path(sys.argv[2]).resolve()
XREF = HERE / "xref"
RESULTS = HERE / "dryrun_results.json"
INT_LIB = "crates/rodas5p-integrators/src/lib.rs"
VIG_LIB = "crates/vigilode/src/lib.rs"
RED_CMD = "cargo test --locked -p vigilode --release --test switching_v2_remediation_contracts"


def git_show(path: str) -> str | None:
    out = subprocess.run(["git", "show", f"{D.BASE}:{path}"], cwd=REPO, capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else None


BASE_SW = git_show(D.SW)
a = BASE_SW.index("                                if config.audit_replay_safety && !replay_safety_audited {")
b = BASE_SW.index("                                local.h0 = entry_pilot_checkpoint_seed(", a)
F6_OLD_BLOCK = BASE_SW[a:b]
assert BASE_SW.count(F6_OLD_BLOCK) == 1


def resolve(old):
    return F6_OLD_BLOCK if old == "@@F6_OLD_AUDIT_BLOCK@@" else old


XEDITS = {
    "X4": [("crates/rodas5p-integrators/src/dopri5_stiffness.rs", None, (XREF / "x4_dopri5_stiffness.rs").read_text(), 0),
           (INT_LIB, "mod dopri5_dense;\n", "mod dopri5_dense;\npub mod dopri5_stiffness;\n", 1)],
    "X6": [("crates/vigilode/src/coloring.rs", None, (XREF / "x6_column_coloring.rs").read_text(), 0),
           (VIG_LIB, "mod cofactor_chart;\n", "mod cofactor_chart;\npub mod coloring;\n", 1)],
    "X8": [("crates/vigilode/src/margin.rs", None, (XREF / "x8_margin.rs").read_text(), 0),
           (VIG_LIB, "mod lawson;\n", "mod lawson;\npub mod margin;\n", 1)],
}
ALL_EDITS = {**D.EDITS, **XEDITS}
ORDER = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "X4", "X6", "X8"]
MUTATION_WAIVERS = {
    "F3": {"delete field work_model from struct Self expression in SwitchingConfig::route_registry":
           "equivalent: the struct-update base (adaptive_evidence, then Default) also sets WorkModelVersion::V1; "
           "f3_route_registry_defaults_to_work_model_v1 pins the value"},
}

# ----------------------------------------------------------------- replay
files: dict[str, str | None] = {}


def get(path: str) -> str | None:
    if path not in files:
        # the skeleton test file is part of the design commit, not of the base commit
        skeleton = next(f for f in (HERE / "skeleton_contracts.rs", HERE / "skeleton_contracts.rs.bak") if f.exists())
        files[path] = skeleton.read_text() if path == D.CT else git_show(path)
    return files[path]


computed_anchors: dict[str, list[dict]] = {}
for tid in ORDER:
    anchors = []
    # anchors: the find-texts that exist when the task starts (before its own edits)
    for (path, old, _new, count) in ALL_EDITS[tid]:
        old = resolve(old)
        if old is None:
            continue
        text = get(path)
        if text is not None and text.count(old) == count:
            base_text = git_show(path)
            at = "base" if base_text is not None and base_text.count(old) == count else "task_start"
            anchors.append({"file": path, "text": old, "count": count, "at": at})
    if tid in D.TESTS:
        anchors.append({"file": D.CT, "text": f"// ---- T-{tid} ", "count": 1, "at": "head"})
    computed_anchors[tid] = anchors
    for (path, old, new, count) in ALL_EDITS[tid]:
        old = resolve(old)
        if old is None:
            assert get(path) is None, f"{tid}: {path} already exists"
            files[path] = new
            continue
        text = get(path)
        n = text.count(old)
        assert n == count, f"{tid}: {path}: find-text occurs {n}x, expected {count}: {old[:80]!r}"
        files[path] = text.replace(old, new)
    if tid in D.TESTS:
        ct = get(D.CT)
        marker = f"// ---- T-{tid} "
        assert ct.count(marker) == 1, f"marker for {tid}"
        i = ct.index(marker)
        j = ct.index("\n", i) + 1
        files[D.CT] = ct[:j] + "\n" + D.TESTS[tid].strip("\n") + "\n" + ct[j:]

for path, text in files.items():
    if text is None:
        continue
    dest = OUT / "replay" / path
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text)

# ----------------------------------------------------------------- TASKS.json
cmd_by_name = {c["name"]: c for c in [D.C_CONTRACTS, D.C_LIB, D.C_FULL, D.C_CLIPPY, D.C_FMT, D.C_NODE]}
tasks = []
for meta in M.TASKS:
    t = json.loads(json.dumps(meta))
    tid = t["id"]
    if tid in computed_anchors:
        seen = {(a["file"], a["text"]) for a in computed_anchors[tid]}
        t["anchors"] = computed_anchors[tid] + [a for a in t["anchors"] if (a["file"], a["text"]) not in seen]
    if tid in D.RED:
        r = D.RED[tid]
        t["red"] = {"cmd": RED_CMD, "copy": [D.CT], "tests": r["tests"], "allow_compile_red": r["allow_compile_red"],
                    "new_symbols": r["new_symbols"], "timeout_s": 3600}
    t["removals_allowed"] = D.REMOVALS_ALLOWED.get(tid, [])
    t["mutation_waivers"] = MUTATION_WAIVERS.get(tid, {})
    tasks.append(t)

spec = {
    "schema": "vigilode.agent-tasks.v1",
    "base_commit": D.BASE,
    "design_doc": D.DOC_DIR + "/DESIGN.md",
    "generated": "2026-11-03",
    "notes": "Generated from one source together with DESIGN.md. Agents may not edit this file.",
    "global": {**D.GLOBAL, "invariants": [{"id": i, "text": txt} for i, txt in D.INVARIANTS]},
    "tasks": tasks,
}
doc_dir = OUT / D.DOC_DIR
doc_dir.mkdir(parents=True, exist_ok=True)
(doc_dir / "TASKS.json").write_text(json.dumps(spec, indent=2, ensure_ascii=False) + "\n")


# ----------------------------------------------------------------- DESIGN.md
def lang(path: str) -> str:
    ext = path.rsplit(".", 1)[-1]
    return {"rs": "rust", "py": "python", "json": "json", "md": "markdown", "toml": "toml", "jsonl": "json"}.get(ext, "text")


def fence(code: str, language: str) -> str:
    return f"```{language}\n{code.rstrip(chr(10))}\n```"


def render_edit(k: int, total: int, path: str, old, new: str, count: int) -> str:
    old = resolve(old)
    if old is None:
        return f"**Edit {k} of {total}** — create `{path}` with exactly this content:\n\n{fence(new, lang(path))}\n"
    plural = "s" if count != 1 else ""
    head = f"**Edit {k} of {total}** — `{path}` — find ({count} occurrence{plural}):\n\n{fence(old, lang(path))}\n\n"
    if new == "":
        return head + "replace it with nothing (delete the block, including its trailing newline).\n"
    return head + f"replace with:\n\n{fence(new, lang(path))}\n"


results = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}


def card(t: dict) -> str:
    mut = next((f"package `{a['package']}`, tests `{' '.join(a['cargo_test_args'])}`, max missed {a['max_missed']}"
                for a in t["acceptance"] if a["kind"] == "mutation"), "—")
    rows = [
        ("tier / kind", f"{t['tier']} / {t['kind']}" + (" / spec only" if t["spec_only"] else "")),
        ("depends on", ", ".join(t["depends_on"]) or "—"),
        ("files_allowed", "<br>".join(f"`{f}`" for f in t["files_allowed"]) or "— (PROGRESS.md only)"),
        ("invariants", ", ".join(t["invariants"])),
        ("tests_required", "<br>".join(f"`{x['name']}` ({x['file'].split('/')[-1]})" for x in t["tests_required"]) or "—"),
        ("red on base", "<br>".join(f"`{n}`" for n in t["red"]["tests"]) or "— (no red check)"),
        ("commands", ", ".join(c["name"] for c in t["commands"]) or "—"),
        ("mutation gate", mut),
        ("pre-reviewed equivalent mutants", "<br>".join(f"`{k}`: {v}" for k, v in t["mutation_waivers"].items()) or "—"),
        ("ledger row", "required" if t["ledger_required"] else "no"),
        ("commit message", f"`{t['commit_message']}`"),
    ]
    return "| field | value |\n|---|---|\n" + "\n".join(f"| {k} | {v} |" for k, v in rows)


def task_section(t: dict) -> str:
    tid = t["id"]
    parts = [f'<a id="{t["design_section"]}"></a>', f"### {tid} — {t['title']}", "", card(t), ""]
    parts += [f"**Why.** {P.WHY[tid]}", ""]
    if tid in P.SPEC:
        script = (HERE / "g1" / "agent_reproduce_e1_prereg.py").read_text().rstrip("\n")
        parts += [P.SPEC[tid].replace("{{G1_SCRIPT}}", script).rstrip("\n"), ""]
    edits = ALL_EDITS.get(tid, [])
    if tid in D.TESTS:
        parts += [f"**Step 1 — tests.** Insert after the marker line `// ---- T-{tid} ----…` in `CT`:", "",
                  fence(D.TESTS[tid].strip("\n"), "rust"), ""]
        if tid in P.RED_REASON:
            parts += [f"**Red on base** (`python3 tools/agent_gate.py red {tid}`): {P.RED_REASON[tid]}", ""]
    main_edits = [e for e in edits if not ("/tests/" in e[0] and e[0] != D.CT)]
    test_edits = [e for e in edits if "/tests/" in e[0] and e[0] != D.CT]
    if main_edits:
        step = "Step 2 — edits" if tid in D.TESTS else "Edits"
        parts += [f"**{step}** (apply in this order):", ""]
        for k, (path, old, new, count) in enumerate(main_edits, 1):
            parts += [render_edit(k, len(main_edits), path, old, new, count)]
        if tid in ("X4", "X6", "X8"):
            parts += ["The module's unit tests are part of the created file and are the task's tests. A new module "
                      "has no red check (nothing exists that could fail); the mutation gate takes its place.", ""]
    if test_edits:
        parts += ["**Existing tests that change** (only these; R-06):", ""]
        for k, (path, old, new, count) in enumerate(test_edits, 1):
            parts += [render_edit(k, len(test_edits), path, old, new, count)]
    if tid in P.NOTES:
        parts += ["**Notes and pitfalls.**", ""] + [f"- {n}" for n in P.NOTES[tid]] + [""]
    extra = [e for e in t["escalate_if"] if e not in M.COMMON_ESCALATE]
    if extra:
        parts += ["**Task-specific escalation.**", ""] + [f"- {e}" for e in extra] + [""]
    return "\n".join(parts)


def dag() -> str:
    lines = ["flowchart TD"]
    for t in tasks:
        lines.append(f'    {t["id"]}["{t["id"]} · {t["tier"]}"]')
    for t in tasks:
        for d in t["depends_on"]:
            lines.append(f"    {d} --> {t['id']}")
    lines += ["    classDef S fill:#e6f4ea,stroke:#2e7d32", "    classDef M fill:#e8f0fe,stroke:#1a73e8",
              "    classDef L fill:#fff4e5,stroke:#e8710a", "    classDef H fill:#fce8e6,stroke:#c5221f"]
    for tier in "SMLH":
        ids = [t["id"] for t in tasks if t["tier"] == tier]
        if ids:
            lines.append(f"    class {','.join(ids)} {tier}")
    return "\n".join(lines)


def task_table() -> str:
    rows = ["| id | title | tier | kind | depends on | code in this document |", "|---|---|---|---|---|---|"]
    for t in tasks:
        verified = "spec only" if t["spec_only"] else ("replayed and tested" if t["id"] in ORDER else "procedure")
        rows.append(f"| [{t['id']}](#{t['design_section']}) | {t['title']} | {t['tier']} | {t['kind']} | "
                    f"{', '.join(t['depends_on']) or '—'} | {verified} |")
    return "\n".join(rows)


def counters_table() -> str:
    return "\n".join([
        "| factory | base `4339facc`: rotation | base: stiff-forced | after F1–F9: rotation | after F1–F9: stiff-forced |",
        "|---|---|---|---|---|",
        "| `evidence_driven` | (892, 5371, 355) | (66, 456) | (892, 5371, 355) | (66, 456) |",
        "| `adaptive_evidence` | (882, 5299, 119) | (58, 394) | (892, 5371, 355) | (58, 394) |",
        "| `route_registry` | (925, 5641, 1509) | (319, 1968) | (883, 5307, 149) | (59, 400) |",
        "| `route_registry` with the rejected B7 `min` variant (not applied) | — | — | — | (298, 1840) |",
        "",
        "Final states (bit patterns of `y(1)`) of `evidence_driven` are identical before and after. F7 alone changes no "
        "counter or state of any factory on either model.",
    ])


def dryrun_notes() -> str:
    if not results:
        return "_Dry-run results not yet recorded._"
    lines = ["Verification of the replayed tree:", ""] + [f"- {x}" for x in results.get("checks", [])]
    if results.get("existing_test_updates"):
        lines += ["", "Existing tests that the remediation changes (each appears in its task under \"Existing tests that change\"):", ""]
        lines += [f"- {x}" for x in results["existing_test_updates"]]
    if results.get("mutation"):
        lines += ["", "Mutation runs:", ""] + [f"- {x}" for x in results["mutation"]]
    if results.get("e2e"):
        lines += ["", "End-to-end run of the harness (gate, hooks, red check, mutation gate):", ""]
        lines += [f"- {x}" for x in results["e2e"]]
    return "\n".join(lines)


tpl = (HERE / "design_template.md").read_text()
inv_table = "| id | invariant |\n|---|---|\n" + "\n".join(f"| {i} | {txt} |" for i, txt in D.INVARIANTS)
repl = {
    "{{BASE}}": D.BASE, "{{BASE_SHORT}}": D.BASE[:8],
    "{{INVARIANTS_TABLE}}": inv_table, "{{DAG_MERMAID}}": dag(), "{{TASK_TABLE}}": task_table(),
    "{{COUNTERS_TABLE}}": counters_table(), "{{DRYRUN_NOTES}}": dryrun_notes(),
}
for name, c in cmd_by_name.items():
    repl["{{CMD:" + name + "}}"] = c["cmd"]
for k, v in repl.items():
    tpl = tpl.replace(k, v)
tpl = tpl.replace("{{TASK_SECTIONS}}", "\n".join(task_section(t) for t in tasks))
leftover = [line for line in tpl.splitlines() if "{{" in line and "}}" in line and "{{CMD" in line or "{{TASK" in line]
assert not leftover, leftover[:5]
(doc_dir / "DESIGN.md").write_text(tpl)

(doc_dir / "PROGRESS.md").write_text(
    "# Progress log — switcher v2 remediation\n\n"
    "Append-only (the gate rejects edits to earlier lines). One entry per task, format in DESIGN.md section 13.1.\n"
)

CLAUDE_MD = """# VigilODE — instructions for coding agents

Rust workspace of ODE integrators (`crates/rodas5p-*`) and the facade crate `crates/vigilode`.

## Current work: switcher v2 remediation
- Tasks: `docs/agent/switcher_v2_remediation/TASKS.json`. Design: `DESIGN.md` in the same folder.
- Start every session with `python3 tools/agent_gate.py status` and `python3 tools/agent_gate.py next`,
  then read only the DESIGN.md section that `next` names. One task per session.
- Hooks in `.claude/settings.json` run `tools/agent_gate.py`. A `BLOCKED` message is a rule:
  do not work around it; run `python3 tools/agent_gate.py abort <ID> "<reason>"` if you cannot proceed.

## Commands
- One test file: `cargo test --locked -p vigilode --release --test <file_stem>`
- One test: append `-- <test_name> --exact`
- vigilode suite (10–20 min): `cargo test --locked -p vigilode --release --no-fail-fast`
- Lint: `cargo clippy --locked -p vigilode --all-targets --release --no-deps -- -D warnings`
- Format check: `cargo fmt -p vigilode -- --check`
- The toolchain is pinned (1.94.1). If it cannot be installed, prefix cargo and `agent_gate.py finish`
  with `RUSTUP_TOOLCHAIN=stable` and record that in PROGRESS.md.

## Facts the code does not tell you
- `research/LEDGER.jsonl` is append-only and carries authority; rows only via the script in DESIGN.md 13.2.
- WorkModel V2 costs are instruction units (Ir); guard ceilings are callback units.
- Without `--no-fail-fast`, cargo stops at the first failing test binary and hides later failures.
- `.agent/STATE.json` is the gate's state: edit only your task's `self_audit` block, never `status`.
"""
(OUT / "CLAUDE.md").write_text(CLAUDE_MD)

settings = {
    "permissions": {
        "deny": [
            "Bash(git push *)", "Bash(git reset --hard *)", "Bash(git clean *)", "Bash(git rebase *)",
            "Bash(git commit --amend *)", "Bash(python3 tools/agent_gate.py approve *)", "Bash(* agent_gate.py approve *)",
            "Edit(/docs/agent/switcher_v2_remediation/DESIGN.md)", "Edit(/docs/agent/switcher_v2_remediation/TASKS.json)",
            "Edit(/tools/agent_gate.py)", "Edit(/CLAUDE.md)", "Edit(/.claude/**)", "Edit(/.github/**)",
            "Edit(/Cargo.lock)", "Edit(/rust-toolchain.toml)",
        ]
    },
    "hooks": {
        "PreToolUse": [{
            "matcher": "Edit|Write|MultiEdit|NotebookEdit",
            "hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/tools/agent_gate.py\" pre-edit",
                       "timeout": 30, "onFailure": "block"}],
        }],
        "Stop": [{
            "hooks": [{"type": "command", "command": "python3 \"$CLAUDE_PROJECT_DIR/tools/agent_gate.py\" stop",
                       "timeout": 120}],
        }],
    },
}
(OUT / ".claude").mkdir(exist_ok=True)
(OUT / ".claude" / "settings.json").write_text(json.dumps(settings, indent=2) + "\n")
print(f"replayed {len(ORDER)} tasks; {len(tasks)} task cards; DESIGN.md {len(tpl.splitlines())} lines")

SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "vigilode.agent-tasks.v1",
    "title": "VigilODE agent task file",
    "type": "object",
    "required": ["schema", "base_commit", "design_doc", "global", "tasks"],
    "properties": {
        "schema": {"const": "vigilode.agent-tasks.v1"},
        "base_commit": {"type": "string", "pattern": "^[0-9a-f]{40}$"},
        "design_doc": {"type": "string"},
        "global": {
            "type": "object",
            "required": ["forbidden_paths", "always_allowed", "append_only", "forbidden_added_patterns",
                         "forbidden_removed_patterns", "invariants"],
            "properties": {
                "forbidden_paths": {"type": "array", "items": {"type": "string"}},
                "always_allowed": {"type": "array", "items": {"type": "string"}},
                "append_only": {"type": "array", "items": {"type": "string"}},
                "forbidden_added_patterns": {"type": "array", "items": {"$ref": "#/$defs/pattern"}},
                "forbidden_removed_patterns": {"type": "array", "items": {"$ref": "#/$defs/pattern"}},
                "invariants": {"type": "array", "items": {"type": "object", "required": ["id", "text"],
                               "properties": {"id": {"type": "string", "pattern": "^INV-[0-9]{2}$"}, "text": {"type": "string"}}}},
            },
        },
        "tasks": {"type": "array", "items": {"$ref": "#/$defs/task"}},
    },
    "$defs": {
        "pattern": {"type": "object", "required": ["regex", "paths", "reason"],
                    "properties": {"regex": {"type": "string"}, "paths": {"type": "array", "items": {"type": "string"}},
                                   "reason": {"type": "string"}}},
        "command": {"type": "object", "required": ["name", "cmd", "timeout_s"],
                    "properties": {"name": {"type": "string"}, "cmd": {"type": "string"}, "timeout_s": {"type": "integer", "minimum": 1}}},
        "anchor": {"type": "object", "required": ["file", "text", "count"],
                   "properties": {"file": {"type": "string"}, "text": {"type": "string", "minLength": 1},
                                  "count": {"type": "integer", "minimum": 1},
                                  "at": {"enum": ["base", "head", "task_start"]}}},
        "task": {
            "type": "object",
            "required": ["id", "title", "phase", "tier", "kind", "depends_on", "files_allowed", "anchors", "design_section",
                         "invariants", "tests_required", "red", "commands", "acceptance", "ledger_required", "escalate_if",
                         "commit_message", "removals_allowed", "spec_only", "mutation_waivers"],
            "properties": {
                "id": {"type": "string", "pattern": "^[GFX][0-9]{1,2}$"},
                "tier": {"enum": ["S", "M", "L", "H"]},
                "kind": {"enum": ["governance", "bitwise_safe", "opt_in_behaviour", "default_behaviour", "experiment", "docs"]},
                "depends_on": {"type": "array", "items": {"type": "string"}},
                "files_allowed": {"type": "array", "items": {"type": "string"}},
                "anchors": {"type": "array", "items": {"$ref": "#/$defs/anchor"}},
                "design_section": {"type": "string", "pattern": "^t-[a-z0-9]+$"},
                "invariants": {"type": "array", "items": {"type": "string", "pattern": "^INV-[0-9]{2}$"}},
                "tests_required": {"type": "array", "items": {"type": "object", "required": ["file", "name"]}},
                "red": {"type": "object", "required": ["tests", "allow_compile_red", "new_symbols"]},
                "commands": {"type": "array", "items": {"$ref": "#/$defs/command"}},
                "acceptance": {"type": "array", "items": {"type": "object", "required": ["kind"],
                               "properties": {"kind": {"enum": ["grep_present", "grep_absent", "test_name_present", "mutation"]}}}},
                "ledger_required": {"type": "boolean"},
                "spec_only": {"type": "boolean"},
                "mutation_waivers": {"type": "object", "additionalProperties": {"type": "string"}},
                "removals_allowed": {"type": "array", "items": {"type": "object", "required": ["file", "line"]}},
            },
        },
    },
}
(doc_dir / "TASKS.schema.json").write_text(json.dumps(SCHEMA, indent=2) + "\n")
