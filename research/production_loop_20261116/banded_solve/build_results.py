#!/usr/bin/env python3
"""Builds RESULTS.json of loop L1 node V19 (stage a) from the raw logs only.

Inputs: logs/check_S*.log (CHECK lines of the measurement program) and logs/measure_m*_n*.jsonl
(lines of measure.py). Rules: PREDECLARED.json (validity_rules, cost, protocol, verdict_rules).
No number is typed in here; thresholds are the frozen ones of the pre-registration addendum A3.
"""
import glob
import hashlib
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
DECLARED = ["B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8"]
B_FAMILY = ["B", "BV"]
C_FAMILY = ["C2", "C4", "C8", "CV2", "CV4", "CV8"]
ETA_V2, ETA_V3 = 1e-14, 1e-12
MAX_LOAD, MAX_SPREAD = 8.0, 0.10
DECIDING = [10000, 100000]
PASS_A0, PASS_A1, KILL_A1, KEEP_PARTITION = 0.50, 0.67, 0.77, 0.77


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def fnum(text):
    return None if text == "NA" else float(text)


def jnum(value):
    """A float as strict JSON (non-finite values become strings)."""
    if value is None:
        return None
    if value != value:
        return "nan"
    if value in (float("inf"), float("-inf")):
        return "inf"
    return value


def parse_checks():
    cases, files = {}, []
    for path in sorted(glob.glob(os.path.join(HERE, "logs", "check_S*.log"))):
        files.append(path)
        with open(path) as f:
            for line in f:
                if not line.startswith("CHECK "):
                    continue
                kv = dict(tok.split("=", 1) for tok in line.split()[1:])
                cases.setdefault((kv["suite"], kv["case"]), {})[kv["arm"]] = kv
    return cases, files


def eta_ok(row, limit):
    eta = fnum(row["eta"])
    return eta is not None and eta <= limit


def validity(cases):
    arms = sorted({arm for rows in cases.values() for arm in rows})
    v1_cases = [k for k in cases if k[0] in ("S1", "S2", "S3")]
    v1_failures = []
    for key in v1_cases:
        a1 = cases[key]["A1"]
        same = a1["bits_a0"] == "1" if a1["status"] == "ok" else a1["ref_ok"] == "0"
        if not same:
            v1_failures.append("/".join(key))
    excluded = sorted(
        "/".join(k) for k, rows in cases.items()
        if k[0] in ("S2", "S3", "S4") and rows["A0"]["status"] == "ok"
        and not eta_ok(rows["A0"], ETA_V3))
    out = {"V1": {"cases": len(v1_cases), "failures": v1_failures, "pass": not v1_failures},
           "excluded_for_V3": excluded, "arms": {}}
    for arm in arms:
        s1 = [rows[arm] for k, rows in sorted(cases.items()) if k[0] == "S1" and arm in rows]
        s1_bad = [r["case"] for r in s1 if r["status"] != "ok" or not eta_ok(r, ETA_V2)]
        s1_etas = [fnum(r["eta"]) for r in s1 if r["status"] == "ok"]
        v3_rows = [(k, rows[arm]) for k, rows in sorted(cases.items())
                   if k[0] in ("S2", "S3", "S4") and "/".join(k) not in excluded and arm in rows]
        silent = ["/".join(k) + ":eta=" + r["eta"] for k, r in v3_rows
                  if r["status"] == "ok" and not eta_ok(r, ETA_V3)]
        errors = {}
        for k, r in v3_rows:
            if r["status"] != "ok":
                errors.setdefault(k[0], []).append(r["case"] + ":" + r["kind"])
        ok_etas = [fnum(r["eta"]) for k, r in v3_rows if r["status"] == "ok"]
        ok_ferr = [fnum(r["ferr"]) for k, r in v3_rows
                   if r["status"] == "ok" and r["ferr"] != "NA"]
        entry = {
            "V2": {"cases": len(s1), "failures": s1_bad, "pass": bool(s1) and not s1_bad,
                   "worst_eta": jnum(max(s1_etas)) if s1_etas else None},
            "V3": {"cases": len(v3_rows), "silent_failures": silent, "pass": not silent,
                   "error_returns": errors,
                   "worst_eta_without_error": jnum(max(ok_etas)) if ok_etas else None,
                   "worst_forward_error_without_error": jnum(max(ok_ferr)) if ok_ferr else None},
        }
        entry["valid"] = entry["V2"]["pass"] and entry["V3"]["pass"]
        out["arms"][arm] = entry
    explor = {}
    for k, rows in sorted(cases.items()):
        if k[0] in ("S4", "S5"):
            explor.setdefault(k[0], {})[k[1]] = {
                arm: (r["status"] if r["status"] == "ok" else "err:" + r["kind"])
                + ("" if r["status"] != "ok" else ":eta=" + r["eta"] + ":ferr=" + r["ferr"])
                for arm, r in sorted(rows.items())}
    out["S4_S5_outcomes"] = explor
    return out


def parse_measurement(path):
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
    if header is None:
        return None
    r_pat = header["patterns"]
    arms = {}
    for arm in header["arms"]:
        per_round = {}
        problems = 0
        for rec in runs:
            if rec["arm"] != arm or rec["warmup"]:
                continue
            if rec["exit"] != 0 or "errors=0" not in rec["stdout"] or "cycles" not in rec:
                problems += 1
            per_round.setdefault(rec["round"], {})[(rec["patterns"], rec["solves"])] = rec
        pattern, factor, solve, instr, clock = [], [], [], [], []
        for rnd in sorted(per_round):
            trio = per_round[rnd]
            if len(trio) != 3 or any("cycles" not in r for r in trio.values()):
                continue
            c00, cr0, cr8 = trio[(0, 0)], trio[(r_pat, 0)], trio[(r_pat, 8)]
            pattern.append((cr8["cycles"] - c00["cycles"]) / r_pat)
            factor.append((cr0["cycles"] - c00["cycles"]) / r_pat)
            solve.append((cr8["cycles"] - cr0["cycles"]) / (8 * r_pat))
            instr.append((cr8["instructions"] - c00["instructions"]) / r_pat)
            clock.append((cr8["task_clock_ms"] - c00["task_clock_ms"]) / r_pat)
        if not pattern:
            continue
        med = statistics.median(pattern)
        arms[arm] = {
            "rounds": len(pattern),
            "runs_with_problems": problems,
            "cycles_per_pattern": med,
            "spread": (max(pattern) - min(pattern)) / med,
            "cycles_per_factorisation": statistics.median(factor),
            "cycles_per_solve": statistics.median(solve),
            "instructions_per_pattern": statistics.median(instr),
            "task_clock_ms_per_pattern": statistics.median(clock),
        }
    for arm, entry in arms.items():
        for base in ("A0", "A1"):
            if base in arms:
                entry["ratio_to_" + base] = entry["cycles_per_pattern"] / arms[base]["cycles_per_pattern"]
                entry["solve_ratio_to_" + base] = entry["cycles_per_solve"] / arms[base]["cycles_per_solve"]
                entry["factorisation_ratio_to_" + base] = (
                    entry["cycles_per_factorisation"] / arms[base]["cycles_per_factorisation"])
    complete = footer is not None
    loads = [header["load_before"], footer["load_after"]] if complete else [header["load_before"]]
    return {
        "file": os.path.relpath(path, HERE), "sha256": sha256(path), "n": header["n"],
        "lu": header["lu"], "index": header["index"], "patterns": r_pat, "complete": complete,
        "load_before": header["load_before"],
        "load_after": footer["load_after"] if complete else None,
        "load_max_during_runs": max([r["load"] for r in runs] + loads),
        "core_busy_fraction": footer.get(f"cpu{header['core']}_busy_fraction") if complete else None,
        "sibling_busy_fraction": footer.get(f"cpu{header['sibling']}_busy_fraction") if complete else None,
        "load_ok": complete and max(loads) <= MAX_LOAD,
        "arms": arms,
    }


def best(meas, family, valid):
    cands = [(meas["arms"][a]["cycles_per_pattern"], a) for a in family
             if a in meas["arms"] and valid.get(a)]
    return min(cands)[1] if cands else None


def authoritative(meas, arms):
    return (meas["load_ok"] and all(a in meas["arms"] for a in arms)
            and all(meas["arms"][a]["spread"] <= MAX_SPREAD for a in arms))


def decide(measurements, valid):
    per_n, reasons = {}, []
    for n in DECIDING:
        series = sorted((m for m in measurements if m["n"] == n and m["lu"] is None and m["complete"]),
                        key=lambda m: m["index"])
        chosen, label = None, "diagnostic_only"
        for m in series:
            top = best(m, DECLARED, valid)
            if top and authoritative(m, ["A0", "A1", top]):
                chosen, label = m, "authoritative"
                break
        if chosen is None and series:
            chosen = series[-1]
        if chosen is None:
            per_n[str(n)] = None
            reasons.append(f"n={n}: no complete measurement")
            continue
        top = best(chosen, DECLARED, valid)
        top_b, top_c = best(chosen, B_FAMILY, valid), best(chosen, C_FAMILY, valid)
        arms = chosen["arms"]
        entry = {
            "measurement": chosen["file"], "label": label,
            "best_valid_arm": top,
            "ratio_to_A0": arms[top]["ratio_to_A0"] if top else None,
            "ratio_to_A1": arms[top]["ratio_to_A1"] if top else None,
            "meets_pass": bool(top) and arms[top]["ratio_to_A0"] <= PASS_A0
            and arms[top]["ratio_to_A1"] <= PASS_A1,
            "all_valid_above_kill": all(arms[a]["ratio_to_A1"] > KILL_A1 for a in DECLARED
                                        if a in arms and valid.get(a)),
            "kill_comparison_authoritative": authoritative(
                chosen, ["A1"] + [a for a in DECLARED if a in arms and valid.get(a)]),
            "best_B_family": top_b, "best_C_family": top_c,
            "C_over_B": (arms[top_c]["cycles_per_pattern"] / arms[top_b]["cycles_per_pattern"]
                         if top_b and top_c else None),
            "second_question_authoritative": bool(top_b and top_c)
            and authoritative(chosen, [top_b, top_c]),
        }
        per_n[str(n)] = entry
    entries = [per_n.get(str(n)) for n in DECIDING]
    if any(e is None for e in entries):
        stage_a = "NOT_RUN"
        diagnostic = None
        second = "NOT_RUN"
        second_diag = None
    else:
        would_pass = all(e["meets_pass"] for e in entries)
        would_kill = all(e["all_valid_above_kill"] for e in entries)
        diagnostic = "PASS" if would_pass else ("KILL" if would_kill else "FAIL")
        if would_pass:
            decidable = all(e["label"] == "authoritative" for e in entries)
        elif would_kill:
            decidable = all(e["kill_comparison_authoritative"] for e in entries)
        else:
            decidable = all(e["label"] == "authoritative" and e["kill_comparison_authoritative"]
                            for e in entries)
        stage_a = diagnostic if decidable else "NOT_DECIDABLE"
        if all(e["C_over_B"] is not None for e in entries):
            keep = all(e["C_over_B"] <= KEEP_PARTITION for e in entries)
            second_diag = "partitioning kept" if keep else "sequential kernel alone"
            second = second_diag if all(e["second_question_authoritative"] for e in entries) \
                else "NOT_DECIDABLE"
        else:
            second_diag, second = None, "NOT_DECIDABLE"
    return {"per_size": per_n, "stage_a": stage_a, "stage_a_if_the_cells_were_authoritative": diagnostic,
            "second_question": second, "second_question_if_authoritative": second_diag,
            "thresholds": {"pass_ratio_to_A0": PASS_A0, "pass_ratio_to_A1": PASS_A1,
                           "kill_ratio_to_A1": KILL_A1, "keep_partition_C_over_B": KEEP_PARTITION,
                           "max_load": MAX_LOAD, "max_spread": MAX_SPREAD},
            "notes": reasons}


def main():
    cases, check_files = parse_checks()
    val = validity(cases) if cases else None
    valid = {a: e["valid"] for a, e in val["arms"].items()} if val else {}
    if val and not val["V1"]["pass"]:
        valid = {a: False for a in valid}
    measurements = []
    for path in sorted(glob.glob(os.path.join(HERE, "logs", "measure_m*_n*.jsonl"))):
        parsed = parse_measurement(path)
        if parsed:
            measurements.append(parsed)
    frozen = [m for m in measurements if m["lu"] is None and m["n"] in (1000, 10000, 100000)]
    explor = [m for m in measurements if m not in frozen]
    result = {
        "schema": "vigilode.loop_l1.v19_banded_solve.results.v1",
        "node": "V19_BANDED_SOLVE_KERNEL",
        "stage": "a",
        "generated_by": "build_results.py from logs/check_S*.log and logs/measure_m*_n*.jsonl",
        "check_logs": [{"file": os.path.relpath(p, HERE), "sha256": sha256(p)} for p in check_files],
        "validity": val,
        "valid_arms": sorted(a for a, ok in valid.items() if ok),
        "measurements": frozen,
        "verdicts": decide(frozen, valid) if frozen else None,
        "exploratory_measurements": explor,
    }
    with open(os.path.join(HERE, "RESULTS.json"), "w") as f:
        json.dump(result, f, indent=2, sort_keys=True, allow_nan=False)
        f.write("\n")
    print("RESULTS.json written:", len(cases), "check cases,", len(measurements), "measurement files")


if __name__ == "__main__":
    main()
