#!/usr/bin/env python3
"""Deterministic decomposition of adapter work from driver records.

usage: analyze.py RAW_DIR OUT_JSON BIN_DIR SOURCE_COMMIT

Reads the campaign raw files written by run_campaign.sh (warm-up r0 and timed r1..r5),
buckets every switching segment by method and disposition, and writes one JSON with the
per-cell totals, buckets, timing (diagnostic only) and the switch-vs-direct comparison.
Counters are the authority; wall time is reported but never gated.
"""
import hashlib, json, statistics, sys

RAW, OUT, BIN, COMMIT = sys.argv[1:5]
RUNS = list(range(6))
BENCH = [(c, m) for c in ["rotation", "stiff-forced"] for m in ["evidence", "screened-reset", "pressure-only", "dopri", "rodas"]]
SUITE = [("c5", 5, m) for m in ["switching", "rodas5p-mf", "rodas5p-dense"]] + [("c10t", 10, m) for m in ["switching", "dopri5", "rodas5p-mf"]] + [("c10", 10, m) for m in ["switching", "dopri5"]]
SWITCH_MODES = {"evidence", "screened-reset", "pressure-only", "switching"}
BUCKETS = ["explicit_committed", "explicit_probe_discarded", "implicit_committed", "implicit_pilot_discarded"]
SUMF = ["attempts", "accepted_steps", "rejected_steps", "rhs_calls", "jvp_vectors", "jacobian_builds", "linear_solves", "ft_calls", "work_units"]


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def load_bench(case, mode, r):
    d = json.load(open(f"{RAW}/bench_{case}_{mode}_r{r}.json"))
    rec = d["record"]
    return {"kind": "bench", "case": case, "mode": mode, "seconds": d["seconds"], "success": d["success"],
            "error": d["endpoint_max_abs_error"], "accuracy_pass": d["accuracy_pass"], "cap": d["accuracy_cap"],
            "t_span": [0.0, 1.0], "stats": rec["stats"], "final_time": rec.get("final_time", rec["t"][-1]),
            "final_state": rec.get("final_state") or rec["y"][-1], "message": rec.get("message", "")}


def load_suite(tag, case, method, r):
    lines = [json.loads(l) for l in open(f"{RAW}/suite_{tag}_{method}_r{r}.jsonl") if l.strip()]
    hdr = [l for l in lines if l["kind"] == "header"][0]
    res = [l for l in lines if l["kind"] == "result"][0]
    return {"kind": "suite", "case": tag, "mode": method, "seconds": res["elapsed_seconds"], "success": res["solver_success"],
            "error": res["actual_error"], "accuracy_pass": res["status"] in ("PASS", "PASS_SCOPED"), "cap": res["error_limit"],
            "rtol": hdr["rtol"], "atol": hdr["atol"], "reference_sha256": res.get("reference_sha256"), "reference_kind": res.get("reference_kind"),
            "status": res["status"], "t_span": [0.0, hdr["model_contract"]["t_span"][1]], "stats": res["first_member_stats"],
            "final_time": res["final_time"], "final_state": res.get("final_state"), "message": res["message"]}


def direct_summary(st):
    c = st["counters"]
    return {k: st[k] for k in ("attempts", "accepted_steps", "rejected_steps")} | {
        "rhs_calls": c["rhs_calls"], "jvp_vectors": c["jvp_vectors"], "jacobian_builds": c["jacobian_builds"],
        "linear_solves": c["linear_solves"], "ft_calls": c["ft_calls"]}


def seg_rows(st):
    rows = []
    for i, s in enumerate(st["segments"]):
        if s["method"] == "dopri5":
            bucket = "explicit_committed" if s["committed"] else "explicit_probe_discarded"
        else:
            bucket = "implicit_committed" if s["committed"] else "implicit_pilot_discarded"
        rows.append({"index": i, "bucket": bucket, "method": s["method"], "is_probe": s["is_probe"], "committed": s["committed"],
                     "success": s["success"], "windows": s.get("windows", 1), "start": s["start_time"], "target": s["target_time"],
                     "reached": s["reached_time"], "initial_step": s["initial_step"], "decision": s["decision"], "evidence": s["evidence"],
                     "clipped": (s["driver"]["stats"].get("output_clipped_steps")),
                     **{f: s[f] for f in SUMF}})
    return rows


def timing(loads):
    timed = [l["seconds"] for l in loads[1:]]
    return {"warmup_seconds": loads[0]["seconds"], "timed_seconds": timed, "median_seconds": statistics.median(timed),
            "min_seconds": min(timed), "max_seconds": max(timed), "timing_authority": "diagnostic_only"}


def signature(l):
    st = l["stats"]
    if "segments" in st:
        return (st["attempts"], st["accepted_steps"], st["rejected_steps"], st["rhs_calls"], st["jvp_vectors"], st["linear_solves"],
                tuple((x["method"], x["committed"], x["attempts"], x["rhs_calls"], x["reached_time"]) for x in st["segments"]),
                tuple(l["final_state"] or []))
    c = st["counters"]
    return (st["attempts"], st["accepted_steps"], st["rejected_steps"], c["rhs_calls"], c["jvp_vectors"], c["linear_solves"],
            l["final_time"], tuple(l["final_state"] or []))


results = {"provenance": {"source_commit": COMMIT, "bin_dir": BIN,
                          "binaries": {b: sha(f"{BIN}/{b}") for b in ["evidence_switch_bench", "local_bottleneck_suite"]},
                          "protocol": "warm-up 1 + 5 timed runs per cell, taskset one core; WorkCounters are the authority; wall time diagnostic_only",
                          "bench_options": {"rtol": 1e-7, "atol": 1e-10, "h0": 1e-7, "max_steps": 20000, "span": [0, 1], "cap": 1e-5},
                          "suite_options": {"rtol": 1e-6, "atol": 1e-9, "h0": "min(1e-4, end/10)", "max_steps": 100000, "size": "standard", "cap": 1e-4,
                                            "switching_config": "ProvidedOnly, window_count 16, max_dense_dimension 256, pilot budget 32, pilot limit 4"}},
           "runs": {}, "comparison": {}}
loaded = {}
for case, mode in BENCH:
    loaded[("bench", case, mode)] = [load_bench(case, mode, r) for r in RUNS]
for tag, case, method in SUITE:
    loaded[("suite", tag, method)] = [load_suite(tag, case, method, r) for r in RUNS]
for (kind, case, mode), loads in loaded.items():
    k = f"{kind}:{case}:{mode}"
    l1 = loads[1]
    det = {"identical_across_runs": all(signature(l) == signature(loads[0]) for l in loads), "n_runs": len(loads)}
    base = {"kind": kind, "case": case, "mode": mode, "timing": timing(loads), "determinism": det,
            "success": l1["success"], "error": l1["error"], "cap": l1["cap"], "accuracy_pass": l1["accuracy_pass"],
            "final_time": l1["final_time"], "final_state": l1["final_state"], "message": l1["message"]}
    if mode in SWITCH_MODES:
        st = l1["stats"]
        rows = seg_rows(st)
        buckets = {}
        for b in BUCKETS:
            rs = [r for r in rows if r["bucket"] == b]
            buckets[b] = {"count": len(rs), **{f: sum(r[f] for r in rs) for f in SUMF}, "span": sum(r["reached"] - r["start"] for r in rs)}
        totals = {f: st[f] for f in ("attempts", "accepted_steps", "rejected_steps", "rhs_calls", "jvp_vectors", "jacobian_builds", "linear_solves", "ft_calls", "switches")}
        totals["segments"] = len(rows)
        tag = {"explicit_committed": "D", "explicit_probe_discarded": "Dprobe!", "implicit_committed": "R", "implicit_pilot_discarded": "Rpilot!"}
        results["runs"][k] = {**base, "role": "switch", "totals": totals, "buckets": buckets,
                              "reconciled": all(sum(r[f] for r in rows) == st[f] for f in ("attempts", "rhs_calls", "jvp_vectors", "linear_solves")) and st["accounting_complete"],
                              "decision_sequence": " ".join(f"{r['index']}:{tag[r['bucket']]}[{r['attempts']}]" for r in rows),
                              "segments": rows}
    else:
        results["runs"][k] = {**base, "role": "direct", "stats": direct_summary(l1["stats"])}
for k, v in results["runs"].items():
    if v["role"] != "switch":
        continue
    # Direct candidates: completed runs; with a validated cap only those that pass it, otherwise any completed run (reported, not gated).
    same = [(dk, dv) for dk, dv in results["runs"].items() if dv["role"] == "direct" and dv["kind"] == v["kind"] and dv["case"] == v["case"] and dv["success"]]
    cands = [kv for kv in same if kv[1]["accuracy_pass"]] or same
    cands.sort(key=lambda kv: kv[1]["timing"]["median_seconds"])
    if not cands:
        results["comparison"][k] = {"best_direct": None}
        continue
    bk, bv = cands[0]
    results["comparison"][k] = {"best_direct": bk, "best_direct_median_seconds": bv["timing"]["median_seconds"],
                                "switch_median_seconds": v["timing"]["median_seconds"],
                                "time_ratio_median": v["timing"]["median_seconds"] / bv["timing"]["median_seconds"],
                                "switch_attempts": v["totals"]["attempts"], "best_direct_attempts": bv["stats"]["attempts"],
                                "switch_rhs": v["totals"]["rhs_calls"], "best_direct_rhs": bv["stats"]["rhs_calls"],
                                "direct_medians": {dk: dv["timing"]["median_seconds"] for dk, dv in cands}}
json.dump(results, open(OUT, "w"), indent=1)
print("wrote", OUT, "cells", len(results["runs"]))
