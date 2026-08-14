#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

TIMING_FIELDS = {
    "exponential_cold_wall_seconds",
    "exponential_prefix_wall_seconds",
    "exponential_resume_wall_seconds",
    "exponential_total_reused_wall_seconds",
    "gmres_cold_wall_seconds",
    "gmres_prefix_wall_seconds",
    "gmres_resume_wall_seconds",
    "gmres_total_reused_wall_seconds",
    "exponential_prefix_cost_inflation",
    "gmres_prefix_cost_inflation",
    "actual_exponential_to_gmres_cost_ratio",
    "predicted_exponential_to_gmres_cost_ratio",
    "cost_ratio_relative_prediction_error",
}
SUMMARY_TIMING_FIELDS = {
    "median_cost_ratio_prediction_error",
    "p95_cost_ratio_prediction_error",
    "median_exponential_prefix_inflation",
    "p95_exponential_prefix_inflation",
    "median_gmres_prefix_inflation",
    "p95_gmres_prefix_inflation",
    "all_prefix_prediction_gate_pass",
    "all_prefix_cost_gate_pass",
    "selected_p95_cost_ratio_prediction_error",
    "selected_p95_exponential_prefix_inflation",
    "selected_p95_gmres_prefix_inflation",
    "selected_prediction_gate_pass",
    "selected_prefix_cost_gate_pass",
}


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def strip_timing(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: strip_timing(child)
            for key, child in value.items()
            if key not in TIMING_FIELDS and key not in SUMMARY_TIMING_FIELDS
        }
    if isinstance(value, list):
        return [strip_timing(child) for child in value]
    return value


def quantile_higher(values: list[float], probability: float) -> float:
    array = np.asarray([value for value in values if math.isfinite(value)], dtype=float)
    if array.size == 0:
        return math.nan
    return float(np.quantile(array, probability, method="higher"))


def median(values: list[float]) -> float:
    array = np.asarray([value for value in values if math.isfinite(value)], dtype=float)
    return float(np.median(array)) if array.size else math.nan


def load_campaigns(directory: Path) -> tuple[list[Path], list[dict[str, Any]]]:
    paths = sorted(directory.glob("G4S4_CANONICAL_CORRECTED_*.json"))
    if len(paths) != 7:
        raise RuntimeError(f"expected 7 corrected campaigns, found {len(paths)}")
    campaigns = [json.loads(path.read_text()) for path in paths]
    return paths, campaigns


def aggregate_rows(campaigns: list[dict[str, Any]]) -> pd.DataFrame:
    records: list[dict[str, Any]] = []
    for campaign_index, campaign in enumerate(campaigns, start=1):
        for row in campaign["rows"]:
            records.append({"campaign": campaign_index, **row})
    raw = pd.DataFrame(records)
    group_columns = ["case_id", "prefix_dimension"]
    timing_columns = sorted(TIMING_FIELDS)
    scientific_columns = [
        column
        for column in raw.columns
        if column not in timing_columns and column not in group_columns and column != "campaign"
    ]
    aggregated: list[dict[str, Any]] = []
    for keys, group in raw.groupby(group_columns, sort=True):
        first = group.iloc[0]
        row: dict[str, Any] = {
            "case_id": keys[0],
            "prefix_dimension": int(keys[1]),
        }
        for column in scientific_columns:
            values = group[column].tolist()
            # All non-timing fields must be campaign invariant.
            normalized = [json.dumps(value, sort_keys=True) for value in values]
            if len(set(normalized)) != 1:
                raise RuntimeError(f"scientific field drift in {keys}: {column}")
            row[column] = values[0]
        for column in timing_columns:
            values = [float(value) for value in group[column].tolist() if pd.notna(value)]
            row[f"{column}_median"] = median(values)
            row[f"{column}_q25"] = float(np.quantile(values, 0.25)) if values else math.nan
            row[f"{column}_q75"] = float(np.quantile(values, 0.75)) if values else math.nan
            row[f"{column}_min"] = min(values) if values else math.nan
            row[f"{column}_max"] = max(values) if values else math.nan
        row["exponential_total_over_cold_median"] = median(
            (
                group["exponential_total_reused_wall_seconds"]
                / group["exponential_cold_wall_seconds"]
            ).tolist()
        )
        row["gmres_total_over_cold_median"] = median(
            (
                group["gmres_total_reused_wall_seconds"]
                / group["gmres_cold_wall_seconds"]
            ).tolist()
        )
        aggregated.append(row)
    return pd.DataFrame(aggregated)


def campaign_summary(campaigns: list[dict[str, Any]]) -> pd.DataFrame:
    records = []
    for index, campaign in enumerate(campaigns, start=1):
        summary = campaign["summary"]
        records.append(
            {
                "campaign": index,
                "rows": summary["rows"],
                "completed": summary["completed"],
                "selected_prediction_p95": summary[
                    "selected_p95_cost_ratio_prediction_error"
                ],
                "selected_exponential_inflation_p95": summary[
                    "selected_p95_exponential_prefix_inflation"
                ],
                "selected_gmres_inflation_p95": summary[
                    "selected_p95_gmres_prefix_inflation"
                ],
                "selected_prediction_gate_pass": summary[
                    "selected_prediction_gate_pass"
                ],
                "selected_prefix_cost_gate_pass": summary[
                    "selected_prefix_cost_gate_pass"
                ],
                "reuse_parity_gate_pass": summary["reuse_parity_gate_pass"],
            }
        )
    return pd.DataFrame(records)


def plot_results(rows: pd.DataFrame, campaigns: pd.DataFrame, plot_dir: Path) -> None:
    plot_dir.mkdir(parents=True, exist_ok=True)
    selected = rows[rows["prefix_dimension"] == 2].copy()

    # 1. Predicted versus actual E/R wall-cost ratio.
    plt.figure(figsize=(6.4, 5.2))
    for dimension, group in selected.groupby("dimension"):
        plt.scatter(
            group["actual_exponential_to_gmres_cost_ratio_median"],
            group["predicted_exponential_to_gmres_cost_ratio_median"],
            label=f"N={int(dimension)}",
        )
    maximum = float(
        max(
            selected["actual_exponential_to_gmres_cost_ratio_median"].max(),
            selected["predicted_exponential_to_gmres_cost_ratio_median"].max(),
        )
    )
    plt.plot([0, maximum * 1.05], [0, maximum * 1.05], linestyle="--", label="ideal")
    plt.xlabel("Actual E-K / R-JF wall-cost ratio")
    plt.ylabel("Prefix-predicted E-K / R-JF ratio")
    plt.title("Two-vector prefix cost-ratio prediction")
    plt.legend()
    plt.tight_layout()
    plt.savefig(plot_dir / "G4_S4_PREDICTED_VS_ACTUAL_COST_RATIO.png", dpi=180)
    plt.close()

    # 2. Prediction error by dimension and regime.
    selected = selected.sort_values(["dimension", "stiffness"])
    labels = [
        f"N={int(row.dimension)}\nS={int(row.stiffness)}"
        for row in selected.itertuples()
    ]
    values = selected["cost_ratio_relative_prediction_error_median"].to_numpy()
    plt.figure(figsize=(9.2, 4.8))
    plt.bar(np.arange(len(values)), values)
    plt.axhline(0.19, linestyle="--", label="sealed 19% gate")
    plt.xticks(np.arange(len(values)), labels, rotation=30, ha="right")
    plt.ylabel("Relative cost-ratio prediction error")
    plt.title("Selected two-vector prefix: campaign-median error")
    plt.legend()
    plt.tight_layout()
    plt.savefig(plot_dir / "G4_S4_PREDICTION_ERROR_BY_CASE.png", dpi=180)
    plt.close()

    # 3. Prefix-cost inflation.
    x = np.arange(len(selected))
    width = 0.36
    plt.figure(figsize=(9.2, 4.8))
    plt.bar(
        x - width / 2,
        selected["exponential_prefix_cost_inflation_median"],
        width,
        label="E-K Arnoldi prefix",
    )
    plt.bar(
        x + width / 2,
        selected["gmres_prefix_cost_inflation_median"],
        width,
        label="R-JF GMRES prefix",
    )
    plt.axhline(1.9, linestyle="--", label="sealed 1.9× gate")
    plt.xticks(x, labels, rotation=30, ha="right")
    plt.ylabel("Prefix-cost inflation over ideal work share")
    plt.title("Actual reusable-prefix cost")
    plt.legend()
    plt.tight_layout()
    plt.savefig(plot_dir / "G4_S4_PREFIX_COST_INFLATION.png", dpi=180)
    plt.close()

    # 4. End-to-end prefix-resume overhead relative to cold solve.
    plt.figure(figsize=(9.2, 4.8))
    plt.bar(
        x - width / 2,
        selected["exponential_total_over_cold_median"],
        width,
        label="E-K prefix + resume / cold",
    )
    plt.bar(
        x + width / 2,
        selected["gmres_total_over_cold_median"],
        width,
        label="R-JF prefix + resume / cold",
    )
    plt.axhline(1.0, linestyle="--", label="parity")
    plt.xticks(x, labels, rotation=30, ha="right")
    plt.ylabel("Median wall ratio")
    plt.title("Selected-prefix continuation overhead")
    plt.legend()
    plt.tight_layout()
    plt.savefig(plot_dir / "G4_S4_REUSE_TOTAL_OVER_COLD.png", dpi=180)
    plt.close()

    # 5. Campaign sensitivity of the sealed p95 gate.
    plt.figure(figsize=(7.0, 4.6))
    plt.plot(
        campaigns["campaign"],
        campaigns["selected_prediction_p95"],
        marker="o",
        label="campaign p95",
    )
    plt.axhline(0.19, linestyle="--", label="sealed 19% gate")
    plt.xlabel("Canonical campaign")
    plt.ylabel("Selected-prefix p95 prediction error")
    plt.title("Timing-campaign sensitivity")
    plt.legend()
    plt.tight_layout()
    plt.savefig(plot_dir / "G4_S4_CAMPAIGN_SENSITIVITY.png", dpi=180)
    plt.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--campaign-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1]
        / "results"
        / "campaigns_corrected",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    args = parser.parse_args()
    root = args.output_dir
    result_dir = root / "results"
    table_dir = root / "tables"
    plot_dir = root / "plots"
    result_dir.mkdir(parents=True, exist_ok=True)
    table_dir.mkdir(parents=True, exist_ok=True)

    paths, campaigns = load_campaigns(args.campaign_dir)
    scientific_hashes = [
        sha256_bytes(
            json.dumps(
                strip_timing(campaign),
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        )
        for campaign in campaigns
    ]
    rows = aggregate_rows(campaigns)
    campaigns_table = campaign_summary(campaigns)
    rows.to_csv(table_dir / "G4_S4_AGGREGATED_ROWS.csv", index=False)
    campaigns_table.to_csv(table_dir / "G4_S4_CAMPAIGN_SUMMARY.csv", index=False)

    selected = rows[rows["prefix_dimension"] == 2].copy()
    calibration = selected[selected["dimension"] == 32]
    holdout = selected[selected["dimension"].isin([128, 512])]
    selected_errors = selected["cost_ratio_relative_prediction_error_median"].tolist()
    calibration_errors = calibration[
        "cost_ratio_relative_prediction_error_median"
    ].tolist()
    holdout_errors = holdout["cost_ratio_relative_prediction_error_median"].tolist()
    e_inflation = selected["exponential_prefix_cost_inflation_median"].tolist()
    r_inflation = selected["gmres_prefix_cost_inflation_median"].tolist()
    e_total_ratio = selected["exponential_total_over_cold_median"].tolist()
    r_total_ratio = selected["gmres_total_over_cold_median"].tolist()
    max_e_defect = float(selected["exponential_value_relative_defect"].max())
    max_r_defect = float(selected["gmres_solution_relative_defect"].max())

    analysis = {
        "schema": "g4-s4-actual-prefix-kernel-analysis-v1",
        "status": "complete-hold-predictor",
        "active_switching_authorized": False,
        "inputs": {
            "campaigns": [path.name for path in paths],
            "campaign_count": len(paths),
            "scientific_sha256": scientific_hashes[0],
            "scientific_identity": len(set(scientific_hashes)) == 1,
            "timing_instrument": "auto-batched median per operation",
            "invalidated_attempt": "/mnt/data/g4s4_work/G4S4_CANONICAL_1.json",
        },
        "selected_prefix_dimension": 2,
        "negative_control_prefix_dimension": 1,
        "coverage": {
            "rows": int(len(rows)),
            "selected_rows": int(len(selected)),
            "calibration_rows": int(len(calibration)),
            "holdout_rows": int(len(holdout)),
            "completed_fraction": float(selected["completed"].mean()),
        },
        "reuse_parity": {
            "maximum_exponential_relative_defect": max_e_defect,
            "maximum_gmres_relative_defect": max_r_defect,
            "gate_max": 1.0e-12,
            "pass": max(max_e_defect, max_r_defect) <= 1.0e-12,
        },
        "cost_ratio_prediction": {
            "calibration_p95_higher": quantile_higher(calibration_errors, 0.95),
            "holdout_p95_higher": quantile_higher(holdout_errors, 0.95),
            "combined_median": median(selected_errors),
            "combined_p95_higher": quantile_higher(selected_errors, 0.95),
            "gate_max": 0.19,
            "pass": quantile_higher(selected_errors, 0.95) <= 0.19,
            "campaign_p95_range": [
                float(campaigns_table["selected_prediction_p95"].min()),
                float(campaigns_table["selected_prediction_p95"].max()),
            ],
            "campaigns_passing": int(
                campaigns_table["selected_prediction_gate_pass"].sum()
            ),
        },
        "prefix_cost": {
            "exponential_median_inflation": median(e_inflation),
            "exponential_p95_higher": quantile_higher(e_inflation, 0.95),
            "gmres_median_inflation": median(r_inflation),
            "gmres_p95_higher": quantile_higher(r_inflation, 0.95),
            "gate_max": 1.9,
            "pass": max(
                quantile_higher(e_inflation, 0.95),
                quantile_higher(r_inflation, 0.95),
            )
            <= 1.9,
        },
        "resume_economics": {
            "exponential_total_over_cold_median": median(e_total_ratio),
            "exponential_total_over_cold_p95_higher": quantile_higher(
                e_total_ratio, 0.95
            ),
            "gmres_total_over_cold_median": median(r_total_ratio),
            "gmres_total_over_cold_p95_higher": quantile_higher(
                r_total_ratio, 0.95
            ),
        },
        "observed_structure": {
            "gmres_iteration_prediction": (
                "usually exact or within two iterations in this corpus"
            ),
            "exponential_dimension_prediction": (
                "conservative overshoot; projected-exponential and orthogonalization "
                "costs make wall-ratio extrapolation nonlinear"
            ),
            "dominant_failed_case": (
                "N=512, stiffness=1000, nonnormality=0.9, oscillation=50"
            ),
        },
        "gates": {
            "scientific_identity": len(set(scientific_hashes)) == 1,
            "completed_holdout_fraction_one": float(holdout["completed"].mean()) == 1.0,
            "reuse_parity": max(max_e_defect, max_r_defect) <= 1.0e-12,
            "selected_prefix_prediction": quantile_higher(selected_errors, 0.95)
            <= 0.19,
            "selected_prefix_cost": max(
                quantile_higher(e_inflation, 0.95),
                quantile_higher(r_inflation, 0.95),
            )
            <= 1.9,
            "active_switching": False,
        },
        "closeout": {
            "actual_reusable_prefix_kernels": "PROMOTE",
            "exact_prefix_resume_parity": "PROMOTE",
            "two_vector_prefix_cost_predictor": "HOLD",
            "one_vector_prefix_predictor": "REJECT_NEGATIVE_CONTROL",
            "active_switching": "HOLD",
            "next_blocker": (
                "cost-ratio uncertainty must be represented as a conservative interval "
                "on expanded sealed telemetry; post-hoc point-predictor tuning is forbidden"
            ),
        },
    }
    (result_dir / "G4_S4_ANALYSIS.json").write_text(
        json.dumps(analysis, indent=2, sort_keys=True) + "\n"
    )
    plot_results(rows, campaigns_table, plot_dir)


if __name__ == "__main__":
    main()
