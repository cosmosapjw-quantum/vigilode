#!/usr/bin/env python3
from __future__ import annotations

import csv
import glob
import hashlib
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "raw"
RESULTS = ROOT / "results"
PLOTS = ROOT / "plots"
RESULTS.mkdir(parents=True, exist_ok=True)
PLOTS.mkdir(parents=True, exist_ok=True)

campaign_paths = sorted(RAW.glob("G1_ADAPTIVE_T1_CAMPAIGN_*.json"))
campaigns = [json.loads(path.read_text()) for path in campaign_paths]
one_step = json.loads((RAW / "G1_ONE_STEP_CANONICAL.json").read_text())
outer_t4_paths = sorted(RAW.glob("G1_ADAPTIVE_OUTER_T4_*.json"))
outer_t4_docs = [json.loads(path.read_text()) for path in outer_t4_paths]
frozen_context_paths = sorted(RAW.glob("G1_FROZEN_CONTEXT_T1_*.json"))
frozen_context_docs = [json.loads(path.read_text()) for path in frozen_context_paths]
checksums = [doc["scientific_checksum"] for doc in campaigns]
if len(set(checksums)) != 1:
    raise SystemExit(f"T1 scientific checksums differ: {checksums}")
if not outer_t4_docs or any(doc["scientific_checksum"] != checksums[0] for doc in outer_t4_docs):
    raise SystemExit("outer T4 scientific checksum differs from T1")
if frozen_context_docs:
    frozen_checksums = [doc["scientific_checksum"] for doc in frozen_context_docs]
    if len(set(frozen_checksums)) != 1:
        raise SystemExit(f"frozen comparator checksums differ: {frozen_checksums}")
else:
    frozen_checksums = []

walls: dict[str, list[float]] = defaultdict(list)
rows: dict[str, dict] = {}
for document in campaigns:
    for row in document["runs"]:
        rows[row["record_id"]] = row
        walls[row["record_id"]].append(row["wall_seconds"])

for key, values in walls.items():
    if len(values) != len(campaigns):
        raise SystemExit(f"missing timing campaign for {key}")

def median(values):
    return statistics.median(values) if values else None

def quantile(values, q):
    if not values:
        return None
    ordered = sorted(values)
    pos = q * (len(ordered) - 1)
    lo, hi = math.floor(pos), math.ceil(pos)
    frac = pos - lo
    return ordered[lo] * (1.0 - frac) + ordered[hi] * frac

BASELINE = "protected-sequential-jf-rodas5p-gmres-adaptive"
T1 = "transactional-q1-q2-rodas5p-t1-adaptive"
T4 = "transactional-q1-q2-rodas5p-t4-adaptive"
BDF2 = "bdf2-adaptive-reference"
RADAU3 = "radau-iia3-adaptive-reference"
CANDIDATES = [BASELINE, T1, T4, BDF2, RADAU3]

# Stable median timing table.
timing_rows = []
for record_id, row in sorted(rows.items()):
    if row["candidate_id"] not in CANDIDATES:
        continue
    timing_rows.append(
        {
            "record_id": record_id,
            "candidate_id": row["candidate_id"],
            "problem_id": row["problem_id"],
            "rtol": row["rtol"],
            "status": row["status"],
            "max_grid_l2": None if row["errors"] is None else row["errors"]["max_grid_l2"],
            "wall_median_seconds": median(walls[record_id]),
            "wall_min_seconds": min(walls[record_id]),
            "wall_max_seconds": max(walls[record_id]),
            "internal_steps": row["work"]["internal_steps"],
            "rhs_evaluations": row["work"]["counters"]["rhs_evaluations"],
            "jvp_vectors": row["work"]["counters"]["jvp_vectors"],
            "jacobian_builds": row["work"]["counters"]["jacobian_builds"],
            "direct_factorizations": row["work"]["counters"]["direct_factorizations"],
        }
    )

with (RESULTS / "G1_ADAPTIVE_TIMING_ROWS.csv").open("w", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=list(timing_rows[0]))
    writer.writeheader()
    writer.writerows(timing_rows)

# Same-rtol and same-external-error comparisons.
def base_record_for(row: dict) -> str:
    return row["record_id"].replace(row["candidate_id"], BASELINE)

candidate_summary = []
for candidate in CANDIDATES:
    selected = [row for row in rows.values() if row["candidate_id"] == candidate]
    failures = sum(row["status"] != "success" for row in selected)
    same_rtol_speedups = []
    step_ratios = []
    for row in selected:
        if row["status"] != "success":
            continue
        base_id = base_record_for(row)
        if base_id not in rows or rows[base_id]["status"] != "success":
            continue
        same_rtol_speedups.append(median(walls[base_id]) / median(walls[row["record_id"]]))
        bsteps = rows[base_id]["work"]["internal_steps"]
        if bsteps:
            step_ratios.append(row["work"]["internal_steps"] / bsteps)

    same_error_speedups = []
    same_error_step_ratios = []
    same_error_attainments = 0
    baseline_targets = [row for row in rows.values() if row["candidate_id"] == BASELINE and row["status"] == "success"]
    for baseline_row in baseline_targets:
        target = baseline_row["errors"]["max_grid_l2"]
        options = [
            row
            for row in selected
            if row["problem_id"] == baseline_row["problem_id"]
            and row["status"] == "success"
            and row["errors"]["max_grid_l2"] <= target * (1.0 + 1.0e-12)
        ]
        if not options:
            continue
        choice = min(options, key=lambda row: median(walls[row["record_id"]]))
        same_error_attainments += 1
        same_error_speedups.append(
            median(walls[baseline_row["record_id"]]) / median(walls[choice["record_id"]])
        )
        same_error_step_ratios.append(
            choice["work"]["internal_steps"] / baseline_row["work"]["internal_steps"]
        )

    candidate_summary.append(
        {
            "candidate_id": candidate,
            "runs": len(selected),
            "failures": failures,
            "same_rtol_speedup_median": median(same_rtol_speedups),
            "same_rtol_speedup_min": min(same_rtol_speedups) if same_rtol_speedups else None,
            "same_rtol_speedup_max": max(same_rtol_speedups) if same_rtol_speedups else None,
            "step_ratio_median": median(step_ratios),
            "same_error_attainments": same_error_attainments,
            "same_error_targets": len(baseline_targets),
            "same_error_speedup_median": median(same_error_speedups),
            "same_error_step_ratio_median": median(same_error_step_ratios),
        }
    )

with (RESULTS / "G1_CANDIDATE_SUMMARY.csv").open("w", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=list(candidate_summary[0]))
    writer.writeheader()
    writer.writerows(candidate_summary)

# Aggregate transactional accepted-step and critical-depth facts.
transactional_summary = {}
for candidate in [T1, T4]:
    q1 = q2 = fallback = 0
    depths = []
    w_batches = []
    explicit_j = direct_factorizations = nonlinear_iterations = 0
    by_problem = {}
    for problem in sorted({row["problem_id"] for row in rows.values()}):
        prow = [row for row in rows.values() if row["candidate_id"] == candidate and row["problem_id"] == problem]
        pq1 = pq2 = pfb = 0
        pdepths = []
        for row in prow:
            tr = row["transactional"]
            pq1 += tr["accepted_q1_fast_steps"]
            pq2 += tr["accepted_q2_escalated_steps"]
            pfb += tr["accepted_sequential_fallback_steps"]
            pdepths.extend(tr["accepted_critical_path_depths"])
            q1 += tr["accepted_q1_fast_steps"]
            q2 += tr["accepted_q2_escalated_steps"]
            fallback += tr["accepted_sequential_fallback_steps"]
            depths.extend(tr["accepted_critical_path_depths"])
            w_batches.extend(tr["accepted_w_solve_batches"])
            counters = row["work"]["counters"]
            explicit_j += counters["jacobian_builds"]
            direct_factorizations += counters["direct_factorizations"]
            nonlinear_iterations += counters["nonlinear_iterations"]
        total = pq1 + pq2 + pfb
        by_problem[problem] = {
            "accepted_steps": total,
            "q1_fast": pq1,
            "q2_escalated": pq2,
            "fallback": pfb,
            "fast_fraction": 0.0 if total == 0 else (pq1 + pq2) / total,
            "fallback_fraction": 0.0 if total == 0 else pfb / total,
            "median_critical_path_depth": median(pdepths),
            "p95_critical_path_depth": quantile(pdepths, 0.95),
        }
    total = q1 + q2 + fallback
    transactional_summary[candidate] = {
        "accepted_steps": total,
        "q1_fast": q1,
        "q2_escalated": q2,
        "fallback": fallback,
        "fast_fraction": (q1 + q2) / total,
        "strict_sub8_fraction": q1 / total,
        "fallback_fraction": fallback / total,
        "median_critical_path_depth": median(depths),
        "p95_critical_path_depth": quantile(depths, 0.95),
        "median_w_solve_batches": median(w_batches),
        "explicit_jacobian_builds": explicit_j,
        "direct_factorizations": direct_factorizations,
        "nonlinear_iterations": nonlinear_iterations,
        "by_problem": by_problem,
    }

one_summary = one_step["summary"]
kill_reasons = []
if one_summary["fast_fraction"] < 0.9:
    kill_reasons.append("one-step fast fraction below 0.90")
if one_summary["fallback_fraction"] > 0.1:
    kill_reasons.append("one-step fallback fraction above 0.10")
if one_summary["p95_critical_path_depth"] > 6:
    kill_reasons.append("one-step P95 critical path exceeds target 6")
for candidate in [T1, T4]:
    tx = transactional_summary[candidate]
    if tx["fast_fraction"] < 0.9:
        kill_reasons.append(f"{candidate} adaptive fast fraction below 0.90")
    if tx["fallback_fraction"] > 0.1:
        kill_reasons.append(f"{candidate} adaptive fallback above 0.10")
    summary = next(row for row in candidate_summary if row["candidate_id"] == candidate)
    if not (summary["same_error_speedup_median"] and summary["same_error_speedup_median"] >= 1.15):
        kill_reasons.append(f"{candidate} same-error speedup below 1.15x")

paired_cost = {}
for candidate in [T1, T4]:
    wall_ratios = []
    rhs_ratios = []
    jvp_ratios = []
    step_ratios = []
    error_ratios = []
    for row in [r for r in rows.values() if r["candidate_id"] == candidate and r["status"] == "success"]:
        base_id = base_record_for(row)
        if base_id not in rows or rows[base_id]["status"] != "success":
            continue
        base = rows[base_id]
        wall_ratios.append(median(walls[row["record_id"]]) / median(walls[base_id]))
        rhs_ratios.append(row["work"]["counters"]["rhs_evaluations"] / max(base["work"]["counters"]["rhs_evaluations"], 1))
        jvp_ratios.append(row["work"]["counters"]["jvp_vectors"] / max(base["work"]["counters"]["jvp_vectors"], 1))
        step_ratios.append(row["work"]["internal_steps"] / max(base["work"]["internal_steps"], 1))
        error_ratios.append(row["errors"]["max_grid_l2"] / max(base["errors"]["max_grid_l2"], 1e-300))
    paired_cost[candidate] = {
        "candidate_over_baseline_wall_median": median(wall_ratios),
        "baseline_over_candidate_wall_speedup_median": None if not wall_ratios else 1.0 / median(wall_ratios),
        "rhs_ratio_median": median(rhs_ratios),
        "jvp_ratio_median": median(jvp_ratios),
        "step_ratio_median": median(step_ratios),
        "error_ratio_median": median(error_ratios),
    }

# Repeated frozen SABR/q7 context on the overlapping legacy four-problem corpus.
frozen_context = {}
if frozen_context_docs:
    frozen_walls: dict[str, list[float]] = defaultdict(list)
    frozen_rows: dict[str, dict] = {}
    for document in frozen_context_docs:
        for row in document["runs"]:
            frozen_rows[row["record_id"]] = row
            frozen_walls[row["record_id"]].append(row["wall_seconds"])
    baseline_by_key = {
        (row["problem_id"], row["rtol"].hex()): row
        for row in rows.values()
        if row["candidate_id"] == BASELINE and row["status"] == "success"
    }
    frozen_ids = ["sabr5p-adaptive", "homotopy-rodas5p-q7-adaptive"]
    frozen_table = []
    for candidate in frozen_ids:
        selected = [row for row in frozen_rows.values() if row["candidate_id"] == candidate]
        same_rtol_speedups = []
        same_rtol_step_ratios = []
        for row in selected:
            if row["status"] != "success":
                continue
            base = baseline_by_key.get((row["problem_id"], row["rtol"].hex()))
            if base is None:
                continue
            same_rtol_speedups.append(
                median(walls[base["record_id"]]) / median(frozen_walls[row["record_id"]])
            )
            same_rtol_step_ratios.append(
                row["work"]["internal_steps"] / max(base["work"]["internal_steps"], 1)
            )
        attainments = 0
        same_error_speedups = []
        same_error_step_ratios = []
        targets = [base for base in baseline_by_key.values() if any(
            r["problem_id"] == base["problem_id"] for r in selected
        )]
        for base in targets:
            target = base["errors"]["max_grid_l2"]
            options = [
                row for row in selected
                if row["problem_id"] == base["problem_id"]
                and row["status"] == "success"
                and row["errors"]["max_grid_l2"] <= target * (1.0 + 1.0e-12)
            ]
            if not options:
                continue
            choice = min(options, key=lambda row: median(frozen_walls[row["record_id"]]))
            attainments += 1
            same_error_speedups.append(
                median(walls[base["record_id"]]) / median(frozen_walls[choice["record_id"]])
            )
            same_error_step_ratios.append(
                choice["work"]["internal_steps"] / max(base["work"]["internal_steps"], 1)
            )
        frozen_table.append({
            "candidate_id": candidate,
            "runs": len(selected),
            "failures": sum(row["status"] != "success" for row in selected),
            "same_rtol_speedup_median": median(same_rtol_speedups),
            "same_rtol_step_ratio_median": median(same_rtol_step_ratios),
            "same_error_attainments": attainments,
            "same_error_targets": len(targets),
            "same_error_speedup_median": median(same_error_speedups),
            "same_error_step_ratio_median": median(same_error_step_ratios),
        })
    frozen_context = {
        "campaigns": len(frozen_context_docs),
        "scientific_checksum": frozen_checksums[0],
        "scope": "legacy four-problem context only; not part of the bounded G1 decision set",
        "rows": frozen_table,
    }
    with (RESULTS / "G1_FROZEN_COMPARATOR_CONTEXT.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(frozen_table[0]))
        writer.writeheader()
        writer.writerows(frozen_table)

timing_campaign = {
    "t1_campaigns": len(campaigns),
    "t4_campaigns": len(outer_t4_docs),
    "t1_suite_median_seconds": median([doc["execution"]["scientific_suite_wall_seconds"] for doc in campaigns]),
    "t4_suite_median_seconds": median([doc["execution"]["scientific_suite_wall_seconds"] for doc in outer_t4_docs]),
}
timing_campaign["case_level_throughput_speedup"] = timing_campaign["t1_suite_median_seconds"] / timing_campaign["t4_suite_median_seconds"]

analysis = {
    "schema": "generic-q1-q2-g1-analysis-v2",
    "scientific_checksum": checksums[0],
    "campaigns": len(campaigns),
    "campaign_checksums_identical": len(set(checksums)) == 1,
    "outer_t4_campaigns": len(outer_t4_docs),
    "outer_t4_checksum_identical": all(doc["scientific_checksum"] == checksums[0] for doc in outer_t4_docs),
    "one_step": one_summary,
    "adaptive": {
        "problems": len(campaigns[0]["problems"]),
        "runs": len(campaigns[0]["runs"]),
        "candidate_summary": candidate_summary,
        "transactional": transactional_summary,
    },
    "order_gate": {
        "status": "not-promoted-after-earlier-kill",
        "note": "The candidate already fails fast-fraction, fallback, critical-depth, and same-error speed gates. The adaptive path mixes q2 and protected fallback; no independent all-fast fifth-order theorem or promotion claim is made.",
    },
    "paired_same_control": paired_cost,
    "timing_campaign": timing_campaign,
    "frozen_comparators": {
        "bdf2": "context comparator only; source frozen by G0 regression test",
        "radau_iia3": "context comparator only; source frozen by G0 regression test",
        "sabr_q7_context": frozen_context,
    },
    "promotion_gates": {
        "false_convergence_max": 0,
        "fast_fraction_min": 0.9,
        "fallback_fraction_max": 0.1,
        "median_batch_depth_max": 8,
        "p95_batch_depth_target": 6,
        "global_order_min": 4.8,
        "same_error_speedup_min": 1.15,
        "step_ratio_max": 1.2,
    },
    "decision": "HOLD_Q1_Q2_AND_OPEN_PARALLEL_EXPONENTIAL_SIBLING",
    "kill_reasons": sorted(set(kill_reasons)),
}
(RESULTS / "G1_ANALYSIS.json").write_text(json.dumps(analysis, indent=2) + "\n")

# Plots.
labels = [
    "one-step",
    "adaptive t1",
    "adaptive t4",
]
fast = [
    one_summary["fast_fraction"],
    transactional_summary[T1]["fast_fraction"],
    transactional_summary[T4]["fast_fraction"],
]
fallback = [
    one_summary["fallback_fraction"],
    transactional_summary[T1]["fallback_fraction"],
    transactional_summary[T4]["fallback_fraction"],
]
fig, ax = plt.subplots(figsize=(7.2, 4.4))
x = range(len(labels))
ax.bar(x, fast, label="q1/q2 accepted")
ax.bar(x, fallback, bottom=fast, label="protected fallback")
ax.axhline(0.9, linestyle="--", linewidth=1.2, label="fast-fraction gate")
ax.set_xticks(list(x), labels)
ax.set_ylim(0, 1.05)
ax.set_ylabel("fraction of completed/accepted steps")
ax.set_title("Transactional q1→q2 acceptance composition")
ax.legend(loc="upper right")
fig.tight_layout()
fig.savefig(PLOTS / "G1_FAST_FALLBACK_FRACTION.png", dpi=180)
plt.close(fig)

speed_candidates = [T1, T4, BDF2, RADAU3]
speed_labels = ["q1→q2 t1", "q1→q2 t4", "BDF2", "Radau IIA3"]
speed_rows = [next(row for row in candidate_summary if row["candidate_id"] == cid) for cid in speed_candidates]
speeds = [row["same_error_speedup_median"] or 0.0 for row in speed_rows]
coverage = [row["same_error_attainments"] / row["same_error_targets"] for row in speed_rows]
fig, ax = plt.subplots(figsize=(7.2, 4.4))
bars = ax.bar(range(len(speed_labels)), speeds)
ax.axhline(1.0, linestyle=":", linewidth=1.2, label="protected JF RODAS parity")
ax.axhline(1.15, linestyle="--", linewidth=1.2, label="promotion gate")
for bar, cov in zip(bars, coverage):
    ax.annotate(
        f"coverage {cov:.1%}",
        (bar.get_x() + bar.get_width() / 2, bar.get_height()),
        xytext=(0, 4),
        textcoords="offset points",
        ha="center",
        va="bottom",
        fontsize=8,
    )
ax.set_xticks(range(len(speed_labels)), speed_labels, rotation=15, ha="right")
ax.set_ylabel("same-error speedup vs protected JF RODAS5P")
ax.set_title("Matched external-error wall-time comparison")
ax.legend(loc="upper left")
ax.set_ylim(0.0, max(max(speeds) * 1.18, 1.3))
fig.tight_layout()
fig.savefig(PLOTS / "G1_SAME_ERROR_SPEEDUP.png", dpi=180)
plt.close(fig)

problem_order = sorted(transactional_summary[T1]["by_problem"])
problem_labels = [name.replace("manufactured-", "mfg-").replace("advection-diffusion-", "adv-diff-") for name in problem_order]
problem_fast = [transactional_summary[T1]["by_problem"][name]["fast_fraction"] for name in problem_order]
fig, ax = plt.subplots(figsize=(9.2, 4.8))
ax.bar(range(len(problem_order)), problem_fast)
ax.axhline(0.9, linestyle="--", linewidth=1.2, label="fast-fraction gate")
ax.set_xticks(range(len(problem_order)), problem_labels, rotation=35, ha="right")
ax.set_ylim(0, 1.05)
ax.set_ylabel("fast fraction")
ax.set_title("Fast-path coverage by generic problem family")
ax.legend()
fig.tight_layout()
fig.savefig(PLOTS / "G1_FAST_FRACTION_BY_PROBLEM.png", dpi=180)
plt.close(fig)


# Critical-path depth summary.
depth_labels = ["one-step", "adaptive t1", "adaptive t4"]
depth_medians = [
    one_summary["median_critical_path_depth"],
    transactional_summary[T1]["median_critical_path_depth"],
    transactional_summary[T4]["median_critical_path_depth"],
]
depth_p95 = [
    one_summary["p95_critical_path_depth"],
    transactional_summary[T1]["p95_critical_path_depth"],
    transactional_summary[T4]["p95_critical_path_depth"],
]
fig, ax = plt.subplots(figsize=(7.2, 4.4))
x = list(range(len(depth_labels)))
width = 0.36
ax.bar([value - width / 2 for value in x], depth_medians, width=width, label="median")
ax.bar([value + width / 2 for value in x], depth_p95, width=width, label="P95")
ax.axhline(8.0, linestyle=":", linewidth=1.2, label="sequential RODAS depth")
ax.axhline(6.0, linestyle="--", linewidth=1.2, label="P95 target")
ax.set_xticks(x, depth_labels)
ax.set_ylabel("critical-path common-W batches")
ax.set_title("Transactional critical-path depth")
ax.legend(loc="upper left")
fig.tight_layout()
fig.savefig(PLOTS / "G1_CRITICAL_PATH_DEPTH.png", dpi=180)
plt.close(fig)

# Same-control work ratios expose whether step economy is overwhelmed by within-step work.
def paired_ratio(candidate: str, field: str) -> float:
    values = []
    for row in rows.values():
        if row["candidate_id"] != candidate or row["status"] != "success":
            continue
        base_id = base_record_for(row)
        if base_id not in rows or rows[base_id]["status"] != "success":
            continue
        if field == "wall":
            base_value = median(walls[base_id])
            candidate_value = median(walls[row["record_id"]])
        elif field == "rhs":
            base_value = rows[base_id]["work"]["counters"]["rhs_evaluations"]
            candidate_value = row["work"]["counters"]["rhs_evaluations"]
        elif field == "jvp":
            base_value = rows[base_id]["work"]["counters"]["jvp_vectors"]
            candidate_value = row["work"]["counters"]["jvp_vectors"]
        elif field == "steps":
            base_value = rows[base_id]["work"]["internal_steps"]
            candidate_value = row["work"]["internal_steps"]
        else:
            raise ValueError(field)
        if base_value:
            values.append(candidate_value / base_value)
    return median(values)

ratio_fields = ["wall", "rhs", "jvp", "steps"]
ratio_labels = ["wall time", "RHS", "JVP vectors", "steps"]
ratio_t1 = [paired_ratio(T1, field) for field in ratio_fields]
ratio_t4 = [paired_ratio(T4, field) for field in ratio_fields]
fig, ax = plt.subplots(figsize=(7.4, 4.5))
x = list(range(len(ratio_fields)))
width = 0.36
ax.bar([value - width / 2 for value in x], ratio_t1, width=width, label="q1→q2 t1")
ax.bar([value + width / 2 for value in x], ratio_t4, width=width, label="q1→q2 t4")
ax.axhline(1.0, linestyle=":", linewidth=1.2, label="protected JF RODAS parity")
ax.set_xticks(x, ratio_labels)
ax.set_yscale("log")
ax.set_ylabel("candidate / protected JF RODAS5P")
ax.set_title("Same-control cost ratios")
ax.legend(loc="upper right")
fig.tight_layout()
fig.savefig(PLOTS / "G1_PAIRED_COST_RATIOS.png", dpi=180)
plt.close(fig)

# Provenance checksums for the raw inputs used by the analysis.
raw_hashes = {}
for path in sorted(RAW.glob("*.json")):
    raw_hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
(RESULTS / "G1_RAW_SHA256.json").write_text(json.dumps(raw_hashes, indent=2) + "\n")
print(json.dumps(analysis, indent=2))
