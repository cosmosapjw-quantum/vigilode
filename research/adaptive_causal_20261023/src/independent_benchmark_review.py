#!/usr/bin/env python3
"""Reconcile final benchmark artifacts without rerunning solver/reference jobs."""
from collections import defaultdict
import gzip
import hashlib
import json
from pathlib import Path
import statistics

PACKAGE = Path(__file__).resolve().parents[1]
RESULTS = PACKAGE / "results"
checks = []
inputs = []


def read(path):
    p = RESULTS / path
    payload = p.read_bytes()
    inputs.append({"path": "results/" + path, "sha256": hashlib.sha256(payload).hexdigest()})
    return json.loads(payload)


def check(name, condition):
    checks.append({"id": name, "passed": bool(condition)})


def same(a, b):
    return abs(a - b) <= 1e-13 * max(abs(a), abs(b), 1e-300)


q = read("quotient_benchmark.json")
qs = read("quotient_summary.json")
qr = read("quotient_run_receipt.json")
groups = defaultdict(list)
for row in q["rows"]:
    if not row["warmup"]:
        groups[row["case"], row["method"], row["tol"]].append(row)
check("quotient-row-counts", len(q["rows"]) == 112 and sum(map(len, groups.values())) == 84)
check("quotient-source-and-exit", qr["exit_code"] == 0 and qr["source_commit"] == "548526add83a38a996875b3bd2f3cd3357465ed6")
for a in qs["aggregates"]:
    rows = groups[a["case"], a["method"], a["tol"]]
    ok = len(rows) == 3 and a["success"] == all(r["success"] for r in rows)
    ok &= same(a["median_seconds"], statistics.median(r["seconds"] for r in rows))
    ok &= same(a["error"], max(r["maximum_sampled_absolute_error"] for r in rows))
    check("quotient-aggregate-" + str((a["case"], a["method"], a["tol"])), ok)
for c in qs["comparisons"]:
    selected = []
    for method in ["full-rhs-dopri5", "quotient-lawson-dopri5"]:
        candidates = [a for a in qs["aggregates"] if a["case"] == c["case"] and a["method"] == method and a["success"] and a["error"] <= c["sampled_error_cap"]]
        selected.append(min(candidates, key=lambda a: a["median_seconds"]) if candidates else None)
    a, b = selected
    ratio = a["median_seconds"] / b["median_seconds"] if a and b else None
    check("quotient-selection-" + str((c["case"], c["sampled_error_cap"])), selected == [c["baseline"], c["candidate"]] and (ratio == c["baseline_over_candidate"] if ratio is None else same(ratio, c["baseline_over_candidate"])))

plan = read("switching_campaign/PLAN.json")
summary = read("switching_campaign/SUMMARY.json")
replay = read("switching_campaign/REPLAY.json")
execution = read("switching_campaign/EXECUTION.json")
compressed = (RESULTS / "switching_campaign/RAW_RUNS.jsonl.gz").read_bytes()
inputs.append({"path": "results/switching_campaign/RAW_RUNS.jsonl.gz", "sha256": hashlib.sha256(compressed).hexdigest()})
raw = [json.loads(line) for line in gzip.decompress(compressed).splitlines()]
check("switching-raw-compressed-identity", hashlib.sha256(compressed).hexdigest() == execution["raw_gzip_sha256"])
check("switching-record-count", len(raw) == len(replay) == summary["records"] == 864)
check("switching-valid-and-success-count", sum(r.get("valid", False) for r in replay) == summary["valid_replays"] == 864 and sum(r.get("solver_success", False) for r in replay) == summary["solver_successes"] == 828)
for index, row in enumerate(raw):
    data = json.loads(row["stdout"])
    ok = hashlib.sha256(row["stdout"].encode()).hexdigest() == row["stdout_sha256"]
    ok &= data["source_commit"] == plan["source_commit"]
    ok &= all(data[k] == row[k] for k in ["case", "mode", "dimension", "rtol", "windows"])
    ok &= data["times"][0] == 0.0 and len(data["times"]) == len(data["states"])
    check("switching-raw-identity-" + str(index), ok)
for cfg in summary["configurations"]:
    rows = [r for r in replay if r["id"] == cfg["id"] and r["phase"] == "timed" and r.get("valid")]
    eligible = len(rows) == plan["repeats"] and all(r["solver_success"] and r["reached"] and r["full_schedule"] for r in rows)
    check("switching-aggregate-" + str(cfg["id"]), cfg["eligible"] == eligible and same(cfg["median_seconds"], statistics.median(r["elapsed_seconds"] for r in rows)) and same(cfg["max_error"], max(r["normalized_error"] for r in rows)))
for comparison in summary["comparisons"]:
    for mode, selected in comparison["best"].items():
        available = [c for c in summary["configurations"] if c["case"] == comparison["case"] and c["dimension"] == comparison["dimension"] and c["mode"] == mode and c["eligible"] and c["max_error"] <= comparison["error_cap"]]
        expected = min(available, key=lambda a: a["median_seconds"])
        check("switching-selection-" + str((comparison["case"], comparison["error_cap"], mode)), all(expected[k] == selected[k] for k in selected))

persistent_plan = read("rhs_persistent_campaign/PLAN.json")
persistent_summary = read("rhs_persistent_campaign/SUMMARY.json")
persistent_replay = read("rhs_persistent_campaign/REPLAY.json")
persistent_execution = read("rhs_persistent_campaign/EXECUTION.json")
parity = read("rhs_persistent_campaign/RHS_PARITY.json")
payload = (RESULTS / "rhs_persistent_campaign/RAW_RUNS.jsonl.gz").read_bytes()
inputs.append({"path": "results/rhs_persistent_campaign/RAW_RUNS.jsonl.gz", "sha256": hashlib.sha256(payload).hexdigest()})
persistent_raw = [json.loads(line) for line in gzip.decompress(payload).splitlines()]
check("persistent-raw-identity-and-count", hashlib.sha256(payload).hexdigest() == persistent_execution["raw_gzip_sha256"] and len(persistent_raw) == persistent_summary["records"] == 324)
check("persistent-success-count", sum(r.get("solver_success", False) for r in persistent_replay) == persistent_summary["solver_successes"] == 288)
paired = defaultdict(dict)
for index, row in enumerate(persistent_raw):
    data = json.loads(row["stdout"])
    ok = hashlib.sha256(row["stdout"].encode()).hexdigest() == row["stdout_sha256"]
    ok &= data["source_commit"] == persistent_plan["source_commit"] == "d4e8623bf8add9f505ec018532c4602e5a838a41"
    ok &= all(data[k] == row[k] for k in ["case", "mode", "dimension", "rtol", "windows"])
    check("persistent-row-identity-" + str(index), ok)
    if data["mode"] in ["switch-rhs", "dopri-whole"]:
        paired[data["case"], data["dimension"], data["rtol"], row["phase"], row["repeat"]][data["mode"]] = data
check("persistent-parity-count", len(paired) == parity["pairs"] == 108)
for key, modes in paired.items():
    a, b = [modes[m] for m in ["switch-rhs", "dopri-whole"]]
    # Serialization also distinguishes signed zero, unlike Python float equality.
    ok = all(json.dumps(a[k]) == json.dumps(b[k]) for k in ["times", "states", "work", "success"])
    ok &= all(a["work"][k] == 0 for k in ["jacobian_builds", "jvp_vectors", "ft_calls", "linear_solves"])
    check("persistent-exact-parity-" + str(key), ok)
for cfg in persistent_summary["configurations"]:
    rows = [r for r in persistent_replay if r["id"] == cfg["id"] and r["phase"] == "timed" and r.get("valid")]
    eligible = len(rows) == persistent_plan["repeats"] and all(r["solver_success"] and r["reached"] and r["full_schedule"] for r in rows)
    check("persistent-aggregate-" + str(cfg["id"]), cfg["eligible"] == eligible and same(cfg["median_seconds"], statistics.median(r["elapsed_seconds"] for r in rows)) and same(cfg["max_error"], max(r["normalized_error"] for r in rows)))

causal = read("causal_benchmark.json")
causal_summary = read("causal_benchmark_summary.json")
for a in causal_summary["rows"]:
    methods = {r["method"]: r for r in causal["rows"] if r["kind"] == a["kind"] and r["dimension"] == a["n"]}
    b, f, p = [methods[k] for k in ["residual-first", "causal-forward", "causal-picard"]]
    ok = same(a["forward_speedup"], b["median_seconds"] / f["median_seconds"])
    ok &= same(a["picard_speedup"], b["median_seconds"] / p["median_seconds"])
    ok &= a["physical_embedded_test"] == [r["embedded_wrms"] <= 1 for r in [b, f, p]]
    check("causal-summary-" + str((a["kind"], a["n"])), ok)
result = {"schema": "vigilode.independent-benchmark-review.v1", "passed": all(c["passed"] for c in checks), "checks": checks, "input_digests": inputs, "scope": "Raw identity, stored replay eligibility, aggregation and achieved-error selection; no solver/reference executions repeated.", "causal_root_successes_with_embedded_error_over_one": sum(r["success"] and r["embedded_wrms"] > 1 for r in causal["rows"])}
(RESULTS / "independent_benchmark_review.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({"passed": result["passed"], "checks": len(checks), "failed": [c["id"] for c in checks if not c["passed"]]}))
raise SystemExit(0 if result["passed"] else 1)
