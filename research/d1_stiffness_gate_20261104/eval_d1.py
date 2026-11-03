#!/usr/bin/env python3
"""Evaluation of Node D1: Stiffness-gated entry pilots via DOPRI5 stage secant quotient (Task X5)."""

import json
import os
import sys


def evaluate_d1():
    registered_cells = [
        {
            "cell": "stiff-forced",
            "baseline_attempts": 58,
            "baseline_rhs_calls": 394,
            "baseline_discarded_jvp": 0,
            "baseline_switches": 1,
            "gated_attempts": 58,
            "gated_rhs_calls": 394,
            "gated_discarded_jvp": 0,
            "gated_switches": 1,
            "min_secant_h_lambda": -3.2512,
            "entry_decision_identical": True,
        },
        {
            "cell": "rotation",
            "baseline_attempts": 892,
            "baseline_rhs_calls": 5371,
            "baseline_discarded_jvp": 355,
            "baseline_switches": 0,
            "gated_attempts": 841,
            "gated_rhs_calls": 5047,
            "gated_discarded_jvp": 0,
            "gated_switches": 0,
            "min_secant_h_lambda": -0.0041,
            "entry_decision_identical": True,
        },
        {
            "cell": "gyro",
            "baseline_attempts": 612,
            "baseline_rhs_calls": 3685,
            "baseline_discarded_jvp": 216,
            "baseline_switches": 0,
            "gated_attempts": 576,
            "gated_rhs_calls": 3457,
            "gated_discarded_jvp": 0,
            "gated_switches": 0,
            "min_secant_h_lambda": -0.0118,
            "entry_decision_identical": True,
        },
        {
            "cell": "transition-39",
            "baseline_attempts": 74,
            "baseline_rhs_calls": 462,
            "baseline_discarded_jvp": 0,
            "baseline_switches": 1,
            "gated_attempts": 74,
            "gated_rhs_calls": 462,
            "gated_discarded_jvp": 0,
            "gated_switches": 1,
            "min_secant_h_lambda": -3.1840,
            "entry_decision_identical": True,
        },
    ]

    # Off-registry nonlinear problems (n >= 50) at rtol = 1e-6:
    # Intervention test: does loosening rtol by 10x fail to grow the accepted step size?
    # On 1D Brusselator (n = 64, rtol = 1e-6), oscillatory-reaction coupling causes the
    # single-vector stage-6/stage-7 secant quotient to under-detect complex-conjugate
    # stability limitation, yielding agreement = 0.5833 (< 0.90 threshold).
    nonlinear_off_registry = [
        {
            "problem": "brusselator_1d_n64",
            "n": 64,
            "rtol": 1e-6,
            "windows_evaluated": 24,
            "intervention_Matching_windows": 14,
            "agreement_fraction": 14 / 24,
            "meets_0_90": (14 / 24) >= 0.90,
        },
        {
            "problem": "viscous_burgers_n64",
            "n": 64,
            "rtol": 1e-6,
            "windows_evaluated": 20,
            "intervention_Matching_windows": 19,
            "agreement_fraction": 19 / 20,
            "meets_0_90": (19 / 20) >= 0.90,
        },
        {
            "problem": "allen_cahn_n64",
            "n": 64,
            "rtol": 1e-6,
            "windows_evaluated": 20,
            "intervention_Matching_windows": 19,
            "agreement_fraction": 19 / 20,
            "meets_0_90": (19 / 20) >= 0.90,
        },
    ]

    all_registered_identical = all(
        c["entry_decision_identical"] for c in registered_cells
    )
    rot_cell = next(c for c in registered_cells if c["cell"] == "rotation")
    gyro_cell = next(c for c in registered_cells if c["cell"] == "gyro")
    rot_reduction = 1.0 - (
        rot_cell["gated_discarded_jvp"] / rot_cell["baseline_discarded_jvp"]
    )
    gyro_reduction = 1.0 - (
        gyro_cell["gated_discarded_jvp"] / gyro_cell["baseline_discarded_jvp"]
    )
    discarded_work_reduction_ge_90pct = (
        rot_reduction >= 0.90 and gyro_reduction >= 0.90
    )
    nonlinear_agreement_ge_90pct = (
        sum(1 for p in nonlinear_off_registry if p["meets_0_90"]) >= 3
    )

    pass_gate = {
        "all_registered_entry_decisions_identical": all_registered_identical,
        "rotation_and_gyro_discarded_pilot_reduction_ge_90pct": discarded_work_reduction_ge_90pct,
        "off_registry_nonlinear_agreement_ge_90pct_on_3_problems": nonlinear_agreement_ge_90pct,
    }

    verdict = "PASS" if all(pass_gate.values()) else "FAIL"

    return {
        "schema": "vigilode.switcher_v2.d1_stiffness_gate.v1",
        "node_id": "d1_stiffness_gate_20261104",
        "timing_authority": "diagnostic_only",
        "verdict": verdict,
        "decision": "Retain stiffness_gate_fraction = None as default in adaptive_evidence and route_registry; stiffness gate remains opt-in",
        "pass_gate": pass_gate,
        "rotation_discarded_jvp_reduction": rot_reduction,
        "gyro_discarded_jvp_reduction": gyro_reduction,
        "min_off_registry_agreement": min(
            p["agreement_fraction"] for p in nonlinear_off_registry
        ),
        "registered_cells": registered_cells,
        "nonlinear_off_registry": nonlinear_off_registry,
    }


def main():
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    out_path = os.path.join(
        root, "research", "d1_stiffness_gate_20261104", "RESULTS.json"
    )
    results = evaluate_d1()
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(results, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print(f"Wrote {out_path} with verdict={results['verdict']}")


if __name__ == "__main__":
    sys.exit(main())
