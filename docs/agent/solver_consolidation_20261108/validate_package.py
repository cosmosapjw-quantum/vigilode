#!/usr/bin/env python3
"""Static integrity check for the VigilODE research-plan package only.

This script does NOT run or qualify Rust solver code, mathematical proofs,
performance benchmarks, production CI, or release gates.
"""
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent
REQUIRED = [
    "PLAN_BUNDLE.json",
    "DESIGN_CONTRACT.md",
    "AGENT_PROMPT_KO.txt",
    "README.md",
    "VALIDATION_PROTOCOL.md",
    "TASK.schema.json",
    "EVIDENCE.schema.json",
    "PARALLEL_RESEARCH_PLAN.json",
    "STAGE_TRANSFORMATION_THEORY.md",
    "HISTORICAL_RESEARCH_MAP.json",
    "CPU_PARALLEL_NODES.md",
    "PARALLEL_BENCHMARK_PROTOCOL.md",
    "PARALLEL_AGENT_PROMPT_KO.txt",
    "REUSE_FIRST_INVENTORY.md",
    "PEER_METHOD_FAMILY_REVIEW.md",
    "validate_package.py",
    "MANIFEST.json",
]

def fail(message: str) -> None:
    print("FAIL:", message)
    raise SystemExit(1)

missing = [name for name in REQUIRED if not (ROOT / name).is_file()]
if missing:
    fail(f"missing files: {missing}")

def read_json(name: str):
    try:
        return json.loads((ROOT / name).read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON {name}: {exc}")

bundle = read_json("PLAN_BUNDLE.json")
tasks = bundle.get("tasks", [])
task_ids = [task.get("id") for task in tasks]
if not tasks or len(task_ids) != len(set(task_ids)):
    fail("empty or duplicate base task IDs")
known = set(task_ids)
for task in tasks:
    unknown = [dep for dep in task.get("depends_on", []) if dep not in known]
    if unknown:
        fail(f"unknown base dependencies for {task.get('id')}: {unknown}")

parallel = read_json("PARALLEL_RESEARCH_PLAN.json")
historical = read_json("HISTORICAL_RESEARCH_MAP.json")
if parallel.get("gpu") is None or "FORBIDDEN" not in parallel["gpu"]:
    fail("parallel plan must explicitly forbid GPU")
nodes = [
    parallel.get("main_candidate"),
    parallel.get("transform_candidate"),
    parallel.get("source_specific_transform"),
    *parallel.get("other_cpu_nodes", []),
]
if any(not isinstance(node, dict) for node in nodes):
    fail("parallel plan contains a missing/non-object node")
node_ids = [node.get("id") for node in nodes]
if len(node_ids) != len(set(node_ids)) or any(not value for value in node_ids):
    fail("parallel node IDs are missing or duplicated")
for node in nodes:
    if node.get("status") not in {"PLANNED", "IN_PROGRESS", "BLOCKED", "PASS", "FAIL", "CANCELLED"}:
        fail(f"invalid status for node {node.get('id')}")
    if node.get("gpu") != "FORBIDDEN":
        fail(f"GPU policy missing/changed for {node.get('id')}")
if not isinstance(historical, list) or len(historical) < 10:
    fail("historical map is missing expected PR/research lineage entries")
peer_review = (ROOT / "PEER_METHOD_FAMILY_REVIEW.md").read_text(encoding="utf-8")
for peer_term in ["tilde", "multi-implicit", "2011", "sequential", "startup"]:
    if peer_term not in peer_review:
        fail(f"peer method family review omits {peer_term}")
if "Kulikov & Weiner" not in peer_review:
    fail("peer method family review omits published error-estimator source")
prompt = (ROOT / "PARALLEL_AGENT_PROMPT_KO.txt").read_text(encoding="utf-8")
if "GPU is forbidden" not in prompt and "GPU remains disabled" not in prompt:
    fail("agent prompt does not explicitly forbid GPU")
reuse = (ROOT / "REUSE_FIRST_INVENTORY.md").read_text(encoding="utf-8")
for required_reuse in ["ParallelExecution", "Dopri5Ensemble", "stage_batch", "conditioned_polynomial", "fixed_point_continuation", "fourier_path_certificate"]:
    if required_reuse not in reuse:
        fail(f"reuse-first inventory omits {required_reuse}")

manifest = read_json("MANIFEST.json")
manifest_entries = manifest.get("files", [])
entry_map = {item.get("path"): item.get("git_blob_sha1") for item in manifest_entries}
if len(entry_map) != len(manifest_entries):
    fail("duplicate manifest paths")
for name in REQUIRED:
    if name == "MANIFEST.json":
        continue
    if name not in entry_map:
        fail(f"manifest omits {name}")

checked = 0
for relative, expected in entry_map.items():
    if relative == "MANIFEST.json":
        fail("manifest must not hash itself")
    path = ROOT / relative
    if not path.is_file():
        fail(f"manifest path missing: {relative}")
    raw = path.read_bytes()
    header = b"blob " + str(len(raw)).encode("ascii") + b"\0"
    actual = hashlib.sha1(header + raw).hexdigest()
    if actual != expected:
        fail(f"Git blob SHA-1 mismatch for {relative}: {actual} != {expected}")
    checked += 1

print(
    "PASS static package:",
    len(tasks), "base tasks;",
    len(nodes), "parallel nodes;",
    len(historical), "history entries;",
    checked, "Git blob identities verified"
)
print(
    "NOT CHECKED: Rust source, mathematical proof, solver runtime, "
    "CPU benchmark, production CI, or release qualification"
)
