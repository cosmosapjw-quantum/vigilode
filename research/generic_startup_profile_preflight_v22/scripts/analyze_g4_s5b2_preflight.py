#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
RESULTS = ROOT / "results"
PLOTS = ROOT / "plots"
INPUTS = ROOT / "inputs"

ECONOMIC_MARGIN = 1.15
COOLDOWNS = [1, 2, 3, 4, 6, 8, 12]


def _load_parent_module():
    path = (
        REPO
        / "research/generic_asymmetric_transactional_shadow_v21/scripts/analyze_g4_s5b1.py"
    )
    spec = importlib.util.spec_from_file_location("g4_s5b1_parent", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _safe(parent, row: pd.Series) -> bool:
    return bool(parent.safe_precheck(row))


def _oracle_cost(row: pd.Series) -> float:
    e = float(row["e_cost"]) if bool(row["e_admissible"]) else math.inf
    return min(float(row["r_cost"]), e)


def _profile_cost(parent, group: pd.DataFrame, profile: str, *, dual_prefix: bool) -> tuple[float, float, int]:
    first = group.iloc[0]
    if profile == "R":
        overhead = float(first["e_prefix_cost"])
        return float(group["r_cost"].sum()) + overhead, overhead, 0
    if profile != "E":
        raise ValueError(profile)
    # Selected E prefix is reused. A dual-prefix policy discards the R prefix;
    # a one-sided policy does not construct it.
    overhead = float(first["r_prefix_cost"]) if dual_prefix else 0.0
    total = overhead
    state = "E"
    fallbacks = 0
    for _, row in group.iterrows():
        r_cost = float(row["r_cost"])
        e_cost = float(row["e_cost"])
        if state == "E" and bool(row["e_admissible"]) and _safe(parent, row):
            total += e_cost
        elif state == "E":
            total += e_cost + r_cost
            state = "R"
            fallbacks += 1
        else:
            total += r_cost
    return total, overhead, fallbacks


def _simulate_startup_first_step(parent, frame: pd.DataFrame) -> pd.DataFrame:
    rows: list[dict[str, Any]] = []
    for key, group in frame.groupby("trajectory_key", sort=True):
        group = group.sort_values("step_index")
        first = group.iloc[0]
        choose_e = (
            _safe(parent, first)
            and bool(first["e_admissible"])
            and float(first["e_unit"]) * ECONOMIC_MARGIN < float(first["r_unit"])
        )
        profile = "E" if choose_e else "R"
        total, overhead, fallbacks = _profile_cost(parent, group, profile, dual_prefix=True)
        oracle = float(sum(_oracle_cost(row) for _, row in group.iterrows()))
        static = float(group["r_cost"].sum())
        rows.append(
            {
                "trajectory_key": key,
                "split": first["split"],
                "corpus": first["corpus"],
                "trajectory_id": first["trajectory_id"],
                "rtol": float(first["rtol"]),
                "steps": int(len(group)),
                "profile": profile,
                "first_unit_cost_ratio_e_over_r": float(first["e_unit"]) / max(float(first["r_unit"]), 1e-300),
                "selected_cost": total,
                "oracle_cost": oracle,
                "static_r_cost": static,
                "regret": total / oracle,
                "speedup_vs_static_r": static / total,
                "startup_overhead": overhead,
                "startup_overhead_ratio": overhead / max(total - overhead, 1e-300),
                "fallbacks": fallbacks,
            }
        )
    return pd.DataFrame(rows)


def _simulate_profile_oracle(parent, frame: pd.DataFrame, *, dual_prefix: bool) -> pd.DataFrame:
    """Best possible whole-trajectory startup profile under the fixed-profile contract.

    This is stronger than any realizable prefix classifier: it chooses the cheaper
    profile using the complete future trajectory. If this fails, no startup-only
    classifier can pass the same gates under the same state machine.
    """
    rows: list[dict[str, Any]] = []
    for key, group in frame.groupby("trajectory_key", sort=True):
        group = group.sort_values("step_index")
        first = group.iloc[0]
        r_total, r_overhead, _ = _profile_cost(parent, group, "R", dual_prefix=dual_prefix)
        e_total, e_overhead, e_fallbacks = _profile_cost(parent, group, "E", dual_prefix=dual_prefix)
        if e_total < r_total:
            profile = "E"
            total, overhead, fallbacks = e_total, e_overhead, e_fallbacks
        else:
            profile = "R"
            total, overhead, fallbacks = r_total, r_overhead, 0
        oracle = float(sum(_oracle_cost(row) for _, row in group.iterrows()))
        static = float(group["r_cost"].sum())
        rows.append(
            {
                "trajectory_key": key,
                "split": first["split"],
                "corpus": first["corpus"],
                "trajectory_id": first["trajectory_id"],
                "rtol": float(first["rtol"]),
                "steps": int(len(group)),
                "profile": profile,
                "selected_cost": total,
                "oracle_cost": oracle,
                "static_r_cost": static,
                "regret": total / oracle,
                "speedup_vs_static_r": static / total,
                "startup_overhead": overhead,
                "startup_overhead_ratio": overhead / max(total - overhead, 1e-300),
                "fallbacks": fallbacks,
                "r_profile_cost": r_total,
                "e_profile_cost": e_total,
            }
        )
    return pd.DataFrame(rows)


def _simulate_reentry_upper_bound(parent, frame: pd.DataFrame, cooldown: int) -> pd.DataFrame:
    """Optimistic one-sided startup + safety-triggered E re-entry upper bound.

    The initial profile is chosen with full future knowledge. Only an E prefix is
    charged when R is selected; an E-selected prefix is reused. E failure triggers
    R and a bounded cooldown, then E is retried. This is not deployable; it locates
    the remaining architecture blocker after startup-only is killed.
    """
    static_profiles = _simulate_profile_oracle(parent, frame, dual_prefix=False).set_index("trajectory_key")
    rows: list[dict[str, Any]] = []
    for key, group in frame.groupby("trajectory_key", sort=True):
        group = group.sort_values("step_index")
        first = group.iloc[0]
        profile = str(static_profiles.loc[key, "profile"])
        state = profile
        cooldown_left = 0
        total = 0.0
        overhead = 0.0
        fallbacks = 0
        reentries = 0
        if profile == "R":
            overhead = float(first["e_prefix_cost"])
            total += overhead
        for _, row in group.iterrows():
            r_cost = float(row["r_cost"])
            e_cost = float(row["e_cost"])
            e_ok = bool(row["e_admissible"]) and _safe(parent, row)
            if profile == "R":
                total += r_cost
                continue
            if state == "E":
                if e_ok:
                    total += e_cost
                else:
                    total += e_cost + r_cost
                    state = "R"
                    cooldown_left = cooldown
                    fallbacks += 1
            elif cooldown_left > 0:
                total += r_cost
                cooldown_left -= 1
            elif e_ok:
                total += e_cost
                state = "E"
                reentries += 1
            else:
                total += e_cost + r_cost
                cooldown_left = cooldown
                fallbacks += 1
        oracle = float(sum(_oracle_cost(row) for _, row in group.iterrows()))
        static = float(group["r_cost"].sum())
        rows.append(
            {
                "trajectory_key": key,
                "split": first["split"],
                "corpus": first["corpus"],
                "trajectory_id": first["trajectory_id"],
                "rtol": float(first["rtol"]),
                "steps": int(len(group)),
                "startup_profile": profile,
                "cooldown": cooldown,
                "selected_cost": total,
                "oracle_cost": oracle,
                "static_r_cost": static,
                "regret": total / oracle,
                "speedup_vs_static_r": static / total,
                "startup_overhead": overhead,
                "startup_overhead_ratio": overhead / max(total - overhead, 1e-300),
                "fallbacks": fallbacks,
                "reentries": reentries,
            }
        )
    return pd.DataFrame(rows)


def _metrics(frame: pd.DataFrame) -> dict[str, Any]:
    return {
        "trajectories": int(len(frame)),
        "median_regret": float(frame["regret"].median()),
        "p95_regret": float(frame["regret"].quantile(0.95)),
        "maximum_regret": float(frame["regret"].max()),
        "median_startup_overhead": float(frame["startup_overhead_ratio"].median()),
        "p95_startup_overhead": float(frame["startup_overhead_ratio"].quantile(0.95)),
        "aggregate_speedup_vs_static_r": float(frame["static_r_cost"].sum() / frame["selected_cost"].sum()),
        "profiles": frame.get("profile", frame.get("startup_profile", pd.Series(dtype=str))).value_counts().to_dict(),
    }


def _analyze(parent, campaigns: dict[int, pd.DataFrame]) -> dict[str, Any]:
    median = parent.median_campaign(campaigns)
    first = _simulate_startup_first_step(parent, median)
    profile_dual = _simulate_profile_oracle(parent, median, dual_prefix=True)
    profile_one = _simulate_profile_oracle(parent, median, dual_prefix=False)

    grid_rows: list[dict[str, Any]] = []
    reentry_frames: dict[int, pd.DataFrame] = {}
    for cooldown in COOLDOWNS:
        local = _simulate_reentry_upper_bound(parent, median, cooldown)
        reentry_frames[cooldown] = local
        cal = _metrics(local[local["split"] == "calibration"])
        hold = _metrics(local[local["split"] == "holdout"])
        grid_rows.append(
            {
                "cooldown": cooldown,
                **{f"calibration_{k}": v for k, v in cal.items() if k != "profiles"},
                **{f"holdout_{k}": v for k, v in hold.items() if k != "profiles"},
            }
        )
    grid = pd.DataFrame(grid_rows)
    grid["gate_violation"] = (
        np.maximum(0.0, grid["calibration_median_regret"] / 1.15 - 1.0)
        + np.maximum(0.0, grid["calibration_p95_regret"] / 1.5 - 1.0)
        + np.maximum(0.0, grid["calibration_median_startup_overhead"] / 0.03 - 1.0)
        + np.maximum(0.0, grid["calibration_p95_startup_overhead"] / 0.05 - 1.0)
    )
    grid = grid.sort_values(["gate_violation", "calibration_p95_regret", "cooldown"])
    selected_cooldown = int(grid.iloc[0]["cooldown"])
    selected_reentry = reentry_frames[selected_cooldown]

    campaign_summary: list[dict[str, Any]] = []
    for index, campaign in campaigns.items():
        profile = _simulate_profile_oracle(parent, campaign, dual_prefix=True)
        reentry = _simulate_reentry_upper_bound(parent, campaign, selected_cooldown)
        for label, local in [("profile-oracle", profile), ("reentry-upper-bound", reentry)]:
            for split in ["calibration", "holdout"]:
                m = _metrics(local[local["split"] == split])
                campaign_summary.append({"campaign": index, "policy": label, "split": split, **{k: v for k, v in m.items() if k != "profiles"}})

    RESULTS.mkdir(parents=True, exist_ok=True)
    first.to_csv(RESULTS / "G4_S5B2_FIRST_STEP_ORACLE.csv", index=False)
    profile_dual.to_csv(RESULTS / "G4_S5B2_PROFILE_ORACLE_DUAL_PREFIX.csv", index=False)
    profile_one.to_csv(RESULTS / "G4_S5B2_PROFILE_ORACLE_ONE_SIDED.csv", index=False)
    selected_reentry.to_csv(RESULTS / "G4_S5B2_REENTRY_UPPER_BOUND.csv", index=False)
    grid.to_csv(RESULTS / "G4_S5B2_REENTRY_COOLDOWN_GRID.csv", index=False)
    pd.DataFrame(campaign_summary).to_csv(RESULTS / "G4_S5B2_CAMPAIGN_SUMMARY.csv", index=False)

    short = profile_dual[profile_dual["steps"] <= 6]
    long = profile_dual[profile_dual["steps"] > 6]
    profile_metrics = {
        split: _metrics(profile_dual[profile_dual["split"] == split])
        for split in ["calibration", "holdout"]
    }
    reentry_metrics = {
        split: _metrics(selected_reentry[selected_reentry["split"] == split])
        for split in ["calibration", "holdout"]
    }
    result = {
        "schema": "g4-s5b2-startup-profile-preflight-v1",
        "status": "complete-blocked-before-prefix-implementation",
        "active_switching": False,
        "candidate_source_modified": False,
        "contract_under_test": "one-time-startup-economic-profile-without-periodic-review",
        "scientific_sha256": hashlib.sha256(
            median.sort_values(["trajectory_key", "step_index"]).to_json(orient="records", double_precision=15).encode()
        ).hexdigest(),
        "first_step_oracle": {
            split: _metrics(first[first["split"] == split]) for split in ["calibration", "holdout"]
        },
        "perfect_trajectory_profile_oracle_dual_prefix": profile_metrics,
        "perfect_trajectory_profile_oracle_one_sided": {
            split: _metrics(profile_one[profile_one["split"] == split]) for split in ["calibration", "holdout"]
        },
        "short_trajectory_prefix_geometry": {
            "rows_steps_le_6": int(len(short)),
            "median_overhead": float(short["startup_overhead_ratio"].median()) if len(short) else None,
            "p95_overhead": float(short["startup_overhead_ratio"].quantile(0.95)) if len(short) else None,
            "long_rows": int(len(long)),
            "long_median_overhead": float(long["startup_overhead_ratio"].median()) if len(long) else None,
        },
        "optimistic_reentry_upper_bound": {
            "selected_cooldown": selected_cooldown,
            "calibration": reentry_metrics["calibration"],
            "holdout": reentry_metrics["holdout"],
            "interpretation": "regret gates pass, but startup-prefix P95 overhead remains above gate because short R-profile trajectories cannot amortize a dual/one-sided economic probe",
        },
        "gates": {
            "startup_only_profile_calibration_median_regret_le_1_15": profile_metrics["calibration"]["median_regret"] <= 1.15,
            "startup_only_profile_calibration_p95_regret_le_1_5": profile_metrics["calibration"]["p95_regret"] <= 1.5,
            "startup_only_profile_holdout_median_regret_le_1_15": profile_metrics["holdout"]["median_regret"] <= 1.15,
            "startup_only_profile_holdout_p95_regret_le_1_5": profile_metrics["holdout"]["p95_regret"] <= 1.5,
            "startup_only_profile_prefix_overhead_gate": (
                profile_metrics["calibration"]["median_startup_overhead"] < 0.03
                and profile_metrics["calibration"]["p95_startup_overhead"] < 0.05
                and profile_metrics["holdout"]["median_startup_overhead"] < 0.03
                and profile_metrics["holdout"]["p95_startup_overhead"] < 0.05
            ),
            "actual_prefix_implementation_authorized": False,
            "active_switching_authorized": False,
        },
        "closeout": {
            "startup_only_architecture": "REJECT",
            "actual_dual_prefix_bakeoff": "NOT_IMPLEMENTED_BECAUSE_ORACLE_UPPER_BOUND_FAILED",
            "reusable_prefix_kernel": "PRESERVE",
            "automatic_switching_research": "CONTINUE_ONLY_WITH_EVENT_TRIGGERED_REENTRY_AND_SHORT_HORIZON_ADMISSION",
        },
    }
    return result


def _plots(result: dict[str, Any]) -> None:
    PLOTS.mkdir(parents=True, exist_ok=True)
    profile = pd.read_csv(RESULTS / "G4_S5B2_PROFILE_ORACLE_DUAL_PREFIX.csv")
    reentry = pd.read_csv(RESULTS / "G4_S5B2_REENTRY_UPPER_BOUND.csv")
    campaign = pd.read_csv(RESULTS / "G4_S5B2_CAMPAIGN_SUMMARY.csv")

    labels = profile["trajectory_id"] + "@" + profile["rtol"].map(lambda x: f"{x:g}")
    x = np.arange(len(profile))
    plt.figure(figsize=(12, 5.2))
    plt.bar(x - 0.18, profile["regret"], 0.36, label="Perfect startup-only profile oracle")
    aligned = reentry.set_index("trajectory_key").reindex(profile["trajectory_key"])
    plt.bar(x + 0.18, aligned["regret"], 0.36, label="Optimistic safety-reentry upper bound")
    plt.axhline(1.15, linestyle="--", label="median gate 1.15")
    plt.axhline(1.5, linestyle=":", label="P95 gate 1.50")
    plt.xticks(x, labels, rotation=65, ha="right", fontsize=7)
    plt.ylabel("Trajectory regret")
    plt.title("Startup-only architecture fails even with future-perfect profile choice")
    plt.legend(fontsize=8)
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B2_STARTUP_PROFILE_REGRET.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.5, 4.8))
    plt.scatter(profile["steps"], profile["startup_overhead_ratio"], label="Dual-prefix profile oracle")
    plt.axhline(0.03, linestyle="--", label="median gate 3%")
    plt.axhline(0.05, linestyle=":", label="P95 gate 5%")
    plt.axvline(6, linestyle="-.", label="six-step amortization reference")
    plt.xlabel("Accepted steps in trajectory")
    plt.ylabel("Startup prefix / selected-method cost")
    plt.title("Short trajectories cannot amortize a startup bake-off")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B2_PREFIX_OVERHEAD_VS_LENGTH.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.5, 4.8))
    for policy in ["profile-oracle", "reentry-upper-bound"]:
        local = campaign[(campaign["policy"] == policy) & (campaign["split"] == "holdout")]
        plt.plot(local["campaign"], local["p95_regret"], marker="o", label=policy)
    plt.axhline(1.5, linestyle="--", label="P95 gate 1.50")
    plt.xlabel("Canonical campaign")
    plt.ylabel("Holdout P95 trajectory regret")
    plt.title("Campaign robustness of the architecture verdict")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B2_CAMPAIGN_ROBUSTNESS.png", dpi=180)
    plt.close()

    reason_rows = []
    for _, row in profile.iterrows():
        reason = "short-prefix-overhead" if row["steps"] <= 6 and row["startup_overhead_ratio"] > 0.05 else "regime-drift"
        if row["regret"] <= 1.15:
            reason = "within-median-gate"
        reason_rows.append(reason)
    counts = pd.Series(reason_rows).value_counts()
    plt.figure(figsize=(6.8, 4.6))
    plt.bar(counts.index, counts.values)
    plt.ylabel("Trajectory count")
    plt.title("Failure geometry of one-time startup admission")
    plt.xticks(rotation=20)
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B2_FAILURE_GEOMETRY.png", dpi=180)
    plt.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=RESULTS / "G4_S5B2_ANALYSIS.json")
    args = parser.parse_args()
    parent = _load_parent_module()
    hard_root = Path(tempfile.mkdtemp())
    atlas_root = Path(tempfile.mkdtemp())
    try:
        parent._extract_tar_zst(
            REPO / "research/generic_asymmetric_transactional_shadow_v21/inputs/GENERIC_Step_Local_Shadow_G4S1_v1.5.0-alpha1_RAW.tar.zst",
            hard_root,
        )
        parent._extract_tar_zst(
            REPO / "research/generic_asymmetric_transactional_shadow_v21/inputs/GENERIC_Regime_Atlas_G4S5B0_v2.0.0-alpha1_canonical_raw.tar.zst",
            atlas_root,
        )
        campaigns = parent.build_campaigns(hard_root / "campaigns_v2", atlas_root / "results/campaigns")
        result = _analyze(parent, campaigns)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        _plots(result)
        print(json.dumps(result, indent=2, sort_keys=True))
    finally:
        import shutil
        shutil.rmtree(hard_root, ignore_errors=True)
        shutil.rmtree(atlas_root, ignore_errors=True)


if __name__ == "__main__":
    import tempfile
    main()
