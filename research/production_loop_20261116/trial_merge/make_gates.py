#!/usr/bin/env python3
"""Loop L1 node V02 (trial merge): assemble T_GATES.json, T3_GOLDEN_DIFF.json and
MERGE_FACTS.json from the raw logs and from git.  Nothing is typed in: exit codes and
counts are read from the logs written by run_gate.sh, the rest from git and the tree.

Usage: python3 -I make_gates.py <base> <merge_commit> <head>
"""

from __future__ import annotations

import collections
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
LOGS = HERE / "logs"
REL = "research/production_loop_20261116/trial_merge"


# Readings and decisions for the owner. Text only; every number cited lives in the JSON
# files named in "evidence".
REVIEW_ITEMS = [
    {
        "id": "R1",
        "topic": "T2 sentinel tests pin protected sources",
        "text": (
            "g3_protected_sources_are_byte_identical (ppsw1_6b_contracts.rs) and "
            "h2_protected_sources_are_byte_identical (ptsw2b_contracts.rs) pin the SHA-256 of "
            "protected sources and stop at the first mismatch. The merge changes four pinned files "
            "of crates/rodas5p-integrators/src: sequential.rs, rodas5p_matrix_free_fast.rs, "
            "problem.rs, adaptive.rs. The tests cannot pass with the switcher hunks in place: "
            "either those hunks are dropped or the owner re-pins."
        ),
        "evidence": ["SENTINEL_PINS.json", "logs/t2_b_integrators_peer.log"],
    },
    {
        "id": "R2",
        "topic": "g3 lib.rs rule already broken on the loop base",
        "text": (
            "g3 also requires src/lib.rs minus the single line `pub mod ppsw1;` to hash to a pinned "
            "value. The rule holds at the baseline commit 074a5699 and not at the loop base 2dc9a2c5 "
            "(bootstrap added `pub mod mol;` and `pub mod stage_clients;`), so g3 is expected to "
            "fail on the loop branch without any merge. Computed by check_sentinel_pins.py; the "
            "test was not run at 2dc9a2c5."
        ),
        "evidence": ["SENTINEL_PINS.json"],
    },
    {
        "id": "R3",
        "topic": "T3 strict identity fails through added record fields only",
        "text": (
            "Only production_automatic differs: values changed 0, removed 0, keys added 600. "
            "Cause: ten #[serde(default)] fields appended to struct SwitchingConfig in "
            "crates/vigilode/src/switching.rs. The hunk cannot be excluded: the fields are read in "
            "the switcher source and set in the switcher tests. A serialisation-only alternative "
            "(skip_serializing_if at the default value) would restore strict identity; it is "
            "outside fix-up classes (a)-(d) and was not tried. With "
            "VIGILODE_DEFAULT_PATH_ALLOW_ADDED_KEYS=1 the oracle passes. Never blessed."
        ),
        "evidence": ["T3_GOLDEN_DIFF.json", "logs/t3_golden_strict_run1.log", "logs/t3_golden_allow_added_keys.log"],
    },
    {
        "id": "R4",
        "topic": "Fix-up (c) semantics, decision needs review",
        "text": (
            "Jacobian::SparseJvp gets the Jvp permissions (provided_jvp = supplied && "
            "allow_matrix_free) as instructed. The facade runs a banded SparseJvp through coloured "
            "assembly and a banded factorisation, not matrix-free, so allow_matrix_free = false "
            "withholds a route that never runs matrix-free and a banded SparseJvp is not counted "
            "as explicit_jacobian. Splitting on pattern.is_banded() may fit better."
        ),
        "evidence": ["logs/t1_control_without_fixup_c.log"],
    },
    {
        "id": "R5",
        "topic": "Merged hooks block edits in a fresh worktree",
        "text": (
            "The merge adds .claude/settings.json, a CLAUDE.md section and tools/agent_gate.py. "
            "The gate state .agent/STATE.json is git-ignored and does not travel with the merge, "
            "so in a fresh worktree there is no current task and the PreToolUse hook answers "
            "BLOCKED for every Edit/Write inside the repository except .agent/* and "
            "docs/agent/switcher_v2_remediation/PROGRESS.md. Read from the script; the hook was "
            "not run."
        ),
        "evidence": ["MERGE_FACTS.json#merged_claude_settings"],
    },
    {
        "id": "R6",
        "topic": "adaptive.rs gains an opt-in controller",
        "text": (
            "The merge adds ControllerKind::H211b and the state field previous_step_ratio to "
            "crates/rodas5p-integrators/src/adaptive.rs. Existing controller arms are unchanged "
            "and the golden oracle shows no value change, but the file holds controller code that "
            "the loop treats as immutable, so the hunk needs an owner ruling of its own."
        ),
        "evidence": ["BLOB_DELTA.json"],
    },
    {
        "id": "R7",
        "topic": "Four evaluation scripts execute no solver",
        "text": (
            "eval_k1.py (L-0209), eval_c1.py (L-0214, the one PASS), eval_b7.py (L-0210) and "
            "eval_d1.py (L-0213) import no subprocess, call no binary and read no input file. "
            "K1 and C1 are closed-form arithmetic on hard-coded step tables; B7 and D1 are "
            "literal tables. No ledger row was appended by this node."
        ),
        "evidence": ["MERGE_FACTS.json#eval_scripts"],
    },
    {
        "id": "R8",
        "topic": "Clippy baseline",
        "text": (
            "The 8 baseline lints of source_owned_graph_schur.rs are gone because the switcher "
            "line puts #[expect(clippy::neg_cmp_op_on_partial_ord, clippy::needless_range_loop)] "
            "on the module declaration in lib.rs; the file itself is unchanged. The loop "
            "baseline's clippy output was not re-run here; the figure 8 is the one given in the task."
        ),
        "evidence": ["logs/t5_clippy_integrators_lib_final_head.log"],
    },
    {
        "id": "R9",
        "topic": "Limits",
        "text": (
            "Out-of-workspace crates vigilode-bench and vigilode-hybrid (rust-version 1.99.0) were "
            "not built. Only the named integrators test targets were run. The oracle leaves opt-in "
            "derivative and switching routes out by design. REPORT.md was not written: the session "
            "harness refuses report .md files from subagents; its content is carried by these items."
        ),
        "evidence": [],
    },
]


def git(*args: str) -> str:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {out.stderr.strip()}")
    return out.stdout


def dump(name: str, obj: dict) -> None:
    (HERE / name).write_text(json.dumps(obj, indent=2, allow_nan=False) + "\n", encoding="utf-8")


def log_run(name: str) -> dict:
    path = LOGS / name
    header = {}
    text = path.read_text(encoding="utf-8", errors="replace")
    for line in text.splitlines():
        if line.startswith("# "):
            key, _, value = line[2:].partition(": ")
            header[key] = value
    return {
        "log": f"{REL}/logs/{name}",
        "log_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "command": header.get("cmd"),
        "head": header.get("head"),
        "start": header.get("start"),
        "end": header.get("end"),
        "exit_code": int(header["exit_code"]),
        "compiler_errors": len(re.findall(r"^error(\[E\d+\])?:", text, flags=re.M)),
        "compiler_warnings": len(re.findall(r"^warning: (?!build failed)", text, flags=re.M)),
    }


def status(runs: list[dict]) -> str:
    return "PASS" if all(r["exit_code"] == 0 for r in runs) else "FAIL"


def golden_diff(full_report_log: str) -> dict:
    text = (LOGS / full_report_log).read_text(encoding="utf-8")
    counts = {k: int(v) for k, v in re.findall(r"^(changed|removed|added): (\d+)$", text, flags=re.M)}
    tests = dict(re.findall(r"^test (\w+) \.\.\. (ok|FAILED)$", text, flags=re.M))
    paths = [line.strip() for line in text.splitlines() if line.startswith("  /cases/")]
    golden = json.loads(
        (ROOT / "crates/vigilode/tests/golden/default_path/production_automatic.json").read_text()
    )
    case_ids = sorted(golden["cases"], key=len, reverse=True)
    per_case: dict[str, list[str]] = collections.OrderedDict()
    for p in paths:
        rest = p[len("/cases/") :]
        cid = next(c for c in case_ids if rest.startswith(c + "/"))
        per_case.setdefault(cid, []).append(rest[len(cid) + 1 :])
    leaf = collections.Counter(p.rsplit("/", 1)[-1] for p in paths)
    parent = collections.Counter(
        next(f for c, fs in per_case.items() for f in fs if p.endswith(f)).rsplit("/", 1)[0]
        for p in paths
    )
    return {
        "schema": "vigilode.loop-l1.trial-merge.golden-diff.v1",
        "generator": f"{REL}/make_gates.py",
        "source_log": f"{REL}/logs/{full_report_log}",
        "note": (
            "The source log is a run of an untracked copy of default_path_golden.rs whose only "
            "change is REPORTED_PATHS = usize::MAX (the oracle prints 40 paths per kind); the "
            "copy was moved out of the tree after the run and is not committed."
        ),
        "golden_files": 9,
        "oracle_tests": tests,
        "differing_golden_files": sorted(k for k, v in tests.items() if v == "FAILED"),
        "values_changed": counts.get("changed"),
        "values_removed": counts.get("removed"),
        "keys_added": counts.get("added"),
        "listed_added_paths": len(paths),
        "cases_in_differing_file": len(golden["cases"]),
        "differing_cases": len(per_case),
        "added_key_names": dict(sorted(leaf.items())),
        "added_key_parents": dict(sorted(parent.items())),
        "cases": [
            {
                "case": cid,
                "kind": "added_keys_only",
                "added_keys": len(fields),
                "first_differing_field": fields[0],
            }
            for cid, fields in per_case.items()
        ],
    }


def eval_scripts() -> list[dict]:
    rows = []
    ledger = [json.loads(line) for line in (ROOT / "research/LEDGER.jsonl").read_text().splitlines()]
    for rel in (
        "research/k1_shared_krylov_basis_20261104/eval_k1.py",
        "research/c1_coloured_assembly_20261104/eval_c1.py",
        "research/b7_first_batch_20261104/eval_b7.py",
        "research/d1_stiffness_gate_20261104/eval_d1.py",
    ):
        src = (ROOT / rel).read_text(encoding="utf-8")
        node = rel.split("/")[1]
        markers = {
            "imports_subprocess": bool(re.search(r"^\s*(import|from) subprocess", src, flags=re.M)),
            "calls_os_system_or_popen": bool(re.search(r"os\.(system|popen|exec)", src)),
            "mentions_cargo_or_binary": bool(re.search(r"cargo|target/|\.so\b|ctypes|valgrind", src)),
            "reads_input_file": bool(
                re.search(r"read_text|read_bytes|json\.load\(|csv\.reader", src)
                or any('"w"' not in call for call in re.findall(r"\bopen\(([^)]*)\)", src))
            ),
        }
        rows.append(
            {
                "script": rel,
                "sha256": hashlib.sha256(src.encode("utf-8")).hexdigest(),
                "lines": src.count("\n"),
                **markers,
                "executes_a_solver": any(
                    markers[k]
                    for k in ("imports_subprocess", "calls_os_system_or_popen", "mentions_cargo_or_binary")
                ),
                "numeric_literals_in_source": len(re.findall(r"(?<![\w.])\d+(?:\.\d+)?(?:e-?\d+)?(?![\w.])", src)),
                "ledger_rows_covering_node": [
                    {"id": r["id"], "verdict": r["verdict"]}
                    for r in ledger
                    if any(p.startswith(f"research/{node}/") for p in r.get("outputs_sha256", {}))
                ],
            }
        )
    return rows


def main() -> int:
    base, merge, head = (git("rev-parse", r).strip() for r in sys.argv[1:4])

    t1 = [log_run("t1_check_attempt1.log"), log_run("t1_check_attempt2.log"), log_run("t1_check_final_head.log")]
    t2_results = json.loads((HERE / "T2_RESULTS.json").read_text())
    t2_runs = []
    for s in t2_results["suites"]:
        t2_runs.append(
            {
                "log": s["log"],
                "command": s["command"],
                "head": s["head"],
                "exit_code": s["exit_code"],
                "test_binaries": s["test_binaries"],
                "passed": s["passed"],
                "failed": s["failed"],
                "ignored": s["ignored"],
                "failed_tests": s["failed_tests"],
            }
        )
    t3_strict = [log_run("t3_golden_strict_run1.log"), log_run("t3_golden_strict_run2.log")]
    t3_additive = log_run("t3_golden_allow_added_keys.log")
    t3 = golden_diff("t3_golden_strict_full_report.log")
    dump("T3_GOLDEN_DIFF.json", t3)
    t5 = [
        log_run("t5_clippy_facade_core_attempt1.log"),
        log_run("t5_clippy_integrators_lib_attempt1.log"),
        log_run("t5_clippy_facade_core_final_head.log"),
        log_run("t5_clippy_integrators_lib_final_head.log"),
    ]
    t5_extra = log_run("t5_clippy_integrators_lib_testcfg.log")
    t6 = log_run("t6_check_research_node.log")
    t6_text = (LOGS / "t6_check_research_node.log").read_text()

    gates = {
        "schema": "vigilode.loop-l1.trial-merge.gates.v1",
        "generator": f"{REL}/make_gates.py",
        "node": "V02",
        "base_commit": base,
        "merged_commit": git("rev-parse", f"{merge}^2").strip(),
        "merge_commit": merge,
        "evaluated_head": head,
        "toolchain": "1.94.1",
        "wall_time": "diagnostic_only",
        "gates": {
            "T1": {
                "status": "PASS" if t1[-1]["exit_code"] == 0 and t1[1]["exit_code"] == 0 else "FAIL",
                "command": t1[-1]["command"],
                "log": t1[-1]["log"],
                "runs": t1,
                "note": "attempt1 = after fix-up (a) only (fails on try_update); attempt2 = after fix-ups (a)-(c); final_head = after the oracle cherry-pick.",
            },
            "T2": {
                "status": status(t2_runs),
                "command": [r["command"] for r in t2_runs],
                "log": [r["log"] for r in t2_runs],
                "passed": sum(r["passed"] for r in t2_runs),
                "failed": sum(r["failed"] for r in t2_runs),
                "ignored": sum(r["ignored"] for r in t2_runs),
                "failed_tests": [f for r in t2_runs for f in r["failed_tests"]],
                "runs": t2_runs,
            },
            "T3": {
                "status": status(t3_strict),
                "command": t3_strict[0]["command"],
                "log": t3_strict[0]["log"],
                "blessed": False,
                "strict_runs": t3_strict,
                "strict_byte_identity": all(r["exit_code"] == 0 for r in t3_strict),
                "differing_golden_files": t3["differing_golden_files"],
                "identical_golden_files": sorted(
                    k for k, v in t3["oracle_tests"].items()
                    if v == "ok" and k not in ("format_helpers_are_bit_faithful", "golden_directory_is_exactly_the_listed_files")
                ),
                "values_changed": t3["values_changed"],
                "values_removed": t3["values_removed"],
                "keys_added": t3["keys_added"],
                "differing_cases": t3["differing_cases"],
                "allow_added_keys_run": {**t3_additive, "status": "PASS" if t3_additive["exit_code"] == 0 else "FAIL"},
                "details": f"{REL}/T3_GOLDEN_DIFF.json",
            },
            "T5": {
                "status": status(t5),
                "command": [t5[2]["command"], t5[3]["command"]],
                "log": [t5[2]["log"], t5[3]["log"]],
                "warnings": sum(r["compiler_warnings"] for r in t5[2:]),
                "errors": sum(r["compiler_errors"] for r in t5[2:]),
                "runs": t5,
                "extra_test_cfg_run": t5_extra,
            },
            "T6": {
                "status": "PASS" if t6["exit_code"] == 0 else "FAIL",
                "command": t6["command"],
                "log": t6["log"],
                "exit_code": t6["exit_code"],
                "failures": re.findall(r"^FAIL: (.+)$", t6_text, flags=re.M),
                "summary_line": next(
                    (line for line in t6_text.splitlines() if line.startswith("check-research-node:")), None
                ),
            },
        },
    }
    dump("T_GATES.json", gates)

    # Facts read from git and the tree.
    ledger_ids = [json.loads(line)["id"] for line in (ROOT / "research/LEDGER.jsonl").read_text().splitlines()]
    base_ids = [json.loads(line)["id"] for line in git("show", f"{base}:research/LEDGER.jsonl").splitlines()]
    rust_versions = {}
    for manifest in git("ls-files", "*Cargo.toml").split():
        m = re.search(r"^rust-version(?:\.workspace)?\s*=\s*(.+)$", (ROOT / manifest).read_text(), flags=re.M)
        rust_versions[manifest] = m.group(1).strip().strip('"') if m else None
    members = re.findall(r'"(crates/[^"]+)"', (ROOT / "Cargo.toml").read_text().split("[workspace.package]")[0])
    settings = json.loads((ROOT / ".claude/settings.json").read_text())
    name_status = collections.Counter(
        line.split("\t", 1)[0][0] for line in git("diff", "--name-status", base, merge).splitlines()
    )
    added_tests = sorted(
        line.split("\t", 1)[1]
        for line in git("diff", "--name-status", "--diff-filter=A", base, head).splitlines()
        if re.match(r"crates/[^/]+/tests/[^/]+\.rs$", line.split("\t", 1)[1])
    )
    facts = {
        "schema": "vigilode.loop-l1.trial-merge.facts.v1",
        "generator": f"{REL}/make_gates.py",
        "commits": [
            {"commit": c.split(" ", 1)[0], "subject": c.split(" ", 1)[1]}
            for c in git("log", "--format=%H %s", f"{base}..{head}", "--first-parent").splitlines()
        ],
        "merge_name_status_counts": dict(name_status),
        "cargo_lock_changed_by_merge": bool(git("diff", "--name-only", base, merge, "--", "Cargo.lock").strip()),
        "workspace_members": members,
        "rust_version_declarations_at_head": rust_versions,
        "out_of_workspace_manifests_not_1_94_1": {
            k: v
            for k, v in rust_versions.items()
            if v not in (None, "true", "1.94.1") and k != "Cargo.toml"
            and k.rsplit("/", 1)[0] not in members
        },
        "ledger": {
            "rows_at_base": len(base_ids),
            "rows_after_merge": len(ledger_ids),
            "base_is_prefix": ledger_ids[: len(base_ids)] == base_ids,
            "appended_ids": ledger_ids[len(base_ids) :],
            "duplicate_ids": sorted(k for k, v in collections.Counter(ledger_ids).items() if v > 1),
            "ids_in_file_order_tail": ledger_ids[-14:],
        },
        "test_targets_added_by_merge": added_tests,
        "eval_scripts": eval_scripts(),
        "switching_config_added_fields": [
            {"line": i + 1, "field": m.group(1)}
            for i, line in enumerate((ROOT / "crates/vigilode/src/switching.rs").read_text().splitlines())
            for m in [re.match(r"\s+pub (\w+): ", line)]
            if m and m.group(1) in t3["added_key_names"]
        ],
        "review_items": REVIEW_ITEMS,
        "merged_claude_settings": {
            "path": ".claude/settings.json",
            "added_by_merge": not bool(
                subprocess.run(
                    ["git", "cat-file", "-e", f"{base}:.claude/settings.json"], cwd=ROOT, capture_output=True
                ).returncode
                == 0
            ),
            "permissions_deny": settings.get("permissions", {}).get("deny", []),
            "hooks": {
                event: [
                    {"matcher": entry.get("matcher"), "commands": [h.get("command") for h in entry.get("hooks", [])]}
                    for entry in entries
                ]
                for event, entries in settings.get("hooks", {}).items()
            },
        },
    }
    dump("MERGE_FACTS.json", facts)
    for name, gate in gates["gates"].items():
        print(name, gate["status"])
    print("T3 changed/removed/added:", t3["values_changed"], t3["values_removed"], t3["keys_added"],
          "cases", t3["differing_cases"], "/", t3["cases_in_differing_file"])
    print("ledger appended:", facts["ledger"]["appended_ids"])
    for e in facts["eval_scripts"]:
        print(e["script"], "executes_a_solver=", e["executes_a_solver"], e["ledger_rows_covering_node"])
    print("out-of-workspace rust-version:", facts["out_of_workspace_manifests_not_1_94_1"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
