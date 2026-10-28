#!/usr/bin/env python3
"""Synthetic mutation tests for tools/as04_stage_telemetry_check.py (node AS04).

Tests:
1. Valid baseline fixture passes (exit 0, verdict PASS).
2. Validity mutations yield INVALID (exit 2).
3. Gate item 1..6 individual mutations yield FAIL (exit 1).

Run:
    python3 -m unittest discover -s tools -p "test_as04_*.py"
"""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import as04_stage_telemetry_check as checker

CHECKER = TOOLS / "as04_stage_telemetry_check.py"
NATIVE = ROOT / "research" / "reaudit_accuracy_speed_20261010" / "NATIVE.json"
ALG06 = ROOT / "research" / "alg06_guard_v2_20261010" / "RUNS.json"


def hex_f64(v: float) -> str:
    return struct.pack(">d", v).hex()


def make_valid_fixture() -> dict:
    """Constructs a minimal valid RUNS fixture that satisfies all AS04 gate items."""
    alg06_data = json.loads(ALG06.read_text())
    native_data = json.loads(NATIVE.read_text())

    alg06_map = {}
    for r in alg06_data.get("rows", []):
        case = r.get("case")
        rtol = r.get("rtol")
        alg06_map[(case, rtol)] = r

    rows = []
    for cid in checker.REGISTERED_CELL_IDS:
        if cid in checker.ALG06_ANCHOR_CELLS:
            case, rtol, arm = checker.ALG06_ANCHOR_CELLS[cid]
            lookup_case = case
            alg_row = alg06_map[(lookup_case, rtol)]
            arm_data = alg_row["arms"][arm]

            obs = {
                "success": arm_data.get("success", True),
                "message": arm_data.get("message", "success"),
                "t": arm_data["t"],
                "y": [arm_data["y_last"]],
                "y_last": arm_data["y_last"],
                "attempts": arm_data["attempts"],
                "accepted": arm_data["accepted"],
                "rejected": arm_data["rejected"],
                "state_reuses": arm_data.get("state_reuses", 0),
                "internal_steps": arm_data.get("internal_steps", arm_data["accepted"]),
                "output_clipped_steps": arm_data.get("output_clipped_steps", 0),
                "counters": arm_data["counters"],
            }
            dim = len(arm_data["y_last"])
        elif cid == "brusselator-1d-50_1e-6_proj_l2":
            case = "brusselator-1d-50"
            rtol = 1e-6
            arm = "proj_l2"
            dim = 100
            obs = {
                "success": True,
                "message": "success",
                "t": [hex_f64(0.0), hex_f64(10.0)],
                "y": [[hex_f64(1.0)] * dim, [hex_f64(1.0)] * dim],
                "y_last": [hex_f64(1.0)] * dim,
                "attempts": 2,
                "accepted": 2,
                "rejected": 0,
                "state_reuses": 0,
                "internal_steps": 2,
                "output_clipped_steps": 0,
                "counters": {
                    "accepted_steps": 2,
                    "rejected_steps": 0,
                    "linear_matvecs": 16,
                    "jvp_calls": 16,
                },
            }
        elif cid == "D1_transactional_failure":
            case = "d1"
            rtol = 1e-6
            arm = "D1"
            dim = 4
            obs = {
                "success": False,
                "message": "linear failure",
                "t": [hex_f64(0.0)],
                "y": [[hex_f64(1.0)] * dim],
                "y_last": [hex_f64(1.0)] * dim,
                "attempts": 2,
                "accepted": 1,
                "rejected": 1,
                "state_reuses": 0,
                "internal_steps": 1,
                "output_clipped_steps": 0,
                "counters": {
                    "accepted_steps": 1,
                    "rejected_steps": 1,
                    "linear_matvecs": 12,
                    "jvp_calls": 12,
                    "linear_solve_failures": 1,
                },
            }
        elif cid == "D2_epoch":
            case = "d2"
            rtol = 1e-8
            arm = "D2"
            dim = 2
            obs = {
                "success": True,
                "message": "success",
                "t": [hex_f64(0.0), hex_f64(1.0)],
                "y": [[hex_f64(1.0)] * dim, [hex_f64(1.0)] * dim],
                "y_last": [hex_f64(1.0)] * dim,
                "attempts": 4,
                "accepted": 4,
                "rejected": 0,
                "state_reuses": 0,
                "internal_steps": 4,
                "output_clipped_steps": 0,
                "counters": {
                    "accepted_steps": 4,
                    "rejected_steps": 0,
                    "linear_matvecs": 32,
                    "jvp_calls": 32,
                },
            }

        attempts = obs["attempts"]
        total_jvps = obs["counters"]["jvp_calls"]
        jvp_per_attempt = total_jvps // attempts
        remainder_jvp = total_jvps % attempts

        stage_records = []
        attempt_records = []
        for ai in range(attempts):
            jvps_here = jvp_per_attempt + (remainder_jvp if ai == 0 else 0)

            # Outcomes for D1
            if cid == "D1_transactional_failure":
                outcome = "Committed" if ai == 0 else "FailedLinear"
            else:
                outcome = "Committed"

            # Operator epoch for D2
            if cid == "D2_epoch":
                gen = 1 if ai < 3 else 2
                m_ep = 100 if ai < 3 else 200
            else:
                gen = 1
                m_ep = None

            # Metric
            if cid == "brusselator-1d-50_1e-6_proj_l2":
                met = "UnscaledL2"
                tau_e = 0.0
                heur_debit = 0.0
            else:
                met = {"ScaledWrms": {"atol_bits": 4412443251819771523, "rtol_bits": 4472406533629990549, "state_digest": [0]*32}}
                tau_e = 1.0
                heur_debit = 0.0

            st_rec = {
                "attempt": ai,
                "stage": 0,
                "telemetry": {
                    "reason": "Converged",
                    "requested_target": 1e-6,
                    "raised_target": 1e-6,
                    "tightened_target": 1e-6,
                    "attainable_floor": None,
                    "actual_residual": 1e-7,
                    "actual_residual_inf": 1e-7,
                    "attainment": "Met",
                    "heuristic_gain": {"nu_max": 1.0, "nu_evaluations": 0},
                    "verified_bound": "Unavailable",
                    "arnoldi_jvps": jvps_here,
                    "residual_jvps": 0,
                    "fallback_evaluations": 0,
                    "acceptance_vector_copies": 0,
                    "nu_flops": 0,
                    "uncounted_shadow_matvecs": 0,
                },
                "metric": met,
                "operator_epoch": {"model_epoch": m_ep, "generation": gen},
                "h_bits": 4607182418800017408,
                "gamma_bits": 4596803875735556352,
                "tau_e": tau_e,
                "heuristic_debit": heur_debit,
            }
            stage_records.append(st_rec)

            att_records = {
                "t": 0.0,
                "h": 0.1,
                "outcome": outcome,
                "converged": 1,
                "stall_accepted": 0,
                "floor_accepted": 0,
                "fallback_accepted": 0,
                "failed": 0,
                "budget_exceeded_stages": 0,
                "max_actual_over_requested": 0.1,
                "debit_all": heur_debit,
                "debit_fallback_only": 0.0,
                "jvp_calls": jvps_here,
                "arnoldi_jvps": jvps_here,
                "residual_jvps": 0,
                "shadow_matvecs": 0,
                "vector_copies": 0,
            }
            attempt_records.append(att_records)

        stats = {
            "solves": len(stage_records),
            "converged": len(stage_records),
            "stall_accepted": 0,
            "floor_accepted": 0,
            "fallback_accepted": 0,
            "failed": 0,
            "budget_exhausted": 0,
            "columns": total_jvps,
            "cycles": len(stage_records),
            "true_residuals": len(stage_records),
            "confirmations": len(stage_records),
            "failed_confirmations": 0,
            "guard_contraction": 0,
            "guard_overrun": 0,
            "guard_true": 0,
            "guard_false": 0,
            "guard_accepted_true": 0,
            "guard_accepted_false": 0,
            "fallback_after_guard": 0,
            "failed_guard": 0,
            "failed_nonfinite": 0,
            "failed_breakdown": 0,
            "failed_budget": 0,
            "exhaustion_solves": 0,
            "roundoff_floor_binds": 0,
            "nu_tightened": 0,
            "nu_evaluations": 0,
            "nu_flops": 0,
            "nu_max": 1.0,
            "max_columns": 10,
            "charged_attempts": 0,
            "charge_sum": 0.0,
            "charge_max": 0.0,
            "charge_crosses_one": 0,
        }

        tel = {
            "stage_records": stage_records,
            "attempt_records": attempt_records,
            "fallback_evaluations": [],
            "committed_debit": 0.0,
            "work_all_attempts": {
                "jvp_calls": total_jvps,
                "arnoldi_jvps": total_jvps,
                "residual_jvps": 0,
                "shadow_matvecs": 0,
                "vector_copies": 0,
            },
            "jvp_outside_attempts": 0,
        }

        row = {
            "id": cid,
            "case": case,
            "arm": arm,
            "rtol": rtol,
            "dimension": dim,
            "telemetry_on": {
                "observed": obs,
                "stage_statistics": stats,
                "charges": [],
                "telemetry": tel,
            },
            "telemetry_off": {
                "observed": copy.deepcopy(obs),
                "stage_statistics": copy.deepcopy(stats),
                "charges": [],
            },
        }
        rows.append(row)

    # Build solver cases
    native_s1 = native_data["implementation_adversaries"]["stage_acceptance"]
    s1_cases = []
    for item in native_s1:
        sid = item["id"]
        nat_rep = item["observed"]["report"]
        is_exceeded = sid in ("triangular2_stall", "triangular2_floor", "triangular4_floor")

        rep = {
            "outcome": nat_rep["outcome"],
            "failure": nat_rep["failure"],
            "guard_abort": nat_rep["guard_abort"],
            "shadow_converged": None,
            "exhaustion": False,
            "budget_exhausted": nat_rep["budget_exhausted"],
            "right_norm": nat_rep["right_norm"],
            "threshold": nat_rep["threshold"],
            "final_threshold": nat_rep["final_threshold"],
            "residual_norm": nat_rep["residual_norm"],
            "residual_norm_bits": hex_f64(nat_rep["residual_norm"]),
            "threshold_bits": hex_f64(nat_rep["threshold"]),
            "columns": nat_rep["columns"],
            "matvecs": nat_rep["matvecs"],
            "cycles": nat_rep["cycles"],
            "true_residuals": nat_rep["true_residuals"],
            "confirmations": nat_rep["confirmations"],
            "failed_confirmations": nat_rep["failed_confirmations"],
            "nu_evaluations": nat_rep.get("nu_evaluations", 0),
            "nu_max": nat_rep["nu_max"],
            "nu_flops": nat_rep["nu_flops"],
        }
        actual = nat_rep["residual_norm"]
        req = nat_rep["threshold"]
        tel = {
            "reason": nat_rep["outcome"],
            "requested_target": req,
            "raised_target": req,
            "tightened_target": nat_rep["final_threshold"],
            "attainable_floor": 1e-12 if is_exceeded else None,
            "actual_residual": actual,
            "actual_residual_inf": actual,
            "attainment": "BudgetExceeded" if is_exceeded else "Met",
            "heuristic_gain": {"nu_max": 1.0, "nu_evaluations": 0},
            "verified_bound": "Unavailable",
            "arnoldi_jvps": nat_rep["columns"],
            "residual_jvps": nat_rep["true_residuals"],
            "fallback_evaluations": 0,
            "acceptance_vector_copies": 0,
            "nu_flops": 0,
            "uncounted_shadow_matvecs": 0,
        }
        s1_cases.append({
            "id": sid,
            "case": sid.split("_")[0],
            "mode": sid.split("_")[1],
            "report": rep,
            "telemetry": tel,
            "output": [hex_f64(0.0)],
        })

    solver_cases = {
        "s1": s1_cases,
        "s2": {
            "seam_overflow": {"failed": True, "report": {"outcome": "Failed"}, "telemetry": {}},
            "seam_finite": {"failed": True, "report": {"outcome": "Failed"}, "telemetry": {}},
            "fallback_accepted_finite": {"report": {"outcome": "FallbackAccepted"}, "telemetry": {"attainment": "BudgetExceeded"}},
            "fallback_accepted_homogeneous": {"report": {"outcome": "FallbackAccepted"}, "telemetry": {"attainment": "BudgetExceeded"}},
        },
        "s3": {
            "classify_on": {
                "report": {"outcome": "FallbackAccepted", "matvecs": 10},
                "telemetry": {"uncounted_shadow_matvecs": 25},
                "wrapper_shadow_matvecs": 25,
            },
            "classify_off": {
                "report": {"outcome": "FallbackAccepted", "matvecs": 10},
                "telemetry": {"uncounted_shadow_matvecs": 0},
                "wrapper_shadow_matvecs": 0,
            },
        },
        "s4": {
            "report": {"outcome": "Converged"},
            "telemetry": {
                "reason": "Converged",
                "attainment": "BudgetExceeded",
                "requested_target": 1e-15,
                "raised_target": 1e-2,
                "tightened_target": 1e-2,
                "actual_residual": 1e-3,
                "actual_residual_inf": 1e-3,
                "attainable_floor": None,
                "heuristic_gain": {"nu_max": 1.0, "nu_evaluations": 0},
                "verified_bound": "Unavailable",
                "arnoldi_jvps": 2,
                "residual_jvps": 1,
                "fallback_evaluations": 0,
                "acceptance_vector_copies": 0,
                "nu_flops": 0,
                "uncounted_shadow_matvecs": 0,
            },
        },
        "s5": {
            "non_finite": {"report": {"outcome": "Failed", "failure": "NonFinite"}, "telemetry": {"reason": "Failed", "attainment": "Unverified"}},
            "breakdown": {"report": {"outcome": "Failed", "failure": "Breakdown"}, "telemetry": {"reason": "Failed", "attainment": "Unverified"}},
            "budget": {"report": {"outcome": "Failed", "failure": "BudgetExhausted"}, "telemetry": {"reason": "Failed", "attainment": "BudgetExceeded"}},
        },
    }

    return {
        "schema": checker.RUNS_SCHEMA,
        "node": checker.NODE,
        "export": "export_runs",
        "commit": "0000000000000000000000000000000000000000",
        "git_status": "clean",
        "rows": rows,
        "solver_cases": solver_cases,
    }


class TestAs04Checker(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.tmp = Path(self.tmpdir.name)
        self.runs_path = self.tmp / "RUNS.json"
        self.output_path = self.tmp / "RESULTS.json"

        self.valid_fixture = make_valid_fixture()
        self.runs_path.write_text(json.dumps(self.valid_fixture, indent=2))

    def tearDown(self):
        self.tmpdir.cleanup()

    def run_checker(self, runs_path=None, native_path=None, alg06_path=None):
        r_path = runs_path or self.runs_path
        n_path = native_path or NATIVE
        a_path = alg06_path or ALG06
        cmd = [
            sys.executable,
            str(CHECKER),
            "--runs",
            str(r_path),
            "--native",
            str(n_path),
            "--alg06",
            str(a_path),
            "--output",
            str(self.output_path),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        results = {}
        if self.output_path.exists():
            results = json.loads(self.output_path.read_text())
        return proc.returncode, results, proc.stdout, proc.stderr

    def test_baseline_passes(self):
        code, res, out, err = self.run_checker()
        self.assertEqual(code, 0, f"Baseline should PASS with code 0: {out} {err}")
        self.assertEqual(res.get("verdict"), "PASS")
        self.assertEqual(res.get("failures"), [])

    # Validity tests (exit 2, INVALID)
    def test_validity_dirty_tree(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["git_status"] = "dirty"
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 2)
        self.assertEqual(res.get("verdict"), "INVALID")

    def test_validity_missing_row(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"].pop()
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 2)
        self.assertEqual(res.get("verdict"), "INVALID")

    def test_validity_bad_native_hash(self):
        bad_native = self.tmp / "BAD_NATIVE.json"
        bad_native.write_text("{}")
        code, res, _, _ = self.run_checker(native_path=bad_native)
        self.assertEqual(code, 2)
        self.assertEqual(res.get("verdict"), "INVALID")

    # Gate item tests (exit 1, FAIL)
    def test_gate_1a_telemetry_on_off_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"][0]["telemetry_on"]["observed"]["attempts"] += 1
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_1a_no_behaviour_change_telemetry_on_off"]["pass"])

    def test_gate_1b_alg06_anchor_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        # Mutate both on and off attempts so 1a passes, but 1b fails against ALG06
        mut["rows"][0]["telemetry_on"]["observed"]["attempts"] += 10
        mut["rows"][0]["telemetry_off"]["observed"]["attempts"] += 10
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_1b_no_behaviour_change_alg06_anchor"]["pass"])

    def test_gate_2_jvp_closure_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"][0]["telemetry_on"]["telemetry"]["attempt_records"][0]["jvp_calls"] += 1
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_2_no_hidden_jvp_or_copy"]["pass"])

    def test_gate_2_shadow_matvecs_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["solver_cases"]["s3"]["classify_on"]["telemetry"]["uncounted_shadow_matvecs"] += 5
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_2_no_hidden_jvp_or_copy"]["pass"])

    def test_gate_3_attainment_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        # Change S1 triangular2_stall attainment from BudgetExceeded to Met
        for s in mut["solver_cases"]["s1"]:
            if s["id"] == "triangular2_stall":
                s["telemetry"]["attainment"] = "Met"
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_3_accepted_residual_above_target_visible"]["pass"])

    def test_gate_4_record_count_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"][0]["telemetry_on"]["stage_statistics"]["solves"] += 5
        mut["rows"][0]["telemetry_off"]["stage_statistics"]["solves"] += 5
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_4_complete_and_consistent_records"]["pass"])

    def test_gate_5_committed_debit_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"][0]["telemetry_on"]["telemetry"]["committed_debit"] += 1.0
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_5_ledgers"]["pass"])

    def test_gate_5_debit_all_less_than_fallback(self):
        mut = copy.deepcopy(self.valid_fixture)
        mut["rows"][0]["telemetry_on"]["telemetry"]["attempt_records"][0]["debit_fallback_only"] = 1.0
        mut["rows"][0]["telemetry_on"]["telemetry"]["attempt_records"][0]["debit_all"] = 0.5
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_5_ledgers"]["pass"])

    def test_gate_6_d2_generation_mismatch(self):
        mut = copy.deepcopy(self.valid_fixture)
        for r in mut["rows"]:
            if r["id"] == "D2_epoch":
                r["telemetry_on"]["telemetry"]["stage_records"][3]["operator_epoch"]["generation"] = 1
        self.runs_path.write_text(json.dumps(mut, indent=2))
        code, res, _, _ = self.run_checker()
        self.assertEqual(code, 1)
        self.assertEqual(res.get("verdict"), "FAIL")
        self.assertFalse(res["details"]["gate_6_epoch_and_metric"]["pass"])


if __name__ == "__main__":
    unittest.main()
