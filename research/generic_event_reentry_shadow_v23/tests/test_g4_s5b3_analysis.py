from __future__ import annotations

import importlib.util
from pathlib import Path

import pandas as pd


SCRIPT = Path(__file__).parents[1] / "scripts" / "analyze_g4_s5b3_attempt_geometry.py"
SPEC = importlib.util.spec_from_file_location("g4_s5b3_analysis", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def test_threshold_sweep_includes_both_boundary_sentinels_without_selection() -> None:
    frame = pd.DataFrame(
        {
            "eta_c2": [0.1, 0.2, 0.3, 0.4],
            "finite_eta": [True, True, True, True],
            "accepted": [True, True, False, False],
            "rejected_error_control": [False, False, True, True],
        }
    )
    sweep = MODULE.threshold_sweep(frame)
    assert len(sweep) == 6
    assert sweep.iloc[0]["hypothetically_admitted"] == 0
    assert sweep.iloc[-1]["hypothetically_admitted"] == 4
    assert "selected_threshold" not in sweep.columns


def test_rank_auc_collision_and_tradeoff_are_threshold_free() -> None:
    frame = pd.DataFrame(
        {
            "trajectory_id": ["a", "b", "c", "d"],
            "problem_id": ["p", "p", "p", "p"],
            "rtol": [1e-4, 1e-6, 1e-8, 1e-8],
            "attempt_index": [0, 0, 0, 1],
            "step_size": [0.1, 0.1, 0.1, 0.05],
            "eta_c2": [0.1, 0.1, 0.1, 0.4],
            "finite_eta": [True, True, True, True],
            "accepted": [True, True, False, False],
            "rejected_error_control": [False, False, True, True],
            "outcome": [
                "accepted",
                "accepted",
                "rejected-error-control",
                "rejected-error-control",
            ],
            "total_error_norm": [0.1, 0.2, 2.0, 3.0],
        }
    )
    sweep = MODULE.threshold_sweep(frame)
    summary = MODULE.threshold_tradeoff_summary(sweep)
    collisions = MODULE.exact_cross_outcome_collisions(frame)
    assert MODULE.rank_auc(frame) == 0.75
    assert len(collisions) == 1
    assert collisions[0]["eta_c2"] == 0.1
    assert summary["best_zero_false_accept"]["accepted_retained"] == 0
    assert "selected_threshold" not in summary
    frontier = MODULE.pareto_frontier(sweep)
    assert not frontier.empty
