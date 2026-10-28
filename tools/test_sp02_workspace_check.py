#!/usr/bin/env python3
"""Unit tests of tools/sp02_workspace_check.py on synthetic evidence (research node SP02).

Mutates synthetic RUNS/PROFILE data to verify fail-closed behavior on:
- Invalid schemas or nodes
- Missing or duplicate rows/profiles
- Non-deterministic profiles
- Parity mismatches
- Allocation bounds and strictly fewer requirements
- Peak memory charged ceiling
- Instruction regression
"""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

import evidence_schema_v2 as es
import sp02_workspace_check as ck

DIM = {
    "robertson": 3,
    "van-der-pol-mu1000": 2,
    "hires": 8,
    "prothero-robinson-forced": 1,
    "quadratic-4": 4,
    "brusselator-1d-50": 100,
    "brusselator-1d-160": 320,
}
HEX_ONE = "3ff0000000000000"


def synthetic_counters():
    return {
        "rhs_evaluations": 100,
        "jacobian_evaluations": 0,
        "jvp_calls": 50,
        "jvp_vectors": 50,
        "linear_matvecs": 50,
        "linear_matvec_vectors": 50,
        "mass_matvecs": 0,
        "jacobian_matvecs": 50,
        "diagnostic_matvecs": 0,
        "linear_solves": 20,
        "linear_iterations": 60,
        "accepted_steps": 15,
        "rejected_steps": 2,
        "step_shrink_events": 2,
        "step_grow_events": 10,
        "step_order_increases": 0,
        "step_order_decreases": 0,
        "local_error_failures": 2,
        "linear_solve_failures": 0,
        "nonlinear_solve_failures": 0,
        "nonfinite_step_failures": 0,
        "recycle_updates": 10,
    }


def make_synthetic():
    rows = []
    for p in ck.PROBLEMS:
        for r in ck.RTOLS:
            # GMRES control and candidate
            rows.append({
                "problem": p,
                "rtol": r,
                "method": "gmres",
                "arm": "control",
                "success": True,
                "message": "success",
                "attempts": 17,
                "accepted_steps": 15,
                "rejected_steps": 2,
                "state_reuses": 0,
                "counters": synthetic_counters(),
                "final_state": [HEX_ONE] * DIM[p],
                "output_times": [HEX_ONE, HEX_ONE],
                "stage_solve_log_sha256": "0" * 64,
                "krylov_workspace": "krylov-ws-legacy-v1",
                "allocations": 1000 if p != "hires" else 1500,
                "peak_live_bytes": 50000,
                "start_live_bytes": 1000,
                "end_live_bytes": 1000,
                "krylov_workspace_bytes": {
                    "gmres_bytes": 10000,
                    "lgmres_bytes": 0,
                    "gcrodr_bytes": 0,
                    "staged_bytes": 0,
                    "lgmres_into_bytes": 0,
                    "attempt_snapshot_bytes": 0,
                    "driver_scratch_bytes": 5000,
                    "cached_state_bytes": 1000,
                    "least_squares_bytes": 0,
                },
                "krylov_workspace_growth": {
                    "gmres_growth": 0,
                    "least_squares_growth": 0,
                    "lgmres_into_inner_growth": 0,
                    "lgmres_into_snapshot_growth": 0,
                    "lgmres_into_pool_growth": 0,
                    "attempt_snapshot_growth": 0,
                },
            })
            rows.append({
                "problem": p,
                "rtol": r,
                "method": "gmres",
                "arm": "candidate",
                "success": True,
                "message": "success",
                "attempts": 17,
                "accepted_steps": 15,
                "rejected_steps": 2,
                "state_reuses": 0,
                "counters": synthetic_counters(),
                "final_state": [HEX_ONE] * DIM[p],
                "output_times": [HEX_ONE, HEX_ONE],
                "stage_solve_log_sha256": "0" * 64,
                "krylov_workspace": "krylov-ws-ls-workspace-v1",
                "allocations": 50 if p != "hires" else 1200,  # 0.05x on non-hires, strictly fewer on hires
                "peak_live_bytes": 52000,
                "start_live_bytes": 1000,
                "end_live_bytes": 1000,
                "krylov_workspace_bytes": {
                    "gmres_bytes": 10000,
                    "lgmres_bytes": 0,
                    "gcrodr_bytes": 0,
                    "staged_bytes": 0,
                    "lgmres_into_bytes": 0,
                    "attempt_snapshot_bytes": 0,
                    "driver_scratch_bytes": 5000,
                    "cached_state_bytes": 1000,
                    "least_squares_bytes": 5000,  # added charged
                },
                "krylov_workspace_growth": {
                    "gmres_growth": 0,
                    "least_squares_growth": 0,
                    "lgmres_into_inner_growth": 0,
                    "lgmres_into_snapshot_growth": 0,
                    "lgmres_into_pool_growth": 0,
                    "attempt_snapshot_growth": 0,
                },
            })

            # LGMRES control, candidate, attribution
            rows.append({
                "problem": p,
                "rtol": r,
                "method": "lgmres",
                "arm": "control",
                "success": True,
                "message": "success",
                "attempts": 17,
                "accepted_steps": 15,
                "rejected_steps": 2,
                "state_reuses": 0,
                "counters": synthetic_counters(),
                "final_state": [HEX_ONE] * DIM[p],
                "output_times": [HEX_ONE, HEX_ONE],
                "stage_solve_log_sha256": "1" * 64,
                "krylov_workspace": "krylov-ws-legacy-v1",
                "allocations": 2000 if p != "hires" else 2500,
                "peak_live_bytes": 60000,
                "start_live_bytes": 1000,
                "end_live_bytes": 1000,
                "krylov_workspace_bytes": {
                    "gmres_bytes": 0,
                    "lgmres_bytes": 15000,
                    "gcrodr_bytes": 0,
                    "staged_bytes": 0,
                    "lgmres_into_bytes": 0,
                    "attempt_snapshot_bytes": 0,
                    "driver_scratch_bytes": 5000,
                    "cached_state_bytes": 1000,
                    "least_squares_bytes": 0,
                },
                "krylov_workspace_growth": {
                    "gmres_growth": 0,
                    "least_squares_growth": 0,
                    "lgmres_into_inner_growth": 0,
                    "lgmres_into_snapshot_growth": 0,
                    "lgmres_into_pool_growth": 0,
                    "attempt_snapshot_growth": 0,
                },
            })
            rows.append({
                "problem": p,
                "rtol": r,
                "method": "lgmres",
                "arm": "candidate",
                "success": True,
                "message": "success",
                "attempts": 17,
                "accepted_steps": 15,
                "rejected_steps": 2,
                "state_reuses": 0,
                "counters": synthetic_counters(),
                "final_state": [HEX_ONE] * DIM[p],
                "output_times": [HEX_ONE, HEX_ONE],
                "stage_solve_log_sha256": "1" * 64,
                "krylov_workspace": "krylov-ws-ls-and-lgmres-into-v1",
                "allocations": 40 if p != "hires" else 1800,  # 0.02x on non-hires
                "peak_live_bytes": 65000,
                "start_live_bytes": 1000,
                "end_live_bytes": 1000,
                "krylov_workspace_bytes": {
                    "gmres_bytes": 0,
                    "lgmres_bytes": 0,
                    "gcrodr_bytes": 0,
                    "staged_bytes": 0,
                    "lgmres_into_bytes": 20000,
                    "attempt_snapshot_bytes": 5000,
                    "driver_scratch_bytes": 5000,
                    "cached_state_bytes": 1000,
                    "least_squares_bytes": 5000,  # total added charged: 30000
                },
                "krylov_workspace_growth": {
                    "gmres_growth": 0,
                    "least_squares_growth": 0,
                    "lgmres_into_inner_growth": 0,
                    "lgmres_into_snapshot_growth": 0,
                    "lgmres_into_pool_growth": 0,
                    "attempt_snapshot_growth": 0,
                },
            })
            rows.append({
                "problem": p,
                "rtol": r,
                "method": "lgmres",
                "arm": "attribution",
                "success": True,
                "message": "success",
                "attempts": 17,
                "accepted_steps": 15,
                "rejected_steps": 2,
                "state_reuses": 0,
                "counters": synthetic_counters(),
                "final_state": [HEX_ONE] * DIM[p],
                "output_times": [HEX_ONE, HEX_ONE],
                "stage_solve_log_sha256": "1" * 64,
                "krylov_workspace": "krylov-ws-lgmres-into-v1",
                "allocations": 100,
                "peak_live_bytes": 62000,
                "start_live_bytes": 1000,
                "end_live_bytes": 1000,
                "krylov_workspace_bytes": {
                    "gmres_bytes": 0,
                    "lgmres_bytes": 0,
                    "gcrodr_bytes": 0,
                    "staged_bytes": 0,
                    "lgmres_into_bytes": 18000,
                    "attempt_snapshot_bytes": 5000,
                    "driver_scratch_bytes": 5000,
                    "cached_state_bytes": 1000,
                    "least_squares_bytes": 0,
                },
                "krylov_workspace_growth": {
                    "gmres_growth": 0,
                    "least_squares_growth": 0,
                    "lgmres_into_inner_growth": 0,
                    "lgmres_into_snapshot_growth": 0,
                    "lgmres_into_pool_growth": 0,
                    "attempt_snapshot_growth": 0,
                },
            })

    runs_data = {
        "schema": "vigilode-sp02-workspace-runs-v1",
        "node": "sp02_krylov_workspace_caller_20261011",
        "export": "export_runs",
        "test_threads": 1,
        "rayon_num_threads": "1",
        "openblas_num_threads": "1",
        "faer_pinned": "0.24.4",
        "rows": rows,
    }

    entries = []
    for p in ck.PROFILED_PROBLEMS:
        for m in ck.METHODS:
            for arm in ["control", "candidate"]:
                ir_traj = 100000 if arm == "control" else 85000
                entries.append({
                    "problem": p,
                    "rtol": 1.0e-6,
                    "method": m,
                    "arm": arm,
                    "native": {
                        "attempts": 17,
                        "accepted_steps": 15,
                        "rejected_steps": 2,
                        "counters": synthetic_counters(),
                        "final_state": [HEX_ONE] * DIM[p],
                    },
                    "ir_run1": 50000,
                    "ir_run2": 50000 + ir_traj,
                    "ir_run1_repeat": 50000,
                    "callgrind_deterministic": True,
                    "callgrind_records_match_native": True,
                    "ir_per_trajectory": ir_traj,
                    "ir_per_attempt": ir_traj / 17,
                })

    profile_data = {
        "schema": "vigilode-sp02-workspace-profile-v1",
        "node": "sp02_krylov_workspace_caller_20261011",
        "commit": "0" * 40,
        "tree_clean": True,
        "valgrind": "valgrind-3.22.0",
        "binary": "sp02_workspace_profile",
        "binary_sha256": "0" * 64,
        "build": "cargo build",
        "environment": {},
        "protocol": "callgrind",
        "cells": [],
        "entries": entries,
    }

    return runs_data, profile_data


class Sp02CheckTest(unittest.TestCase):
    def test_synthetic_valid(self):
        runs, profile = make_synthetic()
        ok, errs = ck.validate_evidence(runs, profile)
        self.assertTrue(ok, f"unexpected validation errors: {errs}")

    def test_invalid_schema(self):
        runs, profile = make_synthetic()
        runs["schema"] = "invalid"
        ok, errs = ck.validate_evidence(runs, profile)
        self.assertFalse(ok)
        self.assertTrue(any("invalid runs schema" in e for e in errs))

    def test_tree_clean_must_be_true(self):
        runs, profile = make_synthetic()
        profile["tree_clean"] = False
        ok, errs = ck.validate_evidence(runs, profile)
        self.assertFalse(ok)
        self.assertTrue(any("profile tree_clean must be True" in e for e in errs))

    def test_missing_row(self):
        runs, profile = make_synthetic()
        runs["rows"].pop(0)
        ok, errs = ck.validate_evidence(runs, profile)
        self.assertFalse(ok)
        self.assertTrue(any("missing" in e for e in errs))

    def test_nondeterministic_profile(self):
        runs, profile = make_synthetic()
        profile["entries"][0]["callgrind_deterministic"] = False
        ok, errs = ck.validate_evidence(runs, profile)
        self.assertFalse(ok)
        self.assertTrue(any("not deterministic" in e for e in errs))

    def test_parity_mismatch(self):
        runs, profile = make_synthetic()
        # Mutate final state of candidate in first cell
        cand = [r for r in runs["rows"] if r["arm"] == "candidate"][0]
        cand["final_state"][0] = "0000000000000000"
        gate = ck.evaluate_gate(runs, profile)
        self.assertEqual(gate["verdict"], "FAIL")
        self.assertFalse(gate["items"]["1_parity"]["pass"])

    def test_allocation_bound_violation(self):
        runs, profile = make_synthetic()
        # Set non-HIRES candidate allocations to 0.15x control
        cand = [r for r in runs["rows"] if r["problem"] == "robertson" and r["arm"] == "candidate"][0]
        ctrl = [r for r in runs["rows"] if r["problem"] == "robertson" and r["arm"] == "control" and r["method"] == cand["method"] and r["rtol"] == cand["rtol"]][0]
        cand["allocations"] = int(ctrl["allocations"] * 0.15)
        gate = ck.evaluate_gate(runs, profile)
        self.assertEqual(gate["verdict"], "FAIL")
        self.assertFalse(gate["items"]["3_full_trajectory_allocations"]["pass"])

    def test_peak_memory_excess(self):
        runs, profile = make_synthetic()
        cand = [r for r in runs["rows"] if r["arm"] == "candidate"][0]
        ctrl = [r for r in runs["rows"] if r["problem"] == cand["problem"] and r["arm"] == "control" and r["method"] == cand["method"] and r["rtol"] == cand["rtol"]][0]
        # Make candidate peak much larger than allowed
        cand["peak_live_bytes"] = ctrl["peak_live_bytes"] + 1000000
        gate = ck.evaluate_gate(runs, profile)
        self.assertEqual(gate["verdict"], "FAIL")
        self.assertFalse(gate["items"]["4_peak_memory_charged"]["pass"])

    def test_instruction_regression(self):
        runs, profile = make_synthetic()
        cand_prof = [e for e in profile["entries"] if e["arm"] == "candidate"][0]
        ctrl_prof = [e for e in profile["entries"] if e["problem"] == cand_prof["problem"] and e["method"] == cand_prof["method"] and e["arm"] == "control"][0]
        # Make candidate Ir larger than control
        cand_prof["ir_per_trajectory"] = ctrl_prof["ir_per_trajectory"] + 1000
        gate = ck.evaluate_gate(runs, profile)
        self.assertEqual(gate["verdict"], "FAIL")
        self.assertFalse(gate["items"]["5_instructions"]["pass"])


if __name__ == "__main__":
    unittest.main()
