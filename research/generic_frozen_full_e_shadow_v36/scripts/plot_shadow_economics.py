#!/usr/bin/env python3
"""Render adversarial v3.6 wall and work-economics figures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def load_summary(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        summary = json.load(handle)
    if summary.get("verdict") != "PASS_DESCRIPTIVE_ECONOMICS":
        raise ValueError("economics summary has not passed structural verification")
    return summary


def plot_wall_ratios(summary: dict, output: Path) -> None:
    profiles = summary["profiles"]
    figure, axis = plt.subplots(figsize=(9.6, 5.8), constrained_layout=True)
    for profile_index, profile in enumerate(profiles):
        ratios = profile["measured_wall_ratios_shadow_over_rjf"]
        for pair_index, ratio in enumerate(ratios):
            offset = (pair_index - 3) * 0.035
            color = "#2a6fbb" if pair_index % 2 == 0 else "#d95f02"
            marker = "o" if pair_index % 2 == 0 else "s"
            label = None
            if profile_index == 0 and pair_index == 0:
                label = "R-JF first"
            if profile_index == 0 and pair_index == 1:
                label = "shadow first"
            axis.scatter(
                profile_index + offset,
                ratio,
                s=42,
                color=color,
                marker=marker,
                alpha=0.85,
                label=label,
                zorder=3,
            )
        axis.scatter(
            profile_index,
            profile["median_wall_ratio_shadow_over_rjf"],
            s=85,
            color="black",
            marker="D",
            label="seven-pair median" if profile_index == 0 else None,
            zorder=4,
        )
    axis.axhline(1.0, color="#555555", linewidth=1.2, linestyle="--")
    axis.set_yscale("log")
    axis.set_xticks(range(len(profiles)), [f"N={row['dimension']}" for row in profiles])
    axis.set_ylabel("whole-suite wall ratio, shadow / R-JF (log scale)")
    axis.set_title("Frozen full-E shadow: all 35 measured pairs retained")
    axis.grid(axis="y", which="both", alpha=0.25)
    axis.legend(frameon=False, ncols=3, loc="upper left")
    axis.text(
        0.01,
        0.02,
        "No speedup threshold; N=384 visibly host-noise dominated",
        transform=axis.transAxes,
        fontsize=9,
        color="#444444",
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(output, dpi=180, metadata={"Software": "VigilODE v3.6"})
    plt.close(figure)


def plot_realized_work(summary: dict, output: Path) -> None:
    profiles = summary["profiles"]
    x = list(range(len(profiles)))
    prefix = [100.0 * row["realized_prefix_over_committed_rjf_jvp"] for row in profiles]
    continuation = [
        100.0 * row["realized_continuation_over_committed_rjf_jvp"] for row in profiles
    ]
    figure, axis = plt.subplots(figsize=(9.6, 5.4), constrained_layout=True)
    axis.bar(x, prefix, color="#4c78a8", label="retained prefix")
    axis.bar(x, continuation, bottom=prefix, color="#f58518", label="full-E continuation")
    for index, row in enumerate(profiles):
        total = 100.0 * row["realized_total_speculative_over_committed_rjf_jvp"]
        axis.text(index, total + 0.025, f"{total:.3f}%", ha="center", va="bottom", fontsize=9)
    axis.set_xticks(x, [f"N={row['dimension']}" for row in profiles])
    axis.set_ylabel("speculative JVP / committed R-JF JVP (%)")
    axis.set_title("Realized complete speculative ledger by consumed profile")
    axis.grid(axis="y", alpha=0.25)
    axis.legend(frameon=False, ncols=2, loc="upper left")
    output.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(output, dpi=180, metadata={"Software": "VigilODE v3.6"})
    plt.close(figure)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--wall-output", type=Path, required=True)
    parser.add_argument("--work-output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = load_summary(args.summary)
    plot_wall_ratios(summary, args.wall_output)
    plot_realized_work(summary, args.work_output)
    print("SHADOW_ECONOMICS_PLOTS_COMPLETE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
