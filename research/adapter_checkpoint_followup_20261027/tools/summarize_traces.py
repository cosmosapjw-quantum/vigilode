#!/usr/bin/env python3
"""Compact per-decision-point summary of checkpoint traces (added after execution; reads raw only)."""
import json
import sys
from pathlib import Path


def method(t):
    rows = t["attempt_rows"]
    acc = [r for r in rows if r["accepted"] and "error" in r]
    reasons = {}
    for r in rows:
        reasons[r["reason"]] = reasons.get(r["reason"], 0) + 1
    if t["method"] == "dopri5":
        alloc = sum(r["allocations"] for r in rows)
        ns = sum(r["attempt_ns_diagnostic"] for r in rows)
    else:
        alloc = sum(max(0, r["allocations_marginal"]) for r in rows[1:])
        ns = t["total_ns_min_of_3_diagnostic"]
    return {k: t.get(k) for k in ("method", "h0", "reached_target", "attempts", "accepted", "rejected", "rhs",
                                  "jvp_vectors", "linear_solves", "direct_factorizations", "linear_iterations",
                                  "work_units")} | {
        "landing_clipped_attempts": sum(1 for r in rows if r["landing_clipped"]),
        "reasons": reasons,
        "hot_allocations_after_first_attempt": alloc,
        "max_true_local_error_ratio": max((r["error"]["true_local_error_ratio_max"] for r in acc), default=None),
        "max_reference_disagreement": max((r["error"]["reference_disagreement_abs"] for r in acc), default=None),
        "elapsed_ns_diagnostic": ns}


def main():
    out = []
    for f in sorted(Path(sys.argv[1]).glob("*.json")):
        if f.name == "RUN.json":
            continue
        o = json.loads(f.read_text())
        rerun = o["rerun"]
        for p in o["points"]:
            row = {"case": o["case"], "probe_index": p["selection"]["probe_index"],
                   "interval_kind": p["selection"]["interval_kind"], "start": p["start"], "target": p["target"],
                   "raw_probe_decision": p["raw_probe"]["decision"]}
            for k in ("dopri", "rodas", "rodas_dense_unavailable_to_selector"):
                if k in p:
                    row[k] = method(p[k])
            out.append(row)
    print(json.dumps({"schema": "vigilode.adapter-checkpoint-trace-summary.v1",
                      "timing_authority": "diagnostic_only", "points": out}, indent=1))


if __name__ == "__main__":
    main()
