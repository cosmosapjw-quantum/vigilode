#!/usr/bin/env python3
"""Strict structural checks for the VigilODE production-solver programme."""
import json, pathlib, sys
ROOT = pathlib.Path(__file__).resolve().parent

def load(name):
    def reject_constant(value):
        raise ValueError(f"non-RFC8259 constant: {value}")
    return json.loads((ROOT / name).read_text(encoding="utf-8"), parse_constant=reject_constant)

dag = load("RESEARCH_DAG.json")
catalog = load("METHOD_CATALOG.json")
lock = load("SOURCE_LOCK.json")
ids = [n["id"] for n in dag["nodes"]]
assert len(ids) == len(set(ids)), "duplicate DAG node id"
known = set(ids)
edges = {n["id"]: list(n.get("depends", [])) for n in dag["nodes"]}
assert all(d in known for ds in edges.values() for d in ds), "unknown DAG dependency"
visiting, visited = set(), set()
def visit(node):
    assert node not in visiting, f"DAG cycle at {node}"
    if node in visited: return
    visiting.add(node)
    for parent in edges[node]: visit(parent)
    visiting.remove(node); visited.add(node)
for node in ids: visit(node)
method_ids = [m["id"] for m in catalog["methods"]]
assert len(method_ids) == len(set(method_ids)), "duplicate method identity"
assert catalog["production_verdict"] == "HOLD"
assert dag["official_g0_mutated"] is False
g0 = lock["official_g0"]
assert g0["immutable"] and (g0["node_count"], g0["edge_count"]) == (23, 34)
assert lock["current_checkpoint"]["main_mutated"] is False
print(f"PASS strict JSON: 3 files; PASS DAG: {len(ids)} nodes, acyclic")
print(f"PASS methods: {len(method_ids)} unique IDs; PASS immutable G0: 23/34; PRODUCTION HOLD")
print("NOTE: structural validation is not numerical or runtime qualification.")
