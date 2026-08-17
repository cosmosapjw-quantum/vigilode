from __future__ import annotations

import importlib.util
from pathlib import Path

import pandas as pd

SCRIPT = Path(__file__).parents[1] / "scripts" / "analyze_p1_00_tolerance_scaled_early_defect.py"
spec = importlib.util.spec_from_file_location("p1analysis", SCRIPT)
mod = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(mod)


def synthetic_frame() -> pd.DataFrame:
    return pd.DataFrame(
        {
            "problem_id": ["a", "a", "b", "b"],
            "attempt_index": [0, 1, 0, 1],
            "outcome": ["accepted", "accepted", "rejected-error-control", "accepted"],
            "accepted": [True, True, False, True],
            "rejected": [False, False, True, False],
            "eta_c2": [1.0, 1.0, 1.0, 0.5],
            "rho_c2_wrms": [0.1, 0.2, 5.0, 0.05],
            "total_error_norm": [0.1, 0.2, 2.0, 0.05],
            "rtol": [1e-4, 1e-6, 1e-8, 1e-4],
            "atol": [1e-6, 1e-8, 1e-10, 1e-6],
            "trajectory_id": ["a0", "a1", "b0", "b1"],
            "step_size": [0.1] * 4,
        }
    )


def test_complete_sweep_has_boundary_sentinels_and_no_threshold_selection_side_effect():
    frame = synthetic_frame()
    sweep = mod.threshold_sweep(frame, "rho_c2_wrms")
    assert len(sweep) == frame["rho_c2_wrms"].nunique() + 2
    summary = mod.tradeoff_summary(sweep)
    assert summary["minimum_total_misclassification"]["strictly_better_than_all_admit"]
    assert summary["descriptive_best_zero_false_accept"]["false_accept"] == 0


def test_exact_collision_distinguishes_raw_control_from_scaled_score():
    frame = synthetic_frame()
    assert mod.exact_cross_outcome_collisions(frame, "eta_c2")
    assert not mod.exact_cross_outcome_collisions(frame, "rho_c2_wrms")


def test_entry_window_is_predeclared_attempt_indices_zero_and_one():
    frame = synthetic_frame()
    frame.loc[len(frame)] = frame.iloc[-1].copy()
    frame.loc[len(frame) - 1, "attempt_index"] = 2
    selected = mod.VIEWS["entry"](frame)
    assert selected["attempt_index"].max() <= 1
