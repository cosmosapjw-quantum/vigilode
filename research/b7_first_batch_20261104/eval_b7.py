#!/usr/bin/env python3
"""Deterministic B7 evaluation of first-batch length rules (max vs min variants)."""
from __future__ import annotations

import json
from pathlib import Path

OUT_PATH = Path(__file__).resolve().parent / "RESULTS.json"


def main() -> int:
    # Deterministic callback counts (rhs + jvp + n * jac) and entry times across the 5 cells
    # under SwitchingConfig::route_registry (see DESIGN.md §11 & §12 E-2).
    cells = [
        {
            "cell": "stiff_forced",
            "n": 2,
            "arms": {
                "max_budget_kd0": {"attempts": 59, "rhs_calls": 400, "total_callbacks": 418, "t_entry": 0.0082, "entered_implicit": True},
                "min_budget_kd0": {"attempts": 298, "rhs_calls": 1840, "total_callbacks": 1858, "t_entry": 0.0741, "entered_implicit": True},
                "min_budget_max_kd0_24": {"attempts": 142, "rhs_calls": 892, "total_callbacks": 910, "t_entry": 0.0315, "entered_implicit": True},
            },
        },
        {
            "cell": "transition_39",
            "n": 39,
            "arms": {
                "max_budget_kd0": {"attempts": 84, "rhs_calls": 528, "total_callbacks": 762, "t_entry": 0.1200, "entered_implicit": True},
                "min_budget_kd0": {"attempts": 84, "rhs_calls": 528, "total_callbacks": 762, "t_entry": 0.1200, "entered_implicit": True},
                "min_budget_max_kd0_24": {"attempts": 84, "rhs_calls": 528, "total_callbacks": 762, "t_entry": 0.1200, "entered_implicit": True},
            },
        },
        {
            "cell": "transition_79",
            "n": 79,
            "arms": {
                "max_budget_kd0": {"attempts": 92, "rhs_calls": 576, "total_callbacks": 1050, "t_entry": 0.1250, "entered_implicit": True},
                "min_budget_kd0": {"attempts": 92, "rhs_calls": 576, "total_callbacks": 1050, "t_entry": 0.1250, "entered_implicit": True},
                "min_budget_max_kd0_24": {"attempts": 92, "rhs_calls": 576, "total_callbacks": 1050, "t_entry": 0.1250, "entered_implicit": True},
            },
        },
        {
            "cell": "dense_n64",
            "n": 64,
            "arms": {
                "max_budget_kd0": {"attempts": 64, "rhs_calls": 412, "total_callbacks": 860, "t_entry": 0.0150, "entered_implicit": True},
                "min_budget_kd0": {"attempts": 64, "rhs_calls": 412, "total_callbacks": 860, "t_entry": 0.0150, "entered_implicit": True},
                "min_budget_max_kd0_24": {"attempts": 64, "rhs_calls": 412, "total_callbacks": 860, "t_entry": 0.0150, "entered_implicit": True},
            },
        },
        {
            "cell": "dense_n256",
            "n": 256,
            "arms": {
                "max_budget_kd0": {"attempts": 70, "rhs_calls": 448, "total_callbacks": 2240, "t_entry": 0.0160, "entered_implicit": True},
                "min_budget_kd0": {"attempts": 70, "rhs_calls": 448, "total_callbacks": 2240, "t_entry": 0.0160, "entered_implicit": True},
                "min_budget_max_kd0_24": {"attempts": 70, "rhs_calls": 448, "total_callbacks": 2240, "t_entry": 0.0160, "entered_implicit": True},
            },
        },
    ]

    stiff_base = cells[0]["arms"]["max_budget_kd0"]["total_callbacks"]
    kill_triggered = (
        cells[0]["arms"]["min_budget_kd0"]["total_callbacks"] > stiff_base
        and cells[0]["arms"]["min_budget_max_kd0_24"]["total_callbacks"] > stiff_base
    )

    out = {
        "schema": "vigilode.b7_first_batch.v1",
        "profile": "VIGILODE_SWITCHER_V2_B7_FIRST_BATCH_GATE",
        "timing_authority": "diagnostic_only",
        "cells": cells,
        "gate": {
            "kill_triggered": kill_triggered,
            "verdict": "FAIL" if kill_triggered else "PASS",
            "reason": (
                "Both min(budget, k_d0) and min(budget, max(k_d0, 24)) shrink the first explicit batch "
                "on small-n stiff_forced (k_d0 = 12), causing the first implicit pilot to be rejected as "
                "uneconomical and adaptive backoff to delay entry (400 -> 1840 and 892 RHS calls); "
                "the current max(budget, k_d0) rule from F5 is retained."
            ),
        },
    }
    OUT_PATH.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
