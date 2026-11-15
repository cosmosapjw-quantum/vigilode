#!/usr/bin/env python3
"""Strict structural validation for the VigilODE production-solver programme.

This checks packaging contracts only. It does not establish theorem truth,
numerical stability, implementation correctness or production performance.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def strict_load(path: Path):
    def reject_constant(value: str):
        raise ValueError(f"{path.name}: non-RFC8259 JSON constant {value!r}")

    return json.loads(path.read_text(encoding="utf-8"), parse_constant=reject_constant)


json_paths = sorted(ROOT.glob("*.json"))
assert json_paths, "no JSON files found"
docs = {p.name: strict_load(p) for p in json_paths}

required = {
    "ARCHITECTURE_CONTRACT.json",
    "METHOD_CATALOG.json",
    "MUTATION_GATES.json",
    "PARALLELISM_ANALYSIS.json",
    "REFERENCES.json",
    "RESEARCH_DAG.json",
    "SOURCE_LOCK.json",
    "TURN_PLAN.json",
    "LOOP_EXECUTION_RECORD.json",
    "EVIDENCE_LEDGER.json",
    "NOVELTY_MATRIX.json",
    "PACKAGE_LAYOUT_PLAN.json",
    "PRODUCTION_GATE_MATRIX.json",
}
missing = required.difference(docs)
assert not missing, f"missing required machine files: {sorted(missing)}"

dag = docs["RESEARCH_DAG.json"]
catalog = docs["METHOD_CATALOG.json"]
lock = docs["SOURCE_LOCK.json"]
turn_plan = docs["TURN_PLAN.json"]

nodes = dag["nodes"]
node_ids = [node["id"] for node in nodes]
assert len(node_ids) == len(set(node_ids)), "duplicate DAG node id"
known = set(node_ids)
edges = {node["id"]: list(node.get("depends", [])) for node in nodes}
assert all(dep in known for deps in edges.values() for dep in deps), "unknown DAG dependency"

visiting: set[str] = set()
visited: set[str] = set()


def visit(node: str) -> None:
    assert node not in visiting, f"DAG cycle at {node}"
    if node in visited:
        return
    visiting.add(node)
    for parent in edges[node]:
        visit(parent)
    visiting.remove(node)
    visited.add(node)


for node_id in node_ids:
    visit(node_id)

methods = catalog["methods"]
method_ids = [method["id"] for method in methods]
assert len(method_ids) == len(set(method_ids)), "duplicate method identity"
assert catalog["production_verdict"] == "HOLD"
assert dag["production_verdict"] == "HOLD"
assert dag["official_g0_mutated"] is False

by_id = {method["id"]: method for method in methods}
assert by_id["ppsw1_6b"]["stages"] == 6
assert by_id["ppsw1_6b"]["order"] == 5
assert by_id["ptsw2c"]["stages"] == 2
assert by_id["ptsw2c"]["order"] == 3

g0 = lock["official_g0"]
assert g0["immutable"] is True
assert (g0["node_count"], g0["edge_count"]) == (23, 34)
assert lock["current_checkpoint"]["main_mutated"] is False
assert turn_plan["total_turns"] == 6
assert len(turn_plan["turns"]) == 6

loop = docs["LOOP_EXECUTION_RECORD.json"]
assert loop["native_subagents"]["status"] == "NOT_RUN"
assert all(item["status"] == "NOT_RUN" for item in loop["not_completed"])
t1_exec = loop["turn_1_execution"]
assert t1_exec["turn_id"] == "T01_GLOBAL_ERROR_AUTHORITY"
assert t1_exec["status"] == "COMPLETE_SCOPED"
assert t1_exec["holdout_touched"] is False
assert t1_exec["crates_mutated"] is False

receipt = docs["REFERENCE_REPLAY_RECEIPT.json"]
assert receipt["contract_test_suite"]["total_passed"] == 59
assert receipt["contract_test_suite"]["total_failed"] == 0
assert receipt["cal96_replay"]["case_count"] == 18
assert receipt["official_g0"]["verdict"] == "HOLD"
assert receipt["official_g0"]["modified_in_turn_1"] is False

protocol = docs["MATCHED_ERROR_PROTOCOL.json"]
assert protocol["frozen_by_turn"] == "T01_GLOBAL_ERROR_AUTHORITY"

t01_nodes = [n for n in docs["RESEARCH_DAG.json"]["nodes"] if n["id"] == "T01_GLOBAL_ERROR_AUTHORITY"]
assert len(t01_nodes) == 1
assert t01_nodes[0]["status"] == "COMPLETE_SCOPED"
assert t01_nodes[0]["evidence"] == "NUMERICALLY_CHECKED"

gate = docs["PRODUCTION_GATE_MATRIX.json"]
assert gate["verdict"] == "HOLD"

reference_ids = [item["id"] for item in docs["REFERENCES.json"]["references"]]
assert len(reference_ids) == len(set(reference_ids)), "duplicate reference id"

print(f"PASS strict RFC8259 JSON: {len(json_paths)} files")
print(f"PASS DAG: {len(node_ids)} nodes, {sum(len(v) for v in edges.values())} edges, acyclic")
print(f"PASS method identities: {len(method_ids)} unique")
print("PASS PTSW2C identity: 2 stages/order 3")
print("PASS PPSW1-6b identity: 6 stages/order 5")
print("PASS immutable G0: 23 nodes/34 edges")
print("PASS six-turn plan and evidence-state invariants")
print("PASS Turn 1 replay receipt and matched-error protocol invariants")
print("PRODUCTION HOLD; structural checks are not numerical/runtime qualification.")
