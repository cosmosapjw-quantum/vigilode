#!/usr/bin/env python3
"""Static plan package check only; no solver qualification is implied."""
import json, pathlib, sys
root = pathlib.Path(__file__).resolve().parent
required = ["PLAN_BUNDLE.json", "DESIGN_CONTRACT.md", "AGENT_PROMPT_KO.txt", "README.md", "VALIDATION_PROTOCOL.md"]
missing = [name for name in required if not (root / name).is_file()]
if missing:
    print("FAIL missing:", missing); sys.exit(1)
try:
    data = json.loads((root / "PLAN_BUNDLE.json").read_text(encoding="utf-8"))
except Exception as exc:
    print("FAIL invalid PLAN_BUNDLE.json:", exc); sys.exit(1)
tasks = data.get("tasks", [])
ids = [task.get("id") for task in tasks]
if not tasks or len(ids) != len(set(ids)):
    print("FAIL empty or duplicate task IDs"); sys.exit(1)
known = set(ids)
for task in tasks:
    for dep in task.get("depends_on", []):
        if dep not in known:
            print("FAIL unknown dependency", task.get("id"), dep); sys.exit(1)
print("PASS static package: required files present, JSON parsed,", len(tasks), "tasks, dependency IDs valid")
print("NOT CHECKED: Rust source, mathematical proof, solver runtime, benchmark, CI, or release qualification")
