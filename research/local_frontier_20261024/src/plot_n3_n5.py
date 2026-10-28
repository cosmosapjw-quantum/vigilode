#!/usr/bin/env python3
"""Render descriptive timing summaries from the preserved JSON analysis."""
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

OUT = Path(__file__).resolve().parents[1] / "analysis"
n3 = json.loads((OUT / "N3_REGIME_COST.json").read_text())
n5 = json.loads((OUT / "N5_PARALLEL_CALLBACKS.json").read_text())
plt.rcParams.update({"font.size": 10, "axes.spines.top": False, "axes.spines.right": False})
fig, axes = plt.subplots(1, 2, figsize=(14, 6), gridspec_kw={"width_ratios": [1.15, 1]})
labels = [c["run"].replace("n3-final-", "") for c in n3["cases"]]
ratios = [c["guard_comparison"]["priced_transfer_speedup"] for c in n3["cases"]]
y = np.arange(len(labels))
axes[0].barh(y, ratios, color=["#267c81" if x > 1 else "#b87251" for x in ratios], height=.65)
axes[0].set_yticks(y, labels)
axes[0].invert_yaxis()
axes[0].axvline(1, color="#333333", linewidth=1, linestyle="--")
axes[0].set_xlim(0, 1.2)
for j, r in enumerate(ratios):
    axes[0].text(r + .015, j, f"{r:.3f}", va="center", fontsize=9)
axes[0].set_xlabel("Existing priced controller / locality guard (time ratio)")
axes[0].set_title("N3: extra calibration often costs more\nAbove 1 favors the opt-in guard", loc="left")
modes = ["fixed-causal", "fixed-picard", "fixed-residual-first", "protected-raw", "protected-dense"]
display = ["Forward causal", "Picard", "Residual-first", "Protected raw-U", "Protected dense"]
yp = np.arange(len(modes))
for idx, (campaign, color) in enumerate(zip(n5["campaigns"], ["#355a85", "#31a0a0"])):
    values = [1e3 * campaign["modes"][m]["median_seconds"] for m in modes]
    lower = [values[j] - 1e3 * campaign["modes"][m]["min_seconds"] for j, m in enumerate(modes)]
    upper = [1e3 * campaign["modes"][m]["max_seconds"] - values[j] for j, m in enumerate(modes)]
    axes[1].errorbar(values, yp + (idx - .5) * .24, xerr=[lower, upper],
                    fmt="o" if idx == 0 else "s", markersize=6, color=color,
                    label=f"{idx+1} worker{'s' if idx else ''}",
                    elinewidth=1, capsize=3, linestyle="none")
axes[1].set_yticks(yp, display)
axes[1].invert_yaxis()
axes[1].set_xscale("log")
axes[1].set_xlabel("Complete-solve milliseconds (log scale; lower is better)")
axes[1].set_title("N5: real nonlinear pair-coupled RHS\nParallel gains do not defeat the dense comparator", loc="left")
axes[1].legend(loc="lower right", frameon=False)
fig.text(.5, .02, "Three cyclic samples per mode, median and observed range; no confidence interval. All 279 timed trajectories passed the same error cap.", ha="center", fontsize=9)
fig.tight_layout(rect=[0, .06, 1, 1])
for suffix in ["png", "svg"]:
    fig.savefig(OUT / ("n3_n5_local_discrimination." + suffix), dpi=180)
