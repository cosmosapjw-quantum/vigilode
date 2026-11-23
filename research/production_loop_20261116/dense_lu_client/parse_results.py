#!/usr/bin/env python3
"""Builds RESULTS.json of loop L1 node V18_DENSE_LU_CLIENT from the raw logs.

Usage (from the worktree root):

    python3 -I research/production_loop_20261116/dense_lu_client/parse_results.py

Every number in RESULTS.json is read from a file under logs/ (written by the runner, by the
test binaries or by perf) or computed from such numbers by the rules of PREDECLARED.json. The
only literals in this file are the frozen thresholds of addendum A2 and the constants of the
timing quality rule. A stage without logs is reported as NOT_RUN, never filled in.
"""

import hashlib
import json
import math
import statistics
import struct
import sys
from pathlib import Path

import mpmath

HERE = Path(__file__).resolve().parent
LOGS = HERE / "logs"
PREDECLARED = json.loads((HERE / "PREDECLARED.json").read_text())
CELLS = PREDECLARED["cells"]

# Frozen thresholds (PREREGISTRATION.md, addendum A2, row V18).
REUSE_WALL_PASS = 1.10
REUSE_WALL_KILL = 1.03
REUSE_FAULTS_PER_ATTEMPT_MAX = 1.0
THREADED_PASS = 1.5
THREADED_KILL = 1.15
THREADED_ATTEMPT_TOLERANCE = 0.01
THREADED_ERROR_BAND = (0.9, 1.1)
# Timing quality rule of the loop.
LOAD_LIMIT = 8.0
SPREAD_LIMIT = 0.10
# Matched-error rule of PREDECLARED.json.
EXTRAPOLATION_FACTOR = 1.1
REFERENCE_CROSS_CHECK = 1.0e-12
VARIANTS = (93, 108, 124)

mpmath.mp.dps = 40


def rows_of(name):
    path = LOGS / name
    if not path.exists():
        return []
    out = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if line.startswith("{"):
            out.append(json.loads(line))
    return out


def marked(name, marker):
    """The JSON rows a test binary printed after `marker`."""
    path = LOGS / name
    if not path.exists():
        return []
    out = []
    for line in path.read_text().splitlines():
        at = line.find(marker + " ")
        if at >= 0:
            out.append(json.loads(line[at + len(marker) + 1 :]))
    return out


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest():
    path = LOGS / "MANIFEST.jsonl"
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


MANIFEST = manifest()


def last_invocations(stage):
    """The last manifest entry of every log of a stage (a stage may have been run again)."""
    latest = {}
    for entry in MANIFEST:
        if entry.get("stage") == stage and "log" in entry:
            latest[entry["log"]] = entry
    return latest


def rtol_text(rtol):
    return f"{rtol:.0e}".replace("e-0", "e-")


def from_hex(words):
    return [struct.unpack(">d", bytes.fromhex(word))[0] for word in words]


def median(values):
    return statistics.median(values)


# ---------------------------------------------------------------------------------------------
# Test binaries.
# ---------------------------------------------------------------------------------------------


def test_outcome(name, entry):
    path = LOGS / name
    if not path.exists() or entry is None:
        return {"log": name, "status": "NOT_RUN"}
    lines = path.read_text().splitlines()
    results = [line.strip() for line in lines if line.startswith("test result:")]
    # "test <name> ... ok" on one line, or, for a test that prints rows, the closing word on a
    # later line of its own.
    tests = {}
    for index, line in enumerate(lines):
        if not (line.startswith("test ") and " ... " in line) or line.startswith("test result:"):
            continue
        test_name, _, rest = line[5:].partition(" ... ")
        rest = rest.strip()
        if rest in ("ok", "FAILED"):
            tests[test_name] = rest
        elif rest.startswith("ignored"):
            tests[test_name] = "ignored"
        else:
            tests[test_name] = "no closing line"
            for later in lines[index + 1 :]:
                if later.strip() in ("ok", "FAILED"):
                    tests[test_name] = later.strip()
                    break
                if later.startswith("test "):
                    break
    return {
        "log": name,
        "status": "RUN",
        "exit_code": entry["exit_code"],
        "result_lines": results,
        "tests": tests,
        "all_passed": entry["exit_code"] == 0 and bool(results) and all("0 failed" in r and r.startswith("test result: ok") for r in results),
    }


def identity_section():
    tests = last_invocations("tests")
    main = test_outcome("test_identity.log", tests.get("test_identity.log"))
    big = test_outcome("test_identity_n2048.log", tests.get("test_identity_n2048.log"))
    test_rows = []
    for name in ("test_identity.log", "test_identity_n2048.log"):
        for row in marked(name, "DENSE_LU_REUSE_IDENTITY_JSON"):
            row["log"] = name
            test_rows.append(row)
    threaded_kernel = marked("test_identity.log", "DENSE_LU_REUSE_THREADED_JSON")
    threaded_runs = marked("test_identity.log", "DENSE_LU_REUSE_THREADED_RUN_JSON")

    runner_rows = []
    for name, entry in sorted(last_invocations("identity").items()):
        rows = rows_of(name)
        header = next((r for r in rows if r["type"] == "header"), None)
        reference = next((r for r in rows if r["type"] == "identity_reference"), None)
        if entry["exit_code"] != 0 or header is None or reference is None:
            runner_rows.append({"log": name, "status": "FAILED", "exit_code": entry["exit_code"]})
            continue
        for row in rows:
            if row["type"] != "identity":
                continue
            against = row["against_protected"]
            runner_rows.append(
                {
                    "log": name,
                    "status": "RUN",
                    "cell": row["cell"],
                    "n": header["cell"]["n"],
                    "rtol": header["cell"]["rtol"],
                    "t_span": header["cell"]["t_span"],
                    "max_attempts": header["cell"]["max_attempts"],
                    "protected_policy_mode": header["cell"]["protected_policy_mode"],
                    "protected_lu": reference["lu"],
                    "arm": row["arm"],
                    "complete": row["complete"],
                    "attempts_protected": reference["attempts"],
                    "accepted_protected": reference["accepted_steps"],
                    "rejected_protected": reference["rejected_steps"],
                    "attempts": row["attempts"],
                    "accepted_states_compared": reference["accepted_states"],
                    "bit_identical": against["bit_identical"],
                    "first_difference": against["first_difference"],
                    "attempts_equal": against["attempts_equal"],
                    "accepted_equal": against["accepted_equal"],
                    "rejected_equal": against["rejected_equal"],
                    "counters_equal": against["counters_equal"],
                    "accepted_trajectory_equal": against["accepted_trajectory_equal"],
                    "end_state_equal": against["end_state_equal"],
                    "end_state_max_abs_difference": against["end_state_max_abs_difference"],
                    "rows_moved_by_last_factorization": row["rows_moved_by_last_factorization"],
                    "error_max_abs_protected": reference["error_max_abs"],
                }
            )

    def count(rows, predicate):
        return sum(1 for row in rows if predicate(row))

    ran = [row for row in runner_rows if row["status"] == "RUN"]
    reuse = [row for row in ran if row["arm"] == "reuse"]
    kit = [row for row in ran if row["arm"] == "kit"]
    threaded = [row for row in ran if row["arm"].startswith("threaded")]
    kit_set = [row for row in test_rows if row["set"] == "client-kit"]
    faer_test = [row for row in test_rows if row["set"] == "faer-path" and row["arm"] == "reuse"]
    sizes_runner = sorted({row["n"] for row in reuse})
    sizes_test = sorted({row["n"] for row in faer_test})
    complete = (
        main["status"] == "RUN"
        and big["status"] == "RUN"
        and len(kit_set) == 15
        and {128, 512, 2048} <= set(sizes_runner)
        and {128, 512, 2048} <= set(sizes_test)
        and not [row for row in runner_rows if row["status"] != "RUN"]
    )
    all_identical = (
        main.get("all_passed", False)
        and big.get("all_passed", False)
        and all(row["bit_identical"] for row in test_rows)
        and all(row["bit_identical"] for row in reuse)
    )
    first_failure = next((row for row in reuse if not row["bit_identical"]), None)
    if not complete:
        verdict = "NOT_RUN" if not test_rows and not runner_rows else "INCOMPLETE"
    else:
        verdict = "PASS" if all_identical else "FAIL"
    return {
        "test_binary": main,
        "test_binary_n2048": big,
        "test_rows": test_rows,
        "runner_rows": runner_rows,
        "threaded_kernel_rows": threaded_kernel,
        "threaded_test_run_rows": threaded_runs,
        "summary": {
            "client_kit_rows": len(kit_set),
            "client_kit_rows_identical": count(kit_set, lambda r: r["bit_identical"]),
            "client_kit_accepted_states_compared": sum(r["accepted_states_compared"] for r in kit_set),
            "faer_path_test_rows_reuse": len(faer_test),
            "faer_path_test_rows_reuse_identical": count(faer_test, lambda r: r["bit_identical"]),
            "faer_path_test_sizes": sizes_test,
            "runner_rows_reuse": len(reuse),
            "runner_rows_reuse_identical": count(reuse, lambda r: r["bit_identical"]),
            "runner_rows_kit": len(kit),
            "runner_rows_kit_identical": count(kit, lambda r: r["bit_identical"]),
            "runner_rows_threaded": len(threaded),
            "runner_rows_threaded_identical": count(threaded, lambda r: r["bit_identical"]),
            "runner_sizes": sizes_runner,
            "runner_accepted_states_compared_reuse": sum(r["accepted_states_compared"] for r in reuse),
            "runner_reuse_rows_with_row_interchanges": count(
                reuse, lambda r: (r["rows_moved_by_last_factorization"] or 0) > 0
            ),
            "largest_rows_moved_by_last_factorization": max(
                [r["rows_moved_by_last_factorization"] or 0 for r in reuse], default=None
            ),
            "threaded_largest_end_state_difference": max(
                [r["end_state_max_abs_difference"] or 0.0 for r in threaded], default=None
            ),
            "threaded_kernel_rows": len(threaded_kernel),
            "threaded_kernel_rows_equal_sequential_bits": count(
                threaded_kernel, lambda r: r["factors_equal_sequential_bits"]
            ),
        },
        "first_differing_reuse_row": first_failure,
        "verdict": verdict,
    }


def allocation_section():
    entry = last_invocations("tests").get("test_alloc.log")
    outcome = test_outcome("test_alloc.log", entry)
    rows = marked("test_alloc.log", "DENSE_LU_REUSE_ALLOC_JSON")
    by_arm = {}
    for row in rows:
        by_arm.setdefault(row["arm"], []).append(row)
    summary = {}
    for arm, group in sorted(by_arm.items()):
        summary[arm] = {
            "rows": len(group),
            "counted_steps": sum(r["counted_steps"] for r in group),
            "counted_allocations_total": sum(r["counted_allocations_total"] for r in group),
            "allocations_per_attempt_min": min(r["allocations_per_attempt"] for r in group),
            "allocations_per_attempt_max": max(r["allocations_per_attempt"] for r in group),
            "allocations_per_step_max": max(r["allocations_per_step_max"] for r in group),
            "bytes_per_attempt_by_n": {
                str(n): sorted({r["bytes_per_attempt"] for r in group if r["n"] == n})
                for n in sorted({r["n"] for r in group})
            },
        }
    reuse = by_arm.get("reuse", [])
    if outcome["status"] != "RUN" or not reuse:
        verdict = "NOT_RUN"
    else:
        zero = all(r["counted_allocations_total"] == 0 for r in reuse)
        verdict = "PASS" if zero and outcome["all_passed"] else "FAIL"
    return {"test_binary": outcome, "rows": rows, "summary_by_arm": summary, "verdict": verdict}


def fault_section():
    rows = []
    for name, entry in sorted(last_invocations("faults").items()):
        data = rows_of(name)
        header = next((r for r in data if r["type"] == "header"), None)
        if entry["exit_code"] != 0 or header is None:
            rows.append({"log": name, "status": "FAILED", "exit_code": entry["exit_code"]})
            continue
        for row in data:
            if row["type"] != "faults":
                continue
            steps = row["steady_steps"]
            rows.append(
                {
                    "log": name,
                    "status": "RUN",
                    "cell": row["cell"],
                    "n": header["cell"]["n"],
                    "rtol": header["cell"]["rtol"],
                    "argv0_length": header["argv0_length"],
                    "link_length_requested": entry["link_length"],
                    "arm": row["arm"],
                    "run_index": row["run_index"],
                    "attempts": row["attempts"],
                    "accepted_steps": row["accepted_steps"],
                    "rejected_steps": row["rejected_steps"],
                    "warm_up_steps": row["warm_up_steps"],
                    "steady_steps": steps,
                    "steady_minor_faults": row["steady_minor_faults"],
                    "steady_minor_faults_max_step": row["steady_minor_faults_max_step"],
                    "minor_faults_per_accepted_step": row["steady_minor_faults"] / steps if steps else None,
                    "run_minor_faults": row["minor_faults"],
                    "run_major_faults": row["major_faults"],
                    "run_minor_faults_per_attempt": row["minor_faults"] / row["attempts"],
                }
            )
    ran = [row for row in rows if row["status"] == "RUN"]
    table = {}
    for row in ran:
        key = (row["cell"], row["argv0_length"], row["arm"])
        table.setdefault(key, []).append(row["minor_faults_per_accepted_step"])
    summary = [
        {
            "cell": cell,
            "argv0_length": length,
            "arm": arm,
            "runs": len(rates),
            "minor_faults_per_accepted_step_min": min(rates),
            "minor_faults_per_accepted_step_max": max(rates),
        }
        for (cell, length, arm), rates in sorted(table.items())
    ]
    reuse = [row for row in ran if row["arm"] == "reuse"]
    lengths_ok = all(row["argv0_length"] == row["link_length_requested"] for row in ran)
    needed = {("meanfield.n512", length) for length in VARIANTS} | {("skewnet.n512", length) for length in VARIANTS}
    needed |= {(cell, 93) for cell in ("meanfield.n128", "skewnet.n128", "meanfield.n2048", "skewnet.n2048")}
    have = {(row["cell"], row["argv0_length"]) for row in reuse}
    if not reuse:
        verdict = "NOT_RUN"
    elif not needed <= have or not lengths_ok or [r for r in rows if r["status"] != "RUN"]:
        verdict = "INCOMPLETE"
    else:
        worst = max(row["minor_faults_per_accepted_step"] for row in reuse)
        verdict = "PASS" if worst <= REUSE_FAULTS_PER_ATTEMPT_MAX else "FAIL"
    return {
        "rows": rows,
        "summary": summary,
        "reuse_worst_minor_faults_per_accepted_step": max(
            [row["minor_faults_per_accepted_step"] for row in reuse], default=None
        ),
        "reuse_runs": len(reuse),
        "link_lengths_as_requested": lengths_ok,
        "verdict": verdict,
    }


# ---------------------------------------------------------------------------------------------
# Timing invocations.
# ---------------------------------------------------------------------------------------------


def reference_solutions():
    """phi(t_end) of the skewnet cells, evaluated here with mpmath."""
    out = {}
    for name in sorted(last_invocations("reference")):
        for row in rows_of(name):
            if row["type"] != "reference_inputs":
                continue
            p = [mpmath.mpf(v) for v in from_hex(row["p_hex"])]
            q = [mpmath.mpf(v) for v in from_hex(row["q_hex"])]
            t = mpmath.mpf(row["t_end"])
            alpha = mpmath.mpf(row["alpha_amplitude"]) * mpmath.sin(
                mpmath.mpf(row["alpha_frequency"]) * t + mpmath.mpf(row["alpha_phase"])
            )
            beta = mpmath.mpf(row["beta_amplitude"]) * mpmath.cos(mpmath.mpf(row["beta_frequency"]) * t)
            phi = [mpmath.atanh(alpha * pj + beta * qj) for pj, qj in zip(p, q)]
            runner_phi = from_hex(row["exact_end_hex"])
            out[row["cell"]] = {
                "phi": phi,
                "t_end": row["t_end"],
                "n": row["n"],
                "log": name,
                "runner_value_max_abs_difference": float(
                    max(abs(mpmath.mpf(a) - b) for a, b in zip(runner_phi, phi))
                ),
            }
    return out


REFERENCES = reference_solutions()


def parse_measure(name, entry):
    """One timing invocation: per-arm statistics and the quality label."""
    data = rows_of(name)
    header = next((r for r in data if r["type"] == "header"), None)
    footer = next((r for r in data if r["type"] == "footer"), None)
    if entry["exit_code"] != 0 or header is None or footer is None:
        return {"log": name, "status": "FAILED", "exit_code": entry["exit_code"], "authoritative": False}
    arms = {}
    for row in data:
        if row["type"] == "run" and row["kind"] == "timed":
            arms.setdefault(row["arm"], []).append(row)
    pairs = {}
    for row in data:
        if row["type"] == "pair":
            pairs.setdefault(row["arm"], []).append(row)
    end_states = {row["arm"]: row for row in data if row["type"] == "end_state"}
    cell = header["cell"]["id"]
    reference = REFERENCES.get(cell)
    stats = {}
    for arm, runs in arms.items():
        walls = [r["wall_ns"] for r in runs]
        fingerprints = sorted({r["end_state_fnv1a64"] for r in runs})
        attempts = sorted({r["attempts"] for r in runs})
        steady_steps = sum(r["steady_steps"] for r in runs)
        item = {
            "repetitions": len(runs),
            "wall_ns": walls,
            "wall_median_ns": median(walls),
            "wall_min_ns": min(walls),
            "wall_max_ns": max(walls),
            "spread": (max(walls) - min(walls)) / median(walls),
            "attempts": attempts[0] if len(attempts) == 1 else None,
            "attempts_distinct": attempts,
            "accepted_steps": runs[0]["accepted_steps"],
            "rejected_steps": runs[0]["rejected_steps"],
            "complete": all(r["complete"] for r in runs),
            "end_state_fingerprints": fingerprints,
            "deterministic": len(fingerprints) == 1 and len(attempts) == 1,
            "wall_median_ms_per_attempt": median(walls) / 1.0e6 / runs[0]["attempts"],
            "run_minor_faults_median": median([r["minor_faults"] for r in runs]),
            "run_minor_faults_per_attempt_median": median([r["minor_faults"] / r["attempts"] for r in runs]),
            "steady_minor_faults_per_accepted_step": (
                sum(r["steady_minor_faults"] for r in runs) / steady_steps if steady_steps else None
            ),
            "major_faults_total": sum(r["major_faults"] for r in runs),
            "error_max_abs_runner": runs[0]["error_max_abs"],
            "rows_moved_by_last_factorization": runs[0]["rows_moved_by_last_factorization"],
        }
        if arm in pairs:
            item["bit_identical_to_protected_in_every_repetition"] = all(p["bit_identical"] for p in pairs[arm])
            item["largest_end_state_difference_to_protected"] = max(
                [p["end_state_max_abs_difference"] or 0.0 for p in pairs[arm]]
            )
        if arm in end_states and reference is not None and end_states[arm]["end_time"] == reference["t_end"]:
            y = from_hex(end_states[arm]["y_hex"])
            error = float(max(abs(mpmath.mpf(a) - b) for a, b in zip(y, reference["phi"])))
            item["error_max_abs"] = error
            item["error_cross_check_difference"] = abs(error - runs[0]["error_max_abs"])
            item["error_valid"] = (
                item["error_cross_check_difference"] <= REFERENCE_CROSS_CHECK
                and item["deterministic"]
                and end_states[arm]["end_state_fnv1a64"] == fingerprints[0]
            )
        stats[arm] = item
    load_before = header["loadavg_before"][0]
    load_after = footer["loadavg_after"][0]
    worst_spread = max(item["spread"] for item in stats.values())
    return {
        "log": name,
        "status": "RUN",
        "try": entry.get("try"),
        "cell": cell,
        "n": header["cell"]["n"],
        "rtol": header["cell"]["rtol"],
        "t_span": header["cell"]["t_span"],
        "max_attempts": header["cell"]["max_attempts"],
        "argv0_length": header["argv0_length"],
        "link_length_requested": entry["link_length"],
        "cpus_allowed_list": header["cpus_allowed_list"],
        "runner_sha256": entry["runner_sha256"],
        "loadavg_before": header["loadavg_before"],
        "loadavg_after": footer["loadavg_after"],
        "elapsed_s": footer["elapsed_s"],
        "worst_spread": worst_spread,
        "authoritative": load_before <= LOAD_LIMIT and load_after <= LOAD_LIMIT and worst_spread <= SPREAD_LIMIT,
        "quality_misses": [
            text
            for text, missed in (
                (f"1-minute load before {load_before}", load_before > LOAD_LIMIT),
                (f"1-minute load after {load_after}", load_after > LOAD_LIMIT),
                (f"spread {worst_spread:.3f}", worst_spread > SPREAD_LIMIT),
            )
            if missed
        ],
        "arms": stats,
    }


def invocation(stage, base):
    """All tries of one timing invocation and the one the rule selects."""
    entries = last_invocations(stage)
    tries = []
    # Round 1 is the node's own run (at most three tries); a later round, if any, is a
    # re-measurement that keeps the earlier logs (suffix .round<N>).
    for round_index in range(1, 10):
        suffix = "" if round_index == 1 else f".round{round_index}"
        for attempt in (1, 2, 3):
            name = f"{base}{suffix}.try{attempt}.jsonl"
            if name in entries:
                parsed = parse_measure(name, entries[name])
                parsed["round"] = round_index
                tries.append(parsed)
    if not tries:
        return {"base": base, "status": "NOT_RUN", "tries": [], "label": None, "used": None}
    used = next((t for t in tries if t["authoritative"]), None)
    shown = used if used is not None else next((t for t in reversed(tries) if t["status"] == "RUN"), None)
    return {
        "base": base,
        "status": "RUN" if shown is not None else "FAILED",
        "tries": tries,
        "label": "authoritative" if used is not None else "diagnostic_only",
        "used": shown,
    }


def ratio(inv, numerator, denominator):
    used = inv["used"]
    if used is None or numerator not in used["arms"] or denominator not in used["arms"]:
        return None
    return used["arms"][numerator]["wall_median_ns"] / used["arms"][denominator]["wall_median_ns"]


def reuse_wall_section():
    cells = {}
    for cell in ("skewnet.n512", "meanfield.n512"):
        variants = []
        for length in VARIANTS:
            inv = invocation("wall", f"wall.{cell}.rtol1e-6.len{length}")
            used = inv["used"]
            variants.append(
                {
                    "argv0_length_requested": length,
                    "argv0_length": used["argv0_length"] if used else None,
                    "label": inv["label"],
                    "status": inv["status"],
                    "speedup_protected_over_reuse": ratio(inv, "protected", "reuse"),
                    "invocation": inv,
                }
            )
        cells[cell] = {"variants": variants, "verdict": wall_verdict(variants)}
    everything = [v for cell in cells.values() for v in cell["variants"]]
    return {"cells": cells, "verdict_combined": wall_verdict(everything)}


def wall_verdict(variants):
    if all(v["status"] == "NOT_RUN" for v in variants):
        return "NOT_RUN"
    if any(v["status"] != "RUN" or v["speedup_protected_over_reuse"] is None for v in variants):
        return "INCOMPLETE"
    if any(v["argv0_length"] != v["argv0_length_requested"] for v in variants):
        return "INVALID"
    speedups = [v["speedup_protected_over_reuse"] for v in variants]
    authoritative = all(v["label"] == "authoritative" for v in variants)
    if not authoritative:
        return "NOT_DECIDABLE"
    if all(s >= REUSE_WALL_PASS for s in speedups):
        return "PASS"
    if all(s < REUSE_WALL_KILL for s in speedups):
        return "KILL"
    return "FAIL"


def interpolate(points, error):
    """T'(e) of the matched-error rule; None when e is too far outside the curve."""
    points = sorted(points)
    errors = [math.log(e) for e, _ in points]
    walls = [math.log(t) for _, t in points]
    x = math.log(error)
    if x < errors[0]:
        if points[0][0] / error > EXTRAPOLATION_FACTOR:
            return None
        lo, hi = 0, 1
    elif x > errors[-1]:
        if error / points[-1][0] > EXTRAPOLATION_FACTOR:
            return None
        lo, hi = len(points) - 2, len(points) - 1
    else:
        hi = next(i for i, value in enumerate(errors) if value >= x)
        hi = max(hi, 1)
        lo = hi - 1
    slope = (walls[hi] - walls[lo]) / (errors[hi] - errors[lo])
    return math.exp(walls[lo] + slope * (x - errors[lo]))


def threaded_section():
    sizes = {}
    for cell in ("skewnet.n512", "skewnet.n2048"):
        invocations = []
        for rtol in CELLS[cell]["rtols"]:
            inv = invocation("wp", f"wp.{cell}.rtol{rtol_text(rtol)}")
            inv["rtol"] = rtol
            invocations.append(inv)
        ran = [inv for inv in invocations if inv["status"] == "RUN"]
        curves = {}
        for inv in ran:
            for arm, item in inv["used"]["arms"].items():
                curves.setdefault(arm, []).append(
                    {
                        "rtol": inv["rtol"],
                        "error_max_abs": item.get("error_max_abs"),
                        "error_valid": item.get("error_valid", False),
                        "wall_median_ns": item["wall_median_ns"],
                        "attempts": item["attempts"],
                        "spread": item["spread"],
                        "label": inv["label"],
                        "bit_identical_to_protected": item.get("bit_identical_to_protected_in_every_repetition"),
                    }
                )
        workers = {}
        for arm in ("threaded2", "threaded4", "threaded6", "reuse"):
            if arm not in curves or "protected" not in curves or len(curves[arm]) != len(CELLS[cell]["rtols"]):
                continue
            usable = all(p["error_valid"] for p in curves[arm]) and all(p["error_valid"] for p in curves["protected"])
            points = []
            if usable:
                threaded_points = [(p["error_max_abs"], p["wall_median_ns"]) for p in curves[arm]]
                for protected, other in zip(curves["protected"], curves[arm]):
                    matched = interpolate(threaded_points, protected["error_max_abs"])
                    points.append(
                        {
                            "rtol": protected["rtol"],
                            "error_protected": protected["error_max_abs"],
                            "error_arm": other["error_max_abs"],
                            "error_ratio": other["error_max_abs"] / protected["error_max_abs"],
                            "attempts_protected": protected["attempts"],
                            "attempts_arm": other["attempts"],
                            "attempts_relative_difference": abs(other["attempts"] - protected["attempts"])
                            / protected["attempts"],
                            "wall_protected_ns": protected["wall_median_ns"],
                            "wall_arm_at_equal_tolerance_ns": other["wall_median_ns"],
                            "wall_arm_at_matched_error_ns": matched,
                            "speedup_at_matched_error": protected["wall_median_ns"] / matched if matched else None,
                            "speedup_at_equal_tolerance": protected["wall_median_ns"] / other["wall_median_ns"],
                            "bit_identical_to_protected": other["bit_identical_to_protected"],
                            "label": protected["label"],
                        }
                    )
            speedups = [p["speedup_at_matched_error"] for p in points]
            workers[arm] = {
                "usable": usable and all(s is not None for s in speedups) and bool(points),
                "points": points,
                "speedup_min": min(speedups) if points and all(s is not None for s in speedups) else None,
                "speedup_max": max(speedups) if points and all(s is not None for s in speedups) else None,
                "attempts_within_1_percent": all(
                    p["attempts_relative_difference"] <= THREADED_ATTEMPT_TOLERANCE for p in points
                )
                if points
                else None,
                "error_ratio_within_band": all(
                    THREADED_ERROR_BAND[0] <= p["error_ratio"] <= THREADED_ERROR_BAND[1] for p in points
                )
                if points
                else None,
                "bit_identical_at_every_tolerance": all(p["bit_identical_to_protected"] for p in points)
                if points
                else None,
                "all_authoritative": all(p["label"] == "authoritative" for p in points) if points else None,
            }
        sizes[cell] = {
            "invocations": invocations,
            "curves": curves,
            "matched_error": workers,
            "all_run": len(ran) == len(invocations),
        }

    def reading(cell):
        size = sizes[cell]
        six = size["matched_error"].get("threaded6")
        if not size["invocations"] or all(inv["status"] == "NOT_RUN" for inv in size["invocations"]):
            return "NOT_RUN", None
        if not size["all_run"] or six is None or not six["usable"]:
            return "INCOMPLETE", six
        if not six["all_authoritative"]:
            return "NOT_DECIDABLE", six
        return "DECIDABLE", six

    state_512, six_512 = reading("skewnet.n512")
    state_2048, six_2048 = reading("skewnet.n2048")

    def passes(six):
        return (
            six["speedup_min"] >= THREADED_PASS and six["attempts_within_1_percent"] and six["error_ratio_within_band"]
        )

    if state_512 == "NOT_RUN" and state_2048 == "NOT_RUN":
        pass_verdict = "NOT_RUN"
    elif "INCOMPLETE" in (state_512, state_2048) or "NOT_RUN" in (state_512, state_2048):
        pass_verdict = "INCOMPLETE"
    elif "NOT_DECIDABLE" in (state_512, state_2048):
        pass_verdict = "NOT_DECIDABLE"
    else:
        pass_verdict = "PASS" if passes(six_512) and passes(six_2048) else "FAIL"
    if state_2048 in ("NOT_RUN", "INCOMPLETE", "NOT_DECIDABLE"):
        kill_verdict = state_2048
    else:
        kill_verdict = "KILL" if six_2048["speedup_max"] < THREADED_KILL else "NOT_TRIGGERED"
    per_size = {}
    for cell, state, six in (("skewnet.n512", state_512, six_512), ("skewnet.n2048", state_2048, six_2048)):
        per_size[cell] = {
            "state": state,
            "meets_pass_thresholds": passes(six) if state == "DECIDABLE" else None,
            "speedup_min_6_workers": six["speedup_min"] if six else None,
            "speedup_max_6_workers": six["speedup_max"] if six else None,
        }
    probe = LOGS / "threaded_faer_par_compile_probe.log"
    feature = LOGS / "threaded_faer_feature_state.log"
    probe_text = probe.read_text() if probe.exists() else ""
    feature_text = feature.read_text() if feature.exists() else ""
    features = sorted(
        {line.split('"')[1] for line in feature_text.splitlines() if 'faer feature "' in line and line.count('"') >= 2}
    )
    return {
        "faer_par_argument": {
            "verdict": "NOT_RUN",
            "reason": PREDECLARED["threaded_arm_mechanism"]["finding"],
            "compile_probe_log": probe.name if probe.exists() else None,
            "compile_probe_exit_code": next(
                (int(line.split(":")[1]) for line in probe_text.splitlines() if line.startswith("# exit:")), None
            ),
            "compile_probe_error": next(
                (line.strip() for line in probe_text.splitlines() if line.startswith("error[")), None
            ),
            "faer_features_in_the_build": features,
            "feature_log": feature.name if feature.exists() else None,
        },
        "client_side_arm": {
            "definition": PREDECLARED["threaded_arm_mechanism"]["arm_that_is_built_and_measured"],
            "sizes": sizes,
            "per_size": per_size,
            "pass_criterion": pass_verdict,
            "kill_criterion": kill_verdict,
        },
    }


def exploratory_section():
    out = {"timing": [], "instructions": []}
    for base, arms in (
        ("extra.meanfield.n512.rtol1e-6.threads", ("threaded2", "threaded4", "threaded6")),
        ("extra.meanfield.n2048.rtol1e-6.threads", ("threaded2", "threaded4", "threaded6")),
        ("extra.skewnet.n2048.rtol1e-4.reuse", ("reuse",)),
    ):
        inv = invocation("extra", base)
        out["timing"].append(
            {
                "base": base,
                "status": inv["status"],
                "label": inv["label"],
                "speedup_over_protected_at_equal_tolerance": {arm: ratio(inv, "protected", arm) for arm in arms},
                "invocation": inv,
            }
        )
    perf = {}
    for name, entry in sorted(last_invocations("perf").items()):
        single = next((r for r in rows_of(name) if r["type"] == "single"), None)
        csv = LOGS / entry["perf_csv"]
        if entry["exit_code"] != 0 or single is None or not csv.exists():
            out["instructions"].append({"log": name, "status": "FAILED", "exit_code": entry["exit_code"]})
            continue
        counts = {}
        for line in csv.read_text().splitlines():
            fields = line.split(",")
            if len(fields) > 2 and fields[0].isdigit():
                counts[fields[2]] = int(fields[0])
        perf[(entry["cell"], entry["arm"])] = {
            "log": name,
            "perf_csv": entry["perf_csv"],
            "runs": single["runs"],
            "attempts_total": single["attempts_total"],
            "instructions_user": counts.get("instructions:u"),
            "minor_faults": counts.get("minor-faults"),
        }
    for cell in sorted({key[0] for key in perf}):
        base = perf.get((cell, "none"))
        row = {"cell": cell, "status": "RUN", "set_up": base, "arms": {}}
        for arm in ("protected", "kit", "reuse"):
            item = perf.get((cell, arm))
            if item is None or base is None or item["instructions_user"] is None:
                continue
            attempts = item["attempts_total"]
            row["arms"][arm] = dict(
                item,
                instructions_user_per_attempt=(item["instructions_user"] - base["instructions_user"]) / attempts,
                minor_faults_per_attempt=(item["minor_faults"] - base["minor_faults"]) / attempts,
            )
        protected = row["arms"].get("protected")
        for arm in ("kit", "reuse"):
            if protected and arm in row["arms"]:
                row["arms"][arm]["instructions_ratio_to_protected"] = (
                    row["arms"][arm]["instructions_user_per_attempt"] / protected["instructions_user_per_attempt"]
                )
        out["instructions"].append(row)
    return out


def main():
    identity = identity_section()
    allocations = allocation_section()
    faults = fault_section()
    wall = reuse_wall_section()
    threaded = threaded_section()
    parts = {
        "identity": identity["verdict"],
        "allocations": allocations["verdict"],
        "page_faults": faults["verdict"],
        "wall": wall["verdict_combined"],
    }
    if all(v == "PASS" for v in parts.values()):
        reuse_pass = "PASS"
    elif any(v in ("NOT_RUN", "INCOMPLETE", "INVALID") for v in parts.values()):
        reuse_pass = "INCOMPLETE"
    elif parts["wall"] == "NOT_DECIDABLE" and all(v == "PASS" for k, v in parts.items() if k != "wall"):
        reuse_pass = "NOT_DECIDABLE"
    else:
        reuse_pass = "FAIL"
    if parts["identity"] == "FAIL":
        reuse_kill = "KILL (bit identity)"
    elif parts["wall"] == "KILL":
        reuse_kill = "KILL (wall)"
    elif parts["wall"] in ("PASS", "FAIL") and parts["identity"] == "PASS":
        reuse_kill = "NOT_TRIGGERED"
    else:
        reuse_kill = parts["wall"] if parts["identity"] == "PASS" else parts["identity"]
    failed = [
        {"stage": e.get("stage"), "log": e["log"], "exit_code": e["exit_code"]}
        for e in MANIFEST
        if "exit_code" in e and e["exit_code"] != 0
    ]
    logs = sorted(p for p in LOGS.iterdir() if p.is_file())
    results = {
        "schema": "vigilode.loop_l1.v18_dense_lu_client.results.v1",
        "node": "V18_DENSE_LU_CLIENT",
        "predeclared": "PREDECLARED.json",
        "frozen_thresholds": {
            "reuse_wall_pass": REUSE_WALL_PASS,
            "reuse_wall_kill": REUSE_WALL_KILL,
            "reuse_minor_faults_per_attempt_max": REUSE_FAULTS_PER_ATTEMPT_MAX,
            "threaded_pass": THREADED_PASS,
            "threaded_kill": THREADED_KILL,
            "threaded_attempt_tolerance": THREADED_ATTEMPT_TOLERANCE,
            "threaded_error_band": list(THREADED_ERROR_BAND),
        },
        "timing_quality_rule": {"load_limit": LOAD_LIMIT, "spread_limit": SPREAD_LIMIT},
        "sources": {
            "manifest_entries": len(MANIFEST),
            "runner_sha256": sorted({e["runner_sha256"] for e in MANIFEST if "runner_sha256" in e}),
            "heap_layout_links": [e for e in MANIFEST if e.get("stage") == "links" and "link" in e][-3:],
            "logs": [{"name": p.name, "bytes": p.stat().st_size, "sha256": sha256(p)} for p in logs],
            "invocations_with_non_zero_exit": failed,
            "wait_budget": json.loads((LOGS / "WAIT_BUDGET.json").read_text())
            if (LOGS / "WAIT_BUDGET.json").exists()
            else {"waited_s": 0.0},
        },
        "references": {
            cell: {k: v for k, v in ref.items() if k != "phi"} | {"digits": mpmath.mp.dps}
            for cell, ref in REFERENCES.items()
        },
        "identity": identity,
        "allocations": allocations,
        "page_faults": faults,
        "reuse_wall": wall,
        "threaded": threaded,
        "exploratory": exploratory_section(),
        "verdicts": {
            "reuse_parts": parts,
            "reuse_pass_criterion": reuse_pass,
            "reuse_kill_criterion": reuse_kill,
            "reuse_wall_per_cell": {cell: item["verdict"] for cell, item in wall["cells"].items()},
            "threaded_faer_par_argument": threaded["faer_par_argument"]["verdict"],
            "threaded_client_side_pass_criterion": threaded["client_side_arm"]["pass_criterion"],
            "threaded_client_side_kill_criterion": threaded["client_side_arm"]["kill_criterion"],
            "threaded_client_side_per_size": threaded["client_side_arm"]["per_size"],
        },
    }
    text = json.dumps(results, indent=1, sort_keys=True, allow_nan=False)
    (HERE / "RESULTS.json").write_text(text + "\n")
    print(json.dumps(results["verdicts"], indent=1, sort_keys=True))


if __name__ == "__main__":
    sys.exit(main())
