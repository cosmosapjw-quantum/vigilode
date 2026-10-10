#!/usr/bin/env python3
"""Unit tests of tools/sp01_dupfix_check.py on synthetic evidence (research node SP01).

No solver, cargo or git command is run: the gate functions are called directly on synthetic RUNS/PROFILE/trial
documents written to a temporary directory, and every mutation must turn the expected gate item to FAIL or the
evidence to INVALID.

Run: python3 -m unittest discover -s tools -p test_sp01_dupfix_check.py
"""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as es  # noqa: E402
import sp01_dupfix_check as ck  # noqa: E402

DIM = {"robertson": 3, "van-der-pol-mu1000": 2, "hires": 8, "brusselator-1d-50": 100,
       "prothero-robinson-forced": 1, "quadratic-4": 4, "brusselator-1d-160": 320}
ONE = "3ff0000000000000"


def counters(jvp: int, diagnostic: int) -> dict:
    c = {k: 0 for k in es.COUNTERS_FULL}
    c.update(jvp_calls=jvp, jvp_vectors=jvp, linear_matvec_vectors=jvp, diagnostic_matvecs=diagnostic,
             linear_matvecs=jvp - diagnostic, linear_solves=10, linear_iterations=40, accepted_steps=9,
             rejected_steps=1)
    return c


def record(path: str, dim: int, jvp: int, diagnostic: int) -> dict:
    log = {"count": 80, "iterations": 400, "sha256": "ab" * 32}
    if path.startswith("uform"):
        return {"ok": True, "success": True, "message": "success", "t": ["0000000000000000", ONE],
                "y_last": [ONE] * dim, "attempts": 10, "accepted": 9, "rejected": 1, "state_reuses": 1,
                "internal_steps": 9, "output_clipped_steps": 0, "counters": counters(jvp, diagnostic),
                "solve_log": log}
    if path == "kform_integrate":
        return {"ok": True, "success": True, "message": "success", "t": ["0000000000000000", ONE],
                "y_last": [ONE] * dim, "attempts": 10, "accepted": 9, "rejected": 1, "linear_solve_failures": 0,
                "error_norms": ["3fe0000000000000"] * 9 + ["7ff0000000000000"], "internal_steps": 9,
                "output_clipped_steps": 0, "counters": counters(jvp, diagnostic), "solve_log": log}
    return {"ok": True, "success": True, "t_last": [ONE], "y_last": [ONE] * dim, "attempts": 10, "accepted": 9,
            "rejected": 1, "error_norms": ["3fe0000000000000"] * 10, "failed_attempts": [],
            "counters": counters(jvp, diagnostic), "solve_log": log}


def observer(confirmed: int) -> dict:
    return {"jvp_applications": 100, "confirmed_exits": confirmed, "unconfirmed_duplicates": 0, "zero_inputs": 5,
            "other_events": 50}


def synthetic():
    rows = []
    for path, case, rtol in ck.REGISTERED_CELLS:
        dim = DIM[case]
        v1, v2 = record(path, dim, 100, 10), record(path, dim, 95, 5)
        arms = {ck.V1: v1, ck.V2: v2}
        if path == "uform_into":
            arms["dup_fix"] = record(path, dim, 90, 0)
        rows.append({"path": path, "case": case, "rtol": rtol, "dimension": dim, "arms": arms,
                     "observed_arms": copy.deepcopy({ck.V1: v1, ck.V2: v2}),
                     "observer": {ck.V1: observer(5), ck.V2: observer(0)}})
    runs = {"node": ck.NODE, "export": "export_runs", "accountings": dict(ck.ACCOUNTING_IDS),
            "default_accounting": ck.ACCOUNTING_IDS[ck.V1], "uform_budget": 200, "kform_linear_config": "cfg",
            "rows": rows}
    entries = []
    for path, case, rtol in ck.PROFILED:
        for acc, jvp, diag, ir2 in ((ck.V1, 100, 10, 2000), (ck.V2, 95, 5, 1900)):
            native = {"success": True, "attempts": 10, "accepted_steps": 9, "rejected_steps": 1,
                      "counters": counters(jvp, diag), "final_state": [ONE] * DIM[case], "logged_solves": 80,
                      "path": path, "problem": case, "rtol": rtol, "accounting": ck.ACCOUNTING_IDS[acc],
                      "repetitions": 1}
            entries.append({"path": path, "problem": case, "rtol": rtol, "accounting": acc, "native": native,
                            "ir_run1": 1000, "ir_run2": ir2, "ir_run1_repeat": 1000, "callgrind_deterministic": True,
                            "callgrind_records_match_native": True, "ir_per_trajectory": ir2 - 1000,
                            "ir_per_attempt": (ir2 - 1000) / 10})
    profile = {"schema": "vigilode-sp01-dupfix-profile-v1", "node": ck.NODE, "commit": "c" * 40, "tree_clean": True,
               "valgrind": "valgrind-3.22.0", "binary": "sp01_dupfix_profile", "binary_sha256": "d" * 64,
               "build": "b", "environment": {}, "protocol": "p", "cells": [], "entries": entries}
    trial = {"node": ck.NODE, "trial": "t", "tree": "x", "commands": [], "steps": [
        {"step": "s", "command": "c", "exit": 101, "passed": 10, "failed": 1, "ignored": 0, "failing_tests": ["t"]}],
        "failing_pre_existing_tests": ["crate::t"], "outcome": "versioned"}
    alg04 = {"rows": [{"group": "C1", "case": c, "rtol": r, "arms": {"legacy": {}, "dup_fix": {}}}
                      for c in ck.SPD07 for r in ck.RTOLS]}
    return runs, profile, trial, alg04


class Sp01CheckTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.tmp = Path(self.dir.name)
        self.runs, self.profile, self.trial, self.alg04 = synthetic()

    def tearDown(self):
        self.dir.cleanup()

    def write(self, name, doc, raw=None):
        path = self.tmp / name
        path.write_text(raw if raw is not None else json.dumps(doc))
        return path

    def validated(self, runs=None, profile=None, trial=None, raw_runs=None):
        return ck.validate(self.write("runs.json", runs or self.runs, raw_runs),
                           self.write("profile.json", profile or self.profile),
                           self.write("trial.json", trial or self.trial),
                           self.write("alg04.json", self.alg04), None)

    def gates(self, runs=None, profile=None):
        v = self.validated(runs=runs, profile=profile)
        rows = ck.rows_by_key(v.docs["runs"])
        contract = {"pass": True}
        return (ck.item_parity(rows, contract), ck.item_accounting(rows),
                ck.item_instructions(v.docs["profile"], rows), ck.reported(rows, v.docs["alg04_runs"]))

    def test_valid_synthetic_evidence_passes_items_1_to_3(self):
        parity, accounting, instructions, reported = self.gates()
        self.assertTrue(parity["pass"])
        self.assertEqual(parity["cells_checked"], 44)
        self.assertTrue(accounting["pass"], accounting["failing_cells"])
        self.assertTrue(instructions["pass"], instructions["cells"])
        self.assertAlmostEqual(instructions["cells"][0]["ir_ratio_v2_over_v1"], 0.9)
        self.assertEqual(len(reported["jvp_ratio_v2_over_v1"]), 44)

    def test_a_state_difference_fails_parity(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][3]["arms"][ck.V2]["y_last"][0] = "3ff0000000000001"
        self.assertFalse(self.gates(runs=runs)[0]["pass"])

    def test_a_solve_log_difference_fails_parity(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][40]["arms"][ck.V2]["solve_log"]["sha256"] = "cd" * 32
        self.assertFalse(self.gates(runs=runs)[0]["pass"])

    def test_an_iteration_difference_fails_parity(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][20]["arms"][ck.V2]["counters"]["linear_iterations"] += 1
        self.assertFalse(self.gates(runs=runs)[0]["pass"])

    def test_a_failed_contract_test_fails_parity(self):
        v = self.validated()
        self.assertFalse(ck.item_parity(ck.rows_by_key(v.docs["runs"]), {"pass": False})["pass"])

    def test_a_jvp_difference_off_the_count_fails_accounting(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][5]["observer"][ck.V1]["confirmed_exits"] = 4
        self.assertFalse(self.gates(runs=runs)[1]["pass"])

    def test_another_counter_difference_fails_accounting(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][7]["arms"][ck.V2]["counters"]["rhs_calls"] = 3
        self.assertFalse(self.gates(runs=runs)[1]["pass"])

    def test_an_observed_run_that_differs_fails_accounting(self):
        runs = copy.deepcopy(self.runs)
        runs["rows"][9]["observed_arms"][ck.V1]["attempts"] = 11
        self.assertFalse(self.gates(runs=runs)[1]["pass"])

    def test_an_instruction_regression_fails_item_3(self):
        profile = copy.deepcopy(self.profile)
        e = next(e for e in profile["entries"] if e["accounting"] == ck.V2)
        e["ir_run2"], e["ir_per_trajectory"] = 2001, 1001
        self.assertFalse(self.gates(profile=profile)[2]["pass"])

    def test_a_nondeterministic_or_unbound_profile_fails_item_3(self):
        for field, value in (("callgrind_deterministic", False), ("callgrind_records_match_native", False)):
            profile = copy.deepcopy(self.profile)
            profile["entries"][0][field] = value
            self.assertFalse(self.gates(profile=profile)[2]["pass"], field)
        profile = copy.deepcopy(self.profile)
        profile["entries"][1]["native"]["attempts"] = 12
        self.assertFalse(self.gates(profile=profile)[2]["pass"])

    def test_malformed_evidence_is_invalid(self):
        runs = copy.deepcopy(self.runs)
        del runs["rows"][-1]
        with self.assertRaises(es.ValidationError):
            self.validated(runs=runs)
        runs = copy.deepcopy(self.runs)
        runs["rows"].append(copy.deepcopy(runs["rows"][0]))
        with self.assertRaises(es.ValidationError):
            self.validated(runs=runs)
        runs = copy.deepcopy(self.runs)
        runs["rows"][2]["arms"][ck.V1]["counters"]["jvp_calls"] = 1.0
        with self.assertRaises(es.ValidationError):
            self.validated(runs=runs)
        runs = copy.deepcopy(self.runs)
        runs["rows"][2]["arms"][ck.V1]["y_last"].pop()
        with self.assertRaises(es.ValidationError):
            self.validated(runs=runs)
        with self.assertRaises(es.ValidationError):
            self.validated(raw_runs=json.dumps(self.runs).replace('"uform_budget": 200', '"uform_budget": NaN'))
        profile = copy.deepcopy(self.profile)
        profile["entries"].pop()
        with self.assertRaises(es.ValidationError):
            self.validated(profile=profile)

    def test_alg04_runs_are_bound_to_their_digest(self):
        with self.assertRaises(es.ValidationError):
            ck.validate(self.write("runs.json", self.runs), self.write("profile.json", self.profile),
                        self.write("trial.json", self.trial), self.write("alg04.json", self.alg04),
                        "0" * 64)


if __name__ == "__main__":
    unittest.main()
