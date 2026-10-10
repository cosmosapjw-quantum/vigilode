#!/usr/bin/env python3
"""Dependency-free semantic validation of this review's machine-readable bundle.

ARTIFACT_SCHEMA.json is the portable Draft2020-12 structural schema. This command
checks the cross-file invariants a JSON Schema alone cannot express. It does not
rerun native science or overwrite its evidence.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load(name):
    def nonfinite(value):
        raise ValueError(f"nonstandard JSON number {value}: {name}")
    return json.loads((HERE/name).read_text(), parse_constant=nonfinite)


def main():
    # Parse all machine artifacts, including preserved failed evidence.
    files = list(HERE.rglob("*.json"))
    for path in files:
        load(path.relative_to(HERE))
    dag = load("NEXT_DEVELOPMENT_DAG.json")
    require(dag["schema_version"] == "1.0", "DAG schema version")
    require(dag["artifact"] == "vigilode_next_development_dag", "DAG artifact type")
    nodes = dag["nodes"]
    ids = [n["id"] for n in nodes]
    require(len(ids) == len(set(ids)) == 22, "DAG unique 22 nodes")
    required = {"id", "title_ko", "priority", "kind", "status", "depends_on", "edit_targets",
                "contract", "implementation_steps", "acceptance_gate", "kill_or_hold_gate",
                "reused_evidence", "measurement_units", "execution", "claim_ceiling"}
    for n in nodes:
        require(required <= n.keys(), f"required fields {n['id']}")
        require(n["priority"] in {"P0", "P1", "P2", "P3"}, "priority")
        require(n["status"] == "PROPOSED_NOT_EXECUTED", "future node falsely completed")
        require(set(n["depends_on"]) <= set(ids), "unknown dependency")
        require(n["id"] not in n["depends_on"], "self dependency")
        for k in ("edit_targets", "contract", "implementation_steps", "acceptance_gate",
                  "kill_or_hold_gate", "reused_evidence", "measurement_units"):
            require(isinstance(n[k], list) and n[k] and all(isinstance(v, str) and v for v in n[k]), f"{n['id']}.{k}")
        require(n["execution"]["command_status"] == "PLANNED_COMMAND_REQUIRES_IMPLEMENTATION_AND_NEW_TEST_FILE", "command authority")
        for target in n["edit_targets"]:
            if target.startswith("crates/") and "(new)" not in target:
                require((ROOT/target.split("::")[0]).exists(), f"nonexistent existing target {target}")
    pending = {n["id"]: set(n["depends_on"]) for n in nodes}
    order = []
    while pending:
        ready = sorted(k for k, deps in pending.items() if deps <= set(order))
        require(ready, "cyclic DAG")
        order.extend(ready)
        for k in ready:
            del pending[k]
    findings = load("FINDINGS.json")["findings"]
    require(len(findings) == len({f["id"] for f in findings}) == 8, "finding inventory")
    for f in findings:
        require(set(f["next_nodes"]) <= set(ids), "finding node reference")
        for e in f["evidence"]:
            require((HERE/e["path"]).is_file(), f"missing finding evidence {e['path']}")
    claims = load("CLAIMS.json")["claims"]
    require(len(claims) == len({c["id"] for c in claims}) == 12, "claim inventory")
    for c in claims:
        for p in c["evidence_paths"]:
            require((HERE/p).is_file(), f"missing claim evidence {p}")
    results = load("RESULTS.json")
    require(results["verdict"] == "PASS" and results["production_promotion"] == "HOLD", "bounded decision")
    require(results["numeric_summary"]["passed_cells"] == 30, "numeric cells")
    require(results["numeric_summary"]["rejected_invalid_controls"] == 12, "negative controls")
    require(load("RESULTS_FIRST.json")["verdict"] == "FAIL", "first failure preservation")
    decision = load("evidence/independent_decision.json")
    require(decision["decision"] == "ACCEPT_BOUNDED_NATIVE_RESEARCH_SEAM", "independent scope")
    require(not decision["blocking_findings"], "independent blocker")
    require(all(decision[k] == "HOLD" for k in ("production_promotion", "solver_or_global_accuracy_promotion", "speed_promotion", "default_dispatch_promotion")), "promotion ceiling")
    for name, digest in decision["artifact_hashes"].items():
        require(hashlib.sha256((HERE/name).read_bytes()).hexdigest() == digest, f"reviewed bytes changed: {name}")
    require(load("CHECKER_PROBES.json")["source_hashes_unchanged"], "protected historical bytes")
    print(json.dumps({"status": "PASS", "json_files": len(files), "dag_nodes": len(nodes),
                      "findings": len(findings), "claims": len(claims), "topological_order": order}))


if __name__ == "__main__":
    main()
