#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BASE = ROOT / "research" / "homotopy_order_policy_v06"
CANONICAL_T1 = BASE / "results" / "CANONICAL_RELEASE_THREADS1.json"
CANONICAL_T4 = BASE / "results" / "CANONICAL_RELEASE_THREADS4.json"
THREAD_SCALING = BASE / "results" / "THREAD_SCALING.csv"

serial = json.loads(CANONICAL_T1.read_text(encoding="utf-8"))
parallel = json.loads(CANONICAL_T4.read_text(encoding="utf-8"))
scientific_fields = [
    "source_summary",
    "policies",
    "replay_rows",
    "policy_summaries",
    "family_winners",
    "trajectory_rows",
    "trajectory_gates",
]
scientific_equal = all(serial[field] == parallel[field] for field in scientific_fields)

summary_by_policy: dict[str, dict[str, dict]] = defaultdict(dict)
for row in serial["policy_summaries"]:
    summary_by_policy[row["policy_id"]][row["split"]] = row

safe_both_splits = []
for policy_id, splits in sorted(summary_by_policy.items()):
    calibration = splits["calibration"]
    holdout = splits["holdout"]
    if calibration["false_accepts"] == 0 and holdout["false_accepts"] == 0:
        safe_both_splits.append(
            {
                "policy_id": policy_id,
                "policy_family": calibration["policy_family"],
                "calibration_accepts": calibration["accepts"],
                "calibration_low_depth_accepts": calibration["low_depth_accepts"],
                "holdout_accepts": holdout["accepts"],
                "holdout_low_depth_accepts": holdout["low_depth_accepts"],
                "holdout_fallback_fraction": holdout["fallback_fraction"],
            }
        )

winner_false_accept_details = []
for winner in serial["family_winners"]:
    policy_id = winner["policy_id"]
    rows = [
        row
        for row in serial["replay_rows"]
        if row["policy_id"] == policy_id
        and row["split"] == "holdout"
        and row["false_accept"]
    ]
    ratios = [
        row["oracle_output_wrms"] / row["output_wrms"]
        for row in rows
        if row["oracle_output_wrms"] is not None
        and row["output_wrms"] is not None
        and row["output_wrms"] > 0
    ]
    winner_false_accept_details.append(
        {
            "policy_family": winner["policy_family"],
            "policy_id": policy_id,
            "holdout_false_accepts": len(rows),
            "by_case_family": dict(sorted(Counter(row["case_family"] for row in rows).items())),
            "by_q": {str(key): value for key, value in sorted(Counter(row["q"] for row in rows).items())},
            "oracle_to_certificate_ratio_median": statistics.median(ratios) if ratios else None,
            "oracle_to_certificate_ratio_max": max(ratios) if ratios else None,
        }
    )

thread_rows = list(csv.DictReader(THREAD_SCALING.open(encoding="utf-8")))
thread_values: dict[int, list[float]] = {}
thread_medians: dict[int, float] = {}
for threads in (1, 2, 4):
    values = [
        float(row["elapsed_seconds"])
        for row in thread_rows
        if row["phase"] == "timed" and int(row["threads"]) == threads
    ]
    thread_values[threads] = values
    thread_medians[threads] = statistics.median(values)

thread_scaling_summary = {
    "median_seconds": {str(key): value for key, value in thread_medians.items()},
    "speedup": {
        "2": thread_medians[1] / thread_medians[2],
        "4": thread_medians[1] / thread_medians[4],
    },
    "parallel_efficiency": {
        "2": thread_medians[1] / thread_medians[2] / 2.0,
        "4": thread_medians[1] / thread_medians[4] / 4.0,
    },
    "values_seconds": {str(key): value for key, value in thread_values.items()},
}

(BASE / "results" / "THREAD_SCALING_SUMMARY.json").write_text(
    json.dumps(
        {"schema": "rodas5p-homotopy-order-policy-thread-scaling-summary-v1", **thread_scaling_summary},
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)

homotopy_gates = [row for row in serial["trajectory_gates"] if row["method"] != "sequential-direct"]
promoted_homotopy_gates = [row for row in homotopy_gates if row["promote"]]

winner_ids = {row["policy_family"]: row["policy_id"] for row in serial["family_winners"]}
method_specific_false_accepts = {}
for family, policy_id in winner_ids.items():
    method_specific_false_accepts[family] = {
        gate["method"]: gate["holdout_false_accepts"]
        for gate in homotopy_gates
        if gate["policy_id"] == policy_id and gate["problem_id"] == "manufactured-vector-order"
    }
absolute_id = winner_ids["absolute"]
absolute_gate_rows = [
    row
    for row in homotopy_gates
    if row["policy_id"] == absolute_id
]

analysis = {
    "schema": "rodas5p-homotopy-order-policy-analysis-v1",
    "canonical_profile": serial["profile"],
    "scientific_fields_equal_threads_1_4": scientific_equal,
    "source_summary": serial["source_summary"],
    "policy_grid_size": len(serial["policies"]),
    "replay_rows": len(serial["replay_rows"]),
    "trajectory_rows": len(serial["trajectory_rows"]),
    "trajectory_gates": len(serial["trajectory_gates"]),
    "family_winners": serial["family_winners"],
    "safe_both_splits": safe_both_splits,
    "winner_false_accept_details": winner_false_accept_details,
    "method_specific_false_accepts": method_specific_false_accepts,
    "homotopy_promoted_gate_count": len(promoted_homotopy_gates),
    "absolute_winner_gate_counts": {
        "rows": len(absolute_gate_rows),
        "fifth_order_pass": sum(row["fifth_order_pass"] for row in absolute_gate_rows),
        "stiff_regression_pass": sum(row["stiff_regression_pass"] for row in absolute_gate_rows),
        "low_depth_nonempty": sum(row["low_depth_nonempty"] for row in absolute_gate_rows),
        "promote": sum(row["promote"] for row in absolute_gate_rows),
    },
    "thread_scaling": thread_scaling_summary,
    "conclusions": [
        "Only the three absolute-budget policies have zero false accepts on both calibration and holdout replay.",
        "Embedded-relative, step-power, and mixed family winners exhibit holdout false accepts, concentrated in manufactured-mass and Prothero-Robinson cases.",
        "No homotopy trajectory gate is promoted: low-depth absolute paths lose fifth order or stiff robustness, while fifth-order/stiff-safe q=7 or corrected paths are not low depth.",
        "Rayon produces byte-identical scientific fields across one and four threads, but end-to-end canonical speedup is modest because source generation and 108 MiB JSON materialization remain serial-dominant.",
    ],
}

(BASE / "results" / "ORDER_POLICY_ANALYSIS.json").write_text(
    json.dumps(analysis, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)

with (BASE / "tables" / "POLICY_HOLDOUT_SUMMARY.csv").open("w", newline="", encoding="utf-8") as handle:
    fieldnames = [
        "policy_family",
        "policy_id",
        "accepts",
        "low_depth_accepts",
        "false_accepts",
        "failures",
        "fallback_fraction",
        "median_w_solve_batches_accepted",
        "median_w_solve_vectors_accepted",
    ]
    writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for row in serial["policy_summaries"]:
        if row["split"] == "holdout":
            writer.writerow({key: row[key] for key in fieldnames})

with (BASE / "tables" / "TRAJECTORY_GATE_SUMMARY.csv").open("w", newline="", encoding="utf-8") as handle:
    fieldnames = list(serial["trajectory_gates"][0])
    writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    writer.writerows(serial["trajectory_gates"])

print(json.dumps({
    "scientific_equal": scientific_equal,
    "safe_policies": [row["policy_id"] for row in safe_both_splits],
    "homotopy_promoted_gate_count": len(promoted_homotopy_gates),
    "thread_scaling": thread_scaling_summary,
}, indent=2))
