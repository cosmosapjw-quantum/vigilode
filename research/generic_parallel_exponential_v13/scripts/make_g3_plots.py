#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
TABLES = ROOT / "tables"
PLOTS = ROOT / "plots"
RESULTS = ROOT / "results"
PRIMARY = "pexprb54s4-fused-full-mgs-t1"

PLOTS.mkdir(parents=True, exist_ok=True)


def read_csv(name: str):
    with (TABLES / name).open() as handle:
        return list(csv.DictReader(handle))


def save(fig, name: str):
    fig.tight_layout()
    fig.savefig(PLOTS / name, dpi=180, bbox_inches="tight")
    plt.close(fig)


pareto = read_csv("G3_SAME_ERROR_PARETO.csv")
primary = [row for row in pareto if row["candidate_id"] == PRIMARY]
problem_order = ["complex-dahlquist", "advection-diffusion", "quadratic", "oscillatory-pr"]
labels = []
speedups = []
step_ratios = []
for problem in problem_order:
    rows = sorted(
        [row for row in primary if row["problem_id"] == problem],
        key=lambda row: int(row["target_index"]),
    )
    for row in rows:
        labels.append(f"{problem}\nT{row['target_index']}")
        if row["attained"] == "True":
            speedups.append(float(row["wall_speedup"]))
            step_ratios.append(float(row["step_ratio"]))
        else:
            speedups.append(float("nan"))
            step_ratios.append(float("nan"))

fig, ax = plt.subplots(figsize=(11, 5))
ax.plot(range(len(labels)), speedups)
ax.axhline(1.0)
ax.axhline(1.15)
ax.set_yscale("log")
ax.set_xticks(range(len(labels)), labels, rotation=35, ha="right")
ax.set_ylabel("RODAS wall / pexprb54s4 wall")
ax.set_title("G3 primary same-error speedup by problem and target")
ax.grid(True, which="both", alpha=0.25)
save(fig, "G3_PRIMARY_SAME_ERROR_SPEEDUP.png")

fig, ax = plt.subplots(figsize=(11, 5))
ax.plot(range(len(labels)), step_ratios)
ax.axhline(1.0)
ax.axhline(1.2)
ax.set_yscale("log")
ax.set_xticks(range(len(labels)), labels, rotation=35, ha="right")
ax.set_ylabel("pexprb54s4 accepted steps / RODAS accepted steps")
ax.set_title("G3 primary step economy at matched external error")
ax.grid(True, which="both", alpha=0.25)
save(fig, "G3_PRIMARY_STEP_RATIO.png")

primary_problem = read_csv("G3_PRIMARY_BY_PROBLEM.csv")
fig, ax = plt.subplots(figsize=(8, 5))
problems = [row["problem_id"] for row in primary_problem]
values = [float(row["median_speedup"]) for row in primary_problem]
ax.bar(problems, values)
ax.axhline(1.0)
ax.axhline(1.15)
ax.set_yscale("log")
ax.set_ylabel("Median same-error speedup")
ax.set_title("G3 primary speedup is strongly regime dependent")
ax.tick_params(axis="x", rotation=30)
ax.grid(True, axis="y", which="both", alpha=0.25)
save(fig, "G3_PRIMARY_SPEEDUP_BY_PROBLEM.png")

phi = read_csv("G3_PHI_FUSION_CELLS.csv")
by_orth = defaultdict(list)
for row in phi:
    by_orth[row["orthogonalization"]].append(row)
orth_names = sorted(by_orth)
wall_medians = []
jvp_medians = []
orth_medians = []
for name in orth_names:
    rows = by_orth[name]
    wall_medians.append(sorted(float(row["wall_speedup_median"]) for row in rows)[len(rows)//2])
    jvp_medians.append(sorted(float(row["jvp_reduction"]) for row in rows)[len(rows)//2])
    orth_medians.append(sorted(float(row["orthogonalization_reduction"]) for row in rows)[len(rows)//2])

fig, ax = plt.subplots(figsize=(7, 5))
ax.bar(orth_names, wall_medians)
ax.axhline(1.0)
ax.set_ylabel("Separate / fused wall time")
ax.set_title("Fused phi-combination wall speedup")
ax.tick_params(axis="x", rotation=20)
ax.grid(True, axis="y", alpha=0.25)
save(fig, "G3_PHI_FUSION_WALL_SPEEDUP.png")

fig, ax = plt.subplots(figsize=(7, 5))
indices = list(range(len(orth_names)))
ax.plot(indices, jvp_medians, label="JVP reduction")
ax.plot(indices, orth_medians, label="Orthogonalization reduction")
ax.axhline(1.0)
ax.set_xticks(indices, orth_names, rotation=20)
ax.set_yscale("log")
ax.set_ylabel("Separate work / fused work")
ax.set_title("Fused phi-combination work reduction")
ax.legend()
ax.grid(True, which="both", alpha=0.25)
save(fig, "G3_PHI_FUSION_WORK_REDUCTION.png")

fig, ax = plt.subplots(figsize=(7, 5))
residual = [float(row["residual_error_estimate"]) for row in phi]
nested = [float(row["nested_difference_estimate"]) for row in phi]
for name in orth_names:
    rows = [row for row in phi if row["orthogonalization"] == name]
    ax.scatter(
        [float(row["nested_difference_estimate"]) for row in rows],
        [float(row["residual_error_estimate"]) for row in rows],
        label=name,
    )
lo = min(min(residual), min(nested))
hi = max(max(residual), max(nested))
ax.plot([lo, hi], [lo, hi])
ax.set_xscale("log")
ax.set_yscale("log")
ax.set_xlabel("Nested-dimension difference")
ax.set_ylabel("Residual-based first-term estimate")
ax.set_title("Residual estimator is not the nested-dimension heuristic")
ax.legend()
ax.grid(True, which="both", alpha=0.25)
save(fig, "G3_RESIDUAL_VS_NESTED_ESTIMATOR.png")

runs = read_csv("G3_AGGREGATED_RUNS.csv")
selected_candidates = [
    "protected-jf-rodas5p",
    PRIMARY,
    "frozen-radau-iia3",
    "frozen-bdf2",
    "held-g1-q1-q2",
]
fig, ax = plt.subplots(figsize=(8, 6))
for candidate in selected_candidates:
    rows = sorted(
        [row for row in runs if row["problem_id"] == "oscillatory-pr" and row["candidate_id"] == candidate and row["success"] == "True"],
        key=lambda row: float(row["endpoint_l2_error"]),
        reverse=True,
    )
    ax.plot(
        [float(row["wall_median"]) for row in rows],
        [float(row["endpoint_l2_error"]) for row in rows],
        label=candidate,
    )
ax.set_xscale("log")
ax.set_yscale("log")
ax.set_xlabel("Median wall time [s]")
ax.set_ylabel("Endpoint L2 error")
ax.set_title("Oscillatory Prothero-Robinson work-precision")
ax.legend(fontsize=8)
ax.grid(True, which="both", alpha=0.25)
save(fig, "G3_OSCILLATORY_PR_WORK_PRECISION.png")

before_after_path = TABLES / "G3_RESIDUAL_ESTIMATOR_BEFORE_AFTER.csv"
if before_after_path.exists():
    before_after = read_csv("G3_RESIDUAL_ESTIMATOR_BEFORE_AFTER.csv")
    rows = [row for row in before_after if row["candidate_id"] == PRIMARY]
    xlabels = [f"{row['problem_id']}\n{float(row['rtol']):.0e}" for row in rows]
    ratios = [float(row["new_over_old_wall"]) for row in rows]
    fig, ax = plt.subplots(figsize=(10, 5))
    ax.bar(xlabels, ratios)
    ax.axhline(1.0)
    ax.set_yscale("log")
    ax.set_ylabel("Residual-estimator wall / nested-heuristic wall")
    ax.set_title("Controller cost of replacing nested difference by a residual estimate")
    ax.tick_params(axis="x", rotation=35)
    ax.grid(True, axis="y", which="both", alpha=0.25)
    save(fig, "G3_RESIDUAL_ESTIMATOR_BEFORE_AFTER.png")

print(f"wrote plots to {PLOTS}")

# Within-step T1/T4 comparison for the full-MGS primary. Values below one
# mean that the four-thread path is slower than the one-thread path.
full_t1 = {
    (row["problem_id"], float(row["rtol"])): row
    for row in runs
    if row["candidate_id"] == "pexprb54s4-fused-full-mgs-t1" and row["success"] == "True"
}
full_t4 = {
    (row["problem_id"], float(row["rtol"])): row
    for row in runs
    if row["candidate_id"] == "pexprb54s4-fused-full-mgs-t4" and row["success"] == "True"
}
thread_by_problem = defaultdict(list)
for key in sorted(set(full_t1) & set(full_t4)):
    left, right = full_t1[key], full_t4[key]
    thread_by_problem[key[0]].append(float(left["wall_median"]) / float(right["wall_median"]))
fig, ax = plt.subplots(figsize=(8, 5))
thread_problems = sorted(thread_by_problem)
thread_values = [sorted(thread_by_problem[p])[len(thread_by_problem[p]) // 2] for p in thread_problems]
ax.bar(thread_problems, thread_values)
ax.axhline(1.0)
ax.set_ylabel("T1 wall / T4 wall")
ax.set_title("Within-step four-thread path is not beneficial on the current small-state corpus")
ax.tick_params(axis="x", rotation=30)
ax.grid(True, axis="y", alpha=0.25)
save(fig, "G3_FULL_MGS_T1_T4_SPEEDUP.png")

# Preserve the real work spent by failed IOP2 attempts rather than plotting
# them as zero-cost failures.
failed_iop = [
    row for row in runs
    if row["candidate_id"].startswith("pexprb54s4-fused-iop2") and row["success"] == "False"
]
if failed_iop:
    labels = [row["candidate_id"].replace("pexprb54s4-fused-", "") for row in failed_iop]
    rhs = [float(row["rhs_evaluations"]) for row in failed_iop]
    jvp = [float(row["jvp_vectors"]) for row in failed_iop]
    fig, ax = plt.subplots(figsize=(7, 5))
    indices = list(range(len(labels)))
    width = 0.35
    ax.bar([i - width / 2 for i in indices], rhs, width=width, label="RHS evaluations")
    ax.bar([i + width / 2 for i in indices], jvp, width=width, label="JVP vectors")
    ax.set_xticks(indices, labels, rotation=20)
    ax.set_yscale("log")
    ax.set_ylabel("Attempted work retained")
    ax.set_title("Failed incomplete-orthogonalization runs are not zero-cost")
    ax.legend()
    ax.grid(True, axis="y", which="both", alpha=0.25)
    save(fig, "G3_IOP_FAILURE_WORK.png")
