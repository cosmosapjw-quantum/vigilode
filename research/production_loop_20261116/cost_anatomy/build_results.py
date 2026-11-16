#!/usr/bin/env python3
"""Build RESULTS.json of loop L1 node V04 from the raw logs only.

Inputs: PREDECLARED.json (cell table, rules), SIZING.json (attempt budgets), logs/measure/*.jsonl
(runner output of the wall protocol), logs/callgrind/*.annotate.txt and *.stdout.jsonl,
logs/perfstat/*, logs/identity/*.jsonl, logs/pilot/*.jsonl, logs/environment.json.
Every number in the output is computed here from those files; nothing is typed in.
"""
import json
import hashlib
import re
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RATIO_LIMIT = 1.02
SUM_LOW, SUM_HIGH = 0.90, 1.00
SMALL_N = 128


def strict_load(path):
    def invalid(value):
        raise ValueError("nonfinite JSON constant: " + value)

    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON key: " + key)
            result[key] = value
        return result
    return json.loads(Path(path).read_text(), object_pairs_hook=pairs, parse_constant=invalid)


def read_jsonl(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def med_min(values):
    return {"median": statistics.median(values), "min": min(values)}


def stats(values):
    out = med_min(values)
    out["max"] = max(values)
    out["values"] = list(values)
    return out


# ------------------------------------------------------------------------------------------------
# wall protocol
# ------------------------------------------------------------------------------------------------

def choose_measure_log(cell_id):
    """The log of the cell's measurement: the first run, or its single rerun when the first
    failed. Returns (path, meta, notes)."""
    base = HERE / "logs" / "measure" / (cell_id + ".jsonl")
    retry = HERE / "logs" / "measure" / (cell_id + ".retry.jsonl")
    notes = []
    if not base.exists():
        return None, None, ["no measurement log"]
    meta = strict_load(str(base) + ".meta.json")
    if meta["exit_code"] == 0:
        return base, meta, notes
    notes.append(f"first run failed with exit code {meta['exit_code']} (log kept)")
    if retry.exists():
        retry_meta = strict_load(str(retry) + ".meta.json")
        if retry_meta["exit_code"] == 0:
            notes.append("numbers are from the single rerun")
            return retry, retry_meta, notes
        notes.append(f"rerun failed with exit code {retry_meta['exit_code']}")
    return None, meta, notes


def phase_split(run, matrix_free):
    """Nanoseconds of the phases of one instrumented run."""
    total = run["wall_ns"]
    ph = run["phases"]
    if matrix_free:
        rhs = ph["rhs_ns"]
        jvp = sum(ph["jvp_ns"])
        intervals = sum(ph["solve_interval_ns"])
        return {
            "total": total,
            "rhs": rhs,
            "jvp": jvp,
            "solve_intervals": intervals,
            "solve_intervals_stage_1": ph["solve_interval_ns"][0],
            "solve_intervals_stages_2_to_8": sum(ph["solve_interval_ns"][1:]),
            "jvp_stages_2_to_8": sum(ph["jvp_ns"][1:]),
            "krylov_orthogonalisation_and_vector_work_in_solves": intervals - jvp,
            "other": ph["other_ns"],
            "remainder_task_text": total - (rhs + jvp),
            "timed_sum": rhs + jvp,
            "timed_sum_with_solve_intervals": rhs + intervals,
        }
    jac, fac, sol, rhs = ph["linearize_ns"], ph["factor_ns"], ph["solve_ns"], ph["rhs_ns"]
    timed = jac + fac + sol + rhs
    return {
        "total": total,
        "jacobian": jac,
        "factor": fac,
        "solve": sol,
        "rhs": rhs,
        "remainder": total - timed,
        "timed_sum": timed,
    }


def summarize_measure(cell, size, log, meta, notes):
    rows = read_jsonl(log)
    header = next(r for r in rows if r["type"] == "header")
    footer = next((r for r in rows if r["type"] == "footer"), None)
    runs = [r for r in rows if r["type"] == "run"]
    idents = [r for r in rows if r["type"] == "identity"]
    matrix_free = cell["driver"] == "MatrixFree"
    timed = [r for r in runs if r["kind"] == "timed"]
    plain = sorted((r for r in timed if r["arm"] == "plain"), key=lambda r: r["rep"])
    instr = sorted((r for r in timed if r["arm"] == "instrumented"), key=lambda r: r["rep"])
    expected = size["repetitions"]
    complete = (footer is not None and len(plain) == expected and len(instr) == expected
                and len(idents) == size["warmups"] + size["repetitions"])
    out = {
        "log": str(log.relative_to(HERE)),
        "log_sha256": sha256(log),
        "binary_sha256": meta["binary_sha256"],
        "process_wall_seconds": meta["wall_seconds"],
        "complete": complete,
        "notes": notes,
    }
    if not plain or not instr:
        out["complete"] = False
        return out, header

    first = plain[0]
    attempts, accepted = first["attempts"], first["accepted"]
    # Every run of the cell, warm-ups and both arms, must be the same integration.
    signature = lambda r: (r["attempts"], r["accepted"], r["rejected"], r["reuses"], r["success"],
                           r["stop_reason"], r["t_end_bits"], r["y_end_fnv64"],
                           json.dumps(r["counters"], sort_keys=True), r["fd_rhs_calls"])
    deterministic = len({signature(r) for r in runs}) == 1
    failed_fields = sorted({name for row in idents for name, ok in row["fields"].items() if not ok})
    out["run"] = {
        "attempts": attempts,
        "accepted_steps": accepted,
        "rejected_steps": first["rejected"],
        "state_reuses": first["reuses"],
        "success": first["success"],
        "stop_reason": first["stop_reason"],
        "t_end": first["t_end"],
        "y_end_fnv64": first["y_end_fnv64"],
        "y_end_finite": first["y_end_finite"],
        "lu_reported_by_protected_driver": first["lu"],
        "driver_id_plain": first["driver_id"],
        "driver_id_instrumented": instr[0]["driver_id"],
    }
    out["identity"] = {
        "pairs_checked": len(idents),
        "all_identical": bool(idents) and all(row["identical"] for row in idents),
        "failed_fields": failed_fields,
        "all_runs_same_integration": deterministic,
    }

    plain_ns = [r["wall_ns"] for r in plain]
    instr_ns = [r["wall_ns"] for r in instr]
    paired = [i["wall_ns"] / p["wall_ns"] for p, i in zip(plain, instr)]
    out["wall"] = {
        "plain_ns": stats(plain_ns),
        "instrumented_ns": stats(instr_ns),
        "plain_ns_per_attempt": med_min([v / attempts for v in plain_ns]),
        "plain_ns_per_accepted_step": med_min([v / accepted for v in plain_ns]) if accepted else None,
        "instrumented_ns_per_attempt": med_min([v / attempts for v in instr_ns]),
        "instrumented_ns_per_accepted_step": med_min([v / accepted for v in instr_ns]) if accepted else None,
        "instrumented_over_plain": {
            "ratio_of_medians": statistics.median(instr_ns) / statistics.median(plain_ns),
            "ratio_of_minima": min(instr_ns) / min(plain_ns),
            "median_of_paired_ratios": statistics.median(paired),
        },
        "plain_relative_spread": (max(plain_ns) - min(plain_ns)) / statistics.median(plain_ns),
        "instrumented_relative_spread": (max(instr_ns) - min(instr_ns)) / statistics.median(instr_ns),
    }

    splits = [phase_split(r, matrix_free) for r in instr]
    names = [k for k in splits[0] if k != "total"]
    out["phases_ns_per_attempt"] = {
        k: med_min([s[k] / attempts for s in splits]) for k in names}
    out["phases_ns_per_accepted_step"] = {
        k: med_min([s[k] / accepted for s in splits]) for k in names} if accepted else None
    out["phase_fractions"] = {
        k: statistics.median([s[k] / s["total"] for s in splits]) for k in names}
    sums = [s["timed_sum"] / s["total"] for s in splits]
    out["sum_check"] = {"median": statistics.median(sums), "min": min(sums), "max": max(sums)}
    if matrix_free:
        ext = [s["timed_sum_with_solve_intervals"] / s["total"] for s in splits]
        out["sum_check_with_solve_intervals"] = {
            "median": statistics.median(ext), "min": min(ext), "max": max(ext)}
        partition = [abs(s["rhs"] + s["solve_intervals"] + s["other"] - s["total"]) / s["total"]
                     for s in splits]
        out["matrix_free"] = {
            "timeline_partition_max_relative_defect": max(partition),
            "solve_interval_fraction_by_stage": [
                statistics.median([r["phases"]["solve_interval_ns"][i] / r["wall_ns"] for r in instr])
                for i in range(8)],
            "jvp_fraction_by_stage": [
                statistics.median([r["phases"]["jvp_ns"][i] / r["wall_ns"] for r in instr])
                for i in range(8)],
            "jvp_calls_by_stage_per_attempt": [
                instr[0]["phases"]["jvp_calls"][i] / attempts for i in range(8)],
            "attempts_seen_by_observer": instr[0]["phases"]["attempts_seen"],
        }

    counters = first["counters"]
    ph = instr[0]["phases"]
    if matrix_free:
        totals = {
            "rhs_calls": ph["rhs_calls"],
            "jvp_calls": sum(ph["jvp_calls"]),
            "stage_solves": counters.get("linear_solves", 0),
            "krylov_iterations": counters.get("linear_iterations", 0),
            "krylov_matvecs": counters.get("linear_matvecs", 0),
            "preconditioner_applications": counters.get("preconditioner_apps", 0),
            "jacobian_builds": counters.get("jacobian_builds", 0),
            "factorizations": counters.get("direct_factorizations", 0),
            "fd_rhs_calls": first["fd_rhs_calls"],
        }
        consistent = (ph["rhs_calls"] == counters.get("rhs_calls")
                      and sum(ph["jvp_calls"]) == counters.get("jvp_calls")
                      and ph["attempts_seen"] == attempts)
    else:
        totals = {
            "jacobian_builds": ph["linearize_calls"],
            "factorizations": ph["factor_calls"],
            "stage_solves": ph["solve_calls"],
            "rhs_calls": ph["rhs_calls"],
            "fd_rhs_calls": first["fd_rhs_calls"],
        }
        consistent = (ph["linearize_calls"] == counters.get("jacobian_builds")
                      and ph["factor_calls"] == counters.get("direct_factorizations")
                      and ph["solve_calls"] == counters.get("direct_solve_calls")
                      and ph["rhs_calls"] == counters.get("rhs_calls"))
    out["counts"] = {
        "totals": totals,
        "per_attempt": {k: v / attempts for k, v in totals.items()},
        "per_accepted_step": {k: v / accepted for k, v in totals.items()} if accepted else None,
        "clock_calls_equal_driver_counters": consistent,
    }
    out["load_average"] = {
        "runner_before": header["loadavg_before"],
        "runner_after": footer["loadavg_after"] if footer else None,
        "script_before": meta["loadavg_before"],
        "script_after": meta["loadavg_after"],
    }
    out["cpus_allowed_list"] = header["cpus_allowed_list"]
    out["dense_lu_mode_of_client"] = header["dense_lu_mode"]
    return out, header


# ------------------------------------------------------------------------------------------------
# callgrind
# ------------------------------------------------------------------------------------------------

ANNOTATE_LINE = re.compile(r"^\s*([\d,]+)\s+\(\s*[\d.]+%\)\s+\S*:(v04_\w+)\s")
SYMBOLS = {
    "v04_region": "region",
    "v04_phase_linearize": "jacobian",
    "v04_phase_factor": "factor",
    "v04_phase_solve": "solve",
    "v04_phase_rhs": "rhs",
    "v04_phase_jvp": "jvp",
}


def parse_annotate(path):
    found = {}
    for line in Path(path).read_text().splitlines():
        match = ANNOTATE_LINE.match(line)
        if match and match.group(2) in SYMBOLS:
            found[SYMBOLS[match.group(2)]] = int(match.group(1).replace(",", ""))
    return found


def summarize_callgrind(cell):
    base = HERE / "logs" / "callgrind"
    result = {}
    single = {}
    for arm in ("marked", "plain"):
        annotate = base / f"{cell['id']}.{arm}.annotate.txt"
        stdout = base / f"{cell['id']}.{arm}.stdout.jsonl"
        if not annotate.exists() or not stdout.exists():
            return None
        meta = strict_load(str(stdout) + ".meta.json")
        if meta["exit_code"] != 0:
            return {"error": f"callgrind run of arm {arm} failed with exit code {meta['exit_code']}"}
        rows = read_jsonl(stdout)
        run = next((r for r in rows if r["type"] == "single_run"), None)
        if run is None:
            return {"error": f"no run row for arm {arm}"}
        single[arm] = run
        result[arm] = parse_annotate(annotate)
        result[arm + "_meta"] = meta
    marked, plain = result["marked"], result["plain"]
    if "region" not in marked or "region" not in plain:
        return {"error": "v04_region not found in the callgrind annotation"}
    run = single["marked"]
    attempts = run["attempts"]
    matrix_free = cell["driver"] == "MatrixFree"
    region = marked["region"]
    phases = {k: marked.get(k, 0) for k in (("rhs", "jvp") if matrix_free
                                             else ("jacobian", "factor", "solve", "rhs"))}
    timed = sum(phases.values())
    phases["remainder"] = region - timed
    same = all(single["marked"][k] == single["plain"][k] for k in
               ("attempts", "accepted", "rejected", "t_end_bits", "y_end_fnv64", "counters"))
    return {
        "label": "instruction_counts (callgrind inclusive Ir of the marker symbols)",
        "role": result["marked_meta"]["role"],
        "attempt_budget": result["marked_meta"]["attempt_budget"],
        "attempts": attempts,
        "accepted_steps": run["accepted"],
        "region_ir_marked": region,
        "region_ir_plain": plain["region"],
        "marked_over_plain": region / plain["region"],
        "phase_ir": phases,
        "phase_ir_per_attempt": {k: v / attempts for k, v in phases.items()},
        "region_ir_per_attempt": region / attempts,
        "phase_fractions": {k: v / region for k, v in phases.items()},
        "sum_check": timed / region,
        "marked_run_identical_to_plain_run": same,
        "logs": [f"logs/callgrind/{cell['id']}.{arm}.annotate.txt" for arm in ("marked", "plain")],
    }


def summarize_perfstat(cell):
    base = HERE / "logs" / "perfstat"
    counts = {}
    for arm in ("none", "plain", "instrumented"):
        path = base / f"{cell['id']}.{arm}.perf.csv"
        if not path.exists():
            return None
        value = None
        for line in path.read_text().splitlines():
            parts = line.split(",")
            if len(parts) > 2 and parts[2].startswith("instructions"):
                try:
                    value = int(parts[0])
                except ValueError:
                    value = None
        if value is None:
            return {"error": f"no instruction count for arm {arm}"}
        counts[arm] = value
    meta = strict_load(str(base / f"{cell['id']}.plain.stdout.jsonl") + ".meta.json")
    denominator = counts["plain"] - counts["none"]
    return {
        "label": "exploratory: perf stat -e instructions:u of single runs",
        "attempt_budget": meta["attempt_budget"],
        "instructions_user": counts,
        "instrumented_over_plain_net_of_setup": (
            (counts["instrumented"] - counts["none"]) / denominator if denominator > 0 else None),
    }


def summarize_identity_sweep(cell):
    log = HERE / "logs" / "identity" / (cell["id"] + ".jsonl")
    if not log.exists():
        return None
    meta = strict_load(str(log) + ".meta.json")
    rows = read_jsonl(log)
    ident = next((r for r in rows if r["type"] == "identity"), None)
    if meta["exit_code"] != 0 or ident is None:
        return {"exit_code": meta["exit_code"], "ran": False}
    return {
        "ran": True,
        "attempt_budget": ident["attempt_budget"],
        "instrumented_identical_to_plain": ident["instrumented_vs_plain"]["identical"],
        "marked_identical_to_plain": ident["marked_vs_plain"]["identical"],
        "log": str(log.relative_to(HERE)),
    }


# ------------------------------------------------------------------------------------------------
# labels
# ------------------------------------------------------------------------------------------------

def label(cell, size, measure):
    """The admissibility of the wall numbers of a cell, by the rules of PREDECLARED.json."""
    if measure is None or not measure.get("complete") or "wall" not in measure:
        return "not_measured", ["no complete measurement"], {}
    ratio = measure["wall"]["instrumented_over_plain"]["ratio_of_medians"]
    total = measure["sum_check"]["median"]
    identity = measure["identity"]["all_identical"] and measure["identity"]["all_runs_same_integration"]
    criteria = {
        "full_protocol_pinned_interleaved_2_plus_7": (
            size["protocol"] == "full" and size["warmups"] == 2 and size["repetitions"] == 7
            and measure["cpus_allowed_list"] == "10"
            and measure["load_average"]["runner_after"] is not None),
        "i_instrumented_over_plain_le_1.02": ratio <= RATIO_LIMIT,
        "ii_phase_sum_in_0.90_1.00": SUM_LOW <= total <= SUM_HIGH,
        "iii_bit_identical_to_protected_driver": identity,
    }
    reasons = [name for name, ok in criteria.items() if not ok]
    if cell["n"] < SMALL_N:
        reasons.insert(0, "n < 128: wall is diagnostic, instruction counts are primary")
        return "diagnostic_only", reasons, criteria
    if size["protocol"] == "reduced":
        reasons.insert(0, "reduced_protocol")
    return ("authoritative" if not reasons else "diagnostic_only"), reasons, criteria


def main():
    pre = strict_load(HERE / "PREDECLARED.json")
    sizing_doc = strict_load(HERE / "SIZING.json")
    sizing = {c["cell"]: c for c in sizing_doc["cells"]}
    env_path = HERE / "logs" / "environment.json"
    cells_out = []
    for cell in pre["regime_grid"]["cells"]:
        size = sizing.get(cell["id"], {"protocol": "not_measured", "reason": "not sized"})
        entry = {
            "cell": cell["id"],
            "n": cell["n"],
            "structure": cell["structure"],
            "driver": cell["driver"],
            "problem": cell["family"],
            "problem_description": cell["problem"],
            "jacobian_source": cell["jacobian_source"],
            "rhs_multiplier": cell["rhs_multiplier"],
            "band": ({"lower": cell["band_lower"], "upper": cell["band_upper"]}
                     if cell["driver"] == "Banded" else None),
            "tolerances": {"rtol": cell["rtol"], "atol": cell["atol"],
                           "initial_step": cell["initial_step"], "t_span": cell["t_span"]},
            "exploratory": cell["exploratory"],
            "sizing": size,
            "protocol": size["protocol"],
            "reduced_protocol": size["protocol"] == "reduced",
        }
        measure = None
        if size["protocol"] in ("full", "reduced"):
            log, meta, notes = choose_measure_log(cell["id"])
            if log is not None:
                measure, header = summarize_measure(cell, size, log, meta, notes)
                entry["lu_mode"] = (
                    {"InPlace": "in_place_zero_skipping", "Faer": "faer"}.get(
                        header["dense_lu_mode"], header["dense_lu_mode"])
                    if cell["driver"] == "Dense"
                    else ("banded_indexed" if cell["driver"] == "Banded" else "matrix_free_gmres"))
                entry["matrix_free_linear_config"] = header["matrix_free_linear_config"]
            else:
                entry["measurement_notes"] = notes
        if measure is not None:
            entry.update({k: v for k, v in measure.items()})
        verdict, reasons, criteria = label(cell, size, measure)
        entry["criteria"] = criteria
        entry["admissibility"] = verdict
        entry["admissibility_reasons"] = reasons
        if cell["driver"] == "MatrixFree" and measure is not None and "sum_check_with_solve_intervals" in measure:
            extended = dict(criteria)
            total = measure["sum_check_with_solve_intervals"]["median"]
            extended["ii_phase_sum_in_0.90_1.00"] = SUM_LOW <= total <= SUM_HIGH + 1e-9
            entry["admissibility_with_solve_intervals_extension"] = (
                "authoritative" if all(extended.values()) and size["protocol"] == "full"
                else "diagnostic_only")
        instruction = summarize_callgrind(cell)
        if instruction is not None:
            entry["instruction_counts"] = instruction
        entry["fraction_basis"] = (
            "instructions" if cell["n"] < SMALL_N and instruction and "phase_fractions" in instruction
            else ("wall" if measure is not None and "phase_fractions" in measure else None))
        perf = summarize_perfstat(cell)
        if perf is not None:
            entry["perf_stat"] = perf
        sweep = summarize_identity_sweep(cell)
        if sweep is not None:
            entry["identity_sweep"] = sweep
        cells_out.append(entry)

    labels = {}
    for entry in cells_out:
        labels[entry["admissibility"]] = labels.get(entry["admissibility"], 0) + 1
    doc = {
        "schema": "vigilode.loop_l1.v04_cost_anatomy.results.v1",
        "node": "V04_STAGE_COST_ANATOMY",
        "generated_by": "build_results.py (parses logs/; no typed-in numbers)",
        "predeclared_sha256": sha256(HERE / "PREDECLARED.json"),
        "sizing_sha256": sha256(HERE / "SIZING.json"),
        "environment": strict_load(env_path) if env_path.exists() else None,
        "thresholds": {"instrumented_over_plain_max": RATIO_LIMIT,
                       "phase_sum_range": [SUM_LOW, SUM_HIGH], "small_n_below": SMALL_N},
        "units": "times in nanoseconds; fractions are of the instrumented wall of the same run",
        "summary": {
            "cells": len(cells_out),
            "by_admissibility": labels,
            "identity_failures": [e["cell"] for e in cells_out
                                  if "identity" in e and not (e["identity"]["all_identical"]
                                                             and e["identity"]["all_runs_same_integration"])],
            "identity_sweep_failures": [
                e["cell"] for e in cells_out
                if "identity_sweep" in e and not (e["identity_sweep"].get("ran")
                                                  and e["identity_sweep"]["instrumented_identical_to_plain"]
                                                  and e["identity_sweep"]["marked_identical_to_plain"])],
        },
        "cells": cells_out,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False, allow_nan=False) + "\n"
    (HERE / "RESULTS.json").write_text(text)
    print("wrote RESULTS.json:", labels)


if __name__ == "__main__":
    sys.exit(main())
