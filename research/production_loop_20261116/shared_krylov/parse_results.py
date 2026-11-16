#!/usr/bin/env python3
"""Loop node V06: build RESULTS.json from the raw logs only.

Reads logs/<name>.log (V06_META, V06_RUN_JSON, V06_STATE_JSON lines written by
run_measurements.sh and the measurement program), logs/<name>.perf (perf stat
CSV), reference/<family>.json (make_reference.py) and PREDECLARED.json. No
measured number is typed in here: the only literals are the frozen thresholds
of PREREGISTRATION.md (row V06) and the matching band of PREDECLARED.json.

usage: python3 -I parse_results.py            writes RESULTS.json, prints a summary
"""
import json
import math
import statistics
import struct
import sys
from pathlib import Path

NODE = Path(__file__).resolve().parent
LOGS = NODE / "logs"

# Frozen (PREREGISTRATION.md, row V06).
STAGE_A_JVP_RATIO = 1.5
STAGE_A_STEP_RATIO = 1.05
STAGE_A_FAMILIES_NEEDED = 2
STAGE_A_CANCEL_MEDIAN = 1.2
FINAL_JVP_FRACTION = 0.5
FINAL_INSTRUCTION_FRACTION = 0.8
FINAL_FAMILIES_NEEDED = 3
# PREDECLARED.json, definitions.final_verdict_if_reached.
FINAL_ERROR_BAND = 1.25


def strict_load(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError(f"duplicate key {key} in {path}")
            result[key] = value
        return result

    def constant(value):
        raise ValueError(f"non-finite constant {value} in {path}")

    return json.loads(path.read_text(), object_pairs_hook=pairs, parse_constant=constant)


def decode_state(text):
    return [struct.unpack(">d", bytes.fromhex(text[i : i + 16]))[0] for i in range(0, len(text), 16)]


def read_log(path):
    meta, run, state = {}, None, None
    for line in path.read_text(errors="replace").splitlines():
        if line.startswith("V06_META "):
            for item in line.split()[1:]:
                key, _, value = item.partition("=")
                meta[key] = value
        elif line.startswith("V06_RUN_JSON "):
            run = json.loads(line.split(" ", 1)[1])
        elif line.startswith("V06_STATE_JSON "):
            state = json.loads(line.split(" ", 1)[1])
    return meta, run, state


def read_perf(path):
    if not path.exists():
        return None
    for line in path.read_text().splitlines():
        fields = line.split(",")
        if len(fields) > 2 and fields[2].startswith("instructions"):
            try:
                return int(fields[0])
            except ValueError:
                return None
    return None


def finite(value):
    return value if isinstance(value, (int, float)) and math.isfinite(value) else None


def ratio(numerator, denominator):
    if numerator is None or denominator is None or denominator == 0:
        return None
    return finite(numerator / denominator)


def error_against(reference, state, rtol):
    ref = reference["values"]
    if len(ref) != len(state):
        return None
    atol = rtol * reference["atol_over_rtol"]
    differences = [abs(a - b) for a, b in zip(state, ref)]
    return {
        "max_tolerance_units": finite(max(d / (atol + rtol * abs(r)) for d, r in zip(differences, ref))),
        "max_abs": finite(max(differences)),
        "rms": finite(math.sqrt(sum(d * d for d in differences) / len(differences))),
    }


COUNTER_KEYS = [
    "jvp_calls",
    "linear_iterations",
    "linear_matvecs",
    "diagnostic_matvecs",
    "linear_solves",
    "rhs_calls",
    "jacobian_builds",
    "preconditioner_apps",
    "accepted_steps",
    "rejected_steps",
    "local_error_failures",
    "linear_solve_failures",
    "nonfinite_step_failures",
]


def load_runs(references):
    runs, states = {}, {}
    for path in sorted(LOGS.glob("*.log")):
        name = path.stem
        if not (name.startswith("a_") or name.startswith("x_") or name.startswith("f_")):
            continue
        meta, run, state = read_log(path)
        record = {
            "name": name,
            "log": f"logs/{path.name}",
            "family": meta.get("family"),
            "rtol": float(meta["rtol"]) if "rtol" in meta else None,
            "arm": meta.get("arm"),
            "precond": meta.get("precond"),
            "audit": meta.get("audit") == "1",
            "max_dim": int(meta["max_dim"]) if meta.get("arm") in ("C", "C2") and "max_dim" in meta else None,
            "keep": meta.get("keep") == "1",
            "exit_status": int(meta["exit_status"]) if "exit_status" in meta else None,
            "start": meta.get("start"),
            "end": meta.get("end"),
            "loadavg_before": meta.get("loadavg_before"),
            "loadavg_after": meta.get("loadavg_after"),
            "instructions_user": read_perf(LOGS / f"{name}.perf"),
            "program_output_found": run is not None,
        }
        if record["exit_status"] == 124:
            record["status"] = "NOT_COMPLETED_TIME_LIMIT"
        elif run is None or record["exit_status"] != 0:
            record["status"] = "NO_OUTPUT"
        elif run.get("setup_only"):
            record["status"] = "SETUP"
        elif not run.get("completed"):
            record["status"] = "ERROR"
            record["error"] = run.get("error")
        else:
            record["status"] = "OK" if run.get("success") else "STOPPED_BEFORE_T1"
        if run is not None and run.get("completed"):
            counters = run["counters"]
            record.update(
                {
                    "n": run["n"],
                    "atol": run["atol"],
                    "success": run["success"],
                    "attempts": run["attempts"],
                    "accepted": run["accepted"],
                    "rejected": run["rejected"],
                    "t_end": run["t_end"],
                    "jvp_calls": run["jvp_calls"],
                    "counters": {key: counters.get(key, 0) for key in COUNTER_KEYS},
                    "counters_all": counters,
                    "wall_seconds_diagnostic_only": finite(run.get("wall_seconds_diagnostic_only")),
                    "state_sha256": run.get("state_sha256"),
                }
            )
            if "client" in run:
                record["client"] = run["client"]
                record["jvp_accounting_closed"] = run.get("jvp_accounting_closed")
            if "residual_audit" in run:
                record["residual_audit"] = run["residual_audit"]
            if state is not None:
                values = decode_state(state["state_hex"])
                states[name] = values
                reference = references.get(record["family"])
                if reference is not None and run["success"]:
                    record["error"] = error_against(reference, values, run["rtol"])
        elif run is not None:
            record["n"] = run.get("n")
        runs[name] = record
    return runs, states


def same_run(first, second):
    """Two runs of one arm (audit on and off): identical arm work and state."""
    if first is None or second is None:
        return None
    if first["status"] != "OK" or second["status"] != "OK":
        return None
    return (
        first["counters_all"] == second["counters_all"]
        and first["state_sha256"] == second["state_sha256"]
        and first.get("client") == second.get("client")
        and (first["attempts"], first["accepted"], first["rejected"])
        == (second["attempts"], second["accepted"], second["rejected"])
    )


def audit_clean(run):
    if run is None or "residual_audit" not in run:
        return None
    audit = run["residual_audit"]
    return audit["failures"] == 0 and audit["checks"] > 0


def client_summary(run, setup):
    if run is None or run["status"] != "OK":
        return {"status": None if run is None else run["status"]}
    out = {
        "status": run["status"],
        "run": run["name"],
        "attempts": run["attempts"],
        "accepted": run["accepted"],
        "rejected": run["rejected"],
        "linear_solve_failures": run["counters"]["linear_solve_failures"],
        "local_error_failures": run["counters"]["local_error_failures"],
        "jvp": run["jvp_calls"],
        "error": run.get("error"),
        "instructions_user": run["instructions_user"],
        "wall_seconds_diagnostic_only": run["wall_seconds_diagnostic_only"],
    }
    if setup is not None and setup.get("instructions_user") and run["instructions_user"]:
        out["instructions_user_net_of_setup"] = run["instructions_user"] - setup["instructions_user"]
    client = run.get("client")
    if client:
        attempts = max(client["attempts"], 1)
        out.update(
            {
                "krylov_vectors": client["krylov_vectors"],
                "stopping_test_residual_jvps": client["residual_jvps"],
                "jvp_accounting_closed": run.get("jvp_accounting_closed"),
                "solves": client["solves"],
                "failed_solves": client["failed_solves"],
                "krylov_vectors_per_solve_mean": ratio(client["krylov_vectors"], client["solves"]),
                "krylov_vectors_per_solve_max": client["max_solve_vectors"],
                "restart_passes": client["restart_passes"],
                "zero_iterate_accepts": client["zero_iterate_accepts"],
                "krylov_vectors_per_attempt_by_stage": [
                    finite(value / attempts) for value in client["stage_krylov_vectors"]
                ],
                "gmres_storage_bytes": client["gmres_storage_bytes"],
            }
        )
        if client["shared_storage_bytes"]:
            out.update(
                {
                    "basis_dim_max": client["basis_dim_max"],
                    "basis_dim_mean_at_attempt_end": ratio(client["basis_dim_sum"], client["basis_dim_attempts"]),
                    "shared_accepted_solves": client["shared_accepted_solves"],
                    "fallback_solves": client["fallback_solves"],
                    "fallback_krylov_vectors": client["fallback_krylov_vectors"],
                    "confirmation_failures": client["confirmation_failures"],
                    "basis_kept": client["basis_kept"],
                    "shared_storage_bytes": client["shared_storage_bytes"],
                }
            )
    if "residual_audit" in run:
        out["residual_audit"] = run["residual_audit"]
    return out


def max_abs_difference(states, first, second):
    if first not in states or second not in states or len(states[first]) != len(states[second]):
        return None
    return finite(max(abs(a - b) for a, b in zip(states[first], states[second])))


def compare(runs, states, prefix, baseline_tag, candidate_tag, setup, audited_only=False):
    """JVP and step ratios of a candidate client arm against a baseline client arm."""
    b_audit = runs.get(f"{prefix}_{baseline_tag}_audit1")
    b_plain = runs.get(f"{prefix}_{baseline_tag}_audit0")
    c_audit = runs.get(f"{prefix}_{candidate_tag}_audit1")
    c_plain = runs.get(f"{prefix}_{candidate_tag}_audit0")
    b = b_audit if b_audit is not None else b_plain
    c = c_audit if c_audit is not None else c_plain
    out = {
        "baseline": client_summary(b, setup),
        "candidate": client_summary(c, setup),
    }
    ok = b is not None and c is not None and b["status"] == "OK" and c["status"] == "OK"
    out["both_completed"] = ok
    if ok:
        out["jvp_ratio_baseline_over_candidate"] = ratio(b["jvp_calls"], c["jvp_calls"])
        out["step_ratio_candidate_over_baseline"] = ratio(c["attempts"], b["attempts"])
        out["krylov_vector_ratio_baseline_over_candidate_exploratory"] = ratio(
            b["client"]["krylov_vectors"], c["client"]["krylov_vectors"]
        )
        if b.get("error") and c.get("error"):
            out["error_ratio_candidate_over_baseline"] = ratio(
                c["error"]["max_tolerance_units"], b["error"]["max_tolerance_units"]
            )
        out["final_state_max_abs_difference"] = max_abs_difference(states, b["name"], c["name"])
    out["residual_audit_clean"] = {"baseline": audit_clean(b_audit), "candidate": audit_clean(c_audit)}
    if not audited_only:
        out["audit_on_equals_audit_off"] = {
            "baseline": same_run(b_audit, b_plain),
            "candidate": same_run(c_audit, c_plain),
        }
        if b_plain is not None and c_plain is not None and b_plain["status"] == "OK" and c_plain["status"] == "OK":
            raw = ratio(c_plain["instructions_user"], b_plain["instructions_user"])
            out["instructions_user_audit_off"] = {
                "baseline": b_plain["instructions_user"],
                "candidate": c_plain["instructions_user"],
                "candidate_over_baseline": raw,
            }
            if setup is not None and setup.get("instructions_user"):
                base = setup["instructions_user"]
                out["instructions_user_audit_off"]["setup"] = base
                out["instructions_user_audit_off"]["candidate_over_baseline_net_of_setup"] = ratio(
                    (c_plain["instructions_user"] or 0) - base, (b_plain["instructions_user"] or 0) - base
                )
            out["wall_seconds_audit_off_diagnostic_only"] = {
                "baseline": b_plain["wall_seconds_diagnostic_only"],
                "candidate": c_plain["wall_seconds_diagnostic_only"],
            }
    return out


def stage_a_cell(runs, states, family, rtol_text):
    prefix = f"a_{family}_{rtol_text}"
    setup = runs.get(f"{prefix}_SETUP")
    protected = runs.get(f"{prefix}_A_none")
    shipped = runs.get(f"{prefix}_B0_none")
    primary = compare(runs, states, prefix, "B_none", "C_none", setup)
    cell = {
        "family": family,
        "rtol": float(rtol_text),
        "arm_A": client_summary(protected, setup),
        "arm_B0": client_summary(shipped, setup),
        "B_vs_C_none": primary,
        "B_vs_C_jacobi_secondary": compare(runs, states, prefix, "B_jacobi", "C_jacobi", setup, audited_only=True),
    }
    b = runs.get(f"{prefix}_B_none_audit1")
    stand_in = {}
    if protected and protected["status"] == "OK":
        if shipped and shipped["status"] == "OK":
            stand_in["B0_over_A"] = {
                "attempts": ratio(shipped["attempts"], protected["attempts"]),
                "jvp": ratio(shipped["jvp_calls"], protected["jvp_calls"]),
                "counters_identical": {
                    key: shipped["counters"][key] == protected["counters"][key]
                    for key in ("jvp_calls", "linear_iterations", "linear_matvecs", "diagnostic_matvecs", "rhs_calls")
                },
                "final_state_max_abs_difference": max_abs_difference(states, shipped["name"], protected["name"]),
            }
        if b and b["status"] == "OK":
            stand_in["B_over_A"] = {
                "attempts": ratio(b["attempts"], protected["attempts"]),
                "jvp": ratio(b["jvp_calls"], protected["jvp_calls"]),
                "final_state_max_abs_difference": max_abs_difference(states, b["name"], protected["name"]),
                "error_ratio": ratio(
                    (b.get("error") or {}).get("max_tolerance_units"),
                    (protected.get("error") or {}).get("max_tolerance_units"),
                ),
            }
    cell["client_as_stand_in_for_A"] = stand_in
    jvp_ratio = primary.get("jvp_ratio_baseline_over_candidate")
    step_ratio = primary.get("step_ratio_candidate_over_baseline")
    checks = {
        "both_completed": primary["both_completed"],
        "jvp_ratio_at_least_1.5": jvp_ratio is not None and jvp_ratio >= STAGE_A_JVP_RATIO,
        "step_ratio_at_most_1.05": step_ratio is not None and step_ratio <= STAGE_A_STEP_RATIO,
        "residual_audit_clean_B": primary["residual_audit_clean"]["baseline"] is True,
        "residual_audit_clean_C": primary["residual_audit_clean"]["candidate"] is True,
    }
    cell["frozen_checks"] = checks
    cell["passes"] = all(checks.values())
    cell["jvp_ratio"] = jvp_ratio
    cell["step_ratio"] = step_ratio
    return cell


def stage_a(runs, states, predeclared):
    families = [family["id"] for family in predeclared["families_stage_a"]]
    tolerances = ["1e-4", "1e-6"]
    cells, family_rows = [], []
    for family in families:
        family_cells = [stage_a_cell(runs, states, family, rtol) for rtol in tolerances]
        cells.extend(family_cells)
        ratios = [cell["jvp_ratio"] for cell in family_cells]
        complete = all(value is not None for value in ratios)
        family_rows.append(
            {
                "family": family,
                "passes": all(cell["passes"] for cell in family_cells),
                "jvp_ratio_by_rtol": {rtol: cell["jvp_ratio"] for rtol, cell in zip(tolerances, family_cells)},
                "step_ratio_by_rtol": {rtol: cell["step_ratio"] for rtol, cell in zip(tolerances, family_cells)},
                "family_level_jvp_ratio_min_over_rtol": min(ratios) if complete else None,
                "geometric_mean_jvp_ratio_reported_only": (
                    finite(math.exp(sum(math.log(value) for value in ratios) / len(ratios))) if complete else None
                ),
            }
        )
    decided = all(row["family_level_jvp_ratio_min_over_rtol"] is not None for row in family_rows)
    passed = sum(1 for row in family_rows if row["passes"])
    verdict = {
        "all_declared_cells_have_both_arms": decided,
        "families_passing": passed,
        "families_needed": STAGE_A_FAMILIES_NEEDED,
        "families": family_rows,
    }
    if decided:
        median = statistics.median(row["family_level_jvp_ratio_min_over_rtol"] for row in family_rows)
        verdict["median_family_level_jvp_ratio"] = median
        verdict["v2_cancelled_by_median_rule"] = median < STAGE_A_CANCEL_MEDIAN
        verdict["verdict"] = "PASS" if passed >= STAGE_A_FAMILIES_NEEDED else "FAIL"
        verdict["median_of_the_six_cells_reported_only"] = statistics.median(
            cell["jvp_ratio"] for cell in cells
        )
        verdict["median_krylov_vector_ratio_of_the_six_cells_exploratory"] = statistics.median(
            cell["B_vs_C_none"]["krylov_vector_ratio_baseline_over_candidate_exploratory"] for cell in cells
        )
    else:
        verdict["verdict"] = "NOT_DECIDABLE" if passed < STAGE_A_FAMILIES_NEEDED else "PASS"
    return {"cells": cells, "verdict": verdict}


def exploratory(runs, states, predeclared):
    out = {"label": "exploratory: not part of any frozen verdict", "cells": []}
    for family in [family["id"] for family in predeclared["families_stage_a"]]:
        for rtol in ["1e-4", "1e-6"]:
            baseline = runs.get(f"a_{family}_{rtol}_B_none_audit1")
            declared = runs.get(f"a_{family}_{rtol}_C_none_audit1")
            row = {"family": family, "rtol": float(rtol)}
            for tag in ("C60", "Ckeep"):
                run = runs.get(f"x_{family}_{rtol}_{tag}_none_audit1")
                if run is None:
                    continue
                entry = client_summary(run, None)
                if run["status"] == "OK" and baseline is not None and baseline["status"] == "OK":
                    entry["jvp_ratio_B_over_this"] = ratio(baseline["jvp_calls"], run["jvp_calls"])
                    entry["step_ratio_this_over_B"] = ratio(run["attempts"], baseline["attempts"])
                if run["status"] == "OK" and declared is not None and declared["status"] == "OK":
                    entry["jvp_ratio_declared_C_over_this"] = ratio(declared["jvp_calls"], run["jvp_calls"])
                row[tag] = entry
            if len(row) > 2:
                out["cells"].append(row)
    stiff = {}
    for tag in ("A_none", "B_none_audit1", "C60_none_audit1"):
        run = runs.get(f"x_bruss1d_stiff_1e-4_{tag}")
        if run is not None:
            stiff[tag] = client_summary(run, runs.get("x_bruss1d_stiff_1e-4_SETUP"))
    if stiff:
        b, c = runs.get("x_bruss1d_stiff_1e-4_B_none_audit1"), runs.get("x_bruss1d_stiff_1e-4_C60_none_audit1")
        if b and c and b["status"] == "OK" and c["status"] == "OK":
            stiff["jvp_ratio_B_over_C60"] = ratio(b["jvp_calls"], c["jvp_calls"])
            stiff["step_ratio_C60_over_B"] = ratio(c["attempts"], b["attempts"])
        out["bruss1d_stiff_1e-4"] = stiff
    return out


def main():
    predeclared = strict_load(NODE / "PREDECLARED.json")
    references = {}
    for path in sorted((NODE / "reference").glob("*.json")):
        record = strict_load(path)
        references[record["family"]] = {
            "values": decode_state(record["state_hex"]),
            "atol_over_rtol": record["atol_over_rtol"],
            "method": record["reference"],
            "cross_check": record["cross_check"],
            "uncertainty": record["uncertainty"],
            "file": f"reference/{path.name}",
        }
    runs, states = load_runs(references)
    environment = {}
    for path in sorted(LOGS.glob("environment_*.log")):
        entries = {}
        for line in path.read_text().splitlines():
            if line.startswith("V06_ENV "):
                key, _, value = line[len("V06_ENV ") :].partition("=")
                entries[key] = value
        environment[path.name] = entries
    results = {
        "schema": "vigilode.loop-l1.v06.results.v1",
        "node": "V06_SHARED_KRYLOV_BASIS",
        "built_by": "parse_results.py from logs/, reference/ and PREDECLARED.json",
        "frozen_thresholds": {
            "stage_a_jvp_ratio_min": STAGE_A_JVP_RATIO,
            "stage_a_step_ratio_max": STAGE_A_STEP_RATIO,
            "stage_a_families_needed": STAGE_A_FAMILIES_NEEDED,
            "stage_a_cancel_median_below": STAGE_A_CANCEL_MEDIAN,
            "final_jvp_fraction_max": FINAL_JVP_FRACTION,
            "final_instruction_fraction_max": FINAL_INSTRUCTION_FRACTION,
            "final_families_needed": FINAL_FAMILIES_NEEDED,
            "final_error_band_max": FINAL_ERROR_BAND,
        },
        "wall_time_label": "diagnostic_only",
        "environment": environment,
        "references": {
            family: {key: value for key, value in reference.items() if key != "values"}
            for family, reference in references.items()
        },
        "stage_a": stage_a(runs, states, predeclared),
        "exploratory": exploratory(runs, states, predeclared),
        "runs": {
            name: {key: value for key, value in run.items() if key != "counters_all"} for name, run in runs.items()
        },
    }
    extension = NODE / "parse_final.py"
    if extension.exists():
        namespace = {}
        exec(compile(extension.read_text(), str(extension), "exec"), namespace)
        results["final"] = namespace["final"](runs, states, predeclared, sys.modules[__name__])
    text = json.dumps(results, indent=1, allow_nan=False, sort_keys=False)
    (NODE / "RESULTS.json").write_text(text + "\n")

    verdict = results["stage_a"]["verdict"]
    print(f"runs parsed: {len(runs)}")
    for cell in results["stage_a"]["cells"]:
        primary = cell["B_vs_C_none"]
        print(
            f"  {cell['family']:12s} rtol {cell['rtol']:g}: jvp_ratio {cell['jvp_ratio']} "
            f"step_ratio {cell['step_ratio']} pass {cell['passes']} "
            f"instr C/B {((primary.get('instructions_user_audit_off') or {}).get('candidate_over_baseline'))}"
        )
    print(f"stage a: {verdict['verdict']} ({verdict['families_passing']} of 3 families)")
    if "median_family_level_jvp_ratio" in verdict:
        print(
            f"  median family-level JVP ratio {verdict['median_family_level_jvp_ratio']:.4f}, "
            f"v2 cancelled: {verdict['v2_cancelled_by_median_rule']}"
        )


if __name__ == "__main__":
    main()
