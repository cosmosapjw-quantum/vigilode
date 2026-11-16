#!/usr/bin/env python3
"""Builds RESULTS.json of loop node V07 (stage a) from the raw logs.

Inputs (nothing else, and no typed-in result):
  logs/measure_adaptive.log, logs/measure_fixed_step.log  `SPECJ {json}` lines of the Rust
                                                           measurement programs
  logs/check_models.log                                    `MODELCHECK {json}` lines
  reference/reference.json                                 built by reference/build_reference.py
                                                           from the raw reference logs
  PREDECLARED.json                                         the protocol (hash and lists only)

The decision rules are those of PREDECLARED.json; the thresholds are the frozen row of
PREREGISTRATION.md (V07, stage a).

Run:  python3 -I build_results.py
"""
import hashlib
import json
import math
import struct
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Frozen discriminators (PREREGISTRATION.md, row V07, stage a).
STEP_MEDIAN_MAX = Fraction(102, 100)
STEP_MAX = Fraction(105, 100)
STEP_KILL = Fraction(110, 100)
ERROR_BAND = (0.67, 1.5)
ORDER_MIN = 4.8
# Predeclared protocol constants (PREDECLARED.json).
ROUND_OFF_GUARD = 1.0e-11
FIT_POINTS = 6
FIT_POINTS_MIN = 4
FROZEN_MODES = ("g8", "g7")
CLIENT_ARMS = ("g8", "g7", "lagged")
JITTER_ARMS = ("protected_rtol_up", "protected_rtol_down")


def number(bits):
    return struct.unpack(">d", bytes.fromhex(bits))[0]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_log(path, prefix):
    """The json records of a log and its `# key: value` header lines."""
    records, meta = [], {}
    for line in path.read_text().splitlines():
        if line.startswith(prefix + " "):
            records.append(json.loads(line[len(prefix) + 1:]))
        elif line.startswith("# ") and ": " in line:
            key, value = line[2:].split(": ", 1)
            meta.setdefault(key, value)
    return records, meta


def median(values):
    ordered = sorted(values)
    count = len(ordered)
    if count == 0:
        return None
    if count % 2 == 1:
        return ordered[count // 2]
    return (ordered[count // 2 - 1] + ordered[count // 2]) / 2


def wrms(difference, reference, atol, rtol):
    total = 0.0
    for d, r in zip(difference, reference):
        total += (d / (atol + rtol * abs(r))) ** 2
    return math.sqrt(total / len(reference))


def ols_slope(xs, ys):
    """Least-squares slope of ys against xs."""
    count = len(xs)
    mean_x, mean_y = sum(xs) / count, sum(ys) / count
    sxx = sum((x - mean_x) ** 2 for x in xs)
    sxy = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys))
    return sxy / sxx


def end_state(record):
    return [number(b) for b in record["y_end_bits"]]


def run_summary(record):
    """The counters of one run record that the report uses."""
    out = {"success": bool(record.get("success")), "error": record.get("error")}
    if "attempts" in record:
        counters = record["counters"]
        out.update({
            "attempts": record["attempts"], "accepted": record["accepted"],
            "rejected": record["rejected"], "jacobian_reuses": record["jacobian_reuses"],
            "jacobian_builds": counters["jacobian_builds"], "rhs_calls": counters["rhs_calls"],
            "direct_factorizations": counters["direct_factorizations"],
            "linear_solves": counters["linear_solves"],
            "local_error_failures": counters.get("local_error_failures", 0),
            "error_type_failures": counters.get("linear_solve_failures", 0)
            + counters.get("nonlinear_solve_failures", 0)
            + counters.get("nonfinite_step_failures", 0),
        })
    if "linearizations" in record:
        out["linearizations"] = record["linearizations"]
    if "partial" in record:
        out["partial"] = record["partial"]
    return out


# ------------------------------------------------------------------ adaptive


def error_cell(e_arm, e_prot, uncertainty):
    """Error ratio, its interval under the reference uncertainty and the cell verdict."""
    low = max(e_arm - uncertainty, 0.0) / (e_prot + uncertainty)
    high = (e_arm + uncertainty) / (e_prot - uncertainty) if e_prot > uncertainty else None
    ratio = e_arm / e_prot if e_prot > 0.0 else None
    if high is not None and low >= ERROR_BAND[0] and high <= ERROR_BAND[1]:
        verdict = "PASS"
    elif (high is not None and high < ERROR_BAND[0]) or low > ERROR_BAND[1]:
        verdict = "FAIL"
    else:
        verdict = "NOT_DECIDABLE"
    return {"error_ratio": ratio, "interval_low": low, "interval_high": high,
            "interval_high_unbounded": high is None, "verdict": verdict}


def step_cell(attempts_arm, attempts_prot):
    ratio = Fraction(attempts_arm, attempts_prot)
    return {"step_ratio": float(ratio), "attempts": attempts_arm,
            "attempts_protected": attempts_prot,
            "le_1_02": ratio <= STEP_MEDIAN_MAX, "le_1_05": ratio <= STEP_MAX,
            "gt_1_10": ratio > STEP_KILL}


def adaptive(records, reference):
    cells, order = {}, []
    probes, series = {}, []
    for record in records:
        kind = record.get("kind")
        if kind == "adaptive":
            key = (record["problem"], record["rtol"])
            if key not in cells:
                cells[key] = {}
                order.append(key)
            cells[key][record["arm"]] = record
        elif kind == "jdiff_adaptive":
            probes[(record["problem"], record["rtol"])] = record
        elif kind == "jdiff_series":
            series.append(record)

    out_cells = []
    ratios = {arm: [] for arm in CLIENT_ARMS + JITTER_ARMS}
    for key in order:
        problem, rtol = key
        arms = cells[key]
        protected, exact = arms["protected"], arms["exact"]
        rtol_exact = number(protected["rtol_bits"])
        atol = number(protected["atol_bits"])
        ref = reference["adaptive"][problem]
        valid = bool(protected.get("success")) and bool(exact.get("exact_equals_protected"))
        cell = {
            "problem": problem, "n": protected["n"], "rtol": rtol, "atol": atol,
            "t_end": protected["t_end"],
            "protected": run_summary(protected),
            "exact": run_summary(exact),
            "exact_equals_protected": bool(exact.get("exact_equals_protected")),
            "valid": valid,
            "reference_kind": ref["kind"],
            "reference_uncertainty_defined": bool(ref.get("uncertainty_defined")),
            "arms": {},
        }
        y_ref = ref.get("y_ref")
        e_prot = uncertainty = None
        if valid and y_ref is not None:
            y = end_state(protected)
            e_prot = wrms([a - b for a, b in zip(y, y_ref)], y_ref, atol, rtol_exact)
            cell["protected"]["end_point_error"] = e_prot
            # The size of one tolerance unit for the largest component of the reference.
            cell["tolerance_unit"] = atol + rtol_exact * max(abs(v) for v in y_ref)
            if ref.get("uncertainty_defined"):
                uncertainty = wrms(ref["u"], y_ref, atol, rtol_exact)
                cell["reference_uncertainty"] = uncertainty
        for arm in CLIENT_ARMS + JITTER_ARMS:
            record = arms[arm]
            entry = run_summary(record)
            if arm in JITTER_ARMS:
                entry["jitter_rtol"] = number(record["jitter_rtol_bits"])
                entry["jitter_atol"] = number(record["jitter_atol_bits"])
            if not valid:
                entry["status"] = "CELL_INVALID"
            elif not record.get("success"):
                entry["status"] = "ARM_FAILED"
                entry["obstacle"] = ("the arm did not reach t_end; its step ratio and error "
                                     "ratio are not defined")
            else:
                entry["status"] = "OK"
                entry["step"] = step_cell(record["attempts"], protected["attempts"])
                entry["jacobian_build_ratio"] = (
                    record["counters"]["jacobian_builds"]
                    / protected["counters"]["jacobian_builds"])
                if e_prot is not None:
                    y = end_state(record)
                    e_arm = wrms([a - b for a, b in zip(y, y_ref)], y_ref, atol, rtol_exact)
                    entry["end_point_error"] = e_arm
                    if uncertainty is not None:
                        entry["error"] = error_cell(e_arm, e_prot, uncertainty)
                    else:
                        entry["error"] = {
                            "error_ratio": e_arm / e_prot if e_prot > 0.0 else None,
                            "verdict": "NOT_DECIDABLE",
                            "obstacle": ref.get("obstacle", "no reference uncertainty")}
                ratios[arm].append((key, Fraction(record["attempts"], protected["attempts"])))
            cell["arms"][arm] = entry
        probe = probes.get(key)
        if probe is not None:
            cell["jacobian_probe"] = {k: probe[k] for k in (
                "steps_probed", "steps_without_recorded_attempt", "h", "e7_rel", "e8_rel",
                "e_lag_rel", "d7_tolerance_units", "d8_tolerance_units")}
        out_cells.append(cell)

    summary = {}
    for arm in CLIENT_ARMS + JITTER_ARMS:
        valid_cells = [c for c in out_cells if c["valid"]]
        arm_cells = [c for c in valid_cells if c["arms"][arm]["status"] == "OK"]
        failed = [[c["problem"], c["rtol"]] for c in valid_cells
                  if c["arms"][arm]["status"] == "ARM_FAILED"]
        fractions = [f for _, f in ratios[arm]]
        entry = {
            "cells_total": len(out_cells), "cells_valid": len(valid_cells),
            "cells_with_ratio": len(arm_cells), "cells_arm_failed": failed,
        }
        if fractions:
            middle = median(fractions)
            largest = max(fractions)
            smallest = min(fractions)
            worst_key = max(ratios[arm], key=lambda item: item[1])[0]
            entry["step"] = {
                "median": float(middle), "max": float(largest), "min": float(smallest),
                "max_cell": list(worst_key),
                "median_le_1_02": middle <= STEP_MEDIAN_MAX,
                "max_le_1_05": largest <= STEP_MAX,
                "cells_above_1_02": sum(1 for f in fractions if f > STEP_MEDIAN_MAX),
                "cells_above_1_05": [[k[0], k[1], float(f)] for k, f in ratios[arm]
                                     if f > STEP_MAX],
                "cells_above_1_10": [[k[0], k[1], float(f)] for k, f in ratios[arm]
                                     if f > STEP_KILL],
                "cells_below_1": sum(1 for f in fractions if f < 1),
                "cells_equal_1": sum(1 for f in fractions if f == 1),
                "cells_above_1": sum(1 for f in fractions if f > 1),
                "verdict": "PASS" if (middle <= STEP_MEDIAN_MAX and largest <= STEP_MAX)
                else "FAIL",
            }
            total_arm = sum(c["arms"][arm]["attempts"] for c in arm_cells)
            total_prot = sum(c["protected"]["attempts"] for c in arm_cells)
            entry["step"]["attempts_total_ratio"] = total_arm / total_prot
            builds_arm = sum(c["arms"][arm]["jacobian_builds"] for c in arm_cells)
            builds_prot = sum(c["protected"]["jacobian_builds"] for c in arm_cells)
            entry["jacobian_builds"] = {"arm_total": builds_arm, "protected_total": builds_prot,
                                        "ratio": builds_arm / builds_prot}
        errors = [c["arms"][arm]["error"] for c in arm_cells if "error" in c["arms"][arm]]
        if errors:
            verdicts = [e["verdict"] for e in errors]
            values = [e["error_ratio"] for e in errors if e.get("error_ratio") is not None]
            outside = [[c["problem"], c["rtol"], c["arms"][arm]["error"]["error_ratio"],
                        c["arms"][arm]["error"]["verdict"]]
                       for c in arm_cells if "error" in c["arms"][arm]
                       and c["arms"][arm]["error"]["verdict"] != "PASS"]
            if "FAIL" in verdicts:
                verdict = "FAIL"
            elif all(v == "PASS" for v in verdicts) and len(errors) == len(valid_cells):
                verdict = "PASS"
            else:
                verdict = "NOT_DECIDABLE"
            entry["error"] = {
                "cells_pass": verdicts.count("PASS"), "cells_fail": verdicts.count("FAIL"),
                "cells_not_decidable": verdicts.count("NOT_DECIDABLE"),
                "cells_not_pass": outside,
                "median": median(values), "min": min(values), "max": max(values),
                "verdict": verdict,
            }
        if arm in CLIENT_ARMS:
            stats = [c["arms"][arm].get("linearizations") for c in arm_cells]
            stats = [s for s in stats if s]
            entry["linearizations"] = {
                key: sum(s[key] for s in stats) for key in (
                    "total", "speculative", "exact_first", "exact_after_failure",
                    "exact_no_point", "exact_by_mode")}
        summary[arm] = entry
    summary["kill_g7"] = bool(summary["g7"].get("step", {}).get("cells_above_1_10"))
    return {"cells": out_cells, "summary": summary, "series": series}


def matched_error(cells):
    """E5: attempts of an arm at the protected arm's achieved error, by log-log
    interpolation of the arm's (error, attempts) points per problem."""
    out = {}
    problems = []
    for cell in cells:
        if cell["problem"] not in problems:
            problems.append(cell["problem"])
    for arm in CLIENT_ARMS:
        rows = []
        for problem in problems:
            group = sorted((c for c in cells if c["problem"] == problem),
                           key=lambda c: -c["rtol"])
            usable = all(c["valid"] and c["arms"][arm]["status"] == "OK"
                         and "end_point_error" in c["arms"][arm]
                         and "end_point_error" in c["protected"] for c in group)
            if not usable:
                rows.append({"problem": problem, "status": "not evaluable"})
                continue
            # Errors of different tolerances are put on one scale by the size of the
            # tolerance unit of each cell (atol + rtol max |y_ref|).
            arm_points = [(math.log(c["arms"][arm]["end_point_error"] * c["tolerance_unit"]),
                           math.log(c["arms"][arm]["attempts"])) for c in group]
            xs = [p[0] for p in arm_points]
            if not all(a > b for a, b in zip(xs, xs[1:])):
                rows.append({"problem": problem, "status": "arm error not monotone in rtol"})
                continue
            ratios = []
            for c in group:
                x = math.log(c["protected"]["end_point_error"] * c["tolerance_unit"])
                # piecewise linear in (log error, log attempts); end segments extrapolate
                if x >= xs[1]:
                    (x0, y0), (x1, y1) = arm_points[0], arm_points[1]
                else:
                    (x0, y0), (x1, y1) = arm_points[1], arm_points[2]
                attempts = math.exp(y0 + (y1 - y0) * (x - x0) / (x1 - x0))
                ratios.append({"rtol": c["rtol"],
                               "matched_error_step_ratio": attempts / c["protected"]["attempts"]})
            rows.append({"problem": problem, "status": "ok", "points": ratios})
        tagged = [(p["matched_error_step_ratio"], r["problem"], p["rtol"])
                  for r in rows if r["status"] == "ok" for p in r["points"]]
        values = [t[0] for t in tagged]
        out[arm] = {"rows": rows, "values": len(values),
                    "median": median(values) if values else None,
                    "max": max(values) if values else None,
                    "min": min(values) if values else None,
                    "max_cell": list(max(tagged)[1:]) if tagged else None,
                    "min_cell": list(min(tagged)[1:]) if tagged else None}
    return out


# --------------------------------------------------------------- fixed step


def fixed_step_ok(record, steps):
    """The predeclared constant-step check of one run."""
    return (bool(record.get("success")) and record.get("accepted") == steps
            and record.get("rejected") == 0
            and record.get("h_min_bits") == record["h_bits"]
            and record.get("h_max_bits") == record["h_bits"])


def order_set(records, reference):
    by_problem, problems = {}, []
    probes = {}
    for record in records:
        if record.get("kind") == "fixed":
            if record["problem"] not in by_problem:
                by_problem[record["problem"]] = {}
                problems.append(record["problem"])
            by_problem[record["problem"]].setdefault(record["steps"], {})[record["arm"]] = record
        elif record.get("kind") == "jdiff_fixed":
            probes[(record["problem"], record["steps"])] = record
    arms = ("protected", "exact", "g8", "g7", "lagged")
    out = []
    for problem in problems:
        exact_solution = reference["order"][problem]["y_ref"]
        scale = max(abs(v) for v in exact_solution)
        grid = []
        for steps in sorted(by_problem[problem]):
            row = {"steps": steps}
            for arm in arms:
                record = by_problem[problem][steps][arm]
                row["h"] = number(record["h_bits"])
                ok = fixed_step_ok(record, steps)
                entry = {"fixed_step_ok": ok}
                if record.get("success") and record.get("y_end_bits"):
                    y = end_state(record)
                    entry["error"] = max(abs(a - b) for a, b in zip(y, exact_solution)) / scale
                if arm == "exact":
                    entry["exact_equals_protected"] = bool(record.get("exact_equals_protected"))
                row[arm] = entry
            probe = probes.get((problem, steps))
            if probe is not None and probe.get("e7_abs"):
                row["jacobian_probe"] = {
                    "e7_abs_max": probe["e7_abs"]["max"], "e8_abs_max": probe["e8_abs"]["max"],
                    "e_lag_abs_max": probe["e_lag_abs"]["max"],
                    "d7_inf_max": probe["d7_inf"]["max"], "d8_inf_max": probe["d8_inf"]["max"],
                    "d_lag_inf_max": probe["d_lag_inf"]["max"],
                    "norm_j_max": probe["norm_j"]["max"]}
            grid.append(row)
        # Fit rule: largest h downwards, stop before the first exact-arm error below the
        # round-off guard (or an exact-arm run that is not a fixed-step run); then the six
        # smallest h of what remains.
        contiguous = []
        stopped_by = None
        for row in grid:  # ascending steps = descending h
            exact = row["exact"]
            if not exact["fixed_step_ok"] or "error" not in exact:
                stopped_by = {"steps": row["steps"], "reason": "exact arm is not a fixed-step run"}
                break
            if exact["error"] < ROUND_OFF_GUARD:
                stopped_by = {"steps": row["steps"], "reason": "exact-arm error below 1e-11"}
                break
            contiguous.append(row)
        selected = contiguous[-FIT_POINTS:]
        fit = {"points_before_round_off_guard": len(contiguous), "stopped_by": stopped_by,
               "steps_used": [row["steps"] for row in selected],
               "h_used": [row["h"] for row in selected], "slopes": {}, "fitted": {}}
        if len(selected) >= FIT_POINTS_MIN:
            log_h = [math.log10(row["h"]) for row in selected]
            for arm in arms:
                usable = all(row[arm]["fixed_step_ok"] and row[arm].get("error", 0.0) > 0.0
                             for row in selected)
                fit["fitted"][arm] = usable
                if usable:
                    fit["slopes"][arm] = ols_slope(
                        log_h, [math.log10(row[arm]["error"]) for row in selected])
            probe_rows = [row for row in selected if "jacobian_probe" in row]
            if len(probe_rows) == len(selected):
                fit["jacobian_difference_slopes"] = {}
                for name in ("e7_abs_max", "e8_abs_max", "e_lag_abs_max", "d7_inf_max",
                             "d8_inf_max", "d_lag_inf_max"):
                    values = [row["jacobian_probe"][name] for row in selected]
                    if all(v > 0.0 for v in values):
                        fit["jacobian_difference_slopes"][name] = ols_slope(
                            log_h, [math.log10(v) for v in values])
                    else:
                        fit["jacobian_difference_slopes"][name] = None
        else:
            for arm in arms:
                fit["fitted"][arm] = False
        exact_slope = fit["slopes"].get("exact")
        fit["exact_discriminating"] = exact_slope is not None and exact_slope >= ORDER_MIN
        fit["mode_verdicts"] = {}
        for arm in FROZEN_MODES + ("lagged",):
            if not fit["exact_discriminating"]:
                fit["mode_verdicts"][arm] = "NOT_DISCRIMINATING"
            elif arm not in fit["slopes"]:
                fit["mode_verdicts"][arm] = "FAIL"
            else:
                fit["mode_verdicts"][arm] = "PASS" if fit["slopes"][arm] >= ORDER_MIN else "FAIL"
        # Local orders between consecutive grid points (context).
        local = []
        for upper, lower in zip(grid, grid[1:]):
            entry = {"steps": [upper["steps"], lower["steps"]]}
            for arm in ("exact", "g8", "g7", "lagged"):
                a, b = upper[arm].get("error"), lower[arm].get("error")
                if a and b:
                    entry[arm] = math.log(a / b) / math.log(upper["h"] / lower["h"])
            local.append(entry)
        out.append({"problem": problem, "grid": grid, "fit": fit, "local_orders": local,
                    "all_exact_runs_equal_protected": all(
                        row["exact"].get("exact_equals_protected") for row in grid),
                    "all_runs_fixed_step": all(row[arm]["fixed_step_ok"]
                                               for row in grid for arm in arms)})
    summary = {}
    for arm in FROZEN_MODES + ("lagged",):
        verdicts = {p["problem"]: p["fit"]["mode_verdicts"][arm] for p in out}
        discriminating = [k for k, v in verdicts.items() if v != "NOT_DISCRIMINATING"]
        if not discriminating:
            verdict = "NOT_DECIDABLE"
        elif any(verdicts[k] == "FAIL" for k in discriminating):
            verdict = "FAIL"
        else:
            verdict = "PASS"
        slopes = [p["fit"]["slopes"][arm] for p in out
                  if p["problem"] in discriminating and arm in p["fit"]["slopes"]]
        summary[arm] = {"per_problem": verdicts, "discriminating_problems": discriminating,
                        "min_slope_on_discriminating": min(slopes) if slopes else None,
                        "verdict": verdict}
    summary["exact_slopes"] = {p["problem"]: p["fit"]["slopes"].get("exact") for p in out}
    return {"problems": out, "summary": summary}


# ------------------------------------------------------------- exploratory


def error_context(cells):
    """Context for the error-ratio criterion (no gate): the arms against the
    tolerance-jitter controls cell by cell, and the verdict counts per problem. None of
    this replaces the predeclared rule."""
    arms = FROZEN_MODES + JITTER_ARMS
    by_problem, problems = {}, []
    beyond_control = {mode: [] for mode in FROZEN_MODES}
    rows = []
    for cell in cells:
        problem = cell["problem"]
        if problem not in by_problem:
            by_problem[problem] = {arm: {"PASS": 0, "FAIL": 0, "NOT_DECIDABLE": 0}
                                   for arm in arms}
            by_problem[problem]["protected_error_tolerance_units"] = []
            problems.append(problem)
        if not cell["valid"] or "end_point_error" not in cell["protected"]:
            continue
        e_prot = cell["protected"]["end_point_error"]
        by_problem[problem]["protected_error_tolerance_units"].append(e_prot)
        ratios = {}
        for arm in arms:
            entry = cell["arms"][arm]
            if entry["status"] == "OK" and "error" in entry:
                by_problem[problem][arm][entry["error"]["verdict"]] += 1
                ratios[arm] = entry["error"]["error_ratio"]
        row = {"problem": problem, "rtol": cell["rtol"],
               "protected_error_tolerance_units": e_prot, "error_ratios": ratios}
        controls = [abs(math.log(ratios[a])) for a in JITTER_ARMS
                    if ratios.get(a) and ratios[a] > 0.0]
        if controls:
            row["largest_control_log_deviation"] = max(controls)
            for mode in FROZEN_MODES:
                if ratios.get(mode) and ratios[mode] > 0.0:
                    deviation = abs(math.log(ratios[mode]))
                    row[mode + "_log_deviation"] = deviation
                    if deviation > max(controls):
                        beyond_control[mode].append([problem, cell["rtol"], ratios[mode]])
        rows.append(row)
    for problem in problems:
        values = by_problem[problem].pop("protected_error_tolerance_units")
        by_problem[problem]["protected_error_tolerance_units_min"] = min(values)
        by_problem[problem]["protected_error_tolerance_units_max"] = max(values)
    spread = {}
    for arm in arms:
        logs = sorted(abs(math.log(r["error_ratios"][arm])) for r in rows
                      if r["error_ratios"].get(arm) and r["error_ratios"][arm] > 0.0)
        if logs:
            spread[arm] = {
                "cells": len(logs),
                "median_abs_log_ratio": median(logs),
                "q90_abs_log_ratio": logs[min(len(logs) - 1, int(round(0.9 * (len(logs) - 1))))],
                "max_abs_log_ratio": logs[-1],
                "cells_outside_band": sum(1 for r in rows if r["error_ratios"].get(arm)
                                          and not (ERROR_BAND[0] <= r["error_ratios"][arm]
                                                   <= ERROR_BAND[1])),
            }
    return {"by_problem": by_problem, "rows": rows, "spread": spread,
            "cells_where_the_arm_deviates_more_than_both_controls": beyond_control,
            "note": "abs log ratio = |ln(e_arm / e_protected)|; the controls are the protected "
                    "method with rtol and atol multiplied or divided by 1.02"}


def without_run_b(cells, reference):
    """The error verdicts with the uncertainty u_i = |ref - A|_i + |ref - C|_i (run B left
    out). Exploratory: the predeclared rule takes the larger of B and C."""
    out = {arm: {"PASS": 0, "FAIL": 0, "NOT_DECIDABLE": 0, "changed_cells": []}
           for arm in FROZEN_MODES + JITTER_ARMS}
    for cell in cells:
        ref = reference["adaptive"][cell["problem"]]
        if not cell["valid"] or "end_point_error" not in cell["protected"]:
            continue
        alternative = ref.get("u_without_b", ref.get("u"))
        if alternative is None:
            continue
        rtol, atol = cell["rtol"], cell["atol"]
        uncertainty = wrms(alternative, ref["y_ref"], atol, rtol)
        for arm in out:
            entry = cell["arms"][arm]
            if entry["status"] != "OK" or "end_point_error" not in entry:
                continue
            verdict = error_cell(entry["end_point_error"],
                                 cell["protected"]["end_point_error"], uncertainty)["verdict"]
            out[arm][verdict] += 1
            if "error" in entry and entry["error"]["verdict"] != verdict:
                out[arm]["changed_cells"].append(
                    [cell["problem"], cell["rtol"], entry["error"]["verdict"], verdict])
    return out


def e5_invariant(records, reference):
    """A lead for the e5 end-point errors (no gate): f3 = f2 - f4 makes y3 - y2 + y4 a
    linear invariant with value 0. Its residual at t_end, for the reference and for every
    arm, next to the size of the reference end value."""
    ref = reference["adaptive"].get("e5", {}).get("y_ref")
    if ref is None:
        return None
    rows = []
    for record in records:
        if record.get("kind") == "adaptive" and record["problem"] == "e5" \
                and record.get("success") and record["arm"] in ("protected", "g8", "g7"):
            y = end_state(record)
            rows.append({"rtol": record["rtol"], "arm": record["arm"],
                         "y3_minus_y2_plus_y4": y[2] - y[1] + y[3]})
    return {"reference_end_value_size": max(abs(v) for v in ref),
            "reference_residual": ref[2] - ref[1] + ref[3], "runs": rows}


def series_summary(series):
    """E2 along one adaptive run: quantiles and the least-squares slope of log norm against
    log h over the steps (descriptive: the constant varies along the run)."""
    if not series:
        return None
    usable = [s for s in series if s["h"] > 0.0]
    out = {"problem": series[0]["problem"], "rtol": series[0]["rtol"], "steps": len(series)}
    for name in ("e7_abs", "e8_abs", "e_lag_abs"):
        rows = [s for s in usable if s[name] > 0.0]
        out[name + "_slope_vs_h"] = ols_slope([math.log10(s["h"]) for s in rows],
                                              [math.log10(s[name]) for s in rows]) \
            if len(rows) >= 3 else None
        relative = sorted(s[name] / s["norm_j"] for s in usable)
        out[name + "_over_norm_j"] = {"median": median(relative), "max": relative[-1],
                                      "min": relative[0]}
    return out


def git_head():
    try:
        return subprocess.check_output(["git", "-C", str(HERE), "rev-parse", "HEAD"],
                                       text=True).strip()
    except Exception:  # the results do not depend on it
        return None


def main():
    predeclared_path = HERE / "PREDECLARED.json"
    predeclared = json.loads(predeclared_path.read_text())
    reference_path = HERE / "reference" / "reference.json"
    reference = json.loads(reference_path.read_text())
    adaptive_log = HERE / "logs" / "measure_adaptive.log"
    fixed_log = HERE / "logs" / "measure_fixed_step.log"
    models_log = HERE / "logs" / "check_models.log"
    adaptive_records, adaptive_meta = read_log(adaptive_log, "SPECJ")
    fixed_records, fixed_meta = read_log(fixed_log, "SPECJ")
    model_records, _ = read_log(models_log, "MODELCHECK")

    def complete(records, program):
        return any(r.get("kind") == "footer" and r.get("program") == program
                   and r.get("complete") for r in records)

    adaptive_result = adaptive(adaptive_records, reference)
    order_result = order_set(fixed_records, reference)
    model_footer = next((r for r in model_records if r.get("kind") == "footer"), {})

    # Consistency of the logs with the predeclared lists.
    predeclared_cells = [(p["id"], r) for p in predeclared["adaptive_set"]["problems"]
                         for r in predeclared["adaptive_set"]["rtols"]]
    measured_cells = [(c["problem"], c["rtol"]) for c in adaptive_result["cells"]]
    predeclared_order = [p["id"] for p in predeclared["order_set"]["problems"]]
    measured_order = [p["problem"] for p in order_result["problems"]]
    measured_steps = sorted({row["steps"] for p in order_result["problems"] for row in p["grid"]})
    protocol = {
        "adaptive_cells_match_predeclared": measured_cells == predeclared_cells,
        "order_problems_match_predeclared": measured_order == predeclared_order,
        "order_steps_match_predeclared": measured_steps == sorted(predeclared["order_set"]["N"]),
        "adaptive_log_complete": complete(adaptive_records, "adaptive"),
        "fixed_step_log_complete": complete(fixed_records, "fixed_step"),
        "all_exact_arms_equal_protected": all(c["exact_equals_protected"]
                                              for c in adaptive_result["cells"])
        and all(p["all_exact_runs_equal_protected"] for p in order_result["problems"]),
        "all_cells_valid": all(c["valid"] for c in adaptive_result["cells"]),
        "model_definitions_agree": bool(model_footer.get("all_within_limit")),
        "model_probes": model_footer.get("probes"),
        "largest_model_rhs_relative_difference":
            model_footer.get("largest_rhs_relative_difference"),
    }

    summary = adaptive_result["summary"]
    verdicts = {}
    for mode in FROZEN_MODES:
        step = summary[mode].get("step", {}).get("verdict", "NOT_DECIDABLE")
        error = summary[mode].get("error", {}).get("verdict", "NOT_DECIDABLE")
        order = order_result["summary"][mode]["verdict"]
        arm_failed = summary[mode]["cells_arm_failed"]
        kill = mode == "g7" and summary["kill_g7"]
        criteria = {"step_ratio": step, "error_ratio": error, "observed_order": order}
        if kill:
            verdict = "KILL"
        elif "FAIL" in criteria.values() or arm_failed:
            verdict = "FAIL"
        elif all(v == "PASS" for v in criteria.values()):
            verdict = "PASS"
        else:
            verdict = "NOT_DECIDABLE"
        verdicts[mode] = {
            "criteria": criteria, "kill": kill, "cells_arm_failed": arm_failed,
            "undecided": [k for k, v in criteria.items() if v == "NOT_DECIDABLE"],
            "failed": [k for k, v in criteria.items() if v == "FAIL"],
            "verdict": verdict,
        }

    reference_summary = {}
    for problem, entry in reference["adaptive"].items():
        item = {"kind": entry["kind"],
                "uncertainty_defined": bool(entry.get("uncertainty_defined"))}
        if entry["kind"] == "numerical":
            item["differences_max_rel"] = {k: v["max_rel"]
                                           for k, v in entry.get("differences", {}).items()}
            item["independent_runs_used"] = entry.get("independent_runs_used")
            item["runs_usable"] = {k: v.get("usable") for k, v in entry.get("runs", {}).items()}
        reference_summary[problem] = item

    results = {
        "schema": "vigilode.loop-l1.v07-stage-a.results.v1",
        "node": "V07_SPECULATIVE_JACOBIAN",
        "stage": "a (sequential emulation; no threads, no timing claim)",
        "provenance": {
            "built_at_git_head": git_head(),
            "predeclared_sha256": sha256(predeclared_path),
            "inputs": {
                "logs/measure_adaptive.log": {"sha256": sha256(adaptive_log),
                                              "git_head": adaptive_meta.get("git HEAD"),
                                              "command": adaptive_meta.get("command"),
                                              "rustc": adaptive_meta.get("rustc"),
                                              "exit_code": adaptive_meta.get("exit code"),
                                              "started": adaptive_meta.get("started"),
                                              "finished": adaptive_meta.get("finished")},
                "logs/measure_fixed_step.log": {"sha256": sha256(fixed_log),
                                                "git_head": fixed_meta.get("git HEAD"),
                                                "command": fixed_meta.get("command"),
                                                "rustc": fixed_meta.get("rustc"),
                                                "exit_code": fixed_meta.get("exit code"),
                                                "started": fixed_meta.get("started"),
                                                "finished": fixed_meta.get("finished")},
                "logs/check_models.log": {"sha256": sha256(models_log)},
                "reference/reference.json": {"sha256": sha256(reference_path),
                                             "sources": reference["sources"],
                                             "versions": reference["versions"]},
            },
            "wall_time": "not measured; every number here is a deterministic counter or a "
                         "floating-point state",
        },
        "frozen_thresholds": {
            "step_ratio_median_max": float(STEP_MEDIAN_MAX), "step_ratio_max": float(STEP_MAX),
            "step_ratio_kill_g7": float(STEP_KILL), "error_ratio_band": list(ERROR_BAND),
            "observed_order_min": ORDER_MIN,
        },
        "protocol_checks": protocol,
        "reference_summary": reference_summary,
        "verdicts": verdicts,
        "adaptive": {"summary": summary, "cells": adaptive_result["cells"]},
        "order": order_result,
        "exploratory_no_gate": {
            "E1_lagged": {"adaptive": summary["lagged"],
                          "order": order_result["summary"]["lagged"]},
            "E2_jacobian_differences_fixed_step": {
                p["problem"]: p["fit"].get("jacobian_difference_slopes")
                for p in order_result["problems"]},
            "E2_series": series_summary(adaptive_result["series"]),
            "E4_tolerance_jitter": {arm: summary[arm] for arm in JITTER_ARMS},
            "E4_error_ratio_context": error_context(adaptive_result["cells"]),
            "E5_matched_error": matched_error(adaptive_result["cells"]),
            "E6_error_verdicts_without_reference_run_B": without_run_b(
                adaptive_result["cells"], reference),
            "E7_e5_linear_invariant": e5_invariant(adaptive_records, reference),
        },
    }
    (HERE / "RESULTS.json").write_text(json.dumps(results, indent=1, allow_nan=False) + "\n")
    print(json.dumps({"verdicts": {k: v["verdict"] for k, v in verdicts.items()},
                      "protocol_checks": protocol}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
