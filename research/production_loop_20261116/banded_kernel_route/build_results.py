#!/usr/bin/env python3
"""RESULTS.json of node V22 from the logs (PREDECLARED.json holds the rule).

usage: build_results.py

Reads logs/drive_s*_*.jsonl (width scan, deciding series), logs/drive_c*_*.jsonl (small-n
confirmation), logs/drive_f*_*.jsonl (D4: facade against the direct Indexed call) and
logs/drive_x*_*.jsonl (exploratory, reported and not used), with the cell evaluation of
../banded_solve/build_results_v21.py. The logs of the tests (D1 to D3) are named with their
summary lines. Nothing is measured here.
"""
import glob
import hashlib
import importlib.util
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "v21", os.path.join(HERE, "..", "banded_solve", "build_results_v21.py"))
v21 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(v21)

PRE = json.load(open(os.path.join(HERE, "PREDECLARED.json")))
SCAN = {
    "acstrip": {w: w for w in PRE["stage_a_width_scan"]["families"]["acstrip"]["sizes_w"]},
    "brstrip": {m: 2 * m for m in PRE["stage_a_width_scan"]["families"]["brstrip"]["sizes_m"]},
}
QUALIFY, TREND, CONFIRM = 0.95, 0.03, 1.00
D4_PASS_I, D4_PASS_C = 0.60, 0.75
D4_KEYS = ("attempts", "accepted", "rejected", "state_hash")


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def cells(tag, **kwargs):
    out = []
    for path in sorted(glob.glob(os.path.join(HERE, "logs", f"drive_{tag}*_*.jsonl"))):
        cell = v21.drive_cell(path, **kwargs)
        cell["file"] = os.path.relpath(path, HERE)
        out.append(cell)
    return out


def ratio_entry(cell, auth):
    slices = cell["kernels"]["slices"]
    return {
        "measurement": cell["file"], "n": cell["n"], "attempts": slices["attempts_of_the_long_run"],
        "instructions_ratio": slices["instructions_ratio_to_indexed"],
        "cycles_ratio": slices["cycles_ratio_to_indexed"],
        "cycles_label": "authoritative" if auth else "diagnostic_only",
        "load_before": cell["load_before"], "load_after": cell["load_after"],
        "cycles_spread_indexed": cell["kernels"]["indexed"]["cycles_spread"],
        "cycles_spread_slices": slices["cycles_spread"],
    }


def scan_table(all_cells):
    """Per family and half band width: the first authoritative measurement, else the last."""
    table = {}
    for family, sizes in SCAN.items():
        for size, width in sizes.items():
            taken = sorted((c for c in all_cells if (c["family"], c["size"]) == (family, size)),
                           key=lambda c: c["index"])
            # At most three measurements of a cell count (one and two re-measurements); a
            # measurement without both kernels (interrupted) counts as taken and cannot decide.
            mine = [c for c in taken[:3] if "slices" in c["kernels"] and "indexed" in c["kernels"]]
            cell, auth = v21.first_authoritative(mine, "cycles_authoritative")
            if cell is None:
                continue
            entry = ratio_entry(cell, auth)
            entry["measurements_taken"] = len(taken)
            entry["measurements_beyond_the_third_ignored"] = max(0, len(taken) - 3)
            entry["qualifies"] = bool(auth and entry["instructions_ratio"] <= QUALIFY
                                      and entry["cycles_ratio"] <= QUALIFY)
            table.setdefault(width, {})[family] = entry
    return table


def run_validity(scan):
    """The validity rule of PREDECLARED.json over every recorded run of the series.

    A difference is a budget of one log at which the runs of the two kernels do not all print
    the same attempts, accepted and rejected steps, counted factor and solve operations and
    state hash. A measurement that was interrupted (no footer) is compared in the runs it
    holds; a budget that only one kernel reached there has no pair and is listed, it is not a
    difference. A complete log must hold both kernels at both budgets and no failed run.
    """
    keys = ("attempts", "accepted", "rejected", "factor_ops", "solve_ops", "state_hash")
    differences, defective, interrupted, pairs = [], [], [], 0
    for cell in scan:
        path = os.path.join(HERE, cell["file"])
        header, footer, runs = v21.read_lines(path)
        per, failed = {}, 0
        for rec in runs:
            if rec["exit"] != 0 or not rec["stdout"].startswith("DRIVE "):
                failed += 1
                continue
            fields = dict(tok.split("=", 1) for tok in rec["stdout"].split()[1:])
            per.setdefault(rec["budget"], {}).setdefault(rec["kernel"], set()).add(
                tuple(fields[k] for k in keys))
        paired = {b: sig for b, sig in per.items() if len(sig) == len(header["kernels"])}
        pairs += len(paired)
        differing = sorted(b for b, sig in paired.items() if len(set().union(*sig.values())) != 1)
        unstable = sorted(b for b, sig in per.items() if any(len(s) != 1 for s in sig.values()))
        if differing or unstable:
            differences.append({"file": cell["file"], "budgets_where_the_kernels_differ": differing,
                                "budgets_where_one_kernel_differs_between_its_runs": unstable})
        if footer is None:
            interrupted.append({"file": cell["file"], "runs_recorded": len(runs),
                                "budgets_compared": sorted(paired),
                                "budgets_without_a_pair": sorted(set(per) - set(paired))})
        elif failed or len(paired) != 2:
            defective.append({"file": cell["file"], "failed_runs": failed,
                              "budgets_compared": sorted(paired)})
    return {"holds": bool(scan) and not differences and not defective,
            "budget_pairs_compared": pairs, "differences": differences,
            "complete_logs_with_a_failed_run_or_a_missing_pair": defective,
            "interrupted_measurements": interrupted}


def choose_width(table):
    widths = sorted(table)
    expected = sorted({w for sizes in SCAN.values() for w in sizes.values()})
    complete = widths == expected and all(
        set(table[w]) == {f for f, sizes in SCAN.items() if w in sizes.values()} for w in widths)
    qualifies = {w: all(e["qualifies"] for e in table[w].values()) for w in widths}
    chosen = None
    for w in widths:
        if all(qualifies[v] for v in widths if v >= w):
            chosen = w
            break
    rises = []
    if chosen is not None:
        for family in SCAN:
            series = [(w, table[w][family]) for w in widths if w >= chosen and family in table[w]]
            for (w0, e0), (w1, e1) in zip(series, series[1:]):
                for key in ("instructions_ratio", "cycles_ratio"):
                    if e1[key] - e0[key] > TREND:
                        rises.append({"family": family, "from_width": w0, "to_width": w1,
                                      "ratio": key, "rise": e1[key] - e0[key]})
    return complete, qualifies, chosen, rises


def d4_entry(cell):
    k = cell["kernels"]
    if "direct" not in k or "facade" not in k:
        return None
    auth = bool(cell["load_ok"] and all(k[a]["cycles_spread"] <= v21.MAX_SPREAD
                                        for a in ("direct", "facade")))
    return {
        "measurement": cell["file"], "family": cell["family"], "size": cell["size"], "n": cell["n"],
        "band": cell["band"], "index": cell["index"],
        "attempts": k["facade"]["attempts_of_the_long_run"],
        "equal_counts_and_state_hash": cell["counts_operations_and_state_hash_equal_in_every_run"],
        "instructions_ratio_facade_to_direct":
            k["facade"]["instructions_per_attempt"] / k["direct"]["instructions_per_attempt"],
        "cycles_ratio_facade_to_direct":
            k["facade"]["cycles_per_attempt"] / k["direct"]["cycles_per_attempt"],
        "cycles_label": "authoritative" if auth else "diagnostic_only",
        "load_before": cell["load_before"], "load_after": cell["load_after"],
    }


def test_log(name, pattern):
    path = os.path.join(HERE, "logs", name)
    if not os.path.exists(path):
        return None
    with open(path) as f:
        lines = [line.rstrip("\n") for line in f]
    return {"file": os.path.join("logs", name), "sha256": sha256(path),
            "lines": [line for line in lines if re.search(pattern, line)]}


def main():
    scan = cells("s")
    table = scan_table(scan)
    complete, qualifies, width, rises = choose_width(table)
    validity = run_validity(scan)
    valid = validity["holds"]
    stage_a = {
        "cells_measured": len(scan), "scan_complete": complete,
        "validity_equal_counts_operations_and_state_hash_in_every_run": valid,
        "validity_detail": validity,
        "by_half_band_width": {str(w): table[w] for w in sorted(table)},
        "width_qualifies": {str(w): q for w, q in qualifies.items()},
        "W": width, "rises_above_the_trend_bound": rises,
        "thresholds": {"qualify_instructions_and_cycles": QUALIFY, "trend": TREND},
    }

    confirmation = None
    if width is not None:
        size = next((s for s in (8, 16, 32) if s >= width), None)
        # AMENDMENT_1.json: the confirmation problem is repeated 200 times per process (ac2dr).
        mine = [c for c in cells("c") if (c["family"], c["size"]) == ("ac2dr", size)]
        cell, auth = v21.first_authoritative(mine, "cycles_authoritative")
        confirmation = {"problem": f"ac2d {size}, 200 integrations per process (ac2dr)",
                        "measurements_taken": len(mine)}
        if cell is not None:
            entry = ratio_entry(cell, auth)
            entry["equal_counts_operations_and_state_hash_in_every_run"] = \
                cell["counts_operations_and_state_hash_equal_in_every_run"]
            confirmation.update(entry)
            confirmation["holds"] = bool(
                auth and entry["instructions_ratio"] <= CONFIRM and entry["cycles_ratio"] <= CONFIRM
                and entry["equal_counts_operations_and_state_hash_in_every_run"])

    d4_cells = [e for e in (d4_entry(c) for c in cells("f", keys=D4_KEYS, other="facade")) if e]
    d4 = {"thresholds": {"instructions": D4_PASS_I, "cycles": D4_PASS_C}, "measurements": d4_cells}
    if d4_cells:
        chosen = next((e for e in d4_cells if e["cycles_label"] == "authoritative"), d4_cells[-1])
        d4["deciding_measurement"] = chosen["measurement"]
        d4["identity"] = all(e["equal_counts_and_state_hash"] for e in d4_cells)
        d4["PASS_I"] = bool(d4["identity"]
                            and chosen["instructions_ratio_facade_to_direct"] <= D4_PASS_I)
        d4["PASS_C"] = ("NOT_DECIDABLE" if chosen["cycles_label"] != "authoritative"
                        else "PASS" if chosen["cycles_ratio_facade_to_direct"] <= D4_PASS_C
                        else "FAIL")

    tests = {
        "D1_identity_of_the_twins": test_log("d1_identity.log", r"^V22_D1_|^test |test result"),
        "D2_facade_bits_and_labels": test_log("d2_facade.log", r"^V22_D2_|^test |test result|^D2_"),
        "D3_no_change_elsewhere": test_log("d3_summary.log", r"exit=|test result|^D3_"),
    }

    exploratory = []
    for cell in cells("x"):
        if "slices" in cell["kernels"] and "indexed" in cell["kernels"]:
            entry = ratio_entry(cell, False)
            entry.update({"family": cell["family"], "half_band_width": cell["band"],
                          "equal_counts_operations_and_state_hash_in_every_run":
                              cell["counts_operations_and_state_hash_equal_in_every_run"]})
            exploratory.append(entry)

    results = {
        "schema": "vigilode.loop-l1.v22-results.v1",
        "node": "V22_FACADE_BANDED_KERNEL",
        "generated_by": "build_results.py from logs/drive_[scfx]*_*.jsonl and the test logs",
        "predeclared_sha256": sha256(os.path.join(HERE, "PREDECLARED.json")),
        "stage_a_width_scan": stage_a,
        "small_n_confirmation": confirmation,
        "stage_b": {"D4_facade_speed": d4, "tests": tests},
        "exploratory_series_not_used_for_the_decision": exploratory,
    }
    with open(os.path.join(HERE, "RESULTS.json"), "w") as f:
        f.write(json.dumps(results, indent=2, sort_keys=True) + "\n")
    print("RESULTS.json written: scan cells", len(scan), "W", width,
          "confirmation", None if not confirmation else confirmation.get("holds"),
          "D4", d4.get("PASS_I"), d4.get("PASS_C"))


if __name__ == "__main__":
    main()
