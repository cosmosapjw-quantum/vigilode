#!/usr/bin/env python3
"""Threshold-free P1-00 analysis for raw and tolerance-scaled early defect.

The script exhaustively sweeps observed score boundaries but never selects,
recommends, or freezes an admission threshold. Two views are predeclared:
all attempts and entry-window attempts with attempt_index <= 1.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

SCHEMA = "p1-00-tolerance-scaled-early-defect-analysis-v1"
SCORES = {"raw": "eta_c2", "scaled": "rho_c2_wrms"}
VIEWS = {"all": lambda f: f, "entry": lambda f: f[f["attempt_index"] <= 1].copy()}


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--input", type=Path, required=True)
    p.add_argument("--output-dir", type=Path, required=True)
    return p.parse_args()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def finite(values: Iterable[float]) -> np.ndarray:
    a = np.asarray(list(values), dtype=float)
    return a[np.isfinite(a)]


def percentile(values: Iterable[float], q: float) -> float | None:
    a = finite(values)
    return None if a.size == 0 else float(np.quantile(a, q, method="linear"))


def mad(values: Iterable[float]) -> float | None:
    a = finite(values)
    if a.size == 0:
        return None
    center = np.median(a)
    return float(np.median(np.abs(a - center)))


def ks_gap(left: Iterable[float], right: Iterable[float]) -> float | None:
    a = np.sort(finite(left))
    b = np.sort(finite(right))
    if a.size == 0 or b.size == 0:
        return None
    grid = np.unique(np.concatenate([a, b]))
    fa = np.searchsorted(a, grid, side="right") / a.size
    fb = np.searchsorted(b, grid, side="right") / b.size
    return float(np.max(np.abs(fa - fb)))


def rank_auc(frame: pd.DataFrame, score: str) -> float | None:
    rejected = finite(frame.loc[frame["rejected"], score])
    accepted = finite(frame.loc[frame["accepted"], score])
    if rejected.size == 0 or accepted.size == 0:
        return None
    greater = np.greater(rejected[:, None], accepted[None, :]).sum()
    ties = np.equal(rejected[:, None], accepted[None, :]).sum()
    return float((greater + 0.5 * ties) / (rejected.size * accepted.size))


def flatten_attempts(report: dict[str, Any]) -> pd.DataFrame:
    frame = pd.json_normalize(report["attempts"], sep=".")
    for score in SCORES.values():
        frame[score] = pd.to_numeric(frame[score], errors="coerce")
        frame[f"finite_{score}"] = np.isfinite(frame[score])
    for col in ["rtol", "atol", "total_error_norm", "step_size", "attempt_index"]:
        frame[col] = pd.to_numeric(frame[col], errors="raise")
    frame["accepted"] = frame["outcome"].eq("accepted")
    frame["rejected"] = frame["outcome"].eq("rejected-error-control")
    frame["entry_window"] = frame["attempt_index"] <= 1
    return frame


def threshold_sweep(frame: pd.DataFrame, score: str) -> pd.DataFrame:
    supported = frame[np.isfinite(frame[score])].copy()
    values = np.sort(supported[score].unique())
    require(values.size > 0, f"no finite values for {score}")
    thresholds = np.concatenate(
        ([np.nextafter(values[0], -np.inf)], values, [np.nextafter(values[-1], np.inf)])
    )
    accepted_total = int(frame["accepted"].sum())
    rejected_total = int(frame["rejected"].sum())
    rows = []
    for i, tau in enumerate(thresholds):
        admit = supported[score] <= tau
        fa = int((supported["rejected"] & admit).sum())
        fr = int((supported["accepted"] & ~admit).sum())
        rows.append(
            {
                "threshold_index": i,
                "tau": float(tau),
                "false_accept": fa,
                "false_reject": fr,
                "total_misclassified": fa + fr,
                "accepted_total": accepted_total,
                "rejected_total": rejected_total,
                "hypothetically_admitted": int(admit.sum()),
                "hypothetically_rejected": int((~admit).sum()),
            }
        )
    return pd.DataFrame(rows)


def pareto_frontier(sweep: pd.DataFrame) -> pd.DataFrame:
    keep = []
    for idx, row in sweep.iterrows():
        dominated = (
            (sweep["false_accept"] <= row["false_accept"])
            & (sweep["false_reject"] <= row["false_reject"])
            & (
                (sweep["false_accept"] < row["false_accept"])
                | (sweep["false_reject"] < row["false_reject"])
            )
        ).any()
        if not dominated:
            keep.append(idx)
    return (
        sweep.loc[keep]
        .drop_duplicates(["false_accept", "false_reject"])
        .sort_values(["false_accept", "false_reject", "tau"])
        .reset_index(drop=True)
    )


def tradeoff_summary(sweep: pd.DataFrame) -> dict[str, Any]:
    accepted_total = int(sweep["accepted_total"].iloc[0])
    rejected_total = int(sweep["rejected_total"].iloc[0])
    zero_fa = sweep[sweep["false_accept"] == 0].sort_values(["false_reject", "tau"])
    require(not zero_fa.empty, "zero-false-accept sentinel missing")
    z = zero_fa.iloc[0]
    minimum = int(sweep["total_misclassified"].min())
    minimum_rows = sweep[sweep["total_misclassified"] == minimum]
    frontier = pareto_frontier(sweep)
    return {
        "all_admit_baseline": {
            "false_accept": rejected_total,
            "false_reject": 0,
            "total_misclassified": rejected_total,
        },
        "descriptive_best_zero_false_accept": {
            "tau": float(z["tau"]),
            "false_accept": 0,
            "false_reject": int(z["false_reject"]),
            "accepted_retained": accepted_total - int(z["false_reject"]),
            "accepted_retained_fraction": (
                float((accepted_total - int(z["false_reject"])) / accepted_total)
                if accepted_total
                else None
            ),
        },
        "minimum_total_misclassification": {
            "count": minimum,
            "strictly_better_than_all_admit": minimum < rejected_total,
            "rows": int(len(minimum_rows)),
        },
        "pareto_points": [
            {
                "tau": float(r.tau),
                "false_accept": int(r.false_accept),
                "false_reject": int(r.false_reject),
            }
            for r in frontier.itertuples(index=False)
        ],
    }


def outcome_stats(frame: pd.DataFrame, score: str) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for label, mask in [("accepted", frame["accepted"]), ("rejected", frame["rejected"])]:
        values = finite(frame.loc[mask, score])
        result[label] = {
            "rows": int(mask.sum()),
            "minimum": float(values.min()) if values.size else None,
            "median": float(np.median(values)) if values.size else None,
            "maximum": float(values.max()) if values.size else None,
            "p05": percentile(values, 0.05),
            "p95": percentile(values, 0.95),
        }
    result["ks_gap"] = ks_gap(frame.loc[frame["accepted"], score], frame.loc[frame["rejected"], score])
    result["rank_auc_larger_score_predicts_reject"] = rank_auc(frame, score)
    result["spearman_vs_total_error_norm"] = float(
        frame[[score, "total_error_norm"]].corr(method="spearman").iloc[0, 1]
    )
    return result


def exact_cross_outcome_collisions(frame: pd.DataFrame, score: str) -> list[dict[str, Any]]:
    supported = frame[np.isfinite(frame[score])]
    values = supported.groupby(score, sort=True)["outcome"].nunique()
    collisions = values[values > 1].index
    result = []
    for value in collisions:
        rows = supported[supported[score] == value]
        result.append(
            {
                "score": float(value),
                "rows": [
                    {
                        "trajectory_id": str(r.trajectory_id),
                        "problem_id": str(r.problem_id),
                        "rtol": float(r.rtol),
                        "attempt_index": int(r.attempt_index),
                        "outcome": str(r.outcome),
                        "total_error_norm": float(r.total_error_norm),
                    }
                    for r in rows.itertuples(index=False)
                ],
            }
        )
    return result


def lopo_table(frame: pd.DataFrame) -> pd.DataFrame:
    rows = []
    for view_name, select in VIEWS.items():
        view = select(frame)
        for left_out in sorted(frame["problem_id"].unique()):
            subset = view[view["problem_id"] != left_out].copy()
            for score_name, score in SCORES.items():
                sweep = threshold_sweep(subset, score)
                summary = tradeoff_summary(sweep)
                z = summary["descriptive_best_zero_false_accept"]
                rows.append(
                    {
                        "view": view_name,
                        "left_out_problem": left_out,
                        "score": score_name,
                        "rows": len(subset),
                        "accepted": int(subset["accepted"].sum()),
                        "rejected": int(subset["rejected"].sum()),
                        "rank_auc": rank_auc(subset, score),
                        "minimum_total_misclassification": summary[
                            "minimum_total_misclassification"
                        ]["count"],
                        "strictly_better_than_all_admit": summary[
                            "minimum_total_misclassification"
                        ]["strictly_better_than_all_admit"],
                        "zero_fa_false_reject": z["false_reject"],
                        "zero_fa_accepted_retained_fraction": z[
                            "accepted_retained_fraction"
                        ],
                    }
                )
    return pd.DataFrame(rows)


def overhead_table(report: dict[str, Any]) -> pd.DataFrame:
    overhead = report.get("overhead") or {}
    rows = overhead.get("measured_rows") or []
    return pd.json_normalize(rows, sep=".")


def grouped_distribution(frame: pd.DataFrame, group_col: str) -> pd.DataFrame:
    rows = []
    for group_value, group in frame.groupby(group_col, sort=True):
        for score_name, score in SCORES.items():
            for outcome, mask in [("accepted", group["accepted"]), ("rejected-error-control", group["rejected"])]:
                values = finite(group.loc[mask, score])
                rows.append(
                    {
                        group_col: group_value,
                        "score": score_name,
                        "outcome": outcome,
                        "rows": int(mask.sum()),
                        "minimum": float(values.min()) if values.size else None,
                        "median": float(np.median(values)) if values.size else None,
                        "maximum": float(values.max()) if values.size else None,
                        "p05": percentile(values, 0.05),
                        "p95": percentile(values, 0.95),
                    }
                )
    return pd.DataFrame(rows)


def plot_threshold_compare(sweeps: dict[str, pd.DataFrame], output: Path, view: str) -> None:
    fig, ax = plt.subplots(figsize=(6.8, 4.6))
    for score_name in ["raw", "scaled"]:
        s = sweeps[f"{view}_{score_name}"]
        ax.plot(s["false_accept"], s["false_reject"], marker="o", markersize=2.8, linewidth=1.0, label=score_name)
    ax.set_xlabel("False accept count")
    ax.set_ylabel("False reject count")
    ax.set_title(f"Complete threshold sweep — {view} view")
    ax.legend()
    ax.grid(True, alpha=0.25)
    fig.tight_layout()
    fig.savefig(output, dpi=180)
    plt.close(fig)


def plot_score_error(frame: pd.DataFrame, score: str, output: Path, title: str) -> None:
    fig, ax = plt.subplots(figsize=(6.8, 4.6))
    for label, marker in [("accepted", "o"), ("rejected-error-control", "x")]:
        sub = frame[frame["outcome"] == label]
        ax.scatter(sub[score], sub["total_error_norm"], marker=marker, s=24, label=label, alpha=0.8)
    if (frame[score] > 0).all():
        ax.set_xscale("log")
    if (frame["total_error_norm"] > 0).all():
        ax.set_yscale("log")
    ax.axhline(1.0, linewidth=1.0, linestyle="--")
    ax.set_xlabel(score)
    ax.set_ylabel("Adaptive total-error norm")
    ax.set_title(title)
    ax.legend()
    ax.grid(True, alpha=0.25)
    fig.tight_layout()
    fig.savefig(output, dpi=180)
    plt.close(fig)


def plot_quadratic_triplet(frame: pd.DataFrame, output: Path) -> None:
    q = frame[(frame["problem_id"] == "quadratic") & (frame["attempt_index"] == 0)].sort_values("rtol")
    require(len(q) == 3, "quadratic triplet not found")
    fig, ax = plt.subplots(figsize=(6.8, 4.6))
    ax.plot(q["rtol"], q["rho_c2_wrms"], marker="o", label="scaled defect")
    ax.plot(q["rtol"], q["total_error_norm"], marker="s", label="adaptive total-error norm")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.invert_xaxis()
    ax.axhline(1.0, linewidth=1.0, linestyle="--")
    ax.set_xlabel("rtol")
    ax.set_ylabel("Dimensionless magnitude")
    ax.set_title("Quadratic first-attempt tolerance triplet")
    ax.legend()
    ax.grid(True, alpha=0.25)
    fig.tight_layout()
    fig.savefig(output, dpi=180)
    plt.close(fig)


def plot_ecdf(frame: pd.DataFrame, score: str, output: Path, title: str) -> None:
    fig, ax = plt.subplots(figsize=(6.8, 4.6))
    for label in ["accepted", "rejected-error-control"]:
        values = np.sort(finite(frame.loc[frame["outcome"] == label, score]))
        if not values.size:
            continue
        y = np.arange(1, values.size + 1) / values.size
        ax.step(values, y, where="post", label=label)
    if (frame[score] > 0).all():
        ax.set_xscale("log")
    ax.set_xlabel(score)
    ax.set_ylabel("Empirical CDF")
    ax.set_title(title)
    ax.legend()
    ax.grid(True, alpha=0.25)
    fig.tight_layout()
    fig.savefig(output, dpi=180)
    plt.close(fig)


def plot_overhead(overhead: pd.DataFrame, output: Path) -> None:
    require(not overhead.empty, "paired overhead rows missing")
    ratio = overhead["wall_ratio_read_only_over_disabled"].to_numpy(float)
    fig, ax = plt.subplots(figsize=(6.8, 4.6))
    ax.plot(np.arange(1, len(ratio) + 1), ratio, marker="o")
    ax.axhline(1.0, linewidth=1.0, linestyle="--")
    ax.set_xlabel("Measured pair")
    ax.set_ylabel("scaled-read-only / disabled wall ratio")
    ax.set_title("Paired telemetry overhead")
    ax.grid(True, alpha=0.25)
    fig.tight_layout()
    fig.savefig(output, dpi=180)
    plt.close(fig)


def main() -> None:
    args = parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "tables").mkdir(exist_ok=True)
    (args.output_dir / "figures").mkdir(exist_ok=True)

    report = json.loads(args.input.read_text())
    require(report["schema"] == "p1-00-tolerance-scaled-early-defect-raw-v1", "unexpected input schema")
    require(report["status"] == "pass", "raw campaign status is not pass")
    require(report["hard_gates"]["passed"], "raw campaign hard gate failed")
    require(not report["active_switching"] and not report["early_abort"], "active behavior leaked into P1-00")
    require(not report["threshold_selected"] and report["selected_threshold"] is None, "threshold was selected")

    frame = flatten_attempts(report)
    require(len(frame) == 248, f"expected 248 attempts, found {len(frame)}")
    require(frame["accepted"].sum() == 241 and frame["rejected"].sum() == 7, "unexpected outcome counts")
    require(np.isfinite(frame["eta_c2"]).all(), "raw eta coverage incomplete")
    require(np.isfinite(frame["rho_c2_wrms"]).all(), "scaled rho coverage incomplete")

    frame.to_csv(args.output_dir / "tables" / "attempts.csv", index=False)

    sweeps: dict[str, pd.DataFrame] = {}
    summaries: dict[str, Any] = {}
    collisions: dict[str, Any] = {}
    for view_name, select in VIEWS.items():
        view = select(frame)
        summaries[view_name] = {
            "rows": int(len(view)),
            "accepted": int(view["accepted"].sum()),
            "rejected": int(view["rejected"].sum()),
            "scores": {},
        }
        collisions[view_name] = {}
        for score_name, score in SCORES.items():
            sweep = threshold_sweep(view, score)
            sweeps[f"{view_name}_{score_name}"] = sweep
            sweep.to_csv(args.output_dir / "tables" / f"threshold_sweep_{view_name}_{score_name}.csv", index=False)
            pareto_frontier(sweep).to_csv(args.output_dir / "tables" / f"pareto_{view_name}_{score_name}.csv", index=False)
            summaries[view_name]["scores"][score_name] = {
                "outcome_stats": outcome_stats(view, score),
                "tradeoff": tradeoff_summary(sweep),
            }
            collisions[view_name][score_name] = exact_cross_outcome_collisions(view, score)

    lopo = lopo_table(frame)
    lopo.to_csv(args.output_dir / "tables" / "leave_one_problem_out.csv", index=False)
    by_problem = grouped_distribution(frame, "problem_id")
    by_problem.to_csv(args.output_dir / "tables" / "distribution_by_problem.csv", index=False)
    by_rtol = grouped_distribution(frame, "rtol")
    by_rtol.to_csv(args.output_dir / "tables" / "distribution_by_rtol.csv", index=False)
    overhead = overhead_table(report)
    overhead.to_csv(args.output_dir / "tables" / "paired_overhead.csv", index=False)
    wall_ratios = overhead["wall_ratio_read_only_over_disabled"].to_numpy(float)

    triplet = frame[(frame["problem_id"] == "quadratic") & (frame["attempt_index"] == 0)].sort_values("rtol")
    require(len(triplet) == 3, "quadratic first-attempt triplet missing")
    require(triplet["eta_c2"].nunique() == 1, "raw quadratic collision unexpectedly disappeared")
    require(triplet["rho_c2_wrms"].nunique() == 3, "scaled quadratic collision was not broken")
    require(np.all(np.diff(triplet["rho_c2_wrms"].to_numpy(float)) < 0), "scaled triplet does not increase as tolerance tightens")
    require(np.all(np.diff(triplet["total_error_norm"].to_numpy(float)) < 0), "error triplet does not increase as tolerance tightens")

    all_scaled_better = summaries["all"]["scores"]["scaled"]["tradeoff"]["minimum_total_misclassification"]["strictly_better_than_all_admit"]
    entry_scaled_better = summaries["entry"]["scores"]["scaled"]["tradeoff"]["minimum_total_misclassification"]["strictly_better_than_all_admit"]

    analysis = {
        "schema": SCHEMA,
        "status": "pass" if all_scaled_better and entry_scaled_better else "negative-preflight",
        "threshold_selected": False,
        "selected_threshold": None,
        "active_switching": False,
        "early_abort": False,
        "attempt_counts": {
            "all": int(len(frame)),
            "accepted": int(frame["accepted"].sum()),
            "rejected": int(frame["rejected"].sum()),
            "entry": int(frame["entry_window"].sum()),
        },
        "quadratic_triplet": triplet[["rtol", "eta_c2", "rho_c2_wrms", "total_error_norm", "outcome"]].to_dict(orient="records"),
        "exact_cross_outcome_collisions": collisions,
        "views": summaries,
        "scaled_preflight_kill_tests": {
            "raw_quadratic_collision_preserved_as_control": True,
            "scaled_quadratic_collision_broken": True,
            "scaled_triplet_moves_with_tolerance_direction": True,
            "all_attempt_view_strictly_better_than_all_admit": bool(all_scaled_better),
            "entry_view_strictly_better_than_all_admit": bool(entry_scaled_better),
            "pass": bool(all_scaled_better and entry_scaled_better),
        },
        "concentration": {
            "problem_attempt_counts": {str(k): int(v) for k, v in frame["problem_id"].value_counts().items()},
            "problem_reject_counts": {str(k): int(v) for k, v in frame.loc[frame["rejected"], "problem_id"].value_counts().items()},
            "rtol_attempt_counts": {str(k): int(v) for k, v in frame["rtol"].value_counts().sort_index().items()},
            "all_rejects_are_entry_window": bool(frame.loc[frame["rejected"], "entry_window"].all()),
        },
        "paired_overhead": {
            "pairs": int(len(wall_ratios)),
            "median_ratio": float(np.median(wall_ratios)),
            "mad": mad(wall_ratios),
            "observed_p95": percentile(wall_ratios, 0.95),
            "minimum": float(wall_ratios.min()),
            "maximum": float(wall_ratios.max()),
            "identities_passed": bool(report["overhead"]["all_suite_identities_passed"]),
        },
        "interpretation_ceiling": [
            "No admission threshold is selected or recommended.",
            "Attempt rows within a trajectory are serially dependent; AUC/KS/Spearman are descriptive only.",
            "The frozen corpus is dominated by oscillatory-pr attempts and all seven rejects occur in the entry window.",
            "Passing P1-00 opens a read-only event-conditioned shadow study, not active switching or an early-abort claim.",
        ],
    }
    (args.output_dir / "ANALYSIS_SUMMARY.json").write_text(json.dumps(analysis, indent=2, sort_keys=True) + "\n")

    plot_threshold_compare(sweeps, args.output_dir / "figures" / "THRESHOLD_SWEEP_ALL_RAW_VS_SCALED.png", "all")
    plot_threshold_compare(sweeps, args.output_dir / "figures" / "THRESHOLD_SWEEP_ENTRY_RAW_VS_SCALED.png", "entry")
    plot_score_error(frame, "eta_c2", args.output_dir / "figures" / "RAW_SCORE_VS_ERROR_ALL.png", "Raw early defect vs adaptive error — all attempts")
    plot_score_error(frame, "rho_c2_wrms", args.output_dir / "figures" / "SCALED_SCORE_VS_ERROR_ALL.png", "Tolerance-scaled early defect vs adaptive error — all attempts")
    plot_score_error(frame[frame["entry_window"]], "rho_c2_wrms", args.output_dir / "figures" / "SCALED_SCORE_VS_ERROR_ENTRY.png", "Tolerance-scaled early defect vs adaptive error — entry window")
    plot_quadratic_triplet(frame, args.output_dir / "figures" / "QUADRATIC_TOLERANCE_TRIPLET.png")
    plot_ecdf(frame, "rho_c2_wrms", args.output_dir / "figures" / "SCALED_SCORE_ECDF_ALL.png", "Scaled-score outcome ECDF — all attempts")
    plot_ecdf(frame[frame["entry_window"]], "rho_c2_wrms", args.output_dir / "figures" / "SCALED_SCORE_ECDF_ENTRY.png", "Scaled-score outcome ECDF — entry window")
    plot_overhead(overhead, args.output_dir / "figures" / "PAIRED_OVERHEAD.png")

    print(json.dumps(analysis, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
