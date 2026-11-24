#!/usr/bin/env python3
"""Validate the stage-lattice referee run: record structure, evidence paths, plan DAG, idea
matrix, and re-run the three cheap independent checks (prior claims, lattice depths,
two-way = partial pivoting). Exit status 0 when everything holds.

Usage (from the repository root): python3 -I research/stage_lattice_referee_20261124/verify.py
"""
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def load(name):
    return json.loads((HERE / name).read_text())


run = load("RUN.json")
stages = [f"{p}{i}" for p in ("R", "C") for i in range(1, 11)]
assert [s["id"] for s in run["stages"]] == stages, "stage ids"
evidence = {e["id"]: e for e in run["evidence"]}
assert len(evidence) == len(run["evidence"]), "duplicate evidence ids"
for s in run["stages"]:
    assert s["status"] == "complete" and s["evidence_refs"], s["id"]
    assert set(s["evidence_refs"]) <= evidence.keys(), s["id"]
for e in evidence.values():
    assert (HERE / e["locator"]).is_file(), e["locator"]

plan = load("NEXT_PLAN.json")
tasks = {t["id"]: t for t in plan["tasks"]}
assert len(tasks) == len(plan["tasks"]), "duplicate task ids"
for t in tasks.values():
    for key in ("title", "depends_on", "owner_decision", "files", "question", "implementation",
                "acceptance", "kill", "fallback", "commands"):
        assert key in t, (t["id"], key)
    assert set(t["depends_on"]) <= tasks.keys(), t["id"]
done = set()
while len(done) < len(tasks):
    ready = {k for k, t in tasks.items() if set(t["depends_on"]) <= done} - done
    assert ready, "cycle or unresolved dependency in NEXT_PLAN.json"
    done |= ready
assert set(plan["first_parallel"]) <= tasks.keys() and set(plan["priority"]) <= tasks.keys()

matrix = load("IDEA_SOLVER_MATRIX.json")
families = set(matrix["families"])
for idea in matrix["ideas"]:
    assert set(idea["cells"]) == families, idea["id"]
    for fam, cell in idea["cells"].items():
        assert cell["status"] in matrix["statuses"], (idea["id"], fam, cell["status"])

inventory = json.loads((HERE / "results/INTEGRATOR_INVENTORY.json").read_text())
assert {m["status"] for m in inventory["modules"]} <= set(inventory["statuses"])

scripts = HERE / "scripts"
with tempfile.TemporaryDirectory() as tmp:
    tmp = pathlib.Path(tmp)
    for script, out in (
        ("verify_prior_claims.py", "prior.json"),
        ("stage_lattice_dag.py", "lattice.json"),
        ("twoway_gepp_check.py", "gepp.json"),
    ):
        result = subprocess.run(
            [sys.executable, "-I", str(scripts / script), str(tmp / out)],
            capture_output=True, text=True, cwd=ROOT,
        )
        assert result.returncode == 0, (script, result.stdout[-2000:], result.stderr[-2000:])
    prior = json.loads((tmp / "prior.json").read_text())
    assert not prior["failed"], prior["failed"]
    lattice = json.loads((tmp / "lattice.json").read_text())
    assert lattice["claims"]["all_fits_exact_and_slopes_as_claimed"]
    gepp = json.loads((tmp / "gepp.json").read_text())
    assert gepp["summary"]["passed"] == gepp["summary"]["cases"] == 30

print(f"{len(stages)} stages, {len(evidence)} evidence files, {len(tasks)} acyclic tasks, "
      f"{len(matrix['ideas'])} ideas x {len(families)} families, "
      f"{len(inventory['modules'])} inventoried modules, 3 independent checks re-run: OK")
