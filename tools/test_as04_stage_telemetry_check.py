#!/usr/bin/env python3
"""Synthetic and mutation tests of tools/as04_stage_telemetry_check.py (research node AS04).

A synthetic RUNS document (all registered solver cells and driver runs, with self-consistent records, S1 reports
taken from the committed NATIVE.json and a synthetic ALG06 anchor) must PASS; every mutation must give the expected
INVALID (ValidationError, exit 2) or FAIL of the expected item. Run: python3 tools/test_as04_stage_telemetry_check.py
"""

from __future__ import annotations

import copy
import json
import math
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import as04_stage_telemetry_check as chk  # noqa: E402
import evidence_schema_v2 as evidence  # noqa: E402

NATIVE_PATH = ROOT / "research" / "reaudit_accuracy_speed_20261010" / "NATIVE.json"
ALG06_PATH = ROOT / "research" / "alg06_guard_v2_20261010" / "RUNS.json"
NATIVE = json.loads(NATIVE_PATH.read_text())
hx, f = chk.hx, chk.f
NAN = math.nan
AUDIT_OK = {"added_test_files": ["A crates/x/tests/new.rs"], "modified_test_files": [], "changed_test_tails": [],
            "ok": True}
SUITES_OK = {"command": "cargo test", "exit_code": 0, "binaries": 2, "passed": 10, "failed": 0, "ok": True}


# ------------------------------------------------------------------------------------------- builder
def telemetry(reason: str, requested: float, raised: float, actual: float, *, tightened: float | None = None,
              floor: float = NAN, arnoldi: int = 2, residual: int = 1, evaluations: int = 0, copies: int = 0,
              shadow: int = 0, guard: str | None = None, budget: bool = False, nu_max: float = 1.0) -> dict:
    return {
        "reason": reason, "requested_target": hx(requested), "raised_target": hx(raised),
        "tightened_target": hx(raised if tightened is None else tightened), "attainable_floor": hx(floor),
        "actual_residual": hx(actual), "actual_residual_inf": hx(abs(actual)),
        "attainment": chk.classify(actual, requested),
        "heuristic_gain": {"label": "heuristic", "nu_max": hx(nu_max), "nu_evaluations": 0},
        "verified_bound": "Unavailable", "arnoldi_jvps": arnoldi, "residual_jvps": residual,
        "fallback_evaluations": evaluations, "acceptance_vector_copies": copies, "nu_flops": 0,
        "uncounted_shadow_matvecs": shadow, "columns": arnoldi, "matvecs": arnoldi + residual,
        "guard_abort": guard, "budget_exhausted": budget,
    }


def report_of(t: dict, failure: str | None = None) -> dict:
    return {
        "outcome": t["reason"], "failure": failure, "accepted": t["reason"] != "Failed",
        "threshold": t["raised_target"], "final_threshold": t["tightened_target"],
        "residual_norm": t["actual_residual"], "residual_is_finite": math.isfinite(f(t["actual_residual"])),
        "projected_residual": hx(NAN), "right_norm": hx(1.0), "columns": t["columns"], "cycles": 1,
        "true_residuals": t["residual_jvps"], "confirmations": 0, "failed_confirmations": 0,
        "budget_exhausted": t["budget_exhausted"], "guard_abort": t["guard_abort"], "matvecs": t["matvecs"],
        "nu_max": t["heuristic_gain"]["nu_max"], "nu_flops": 0, "nu_evaluations": 0, "shadow_converged": None,
        "exhaustion": False,
    }


def counters(**values) -> dict:
    out = {k: 0 for k in evidence.COUNTERS_STAGED}
    out.update(values)
    return out


def evaluation(branch: str, r_le_atol: bool | None, accepted: bool, exit_: str = "Compared") -> dict:
    e = {"branch": branch, "exit": exit_, "r_le_atol": r_le_atol, "vector_copies": 0, "accepted": accepted}
    e["vector_copies"] = chk.documented_copies(e)
    return e


def cell(cell_: str, id_: str, t: dict, failure: str | None = None, evaluations: list | None = None,
         classify: bool = False, independent: int | None = None, report: dict | None = None) -> dict:
    return {
        "cell": cell_, "id": id_, "dimension": 2,
        "config": {"restart": 2, "max_columns": 16, "rtol": hx(1e-30), "atol": hx(0.0), "stall_rule": False,
                   "floor_at_confirmations": False, "stagnation_guard": classify, "classify_guard_aborts": classify,
                   "classify_accepted_guard_aborts": classify},
        "report": report if report is not None else report_of(t, failure),
        "telemetry": t, "evaluations": evaluations or [], "output": [hx(1.0), hx(0.0)],
        "counters": counters(linear_matvecs=t["matvecs"]), "independent_shadow_matvecs": independent,
    }


def native_hex(v):
    if v == "NaN":
        return hx(NAN)
    return hx(float(v))


def s1_cells() -> list:
    out = []
    for s in NATIVE["implementation_adversaries"]["stage_acceptance"]:
        r = s["observed"]["report"]
        reason = r["outcome"]
        threshold, actual = float(r["threshold"]), float(r["residual_norm"])
        floor = 1.0e-12 if reason in ("StallAccepted", "FloorAccepted") else NAN
        t = telemetry(reason, threshold, threshold, actual, tightened=float(r["final_threshold"]), floor=floor,
                      arnoldi=r["columns"], residual=r["true_residuals"])
        rep = report_of(t)
        rep.update({k: (native_hex(r[k]) if k in chk.REPORT_HEX else r[k]) for k in chk.NATIVE_REPORT_KEYS})
        out.append(cell("S1", s["id"], t, report=rep))
    return out


def solver_cells() -> list:
    out = s1_cells()
    above = 1.8371173070873834
    for id_, accepted, branch in (("overflowing_norms", False, "ScaledHomogeneous"),
                                  ("finite_norm_control", False, "ScaledLiteral"),
                                  ("loose_finite", True, "ScaledLiteral"),
                                  ("loose_homogeneous", True, "ScaledHomogeneous")):
        ev = evaluation(branch, False, accepted)
        t = telemetry("FallbackAccepted" if accepted else "Failed", 2.1e-14, 2.1e-14, above, arnoldi=1,
                      evaluations=1, copies=ev["vector_copies"], budget=True)
        out.append(cell("S2", id_, t, None if accepted else "BudgetExhausted", [ev]))
    for id_, classify, shadow in (("contraction_unclassified", False, 0), ("contraction_classified", True, 90)):
        ev = evaluation("UnscaledLiteral", False, True)
        t = telemetry("FallbackAccepted", 1e-10, 1e-10, 1.0, arnoldi=10, evaluations=1, shadow=shadow,
                      guard="Contraction")
        out.append(cell("S3", id_, t, None, [ev], classify=classify, independent=shadow))
    out.append(cell("S4", "raised_target", telemetry("Converged", 1e-30, 0.7, 0.45)))
    out.append(cell("S5", "nonfinite", telemetry("Failed", 1e-14, 1e-14, math.inf), "NonFinite"))
    out.append(cell("S5", "breakdown", telemetry("Failed", 1e-10, 1e-10, 3.7, arnoldi=1, residual=0),
                    "Breakdown"))
    out.append(cell("S5", "budget_exhausted", telemetry("Failed", 2.1e-14, 2.1e-14, above, arnoldi=1,
                                                        budget=True), "BudgetExhausted"))
    return out


def stage(attempt: int, index: int, t: dict, coupled: bool, n: int, epoch: dict, fallback: dict | None = None):
    tau = 1.5
    debit = tau * (f(t["actual_residual"]) / math.sqrt(n)) if coupled else NAN
    metric = ({"kind": "ScaledWrms", "atol": hx(1e-6), "rtol": hx(1e-6), "state_digest": "0123456789abcdef"}
              if coupled else {"kind": "UnscaledL2"})
    return {"attempt": attempt, "stage": index, "solver": t, "metric": metric, "operator_epoch": dict(epoch),
            "h": hx(0.1), "gamma": hx(0.2), "tau_e": hx(tau if coupled else NAN), "debit": hx(debit),
            "fallback": fallback, "jvp_calls": t["matvecs"]}


def driver_run(key: tuple, plan: list, coupled: bool, n: int, d2: bool = False) -> tuple[dict, dict]:
    """plan: per attempt (outcome, [stage reasons], epoch, relinearized)."""
    options, budget, rtol, dimension = chk.DRIVER_RUNS[key]
    stages, attempts, charges = [], [], []
    committed = 0.0
    total = {k: 0 for k in chk.WORK_KEYS}
    stats = {k: 0 for k in evidence.STAGE_STATISTICS_INT}
    jvp = 0
    for a, (outcome, reasons, epoch, relin) in enumerate(plan):
        mine = []
        for i, reason in enumerate(reasons):
            if reason == "FallbackAccepted":
                ev = evaluation("ScaledLiteral" if coupled else "UnscaledLiteral", False, True)
                t = telemetry(reason, 1e-8, 1e-8, 3e-8, evaluations=1, copies=ev["vector_copies"], budget=True)
            elif reason == "Failed":
                ev = evaluation("ScaledHomogeneous", False, False)
                t = telemetry(reason, 1e-8, 1e-8, 5.0, evaluations=1, copies=ev["vector_copies"], budget=True)
            elif reason == "FloorAccepted":
                ev = None
                t = telemetry(reason, 1e-8, 1e-8, 2e-8, floor=1.0)
            else:
                ev = None
                t = telemetry(reason, 1e-8, 1e-8, 5e-9)
            mine.append(stage(a, i, t, coupled, n, epoch, ev))
        debit_all = sum_in_order(f(s["debit"]) for s in mine
                                 if coupled and s["solver"]["reason"] != "Failed"
                                 and s["solver"]["attainment"] != "Met")
        debit_fb = sum_in_order(f(s["debit"]) for s in mine if coupled and s["solver"]["reason"] == "FallbackAccepted")
        work = {"jvp_calls": sum(s["solver"]["matvecs"] for s in mine),
                "linear_matvecs": sum(s["solver"]["matvecs"] for s in mine),
                "rhs_calls": len(mine) - 1 + (1 if relin else 0) if mine else 1,
                "krylov_columns": sum(s["solver"]["columns"] for s in mine),
                "arnoldi_jvps": sum(s["solver"]["arnoldi_jvps"] for s in mine),
                "residual_jvps": sum(s["solver"]["residual_jvps"] for s in mine),
                "shadow_matvecs": 0, "vector_copies": sum(s["solver"]["acceptance_vector_copies"] for s in mine)}
        jvp += work["jvp_calls"]
        error = 0.5 if outcome == "Committed" else (2.0 if outcome == "RejectedError" else NAN)
        counts = {chk.REASON_ATTEMPT[r]: sum(1 for s in mine if s["solver"]["reason"] == r) for r in chk.REASONS}
        ratio = NAN
        for s in mine:
            if s["solver"]["reason"] != "Failed":
                r = f(s["solver"]["actual_residual"]) / f(s["solver"]["requested_target"])
                ratio = r if math.isnan(ratio) or r > ratio else ratio
        rec = {"attempt": a, "t": hx(0.1 * a), "h": hx(0.1), "outcome": outcome, "operator_epoch": dict(epoch),
               "relinearized": relin, "stages": len(mine), **counts,
               "budget_exceeded": sum(1 for s in mine if s["solver"]["reason"] != "Failed"
                                      and s["solver"]["attainment"] == "BudgetExceeded"),
               "max_actual_over_requested": hx(ratio), "debit_all": hx(debit_all), "debit_fallback_only": hx(debit_fb),
               "error_norm": hx(error), "work": work}
        if outcome == "Committed":
            committed += debit_all
        if counts["fallback_accepted"] > 0 and not math.isnan(error):
            charges.append([rec["t"], rec["h"], rec["error_norm"], rec["debit_fallback_only"],
                            counts["fallback_accepted"], options["charge_fallback_residual"]])
        for k in chk.WORK_KEYS:
            total[k] += work[k]
        for r in chk.REASONS:
            stats[chk.REASON_STAT[r]] += counts[chk.REASON_STAT[r]]
        attempts.append(rec)
        stages.extend(mine)
    stats["solves"] = len(stages)
    stats["charged_attempts"] = len(charges)
    stat_all = {**stats, "charge_max": hx(0.0), "charge_sum": hx(0.0), "nu_max": hx(1.0)}
    accepted = sum(1 for p in plan if p[0] == "Committed")
    result = {"ok": True, "success": accepted > 0, "message": "success", "t": [hx(0.0), hx(1.0)],
              "y": [[hx(1.0)] * dimension, [hx(0.5)] * dimension], "attempts": len(plan), "accepted": accepted,
              "rejected": len(plan) - accepted, "state_reuses": 0, "internal_steps": accepted,
              "output_clipped_steps": 0, "counters": counters(jvp_calls=jvp, linear_matvecs=jvp),
              "stage_statistics": stat_all, "charges": charges}
    header = {"cell": key[0], "id": key[1], "arm": key[2], "dimension": dimension, "budget": budget,
              "options": copy.deepcopy(options), "rtol": rtol}
    records = {"stages": stages, "attempts": attempts, "committed_debit": hx(committed), "work_all_attempts": total,
               "jvp_outside_attempts": 0, "memory_bytes": 1000}
    on = {**header, "telemetry": True, "result": result, "records": records}
    off = {**header, "telemetry": False, "result": copy.deepcopy(result), "records": None}
    return on, off


def sum_in_order(values) -> float:
    s = 0.0
    for v in values:
        s += v
    return s


def build() -> tuple[dict, dict]:
    """A synthetic RUNS document and a matching ALG06 anchor document."""
    e1 = {"model_epoch": None, "generation": 1}
    runs = []
    for key in chk.DRIVER_RUNS:
        cell_, rid, arm = key
        coupled = chk.DRIVER_RUNS[key][0]["policy"] in chk.COUPLED_POLICIES
        n = chk.DRIVER_RUNS[key][3]
        if cell_ == "D1":
            plan = [("FailedLinear", ["Failed"], e1, True)]
        elif cell_ == "D2":
            plan = [("RejectedError", ["Converged"] * 8, {"model_epoch": 1, "generation": 1}, True),
                    ("Committed", ["Converged"] * 8, {"model_epoch": 1, "generation": 1}, False),
                    ("Committed", ["Converged"] * 8, {"model_epoch": 1, "generation": 2}, True),
                    ("Committed", ["Converged"] * 8, {"model_epoch": 2, "generation": 3}, True),
                    ("Committed", ["Converged"] * 8, {"model_epoch": 2, "generation": 4}, True)]
        else:
            plan = [("Committed", ["Converged"] * 6 + ["FallbackAccepted", "FloorAccepted"], e1, True),
                    ("RejectedError", ["Converged"] * 8, {"model_epoch": None, "generation": 2}, True)]
        if cell_ == "D2":
            on, off = driver_run(key, plan, coupled, n, d2=True)
        else:
            on, off = driver_run(key, plan, coupled, n)
        runs += [on, off]
    doc = {"schema": chk.RUNS_SCHEMA, "node": chk.NODE, "export": "export_runs",
           "provenance": {"source_commit": "0" * 40, "tree_status_before": "", "tree_status_after": "",
                          "rayon_num_threads": "1", "openblas_num_threads": "1"},
           "stages_per_attempt": 8, "solver_cells": solver_cells(), "driver_runs": runs}
    # ALG06 anchor rows from the telemetry-off results (ALG06 format: floats as JSON numbers).
    rows = []
    for (rid, arm), (case, rtol, alg_arm) in chk.ANCHORS.items():
        off = [r for r in runs if r["id"] == rid and r["arm"] == arm and not r["telemetry"]][0]["result"]
        rec = {k: off[k] for k in chk.ALG06_COMPARED}
        rec["y_last"] = off["y"][-1]
        rec["counters"] = {k: v for k, v in off["counters"].items() if k != "preconditioner_vectors"}
        rec["stage_statistics"] = {k: (f(v) if k in chk.STAT_FLOAT else v)
                                   for k, v in off["stage_statistics"].items()}
        rec["charges"] = [[f(c[0]), f(c[1]), f(c[2]), f(c[3]), c[4], c[5]] for c in off["charges"]]
        row = next((r for r in rows if r["case"] == case and r["rtol"] == rtol), None)
        if row is None:
            row = {"case": case, "rtol": rtol, "arms": {}}
            rows.append(row)
        row["arms"][alg_arm] = rec
    return doc, copy.deepcopy({"rows": rows})


def run_of(doc: dict, cell_: str, rid: str, arm: str, on: bool = True) -> dict:
    return [r for r in doc["driver_runs"] if (r["cell"], r["id"], r["arm"], r["telemetry"]) == (cell_, rid, arm, on)][0]


def cell_of(doc: dict, cell_: str, cid: str) -> dict:
    return [c for c in doc["solver_cells"] if (c["cell"], c["id"]) == (cell_, cid)][0]


ROB = ("R", "robertson-4e10/1e-5", "B3")


class SyntheticGate(unittest.TestCase):
    def evaluate(self, doc, alg06, audit=AUDIT_OK, suites=SUITES_OK):
        return chk.evaluate(doc, NATIVE, alg06, audit, suites)

    def failing(self, result) -> set:
        return {k for k, v in result["items"].items() if not v["pass"]}

    def assert_fails(self, mutate, items: set, alg06_mutate=None):
        doc, alg06 = build()
        mutate(doc)
        if alg06_mutate:
            alg06_mutate(alg06)
        result = self.evaluate(doc, alg06)
        self.assertEqual(result["verdict"], "FAIL")
        self.assertEqual(self.failing(result), items, json.dumps(result["items"], indent=1)[:3000])
        return result

    def assert_invalid(self, mutate):
        doc, alg06 = build()
        mutate(doc)
        with self.assertRaises(evidence.ValidationError):
            self.evaluate(doc, alg06)

    def test_synthetic_document_passes(self):
        doc, alg06 = build()
        result = self.evaluate(doc, alg06)
        self.assertEqual(result["verdict"], "PASS", json.dumps(result["items"], indent=1)[:4000])
        self.assertEqual(result["kill_rule"], "not triggered")

    # ---- item 1a / 1b
    def test_twin_difference_is_1a_and_kill_rule(self):
        def m(doc):
            run_of(doc, *ROB, on=False)["result"]["y"][-1][0] = hx(0.25)
        result = self.assert_fails(m, {"1a", "1b"})
        self.assertIn("not adopted", result["kill_rule"])

    def test_counter_difference_is_1a(self):
        def m(doc):
            run_of(doc, "D2", "as04-d2-linear-2", "CoupledGuarded2")["result"]["counters"]["rhs_calls"] += 1
        self.assert_fails(m, {"1a"})

    def test_audit_and_suites_gate_1a(self):
        doc, alg06 = build()
        for audit, suites in ((dict(AUDIT_OK, ok=False), SUITES_OK), (AUDIT_OK, None),
                              (AUDIT_OK, dict(SUITES_OK, ok=False)), (None, SUITES_OK)):
            result = self.evaluate(copy.deepcopy(doc), alg06, audit, suites)
            self.assertEqual(self.failing(result), {"1a"})

    def test_anchor_drift_is_1b_only(self):
        result = self.assert_fails(lambda doc: None, {"1b"},
                                   alg06_mutate=lambda a: a["rows"][0]["arms"]["b3"]["counters"].update(jvp_calls=7))
        self.assertIn("anchor drift", result["anchor"])
        self.assertEqual(result["kill_rule"], "not triggered")

    def test_anchor_charge_and_statistic_drift(self):
        def a(alg06):
            row = [r for r in alg06["rows"] if r["case"] == "robertson-4e10" and r["rtol"] == 1e-05][0]
            row["arms"]["b3"]["charges"][0][3] *= 1.0 + 2.0 ** -52
        self.assert_fails(lambda doc: None, {"1b"}, alg06_mutate=a)

        def b(alg06):
            alg06["rows"][0]["arms"]["b3"]["stage_statistics"]["nu_max"] = 1.5
        self.assert_fails(lambda doc: None, {"1b"}, alg06_mutate=b)

    # ---- item 2
    def test_hidden_application_is_2(self):
        def m(doc):
            s = run_of(doc, *ROB)["records"]["stages"][0]["solver"]
            s["matvecs"] += 1
        self.assert_fails(m, {"2"})

    def test_jvp_closure_is_2(self):
        def m(doc):
            run_of(doc, *ROB)["records"]["jvp_outside_attempts"] += 1
        self.assert_fails(m, {"2"})

    def test_copies_are_2(self):
        def m(doc):
            r = run_of(doc, *ROB)["records"]
            fb = [s for s in r["stages"] if s["fallback"]][0]
            fb["solver"]["acceptance_vector_copies"] += 1
            r["attempts"][0]["work"]["vector_copies"] += 1
            r["work_all_attempts"]["vector_copies"] += 1
        self.assert_fails(m, {"2"})

        def unrecorded(doc):
            r = run_of(doc, *ROB)["records"]
            fb = [s for s in r["stages"] if s["fallback"]][0]
            fb["fallback"] = None
            fb["solver"]["acceptance_vector_copies"] = 0
            r["attempts"][0]["work"]["vector_copies"] -= 1
            r["work_all_attempts"]["vector_copies"] -= 1
        self.assert_fails(unrecorded, {"2"})

    def test_shadow_count_is_2(self):
        def m(doc):
            cell_of(doc, "S3", "contraction_classified")["independent_shadow_matvecs"] += 1
        self.assert_fails(m, {"2"})

        def unclassified(doc):
            c = cell_of(doc, "S3", "contraction_unclassified")
            c["telemetry"]["uncounted_shadow_matvecs"] = 3
            c["independent_shadow_matvecs"] = 3
        self.assert_fails(unclassified, {"2"})

    def test_copies_table(self):
        e = lambda b, r, x="Compared": chk.documented_copies({"branch": b, "exit": x, "r_le_atol": r})  # noqa
        self.assertEqual(e("ScaledLiteral", False), 1)
        self.assertEqual(e("ScaledHomogeneous", False), 2)
        self.assertEqual(e("ScaledHomogeneous", True), 1)
        self.assertEqual(e("UnscaledLiteral", True), 0)
        self.assertEqual(e("UnscaledHomogeneous", False), 1)
        self.assertEqual(e("UnscaledHomogeneous", True), 1)  # literal registered table
        self.assertEqual(e("ScaledLiteral", None, "CandidateRejected"), 0)
        self.assertEqual(e("ScaledHomogeneous", None, "ResidualRejected"), 1)
        self.assertEqual(e("UnscaledLiteral", None, "ResidualRejected"), 0)
        self.assertEqual(e("ScaledUnrepresentable", False), 1)
        self.assertEqual(e("UnscaledUnrepresentable", False), 0)

    # ---- item 3
    def test_hidden_budget_excess_is_3(self):
        def m(doc):
            r = run_of(doc, *ROB)["records"]
            fb = [s for s in r["stages"] if s["solver"]["reason"] == "FallbackAccepted"][0]
            fb["solver"]["attainment"] = "Met"
        result = self.assert_fails(m, {"3", "4", "5"})
        self.assertIn("decision change", result["hold_rule"])

    def test_s1_and_s4_patterns_are_3(self):
        def m(doc):
            t = cell_of(doc, "S1", "triangular2_stall")["telemetry"]
            t["requested_target"] = hx(1.0)
            t["attainment"] = "Met"
        self.assert_fails(m, {"3", "4"})

        def s4(doc):
            t = cell_of(doc, "S4", "raised_target")["telemetry"]
            t["requested_target"] = hx(0.5)
            t["attainment"] = "Met"
        self.assert_fails(s4, {"3"})

    def test_attempt_budget_exceeded_count_is_3(self):
        def m(doc):
            run_of(doc, *ROB)["records"]["attempts"][0]["budget_exceeded"] += 1
        self.assert_fails(m, {"3"})

    # ---- item 4
    def test_statistics_mismatch_is_4(self):
        def m(doc):
            for on in (True, False):
                run_of(doc, *ROB, on=on)["result"]["stage_statistics"]["converged"] += 1
        self.assert_fails(m, {"1b", "4"})

    def test_missing_attempt_record_is_4(self):
        def m(doc):
            for on in (True, False):
                res = run_of(doc, "D2", "as04-d2-linear-2", "CoupledGuarded2", on=on)["result"]
                res["attempts"] += 1
        self.assert_fails(m, {"4"})

    def test_target_order_is_4(self):
        def m(doc):
            t = cell_of(doc, "S2", "loose_finite")["telemetry"]
            t["requested_target"] = hx(1.0)
            t["attainment"] = "BudgetExceeded"
        self.assert_fails(m, {"4"})

    def test_reason_implications_are_4(self):
        def m(doc):
            s = run_of(doc, *ROB)["records"]["stages"]
            [x for x in s if x["solver"]["reason"] == "FloorAccepted"][0]["solver"]["attainable_floor"] = hx(1e-12)
        self.assert_fails(m, {"4"})

    def test_native_report_is_4(self):
        def m(doc):
            cell_of(doc, "S1", "identity2_strict")["report"]["cycles"] += 1
        self.assert_fails(m, {"4"})

    def test_verified_bound_is_4(self):
        def m(doc):
            cell_of(doc, "S4", "raised_target")["telemetry"]["verified_bound"] = "Available"
        self.assert_fails(m, {"4"})

    def test_registered_cell_outcome_is_4(self):
        def m(doc):
            c = cell_of(doc, "S2", "overflowing_norms")
            c["report"]["failure"] = None
        self.assert_fails(m, {"4"})

    # ---- item 5
    def test_committed_debit_is_5(self):
        def m(doc):
            r = run_of(doc, *ROB)["records"]
            r["committed_debit"] = hx(f(r["committed_debit"]) * 2.0)
        self.assert_fails(m, {"5"})

    def test_charge_log_is_5(self):
        def m(doc):
            for on in (True, False):
                run_of(doc, *ROB, on=on)["result"]["charges"][0][3] = hx(1e-3)
        self.assert_fails(m, {"1b", "5"})

    def test_debit_order_is_5(self):
        def m(doc):
            r = run_of(doc, *ROB)["records"]
            a = r["attempts"][0]
            a["debit_fallback_only"] = hx(f(a["debit_all"]) * 4.0)
        self.assert_fails(m, {"5"})

    def test_work_ledger_is_5(self):
        def m(doc):
            run_of(doc, *ROB)["records"]["work_all_attempts"]["rhs_calls"] += 1
        self.assert_fails(m, {"5"})

    def test_d1_needs_failed_work_is_5(self):
        def m(doc):
            r = run_of(doc, "D1", "as01-cyclic-4-overflow", "Coupled+fallback")["records"]
            r["attempts"][0]["outcome"] = "FailedOther"
        self.assert_fails(m, {"5"})

    # ---- item 6
    def test_generation_is_6(self):
        def m(doc):
            r = run_of(doc, "D2", "as04-d2-linear-2", "CoupledGuarded2")["records"]
            r["attempts"][1]["relinearized"] = True
            r["attempts"][1]["work"]["rhs_calls"] += 1
            r["work_all_attempts"]["rhs_calls"] += 1
        self.assert_fails(m, {"6"})

    def test_epoch_change_is_6(self):
        def m(doc):
            r = run_of(doc, "D2", "as04-d2-linear-2", "CoupledGuarded2")["records"]
            r["attempts"][3]["operator_epoch"]["model_epoch"] = 1
            for s in r["stages"]:
                if s["attempt"] == 3:
                    s["operator_epoch"]["model_epoch"] = 1
        self.assert_fails(m, {"6"})

    def test_metric_is_6(self):
        def m(doc):
            r = run_of(doc, "R", "brusselator-1d-50/1e-6", "ProjL2+fallback")["records"]
            r["stages"][0]["metric"] = {"kind": "ScaledWrms", "atol": hx(1.0), "rtol": hx(1.0),
                                        "state_digest": "0" * 16}
        self.assert_fails(m, {"6"})

    # ---- INVALID
    def test_missing_or_duplicated_cases_are_invalid(self):
        self.assert_invalid(lambda doc: doc["solver_cells"].pop())
        self.assert_invalid(lambda doc: doc["solver_cells"].append(copy.deepcopy(doc["solver_cells"][0])))
        self.assert_invalid(lambda doc: doc["driver_runs"].remove(run_of(doc, *ROB, on=False)))
        self.assert_invalid(lambda doc: doc["driver_runs"].append(copy.deepcopy(doc["driver_runs"][0])))

    def test_malformed_records_are_invalid(self):
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["options"].update(charge_fallback_residual=False))
        self.assert_invalid(lambda doc: run_of(doc, *ROB).update(budget=200))
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["records"]["stages"][0].update(h="1.0"))
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["records"]["stages"][0].update(extra=1))
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["records"]["attempts"][0].update(outcome="Done"))
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["records"]["attempts"][0]["work"].update(jvp_calls=-1))
        self.assert_invalid(lambda doc: run_of(doc, *ROB, on=False).update(records={}))
        self.assert_invalid(lambda doc: run_of(doc, *ROB).update(records=None))
        self.assert_invalid(lambda doc: doc["provenance"].update(source_commit="abc"))
        self.assert_invalid(lambda doc: doc["provenance"].update(rayon_num_threads=None))
        self.assert_invalid(lambda doc: doc.update(schema="other"))
        self.assert_invalid(lambda doc: cell_of(doc, "S3", "contraction_classified").update(
            independent_shadow_matvecs=None))
        self.assert_invalid(lambda doc: cell_of(doc, "S1", "identity2_strict")["telemetry"].update(
            attainment="Maybe"))
        self.assert_invalid(lambda doc: run_of(doc, *ROB)["result"]["counters"].pop("jvp_calls"))

    def test_strict_json_rejects_nan(self):
        with self.assertRaises(evidence.ValidationError):
            evidence.loads_strict(b'{"a": NaN}')

    def test_main_is_invalid_on_unbound_evidence(self):
        doc, _ = build()
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            runs = tmp / "RUNS.json"
            runs.write_text(json.dumps(doc))
            # Fake source commit: provenance INVALID.
            out = tmp / "RESULTS.json"
            code = chk.main(["--runs", str(runs), "--native", str(NATIVE_PATH), "--alg06", str(ALG06_PATH),
                             "--output", str(out)], suites_runner=lambda: SUITES_OK)
            self.assertEqual(code, 2)
            self.assertEqual(json.loads(out.read_text())["verdict"], "INVALID")
            # The output is immutable.
            self.assertEqual(chk.main(["--runs", str(runs), "--native", str(NATIVE_PATH), "--alg06",
                                       str(ALG06_PATH), "--output", str(out)]), 2)
            # A NATIVE.json that is not the committed one.
            native = tmp / "NATIVE.json"
            native.write_bytes(NATIVE_PATH.read_bytes() + b" ")
            out2 = tmp / "RESULTS2.json"
            code = chk.main(["--runs", str(runs), "--native", str(native), "--alg06", str(ALG06_PATH),
                             "--output", str(out2)], suites_runner=lambda: SUITES_OK)
            self.assertEqual(code, 2)
            self.assertTrue(any("registered immutable input" in r
                                for r in json.loads(out2.read_text())["reasons"]))
            # Dirty recorded tree.
            doc["provenance"]["tree_status_after"] = " M crates/x.rs\n"
            runs.write_text(json.dumps(doc))
            out3 = tmp / "RESULTS3.json"
            self.assertEqual(chk.main(["--runs", str(runs), "--native", str(NATIVE_PATH), "--alg06",
                                       str(ALG06_PATH), "--output", str(out3)], suites_runner=lambda: SUITES_OK), 2)
            self.assertTrue(any("dirty" in r for r in json.loads(out3.read_text())["reasons"]))

    def test_provenance_binds_the_checker_to_the_source_commit(self):
        head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True,
                              text=True).stdout.strip()
        doc, _ = build()
        doc["provenance"]["source_commit"] = head
        at_head = chk.git_blob(head, chk.CHECKER_PATH)
        running = chk.git("hash-object", str(TOOLS / "as04_stage_telemetry_check.py")).strip()
        if at_head == running:
            # The checker is committed unchanged at HEAD; the markers decide.
            try:
                chk.check_provenance(doc, TOOLS / "as04_stage_telemetry_check.py")
            except evidence.ValidationError as exc:
                self.assertTrue(all("lacks" in r or "missing" in r for r in exc.reasons), exc.reasons)
        else:
            with self.assertRaises(evidence.ValidationError):
                chk.check_provenance(doc, TOOLS / "as04_stage_telemetry_check.py")

    def test_registered_ids_are_the_exported_ids(self):
        source = (ROOT / "crates" / "rodas5p-integrators" / "tests" / "stage_residual_telemetry.rs").read_text()
        for (cell_, rid, arm) in chk.DRIVER_RUNS:
            self.assertIn(f'"{arm}"', source)
            if cell_ != "R":
                self.assertIn(f'"{rid}"', source)
        for cell_, cid in chk.SOLVER_CELLS:
            if cell_ != "S1":
                self.assertIn(f'"{cid}"', source)
        self.assertIn('"vigilode-as04-runs-v1"', source)


if __name__ == "__main__":
    unittest.main()
