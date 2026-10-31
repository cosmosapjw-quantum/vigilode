#!/usr/bin/env python3
"""Validate only mission overlay structure; never certify physical/DAG admission."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

VALID_STATES = {"blocked", "partial", "ready", "admitted"}
VALID_OWNERS = {"rei_bianchi", "bass", "rec_bianchi", "vigilode"}


def validate(doc: object) -> list[str]:
    errors: list[str] = []
    if not isinstance(doc, dict):
        return ["top-level mission DAG must be a JSON object"]
    if doc.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if doc.get("document_kind") != "noncanonical_mission_alignment_overlay":
        errors.append("must remain a noncanonical mission overlay")
    if doc.get("canonical_dag_delta_from_this_change") != 0:
        errors.append("overlay must not claim any canonical DAG delta")
    if doc.get("admitted_mission_nodes") != []:
        errors.append("overlay is not a canonical admission mechanism")

    authority = doc.get("canonical_authority", {})
    if not isinstance(authority, dict) or not all(
        isinstance(authority.get(k), str) and authority[k].strip()
        for k in ("rei_main_state", "rei_d0_roadmap", "vigilode_original_solver_dag", "vigilode_research_ledger")
    ):
        errors.append("required distinct canonical authority references absent")
    counts = doc.get("reported_not_recomputed_counters", {})
    if not isinstance(counts, dict) or not all(
        isinstance(counts.get(k), str) and "/" in counts[k]
        for k in ("vigilode_scope", "rei_central", "rei_working", "bass_ce1")
    ) or counts.get("new_owner_ivps") != 0:
        errors.append("historical reported counters and zero-IVP caveat must be separate")

    nodes = doc.get("nodes")
    if not isinstance(nodes, list) or len(nodes) == 0:
        return errors + ["nodes must be a nonempty array"]
    by_id: dict[str, dict] = {}
    for i, n in enumerate(nodes):
        if not isinstance(n, dict):
            errors.append(f"nodes[{i}] must be an object")
            continue
        nid = n.get("id")
        if not isinstance(nid, str) or not nid:
            errors.append(f"nodes[{i}] has no valid id")
            continue
        if nid in by_id:
            errors.append(f"duplicate node id {nid}")
            continue
        by_id[nid] = n
        if n.get("owner") not in VALID_OWNERS:
            errors.append(f"{nid}: unknown owner")
        others = n.get("supporting_owners", [])
        if not isinstance(others, list) or any(o not in VALID_OWNERS for o in others):
            errors.append(f"{nid}: invalid supporting owners")
        if n.get("state") not in VALID_STATES:
            errors.append(f"{nid}: invalid state")
        if n.get("state") == "admitted":
            errors.append(f"{nid}: admission requires external canonical owner, not this overlay")
        for field in ("name", "blocker", "observable_relevance", "next_decisive_evidence"):
            if not isinstance(n.get(field), str) or not n[field].strip():
                errors.append(f"{nid}: {field} missing")
        deps = n.get("depends_on")
        if not isinstance(deps, list) or any(not isinstance(x, str) for x in deps):
            errors.append(f"{nid}: depends_on must be an array of IDs")
        elif len(set(deps)) != len(deps):
            errors.append(f"{nid}: repeated dependency")
    for nid, n in by_id.items():
        deps = n.get("depends_on", [])
        if not isinstance(deps, list):
            continue
        for dep in deps:
            if dep not in by_id:
                errors.append(f"{nid}: missing dependency {dep}")
            if dep == nid:
                errors.append(f"{nid}: self dependency")
        if n.get("state") == "ready" and any(
            dep in by_id and by_id[dep].get("state") != "admitted" for dep in deps
        ):
            errors.append(f"{nid}: ready despite non-admitted dependency")

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(nid: str) -> None:
        if nid in visited:
            return
        if nid in visiting:
            errors.append(f"cycle involving {nid}")
            return
        visiting.add(nid)
        n = by_id[nid]
        deps = n.get("depends_on", [])
        if isinstance(deps, list):
            for dep in deps:
                if isinstance(dep, str) and dep in by_id:
                    visit(dep)
        visiting.remove(nid)
        visited.add(nid)

    for node_id in by_id:
        visit(node_id)

    p = doc.get("work_stream_policy", {})
    if not isinstance(p, dict) or p.get("critical_path_next") not in by_id:
        errors.append("critical path must reference an existing node")
    else:
        for guard, expected in (("production", "HOLD"), ("no_merge", True),
                                ("no_force_push", True), ("auto_next_node", False),
                                ("no_more_ivps", True)):
            if p.get(guard) != expected:
                errors.append(f"protected guard {guard} changed")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dag", type=Path, default=Path("governance/mission_dag.json"))
    args = parser.parse_args(argv)
    try:
        doc = json.loads(args.dag.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"mission DAG unreadable: {exc}", file=sys.stderr)
        return 2
    errors = validate(doc)
    if errors:
        for item in errors:
            print(f"FAIL: {item}", file=sys.stderr)
        return 1
    print(f"mission DAG STRUCTURE PASS ({len(doc['nodes'])} overlay nodes, canonical delta=0; no physics/admission verified)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
