#!/usr/bin/env python3
"""Validate package identity, JSON parseability, task graph and HOLD guard. Stdlib only."""
from __future__ import annotations
import hashlib, json
from pathlib import Path, PurePosixPath

ROOT=Path(__file__).resolve().parent

def fail(msg: str) -> None:
    raise SystemExit("FAIL: "+msg)

def load(name: str):
    try:
        return json.loads((ROOT/name).read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"{name}: {exc}")

manifest=load("MANIFEST.json")
queue=load("TASK_QUEUE.json")
graph=load("TASK_GRAPH.json")
verdict=load("PRODUCTION_VERDICT.json")
audit=load("CURRENT_AUDIT.json")
for name in ["BENCHMARK_AUDIT.json","ACCEPTANCE_MATRIX.json","TASK.schema.json","EVIDENCE.schema.json","MUTATION_POLICY.json","ANTI_DRIFT.json","SOURCE_REFERENCES.json"]:
    load(name)

if manifest.get("schema")!="vigilode.qualification_package_manifest.v3":
    fail("manifest schema mismatch")
if queue.get("schema")!="vigilode.production_task_queue.v3":
    fail("task queue schema mismatch")
if graph.get("schema")!="vigilode.task_graph.v3":
    fail("task graph schema mismatch")
if verdict.get("overall")!="HOLD_NOT_RELEASE_QUALIFIED":
    fail("false-PASS guard: production status must remain HOLD")
if audit.get("source_identity",{}).get("head")!=manifest.get("source",{}).get("head"):
    fail("audit / manifest source HEAD mismatch")

files=manifest.get("files",{})
if not files:
    fail("empty manifest")
for rel, record in files.items():
    path=(ROOT/rel).resolve()
    if ROOT.resolve() not in path.parents:
        fail(f"path traversal: {rel}")
    if not path.is_file():
        fail(f"missing file: {rel}")
    data=path.read_bytes()
    header=f"blob {len(data)}\0".encode("ascii")
    actual=hashlib.sha1(header+data).hexdigest()
    if actual!=record.get("git_blob_sha1"):
        fail(f"Git blob identity mismatch: {rel}")

tasks=queue.get("tasks",[])
ids=[t.get("id") for t in tasks]
if len(ids)!=len(set(ids)) or len(ids)!=queue.get("task_count"):
    fail("duplicate task id or task_count mismatch")
if set(ids)!=set(graph.get("nodes",[])):
    fail("task queue and graph nodes differ")
task_map={t["id"]:t for t in tasks}
for task in tasks:
    for dep in task.get("depends_on",[]):
        if dep not in task_map:
            fail(f"unknown dependency {dep} in {task['id']}")
    for p in task.get("write_allowlist",[]):
        path=PurePosixPath(p)
        if path.is_absolute() or ".." in path.parts or "*" in p or "?" in p:
            fail(f"unsafe write_allowlist entry: {task['id']} -> {p}")

remaining={k:set(v.get("depends_on",[])) for k,v in task_map.items()}
order=[]
while remaining:
    ready=sorted(k for k,v in remaining.items() if not v)
    if not ready:
        fail("task graph contains a cycle")
    order.extend(ready)
    for k in ready:
        remaining.pop(k)
    for deps in remaining.values():
        deps.difference_update(ready)
if set(order)!=set(ids):
    fail("topological traversal incomplete")
if len(tasks)!=13:
    fail(f"expected 13 tasks, found {len(tasks)}")
prompt=(ROOT/"AGENT_PROMPT_KO.txt").read_text(encoding="utf-8")
if "HOLD_NOT_RELEASE_QUALIFIED" not in prompt or "write_allowlist" not in prompt:
    fail("agent prompt missing HOLD or mutation gate")
print(f"PASS: {len(files)} manifest-bound files; {len(tasks)} tasks; DAG acyclic; production HOLD preserved.")
