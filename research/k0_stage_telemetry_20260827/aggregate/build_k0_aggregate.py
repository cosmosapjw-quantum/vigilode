#!/usr/bin/env python3
"""Build the deterministic K0 aggregate, failure ledger, source diff, and plots."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import statistics
import subprocess
from typing import Any

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.patches import Patch  # noqa: E402


def load(path: pathlib.Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: pathlib.Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def finite_values(stages: list[dict[str, Any]], key: str) -> list[float]:
    values = [stage.get(key) for stage in stages]
    return [float(value) for value in values if isinstance(value, (int, float))]


def geometry_values(stages: list[dict[str, Any]], key: str) -> list[float]:
    values = [stage.get("geometry", {}).get(key) for stage in stages]
    return [float(value) for value in values if isinstance(value, (int, float))]


def summarize_cell(cell: dict[str, Any]) -> dict[str, Any]:
    stages = cell.get("stages", [])
    work = [stage["work"] for stage in stages if isinstance(stage.get("work"), dict)]
    initial = finite_values(stages, "initial_true_residual")
    final = finite_values(stages, "final_true_residual")
    remainders = geometry_values(stages, "scaled_nonlinear_remainder")
    angles = geometry_values(stages, "rhs_to_retained_angle_rad")
    receipt = cell.get("receipt") or {}
    return {
        "family": cell["family"],
        "kernel_arm": cell["kernel_arm"],
        "execution_state": cell["execution_state"],
        "stage_receipts": len(stages),
        "stage_attempts": len({stage["attempt_id"] for stage in stages}),
        "linear_matvecs": sum(row["linear_matvecs"] for row in work),
        "diagnostic_matvecs": sum(row["diagnostic_matvecs"] for row in work),
        "operator_applies_total": sum(row["operator_applies_total"] for row in work),
        "telemetry_jvp_overhead": sum(row["telemetry_jvp_overhead"] for row in work),
        "initial_true_residual_max": max(initial, default=None),
        "final_true_residual_max": max(final, default=None),
        "scaled_nonlinear_remainder_median": statistics.median(remainders) if remainders else None,
        "scaled_nonlinear_remainder_max": max(remainders, default=None),
        "rhs_novelty_angle_median": statistics.median(angles) if angles else None,
        "rhs_novelty_angle_max": max(angles, default=None),
        "a1_attempts": receipt.get("attempts"),
        "a1_accepted_steps": receipt.get("accepted_steps"),
        "a1_rejected_steps": receipt.get("rejected_steps"),
        "a1_trace_digest": receipt.get("trace_digest"),
        "audit_event_count": len(receipt.get("event_rows", [])),
        "recommendation_count": len(receipt.get("recommendation_rows", [])),
        "frozen_hard_gates_passed": receipt.get("hard_gates", {}).get("passed") is True,
    }


def totals_by_kernel(cells: list[dict[str, Any]], kernels: tuple[str, ...]) -> dict[str, Any]:
    fields = (
        "stage_receipts",
        "stage_attempts",
        "linear_matvecs",
        "diagnostic_matvecs",
        "operator_applies_total",
        "telemetry_jvp_overhead",
        "a1_attempts",
        "a1_accepted_steps",
        "a1_rejected_steps",
    )
    return {
        kernel: {
            field: sum(int(row[field]) for row in cells if row["kernel_arm"] == kernel)
            for field in fields
        }
        for kernel in kernels
    }


def render_markdown(aggregate: dict[str, Any]) -> str:
    lines = [
        "# VigilODE K0 stage-telemetry aggregate",
        "",
        f"- Verdict: `{aggregate['verdict']}`",
        f"- Claim class: `{aggregate['claim_class']}`",
        f"- Execution head: `{aggregate['execution_identity']['execution_head_sha']}`",
        "- Frozen contract: LegacyFixed (`rtol=1e-10`, `atol=1e-12`), two kernels, six families.",
        "- Claim boundary: raw receipt, geometry, residual, and work observations only; no performance or activation inference.",
        "",
        "| Family | Kernel | State | Stages | Linear | Diagnostic | Checked total | Telemetry JVP | A1 accepted | A1 rejected |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in aggregate["cells"]:
        lines.append(
            "| {family} | {kernel_arm} | {execution_state} | {stage_receipts} | "
            "{linear_matvecs} | {diagnostic_matvecs} | {operator_applies_total} | "
            "{telemetry_jvp_overhead} | {a1_accepted_steps} | {a1_rejected_steps} |".format(
                **row
            )
        )
    return "\n".join(lines) + "\n"


def labels(cells: list[dict[str, Any]]) -> list[str]:
    abbreviations = {
        "robertson-ramped": "ROB",
        "hires-ramped": "HIR",
        "van-der-pol-ramped": "VDP",
        "rotating-nonnormal": "ROT",
        "nonautonomous-stiff-forcing": "NAF",
        "semilinear-advection-diffusion-ramped": "SAD",
    }
    arms = {
        "legacy-restarted-gmres": "L",
        "incremental-givens-candidate": "G",
    }
    return [f"{abbreviations[row['family']]}-{arms[row['kernel_arm']]}" for row in cells]


def style_axis(axis: Any, title: str, ylabel: str, cell_labels: list[str]) -> None:
    axis.set_title(title)
    axis.set_ylabel(ylabel)
    axis.set_xticks(range(len(cell_labels)), cell_labels, rotation=45, ha="right")
    axis.grid(axis="y", alpha=0.25)


def mark_failures(axis: Any, cells: list[dict[str, Any]], heights: list[float]) -> None:
    for index, (cell, height) in enumerate(zip(cells, heights, strict=True)):
        if cell["execution_state"] != "COMPLETE":
            axis.text(index, height, cell["execution_state"], color="red", rotation=90, va="bottom")


def save_plot(path: pathlib.Path, figure: Any) -> str:
    figure.tight_layout()
    figure.savefig(path, dpi=150, metadata={"Software": "VigilODE K0 evidence"})
    plt.close(figure)
    return digest(path)


def build_plots(cells: list[dict[str, Any]], plots_dir: pathlib.Path) -> list[dict[str, str]]:
    plots_dir.mkdir(parents=True, exist_ok=True)
    cell_labels = labels(cells)
    colors = ["#4C78A8" if row["kernel_arm"].startswith("legacy") else "#F58518" for row in cells]
    manifest: list[dict[str, str]] = []

    figure, axis = plt.subplots(figsize=(12, 5))
    totals = [row["operator_applies_total"] for row in cells]
    axis.bar(range(len(cells)), totals, color=colors)
    axis.bar(
        range(len(cells)),
        [row["telemetry_jvp_overhead"] for row in cells],
        bottom=totals,
        color="#54A24B",
    )
    style_axis(axis, "Raw stage work by frozen cell", "operator/JVP applications", cell_labels)
    axis.legend(
        handles=[
            Patch(color="#4C78A8", label="legacy checked linear + diagnostic"),
            Patch(color="#F58518", label="Givens checked linear + diagnostic"),
            Patch(color="#54A24B", label="named telemetry JVP overhead"),
        ]
    )
    mark_failures(axis, cells, totals)
    file = "work_operator_applies.png"
    manifest.append({"metric": "work", "file": file, "sha256": save_plot(plots_dir / file, figure), "reading": "Checked linear plus diagnostic applies are shown separately from the named telemetry JVP overhead; this is not a duration plot."})

    figure, axis = plt.subplots(figsize=(12, 5))
    initial = [max(row["initial_true_residual_max"] or 0.0, 1e-300) for row in cells]
    final = [max(row["final_true_residual_max"] or 0.0, 1e-300) for row in cells]
    x = list(range(len(cells)))
    axis.scatter(x, initial, marker="o", label="maximum initial true residual", color="#4C78A8")
    axis.scatter(x, final, marker="x", label="maximum final true residual", color="#E45756")
    axis.set_yscale("log")
    style_axis(axis, "Unpreconditioned true-residual observations", "residual norm", cell_labels)
    axis.legend()
    mark_failures(axis, cells, initial)
    file = "true_residuals.png"
    manifest.append({"metric": "true_residual", "file": file, "sha256": save_plot(plots_dir / file, figure), "reading": "Only unpreconditioned true-residual observations are plotted; projected residuals remain advisory and are not used here."})

    figure, axis = plt.subplots(figsize=(12, 5))
    rho = [row["scaled_nonlinear_remainder_max"] or 0.0 for row in cells]
    axis.bar(range(len(cells)), rho, color=colors)
    style_axis(axis, "Maximum exact scaled nonlinear remainder", "INV-K0-013 rho", cell_labels)
    axis.legend(
        handles=[
            Patch(color="#4C78A8", label="legacy"),
            Patch(color="#F58518", label="Givens"),
        ]
    )
    mark_failures(axis, cells, rho)
    file = "scaled_nonlinear_remainder.png"
    manifest.append({"metric": "scaled_nonlinear_remainder", "file": file, "sha256": save_plot(plots_dir / file, figure), "reading": "Each bar is the uncapped maximum of the exact Euclidean INV-K0-013 ratio for one cell."})

    figure, axis = plt.subplots(figsize=(12, 5))
    angles = [row["rhs_novelty_angle_max"] or 0.0 for row in cells]
    axis.bar(range(len(cells)), angles, color=colors)
    style_axis(axis, "Maximum current-RHS novelty angle", "radians", cell_labels)
    axis.legend(
        handles=[
            Patch(color="#4C78A8", label="legacy"),
            Patch(color="#F58518", label="Givens"),
        ]
    )
    mark_failures(axis, cells, angles)
    file = "rhs_novelty_angle.png"
    manifest.append({"metric": "rhs_novelty_angle", "file": file, "sha256": save_plot(plots_dir / file, figure), "reading": "Angles use the pre-solve retained RHS prefix, so the plot does not show nested-space self-overlap."})

    figure, axis = plt.subplots(figsize=(12, 5))
    accepted = [row["a1_accepted_steps"] or 0 for row in cells]
    rejected = [row["a1_rejected_steps"] or 0 for row in cells]
    axis.bar(range(len(cells)), accepted, color=colors)
    axis.bar(range(len(cells)), rejected, bottom=accepted, color="#E45756")
    style_axis(axis, "Frozen A1 output acceptance counts", "attempts", cell_labels)
    axis.legend(
        handles=[
            Patch(color="#4C78A8", label="legacy accepted"),
            Patch(color="#F58518", label="Givens accepted"),
            Patch(color="#E45756", label="rejected"),
        ]
    )
    mark_failures(axis, cells, [a + r for a, r in zip(accepted, rejected, strict=True)])
    file = "output_acceptance.png"
    manifest.append({"metric": "output_acceptance", "file": file, "sha256": save_plot(plots_dir / file, figure), "reading": "Accepted and rejected counts are preserved outputs of the frozen controller; telemetry does not feed the decision path."})
    return manifest


def git(root: pathlib.Path, *args: str) -> str:
    completed = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=True)
    return completed.stdout


def write_source_diff(root: pathlib.Path, authority: dict[str, Any], aggregate_dir: pathlib.Path) -> None:
    base = authority["source_base_sha"]
    head = authority["execution_head_sha"]
    changed = git(root, "diff", "--name-only", base, head).splitlines()
    forbidden = (
        "crates/rodas5p-integrators/src/bdf.rs",
        "crates/rodas5p-integrators/src/radau.rs",
        "crates/rodas5p-integrators/src/homotopy.rs",
        "crates/rodas5p-integrators/src/homotopy_policy.rs",
        "crates/rodas5p-fair-ab/src/adaptive_global_error.rs",
        "research/a1_inner_tolerance_audit_20260825/A1_TWO_ARM_AUTHORITY_RECEIPT.json",
        "Cargo.lock",
    )
    forbidden_changed = [path for path in changed if path in forbidden]
    stat = git(root, "diff", "--stat", base, head).rstrip()
    text = "\n".join(
        [
            f"source_base={base}",
            f"execution_head={head}",
            f"execution_tree={authority['execution_head_tree']}",
            f"command=git diff --binary {base} {head}",
            f"changed_paths={len(changed)}",
            f"forbidden_paths_changed={len(forbidden_changed)}",
            "",
            "changed_path_list:",
            *changed,
            "",
            "diff_stat:",
            stat,
            "",
        ]
    )
    (aggregate_dir / "source_diff.txt").write_text(text, encoding="utf-8")
    (aggregate_dir / "source_diff.patch").write_text(
        git(root, "diff", "--binary", base, head), encoding="utf-8"
    )
    if forbidden_changed:
        raise ValueError(f"forbidden source paths changed: {forbidden_changed}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True, type=pathlib.Path)
    parser.add_argument("--campaign-dir", required=True, type=pathlib.Path)
    args = parser.parse_args()
    root = args.repo_root.resolve()
    campaign = (root / args.campaign_dir).resolve()
    evidence = campaign / "evidence"
    aggregate_dir = campaign / "aggregate"
    plots_dir = campaign / "plots"
    authority = load(aggregate_dir / "campaign-authority.json")

    spec = importlib.util.spec_from_file_location(
        "validate_k0_cells", campaign / "schema/validate_k0_cells.py"
    )
    assert spec is not None and spec.loader is not None
    validator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(validator)
    validation = validator.validate_evidence_dir(evidence)

    cell_paths = sorted(evidence.glob("*.json"))
    raw_cells = [load(path) for path in cell_paths]
    cells = sorted(
        (summarize_cell(cell) for cell in raw_cells),
        key=lambda row: (row["family"], row["kernel_arm"]),
    )
    manifest = {path.name: digest(path) for path in cell_paths}
    log_manifest = {path.name: digest(path) for path in sorted(evidence.glob("*.stderr.log"))}
    aggregate = {
        "schema": "vigilode-k0-stage-telemetry-aggregate/v1",
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
        "verdict": validation["verdict"],
        "claim_boundary": "Raw same-profile, same-tolerance receipt, residual, geometry, and checked-work observations only; no performance, ranking, activation, or promotion inference.",
        "execution_identity": validation["execution_identity"],
        "profile": authority["profile"],
        "tolerance": authority["tolerance"],
        "kernel_arms": authority["kernel_arms"],
        "families": authority["families"],
        "expected_cell_count": 12,
        "observed_cell_count": len(cells),
        "cell_states": validation["cell_states"],
        "cell_content_manifest": manifest,
        "stderr_content_manifest": log_manifest,
        "stage_totals_by_kernel": totals_by_kernel(cells, tuple(authority["kernel_arms"])),
        "cells": cells,
    }
    write_json(aggregate_dir / "K0_STAGE_TELEMETRY_AGGREGATE.json", aggregate)
    (aggregate_dir / "K0_STAGE_TELEMETRY_AGGREGATE.md").write_text(
        render_markdown(aggregate), encoding="utf-8"
    )

    failures = [
        {
            "family": cell["family"],
            "kernel_arm": cell["kernel_arm"],
            "execution_state": cell["execution_state"],
            "error": cell.get("error"),
        }
        for cell in raw_cells
        if cell["execution_state"] != "COMPLETE"
    ]
    write_json(
        aggregate_dir / "failure_ledger.json",
        {
            "schema": "vigilode-k0-failure-ledger/v1",
            "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
            "cell_states": validation["cell_states"],
            "failures": failures,
        },
    )

    records = [load(path) for path in sorted((aggregate_dir / "command-records").glob("*.json"))]
    if len(records) != 12:
        raise ValueError(f"expected 12 command records, found {len(records)}")
    write_json(
        aggregate_dir / "executed_commands.json",
        {
            "schema": "vigilode-k0-executed-commands/v1",
            "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
            "commands": records,
        },
    )
    plots = build_plots(cells, plots_dir)
    write_json(
        plots_dir / "plot_manifest.json",
        {
            "schema": "vigilode-k0-plot-manifest/v1",
            "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
            "plots": plots,
        },
    )
    write_source_diff(root, authority, aggregate_dir)
    print(json.dumps({"verdict": aggregate["verdict"], "cells": len(cells), "plots": len(plots)}))
    return 0 if aggregate["verdict"] == "COMPLETE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
