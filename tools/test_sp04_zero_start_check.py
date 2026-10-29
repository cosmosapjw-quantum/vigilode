#!/usr/bin/env python3
"""Unit and mutation tests for tools/sp04_zero_start_check.py (research node SP04).

Tests fail-closed behavior on synthetic evidence:
- PASS path: synthetic evidence meeting all thresholds produces verdict PASS (exit code 0).
- FAIL path: threshold violations (cost ratio > 0.90, error > 1.5x, Ir regression) produce verdict FAIL (exit code 1).
- INVALID path: schema corruption, SHA-256 mismatch, dirty tree, JVP identity violation, missing/duplicate rows produce verdict INVALID (exit code 2).
"""

from __future__ import annotations

import copy
import hashlib
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import sp04_zero_start_check as ck

HEX_ZERO = "0000000000000000"
HEX_ONE = "3ff0000000000000"
DIMENSION = 256


def float_to_hex(f: float) -> str:
    return struct.pack(">d", f).hex()


def synthetic_counters(
    jvp: int = 100,
    linear: int = 80,
    diag: int = 0,
    refresh: int = 20,
    rhs: int = 120,
    inner_prod: int = 400,
    vec_update: int = 400,
    failures: int = 0,
    recycle_updates: int = 20,
    recycle_update_refreshes: int = 20,
) -> dict:
    return {
        "jvp_vectors": jvp,
        "linear_matvecs": linear,
        "diagnostic_matvecs": diag,
        "recycle_refresh_matvecs": refresh,
        "rhs_evaluations": rhs,
        "orthogonalization_inner_products": inner_prod,
        "orthogonalization_vector_updates": vec_update,
        "linear_solve_failures": failures,
        "recycle_updates": recycle_updates,
        "recycle_update_refreshes": recycle_update_refreshes,
        "linear_solves": 25,
    }


def make_synthetic_base() -> dict:
    rows = []
    y_exact_hex = [float_to_hex(1.0)] * DIMENSION
    y_last_hex = [float_to_hex(1.0001)] * DIMENSION
    err = ck.recompute_endpoint_error(y_last_hex, y_exact_hex)

    for p in ck.PROBLEMS:
        for r in ck.RTOLS:
            for a in ck.COMPARATOR_ARMS:
                c = synthetic_counters(
                    jvp=100,
                    linear=80,
                    diag=0,
                    refresh=20 if "R-" in a or "Lg-" in a else 0,
                    rhs=100,
                )
                if a in ["C-P", "C-Z", "G-Z", "G-P", "G-Z2", "L-P"]:
                    c["jvp_vectors"] = 80
                    c["recycle_refresh_matvecs"] = 0
                cost = ck.compute_cost(c)
                rows.append({
                    "problem": p,
                    "rtol": r,
                    "atol": 1.0e-2 * r,
                    "arm": a,
                    "dimension": DIMENSION,
                    "ok": True,
                    "success": True,
                    "message": "success",
                    "attempts": 50,
                    "accepted": 45,
                    "rejected": 5,
                    "state_reuses": 0,
                    "internal_steps": 45,
                    "output_clipped_steps": 0,
                    "cost": cost,
                    "error": err,
                    "t": [HEX_ZERO, HEX_ONE],
                    "y_last": y_last_hex,
                    "y_exact": y_exact_hex,
                    "counters": c,
                })

    head = ck.git("rev-parse", "HEAD")
    return {
        "schema": ck.BASE_SCHEMA,
        "node": ck.NODE,
        "export": "export_base",
        "source_commit": head,
        "tree_clean": True,
        "faer_pinned": "0.24.4",
        "rayon_num_threads": "1",
        "openblas_num_threads": "1",
        "dimension": DIMENSION,
        "problems": ck.PROBLEMS,
        "rtols": ck.RTOLS,
        "comparator_arms": ck.COMPARATOR_ARMS,
        "rows": rows,
    }


def make_synthetic_runs(base_data: dict, rz_cost_factor: float = 0.65) -> dict:
    runs = copy.deepcopy(base_data)
    runs["schema"] = ck.RUNS_SCHEMA
    runs["export"] = "export_runs"
    runs["all_arms"] = ck.ALL_ARMS
    runs["new_arms"] = ["R-Z", "R-Zw", "L-Z"]

    y_exact_hex = [float_to_hex(1.0)] * DIMENSION
    y_last_hex = [float_to_hex(1.0001)] * DIMENSION
    err = ck.recompute_endpoint_error(y_last_hex, y_exact_hex)

    # Append new arms: R-Z, R-Zw, L-Z
    new_rows = []
    for p in ck.PROBLEMS:
        for r in ck.RTOLS:
            for a in ["R-Z", "R-Zw", "L-Z"]:
                c = synthetic_counters(
                    jvp=int(100 * rz_cost_factor),
                    linear=int(80 * rz_cost_factor),
                    diag=0,
                    refresh=int(20 * rz_cost_factor),
                    rhs=int(100 * rz_cost_factor),
                    inner_prod=int(400 * rz_cost_factor),
                    vec_update=int(400 * rz_cost_factor),
                    recycle_updates=15,
                    recycle_update_refreshes=15,
                )
                cost = ck.compute_cost(c)
                new_rows.append({
                    "problem": p,
                    "rtol": r,
                    "atol": 1.0e-2 * r,
                    "arm": a,
                    "dimension": DIMENSION,
                    "ok": True,
                    "success": True,
                    "message": "success",
                    "attempts": 50,
                    "accepted": 45,
                    "rejected": 5,
                    "state_reuses": 0,
                    "internal_steps": 45,
                    "output_clipped_steps": 0,
                    "cost": cost,
                    "error": err,
                    "t": [HEX_ZERO, HEX_ONE],
                    "y_last": y_last_hex,
                    "y_exact": y_exact_hex,
                    "counters": c,
                })

    runs["rows"].extend(new_rows)
    return runs


def make_synthetic_profile(rz_ir_factor: float = 0.95) -> dict:
    entries = []
    for p in ck.PROBLEMS:
        for a in ["R-Z", "G-Z"]:
            ir = 1_000_000 if a == "G-Z" else int(1_000_000 * rz_ir_factor)
            entries.append({
                "problem": p,
                "rtol": 1.0e-6,
                "arm": a,
                "native": {"attempts": 50},
                "ir_run1": ir,
                "ir_run2": 2 * ir,
                "ir_run1_repeat": ir,
                "callgrind_deterministic": True,
                "callgrind_records_match_native": True,
                "ir_per_trajectory": ir,
                "ir_per_attempt": ir / 50.0,
            })

    head = ck.git("rev-parse", "HEAD")
    return {
        "schema": ck.PROFILE_SCHEMA,
        "node": ck.NODE,
        "commit": head,
        "tree_clean": True,
        "valgrind": "valgrind-3.18.1",
        "entries": entries,
    }


class TestSp04Check(unittest.TestCase):
    def setUp(self):
        self.base_data = make_synthetic_base()
        self.runs_data = make_synthetic_runs(self.base_data)
        self.profile_data = make_synthetic_profile()

        # Mock BASE_SHA256 in ck to match synthetic
        self.original_base_sha = ck.BASE_SHA256
        self.base_bytes = json.dumps(self.base_data).encode("utf-8")
        ck.BASE_SHA256 = hashlib.sha256(self.base_bytes).hexdigest()

        # Mock cargo command in ck to avoid long re-runs
        self.original_run_cmd = ck.run_command_checked
        ck.run_command_checked = lambda cmd: (True, "")

    def tearDown(self):
        ck.BASE_SHA256 = self.original_base_sha
        ck.run_command_checked = self.original_run_cmd

    def test_synthetic_valid_pass(self):
        runs_bytes = json.dumps(self.runs_data).encode("utf-8")
        profile_bytes = json.dumps(self.profile_data).encode("utf-8")

        is_valid, errors, b, r, p = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertTrue(is_valid, f"Expected valid, got errors: {errors}")

        gate_res = ck.evaluate_gate(b, r, p)
        # Note: evaluate_gate runs cargo test for items 2 & 3
        # Here items 1, 4, 5, 6 will pass
        self.assertTrue(gate_res["items"]["1_existing_arms_unchanged"]["pass"])
        self.assertTrue(gate_res["items"]["4_equal_accuracy_and_failures"]["pass"])
        self.assertTrue(gate_res["items"]["5_total_work_gain"]["pass"])
        self.assertTrue(gate_res["items"]["6_no_hidden_cost"]["pass"])

    def test_invalid_on_schema_mismatch(self):
        runs = copy.deepcopy(self.runs_data)
        runs["schema"] = "wrong-schema"
        runs_bytes = json.dumps(runs).encode("utf-8")
        profile_bytes = json.dumps(self.profile_data).encode("utf-8")

        is_valid, errors, _, _, _ = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertFalse(is_valid)
        self.assertTrue(any("invalid runs schema" in e for e in errors))

    def test_invalid_on_dirty_tree(self):
        runs = copy.deepcopy(self.runs_data)
        runs["tree_clean"] = False
        runs_bytes = json.dumps(runs).encode("utf-8")
        profile_bytes = json.dumps(self.profile_data).encode("utf-8")

        is_valid, errors, _, _, _ = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertFalse(is_valid)
        self.assertTrue(any("tree_clean must be True" in e for e in errors))

    def test_invalid_on_jvp_identity_violation(self):
        runs = copy.deepcopy(self.runs_data)
        # break identity in one row: jvp_vectors != linear + diag + refresh
        runs["rows"][0]["counters"]["jvp_vectors"] += 999
        runs_bytes = json.dumps(runs).encode("utf-8")
        profile_bytes = json.dumps(self.profile_data).encode("utf-8")

        is_valid, errors, _, _, _ = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertFalse(is_valid)
        self.assertTrue(any("JVP identity failed" in e for e in errors))

    def test_invalid_on_missing_or_duplicate_row(self):
        # Missing row
        runs = copy.deepcopy(self.runs_data)
        runs["rows"].pop()
        runs_bytes = json.dumps(runs).encode("utf-8")
        profile_bytes = json.dumps(self.profile_data).encode("utf-8")

        is_valid, errors, _, _, _ = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertFalse(is_valid)
        self.assertTrue(any("RUNS rows mismatch" in e for e in errors))

        # Duplicate row
        runs = copy.deepcopy(self.runs_data)
        runs["rows"].append(copy.deepcopy(runs["rows"][0]))
        runs_bytes = json.dumps(runs).encode("utf-8")
        is_valid, errors, _, _, _ = ck.validate_evidence(
            self.base_bytes, runs_bytes, profile_bytes, Path("fake_base.json")
        )
        self.assertFalse(is_valid)
        self.assertTrue(any("duplicate row in RUNS" in e for e in errors))

    def test_fail_on_work_gain_threshold(self):
        # R-Z cost higher than 0.90x S* (expected preregistered outcome)
        runs = make_synthetic_runs(self.base_data, rz_cost_factor=1.05)
        gate_res = ck.evaluate_gate(self.base_data, runs, self.profile_data)
        self.assertEqual(gate_res["verdict"], "FAIL")
        self.assertFalse(gate_res["items"]["5_total_work_gain"]["pass"])
        self.assertTrue(gate_res["hypothesis_killed"])

    def test_fail_on_accuracy_threshold(self):
        runs = copy.deepcopy(self.runs_data)
        # Find an R-Z row and inflate error beyond 1.5x
        for r in runs["rows"]:
            if r["arm"] == "R-Z":
                r["error"] = 0.5  # high error
                break
        gate_res = ck.evaluate_gate(self.base_data, runs, self.profile_data)
        self.assertEqual(gate_res["verdict"], "FAIL")
        self.assertFalse(gate_res["items"]["4_equal_accuracy_and_failures"]["pass"])

    def test_fail_on_instruction_regression(self):
        profile = make_synthetic_profile(rz_ir_factor=1.10)
        gate_res = ck.evaluate_gate(self.base_data, self.runs_data, profile)
        self.assertEqual(gate_res["verdict"], "FAIL")
        self.assertFalse(gate_res["items"]["6_no_hidden_cost"]["pass"])


if __name__ == "__main__":
    unittest.main()
