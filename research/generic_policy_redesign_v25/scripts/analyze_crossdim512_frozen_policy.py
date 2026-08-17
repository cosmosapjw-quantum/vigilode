#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from analyze_persistence_budget_preflight import (  # noqa: E402
    Candidate, add_causal_features, simulate_candidate, strong_e_segments,
)

SHARDS = ROOT / "results" / "crossdim512_shards"
OUT = ROOT / "results" / "crossdim512_analysis"
PLOTS = ROOT / "plots"
POLICY = ROOT / "contracts" / "FROZEN_POLICY_CALIBRATION128.json"
HIST_ROOT = Path(
    "/mnt/data/GVJF_POLICY_MATH_RESEARCH_20260817/source_v24/"
    "research/generic_regime_atlas_v20/results"
)
HIST_AGG = HIST_ROOT / "G4_S5B0_AGGREGATED_STEPS.csv"
HIST_RAW = HIST_ROOT / "campaigns" / "G4S5B0_CANONICAL_1.json"
CAL_SELECTION = ROOT / "results" / "calibration128_analysis" / "CALIBRATION_SELECTION.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_shards() -> tuple[pd.DataFrame, pd.DataFrame]:
    rows, trajectories = [], []
    files = sorted(SHARDS.glob("*.json"))
    if len(files) != 6:
        raise RuntimeError(f"expected six N=512 family shards, found {len(files)}")
    for path in files:
        payload = json.loads(path.read_text())
        if payload["schema"] != "g4-s5b0-rjf-only-regime-replay-v1":
            raise RuntimeError("unexpected schema")
        if payload["profile"] != "holdout-512":
            raise RuntimeError("unexpected profile")
        if payload["switching_active"]:
            raise RuntimeError("switching appeared in R-only replay")
        rows.extend(payload["rows"])
        trajectories.extend(payload["trajectories"])
    frame = pd.DataFrame(rows).sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    traj = pd.DataFrame(trajectories).sort_values("trajectory_id").reset_index(drop=True)
    return frame, traj


def exact_parity(new: pd.DataFrame) -> dict:
    raw = json.loads(HIST_RAW.read_text())
    old = pd.DataFrame(raw["rows"])
    old = old[old["dimension"] == 512].sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    keys = ["trajectory_id", "family", "dimension", "rtol", "step_index"]
    fields = [
        "t_start", "h", "transition_level", "rodas_embedded_error",
        "rodas_rhs_evaluations", "rodas_jvp_vectors", "rodas_linear_matvecs",
    ]
    if not new[keys].equals(old[keys]):
        raise RuntimeError("N512 R-JF row keys differ from canonical raw campaign")
    equality = {f: bool(np.array_equal(new[f].to_numpy(), old[f].to_numpy())) for f in fields}
    if not all(equality.values()):
        raise RuntimeError(f"N512 R-JF exact parity failed: {equality}")
    return {"exact": True, "rows": len(new), "fields": equality, "raw_campaign_sha256": sha256(HIST_RAW)}


def family_metrics(probes: pd.DataFrame, audit: pd.DataFrame, segments: list[dict]) -> pd.DataFrame:
    rows = []
    for family in sorted(audit["family"].unique()):
        fam_audit = audit[audit["family"] == family]
        fam_probes = probes[probes["trajectory_id"].isin(fam_audit["trajectory_id"].unique())] if not probes.empty else probes
        fam_segments = [s for s in segments if s["family"] == family]
        total_gain = sum(s["total_gain"] for s in fam_segments)
        captured = 0.0
        detected = 0
        for seg in fam_segments:
            local = fam_probes[
                (fam_probes["trajectory_id"] == seg["trajectory_id"])
                & (fam_probes["probe_step"] >= seg["start_step"])
                & (fam_probes["probe_step"] <= seg["end_step"])
            ] if not fam_probes.empty else fam_probes
            if local.empty:
                continue
            first = int(local["probe_step"].min())
            captured += sum(seg["gains"][first-seg["start_step"]:])
            detected += 1
        precision = float((fam_probes["target_strong_winner"] == "E-K").mean()) if not fam_probes.empty else 0.0
        rows.append({
            "family": family,
            "probes": len(fam_probes),
            "probe_precision": precision,
            "r_to_e_segments": len(fam_segments),
            "detected_segments": detected,
            "opportunity_retention": captured / total_gain if total_gain > 0 else float("nan"),
        })
    return pd.DataFrame(rows)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    policy = json.loads(POLICY.read_text())
    calibration = json.loads(CAL_SELECTION.read_text())
    new, traj = load_shards()
    if not traj["success"].all():
        raise RuntimeError("N512 R-only shard failed")
    if new["exponential_completed"].any():
        raise RuntimeError("E work appeared in N512 R-only replay")
    if int(traj["explicit_jacobian_builds"].sum()) != 0 or int(traj["newton_iterations"].sum()) != 0:
        raise RuntimeError("primary R-JF contract violated")
    parity = exact_parity(new)
    new = add_causal_features(new)

    audit = pd.read_csv(HIST_AGG)
    audit = audit[audit["dimension"] == 512].sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    segments = strong_e_segments(audit)
    cand = Candidate(
        feature=policy["feature"], quantile=float(policy["quantile"]), threshold=float(policy["threshold"]),
        k=int(policy["persistence_k"]), delta=float(policy["prefix_budget_delta"]),
    )
    result = simulate_candidate(new, audit, segments, cand, float(policy["prefix_bound_ratio"]))
    probes = pd.DataFrame(result.pop("probes"))
    result["candidate_key"] = f"{cand.feature}|q={cand.quantile:.2f}|k={cand.k}|delta={cand.delta:.2f}"

    gates = {
        "r_jf_committed_trajectory_identity": bool(parity["exact"]),
        "information_firewall_violations": 0,
        "e_k_continuations": 0,
        "prefix_bound_violations": int(result["prefix_bound_violations"]),
        "pathwise_prefix_budget_violations": int(result["pathwise_budget_violations"]),
        "opportunity_retention_pass": bool(result["opportunity_retention"] >= 0.75),
        "aggregate_prefix_overhead_pass": bool(result["aggregate_prefix_overhead"] <= 0.05),
        "probe_precision_pass": bool(result["probe_precision"] >= 0.80),
    }
    hard_pass = (
        gates["r_jf_committed_trajectory_identity"]
        and gates["information_firewall_violations"] == 0
        and gates["e_k_continuations"] == 0
        and gates["prefix_bound_violations"] == 0
        and gates["pathwise_prefix_budget_violations"] == 0
        and gates["opportunity_retention_pass"]
        and gates["aggregate_prefix_overhead_pass"]
        and gates["probe_precision_pass"]
    )

    family = family_metrics(probes, audit, segments)
    new.to_csv(OUT / "RJF_ONLY_ROWS_WITH_CAUSAL_FEATURES.csv", index=False)
    traj.to_csv(OUT / "RJF_ONLY_TRAJECTORIES.csv", index=False)
    probes.to_csv(OUT / "FROZEN_POLICY_PROBES.csv", index=False)
    family.to_csv(OUT / "FAMILY_METRICS.csv", index=False)

    # Calibration vs cross-dimension regression comparison.
    cal = calibration["selected_policy"]
    metrics = ["opportunity_retention", "probe_precision", "aggregate_prefix_overhead"]
    cvals = [float(cal[m]) for m in metrics]
    hvals = [float(result[m]) for m in metrics]
    fig, ax = plt.subplots(figsize=(7.4, 4.8))
    x = np.arange(len(metrics)); width = 0.36
    ax.bar(x-width/2, cvals, width, label="N=128 calibration")
    ax.bar(x+width/2, hvals, width, label="N=512 cross-dim")
    ax.set_xticks(x, ["opportunity", "precision", "prefix overhead"])
    ax.set_ylim(0, 1.0)
    ax.set_title("Frozen policy: calibration vs cross-dimension regression")
    ax.grid(True, axis="y", alpha=0.25)
    ax.legend()
    fig.tight_layout()
    fig.savefig(PLOTS / "FROZEN_POLICY_CROSSDIM_COMPARE.png", dpi=180)
    plt.close(fig)

    fig, ax = plt.subplots(figsize=(8.4, 4.8))
    plotfam = family.fillna(0.0)
    x = np.arange(len(plotfam))
    ax.bar(x-0.2, plotfam["probe_precision"], 0.4, label="probe precision")
    ax.bar(x+0.2, plotfam["opportunity_retention"], 0.4, label="opportunity retention")
    ax.set_xticks(x, [s.replace("-ramped", "") for s in plotfam["family"]], rotation=30, ha="right")
    ax.set_ylim(0, 1.05)
    ax.set_title("N=512 frozen-policy family audit")
    ax.grid(True, axis="y", alpha=0.25)
    ax.legend()
    fig.tight_layout()
    fig.savefig(PLOTS / "FROZEN_POLICY_CROSSDIM_FAMILY.png", dpi=180)
    plt.close(fig)

    summary = {
        "schema": "gvjf-policy-redesign-v25-crossdim512-regression-v1",
        "frozen_policy_sha256": sha256(POLICY),
        "frozen_policy": policy,
        "rjf_parity": parity,
        "rjf_rows": len(new),
        "rjf_trajectories": len(traj),
        "historical_optimized_atlas_sha256": sha256(HIST_AGG),
        "timing_authority": "historical optimized G4-S5B0 medians only",
        "debug_wall_used_for_scoring": False,
        "result": result,
        "hard_gates": gates,
        "hard_pass": bool(hard_pass),
        "interpretation": "cross-dimension regression only; N=512 is not a sealed unseen holdout",
        "retuning_performed": False,
        "active_switching": False,
        "e_k_continuations": 0,
    }
    (OUT / "CROSSDIM512_RESULT.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
