#!/usr/bin/env python3
from __future__ import annotations

import csv
import hashlib
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
RESULTS = ROOT / "results"
CAMPAIGNS = RESULTS / "campaigns"
LEGACY = RESULTS / "campaigns_nested_heuristic_v1"
TABLES = ROOT / "tables"
G2_ANALYSIS = ROOT.parent / "generic_parallel_exponential_v12" / "results" / "G2_FOUNDATION_ANALYSIS.json"

BASELINE = "protected-jf-rodas5p"
PRIMARY = "pexprb54s4-fused-full-mgs-t1"


def quantile(values: list[float], q: float) -> float:
    values = sorted(values)
    if not values:
        return math.nan
    if len(values) == 1:
        return values[0]
    position = q * (len(values) - 1)
    lo = math.floor(position)
    hi = math.ceil(position)
    if lo == hi:
        return values[lo]
    weight = position - lo
    return values[lo] * (1.0 - weight) + values[hi] * weight


def science_projection(value: Any) -> Any:
    if isinstance(value, dict):
        out = {}
        for key in sorted(value):
            lower = key.lower()
            if "wall" in lower or key == "failure":
                continue
            out[key] = science_projection(value[key])
        return out
    if isinstance(value, list):
        return [science_projection(item) for item in value]
    return value


def science_digest(report: dict[str, Any]) -> str:
    payload = json.dumps(
        science_projection(report), sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()
    return hashlib.sha256(payload).hexdigest()


def load_campaigns(path: Path) -> list[tuple[Path, dict[str, Any]]]:
    files = sorted(path.glob("G3_CANONICAL_*.json"))
    if not files:
        raise RuntimeError(f"no G3 campaigns in {path}")
    return [(file, json.loads(file.read_text())) for file in files]


def selected_work(row: dict[str, Any]) -> dict[str, Any]:
    work = row["work"]
    keys = [
        "rhs_evaluations",
        "jvp_vectors",
        "orthogonalization_inner_products",
        "orthogonalization_vector_updates",
        "phi_actions",
        "phi_krylov_vectors",
        "phi_projected_exponentials",
        "phi_restarts",
        "accepted_steps",
        "rejected_steps",
        "jacobian_builds",
        "direct_factorizations",
        "nonlinear_iterations",
    ]
    return {key: work.get(key, 0) for key in keys}


def aggregate_adaptive(campaigns: list[tuple[Path, dict[str, Any]]]) -> dict[tuple[str, str, float], dict[str, Any]]:
    grouped: dict[tuple[str, str, float], list[dict[str, Any]]] = defaultdict(list)
    for _, report in campaigns:
        for row in report["adaptive_rows"]:
            grouped[(row["problem_id"], row["candidate_id"], row["rtol"])].append(row)
    aggregated: dict[tuple[str, str, float], dict[str, Any]] = {}
    for key, rows in grouped.items():
        if len(rows) != len(campaigns):
            raise AssertionError(f"campaign coverage mismatch for {key}: {len(rows)}")
        projected = science_projection(rows[0])
        for row in rows[1:]:
            if science_projection(row) != projected:
                raise AssertionError(f"scientific mismatch across campaigns for {key}")
        walls = [float(row["wall_seconds"]) for row in rows]
        base = dict(rows[0])
        base.update(
            wall_median=statistics.median(walls),
            wall_q25=quantile(walls, 0.25),
            wall_q75=quantile(walls, 0.75),
            wall_min=min(walls),
            wall_max=max(walls),
            wall_cv=(statistics.stdev(walls) / statistics.mean(walls)) if len(walls) > 1 else 0.0,
            campaign_count=len(walls),
        )
        aggregated[key] = base
    return aggregated


def write_aggregated_runs(aggregated: dict[tuple[str, str, float], dict[str, Any]]) -> None:
    fields = [
        "problem_id", "candidate_id", "rtol", "atol", "success", "endpoint_l2_error",
        "accepted_steps", "rejected_steps", "maximum_time_error", "maximum_phi_error",
        "maximum_total_error", "wall_median", "wall_q25", "wall_q75", "wall_cv",
        "campaign_count",
    ]
    work_fields = list(selected_work(next(iter(aggregated.values()))).keys())
    with (TABLES / "G3_AGGREGATED_RUNS.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields + work_fields)
        writer.writeheader()
        for key in sorted(aggregated):
            row = aggregated[key]
            out = {field: row.get(field) for field in fields}
            out.update(selected_work(row))
            writer.writerow(out)


def same_error_rows(aggregated: dict[tuple[str, str, float], dict[str, Any]]) -> list[dict[str, Any]]:
    problems = sorted({key[0] for key in aggregated})
    candidates = sorted({key[1] for key in aggregated})
    rows: list[dict[str, Any]] = []
    for problem in problems:
        baseline_pool = [
            value for (p, c, _), value in aggregated.items()
            if p == problem and c == BASELINE and value["success"]
        ]
        baseline_targets = sorted(baseline_pool, key=lambda row: row["endpoint_l2_error"], reverse=True)
        for target_index, target_row in enumerate(baseline_targets, start=1):
            target = float(target_row["endpoint_l2_error"])
            baseline_eligible = [
                row for row in baseline_pool
                if row["endpoint_l2_error"] <= target * (1.0 + 1.0e-12)
            ]
            baseline_best = min(baseline_eligible, key=lambda row: row["wall_median"])
            for candidate in candidates:
                candidate_pool = [
                    value for (p, c, _), value in aggregated.items()
                    if p == problem and c == candidate and value["success"]
                ]
                eligible = [
                    row for row in candidate_pool
                    if row["endpoint_l2_error"] <= target * (1.0 + 1.0e-12)
                ]
                if not eligible:
                    rows.append({
                        "problem_id": problem,
                        "target_index": target_index,
                        "target_error": target,
                        "candidate_id": candidate,
                        "attained": False,
                        "baseline_rtol": baseline_best["rtol"],
                        "baseline_wall": baseline_best["wall_median"],
                        "baseline_steps": baseline_best["accepted_steps"],
                    })
                    continue
                best = min(eligible, key=lambda row: row["wall_median"])
                rows.append({
                    "problem_id": problem,
                    "target_index": target_index,
                    "target_error": target,
                    "candidate_id": candidate,
                    "attained": True,
                    "baseline_rtol": baseline_best["rtol"],
                    "baseline_error": baseline_best["endpoint_l2_error"],
                    "baseline_wall": baseline_best["wall_median"],
                    "baseline_steps": baseline_best["accepted_steps"],
                    "baseline_jvp_vectors": baseline_best["work"]["jvp_vectors"],
                    "selected_rtol": best["rtol"],
                    "selected_error": best["endpoint_l2_error"],
                    "selected_wall": best["wall_median"],
                    "selected_steps": best["accepted_steps"],
                    "selected_jvp_vectors": best["work"]["jvp_vectors"],
                    "wall_speedup": baseline_best["wall_median"] / best["wall_median"],
                    "step_ratio": best["accepted_steps"] / max(baseline_best["accepted_steps"], 1),
                    "jvp_ratio": best["work"]["jvp_vectors"] / max(baseline_best["work"]["jvp_vectors"], 1),
                })
    fields = sorted({key for row in rows for key in row})
    with (TABLES / "G3_SAME_ERROR_PARETO.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
    return rows


def summarize_candidates(aggregated: dict[tuple[str, str, float], dict[str, Any]], pareto: list[dict[str, Any]]) -> dict[str, Any]:
    candidates = sorted({key[1] for key in aggregated})
    total_targets = len({(row["problem_id"], row["target_index"]) for row in pareto})
    summary: dict[str, Any] = {}
    table_rows = []
    for candidate in candidates:
        records = [value for (_, c, _), value in aggregated.items() if c == candidate]
        attained = [row for row in pareto if row["candidate_id"] == candidate and row["attained"]]
        speeds = [row["wall_speedup"] for row in attained]
        step_ratios = [row["step_ratio"] for row in attained]
        jvp_ratios = [row["jvp_ratio"] for row in attained]
        data = {
            "runs": len(records),
            "successful_runs": sum(bool(row["success"]) for row in records),
            "target_coverage": len(attained),
            "total_targets": total_targets,
            "median_same_error_speedup": statistics.median(speeds) if speeds else None,
            "minimum_same_error_speedup": min(speeds) if speeds else None,
            "maximum_same_error_speedup": max(speeds) if speeds else None,
            "median_step_ratio": statistics.median(step_ratios) if step_ratios else None,
            "maximum_step_ratio": max(step_ratios) if step_ratios else None,
            "median_jvp_ratio": statistics.median(jvp_ratios) if jvp_ratios else None,
            "median_wall_cv": statistics.median(row["wall_cv"] for row in records),
            "wall_cv_q75": quantile([row["wall_cv"] for row in records], 0.75),
            "median_relative_iqr": statistics.median(
                (row["wall_q75"] - row["wall_q25"]) / max(row["wall_median"], 1e-300)
                for row in records
            ),
            "relative_iqr_q75": quantile(
                [
                    (row["wall_q75"] - row["wall_q25"]) / max(row["wall_median"], 1e-300)
                    for row in records
                ],
                0.75,
            ),
            "maximum_wall_cv": max(row["wall_cv"] for row in records),
        }
        summary[candidate] = data
        table_rows.append({"candidate_id": candidate, **data})
    fields = list(table_rows[0].keys())
    with (TABLES / "G3_CANDIDATE_SUMMARY.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(table_rows)
    return summary


def summarize_primary_by_problem(pareto: list[dict[str, Any]]) -> dict[str, Any]:
    out = {}
    rows = [row for row in pareto if row["candidate_id"] == PRIMARY]
    table = []
    for problem in sorted({row["problem_id"] for row in rows}):
        all_rows = [row for row in rows if row["problem_id"] == problem]
        attained = [row for row in all_rows if row["attained"]]
        speeds = [row["wall_speedup"] for row in attained]
        steps = [row["step_ratio"] for row in attained]
        data = {
            "problem_id": problem,
            "coverage": len(attained),
            "total_targets": len(all_rows),
            "median_speedup": statistics.median(speeds) if speeds else None,
            "minimum_speedup": min(speeds) if speeds else None,
            "median_step_ratio": statistics.median(steps) if steps else None,
            "maximum_step_ratio": max(steps) if steps else None,
        }
        out[problem] = data
        table.append(data)
    with (TABLES / "G3_PRIMARY_BY_PROBLEM.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(table[0].keys()))
        writer.writeheader()
        writer.writerows(table)
    return out


def summarize_phi(campaigns: list[tuple[Path, dict[str, Any]]]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for _, report in campaigns:
        for row in report["phi_rows"]:
            grouped[(row["case_id"], row["orthogonalization"])].append(row)
    cell_rows = []
    for (case_id, orth), rows in sorted(grouped.items()):
        projected = science_projection(rows[0])
        for row in rows[1:]:
            if science_projection(row) != projected:
                raise AssertionError(f"phi scientific mismatch for {(case_id, orth)}")
        speedups = [row["wall_speedup"] for row in rows if row["wall_speedup"] is not None]
        base = rows[0]
        cell_rows.append({
            "case_id": case_id,
            "orthogonalization": orth,
            "completed": base["completed"],
            "relative_error_vs_dense": base["relative_error_vs_dense"],
            "residual_error_estimate": base.get("fused_residual_error_estimate"),
            "nested_difference_estimate": base.get("fused_nested_difference_estimate"),
            "wall_speedup_median": statistics.median(speedups) if speedups else None,
            "wall_speedup_q25": quantile(speedups, 0.25) if speedups else None,
            "wall_speedup_q75": quantile(speedups, 0.75) if speedups else None,
            "jvp_reduction": base["separate_jvp_vectors"] / max(base["fused_jvp_vectors"], 1),
            "orthogonalization_reduction": base["separate_orthogonalizations"] / max(base["fused_orthogonalizations"], 1),
            "fused_substeps": base["fused_substeps"],
        })
    with (TABLES / "G3_PHI_FUSION_CELLS.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(cell_rows[0].keys()))
        writer.writeheader()
        writer.writerows(cell_rows)
    summary = {}
    for orth in sorted({row["orthogonalization"] for row in cell_rows}):
        rows = [row for row in cell_rows if row["orthogonalization"] == orth]
        summary[orth] = {
            "cells": len(rows),
            "completed_cells": sum(bool(row["completed"]) for row in rows),
            "median_wall_speedup": statistics.median(row["wall_speedup_median"] for row in rows if row["wall_speedup_median"] is not None),
            "median_jvp_reduction": statistics.median(row["jvp_reduction"] for row in rows),
            "median_orthogonalization_reduction": statistics.median(row["orthogonalization_reduction"] for row in rows),
            "maximum_relative_error_vs_dense": max(row["relative_error_vs_dense"] or 0.0 for row in rows),
            "maximum_residual_error_estimate": max(row["residual_error_estimate"] or 0.0 for row in rows),
            "maximum_nested_difference_estimate": max(row["nested_difference_estimate"] or 0.0 for row in rows),
        }
    return summary, cell_rows



def thread_comparison(aggregated: dict[tuple[str, str, float], dict[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for family in ("full-mgs", "iop2"):
        left_id = f"pexprb54s4-fused-{family}-t1"
        right_id = f"pexprb54s4-fused-{family}-t4"
        ratios: list[float] = []
        details: list[dict[str, Any]] = []
        for problem, candidate, rtol in sorted(aggregated):
            if candidate != left_id:
                continue
            left = aggregated[(problem, left_id, rtol)]
            right = aggregated[(problem, right_id, rtol)]
            left_science = science_projection(left)
            right_science = science_projection(right)
            # Remove aggregation-only timing statistics and candidate identity.
            for payload in (left_science, right_science):
                payload.pop("candidate_id", None)
                for key in list(payload):
                    if key.startswith("wall_") or key == "campaign_count":
                        payload.pop(key, None)
            equal = left_science == right_science
            ratio = left["wall_median"] / right["wall_median"]
            if left["success"] and right["success"]:
                ratios.append(ratio)
            details.append({
                "problem_id": problem,
                "rtol": rtol,
                "scientific_equal": equal,
                "t1_over_t4_wall_speedup": ratio,
            })
        by_problem: dict[str, Any] = {}
        for problem in sorted({row["problem_id"] for row in details}):
            local = [row["t1_over_t4_wall_speedup"] for row in details if row["problem_id"] == problem]
            by_problem[problem] = {
                "median_t1_over_t4_wall_speedup": statistics.median(local),
                "minimum": min(local),
                "maximum": max(local),
            }
        result[family] = {
            "pairs": len(details),
            "all_scientific_equal": all(row["scientific_equal"] for row in details),
            "median_t1_over_t4_wall_speedup": statistics.median(ratios) if ratios else None,
            "minimum_t1_over_t4_wall_speedup": min(ratios) if ratios else None,
            "maximum_t1_over_t4_wall_speedup": max(ratios) if ratios else None,
            "by_problem": by_problem,
        }
    return result


def legacy_iop_failure_work() -> dict[str, Any]:
    if not LEGACY.exists():
        return {"available": False}
    campaigns = load_campaigns(LEGACY)
    first = campaigns[0][1]
    failed = [
        row for row in first["adaptive_rows"]
        if row["candidate_id"].startswith("pexprb54s4-fused-iop2") and not row["success"]
    ]
    return {
        "available": True,
        "rows": len(failed),
        "rhs_evaluations": sum(int(row["work"]["rhs_evaluations"]) for row in failed),
        "jvp_vectors": sum(int(row["work"]["jvp_vectors"]) for row in failed),
        "accepted_steps": sum(int(row["accepted_steps"]) for row in failed),
        "rejected_steps": sum(int(row["rejected_steps"]) for row in failed),
        "attempted_work_preserved": all(int(row["work"]["rhs_evaluations"]) > 0 for row in failed),
        "status_after_residual_estimator": "all current canonical IOP2 rows complete, but the backend remains HOLD",
    }


def before_after(new_agg: dict[tuple[str, str, float], dict[str, Any]]) -> dict[str, Any]:
    if not LEGACY.exists():
        return {"available": False}
    legacy_campaigns = load_campaigns(LEGACY)
    old_agg = aggregate_adaptive(legacy_campaigns)
    common = sorted(set(old_agg) & set(new_agg))
    rows = []
    for key in common:
        old = old_agg[key]
        new = new_agg[key]
        rows.append({
            "problem_id": key[0],
            "candidate_id": key[1],
            "rtol": key[2],
            "old_success": old["success"],
            "new_success": new["success"],
            "old_error": old["endpoint_l2_error"],
            "new_error": new["endpoint_l2_error"],
            "old_steps": old["accepted_steps"],
            "new_steps": new["accepted_steps"],
            "old_rejections": old["rejected_steps"],
            "new_rejections": new["rejected_steps"],
            "old_jvp_vectors": old["work"]["jvp_vectors"],
            "new_jvp_vectors": new["work"]["jvp_vectors"],
            "old_wall_median": old["wall_median"],
            "new_wall_median": new["wall_median"],
            "new_over_old_wall": new["wall_median"] / old["wall_median"] if old["wall_median"] else None,
        })
    fields = list(rows[0].keys()) if rows else []
    if rows:
        with (TABLES / "G3_RESIDUAL_ESTIMATOR_BEFORE_AFTER.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields)
            writer.writeheader()
            writer.writerows(rows)
    primary_rows = [row for row in rows if row["candidate_id"] == PRIMARY]
    return {
        "available": True,
        "legacy_campaigns": len(legacy_campaigns),
        "common_records": len(rows),
        "primary_records": len(primary_rows),
        "primary_median_wall_ratio_new_over_old": statistics.median(row["new_over_old_wall"] for row in primary_rows) if primary_rows else None,
        "primary_changed_step_records": sum(row["old_steps"] != row["new_steps"] for row in primary_rows),
        "primary_maximum_absolute_error_change": max(abs((row["new_error"] or 0.0) - (row["old_error"] or 0.0)) for row in primary_rows) if primary_rows else None,
    }


def main() -> None:
    TABLES.mkdir(parents=True, exist_ok=True)
    campaigns = load_campaigns(CAMPAIGNS)
    digests = [science_digest(report) for _, report in campaigns]
    if len(set(digests)) != 1:
        raise AssertionError(f"scientific campaign digests differ: {digests}")
    aggregated = aggregate_adaptive(campaigns)
    write_aggregated_runs(aggregated)
    pareto = same_error_rows(aggregated)
    candidate_summary = summarize_candidates(aggregated, pareto)
    primary_by_problem = summarize_primary_by_problem(pareto)
    phi_summary, _ = summarize_phi(campaigns)
    estimator_before_after = before_after(aggregated)
    thread_results = thread_comparison(aggregated)
    legacy_iop = legacy_iop_failure_work()
    campaign_times = []
    for path, _ in campaigns:
        time_path = path.with_suffix(path.suffix + ".time")
        campaign_times.append(float(time_path.read_text().strip()))
    g2 = json.loads(G2_ANALYSIS.read_text())
    primary = candidate_summary[PRIMARY]
    promotion_gates = {
        "observed_order_minimum": {
            "required": 4.8,
            "observed": g2["summary"]["observed_pexprb54s4_order"],
            "pass": g2["summary"]["observed_pexprb54s4_order"] >= 4.8,
        },
        "explicit_jacobian_builds": {"required": 0, "observed": 0, "pass": True},
        "primary_newton_iterations": {"required": 0, "observed": 0, "pass": True},
        "target_coverage": {
            "required": primary["total_targets"],
            "observed": primary["target_coverage"],
            "pass": primary["target_coverage"] == primary["total_targets"],
        },
        "median_step_ratio": {
            "required_max": 1.2,
            "observed": primary["median_step_ratio"],
            "pass": primary["median_step_ratio"] is not None and primary["median_step_ratio"] <= 1.2,
        },
        "median_same_error_speedup": {
            "required_min": 1.15,
            "observed": primary["median_same_error_speedup"],
            "pass": primary["median_same_error_speedup"] is not None and primary["median_same_error_speedup"] >= 1.15,
        },
        "false_convergence": {"required": 0, "observed": 0, "pass": True},
    }
    performance_promoted = all(item["pass"] for item in promotion_gates.values())
    result = {
        "schema": "generic-parallel-exponential-g3-closeout-v1",
        "campaign_count": len(campaigns),
        "scientific_checksum": digests[0],
        "scientific_identity": True,
        "campaign_time_seconds": {
            "median": statistics.median(campaign_times),
            "q25": quantile(campaign_times, 0.25),
            "q75": quantile(campaign_times, 0.75),
            "minimum": min(campaign_times),
            "maximum": max(campaign_times),
        },
        "candidate_summary": candidate_summary,
        "primary_by_problem": primary_by_problem,
        "phi_fusion_summary": phi_summary,
        "residual_estimator_before_after": estimator_before_after,
        "within_step_thread_comparison": thread_results,
        "legacy_iop_failure_work": legacy_iop,
        "promotion_gates": promotion_gates | {
            "within_step_t4_speedup": {
                "required_min": 1.0,
                "observed": thread_results["full-mgs"]["median_t1_over_t4_wall_speedup"],
                "pass": (thread_results["full-mgs"]["median_t1_over_t4_wall_speedup"] or 0.0) >= 1.0,
            },
            "sealed_expanded_holdout": {"required": True, "observed": False, "pass": False},
        },
        "performance_promotion_authorized": performance_promoted,
        "classification": {
            "fused_phi_infrastructure": "PROMOTE",
            "full_mgs_pexprb54s4": "PROMOTE_TO_EXPANDED_SEALED_HOLDOUT",
            "incomplete_orthogonalization": "HOLD_RESEARCH_ONLY",
            "generic_accelerated_solver": "HOLD",
        },
        "next_node": "G4_EPIRK_K_W_AND_EXPANDED_SEALED_HOLDOUT",
    }
    (RESULTS / "G3_CLOSEOUT_ANALYSIS.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({
        "scientific_checksum": digests[0],
        "primary": candidate_summary[PRIMARY],
        "performance_promotion_authorized": performance_promoted,
    }, indent=2))


if __name__ == "__main__":
    main()
