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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("what", choices=["recording", "choice"])
    args = parser.parse_args()
    if args.what == "recording":
        write_json(os.path.join(NODE, "RECORDING_A.json"), build_recording())
    elif args.what == "choice":
        choice = build_choice()
        write_json(os.path.join(NODE, "CHOICE.json"), choice)
        print("score", choice.get("score"), "chosen", choice.get("chosen"), "-", choice.get("reason"))
        print("checks:", choice["checks"])


if __name__ == "__main__":
    sys.exit(main())
