#!/usr/bin/env python3
"""Plot measured work and independent-reference endpoint evidence."""
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parent
rows = json.loads((root / "PILOT_SUMMARY.json").read_text())
match = json.loads((root / "STABILITY_MATCH.json").read_text())
fig, axes = plt.subplots(1, 2, figsize=(10.5, 4.1), constrained_layout=True)
ax = axes[0]
for method, color in [("literal", "#bb5566"), ("cofactor", "#0077bb")]:
    subset = [r for r in rows if r["end"] == 2 and r["method"] == method]
    ax.plot([r["epsilon"] for r in subset], [r["attempts"] for r in subset],
            "o-", label=method, color=color)
    for r in subset:
        if not r["success"]:
            ax.annotate("budget exhausted\nT reached: 0.6455 / 2", (r["epsilon"],r["attempts"]),
                        xytext=(12,-45), textcoords="offset points", fontsize=8)
ax.set(xscale="log", yscale="log", xlabel="Relaxation parameter epsilon", ylabel="Native DOPRI5 attempts",
       title="Same physical IVP, T=2")
ax.legend(); ax.grid(alpha=.25, which="both")
ax = axes[1]
names = ["Cofactor relaxation", "KS (1024 cells)"]
ax.scatter(range(len(names)), [r["ratio_to_linear_stability_limit"] for r in match], s=90, color=["#228833","#4477aa"])
ax.set_xticks(range(len(names)), names)
ax.set_xlim(-.5, 1.5)
ax.axhline(1, color="black", linestyle="--", linewidth=1)
ax.set(ylim=(.995,1.002), ylabel="Mean accepted h / linear stability limit",
       title="Published small-step failures")
for i,r in enumerate(match):
    ax.text(i, r["ratio_to_linear_stability_limit"]+.0001,
            f'{r["ratio_to_linear_stability_limit"]:.6f}', ha="center", fontsize=9)
ax.grid(axis="y", alpha=.25)
fig.savefig(root / "adaptation_diagnostics.png", dpi=180)
fig.savefig(root / "adaptation_diagnostics.svg")
