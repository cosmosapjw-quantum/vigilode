#!/usr/bin/env python3
"""Render descriptive posthoc figures from one frozen analysis JSON only.

No solver, CAS, benchmark, exact oracle, sampling or acceptance decision runs
here. Error bars are the observed min/max of all seven recorded repeats,
not confidence intervals. Original run records are provenance, not runtime
inputs to this plotting script.

Input record:
  research/production_rvj_nag_20261028/analysis/BENCHMARK_ANALYSIS.json
  SHA256 b8b6ae128b8913c5a2e151d277929b5cd981927df75b1baa71d40795d8634a43
Original records, whose hashes are embedded in the frozen analysis:
  /tmp/vigilode-rvj-registered-01/closure.json
  /tmp/vigilode-rvj-registered-01/trace.json
  /tmp/vigilode-rvj-registered-01/ORACLE_RESULTS.json
  /tmp/vigilode-rvj-registered-01/RUN_RECEIPT.json
Executed source: 64004e23042f2cf076da5378be724f50c3856f4c.
"""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
from fractions import Fraction
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", str(Path(tempfile.gettempdir()) / "vigilode-rvj-matplotlib"))
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import LogLocator, NullLocator, ScalarFormatter


REPORT = Path(__file__).resolve().parent
INPUT = REPORT.parent / "analysis" / "BENCHMARK_ANALYSIS.json"
EXPECTED_INPUT_SHA256 = "b8b6ae128b8913c5a2e151d277929b5cd981927df75b1baa71d40795d8634a43"
SOURCE_SHA = "64004e23042f2cf076da5378be724f50c3856f4c"
ARMS = (
    "historical_power_legacy",
    "direct_bernstein_legacy",
    "direct_bernstein_tight",
)
ARM_LABELS = ("Power / legacy", "Direct / legacy", "Direct / tight")
COLORS = ("#657789", "#2167a5", "#c27624")
MARKERS = ("o", "s", "^")


def configure_style() -> None:
    plt.rcParams.update(
        {
            "font.family": "DejaVu Sans",
            "font.size": 9.5,
            "axes.titlesize": 10,
            "axes.labelsize": 9.5,
            "xtick.labelsize": 8.7,
            "ytick.labelsize": 8.7,
            "legend.fontsize": 8.2,
            "axes.spines.top": False,
            "axes.spines.right": False,
            "axes.linewidth": 0.7,
            "lines.linewidth": 1.5,
            "lines.markersize": 4.6,
            "pdf.fonttype": 42,
            "savefig.facecolor": "white",
            "figure.facecolor": "white",
        }
    )


def two_panels():
    fig, axes = plt.subplots(1, 2, figsize=(9.2, 3.7))
    fig.subplots_adjust(left=0.085, right=0.98, bottom=0.19, top=0.9, wspace=0.36)
    for ax in axes:
        ax.grid(axis="y", which="major", color="#e4e9ee", linewidth=0.6, zorder=0)
        ax.tick_params(direction="out", length=3, width=0.6)
    return fig, axes


def grid_axis(ax, values) -> None:
    ax.set_xscale("log", base=2)
    ax.set_xticks(values)
    ax.set_xticklabels([str(value) for value in values])
    ax.xaxis.set_minor_locator(NullLocator())
    ax.set_xlabel("Number of slabs, $N$")


def save(fig, name: str, subject: str) -> None:
    provenance = (
        f"Descriptive posthoc figure; source {SOURCE_SHA}; "
        f"analysis SHA256 {EXPECTED_INPUT_SHA256}; {subject}"
    )
    fig.savefig(
        REPORT / f"{name}.pdf",
        metadata={
            "Title": name.replace("_", " ").title(),
            "Author": "VigilODE research",
            "Subject": provenance,
            "Keywords": "posthoc, descriptive, seven repeats, no confidence interval",
            "CreationDate": None,
            "ModDate": None,
        },
    )
    fig.savefig(
        REPORT / f"{name}.png",
        dpi=200,
        metadata={"Description": provenance},
    )
    plt.close(fig)


def closure_figure(data: dict) -> None:
    fig, (left, right) = two_panels()
    cap = float(Fraction(data["closure"]["physical_cap_exact"]))
    ns = sorted({row["steps"] for row in data["closure"]["cells"]})
    for arm, label, color, marker in zip(ARMS, ARM_LABELS, COLORS, MARKERS):
        rows = sorted(
            (row for row in data["closure"]["cells"] if row["n"] == 16 and row["arm"] == arm),
            key=lambda row: row["steps"],
        )
        left.plot(
            [row["steps"] for row in rows],
            [row["all_state_bound_display"] for row in rows],
            label=label,
            color=color,
            marker=marker,
            zorder=3,
        )
    left.axhline(cap, color="#343b42", linestyle=(0, (4, 3)), linewidth=1.1,
                 label=r"Cap $10^{-8}$", zorder=2)
    left.set_yscale("log")
    grid_axis(left, ns)
    left.set_xlim(ns[0] / 1.15, ns[-1] * 1.15)
    left.set_ylim(7e-11, 2e-7)
    left.yaxis.set_minor_locator(NullLocator())
    left.set_ylabel("Certified all-state error bound")
    left.set_title(r"(a) Original-equation certificate, $n=16$", loc="left", pad=9)
    left.legend(loc="lower left", frameon=False, handlelength=2, labelspacing=0.38)

    first = sorted(data["closure"]["first_passing"], key=lambda row: row["n"])
    all_min, all_max = [], []
    for j, (arm, color, marker) in enumerate(zip(ARMS, COLORS, MARKERS)):
        points = [row["arms"][arm]["first_passing_cell"] for row in first]
        summaries = [point["timing"]["total_seconds"] for point in points]
        middle = [1000 * value["median"] for value in summaries]
        lower = [1000 * (value["median"] - value["min"]) for value in summaries]
        upper = [1000 * (value["max"] - value["median"]) for value in summaries]
        all_min.extend(1000 * value["min"] for value in summaries)
        all_max.extend(1000 * value["max"] for value in summaries)
        right.errorbar(
            [i + (j - 1) * 0.18 for i in range(len(first))],
            middle,
            yerr=[lower, upper],
            fmt=marker,
            color=color,
            capsize=3,
            capthick=0.8,
            elinewidth=1.1,
            markersize=5,
            zorder=3,
        )
    right.set_yscale("log")
    right.set_ylim(min(all_min) / 1.5, max(all_max) * 1.5)
    right.set_xlim(-0.45, len(first) - 0.55)
    right.set_xticks(range(len(first)))
    right.set_xticklabels([str(row["n"]) for row in first])
    right.set_yticks([0.3, 1, 3, 10])
    right.yaxis.set_major_formatter(ScalarFormatter())
    right.yaxis.set_minor_locator(NullLocator())
    right.set_xlabel("Cyclic closure dimension, $n$")
    right.set_ylabel("Total elapsed time (ms, logarithmic scale)")
    right.set_title("(b) First passing grid; all three arms", loc="left", pad=9)
    right.text(
        0.025, 0.965,
        "Power: $N=64$; direct: $N=32$\nMedian and observed min–max; 7 repeats",
        transform=right.transAxes,
        va="top", fontsize=8.3, color="#424b54",
    )
    fig.text(
        0.5, 0.045,
        "Whole call includes endpoint preparation and certification. Ranges are not confidence intervals.",
        ha="center", fontsize=8.3, color="#49545e",
    )
    save(fig, "closure_results", "All-state bounds and first-passing whole-call min/median/max timings")


def trace_figure(data: dict) -> None:
    fig, (left, right) = two_panels()
    rows = sorted(
        (row for row in data["trace"]["runs"] if row["route"] == "direct_moments"),
        key=lambda row: row["steps"],
    )
    ns = [row["steps"] for row in rows]
    for key, label, color, marker in (
        ("whole_path_error_upper_display", "Residual certificate (external)", "#b34e37", "o"),
        ("direct_quintic_path_error_upper_display", "Exact-reference path bound", "#2c8266", "s"),
        ("exact_node_error_inf_display", "Node error (diagnostic)", "#2167a5", "^"),
    ):
        left.plot(ns, [row[key] for row in rows], label=label, color=color, marker=marker, zorder=3)
    left.axhline(1e-8, color="#343b42", linestyle=(0, (4, 3)), linewidth=1.1,
                 label=r"Cap $10^{-8}$", zorder=2)
    left.set_yscale("log")
    grid_axis(left, ns)
    left.set_xlim(ns[0] / 1.15, ns[-1] * 1.15)
    left.set_ylim(4e-12, 6e-5)
    left.yaxis.set_major_locator(LogLocator(base=10, numticks=8))
    left.yaxis.set_minor_locator(NullLocator())
    left.set_ylabel("Infinity-norm error or upper bound")
    left.set_title("(a) Whole-path bounds and node error", loc="left", pad=9)
    left.legend(loc="lower left", frameon=False, handlelength=2, labelspacing=0.32)

    phase_order = ("candidate_field", "interval_field", "combined_field", "candidate_trajectory")
    phase_labels = ("Candidate\nfield", "Interval\nfield", "Candidate +\nenclosure", "Full candidate\ntrajectory")
    phase_map = {row["phase"]: row for row in data["trace"]["phases"]}
    right.axvline(1, color="#65717b", linestyle=(0, (4, 3)), linewidth=1, zorder=1)
    for index, phase in enumerate(phase_order):
        ratio = phase_map[phase]["ratio_of_medians"]
        color = "#2167a5" if ratio >= 1 else "#b34e37"
        right.hlines(index, min(1, ratio), max(1, ratio), color=color, linewidth=3, zorder=2)
        right.scatter([ratio], [index], color=color, s=34, zorder=3)
        right.annotate(
            f"{ratio:.3f}", (ratio, index),
            xytext=(7, -3) if ratio >= 1 else (-7, -3),
            textcoords="offset points",
            ha="left" if ratio >= 1 else "right",
            fontsize=9, color=color,
        )
    right.set_xlim(0.72, 1.73)
    right.set_ylim(3.88, -0.5)
    right.set_yticks(range(4))
    right.set_yticklabels(phase_labels, fontsize=8.7)
    right.set_xticks([0.8, 1.0, 1.2, 1.4, 1.6])
    right.grid(axis="y", visible=False)
    right.grid(axis="x", which="major", color="#e4e9ee", linewidth=0.6, zorder=0)
    right.set_xlabel("Direct / shared median elapsed time")
    right.set_title("(b) Same-target native phase timings", loc="left", pad=9)
    right.text(0.97, 0.965, ">1: shared is faster", transform=right.transAxes,
               ha="right", va="top", fontsize=8.2, color="#49545e")
    paired = phase_map["candidate_trajectory"]["paired_repeat_ratio"]["median"]
    right.text(
        0.025, 0.025,
        f"Trajectory ranges overlap;\nmedian paired-repeat ratio: {paired:.3f}",
        transform=right.transAxes, va="bottom", fontsize=8.2, color="#49545e",
    )
    fig.text(
        0.5, 0.045,
        "Both routes export identical state bits. Native timing excludes the independent exact whole-path oracle.",
        ha="center", fontsize=8.3, color="#49545e",
    )
    save(fig, "trace_results", "External path bounds and descriptive native median phase ratios; no full certified speedup claim")


def main() -> None:
    raw = INPUT.read_bytes()
    actual = hashlib.sha256(raw).hexdigest()
    if actual != EXPECTED_INPUT_SHA256:
        raise SystemExit(f"Frozen analysis digest mismatch: {actual}")
    data = json.loads(raw)
    if data["source_sha"] != SOURCE_SHA:
        raise SystemExit("Unexpected executed source in analysis")
    configure_style()
    closure_figure(data)
    trace_figure(data)
    print(json.dumps({
        "input": str(INPUT), "input_sha256": actual,
        "matplotlib_version": matplotlib.__version__,
        "figures": [str(REPORT / f"{name}.{suffix}") for name in
                    ("closure_results", "trace_results") for suffix in ("pdf", "png")],
        "new_solver_or_cas_runs": 0,
    }, indent=2))


if __name__ == "__main__":
    main()
