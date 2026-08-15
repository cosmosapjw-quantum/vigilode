#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import math
from collections import Counter
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
CAMPAIGN_DIR = ROOT / "results" / "campaigns"
RESULTS = ROOT / "results"
PLOTS = ROOT / "plots"

TIMING_FIELDS = {
    "rodas_wall_seconds",
    "exponential_wall_seconds",
    "exponential_prefix_wall_seconds",
}
NON_SCIENCE_FIELDS = TIMING_FIELDS | {"exponential_failure"}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def residence_segments(rows: pd.DataFrame, winner_col: str) -> list[dict]:
    out: list[dict] = []
    for trajectory_id, group in rows.groupby("trajectory_id", sort=False):
        group = group.sort_values("step_index")
        segment_index = 0
        start = 0
        values = group[winner_col].tolist()
        records = group.to_dict("records")
        while start < len(values):
            method = values[start]
            end = start + 1
            while end < len(values) and values[end] == method:
                end += 1
            local = records[start:end]
            out.append(
                {
                    "trajectory_id": trajectory_id,
                    "family": local[0]["family"],
                    "dimension": int(local[0]["dimension"]),
                    "segment_index": segment_index,
                    "winner": method,
                    "start_step": int(local[0]["step_index"]),
                    "end_step": int(local[-1]["step_index"]),
                    "length_steps": end - start,
                    "duration": float(sum(row["h"] for row in local)),
                    "transition_start": float(local[0]["transition_level"]),
                    "transition_end": float(local[-1]["transition_level"]),
                    "rodas_wall_sum": float(sum(row["rodas_wall_median"] for row in local)),
                    "exponential_wall_sum": float(
                        sum(row["exponential_wall_median"] for row in local)
                    ),
                    "entry_prefix_wall": (
                        float(local[0]["exponential_prefix_wall_median"])
                        if method == "E-K"
                        else 0.0
                    ),
                }
            )
            segment_index += 1
            start = end
    return out


def main() -> None:
    files = sorted(CAMPAIGN_DIR.glob("G4S5B0_CANONICAL_[1-7].json"))
    if len(files) != 7:
        raise RuntimeError(f"expected seven campaigns, found {len(files)}")
    campaigns = [json.loads(path.read_text()) for path in files]

    # Scientific/work identity, excluding only timing and free-form failure text.
    scientific_hashes: list[str] = []
    for campaign in campaigns:
        reduced_rows = [
            {key: value for key, value in row.items() if key not in NON_SCIENCE_FIELDS}
            for row in campaign["rows"]
        ]
        payload = {
            "schema": campaign["schema"],
            "status": campaign["status"],
            "profile": campaign["profile"],
            "switching_active": campaign["switching_active"],
            "committed_method": campaign["committed_method"],
            "rows": reduced_rows,
            "trajectories": campaign["trajectories"],
        }
        scientific_hashes.append(
            sha256_bytes(json.dumps(payload, sort_keys=True, separators=(",", ":")).encode())
        )
    scientific_identity = len(set(scientific_hashes)) == 1

    # Flatten campaign rows.
    flat: list[dict] = []
    for campaign_index, campaign in enumerate(campaigns, start=1):
        for row in campaign["rows"]:
            flat.append({"campaign": campaign_index, **row})
    frame = pd.DataFrame(flat)

    key_cols = [
        "trajectory_id",
        "family",
        "dimension",
        "rtol",
        "step_index",
        "t_start",
        "h",
        "transition_level",
    ]
    scientific_cols = [
        "rodas_embedded_error",
        "rodas_rhs_evaluations",
        "rodas_jvp_vectors",
        "rodas_linear_matvecs",
        "exponential_completed",
        "exponential_total_error",
        "exponential_locally_admissible",
        "exponential_rhs_evaluations",
        "exponential_jvp_vectors",
        "exponential_maximum_krylov_dimension",
        "exponential_phi_substeps",
    ]

    aggregated: list[dict] = []
    winner_campaign_counts: list[int] = []
    for keys, group in frame.groupby(key_cols, sort=True, dropna=False):
        row = dict(zip(key_cols, keys))
        first = group.iloc[0]
        for field in scientific_cols:
            row[field] = first[field]
        row["campaign_count"] = int(len(group))
        for field in TIMING_FIELDS:
            values = group[field].dropna().astype(float).to_numpy()
            row[f"{field.removesuffix('_seconds')}_median"] = (
                float(np.median(values)) if len(values) else math.nan
            )
            row[f"{field.removesuffix('_seconds')}_q25"] = (
                float(np.quantile(values, 0.25)) if len(values) else math.nan
            )
            row[f"{field.removesuffix('_seconds')}_q75"] = (
                float(np.quantile(values, 0.75)) if len(values) else math.nan
            )
            row[f"{field.removesuffix('_seconds')}_cv"] = (
                float(np.std(values, ddof=1) / np.mean(values))
                if len(values) > 1 and np.mean(values) > 0
                else 0.0
            )

        campaign_e_wins = 0
        for _, item in group.iterrows():
            e_wall = item["exponential_wall_seconds"]
            e_wins = (
                bool(item["exponential_locally_admissible"])
                and pd.notna(e_wall)
                and float(e_wall) < float(item["rodas_wall_seconds"])
            )
            campaign_e_wins += int(e_wins)
        winner_campaign_counts.append(campaign_e_wins)
        row["e_winner_campaigns"] = campaign_e_wins
        row["winner_stability"] = max(campaign_e_wins, 7 - campaign_e_wins) / 7.0

        r_wall = row["rodas_wall_median"]
        e_wall = row["exponential_wall_median"]
        admissible = bool(row["exponential_locally_admissible"])
        if admissible and math.isfinite(e_wall) and e_wall < r_wall:
            row["raw_winner"] = "E-K"
            row["raw_oracle_wall"] = e_wall
        else:
            row["raw_winner"] = "R-JF"
            row["raw_oracle_wall"] = r_wall

        if not admissible or not math.isfinite(e_wall):
            row["strong_winner"] = "R-JF"
            row["r_winner_reason"] = "e-k-inadmissible"
        elif e_wall * 1.05 < r_wall:
            row["strong_winner"] = "E-K"
            row["r_winner_reason"] = ""
        elif r_wall * 1.05 < e_wall:
            row["strong_winner"] = "R-JF"
            row["r_winner_reason"] = "r-jf-at-least-5pct-faster"
        else:
            row["strong_winner"] = "UNCERTAIN"
            row["r_winner_reason"] = "near-tie"

        row["wall_ratio_e_over_r"] = (
            e_wall / r_wall if math.isfinite(e_wall) and r_wall > 0 else math.inf
        )
        aggregated.append(row)

    agg = pd.DataFrame(aggregated).sort_values(
        ["family", "dimension", "step_index"]
    )
    agg.to_csv(RESULTS / "G4_S5B0_AGGREGATED_STEPS.csv", index=False)

    raw_segments = pd.DataFrame(residence_segments(agg, "raw_winner"))
    strong_segments = pd.DataFrame(residence_segments(agg, "strong_winner"))
    raw_segments.to_csv(RESULTS / "G4_S5B0_RESIDENCE_SEGMENTS.csv", index=False)
    strong_segments.to_csv(
        RESULTS / "G4_S5B0_STRONG_RESIDENCE_SEGMENTS.csv", index=False
    )

    trajectory_rows: list[dict] = []
    for trajectory_id, group in agg.groupby("trajectory_id", sort=True):
        group = group.sort_values("step_index")
        segments = raw_segments[raw_segments["trajectory_id"] == trajectory_id]
        baseline = float(group["rodas_wall_median"].sum())
        raw_oracle = float(group["raw_oracle_wall"].sum())
        e_entries = segments[segments["winner"] == "E-K"]
        prefix_penalty = float(e_entries["entry_prefix_wall"].sum())
        sealed_adjusted = raw_oracle + prefix_penalty
        reuse_corrected = raw_oracle
        e_lengths = e_entries["length_steps"].astype(int).tolist()
        r_lengths = segments[segments["winner"] == "R-JF"][
            "length_steps"
        ].astype(int).tolist()
        e_first_cost = float(group["exponential_wall_median"].sum()) + float(
            group.loc[
                ~group["exponential_locally_admissible"], "rodas_wall_median"
            ].sum()
        )
        trajectory_rows.append(
            {
                "trajectory_id": trajectory_id,
                "family": group.iloc[0]["family"],
                "dimension": int(group.iloc[0]["dimension"]),
                "accepted_steps": int(len(group)),
                "e_winner_steps": int((group["raw_winner"] == "E-K").sum()),
                "r_winner_steps": int((group["raw_winner"] == "R-JF").sum()),
                "e_residence_segments": len(e_lengths),
                "r_residence_segments": len(r_lengths),
                "max_e_residence": max(e_lengths, default=0),
                "max_r_residence": max(r_lengths, default=0),
                "e_residence_ge6_count": sum(length >= 6 for length in e_lengths),
                "baseline_rodas_wall": baseline,
                "oracle_wall_without_entry": raw_oracle,
                "sealed_entry_prefix_penalty": prefix_penalty,
                "sealed_entry_adjusted_wall": sealed_adjusted,
                "entry_adjusted_speedup": baseline / sealed_adjusted,
                "reuse_corrected_speedup": baseline / reuse_corrected,
                "e_first_transactional_wall": e_first_cost,
                "e_first_transactional_speedup": baseline / e_first_cost,
                "e_first_fallback_steps": int(
                    (~group["exponential_locally_admissible"]).sum()
                ),
                "prefix_penalty_over_oracle": prefix_penalty / raw_oracle,
                "median_step_winner_stability": float(
                    np.median(group["winner_stability"])
                ),
            }
        )
    trajectory = pd.DataFrame(trajectory_rows)
    trajectory.to_csv(RESULTS / "G4_S5B0_TRAJECTORY_SUMMARY.csv", index=False)

    # Campaign sensitivity using one-shot paired timings from each full campaign.
    campaign_summaries: list[dict] = []
    for campaign_index, group in frame.groupby("campaign", sort=True):
        baseline = float(group["rodas_wall_seconds"].sum())
        oracle = 0.0
        prefix = 0.0
        for _, trajectory_group in group.groupby("trajectory_id", sort=False):
            trajectory_group = trajectory_group.sort_values("step_index")
            previous = None
            for _, row in trajectory_group.iterrows():
                e_wins = (
                    bool(row["exponential_locally_admissible"])
                    and pd.notna(row["exponential_wall_seconds"])
                    and float(row["exponential_wall_seconds"])
                    < float(row["rodas_wall_seconds"])
                )
                winner = "E-K" if e_wins else "R-JF"
                oracle += (
                    float(row["exponential_wall_seconds"])
                    if e_wins
                    else float(row["rodas_wall_seconds"])
                )
                if winner == "E-K" and previous != "E-K":
                    prefix += float(row["exponential_prefix_wall_seconds"])
                previous = winner
        campaign_summaries.append(
            {
                "campaign": int(campaign_index),
                "baseline_rodas_wall": baseline,
                "oracle_wall_without_entry": oracle,
                "entry_prefix_penalty": prefix,
                "entry_adjusted_speedup": baseline / (oracle + prefix),
            }
        )
    campaign_df = pd.DataFrame(campaign_summaries)
    campaign_df.to_csv(RESULTS / "G4_S5B0_CAMPAIGN_SENSITIVITY.csv", index=False)

    # Aggregate economics and residence distributions.
    aggregate_baseline = float(trajectory["baseline_rodas_wall"].sum())
    aggregate_oracle = float(trajectory["oracle_wall_without_entry"].sum())
    aggregate_penalty = float(trajectory["sealed_entry_prefix_penalty"].sum())
    aggregate_adjusted = aggregate_oracle + aggregate_penalty
    aggregate_e_first = float(trajectory["e_first_transactional_wall"].sum())
    e_segments = raw_segments[raw_segments["winner"] == "E-K"]
    r_segments = raw_segments[raw_segments["winner"] == "R-JF"]
    long_e_segments = e_segments[e_segments["length_steps"] >= 6]
    long_e_steps = int(long_e_segments["length_steps"].sum())
    all_e_steps = int(e_segments["length_steps"].sum())
    reasons = Counter(agg.loc[agg["raw_winner"] == "R-JF", "r_winner_reason"])

    all_trajectory_success = all(
        item["success"] for item in campaigns[0]["trajectories"]
    )
    matrix_free = all(
        item["explicit_jacobian_builds"] == 0
        and item["direct_factorizations"] == 0
        and item["newton_iterations"] == 0
        for item in campaigns[0]["trajectories"]
    )
    nontrivial_e = bool((e_segments["length_steps"] >= 2).any())
    nontrivial_r = bool((r_segments["length_steps"] >= 2).any())
    long_e_exists = bool((e_segments["length_steps"] >= 6).any())
    selector_gain = aggregate_baseline / aggregate_adjusted

    sensitivity_rows = []
    for economic_margin in [1.0, 1.05, 1.15, 1.25]:
        for e_multiplier, r_multiplier, label in [
            (1.0, 1.0, "nominal"),
            (1.2, 0.8, "adverse-to-e-k"),
            (0.8, 1.2, "favorable-to-e-k"),
        ]:
            baseline = float((agg["rodas_wall_median"] * r_multiplier).sum())
            total = 0.0
            e_steps = 0
            for _, group in agg.groupby("trajectory_id", sort=False):
                group = group.sort_values("step_index")
                previous = None
                for _, row in group.iterrows():
                    r_cost = float(row["rodas_wall_median"]) * r_multiplier
                    e_cost = float(row["exponential_wall_median"]) * e_multiplier
                    e_wins = (
                        bool(row["exponential_locally_admissible"])
                        and e_cost * economic_margin < r_cost
                    )
                    winner = "E-K" if e_wins else "R-JF"
                    total += e_cost if e_wins else r_cost
                    if e_wins and previous != "E-K":
                        total += float(row["exponential_prefix_wall_median"])
                    e_steps += int(e_wins)
                    previous = winner
            sensitivity_rows.append(
                {
                    "economic_margin": economic_margin,
                    "timing_mutation": label,
                    "e_k_winner_steps": e_steps,
                    "entry_adjusted_speedup": baseline / total,
                }
            )
    sensitivity = pd.DataFrame(sensitivity_rows)
    sensitivity.to_csv(RESULTS / "G4_S5B0_ECONOMIC_SENSITIVITY.csv", index=False)

    analysis = {
        "schema": "g4-s5b0-expanded-paired-regime-atlas-analysis-v1",
        "status": "complete-read-only-atlas",
        "active_switching": False,
        "candidate_source_modified_after_protocol": False,
        "campaigns": len(campaigns),
        "scientific_identity": scientific_identity,
        "scientific_sha256": scientific_hashes[0],
        "corpus": {
            "families": sorted(agg["family"].unique().tolist()),
            "dimensions": sorted(int(value) for value in agg["dimension"].unique()),
            "trajectories": int(agg["trajectory_id"].nunique()),
            "accepted_step_rows": int(len(agg)),
            "all_trajectories_successful": all_trajectory_success,
            "scope_note": (
                "Transition-bearing scaled/replicated atlas variants; not exact historical "
                "work-precision reproductions of Robertson, HIRES, or van der Pol."
            ),
        },
        "winner_map": {
            "e_k_steps": int((agg["raw_winner"] == "E-K").sum()),
            "r_jf_steps": int((agg["raw_winner"] == "R-JF").sum()),
            "uncertain_strong_steps": int((agg["strong_winner"] == "UNCERTAIN").sum()),
            "unanimous_campaign_winner_rows": int(
                ((agg["e_winner_campaigns"] == 0) | (agg["e_winner_campaigns"] == 7)).sum()
            ),
            "winner_stability_histogram": {
                str(key): int(value)
                for key, value in Counter(winner_campaign_counts).items()
            },
            "r_jf_reason_counts": {str(key): int(value) for key, value in reasons.items()},
        },
        "residence": {
            "e_k_segments": int(len(e_segments)),
            "r_jf_segments": int(len(r_segments)),
            "e_k_segment_lengths": [int(value) for value in e_segments["length_steps"]],
            "r_jf_segment_lengths": [int(value) for value in r_segments["length_steps"]],
            "e_k_segments_ge6": int(len(long_e_segments)),
            "fraction_e_k_segments_ge6": float(
                len(long_e_segments) / len(e_segments) if len(e_segments) else 0.0
            ),
            "fraction_e_k_winner_steps_in_ge6_segments": float(
                long_e_steps / all_e_steps if all_e_steps else 0.0
            ),
            "trajectories_with_e_k_residence_ge6": int(
                (trajectory["max_e_residence"] >= 6).sum()
            ),
        },
        "economics": {
            "static_rodas_wall": aggregate_baseline,
            "oracle_wall_without_entry": aggregate_oracle,
            "sealed_conservative_entry_prefix_penalty": aggregate_penalty,
            "sealed_entry_adjusted_oracle_wall": aggregate_adjusted,
            "reuse_corrected_oracle_speedup": aggregate_baseline / aggregate_oracle,
            "sealed_entry_adjusted_speedup": selector_gain,
            "sealed_entry_adjusted_gain_fraction": selector_gain - 1.0,
            "e_first_transactional_wall": aggregate_e_first,
            "e_first_transactional_speedup": aggregate_baseline / aggregate_e_first,
            "e_first_fallback_steps": int(
                (~agg["exponential_locally_admissible"]).sum()
            ),
            "prefix_penalty_over_oracle": aggregate_penalty / aggregate_oracle,
            "campaign_speedup_median": float(
                np.median(campaign_df["entry_adjusted_speedup"])
            ),
            "campaign_speedup_min": float(
                campaign_df["entry_adjusted_speedup"].min()
            ),
            "campaign_speedup_max": float(
                campaign_df["entry_adjusted_speedup"].max()
            ),
        },
        "economic_sensitivity": {
            "nominal_margin_5pct_speedup": float(
                sensitivity[
                    (sensitivity["economic_margin"] == 1.05)
                    & (sensitivity["timing_mutation"] == "nominal")
                ]["entry_adjusted_speedup"].iloc[0]
            ),
            "adverse_20pct_e_penalty_r_discount_speedup": float(
                sensitivity[
                    (sensitivity["economic_margin"] == 1.05)
                    & (sensitivity["timing_mutation"] == "adverse-to-e-k")
                ]["entry_adjusted_speedup"].iloc[0]
            ),
            "favorable_20pct_e_discount_r_penalty_speedup": float(
                sensitivity[
                    (sensitivity["economic_margin"] == 1.05)
                    & (sensitivity["timing_mutation"] == "favorable-to-e-k")
                ]["entry_adjusted_speedup"].iloc[0]
            ),
        },
        "timing_quality": {
            "row_wall_cv_median_rodas": float(np.median(agg["rodas_wall_cv"])),
            "row_wall_cv_p95_rodas": float(np.quantile(agg["rodas_wall_cv"], 0.95)),
            "row_wall_cv_median_exponential": float(
                np.median(agg["exponential_wall_cv"])
            ),
            "row_wall_cv_p95_exponential": float(
                np.quantile(agg["exponential_wall_cv"], 0.95)
            ),
            "fixed_measurement_order": "R-JF committed step is timed before E-K shadow step",
            "authority_ceiling": (
                "Repeated paired atlas economics, not final randomized work-precision ranking."
            ),
        },
        "gates": {
            "scientific_identity": scientific_identity,
            "all_trajectories_successful": all_trajectory_success,
            "matrix_free_newton_free": matrix_free,
            "both_methods_nontrivial_residence": nontrivial_e and nontrivial_r,
            "e_k_residence_ge6_exists": long_e_exists,
            "entry_adjusted_oracle_gain_ge5pct": selector_gain >= 1.05,
            "selector_research_continues": (
                scientific_identity
                and all_trajectory_success
                and matrix_free
                and nontrivial_e
                and nontrivial_r
                and long_e_exists
                and selector_gain >= 1.05
            ),
            "active_switching_authorized": False,
        },
        "closeout": {
            "paired_regime_atlas": "PROMOTE_READ_ONLY",
            "residence_time_economics": "PROMOTE_TO_ASYMMETRIC_SHADOW",
            "active_switching": "HOLD",
            "next_node": "G4-S5B1 asymmetric one-sided shadow selector",
        },
    }
    (RESULTS / "G4_S5B0_ANALYSIS.json").write_text(
        json.dumps(analysis, indent=2, sort_keys=True) + "\n"
    )

    # Plots — each figure is distinct and uses matplotlib defaults.
    trajectory_order = trajectory.sort_values(["family", "dimension"])[
        "trajectory_id"
    ].tolist()
    plt.figure(figsize=(10.5, 6.3))
    for y_index, trajectory_id in enumerate(trajectory_order):
        group = agg[agg["trajectory_id"] == trajectory_id].sort_values("t_start")
        t0 = group["t_start"].min()
        tf = (group["t_start"] + group["h"]).max()
        x = (group["t_start"] - t0) / max(tf - t0, 1e-30)
        e_mask = group["raw_winner"] == "E-K"
        r_mask = ~e_mask
        plt.scatter(x[e_mask], np.full(e_mask.sum(), y_index), marker="o", s=13)
        plt.scatter(x[r_mask], np.full(r_mask.sum(), y_index), marker="x", s=18)
    plt.yticks(range(len(trajectory_order)), trajectory_order)
    plt.xlabel("Normalized committed-trajectory time")
    plt.ylabel("Trajectory")
    plt.title("Step-local oracle winner timeline: circles E-K, crosses R-JF")
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_WINNER_TIMELINE.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.3, 4.8))
    e_lengths = np.sort(e_segments["length_steps"].to_numpy())
    r_lengths = np.sort(r_segments["length_steps"].to_numpy())
    plt.plot(np.arange(1, len(e_lengths) + 1), e_lengths, marker="o", label="E-K")
    plt.plot(np.arange(1, len(r_lengths) + 1), r_lengths, marker="x", label="R-JF")
    plt.axhline(6, linestyle="--", label="Six-step amortization threshold")
    plt.xlabel("Residence segment rank")
    plt.ylabel("Accepted-step residence length")
    plt.title("Winner-regime residence lengths")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_RESIDENCE_LENGTHS.png", dpi=180)
    plt.close()

    plt.figure(figsize=(9.5, 4.8))
    ordered = trajectory.sort_values(["family", "dimension"])
    plt.bar(np.arange(len(ordered)), ordered["entry_adjusted_speedup"])
    plt.axhline(1.05, linestyle="--", label="Continue-selector bar: 1.05")
    plt.xticks(np.arange(len(ordered)), ordered["trajectory_id"], rotation=45, ha="right")
    plt.ylabel("Static R-JF / entry-adjusted oracle wall")
    plt.title("Entry-adjusted oracle speedup by trajectory")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_ENTRY_ADJUSTED_SPEEDUP.png", dpi=180)
    plt.close()

    plt.figure(figsize=(9.5, 4.8))
    ordered = trajectory.sort_values(["family", "dimension"])
    plt.bar(np.arange(len(ordered)), ordered["e_first_transactional_speedup"])
    plt.axhline(1.0, linestyle="--", label="Static R-JF parity")
    plt.xticks(np.arange(len(ordered)), ordered["trajectory_id"], rotation=45, ha="right")
    plt.ylabel("Static R-JF / E-first-with-fallback wall")
    plt.title("Transactional E-first attempt with R-JF fallback")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_E_FIRST_TRANSACTIONAL_SPEEDUP.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.3, 4.8))
    for _, segment in e_segments.iterrows():
        overhead = segment["entry_prefix_wall"] / max(
            segment["exponential_wall_sum"], 1e-30
        )
        plt.scatter(segment["length_steps"], overhead)
    plt.axvline(6, linestyle="--", label="Six-step threshold")
    plt.axhline(0.03, linestyle=":", label="Three-percent overhead")
    plt.xlabel("E-K residence length [accepted steps]")
    plt.ylabel("Entry prefix / E-K segment wall")
    plt.title("E-K prefix amortization by residence segment")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_PREFIX_AMORTIZATION.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.3, 4.8))
    counts = Counter(winner_campaign_counts)
    xs = sorted(counts)
    plt.bar(xs, [counts[value] for value in xs])
    plt.xlabel("Campaigns (of 7) in which E-K won the step")
    plt.ylabel("Accepted-step rows")
    plt.title("Winner-label sensitivity across repeated campaigns")
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_WINNER_STABILITY.png", dpi=180)
    plt.close()

    plt.figure(figsize=(7.3, 4.8))
    admissible = agg[agg["exponential_locally_admissible"]]
    inadmissible = agg[~agg["exponential_locally_admissible"]]
    plt.scatter(
        admissible["transition_level"],
        admissible["wall_ratio_e_over_r"],
        marker="o",
        s=12,
        label="E-K locally admissible",
    )
    plt.scatter(
        inadmissible["transition_level"],
        inadmissible["wall_ratio_e_over_r"],
        marker="x",
        s=18,
        label="E-K inadmissible",
    )
    plt.axhline(1.0, linestyle="--", label="Equal wall")
    plt.yscale("log")
    plt.xlabel("Controlled transition level")
    plt.ylabel("Median E-K / R-JF step wall")
    plt.title("Transition level, admissibility, and paired step cost")
    plt.legend()
    plt.tight_layout()
    plt.savefig(PLOTS / "G4_S5B0_TRANSITION_COST_RATIO.png", dpi=180)
    plt.close()

    print(json.dumps(analysis, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
