#!/usr/bin/env python3
"""Build RESULTS.json of loop node V08 from the raw logs under logs/ only.

No number is typed in: every value comes from a tagged JSON line of an
executed run (or from the header lines the run script wrote), and every
verdict is computed here from those values with the rules frozen in
PREDECLARED.json. Run with:  python3 -I build_results.py
"""

import glob
import hashlib
import json
import os
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(HERE, "logs")
WORKERS = (1, 2, 4, 6)
VERDICT_WORKERS = (2, 4, 6)
LOAD_MAX = 8.0
SPREAD_MAX = 0.10
RHS_COST_MIN_NS = 50_000.0
PASS_RATIO = 1.5
KILL_RATIO = 1.1
MAX_MEASUREMENTS = 3  # one measurement and at most two re-measurements (quality rule)
CHECK_COLUMNS = 8


def refuse_constant(text):
    raise ValueError("non-strict JSON constant " + text)


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle, parse_constant=refuse_constant)


def tagged(path, tag):
    """The JSON objects behind `tag` in a log, in order."""
    rows = []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith(tag + " "):
                rows.append(json.loads(line[len(tag) + 1 :], parse_constant=refuse_constant))
    return rows


def header(path):
    """The '# key: value' lines of a log."""
    fields = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("# ") and ": " in line:
                key, value = line[2:].rstrip("\n").split(": ", 1)
                fields[key] = value
    return fields


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def rel(path):
    return os.path.relpath(path, HERE)


def git(*args):
    try:
        return subprocess.run(
            ["git", "-C", HERE, *args], capture_output=True, text=True, check=True
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def spread(values):
    middle = statistics.median(values)
    return (max(values) - min(values)) / middle if middle > 0 else None


# ---------------------------------------------------------------- identity

IDENTITY_KEYS = (
    "success",
    "stop_reason",
    "attempts",
    "accepted_steps",
    "rejected_steps",
    "jacobian_reuses",
    "t_end_bits",
    "final_fingerprint",
    "state_fingerprint",
    "jacobian_fingerprint",
    "builds",
    "jacobian_builds",
    "direct_factorizations",
    "direct_solve_calls",
)


def expected_checks(row):
    """(verifications, columns, right-hand sides) a run must report."""
    threaded = row["arm"] != "seq"
    if row["route"] == "dense" and not threaded:
        return 0, 0, 0
    return 1, CHECK_COLUMNS, CHECK_COLUMNS + (1 if threaded else 0)


def call_count_failures(row, stage_calls=None):
    """Why the right-hand-side counts of a client run are not the declared ones."""
    failures = []
    per_build = (row["colours"] + 1) if row["route"] == "coloured" else row["n"] + 1
    if row["rhs_per_build"] != per_build:
        failures.append(f"rhs_per_build {row['rhs_per_build']} != {per_build}")
    if row["assembly_rhs_calls"] != row["builds"] * per_build:
        failures.append(
            f"assembly calls {row['assembly_rhs_calls']} != builds {row['builds']} * {per_build}"
        )
    if row["failed_builds"] != 0:
        failures.append(f"{row['failed_builds']} failed builds")
    checks, columns, calls = expected_checks(row)
    if (row["verifications"], row["verified_columns"], row["verification_rhs_calls"]) != (
        checks,
        columns,
        calls,
    ):
        failures.append(
            "checks (%d, %d columns, %d calls) != declared (%d, %d, %d)"
            % (
                row["verifications"],
                row["verified_columns"],
                row["verification_rhs_calls"],
                checks,
                columns,
                calls,
            )
        )
    if stage_calls is not None:
        total = stage_calls + row["assembly_rhs_calls"] + row["verification_rhs_calls"]
        if row["rhs_calls"] != total:
            failures.append(f"run counters {row['rhs_calls']} != {total}")
        counted = row.get("rhs_calls_counted_by_the_callback")
        if counted != total:
            failures.append(f"callback count {counted} != {total}")
    return failures


def identity_section(declared_cells):
    cells = {}
    call_runs = 0
    call_failures = []
    all_equal = True
    for path in sorted(glob.glob(os.path.join(LOGS, "identity_*.log"))):
        rows = tagged(path, "V08_IDENTITY_JSON")
        groups = []
        for row in rows:
            if row.get("arm") == "prot":
                groups.append({"reference": row, "arms": []})
            elif groups:
                groups[-1]["arms"].append(row)
        for group in groups:
            reference = group["reference"]
            cell = reference["cell"]
            arms = {}
            for row in group["arms"]:
                if "error" in row:
                    differences = ["run ended with an error: " + row["error"]]
                else:
                    differences = [
                        key for key in IDENTITY_KEYS if row.get(key) != reference.get(key)
                    ]
                    failures = call_count_failures(row, reference["rhs_calls_without_jacobian"])
                    call_runs += 1
                    call_failures += [
                        f"{rel(path)} {cell} {row['backend']} {row['arm']}: {failure}"
                        for failure in failures
                    ]
                all_equal = all_equal and not differences
                arms[row["arm"]] = {
                    "equal_to_protected_run": not differences,
                    "differing_fields": differences,
                    "final_fingerprint": row.get("final_fingerprint"),
                    "state_fingerprint": row.get("state_fingerprint"),
                    "jacobian_fingerprint": row.get("jacobian_fingerprint"),
                    "builds": row.get("builds"),
                    "colours": row.get("colours"),
                    "rhs_per_build": row.get("rhs_per_build"),
                    "assembly_rhs_calls": row.get("assembly_rhs_calls"),
                    "verification_rhs_calls": row.get("verification_rhs_calls"),
                    "rhs_calls": row.get("rhs_calls"),
                    "rhs_calls_counted_by_the_callback": row.get(
                        "rhs_calls_counted_by_the_callback"
                    ),
                }
            backend = group["arms"][0]["backend"] if group["arms"] else "?"
            cells.setdefault(cell, {})[backend] = {
                "log": rel(path),
                "protected_driver": reference["backend"],
                "protected_lu": reference["lu"],
                "attempts": reference["attempts"],
                "accepted_steps": reference["accepted_steps"],
                "rejected_steps": reference["rejected_steps"],
                "success": reference["success"],
                "stop_reason": reference["stop_reason"],
                "t_end": reference["t_end"],
                "builds": reference["builds"],
                "stage_rhs_calls": reference["rhs_calls_without_jacobian"],
                "final_fingerprint": reference["final_fingerprint"],
                "state_fingerprint": reference["state_fingerprint"],
                "jacobian_fingerprint": reference["jacobian_fingerprint"],
                "arms": arms,
            }
    missing = [cell for cell in declared_cells if cell not in cells]
    arms_complete = all(
        set(group["arms"]) == {"seq", "t1", "t2", "t4", "t6"}
        for backends in cells.values()
        for group in backends.values()
    )
    if not cells:
        verdict = "NOT_RUN"
    elif missing or not arms_complete:
        verdict = "NOT_DECIDABLE"
    else:
        verdict = "PASS" if all_equal else "FAIL"
    return (
        {
            "verdict": verdict,
            "rule": "every arm of every declared problem equals the protected driver run with "
            "reference finite differences in: " + ", ".join(IDENTITY_KEYS),
            "missing_cells": missing,
            "arms_complete": arms_complete,
            "runs_compared": sum(
                len(group["arms"]) for backends in cells.values() for group in backends.values()
            ),
            "cells": cells,
        },
        call_runs,
        call_failures,
    )


# ------------------------------------------------------------- fail closed


def fail_closed_section():
    path = os.path.join(LOGS, "failclosed.log")
    if not os.path.exists(path):
        return {"verdict": "NOT_RUN", "reason": "logs/failclosed.log is missing"}
    rows = tagged(path, "V08_FAILCLOSED_JSON")
    controls = []
    passed = True
    numbers = set()
    for row in rows:
        closed = (
            row["outcome"] == "error"
            and row["driver_error_kind"] == "InvalidInput"
            and row["typed_error_is_contract_violation"] is True
            and row["completed_builds"] == 0
            and row["accepted_steps"] is None
            and row["direct_full_check_failed"] is True
        )
        if row["must_fail"]:
            passed = passed and closed
            numbers.add(row["control"].split(" ", 1)[0])
        controls.append(
            {
                "control": row["control"],
                "problem": row["problem"],
                "arm": row["arm"],
                "first_use_check": row["first_use_check"],
                "must_fail": row["must_fail"],
                "failed_closed": closed,
                "outcome": row["outcome"],
                "driver_error_kind": row["driver_error_kind"],
                "typed_error": row["typed_error"],
                "accepted_steps": row["accepted_steps"],
                "completed_builds": row["completed_builds"],
                "direct_full_check": row["direct_full_check"],
            }
        )
    complete = numbers == {"1", "2", "3", "4"}
    verdict = "NOT_DECIDABLE" if not complete else ("PASS" if passed else "FAIL")
    return {
        "verdict": verdict,
        "rule": "every negative control ends with InvalidInput, a typed contract error, no "
        "completed build and no accepted step, and the direct full check fails too",
        "declared_controls_present": sorted(numbers),
        "log": rel(path),
        "controls": controls,
    }


# ------------------------------------------------------------------ timing


def arm_summary(rows):
    def column(key):
        return [row[key] for row in rows if key in row]

    wall = column("wall_ns")
    summary = {
        "repetitions": len(wall),
        "wall_ns": wall,
        "wall_median_ns": statistics.median(wall),
        "wall_min_ns": min(wall),
        "wall_max_ns": max(wall),
        "wall_spread": spread(wall),
    }
    linearize = column("linearize_ns")
    if linearize:
        summary.update(
            {
                "linearize_ns": linearize,
                "linearize_median_ns": statistics.median(linearize),
                "linearize_spread": spread(linearize),
                "factor_median_ns": statistics.median(column("factor_ns")),
                "solve_median_ns": statistics.median(column("solve_ns")),
                "construct_median_ns": statistics.median(column("construct_ns")),
                "jacobian_share_of_wall": statistics.median(linearize) / statistics.median(wall),
            }
        )
    return summary


def timing_try(path):
    rows = tagged(path, "V08_RUN_JSON")
    loads = {row["when"]: row["loadavg"] for row in tagged(path, "V08_LOAD_JSON")}
    costs = tagged(path, "V08_RHS_COST_JSON")
    meta = tagged(path, "V08_META_JSON")
    timed = [row for row in rows if not row["warm_up"]]
    arms = {}
    for row in timed:
        arms.setdefault(row["arm"], []).append(row)
    fingerprints = sorted({row["final_fingerprint"] for row in rows})
    call_failures = []
    for row in rows:
        if row["arm"] != "prot":
            call_failures += [
                f"{rel(path)} {row['cell']} {row['arm']} round {row['round']}: {failure}"
                for failure in call_count_failures(row)
            ]
    summaries = {arm: arm_summary(arm_rows) for arm, arm_rows in arms.items()}
    load_before = loads.get("before", {}).get("load_1m")
    load_after = loads.get("after", {}).get("load_1m")
    quiet = (
        load_before is not None
        and load_after is not None
        and load_before <= LOAD_MAX
        and load_after <= LOAD_MAX
    )
    ratios = {}
    if "seq" in summaries:
        seq = summaries["seq"]
        for workers in WORKERS:
            arm = summaries.get(f"t{workers}")
            if arm is None:
                continue
            reasons = []
            if not quiet:
                reasons.append(f"1-minute load {load_before} before, {load_after} after (limit 8)")
            for name, summary in (("seq", seq), (f"t{workers}", arm)):
                if summary["repetitions"] != 7:
                    reasons.append(f"{name}: {summary['repetitions']} repetitions")
                if summary["wall_spread"] is None or summary["wall_spread"] > SPREAD_MAX:
                    reasons.append(f"{name}: wall spread {summary['wall_spread']:.3f} > 0.10")
            if len(fingerprints) != 1:
                reasons.append("the runs of this log do not end in the same state")
            jacobian_spreads_ok = all(
                summary["linearize_spread"] is not None
                and summary["linearize_spread"] <= SPREAD_MAX
                for summary in (seq, arm)
            )
            share = seq["jacobian_share_of_wall"]
            ratios[f"t{workers}"] = {
                "workers": workers,
                "whole_solve_speed_up": seq["wall_median_ns"] / arm["wall_median_ns"],
                "authoritative": not reasons,
                "why_not_authoritative": reasons,
                "jacobian_phase_speed_up": seq["linearize_median_ns"] / arm["linearize_median_ns"],
                "jacobian_phase_authoritative": quiet and jacobian_spreads_ok,
                "amdahl_bound_from_the_measured_jacobian_share": 1.0
                / (1.0 - share * (1.0 - 1.0 / workers)),
            }
    cost = None
    if costs:
        per_call = costs[0]["per_call_ns"]
        cost = {
            "per_call_ns": per_call,
            "median_ns": statistics.median(per_call),
            "calls_per_batch": costs[0]["calls_per_batch"],
        }
    first = rows[0] if rows else {}
    # How much of the sequential Jacobian phase is right-hand-side time:
    # builds * right-hand sides per build * the calibrated cost per call.
    seq_rows = arms.get("seq", [])
    if seq_rows and cost and "seq" in summaries:
        builds = seq_rows[0]["builds"]
        per_build = seq_rows[0]["rhs_per_build"]
        rhs_ns = builds * per_build * cost["median_ns"]
        summaries["seq"]["builds"] = builds
        summaries["seq"]["rhs_per_build"] = per_build
        summaries["seq"]["jacobian_phase_per_build_ns"] = (
            summaries["seq"]["linearize_median_ns"] / builds
        )
        summaries["seq"]["rhs_time_in_jacobian_phase_estimate_ns"] = rhs_ns
        summaries["seq"]["rhs_share_of_jacobian_phase_estimate"] = (
            rhs_ns / summaries["seq"]["linearize_median_ns"]
        )
        summaries["seq"]["rhs_share_of_wall_estimate"] = rhs_ns / summaries["seq"]["wall_median_ns"]
    result = {
        "log": rel(path),
        "log_sha256": sha256(path),
        "binary_sha256": header(path).get("binary sha256"),
        "git_head": header(path).get("git head"),
        "command": header(path).get("command"),
        "cpus_allowed_list": meta[0].get("cpus_allowed_list") if meta else None,
        "load_1m_before": load_before,
        "load_1m_after": load_after,
        "quiet_host": quiet,
        "cell": first.get("cell"),
        "backend": first.get("backend_cell"),
        "n": first.get("n"),
        "colours": next((row.get("colours") for row in rows if row["arm"] != "prot"), None),
        "attempts": first.get("attempts"),
        "accepted_steps": first.get("accepted_steps"),
        "rejected_steps": first.get("rejected_steps"),
        "t_end": first.get("t_end"),
        "final_fingerprints": fingerprints,
        "same_final_state_in_every_run": len(fingerprints) == 1,
        "rhs_cost": cost,
        "arms": summaries,
        "ratios": ratios,
    }
    if "seq" in summaries and "prot" in summaries:
        result["client_seq_over_protected_driver_wall"] = (
            summaries["seq"]["wall_median_ns"] / summaries["prot"]["wall_median_ns"]
        )
    return result, call_failures, len([row for row in rows if row["arm"] != "prot"])


def cell_verdict(tries):
    """The frozen cell rule on the first authoritative measurement of each ratio."""
    used = {}
    for workers in VERDICT_WORKERS:
        for index, attempt in enumerate(tries):
            ratio = attempt["ratios"].get(f"t{workers}")
            if ratio and ratio["authoritative"]:
                used[f"t{workers}"] = {
                    "try": index + 1,
                    "log": attempt["log"],
                    "whole_solve_speed_up": ratio["whole_solve_speed_up"],
                    "jacobian_phase_speed_up": ratio["jacobian_phase_speed_up"],
                }
                break
    values = [entry["whole_solve_speed_up"] for entry in used.values()]
    complete = len(used) == len(VERDICT_WORKERS)
    # The reading of the builder's first parser: a cell without an authoritative ratio at
    # every worker count is NOT_DECIDABLE unless it passes.
    if values and max(values) >= PASS_RATIO:
        strict = "PASS"
    elif complete and max(values) < KILL_RATIO:
        strict = "KILL"
    elif complete:
        strict = "FAIL"
    else:
        strict = "NOT_DECIDABLE"
    # PREDECLARED.json, verdict_rules.wall_cell (correction after the independent review,
    # finding F-03): the best ratio is taken among the authoritative ratios; NOT_DECIDABLE is
    # reserved for a cell with no authoritative ratio. KILL needs the speed-up at every worker
    # count. While an allowed measurement is left, a cell that does not pass stays open.
    measurements_left = len(tries) < MAX_MEASUREMENTS
    if not values:
        verdict = "NOT_DECIDABLE"
    elif max(values) >= PASS_RATIO:
        verdict = "PASS"
    elif complete and max(values) < KILL_RATIO:
        verdict = "KILL"
    elif not complete and measurements_left:
        verdict = "NOT_DECIDABLE"
    else:
        verdict = "FAIL"
    last = tries[-1]["ratios"] if tries else {}
    return {
        "verdict": verdict,
        "verdict_if_every_worker_count_must_be_authoritative": strict,
        "measurements": len(tries),
        "authoritative_ratios_used": used,
        "best_authoritative_whole_solve_speed_up": max(values) if values else None,
        "diagnostic_ratios_of_the_last_try": {
            arm: ratio["whole_solve_speed_up"] for arm, ratio in last.items()
        },
    }


def timing_section(predeclared):
    by_cell = {}
    call_runs = 0
    call_failures = []
    for path in sorted(glob.glob(os.path.join(LOGS, "timing_*.log"))):
        attempt, failures, runs = timing_try(path)
        if attempt["cell"] is None:
            continue
        call_runs += runs
        call_failures += failures
        by_cell.setdefault(attempt["cell"], {}).setdefault(attempt["backend"], []).append(attempt)
    cells = {}
    for problem in predeclared["problems"]:
        cell = problem["id"]
        backends = by_cell.get(cell, {})
        primary = next(
            (name for name in backends if name != "banded_protected"), None
        )
        entry = {
            "class": problem["class"],
            "route": problem["route"],
            "n": problem["n"],
            "primary_backend": primary,
            "backends": {},
        }
        for backend, tries in backends.items():
            entry["backends"][backend] = {"tries": tries, **cell_verdict(tries)}
        if primary:
            first = backends[primary][0]
            cost = first["rhs_cost"]
            entry["rhs_cost_median_ns"] = cost["median_ns"] if cost else None
            entry["rhs_cost_source"] = first["log"]
            entry["rhs_cost_all_tries_median_ns"] = [
                attempt["rhs_cost"]["median_ns"]
                for tries in backends.values()
                for attempt in tries
                if attempt["rhs_cost"]
            ]
            entry["qualifies"] = (
                problem["class"] == "qualifying"
                and cost is not None
                and cost["median_ns"] >= RHS_COST_MIN_NS
            )
            entry["verdict"] = entry["backends"][primary]["verdict"]
        else:
            entry["qualifies"] = None
            entry["verdict"] = "NOT_RUN"
        cells[cell] = entry
    qualifying = {
        cell: entry["verdict"] for cell, entry in cells.items() if entry["qualifies"]
    }
    declared_qualifying = [
        problem["id"] for problem in predeclared["problems"] if problem["class"] == "qualifying"
    ]
    not_run = [cell for cell in declared_qualifying if cells[cell]["verdict"] == "NOT_RUN"]
    verdicts = list(qualifying.values())
    if not_run or not verdicts:
        node = "NOT_RUN" if len(not_run) == len(declared_qualifying) else "NOT_DECIDABLE"
    elif all(verdict == "PASS" for verdict in verdicts):
        node = "PASS"
    elif all(verdict == "KILL" for verdict in verdicts):
        node = "KILL"
    elif "NOT_DECIDABLE" not in verdicts:
        node = "FAIL"
    else:
        decided = [verdict for verdict in verdicts if verdict != "NOT_DECIDABLE"]
        cannot_pass = any(verdict in ("FAIL", "KILL") for verdict in decided)
        cannot_kill = any(verdict in ("PASS", "FAIL") for verdict in decided)
        node = "FAIL" if cannot_pass and cannot_kill else "NOT_DECIDABLE"
    return (
        {
            "node_verdict": node,
            "node_rule": predeclared["verdict_rules"]["wall_node"],
            "cell_rule": predeclared["verdict_rules"]["wall_cell"],
            "quality_rule": predeclared["timing_protocol"]["quality_rule"],
            "qualifying_cells": qualifying,
            "declared_qualifying_not_qualifying": [
                cell
                for cell in declared_qualifying
                if cells[cell]["qualifies"] is False
            ],
            "cells": cells,
        },
        call_runs,
        call_failures,
    )


# ------------------------------------------------------------ other sections


def tests_section():
    path = os.path.join(LOGS, "tests.log")
    if not os.path.exists(path):
        return {"status": "NOT_RUN", "reason": "logs/tests.log is missing"}
    rows = tagged(path, "STAGE_CLIENTS_ASSEMBLY_ALLOC_JSON")
    sequential = [row for row in rows if row["execution"] == "sequential"]
    threaded = [row for row in rows if row["execution"] == "threaded"]
    results = []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("test result:") or line.startswith("     Running "):
                results.append(line.strip())
    return {
        "log": rel(path),
        "test_binaries_and_results": results,
        "allocation": {
            "sequential_runs": len(sequential),
            "sequential_steady_steps_measured": sum(
                row["steady_steps_measured"] for row in sequential
            ),
            "sequential_steady_allocations_total": sum(
                row["steady_allocations_total"] for row in sequential
            ),
            "sequential_clients": sorted({row["client"] for row in sequential}),
            "threaded_runs_reported": [
                {
                    "client": row["client"],
                    "rtol": row["rtol"],
                    "steady_steps_measured": row["steady_steps_measured"],
                    "steady_allocations_total": row["steady_allocations_total"],
                    "steady_allocations_max_per_step": row["steady_allocations_max_per_step"],
                }
                for row in threaded
            ],
            "verdict": "PASS"
            if sequential and all(row["steady_allocations_total"] == 0 for row in sequential)
            else "FAIL",
        },
    }


def perf_section():
    runs = []
    for path in sorted(glob.glob(os.path.join(LOGS, "perf_*.log"))):
        single = tagged(path, "V08_SINGLE_JSON")
        instructions = None
        with open(path, encoding="utf-8", errors="replace") as handle:
            for line in handle:
                fields = line.strip().split(",")
                if len(fields) > 2 and fields[2].startswith("instructions"):
                    try:
                        instructions = int(fields[0])
                    except ValueError:
                        instructions = None
        if single:
            runs.append(
                {
                    "log": rel(path),
                    "cell": single[0]["cell"],
                    "arm": single[0]["arm"],
                    "instructions_user_all_threads": instructions,
                    "wall_ns": single[0]["wall_ns"],
                    "final_fingerprint": single[0]["final_fingerprint"],
                }
            )
    return {
        "label": "diagnostic_only: one solve per arm, instructions:u of the whole process "
        "(test harness and problem set-up included)",
        "runs": runs,
    }


def tagged_log_section(name, tag):
    path = os.path.join(LOGS, name)
    if not os.path.exists(path):
        return {"status": "NOT_RUN", "reason": f"logs/{name} is missing"}
    return {"log": rel(path), "log_sha256": sha256(path), "rows": tagged(path, tag)}


def main():
    predeclared = load_json(os.path.join(HERE, "PREDECLARED.json"))
    declared_cells = [problem["id"] for problem in predeclared["problems"]]
    identity, identity_runs, identity_failures = identity_section(declared_cells)
    timing, timing_runs, timing_failures = timing_section(predeclared)
    failures = identity_failures + timing_failures
    runs = identity_runs + timing_runs
    rhs_calls = {
        "verdict": "NOT_RUN" if runs == 0 else ("PASS" if not failures else "FAIL"),
        "rule": "assembly calls = builds * (colours + 1) on the coloured route and builds * "
        "(n + 1) on the dense route; check calls as declared (8 columns on first use, plus "
        "the base evaluation with threads; none on the sequential dense route); in the "
        "identity pass also: run counters = callback count = stage calls of the protected "
        "run + assembly + checks",
        "client_runs_checked": runs,
        "of_which_with_callback_count": identity_runs,
        "failures": failures,
    }
    logs = sorted(glob.glob(os.path.join(LOGS, "*.log")))
    results = {
        "schema": "vigilode.loop_l1.v08.results.v1",
        "node": "V08_JACOBIAN_ASSEMBLY",
        "built_by": "build_results.py from the raw logs only",
        "predeclared_commit": git("log", "-1", "--format=%H", "--", "PREDECLARED.json"),
        "head_at_build": git("rev-parse", "HEAD"),
        "logs": [{"file": rel(path), "sha256": sha256(path)} for path in logs],
        "discriminators": {
            "identity": identity,
            "rhs_calls": rhs_calls,
            "fail_closed": fail_closed_section(),
            "wall": timing,
        },
        "tests": tests_section(),
        "instructions": perf_section(),
        "facade": tagged_log_section("facade.log", "V08_FACADE_JSON"),
        "reference_error": tagged_log_section("reference_error.log", "V08_REFERENCE_JSON"),
        "checks": tagged_log_section("checks.log", "V08_CHECK_JSON"),
    }
    out = os.path.join(HERE, "RESULTS.json")
    with open(out, "w", encoding="utf-8") as handle:
        json.dump(results, handle, indent=1, allow_nan=False, ensure_ascii=False)
        handle.write("\n")
    load_json(out)
    summary = {
        "identity": identity["verdict"],
        "rhs_calls": rhs_calls["verdict"],
        "fail_closed": results["discriminators"]["fail_closed"]["verdict"],
        "wall_node": timing["node_verdict"],
        "wall_cells": {cell: entry["verdict"] for cell, entry in timing["cells"].items()},
    }
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
