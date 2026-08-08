#!/usr/bin/env python3
"""Reproducible post-processing for the canonical homotopy numerical screen.

This script uses only the Python standard library.  It does not execute any numerical
integrator; it audits and summarizes the deterministic JSON emitted by the Rust binary.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


def finite(values: Iterable[Any]) -> list[float]:
    out: list[float] = []
    for value in values:
        if isinstance(value, (int, float)) and math.isfinite(float(value)):
            out.append(float(value))
    return out


def median(values: Iterable[Any]) -> float | None:
    items = finite(values)
    return statistics.median(items) if items else None


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    columns: list[str] = []
    seen: set[str] = set()
    for row in rows:
        for key in row:
            if key not in seen:
                seen.add(key)
                columns.append(key)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    raw = args.input.read_bytes()
    document = json.loads(raw)
    if document.get("schema") != "rodas5p-homotopy-experiment-screen-v1":
        raise SystemExit("unexpected experiment schema")
    if document.get("profile") != "canonical":
        raise SystemExit("analysis requires the canonical profile")

    source_sha256 = hashlib.sha256(raw).hexdigest()
    cases = {row["case_id"]: row for row in document["cases"]}
    controls = {
        (row["case_id"], row["method"]): row for row in document["controls"]
    }
    candidates: list[dict[str, Any]] = document["candidates"]

    q_correction_groups: dict[tuple[int, int], list[dict[str, Any]]] = defaultdict(list)
    family_correction_groups: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    configuration_groups: dict[tuple[Any, ...], list[dict[str, Any]]] = defaultdict(list)
    low_depth_rows: list[dict[str, Any]] = []
    false_rejects = 0
    accepted_oracle_values: list[float] = []
    certificate_ratios: list[float] = []

    for row in candidates:
        case = cases[row["case_id"]]
        row["_family"] = case["family"]
        q_correction_groups[(row["corrections_per_point"], row["q"])].append(row)
        family_correction_groups[(case["family"], row["corrections_per_point"])].append(row)
        configuration_groups[
            (
                row["theta"],
                row["q"],
                row["path_rounds"],
                row["predictor"],
                row["corrections_per_point"],
            )
        ].append(row)

        oracle = row.get("oracle_output_wrms")
        if (
            isinstance(oracle, (int, float))
            and oracle <= document["output_wrms_budget"]
            and not row["fast_accepted"]
            and row.get("failure") is None
        ):
            false_rejects += 1
        if row["fast_accepted"] and isinstance(oracle, (int, float)):
            accepted_oracle_values.append(float(oracle))
        certificate = row.get("output_wrms")
        if (
            row["fast_accepted"]
            and
            isinstance(oracle, (int, float))
            and isinstance(certificate, (int, float))
            and oracle > 0.0
        ):
            certificate_ratios.append(float(certificate) / float(oracle))

        work = row.get("work")
        if (
            row["fast_accepted"]
            and isinstance(work, dict)
            and work.get("w_solve_batches", 10**9) < 8
        ):
            low_depth_rows.append(row)

    q_correction_summary: list[dict[str, Any]] = []
    for (corrections, q), rows in sorted(q_correction_groups.items()):
        q_correction_summary.append(
            {
                "corrections_per_point": corrections,
                "q": q,
                "rows": len(rows),
                "fast_accepts": sum(row["fast_accepted"] for row in rows),
                "fallbacks": sum(row["used_fallback"] for row in rows),
                "failures": sum(row.get("failure") is not None for row in rows),
                "fast_accept_rate": sum(row["fast_accepted"] for row in rows) / len(rows),
                "median_oracle_output_wrms": median(
                    row.get("oracle_output_wrms") for row in rows
                ),
                "median_w_solve_batches": median(
                    (row.get("work") or {}).get("w_solve_batches") for row in rows
                ),
                "median_rhs_evaluations": median(
                    row["counters"].get("rhs_evaluations") for row in rows
                ),
                "median_jvp_vectors": median(
                    row["counters"].get("jvp_vectors") for row in rows
                ),
            }
        )

    family_correction_summary: list[dict[str, Any]] = []
    for (family, corrections), rows in sorted(family_correction_groups.items()):
        family_correction_summary.append(
            {
                "family": family,
                "corrections_per_point": corrections,
                "rows": len(rows),
                "fast_accepts": sum(row["fast_accepted"] for row in rows),
                "fast_accept_rate": sum(row["fast_accepted"] for row in rows) / len(rows),
                "median_oracle_output_wrms": median(
                    row.get("oracle_output_wrms") for row in rows
                ),
            }
        )

    case_correction_groups: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    for row in candidates:
        case_correction_groups[(row["case_id"], row["corrections_per_point"])].append(row)
    case_acceptance_summary: list[dict[str, Any]] = []
    for (case_id, corrections), rows in sorted(case_correction_groups.items()):
        case = cases[case_id]
        case_acceptance_summary.append(
            {
                "case_id": case_id,
                "family": case["family"],
                "stiffness": case.get("stiffness"),
                "nonlinearity": case.get("nonlinearity"),
                "nonnormality": case.get("nonnormality"),
                "h": case["h"],
                "corrections_per_point": corrections,
                "rows": len(rows),
                "fast_accepts": sum(row["fast_accepted"] for row in rows),
                "fast_accept_rate": sum(row["fast_accepted"] for row in rows) / len(rows),
                "median_oracle_output_wrms": median(
                    row.get("oracle_output_wrms") for row in rows
                ),
                "minimum_oracle_output_wrms": min(
                    finite(row.get("oracle_output_wrms") for row in rows)
                ),
            }
        )

    configuration_summary: list[dict[str, Any]] = []
    for key, rows in sorted(configuration_groups.items(), key=lambda item: item[0]):
        theta, q, rounds, predictor, corrections = key
        configuration_summary.append(
            {
                "theta": theta,
                "q": q,
                "path_rounds": rounds,
                "predictor": predictor,
                "corrections_per_point": corrections,
                "rows": len(rows),
                "fast_accepts": sum(row["fast_accepted"] for row in rows),
                "fast_accept_rate": sum(row["fast_accepted"] for row in rows) / len(rows),
                "false_accepts": sum(row["false_accept"] for row in rows),
                "median_oracle_output_wrms": median(
                    row.get("oracle_output_wrms") for row in rows
                ),
                "median_w_solve_batches": median(
                    (row.get("work") or {}).get("w_solve_batches") for row in rows
                ),
                "median_w_solve_vectors": median(
                    (row.get("work") or {}).get("w_solve_vectors") for row in rows
                ),
            }
        )

    low_depth_case_ids = sorted({row["case_id"] for row in low_depth_rows})
    low_depth_cases = [cases[case_id] for case_id in low_depth_case_ids]
    family_case_counts: dict[str, int] = defaultdict(int)
    for case in low_depth_cases:
        family_case_counts[case["family"]] += 1

    work_comparison_rows: list[dict[str, Any]] = []
    dominates_sequential = 0
    dominates_sabr = 0
    low_depth_dominates_sabr = 0
    dominates_sequential_rhs_jvp = 0
    dominates_sabr_rhs_jvp = 0
    for row in candidates:
        if not row["fast_accepted"]:
            continue
        sequential = controls[(row["case_id"], "sequential-direct")]["counters"]
        sabr = controls[(row["case_id"], "sabr-nilpotent-3")]["counters"]
        candidate = row["counters"]
        metrics = ("rhs_evaluations", "jvp_vectors", "direct_solve_calls")
        ratios: dict[str, float] = {}
        sequential_dominance = True
        sabr_dominance = True
        for metric in metrics:
            ratios[f"{metric}_ratio_sequential"] = candidate[metric] / max(
                sequential[metric], 1
            )
            ratios[f"{metric}_ratio_sabr"] = candidate[metric] / max(sabr[metric], 1)
            sequential_dominance &= candidate[metric] <= sequential[metric]
            sabr_dominance &= candidate[metric] <= sabr[metric]
        sequential_rhs_jvp = all(
            candidate[metric] <= sequential[metric]
            for metric in ("rhs_evaluations", "jvp_vectors")
        )
        sabr_rhs_jvp = all(
            candidate[metric] <= sabr[metric]
            for metric in ("rhs_evaluations", "jvp_vectors")
        )
        work = row.get("work") or {}
        is_low_depth = work.get("w_solve_batches", 10**9) < 8
        dominates_sequential += int(sequential_dominance)
        dominates_sabr += int(sabr_dominance)
        low_depth_dominates_sabr += int(is_low_depth and sabr_dominance)
        dominates_sequential_rhs_jvp += int(sequential_rhs_jvp)
        dominates_sabr_rhs_jvp += int(sabr_rhs_jvp)
        work_comparison_rows.append(
            {
                "case_id": row["case_id"],
                "family": cases[row["case_id"]]["family"],
                "theta": row["theta"],
                "q": row["q"],
                "path_rounds": row["path_rounds"],
                "predictor": row["predictor"],
                "corrections_per_point": row["corrections_per_point"],
                "w_solve_batches": work.get("w_solve_batches"),
                "low_depth": is_low_depth,
                "dominates_sequential_three_metrics": sequential_dominance,
                "dominates_sabr_three_metrics": sabr_dominance,
                "dominates_sequential_rhs_and_jvp": sequential_rhs_jvp,
                "dominates_sabr_rhs_and_jvp": sabr_rhs_jvp,
                **ratios,
            }
        )

    low_depth_work_rows = [row for row in work_comparison_rows if row["low_depth"]]
    for row in low_depth_work_rows:
        row["maximum_ratio_to_sabr"] = max(
            row["rhs_evaluations_ratio_sabr"],
            row["jvp_vectors_ratio_sabr"],
            row["direct_solve_calls_ratio_sabr"],
        )
    best_low_depth_rows = sorted(
        low_depth_work_rows,
        key=lambda row: (
            row["maximum_ratio_to_sabr"],
            row["w_solve_batches"],
            row["case_id"],
        ),
    )[:100]

    order_rows = document["order_screens"]
    order_by_method: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in order_rows:
        order_by_method[row["method"]].append(row)
    order_findings: list[dict[str, Any]] = []
    for method, rows in sorted(order_by_method.items()):
        finite_orders = finite(row.get("observed_order") for row in rows)
        all_fast_rows = [row for row in rows if row["all_fast"]]
        all_fast_orders: list[float] = []
        for previous, current in zip(rows, rows[1:]):
            is_baseline = method == "sequential-direct"
            if (is_baseline or (previous["all_fast"] and current["all_fast"])) and isinstance(
                current.get("observed_order"), (int, float)
            ):
                value = float(current["observed_order"])
                if math.isfinite(value):
                    all_fast_orders.append(value)
        order_findings.append(
            {
                "method": method,
                "rows": len(rows),
                "all_fast_rows": len(all_fast_rows),
                "fallbacks": sum(row["fallbacks"] for row in rows),
                "orders": finite_orders,
                "median_order": median(finite_orders),
                "all_fast_orders": all_fast_orders,
                "median_all_fast_order": median(all_fast_orders),
                "finest_error_l2": rows[-1]["error_l2"],
            }
        )

    accepted_ratios = finite(certificate_ratios)
    accepted_oracle = finite(accepted_oracle_values)
    summary = {
        "schema": "rodas5p-homotopy-experiment-analysis-v1",
        "source": {
            "path": str(args.input),
            "sha256": source_sha256,
            "profile": document["profile"],
        },
        "counts": document["summary"],
        "safety": {
            "false_accepts": document["summary"]["false_accepts"],
            "false_rejects_against_one_step_sequential_oracle": false_rejects,
            "max_oracle_output_wrms_among_fast_accepts": max(accepted_oracle),
            "certificate_to_oracle_ratio_min": min(accepted_ratios),
            "certificate_to_oracle_ratio_median": median(accepted_ratios),
            "certificate_to_oracle_ratio_max": max(accepted_ratios),
            "scope_warning": (
                "The one-step certificate had no false accepts at the fixed 0.1 budget, "
                "but the global order screen shows that this fixed budget does not preserve "
                "fifth-order convergence for low-depth paths."
            ),
        },
        "low_depth": {
            "definition": "fast_accepted and w_solve_batches < 8",
            "rows": len(low_depth_rows),
            "cases": len(low_depth_case_ids),
            "family_case_counts": dict(sorted(family_case_counts.items())),
            "median_w_solve_batches": median(
                row["work"]["w_solve_batches"] for row in low_depth_rows
            ),
            "median_w_solve_vectors": median(
                row["work"]["w_solve_vectors"] for row in low_depth_rows
            ),
            "median_nonlinear_snapshots": median(
                row["work"]["nonlinear_snapshots"] for row in low_depth_rows
            ),
        },
        "equal_work": {
            "fast_rows": sum(row["fast_accepted"] for row in candidates),
            "dominates_sequential_in_rhs_jvp_and_direct_solves": dominates_sequential,
            "dominates_sabr_in_rhs_jvp_and_direct_solves": dominates_sabr,
            "low_depth_dominates_sabr_in_all_three_metrics": low_depth_dominates_sabr,
            "dominates_sequential_in_rhs_and_jvp": dominates_sequential_rhs_jvp,
            "dominates_sabr_in_rhs_and_jvp": dominates_sabr_rhs_jvp,
            "median_low_depth_common_w_vector_ratio_to_sequential": median(
                row["work"]["w_solve_vectors"] / 8.0 for row in low_depth_rows
            ),
            "direct_solve_call_caveat": (
                "direct_solve_calls counts one solve_rows batch as one call, whereas sequential "
                "RODAS records eight per-stage calls. It is a call-depth proxy, not scalar solve work."
            ),
            "interpretation": (
                "No fast-accepted candidate dominated either control even in the two scalar-work "
                "counters RHS evaluations and JVP vectors. Batch critical depth is therefore "
                "not yet an end-to-end work advantage."
            ),
        },
        "order_screen": {
            "problem": "manufactured-vector-n6-s80-m10-eta0, T=0.2",
            "findings": order_findings,
            "decision": (
                "q=7 and theta=1,q=2,c=1 reproduce the protected fifth-order trend before "
                "roundoff, but are substantially more expensive.  The low-depth c=0 paths "
                "do not robustly preserve fifth order under the fixed 0.1 local output budget."
            ),
        },
        "decisions": {
            "exact_nonlinear_certificate": "PROMOTE_AS_ONE_STEP_REFERENCE_ORACLE",
            "q7_path": "PROMOTE_AS_CORRECTNESS_ORACLE_ONLY",
            "theta1_q2_c1": "HOLD_ACCURATE_BUT_WORK_HEAVY",
            "low_depth_fixed_budget": "HOLD_GLOBAL_ORDER_FAILURE",
            "fixed_schedule_fast_path": "HOLD_NO_EQUAL_WORK_ADVANTAGE",
            "adaptive_or_lte_scaled_budget": "NEXT_RESEARCH_REQUIRED",
            "production_speedup_claim": "PROHIBITED",
        },
    }

    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "HOMOTOPY_EXPERIMENT_ANALYSIS.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    write_csv(args.output_dir / "HOMOTOPY_Q_CORRECTION_SUMMARY.csv", q_correction_summary)
    write_csv(args.output_dir / "HOMOTOPY_FAMILY_SUMMARY.csv", family_correction_summary)
    write_csv(args.output_dir / "HOMOTOPY_CASE_ACCEPTANCE_SUMMARY.csv", case_acceptance_summary)
    write_csv(args.output_dir / "HOMOTOPY_CONFIGURATION_SUMMARY.csv", configuration_summary)
    write_csv(args.output_dir / "HOMOTOPY_LOW_DEPTH_CASES.csv", low_depth_cases)
    write_csv(args.output_dir / "HOMOTOPY_WORK_COMPARISON.csv", work_comparison_rows)
    write_csv(args.output_dir / "HOMOTOPY_BEST_LOW_DEPTH.csv", best_low_depth_rows)
    write_csv(args.output_dir / "HOMOTOPY_ORDER_SCREEN.csv", order_rows)


if __name__ == "__main__":
    main()
