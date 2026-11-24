#!/usr/bin/env python3
"""Validate the kernel/spectral foundation run: record and plan structure, evidence paths,
the foundation checks, the golden label diff of SL-N12, the feature-gate record of SL-N6 and
the timing record of SL-N5. Exit status 0 when everything holds.

Usage (repository root): python3 -I research/kernel_spectral_foundation_20261124/scripts/verify_run.py
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parents[1]
ROOT = HERE.parents[1]


def load(path):
    return json.loads((HERE / path).read_text())


def main():
    problems = []

    def check(condition, message):
        if not condition:
            problems.append(message)

    run = load("RUN.json")
    for key in ("schema_version", "task", "status", "base", "environment", "stages", "claims", "decisions", "failures"):
        check(key in run, f"RUN.json lacks {key}")
    ids = [s["id"] for s in run["stages"]]
    check(len(ids) == len(set(ids)), "duplicate stage ids")
    for stage in run["stages"]:
        for ref in stage.get("evidence", []):
            path = ref.split("::")[0]
            check((ROOT / path).exists(), f"{stage['id']}: missing evidence {path}")
    for claim in run["claims"]:
        check(claim["status"] in set("DIATNPCO"), f"claim {claim['id']}: status {claim['status']}")

    plan = load("PLAN.json")
    task_ids = {t["id"] for t in plan["tasks"]}
    for task in plan["tasks"]:
        for key in ("id", "title", "depends_on", "owner_decision", "files", "question", "implementation", "acceptance", "kill", "fallback", "commands"):
            check(key in task, f"{task.get('id')}: lacks {key}")
        check(set(task["depends_on"]) <= task_ids, f"{task['id']}: unknown dependency")
    for decision in plan["decisions_applied"].values():
        for path in decision["evidence"]:
            check((ROOT / path.split("::")[0]).exists(), f"plan evidence missing: {path}")

    foundation = subprocess.run(
        [sys.executable, "-I", str(HERE / "scripts" / "verify_foundation.py"), "/tmp/ks_foundation_check"],
        capture_output=True,
        text=True,
    )
    check(foundation.returncode == 0, "verify_foundation.py failed:\n" + foundation.stdout + foundation.stderr)

    labels = load("results/SL_N12_GOLDEN_LABEL_DIFF.json")
    for name, entry in labels.items():
        check(set(entry["by_key"]) <= {"entry_point", "driver_id"}, f"{name}: a non-label value changed")

    gate = load("results/SL_N6_FEATURE_GATE.json")
    gated = gate["rodas5p-integrators"]["research_gates_modules"]
    for module in gate["reclassified_to_default"]:
        check(module not in gated, f"{module} is gated although a default route imports it")
    lib = (ROOT / "crates/rodas5p-integrators/src/lib.rs").read_text().split("\n")
    for i, line in enumerate(lib):
        for module in gated:
            if line.strip() in (f"mod {module};", f"pub mod {module};"):
                check(lib[i - 1].strip() == '#[cfg(feature = "research-gates")]', f"{module} is not gated in lib.rs")

    timing = [json.loads(l) for l in (HERE / "results/SL_N5_TIMING.jsonl").read_text().splitlines() if l.startswith("{")]
    whole = [t for t in timing if t.get("sl_n5") == "whole_solve"]
    check(len(whole) >= 6, "fewer than two grids of whole-solve timing")
    by_grid = {}
    for t in whole:
        by_grid.setdefault(t["grid"], {})[t["arm"]] = t
    for grid, arms in by_grid.items():
        if {"shipped", "separable-trusted"} <= arms.keys():
            check(arms["separable-trusted"]["relative_error"] <= 10 * max(arms["shipped"]["relative_error"], 1e-12),
                  f"{grid}: separable error not within 10x of shipped")

    if problems:
        print("\n".join(problems))
        return 1
    print(f"OK: {len(run['stages'])} stages, {len(run['claims'])} claims, {len(plan['tasks'])} plan tasks, "
          f"{len(by_grid)} timed grids, foundation checks pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
