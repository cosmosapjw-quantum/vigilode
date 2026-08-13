#!/usr/bin/env python3
"""Read-only G4-S0 regime telemetry and offline oracle map.

This script consumes the seven frozen G3 campaigns.  It does not rerun, retune,
or modify any candidate integrator.  The protected JF RODAS5P endpoint error at
matching problem/rtol is the external target for each cell.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import random
import statistics
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

STRICT_PORTFOLIO = (
    "protected-jf-rodas5p",
    "pexprb54s4-fused-full-mgs-t1",
    "held-g1-q1-q2",
)
AUTO_PORTFOLIO = STRICT_PORTFOLIO + (
    "frozen-radau-iia3",
    "frozen-bdf2",
)
BASELINE = "protected-jf-rodas5p"
PRIMARY_EXP = "pexprb54s4-fused-full-mgs-t1"

PROBLEM_SPANS = {
    "quadratic": 0.25,
    "complex-dahlquist": 0.2,
    "oscillatory-pr": 0.2,
    "advection-diffusion": 0.1,
}


def median(values: Iterable[float]) -> float | None:
    values = sorted(float(v) for v in values)
    if not values:
        return None
    return statistics.median(values)


def quantile(values: Iterable[float], q: float) -> float | None:
    values = sorted(float(v) for v in values)
    if not values:
        return None
    if len(values) == 1:
        return values[0]
    x = q * (len(values) - 1)
    lo = int(math.floor(x))
    hi = int(math.ceil(x))
    if lo == hi:
        return values[lo]
    w = x - lo
    return (1.0 - w) * values[lo] + w * values[hi]


def stable_json_sha(payload: Any) -> str:
    raw = json.dumps(payload, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    return hashlib.sha256(raw).hexdigest()



def scientific_projection(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: scientific_projection(item)
            for key, item in sorted(value.items())
            if "wall" not in key.lower() and key != "failure"
        }
    if isinstance(value, list):
        return [scientific_projection(item) for item in value]
    return value


def scientific_digest(value: Any) -> str:
    return stable_json_sha(scientific_projection(value))


def load_campaigns(campaign_dir: Path) -> list[dict[str, Any]]:
    files = sorted(campaign_dir.glob("G3_CANONICAL_[1-7].json"))
    if len(files) != 7:
        raise RuntimeError(f"expected 7 frozen campaigns, found {len(files)}")
    out = []
    for path in files:
        doc = json.loads(path.read_text())
        if doc.get("schema") != "generic-parallel-exponential-g3-v1":
            raise RuntimeError(f"unexpected schema in {path}")
        out.append({"index": int(path.stem.rsplit("_", 1)[1]), "path": str(path), "doc": doc})
    return out


def aggregate_rows(campaigns: list[dict[str, Any]]) -> tuple[dict, list[dict[str, Any]]]:
    grouped: dict[tuple[str, str, float], list[dict[str, Any]]] = defaultdict(list)
    all_rows = []
    for campaign in campaigns:
        for row in campaign["doc"]["adaptive_rows"]:
            item = dict(row)
            item["campaign"] = campaign["index"]
            grouped[(row["problem_id"], row["candidate_id"], float(row["rtol"]))].append(item)
            all_rows.append(item)

    aggregated: dict[tuple[str, str, float], dict[str, Any]] = {}
    for key, rows in grouped.items():
        def md(field: str) -> float | None:
            return median(row[field] for row in rows if row.get(field) is not None)

        work_fields = rows[0]["work"].keys()
        aggregated[key] = {
            "problem_id": key[0],
            "candidate_id": key[1],
            "rtol": key[2],
            "atol": median(row["atol"] for row in rows),
            "success": all(bool(row["success"]) for row in rows),
            "successes": sum(bool(row["success"]) for row in rows),
            "campaigns": len(rows),
            "wall_median": md("wall_seconds"),
            "wall_q25": quantile((row["wall_seconds"] for row in rows), 0.25),
            "wall_q75": quantile((row["wall_seconds"] for row in rows), 0.75),
            "endpoint_error": md("endpoint_l2_error"),
            "accepted_steps": md("accepted_steps"),
            "rejected_steps": md("rejected_steps"),
            "maximum_time_error": md("maximum_time_error"),
            "maximum_phi_error": md("maximum_phi_error"),
            "maximum_total_error": md("maximum_total_error"),
            "work": {field: median(row["work"][field] for row in rows) for field in work_fields},
        }
    return aggregated, all_rows


def target_feasible(row: dict[str, Any], target: float, multiplier: float = 1.0) -> bool:
    err = row.get("endpoint_error")
    return bool(row.get("success")) and err is not None and err <= multiplier * target * (1.0 + 1.0e-12) + 1.0e-30


def oracle_rows(aggregated: dict, portfolio_name: str, portfolio: tuple[str, ...], target_multiplier: float = 1.0) -> list[dict[str, Any]]:
    """Build work-precision oracle rows across each candidate's full tolerance ladder.

    Each target is one protected-RODAS error level.  A candidate may satisfy that
    target with any of its configured internal tolerances; the cheapest eligible
    run is used, matching the frozen G3 work-precision authority.
    """
    problems = sorted({key[0] for key in aggregated})
    out = []
    for problem in problems:
        baseline_pool = [
            row for (p, c, _), row in aggregated.items()
            if p == problem and c == BASELINE and row["success"]
        ]
        baseline_targets = sorted(baseline_pool, key=lambda row: row["endpoint_error"], reverse=True)
        for target_index, target_row in enumerate(baseline_targets, start=1):
            target = float(target_row["endpoint_error"]) * target_multiplier
            baseline_eligible = [row for row in baseline_pool if target_feasible(row, target)]
            if not baseline_eligible:
                # Can occur only for a target multiplier < 1.  Keep the target but
                # mark the portfolio uncovered rather than silently changing it.
                baseline_best = None
            else:
                baseline_best = min(baseline_eligible, key=lambda row: row["wall_median"])
            feasible = []
            for candidate in portfolio:
                candidate_pool = [
                    row for (p, c, _), row in aggregated.items()
                    if p == problem and c == candidate and row["success"]
                ]
                eligible = [row for row in candidate_pool if target_feasible(row, target)]
                if eligible:
                    best = min(eligible, key=lambda row: row["wall_median"])
                    feasible.append({
                        "candidate_id": candidate,
                        "selected_rtol": best["rtol"],
                        "wall_seconds": best["wall_median"],
                        "endpoint_error": best["endpoint_error"],
                        "accepted_steps": best["accepted_steps"],
                    })
            feasible.sort(key=lambda x: (x["wall_seconds"], x["candidate_id"]))
            winner = feasible[0] if feasible else None
            second = feasible[1] if len(feasible) > 1 else None
            out.append({
                "portfolio": portfolio_name,
                "problem_id": problem,
                "target_index": target_index,
                "rtol": target_row["rtol"],
                "target_error": target,
                "baseline_selected_rtol": None if baseline_best is None else baseline_best["rtol"],
                "baseline_wall_seconds": None if baseline_best is None else baseline_best["wall_median"],
                "feasible_candidates": [x["candidate_id"] for x in feasible],
                "winner": None if winner is None else winner["candidate_id"],
                "winner_selected_rtol": None if winner is None else winner["selected_rtol"],
                "oracle_wall_seconds": None if winner is None else winner["wall_seconds"],
                "oracle_speedup_vs_rodas": None if winner is None or baseline_best is None else baseline_best["wall_median"] / winner["wall_seconds"],
                "winner_margin_second_over_first": None if second is None else second["wall_seconds"] / winner["wall_seconds"],
            })
    return out


def campaign_winner_stability(campaigns: list[dict[str, Any]], median_oracle: list[dict[str, Any]], portfolio: tuple[str, ...]) -> list[dict[str, Any]]:
    by_cell = {(row["problem_id"], int(row["target_index"])): row for row in median_oracle}
    counts: dict[tuple[str, int], Counter[str]] = defaultdict(Counter)
    for campaign in campaigns:
        rows = campaign["doc"]["adaptive_rows"]
        problems = sorted({row["problem_id"] for row in rows})
        for problem in problems:
            baseline_pool = [row for row in rows if row["problem_id"] == problem and row["candidate_id"] == BASELINE and row["success"]]
            targets = sorted(baseline_pool, key=lambda row: row["endpoint_l2_error"], reverse=True)
            for target_index, target_row in enumerate(targets, start=1):
                target = float(target_row["endpoint_l2_error"])
                feasible = []
                for candidate in portfolio:
                    pool = [row for row in rows if row["problem_id"] == problem and row["candidate_id"] == candidate and row["success"]]
                    eligible = [row for row in pool if row["endpoint_l2_error"] is not None and row["endpoint_l2_error"] <= target * (1.0 + 1.0e-12) + 1e-30]
                    if eligible:
                        best = min(eligible, key=lambda row: row["wall_seconds"] )
                        feasible.append((float(best["wall_seconds"]), candidate))
                if feasible:
                    feasible.sort()
                    counts[(problem, target_index)][feasible[0][1]] += 1
    out = []
    for key in sorted(counts):
        median_row = by_cell[key]
        median_winner = median_row["winner"]
        winner_count = counts[key][median_winner]
        out.append({
            "problem_id": key[0],
            "target_index": key[1],
            "rtol": median_row["rtol"],
            "median_winner": median_winner,
            "campaign_winner_counts": dict(sorted(counts[key].items())),
            "median_winner_fraction": winner_count / len(campaigns),
        })
    return out


def load_jvp_features(campaigns: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    rows = campaigns[0]["doc"]["fresh_jvp_rows"]
    # G3 scientific identity means these are exact across all campaigns.
    return {row["problem_id"]: dict(row) for row in rows}


def feature_rows(aggregated: dict, strict_oracle: list[dict[str, Any]], jvp: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    out = []
    for decision in strict_oracle:
        problem = decision["problem_id"]
        target = float(decision["target_error"] )
        baseline_pool = [row for (p, c, _), row in aggregated.items() if p == problem and c == BASELINE and row["success"]]
        baseline = min([row for row in baseline_pool if target_feasible(row, target)], key=lambda row: row["wall_median"] )
        exp_pool = [row for (p, c, _), row in aggregated.items() if p == problem and c == PRIMARY_EXP and row["success"]]
        exp_eligible = [row for row in exp_pool if target_feasible(row, target)]
        exp_feasible = bool(exp_eligible)
        exp_row = min(exp_eligible, key=lambda row: row["wall_median"]) if exp_eligible else min(exp_pool, key=lambda row: row["endpoint_error"] )
        accepted = max(float(exp_row["accepted_steps"]), 1.0)
        attempts = max(float(exp_row["accepted_steps"] + exp_row["rejected_steps"]), 1.0)
        phi = float(exp_row["maximum_phi_error"] or 0.0)
        time_error = float(exp_row["maximum_time_error"] or 0.0)
        work = exp_row["work"]
        jvp_row = jvp[problem]
        out.append({
            "problem_id": problem,
            "target_index": int(decision["target_index"]),
            "rtol": decision["rtol"],
            "target_error": target,
            "dimension": int(jvp_row["dimension"]),
            "strict_oracle_winner": decision["winner"],
            "exp_is_oracle_winner": decision["winner"] == PRIMARY_EXP,
            "exp_target_feasible": exp_feasible,
            "exp_selected_rtol": exp_row["rtol"],
            "exp_same_error_speedup_vs_rodas": float(baseline["wall_median"]) / float(exp_row["wall_median"]),
            "exp_endpoint_error_ratio_to_target": float(exp_row["endpoint_error"]) / max(target, 1e-300),
            "exp_step_ratio_to_rodas": float(exp_row["accepted_steps"]) / max(float(baseline["accepted_steps"]), 1.0),
            "exp_rejection_fraction": float(exp_row["rejected_steps"]) / attempts,
            "exp_mean_step_fraction_of_span": 1.0 / accepted,
            "exp_krylov_vectors_per_accepted_step": float(work["phi_krylov_vectors"]) / accepted,
            "exp_jvp_vectors_per_accepted_step": float(work["jvp_vectors"]) / accepted,
            "exp_phi_actions_per_accepted_step": float(work["phi_actions"]) / accepted,
            "exp_phi_restarts_per_accepted_step": float(work["phi_restarts"]) / accepted,
            "exp_maximum_time_error": time_error,
            "exp_maximum_phi_error": phi,
            "exp_log10_time_error": math.log10(max(time_error, 1e-300)),
            "exp_log10_phi_error": math.log10(max(phi, 1e-300)),
            "fresh_jvp_half_step_disagreement": float(jvp_row["half_step_disagreement"]),
            "fresh_jvp_relative_error": float(jvp_row["relative_error_vs_supplied_jvp"]),
            "fresh_jvp_probe_rhs_equivalent_calls": int(jvp_row["rhs_equivalent_calls"]),
            "jvp_probe_fraction_of_rodas_rhs": int(jvp_row["rhs_equivalent_calls"]) / max(float(baseline["work"]["rhs_evaluations"]), 1.0),
        })
    return out


def best_threshold(rows: list[dict[str, Any]], feature: str, training_problems: set[str] | None = None) -> dict[str, Any]:
    train = [r for r in rows if training_problems is None or r["problem_id"] in training_problems]
    values = sorted({float(r[feature]) for r in train})
    thresholds = []
    if values:
        thresholds = [values[0] - 1.0]
        thresholds += [(a + b) / 2.0 for a, b in zip(values, values[1:])]
        thresholds += [values[-1] + 1.0]
    best = None
    for polarity in ("le", "ge"):
        for threshold in thresholds:
            correct = 0
            unsafe_exp = 0
            for row in train:
                pred_exp = float(row[feature]) <= threshold if polarity == "le" else float(row[feature]) >= threshold
                truth_exp = bool(row["exp_is_oracle_winner"])
                correct += pred_exp == truth_exp
                unsafe_exp += pred_exp and not row["exp_target_feasible"]
            score = (correct, -unsafe_exp, -abs(threshold))
            candidate = {
                "feature": feature,
                "polarity": polarity,
                "threshold": threshold,
                "training_accuracy": correct / len(train),
                "training_unsafe_exp_selections": unsafe_exp,
                "score": score,
            }
            if best is None or score > best["score"]:
                best = candidate
    assert best is not None
    best.pop("score")
    return best


def evaluate_rule(rows: list[dict[str, Any]], rule: dict[str, Any], problems: set[str]) -> dict[str, Any]:
    test = [r for r in rows if r["problem_id"] in problems]
    correct = 0
    unsafe = 0
    selections = Counter()
    for row in test:
        value = float(row[rule["feature"]])
        pred_exp = value <= rule["threshold"] if rule["polarity"] == "le" else value >= rule["threshold"]
        truth = bool(row["exp_is_oracle_winner"])
        correct += pred_exp == truth
        unsafe += pred_exp and not row["exp_target_feasible"]
        selections["exp" if pred_exp else "rodas"] += 1
    return {
        "rows": len(test),
        "accuracy": correct / len(test) if test else None,
        "unsafe_exp_selections": unsafe,
        "selections": dict(selections),
    }


def univariate_feature_audit(rows: list[dict[str, Any]]) -> dict[str, Any]:
    features = [
        "dimension",
        "rtol",
        "exp_rejection_fraction",
        "exp_krylov_vectors_per_accepted_step",
        "exp_jvp_vectors_per_accepted_step",
        "exp_phi_restarts_per_accepted_step",
        "exp_log10_time_error",
        "exp_log10_phi_error",
        "fresh_jvp_half_step_disagreement",
        "fresh_jvp_relative_error",
    ]
    full = []
    for feature in features:
        rule = best_threshold(rows, feature)
        eval_all = evaluate_rule(rows, rule, {r["problem_id"] for r in rows})
        full.append({**rule, **{f"full_{k}": v for k, v in eval_all.items()}})

    problems = sorted({r["problem_id"] for r in rows})
    lopo = []
    for feature in features:
        folds = []
        for holdout in problems:
            train_problems = set(problems) - {holdout}
            rule = best_threshold(rows, feature, train_problems)
            test = evaluate_rule(rows, rule, {holdout})
            folds.append({"holdout_problem": holdout, "rule": rule, "test": test})
        lopo.append({
            "feature": feature,
            "mean_holdout_accuracy": statistics.mean(f["test"]["accuracy"] for f in folds),
            "total_unsafe_exp_selections": sum(f["test"]["unsafe_exp_selections"] for f in folds),
            "folds": folds,
        })
    full.sort(key=lambda x: (-x["full_accuracy"], x["full_unsafe_exp_selections"], x["feature"]))
    lopo.sort(key=lambda x: (-x["mean_holdout_accuracy"], x["total_unsafe_exp_selections"], x["feature"]))
    return {"full_fit": full, "leave_one_problem_out": lopo}


def timing_mutation_stability(aggregated: dict, oracle: list[dict[str, Any]], portfolio: tuple[str, ...], fraction: float, draws: int = 1000) -> list[dict[str, Any]]:
    rng = random.Random(20260814 + int(1000 * fraction))
    out = []
    for decision in oracle:
        problem = decision["problem_id"]
        target = float(decision["target_error"] )
        nominal = decision["winner"]
        counts = Counter()
        for _ in range(draws):
            feasible = []
            for candidate in portfolio:
                pool = [row for (p, c, _), row in aggregated.items() if p == problem and c == candidate and row["success"]]
                eligible = [row for row in pool if target_feasible(row, target)]
                if eligible:
                    best_wall = min(float(row["wall_median"]) * (1.0 + rng.uniform(-fraction, fraction)) for row in eligible)
                    feasible.append((best_wall, candidate))
            if feasible:
                feasible.sort()
                counts[feasible[0][1]] += 1
        out.append({
            "problem_id": problem,
            "target_index": decision["target_index"],
            "rtol": decision["rtol"],
            "nominal_winner": nominal,
            "mutation_fraction": fraction,
            "nominal_winner_fraction": counts[nominal] / draws if nominal else 0.0,
            "winner_counts": dict(sorted(counts.items())),
        })
    return out


def static_candidate_summary(aggregated: dict, portfolio: tuple[str, ...]) -> dict[str, Any]:
    problems = sorted({key[0] for key in aggregated})
    rows = []
    total_targets = sum(len([1 for (p, c, _), row in aggregated.items() if p == problem and c == BASELINE and row["success"]]) for problem in problems)
    for candidate in portfolio:
        coverage = 0
        total_wall = 0.0
        for problem in problems:
            baseline_pool = [row for (p, c, _), row in aggregated.items() if p == problem and c == BASELINE and row["success"]]
            targets = sorted(baseline_pool, key=lambda row: row["endpoint_error"], reverse=True)
            candidate_pool = [row for (p, c, _), row in aggregated.items() if p == problem and c == candidate and row["success"]]
            for target_row in targets:
                target = float(target_row["endpoint_error"] )
                eligible = [row for row in candidate_pool if target_feasible(row, target)]
                if eligible:
                    coverage += 1
                    total_wall += min(float(row["wall_median"]) for row in eligible)
        rows.append({
            "candidate_id": candidate,
            "coverage": coverage,
            "total_cells": total_targets,
            "sum_wall_seconds_on_covered_cells": total_wall,
        })
    full = [row for row in rows if row["coverage"] == row["total_cells"]]
    full.sort(key=lambda row: (row["sum_wall_seconds_on_covered_cells"], row["candidate_id"]))
    return {
        "candidates": rows,
        "best_full_coverage_candidate": full[0] if full else None,
    }


def portfolio_summary(oracle: list[dict[str, Any]]) -> dict[str, Any]:
    complete = [row for row in oracle if row["winner"] is not None]
    baseline_sum = sum(row["baseline_wall_seconds"] for row in complete)
    oracle_sum = sum(row["oracle_wall_seconds"] for row in complete)
    speedups = [row["oracle_speedup_vs_rodas"] for row in complete]
    return {
        "cells": len(oracle),
        "covered_cells": len(complete),
        "winner_counts": dict(sorted(Counter(row["winner"] for row in complete).items())),
        "switch_opportunity_cells": sum(row["winner"] != BASELINE for row in complete),
        "aggregate_sum_cost_speedup_vs_rodas": baseline_sum / oracle_sum,
        "median_cell_speedup_vs_rodas": median(speedups),
        "minimum_cell_speedup_vs_rodas": min(speedups),
        "maximum_cell_speedup_vs_rodas": max(speedups),
        "median_second_best_margin": median(row["winner_margin_second_over_first"] for row in complete if row["winner_margin_second_over_first"] is not None),
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        path.write_text("")
        return
    fields = sorted({key for row in rows for key in row})
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--campaign-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)

    campaigns = load_campaigns(args.campaign_dir)
    aggregated, _ = aggregate_rows(campaigns)
    strict = oracle_rows(aggregated, "strict-jf", STRICT_PORTFOLIO)
    auto = oracle_rows(aggregated, "auto", AUTO_PORTFOLIO)
    jvp = load_jvp_features(campaigns)
    features = feature_rows(aggregated, strict, jvp)
    feature_audit = univariate_feature_audit(features)

    target_sensitivity = {}
    for multiplier in (1.0, 1.05, 1.2):
        target_sensitivity[str(multiplier)] = {
            "strict": portfolio_summary(oracle_rows(aggregated, "strict-jf", STRICT_PORTFOLIO, multiplier)),
            "auto": portfolio_summary(oracle_rows(aggregated, "auto", AUTO_PORTFOLIO, multiplier)),
        }

    report = {
        "schema": "generic-regime-telemetry-g4-s0-v1",
        "status": "pass-read-only-oracle-map",
        "source": {
            "campaign_count": len(campaigns),
            "campaign_scientific_digests": [scientific_digest(c["doc"]) for c in campaigns],
            "campaign_files": [Path(c["path"]).name for c in campaigns],
            "candidate_algorithms_modified": False,
            "switching_activated": False,
        },
        "portfolios": {
            "strict-jf": list(STRICT_PORTFOLIO),
            "auto": list(AUTO_PORTFOLIO),
        },
        "strict_oracle": strict,
        "auto_oracle": auto,
        "strict_summary": portfolio_summary(strict),
        "auto_summary": portfolio_summary(auto),
        "strict_static_candidates": static_candidate_summary(aggregated, STRICT_PORTFOLIO),
        "auto_static_candidates": static_candidate_summary(aggregated, AUTO_PORTFOLIO),
        "strict_campaign_winner_stability": campaign_winner_stability(campaigns, strict, STRICT_PORTFOLIO),
        "auto_campaign_winner_stability": campaign_winner_stability(campaigns, auto, AUTO_PORTFOLIO),
        "feature_rows": features,
        "univariate_feature_audit": feature_audit,
        "timing_mutation": {
            "strict_5pct": timing_mutation_stability(aggregated, strict, STRICT_PORTFOLIO, 0.05),
            "strict_10pct": timing_mutation_stability(aggregated, strict, STRICT_PORTFOLIO, 0.10),
            "auto_5pct": timing_mutation_stability(aggregated, auto, AUTO_PORTFOLIO, 0.05),
            "auto_10pct": timing_mutation_stability(aggregated, auto, AUTO_PORTFOLIO, 0.10),
        },
        "target_sensitivity": target_sensitivity,
        "detector_overhead": {
            "free_features": [
                "accepted/rejected step counts",
                "embedded time-error telemetry",
                "phi residual telemetry",
                "Krylov vectors/restarts/orthogonalization",
            ],
            "sampled_feature": "fresh JVP epsilon-vs-half-epsilon comparison",
            "median_rhs_fraction_vs_protected_rodas": median(r["jvp_probe_fraction_of_rodas_rhs"] for r in features),
            "maximum_rhs_fraction_vs_protected_rodas": max(r["jvp_probe_fraction_of_rodas_rhs"] for r in features),
            "policy": "sample only on mode entry, repeated rejection, or cooldown expiration",
        },
        "claim": {
            "oracle_complementarity_exists": portfolio_summary(strict)["switch_opportunity_cells"] > 0,
            "single_free_feature_is_sufficient": False,
            "active_switching_authorized": False,
            "next_required_measurement": "accepted-step-local shadow telemetry with cheap nonlinear-defect probe and switch/setup cost",
        },
    }
    report["scientific_sha256"] = stable_json_sha({k: v for k, v in report.items() if k != "timing_mutation"})

    (args.output_dir / "G4_S0_ORACLE_MAP.json").write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n")
    write_csv(args.output_dir / "G4_S0_STRICT_ORACLE.csv", strict)
    write_csv(args.output_dir / "G4_S0_AUTO_ORACLE.csv", auto)
    write_csv(args.output_dir / "G4_S0_FEATURE_ROWS.csv", features)
    print(json.dumps({
        "status": report["status"],
        "strict_summary": report["strict_summary"],
        "auto_summary": report["auto_summary"],
        "best_full_feature": feature_audit["full_fit"][0],
        "best_lopo_feature": feature_audit["leave_one_problem_out"][0],
        "scientific_sha256": report["scientific_sha256"],
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
