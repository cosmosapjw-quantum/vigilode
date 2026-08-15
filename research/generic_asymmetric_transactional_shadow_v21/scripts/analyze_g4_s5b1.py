#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results"
PLOTS = ROOT / "plots"
INPUTS = ROOT / "inputs"

MIN_RESIDENCE = 6
ECONOMIC_MARGIN = 1.05
FLOW_MAX = 2.0
JVP_MAX = 1e-6
DRIFT_TRIGGER = 2.0
HARD_E_PREFIX_FRACTION = 0.156379  # G4-S4 selected-prefix p95 / E cold wall
HARD_R_PREFIX_FRACTION = 0.211150  # G4-S4 selected-prefix p95 / R cold wall


@dataclass(frozen=True)
class PolicyParams:
    cooldown: int
    review_interval: int


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def _extract_tar_zst(archive: Path, target: Path) -> None:
    import subprocess

    subprocess.run(
        ["bash", "-lc", f"zstd -dc '{archive}' | tar -xf - -C '{target}'"],
        check=True,
    )


def _hard_campaign_rows(path: Path, campaign: int) -> pd.DataFrame:
    payload = _load_json(path)
    rows: list[dict[str, Any]] = []
    for row in payload["rows"]:
        h = float(row["h"])
        r_unit = float(row["rodas_predicted_cost_per_unit_time"])
        e_unit = float(row["exponential_predicted_cost_per_unit_time"])
        e_wall = float(row["exponential_wall_seconds"])
        r_wall = float(row["rodas_wall_seconds"])
        rows.append(
            {
                "campaign": campaign,
                "corpus": "hard",
                "trajectory_id": row["trajectory_id"],
                "family": row["trajectory_id"],
                "dimension": int(row.get("dimension", 0) or 0),
                "split": row["split"],
                "rtol": float(row["rtol"]),
                "step_index": int(row["step_index"]),
                "h": h,
                "r_wall": r_wall,
                "e_wall": e_wall,
                # P0 correction: methods predict different next h. Compare the
                # expected cost to cover the same physical interval h.
                "r_unit": r_unit,
                "e_unit": e_unit,
                "r_cost": r_unit * h,
                "e_cost": e_unit * h,
                "e_admissible": bool(row["exponential_locally_admissible"]),
                "flow_defect": float(row["last_sampled_flow_defect"]),
                "jvp_quality": float(row["last_sampled_jvp_quality"]),
                "e_prefix_cost": HARD_E_PREFIX_FRACTION * e_wall,
                "r_prefix_cost": HARD_R_PREFIX_FRACTION * r_wall,
                "transition_level": math.nan,
            }
        )
    return pd.DataFrame(rows)


def _atlas_campaign_rows(path: Path, campaign: int) -> pd.DataFrame:
    payload = _load_json(path)
    rows: list[dict[str, Any]] = []
    for row in payload["rows"]:
        h = float(row["h"])
        e_wall_raw = row.get("exponential_wall_seconds")
        e_wall = float(e_wall_raw) if e_wall_raw is not None else math.inf
        r_wall = float(row["rodas_wall_seconds"])
        dimension = int(row["dimension"])
        rows.append(
            {
                "campaign": campaign,
                "corpus": "atlas",
                "trajectory_id": row["trajectory_id"],
                "family": row["family"],
                "dimension": dimension,
                "split": "calibration" if dimension == 128 else "holdout",
                "rtol": float(row["rtol"]),
                "step_index": int(row["step_index"]),
                "h": h,
                "r_wall": r_wall,
                "e_wall": e_wall,
                "r_unit": r_wall / h,
                "e_unit": e_wall / h if math.isfinite(e_wall) else math.inf,
                "r_cost": r_wall,
                "e_cost": e_wall,
                "e_admissible": bool(row["exponential_locally_admissible"]),
                # G4-S5B0 did not measure G4-S1 flow/JVP features. Do not
                # invent them; atlas E entry is checked by its own local error.
                "flow_defect": math.nan,
                "jvp_quality": math.nan,
                "e_prefix_cost": float(row["exponential_prefix_wall_seconds"]),
                "r_prefix_cost": HARD_R_PREFIX_FRACTION * r_wall,
                "transition_level": float(row["transition_level"]),
            }
        )
    return pd.DataFrame(rows)


def build_campaigns(hard_root: Path, atlas_root: Path) -> dict[int, pd.DataFrame]:
    campaigns: dict[int, pd.DataFrame] = {}
    for index in range(1, 8):
        hard = _hard_campaign_rows(hard_root / f"G4S1_T1_{index}.json", index)
        atlas = _atlas_campaign_rows(
            atlas_root / f"G4S5B0_CANONICAL_{index}.json", index
        )
        combined = pd.concat([hard, atlas], ignore_index=True)
        combined["trajectory_key"] = (
            combined["corpus"]
            + "|"
            + combined["trajectory_id"]
            + "|"
            + combined["rtol"].map(lambda value: f"{value:.16g}")
        )
        campaigns[index] = combined.sort_values(
            ["trajectory_key", "step_index"]
        ).reset_index(drop=True)
    return campaigns


def median_campaign(campaigns: dict[int, pd.DataFrame]) -> pd.DataFrame:
    frame = pd.concat(campaigns.values(), ignore_index=True)
    key = [
        "corpus",
        "trajectory_id",
        "family",
        "dimension",
        "split",
        "rtol",
        "step_index",
        "h",
        "trajectory_key",
    ]
    bool_fields = ["e_admissible"]
    numeric = [
        "r_wall",
        "e_wall",
        "r_unit",
        "e_unit",
        "r_cost",
        "e_cost",
        "flow_defect",
        "jvp_quality",
        "e_prefix_cost",
        "r_prefix_cost",
        "transition_level",
    ]
    rows: list[dict[str, Any]] = []
    for keys, group in frame.groupby(key, sort=True, dropna=False):
        row = dict(zip(key, keys))
        for field in numeric:
            values = group[field].astype(float).to_numpy()
            finite = values[np.isfinite(values)]
            row[field] = float(np.median(finite)) if len(finite) else math.nan
        for field in bool_fields:
            row[field] = bool(group[field].astype(bool).iloc[0])
        row["campaign_count"] = int(len(group))
        rows.append(row)
    return pd.DataFrame(rows).sort_values(["trajectory_key", "step_index"])


def safe_precheck(row: pd.Series) -> bool:
    if row["corpus"] == "atlas":
        return True
    flow = float(row["flow_defect"])
    jvp = float(row["jvp_quality"])
    return (
        math.isfinite(flow)
        and flow <= FLOW_MAX
        and math.isfinite(jvp)
        and jvp <= JVP_MAX
    )


def oracle_cost(row: pd.Series) -> float:
    e = float(row["e_cost"]) if bool(row["e_admissible"]) else math.inf
    return min(float(row["r_cost"]), e)


def _r_default(group: pd.DataFrame, params: PolicyParams) -> list[dict[str, Any]]:
    state = "R"
    residence = 0
    cooldown = 0
    since_probe = 10**9
    r_reference: float | None = None
    out: list[dict[str, Any]] = []
    for _, row in group.iterrows():
        r_cost = float(row["r_cost"])
        e_cost = float(row["e_cost"])
        r_unit = float(row["r_unit"])
        e_unit = float(row["e_unit"])
        e_adm = bool(row["e_admissible"])
        prefix = float(row["e_prefix_cost"])
        trial_overhead = 0.0
        fallback = False
        switch = False
        reason = ""

        if r_reference is None:
            r_reference = r_unit
        drift = r_unit >= DRIFT_TRIGGER * max(r_reference, 1e-300)

        if state == "R":
            can_probe = (
                residence >= MIN_RESIDENCE
                and cooldown <= 0
                and safe_precheck(row)
                and (since_probe >= params.review_interval or drift)
            )
            if can_probe:
                since_probe = 0
                if e_adm and e_unit * ECONOMIC_MARGIN < r_unit:
                    # Prefix is reused by the selected E step.
                    selected_cost = e_cost
                    state = "E"
                    residence = 0
                    switch = True
                    reason = "e-prefix-economic-entry"
                else:
                    selected_cost = r_cost + prefix
                    trial_overhead += prefix
                    cooldown = params.cooldown
                    reason = "discarded-e-prefix"
            else:
                selected_cost = r_cost
                reason = "remain-r"
        else:
            # E remains active until its own admissibility/safety gate fails.
            if not e_adm or not safe_precheck(row):
                # A full E step is needed to obtain the local-error failure.
                selected_cost = e_cost + r_cost
                trial_overhead += e_cost
                fallback = True
                state = "R"
                residence = 0
                cooldown = params.cooldown
                switch = True
                reason = "e-failure-to-r"
            else:
                selected_cost = e_cost
                reason = "remain-e"

        out.append(
            {
                **row.to_dict(),
                "policy": "r-default",
                "selected_method": state,
                "selected_cost": selected_cost,
                "oracle_cost": oracle_cost(row),
                "static_r_cost": r_cost,
                "trial_overhead": trial_overhead,
                "fallback": fallback,
                "switch": switch,
                "reason": reason,
                "unsafe": False,
            }
        )
        residence += 1
        since_probe += 1
        cooldown = max(0, cooldown - 1)
        r_reference = min(r_reference, r_unit)
    return out


def _e_first(group: pd.DataFrame, params: PolicyParams) -> list[dict[str, Any]]:
    first = group.iloc[0]
    state = "E" if safe_precheck(first) else "R"
    residence = 0
    cooldown = 0
    since_review = 10**9
    out: list[dict[str, Any]] = []
    for _, row in group.iterrows():
        r_cost = float(row["r_cost"])
        e_cost = float(row["e_cost"])
        r_unit = float(row["r_unit"])
        e_unit = float(row["e_unit"])
        e_adm = bool(row["e_admissible"])
        e_prefix = float(row["e_prefix_cost"])
        r_prefix = float(row["r_prefix_cost"])
        trial_overhead = 0.0
        fallback = False
        switch = False
        reason = ""

        if state == "E":
            if not e_adm or not safe_precheck(row):
                selected_cost = e_cost + r_cost
                trial_overhead += e_cost
                fallback = True
                state = "R"
                residence = 0
                cooldown = params.cooldown
                since_review = 0
                switch = True
                reason = "e-failure-to-r"
            elif residence >= MIN_RESIDENCE and since_review >= params.review_interval:
                since_review = 0
                if r_unit * ECONOMIC_MARGIN < e_unit:
                    # The selected R prefix is reused by the R solve.
                    selected_cost = r_cost
                    state = "R"
                    residence = 0
                    switch = True
                    reason = "r-prefix-economic-entry"
                else:
                    selected_cost = e_cost + r_prefix
                    trial_overhead += r_prefix
                    reason = "discarded-r-prefix"
            else:
                selected_cost = e_cost
                reason = "remain-e"
        else:
            can_probe = (
                residence >= MIN_RESIDENCE
                and cooldown <= 0
                and safe_precheck(row)
                and since_review >= params.review_interval
            )
            if can_probe:
                since_review = 0
                if e_adm and e_unit * ECONOMIC_MARGIN < r_unit:
                    selected_cost = e_cost
                    state = "E"
                    residence = 0
                    switch = True
                    reason = "e-prefix-economic-entry"
                else:
                    selected_cost = r_cost + e_prefix
                    trial_overhead += e_prefix
                    cooldown = params.cooldown
                    reason = "discarded-e-prefix"
            else:
                selected_cost = r_cost
                reason = "remain-r"

        out.append(
            {
                **row.to_dict(),
                "policy": "e-first",
                "selected_method": state,
                "selected_cost": selected_cost,
                "oracle_cost": oracle_cost(row),
                "static_r_cost": r_cost,
                "trial_overhead": trial_overhead,
                "fallback": fallback,
                "switch": switch,
                "reason": reason,
                "unsafe": False,
            }
        )
        residence += 1
        since_review += 1
        cooldown = max(0, cooldown - 1)
    return out


def _safe_e_diagnostic(group: pd.DataFrame) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for _, row in group.iterrows():
        r_cost = float(row["r_cost"])
        e_cost = float(row["e_cost"])
        e_adm = bool(row["e_admissible"])
        trial_overhead = 0.0
        fallback = False
        if safe_precheck(row):
            if e_adm:
                selected_cost = e_cost
                method = "E"
                reason = "safety-admitted-e"
            else:
                # Full E trial supplies the local error certificate.
                selected_cost = e_cost + r_cost
                trial_overhead = e_cost
                fallback = True
                method = "R"
                reason = "e-inadmissible-fallback"
        else:
            selected_cost = r_cost
            method = "R"
            reason = "safety-routed-r"
        out.append(
            {
                **row.to_dict(),
                "policy": "safe-e-diagnostic",
                "selected_method": method,
                "selected_cost": selected_cost,
                "oracle_cost": oracle_cost(row),
                "static_r_cost": r_cost,
                "trial_overhead": trial_overhead,
                "fallback": fallback,
                "switch": False,
                "reason": reason,
                "unsafe": False,
            }
        )
    return out


def simulate(frame: pd.DataFrame, policy: str, params: PolicyParams | None) -> pd.DataFrame:
    out: list[dict[str, Any]] = []
    for _, group in frame.groupby("trajectory_key", sort=False):
        group = group.sort_values("step_index")
        if policy == "r-default":
            assert params is not None
            out.extend(_r_default(group, params))
        elif policy == "e-first":
            assert params is not None
            out.extend(_e_first(group, params))
        elif policy == "safe-e-diagnostic":
            out.extend(_safe_e_diagnostic(group))
        else:
            raise ValueError(policy)
    return pd.DataFrame(out)


def trajectory_summary(rows: pd.DataFrame) -> pd.DataFrame:
    grouped = rows.groupby(
        ["corpus", "trajectory_id", "family", "dimension", "split", "rtol"],
        sort=True,
    )
    summary = grouped.agg(
        selected_cost=("selected_cost", "sum"),
        oracle_cost=("oracle_cost", "sum"),
        static_r_cost=("static_r_cost", "sum"),
        trial_overhead=("trial_overhead", "sum"),
        accepted_steps=("step_index", "count"),
        fallbacks=("fallback", "sum"),
        switches=("switch", "sum"),
        unsafe=("unsafe", "sum"),
    ).reset_index()
    summary["regret"] = summary["selected_cost"] / summary["oracle_cost"]
    summary["speedup_vs_static_r"] = (
        summary["static_r_cost"] / summary["selected_cost"]
    )
    successful_cost = summary["selected_cost"] - summary["trial_overhead"]
    summary["trial_overhead_ratio"] = summary["trial_overhead"] / np.maximum(
        successful_cost, 1e-300
    )
    return summary


def metrics(summary: pd.DataFrame) -> dict[str, Any]:
    return {
        "trajectories": int(len(summary)),
        "unsafe": int(summary["unsafe"].sum()),
        "median_regret": float(summary["regret"].median()),
        "p95_regret": float(summary["regret"].quantile(0.95)),
        "median_trial_overhead": float(summary["trial_overhead_ratio"].median()),
        "p95_trial_overhead": float(summary["trial_overhead_ratio"].quantile(0.95)),
        "aggregate_speedup_vs_static_r": float(
            summary["static_r_cost"].sum() / summary["selected_cost"].sum()
        ),
        "median_speedup_vs_static_r": float(summary["speedup_vs_static_r"].median()),
        "trajectories_beating_static_r": int(
            (summary["speedup_vs_static_r"] > 1.0).sum()
        ),
        "fallbacks": int(summary["fallbacks"].sum()),
        "switches": int(summary["switches"].sum()),
    }


def choose_params(frame: pd.DataFrame, policy: str) -> tuple[PolicyParams, pd.DataFrame]:
    records: list[dict[str, Any]] = []
    for cooldown in [3, 6, 12]:
        for interval in [6, 8, 12]:
            params = PolicyParams(cooldown=cooldown, review_interval=interval)
            rows = simulate(frame, policy, params)
            summary = trajectory_summary(rows)
            calibration = summary[summary["split"] == "calibration"]
            record = {
                "policy": policy,
                "cooldown": cooldown,
                "review_interval": interval,
                **metrics(calibration),
            }
            records.append(record)
    grid = pd.DataFrame(records)
    # Calibration choice is gate-aware rather than headline-median-aware.
    # A policy that shaves a small amount of median regret but creates large
    # trial-overhead violations must not win calibration.
    grid["gate_violation_score"] = (
        np.maximum(0.0, grid["median_regret"] / 1.15 - 1.0)
        + np.maximum(0.0, grid["p95_regret"] / 1.50 - 1.0)
        + np.maximum(0.0, grid["median_trial_overhead"] / 0.03 - 1.0)
        + np.maximum(0.0, grid["p95_trial_overhead"] / 0.05 - 1.0)
    )
    grid = grid.sort_values(
        [
            "unsafe",
            "gate_violation_score",
            "p95_regret",
            "median_regret",
            "p95_trial_overhead",
            "median_trial_overhead",
            "review_interval",
            "cooldown",
        ],
        ascending=[True, True, True, True, True, True, False, False],
    )
    best = grid.iloc[0]
    return (
        PolicyParams(
            cooldown=int(best["cooldown"]),
            review_interval=int(best["review_interval"]),
        ),
        grid,
    )


def _scientific_hash(frame: pd.DataFrame) -> str:
    cols = [
        "corpus",
        "trajectory_id",
        "split",
        "rtol",
        "step_index",
        "h",
        "e_admissible",
        "flow_defect",
        "jvp_quality",
    ]
    payload = frame[cols].sort_values(cols[:5]).to_json(
        orient="records", double_precision=15
    )
    return _sha256(payload.encode())


def plot_results(
    trajectory: pd.DataFrame,
    campaign_summary: pd.DataFrame,
    metric_correction: pd.DataFrame,
) -> None:
    PLOTS.mkdir(parents=True, exist_ok=True)

    hold = trajectory[trajectory["split"] == "holdout"].copy()
    labels = (
        hold["corpus"]
        + ":"
        + hold["trajectory_id"]
        + "@"
        + hold["rtol"].map(lambda value: f"{value:g}")
    )
    policies = ["r-default", "e-first", "safe-e-diagnostic"]
    x = np.arange(len(labels.unique()))
    width = 0.25
    plt.figure(figsize=(12, 5.2))
    for offset, policy in enumerate(policies):
        local = hold[hold["policy"] == policy].copy()
        local["label"] = (
            local["corpus"]
            + ":"
            + local["trajectory_id"]
            + "@"
            + local["rtol"].map(lambda value: f"{value:g}")
        )
        local = local.set_index("label").reindex(labels.unique())
        plt.bar(
            x + (offset - 1) * width,
            local["regret"],
            width,
            label=policy,
        )
    plt.axhline(1.15, linestyle="--", label="median gate 1.15")
    plt.axhline(1.5, linestyle=":", label="P95 gate 1.50")
    plt.xticks(x, labels.unique(), rotation=65, ha="right", fontsize=7)
    plt.ylabel("Trajectory regret vs feasible oracle")
    plt.title("G4-S5B1 holdout regret by asymmetric policy")
    plt.legend(fontsize=8)
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B1_HOLDOUT_REGRET.png", dpi=180)
    plt.close()

    plt.figure(figsize=(10, 4.8))
    policy_metrics = []
    for policy in policies:
        local = hold[hold["policy"] == policy]
        policy_metrics.append(
            (
                policy,
                float(local["trial_overhead_ratio"].median()),
                float(local["trial_overhead_ratio"].quantile(0.95)),
            )
        )
    xx = np.arange(len(policy_metrics))
    plt.bar(xx - 0.18, [item[1] for item in policy_metrics], 0.36, label="median")
    plt.bar(xx + 0.18, [item[2] for item in policy_metrics], 0.36, label="P95")
    plt.axhline(0.03, linestyle="--", label="median gate 3%")
    plt.axhline(0.05, linestyle=":", label="P95 gate 5%")
    plt.xticks(xx, [item[0] for item in policy_metrics])
    plt.ylabel("Trial / successful-method cost")
    plt.title("G4-S5B1 holdout trial overhead")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B1_TRIAL_OVERHEAD.png", dpi=180)
    plt.close()

    diagnostic = trajectory[trajectory["policy"] == "safe-e-diagnostic"].copy()
    diagnostic["label"] = (
        diagnostic["split"]
        + ":"
        + diagnostic["trajectory_id"]
        + "@"
        + diagnostic["rtol"].map(lambda value: f"{value:g}")
    )
    diagnostic = diagnostic.sort_values(["split", "regret"], ascending=[True, False])
    plt.figure(figsize=(11, 5.0))
    plt.bar(np.arange(len(diagnostic)), diagnostic["regret"])
    plt.axhline(1.15, linestyle="--", label="median gate 1.15")
    plt.axhline(1.5, linestyle=":", label="P95 gate 1.50")
    plt.xticks(
        np.arange(len(diagnostic)), diagnostic["label"], rotation=65, ha="right", fontsize=7
    )
    plt.ylabel("Trajectory regret")
    plt.title("Safety-only E-first diagnostic: economics remain unresolved")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B1_SAFE_E_DIAGNOSTIC.png", dpi=180)
    plt.close()

    plt.figure(figsize=(8.2, 4.8))
    for policy in policies:
        local = campaign_summary[(campaign_summary["policy"] == policy) & (campaign_summary["split"] == "holdout")]
        plt.plot(
            local["campaign"],
            local["p95_regret"],
            marker="o",
            label=policy,
        )
    plt.axhline(1.5, linestyle="--", label="P95 gate 1.50")
    plt.xlabel("Canonical campaign")
    plt.ylabel("Holdout P95 trajectory regret")
    plt.title("G4-S5B1 campaign sensitivity")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B1_CAMPAIGN_SENSITIVITY.png", dpi=180)
    plt.close()

    hard = metric_correction[metric_correction["corpus"] == "hard"].copy()
    hard["raw_wall_winner"] = np.where(
        hard["e_wall"] < hard["r_wall"], "E-K", "R-JF"
    )
    hard["unit_time_winner"] = np.where(
        hard["e_admissible"] & (hard["e_unit"] < hard["r_unit"]), "E-K", "R-JF"
    )
    cross = pd.crosstab(hard["raw_wall_winner"], hard["unit_time_winner"])
    values = cross.reindex(index=["E-K", "R-JF"], columns=["E-K", "R-JF"], fill_value=0)
    plt.figure(figsize=(5.5, 4.6))
    image = plt.imshow(values.to_numpy())
    for i in range(2):
        for j in range(2):
            plt.text(j, i, int(values.iloc[i, j]), ha="center", va="center")
    plt.xticks([0, 1], ["E-K", "R-JF"])
    plt.yticks([0, 1], ["E-K", "R-JF"])
    plt.xlabel("Winner by cost per unit physical time")
    plt.ylabel("Winner by raw current-step wall")
    plt.title("P0 metric correction on the hard corpus")
    plt.colorbar(image, label="accepted-step rows")
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B1_METRIC_CORRECTION.png", dpi=180)
    plt.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--hard-raw", type=Path, required=True)
    parser.add_argument("--atlas-raw", type=Path, required=True)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as temporary:
        temporary_path = Path(temporary)
        hard_dir = temporary_path / "hard"
        atlas_dir = temporary_path / "atlas"
        hard_dir.mkdir()
        atlas_dir.mkdir()
        _extract_tar_zst(args.hard_raw, hard_dir)
        _extract_tar_zst(args.atlas_raw, atlas_dir)
        hard_campaign_dir = hard_dir / "campaigns_v2"
        atlas_campaign_dir = atlas_dir / "results" / "campaigns"
        campaigns = build_campaigns(hard_campaign_dir, atlas_campaign_dir)

        median = median_campaign(campaigns)
        median.to_csv(RESULTS / "G4_S5B1_COMBINED_STEPS.csv", index=False)
        scientific_hashes = {
            index: _scientific_hash(frame) for index, frame in campaigns.items()
        }

        selected_params: dict[str, PolicyParams] = {}
        grids: list[pd.DataFrame] = []
        for policy in ["r-default", "e-first"]:
            params, grid = choose_params(median, policy)
            selected_params[policy] = params
            grids.append(grid)
        parameter_grid = pd.concat(grids, ignore_index=True)
        parameter_grid.to_csv(RESULTS / "G4_S5B1_PARAMETER_GRID.csv", index=False)

        all_rows: list[pd.DataFrame] = []
        all_trajectory: list[pd.DataFrame] = []
        for policy in ["r-default", "e-first"]:
            rows = simulate(median, policy, selected_params[policy])
            summary = trajectory_summary(rows)
            summary["policy"] = policy
            all_rows.append(rows)
            all_trajectory.append(summary)
        diagnostic_rows = simulate(median, "safe-e-diagnostic", None)
        diagnostic_summary = trajectory_summary(diagnostic_rows)
        diagnostic_summary["policy"] = "safe-e-diagnostic"
        all_rows.append(diagnostic_rows)
        all_trajectory.append(diagnostic_summary)

        policy_rows = pd.concat(all_rows, ignore_index=True)
        trajectory = pd.concat(all_trajectory, ignore_index=True)
        policy_rows.to_csv(RESULTS / "G4_S5B1_POLICY_ROWS.csv", index=False)
        trajectory.to_csv(RESULTS / "G4_S5B1_TRAJECTORY_SUMMARY.csv", index=False)

        campaign_records: list[dict[str, Any]] = []
        campaign_trajectory: list[pd.DataFrame] = []
        for campaign, frame in campaigns.items():
            for policy in ["r-default", "e-first"]:
                rows = simulate(frame, policy, selected_params[policy])
                summary = trajectory_summary(rows)
                summary["campaign"] = campaign
                summary["policy"] = policy
                campaign_trajectory.append(summary)
                for split in ["calibration", "holdout"]:
                    local = summary[summary["split"] == split]
                    campaign_records.append(
                        {
                            "campaign": campaign,
                            "policy": policy,
                            "split": split,
                            **metrics(local),
                        }
                    )
            rows = simulate(frame, "safe-e-diagnostic", None)
            summary = trajectory_summary(rows)
            summary["campaign"] = campaign
            summary["policy"] = "safe-e-diagnostic"
            campaign_trajectory.append(summary)
            for split in ["calibration", "holdout"]:
                local = summary[summary["split"] == split]
                campaign_records.append(
                    {
                        "campaign": campaign,
                        "policy": "safe-e-diagnostic",
                        "split": split,
                        **metrics(local),
                    }
                )
        campaign_summary = pd.DataFrame(campaign_records)
        campaign_summary.to_csv(
            RESULTS / "G4_S5B1_CAMPAIGN_SUMMARY.csv", index=False
        )
        pd.concat(campaign_trajectory, ignore_index=True).to_csv(
            RESULTS / "G4_S5B1_CAMPAIGN_TRAJECTORIES.csv", index=False
        )

        result_metrics: dict[str, Any] = {}
        for policy in ["r-default", "e-first", "safe-e-diagnostic"]:
            result_metrics[policy] = {}
            local = trajectory[trajectory["policy"] == policy]
            for split in ["calibration", "holdout"]:
                result_metrics[policy][split] = metrics(
                    local[local["split"] == split]
                )

        # Gate disposition follows the sealed contract. The diagnostic is
        # informative but is not an activation candidate.
        contract_gates: dict[str, Any] = {}
        for policy in ["r-default", "e-first"]:
            hold = result_metrics[policy]["holdout"]
            contract_gates[policy] = {
                "unsafe_zero": hold["unsafe"] == 0,
                "median_regret_le_1_15": hold["median_regret"] <= 1.15,
                "p95_regret_le_1_5": hold["p95_regret"] <= 1.5,
                "median_trial_overhead_lt_3pct": hold[
                    "median_trial_overhead"
                ]
                < 0.03,
                "p95_trial_overhead_lt_5pct": hold["p95_trial_overhead"] < 0.05,
                "activation_gate_pass": False,
            }
            contract_gates[policy]["activation_gate_pass"] = all(
                value
                for key, value in contract_gates[policy].items()
                if key != "activation_gate_pass"
            )

        safe_campaign = campaign_summary[
            campaign_summary["policy"] == "safe-e-diagnostic"
        ]
        hard_metric = median[median["corpus"] == "hard"].copy()
        raw_e = hard_metric["e_wall"] < hard_metric["r_wall"]
        unit_e = hard_metric["e_admissible"] & (hard_metric["e_unit"] < hard_metric["r_unit"])
        metric_correction_stats = {
            "hard_rows": int(len(hard_metric)),
            "winner_flips": int((raw_e != unit_e).sum()),
            "winner_flip_fraction": float((raw_e != unit_e).mean()),
            "raw_e_to_unit_r": int((raw_e & ~unit_e).sum()),
            "raw_r_to_unit_e": int((~raw_e & unit_e).sum()),
        }

        campaign_robustness: dict[str, Any] = {}
        for policy in ["r-default", "e-first", "safe-e-diagnostic"]:
            campaign_robustness[policy] = {}
            for split in ["calibration", "holdout"]:
                local = campaign_summary[(campaign_summary["policy"] == policy) & (campaign_summary["split"] == split)]
                campaign_robustness[policy][split] = {
                    "campaigns": int(len(local)),
                    "median_regret_median": float(local["median_regret"].median()),
                    "p95_regret_median": float(local["p95_regret"].median()),
                    "p95_regret_min": float(local["p95_regret"].min()),
                    "p95_regret_max": float(local["p95_regret"].max()),
                    "p95_overhead_median": float(local["p95_trial_overhead"].median()),
                    "aggregate_speedup_median": float(local["aggregate_speedup_vs_static_r"].median()),
                }

        analysis = {
            "schema": "g4-s5b1-asymmetric-transactional-shadow-analysis-v1",
            "status": "complete-read-only-hold",
            "active_switching": False,
            "candidate_source_modified": False,
            "metric_correction": {
                "p0_found": True,
                "incorrect_metric": "raw current-step wall",
                "correct_metric": "predicted cost per unit physical time, integrated over the same RODAS interval",
                "reason": "E-K and R-JF propose different next step sizes on the hard corpus",
                "statistics": metric_correction_stats,
            },
            "fixed_contract": {
                "minimum_residence": MIN_RESIDENCE,
                "economic_margin": ECONOMIC_MARGIN,
                "flow_max": FLOW_MAX,
                "jvp_max": JVP_MAX,
                "failed_trial_work_retained": True,
                "routine_dual_prefix": False,
            },
            "selected_parameters": {
                key: {
                    "cooldown": value.cooldown,
                    "review_interval": value.review_interval,
                }
                for key, value in selected_params.items()
            },
            "results": result_metrics,
            "contract_gates": contract_gates,
            "campaign_robustness": campaign_robustness,
            "scientific_hashes": scientific_hashes,
            "closeout": {
                "r_default": "HOLD",
                "e_first": "HOLD",
                "safe_e_diagnostic": "PROMOTE_DIAGNOSTIC_ONLY",
                "active_switching": "HOLD",
                "load_bearing_blocker": "startup economic admission inside the E-admissible regime",
            },
        }
        (RESULTS / "G4_S5B1_ANALYSIS.json").write_text(
            json.dumps(analysis, indent=2, sort_keys=True) + "\n"
        )

        plot_results(trajectory, campaign_summary, median)

        print(json.dumps(analysis, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
