#!/usr/bin/env python3
"""Plot diagnostic observed selected/best ratios from the analyzer output."""

import json
import statistics
import sys
from collections import defaultdict

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt


def main(source, output):
    data = json.load(open(source, encoding="utf-8"))
    groups = defaultdict(list)
    for row in data["windows"]:
        if row["status"] == "COMPARABLE":
            groups[row["window_id"]].append(row["observed_cost_ratio"])
    names = list(sorted(groups))
    medians = [statistics.median(groups[name]) for name in names]
    low = [medians[i] - min(groups[name]) for i, name in enumerate(names)]
    high = [max(groups[name]) - medians[i] for i, name in enumerate(names)]
    fig, ax = plt.subplots(figsize=(7.5, 3.2))
    y = list(range(len(names)))
    ax.errorbar(medians, y, xerr=[low, high], fmt="o", capsize=4)
    ax.axvline(1, color="gray", linestyle="--", linewidth=1)
    ax.set_yticks(y, names)
    ax.set_xscale("log")
    ax.set_xlabel("selected / best tested candidate, paired blocks (diagnostic only)")
    ax.set_title("Same physical error cap; 5 timed blocks per cohort")
    ax.grid(axis="x", alpha=.25)
    fig.tight_layout()
    fig.savefig(output)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
