#!/usr/bin/env python3
"""Loop node V28: builds RESULTS.json from the raw logs only and renders
REPORT_KO.md (Korean, for the owner) from RESULTS.json.

Inputs: PREDECLARED.json (frozen), PROTOCOL.json (protocol choices), the raw
logs under logs/ (cells, replay with the per-system logs *.systems.txt.gz,
ensemble, reproduction), the references and the recorded V25/V24 runs that
PROTOCOL.json names. Nothing is typed in but the level of T, the frozen 0.05 of
PREDECLARED.json (discriminators.T), kept as the exact fraction 1/20. The error
metric is imported from ../mf_stage_solve/parse_results.py (node V17), not
re-written. RESULTS.json carries no time stamp: it regenerates identically
from the same logs.

usage: python3 -I parse_results.py           writes RESULTS.json and REPORT_KO.md
       python3 -I parse_results.py --check   rebuilds both in memory and compares with the files
"""
import argparse
import glob
import gzip
import hashlib
import importlib.util
import json
import math
import os
import struct
import sys
from fractions import Fraction

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
LOOP = os.path.dirname(NODE)
LOGS = os.path.join(NODE, "logs")
V17 = os.path.join(LOOP, "mf_stage_solve")

_spec = importlib.util.spec_from_file_location("v17_parse_results", os.path.join(V17, "parse_results.py"))
assert _spec is not None and _spec.loader is not None, "mf_stage_solve/parse_results.py (node V17) is needed"
v17 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(v17)

# The level of T (PREDECLARED.json discriminators.T: "at least 0.05 ... fixed here").
T_LEVEL = Fraction(1, 20)
ARMS = ("S", "P1")
TRAJECTORIES = range(1, 9)
KERNELS = ("shipped", "P1")
FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d", "fhn3d", "cusp1d", "burgers2d", "bruss1d"]
RECORD_FIELDS = ("state_sha256", "attempts", "accepted", "rejected")


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def from_bits(text):
    return struct.unpack(">d", bytes.fromhex(text))[0]


def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def read_log(path, prefix="V28"):
    """Header, footer and tagged JSON lines of one log, or None."""
    if not os.path.exists(path):
        return None
    out = {"begin": None, "end": None, "tags": {}, "path": os.path.relpath(path, NODE)}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith(f"# {prefix}_LOG_BEGIN "):
                out["begin"] = json.loads(line[len(f"# {prefix}_LOG_BEGIN "):])
            elif line.startswith(f"# {prefix}_LOG_END "):
                out["end"] = json.loads(line[len(f"# {prefix}_LOG_END "):])
            elif line.startswith(f"{prefix}_") and " " in line:
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


def identity(store, log):
    begin = (log or {}).get("begin") or {}
    if begin.get("binary_sha256"):
        store["binaries"][begin["binary_sha256"]] = store["binaries"].get(begin["binary_sha256"], 0) + 1
    if begin.get("git_head"):
        key = f"{begin['git_head']} dirty_tracked={begin.get('git_dirty_tracked')}"
        store["git_heads"][key] = store["git_heads"].get(key, 0) + 1


def cell_name(cell):
    return f"c{cell['index']:02d}__{cell['family']}__{cell['rtol_label']}"


def ratio(a, b):
    if a is None or b is None or b == 0:
        return None
    return a / b


def geometric_mean(values):
    values = [v for v in values if v is not None and v > 0 and math.isfinite(v)]
    if not values:
        return None
    return math.exp(sum(math.log(v) for v in values) / len(values))


def summary_of(values):
    finite = [v for v in values if v is not None and math.isfinite(v)]
    return {"count": len(finite), "missing": len(values) - len(finite),
            "min": min(finite) if finite else None, "max": max(finite) if finite else None,
            "geometric_mean": geometric_mean(finite)}


# ---------------------------------------------------------------- cells

def check_cells(protocol, store):
    log = read_log(os.path.join(LOGS, "cells", "cells.log"))
    identity(store, log)
    rows = (log or {}).get("tags", {}).get("V28_CELL_JSON", [])
    mismatches = []
    for cell in protocol["cells"]:
        row = next((r for r in rows if r.get("cell") == cell["index"]), None)
        if row is None:
            mismatches.append({"cell": cell["index"], "reason": "not in the CELLS log"})
            continue
        for key, built in (("family", row.get("family")), ("n", row.get("n")), ("t_span", row.get("t_span")),
                           ("rtol", row.get("rtol")), ("atol", row.get("atol"))):
            if built != cell[key]:
                mismatches.append({"cell": cell["index"], "field": key, "protocol": cell[key], "built": built})
    return {"log": (log or {}).get("path"), "cells_built": len(rows), "mismatches": mismatches,
            "exit_code": ((log or {}).get("end") or {}).get("exit_code")}


# ---------------------------------------------------------------- replay: K and V

def kernel_class(code):
    return "solved" if code >= 0 else ("budget" if code == -1 else "other")


def recount_systemlog(path):
    """Recount every line of one per-system log."""
    out = {"attempts": 0, "systems": 0, "rebuild_failures": 0, "elsewhere_not_rebuilt": 0, "unparsed": 0,
           "solved": [0, 0], "budget": [0, 0], "other": [0, 0], "columns": [0, 0],
           "p1_budget_shipped_not": 0, "shipped_budget_p1_not": 0, "both_budget": 0,
           "violations": [0, 0], "reported_violations": [0, 0], "max_ratio": [0.0, 0.0],
           "p1_threshold_bits_differ": 0, "replay_differs_from_driver": 0,
           "driver": {"C": 0, "B": 0, "O": 0, "E": 0}, "disagreements": [], "k_systems": [],
           "violating_systems": []}
    h_of = {}
    raw = hashlib.sha256()
    with gzip.open(path, "rb") as handle:
        data = handle.read()
    raw.update(data)
    out["raw_sha256"] = raw.hexdigest()
    for line in data.decode("ascii").splitlines():
        fields = line.split(" ")
        try:
            if fields[0] == "A":
                out["attempts"] += 1
                h_of[int(fields[1])] = fields[3]
            elif fields[0] == "R":
                if fields[3] == "E":
                    out["elsewhere_not_rebuilt"] += 1
                else:
                    out["rebuild_failures"] += 1
            elif fields[0] == "Y":
                out["systems"] += 1
                attempt, stage, driver_code, driver_columns = int(fields[1]), int(fields[2]), fields[3], fields[4]
                threshold_bits = fields[6]
                threshold = from_bits(threshold_bits)
                codes = [int(fields[7]), int(fields[10])]
                residuals = [from_bits(fields[8]), from_bits(fields[11])]
                reported = [from_bits(fields[9]), from_bits(fields[12])]
                kernel_threshold_bits = fields[13]
                out["driver"][driver_code] += 1
                for k in (0, 1):
                    cls = kernel_class(codes[k])
                    out[cls][k] += 1
                    if cls == "solved":
                        out["columns"][k] += codes[k]
                        if not (math.isfinite(residuals[k]) and residuals[k] <= threshold):
                            out["violations"][k] += 1
                            if k == 1:
                                out["violating_systems"].append([attempt, stage, residuals[k], threshold])
                        if not (math.isfinite(reported[k]) and reported[k] <= threshold):
                            out["reported_violations"][k] += 1
                        if threshold > 0 and math.isfinite(residuals[k] / threshold):
                            out["max_ratio"][k] = max(out["max_ratio"][k], residuals[k] / threshold)
                if codes[1] >= 0 and kernel_threshold_bits != threshold_bits:
                    out["p1_threshold_bits_differ"] += 1
                shipped_budget, p1_budget = codes[0] == -1, codes[1] == -1
                out["p1_budget_shipped_not"] += int(p1_budget and not shipped_budget)
                out["shipped_budget_p1_not"] += int(shipped_budget and not p1_budget)
                out["both_budget"] += int(p1_budget and shipped_budget)
                entry = [attempt, stage, h_of.get(attempt), driver_code, codes[0], codes[1]]
                if kernel_class(codes[0]) != kernel_class(codes[1]):
                    out["disagreements"].append(entry)
                if p1_budget and not shipped_budget:
                    out["k_systems"].append(entry)
                out.setdefault("_own", []).append((driver_code, driver_columns, codes))
            else:
                out["unparsed"] += 1
        except (IndexError, ValueError, KeyError):
            out["unparsed"] += 1
    return out


def evaluate_replay_run(protocol_cell, arm, store):
    name = f"{cell_name(protocol_cell)}__{arm}"
    path = os.path.join(LOGS, "replay", name + ".log")
    log = read_log(path)
    identity(store, log)
    row = {"log": os.path.relpath(path, NODE), "cell": protocol_cell["index"], "family": protocol_cell["family"],
           "rtol": protocol_cell["rtol_label"], "trajectory": arm, "coverage_issues": []}
    if log is None:
        row["coverage_issues"].append("the log is missing")
        return row, None
    record = first(log, "V28_SHADOW_JSON")
    end = log["end"] or {}
    row["exit_code"] = end.get("exit_code")
    if end.get("exit_code") != 0 or end.get("timed_out"):
        row["coverage_issues"].append(f"exit code {end.get('exit_code')}, timed out {end.get('timed_out')}")
    if record is None:
        row["coverage_issues"].append("no V28_SHADOW_JSON")
        return row, None
    if record.get("cell") != protocol_cell["index"] or record.get("arm") != arm:
        row["coverage_issues"].append("the record names another cell or arm")
    if not (record.get("completed") and record.get("success")):
        row["coverage_issues"].append(f"the driver did not run to t1: {record.get('error')}")
    if record.get("write_error"):
        row["coverage_issues"].append(f"write error {record.get('write_error')}")
    stats = record.get("policy_statistics") or {}
    for key in ("success", "attempts", "accepted", "rejected", "state_sha256", "systems", "replay_differs_from_driver",
                "rebuild_failures", "attempts_failed_elsewhere", "max_ratio"):
        row[key] = record.get(key)
    row["driver_budget_exhausted_solves"] = stats.get("budget_exhausted_solves")
    row["driver_other_linear_failures"] = stats.get("other_linear_failures")
    systemlog = end.get("systemlog")
    if not systemlog:
        row["coverage_issues"].append("no per-system log in the footer")
        return row, None
    gz = os.path.join(LOGS, "replay", systemlog["file"])
    if not os.path.exists(gz):
        row["coverage_issues"].append("the per-system log is missing")
        return row, None
    if sha256_file(gz) != systemlog.get("gz_sha256"):
        row["coverage_issues"].append("the gz sha256 of the per-system log differs from the footer")
    recount = recount_systemlog(gz)
    if recount["raw_sha256"] != systemlog.get("raw_sha256"):
        row["coverage_issues"].append("the raw sha256 of the per-system log differs from the footer")
    own_kernel = 0 if arm == "S" else 1
    differs = 0
    for driver_code, driver_columns, codes in recount.pop("_own", []):
        own = codes[own_kernel]
        if driver_code == "C":
            differs += int(own != int(driver_columns))
        elif driver_code == "B":
            differs += int(own != -1)
        else:
            differs += int(own != -2)
    recount["replay_differs_from_driver"] = differs
    for key in ("attempts", "systems", "solved", "budget", "other", "p1_budget_shipped_not", "shipped_budget_p1_not",
                "both_budget", "violations", "reported_violations", "replay_differs_from_driver", "rebuild_failures"):
        summary_key = "attempts_observed" if key == "attempts" else key
        if recount[key] != record.get(summary_key):
            row["coverage_issues"].append(f"recount {key} {recount[key]} != summary {record.get(summary_key)}")
    if recount["attempts"] != record.get("attempts"):
        row["coverage_issues"].append(f"attempt lines {recount['attempts']} != attempts {record.get('attempts')}")
    if recount["driver"]["B"] != stats.get("budget_exhausted_solves"):
        row["coverage_issues"].append(f"budget failures replayed {recount['driver']['B']} != driver statistics "
                                      f"{stats.get('budget_exhausted_solves')}")
    if recount["unparsed"]:
        row["coverage_issues"].append(f"{recount['unparsed']} unparsed lines")
    if recount["rebuild_failures"]:
        row["coverage_issues"].append(f"{recount['rebuild_failures']} systems not rebuilt")
    if differs:
        row["coverage_issues"].append(f"the driver's own kernel differs from the driver on {differs} systems")
    row["recount"] = {key: recount[key] for key in (
        "attempts", "systems", "driver", "solved", "budget", "other", "columns", "p1_budget_shipped_not",
        "shipped_budget_p1_not", "both_budget", "violations", "reported_violations", "max_ratio",
        "p1_threshold_bits_differ", "replay_differs_from_driver", "rebuild_failures", "elsewhere_not_rebuilt")}
    row["systemlog"] = {"file": os.path.relpath(gz, NODE), "gz_sha256": systemlog.get("gz_sha256"),
                        "raw_sha256": systemlog.get("raw_sha256")}
    return row, recount


def evaluate_k_and_v(protocol, store):
    runs, k_list, disagreements, violating = [], [], [], []
    totals = {"systems": 0, "attempts": 0, "solved": [0, 0], "budget": [0, 0], "other": [0, 0], "columns": [0, 0],
              "p1_budget_shipped_not": 0, "shipped_budget_p1_not": 0, "both_budget": 0, "violations": [0, 0],
              "reported_violations": [0, 0], "max_ratio": [0.0, 0.0], "p1_threshold_bits_differ": 0}
    by_family = {}
    coverage = []
    for cell in protocol["cells"]:
        for arm in ARMS:
            row, recount = evaluate_replay_run(cell, arm, store)
            runs.append(row)
            for issue in row["coverage_issues"]:
                coverage.append(f"{row['log']}: {issue}")
            if recount is None:
                continue
            key = f"{cell['family']} trajectory {arm}"
            block = by_family.setdefault(key, {"systems": 0, "budget": [0, 0], "p1_budget_shipped_not": 0,
                                               "shipped_budget_p1_not": 0, "both_budget": 0, "solved": [0, 0],
                                               "columns": [0, 0]})
            for target in (totals, block):
                for name in ("systems", "p1_budget_shipped_not", "shipped_budget_p1_not", "both_budget"):
                    target[name] += recount[name]
                for name in ("solved", "budget", "columns"):
                    target[name] = [a + b for a, b in zip(target[name], recount[name])]
            totals["attempts"] += recount["attempts"]
            for name in ("other", "violations", "reported_violations"):
                totals[name] = [a + b for a, b in zip(totals[name], recount[name])]
            totals["max_ratio"] = [max(a, b) for a, b in zip(totals["max_ratio"], recount["max_ratio"])]
            totals["p1_threshold_bits_differ"] += recount["p1_threshold_bits_differ"]
            prefix = {"cell": cell["index"], "family": cell["family"], "rtol": cell["rtol_label"], "trajectory": arm}
            columns = ["attempt", "stage", "h_bits", "driver", "shipped", "P1"]
            for entry in recount["k_systems"]:
                k_list.append({**prefix, **dict(zip(columns, entry))})
            for entry in recount["disagreements"]:
                disagreements.append({**prefix, **dict(zip(columns, entry))})
            for attempt, stage, residual, threshold in recount["violating_systems"]:
                violating.append({**prefix, "attempt": attempt, "stage": stage, "residual": residual,
                                  "threshold": threshold})
    k_verdict = "FAIL" if k_list else ("NOT_DECIDABLE" if coverage else "PASS")
    v_violations = totals["violations"][1] + totals["reported_violations"][1]
    v_verdict = "FAIL" if v_violations else ("NOT_DECIDABLE" if coverage else "PASS")
    k = {
        "frozen": "there is no replayed stage system on which P1 exhausts the Krylov budget and S does not (subset), "
                  "in any cell, along either trajectory",
        "verdict": k_verdict,
        "replay_runs": len(runs),
        "k_systems_count": len(k_list),
        "k_systems": k_list,
        "systems_where_the_kernels_differ_count": len(disagreements),
        "systems_where_the_kernels_differ": disagreements,
        "systems_where_the_kernels_differ_columns": "cell, trajectory, attempt, stage, h bits (the attempt's step), "
                                                    "the driver's own outcome (C completed, B budget, O other linear "
                                                    "error, E other error), shipped and P1 codes (Arnoldi columns of "
                                                    "an iterate, -1 budget, -2 other error)",
        "totals_kernel_order": list(KERNELS),
        "totals": totals,
        "totals_by_family_and_trajectory": by_family,
        "coverage_issues": coverage,
    }
    v = {
        "frozen": "0 violations of the stage threshold by a completed P1 solve in the replay",
        "verdict": v_verdict,
        "p1_completed_solves": totals["solved"][1],
        "violations_independent_residual": totals["violations"][1],
        "violations_reported_residual": totals["reported_violations"][1],
        "largest_independent_residual_over_threshold": totals["max_ratio"][1],
        "p1_threshold_bits_differ_from_the_target_threshold": totals["p1_threshold_bits_differ"],
        "violating_systems": violating,
        "shipped_kernel_reported_not_a_rule": {
            "completed_solves": totals["solved"][0], "violations_independent_residual": totals["violations"][0],
            "violations_reported_residual": totals["reported_violations"][0],
            "largest_independent_residual_over_threshold": totals["max_ratio"][0]},
        "coverage_issues": coverage,
    }
    return k, v, runs


# ---------------------------------------------------------------- ensemble: T

def collect_run(phase, name, store):
    path = os.path.join(LOGS, phase, name + ".log")
    log = read_log(path)
    identity(store, log)
    if log is None:
        return None
    record = first(log, "V28_RUN_JSON")
    state = first(log, "V28_STATE_JSON")
    end = log["end"] or {}
    return {"log": os.path.relpath(path, NODE), "record": record, "state_hex": state["state_hex"] if state else None,
            "exit_code": end.get("exit_code"), "timed_out": end.get("timed_out")}


def load_references(protocol):
    out = {}
    for cell in protocol["cells"]:
        relative = cell["reference_for_the_error_report"]
        if relative in out:
            continue
        path = os.path.join(LOOP, relative)
        reference = load(path)
        out[relative] = {"state": v17.unhex(reference["state_hex"]), "t_span": reference["t_span"],
                         "sha256": sha256_file(path), "u_ref": reference.get("uncertainty_max_scaled_1_plus_abs")}
    return out


def run_summary(run, reference, protocol_cell):
    if run is None:
        return {"present": False}
    record = run["record"] or {}
    stats = record.get("policy_statistics")
    out = {"present": True, "log": run["log"], "exit_code": run["exit_code"], "timed_out": run["timed_out"],
           "completed": record.get("completed"), "success": record.get("success"), "error": record.get("error"),
           "attempts": record.get("attempts"), "accepted": record.get("accepted"), "rejected": record.get("rejected"),
           "budget_exhausted_solves": (stats or {}).get("budget_exhausted_solves"), "has_statistics": stats is not None,
           "start_sha256": record.get("start_sha256"), "y0_sha256": record.get("y0_sha256"),
           "state_sha256": record.get("state_sha256"), "j": record.get("j"),
           "cell_ok": record.get("cell") == protocol_cell["index"] and record.get("n") == protocol_cell["n"]
           and record.get("t_span") == protocol_cell["t_span"] and record.get("rtol") == protocol_cell["rtol"]}
    out["E"] = None
    if run["state_hex"] and record.get("completed") and record.get("success"):
        state = v17.unhex(run["state_hex"])
        if len(state) == len(reference["state"]) and reference["t_span"] == protocol_cell["t_span"]:
            out["E"] = v17.error_metrics(state, reference["state"])["E"]
    return out


def upper_tail(w, n):
    """P(X >= w), X ~ Bin(n, 1/2), exact."""
    if n == 0:
        return Fraction(1)
    return Fraction(sum(math.comb(n, k) for k in range(w, n + 1)), 2 ** n)


def evaluate_t(protocol, references, store):
    pairs, coverage = [], []
    for cell in protocol["cells"]:
        reference = references[cell["reference_for_the_error_report"]]
        for j in TRAJECTORIES:
            runs = {arm: run_summary(collect_run("ensemble", f"{cell_name(cell)}__j{j}__{arm}", store), reference, cell)
                    for arm in ARMS}
            pair = {"cell": cell["index"], "family": cell["family"], "rtol": cell["rtol_label"], "j": j,
                    "S": runs["S"], "P1": runs["P1"], "d": None, "issues": []}
            for arm in ARMS:
                run = runs[arm]
                if not run["present"]:
                    pair["issues"].append(f"{arm}: the run is missing")
                    continue
                if run["timed_out"] or run["exit_code"] != 0:
                    pair["issues"].append(f"{arm}: exit code {run['exit_code']}, timed out {run['timed_out']}")
                if not run["has_statistics"]:
                    pair["issues"].append(f"{arm}: no policy_statistics")
                if not run["cell_ok"] or run["j"] != j:
                    pair["issues"].append(f"{arm}: the record is not this cell and trajectory")
                if run["start_sha256"] == run["y0_sha256"]:
                    pair["issues"].append(f"{arm}: the start is not perturbed")
            if runs["S"]["present"] and runs["P1"]["present"] and runs["S"]["start_sha256"] != runs["P1"]["start_sha256"]:
                pair["issues"].append("S and P1 do not start from the same state")
            if not pair["issues"]:
                pair["d"] = runs["P1"]["budget_exhausted_solves"] - runs["S"]["budget_exhausted_solves"]
            pair["attempts_ratio"] = ratio(runs["P1"].get("attempts"), runs["S"].get("attempts"))
            pair["error_ratio"] = ratio(runs["P1"].get("E"), runs["S"].get("E"))
            for issue in pair["issues"]:
                coverage.append(f"cell {cell['index']} j {j}: {issue}")
            pairs.append(pair)
    ds = [pair["d"] for pair in pairs if pair["d"] is not None]
    w = sum(1 for d in ds if d > 0)
    n = sum(1 for d in ds if d != 0)
    p = upper_tail(w, n)
    verdict = "NOT_DECIDABLE" if coverage or len(ds) != 320 else ("PASS" if p >= T_LEVEL else "FAIL")
    histogram = {}
    for d in ds:
        histogram[str(d)] = histogram.get(str(d), 0) + 1
    per_cell = []
    for cell in protocol["cells"]:
        cp = [pair for pair in pairs if pair["cell"] == cell["index"]]
        errors = {arm: [pair[arm].get("E") for pair in cp if pair[arm].get("E") is not None] for arm in ARMS}
        per_cell.append({
            "cell": cell["index"], "family": cell["family"], "rtol": cell["rtol_label"],
            "d_by_j": [pair["d"] for pair in cp],
            "budget_by_j": {arm: [pair[arm].get("budget_exhausted_solves") for pair in cp] for arm in ARMS},
            "attempts_by_j": {arm: [pair[arm].get("attempts") for pair in cp] for arm in ARMS},
            "E_min_max": {arm: [min(errors[arm]), max(errors[arm])] if errors[arm] else None for arm in ARMS},
        })
    by_family = {}
    for family in FAMILIES:
        fam = [pair for pair in pairs if pair["family"] == family]
        fds = [pair["d"] for pair in fam if pair["d"] is not None]
        by_family[family] = {
            "pairs": len(fam), "W": sum(1 for d in fds if d > 0), "n": sum(1 for d in fds if d != 0),
            "pairs_with_d_below_0": sum(1 for d in fds if d < 0), "sum_d": sum(fds),
            "budget_exhausted_total": {arm: sum((pair[arm].get("budget_exhausted_solves") or 0) for pair in fam)
                                       for arm in ARMS},
            "attempts_total": {arm: sum((pair[arm].get("attempts") or 0) for pair in fam) for arm in ARMS},
            "attempts_ratio_per_pair": summary_of([pair["attempts_ratio"] for pair in fam]),
            "error_ratio_per_pair": summary_of([pair["error_ratio"] for pair in fam]),
        }
    return {
        "frozen": "over the 320 pairs, d = exhausting solves of P1 minus those of S; W = number of pairs with d > 0, "
                  "n = number of pairs with d != 0; T holds iff P(X >= W) >= 0.05, X ~ Bin(n, 1/2) (n = 0: holds)",
        "verdict": verdict,
        "pairs": len(pairs), "pairs_with_d": len(ds), "W": w, "n": n,
        "pairs_with_d_below_0": sum(1 for d in ds if d < 0), "pairs_with_d_equal_0": sum(1 for d in ds if d == 0),
        "P_X_ge_W_exact": f"{p.numerator}/{p.denominator}", "P_X_ge_W": float(p), "level": f"{T_LEVEL.numerator}/{T_LEVEL.denominator}",
        "d_histogram": dict(sorted(histogram.items(), key=lambda item: int(item[0]))),
        "by_family_reported_not_a_rule": by_family,
        "per_cell_reported_not_a_rule": per_cell,
        "coverage_issues": coverage,
        "per_pair": [{key: pair[key] for key in ("cell", "family", "rtol", "j", "d", "attempts_ratio", "error_ratio")}
                     | {f"{arm}_{key}": pair[arm].get(key) for arm in ARMS
                        for key in ("budget_exhausted_solves", "attempts", "accepted", "rejected", "completed",
                                    "success", "E", "state_sha256", "start_sha256", "log")}
                     for pair in pairs],
        "per_pair_note": "attempts = accepted + rejected of the run; E = max_i |y_i(t1) - ref_i| / (1 + |ref_i|) "
                         "against the unperturbed reference of the family (not corrected for the perturbation of "
                         "the start, relative 1e-8); attempts_ratio and error_ratio are P1 / S",
    }


# ---------------------------------------------------------------- reproduction (reported)

def recorded_run(relative):
    path = os.path.join(LOOP, relative)
    prefix = "V25" if relative.startswith("mf_p1_confirm") else "V24"
    log = read_log(path, prefix)
    record = first(log, f"{prefix}_RUN_JSON")
    if record is None:
        return None
    out = {key: record.get(key) for key in RECORD_FIELDS}
    stats = record.get("policy_statistics")
    out["budget_exhausted_solves"] = stats.get("budget_exhausted_solves") if stats else None
    return out


def evaluate_reproduction(protocol, replay_runs, store):
    rows, mismatches = [], []
    replay = {(row["cell"], row["trajectory"]): row for row in replay_runs}
    for cell in protocol["cells"]:
        for arm in ARMS:
            recorded = recorded_run(cell["recorded_runs_for_the_reproduction_check"][arm])
            run = collect_run("reproduction", f"{cell_name(cell)}__j0__{arm}", store)
            record = (run or {}).get("record") or {}
            mine = {key: record.get(key) for key in RECORD_FIELDS}
            mine["budget_exhausted_solves"] = (record.get("policy_statistics") or {}).get("budget_exhausted_solves")
            shadow = replay.get((cell["index"], arm)) or {}
            from_shadow = {key: shadow.get(key) for key in RECORD_FIELDS}
            from_shadow["budget_exhausted_solves"] = shadow.get("driver_budget_exhausted_solves")
            row = {"cell": cell["index"], "family": cell["family"], "rtol": cell["rtol_label"], "arm": arm,
                   "recorded_log": cell["recorded_runs_for_the_reproduction_check"][arm], "recorded": recorded,
                   "run_j0": mine if run else None, "replay_driver": from_shadow, "differences": []}
            if recorded is None:
                row["differences"].append("no recorded run")
            for label, values in (("run_j0", mine if run else None), ("replay_driver", from_shadow)):
                if values is None:
                    row["differences"].append(f"{label}: missing")
                    continue
                for key in (*RECORD_FIELDS, "budget_exhausted_solves"):
                    if recorded is None or recorded.get(key) is None:
                        continue
                    if values.get(key) != recorded.get(key):
                        row["differences"].append(f"{label}.{key}: {values.get(key)} != recorded {recorded.get(key)}")
            if row["differences"]:
                mismatches.append(row)
            rows.append(row)
    return {"what": "reported, decides nothing (PROTOCOL.json reproduction_check)", "rows": len(rows),
            "rows_with_differences": len(mismatches), "differences": mismatches,
            "fields": [*RECORD_FIELDS, "budget_exhausted_solves (where recorded)"], "per_cell": rows}


# ---------------------------------------------------------------- totals (reported)

def totals_report(protocol, k_runs, t_result, reproduction):
    out = {}
    for family in FAMILIES:
        block = {}
        for arm in ARMS:
            ens = [pair[f"{arm}_budget_exhausted_solves"] for pair in t_result["per_pair"] if pair["family"] == family]
            unperturbed = [row["driver_budget_exhausted_solves"] for row in k_runs
                           if row["family"] == family and row["trajectory"] == arm]
            j0 = [(row["run_j0"] or {}).get("budget_exhausted_solves") for row in reproduction["per_cell"]
                  if row["family"] == family and row["arm"] == arm]
            block[arm] = {
                "ensemble_j1_to_8": sum(v for v in ens if v is not None),
                "ensemble_runs": len(ens),
                "unperturbed_replay_driver": sum(v for v in unperturbed if v is not None),
                "unperturbed_j0_run": sum(v for v in j0 if v is not None),
            }
        out[family] = block
    return out


# ---------------------------------------------------------------- results

def build_results():
    protocol = load(os.path.join(NODE, "PROTOCOL.json"))
    store = {"binaries": {}, "git_heads": {}}
    cells_check = check_cells(protocol, store)
    references = load_references(protocol)
    k, v, k_runs = evaluate_k_and_v(protocol, store)
    t = evaluate_t(protocol, references, store)
    reproduction = evaluate_reproduction(protocol, k_runs, store)
    verdicts = {"K": k["verdict"], "V": v["verdict"], "T": t["verdict"]}
    if verdicts["V"] == "FAIL":
        verdict = "KILL"
    elif all(value == "PASS" for value in verdicts.values()):
        verdict = "SWITCH"
    elif verdicts["V"] == "PASS" and "FAIL" in (verdicts["K"], verdicts["T"]):
        verdict = "KEEP"
    else:
        verdict = "NOT_DECIDABLE"
    results = {
        "schema": "vigilode.loop-l1.v28.results.v1",
        "node": "V28_MF_P1_BUDGET_FOLLOWUP",
        "predeclared_sha256": sha256_file(os.path.join(NODE, "PREDECLARED.json")),
        "protocol_sha256": sha256_file(os.path.join(NODE, "PROTOCOL.json")),
        "protocol_binary_sha256": protocol["binary"]["sha256"],
        "binaries_of_the_logs": store["binaries"],
        "git_heads_of_the_logs": store["git_heads"],
        "cells_check": cells_check,
        "references": {key: {"sha256": value["sha256"], "t_span": value["t_span"], "u_ref": value["u_ref"]}
                       for key, value in references.items()},
        "discriminators": {"K": k, "V": v, "T": t},
        "reports_without_rule": {
            "budget_exhausted_solves_per_family_and_arm": totals_report(protocol, k_runs, t, reproduction),
            "attempts_ratio_and_error_ratio_per_pair": "discriminators.T.per_pair and "
                                                       "discriminators.T.by_family_reported_not_a_rule",
            "replay_runs": k_runs,
        },
        "reproduction_check": reproduction,
        "verdict": {
            "discriminators": verdicts,
            "frozen_verdict": verdict,
            "step_6_D0_G_R": "run after a switch (step_6)" if verdict == "SWITCH" else
                             "NOT_RUN: the frozen verdict is not SWITCH; no default and no protected file changed",
            "rule": "SWITCH iff K, V and T hold; KEEP iff K or T fails and V holds; KILL iff V fails "
                    "(PREDECLARED.json verdicts); NOT_DECIDABLE otherwise (PROTOCOL.json verdict_computation)",
        },
    }
    step6 = os.path.join(NODE, "STEP6.json")
    if os.path.exists(step6):
        results["step_6"] = load(step6)
    return v17.clean(results)


def results_text(results):
    return json.dumps(results, indent=1, ensure_ascii=False, allow_nan=False) + "\n"


# ---------------------------------------------------------------- report (Korean)

def fmt(value, digits=4):
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.{digits}g}"
    return str(value)


def report_text(results):
    d = results["discriminators"]
    k, v, t = d["K"], d["V"], d["T"]
    verdict = results["verdict"]
    lines = [
        "# V28: 행렬 없는 단계 풀이 P1의 규칙 B 단독 재시험 (OD-L1-23, 부록 A9)",
        "",
        "이 보고서는 `parse_results.py`가 `RESULTS.json`에서 만든 것입니다. 모든 수치는 실행된 원시 로그에서 계산되었습니다.",
        "",
        "**정정 고지 (2026-11-25, 독립 리뷰 `REVIEW.json`, 판정 accept_with_corrections).** 리뷰어가 원시 로그를 자기 "
        "스크립트로 다시 세어 동결 판정 KEEP(K PASS, V PASS, T FAIL)을 같게 얻었습니다. 아래 다섯 가지를 바로잡습니다. "
        "아래 수치는 리뷰의 재계산입니다.",
        "",
        "1. K와 V의 '범위 문제 0건'은 재생할 계가 있는 단계에 대한 말입니다. cusp1d에서 시도를 끝낸 단계 69개(S 33, P1 36)는 "
        "단계 우변이 유한하지 않아 선형 풀이가 시도되지 않았고, 재생할 계가 없었습니다. 이들은 해시로 고정된 측정 코드에 따라 "
        "별도 부류(elsewhere, 드라이버의 nonfinite_step_failures와 같음)로 세며 재구성 실패가 아닙니다. K와 V는 그대로입니다.",
        "2. 기록된 '엄격 JSON' 검사(`logs/checks/strict_json.log`)는 PREDECLARED.json, PROTOCOL.json, RESULTS.json 세 파일만 "
        "다뤘습니다. CHECKS.json은 검사 뒤에 쓰였고, 리뷰가 따로 엄격 JSON임을 확인했습니다.",
        "3. 노드 디렉터리 '41 MB'에는 추적되지 않는 `.mypy_cache`(9.0 MB)가 들어 있습니다. 커밋된 파일은 약 31.6 MB입니다"
        "(ensemble 23 MB, replay 5.6 MB, reproduction 2.9 MB).",
        "4. 빌더 작업 트리의 읽기 전용 경로(`mf_p1_confirm/`, `mf_stage_solve/`)에 git이 무시하는 `.mypy_cache`가 생겼습니다"
        "(도구 hook으로 보임). 커밋되지 않았고 결과에 영향이 없습니다.",
        "5. 실패한 첫 plumbing 실행(개발 인스턴스 dev_cusp1d, 유한하지 않은 단계 우변)의 출력은 보존되지 않았습니다. "
        "프로토콜 커밋 전이었고 결정 셀은 관여하지 않았습니다. 내용은 `NOTES.txt` 1단계에 있습니다.",
        "",
        "## 결론",
        "",
        f"- 동결된 판정: **{verdict['frozen_verdict']}** (K {verdict['discriminators']['K']}, "
        f"V {verdict['discriminators']['V']}, T {verdict['discriminators']['T']})",
        "- 규칙: K, V, T가 모두 성립하면 SWITCH(P1을 기본값으로), K 또는 T가 실패하고 V가 성립하면 KEEP(출하 정책 S 유지), "
        "V가 실패하면 KILL.",
    ]
    if verdict["frozen_verdict"] == "SWITCH":
        lines.append("- SWITCH이면 V25의 6단계(D0, G, R)가 이어집니다. 그 결과는 아래 6단계 절에 있습니다.")
    elif verdict["frozen_verdict"] == "KEEP":
        lines.append("- 출하 정책 S가 기본값으로 남고 P1은 선택 사항(opt-in)으로 남습니다. 어떤 기본값도 바뀌지 않았습니다.")
        lines.append("- SWITCH가 아니므로 V25의 6단계(D0, G, R)는 실행하지 않았습니다 (NOT_RUN). 보호된 파일은 하나도 편집하지 않았습니다.")
    elif verdict["frozen_verdict"] == "KILL":
        lines.append("- 기본값은 바뀌지 않습니다. P1의 문서에 위반 사실을 적어야 합니다.")
    else:
        lines.append("- 판정할 수 없습니다. 기본값은 바뀌지 않았습니다. 장애 사항은 아래에 있습니다.")
    lines += [
        "",
        "## 무엇을 시험했나",
        "",
        "- 셀: V25의 결정 셀 20개(ac3d, robertson_rd1d, schnakenberg2d, porous2d)와 V24의 결정 셀 20개(fhn3d, cusp1d, "
        "burgers2d, bruss1d), 각 가족마다 rtol = atol = 1e-3 ... 1e-7, 모두 40개. 생성기, 크기, 구간, 허용오차는 두 노드의 프로토콜 그대로입니다.",
        "- K (같은 단계 계의 재풀이): 섭동하지 않은 S 실행과 P1 실행의 모든 단계 계를 출하 커널과 P1 커널로 0에서 다시 풀어, "
        "P1이 크릴로프 예산(200 열)을 다 쓰고 S는 그렇지 않은 계가 하나라도 있는지 셉니다.",
        "- V: 재풀이에서 P1이 돌려준 모든 반복값의 참 잔차를 프로그램 자체의 JVP로 다시 계산해 단계 문턱값과 비교합니다.",
        "- T (섭동 궤적): 셀마다 j = 1..8, 시작 상태를 y0_i (1 + 1e-8 xi_ij)로 섭동(xi는 (셀, j, i)로 시드한 splitmix64), "
        "같은 섭동 상태에서 S와 P1을 실행, 쌍마다 d = (P1의 예산 소진 풀이 수) - (S의 수). W = d > 0인 쌍 수, n = d != 0인 쌍 수, "
        "P(X >= W) (X ~ Bin(n, 1/2))가 0.05 이상이면 성립.",
        "",
        "## K: 같은 단계 계에서 어느 커널이 예산을 다 쓰는가",
        "",
        f"- 재풀이 실행 {k['replay_runs']}개, 재풀이한 단계 계 {k['totals']['systems']}개 (시도 {k['totals']['attempts']}개).",
        f"- P1이 예산을 다 쓰고 S는 아닌 계: **{k['k_systems_count']}개** -> K {k['verdict']}.",
        f"- S가 예산을 다 쓰고 P1은 아닌 계: {k['totals']['shipped_budget_p1_not']}개; 둘 다 예산을 다 쓴 계: {k['totals']['both_budget']}개.",
        f"- 두 커널의 결과 종류(풀림/예산/기타)가 다른 계: {k['systems_where_the_kernels_differ_count']}개 "
        "(모두 `RESULTS.json`의 discriminators.K.systems_where_the_kernels_differ에 나열).",
        f"- 커널별 [S, P1]: 풀림 {k['totals']['solved']}, 예산 소진 {k['totals']['budget']}, 기타 오류 {k['totals']['other']}, "
        f"풀린 계의 아르놀디 열 합 {k['totals']['columns']}.",
        f"- 범위 점검 문제: {len(k['coverage_issues'])}건.",
        "",
        "| 가족, 궤적 | 계 | 예산 소진 [S, P1] | P1만 소진 | S만 소진 | 둘 다 |",
        "|---|---|---|---|---|---|",
    ]
    for key, block in k["totals_by_family_and_trajectory"].items():
        label = key.replace(" trajectory ", ", 궤적 ")
        lines.append(f"| {label} | {block['systems']} | {block['budget']} | {block['p1_budget_shipped_not']} | "
                     f"{block['shipped_budget_p1_not']} | {block['both_budget']} |")
    lines += [
        "",
        "## V: 재풀이에서 P1 반복값의 문턱값 위반",
        "",
        f"- P1이 돌려준 반복값 {v['p1_completed_solves']}개, 독립 잔차 위반 **{v['violations_independent_residual']}개**, "
        f"커널 보고 잔차 위반 {v['violations_reported_residual']}개 -> V {v['verdict']}.",
        f"- 가장 큰 (독립 잔차 / 문턱값): {fmt(v['largest_independent_residual_over_threshold'], 6)}; "
        f"커널 문턱값과 프로그램 문턱값의 비트가 다른 풀이: {v['p1_threshold_bits_differ_from_the_target_threshold']}개.",
        f"- 참고(규칙 없음), 출하 커널: 반복값 {v['shipped_kernel_reported_not_a_rule']['completed_solves']}개, 위반 "
        f"{v['shipped_kernel_reported_not_a_rule']['violations_independent_residual']}개, 최대 비 "
        f"{fmt(v['shipped_kernel_reported_not_a_rule']['largest_independent_residual_over_threshold'], 6)}.",
        "",
        "## T: 섭동 궤적 320쌍",
        "",
        f"- d가 계산된 쌍 {t['pairs_with_d']}개 / {t['pairs']}개; W = {t['W']}, n = {t['n']}, d < 0인 쌍 {t['pairs_with_d_below_0']}개, "
        f"d = 0인 쌍 {t['pairs_with_d_equal_0']}개.",
        f"- P(X >= W) = {t['P_X_ge_W_exact']} = {fmt(t['P_X_ge_W'], 6)} (수준 {t['level']}) -> T {t['verdict']}.",
        f"- d 분포: {json.dumps(t['d_histogram'])}.",
        "",
        "| 가족 | 쌍 | W | n | d<0 | d 합 | 예산 소진 합 S / P1 | 시도 비 P1/S 기하평균 (최소-최대) | 오차 비 P1/S 기하평균 (최소-최대) |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for family, block in t["by_family_reported_not_a_rule"].items():
        a, e = block["attempts_ratio_per_pair"], block["error_ratio_per_pair"]
        lines.append(f"| {family} | {block['pairs']} | {block['W']} | {block['n']} | {block['pairs_with_d_below_0']} | "
                     f"{block['sum_d']} | {block['budget_exhausted_total']['S']} / {block['budget_exhausted_total']['P1']} | "
                     f"{fmt(a['geometric_mean'])} ({fmt(a['min'])}-{fmt(a['max'])}) | "
                     f"{fmt(e['geometric_mean'])} ({fmt(e['min'])}-{fmt(e['max'])}) |")
    lines += [
        "",
        "시도 비와 오차 비는 규칙 없는 보고입니다. 오차는 섭동하지 않은 기준해에 대한 V17 지표이며, 시작 상태의 섭동(상대 1e-8)은 기준해에서 빼지 않았습니다.",
        "",
        "### 예산 소진 풀이가 한 번이라도 있었던 셀 (j = 1..8 순서)",
        "",
        "| 셀 | 가족 | rtol | d (P1 - S) | S 소진 수 | P1 소진 수 | S 시도 | P1 시도 | S 오차 범위 | P1 오차 범위 |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    exhausting_cells = [row for row in t["per_cell_reported_not_a_rule"]
                        if any(row["budget_by_j"]["S"]) or any(row["budget_by_j"]["P1"])]
    for row in exhausting_cells:
        e_s, e_p = row["E_min_max"]["S"], row["E_min_max"]["P1"]
        lines.append(
            f"| {row['cell']} | {row['family']} | {row['rtol']} | {row['d_by_j']} | {row['budget_by_j']['S']} | "
            f"{row['budget_by_j']['P1']} | {row['attempts_by_j']['S']} | {row['attempts_by_j']['P1']} | "
            f"{fmt(e_s[0], 3) if e_s else '-'} ~ {fmt(e_s[1], 3) if e_s else '-'} | "
            f"{fmt(e_p[0], 3) if e_p else '-'} ~ {fmt(e_p[1], 3) if e_p else '-'} |")
    families_with_d = [family for family, block in t["by_family_reported_not_a_rule"].items() if block["n"]]
    families_equal = [family for family, block in t["by_family_reported_not_a_rule"].items()
                      if not block["n"] and block["budget_exhausted_total"]["S"]]
    families_none = [family for family, block in t["by_family_reported_not_a_rule"].items()
                     if not block["budget_exhausted_total"]["S"] and not block["budget_exhausted_total"]["P1"]]
    lines += [
        "",
        "### 관찰 (규칙 없음)",
        "",
        f"- 같은 단계 계에서는 두 커널이 정확히 같은 계에서 예산을 다 썼습니다 (둘 다 소진 {k['totals']['both_budget']}개, "
        f"한쪽만 소진 {k['totals']['p1_budget_shipped_not'] + k['totals']['shipped_budget_p1_not']}개).",
        f"- 섭동 궤적에서 d가 0이 아닌 쌍이 있는 가족: {', '.join(families_with_d) or '없음'}. "
        f"예산 소진은 있으나 모든 쌍에서 d = 0인 가족: {', '.join(families_equal) or '없음'}. "
        f"예산 소진이 한 번도 없는 가족: {', '.join(families_none) or '없음'}.",
        "- 따라서 P1의 초과 소진은 같은 계를 푸는 커널의 차이가 아니라 P1 궤적이 만나는 계(시도 수와 단계 크기)의 차이와 함께 나타납니다. "
        "이것은 해석이며 판정에 쓰이지 않습니다. 동결된 규칙 T는 궤적까지 포함한 정책 전체를 시험합니다.",
        "- 오차 범위 열은 같은 셀의 섭동 8개에 걸친 최소~최대입니다. 범위가 넓은 셀에서는 쌍별 오차 비가 정책보다 궤적을 더 많이 반영합니다.",
        "",
    ]
    lines += [
        "## 예산 소진 풀이 합계 (가족, 정책별; 규칙 없는 보고)",
        "",
        "| 가족 | S 섭동 j=1..8 | P1 섭동 j=1..8 | S 섭동 없음 (재풀이 실행) | P1 섭동 없음 (재풀이 실행) | S j=0 | P1 j=0 |",
        "|---|---|---|---|---|---|---|",
    ]
    for family, block in results["reports_without_rule"]["budget_exhausted_solves_per_family_and_arm"].items():
        lines.append(f"| {family} | {block['S']['ensemble_j1_to_8']} | {block['P1']['ensemble_j1_to_8']} | "
                     f"{block['S']['unperturbed_replay_driver']} | {block['P1']['unperturbed_replay_driver']} | "
                     f"{block['S']['unperturbed_j0_run']} | {block['P1']['unperturbed_j0_run']} |")
    rep = results["reproduction_check"]
    lines += [
        "",
        "## 재현 점검 (규칙 없음)",
        "",
        f"- 섭동하지 않은 실행(재풀이 실행 안의 구동기 실행과 j = 0 실행)을 V25, V24의 기록된 실행과 비교: {rep['rows']}개 중 "
        f"차이 있는 행 {rep['rows_with_differences']}개 (비교 항목: 끝 상태 sha256, 시도, 수락, 거절, 기록된 경우 예산 소진 수).",
        "",
        "## 범위 점검과 셀 표",
        "",
        f"- CELLS 실행이 만든 셀 {results['cells_check']['cells_built']}개, PROTOCOL.json과 다른 항목 {len(results['cells_check']['mismatches'])}개.",
        f"- 로그의 측정 바이너리: {json.dumps(results['binaries_of_the_logs'])} (PROTOCOL.json의 바이너리 {results['protocol_binary_sha256'][:12]}).",
        f"- K/V 범위 문제 {len(k['coverage_issues'])}건, T 범위 문제 {len(t['coverage_issues'])}건.",
    ]
    if "step_6" in results:
        step = results["step_6"]
        lines += ["", "## 6단계 (SWITCH 이후)", "", f"- {json.dumps(step.get('summary_ko', step.get('summary')), ensure_ascii=False)}"]
    lines += [
        "",
        "## 파일",
        "",
        "- `PREDECLARED.json` (동결), `PROTOCOL.json` (첫 실행 전 커밋), `NOTES.txt`, `run_measurements.py`, `parse_results.py`, `RESULTS.json`.",
        "- 원시 로그: `logs/cells`, `logs/replay` (계별 로그 `*.systems.txt.gz`), `logs/ensemble`, `logs/reproduction`, `logs/plumbing`.",
        "- 측정 대상: `crates/rodas5p-integrators/tests/mf_budget_measure.rs`.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    results = build_results()
    text = results_text(results)
    report = report_text(results)
    results_path = os.path.join(NODE, "RESULTS.json")
    report_path = os.path.join(NODE, "REPORT_KO.md")
    if args.check:
        same = []
        for path, new in ((results_path, text), (report_path, report)):
            with open(path, encoding="utf-8") as handle:
                same.append(handle.read() == new)
        print(f"RESULTS.json identical: {same[0]}; REPORT_KO.md identical: {same[1]}")
        return 0 if all(same) else 1
    with open(results_path, "w", encoding="utf-8") as handle:
        handle.write(text)
    with open(report_path, "w", encoding="utf-8") as handle:
        handle.write(report)
    verdict = results["verdict"]
    print(f"wrote RESULTS.json ({len(text)} bytes) and REPORT_KO.md; discriminators {verdict['discriminators']}, "
          f"frozen verdict {verdict['frozen_verdict']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
