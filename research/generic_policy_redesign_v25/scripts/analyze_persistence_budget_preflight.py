#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
SHARDS = ROOT / "results" / "calibration128_shards"
OUT = ROOT / "results" / "calibration128_analysis"
PLOTS = ROOT / "plots"
HIST_ROOT = Path(
    "/mnt/data/GVJF_POLICY_MATH_RESEARCH_20260817/source_v24/"
    "research/generic_regime_atlas_v20/results"
)
HIST_AGG = HIST_ROOT / "G4_S5B0_AGGREGATED_STEPS.csv"
HIST_RAW = HIST_ROOT / "campaigns" / "G4S5B0_CANONICAL_1.json"
CONTRACT = ROOT / "contracts" / "P1_01R_PERSISTENCE_BUDGET_CONTRACT.json"

FEATURES = ("jvp_pressure", "step_contraction", "error_curvature")
QUANTILES = (0.50, 0.65, 0.80, 0.90)
PERSISTENCE = (1, 2, 3)
DELTAS = (0.02, 0.03, 0.05)
TINY = 1.0e-300


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_new_rows() -> tuple[pd.DataFrame, pd.DataFrame]:
    rows: list[dict] = []
    trajectories: list[dict] = []
    files = sorted(SHARDS.glob("*.json"))
    if len(files) != 6:
        raise RuntimeError(f"expected six N=128 family shards, found {len(files)}")
    for path in files:
        payload = json.loads(path.read_text())
        if payload["schema"] != "g4-s5b0-rjf-only-regime-replay-v1":
            raise RuntimeError(f"unexpected schema in {path}")
        if payload["profile"] != "calibration-128":
            raise RuntimeError(f"unexpected profile in {path}")
        if payload["switching_active"]:
            raise RuntimeError("active switching appeared in R-only replay")
        rows.extend(payload["rows"])
        trajectories.extend(payload["trajectories"])
    frame = pd.DataFrame(rows).sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    traj = pd.DataFrame(trajectories).sort_values("trajectory_id").reset_index(drop=True)
    return frame, traj


def exact_rjf_parity(new: pd.DataFrame) -> dict:
    raw = json.loads(HIST_RAW.read_text())
    old = pd.DataFrame(raw["rows"])
    old = old[old["dimension"] == 128].sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    keys = ["trajectory_id", "family", "dimension", "rtol", "step_index"]
    fields = [
        "t_start", "h", "transition_level", "rodas_embedded_error",
        "rodas_rhs_evaluations", "rodas_jvp_vectors", "rodas_linear_matvecs",
    ]
    if not new[keys].equals(old[keys]):
        raise RuntimeError("R-JF row keys differ from canonical raw campaign")
    equality = {field: bool(np.array_equal(new[field].to_numpy(), old[field].to_numpy())) for field in fields}
    if not all(equality.values()):
        raise RuntimeError(f"R-JF numerical parity failed: {equality}")
    return {
        "raw_campaign_sha256": sha256(HIST_RAW),
        "rows": len(new),
        "fields": equality,
        "exact": True,
    }


def add_causal_features(frame: pd.DataFrame) -> pd.DataFrame:
    out = frame.sort_values(["trajectory_id", "step_index"]).copy()
    out["log_error"] = np.log10(np.maximum(out["rodas_embedded_error"].astype(float), TINY))
    out["jvp_pressure"] = np.log10(
        np.maximum(out["rodas_jvp_vectors"].astype(float), 1.0)
        / np.maximum(np.abs(out["h"].astype(float)), TINY)
    )
    out["step_contraction"] = -np.log10(np.maximum(np.abs(out["h"].astype(float)), TINY))
    two_back = out.groupby("trajectory_id")["log_error"].shift(2)
    out["error_curvature"] = -(out["log_error"] - two_back)
    out["next_step_index"] = out.groupby("trajectory_id")["step_index"].shift(-1)
    return out


def strong_e_segments(audit: pd.DataFrame) -> list[dict]:
    segments: list[dict] = []
    for tid, group in audit.groupby("trajectory_id", sort=False):
        g = group.sort_values("step_index")
        winners = g["strong_winner"].tolist()
        records = g.to_dict("records")
        i = 0
        prev = None
        while i < len(records):
            w = winners[i]
            j = i + 1
            while j < len(records) and winners[j] == w:
                j += 1
            local = records[i:j]
            if w == "E-K" and prev != "E-K" and int(local[0]["step_index"]) > 0:
                gains = [max(float(r["rodas_wall_median"]) - float(r["exponential_wall_median"]), 0.0) for r in local]
                segments.append({
                    "trajectory_id": tid,
                    "family": local[0]["family"],
                    "start_step": int(local[0]["step_index"]),
                    "end_step": int(local[-1]["step_index"]),
                    "length": len(local),
                    "gains": gains,
                    "total_gain": float(sum(gains)),
                })
            prev = w
            i = j
    return segments


@dataclass
class Candidate:
    feature: str
    quantile: float
    threshold: float
    k: int
    delta: float


def simulate_candidate(frame: pd.DataFrame, audit: pd.DataFrame, segments: list[dict], cand: Candidate, bound_ratio: float) -> dict:
    probes: list[dict] = []
    bound_violations = 0
    budget_violations = 0
    for tid, group in frame.groupby("trajectory_id", sort=False):
        g = group.sort_values("step_index")
        audit_g = audit[audit["trajectory_id"] == tid].sort_values("step_index").set_index("step_index")
        streak = 0
        latched = False
        cumulative_r = 0.0
        speculative = 0.0
        for rec in g.to_dict("records"):
            j = int(rec["step_index"])
            rwall = float(audit_g.loc[j, "rodas_wall_median"])
            cumulative_r += rwall
            value = rec.get(cand.feature)
            signal = value is not None and math.isfinite(float(value)) and float(value) >= cand.threshold
            if not signal:
                streak = 0
                latched = False
                continue
            if latched:
                continue
            streak += 1
            if streak < cand.k:
                continue
            latched = True
            next_step = rec.get("next_step_index")
            if next_step is None or not math.isfinite(float(next_step)):
                continue
            next_step = int(next_step)
            if next_step not in audit_g.index:
                continue
            prospective = bound_ratio * rwall
            if speculative + prospective > cand.delta * cumulative_r + 1e-18:
                continue
            actual_prefix = float(audit_g.loc[next_step, "exponential_prefix_wall_median"])
            if actual_prefix > prospective + 1e-15:
                bound_violations += 1
                continue
            speculative += actual_prefix
            if speculative > cand.delta * cumulative_r + 1e-15:
                budget_violations += 1
                continue
            probes.append({
                "trajectory_id": tid,
                "decision_step": j,
                "probe_step": next_step,
                "feature_value": float(value),
                "rwall_decision": rwall,
                "cumulative_r_wall": cumulative_r,
                "prospective_bound": prospective,
                "actual_prefix_wall": actual_prefix,
                "target_strong_winner": str(audit_g.loc[next_step, "strong_winner"]),
            })

    probes_df = pd.DataFrame(probes)
    probe_count = len(probes)
    precision = float((probes_df["target_strong_winner"] == "E-K").mean()) if probe_count else 0.0
    total_gain = float(sum(s["total_gain"] for s in segments))
    captured = 0.0
    detected_segments = 0
    for seg in segments:
        if probe_count == 0:
            continue
        local = probes_df[
            (probes_df["trajectory_id"] == seg["trajectory_id"])
            & (probes_df["probe_step"] >= seg["start_step"])
            & (probes_df["probe_step"] <= seg["end_step"])
        ]
        if local.empty:
            continue
        first_step = int(local["probe_step"].min())
        offset = first_step - seg["start_step"]
        captured += float(sum(seg["gains"][offset:]))
        detected_segments += 1

    total_r_wall = float(audit["rodas_wall_median"].sum())
    total_prefix = float(probes_df["actual_prefix_wall"].sum()) if probe_count else 0.0
    return {
        "feature": cand.feature,
        "quantile": cand.quantile,
        "threshold": cand.threshold,
        "k": cand.k,
        "delta": cand.delta,
        "probe_count": probe_count,
        "probe_precision": precision,
        "detected_r_to_e_segments": detected_segments,
        "r_to_e_segment_count": len(segments),
        "segment_recall_diagnostic": detected_segments / len(segments) if segments else 0.0,
        "opportunity_total": total_gain,
        "opportunity_captured": captured,
        "opportunity_retention": captured / total_gain if total_gain > 0 else 0.0,
        "actual_prefix_wall_total": total_prefix,
        "aggregate_prefix_overhead": total_prefix / total_r_wall if total_r_wall > 0 else math.inf,
        "prefix_bound_violations": bound_violations,
        "pathwise_budget_violations": budget_violations,
        "valid": bound_violations == 0 and budget_violations == 0,
        "probes": probes,
    }


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    PLOTS.mkdir(parents=True, exist_ok=True)
    contract = json.loads(CONTRACT.read_text())
    if contract["schema"] != "gvjf-p1-01r-persistence-budget-contract-v3":
        raise RuntimeError("unexpected contract schema")
    new, traj = load_new_rows()
    if not traj["success"].all():
        raise RuntimeError("an R-JF-only shard failed")
    if new["exponential_completed"].any():
        raise RuntimeError("E-K work appeared in R-only replay")
    if not all(new[c].isna().all() for c in [
        "exponential_total_error", "exponential_wall_seconds", "exponential_prefix_wall_seconds",
        "exponential_rhs_evaluations", "exponential_jvp_vectors", "exponential_failure"
    ]):
        raise RuntimeError("E-K audit field appeared in R-only replay")
    if int(traj["explicit_jacobian_builds"].sum()) != 0 or int(traj["newton_iterations"].sum()) != 0:
        raise RuntimeError("primary R-JF contract violated")

    parity = exact_rjf_parity(new)
    new = add_causal_features(new)
    audit = pd.read_csv(HIST_AGG)
    audit = audit[audit["dimension"] == 128].sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)

    # Next-state prefix / completed-current-R prospective scale.
    audit["next_prefix"] = audit.groupby("trajectory_id")["exponential_prefix_wall_median"].shift(-1)
    next_ratio = audit["next_prefix"] / audit["rodas_wall_median"]
    bound_ratio = 1.25 * float(next_ratio.dropna().max())

    # Feature thresholds are selected from causal R-only replay values only.
    thresholds: dict[str, dict[float, float]] = {}
    for feature in FEATURES:
        vals = new.loc[new["next_step_index"].notna(), feature].dropna().astype(float)
        thresholds[feature] = {q: float(vals.quantile(q)) for q in QUANTILES}

    segments = strong_e_segments(audit)
    results: list[dict] = []
    probe_payloads: dict[str, list[dict]] = {}
    for feature in FEATURES:
        for q in QUANTILES:
            threshold = thresholds[feature][q]
            for k in PERSISTENCE:
                for delta in DELTAS:
                    cand = Candidate(feature, q, threshold, k, delta)
                    result = simulate_candidate(new, audit, segments, cand, bound_ratio)
                    key = f"{feature}|q={q:.2f}|k={k}|delta={delta:.2f}"
                    probe_payloads[key] = result.pop("probes")
                    result["candidate_key"] = key
                    results.append(result)

    frame = pd.DataFrame(results)
    if len(frame) != 108:
        raise RuntimeError(f"expected 108 candidates, found {len(frame)}")
    valid = frame[frame["valid"]].copy()
    if valid.empty:
        raise RuntimeError("no valid calibration candidate")
    selected = valid.sort_values(
        ["opportunity_retention", "probe_precision", "probe_count", "aggregate_prefix_overhead", "candidate_key"],
        ascending=[False, False, True, True, True],
        kind="mergesort",
    ).iloc[0]
    selected_key = str(selected["candidate_key"])

    frame.to_csv(OUT / "CALIBRATION_CANDIDATES.csv", index=False)
    pd.DataFrame(probe_payloads[selected_key]).to_csv(OUT / "SELECTED_POLICY_PROBES.csv", index=False)
    new.to_csv(OUT / "RJF_ONLY_ROWS_WITH_CAUSAL_FEATURES.csv", index=False)
    traj.to_csv(OUT / "RJF_ONLY_TRAJECTORIES.csv", index=False)

    # Plot the 108-policy candidate cloud.
    fig, ax = plt.subplots(figsize=(7.2, 4.8))
    for k, group in valid.groupby("k"):
        ax.scatter(group["opportunity_retention"], group["probe_precision"], s=34, alpha=0.7, label=f"k={k}")
    ax.scatter([selected["opportunity_retention"]], [selected["probe_precision"]], marker="*", s=190, label="selected")
    ax.set_xlabel("Retained R→E retrospective opportunity")
    ax.set_ylabel("Probe precision")
    ax.set_title("N=128 causal persistence-budget calibration candidates")
    ax.grid(True, alpha=0.25)
    ax.legend()
    fig.tight_layout()
    fig.savefig(PLOTS / "PERSISTENCE_BUDGET_CALIBRATION_CLOUD.png", dpi=180)
    plt.close(fig)

    # Selected pathwise prefix budget.
    probes = pd.DataFrame(probe_payloads[selected_key])
    if not probes.empty:
        probes = probes.copy()
        probes["cum_prefix"] = probes["actual_prefix_wall"].cumsum()
        # This cross-trajectory plot is diagnostic only; per-trajectory budget is enforced in simulation.
        fig, ax = plt.subplots(figsize=(7.2, 4.8))
        ax.bar(np.arange(len(probes)), 100 * probes["actual_prefix_wall"] / probes["rwall_decision"])
        ax.axhline(100 * bound_ratio, linestyle="--", label="prospective bound ratio")
        ax.set_xlabel("Selected probe ordinal")
        ax.set_ylabel("Actual next-state prefix / preceding R wall (%)")
        ax.set_title("Selected calibration probes: realized prefix scale")
        ax.grid(True, axis="y", alpha=0.25)
        ax.legend()
        fig.tight_layout()
        fig.savefig(PLOTS / "SELECTED_PREFIX_SCALE.png", dpi=180)
        plt.close(fig)

    summary = {
        "schema": "gvjf-policy-redesign-v25-calibration128-selection-v1",
        "contract_sha256": sha256(CONTRACT),
        "rjf_parity": parity,
        "rjf_rows": len(new),
        "rjf_trajectories": len(traj),
        "historical_optimized_atlas_sha256": sha256(HIST_AGG),
        "timing_authority": "historical optimized G4-S5B0 medians only",
        "debug_wall_used_for_selection": False,
        "causal_alignment": "R row j -> E prefix/audit row j+1",
        "prefix_bound_ratio": bound_ratio,
        "thresholds": {f: {str(q): v for q, v in qs.items()} for f, qs in thresholds.items()},
        "candidate_count": len(frame),
        "valid_candidate_count": len(valid),
        "strong_r_to_e_segment_count": len(segments),
        "selected_policy": {
            key: (bool(value) if isinstance(value, (np.bool_, bool)) else
                  int(value) if isinstance(value, (np.integer,)) else
                  float(value) if isinstance(value, (np.floating,)) else value)
            for key, value in selected.to_dict().items()
        },
        "e_k_continuations": 0,
        "active_switching": False,
        "n512_status": "not yet evaluated in this coding loop; cross-dimension regression only",
    }
    (OUT / "CALIBRATION_SELECTION.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
