#!/usr/bin/env python3
"""Loop node V24: builds RECORDING_A.json, CHOICE.json and RESULTS.json from
the raw logs only.

Inputs: PREDECLARED.json (frozen), PROTOCOL.json (protocol choices), the
reference files of nodes V17 (deciding families) and V06 (development
problems), logs/<phase>/*.log and *.perf. No number is typed in here: the
thresholds are the frozen ones of PREDECLARED.json, and the matched-error
functions are imported from ../mf_stage_solve/parse_results.py (node V17),
not re-written.

usage: python3 -I parse_results.py recording   (writes RECORDING_A.json)
       python3 -I parse_results.py choice      (writes CHOICE.json)
       python3 -I parse_results.py results     (writes RESULTS.json)
"""
import argparse
import glob
import hashlib
import importlib.util
import json
import math
import os
import statistics
import sys
import time

sys.dont_write_bytecode = True

NODE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(NODE, "logs")
V17 = os.path.join(NODE, "..", "mf_stage_solve")
V06 = os.path.join(NODE, "..", "shared_krylov")

_spec = importlib.util.spec_from_file_location("v17_parse_results", os.path.join(V17, "parse_results.py"))
v17 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(v17)

# Frozen thresholds (PREDECLARED.json, discriminators).
A_ERROR_RATIO_MAX = 3.0
P_FAMILY_MAX = 0.50
P_CELL_MAX = 1.00
P_KILL_FAMILY = 0.85
S_FAMILY_MAX = 1.05
S_CELL_MAX = 1.15
G_STATE_UNITS_MAX = 10.0
G_ATTEMPTS_MAX = 0.20
CHOICE_MARGIN = 0.98
# Timing quality rule of the loop.
LOAD_MAX = 8.0
SPREAD_MAX = 0.10
WARM_UP_ROUNDS = 2
REPETITIONS = 7

RTOL_LABELS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
DECIDING = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"]
DEVELOPMENT = ["dev06_bruss1d", "dev06_adr2d"]
# Fields of a run record that are results (everything a run must reproduce).
RESULT_KEYS = ("completed", "success", "attempts", "accepted", "rejected", "state_reuses", "t_end_bits",
               "output_rows", "counters", "state_sha256", "n", "t_span", "rtol", "atol")
BUDGET_TEXT = "GMRES exhausted"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def rtol_value(label):
    return float(label)


def read_log(path):
    """Tagged JSON lines of one log: {tag: [records]}, plus header and footer."""
    if not os.path.exists(path):
        return None
    out = {"begin": None, "end": None, "tags": {}, "load_before": [], "load_after": [],
           "path": os.path.relpath(path, NODE)}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("# V24_LOG_BEGIN "):
                out["begin"] = json.loads(line[len("# V24_LOG_BEGIN "):])
            elif line.startswith("# V24_LOG_END "):
                out["end"] = json.loads(line[len("# V24_LOG_END "):])
            elif line.startswith("V24_LOAD_BEFORE "):
                out["load_before"].append(line[len("V24_LOAD_BEFORE "):])
            elif line.startswith("V24_LOAD_AFTER "):
                out["load_after"].append(line[len("V24_LOAD_AFTER "):])
            elif line.startswith("V24_") and " " in line:
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
    """One RUN log with its perf file."""
    directory = os.path.join(LOGS, phase)
    log = read_log(os.path.join(directory, name + ".log"))
    if log is None:
        return None
    record = first(log, "V24_RUN_JSON")
    state = first(log, "V24_STATE_JSON")
    begin, end = log["begin"] or {}, log["end"] or {}
    out = {
        "log": log["path"], "exit_code": end.get("exit_code"), "timed_out": end.get("timed_out"),
        "record": record, "state_hex": state["state_hex"] if state else None,
        "instructions_raw": v17.read_perf(os.path.join(directory, name + ".perf")),
        "binary_sha256": begin.get("binary_sha256"), "git_head": begin.get("git_head"),
        "git_dirty_tracked": begin.get("git_dirty_tracked"),
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


def setup_instructions(phase, family):
    runs = [v17.read_perf(os.path.join(LOGS, phase, f"{family}__SETUP__r{k}.perf")) for k in (1, 2, 3)]
    runs = [value for value in runs if value is not None]
    return {"runs": runs, "median": statistics.median(runs) if runs else None}


def note_identity(store, run):
    if run is None:
        return
    if run.get("binary_sha256"):
        store["binaries"][run["binary_sha256"]] = store["binaries"].get(run["binary_sha256"], 0) + 1
    if run.get("git_head"):
        key = f"{run['git_head']} dirty_tracked={run.get('git_dirty_tracked')}"
        store["git_heads"][key] = store["git_heads"].get(key, 0) + 1


def write_json(path, value):
    text = json.dumps(v17.clean(value), indent=1, sort_keys=False, allow_nan=False)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text + "\n")
    print(f"wrote {os.path.relpath(path, NODE)} ({len(text)} bytes)")


# ---------------------------------------------------------------- recording of commit A

def v17_arm_a(family, label):
    """Arm A, audit off, of node V17 for a deciding cell (cross-check only)."""
    path = os.path.join(V17, "logs", "runs", f"{family}__{label}__A__audit0.log")
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("V17_RUN_JSON "):
                return json.loads(line.split(" ", 1)[1])
    return None


def build_recording():
    out = {
        "schema": "vigilode.loop-l1.v24.recording-a.v1",
        "node": "V24_MF_ADAPTIVE_STAGE_SOLVE",
        "what": "the shipped matrix-free driver before any edit of the driver or of gmres_staged.rs, on the 20 deciding and the 10 development cells, through three entries (PROTOCOL.json, discriminator_protocol.D0)",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "binaries": {}, "git_heads": {}, "cells": {}, "setup_instructions": {}, "checks": [],
    }
    problems = out["checks"]
    entries = ("route", "route_ls", "audit")
    runs_expected = runs_present = 0
    v17_compared = v17_equal = 0
    for family in DECIDING + DEVELOPMENT:
        out["setup_instructions"][family] = setup_instructions("record_a", family)
        out["cells"][family] = {}
        for label in RTOL_LABELS:
            cell = {}
            fields = {}
            for entry in entries:
                runs_expected += 1
                run = collect_run("record_a", f"{family}__{label}__S__{entry}")
                note_identity(out, run)
                if run is None or not run["record"]:
                    problems.append(f"{family} {label} {entry}: no run record")
                    continue
                runs_present += 1
                record = run["record"]
                fields[entry] = result_fields(run)
                row = dict(fields[entry])
                row.update({
                    "log": run["log"], "exit_code": run["exit_code"], "driver": record.get("driver"),
                    "krylov_workspace": record.get("krylov_workspace"),
                    "wall_seconds_diagnostic_only": record.get("wall_seconds_diagnostic_only"),
                })
                if run["instructions_raw"] is not None:
                    row["instructions_raw"] = run["instructions_raw"]
                for key in ("residual_audit", "solve_log"):
                    if record.get(key):
                        row[key] = record[key]
                if not run["completed"]:
                    problems.append(f"{family} {label} {entry}: the run did not complete")
                cell[entry] = row
            agreement = {}
            for entry in ("route_ls", "audit"):
                agreement[entry] = differing_fields(fields.get("route"), fields.get(entry))
                if agreement[entry]:
                    problems.append(f"{family} {label}: entry {entry} differs from route in {agreement[entry]}")
            cell["entries_differing_from_route"] = agreement
            audit = (cell.get("audit") or {}).get("residual_audit")
            if audit and (audit["failures"] or audit["checks"] != (cell["audit"]["counters"] or {}).get("linear_solves")):
                problems.append(f"{family} {label}: residual audit of the shipped driver: {audit}")
            if family in DECIDING:
                theirs = v17_arm_a(family, label)
                mine = cell.get("route")
                if theirs and mine:
                    v17_compared += 1
                    keys = ("success", "attempts", "accepted", "rejected", "counters", "state_sha256", "t_span", "n")
                    differing = [key for key in keys if theirs.get(key) != mine.get(key)]
                    cell["v17_arm_A_audit0"] = {"compared_fields": list(keys), "differing": differing}
                    if differing:
                        problems.append(f"{family} {label}: the route entry differs from V17 arm A in {differing}")
                    else:
                        v17_equal += 1
            out["cells"][family][label] = cell
    out["summary"] = {
        "runs_expected": runs_expected, "runs_present": runs_present,
        "cells": len(DECIDING + DEVELOPMENT) * len(RTOL_LABELS),
        "v17_arm_A_cells_compared": v17_compared, "v17_arm_A_cells_equal": v17_equal,
        "problems": len(problems),
    }
    return out


# ---------------------------------------------------------------- references and runs

def load_reference(family):
    """The reference state of a family with its uncertainty; nothing is derived here."""
    if family in DECIDING:
        path = os.path.join(V17, "reference", family + ".json")
        with open(path, encoding="utf-8") as handle:
            reference = json.load(handle)
        u_ref = reference["uncertainty_max_scaled_1_plus_abs"]
        note = "V17 reference; u_ref = max over the cross-checks of max_i |ref_i - other_i| / (1 + |ref_i|)"
    else:
        path = os.path.join(V06, "reference", family[len("dev06_"):] + ".json")
        with open(path, encoding="utf-8") as handle:
            reference = json.load(handle)
        u_ref = reference["uncertainty"]["max_abs"]
        note = "V06 reference; u_ref = stored max_i |ref_i - other_i| (an upper bound of the V17 statistic)"
    return {
        "state": v17.unhex(reference["state_hex"]), "u_ref": u_ref, "note": note,
        "file": os.path.relpath(path, NODE), "sha256": sha256_file(path),
        "n": reference["n"], "t_span": reference["t_span"], "method": reference["reference"],
        "y0_hex": reference.get("y0_hex"), "f0_hex": reference.get("f0_hex"),
    }


def summarise_run(run, reference, setup_median, rtol):
    """The numbers of one run that the tables use."""
    if run is None:
        return {"present": False, "completed": False}
    record = run["record"] or {}
    counters = record.get("counters") or {}
    out = {
        "present": True, "log": run["log"], "completed": run["completed"],
        "success": record.get("success"), "error": record.get("error"),
        "exit_code": run["exit_code"], "timed_out": run["timed_out"],
        "arm": record.get("arm"), "entry": record.get("entry"), "driver": record.get("driver"),
        "policy": record.get("policy"), "accounting": record.get("accounting"),
        "attempts": record.get("attempts"), "accepted": record.get("accepted"), "rejected": record.get("rejected"),
        "jvp_calls": record.get("jvp_calls"), "rhs_calls": counters.get("rhs_calls"),
        "linear_solves": counters.get("linear_solves"), "linear_iterations": counters.get("linear_iterations"),
        "linear_matvecs": counters.get("linear_matvecs"), "diagnostic_matvecs": counters.get("diagnostic_matvecs"),
        "linear_solve_failures": counters.get("linear_solve_failures"),
        "local_error_failures": counters.get("local_error_failures"),
        "orthogonalization_inner_products": counters.get("orthogonalization_inner_products"),
        "orthogonalization_vector_updates": counters.get("orthogonalization_vector_updates"),
        "state_sha256": record.get("state_sha256"),
        "wall_seconds_diagnostic_only": record.get("wall_seconds_diagnostic_only"),
        "instructions_raw": run["instructions_raw"],
        "t_span": record.get("t_span"), "n": record.get("n"),
    }
    if run["instructions_raw"] is not None and setup_median is not None:
        out["instructions_net"] = run["instructions_raw"] - setup_median
    if counters.get("linear_solves"):
        out["krylov_vectors_per_completed_and_failed_solve"] = v17.ratio(
            counters.get("linear_iterations"), counters.get("linear_solves") + (counters.get("linear_solve_failures") or 0))
    for key in ("residual_audit", "record_audit", "policy_statistics", "solve_log"):
        if record.get(key):
            out[key] = record[key]
    if run["state_hex"] and reference is not None and run["completed"]:
        state = v17.unhex(run["state_hex"])
        if len(state) == len(reference["state"]):
            metrics = v17.error_metrics(state, reference["state"])
            out.update(metrics)
            out["E_over_rtol"] = metrics["E"] / rtol
            out["reference_limited"] = metrics["E"] < 10.0 * reference["u_ref"]
    return out


def audit_verdict(plain, audited):
    """Discriminator V on one cell of one arm (PROTOCOL.json, checks 1 to 4)."""
    out = {"audited": bool(audited and audited["record"] and audited["record"].get("completed"))}
    if not out["audited"]:
        out.update({"violations": None, "ok": False, "reason": "no completed audit-on run"})
        return out
    record = audited["record"]
    counters = record.get("counters") or {}
    independent = record.get("residual_audit") or {}
    own = record.get("record_audit") or {}
    statistics = record.get("policy_statistics") or {}
    solves = counters.get("linear_solves")
    failures = counters.get("linear_solve_failures")
    coverage = []
    if independent.get("checks") != solves:
        coverage.append(f"independent checks {independent.get('checks')} != completed solves {solves}")
    if own and own.get("records") != solves:
        coverage.append(f"driver records {own.get('records')} != completed solves {solves}")
    if statistics:
        if statistics.get("completed_solves") != solves:
            coverage.append(f"statistics.completed_solves {statistics.get('completed_solves')} != {solves}")
        classified = (statistics.get("budget_exhausted_solves") or 0) + (statistics.get("other_linear_failures") or 0)
        if classified != failures:
            coverage.append(f"classified failures {classified} != linear_solve_failures {failures}")
    same = differing_fields(result_fields(plain), result_fields(audited))
    out.update({
        "completed_solves": solves, "failed_solves": failures,
        "budget_exhausted_solves": statistics.get("budget_exhausted_solves"),
        "other_linear_failures": statistics.get("other_linear_failures"),
        "independent_checks": independent.get("checks"), "independent_failures": independent.get("failures"),
        "independent_max_residual_over_threshold": independent.get("max_ratio"),
        "driver_records": own.get("records") if own else None,
        "driver_record_violations": own.get("violations") if own else None,
        "driver_max_residual_over_threshold": own.get("max_ratio") if own else None,
        "failed_confirmations": own.get("failed_confirmations") if own else None,
        "coverage_mismatches": coverage,
        "audit_on_differs_from_audit_off_in": same,
    })
    out["violations"] = (independent.get("failures") or 0) + ((own.get("violations") or 0) if own else 0) + len(coverage)
    out["deterministic"] = not same
    out["ok"] = out["violations"] == 0 and out["deterministic"] and (independent.get("checks") or 0) > 0
    return out


def collect_arm(phase, setup_phase, family, arm, reference, store):
    """Per tolerance: the audit-off run (instructions), the audit-on run, V."""
    setup = setup_instructions(setup_phase, family)
    cells = {}
    for label in RTOL_LABELS:
        plain = collect_run(phase, f"{family}__{label}__{arm}__route")
        audited = collect_run(phase, f"{family}__{label}__{arm}__audit")
        note_identity(store, plain)
        note_identity(store, audited)
        cell = summarise_run(plain, reference, setup["median"], rtol_value(label))
        cell["audit"] = audit_verdict(plain, audited)
        if audited and audited["record"]:
            for key in ("policy_statistics", "record_audit", "solve_log", "residual_audit"):
                if audited["record"].get(key):
                    cell[key] = audited["record"][key]
        if plain and plain["record"] and reference is not None:
            if plain["record"].get("t_span") != reference["t_span"] or plain["record"].get("n") != reference["n"]:
                cell["completed"] = False
                cell["error"] = "the run is not on the span or size of the reference"
        cells[label] = cell
    return {"setup_instructions": setup, "cells": cells}


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


def compare_arms(cells_s, cells_x, u_ref):
    """Arm X against the shipped arm S on one family: the V17 matched-error
    statistic, the cell numbers of PROTOCOL.json and the ratios at equal
    tolerance."""
    floor = 10.0 * u_ref
    out = {"matched_error": {}, "equal_tolerance": {}, "cells": {}}
    for measure in ("instructions_net", "attempts", "jvp_calls"):
        points_s, completed_s = curve(cells_s, measure)
        points_x, completed_x = curve(cells_x, measure)
        out["matched_error"][measure] = v17.matched(points_s, points_x, floor, completed_s, completed_x, 4)
    block = out["matched_error"]["instructions_net"]
    at_points = {entry["E"]: entry["ratio"] for entry in block.get("ratio_at_points_of_A", [])}
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
                "error_ratio": v17.ratio(x.get("E"), s.get("E")),
                "E_S": s.get("E"), "E_X": x.get("E"),
                "reference_limited": bool(s.get("reference_limited") or x.get("reference_limited")),
                "wall_diagnostic_single_run_speedup": v17.ratio(s.get("wall_seconds_diagnostic_only"),
                                                               x.get("wall_seconds_diagnostic_only")),
            })
            if s.get("E") in at_points:
                row["instructions_matched_at_E_S"] = at_points[s["E"]]
                row["cell_number"] = at_points[s["E"]]
                row["cell_number_kind"] = "matched error at the error of arm S"
            else:
                row["cell_number"] = row["instructions_equal_tolerance"]
                row["cell_number_kind"] = "equal tolerance (the S point is outside the matched range)"
        out["cells"][label] = row
    rows = [out["cells"][label] for label in RTOL_LABELS]
    out["equal_tolerance"] = {
        "instructions_geometric_mean": geometric_mean([row.get("instructions_equal_tolerance") for row in rows]) if all(row["both_completed"] for row in rows) else None,
        "attempts_geometric_mean": geometric_mean([row.get("attempts_equal_tolerance") for row in rows]) if all(row["both_completed"] for row in rows) else None,
        "jvp_geometric_mean": geometric_mean([row.get("jvp_equal_tolerance") for row in rows]) if all(row["both_completed"] for row in rows) else None,
    }
    out["cell_numbers_geometric_mean"] = (
        geometric_mean([row.get("cell_number") for row in rows]) if all(row.get("cell_number") is not None for row in rows) else None)
    return out


# ---------------------------------------------------------------- the choice of the variant

def build_choice():
    out = {
        "schema": "vigilode.loop-l1.v24.choice.v1",
        "node": "V24_MF_ADAPTIVE_STAGE_SOLVE",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "frozen_rule": "the variant with the smaller geometric mean of instructions at matched achieved error on the two development problems; within 2 percent P1 is taken (PREDECLARED.json, policy.choice_of_the_variant)",
        "computation": "PROTOCOL.json, choice_of_the_variant",
        "binaries": {}, "git_heads": {}, "problems": {}, "checks": [],
    }
    scores = {"P1": [], "P2": []}
    fallback_used = False
    violations = {"P1": 0, "P2": 0}
    for family in DEVELOPMENT:
        reference = load_reference(family)
        arms = {arm: collect_arm("dev", "dev", family, arm, reference, out) for arm in ("S", "P1", "P2")}
        entry = {"n": reference["n"], "t_span": reference["t_span"],
                 "reference": {key: reference[key] for key in ("file", "sha256", "u_ref", "note", "method")},
                 "setup_instructions": arms["S"]["setup_instructions"], "arms": {}, "against_S": {}}
        for arm in ("S", "P1", "P2"):
            entry["arms"][arm] = arms[arm]["cells"]
            for label in RTOL_LABELS:
                cell = arms[arm]["cells"][label]
                if not cell.get("completed"):
                    out["checks"].append(f"{family} {label} {arm}: the run did not complete")
                if not cell["audit"]["ok"]:
                    out["checks"].append(f"{family} {label} {arm}: audit {cell['audit']}")
                if arm != "S":
                    violations[arm] += cell["audit"]["violations"] or 0
        for arm in ("P1", "P2"):
            entry["against_S"][arm] = compare_arms(arms["S"]["cells"], arms[arm]["cells"], reference["u_ref"])
        entry["P2_against_P1"] = compare_arms(arms["P1"]["cells"], arms["P2"]["cells"], reference["u_ref"])
        matched = {arm: entry["against_S"][arm]["matched_error"]["instructions_net"].get("geometric_mean_ratio") for arm in ("P1", "P2")}
        if any(value is None for value in matched.values()):
            fallback_used = True
            used = {arm: entry["against_S"][arm]["equal_tolerance"]["instructions_geometric_mean"] for arm in ("P1", "P2")}
            entry["statistic_used"] = "geometric mean of the ratio at equal tolerance (fallback: the matched-error statistic is not decidable for a variant)"
        else:
            used = matched
            entry["statistic_used"] = "V17 matched-error family statistic of the instructions against arm S"
        entry["R"] = used
        entry["R_matched_error"] = matched
        for arm in ("P1", "P2"):
            scores[arm].append(used[arm])
        out["problems"][family] = entry
    score = {arm: geometric_mean(scores[arm]) if all(value is not None for value in scores[arm]) else None for arm in ("P1", "P2")}
    out["score"] = score
    out["fallback_used"] = fallback_used
    out["V_violations_in_development_runs"] = violations
    if score["P1"] is None or score["P2"] is None:
        out["chosen"] = None
        out["reason"] = "a score could not be computed"
    else:
        out["score_ratio_P2_over_P1"] = score["P2"] / score["P1"]
        out["margin"] = CHOICE_MARGIN
        take_p2 = score["P2"] < CHOICE_MARGIN * score["P1"]
        out["chosen"] = "P2" if take_p2 else "P1"
        out["reason"] = ("score(P2) is below 0.98 score(P1)" if take_p2
                         else "score(P2) is not below 0.98 score(P1): P1 is taken")
        if violations[out["chosen"]]:
            out["chosen_is_eligible"] = False
            out["reason"] += "; the chosen variant has a V violation in a development run (KILL by the frozen rule)"
        else:
            out["chosen_is_eligible"] = True
    return out


# ---------------------------------------------------------------- D0

SHIPPED_DRIVER_ID = "rodas5p-mf-fast-transformed-v1"


def build_d0(store):
    """The shipped policy after the edit against the recording of commit A."""
    entries = ("route", "route_ls", "audit")
    out = {"cells": {}, "recorded_fields": list(RESULT_KEYS) + ["error", "state_hex_sha256"],
           "audit_entry_also": ["solve_log", "residual_audit"]}
    counts = {"deciding": [0, 0], "development": [0, 0]}
    problems = []
    instruction_ratios = []
    for family in DECIDING + DEVELOPMENT:
        group = "deciding" if family in DECIDING else "development"
        out["cells"][family] = {}
        for label in RTOL_LABELS:
            before = {entry: collect_run("record_a", f"{family}__{label}__S__{entry}") for entry in entries}
            after = {entry: collect_run("d0", f"{family}__{label}__S__{entry}") for entry in entries + ("policy_shipped",)}
            row = {"differing": {}}
            present = True
            for entry in entries + ("policy_shipped",):
                note_identity(store, after[entry])
                if after[entry] is None or not after[entry]["record"]:
                    present = False
            for entry in entries:
                differing = differing_fields(result_fields(before[entry]), result_fields(after[entry]))
                if entry == "audit" and before[entry] and after[entry] and before[entry]["record"] and after[entry]["record"]:
                    for key in ("solve_log", "residual_audit"):
                        if before[entry]["record"].get(key) != after[entry]["record"].get(key):
                            differing.append(key)
                row["differing"][entry] = differing
            row["differing"]["policy_shipped_against_route_after"] = differing_fields(
                result_fields(after["route"]), result_fields(after["policy_shipped"]))
            drivers = sorted({(after[entry]["record"] or {}).get("driver") for entry in after if after[entry] and after[entry]["record"]}, key=str)
            row["driver_ids_after"] = drivers
            row["present"] = present
            row["equal"] = present and all(not value for value in row["differing"].values()) and drivers == [SHIPPED_DRIVER_ID]
            if before["route"] and after["route"]:
                raw_before, raw_after = before["route"]["instructions_raw"], after["route"]["instructions_raw"]
                row["instructions_raw_before"] = raw_before
                row["instructions_raw_after"] = raw_after
                row["instructions_after_over_before"] = v17.ratio(raw_after, raw_before)
                if row["instructions_after_over_before"] is not None:
                    instruction_ratios.append(row["instructions_after_over_before"])
            counts[group][0] += 1
            counts[group][1] += int(row["equal"])
            if not row["equal"]:
                problems.append(f"{family} {label}: {row['differing']} drivers {drivers} present {present}")
            out["cells"][family][label] = row
    out["deciding_cells"] = {"compared": counts["deciding"][0], "equal": counts["deciding"][1]}
    out["development_cells"] = {"compared": counts["development"][0], "equal": counts["development"][1]}
    out["differences"] = problems
    out["exploratory_instructions_after_over_before"] = {
        "min": min(instruction_ratios) if instruction_ratios else None,
        "max": max(instruction_ratios) if instruction_ratios else None,
        "geometric_mean": geometric_mean(instruction_ratios),
        "note": "raw instructions of the route entry, binary after the edit over binary of commit A (two different binaries; not a discriminator)",
    }
    if counts["deciding"][0] != len(DECIDING) * len(RTOL_LABELS):
        out["verdict"] = "NOT_RUN"
    else:
        out["verdict"] = "PASS" if counts["deciding"][1] == counts["deciding"][0] else "FAIL"
    return out


# ---------------------------------------------------------------- timing

def collect_timing(family, label, arm, plain_states):
    """Every timing measurement of a cell, in the order of the tries (V17 rule, two arms)."""
    directory = os.path.join(LOGS, "timing")
    arms = ("S", arm)
    tries = []
    for attempt in (1, 2):
        single = os.path.join(directory, f"{family}__{label}__try{attempt}.log")
        split = sorted(glob.glob(os.path.join(directory, f"{family}__{label}__try{attempt}__round*.log")))
        paths = [single] if os.path.exists(single) else split
        if not paths:
            continue
        logs = [read_log(path) for path in paths]
        runs, loads_before, loads_after, metas, complete, seconds = [], [], [], [], True, []
        for log in logs:
            runs.extend(log["tags"].get("V24_TIMING_JSON", []))
            metas.extend(log["tags"].get("V24_TIMING_META_JSON", []))
            loads_before.extend(log["load_before"])
            loads_after.extend(log["load_after"])
            if not log["end"] or log["end"].get("exit_code") != 0:
                complete = False
            if log["end"]:
                seconds.append(log["end"].get("seconds"))
        entry = {
            "try": attempt, "logs": [log["path"] for log in logs], "invocations": len(logs),
            "split_long_cell": len(logs) > 1, "process_complete": complete,
            "load_before": loads_before, "load_after": loads_after,
            "longest_invocation_seconds_including_the_wait_for_the_lock": max([value for value in seconds if value is not None], default=None),
            "longest_process_seconds_under_the_lock": max([meta.get("process_seconds") for meta in metas if meta.get("process_seconds") is not None], default=None),
            "arms": {},
        }
        load_values = [float(text.split()[0]) for text in loads_before + loads_after if text.split()]
        entry["load_1min_max"] = max(load_values) if load_values else None
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
        out["speedup_equal_tolerance"] = v17.ratio(out["median_wall"].get("S"), out["median_wall"].get(arm))
        out["thread_cpu_speedup_equal_tolerance"] = v17.ratio(out["median_thread_cpu"].get("S"), out["median_thread_cpu"].get(arm))
    return out


def timing_family(cells_s, cells_n, timings, arm, u_ref):
    """The wall statistic of a family at matched error (V17 statistic on the medians)."""
    out = {}
    labels = [timings[label]["label"] for label in RTOL_LABELS]
    out["cells"] = dict(zip(RTOL_LABELS, labels))
    if all(label == "authoritative" for label in labels):
        out["label"] = "authoritative"
    elif all(timings[label].get("median_wall") for label in RTOL_LABELS):
        out["label"] = "diagnostic_only"
    else:
        out["label"] = "NOT_RUN" if all(label == "NOT_RUN" for label in labels) else "incomplete"
    for key, field in (("wall", "median_wall"), ("thread_cpu", "median_thread_cpu")):
        points = {"S": [], arm: []}
        completed = {"S": 0, arm: 0}
        for name, cells in (("S", cells_s), (arm, cells_n)):
            for label in RTOL_LABELS:
                cell = cells[label]
                work = (timings[label].get(field) or {}).get(name)
                if not cell.get("completed") or cell.get("E") is None or work is None:
                    continue
                completed[name] += 1
                points[name].append((cell["E"], float(work)))
        block = v17.matched(points["S"], points[arm], 10.0 * u_ref, completed["S"], completed[arm], 4)
        if block.get("geometric_mean_ratio"):
            block["speedup"] = 1.0 / block["geometric_mean_ratio"]
            block["speedup_min"] = 1.0 / block["max_ratio"]
            block["speedup_max"] = 1.0 / block["min_ratio"]
        out[key] = block
    out["thread_cpu"]["label"] = "exploratory, never used for a verdict"
    equal = [timings[label].get("speedup_equal_tolerance") for label in RTOL_LABELS]
    out["speedup_equal_tolerance"] = dict(zip(RTOL_LABELS, equal))
    out["speedup_equal_tolerance_geometric_mean"] = geometric_mean(equal) if all(value is not None for value in equal) else None
    return out


# ---------------------------------------------------------------- results

def verdict_of(parts):
    if any(part == "FAIL" for part in parts):
        return "FAIL"
    if any(part != "PASS" for part in parts):
        return "NOT_DECIDABLE"
    return "PASS"


def evaluate_family(family, arm, cells_s, cells_n, comparison, u_ref):
    """The frozen discriminators A, P, S, B and V on one deciding family."""
    out = {}
    rows = comparison["cells"]
    complete = all(rows[label]["both_completed"] for label in RTOL_LABELS)

    # A: achieved error at most 3 times that of the shipped arm, in every cell.
    a_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("error_ratio")
        a_cells[label] = {"error_ratio": value, "reference_limited": rows[label].get("reference_limited"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= A_ERROR_RATIO_MAX else "FAIL")}
    out["A"] = {"cells": a_cells, "max_error_ratio": max((c["error_ratio"] for c in a_cells.values() if c["error_ratio"] is not None), default=None),
                "verdict": verdict_of([c["verdict"] for c in a_cells.values()])}

    # P: instructions at matched achieved error.
    block = comparison["matched_error"]["instructions_net"]
    family_number = block.get("geometric_mean_ratio")
    p_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("cell_number")
        p_cells[label] = {"cell_number": value, "kind": rows[label].get("cell_number_kind"),
                          "equal_tolerance": rows[label].get("instructions_equal_tolerance"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= P_CELL_MAX else "FAIL")}
    family_part = "NOT_DECIDABLE" if family_number is None else ("PASS" if family_number <= P_FAMILY_MAX else "FAIL")
    cells_part = verdict_of([c["verdict"] for c in p_cells.values()])
    out["P"] = {
        "family_number": family_number, "family_part": family_part, "matched_range": block.get("range"),
        "matched_status": block.get("status"), "matched_reason": block.get("reason"),
        "cells": p_cells, "cells_part": cells_part,
        "max_cell_number": max((c["cell_number"] for c in p_cells.values() if c["cell_number"] is not None), default=None),
        "verdict": verdict_of([family_part, cells_part]),
        "above_kill_level_0_85": None if family_number is None else family_number > P_KILL_FAMILY,
        "also_reported": {
            "geometric_mean_of_the_cell_numbers": comparison["cell_numbers_geometric_mean"],
            "geometric_mean_at_equal_tolerance": comparison["equal_tolerance"]["instructions_geometric_mean"],
            "largest_ratio_on_the_grid": block.get("max_ratio"), "smallest_ratio_on_the_grid": block.get("min_ratio"),
            "largest_ratio_at_equal_tolerance": max((c["equal_tolerance"] for c in p_cells.values() if c["equal_tolerance"] is not None), default=None),
        },
    }
    also = out["P"]["also_reported"]
    notes = []
    for name in ("geometric_mean_of_the_cell_numbers", "geometric_mean_at_equal_tolerance"):
        if also[name] is not None and family_number is not None and (also[name] <= P_FAMILY_MAX) != (family_number <= P_FAMILY_MAX):
            notes.append(f"the family part would differ under {name} ({also[name]:.4f})")
    for name in ("largest_ratio_on_the_grid", "largest_ratio_at_equal_tolerance"):
        if also[name] is not None and cells_part == "PASS" and also[name] > P_CELL_MAX:
            notes.append(f"{name} is {also[name]:.4f}, above 1.00, while every cell number is not")
    out["P"]["notes_next_to_the_verdict"] = notes

    # S: attempts at equal tolerance.
    s_cells = {}
    for label in RTOL_LABELS:
        value = rows[label].get("attempts_equal_tolerance")
        s_cells[label] = {"attempts_ratio": value, "attempts_S": cells_s[label].get("attempts"), "attempts_N": cells_n[label].get("attempts"),
                          "verdict": "NOT_DECIDABLE" if value is None else ("PASS" if value <= S_CELL_MAX else "FAIL")}
    mean = comparison["equal_tolerance"]["attempts_geometric_mean"] if complete else None
    family_part = "NOT_DECIDABLE" if mean is None else ("PASS" if mean <= S_FAMILY_MAX else "FAIL")
    out["S"] = {"geometric_mean": mean, "family_part": family_part, "cells": s_cells,
                "cells_part": verdict_of([c["verdict"] for c in s_cells.values()]),
                "max_ratio": max((c["attempts_ratio"] for c in s_cells.values() if c["attempts_ratio"] is not None), default=None),
                "matched_error_attempts_ratio_reported_only": comparison["matched_error"]["attempts"].get("geometric_mean_ratio")}
    out["S"]["verdict"] = verdict_of([family_part, out["S"]["cells_part"]])

    # B: budget-exhausting stage solves.
    b_cells = {}
    for label in RTOL_LABELS:
        stat_s = cells_s[label].get("policy_statistics") or {}
        stat_n = cells_n[label].get("policy_statistics") or {}
        bs, bn = stat_s.get("budget_exhausted_solves"), stat_n.get("budget_exhausted_solves")
        b_cells[label] = {"S": bs, "N": bn, "other_linear_failures_S": stat_s.get("other_linear_failures"),
                          "other_linear_failures_N": stat_n.get("other_linear_failures"),
                          "linear_solve_failures_S": cells_s[label].get("linear_solve_failures"),
                          "linear_solve_failures_N": cells_n[label].get("linear_solve_failures"),
                          "verdict": "NOT_DECIDABLE" if bs is None or bn is None else ("PASS" if bn <= bs else "FAIL")}
    out["B"] = {"cells": b_cells, "total_S": sum(c["S"] or 0 for c in b_cells.values()), "total_N": sum(c["N"] or 0 for c in b_cells.values()),
                "verdict": verdict_of([c["verdict"] for c in b_cells.values()])}

    # V: the audit of every stage solve of arm N (and of arm S, reported).
    v_cells = {}
    totals = {"completed_solves": 0, "failed_solves": 0, "independent_checks": 0, "violations": 0}
    max_ratio = 0.0
    for label in RTOL_LABELS:
        audit = cells_n[label]["audit"]
        v_cells[label] = audit
        for key in ("completed_solves", "failed_solves", "independent_checks", "violations"):
            totals[key] += audit.get(key) or 0
        max_ratio = max(max_ratio, audit.get("independent_max_residual_over_threshold") or 0.0,
                        audit.get("driver_max_residual_over_threshold") or 0.0)
    all_audited = all(v_cells[label].get("audited") for label in RTOL_LABELS)
    all_ok = all(v_cells[label].get("ok") for label in RTOL_LABELS)
    out["V"] = {"cells": v_cells, "totals": totals, "max_residual_over_threshold": max_ratio,
                "every_cell_audited": all_audited, "every_cell_deterministic": all(v_cells[label].get("deterministic") for label in RTOL_LABELS if v_cells[label].get("audited")),
                "verdict": "PASS" if all_ok else ("FAIL" if totals["violations"] > 0 or all_audited else "NOT_DECIDABLE")}
    shipped_checks = sum((cells_s[label]["audit"].get("independent_checks") or 0) for label in RTOL_LABELS)
    shipped_violations = sum((cells_s[label]["audit"].get("violations") or 0) for label in RTOL_LABELS)
    out["V"]["shipped_arm_reported"] = {"independent_checks": shipped_checks, "violations": shipped_violations}
    return out


def adaptive_arm(label):
    return label in ("P1", "P2", "P1R", "P2R")


def collect_shadow(store):
    """Exploratory: the replay of every stage system with the three kernels."""
    out = {"runs": {}, "note": "paired on identical stage systems: the systems of the run of the named arm, each solved again by the shipped kernel, P1 and P2 with their own operator; kernels order [shipped, P1, P2]"}
    for path in sorted(glob.glob(os.path.join(LOGS, "shadow", "*.log"))):
        log = read_log(path)
        record = first(log, "V24_SHADOW_JSON")
        name = os.path.basename(path)[:-len(".log")]
        begin, end = log["begin"] or {}, log["end"] or {}
        note_identity(store, {"binary_sha256": begin.get("binary_sha256"), "git_head": begin.get("git_head"),
                              "git_dirty_tracked": begin.get("git_dirty_tracked")})
        row = {"log": log["path"], "exit_code": end.get("exit_code")}
        if record:
            for key in ("family", "rtol", "arm", "success", "attempts", "state_sha256", "systems", "driver_completed",
                        "driver_failed", "replay_differs_from_driver", "solved", "budget", "other", "columns_of_solved",
                        "all_solved", "columns_where_all_solved", "shipped_solved_p1_budget", "shipped_budget_p1_solved",
                        "shipped_solved_p2_budget", "shipped_budget_p2_solved", "p1_solved_p2_budget", "p1_budget_p2_solved",
                        "disagreements_first_40", "disagreement_columns", "policy_statistics"):
                row[key] = record.get(key)
            row["linear_solve_failures"] = (record.get("counters") or {}).get("linear_solve_failures")
        out["runs"][name] = row
    return out


def build_results():
    with open(os.path.join(NODE, "CHOICE.json"), encoding="utf-8") as handle:
        choice = json.load(handle)
    arm = choice["chosen"]
    predeclared_path = os.path.join(NODE, "PREDECLARED.json")
    protocol_path = os.path.join(NODE, "PROTOCOL.json")
    results = {
        "schema": "vigilode.loop-l1.v24.results.v1",
        "node": "V24_MF_ADAPTIVE_STAGE_SOLVE",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "predeclared_sha256": sha256_file(predeclared_path),
        "protocol_sha256": sha256_file(protocol_path),
        "frozen_thresholds": {
            "A_error_ratio_max": A_ERROR_RATIO_MAX, "P_family_max": P_FAMILY_MAX, "P_cell_max": P_CELL_MAX,
            "P_kill_family": P_KILL_FAMILY, "S_family_max": S_FAMILY_MAX, "S_cell_max": S_CELL_MAX,
            "G_state_units_max": G_STATE_UNITS_MAX, "G_attempts_max": G_ATTEMPTS_MAX,
            "timing_load_max": LOAD_MAX, "timing_spread_max": SPREAD_MAX,
        },
        "arms": {"S": "the driver with the shipped stage-solve policy", "N": f"the chosen adaptive variant: {arm}"},
        "chosen_variant": arm,
        "choice": {key: choice.get(key) for key in ("score", "score_ratio_P2_over_P1", "chosen", "reason", "fallback_used", "V_violations_in_development_runs")},
        "binaries": {}, "git_heads": {}, "checks": [], "families": {},
    }
    problems = results["checks"]
    results["D0"] = build_d0(results)
    all_violations = {"deciding": 0, "development": sum((choice.get("V_violations_in_development_runs") or {}).values()), "exploratory": 0}

    for family in DECIDING:
        reference = load_reference(family)
        shipped = collect_arm("d0", "setup", family, "S", reference, results)
        new = collect_arm("runs", "setup", family, arm, reference, results)
        entry = {
            "n": reference["n"], "t_span": reference["t_span"],
            "reference": {key: reference[key] for key in ("file", "sha256", "u_ref", "note", "method")},
            "setup_instructions": shipped["setup_instructions"],
            "arms": {"S": shipped["cells"], "N": new["cells"]},
        }
        rhs0 = first(read_log(os.path.join(LOGS, "setup", f"{family}__RHS0.log")), "V24_RHS0_JSON")
        if rhs0 and reference["y0_hex"] and reference["f0_hex"]:
            y0_rust, f0_rust = v17.unhex(rhs0["y0_hex"]), v17.unhex(rhs0["f0_hex"])
            y0_ref, f0_ref = v17.unhex(reference["y0_hex"]), v17.unhex(reference["f0_hex"])
            scale = max(abs(value) for value in f0_ref) or 1.0
            consistency = {
                "y0_max_abs_difference": max(abs(a - b) for a, b in zip(y0_rust, y0_ref)),
                "f0_max_relative_difference": max(abs(a - b) for a, b in zip(f0_rust, f0_ref)) / scale,
            }
            consistency["consistent"] = consistency["f0_max_relative_difference"] <= 1e-12 and consistency["y0_max_abs_difference"] <= 1e-12
            entry["reference"]["rhs_consistency"] = consistency
            if not consistency["consistent"]:
                problems.append(f"{family}: numpy and Rust right-hand sides differ at the start state")
        else:
            problems.append(f"{family}: no RHS0 record")
        for name, cells in (("S", shipped["cells"]), ("N", new["cells"])):
            for label in RTOL_LABELS:
                if not cells[label].get("completed"):
                    problems.append(f"{family} {label} {name}: the run did not complete ({cells[label].get('error')})")
        comparison = compare_arms(shipped["cells"], new["cells"], reference["u_ref"])
        entry["comparison"] = comparison
        entry["discriminators"] = evaluate_family(family, arm, shipped["cells"], new["cells"], comparison, reference["u_ref"])
        all_violations["deciding"] += entry["discriminators"]["V"]["totals"]["violations"]
        plain_states = {}
        timings = {}
        for label in RTOL_LABELS:
            plain_states = {"S": shipped["cells"][label].get("state_sha256"), arm: new["cells"][label].get("state_sha256")}
            timings[label] = collect_timing(family, label, arm, plain_states)
        entry["timing_cells"] = timings
        entry["T"] = timing_family(shipped["cells"], new["cells"], timings, arm, reference["u_ref"])
        results["families"][family] = entry

    # The development runs of the chosen arm, repeated with the final binary.
    repeat = {"cells": {}, "compared": 0, "equal": 0}
    for family in DEVELOPMENT:
        for label in RTOL_LABELS:
            for entry_name in ("route", "audit"):
                before = collect_run("dev", f"{family}__{label}__{arm}__{entry_name}")
                after = collect_run("dev_repeat", f"{family}__{label}__{arm}__{entry_name}")
                if after is None:
                    continue
                note_identity(results, after)
                differing = differing_fields(result_fields(before), result_fields(after))
                repeat["compared"] += 1
                repeat["equal"] += int(not differing)
                if differing:
                    repeat["cells"][f"{family} {label} {entry_name}"] = differing
                audit = audit_verdict(collect_run("dev_repeat", f"{family}__{label}__{arm}__route"), after) if entry_name == "audit" else None
                if audit is not None:
                    all_violations["development"] += audit.get("violations") or 0
    results["development_repeat_with_the_final_binary"] = repeat

    # Exploratory arms on the deciding grid.
    exploratory = {}
    present = sorted({os.path.basename(path).split("__")[2] for path in glob.glob(os.path.join(LOGS, "exploratory", "*__route.log"))})
    for label_arm in present:
        block = {"families": {}}
        for family in DECIDING:
            reference = load_reference(family)
            shipped_cells = results["families"][family]["arms"]["S"]
            cells = collect_arm("exploratory", "setup", family, label_arm, reference, results)["cells"]
            comparison = compare_arms(shipped_cells, cells, reference["u_ref"])
            violations = sum((cells[label]["audit"].get("violations") or 0) for label in RTOL_LABELS)
            if adaptive_arm(label_arm):
                all_violations["exploratory"] += violations
            block["families"][family] = {
                "cells": cells,
                "instructions_matched_error": comparison["matched_error"]["instructions_net"].get("geometric_mean_ratio"),
                "instructions_equal_tolerance": comparison["equal_tolerance"]["instructions_geometric_mean"],
                "attempts_equal_tolerance": comparison["equal_tolerance"]["attempts_geometric_mean"],
                "jvp_equal_tolerance": comparison["equal_tolerance"]["jvp_geometric_mean"],
                "max_error_ratio": max((comparison["cells"][label].get("error_ratio") or 0.0) for label in RTOL_LABELS),
                "per_cell_instructions_equal_tolerance": {label: comparison["cells"][label].get("instructions_equal_tolerance") for label in RTOL_LABELS},
                "audit_violations": violations,
                "audit_ok_in_every_cell": all(cells[label]["audit"].get("ok") for label in RTOL_LABELS),
            }
        exploratory[label_arm] = block
    results["exploratory"] = exploratory
    results["exploratory_budget_replay"] = collect_shadow(results)
    # A replay run must be the run of the arm it shadows: same end state.
    for name, row in results["exploratory_budget_replay"]["runs"].items():
        family, label, label_arm = name.split("__")
        if family in results["families"]:
            cells = results["families"][family]["arms"]["S" if label_arm == "S" else "N"] if label_arm in ("S", arm) else None
            if cells is not None:
                row["end_state_equals_the_instruction_run"] = row.get("state_sha256") == cells[label].get("state_sha256")

    # Discriminators and verdict.
    def family_verdicts(name):
        return {family: results["families"][family]["discriminators"][name]["verdict"] for family in DECIDING}

    discriminators = {"D0": {"verdict": results["D0"]["verdict"], "deciding_cells": results["D0"]["deciding_cells"],
                             "development_cells": results["D0"]["development_cells"]}}
    for name in ("A", "P", "S", "B"):
        verdicts = family_verdicts(name)
        discriminators[name] = {"families": verdicts, "verdict": verdict_of(list(verdicts.values()))}
    discriminators["P"]["family_numbers"] = {family: results["families"][family]["discriminators"]["P"]["family_number"] for family in DECIDING}
    discriminators["P"]["max_cell_numbers"] = {family: results["families"][family]["discriminators"]["P"]["max_cell_number"] for family in DECIDING}
    discriminators["P"]["families_above_0_85"] = [family for family in DECIDING if results["families"][family]["discriminators"]["P"]["above_kill_level_0_85"]]
    discriminators["S"]["geometric_means"] = {family: results["families"][family]["discriminators"]["S"]["geometric_mean"] for family in DECIDING}
    discriminators["S"]["max_ratios"] = {family: results["families"][family]["discriminators"]["S"]["max_ratio"] for family in DECIDING}
    discriminators["A"]["max_error_ratios"] = {family: results["families"][family]["discriminators"]["A"]["max_error_ratio"] for family in DECIDING}
    discriminators["B"]["budget_exhausted"] = {family: {"S": results["families"][family]["discriminators"]["B"]["total_S"],
                                                         "N": results["families"][family]["discriminators"]["B"]["total_N"]} for family in DECIDING}
    v_family = family_verdicts("V")
    totals = {"completed_solves": 0, "failed_solves": 0, "independent_checks": 0, "violations": 0}
    max_ratio = 0.0
    for family in DECIDING:
        block = results["families"][family]["discriminators"]["V"]
        for key in totals:
            totals[key] += block["totals"][key]
        max_ratio = max(max_ratio, block["max_residual_over_threshold"])
    total_violations = sum(all_violations.values())
    v_verdict = verdict_of(list(v_family.values()))
    if total_violations > 0:
        v_verdict = "FAIL"
    discriminators["V"] = {"families": v_family, "deciding_grid": totals, "max_residual_over_threshold": max_ratio,
                           "violations_by_group_of_audited_adaptive_runs": all_violations, "violations": total_violations, "verdict": v_verdict}
    labels = {family: results["families"][family]["T"]["label"] for family in DECIDING}
    discriminators["T"] = {
        "labels": labels, "verdict": "reported only",
        "wall_speedup_at_matched_error": {family: results["families"][family]["T"]["wall"].get("speedup") for family in DECIDING},
        "wall_speedup_at_equal_tolerance": {family: results["families"][family]["T"]["speedup_equal_tolerance_geometric_mean"] for family in DECIDING},
    }
    for name, filename in (("G", "GOLDEN_DIFF.json"), ("R", "R_SUITES.json")):
        path = os.path.join(NODE, filename)
        if os.path.exists(path):
            with open(path, encoding="utf-8") as handle:
                data = json.load(handle)
            discriminators[name] = {"verdict": data.get("verdict"), "file": filename, "summary": data.get("summary")}
        else:
            discriminators[name] = {"verdict": "NOT_RUN", "file": None}
    results["discriminators"] = discriminators

    six = [discriminators[name]["verdict"] for name in ("D0", "V", "A", "P", "S", "B")]
    kill_reasons = []
    if total_violations > 0:
        kill_reasons.append(f"{total_violations} violations of V")
    if len(discriminators["P"]["families_above_0_85"]) >= 2:
        kill_reasons.append(f"P above 0.85 on {discriminators['P']['families_above_0_85']}")
    failed = [name for name in ("P", "S", "A", "B") if discriminators[name]["verdict"] == "FAIL"]
    not_pass = [name for name in ("D0", "V", "A", "P", "S", "B", "G", "R") if discriminators[name]["verdict"] != "PASS"]
    if kill_reasons:
        verdict, reason = "KILL", "; ".join(kill_reasons)
    elif failed and discriminators["V"]["verdict"] == "PASS":
        verdict, reason = "FAIL", f"{failed} fail with V intact: the adaptive policy stays opt-in and the default is not changed"
    elif not not_pass:
        verdict, reason = "PASS", "D0, V, A, P, S, B, G and R hold"
    else:
        verdict, reason = "NOT_PASS", f"not PASS: {({name: discriminators[name]['verdict'] for name in not_pass})}"
    results["verdict"] = {
        "frozen_rules": {"PASS": "D0, V, A, P, S, B, G and R hold", "FAIL": "P, S, A or B fails with V intact",
                         "KILL": "V is violated in any solve, or P is above 0.85 on two families"},
        "verdict": verdict, "reason": reason,
        "D0_V_A_P_S_B_all_hold": all(value == "PASS" for value in six),
        "step_5_condition_met": all(value == "PASS" for value in six),
    }
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("what", choices=["recording", "choice", "results"])
    args = parser.parse_args()
    if args.what == "recording":
        write_json(os.path.join(NODE, "RECORDING_A.json"), build_recording())
    elif args.what == "choice":
        choice = build_choice()
        write_json(os.path.join(NODE, "CHOICE.json"), choice)
        print("score", choice.get("score"), "chosen", choice.get("chosen"), "-", choice.get("reason"))
        print("checks:", choice["checks"])
    elif args.what == "results":
        results = build_results()
        write_json(os.path.join(NODE, "RESULTS.json"), results)
        print(json.dumps({name: block.get("verdict") for name, block in results["discriminators"].items()}))
        print(json.dumps(results["verdict"]))
        print("checks:", results["checks"])


if __name__ == "__main__":
    sys.exit(main())
