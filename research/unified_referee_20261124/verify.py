"""Validate the evidence paths, execution contract, DAG, and native comparator cells."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
def load(name):
    return json.loads((HERE / name).read_text())

plan = load("NEXT_PLAN.json")
run = load("RUN.json")
stages = [f"{prefix}{i}" for prefix in ("R","C") for i in range(1,11)]
assert [row["id"] for row in run["stages"]] == stages
ev = {r["id"]:r for r in run["evidence"]}
assert len(ev) == len(run["evidence"])
for row in run["stages"]:
    assert row["status"] == "complete" and row["evidence_refs"] and set(row["evidence_refs"]) <= ev.keys()
for row in ev.values():
    path = (HERE / row["locator"]).resolve()
    assert path.is_relative_to(HERE.parents[1]) and path.is_file(), path
tasks = {t["id"]:t for t in plan["tasks"]}
assert len(tasks) == len(plan["tasks"])
done = set()
while len(done)<len(tasks):
    ready = {k for k, t in tasks.items() if set(t["depends_on"]) <= done} - done
    assert ready, "unresolved dependency or cycle"
    done |= ready
for method in load("METHODS.json")["methods"]:
    assert method["task"] in tasks
res = load("cvode/results.json")
assert len(res["rows"]) == 48 and res["julia_cases"] == 18
assert all((r["arm"]=="julia_rodas5p") or (r["success"] or "failure" in r) for r in res["rows"])
assert res["failure_count"] == sum(not r["success"] for r in res["rows"])
print(f"{len(stages)} executed stages, {len(ev)} evidence paths, {len(tasks)} acyclic tasks, {len(res['rows'])} external-comparison rows")
