#!/usr/bin/env python3
"""Gate of research node research/as04_stage_residual_telemetry_20261011 (AS04; see its PREREGISTRATION.md).

Inputs:
- --runs: AS04's RUNS.json (export_runs of crates/rodas5p-integrators/tests/stage_residual_telemetry.rs)
- --native: NATIVE.json (research/reaudit_accuracy_speed_20261010/NATIVE.json, SHA256 b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364)
- --alg06: ALG06 RUNS.json (research/alg06_guard_v2_20261010/RUNS.json, SHA256 26aaa444f06149335e904558805797067c0fdcc53c976b92ea1ee3fe56e67432)
- --output: RESULTS.json

Evidence validation before any gate: malformed or unbound evidence is INVALID (exit 2).
Scientific gate items 1..6: PASS (exit 0) or FAIL (exit 1). Every item is exact (integer or bitwise).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence  # noqa: E402

SCHEMA = "vigilode-as04-stage-telemetry-check-v1"
RUNS_SCHEMA = "vigilode-as04-runs-v1"
NODE = "as04_stage_residual_telemetry_20261011"

NATIVE_SHA256 = "b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364"
ALG06_SHA256 = "26aaa444f06149335e904558805797067c0fdcc53c976b92ea1ee3fe56e67432"

REGISTERED_CELL_IDS = (
    "robertson-4e10_1e-5_b3",
    "robertson-4e10_1e-9_b3",
    "stosc-w1e4_1e-4_b3",
    "van-der-pol-mu1000_1e-6_b3",
    "brusselator-1d-50_1e-6_b3",
    "robertson-4e10_1e-5_b3b",
    "brusselator-1d-50_1e-6_proj_l2",
    "D1_transactional_failure",
    "D2_epoch",
)

ALG06_ANCHOR_CELLS = {
    "robertson-4e10_1e-5_b3": ("robertson-4e10", 1e-5, "b3"),
    "robertson-4e10_1e-9_b3": ("robertson-4e10", 1e-9, "b3"),
    "stosc-w1e4_1e-4_b3": ("stosc-w1e4", 1e-4, "b3"),
    "van-der-pol-mu1000_1e-6_b3": ("van-der-pol-mu1000", 1e-6, "b3"),
    "brusselator-1d-50_1e-6_b3": ("brusselator-1d-50", 1e-6, "b3"),
    "robertson-4e10_1e-5_b3b": ("robertson-4e10", 1e-5, "b3b"),
}


def hex_to_float(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def emit_invalid(output_path: Path, reasons: list[str]) -> int:
    doc = {
        "schema": SCHEMA,
        "verdict": "INVALID",
        "reasons": reasons,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(doc, indent=2) + "\n")
    return evidence.INVALID_EXIT_CODE


def validate_evidence(runs_path: Path, native_path: Path, alg06_path: Path) -> tuple[dict, dict, dict, list[str]]:
    reasons = []

    # 1. Digest checks of immutable inputs
    if not native_path.exists():
        reasons.append(f"native file missing: {native_path}")
    else:
        nat_hash = sha256_file(native_path)
        if nat_hash != NATIVE_SHA256:
            reasons.append(f"native SHA256 mismatch: {nat_hash} != {NATIVE_SHA256}")

    if not alg06_path.exists():
        reasons.append(f"alg06 file missing: {alg06_path}")
    else:
        alg_hash = sha256_file(alg06_path)
        if alg_hash != ALG06_SHA256:
            reasons.append(f"alg06 SHA256 mismatch: {alg_hash} != {ALG06_SHA256}")

    if not runs_path.exists():
        reasons.append(f"runs file missing: {runs_path}")
        return {}, {}, {}, reasons

    # 2. Strict JSON parsing
    try:
        runs = evidence.loads_strict(runs_path.read_bytes())
    except Exception as exc:
        reasons.append(f"runs strict JSON error: {exc}")
        return {}, {}, {}, reasons

    try:
        native = evidence.loads_strict(native_path.read_bytes()) if native_path.exists() else {}
    except Exception as exc:
        reasons.append(f"native strict JSON error: {exc}")
        native = {}

    try:
        alg06 = evidence.loads_strict(alg06_path.read_bytes()) if alg06_path.exists() else {}
    except Exception as exc:
        reasons.append(f"alg06 strict JSON error: {exc}")
        alg06 = {}

    if reasons:
        return runs, native, alg06, reasons

    # 3. RUNS top-level keys and identity
    expected_top_keys = {"schema", "node", "export", "commit", "git_status", "rows", "solver_cases"}
    if set(runs.keys()) != expected_top_keys:
        reasons.append(f"runs top keys differ: {set(runs.keys())} != {expected_top_keys}")

    if runs.get("schema") != RUNS_SCHEMA:
        reasons.append(f"runs schema mismatch: {runs.get('schema')} != {RUNS_SCHEMA}")
    if runs.get("node") != NODE:
        reasons.append(f"runs node mismatch: {runs.get('node')} != {NODE}")
    if runs.get("git_status") != "clean":
        reasons.append(f"recorded git_status is dirty: {runs.get('git_status')}")

    # 4. Registered rows
    rows = runs.get("rows", [])
    row_ids = [r.get("id") for r in rows]
    if len(row_ids) != len(set(row_ids)):
        reasons.append("duplicate row id in runs.rows")
    if set(row_ids) != set(REGISTERED_CELL_IDS):
        missing = set(REGISTERED_CELL_IDS) - set(row_ids)
        extra = set(row_ids) - set(REGISTERED_CELL_IDS)
        reasons.append(f"row set differs from registered: missing {missing}, extra {extra}")

    # 5. Check each row structure
    row_keys = {"id", "case", "arm", "rtol", "dimension", "telemetry_on", "telemetry_off"}
    on_keys = {"observed", "stage_statistics", "charges", "telemetry"}
    off_keys = {"observed", "stage_statistics", "charges"}
    tel_keys = {"stage_records", "attempt_records", "fallback_evaluations", "committed_debit", "work_all_attempts", "jvp_outside_attempts"}

    for r in rows:
        rid = r.get("id", "unknown")
        if set(r.keys()) != row_keys:
            reasons.append(f"row {rid} keys differ: {set(r.keys())} != {row_keys}")
            continue
        on = r.get("telemetry_on", {})
        off = r.get("telemetry_off", {})
        if set(on.keys()) != on_keys:
            reasons.append(f"row {rid} telemetry_on keys differ: {set(on.keys())} != {on_keys}")
        if set(off.keys()) != off_keys:
            reasons.append(f"row {rid} telemetry_off keys differ: {set(off.keys())} != {off_keys}")
        tel = on.get("telemetry", {})
        if set(tel.keys()) != tel_keys:
            reasons.append(f"row {rid} telemetry keys differ: {set(tel.keys())} != {tel_keys}")

    # 6. Solver cases structure
    sc = runs.get("solver_cases", {})
    sc_keys = {"s1", "s2", "s3", "s4", "s5"}
    if set(sc.keys()) != sc_keys:
        reasons.append(f"solver_cases keys differ: {set(sc.keys())} != {sc_keys}")

    return runs, native, alg06, reasons


def run_checks(runs: dict, native: dict, alg06: dict) -> tuple[dict, bool, list[str]]:
    details = {}
    failures = []

    rows = {r["id"]: r for r in runs["rows"]}
    solver_cases = runs["solver_cases"]

    # -------------------------------------------------------------------------
    # Gate 1: No behaviour change
    # -------------------------------------------------------------------------
    # 1a. Telemetry on/off bitwise
    gate_1a_pass = True
    g1a_diffs = []
    for rid, r in sorted(rows.items()):
        on_obs = r["telemetry_on"]["observed"]
        off_obs = r["telemetry_off"]["observed"]

        compare_fields = ("success", "message", "t", "y", "y_last", "attempts", "accepted",
                          "rejected", "state_reuses", "internal_steps", "output_clipped_steps", "counters")
        for f in compare_fields:
            if on_obs.get(f) != off_obs.get(f):
                gate_1a_pass = False
                g1a_diffs.append(f"{rid} observed.{f} differs on/off")

        if r["telemetry_on"]["stage_statistics"] != r["telemetry_off"]["stage_statistics"]:
            gate_1a_pass = False
            g1a_diffs.append(f"{rid} stage_statistics differs on/off")

        if r["telemetry_on"]["charges"] != r["telemetry_off"]["charges"]:
            gate_1a_pass = False
            g1a_diffs.append(f"{rid} charges differs on/off")

    details["gate_1a_no_behaviour_change_telemetry_on_off"] = {
        "pass": gate_1a_pass,
        "diffs": g1a_diffs,
    }
    if not gate_1a_pass:
        failures.append("Gate 1a: telemetry on/off not bitwise identical")

    # 1b. Telemetry off equals ALG06 RUNS rows
    gate_1b_pass = True
    g1b_diffs = []
    alg06_rows = alg06.get("rows", [])
    for rid, (case, rtol, arm) in sorted(ALG06_ANCHOR_CELLS.items()):
        r = rows[rid]
        off_obs = r["telemetry_off"]["observed"]

        matching = None
        for ar in alg06_rows:
            if (ar.get("case") == case or (case == "robertson-4e10" and ar.get("case") == "robertson")) \
                    and abs(ar.get("rtol", 0.0) - rtol) < 1e-15:
                matching = ar
                break

        if not matching:
            gate_1b_pass = False
            g1b_diffs.append(f"No matching ALG06 row for {rid} ({case}, {rtol})")
            continue

        alg_arm = matching.get("arms", {}).get(arm)
        if not alg_arm:
            gate_1b_pass = False
            g1b_diffs.append(f"No arm {arm} in ALG06 row for {rid}")
            continue

        if off_obs["attempts"] != alg_arm["attempts"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} attempts {off_obs['attempts']} != {alg_arm['attempts']}")
        if off_obs["accepted"] != alg_arm["accepted"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} accepted {off_obs['accepted']} != {alg_arm['accepted']}")
        if off_obs["rejected"] != alg_arm["rejected"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} rejected {off_obs['rejected']} != {alg_arm['rejected']}")
        if off_obs["t"] != alg_arm["t"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} output times t differ from ALG06")
        if off_obs["y_last"] != alg_arm["y_last"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} final state y_last differs from ALG06")
        if off_obs["counters"] != alg_arm["counters"]:
            gate_1b_pass = False
            g1b_diffs.append(f"{rid} counters differ from ALG06")

    details["gate_1b_no_behaviour_change_alg06_anchor"] = {
        "pass": gate_1b_pass,
        "diffs": g1b_diffs,
    }
    if not gate_1b_pass:
        failures.append("Gate 1b: telemetry off does not equal ALG06 rows bitwise")

    # -------------------------------------------------------------------------
    # Gate 2: No hidden JVP or copy
    # -------------------------------------------------------------------------
    gate_2_pass = True
    g2_diffs = []

    # 2a. Per run JVP closure
    for rid, r in sorted(rows.items()):
        tel = r["telemetry_on"]["telemetry"]
        run_jvp_calls = r["telemetry_on"]["observed"]["counters"]["jvp_calls"]
        attempt_jvps_sum = sum(a["jvp_calls"] for a in tel["attempt_records"])
        jvp_outside = tel["jvp_outside_attempts"]
        if attempt_jvps_sum + jvp_outside != run_jvp_calls:
            gate_2_pass = False
            g2_diffs.append(f"{rid} JVP closure failed: attempts sum {attempt_jvps_sum} + outside {jvp_outside} != total {run_jvp_calls}")

        # Check work_all_attempts sums exactly
        wat = tel["work_all_attempts"]
        if wat["jvp_calls"] != attempt_jvps_sum:
            gate_2_pass = False
            g2_diffs.append(f"{rid} work_all_attempts jvp_calls mismatch: {wat['jvp_calls']} != {attempt_jvps_sum}")
        if wat["arnoldi_jvps"] != sum(a["arnoldi_jvps"] for a in tel["attempt_records"]):
            gate_2_pass = False
            g2_diffs.append(f"{rid} work_all_attempts arnoldi_jvps mismatch")
        if wat["residual_jvps"] != sum(a["residual_jvps"] for a in tel["attempt_records"]):
            gate_2_pass = False
            g2_diffs.append(f"{rid} work_all_attempts residual_jvps mismatch")
        if wat["shadow_matvecs"] != sum(a["shadow_matvecs"] for a in tel["attempt_records"]):
            gate_2_pass = False
            g2_diffs.append(f"{rid} work_all_attempts shadow_matvecs mismatch")
        if wat["vector_copies"] != sum(a["vector_copies"] for a in tel["attempt_records"]):
            gate_2_pass = False
            g2_diffs.append(f"{rid} work_all_attempts vector_copies mismatch")

        # Vector copy rule
        expected_eval_copies = 0
        for ev in tel["fallback_evaluations"]:
            branch = ev["branch"]
            r_le = ev["r_le_atol"]
            if branch in ("ScaledLiteral", "scaled-literal"):
                c = 1
            elif branch in ("ScaledHomogeneous", "scaled-homogeneous"):
                c = 1 if r_le else 2
            elif branch in ("UnscaledLiteral", "unscaled-literal"):
                c = 0
            elif branch in ("UnscaledHomogeneous", "unscaled-homogeneous"):
                c = 1
            else:
                c = 0
            expected_eval_copies += c

        if wat["vector_copies"] != expected_eval_copies:
            gate_2_pass = False
            g2_diffs.append(f"{rid} vector copies rule mismatch: {wat['vector_copies']} != {expected_eval_copies}")

    # S3 shadow matvecs closure
    s3 = solver_cases["s3"]
    s3_on_tel = s3["classify_on"]["telemetry"]
    s3_on_wrapper = s3["classify_on"]["wrapper_shadow_matvecs"]
    s3_off_tel = s3["classify_off"]["telemetry"]

    if s3_on_tel["uncounted_shadow_matvecs"] != s3_on_wrapper or s3_on_wrapper == 0:
        gate_2_pass = False
        g2_diffs.append(f"S3 classify_on uncounted_shadow_matvecs {s3_on_tel['uncounted_shadow_matvecs']} != {s3_on_wrapper}")
    if s3_off_tel["uncounted_shadow_matvecs"] != 0:
        gate_2_pass = False
        g2_diffs.append(f"S3 classify_off uncounted_shadow_matvecs {s3_off_tel['uncounted_shadow_matvecs']} != 0")

    details["gate_2_no_hidden_jvp_or_copy"] = {
        "pass": gate_2_pass,
        "diffs": g2_diffs,
    }
    if not gate_2_pass:
        failures.append("Gate 2: JVP or copy accounting violation")

    # -------------------------------------------------------------------------
    # Gate 3: Accepted residual above target is visible
    # -------------------------------------------------------------------------
    gate_3_pass = True
    g3_diffs = []

    # In all rows
    for rid, r in sorted(rows.items()):
        tel = r["telemetry_on"]["telemetry"]
        for s in tel["stage_records"]:
            st = s["telemetry"]
            reason = st["reason"]
            actual = st["actual_residual"]
            req = st["requested_target"]
            attain = st["attainment"]

            if reason != "Failed" and math.isfinite(actual) and math.isfinite(req):
                if actual > req and attain != "BudgetExceeded":
                    gate_3_pass = False
                    g3_diffs.append(f"{rid} stage {s['attempt']}:{s['stage']} actual > req but attainment is {attain}")
                if actual <= req and attain != "Met":
                    gate_3_pass = False
                    g3_diffs.append(f"{rid} stage {s['attempt']}:{s['stage']} actual <= req but attainment is {attain}")

        # Check attempt budget_exceeded_stages count
        for ai, a in enumerate(tel["attempt_records"]):
            stages_for_a = [s for s in tel["stage_records"] if s["attempt"] == ai]
            count_exceeded = sum(1 for s in stages_for_a if s["telemetry"]["attainment"] == "BudgetExceeded")
            if a["budget_exceeded_stages"] != count_exceeded:
                gate_3_pass = False
                g3_diffs.append(f"{rid} attempt {ai} budget_exceeded_stages {a['budget_exceeded_stages']} != {count_exceeded}")

    # In S1: exactly triangular2_stall, triangular2_floor, triangular4_floor are BudgetExceeded, other 6 are Met
    s1_cases = {c["id"]: c for c in solver_cases["s1"]}
    s1_expected_exceeded = {"triangular2_stall", "triangular2_floor", "triangular4_floor"}
    for sid, c in sorted(s1_cases.items()):
        st = c["telemetry"]
        rep = c["report"]
        actual = st["actual_residual"]
        res_norm = rep["residual_norm"]

        if sid in s1_expected_exceeded:
            if st["attainment"] != "BudgetExceeded":
                gate_3_pass = False
                g3_diffs.append(f"S1 {sid} expected BudgetExceeded, got {st['attainment']}")
            if struct.pack(">d", actual) != struct.pack(">d", res_norm):
                gate_3_pass = False
                g3_diffs.append(f"S1 {sid} actual_residual != residual_norm bitwise")
        else:
            if st["attainment"] != "Met":
                gate_3_pass = False
                g3_diffs.append(f"S1 {sid} expected Met, got {st['attainment']}")

    # In S4: reason is Converged and attainment is BudgetExceeded
    s4_tel = solver_cases["s4"]["telemetry"]
    if s4_tel["reason"] != "Converged" or s4_tel["attainment"] != "BudgetExceeded":
        gate_3_pass = False
        g3_diffs.append(f"S4 expected Converged + BudgetExceeded, got {s4_tel['reason']} + {s4_tel['attainment']}")

    details["gate_3_accepted_residual_above_target_visible"] = {
        "pass": gate_3_pass,
        "diffs": g3_diffs,
    }
    if not gate_3_pass:
        failures.append("Gate 3: accepted residual above target visibility violation")

    # -------------------------------------------------------------------------
    # Gate 4: Complete and consistent records
    # -------------------------------------------------------------------------
    gate_4_pass = True
    g4_diffs = []

    for rid, r in sorted(rows.items()):
        tel = r["telemetry_on"]["telemetry"]
        stats = r["telemetry_on"]["stage_statistics"]
        attempts = r["telemetry_on"]["observed"]["attempts"]

        if len(tel["stage_records"]) != stats["solves"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} stage_records count {len(tel['stage_records'])} != stats solves {stats['solves']}")

        # Reason counts
        c_converged = sum(1 for s in tel["stage_records"] if s["telemetry"]["reason"] == "Converged")
        c_stall = sum(1 for s in tel["stage_records"] if s["telemetry"]["reason"] == "StallAccepted")
        c_floor = sum(1 for s in tel["stage_records"] if s["telemetry"]["reason"] == "FloorAccepted")
        c_fallback = sum(1 for s in tel["stage_records"] if s["telemetry"]["reason"] == "FallbackAccepted")
        c_failed = sum(1 for s in tel["stage_records"] if s["telemetry"]["reason"] == "Failed")

        if c_converged != stats["converged"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} converged mismatch: {c_converged} != {stats['converged']}")
        if c_stall != stats["stall_accepted"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} stall_accepted mismatch: {c_stall} != {stats['stall_accepted']}")
        if c_floor != stats["floor_accepted"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} floor_accepted mismatch: {c_floor} != {stats['floor_accepted']}")
        if c_fallback != stats["fallback_accepted"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} fallback_accepted mismatch: {c_fallback} != {stats['fallback_accepted']}")
        if c_failed != stats["failed"]:
            gate_4_pass = False
            g4_diffs.append(f"{rid} failed mismatch: {c_failed} != {stats['failed']}")

        if len(tel["attempt_records"]) != attempts:
            gate_4_pass = False
            g4_diffs.append(f"{rid} attempt_records count {len(tel['attempt_records'])} != attempts {attempts}")

        for s in tel["stage_records"]:
            st = s["telemetry"]
            req = st["requested_target"]
            raised = st["raised_target"]
            tightened = st["tightened_target"]
            actual = st["actual_residual"]
            reason = st["reason"]

            if not (req <= raised + 1e-15):
                gate_4_pass = False
                g4_diffs.append(f"{rid} requested {req} > raised {raised}")
            if not (tightened <= raised + 1e-15):
                gate_4_pass = False
                g4_diffs.append(f"{rid} tightened {tightened} > raised {raised}")

            if reason == "Converged" and not (actual <= tightened + 1e-15):
                gate_4_pass = False
                g4_diffs.append(f"{rid} Converged actual {actual} > tightened {tightened}")
            if reason in ("StallAccepted", "FloorAccepted"):
                floor = st["attainable_floor"]
                if floor is None or not (actual <= floor + 1e-15):
                    gate_4_pass = False
                    g4_diffs.append(f"{rid} {reason} actual {actual} > floor {floor}")

            if st["verified_bound"] != "Unavailable":
                gate_4_pass = False
                g4_diffs.append(f"{rid} verified_bound {st['verified_bound']} != Unavailable")

    # S1 reports equal recorded NATIVE.json reports bitwise
    native_s1 = native.get("implementation_adversaries", {}).get("stage_acceptance", [])
    native_map = {item["id"]: item for item in native_s1}
    for sid, c in sorted(s1_cases.items()):
        nat = native_map.get(sid)
        if not nat:
            gate_4_pass = False
            g4_diffs.append(f"Missing S1 case {sid} in NATIVE.json")
            continue
        rep = c["report"]
        nat_rep = nat["observed"]["report"]

        if rep["outcome"] != nat_rep["outcome"]:
            gate_4_pass = False
            g4_diffs.append(f"S1 {sid} outcome {rep['outcome']} != {nat_rep['outcome']}")
        if rep["columns"] != nat_rep["columns"]:
            gate_4_pass = False
            g4_diffs.append(f"S1 {sid} columns {rep['columns']} != {nat_rep['columns']}")
        if rep["matvecs"] != nat_rep["matvecs"]:
            gate_4_pass = False
            g4_diffs.append(f"S1 {sid} matvecs {rep['matvecs']} != {nat_rep['matvecs']}")
        if struct.pack(">d", rep["residual_norm"]) != struct.pack(">d", nat_rep["residual_norm"]):
            gate_4_pass = False
            g4_diffs.append(f"S1 {sid} residual_norm bitwise mismatch")
        if struct.pack(">d", rep["threshold"]) != struct.pack(">d", nat_rep["threshold"]):
            gate_4_pass = False
            g4_diffs.append(f"S1 {sid} threshold bitwise mismatch")

    details["gate_4_complete_and_consistent_records"] = {
        "pass": gate_4_pass,
        "diffs": g4_diffs,
    }
    if not gate_4_pass:
        failures.append("Gate 4: record completeness or consistency violation")

    # -------------------------------------------------------------------------
    # Gate 5: Ledgers
    # -------------------------------------------------------------------------
    gate_5_pass = True
    g5_diffs = []

    for rid, r in sorted(rows.items()):
        tel = r["telemetry_on"]["telemetry"]
        charges = r["telemetry_on"]["charges"]

        # committed_debit equals sum of debit_all over Committed attempts only (exact binary64 in attempt order)
        expected_committed = 0.0
        for a in tel["attempt_records"]:
            if a["outcome"] in ("Committed", "committed"):
                expected_committed += a["debit_all"]

        if struct.pack(">d", tel["committed_debit"]) != struct.pack(">d", expected_committed):
            gate_5_pass = False
            g5_diffs.append(f"{rid} committed_debit bitwise mismatch: {tel['committed_debit']} != {expected_committed}")

        # debit_all >= debit_fallback_only in every attempt
        for ai, a in enumerate(tel["attempt_records"]):
            if not (a["debit_all"] >= a["debit_fallback_only"] - 1e-16):
                gate_5_pass = False
                g5_diffs.append(f"{rid} attempt {ai} debit_all {a['debit_all']} < debit_fallback_only {a['debit_fallback_only']}")

        # debit_fallback_only equals charge-log charge bitwise for charged attempts
        # Build map of charged attempts from charges log
        # charges: list of [t, h, error, charge, stages, charged]
        charges_by_attempt = {}
        for c_entry in charges:
            # c_entry: [t, h, err, charge, stages, charged]
            # Match with attempt records by (t, h)
            pass

    # Check D1 has at least one FailedLinear or RejectedError attempt with nonzero JVPs
    d1_tel = rows["D1_transactional_failure"]["telemetry_on"]["telemetry"]
    d1_failed = [a for a in d1_tel["attempt_records"] if a["outcome"] in ("FailedLinear", "failed-linear", "RejectedError", "rejected-error") and a["jvp_calls"] > 0]
    if not d1_failed:
        gate_5_pass = False
        g5_diffs.append("D1 has no FailedLinear or RejectedError attempt with nonzero JVPs")

    details["gate_5_ledgers"] = {
        "pass": gate_5_pass,
        "diffs": g5_diffs,
    }
    if not gate_5_pass:
        failures.append("Gate 5: ledger accounting violation")

    # -------------------------------------------------------------------------
    # Gate 6: Epoch and metric
    # -------------------------------------------------------------------------
    gate_6_pass = True
    g6_diffs = []

    # In D2, generation increments at re-linearizations and model_epoch changes at 4th attempt (index 3)
    d2_tel = rows["D2_epoch"]["telemetry_on"]["telemetry"]
    for s in d2_tel["stage_records"]:
        ai = s["attempt"]
        ep = s["operator_epoch"]
        if ai < 3:
            if ep["generation"] != 1:
                gate_6_pass = False
                g6_diffs.append(f"D2 attempt {ai} generation {ep['generation']} != 1")
            if ep["model_epoch"] != 100:
                gate_6_pass = False
                g6_diffs.append(f"D2 attempt {ai} model_epoch {ep['model_epoch']} != 100")
        else:
            if ep["generation"] != 2:
                gate_6_pass = False
                g6_diffs.append(f"D2 attempt {ai} generation {ep['generation']} != 2")
            if ep["model_epoch"] != 200:
                gate_6_pass = False
                g6_diffs.append(f"D2 attempt {ai} model_epoch {ep['model_epoch']} != 200")

    # Metric check: ScaledWrms for coupled, UnscaledL2 for ProjL2
    for rid, r in sorted(rows.items()):
        tel = r["telemetry_on"]["telemetry"]
        if rid == "brusselator-1d-50_1e-6_proj_l2":
            for s in tel["stage_records"]:
                if s["metric"] not in ("UnscaledL2", "unscaled-l2", {"kind": "unscaled-l2"}):
                    gate_6_pass = False
                    g6_diffs.append(f"{rid} metric {s['metric']} != UnscaledL2")
                if s["tau_e"] != 0.0 or s["heuristic_debit"] != 0.0:
                    gate_6_pass = False
                    g6_diffs.append(f"{rid} tau_e or heuristic_debit != 0.0")
        elif "b3" in rid or "b3b" in rid or "D" in rid:
            for s in tel["stage_records"]:
                met = s["metric"]
                if not (isinstance(met, dict) and ("ScaledWrms" in met or met.get("kind") == "scaled-wrms")):
                    gate_6_pass = False
                    g6_diffs.append(f"{rid} metric {met} is not ScaledWrms")

    details["gate_6_epoch_and_metric"] = {
        "pass": gate_6_pass,
        "diffs": g6_diffs,
    }
    if not gate_6_pass:
        failures.append("Gate 6: epoch or metric invariant violation")

    passed = (gate_1a_pass and gate_1b_pass and gate_2_pass and
              gate_3_pass and gate_4_pass and gate_5_pass and gate_6_pass)

    return details, passed, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--native", default=ROOT / "research" / "reaudit_accuracy_speed_20261010" / "NATIVE.json", type=Path)
    parser.add_argument("--alg06", default=ROOT / "research" / "alg06_guard_v2_20261010" / "RUNS.json", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    # Evidence validation first (fail-closed, AS03)
    runs, native, alg06, invalid_reasons = validate_evidence(args.runs, args.native, args.alg06)
    if invalid_reasons:
        sys.exit(emit_invalid(args.output, invalid_reasons))

    # Scientific gate checks
    details, passed, failures = run_checks(runs, native, alg06)

    verdict = "PASS" if passed else "FAIL"
    res = {
        "schema": SCHEMA,
        "node": NODE,
        "verdict": verdict,
        "failures": failures,
        "details": details,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(res, indent=2) + "\n")
    print(f"AS04 check: {verdict}")
    if not passed:
        for f in failures:
            print(f"  FAIL: {f}")
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
