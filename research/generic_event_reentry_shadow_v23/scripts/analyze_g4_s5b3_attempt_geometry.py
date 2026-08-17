#!/usr/bin/env python3
"""Threshold-free analysis for the sealed G4-S5B3 early-defect campaign.

This script intentionally emits the complete threshold sweep and descriptive
geometry only. It never chooses, recommends, or freezes an admission threshold.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any, Iterable

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

SCHEMA = "g4-s5b3-attempt-geometry-analysis-v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def finite(values: Iterable[float]) -> np.ndarray:
    array = np.asarray(list(values), dtype=float)
    return array[np.isfinite(array)]


def percentile(values: Iterable[float], q: float) -> float | None:
    array = finite(values)
    if array.size == 0:
        return None
    return float(np.quantile(array, q, method="linear"))


def mad(values: Iterable[float]) -> float | None:
    array = finite(values)
    if array.size == 0:
        return None
    center = np.median(array)
    return float(np.median(np.abs(array - center)))


def ecdf(values: Iterable[float]) -> tuple[np.ndarray, np.ndarray]:
    array = np.sort(finite(values))
    if array.size == 0:
        return array, array
    return array, np.arange(1, array.size + 1, dtype=float) / array.size


def ks_gap(left: Iterable[float], right: Iterable[float]) -> float | None:
    a = np.sort(finite(left))
    b = np.sort(finite(right))
    if a.size == 0 or b.size == 0:
        return None
    grid = np.unique(np.concatenate([a, b]))
    fa = np.searchsorted(a, grid, side="right") / a.size
    fb = np.searchsorted(b, grid, side="right") / b.size
    return float(np.max(np.abs(fa - fb)))


def rank_auc(frame: pd.DataFrame) -> float | None:
    """Descriptive rank separation; larger eta is treated as reject-like.

    Attempts within a trajectory are serially dependent, so this is not an
    inferential ROC estimate and no confidence interval or p-value is emitted.
    """
    rejected = finite(frame.loc[frame["outcome"] == "rejected-error-control", "eta_c2"])
    accepted = finite(frame.loc[frame["outcome"] == "accepted", "eta_c2"])
    if rejected.size == 0 or accepted.size == 0:
        return None
    greater = np.greater(rejected[:, None], accepted[None, :]).sum()
    ties = np.equal(rejected[:, None], accepted[None, :]).sum()
    return float((greater + 0.5 * ties) / (rejected.size * accepted.size))


def exact_cross_outcome_collisions(frame: pd.DataFrame) -> list[dict[str, Any]]:
    """Return exact eta values observed under both accepted and rejected outcomes."""
    supported = frame[frame["finite_eta"]].copy()
    collision_values = (
        supported.groupby("eta_c2", sort=True)["outcome"].nunique().loc[lambda x: x > 1].index
    )
    rows: list[dict[str, Any]] = []
    for value in collision_values:
        subset = supported[supported["eta_c2"] == value]
        rows.append(
            {
                "eta_c2": float(value),
                "rows": [
                    {
                        "trajectory_id": str(row.trajectory_id),
                        "problem_id": str(row.problem_id),
                        "rtol": float(row.rtol),
                        "attempt_index": int(row.attempt_index),
                        "step_size": float(row.step_size),
                        "outcome": str(row.outcome),
                        "total_error_norm": float(row.total_error_norm),
                    }
                    for row in subset.itertuples(index=False)
                ],
            }
        )
    return rows


def pareto_frontier(sweep: pd.DataFrame) -> pd.DataFrame:
    """Non-dominated false-accept/false-reject rows, retaining one row per count pair."""
    rows: list[pd.Series] = []
    for _, row in sweep.iterrows():
        dominated = (
            (sweep["false_accept"] <= row["false_accept"])
            & (sweep["false_reject"] <= row["false_reject"])
            & (
                (sweep["false_accept"] < row["false_accept"])
                | (sweep["false_reject"] < row["false_reject"])
            )
        ).any()
        if not dominated:
            rows.append(row)
    return (
        pd.DataFrame(rows)
        .drop_duplicates(["false_accept", "false_reject"])
        .sort_values(["false_accept", "false_reject"])
        .reset_index(drop=True)
    )


def threshold_tradeoff_summary(sweep: pd.DataFrame) -> dict[str, Any]:
    accepted_total = int(sweep["accepted_total"].iloc[0])
    rejected_total = int(sweep["rejected_total"].iloc[0])
    zero_false_accept = sweep[sweep["false_accept"] == 0].sort_values(
        ["false_reject", "tau"]
    )
    require(not zero_false_accept.empty, "zero-false-accept sentinel missing")
    safest = zero_false_accept.iloc[0]
    scored = sweep.assign(total_misclassified=sweep["false_accept"] + sweep["false_reject"])
    minimum_total = int(scored["total_misclassified"].min())
    minimum_rows = scored[scored["total_misclassified"] == minimum_total]
    frontier = pareto_frontier(sweep)
    return {
        "all_admit_baseline": {
            "false_accept": rejected_total,
            "false_reject": 0,
            "total_misclassified": rejected_total,
        },
        "best_zero_false_accept": {
            "tau": float(safest["tau"]),
            "false_accept": int(safest["false_accept"]),
            "false_reject": int(safest["false_reject"]),
            "accepted_retained": accepted_total - int(safest["false_reject"]),
            "accepted_retained_fraction": float(
                (accepted_total - int(safest["false_reject"])) / accepted_total
            ),
        },
        "minimum_total_misclassification": {
            "count": minimum_total,
            "only_all_admit_achieves_minimum": bool(
                minimum_rows["false_accept"].eq(rejected_total).all()
                and minimum_rows["false_reject"].eq(0).all()
            ),
            "rows": int(len(minimum_rows)),
        },
        "pareto_points": [
            {
                "tau": float(row.tau),
                "false_accept": int(row.false_accept),
                "false_reject": int(row.false_reject),
                "hypothetically_admitted": int(row.hypothetically_admitted),
            }
            for row in frontier.itertuples(index=False)
        ],
    }


def outcome_stats(frame: pd.DataFrame) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for outcome in ["accepted", "rejected-error-control"]:
        values = finite(frame.loc[frame["outcome"] == outcome, "eta_c2"])
        result[outcome] = {
            "rows": int((frame["outcome"] == outcome).sum()),
            "finite_eta_rows": int(values.size),
            "minimum": float(values.min()) if values.size else None,
            "median": float(np.median(values)) if values.size else None,
            "maximum": float(values.max()) if values.size else None,
            "p05": percentile(values, 0.05),
            "p95": percentile(values, 0.95),
        }
    result["ks_gap"] = ks_gap(
        frame.loc[frame["outcome"] == "accepted", "eta_c2"],
        frame.loc[frame["outcome"] == "rejected-error-control", "eta_c2"],
    )
    result["rank_auc_larger_eta_predicts_reject"] = rank_auc(frame)
    return result


def flatten_attempts(report: dict[str, Any]) -> pd.DataFrame:
    frame = pd.json_normalize(report["attempts"], sep=".")
    if "eta_c2" not in frame:
        frame["eta_c2"] = np.nan
    frame["eta_c2"] = pd.to_numeric(frame["eta_c2"], errors="coerce")
    frame["rtol"] = pd.to_numeric(frame["rtol"], errors="raise")
    frame["atol"] = pd.to_numeric(frame["atol"], errors="raise")
    frame["finite_eta"] = np.isfinite(frame["eta_c2"])
    frame["accepted"] = frame["outcome"].eq("accepted")
    frame["rejected_error_control"] = frame["outcome"].eq("rejected-error-control")
    return frame


def threshold_sweep(frame: pd.DataFrame) -> pd.DataFrame:
    supported = frame[frame["finite_eta"]].copy()
    values = np.sort(supported["eta_c2"].unique())
    require(values.size > 0, "no finite eta values in canonical campaign")
    thresholds = np.concatenate(
        [
            np.asarray([np.nextafter(values[0], -np.inf)]),
            values,
            np.asarray([np.nextafter(values[-1], np.inf)]),
        ]
    )
    accepted_total = int(frame["accepted"].sum())
    rejected_total = int(frame["rejected_error_control"].sum())
    accepted_supported = int((frame["accepted"] & frame["finite_eta"]).sum())
    rejected_supported = int((frame["rejected_error_control"] & frame["finite_eta"]).sum())
    rows: list[dict[str, Any]] = []
    for index, tau in enumerate(thresholds):
        admit = supported["eta_c2"] <= tau
        false_reject = int((supported["accepted"] & ~admit).sum())
        false_accept = int((supported["rejected_error_control"] & admit).sum())
        rows.append(
            {
                "threshold_index": index,
                "tau": float(tau),
                "false_reject": false_reject,
                "false_accept": false_accept,
                "accepted_total": accepted_total,
                "rejected_total": rejected_total,
                "accepted_finite_support": accepted_supported,
                "rejected_finite_support": rejected_supported,
                "unsupported_accepted": accepted_total - accepted_supported,
                "unsupported_rejected": rejected_total - rejected_supported,
                "hypothetically_admitted": int(admit.sum()),
                "hypothetically_rejected": int((~admit).sum()),
            }
        )
    return pd.DataFrame(rows)


def set_eta_scale(axis: Any, values: pd.Series) -> None:
    positive = finite(values[values > 0])
    if positive.size:
        axis.set_xscale("symlog", linthresh=max(float(positive.min()) / 10.0, 1e-300))


def plot_pooled_cdf(frame: pd.DataFrame, path: Path) -> None:
    fig, ax = plt.subplots(figsize=(7.2, 4.6))
    for outcome, label in [
        ("accepted", "accepted"),
        ("rejected-error-control", "rejected by error control"),
    ]:
        x, y = ecdf(frame.loc[frame["outcome"] == outcome, "eta_c2"])
        if x.size:
            ax.step(x, y, where="post", label=f"{label} (n={x.size})")
            ax.scatter(x, y, s=14, alpha=0.65)
    set_eta_scale(ax, frame["eta_c2"])
    ax.set_xlabel(r"early defect $\eta_{c_2}$")
    ax.set_ylabel("empirical CDF")
    ax.set_title("Pooled early-defect distributions by actual attempt outcome")
    ax.grid(True, alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def plot_group_cdf(frame: pd.DataFrame, column: str, value: Any, path: Path, title: str) -> None:
    group = frame[frame[column] == value]
    fig, ax = plt.subplots(figsize=(7.2, 4.6))
    for outcome, label in [
        ("accepted", "accepted"),
        ("rejected-error-control", "rejected by error control"),
    ]:
        x, y = ecdf(group.loc[group["outcome"] == outcome, "eta_c2"])
        if x.size:
            ax.step(x, y, where="post", label=f"{label} (n={x.size})")
            ax.scatter(x, y, s=14, alpha=0.65)
    set_eta_scale(ax, group["eta_c2"])
    ax.set_xlabel(r"early defect $\eta_{c_2}$")
    ax.set_ylabel("empirical CDF")
    ax.set_title(title)
    ax.grid(True, alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def plot_false_count_curve(sweep: pd.DataFrame, path: Path) -> None:
    fig, ax = plt.subplots(figsize=(7.0, 4.8))
    ax.plot(sweep["false_accept"], sweep["false_reject"], marker=".", linewidth=1.0)
    ax.scatter(
        [sweep.iloc[0]["false_accept"], sweep.iloc[-1]["false_accept"]],
        [sweep.iloc[0]["false_reject"], sweep.iloc[-1]["false_reject"]],
        marker="x",
        s=70,
        label="boundary sentinels",
    )
    frontier = pareto_frontier(sweep)
    ax.plot(
        frontier["false_accept"],
        frontier["false_reject"],
        linestyle="none",
        marker="D",
        markersize=7,
        fillstyle="none",
        label="Pareto count frontier",
    )
    ax.set_xlabel("false accepts: rejected attempts hypothetically admitted")
    ax.set_ylabel("false rejects: accepted attempts hypothetically rejected")
    ax.set_title("Complete threshold sweep; no threshold selected")
    ax.grid(True, alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def plot_exact_collision(collision: dict[str, Any], path: Path) -> None:
    frame = pd.DataFrame(collision["rows"]).sort_values("rtol", ascending=False)
    fig, ax = plt.subplots(figsize=(7.2, 4.6))
    accepted = frame[frame["outcome"] == "accepted"]
    rejected = frame[frame["outcome"] == "rejected-error-control"]
    if not accepted.empty:
        ax.scatter(
            accepted["rtol"],
            accepted["total_error_norm"],
            marker="o",
            s=70,
            label="accepted",
        )
    if not rejected.empty:
        ax.scatter(
            rejected["rtol"],
            rejected["total_error_norm"],
            marker="x",
            s=85,
            label="rejected by error control",
        )
    ax.axhline(1.0, linewidth=1.0, linestyle="--", label="acceptance boundary")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.invert_xaxis()
    ax.set_xlabel("relative tolerance")
    ax.set_ylabel("actual total error norm")
    ax.set_title(
        rf"Exact score collision: $\eta_{{c_2}}={collision['eta_c2']:.4g}$"
    )
    ax.grid(True, alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def plot_attempt_concentration(frame: pd.DataFrame, column: str, path: Path, title: str) -> None:
    counts = (
        frame.groupby([column, "outcome"], sort=True)
        .size()
        .unstack(fill_value=0)
        .reindex(columns=["accepted", "rejected-error-control"], fill_value=0)
    )
    x = np.arange(len(counts))
    fig, ax = plt.subplots(figsize=(8.0, 4.8))
    ax.bar(x, counts["accepted"], label="accepted")
    ax.bar(
        x,
        counts["rejected-error-control"],
        bottom=counts["accepted"],
        label="rejected by error control",
    )
    ax.set_xticks(x)
    ax.set_xticklabels([str(value) for value in counts.index], rotation=25, ha="right")
    ax.set_ylabel("attempt count")
    ax.set_title(title)
    ax.grid(True, axis="y", alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def overhead_frame(report: dict[str, Any]) -> pd.DataFrame:
    overhead = report.get("overhead")
    require(overhead is not None, "overhead report missing after hard-gate pass")
    rows: list[dict[str, Any]] = []
    for row in overhead["measured_rows"]:
        rows.append(
            {
                "pair_index": int(row["pair_index"]),
                "order": row["order"],
                "repetitions": int(row["disabled"]["repetitions"]),
                "disabled_wall_seconds": float(row["disabled"]["wall_seconds"]),
                "read_only_wall_seconds": float(row["read_only"]["wall_seconds"]),
                "disabled_gamma": float(row["disabled"]["gamma_seconds_per_interval"]),
                "read_only_gamma": float(row["read_only"]["gamma_seconds_per_interval"]),
                "wall_ratio": float(row["wall_ratio_read_only_over_disabled"]),
                "gamma_ratio": float(row["gamma_ratio_read_only_over_disabled"]),
                "disabled_interval": float(row["disabled"]["proposed_interval"]),
                "read_only_interval": float(row["read_only"]["proposed_interval"]),
                "identity_pass": bool(row["disabled"]["all_suite_identities_passed"])
                and bool(row["read_only"]["all_suite_identities_passed"]),
            }
        )
    return pd.DataFrame(rows)


def plot_overhead(frame: pd.DataFrame, path: Path) -> None:
    fig, ax = plt.subplots(figsize=(7.0, 4.6))
    ax.plot(frame["pair_index"], frame["wall_ratio"], marker="o", label="wall ratio")
    ax.plot(frame["pair_index"], frame["gamma_ratio"], marker="x", label=r"$\Gamma$ ratio")
    ax.axhline(1.0, linewidth=1.0, linestyle="--")
    for row in frame.itertuples(index=False):
        ax.annotate(
            "D→R" if row.order == "disabled-first" else "R→D",
            (row.pair_index, row.wall_ratio),
            xytext=(0, 6),
            textcoords="offset points",
            ha="center",
            fontsize=8,
        )
    ax.set_xlabel("measured pair index")
    ax.set_ylabel("read-only / disabled")
    ax.set_title("Paired telemetry overhead under alternating arm order")
    ax.grid(True, alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def plot_coverage(trajectories: pd.DataFrame, path: Path) -> None:
    frame = trajectories.copy()
    frame["eta_coverage"] = frame["finite_eta_attempts"] / frame["attempts"].clip(lower=1)
    frame["work_coverage"] = frame["per_trial_work_complete"].astype(float)
    frame["identity_coverage"] = frame["all_required_gates_pass"].astype(float)
    x = np.arange(len(frame))
    width = 0.25
    fig, ax = plt.subplots(figsize=(11.0, 5.0))
    ax.bar(x - width, frame["eta_coverage"], width=width, label="finite eta coverage")
    ax.bar(x, frame["work_coverage"], width=width, label="trial-work coverage")
    ax.bar(x + width, frame["identity_coverage"], width=width, label="identity gates")
    ax.set_xticks(x)
    ax.set_xticklabels(frame["trajectory_id"], rotation=45, ha="right")
    ax.set_ylim(0.0, 1.05)
    ax.set_ylabel("coverage fraction")
    ax.set_title("Attempt coverage and work-completeness audit")
    ax.grid(True, axis="y", alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def concentration(frame: pd.DataFrame, column: str) -> dict[str, Any]:
    counts = frame.groupby(column, dropna=False).size().sort_values(ascending=False)
    total = int(counts.sum())
    return {
        "counts": {str(key): int(value) for key, value in counts.items()},
        "largest_share": float(counts.iloc[0] / total) if total else None,
    }


def leave_one_group_out(frame: pd.DataFrame, column: str) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for value in sorted(frame[column].unique(), key=str):
        subset = frame[frame[column] != value]
        result[str(value)] = outcome_stats(subset)
    return result


def main() -> None:
    args = parse_args()
    report = json.loads(args.input.read_text(encoding="utf-8"))
    require(report.get("schema") == "g4-s5b3-attempt-geometry-raw-v1", "unexpected raw schema")
    require(report.get("active_switching") is False, "active switching unexpectedly enabled")
    require(report.get("early_abort") is False, "early abort unexpectedly enabled")
    require(report.get("threshold_selected") is False, "threshold was selected in forbidden node")
    require(report.get("selected_threshold") is None, "selected threshold must remain null")
    require(report["hard_gates"]["passed"] is True, "hard gate failed; analysis forbidden")
    require(report["status"] == "pass", f"raw runner status is {report['status']!r}")

    output = args.output_dir
    results_dir = output / "results"
    plots_dir = output / "plots"
    reports_dir = output / "reports"
    for directory in [results_dir, plots_dir, reports_dir]:
        directory.mkdir(parents=True, exist_ok=True)

    attempts = flatten_attempts(report)
    trajectories = pd.json_normalize(report["trajectories"], sep=".")
    overhead = overhead_frame(report)

    require(len(trajectories) == 12, "canonical campaign did not contain 12 trajectories")
    require(not attempts.empty, "canonical campaign emitted no attempts")
    require((attempts["outcome"] == "trial-failure-unscorable").sum() == 0, "unscorable attempt found")
    require((attempts["outcome"] == "legacy-unclassified").sum() == 0, "legacy-unclassified attempt found")
    require(overhead["identity_pass"].all(), "paired overhead identity failure")
    require(np.array_equal(overhead["disabled_interval"].to_numpy(), overhead["read_only_interval"].to_numpy()), "paired physical intervals differ")

    sweep = threshold_sweep(attempts)
    attempts.to_csv(results_dir / "G4_S5B3_ATTEMPT_ROWS.csv", index=False)
    trajectories.to_csv(results_dir / "G4_S5B3_TRAJECTORY_SUMMARY.csv", index=False)
    sweep.to_csv(results_dir / "G4_S5B3_THRESHOLD_SWEEP.csv", index=False)
    pareto_frontier(sweep).to_csv(
        results_dir / "G4_S5B3_THRESHOLD_PARETO_FRONTIER.csv", index=False
    )
    overhead.to_csv(results_dir / "G4_S5B3_OVERHEAD_PAIRS.csv", index=False)

    plot_pooled_cdf(attempts, plots_dir / "G4_S5B3_POOLED_ETA_CDF.png")
    for problem in sorted(attempts["problem_id"].unique()):
        plot_group_cdf(
            attempts,
            "problem_id",
            problem,
            plots_dir / f"G4_S5B3_ETA_CDF_PROBLEM_{problem}.png",
            f"Early-defect CDF — {problem}",
        )
    for rtol in sorted(attempts["rtol"].unique(), reverse=True):
        token = f"{rtol:.0e}".replace("-", "m")
        plot_group_cdf(
            attempts,
            "rtol",
            rtol,
            plots_dir / f"G4_S5B3_ETA_CDF_RTOL_{token}.png",
            f"Early-defect CDF — rtol={rtol:.0e}",
        )
    plot_false_count_curve(sweep, plots_dir / "G4_S5B3_FALSE_COUNT_CURVE.png")
    plot_overhead(overhead, plots_dir / "G4_S5B3_PAIRED_OVERHEAD.png")
    plot_coverage(trajectories, plots_dir / "G4_S5B3_COVERAGE_AUDIT.png")
    collisions = exact_cross_outcome_collisions(attempts)
    for index, collision in enumerate(collisions):
        plot_exact_collision(
            collision, plots_dir / f"G4_S5B3_EXACT_SCORE_COLLISION_{index}.png"
        )
    plot_attempt_concentration(
        attempts,
        "problem_id",
        plots_dir / "G4_S5B3_ATTEMPT_COUNT_BY_PROBLEM.png",
        "Attempt concentration by problem",
    )
    attempts_for_tolerance = attempts.assign(
        tolerance_label=attempts["rtol"].map(lambda value: f"{float(value):.0e}")
    )
    plot_attempt_concentration(
        attempts_for_tolerance,
        "tolerance_label",
        plots_dir / "G4_S5B3_ATTEMPT_COUNT_BY_TOLERANCE.png",
        "Attempt concentration by relative tolerance",
    )

    zero_eta = int((attempts["finite_eta"] & attempts["eta_c2"].eq(0.0)).sum())
    finite_rows = int(attempts["finite_eta"].sum())
    analysis: dict[str, Any] = {
        "schema": SCHEMA,
        "status": "pass-threshold-free-analysis-complete",
        "source_raw_report": str(args.input),
        "selected_threshold": None,
        "threshold_recommendation": None,
        "active_switching": False,
        "early_abort": False,
        "corpus": {
            "trajectories": int(len(trajectories)),
            "attempts": int(len(attempts)),
            "accepted": int(attempts["accepted"].sum()),
            "rejected_error_control": int(attempts["rejected_error_control"].sum()),
            "finite_eta": finite_rows,
            "finite_eta_fraction": float(finite_rows / len(attempts)),
            "zero_eta": zero_eta,
            "null_eta": int(len(attempts) - finite_rows),
            "degenerate_nonzero_remainder": int(
                attempts.get("degenerate_nonzero_remainder", pd.Series(dtype=bool)).fillna(False).sum()
            ),
            "nonfinite_normalization": int(
                attempts.get("nonfinite_normalization", pd.Series(dtype=bool)).fillna(False).sum()
            ),
        },
        "pooled_geometry": outcome_stats(attempts),
        "threshold_tradeoff": threshold_tradeoff_summary(sweep),
        "exact_cross_outcome_eta_collisions": collisions,
        "per_problem_geometry": {
            str(problem): outcome_stats(group)
            for problem, group in attempts.groupby("problem_id", sort=True)
        },
        "per_tolerance_geometry": {
            f"{float(rtol):.0e}": outcome_stats(group)
            for rtol, group in attempts.groupby("rtol", sort=True)
        },
        "concentration": {
            "problem": concentration(attempts, "problem_id"),
            "tolerance": concentration(attempts, "rtol"),
        },
        "adversarial_leave_one_out": {
            "problem": leave_one_group_out(attempts, "problem_id"),
            "tolerance": leave_one_group_out(attempts, "rtol"),
        },
        "threshold_sweep": {
            "rows": int(len(sweep)),
            "distinct_finite_eta": int(attempts.loc[attempts["finite_eta"], "eta_c2"].nunique()),
            "lower_sentinel": float(sweep.iloc[0]["tau"]),
            "upper_sentinel": float(sweep.iloc[-1]["tau"]),
            "false_accept_range": [int(sweep["false_accept"].min()), int(sweep["false_accept"].max())],
            "false_reject_range": [int(sweep["false_reject"].min()), int(sweep["false_reject"].max())],
        },
        "paired_overhead": {
            "pairs": int(len(overhead)),
            "repetitions_per_arm": int(overhead["repetitions"].iloc[0]),
            "wall_ratio_median": percentile(overhead["wall_ratio"], 0.5),
            "wall_ratio_p95": percentile(overhead["wall_ratio"], 0.95),
            "wall_ratio_mad": mad(overhead["wall_ratio"]),
            "gamma_ratio_median": percentile(overhead["gamma_ratio"], 0.5),
            "gamma_ratio_p95": percentile(overhead["gamma_ratio"], 0.95),
            "gamma_ratio_mad": mad(overhead["gamma_ratio"]),
            "orders": overhead["order"].tolist(),
            "order_stratified_wall_ratio_median": {
                str(order): percentile(group["wall_ratio"], 0.5)
                for order, group in overhead.groupby("order", sort=True)
            },
            "all_identity_pass": bool(overhead["identity_pass"].all()),
        },
        "hard_gates": report["hard_gates"],
        "interpretation_limits": report["limitations"]
        + [
            "rank AUC is descriptive because attempts within trajectories are serially dependent",
            "only seven rejected attempts were observed, six from oscillatory-pr",
            "pooled geometry is dominated by oscillatory-pr and rtol=1e-8",
        ],
    }
    (results_dir / "G4_S5B3_ANALYSIS.json").write_text(
        json.dumps(analysis, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    verifier = {
        "schema": "g4-s5b3-analysis-verifier-v1",
        "hard_gate_pass": bool(report["hard_gates"]["passed"]),
        "all_trajectory_gates_pass": bool(trajectories["all_required_gates_pass"].all()),
        "zero_unscorable": bool((attempts["outcome"] == "trial-failure-unscorable").sum() == 0),
        "zero_legacy_unclassified": bool((attempts["outcome"] == "legacy-unclassified").sum() == 0),
        "all_overhead_identity_pass": bool(overhead["identity_pass"].all()),
        "equal_paired_intervals": bool(np.array_equal(overhead["disabled_interval"], overhead["read_only_interval"])),
        "threshold_selected": False,
        "analysis_pass": True,
    }
    (results_dir / "G4_S5B3_ANALYSIS_VERIFIER.json").write_text(
        json.dumps(verifier, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
