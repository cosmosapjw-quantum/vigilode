#!/usr/bin/env python3
"""Loop node V17: builds RESULTS.json from the raw logs only.

Inputs: PREDECLARED.json (families, tolerances, rules), reference/*.json
(outputs of make_reference.py), logs/setup, logs/runs, logs/timing,
logs/exploratory, logs/v06_regression and, for the regression comparison,
../shared_krylov/logs. No number is typed in here: thresholds are the frozen
ones of PREREGISTRATION.md addendum A2 as restated in PREDECLARED.json.

usage: python3 -I parse_results.py [--out RESULTS.json]
"""
import argparse
import glob
import hashlib
import json
import math
import os
import statistics
import struct
import sys
import time

NODE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(NODE, "logs")
V06_LOGS = os.path.join(NODE, "..", "shared_krylov", "logs")

# Frozen thresholds (PREREGISTRATION.md, addendum A2, row V17).
CADENCE_INSTRUCTIONS_MAX = 0.67
CADENCE_WALL_MIN = 1.3
POLICY_INSTRUCTIONS_MAX = 0.5
POLICY_ATTEMPTS_MAX = 1.10
VALIDITY_ATTEMPTS_B = 1.05
KILL_WALL_MIN = 1.10
# Timing quality rule of the loop.
LOAD_MAX = 8.0
SPREAD_MAX = 0.10
WARM_UP_ROUNDS = 2
REPETITIONS = 7
GRID_POINTS = 201
ARMS = ("A", "B", "C")


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def unhex(text):
    return [struct.unpack(">d", bytes.fromhex(text[i:i + 16]))[0] for i in range(0, len(text), 16)]


def read_log(path):
    """Tagged JSON lines of one log: {tag: [records]}, plus header and footer."""
    out = {"begin": None, "end": None, "tags": {}, "load_before": [], "load_after": [], "path": os.path.relpath(path, NODE)}
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("# V17_LOG_BEGIN "):
                out["begin"] = json.loads(line[len("# V17_LOG_BEGIN "):])
            elif line.startswith("# V17_LOG_END "):
                out["end"] = json.loads(line[len("# V17_LOG_END "):])
            elif line.startswith("V17_LOAD_BEFORE "):
                out["load_before"].append(line[len("V17_LOAD_BEFORE "):])
            elif line.startswith("V17_LOAD_AFTER "):
                out["load_after"].append(line[len("V17_LOAD_AFTER "):])
            elif line.startswith(("V17_", "V06_")) and " " in line:
                tag, body = line.split(" ", 1)
                if tag.endswith("_JSON"):
                    try:
                        out["tags"].setdefault(tag, []).append(json.loads(body))
                    except ValueError:
                        out["tags"].setdefault(tag + "_UNPARSED", []).append(body[:200])
    return out


def read_perf(path):
    """instructions:u of a perf stat -x, file, or None."""
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            fields = line.strip().split(",")
            if len(fields) >= 3 and fields[2].startswith("instructions"):
                try:
                    return int(fields[0])
                except ValueError:
                    return None
    return None


def first(log, tag):
    if log is None:
        return None
    records = log["tags"].get(tag)
    return records[0] if records else None


def clean(value):
    """Strict JSON: non-finite floats become null."""
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, dict):
        return {key: clean(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [clean(item) for item in value]
    return value


def ratio(a, b):
    if a is None or b is None or b == 0:
        return None
    return a / b


def error_metrics(state, reference):
    scaled = max(abs(y - r) / (1.0 + abs(r)) for y, r in zip(state, reference))
    absolute = max(abs(y - r) for y, r in zip(state, reference))
    rms = math.sqrt(sum((y - r) ** 2 for y, r in zip(state, reference)) / len(reference))
    return {"E": scaled, "max_abs": absolute, "rms": rms}


# ---------------------------------------------------------------- matched error

def curve_value(points, level, take):
    """Work at error `level` on the polyline `points` [(E, W)] (tolerance
    order); `take` is min (arm A) or max (candidate) over the segments that
    contain the level. None outside the curve."""
    values = []
    for (e0, w0), (e1, w1) in zip(points, points[1:]):
        lo, hi = min(e0, e1), max(e0, e1)
        if not lo <= level <= hi:
            continue
        if e0 == e1:
            values.extend([w0, w1])
            continue
        s = (math.log(level) - math.log(e0)) / (math.log(e1) - math.log(e0))
        values.append(math.exp(math.log(w0) + s * (math.log(w1) - math.log(w0))))
    if len(points) == 1 and points[0][0] == level:
        values.append(points[0][1])
    return take(values) if values else None


def matched(points_a, points_x, floor, completed_a, completed_x, needed):
    """The declared matched-error statistic of arm X against arm A."""
    out = {"points_A": points_a, "points_X": points_x, "status": "OK"}
    if completed_a < needed or completed_x < needed:
        out["status"] = "NOT_DECIDABLE"
        out["reason"] = f"fewer than {needed} completed tolerances (A {completed_a}, X {completed_x})"
        return out
    if any(e <= 0 or w <= 0 for e, w in points_a + points_x):
        out["status"] = "NOT_DECIDABLE"
        out["reason"] = "a point has a nonpositive error or work"
        return out
    lo = max(min(e for e, _ in points_a), min(e for e, _ in points_x), floor)
    hi = min(max(e for e, _ in points_a), max(e for e, _ in points_x))
    out["range"] = [lo, hi]
    out["floor_10_u_ref"] = floor
    if not lo < hi:
        out["status"] = "NOT_DECIDABLE"
        out["reason"] = "the error ranges of the two arms do not overlap above 10 u_ref"
        return out
    logs = []
    for k in range(GRID_POINTS):
        level = math.exp(math.log(lo) + (math.log(hi) - math.log(lo)) * k / (GRID_POINTS - 1))
        level = min(max(level, lo), hi)
        wa = curve_value(points_a, level, min)
        wx = curve_value(points_x, level, max)
        if wa is None or wx is None:
            continue
        logs.append(math.log(wx / wa))
    if len(logs) < GRID_POINTS:
        out["grid_points_evaluated"] = len(logs)
    if not logs:
        out["status"] = "NOT_DECIDABLE"
        out["reason"] = "no grid point could be evaluated"
        return out
    out["geometric_mean_ratio"] = math.exp(sum(logs) / len(logs))
    out["min_ratio"] = math.exp(min(logs))
    out["max_ratio"] = math.exp(max(logs))
    at_a = []
    for e, w in points_a:
        if lo <= e <= hi:
            wx = curve_value(points_x, e, max)
            if wx is not None:
                at_a.append({"E": e, "ratio": wx / w})
    out["ratio_at_points_of_A"] = at_a
    return out


# ---------------------------------------------------------------- collection

def collect_run(directory, name):
    """One RUN log with its perf file."""
    log = read_log(os.path.join(directory, name + ".log"))
    if log is None:
        return None
    record = first(log, "V17_RUN_JSON")
    state = first(log, "V17_STATE_JSON")
    out = {
        "log": log["path"], "exit_code": (log["end"] or {}).get("exit_code"),
        "timed_out": (log["end"] or {}).get("timed_out"), "record": record,
        "state_hex": state["state_hex"] if state else None,
        "instructions_raw": read_perf(os.path.join(directory, name + ".perf")),
        "binary_sha256": (log["begin"] or {}).get("binary_sha256"),
        "git_head": (log["begin"] or {}).get("git_head"),
        "git_dirty_tracked": (log["begin"] or {}).get("git_dirty_tracked"),
    }
    out["completed"] = bool(record and record.get("completed") and record.get("success") and out["exit_code"] == 0)
    return out


def summarise_run(run, reference_state, setup_median, rtol):
    """The numbers of one run that the tables use."""
    if run is None:
        return {"present": False}
    record = run["record"] or {}
    counters = record.get("counters") or {}
    out = {
        "present": True, "log": run["log"], "completed": run["completed"],
        "success": record.get("success"), "error": record.get("error"),
        "exit_code": run["exit_code"], "timed_out": run["timed_out"],
        "attempts": record.get("attempts"), "accepted": record.get("accepted"), "rejected": record.get("rejected"),
        "jvp_calls": record.get("jvp_calls"), "rhs_calls": counters.get("rhs_calls"),
        "linear_solves": counters.get("linear_solves"), "linear_iterations": counters.get("linear_iterations"),
        "linear_solve_failures": counters.get("linear_solve_failures"),
        "local_error_failures": counters.get("local_error_failures"),
        "nonfinite_step_failures": counters.get("nonfinite_step_failures"),
        "orthogonalization_inner_products": counters.get("orthogonalization_inner_products"),
        "orthogonalization_vector_updates": counters.get("orthogonalization_vector_updates"),
        "diagnostic_matvecs": counters.get("diagnostic_matvecs"),
        "state_sha256": record.get("state_sha256"),
        "wall_seconds_diagnostic_only": record.get("wall_seconds_diagnostic_only"),
        "instructions_raw": run["instructions_raw"],
        "restart": record.get("restart"), "precond": record.get("precond"), "kappa": record.get("kappa"),
        "t_span": record.get("t_span"),
    }
    if run["instructions_raw"] is not None and setup_median is not None:
        out["instructions_net"] = run["instructions_raw"] - setup_median
    client = record.get("client")
    if client:
        solves = client.get("solves") or 0
        out["client"] = client
        out["krylov_vectors_per_solve"] = ratio(client.get("krylov_vectors"), solves)
        out["max_solve_vectors"] = client.get("max_solve_vectors")
        out["policy_bound_share"] = ratio(client.get("policy_bound_solves"), solves)
    if record.get("solve_log"):
        log = record["solve_log"]
        out["solve_log"] = log
        out["krylov_vectors_per_solve"] = ratio(log.get("iterations"), log.get("solves"))
        out["max_solve_vectors"] = log.get("max_iterations")
    elif not client and counters.get("linear_solves"):
        out["krylov_vectors_per_solve"] = ratio(counters.get("linear_iterations"), counters.get("linear_solves"))
    if record.get("residual_audit"):
        out["residual_audit"] = record["residual_audit"]
    if run["state_hex"] and reference_state is not None and run["completed"]:
        state = unhex(run["state_hex"])
        if len(state) == len(reference_state):
            metrics = error_metrics(state, reference_state)
            out.update(metrics)
            out["E_over_rtol"] = metrics["E"] / rtol
    return out


def same_run(plain, audited):
    """Audit-on and audit-off runs agree in every counter and state bit."""
    if plain is None or audited is None or not plain["record"] or not audited["record"]:
        return None
    a, b = plain["record"], audited["record"]
    keys = ("success", "attempts", "accepted", "rejected", "t_end", "counters", "state_sha256", "client")
    differing = [key for key in keys if a.get(key) != b.get(key)]
    if plain["state_hex"] != audited["state_hex"]:
        differing.append("state_hex")
    return {"identical": not differing, "differing": differing}


def collect_timing(family, rtol, plain_states):
    """Every timing measurement of a cell, in the order of the tries."""
    directory = os.path.join(LOGS, "timing")
    tries = []
    for attempt in (1, 2, 3):
        single = os.path.join(directory, f"{family}__{rtol}__try{attempt}.log")
        split = sorted(glob.glob(os.path.join(directory, f"{family}__{rtol}__try{attempt}__round*.log")))
        paths = [single] if os.path.exists(single) else split
        if not paths:
            continue
        logs = [read_log(path) for path in paths]
        runs, loads_before, loads_after, metas, complete = [], [], [], [], True
        for log in logs:
            runs.extend(log["tags"].get("V17_TIMING_JSON", []))
            metas.extend(log["tags"].get("V17_TIMING_META_JSON", []))
            loads_before.extend(log["load_before"])
            loads_after.extend(log["load_after"])
            if not log["end"] or log["end"].get("exit_code") != 0:
                complete = False
        entry = {
            "try": attempt, "logs": [log["path"] for log in logs], "invocations": len(logs),
            "split_long_cell": len(logs) > 1, "process_complete": complete,
            "load_before": loads_before, "load_after": loads_after,
            "loadavg_in_process": [[meta.get("loadavg_start"), meta.get("loadavg_end")] for meta in metas],
            "arms": {},
        }
        load_values = [float(text.split()[0]) for text in loads_before + loads_after if text.split()]
        entry["load_1min_max"] = max(load_values) if load_values else None
        entry["load_1min_before_first"] = float(loads_before[0].split()[0]) if loads_before else None
        entry["load_1min_after_last"] = float(loads_after[-1].split()[0]) if loads_after else None
        reasons = []
        if not complete:
            reasons.append("an invocation did not exit with 0")
        if len(loads_before) != len(logs) or len(loads_after) != len(logs):
            reasons.append("a load reading is missing")
        elif entry["load_1min_max"] > LOAD_MAX:
            reasons.append(f"1-minute load average {entry['load_1min_max']} above {LOAD_MAX}")
        for arm in ARMS:
            mine = sorted((run for run in runs if run.get("arm") == arm), key=lambda run: run.get("round", 0))
            warm = [run["wall_seconds"] for run in mine if run.get("round", 0) < WARM_UP_ROUNDS]
            reps = [run for run in mine if run.get("round", 0) >= WARM_UP_ROUNDS]
            walls = [run["wall_seconds"] for run in reps]
            states_ok = all(run.get("completed") and run.get("success")
                            and run.get("state_sha256") == plain_states.get(arm) for run in mine) and bool(mine)
            arm_entry = {"warm_up_walls": warm, "walls": walls, "repetitions": len(walls), "states_equal_instruction_run": states_ok}
            if walls:
                median = statistics.median(walls)
                arm_entry.update({"median": median, "min": min(walls), "max": max(walls),
                                  "spread": (max(walls) - min(walls)) / median if median > 0 else None})
            # Exploratory: on-CPU time of the solver thread (load-robust).
            cpus = [run.get("thread_cpu_seconds") for run in reps]
            if cpus and all(value is not None for value in cpus):
                median_cpu = statistics.median(cpus)
                arm_entry.update({"thread_cpu_seconds": cpus, "median_thread_cpu": median_cpu,
                                  "spread_thread_cpu": (max(cpus) - min(cpus)) / median_cpu if median_cpu > 0 else None})
            entry["arms"][arm] = arm_entry
            if len(walls) != REPETITIONS or len(warm) != WARM_UP_ROUNDS:
                reasons.append(f"arm {arm}: {len(warm)} warm-ups and {len(walls)} repetitions")
            elif arm_entry["spread"] is None or arm_entry["spread"] > SPREAD_MAX:
                reasons.append(f"arm {arm}: spread {arm_entry['spread']:.3f} above {SPREAD_MAX}")
            if not states_ok:
                reasons.append(f"arm {arm}: a timed run did not end in the state of the instruction run")
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
        out["median_wall"] = {arm: used["arms"][arm].get("median") for arm in ARMS}
        out["spread"] = {arm: used["arms"][arm].get("spread") for arm in ARMS}
        out["median_thread_cpu"] = {arm: used["arms"][arm].get("median_thread_cpu") for arm in ARMS}
        out["spread_thread_cpu"] = {arm: used["arms"][arm].get("spread_thread_cpu") for arm in ARMS}
    return out


# ---------------------------------------------------------------- main

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=os.path.join(NODE, "RESULTS.json"))
    args = parser.parse_args()

    predeclared_path = os.path.join(NODE, "PREDECLARED.json")
    with open(predeclared_path, encoding="utf-8") as handle:
        predeclared = json.load(handle)
    families = predeclared["families_final"]["list"]
    rtols = predeclared["tolerances"]["rtol"]
    rtol_labels = [f"1e-{round(-math.log10(r))}" for r in rtols]
    needed = 4

    results = {
        "schema": "vigilode.loop-l1.v17.results.v1",
        "node": "V17_MATRIX_FREE_STAGE_SOLVE",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "predeclared_sha256": sha256_file(predeclared_path),
        "frozen_thresholds": {
            "cadence_instructions_max": CADENCE_INSTRUCTIONS_MAX, "cadence_wall_min": CADENCE_WALL_MIN,
            "policy_instructions_max": POLICY_INSTRUCTIONS_MAX, "policy_attempts_max": POLICY_ATTEMPTS_MAX,
            "validity_attempts_B": VALIDITY_ATTEMPTS_B, "kill_wall_min": KILL_WALL_MIN,
            "timing_load_max": LOAD_MAX, "timing_spread_max": SPREAD_MAX,
        },
        "families": {}, "checks": [], "binaries": {}, "git_heads": {},
    }
    problems = results["checks"]

    def note_identity(run):
        if run is None:
            return
        if run.get("binary_sha256"):
            results["binaries"][run["binary_sha256"]] = results["binaries"].get(run["binary_sha256"], 0) + 1
        if run.get("git_head"):
            key = f"{run['git_head']} dirty_tracked={run.get('git_dirty_tracked')}"
            results["git_heads"][key] = results["git_heads"].get(key, 0) + 1

    setup_directory = os.path.join(LOGS, "setup")
    runs_directory = os.path.join(LOGS, "runs")
    exploratory_directory = os.path.join(LOGS, "exploratory")

    for family in families:
        fid = family["id"]
        entry = {"n": family["n"], "t_span": family["t_span"], "new": family.get("new"),
                 "strongly_stiff": bool(family.get("strongly_stiff")), "cells": {}}
        results["families"][fid] = entry

        # Reference.
        reference_state, u_ref = None, None
        reference_path = os.path.join(NODE, "reference", fid + ".json")
        if os.path.exists(reference_path):
            with open(reference_path, encoding="utf-8") as handle:
                reference = json.load(handle)
            reference_state = unhex(reference["state_hex"])
            u_ref = reference["uncertainty_max_scaled_1_plus_abs"]
            entry["reference"] = {
                "file": os.path.relpath(reference_path, NODE), "sha256": sha256_file(reference_path),
                "t_span": reference["t_span"], "uncertainty": u_ref,
                "reference": reference["reference"],
                "cross_checks": reference["cross_checks"],
                "span_is_declared": reference["t_span"] == family["t_span"], "n_is_declared": reference["n"] == family["n"],
            }
            if not entry["reference"]["span_is_declared"] or not entry["reference"]["n_is_declared"]:
                problems.append(f"{fid}: the reference is not for the declared span or size")
            rhs0 = first(read_log(os.path.join(setup_directory, f"{fid}__RHS0.log")), "V17_RHS0_JSON")
            if rhs0:
                y0_rust, f0_rust = unhex(rhs0["y0_hex"]), unhex(rhs0["f0_hex"])
                y0_ref, f0_ref = unhex(reference["y0_hex"]), unhex(reference["f0_hex"])
                scale = max(abs(v) for v in f0_ref) or 1.0
                entry["reference"]["rhs_consistency"] = {
                    "y0_max_abs_difference": max(abs(a - b) for a, b in zip(y0_rust, y0_ref)),
                    "f0_max_relative_difference": max(abs(a - b) for a, b in zip(f0_rust, f0_ref)) / scale,
                }
                entry["reference"]["rhs_consistent"] = (
                    entry["reference"]["rhs_consistency"]["f0_max_relative_difference"] <= 1e-12
                    and entry["reference"]["rhs_consistency"]["y0_max_abs_difference"] <= 1e-12)
                if not entry["reference"]["rhs_consistent"]:
                    problems.append(f"{fid}: numpy and Rust right-hand sides differ at the start state")
        else:
            problems.append(f"{fid}: no reference file")

        # Set-up instructions and calibration.
        setups = [read_perf(os.path.join(setup_directory, f"{fid}__SETUP__r{k}.perf")) for k in (1, 2, 3)]
        setups = [value for value in setups if value is not None]
        setup_median = statistics.median(setups) if setups else None
        entry["setup_instructions"] = {"runs": setups, "median": setup_median}
        calibration = {}
        for kind in ("JVP", "RHS"):
            record = first(read_log(os.path.join(setup_directory, f"{fid}__CALIB_{kind}.log")), "V17_CALIB_JSON")
            raw = read_perf(os.path.join(setup_directory, f"{fid}__CALIB_{kind}.perf"))
            if record and raw is not None and setup_median is not None:
                calibration[kind.lower()] = {"calls": record["calls"], "instructions_raw": raw,
                                             "instructions_per_call": (raw - setup_median) / record["calls"]}
        entry["calibration"] = calibration

        # Declared cells.
        for rtol, label in zip(rtols, rtol_labels):
            cell = {"rtol": rtol, "arms": {}, "audit_on": {}, "same_run": {}}
            plain_states = {}
            for arm in ("A", "B", "C", "B0"):
                plain = collect_run(runs_directory, f"{fid}__{label}__{arm}__audit0")
                note_identity(plain)
                cell["arms"][arm] = summarise_run(plain, reference_state, setup_median, rtol)
                if plain and plain["record"]:
                    plain_states[arm] = plain["record"].get("state_sha256")
                    if plain["record"].get("t_span") != family["t_span"]:
                        problems.append(f"{fid} {label} {arm}: the run is not on the declared span")
                        cell["arms"][arm]["completed"] = False
                    if plain["record"].get("n") != family["n"]:
                        problems.append(f"{fid} {label} {arm}: the run is not at the declared size")
                        cell["arms"][arm]["completed"] = False
                if arm == "B0":
                    continue
                audited = collect_run(runs_directory, f"{fid}__{label}__{arm}__audit1")
                note_identity(audited)
                cell["audit_on"][arm] = summarise_run(audited, reference_state, None, rtol)
                cell["same_run"][arm] = same_run(plain, audited)
                if cell["same_run"][arm] is not None and not cell["same_run"][arm]["identical"]:
                    problems.append(f"{fid} {label} {arm}: audit-on and audit-off runs differ in {cell['same_run'][arm]['differing']}")
            a = cell["arms"]["A"]
            equal = {}
            for arm in ("B", "C", "B0"):
                x = cell["arms"][arm]
                if a.get("completed") and x.get("completed"):
                    equal[arm] = {
                        "attempts": ratio(x.get("attempts"), a.get("attempts")),
                        "jvp": ratio(x.get("jvp_calls"), a.get("jvp_calls")),
                        "instructions": ratio(x.get("instructions_net"), a.get("instructions_net")),
                        "error": ratio(x.get("E"), a.get("E")),
                        "orthogonalization_inner_products": ratio(x.get("orthogonalization_inner_products"),
                                                                  a.get("orthogonalization_inner_products")),
                        "wall_diagnostic_single_run": ratio(a.get("wall_seconds_diagnostic_only"),
                                                            x.get("wall_seconds_diagnostic_only")),
                    }
            cell["equal_tolerance_ratio_to_A"] = equal
            cell["timing"] = collect_timing(fid, label, plain_states)
            if cell["timing"].get("median_wall"):
                walls = cell["timing"]["median_wall"]
                cell["timing"]["speedup_equal_tolerance"] = {arm: ratio(walls.get("A"), walls.get(arm)) for arm in ("B", "C")}
                cpus = cell["timing"].get("median_thread_cpu") or {}
                cell["timing"]["thread_cpu_speedup_equal_tolerance"] = {arm: ratio(cpus.get("A"), cpus.get(arm)) for arm in ("B", "C")}
            entry["cells"][label] = cell

        # Matched-error statistics.
        floor = 10.0 * u_ref if u_ref is not None else float("inf")
        entry["matched_error"] = {}
        wall_authoritative = all(entry["cells"][label]["timing"]["label"] == "authoritative" for label in rtol_labels)
        wall_present = all(entry["cells"][label]["timing"].get("median_wall") for label in rtol_labels)
        entry["wall_label"] = "authoritative" if wall_authoritative else ("diagnostic_only" if wall_present else "NOT_RUN")

        def points(arm, measure):
            out, completed = [], 0
            for label in rtol_labels:
                cell = entry["cells"][label]
                run = cell["arms"][arm]
                if not run.get("completed") or run.get("E") is None:
                    continue
                if measure == "wall":
                    work = (cell["timing"].get("median_wall") or {}).get(arm)
                elif measure == "thread_cpu":
                    work = (cell["timing"].get("median_thread_cpu") or {}).get(arm)
                else:
                    work = run.get(measure)
                if work is None:
                    continue
                completed += 1
                out.append((run["E"], float(work)))
            return out, completed

        for arm in ("B", "C"):
            block = {}
            for measure in ("instructions_net", "attempts", "jvp_calls", "wall", "thread_cpu"):
                points_a, completed_a = points("A", measure)
                points_x, completed_x = points(arm, measure)
                block[measure] = matched(points_a, points_x, floor, completed_a, completed_x, needed)
            if block["wall"].get("geometric_mean_ratio"):
                block["wall"]["speedup"] = 1.0 / block["wall"]["geometric_mean_ratio"]
                block["wall"]["speedup_min"] = 1.0 / block["wall"]["max_ratio"]
                block["wall"]["speedup_max"] = 1.0 / block["wall"]["min_ratio"]
            block["wall"]["label"] = entry["wall_label"]
            if block["thread_cpu"].get("geometric_mean_ratio"):
                block["thread_cpu"]["speedup"] = 1.0 / block["thread_cpu"]["geometric_mean_ratio"]
                block["thread_cpu"]["speedup_min"] = 1.0 / block["thread_cpu"]["max_ratio"]
                block["thread_cpu"]["speedup_max"] = 1.0 / block["thread_cpu"]["min_ratio"]
            block["thread_cpu"]["label"] = "exploratory, never used for a verdict"
            entry["matched_error"][arm] = block

        # Validity.
        validity = {}
        for arm in ARMS:
            checks = failures = 0
            max_ratio = 0.0
            covered = True
            missing = []
            for label in rtol_labels:
                cell = entry["cells"][label]
                audited = cell["audit_on"].get(arm) or {}
                audit = audited.get("residual_audit")
                if not audit:
                    missing.append(label)
                    continue
                checks += audit["checks"]
                failures += audit["failures"]
                max_ratio = max(max_ratio, audit.get("max_ratio") or 0.0)
                if arm == "A":
                    solves = audited.get("linear_solves")
                else:
                    client = audited.get("client") or {}
                    solves = (client.get("solves") or 0) - (client.get("failed_solves") or 0)
                if solves != audit["checks"]:
                    covered = False
                same = cell["same_run"].get(arm)
                if not same or not same["identical"]:
                    covered = False
            validity[arm] = {"checks": checks, "failures": failures, "max_residual_over_threshold": max_ratio,
                             "every_solve_checked_and_same_run": covered and not missing,
                             "cells_without_audit": missing,
                             "residual_check_passes": failures == 0 and covered and not missing and checks > 0}
        attempts_b = {label: (entry["cells"][label]["equal_tolerance_ratio_to_A"].get("B") or {}).get("attempts") for label in rtol_labels}
        attempts_c = {label: (entry["cells"][label]["equal_tolerance_ratio_to_A"].get("C") or {}).get("attempts") for label in rtol_labels}
        validity["attempts_B_over_A"] = attempts_b
        validity["attempts_C_over_A"] = attempts_c
        validity["attempts_B_within_1_05"] = all(
            value is not None and 1.0 / VALIDITY_ATTEMPTS_B <= value <= VALIDITY_ATTEMPTS_B for value in attempts_b.values())
        validity["attempts_C_at_most_1_10"] = all(
            value is not None and value <= POLICY_ATTEMPTS_MAX for value in attempts_c.values())
        validity["B"]["valid"] = validity["A"]["residual_check_passes"] and validity["B"]["residual_check_passes"] and validity["attempts_B_within_1_05"]
        validity["C"]["valid"] = validity["A"]["residual_check_passes"] and validity["C"]["residual_check_passes"]
        entry["validity"] = validity

        # Family verdict parts.
        me_b, me_c = entry["matched_error"]["B"], entry["matched_error"]["C"]
        cadence = {"validity": validity["B"]["valid"]}
        r = me_b["instructions_net"].get("geometric_mean_ratio")
        cadence["instructions_ratio"] = r
        cadence["instructions_part"] = "NOT_DECIDABLE" if r is None else ("PASS" if r <= CADENCE_INSTRUCTIONS_MAX else "FAIL")
        s = me_b["wall"].get("speedup")
        cadence["wall_speedup"] = s
        cadence["wall_label"] = entry["wall_label"]
        if s is None or entry["wall_label"] != "authoritative":
            cadence["wall_part"] = "NOT_DECIDABLE"
        else:
            cadence["wall_part"] = "PASS" if s >= CADENCE_WALL_MIN else "FAIL"
        parts = [("PASS" if cadence["validity"] else "FAIL"), cadence["instructions_part"], cadence["wall_part"]]
        cadence["family"] = "FAIL" if "FAIL" in parts else ("NOT_DECIDABLE" if "NOT_DECIDABLE" in parts else "PASS")
        if r is not None:
            worst = me_b["instructions_net"]["max_ratio"]
            cadence["instructions_part_at_worst_grid_point"] = "PASS" if worst <= CADENCE_INSTRUCTIONS_MAX else "FAIL"
        if s is not None:
            cadence["wall_part_at_worst_grid_point_ignoring_label"] = "PASS" if me_b["wall"]["speedup_min"] >= CADENCE_WALL_MIN else "FAIL"
        entry["cadence"] = cadence

        policy = {"validity": validity["C"]["valid"]}
        r = me_c["instructions_net"].get("geometric_mean_ratio")
        policy["instructions_ratio"] = r
        policy["instructions_part"] = "NOT_DECIDABLE" if r is None else ("PASS" if r <= POLICY_INSTRUCTIONS_MAX else "FAIL")
        ra = me_c["attempts"].get("geometric_mean_ratio")
        policy["attempts_ratio_matched_error"] = ra
        policy["attempts_equal_tolerance_max"] = max((v for v in attempts_c.values() if v is not None), default=None)
        if ra is None or any(value is None for value in attempts_c.values()):
            policy["attempts_part"] = "NOT_DECIDABLE"
        else:
            policy["attempts_part"] = "PASS" if (ra <= POLICY_ATTEMPTS_MAX and validity["attempts_C_at_most_1_10"]) else "FAIL"
        parts = [("PASS" if policy["validity"] else "FAIL"), policy["instructions_part"], policy["attempts_part"]]
        policy["family"] = "FAIL" if "FAIL" in parts else ("NOT_DECIDABLE" if "NOT_DECIDABLE" in parts else "PASS")
        if r is not None:
            policy["instructions_part_at_worst_grid_point"] = "PASS" if me_c["instructions_net"]["max_ratio"] <= POLICY_INSTRUCTIONS_MAX else "FAIL"
        entry["policy"] = policy

        # Exploratory: where the gain comes from.
        decomposition = {}
        per_jvp = (calibration.get("jvp") or {}).get("instructions_per_call")
        per_rhs = (calibration.get("rhs") or {}).get("instructions_per_call")
        for label in rtol_labels:
            row = {}
            for arm in ("A", "B", "C", "B0"):
                run = entry["cells"][label]["arms"][arm]
                net = run.get("instructions_net")
                if net is None or per_jvp is None or per_rhs is None or not run.get("completed"):
                    continue
                jvp_part = run["jvp_calls"] * per_jvp
                rhs_part = run["rhs_calls"] * per_rhs
                orth = (run.get("orthogonalization_inner_products") or 0) + (run.get("orthogonalization_vector_updates") or 0)
                row[arm] = {
                    "instructions_net": net, "jvp_part": jvp_part, "rhs_part": rhs_part,
                    "remainder": net - jvp_part - rhs_part,
                    "jvp_share": jvp_part / net, "rhs_share": rhs_part / net,
                    "remainder_share": (net - jvp_part - rhs_part) / net,
                    "orthogonalization_vector_operations": orth,
                    "remainder_per_orthogonalization_element": ratio(net - jvp_part - rhs_part, orth * family["n"]),
                }
            for arm in ("B", "C"):
                if "A" in row and arm in row:
                    saved = row["A"]["instructions_net"] - row[arm]["instructions_net"]
                    if saved > 0:
                        row[f"saving_A_to_{arm}"] = {
                            "instructions": saved,
                            "from_jvp": (row["A"]["jvp_part"] - row[arm]["jvp_part"]) / saved,
                            "from_rhs": (row["A"]["rhs_part"] - row[arm]["rhs_part"]) / saved,
                            "from_remainder": (row["A"]["remainder"] - row[arm]["remainder"]) / saved,
                        }
            decomposition[label] = row
        entry["exploratory_decomposition"] = decomposition

        # Exploratory runs.
        exploratory = {"restart": {}, "jacobi": {}, "kappa": {}}
        for arm in ("A", "B"):
            for restart in ("10", "20", "80"):
                run = collect_run(exploratory_directory, f"{fid}__1e-5__{arm}__restart{restart}__audit0")
                note_identity(run)
                if run:
                    exploratory["restart"][f"{arm}_restart{restart}"] = summarise_run(run, reference_state, setup_median, 1e-5)
            if "1e-5" in entry["cells"]:
                exploratory["restart"][f"{arm}_restart40_declared"] = entry["cells"]["1e-5"]["arms"][arm]
        for audit in ("0", "1"):
            run = collect_run(exploratory_directory, f"{fid}__1e-5__B__jacobi__audit{audit}")
            note_identity(run)
            if run:
                exploratory["jacobi"][f"audit{audit}"] = summarise_run(run, reference_state, setup_median if audit == "0" else None, 1e-5)
        for kappa in ("1e-4", "1e-2"):
            for rtol, label in zip(rtols, rtol_labels):
                run = collect_run(exploratory_directory, f"{fid}__{label}__C__kappa{kappa}__audit0")
                note_identity(run)
                if run:
                    exploratory["kappa"].setdefault(kappa, {})[label] = summarise_run(run, reference_state, setup_median, rtol)
        entry["exploratory"] = exploratory

    # Overall verdicts.
    def count(key):
        values = [results["families"][family["id"]][key]["family"] for family in families]
        passed, failed = values.count("PASS"), values.count("FAIL")
        verdict = "PASS" if passed >= 3 else ("FAIL" if failed >= 2 else "NOT_DECIDABLE")
        return {"families": {family["id"]: results["families"][family["id"]][key]["family"] for family in families},
                "pass": passed, "fail": failed, "open": len(values) - passed - failed, "verdict": verdict}

    def count_part(key, part):
        values = [results["families"][family["id"]][key][part] for family in families]
        return {"pass": values.count("PASS"), "fail": values.count("FAIL"), "open": values.count("NOT_DECIDABLE")}

    verdicts = {"cadence_B": count("cadence"), "policy_C": count("policy")}
    verdicts["cadence_B"]["instructions_part"] = count_part("cadence", "instructions_part")
    verdicts["cadence_B"]["wall_part"] = count_part("cadence", "wall_part")
    verdicts["policy_C"]["instructions_part"] = count_part("policy", "instructions_part")
    verdicts["policy_C"]["attempts_part"] = count_part("policy", "attempts_part")
    total = {arm: {"checks": 0, "failures": 0, "max_residual_over_threshold": 0.0} for arm in ARMS}
    for family in families:
        validity = results["families"][family["id"]]["validity"]
        for arm in ARMS:
            total[arm]["checks"] += validity[arm]["checks"]
            total[arm]["failures"] += validity[arm]["failures"]
            total[arm]["max_residual_over_threshold"] = max(total[arm]["max_residual_over_threshold"],
                                                           validity[arm]["max_residual_over_threshold"])
    verdicts["residual_checks"] = total
    verdicts["validity"] = {
        "residual_check_passes_in_every_solve": all(
            results["families"][family["id"]]["validity"][arm]["residual_check_passes"] for family in families for arm in ARMS),
        "attempts_B_within_1_05_everywhere": all(
            results["families"][family["id"]]["validity"]["attempts_B_within_1_05"] for family in families),
    }
    speedups = {family["id"]: results["families"][family["id"]]["cadence"]["wall_speedup"] for family in families}
    labels = {family["id"]: results["families"][family["id"]]["wall_label"] for family in families}
    kill = {"residual_check_failures_in_B": total["B"]["failures"], "wall_speedup_B": speedups, "wall_labels": labels}
    authoritative = [fid for fid in speedups if labels[fid] == "authoritative" and speedups[fid] is not None]
    if total["B"]["failures"] > 0:
        kill["verdict"] = "KILL"
        kill["reason"] = "a residual check of arm B failed"
    elif len(authoritative) == len(families) and all(speedups[fid] < KILL_WALL_MIN for fid in authoritative):
        kill["verdict"] = "KILL"
        kill["reason"] = "B below 1.10x wall on every family"
    elif any(speedups[fid] >= KILL_WALL_MIN for fid in authoritative):
        kill["verdict"] = "NOT_TRIGGERED"
        kill["reason"] = "no residual-check failure in B and at least one authoritative family at or above 1.10x"
    else:
        kill["verdict"] = "NOT_DECIDABLE"
        kill["reason"] = "no residual-check failure in B; the wall part cannot be decided without authoritative family walls"
    all_checks_present = all(results["families"][family["id"]]["validity"]["B"]["checks"] > 0 for family in families)
    if not all_checks_present and kill["verdict"] == "NOT_TRIGGERED":
        kill["verdict"] = "NOT_DECIDABLE"
        kill["reason"] = "residual checks of arm B are missing for a family"
    verdicts["kill"] = kill
    results["verdicts"] = verdicts

    # Exploratory: the stiff Brusselator.
    stiff = {}
    stiff_reference = None
    path = os.path.join(NODE, "reference", "bruss1d_stiff600.json")
    if os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            reference = json.load(handle)
        stiff_reference = unhex(reference["state_hex"])
        stiff["reference_uncertainty"] = reference["uncertainty_max_scaled_1_plus_abs"]
    setups = [read_perf(os.path.join(exploratory_directory, f"bruss1d_stiff600__SETUP__r{k}.perf")) for k in (1, 2, 3)]
    setups = [value for value in setups if value is not None]
    stiff_setup = statistics.median(setups) if setups else None
    for arm in ARMS:
        for audit in ("0", "1"):
            run = collect_run(exploratory_directory, f"bruss1d_stiff600__1e-4__{arm}__audit{audit}")
            note_identity(run)
            if run:
                stiff[f"{arm}_audit{audit}"] = summarise_run(run, stiff_reference, stiff_setup if audit == "0" else None, 1e-4)
    results["exploratory_stiff_brusselator"] = stiff

    # Regression: V06 cells with the V17 build against the V06 logs.
    regression = []
    for path in sorted(glob.glob(os.path.join(LOGS, "v06_regression", "*.log"))):
        name = os.path.basename(path)
        mine = read_log(path)
        theirs_path = os.path.join(V06_LOGS, name)
        row = {"name": name, "v06_log_present": os.path.exists(theirs_path)}
        if row["v06_log_present"]:
            def tagged(file_path, tag):
                with open(file_path, encoding="utf-8", errors="replace") as handle:
                    for line in handle:
                        if line.startswith(tag + " "):
                            return json.loads(line.split(" ", 1)[1])
                return None
            new_run, old_run = first(mine, "V06_RUN_JSON"), tagged(theirs_path, "V06_RUN_JSON")
            new_state, old_state = first(mine, "V06_STATE_JSON"), tagged(theirs_path, "V06_STATE_JSON")
            if new_run and old_run:
                # Fields that are not results (wall time, free label), and
                # fields the V06 binary of that day did not print yet.
                ignore = {"wall_seconds_diagnostic_only", "label"}
                common = (set(new_run) & set(old_run)) - ignore
                differing = sorted(key for key in common if new_run[key] != old_run[key])
                row["differing_fields"] = differing
                row["fields_compared"] = len(common)
                row["fields_only_in_v06_log"] = sorted(set(old_run) - set(new_run) - ignore)
                row["fields_only_in_new_log"] = sorted(set(new_run) - set(old_run) - ignore)
                row["state_bits_equal"] = bool(new_state and old_state and new_state.get("state_hex") == old_state.get("state_hex"))
                row["identical"] = not differing and not row["fields_only_in_v06_log"] and row["state_bits_equal"]
                row["attempts"] = new_run.get("attempts")
                row["jvp_calls"] = new_run.get("jvp_calls")
            else:
                row["identical"] = False
                row["differing_fields"] = ["a run record is missing"]
        regression.append(row)
    results["v06_regression"] = {
        "rows": regression, "compared": len([row for row in regression if row.get("v06_log_present")]),
        "identical": len([row for row in regression if row.get("identical")]),
    }

    results["log_inventory"] = {
        phase: len(glob.glob(os.path.join(LOGS, phase, "*.log")))
        for phase in ("sizing", "reference", "setup", "runs", "timing", "exploratory", "v06_regression")
    }
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(clean(results), handle, indent=1, allow_nan=False)
        handle.write("\n")
    print(f"wrote {args.out}")
    print(json.dumps({key: value.get("verdict") for key, value in verdicts.items() if isinstance(value, dict) and "verdict" in value}))
    if problems:
        print(f"{len(problems)} consistency problems:")
        for problem in problems[:20]:
            print("  " + problem)


if __name__ == "__main__":
    sys.exit(main())
