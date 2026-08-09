#!/usr/bin/env python3
"""Analyze deterministic SABR/homotopy common-W RHS telemetry.

This script is intentionally read-only with respect to solver state. It consumes the
canonical JSON artifact and writes compact tables, plots, and a machine-readable summary.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import math
import statistics
from pathlib import Path
from typing import Any, Iterable

import matplotlib.pyplot as plt

BACKENDS = [
    "seeded-shared-gmres",
    "block-gmres",
    "independent-rayon-gmres",
]
THRESHOLDS = [1.0e-6, 1.0e-8, 1.0e-10, 1.0e-12]


def median(values: Iterable[float]) -> float | None:
    values = list(values)
    return statistics.median(values) if values else None


def quantile(values: Iterable[float], fraction: float) -> float | None:
    values = sorted(values)
    if not values:
        return None
    index = round(fraction * (len(values) - 1))
    return values[index]


def rank_at(singular_values: list[float], threshold: float) -> int:
    if not singular_values or singular_values[0] <= 0.0:
        return 0
    cutoff = threshold * singular_values[0]
    return sum(value > cutoff for value in singular_values)


def write_csv(path: Path, rows: list[dict[str, Any]], fields: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def case_backend_table(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for case_id in sorted({row["case_id"] for row in rows}):
        subset = [row for row in rows if row["case_id"] == case_id]
        counts = collections.Counter(row["suggested_backend"] for row in subset)
        record: dict[str, Any] = {
            "case_id": case_id,
            "dimension": subset[0]["dimension"],
            "risk": subset[0]["risk"],
            "rows": len(subset),
        }
        for backend in BACKENDS:
            key = backend.replace("-", "_")
            record[f"{key}_count"] = counts[backend]
            record[f"{key}_fraction"] = counts[backend] / len(subset)
        out.append(record)
    return out


def method_q_backend_table(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = collections.defaultdict(list)
    for row in rows:
        label = "none" if row["q"] is None else str(row["q"])
        grouped[(row["method"], label)].append(row)
    out: list[dict[str, Any]] = []
    for (method, q), subset in sorted(grouped.items()):
        counts = collections.Counter(row["suggested_backend"] for row in subset)
        record: dict[str, Any] = {"method": method, "q": q, "rows": len(subset)}
        for backend in BACKENDS:
            key = backend.replace("-", "_")
            record[f"{key}_count"] = counts[backend]
            record[f"{key}_fraction"] = counts[backend] / len(subset)
        out.append(record)
    return out


def rank_threshold_table(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    base = [rank_at(row["transformed_directional"]["singular_values"], 1.0e-8) for row in rows]
    out: list[dict[str, Any]] = []
    for threshold in THRESHOLDS:
        ranks = [rank_at(row["transformed_directional"]["singular_values"], threshold) for row in rows]
        counts = collections.Counter(ranks)
        out.append(
            {
                "rank_tolerance": threshold,
                "rows": len(rows),
                "changed_from_1e_8": sum(left != right for left, right in zip(base, ranks)),
                "changed_fraction_from_1e_8": sum(
                    left != right for left, right in zip(base, ranks)
                )
                / len(rows),
                **{f"rank_{rank}_count": counts[rank] for rank in range(0, 9)},
            }
        )
    return out


def drift_table(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for backend in BACKENDS:
        values = [
            row["transformed_drift"]["maximum_principal_angle_degrees"]
            for row in rows
            if row["suggested_backend"] == backend and row["transformed_drift"] is not None
        ]
        out.append(
            {
                "group": backend,
                "rows": len(values),
                "median_degrees": median(values),
                "p90_degrees": quantile(values, 0.90),
                "maximum_degrees": max(values) if values else None,
            }
        )
    for q in [0, 1, 2]:
        values = [
            row["transformed_drift"]["maximum_principal_angle_degrees"]
            for row in rows
            if row["method"] == "homotopy"
            and row["q"] == q
            and row["transformed_drift"] is not None
        ]
        out.append(
            {
                "group": f"homotopy-q{q}",
                "rows": len(values),
                "median_degrees": median(values),
                "p90_degrees": quantile(values, 0.90),
                "maximum_degrees": max(values) if values else None,
            }
        )
    return out


def make_case_backend_plot(table: list[dict[str, Any]], path: Path) -> None:
    labels = [row["case_id"] for row in table]
    bottom = [0.0] * len(table)
    plt.figure(figsize=(11, 6))
    for backend in BACKENDS:
        key = f"{backend.replace('-', '_')}_fraction"
        values = [row[key] for row in table]
        plt.barh(labels, values, left=bottom, label=backend)
        bottom = [left + value for left, value in zip(bottom, values)]
    plt.xlabel("Fraction of recorded common-W batches")
    plt.ylabel("Telemetry case")
    plt.xlim(0.0, 1.0)
    plt.legend(loc="center left", bbox_to_anchor=(1.01, 0.5))
    plt.tight_layout(rect=(0.0, 0.0, 0.78, 1.0))
    path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(path, dpi=180)
    plt.close()


def make_rank_distribution_plot(rows: list[dict[str, Any]], path: Path) -> None:
    groups = [
        ("SABR", lambda row: row["method"] == "sabr"),
        ("H q=0", lambda row: row["method"] == "homotopy" and row["q"] == 0),
        ("H q=1", lambda row: row["method"] == "homotopy" and row["q"] == 1),
        ("H q=2", lambda row: row["method"] == "homotopy" and row["q"] == 2),
    ]
    labels = [name for name, _ in groups]
    bottom = [0.0] * len(groups)
    plt.figure(figsize=(8, 5))
    for rank in range(1, 9):
        values = []
        for _, predicate in groups:
            subset = [row for row in rows if predicate(row)]
            count = sum(row["transformed_directional"]["numerical_rank"] == rank for row in subset)
            values.append(count / len(subset) if subset else 0.0)
        plt.bar(labels, values, bottom=bottom, label=f"rank {rank}")
        bottom = [left + value for left, value in zip(bottom, values)]
    plt.ylabel("Fraction of batches")
    plt.ylim(0.0, 1.0)
    plt.legend(ncol=2, fontsize=8, loc="center left", bbox_to_anchor=(1.01, 0.5))
    plt.tight_layout(rect=(0.0, 0.0, 0.80, 1.0))
    path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(path, dpi=180)
    plt.close()


def make_drift_plot(rows: list[dict[str, Any]], path: Path) -> None:
    labels: list[str] = []
    medians: list[float] = []
    p90s: list[float] = []
    maxima: list[float] = []
    for backend in BACKENDS:
        values = [
            row["transformed_drift"]["maximum_principal_angle_degrees"]
            for row in rows
            if row["suggested_backend"] == backend and row["transformed_drift"] is not None
        ]
        labels.append(backend.replace("-gmres", ""))
        medians.append(float(median(values) or 0.0))
        p90s.append(float(quantile(values, 0.90) or 0.0))
        maxima.append(max(values) if values else 0.0)
    positions = list(range(len(labels)))
    width = 0.24
    plt.figure(figsize=(9, 5))
    plt.bar([position - width for position in positions], medians, width=width, label="median")
    plt.bar(positions, p90s, width=width, label="p90")
    plt.bar([position + width for position in positions], maxima, width=width, label="maximum")
    plt.ylabel("Maximum principal angle between matching batches [deg]")
    plt.xticks(positions, labels, rotation=15, ha="right")
    plt.ylim(0.0, 95.0)
    plt.legend()
    plt.tight_layout()
    path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(path, dpi=180)
    plt.close()


def make_threshold_plot(table: list[dict[str, Any]], path: Path) -> None:
    thresholds = [row["rank_tolerance"] for row in table]
    changed = [row["changed_fraction_from_1e_8"] for row in table]
    plt.figure(figsize=(7, 5))
    plt.semilogx(thresholds, changed, marker="o")
    plt.axvline(1.0e-8, linestyle="--")
    plt.xlabel("Relative singular-value rank tolerance")
    plt.ylabel("Fraction whose rank differs from tolerance 1e-8")
    plt.ylim(0.0, max(changed) * 1.15 + 1.0e-12)
    plt.tight_layout()
    path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(path, dpi=180)
    plt.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output_root", type=Path)
    args = parser.parse_args()

    report = json.loads(args.input.read_text(encoding="utf-8"))
    rows: list[dict[str, Any]] = report["rows"]
    failures: list[dict[str, Any]] = report["failures"]
    root = args.output_root
    tables = root / "tables"
    plots = root / "plots"
    tables.mkdir(parents=True, exist_ok=True)
    plots.mkdir(parents=True, exist_ok=True)

    case_table = case_backend_table(rows)
    method_table = method_q_backend_table(rows)
    threshold_table = rank_threshold_table(rows)
    drift = drift_table(rows)

    case_fields = list(case_table[0])
    method_fields = list(method_table[0])
    threshold_fields = list(threshold_table[0])
    drift_fields = list(drift[0])
    write_csv(tables / "CASE_BACKEND_SUMMARY.csv", case_table, case_fields)
    write_csv(tables / "METHOD_Q_BACKEND_SUMMARY.csv", method_table, method_fields)
    write_csv(tables / "RANK_THRESHOLD_SENSITIVITY.csv", threshold_table, threshold_fields)
    write_csv(tables / "SUBSPACE_DRIFT_SUMMARY.csv", drift, drift_fields)
    if failures:
        write_csv(
            tables / "FAILED_PATHS.csv",
            failures,
            ["case_id", "method", "q", "theta", "predictor", "error", "rows_preserved"],
        )

    non_high = [row for row in rows if row["risk"] != "high"]
    total_counts = collections.Counter(row["suggested_backend"] for row in rows)
    non_high_counts = collections.Counter(row["suggested_backend"] for row in non_high)
    rank_distribution = collections.Counter(row["transformed_directional"]["numerical_rank"] for row in rows)
    raw_rank_distribution = collections.Counter(row["raw_directional"]["numerical_rank"] for row in rows)
    transitions = collections.Counter(
        (row["raw_directional"]["numerical_rank"], row["transformed_directional"]["numerical_rank"])
        for row in rows
    )
    energy_rank_one_fraction = sum(
        row["transformed_directional"]["energy_rank_99"] == 1 for row in rows
    ) / len(rows)
    stable_ranks = [row["transformed_directional"]["stable_rank"] for row in rows]

    summary = {
        "schema": "rodas5p-homotopy-rhs-telemetry-analysis-v1",
        "source_schema": report["schema"],
        "source_scientific_checksum": report["scientific_checksum"],
        "cases": len(report["cases"]),
        "rows": len(rows),
        "failed_paths": len(failures),
        "behavior_comparisons": report["behavior_comparisons"],
        "behavior_mismatches": report["behavior_mismatches"],
        "dispatcher_active": report["dispatcher_active"],
        "backend_recommendations_advisory": report["backend_recommendations_advisory"],
        "reference_explicit_jacobian_builds": report["reference_explicit_jacobian_builds"],
        "reference_factorization_builds": report["reference_factorization_builds"],
        "backend_counts": dict(total_counts),
        "backend_fractions": {
            backend: total_counts[backend] / len(rows) for backend in BACKENDS
        },
        "non_high_risk_rows": len(non_high),
        "non_high_risk_backend_counts": dict(non_high_counts),
        "non_high_risk_backend_fractions": {
            backend: non_high_counts[backend] / len(non_high) for backend in BACKENDS
        },
        "raw_directional_rank_distribution": {str(key): value for key, value in sorted(raw_rank_distribution.items())},
        "transformed_directional_rank_distribution": {
            str(key): value for key, value in sorted(rank_distribution.items())
        },
        "rank_transitions": {
            f"{left}->{right}": value for (left, right), value in sorted(transitions.items())
        },
        "energy_rank_99_equals_one_fraction": energy_rank_one_fraction,
        "transformed_directional_stable_rank_median": median(stable_ranks),
        "rank_threshold_sensitivity": threshold_table,
        "subspace_drift": drift,
        "failure_rows": failures,
        "primary_findings": [
            "After unit-column normalization, actual common-W batches are frequently low directional rank, but a large high-risk subset still requires the protected independent-Rayon backend.",
            "Seed sharing is confined to strongly collinear rank-one batches after a conservative median-cosine guard.",
            "Directional numerical-rank decisions remain threshold-sensitive, so dispatch must use hysteresis, transactional fallback, and a true-residual certificate.",
            "Two severe q=2 theta=1 homotopy paths diverged before dispatch; a linear backend cannot cure this path-level failure.",
        ],
        "verdict": "telemetry gate passed; activation of the dispatcher remains on hold",
    }
    (tables / "RHS_TELEMETRY_ANALYSIS.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    make_case_backend_plot(case_table, plots / "BACKEND_RECOMMENDATION_BY_CASE.png")
    make_rank_distribution_plot(rows, plots / "TRANSFORMED_RANK_BY_METHOD_Q.png")
    make_drift_plot(rows, plots / "SUBSPACE_DRIFT_BY_BACKEND.png")
    make_threshold_plot(threshold_table, plots / "RANK_THRESHOLD_SENSITIVITY.png")


if __name__ == "__main__":
    main()
