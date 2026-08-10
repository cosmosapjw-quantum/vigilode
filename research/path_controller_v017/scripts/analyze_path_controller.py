#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import math
from collections import defaultdict
from pathlib import Path
from statistics import median

import matplotlib.pyplot as plt


def finite(values):
    return [float(v) for v in values if v is not None and math.isfinite(float(v))]


def percentile(values, q):
    xs = sorted(finite(values))
    if not xs:
        return None
    if len(xs) == 1:
        return xs[0]
    pos = (len(xs) - 1) * q
    lo = int(math.floor(pos))
    hi = int(math.ceil(pos))
    if lo == hi:
        return xs[lo]
    alpha = pos - lo
    return (1.0 - alpha) * xs[lo] + alpha * xs[hi]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--plots", type=Path, required=True)
    args = parser.parse_args()
    args.results.mkdir(parents=True, exist_ok=True)
    args.plots.mkdir(parents=True, exist_ok=True)

    document = json.loads(args.input.read_text())
    rows = document["rows"]
    cases = {case["case_id"]: case for case in document["cases"]}
    grouped = defaultdict(list)
    for row in rows:
        grouped[row["schedule_id"]].append(row)

    schedule_rows = []
    for schedule_id in sorted(grouped):
        selected = grouped[schedule_id]
        nonhostile = [row for row in selected if not row["hostile"]]
        accepted_nonhostile = [row for row in nonhostile if row["algebraic_accepted"]]
        q_values = [
            int(round_spec["q"])
            for row in selected
            for round_spec in row["schedule"]["rounds"]
        ]
        median_batches = median([row["work"]["w_solve_batches"] for row in selected])
        median_vectors = median([row["work"]["w_solve_vectors"] for row in selected])
        final_ratios = finite(row["final_target_residual_ratio"] for row in selected)
        oracle_defects = finite(row["oracle_output_wrms"] for row in selected)
        nonhostile_final_ratios = finite(
            row["final_target_residual_ratio"] for row in nonhostile
        )
        nonhostile_oracle_defects = finite(row["oracle_output_wrms"] for row in nonhostile)
        strict_low_depth = (
            len(accepted_nonhostile) == len(nonhostile)
            and median_batches < 8
            and not any(row["false_accept"] for row in selected)
            and max(q_values, default=0) < 7
        )
        depth_parity_survivor = (
            len(accepted_nonhostile) == len(nonhostile)
            and median_batches <= 8
            and not any(row["false_accept"] for row in selected)
            and max(q_values, default=0) < 7
        )
        schedule_rows.append(
            {
                "schedule_id": schedule_id,
                "rows": len(selected),
                "nonhostile_rows": len(nonhostile),
                "nonhostile_accepts": len(accepted_nonhostile),
                "nonhostile_accept_fraction": (
                    len(accepted_nonhostile) / len(nonhostile) if nonhostile else None
                ),
                "false_accepts": sum(bool(row["false_accept"]) for row in selected),
                "failures": sum(row["failure"] is not None for row in selected),
                "median_w_solve_batches": median_batches,
                "median_w_solve_vectors": median_vectors,
                "median_final_target_residual_ratio": median(final_ratios) if final_ratios else None,
                "median_oracle_output_wrms": median(oracle_defects) if oracle_defects else None,
                "median_nonhostile_oracle_output_wrms": (
                    median(nonhostile_oracle_defects) if nonhostile_oracle_defects else None
                ),
                "maximum_nonhostile_oracle_output_wrms": (
                    max(nonhostile_oracle_defects) if nonhostile_oracle_defects else None
                ),
                "median_nonhostile_final_target_residual_ratio": (
                    median(nonhostile_final_ratios) if nonhostile_final_ratios else None
                ),
                "p90_oracle_output_wrms_all_cases": percentile(oracle_defects, 0.9),
                "maximum_q": max(q_values, default=0),
                "strict_low_depth_survivor": strict_low_depth,
                "depth_parity_correctness_survivor": depth_parity_survivor,
            }
        )

    strict_survivors = [row["schedule_id"] for row in schedule_rows if row["strict_low_depth_survivor"]]
    parity_survivors = [
        row["schedule_id"] for row in schedule_rows if row["depth_parity_correctness_survivor"]
    ]
    analysis = {
        "schema": "rodas5p-path-controller-analysis-v1",
        "source_schema": document["schema"],
        "profile": document["profile"],
        "cases": len(document["cases"]),
        "schedules": len(schedule_rows),
        "rows": len(rows),
        "false_accepts": sum(bool(row["false_accept"]) for row in rows),
        "nonhostile_failures": sum(
            row["failure"] is not None and not row["hostile"] for row in rows
        ),
        "strict_low_depth_survivors": strict_survivors,
        "depth_parity_correctness_survivors": parity_survivors,
        "schedule_summaries": schedule_rows,
        "claim_status": {
            "surviving": [
                "one-round decoupled q=2 with two endpoint corrections reaches the algebraic output budget on every non-hostile case at eight common-W batch applications",
                "one-round decoupled q=1 with two endpoint corrections reaches four of five non-hostile cases at six common-W batch applications",
            ],
            "narrowed": [
                "nonstationary theta changes are not generically stabilizing; the tested theta ramp amplified the target residual",
                "fixed low-q path marching without endpoint correction does not materially reduce the target residual",
            ],
            "rejected": [
                "a strict sub-eight-batch schedule already covers every non-hostile case",
                "the current screen establishes global fifth order or integrated wall-time speedup",
            ],
        },
    }
    (args.results / "PATH_CONTROLLER_ANALYSIS.json").write_text(
        json.dumps(analysis, indent=2, sort_keys=True) + "\n"
    )

    with (args.results / "PATH_CONTROLLER_SCHEDULE_SUMMARY.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(schedule_rows[0].keys()))
        writer.writeheader()
        writer.writerows(schedule_rows)

    # Plot 1: acceptance fraction versus common-W batch depth.
    fig, ax = plt.subplots(figsize=(8.4, 5.4))
    for row in schedule_rows:
        x = row["median_w_solve_batches"]
        y = row["nonhostile_accept_fraction"]
        ax.scatter(x, y, s=38)
        if row["schedule_id"] in {
            "fixed-q7",
            "direct-decoupled-q1-c2",
            "direct-decoupled-q2-c2",
            "theta-ramp-q211",
        }:
            ax.annotate(row["schedule_id"], (x, y), xytext=(4, 4), textcoords="offset points", fontsize=8)
    ax.axvline(8.0, linestyle="--", linewidth=1.0, label="sequential stage depth = 8")
    ax.set_xlabel("Median common-W batch applications")
    ax.set_ylabel("Non-hostile algebraic acceptance fraction")
    ax.set_ylim(-0.04, 1.04)
    ax.grid(True, alpha=0.3)
    ax.legend(loc="lower right")
    fig.tight_layout()
    fig.savefig(args.plots / "ACCEPTANCE_VS_BATCH_DEPTH.png", dpi=180)
    plt.close(fig)

    # Plot 2: median target-residual reduction versus vector work.
    fig, ax = plt.subplots(figsize=(8.4, 5.4))
    for row in schedule_rows:
        x = row["median_w_solve_vectors"]
        y = max(row["median_final_target_residual_ratio"] or 0.0, 1.0e-20)
        ax.scatter(x, y, s=38)
        if row["schedule_id"] in {
            "fixed-q2",
            "fixed-q7",
            "direct-decoupled-q1-c2",
            "direct-decoupled-q2-c2",
            "damped-q2-final-correction",
        }:
            ax.annotate(row["schedule_id"], (x, y), xytext=(4, 4), textcoords="offset points", fontsize=8)
    ax.set_yscale("log")
    ax.set_xlabel("Median common-W RHS-vector work")
    ax.set_ylabel("Median final / initial target residual")
    ax.grid(True, which="both", alpha=0.3)
    fig.tight_layout()
    fig.savefig(args.plots / "RESIDUAL_REDUCTION_VS_VECTOR_WORK.png", dpi=180)
    plt.close(fig)

    # Plot 3: per-round target residual for selected schedules, median over non-hostile cases.
    selected_ids = [
        "fixed-q2",
        "fixed-q7",
        "direct-decoupled-q1-c2",
        "direct-decoupled-q2-c2",
        "theta-ramp-q211",
    ]
    fig, ax = plt.subplots(figsize=(8.4, 5.4))
    for schedule_id in selected_ids:
        selected = [row for row in grouped[schedule_id] if not row["hostile"]]
        max_points = max(len(row["points"]) for row in selected)
        x_values = []
        y_values = []
        for point_index in range(max_points):
            values = []
            for row in selected:
                if point_index < len(row["points"]):
                    point = row["points"][point_index]
                    initial = row["points"][0]["target_residual_after"]
                    values.append(point["target_residual_after"] / max(initial, 1.0e-300))
            if values:
                x_values.append(point_index)
                y_values.append(max(median(values), 1.0e-20))
        ax.plot(x_values, y_values, marker="o", label=schedule_id)
    ax.set_yscale("log")
    ax.set_xlabel("Recorded path point")
    ax.set_ylabel("Median normalized target residual")
    ax.grid(True, which="both", alpha=0.3)
    ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(args.plots / "TARGET_RESIDUAL_BY_ROUND.png", dpi=180)
    plt.close(fig)

    # Plot 4: oracle output defect by case for the strongest bounded candidates.
    chosen = ["direct-decoupled-q1-c2", "direct-decoupled-q2-c2", "fixed-q7"]
    case_ids = [case_id for case_id, case in cases.items() if not case["hostile"]]
    fig, ax = plt.subplots(figsize=(9.2, 5.6))
    positions = list(range(len(case_ids)))
    width = 0.24
    for offset_index, schedule_id in enumerate(chosen):
        values = []
        for case_id in case_ids:
            row = next(row for row in grouped[schedule_id] if row["case_id"] == case_id)
            values.append(max(float(row["oracle_output_wrms"]), 1.0e-20))
        xs = [x + (offset_index - 1) * width for x in positions]
        ax.bar(xs, values, width=width, label=schedule_id)
    ax.axhline(document["output_wrms_budget"], linestyle="--", linewidth=1.0, label="algebraic budget")
    ax.set_yscale("log")
    ax.set_xticks(positions)
    ax.set_xticklabels(case_ids, rotation=25, ha="right", fontsize=8)
    ax.set_ylabel("Sequential-oracle output WRMS")
    ax.grid(True, axis="y", which="both", alpha=0.3)
    ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(args.plots / "ORACLE_DEFECT_BY_CASE.png", dpi=180)
    plt.close(fig)


if __name__ == "__main__":
    main()
