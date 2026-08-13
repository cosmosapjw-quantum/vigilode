#!/usr/bin/env python3
from __future__ import annotations
import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

SHORT = {
    "protected-jf-rodas5p": "R-JF",
    "pexprb54s4-fused-full-mgs-t1": "E-K",
    "held-g1-q1-q2": "C-JF",
    "frozen-radau-iia3": "R-J",
    "frozen-bdf2": "BDF2",
}


def sorted_rows(rows):
    porder = ["quadratic", "complex-dahlquist", "oscillatory-pr", "advection-diffusion"]
    rorder = [1e-4, 1e-6, 1e-8]
    return sorted(rows, key=lambda r: (porder.index(r["problem_id"]), rorder.index(float(r["rtol"]))))


def plot_oracle_map(rows, title, out):
    porder = ["quadratic", "complex-dahlquist", "oscillatory-pr", "advection-diffusion"]
    rorder = [1e-4, 1e-6, 1e-8]
    values = np.zeros((len(porder), len(rorder)))
    labels = [["" for _ in rorder] for _ in porder]
    for row in rows:
        i = porder.index(row["problem_id"])
        j = rorder.index(float(row["rtol"]))
        values[i, j] = float(row["oracle_speedup_vs_rodas"])
        labels[i][j] = f"{SHORT.get(row['winner'], row['winner'])}\n{values[i,j]:.2f}×"
    fig, ax = plt.subplots(figsize=(8.0, 4.6))
    image = ax.imshow(values, aspect="auto")
    for i in range(len(porder)):
        for j in range(len(rorder)):
            text_color = "white" if values[i, j] > (values.min() + values.max()) / 2 else "black"
            ax.text(j, i, labels[i][j], ha="center", va="center", color=text_color, fontsize=9)
    ax.set_xticks(range(len(rorder)), [f"rtol={x:.0e}" for x in rorder])
    ax.set_yticks(range(len(porder)), porder)
    ax.set_title(title)
    cbar = fig.colorbar(image, ax=ax)
    cbar.set_label("oracle speedup vs protected JF RODAS5P")
    fig.tight_layout()
    fig.savefig(out, dpi=180)
    plt.close(fig)


def plot_speedup(rows_strict, rows_auto, out):
    s = sorted_rows(rows_strict)
    a = sorted_rows(rows_auto)
    labels = [f"{r['problem_id']}\n{float(r['rtol']):.0e}" for r in s]
    x = np.arange(len(s))
    width = 0.38
    fig, ax = plt.subplots(figsize=(12.0, 5.2))
    ax.bar(x - width/2, [r["oracle_speedup_vs_rodas"] for r in s], width, label="strict-JF oracle")
    ax.bar(x + width/2, [r["oracle_speedup_vs_rodas"] for r in a], width, label="auto oracle")
    ax.axhline(1.0, linewidth=1.0)
    ax.set_xticks(x, labels, rotation=45, ha="right")
    ax.set_ylabel("speedup vs protected JF RODAS5P")
    ax.set_title("Offline oracle gain by problem/tolerance cell")
    ax.legend()
    fig.tight_layout()
    fig.savefig(out, dpi=180)
    plt.close(fig)


def plot_features(features, out):
    fig, ax = plt.subplots(figsize=(7.5, 5.5))
    markers = {True: "o", False: "x"}
    codes = {"quadratic": "Q", "complex-dahlquist": "CD", "oscillatory-pr": "PR", "advection-diffusion": "AD"}
    for row in features:
        winner = bool(row["exp_is_oracle_winner"])
        ax.scatter(
            row["exp_log10_time_error"],
            row["exp_jvp_vectors_per_accepted_step"],
            marker=markers[winner],
            s=70,
        )
        ax.annotate(
            f"{codes[row['problem_id']]}:{int(round(-np.log10(float(row['rtol']))))}",
            (row["exp_log10_time_error"], row["exp_jvp_vectors_per_accepted_step"]),
            xytext=(4, 4),
            textcoords="offset points",
            fontsize=7,
        )
    ax.set_xlabel("log10(max embedded time-error telemetry)")
    ax.set_ylabel("E-K JVP vectors per accepted step")
    ax.set_title("Existing zero-extra-work features do not cleanly separate E-K and R-JF winners")
    ax.text(0.02, 0.98, "circle: E-K winner; x: R-JF winner\nQ/CD/PR/AD : -log10(rtol)", transform=ax.transAxes, va="top")
    fig.tight_layout()
    fig.savefig(out, dpi=180)
    plt.close(fig)


def plot_stability(report, out):
    strict_emp = sorted_rows(report["strict_campaign_winner_stability"])
    strict_mut = sorted_rows(report["timing_mutation"]["strict_10pct"])
    labels = [f"{r['problem_id']}\n{float(r['rtol']):.0e}" for r in strict_emp]
    x = np.arange(len(labels))
    width = 0.38
    fig, ax = plt.subplots(figsize=(12.0, 5.0))
    ax.bar(x - width/2, [r["median_winner_fraction"] for r in strict_emp], width, label="7-campaign empirical")
    ax.bar(x + width/2, [r["nominal_winner_fraction"] for r in strict_mut], width, label="±10% timing mutation")
    ax.axhline(0.8, linewidth=1.0)
    ax.set_ylim(0.0, 1.05)
    ax.set_xticks(x, labels, rotation=45, ha="right")
    ax.set_ylabel("nominal winner fraction")
    ax.set_title("Strict-JF oracle-label stability")
    ax.legend()
    fig.tight_layout()
    fig.savefig(out, dpi=180)
    plt.close(fig)


def plot_overhead(features, out):
    rows = sorted(features, key=lambda r: (r["problem_id"], -float(r["rtol"])))
    labels = [f"{r['problem_id']}\n{float(r['rtol']):.0e}" for r in rows]
    x = np.arange(len(rows))
    fig, ax = plt.subplots(figsize=(12.0, 4.8))
    ax.bar(x, [100.0*r["jvp_probe_fraction_of_rodas_rhs"] for r in rows])
    ax.axhline(5.0, linewidth=1.0)
    ax.set_xticks(x, labels, rotation=45, ha="right")
    ax.set_ylabel("fresh-JVP probe cost / RODAS RHS work [%]")
    ax.set_title("A 3-RHS JVP-quality probe is too expensive to run every step in several cells")
    fig.tight_layout()
    fig.savefig(out, dpi=180)
    plt.close(fig)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", type=Path, required=True)
    ap.add_argument("--output-dir", type=Path, required=True)
    args = ap.parse_args()
    report = json.loads(args.input.read_text())
    args.output_dir.mkdir(parents=True, exist_ok=True)
    plot_oracle_map(report["strict_oracle"], "Strict-JF offline oracle map", args.output_dir / "G4_S0_STRICT_ORACLE_MAP.png")
    plot_oracle_map(report["auto_oracle"], "Auto-profile offline oracle map", args.output_dir / "G4_S0_AUTO_ORACLE_MAP.png")
    plot_speedup(report["strict_oracle"], report["auto_oracle"], args.output_dir / "G4_S0_ORACLE_GAIN_BY_CELL.png")
    plot_features(report["feature_rows"], args.output_dir / "G4_S0_FEATURE_SEPARABILITY.png")
    plot_stability(report, args.output_dir / "G4_S0_WINNER_STABILITY.png")
    plot_overhead(report["feature_rows"], args.output_dir / "G4_S0_JVP_PROBE_OVERHEAD.png")

if __name__ == "__main__":
    main()
