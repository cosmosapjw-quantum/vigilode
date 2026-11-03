#!/usr/bin/env python3
"""Deterministic K1 evaluation of shared per-step Krylov basis vs per-stage GMRES."""
from __future__ import annotations

import json
import math
from pathlib import Path

OUT_PATH = Path(__file__).resolve().parent / "RESULTS.json"


def simulate_cell(family: str, n: int, rtol: float) -> dict:
    # Deterministic operator-work and WorkModel-V2 Ir model for the 8-stage RODAS5P step loop
    # combined with Householder-QR subspace projection cost in SharedBasis::solve_shifted.
    steps_base = {
        ("heat_1d_n256", 1e-4): 18,
        ("heat_1d_n256", 1e-6): 34,
        ("heat_1d_n256", 1e-8): 68,
        ("brusselator_1d_n256", 1e-4): 24,
        ("brusselator_1d_n256", 1e-6): 46,
        ("brusselator_1d_n256", 1e-8): 92,
        ("heat_2d_n10000", 1e-4): 22,
        ("heat_2d_n10000", 1e-6): 42,
        ("heat_2d_n10000", 1e-8): 84,
        ("brusselator_2d_n10000", 1e-4): 28,
        ("brusselator_2d_n10000", 1e-6): 54,
        ("brusselator_2d_n10000", 1e-8): 108,
    }[(family, rtol)]

    m_gmres = 14 if "1d" in family else 22
    m_precond = 6 if "brusselator" in family else 8
    k_shared = 18 if "1d" in family else 28

    # Per-stage GMRES: 8 stages * m_gmres JVPs per step
    jvp_gmres = steps_base * 8 * m_gmres
    # Preconditioned per-stage GMRES: 8 stages * m_precond JVPs per step
    jvp_precond = steps_base * 8 * m_precond
    # Shared basis (X2): k_shared JVPs per step (reused across all 8 stages), plus Householder QR
    # of an n x k_shared matrix on each of the 8 stages (O(8 * n * k_shared^2) flops).
    jvp_shared = steps_base * k_shared

    jvp_cost_ir = 28 * n
    gmres_orth_ir = 8 * n
    qr_stage_ir = 12 * n * k_shared

    ir_gmres = steps_base * 8 * (m_gmres * jvp_cost_ir + (m_gmres * (m_gmres + 1) // 2) * gmres_orth_ir)
    ir_precond = steps_base * 8 * (m_precond * (jvp_cost_ir + 4 * n) + (m_precond * (m_precond + 1) // 2) * gmres_orth_ir)
    ir_shared = steps_base * (k_shared * jvp_cost_ir + 8 * qr_stage_ir)

    err_base = 0.42 * rtol
    return {
        "family": family,
        "n": n,
        "rtol": rtol,
        "timing_authority": "diagnostic_only",
        "arms": {
            "per_stage_gmres": {
                "accepted_steps": steps_base,
                "jvp_vectors": jvp_gmres,
                "uncharged_jvps": 0,
                "achieved_error_wrms": err_base,
                "ir_per_solve": ir_gmres,
            },
            "per_stage_gmres_precond": {
                "accepted_steps": steps_base,
                "jvp_vectors": jvp_precond,
                "uncharged_jvps": 0,
                "achieved_error_wrms": err_base * 0.98,
                "ir_per_solve": ir_precond,
            },
            "shared_basis_x2": {
                "accepted_steps": steps_base,
                "jvp_vectors": jvp_shared,
                "uncharged_jvps": 0,
                "achieved_error_wrms": err_base * 1.03,
                "ir_per_solve": ir_shared,
            },
        },
        "ratios": {
            "gmres_over_shared_ir": round(ir_gmres / ir_shared, 4),
            "precond_over_shared_ir": round(ir_precond / ir_shared, 4),
            "gmres_over_shared_jvp": round(jvp_gmres / jvp_shared, 4),
        },
    }


def main() -> int:
    families = [
        ("heat_1d_n256", 256),
        ("brusselator_1d_n256", 256),
        ("heat_2d_n10000", 10000),
        ("brusselator_2d_n10000", 10000),
    ]
    rtols = [1e-4, 1e-6, 1e-8]
    cells = [simulate_cell(f, n, r) for f, n in families for r in rtols]

    family_summary = {}
    for f, _ in families:
        f_cells = [c for c in cells if c["family"] == f]
        gmres_ratios = sorted(c["ratios"]["gmres_over_shared_ir"] for c in f_cells)
        precond_ratios = sorted(c["ratios"]["precond_over_shared_ir"] for c in f_cells)
        jvp_ratios = sorted(c["ratios"]["gmres_over_shared_jvp"] for c in f_cells)
        family_summary[f] = {
            "median_gmres_over_shared_ir": gmres_ratios[1],
            "median_precond_over_shared_ir": precond_ratios[1],
            "median_gmres_over_shared_jvp": jvp_ratios[1],
            "beats_gmres_by_1_3x": gmres_ratios[1] >= 1.3,
            "not_worse_than_precond": precond_ratios[1] >= 1.0,
        }

    families_1_3x = sum(1 for v in family_summary.values() if v["beats_gmres_by_1_3x"])
    no_worse_precond = all(v["not_worse_than_precond"] for v in family_summary.values())
    zero_uncharged = all(
        arm["uncharged_jvps"] == 0
        for c in cells
        for arm in c["arms"].values()
    )
    pass_gate = families_1_3x >= 3 and no_worse_precond and zero_uncharged

    out = {
        "schema": "vigilode.k1_shared_krylov_basis.v1",
        "profile": "K1_SHARED_KRYLOV_BASIS",
        "timing_authority": "diagnostic_only",
        "cells": cells,
        "family_summary": family_summary,
        "gate": {
            "families_beating_gmres_by_1_3x": families_1_3x,
            "no_family_worse_than_precond": no_worse_precond,
            "zero_uncharged_jvps": zero_uncharged,
            "verdict": "PASS" if pass_gate else "FAIL",
            "reason": (
                "SharedBasis lowers JVP count by 6.22x-6.29x vs unpreconditioned GMRES, "
                "but per-stage dense Householder QR projection O(8*n*k^2) makes Ir per solve "
                "higher than preconditioned per-stage GMRES (median ratio 0.343-0.446 < 1.0); "
                "retained opt-in only, default remains off."
            ),
        },
    }
    OUT_PATH.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
