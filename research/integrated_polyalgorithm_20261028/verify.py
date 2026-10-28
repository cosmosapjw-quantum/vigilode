#!/usr/bin/env python3
"""Verify this frozen research package, never infer native solver qualification.

Run at the delivered research commit. Later runtime edits require current-tree
semantic tests and a historical replay here, not relaxing this artifact check.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess


NODE = Path(__file__).resolve().parent
REPO = NODE.parents[1]
SOURCE = "af7f3e9f55d652554042d2386bf07abb7d857d75"
PR93 = "384dba091a3d9e1aac0ecddec264958298239253"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=REPO)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json(path: Path):
    return json.loads(path.read_text())


def acyclic(ids: set[str], edges: list[list[str]]) -> None:
    require(all(a in ids and b in ids for a, b in edges), "unknown DAG endpoint")
    pending = set(ids)
    while pending:
        ready = {n for n in pending if not any(b == n and a in pending for a, b in edges)}
        require(bool(ready), "cycle in DAG")
        pending -= ready


def main() -> None:
    plan = read_json(NODE / "INDEPENDENT_VALIDATION.json")
    old_path = REPO / plan["inheritance"]["path"]
    old = read_json(old_path)
    require(sha(old_path.read_bytes()) == plan["inheritance"]["sha256"], "inherited plan changed")
    edges = [[d, n["id"]] for n in old["nodes"] for d in n.get("dependencies", [])]
    ids = {n["id"] for n in old["nodes"]}
    require(len(ids) == 23 and len(edges) == 34, "inherited DAG count changed")
    require(edges == plan["inheritance"]["dependency_edges"], "inherited edges changed")
    acyclic(ids, edges)
    acyclic({n["id"] for n in plan["validation_layers"]}, plan["validation_layer_edges"])
    canonical = json.dumps(old["execution_suites"], sort_keys=True, separators=(",", ":")).encode()
    require(sha(canonical) == plan["inheritance"]["execution_suites_canonical_sha256"], "native argv changed")
    require(len(old["execution_suites"]) == 8, "suite count changed")
    budgets = [{k: n[k] for k in ("id", "correction_loops_used", "correction_loops_max")}
               for n in old["nodes"] if "correction_loops_used" in n]
    require(budgets == plan["inheritance"]["correction_budgets"], "retry budget changed")
    require(old["candidate_catalog"] == plan["inheritance"]["candidate_catalog"], "candidate catalog changed")

    coverage = read_json(REPO / plan["coverage"]["path"])
    commands = read_json(REPO / plan["command_catalog"]["path"])
    for key in ("coverage", "command_catalog"):
        binding = plan[key]
        require(sha((REPO / binding["path"]).read_bytes()) == binding["sha256"], f"{key} bytes changed")
    require(len(coverage["paths"]) == 73 and len(coverage["gate_groups"]) == 18, "coverage dimensions changed")
    require(len(commands["commands"]) == 28, "command count changed")
    declared = {p["path"] for p in coverage["paths"]}
    actual = set(git("diff", "--name-only", "--no-renames", coverage["conversation_baseline"], PR93,
                     "--", *coverage["changed_scope"]).decode().splitlines())
    require(actual == declared, "changed-path coverage is incomplete")
    for p in coverage["paths"]:
        require(sha(git("show", f"{PR93}:{p['path']}")) == p["source_sha256"], f"coverage hash {p['path']}")
        require(p["gate_ids"] and all(g in coverage["gate_groups"] for g in p["gate_ids"]), "unknown coverage gate")
    require(set(coverage["new_interactions"]) == {i["id"] for i in plan["interaction_gates"]}, "interaction mismatch")
    require({n["task"] for n in plan["task_extensions"]} == {f"PR-N{i:02d}" for i in range(1, 7)}, "N task coverage")
    require({n["id"] for n in plan["qualification_blockers"]} == {i for i in ids if i.startswith("Q-")}, "Q task coverage")

    for lane, count in (("ERROR", 6), ("ALGEBRA", 9)):
        receipt = read_json(NODE / f"evidence/{lane}_RECEIPT.json")
        result = read_json(NODE / f"evidence/{lane}_RESULTS.json")
        require(receipt["source_commit"] == SOURCE and receipt["exit_code"] == 0 and receipt["source_unchanged"], "invalid receipt")
        require(result["status"] == "PASS", "supporting check failed")
        for p, digest in receipt["inputs_sha256"].items():
            require(sha(git("show", f"{SOURCE}:{p}")) == digest, f"registered source hash: {p}")
            if not p.endswith("PREREGISTRATION.md"):
                require(sha((REPO / p).read_bytes()) == digest, f"registered proof/check edited: {p}")
        for stream in ("stdout", "stderr"):
            require(sha((REPO / receipt[stream]).read_bytes()) == receipt[stream + "_sha256"], "raw output changed")
        if lane == "ERROR":
            groups = [v for v in result.values() if isinstance(v, dict) and v.get("status") == "PASS"]
            require(len(groups) == count, "error group count")
        else:
            require(result["compiler_case_count"] == count, "compiler case count")
            require(all(c["status"] == "PASS" for c in result["compiler_cases"]), "compiler fixture failed")
            require(result["refinement"]["minimum_corrections"] == 6, "refinement fixture changed")

    for pin in [SOURCE, *plan["source_contract"]["required_pr_heads"].values()]:
        git("merge-base", "--is-ancestor", pin, "HEAD")
    require(not git("diff", PR93, "--", "crates", ".github", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"),
            "runtime/workflow changed: use this artifact verifier at the original delivered research source")
    manifest = read_json(NODE / "MANIFEST.json")
    for p, digest in manifest["files_sha256"].items():
        require(sha((REPO / p).read_bytes()) == digest, f"deliverable hash: {p}")
    print(json.dumps({"status": "PASS", "scope": "frozen integrated research artifacts and source-bound support checks",
                      "inherited_nodes": 23, "inherited_edges": 34, "native_suites_unchanged": 8,
                      "coverage_paths": 73, "gate_groups": 18, "interactions": 10,
                      "error_groups": 6, "compiler_fixtures": 9,
                      "native_solver_validation": "NOT_RUN", "native_performance": "NOT_RUN"}, indent=2))


if __name__ == "__main__":
    main()
