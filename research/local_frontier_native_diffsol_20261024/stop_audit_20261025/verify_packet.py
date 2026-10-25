#!/usr/bin/env python3
"""Offline checks for the compact stopped-benchmark audit packet."""
from __future__ import annotations

import hashlib
import itertools
import json
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def read(relative: str):
    return json.loads((ROOT / relative).read_text())


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_manifest() -> int:
    manifest = read("evidence/SHA256_MANIFEST.json")
    assert manifest["schema"] == "vigilode.local-benchmark-stop-evidence-manifest.v1"
    seen = set()
    for row in manifest["entries"]:
        relative = row["relative_path"]
        assert relative not in seen
        seen.add(relative)
        path = (ROOT / relative).resolve()
        assert path.is_relative_to(ROOT)
        assert path.stat().st_size == row["size_bytes"]
        assert digest(path) == row["sha256"]
    return len(seen)


def check_cost() -> tuple[int, float]:
    path = ROOT / "evidence/state/COST_EVENTS.jsonl"
    previous = "0" * 64
    ids = set()
    slots = {}
    lines = path.read_bytes().splitlines(keepends=True)
    for line in lines:
        assert line.endswith(b"\n")
        row = json.loads(line)
        assert row["schema"] == "vigilode.local-continuation-cost-event.v1"
        assert row["prev_event_sha256"] == previous
        assert row["event_id"] not in ids
        ids.add(row["event_id"])
        attempt = slots.setdefault(row["attempt_id"], {})
        assert row["phase"] not in attempt
        attempt[row["phase"]] = row
        previous = hashlib.sha256(json.dumps(row, sort_keys=True,
            separators=(",", ":"), allow_nan=False).encode()).hexdigest()
    charged = sum(v["TERMINAL"]["charged_seconds"] for v in slots.values()
                  if "TERMINAL" in v)
    assert all("TERMINAL" in v for v in slots.values())
    nested = read("evidence/cost_nested/RECONCILIATION.json")
    assert len(nested["rows"]) == 9
    for row in nested["rows"]:
        receipt = read(row["receipt_relative_path"])
        value = receipt.get("charged_new_seconds", receipt.get("charged_seconds"))
        assert value == row["charged_seconds"]
    assert sum(r["charged_seconds"] for r in nested["rows"]) == nested["charged_seconds"] == 354
    total = charged + nested["charged_seconds"]
    status = read("STOP_STATUS.json")
    assert len(lines) == status["cost"]["event_count"] == 191
    assert digest(path) == status["cost"]["event_chain_sha256"]
    assert math.isclose(total, status["cost"]["cumulative_charged_seconds"], abs_tol=1e-7)
    return len(lines), total


def check_case20() -> float:
    evidence = read("evidence/publication_supplement/CASE20_REFERENCE_EVIDENCE.json")
    levels = {x["label"]: x["full_endpoint_state"] for x in evidence["levels"]}
    assert len(levels) == 4 and {len(v) for v in levels.values()} == {1024}
    distances = []
    for a, b in itertools.combinations(levels, 2):
        difference = max(abs(x-y) for x, y in zip(levels[a], levels[b]))
        key = f"{a}__{b}"
        assert math.isclose(difference, evidence["pairwise_inf"][key], abs_tol=1e-15)
        distances.append(difference)
    uncertainty = max(1e-12, 10 * max(distances))
    reference = read("evidence/publication_supplement/CASE20_EXTERNAL_REFERENCE.json")
    assert reference["reference_state"] == levels["tsit5_1e10"]
    assert reference["uncertainty"] == evidence["empirical_uncertainty"] == uncertainty
    assert uncertainty <= 1e-5
    return uncertainty


def check_status() -> tuple[int, int]:
    old = read("evidence/state/LEGACY_STANDARD_TASK_INDEX_SUMMARY.json")
    checks = old["checks"]
    assert checks["indexed_tasks"] == 1539
    assert checks["indexed_attempts"] == 1540
    assert checks["unvalidated_reference_unresolved"] == 805
    assert checks["final_uninstrumented_timeouts"] == 199
    assert checks["final_typed_step_budget_failures"] == 54
    report = read("evidence/publication_supplement/LOCAL_REPORT_EXPORT_INDEX.json")
    assert report["counts"]["receipt_rows"] == 2565
    assert report["counts"]["duplicate_raw_evidence_keys"] == 0
    julia = read("evidence/publication_supplement/JULIA_COMPARISON_COMPACT.json")
    assert len(julia["groups"]) == 8
    assert sum(len(g["attempts"]) for g in julia["groups"]) == 48
    assert all(g["status"] == "PASS_SCOPED" for g in julia["groups"])
    plan = read("evidence/accuracy/ACCURACY_COHORT_PLAN_V3_CANDIDATE.json")
    selection_path = ROOT / "evidence/accuracy/ACCURACY_COHORT_SELECTION_V3.json"
    assert plan["selection_sha256"] == digest(selection_path)
    assert plan["status"].startswith("NONLAUNCHABLE")
    selection = json.loads(selection_path.read_text())
    assert len(selection["groups"]) == 12 and len(selection["attempts"]) == 72
    assert selection["resource_caps"]["max_rss_bytes"] == 12 * 1024**3
    assert all(a["max_rss_bytes"] == 12 * 1024**3 for a in selection["attempts"])
    stop = read("STOP_STATUS.json")
    assert stop["accuracy_cohort"]["status"] == "NOT_RUN_APPROVAL_PENDING_AT_STOP"
    assert stop["timeout_diagnostics"]["status"] == "NOT_RUN_IMPLEMENTATION_GATE_BLOCKED_AT_STOP"
    assert stop["production"]["broad_status"] == "HOLD"
    return checks["indexed_tasks"], sum(len(g["attempts"]) for g in julia["groups"])


if __name__ == "__main__":
    files = check_manifest()
    events, cost = check_cost()
    u = check_case20()
    tasks, julia = check_status()
    print(json.dumps({"status": "PASS_PACKET_SELF_CHECK", "files": files,
        "cost_events": events, "cumulative_charged_seconds": cost,
        "case20_empirical_uncertainty": u, "legacy_tasks": tasks,
        "julia_attempts": julia}, sort_keys=True))
