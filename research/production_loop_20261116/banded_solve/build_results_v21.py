#!/usr/bin/env python3
"""Builds RESULTS_V21.json of loop L1 node V21 (stages a1 and a2) from the raw logs only.

Inputs: logs/xcheck_S*.log (validity of the second binary), logs/measure_v*_n*.jsonl (kernel
level, measure.py --tag v) and logs/drive_v*_*.jsonl (whole runs, measure_drive.py --tag v).
Rules: PREDECLARED_V21.json and pre-registration addendum A4. No number is typed in here.
"""
import glob
import hashlib
import importlib.util
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("v19_results", os.path.join(HERE, "build_results.py"))
V19 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(V19)

MAX_LOAD, MAX_SPREAD = 8.0, 0.10
A1_PASS_I, A1_PASS_C, A1_KILL_I = 0.40, 0.60, 0.77
A2_PASS_I, A2_PASS_C, A2_KILL_I = 0.50, 0.67, 0.90
DECIDING_N = [10000, 100000]
WIDE = [("ac2d", 64), ("bruss2d", 32), ("ac3d", 16)]


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def read_lines(path):
    header, footer, runs = None, None, []
    with open(path) as f:
        for line in f:
            rec = json.loads(line)
            if rec["type"] == "header":
                header = rec
            elif rec["type"] == "footer":
                footer = rec
            else:
                runs.append(rec)
    return header, footer, runs


def med_spread(values):
    med = statistics.median(values)
    return med, (max(values) - min(values)) / med


def kernel_cell(path):
    header, footer, runs = read_lines(path)
    r_pat, n = header["patterns"], header["n"]
    arms = {}
    for arm in header["arms"]:
        rounds = {}
        problems = 0
        for rec in runs:
            if rec["arm"] != arm or rec["warmup"]:
                continue
            if rec["exit"] != 0 or "errors=0" not in rec["stdout"] or "cycles" not in rec:
                problems += 1
                continue
            rounds.setdefault(rec["round"], {})[(rec["patterns"], rec["solves"])] = rec
        series = {k: [] for k in ("i_pat", "i_fac", "i_sol", "c_pat", "c_fac", "c_sol")}
        for rnd in sorted(rounds):
            trio = rounds[rnd]
            if len(trio) != 3:
                continue
            c00, cr0, cr8 = trio[(0, 0)], trio[(r_pat, 0)], trio[(r_pat, 8)]
            for key, field in (("i", "instructions"), ("c", "cycles")):
                series[key + "_pat"].append((cr8[field] - c00[field]) / r_pat)
                series[key + "_fac"].append((cr0[field] - c00[field]) / r_pat)
                series[key + "_sol"].append((cr8[field] - cr0[field]) / (8 * r_pat))
        if not series["i_pat"]:
            continue
        i_pat, i_spread = med_spread(series["i_pat"])
        c_pat, c_spread = med_spread(series["c_pat"])
        arms[arm] = {
            "rounds": len(series["i_pat"]), "runs_with_problems": problems,
            "instructions_per_pattern": i_pat, "instructions_spread": i_spread,
            "cycles_per_pattern": c_pat, "cycles_spread": c_spread,
            "instructions_per_row_factorisation": statistics.median(series["i_fac"]) / n,
            "instructions_per_row_solve": statistics.median(series["i_sol"]) / n,
            "cycles_per_row_factorisation": statistics.median(series["c_fac"]) / n,
            "cycles_per_row_solve": statistics.median(series["c_sol"]) / n,
        }
    if "A0" in arms:
        for entry in arms.values():
            entry["instructions_ratio_to_A0"] = (
                entry["instructions_per_pattern"] / arms["A0"]["instructions_per_pattern"])
            entry["cycles_ratio_to_A0"] = entry["cycles_per_pattern"] / arms["A0"]["cycles_per_pattern"]
    complete = footer is not None
    load_ok = complete and max(header["load_before"], footer["load_after"]) <= MAX_LOAD
    return {
        "file": os.path.relpath(path, HERE), "sha256": sha256(path), "n": n, "index": header["index"],
        "patterns": r_pat, "complete": complete, "load_before": header["load_before"],
        "load_after": footer["load_after"] if complete else None, "load_ok": load_ok,
        "cycles_authoritative_for_TA": bool(
            load_ok and all(a in arms and arms[a]["cycles_spread"] <= MAX_SPREAD for a in ("A0", "TA"))),
        "arms": arms,
    }


def drive_cell(path):
    header, footer, runs = read_lines(path)
    budget = header["budget"]
    for rec in runs:
        rec["f"] = dict(tok.split("=", 1) for tok in rec["stdout"].split()[1:]) \
            if rec["stdout"].startswith("DRIVE ") else {}
    kernels, signatures = {}, {}
    keys = ("attempts", "accepted", "rejected", "factor_ops", "solve_ops", "state_hash")
    for kernel in header["kernels"]:
        rounds = {}
        for rec in runs:
            if rec["kernel"] != kernel or rec["exit"] != 0 or not rec["f"]:
                continue
            signatures.setdefault(rec["budget"], {}).setdefault(kernel, set()).add(
                tuple(rec["f"][k] for k in keys))
            if not rec["warmup"] and "cycles" in rec:
                rounds.setdefault(rec["round"], {})[rec["budget"]] = rec
        ins, cyc, attempts = [], [], None
        for rnd in sorted(rounds):
            pair = rounds[rnd]
            if len(pair) != 2:
                continue
            lo, hi = pair[1], pair[budget]
            attempts = int(hi["f"]["attempts"])
            d_att = attempts - int(lo["f"]["attempts"])
            ins.append((hi["instructions"] - lo["instructions"]) / d_att)
            cyc.append((hi["cycles"] - lo["cycles"]) / d_att)
        if ins:
            i_med, i_spread = med_spread(ins)
            c_med, c_spread = med_spread(cyc)
            kernels[kernel] = {"rounds": len(ins), "attempts_of_the_long_run": attempts,
                               "instructions_per_attempt": i_med, "instructions_spread": i_spread,
                               "cycles_per_attempt": c_med, "cycles_spread": c_spread}
    identical = bool(signatures) and all(
        len(per) == len(header["kernels"]) and len(set().union(*per.values())) == 1
        for per in signatures.values())
    if "indexed" in kernels:
        for entry in kernels.values():
            entry["instructions_ratio_to_indexed"] = (
                entry["instructions_per_attempt"] / kernels["indexed"]["instructions_per_attempt"])
            entry["cycles_ratio_to_indexed"] = (
                entry["cycles_per_attempt"] / kernels["indexed"]["cycles_per_attempt"])
    sample = next((r["f"] for r in runs if r["f"]), {})
    complete = footer is not None
    load_ok = complete and max(header["load_before"], footer["load_after"]) <= MAX_LOAD
    return {
        "file": os.path.relpath(path, HERE), "sha256": sha256(path), "family": header["family"],
        "size": header["size"], "index": header["index"], "n": int(sample.get("n", 0)),
        "band": int(sample.get("lower", 0)), "budget": budget, "complete": complete,
        "load_before": header["load_before"], "load_after": footer["load_after"] if complete else None,
        "load_ok": load_ok,
        "counts_operations_and_state_hash_equal_in_every_run": identical,
        "cycles_authoritative": bool(load_ok and all(
            k in kernels and kernels[k]["cycles_spread"] <= MAX_SPREAD for k in ("indexed", "slices"))),
        "kernels": kernels,
    }


def first_authoritative(cells, flag):
    """The first authoritative measurement of a cell, else the last one (diagnostic)."""
    cells = sorted(cells, key=lambda c: c["index"])
    for cell in cells:
        if cell[flag]:
            return cell, True
    return (cells[-1], False) if cells else (None, False)


def main():
    cases, check_files = V19.parse_checks("xcheck_S*.log")
    lu2 = {k: rows for k, rows in cases.items() if rows["A0"]["lu"] == "2"}
    ta_rows = [rows["TA"] for rows in lu2.values()]
    ta_identical = [r for r in ta_rows if r["status"] == "ok" and r["bits_a0"] == "1"]
    val = V19.validity(cases)
    validity = {
        "cases_with_lower_upper_2": len(lu2),
        "TA_bit_identical_cases": len(ta_identical),
        "TA_valid": bool(ta_rows) and len(ta_identical) == len(ta_rows),
        "TD_V2": val["arms"]["TD"]["V2"], "TD_V3": val["arms"]["TD"]["V3"],
        "TD_valid": val["arms"]["TD"]["valid"],
    }

    kernel_cells = [kernel_cell(p) for p in sorted(glob.glob(os.path.join(HERE, "logs", "measure_v*_n*.jsonl")))]
    a1 = {"per_size": {}}
    for n in DECIDING_N:
        cells = [c for c in kernel_cells if c["n"] == n and c["complete"] and "TA" in c["arms"]]
        cell, auth = first_authoritative(cells, "cycles_authoritative_for_TA")
        if cell is None:
            a1["per_size"][str(n)] = None
            continue
        ta = cell["arms"]["TA"]
        a1["per_size"][str(n)] = {
            "measurement": cell["file"], "cycles_label": "authoritative" if auth else "diagnostic_only",
            "TA_instructions_ratio": ta["instructions_ratio_to_A0"],
            "TA_cycles_ratio": ta["cycles_ratio_to_A0"],
            "TD_instructions_ratio": cell["arms"].get("TD", {}).get("instructions_ratio_to_A0"),
            "TD_cycles_ratio": cell["arms"].get("TD", {}).get("cycles_ratio_to_A0"),
        }
    sizes = [a1["per_size"].get(str(n)) for n in DECIDING_N]
    if any(s is None for s in sizes):
        a1.update({"PASS_I": "NOT_RUN", "PASS_C": "NOT_RUN"})
    else:
        if not validity["TA_valid"]:
            a1["PASS_I"] = "FAIL (validity)"
        elif all(s["TA_instructions_ratio"] <= A1_PASS_I for s in sizes):
            a1["PASS_I"] = "PASS"
        elif all(s["TA_instructions_ratio"] > A1_KILL_I for s in sizes):
            a1["PASS_I"] = "KILL"
        else:
            a1["PASS_I"] = "FAIL"
        meets_c = all(s["TA_cycles_ratio"] <= A1_PASS_C for s in sizes)
        a1["PASS_C_if_the_cells_were_authoritative"] = "PASS" if meets_c else "FAIL"
        a1["PASS_C"] = (a1["PASS_C_if_the_cells_were_authoritative"]
                        if all(s["cycles_label"] == "authoritative" for s in sizes) else "NOT_DECIDABLE")
    a1["thresholds"] = {"PASS_I": A1_PASS_I, "PASS_C": A1_PASS_C, "KILL_I": A1_KILL_I}

    drive_cells = [drive_cell(p) for p in sorted(glob.glob(os.path.join(HERE, "logs", "drive_v*_*.jsonl")))]
    a2 = {"per_problem": {}}
    for family, size in WIDE:
        cells = [c for c in drive_cells if (c["family"], c["size"]) == (family, size)
                 and c["complete"] and "slices" in c["kernels"]]
        cell, auth = first_authoritative(cells, "cycles_authoritative")
        key = f"{family}{size}"
        if cell is None:
            a2["per_problem"][key] = None
            continue
        sl = cell["kernels"]["slices"]
        a2["per_problem"][key] = {
            "measurement": cell["file"], "n": cell["n"], "band": cell["band"],
            "valid": cell["counts_operations_and_state_hash_equal_in_every_run"],
            "cycles_label": "authoritative" if auth else "diagnostic_only",
            "instructions_ratio": sl["instructions_ratio_to_indexed"],
            "cycles_ratio": sl["cycles_ratio_to_indexed"],
        }
    probs = list(a2["per_problem"].values())
    if any(p is None for p in probs):
        a2.update({"PASS_I": "NOT_RUN", "PASS_C": "NOT_RUN"})
    else:
        if not all(p["valid"] for p in probs):
            a2["PASS_I"] = "FAIL (validity)"
        elif all(p["instructions_ratio"] <= A2_PASS_I for p in probs):
            a2["PASS_I"] = "PASS"
        elif all(p["instructions_ratio"] > A2_KILL_I for p in probs):
            a2["PASS_I"] = "KILL"
        else:
            a2["PASS_I"] = "FAIL"
        meets_c = all(p["cycles_ratio"] <= A2_PASS_C for p in probs)
        a2["PASS_C_if_the_cells_were_authoritative"] = "PASS" if meets_c else "FAIL"
        a2["PASS_C"] = (a2["PASS_C_if_the_cells_were_authoritative"]
                        if all(p["cycles_label"] == "authoritative" for p in probs) else "NOT_DECIDABLE")
    a2["thresholds"] = {"PASS_I": A2_PASS_I, "PASS_C": A2_PASS_C, "KILL_I": A2_KILL_I}

    result = {
        "schema": "vigilode.loop_l1.v21_banded_tight_kernel.results.v1",
        "node": "V21_BANDED_TIGHT_KERNEL",
        "generated_by": "build_results_v21.py from logs/xcheck_S*.log, logs/measure_v*_n*.jsonl and logs/drive_v*_*.jsonl",
        "check_logs": [{"file": os.path.relpath(p, HERE), "sha256": sha256(p)} for p in check_files],
        "validity": validity,
        "stage_a1": a1,
        "stage_a2": a2,
        "kernel_level_measurements": kernel_cells,
        "whole_run_measurements": drive_cells,
    }
    with open(os.path.join(HERE, "RESULTS_V21.json"), "w") as f:
        json.dump(result, f, indent=2, sort_keys=True, allow_nan=False)
        f.write("\n")
    print("RESULTS_V21.json written:", len(kernel_cells), "kernel cells,", len(drive_cells), "whole-run cells")


if __name__ == "__main__":
    main()
