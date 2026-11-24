#!/usr/bin/env python3
"""Loop node V25: builds RECORDING_A.json and RESULTS.json from the raw logs
only.

Inputs: PREDECLARED.json (frozen), PROTOCOL.json (protocol choices),
reference/*.json (make_reference.py), logs/<phase>/*.log, *.perf and the
per-solve logs *.solves.txt.gz. No number is typed in here: the thresholds
are the frozen ones of PREDECLARED.json (addendum A7, with A and S from
addendum A6), and the matched-error functions are imported from
../mf_stage_solve/parse_results.py (node V17), not re-written.

usage: python3 -I parse_results.py recording   (writes RECORDING_A.json)
       python3 -I parse_results.py results     (writes RESULTS.json)
"""
import argparse
import glob
import gzip
import hashlib
import importlib.util
import json
import math
import os
import statistics
import struct
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(NODE, "logs")
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
V17 = os.path.join(NODE, "..", "mf_stage_solve")

_spec = importlib.util.spec_from_file_location("v17_parse_results", os.path.join(V17, "parse_results.py"))
v17 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(v17)

# Frozen thresholds (PREDECLARED.json discriminators; A and S of addendum A6).
A_ERROR_RATIO_MAX = 3.0
S_FAMILY_MAX = 1.05
S_CELL_MAX = 1.15
E_FAMILY_BELOW = 1.00
E_CELL_MAX = 1.00
W_CELL_MAX = 1.00
G_STATE_UNITS_MAX = 10.0
G_ATTEMPTS_MAX = 0.20
# Timing quality rule of the loop.
LOAD_MAX = 8.0
SPREAD_MAX = 0.10
WARM_UP_ROUNDS = 2
REPETITIONS = 7
TRIES = (1, 2, 3)

RTOL_LABELS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
ENTRIES = ("route", "router", "router_ls", "audit")
STAGES = 8
RESULT_KEYS = ("completed", "success", "attempts", "accepted", "rejected", "state_reuses", "t_end_bits",
               "output_rows", "counters", "state_sha256", "n", "t_span", "rtol", "atol")
SHIPPED_DRIVER_ID = "rodas5p-mf-fast-transformed-v1"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def from_bits(text):
    return struct.unpack(">d", bytes.fromhex(text))[0]


def write_json(path, value):
    text = json.dumps(v17.clean(value), indent=1, allow_nan=False)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text + "\n")
    print(f"wrote {os.path.relpath(path, NODE)} ({len(text)} bytes)")


def read_log(path):
    """Tagged JSON lines of one log: {tag: [records]}, plus header and footer."""
    if not os.path.exists(path):
        return None
    out = {"begin": None, "end": None, "tags": {}, "load_before": [], "load_after": [],
           "path": os.path.relpath(path, NODE)}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("# V25_LOG_BEGIN "):
                out["begin"] = json.loads(line[len("# V25_LOG_BEGIN "):])
            elif line.startswith("# V25_LOG_END "):
                out["end"] = json.loads(line[len("# V25_LOG_END "):])
            elif line.startswith("V25_LOAD_BEFORE "):
                out["load_before"].append(line[len("V25_LOAD_BEFORE "):])
            elif line.startswith("V25_LOAD_AFTER "):
                out["load_after"].append(line[len("V25_LOAD_AFTER "):])
            elif line.startswith("V25_") and " " in line:
                tag, body = line.split(" ", 1)
                if tag.endswith("_JSON"):
                    try:
                        out["tags"].setdefault(tag, []).append(json.loads(body))
                    except ValueError:
                        out["tags"].setdefault(tag + "_UNPARSED", []).append(body[:200])
    return out


def first(log, tag):
    if log is None:
        return None
    records = log["tags"].get(tag)
    return records[0] if records else None


def collect_run(phase, name):
    """One RUN log with its perf file and its per-solve log."""
    directory = os.path.join(LOGS, phase)
    log = read_log(os.path.join(directory, name + ".log"))
    if log is None:
        return None
    record = first(log, "V25_RUN_JSON")
    state = first(log, "V25_STATE_JSON")
    begin, end = log["begin"] or {}, log["end"] or {}
    out = {
        "phase": phase, "name": name, "log": log["path"], "exit_code": end.get("exit_code"),
        "timed_out": end.get("timed_out"), "record": record, "state_hex": state["state_hex"] if state else None,
        "instructions_raw": v17.read_perf(os.path.join(directory, name + ".perf")),
        "binary_sha256": begin.get("binary_sha256"), "git_head": begin.get("git_head"),
        "git_dirty_tracked": begin.get("git_dirty_tracked"), "date": begin.get("date"),
        "solvelog": end.get("solvelog"),
    }
    out["completed"] = bool(record and record.get("completed") and record.get("success") and out["exit_code"] == 0)
    return out


def result_fields(run):
    """The fields of a run that a repetition must reproduce."""
    if run is None or not run["record"]:
        return None
    record = run["record"]
    out = {key: record.get(key) for key in RESULT_KEYS}
    out["error"] = record.get("error")
    out["state_hex_sha256"] = hashlib.sha256(run["state_hex"].encode("ascii")).hexdigest() if run["state_hex"] else None
    return out


def differing_fields(a, b):
    if a is None or b is None:
        return ["a run record is missing"]
    out = sorted(key for key in set(a) | set(b) if a.get(key) != b.get(key))
    if "counters" in out:
        ca, cb = a.get("counters") or {}, b.get("counters") or {}
        out.remove("counters")
        out.extend(sorted(f"counters.{key}" for key in set(ca) | set(cb) if ca.get(key) != cb.get(key)))
    return out


def setup_instructions(family):
    runs = [v17.read_perf(os.path.join(LOGS, "setup", f"{family}__SETUP__r{k}.perf")) for k in (1, 2, 3)]
    runs = [value for value in runs if value is not None]
    return {"runs": runs, "median": statistics.median(runs) if runs else None}


def protocol_spans():
    with open(os.path.join(NODE, "PROTOCOL.json"), encoding="utf-8") as handle:
        protocol = json.load(handle)
    return {family["id"]: family["t_span"] for family in protocol["families"]}


def load_reference(family):
    path = os.path.join(NODE, "reference", family + ".json")
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8") as handle:
        reference = json.load(handle)
    return {
        "state": v17.unhex(reference["state_hex"]), "u_ref": reference["uncertainty_max_scaled_1_plus_abs"],
        "file": os.path.relpath(path, NODE), "sha256": sha256_file(path), "n": reference["n"],
        "t_span": reference["t_span"], "method": reference["reference"], "cross_checks": [
            {key: check.get(key) for key in ("method", "rtol", "atol", "difference", "wall_seconds")}
            for check in reference["cross_checks"]],
        "y0_hex": reference.get("y0_hex"), "f0_hex": reference.get("f0_hex"),
        "state_summary": reference.get("state_summary"),
    }


# ---------------------------------------------------------------- the per-solve log (discriminator V)

def audit_solve_log(phase, solvelog):
    """Recount every completed stage solve from the per-solve log."""
    if not solvelog:
        return {"present": False}
    path = os.path.join(LOGS, phase, solvelog["file"])
    if not os.path.exists(path):
        return {"present": False, "file": os.path.relpath(path, NODE)}
    out = {"present": True, "file": os.path.relpath(path, NODE), "gz_sha256_matches": sha256_file(path) == solvelog.get("gz_sha256")}
    raw = hashlib.sha256()
    attempts = failed_attempts = failed_attempts_before_all_stages = 0
    solves = independent_violations = driver_violations = missing_records = 0
    columns_differ_from_log = 0
    max_independent = max_driver = 0.0
    zero_thresholds = 0
    columns = cycles = failed_confirmations = 0
    max_columns = max_cycles = 0
    histogram = {}
    first_violation = None
    with gzip.open(path, "rb") as handle:
        for line in handle:
            raw.update(line)
            parts = line.decode("ascii").split()
            if parts[0] == "A":
                attempts += 1
                if parts[5] == "-":
                    failed_attempts += 1
                    if int(parts[4]) < STAGES:
                        failed_attempts_before_all_stages += 1
                continue
            solves += 1
            residual, threshold = from_bits(parts[3]), from_bits(parts[4])
            if not (math.isfinite(residual) and residual <= threshold):
                independent_violations += 1
                if first_violation is None:
                    first_violation = line.decode("ascii").strip()
            if threshold > 0.0:
                ratio = residual / threshold
                if math.isfinite(ratio):
                    max_independent = max(max_independent, ratio)
            else:
                zero_thresholds += 1
            if parts[5] == "-":
                missing_records += 1
                continue
            reported, used = from_bits(parts[5]), from_bits(parts[6])
            if not (math.isfinite(reported) and reported <= used):
                driver_violations += 1
            if used > 0.0 and math.isfinite(reported / used):
                max_driver = max(max_driver, reported / used)
            solve_columns, solve_cycles = int(parts[9]), int(parts[10])
            columns += solve_columns
            cycles += solve_cycles
            failed_confirmations += int(parts[11])
            max_columns = max(max_columns, solve_columns)
            max_cycles = max(max_cycles, solve_cycles)
            if solve_columns != int(parts[12]):
                columns_differ_from_log += 1
            bucket = str(min(solve_columns, 200) // 10 * 10)
            histogram[bucket] = histogram.get(bucket, 0) + 1
    out.update({
        "raw_sha256": raw.hexdigest(), "raw_sha256_matches": raw.hexdigest() == solvelog.get("raw_sha256"),
        "attempts": attempts, "attempts_without_error_estimate": failed_attempts,
        "attempts_failed_before_the_last_stage": failed_attempts_before_all_stages,
        "solves": solves, "independent_violations": independent_violations, "driver_record_violations": driver_violations,
        "missing_driver_records": missing_records, "max_independent_residual_over_threshold": max_independent,
        "max_driver_residual_over_threshold": max_driver, "zero_thresholds": zero_thresholds,
        "columns": columns, "cycles": cycles, "failed_confirmations": failed_confirmations,
        "max_columns": max_columns, "max_cycles": max_cycles,
        "columns_per_solve": columns / solves if solves else None,
        "columns_histogram_by_10": dict(sorted(histogram.items(), key=lambda item: int(item[0]))),
        "solves_whose_record_columns_differ_from_the_solve_log": columns_differ_from_log,
        "first_violation": first_violation,
    })
    return out


def audit_verdict(plain, audited):
    """Discriminator V on one cell of one arm (PROTOCOL.json, checks 1 to 4)."""
    out = {"audited": bool(audited and audited["record"] and audited["record"].get("completed"))}
    if not out["audited"]:
        out.update({"violations": None, "ok": False, "deterministic": None, "reason": "no completed audit run"})
        return out
    record = audited["record"]
    counters = record.get("counters") or {}
    protected = record.get("residual_audit") or {}
    recorder = record.get("solve_recorder") or {}
    own = record.get("record_audit") or {}
    stats = record.get("policy_statistics") or {}
    recount = audit_solve_log(audited["phase"], audited["solvelog"])
    solves = counters.get("linear_solves")
    failures = counters.get("linear_solve_failures")
    coverage = []
    if not recount.get("present"):
        coverage.append("the per-solve log is missing")
    else:
        if not (recount.get("gz_sha256_matches") and recount.get("raw_sha256_matches")):
            coverage.append("the per-solve log does not match the sha256 of its run")
        if recount["solves"] != solves:
            coverage.append(f"per-solve log lines {recount['solves']} != completed solves {solves}")
        if recount["missing_driver_records"]:
            coverage.append(f"{recount['missing_driver_records']} solves without a driver record")
        if recount["attempts"] != record.get("attempts"):
            coverage.append(f"attempt lines {recount['attempts']} != attempts {record.get('attempts')}")
    if protected.get("checks") != solves:
        coverage.append(f"ProtectedAudit checks {protected.get('checks')} != completed solves {solves}")
    if own.get("records") != solves:
        coverage.append(f"driver records {own.get('records')} != completed solves {solves}")
    if stats.get("completed_solves") != solves:
        coverage.append(f"statistics.completed_solves {stats.get('completed_solves')} != {solves}")
    classified = (stats.get("budget_exhausted_solves") or 0) + (stats.get("other_linear_failures") or 0)
    if classified != failures:
        coverage.append(f"classified failures {classified} != linear_solve_failures {failures}")
    if recount.get("present") and protected.get("failures") != recount["independent_violations"]:
        coverage.append(f"ProtectedAudit failures {protected.get('failures')} != recounted {recount['independent_violations']}")
    same = differing_fields(result_fields(plain), result_fields(audited))
    out.update({
        "completed_solves": solves, "failed_solves": failures,
        "budget_exhausted_solves": stats.get("budget_exhausted_solves"),
        "other_linear_failures": stats.get("other_linear_failures"),
        "recount_from_the_solve_log": recount,
        "protected_audit": protected, "solve_recorder": recorder, "driver_record_audit": own,
        "coverage_mismatches": coverage, "audit_run_differs_from_route_run_in": same,
    })
    out["violations"] = (recount.get("independent_violations") or 0) + (recount.get("driver_record_violations") or 0) + len(coverage)
    out["deterministic"] = not same
    out["ok"] = out["violations"] == 0 and out["deterministic"] and (solves or 0) > 0
    return out


# ---------------------------------------------------------------- runs

def summarise_run(run, reference, setup_median, rtol):
    if run is None:
        return {"present": False, "completed": False}
    record = run["record"] or {}
    counters = record.get("counters") or {}
    out = {
        "present": True, "log": run["log"], "completed": run["completed"], "success": record.get("success"),
        "error": record.get("error"), "exit_code": run["exit_code"], "timed_out": run["timed_out"],
        "arm": record.get("arm"), "entry": record.get("entry"), "driver": record.get("driver"),
        "policy": record.get("policy"), "attempts": record.get("attempts"), "accepted": record.get("accepted"),
        "rejected": record.get("rejected"), "jvp_calls": record.get("jvp_calls"), "rhs_calls": counters.get("rhs_calls"),
        "linear_solves": counters.get("linear_solves"), "linear_iterations": counters.get("linear_iterations"),
        "linear_solve_failures": counters.get("linear_solve_failures"),
        "local_error_failures": counters.get("local_error_failures"),
        "orthogonalization_inner_products": counters.get("orthogonalization_inner_products"),
        "state_sha256": record.get("state_sha256"), "t_span": record.get("t_span"), "n": record.get("n"),
        "wall_seconds_diagnostic_only": record.get("wall_seconds_diagnostic_only"),
        "instructions_raw": run["instructions_raw"], "binary_sha256": run["binary_sha256"],
    }
    if run["instructions_raw"] is not None and setup_median is not None:
        out["instructions_net"] = run["instructions_raw"] - setup_median
    if run["state_hex"] and reference is not None and run["completed"]:
        state = v17.unhex(run["state_hex"])
        if len(state) == len(reference["state"]):
            metrics = v17.error_metrics(state, reference["state"])
            out.update(metrics)
            out["E_over_rtol"] = metrics["E"] / rtol
            out["reference_limited"] = metrics["E"] < 10.0 * reference["u_ref"]
    return out


def collect_arm(phase, family, arm, reference, spans, store):
    """Per tolerance: the route run (instructions), the audit run, V."""
    setup = setup_instructions(family)
    cells = {}
    for label in RTOL_LABELS:
        plain = collect_run(phase, f"{family}__{label}__{arm}__route")
        audited = collect_run(phase, f"{family}__{label}__{arm}__audit")
        for run in (plain, audited):
            note_identity(store, run)
        cell = summarise_run(plain, reference, setup["median"], float(label))
        cell["audit"] = audit_verdict(plain, audited)
        if audited and audited["record"]:
            for key in ("policy_statistics", "record_audit", "solve_log", "residual_audit"):
                if audited["record"].get(key) is not None:
                    cell[key] = audited["record"][key]
            stats = audited["record"].get("policy_statistics") or {}
            cell["budget_exhausted_solves"] = stats.get("budget_exhausted_solves")
        if plain and plain["record"]:
            if plain["record"].get("t_span") != spans[family] or (reference and plain["record"].get("t_span") != reference["t_span"]):
                cell["completed"] = False
                cell["error"] = "the run is not on the span of PROTOCOL.json and of the reference"
        cells[label] = cell
    return {"setup_instructions": setup, "cells": cells}


def note_identity(store, run):
    if run is None:
        return
    if run.get("binary_sha256"):
        store["binaries"][run["binary_sha256"]] = store["binaries"].get(run["binary_sha256"], 0) + 1
    if run.get("git_head"):
        key = f"{run['git_head']} dirty_tracked={run.get('git_dirty_tracked')}"
        store["git_heads"][key] = store["git_heads"].get(key, 0) + 1


def curve(cells, measure):
    points, completed = [], 0
    for label in RTOL_LABELS:
        cell = cells[label]
        if not cell.get("completed") or cell.get("E") is None or cell.get(measure) is None:
            continue
        completed += 1
        points.append((cell["E"], float(cell[measure])))
    return points, completed


def geometric_mean(values):
    values = [value for value in values if value is not None]
    if not values or any(value <= 0 for value in values):
        return None
    return math.exp(sum(math.log(value) for value in values) / len(values))


def containing_segments(points, level):
    """Indices (into `points`) of the endpoints of every segment that contains `level`."""
    used = set()
    for k, ((e0, _), (e1, _)) in enumerate(zip(points, points[1:])):
        if min(e0, e1) <= level <= max(e0, e1):
            used.update((k, k + 1))
    return sorted(used)


def matched_cell(points_s_cell, points_x, block, measure_s):
    """P1 at the error S achieved in this cell over S, or None when outside the matched range."""
    e_s = points_s_cell
    if block.get("status") != "OK":
        return None
    lo, hi = block["range"]
    if not lo <= e_s <= hi:
        return None
    wx = v17.curve_value(points_x, e_s, max)
    return None if wx is None else wx / measure_s


def compare_arms(cells_s, cells_x, u_ref):
    """Arm X against S on one family: the V17 statistic, the cell numbers of
    PROTOCOL.json and the ratios at equal tolerance."""
    floor = 10.0 * u_ref
    out = {"matched_error": {}, "cells": {}}
    for measure in ("instructions_net", "attempts", "jvp_calls"):
        points_s, completed_s = curve(cells_s, measure)
        points_x, completed_x = curve(cells_x, measure)
        out["matched_error"][measure] = v17.matched(points_s, points_x, floor, completed_s, completed_x, 4)
    block = out["matched_error"]["instructions_net"]
    points_x, _ = curve(cells_x, "instructions_net")
    for label in RTOL_LABELS:
        s, x = cells_s[label], cells_x[label]
        both = bool(s.get("completed") and x.get("completed"))
        row = {"both_completed": both}
        if both:
            row.update({
                "instructions_equal_tolerance": v17.ratio(x.get("instructions_net"), s.get("instructions_net")),
                "attempts_equal_tolerance": v17.ratio(x.get("attempts"), s.get("attempts")),
                "jvp_equal_tolerance": v17.ratio(x.get("jvp_calls"), s.get("jvp_calls")),
                "columns_equal_tolerance": v17.ratio(x.get("linear_iterations"), s.get("linear_iterations")),
                "error_ratio": v17.ratio(x.get("E"), s.get("E")), "E_S": s.get("E"), "E_X": x.get("E"),
                "reference_limited": bool(s.get("reference_limited") or x.get("reference_limited")),
                "wall_diagnostic_single_run_speedup": v17.ratio(s.get("wall_seconds_diagnostic_only"),
                                                               x.get("wall_seconds_diagnostic_only")),
            })
            value = None
            if s.get("E") is not None and s.get("instructions_net"):
                value = matched_cell(s["E"], points_x, block, s["instructions_net"])
            if value is not None:
                row["cell_number"] = value
                row["cell_number_kind"] = "matched error at the error of arm S"
            else:
                row["cell_number"] = row["instructions_equal_tolerance"]
                row["cell_number_kind"] = "equal tolerance (the error of S in this cell is outside the matched range)"
                cheaper = [w for e, w in points_x if s.get("E") is not None and e <= s["E"]]
                row["bound_cheapest_P_point_at_least_as_accurate"] = (
                    min(cheaper) / s["instructions_net"] if cheaper and s.get("instructions_net") else None)
                bound = row["bound_cheapest_P_point_at_least_as_accurate"]
                row["verdict_would_differ_under_the_bound"] = (
                    None if bound is None or row["cell_number"] is None else (bound <= E_CELL_MAX) != (row["cell_number"] <= E_CELL_MAX))
        out["cells"][label] = row
    rows = [out["cells"][label] for label in RTOL_LABELS]
    complete = all(row["both_completed"] for row in rows)
    out["equal_tolerance"] = {
        "instructions_geometric_mean": geometric_mean([row.get("instructions_equal_tolerance") for row in rows]) if complete else None,
        "attempts_geometric_mean": geometric_mean([row.get("attempts_equal_tolerance") for row in rows]) if complete else None,
        "jvp_geometric_mean": geometric_mean([row.get("jvp_equal_tolerance") for row in rows]) if complete else None,
        "columns_geometric_mean": geometric_mean([row.get("columns_equal_tolerance") for row in rows]) if complete else None,
    }
    out["cell_numbers_geometric_mean"] = (
        geometric_mean([row.get("cell_number") for row in rows]) if all(row.get("cell_number") is not None for row in rows) else None)
    return out


# ---------------------------------------------------------------- timing (discriminator W)

def timing_paths(family, label, attempt):
    directory = os.path.join(LOGS, "timing")
    single = os.path.join(directory, f"{family}__{label}__try{attempt}.log")
    if os.path.exists(single):
        return [single]
    return sorted(glob.glob(os.path.join(directory, f"{family}__{label}__try{attempt}__round*.log")))


def collect_timing(family, label, plain_states):
    """Every timing measurement of a cell, in the order of the tries (V17 rule, arms S and P1)."""
    arms = ("S", "P1")
    tries = []
    for attempt in TRIES:
        paths = timing_paths(family, label, attempt)
        if not paths:
            continue
        logs = [read_log(path) for path in paths]
        runs, loads_before, loads_after, metas, complete, seconds = [], [], [], [], True, []
        for log in logs:
            runs.extend(log["tags"].get("V25_TIMING_JSON", []))
            metas.extend(log["tags"].get("V25_TIMING_META_JSON", []))
            loads_before.extend(log["load_before"])
            loads_after.extend(log["load_after"])
            if not log["end"] or log["end"].get("exit_code") != 0:
                complete = False
            if log["end"]:
                seconds.append(log["end"].get("seconds"))
        entry = {
            "try": attempt, "logs": [log["path"] for log in logs], "invocations": len(logs),
            "process_complete": complete, "load_before": loads_before, "load_after": loads_after,
            "dates": [(log["begin"] or {}).get("date") for log in logs],
            "binaries": sorted({(log["begin"] or {}).get("binary_sha256") for log in logs}, key=str),
            "longest_invocation_seconds_including_the_wait_for_the_lock": max([v for v in seconds if v is not None], default=None),
            "longest_process_seconds_under_the_lock": max([m.get("process_seconds") for m in metas if m.get("process_seconds") is not None], default=None),
            "arms": {},
        }
        values = [float(text.split()[0]) for text in loads_before + loads_after if text.split()]
        entry["load_1min_max"] = max(values) if values else None
        reasons = []
        if not complete:
            reasons.append("an invocation did not exit with 0")
        if len(loads_before) != len(logs) or len(loads_after) != len(logs):
            reasons.append("a load reading is missing")
        elif entry["load_1min_max"] > LOAD_MAX:
            reasons.append(f"1-minute load average {entry['load_1min_max']} above {LOAD_MAX}")
        for name in arms:
            mine = sorted((run for run in runs if run.get("arm") == name), key=lambda run: run.get("round", 0))
            warm = [run["wall_seconds"] for run in mine if run.get("round", 0) < WARM_UP_ROUNDS]
            reps = [run for run in mine if run.get("round", 0) >= WARM_UP_ROUNDS]
            walls = [run["wall_seconds"] for run in reps]
            states_ok = bool(mine) and all(run.get("completed") and run.get("success")
                                           and run.get("state_sha256") == plain_states.get(name) for run in mine)
            arm_entry = {"warm_up_walls": warm, "walls": walls, "repetitions": len(walls),
                         "states_equal_instruction_run": states_ok}
            if walls:
                median = statistics.median(walls)
                arm_entry.update({"median": median, "min": min(walls), "max": max(walls),
                                  "spread": (max(walls) - min(walls)) / median if median > 0 else None})
            cpus = [run.get("thread_cpu_seconds") for run in reps]
            if cpus and all(value is not None for value in cpus):
                median_cpu = statistics.median(cpus)
                arm_entry.update({"median_thread_cpu": median_cpu,
                                  "spread_thread_cpu": (max(cpus) - min(cpus)) / median_cpu if median_cpu > 0 else None})
            entry["arms"][name] = arm_entry
            if len(walls) != REPETITIONS or len(warm) != WARM_UP_ROUNDS:
                reasons.append(f"arm {name}: {len(warm)} warm-ups and {len(walls)} repetitions")
            elif arm_entry["spread"] is None or arm_entry["spread"] > SPREAD_MAX:
                reasons.append(f"arm {name}: spread {arm_entry['spread']:.3f} above {SPREAD_MAX}")
            if not states_ok:
                reasons.append(f"arm {name}: a timed run did not end in the state of the instruction run")
        entry["meets_rule"] = not reasons
        entry["misses"] = reasons
        tries.append(entry)
    chosen = next((entry for entry in tries if entry["meets_rule"]), None)
    out = {"tries": tries, "measurements": len(tries)}
    if chosen is not None:
        out.update({"label": "authoritative", "used_try": chosen["try"]})
    elif tries:
        out.update({"label": "diagnostic_only", "used_try": tries[-1]["try"]})
    else:
        out.update({"label": "NOT_RUN", "used_try": None})
    used = chosen or (tries[-1] if tries else None)
    if used:
        out["median_wall"] = {name: used["arms"][name].get("median") for name in arms}
        out["spread"] = {name: used["arms"][name].get("spread") for name in arms}
        out["median_thread_cpu"] = {name: used["arms"][name].get("median_thread_cpu") for name in arms}
        out["speedup_equal_tolerance"] = v17.ratio(out["median_wall"].get("S"), out["median_wall"].get("P1"))
    return out


def evaluate_wall(cells_s, cells_x, timings, u_ref):
    """W per cell: P1's median wall at the error S achieved over S's median wall."""
    floor = 10.0 * u_ref
    points = {"S": [], "P1": []}
    labels_of = {"S": [], "P1": []}
    for name, cells in (("S", cells_s), ("P1", cells_x)):
        for label in RTOL_LABELS:
            cell = cells[label]
            wall = (timings[label].get("median_wall") or {}).get(name)
            if not cell.get("completed") or cell.get("E") is None or wall is None:
                continue
            points[name].append((cell["E"], float(wall)))
            labels_of[name].append(label)
    block = v17.matched(points["S"], points["P1"], floor, len(points["S"]), len(points["P1"]), 4)
    cells = {}
    for label in RTOL_LABELS:
        s = cells_s[label]
        timing = timings[label]
        wall_s = (timing.get("median_wall") or {}).get("S")
        row = {"timing_label": timing["label"], "used_try": timing.get("used_try")}
        if wall_s is None or s.get("E") is None:
            row.update({"ratio": None, "authoritative": False, "kind": "not measured"})
            cells[label] = row
            continue
        value = None
        used_labels = [label]
        if block.get("status") == "OK" and block["range"][0] <= s["E"] <= block["range"][1]:
            value = v17.curve_value(points["P1"], s["E"], max)
            if value is not None:
                used_labels = sorted({label} | {labels_of["P1"][k] for k in containing_segments(points["P1"], s["E"])},
                                     key=RTOL_LABELS.index)
        if value is not None:
            row.update({"ratio": value / wall_s, "kind": "matched error at the error of arm S"})
        else:
            wall_x = (timing.get("median_wall") or {}).get("P1")
            row.update({"ratio": v17.ratio(wall_x, wall_s), "kind": "equal tolerance (outside the matched range)"})
        row["timing_cells_used"] = used_labels
        row["authoritative"] = all(timings[used]["label"] == "authoritative" for used in used_labels)
        row["slower"] = row["ratio"] is not None and row["ratio"] > W_CELL_MAX
        cells[label] = row
    measured = [row for row in cells.values() if row.get("kind") != "not measured"]
    failing = [label for label, row in cells.items() if row.get("authoritative") and row.get("slower")]
    if failing:
        verdict = "FAIL"
    elif len(measured) < len(RTOL_LABELS):
        verdict = "NOT_DECIDABLE"
    else:
        verdict = "PASS"
    speed = {}
    if block.get("geometric_mean_ratio"):
        speed = {"speedup_at_matched_error": 1.0 / block["geometric_mean_ratio"],
                 "speedup_min": 1.0 / block["max_ratio"], "speedup_max": 1.0 / block["min_ratio"]}
    return {"cells": cells, "family_statistic": {k: block.get(k) for k in ("status", "reason", "range", "geometric_mean_ratio", "min_ratio", "max_ratio")},
            **speed, "authoritative_cells": sum(1 for row in cells.values() if row.get("authoritative")),
            "slower_authoritative_cells": failing, "verdict": verdict}


# ---------------------------------------------------------------- discriminators per family

def verdict_of(parts):
    if any(part == "FAIL" for part in parts):
        return "FAIL"
    if any(part != "PASS" for part in parts):
        return "NOT_DECIDABLE"
    return "PASS"


def evaluate_family(cells_s, cells_x, comparison):
    out = {}
    rows = comparison["cells"]
    complete = all(rows[label]["both_completed"] for label in RTOL_LABELS)

    a_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("error_ratio")
        a_cells[label] = {"error_ratio": value, "E_S": rows[label].get("E_S"), "E_P1": rows[label].get("E_X"),
                          "reference_limited": rows[label].get("reference_limited"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= A_ERROR_RATIO_MAX else "FAIL")}
    out["A"] = {"cells": a_cells, "max_error_ratio": max((c["error_ratio"] for c in a_cells.values() if c["error_ratio"] is not None), default=None),
                "verdict": verdict_of([c["verdict"] for c in a_cells.values()])}

    block = comparison["matched_error"]["instructions_net"]
    family_number = block.get("geometric_mean_ratio")
    e_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("cell_number")
        e_cells[label] = {"cell_number": value, "kind": rows[label].get("cell_number_kind"),
                          "equal_tolerance": rows[label].get("instructions_equal_tolerance"),
                          "bound_cheapest_P_point_at_least_as_accurate": rows[label].get("bound_cheapest_P_point_at_least_as_accurate"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= E_CELL_MAX else "FAIL")}
    family_part = "NOT_DECIDABLE" if family_number is None else ("PASS" if family_number < E_FAMILY_BELOW else "FAIL")
    cells_part = verdict_of([c["verdict"] for c in e_cells.values()])
    out["E"] = {
        "family_number": family_number, "family_part": family_part, "matched_range": block.get("range"),
        "matched_status": block.get("status"), "matched_reason": block.get("reason"),
        "cells": e_cells, "cells_part": cells_part,
        "max_cell_number": max((c["cell_number"] for c in e_cells.values() if c["cell_number"] is not None), default=None),
        "verdict": verdict_of([family_part, cells_part]),
        "also_reported": {
            "geometric_mean_of_the_cell_numbers": comparison["cell_numbers_geometric_mean"],
            "geometric_mean_at_equal_tolerance": comparison["equal_tolerance"]["instructions_geometric_mean"],
            "largest_ratio_on_the_grid": block.get("max_ratio"), "smallest_ratio_on_the_grid": block.get("min_ratio"),
            "ratio_at_points_of_S": block.get("ratio_at_points_of_A"),
            "attempts_matched_error": comparison["matched_error"]["attempts"].get("geometric_mean_ratio"),
            "jvp_matched_error": comparison["matched_error"]["jvp_calls"].get("geometric_mean_ratio"),
        },
    }

    s_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("attempts_equal_tolerance")
        s_cells[label] = {"attempts_ratio": value, "attempts_S": cells_s[label].get("attempts"), "attempts_P1": cells_x[label].get("attempts"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= S_CELL_MAX else "FAIL")}
    mean = comparison["equal_tolerance"]["attempts_geometric_mean"] if complete else None
    family_part = "NOT_DECIDABLE" if mean is None else ("PASS" if mean <= S_FAMILY_MAX else "FAIL")
    out["S"] = {"geometric_mean": mean, "family_part": family_part, "cells": s_cells,
                "cells_part": verdict_of([c["verdict"] for c in s_cells.values()]),
                "max_ratio": max((c["attempts_ratio"] for c in s_cells.values() if c["attempts_ratio"] is not None), default=None)}
    out["S"]["verdict"] = verdict_of([family_part, out["S"]["cells_part"]])

    b_cells = {}
    for label in RTOL_LABELS:
        bs, bx = cells_s[label].get("budget_exhausted_solves"), cells_x[label].get("budget_exhausted_solves")
        b_cells[label] = {"S": bs, "P1": bx, "linear_solve_failures_S": cells_s[label].get("linear_solve_failures"),
                          "linear_solve_failures_P1": cells_x[label].get("linear_solve_failures"),
                          "verdict": "NOT_DECIDABLE" if bs is None or bx is None else ("PASS" if bx <= bs else "FAIL")}
    out["B"] = {"cells": b_cells, "total_S": sum(c["S"] or 0 for c in b_cells.values()),
                "total_P1": sum(c["P1"] or 0 for c in b_cells.values()), "verdict": verdict_of([c["verdict"] for c in b_cells.values()])}

    v_cells, totals, max_ratio = {}, {"completed_solves": 0, "failed_solves": 0, "violations": 0}, 0.0
    for label in RTOL_LABELS:
        audit = cells_x[label]["audit"]
        recount = audit.get("recount_from_the_solve_log") or {}
        v_cells[label] = {key: audit.get(key) for key in ("audited", "ok", "deterministic", "violations", "completed_solves",
                                                            "failed_solves", "budget_exhausted_solves", "other_linear_failures",
                                                            "coverage_mismatches", "audit_run_differs_from_route_run_in")}
        v_cells[label]["recount"] = {key: recount.get(key) for key in (
            "solves", "independent_violations", "driver_record_violations", "max_independent_residual_over_threshold",
            "max_driver_residual_over_threshold", "attempts", "attempts_without_error_estimate", "columns_per_solve",
            "max_columns", "failed_confirmations", "file", "raw_sha256_matches", "gz_sha256_matches")}
        for key in ("completed_solves", "failed_solves", "violations"):
            totals[key] += audit.get(key) or 0
        max_ratio = max(max_ratio, recount.get("max_independent_residual_over_threshold") or 0.0,
                        recount.get("max_driver_residual_over_threshold") or 0.0)
    all_audited = all(v_cells[label]["audited"] for label in RTOL_LABELS)
    all_ok = all(v_cells[label]["ok"] for label in RTOL_LABELS)
    out["V"] = {"cells": v_cells, "totals": totals, "max_residual_over_threshold": max_ratio,
                "verdict": "PASS" if all_ok else ("FAIL" if totals["violations"] > 0 else "NOT_DECIDABLE")}
    out["V"]["shipped_arm_reported"] = {
        "violations": sum((cells_s[label]["audit"].get("violations") or 0) for label in RTOL_LABELS),
        "completed_solves": sum((cells_s[label]["audit"].get("completed_solves") or 0) for label in RTOL_LABELS),
        "all_ok": all(cells_s[label]["audit"].get("ok") for label in RTOL_LABELS)}
    return out


# ---------------------------------------------------------------- recording of commit A and D0

def build_recording():
    spans = protocol_spans()
    out = {"schema": "vigilode.loop-l1.v25.recording-a.v1", "node": "V25_MF_P1_CONFIRMATION",
           "what": "arms S and P1, each named explicitly, on the 20 deciding cells, before any edit of the driver or of routing.rs (commit A): entries route, router, router_ls, audit (PROTOCOL.json); D0 compares against this file",
           "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"), "binaries": {}, "git_heads": {}, "cells": {}, "checks": []}
    expected = present = 0
    for family in FAMILIES:
        out["cells"][family] = {}
        for label in RTOL_LABELS:
            cell = {}
            for arm in ("S", "P1"):
                rows, fields = {}, {}
                for entry in ENTRIES:
                    expected += 1
                    run = collect_run("record_a", f"{family}__{label}__{arm}__{entry}")
                    note_identity(out, run)
                    if run is None or not run["record"]:
                        out["checks"].append(f"{family} {label} {arm} {entry}: no run record")
                        continue
                    present += 1
                    record = run["record"]
                    fields[entry] = result_fields(run)
                    row = dict(fields[entry])
                    row.update({"log": run["log"], "exit_code": run["exit_code"], "driver": record.get("driver"),
                                "routed_driver_id": record.get("routed_driver_id"), "policy": record.get("policy"),
                                "krylov_workspace": record.get("krylov_workspace")})
                    if run["instructions_raw"] is not None:
                        row["instructions_raw"] = run["instructions_raw"]
                    for key in ("solve_log", "residual_audit", "record_audit", "policy_statistics"):
                        if record.get(key) is not None:
                            row[key] = record[key]
                    if run["solvelog"]:
                        row["solvelog_raw_sha256"] = run["solvelog"].get("raw_sha256")
                    if not run["completed"]:
                        out["checks"].append(f"{family} {label} {arm} {entry}: the run did not complete")
                    if record.get("t_span") != spans[family]:
                        out["checks"].append(f"{family} {label} {arm} {entry}: span {record.get('t_span')} is not the span of PROTOCOL.json")
                    rows[entry] = row
                agreement = {entry: differing_fields(fields.get("route"), fields.get(entry)) for entry in ENTRIES[1:]}
                for entry, differing in agreement.items():
                    if differing:
                        out["checks"].append(f"{family} {label} {arm}: entry {entry} differs from route in {differing}")
                rows["entries_differing_from_route"] = agreement
                cell[arm] = rows
            out["cells"][family][label] = cell
    out["summary"] = {"runs_expected": expected, "runs_present": present, "problems": len(out["checks"])}
    return out


def build_d0(store):
    """After an edit: S and P1 through the entries of commit A with the new binary."""
    if not os.path.isdir(os.path.join(LOGS, "d0")):
        return {"verdict": "NOT_RUN", "reason": "no edit of the driver or the router was made (logs/d0 is absent)"}
    rows, compared, equal, problems = {}, 0, 0, []
    for family in FAMILIES:
        for label in RTOL_LABELS:
            for arm in ("S", "P1"):
                for entry in ENTRIES:
                    before = collect_run("record_a", f"{family}__{label}__{arm}__{entry}")
                    after = collect_run("d0", f"{family}__{label}__{arm}__{entry}")
                    if after is None:
                        continue
                    note_identity(store, after)
                    differing = differing_fields(result_fields(before), result_fields(after))
                    if entry == "audit" and before and after and before["record"] and after["record"]:
                        for key in ("solve_log", "residual_audit", "record_audit", "policy_statistics"):
                            if before["record"].get(key) != after["record"].get(key):
                                differing.append(key)
                        if (before["solvelog"] or {}).get("raw_sha256") != (after["solvelog"] or {}).get("raw_sha256"):
                            differing.append("per-solve log")
                    compared += 1
                    equal += int(not differing)
                    key = f"{family} {label} {arm} {entry}"
                    rows[key] = {"differing": differing,
                                 "driver_before": ((before or {}).get("record") or {}).get("driver"),
                                 "driver_after": ((after or {}).get("record") or {}).get("driver"),
                                 "routed_driver_id_after": ((after or {}).get("record") or {}).get("routed_driver_id")}
                    if differing:
                        problems.append(f"{key}: {differing}")
    expected = len(FAMILIES) * len(RTOL_LABELS) * 2 * len(ENTRIES)
    verdict = "NOT_RUN" if compared == 0 else ("PASS" if compared == expected and equal == compared else ("FAIL" if problems else "NOT_DECIDABLE"))
    return {"compared": compared, "expected": expected, "equal": equal, "differences": problems, "runs": rows, "verdict": verdict}


# ---------------------------------------------------------------- results

def rhs0_consistency(family, reference):
    log = read_log(os.path.join(LOGS, "setup", f"{family}__RHS0.log"))
    rhs0 = first(log, "V25_RHS0_JSON")
    if not rhs0 or not reference or not reference.get("f0_hex"):
        return {"consistent": False, "reason": "no RHS0 record or no reference"}
    y0_rust, f0_rust = v17.unhex(rhs0["y0_hex"]), v17.unhex(rhs0["f0_hex"])
    y0_ref, f0_ref = v17.unhex(reference["y0_hex"]), v17.unhex(reference["f0_hex"])
    scale = max(abs(value) for value in f0_ref) or 1.0
    out = {"y0_max_abs_difference": max(abs(a - b) for a, b in zip(y0_rust, y0_ref)),
           "f0_max_relative_difference": max(abs(a - b) for a, b in zip(f0_rust, f0_ref)) / scale,
           "rule": "V17: max |f0_rust - f0_numpy| / max |f0_numpy| <= 1e-12 and max |y0_rust - y0_numpy| <= 1e-12"}
    out["consistent"] = out["f0_max_relative_difference"] <= 1e-12 and out["y0_max_abs_difference"] <= 1e-12
    return out


def build_results():
    spans = protocol_spans()
    results = {
        "schema": "vigilode.loop-l1.v25.results.v1", "node": "V25_MF_P1_CONFIRMATION",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "predeclared_sha256": sha256_file(os.path.join(NODE, "PREDECLARED.json")),
        "protocol_sha256": sha256_file(os.path.join(NODE, "PROTOCOL.json")),
        "frozen_thresholds": {"A_error_ratio_max": A_ERROR_RATIO_MAX, "S_family_max": S_FAMILY_MAX, "S_cell_max": S_CELL_MAX,
                              "E_family_below": E_FAMILY_BELOW, "E_cell_max": E_CELL_MAX, "W_cell_max": W_CELL_MAX,
                              "G_state_units_max": G_STATE_UNITS_MAX, "G_attempts_max": G_ATTEMPTS_MAX,
                              "timing_load_max": LOAD_MAX, "timing_spread_max": SPREAD_MAX},
        "arms": {"S": "MfStageSolvePolicy::Shipped, named explicitly", "P1": "MfStageSolvePolicy::ColumnTested, named explicitly",
                 "P2": "MfStageSolvePolicy::ColumnTestedDoubling, reported only"},
        "binaries": {}, "git_heads": {}, "checks": [], "families": {},
    }
    problems = results["checks"]
    with open(os.path.join(NODE, "CONSISTENCY.json"), encoding="utf-8") as handle:
        consistency = json.load(handle)
    results["consistency_check"] = {"all_pass": consistency["all_pass"],
                                    "families": {f: {k: v for k, v in block.items() if k in ("passes", "max_d_f", "max_d_jv", "all_components_bitwise_equal")}
                                                 for f, block in consistency["families"].items()}}
    if not consistency["all_pass"]:
        problems.append("the frozen consistency check of the right-hand sides failed")
    all_violations = {"P1": 0, "P2_reported": 0}
    for family in FAMILIES:
        reference = load_reference(family)
        if reference is None:
            problems.append(f"{family}: no reference")
        shipped = collect_arm("record_a", family, "S", reference, spans, results)
        new = collect_arm("record_a", family, "P1", reference, spans, results)
        p2 = collect_arm("p2", family, "P2", reference, spans, results)
        entry = {"n": reference["n"] if reference else None, "t_span": spans[family],
                 "reference": {key: reference[key] for key in ("file", "sha256", "u_ref", "method", "cross_checks", "state_summary")} if reference else None,
                 "setup_instructions": shipped["setup_instructions"],
                 "arms": {"S": shipped["cells"], "P1": new["cells"], "P2": p2["cells"]}}
        entry["rhs0_consistency"] = rhs0_consistency(family, reference)
        if not entry["rhs0_consistency"]["consistent"]:
            problems.append(f"{family}: the start state or f(t0, y0) of the two implementations differ")
        for name, cells in (("S", shipped["cells"]), ("P1", new["cells"]), ("P2", p2["cells"])):
            for label in RTOL_LABELS:
                if not cells[label].get("completed"):
                    problems.append(f"{family} {label} {name}: the run did not complete ({cells[label].get('error')})")
        comparison = compare_arms(shipped["cells"], new["cells"], reference["u_ref"])
        entry["comparison"] = comparison
        entry["discriminators"] = evaluate_family(shipped["cells"], new["cells"], comparison)
        all_violations["P1"] += entry["discriminators"]["V"]["totals"]["violations"]
        timings = {}
        for label in RTOL_LABELS:
            plain_states = {"S": shipped["cells"][label].get("state_sha256"), "P1": new["cells"][label].get("state_sha256")}
            timings[label] = collect_timing(family, label, plain_states)
        entry["timing_cells"] = timings
        entry["discriminators"]["W"] = evaluate_wall(shipped["cells"], new["cells"], timings, reference["u_ref"])
        # P2, reported only.
        comparison_p2 = compare_arms(shipped["cells"], p2["cells"], reference["u_ref"])
        family_p2 = evaluate_family(shipped["cells"], p2["cells"], comparison_p2)
        all_violations["P2_reported"] += family_p2["V"]["totals"]["violations"]
        entry["P2_reported_only"] = {
            "instructions_matched_error": comparison_p2["matched_error"]["instructions_net"].get("geometric_mean_ratio"),
            "instructions_equal_tolerance": comparison_p2["equal_tolerance"]["instructions_geometric_mean"],
            "attempts_equal_tolerance": comparison_p2["equal_tolerance"]["attempts_geometric_mean"],
            "max_error_ratio": family_p2["A"]["max_error_ratio"], "max_attempts_ratio": family_p2["S"]["max_ratio"],
            "budget_exhausted": {"S": family_p2["B"]["total_S"], "P2": family_p2["B"]["total_P1"]},
            "budget_cells": {label: {"S": c["S"], "P2": c["P1"]} for label, c in family_p2["B"]["cells"].items()},
            "verdicts_if_it_were_the_candidate": {name: family_p2[name]["verdict"] for name in ("A", "E", "S", "B", "V")},
            "audit_violations": family_p2["V"]["totals"]["violations"],
        }
        results["families"][family] = entry

    families = results["families"]

    def per_family(name):
        return {family: families[family]["discriminators"][name]["verdict"] for family in FAMILIES}

    discriminators = {}
    # D0: no edit before the decision; after an edit, the d0 runs.
    binaries_of_runs = sorted(results["binaries"])
    changed = subprocess.run(["git", "-C", ROOT, "diff", "--name-only", "80ba83881503c4b3253268df61c53b269740d6d3", "--",
                              "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs", "crates/rodas5p-integrators/src/routing.rs"],
                             capture_output=True, text=True, check=False).stdout.split()
    d0_after = build_d0(results)
    discriminators["D0"] = {
        "binaries_of_the_deciding_runs": binaries_of_runs,
        "driver_or_router_changed_since_the_base": changed,
        "after_the_edit": d0_after,
    }
    if d0_after["verdict"] == "NOT_RUN":
        discriminators["D0"]["verdict"] = "PASS" if not changed and len(binaries_of_runs) == 1 else "NOT_DECIDABLE"
        discriminators["D0"]["reading"] = "no edit of the driver or the router before the decision, and one binary for every deciding run"
    else:
        discriminators["D0"]["verdict"] = d0_after["verdict"]
        discriminators["D0"]["reading"] = "S and P1 named explicitly after the edit against the recording of commit A"
    for name in ("A", "E", "S", "B", "W"):
        verdicts = per_family(name)
        discriminators[name] = {"families": verdicts, "verdict": verdict_of(list(verdicts.values()))}
    discriminators["A"]["max_error_ratios"] = {f: families[f]["discriminators"]["A"]["max_error_ratio"] for f in FAMILIES}
    discriminators["E"]["family_numbers"] = {f: families[f]["discriminators"]["E"]["family_number"] for f in FAMILIES}
    discriminators["E"]["max_cell_numbers"] = {f: families[f]["discriminators"]["E"]["max_cell_number"] for f in FAMILIES}
    discriminators["S"]["geometric_means"] = {f: families[f]["discriminators"]["S"]["geometric_mean"] for f in FAMILIES}
    discriminators["S"]["max_ratios"] = {f: families[f]["discriminators"]["S"]["max_ratio"] for f in FAMILIES}
    discriminators["B"]["budget_exhausted"] = {f: {"S": families[f]["discriminators"]["B"]["total_S"], "P1": families[f]["discriminators"]["B"]["total_P1"]} for f in FAMILIES}
    discriminators["W"]["speedup_at_matched_error"] = {f: families[f]["discriminators"]["W"].get("speedup_at_matched_error") for f in FAMILIES}
    discriminators["W"]["authoritative_cells"] = {f: families[f]["discriminators"]["W"]["authoritative_cells"] for f in FAMILIES}
    v_family = per_family("V")
    totals = {"completed_solves": 0, "failed_solves": 0, "violations": 0}
    max_ratio = 0.0
    for family in FAMILIES:
        block = families[family]["discriminators"]["V"]
        for key in totals:
            totals[key] += block["totals"][key]
        max_ratio = max(max_ratio, block["max_residual_over_threshold"])
    v_verdict = verdict_of(list(v_family.values()))
    if all_violations["P1"] > 0:
        v_verdict = "FAIL"
    discriminators["V"] = {"families": v_family, "totals_P1": totals, "max_residual_over_threshold": max_ratio,
                           "violations": all_violations, "verdict": v_verdict}
    for name, filename in (("G", "GOLDEN_DIFF.json"), ("R", "R_SUITES.json")):
        path = os.path.join(NODE, filename)
        if os.path.exists(path):
            with open(path, encoding="utf-8") as handle:
                data = json.load(handle)
            discriminators[name] = {"verdict": data.get("verdict"), "file": filename, "summary": data.get("summary")}
        else:
            discriminators[name] = {"verdict": "NOT_RUN", "file": None}
    results["discriminators"] = discriminators

    seven = {name: discriminators[name]["verdict"] for name in ("D0", "V", "B", "A", "S", "E", "W")}
    if all_violations["P1"] > 0:
        decision, reason = "KILL", f"{all_violations['P1']} violations of V in solves of P1"
    elif all(value == "PASS" for value in seven.values()):
        decision, reason = "SWITCH", "D0, V, B, A, S, E and W hold"
    elif seven["V"] == "PASS" and any(value == "FAIL" for value in seven.values()):
        decision, reason = "KEEP", f"V holds and {[k for k, v in seven.items() if v == 'FAIL']} fail"
    else:
        decision, reason = "NOT_DECIDABLE", f"no FAIL with V intact, not decidable: {[k for k, v in seven.items() if v != 'PASS']}"
    results["decision_before_step_6"] = {"verdict": decision, "reason": reason, "discriminators": seven,
                                         "frozen_rules": {"SWITCH": "D0, V, B, A, S, E and W hold (then G and R must hold, else HOLD)",
                                                          "KEEP": "V holds and one of D0, B, A, S, E or W does not",
                                                          "KILL": "V is violated in any solve of P1"}}
    final = decision
    if decision == "SWITCH":
        g, r = discriminators["G"]["verdict"], discriminators["R"]["verdict"]
        if g == "NOT_RUN" or r == "NOT_RUN":
            final = "SWITCH_PENDING_G_R"
        elif g == "PASS" and r == "PASS" and discriminators["D0"]["verdict"] == "PASS":
            final = "SWITCH"
        else:
            final = "HOLD"
    results["verdict"] = {"verdict": final, "G": discriminators["G"]["verdict"], "R": discriminators["R"]["verdict"]}
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("what", choices=["recording", "results"])
    args = parser.parse_args()
    if args.what == "recording":
        recording = build_recording()
        write_json(os.path.join(NODE, "RECORDING_A.json"), recording)
        print(json.dumps(recording["summary"]))
        print("checks:", recording["checks"][:20])
    else:
        results = build_results()
        write_json(os.path.join(NODE, "RESULTS.json"), results)
        print(json.dumps({name: block.get("verdict") for name, block in results["discriminators"].items()}))
        print(json.dumps(results["decision_before_step_6"]["verdict"]), json.dumps(results["verdict"]))
        print("checks:", results["checks"][:20])


if __name__ == "__main__":
    sys.exit(main())
