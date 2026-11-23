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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("what", choices=["recording"])
    args = parser.parse_args()
    if args.what == "recording":
        write_json(os.path.join(NODE, "RECORDING_A.json"), build_recording())


if __name__ == "__main__":
    sys.exit(main())
