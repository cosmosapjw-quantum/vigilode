#!/usr/bin/env python3
"""Loop node V25: writes PROTOCOL.json, every protocol choice that the frozen
PREDECLARED.json leaves open, with the final spans read from the sizing
pilot's logs (logs/sizing/*, no typed-in number). Committed before any run
of P1 or P2.

usage: python3 -I build_protocol.py
"""
import glob
import hashlib
import json
import os
import struct
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
SIZING = os.path.join(NODE, "logs", "sizing")
FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
BINARY_SHA256 = "9582b047669995dcfc3830f0675d39823f28ba7ba528d558d4625047c49f469f"
BINARY_PATH = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v25/v25-bins/mf_confirm_measure.956b578d"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_record(path):
    record = state = begin = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("# V25_LOG_BEGIN "):
                begin = json.loads(line[len("# V25_LOG_BEGIN "):])
            elif line.startswith("V25_RUN_JSON "):
                record = json.loads(line.split(" ", 1)[1])
            elif line.startswith("V25_STATE_JSON "):
                state = json.loads(line.split(" ", 1)[1])
    return begin, record, state


def sizing(family):
    with open(os.path.join(SIZING, f"{family}__FINAL.txt"), encoding="utf-8") as handle:
        final = json.loads(handle.readline())
    runs = []
    for path in glob.glob(os.path.join(SIZING, f"{family}__T*__S__route.log")):
        begin, record, state = read_record(path)
        values = [struct.unpack(">d", bytes.fromhex(state["state_hex"][i:i + 16]))[0]
                  for i in range(0, len(state["state_hex"]), 16)] if state else []
        runs.append({
            "pilot_run_index": int(begin["env"]["V25_LABEL"].rsplit(" ", 1)[1]),
            "log": os.path.relpath(path, NODE), "date": begin["date"], "binary_sha256": begin["binary_sha256"],
            "loadavg_before_launch": begin["loadavg_before_launch"],
            "t_span": record["t_span"], "wall_seconds": record["wall_seconds_diagnostic_only"],
            "completed": record["completed"], "success": record["success"], "attempts": record["attempts"],
            "end_state_min": min(values) if values else None, "end_state_max": max(values) if values else None,
        })
    runs.sort(key=lambda run: run["pilot_run_index"])
    return final, runs


def main():
    families = {
        "ac3d": {
            "generator": "mf_confirm_problems.rs: declared(\"ac3d\") = ac3d(16)",
            "n": 4096,
            "layout": "index i + m (j + m k), cell centre ((i + 0.5)/m, (j + 0.5)/m, (k + 0.5)/m) (frozen)",
        },
        "robertson_rd1d": {
            "generator": "mf_confirm_problems.rs: declared(\"robertson_rd1d\") = the V06 generator v06::robertson_rd1d(400) with atol_over_rtol 1.0",
            "n": 1200,
            "layout": "interleaved (y1, y2, y3) per cell, x_i = (i + 1/2) dx, dx = 1/400 (V06 generator)",
        },
        "schnakenberg2d": {
            "generator": "mf_confirm_problems.rs: declared(\"schnakenberg2d\") = schnakenberg2d(32)",
            "n": 2048,
            "layout": "interleaved, index 2 (i + m j) + s, cell centre ((i + 0.5)/m, (j + 0.5)/m) (frozen)",
        },
        "porous2d": {
            "generator": "mf_confirm_problems.rs: declared(\"porous2d\") = porous2d(48)",
            "n": 2304,
            "layout": "index i + m j, cell centre ((i + 0.5)/m, (j + 0.5)/m) (the frozen file states no layout; this one is the layout of the other 2D families)",
        },
    }
    family_list = []
    for family in FAMILIES:
        final, runs = sizing(family)
        entry = {"id": family, **families[family], "t_span": [0.0, final["final_end"]],
                 "reference": f"reference/{family}.json",
                 "sizing": {"doublings": final["doublings"], "halved": final["halved"], "runs": runs}}
        family_list.append(entry)

    protocol = {
        "schema": "vigilode.loop-l1.v25-protocol.v1",
        "node": "V25_MF_P1_CONFIRMATION",
        "written": time.strftime("%Y-%m-%dT%H:%M:%S%z") + " by build_protocol.py, after the sizing pilot of arm S and before any run of P1 or P2",
        "role": "PREDECLARED.json of this directory is frozen and is not edited. This file fixes every protocol choice it leaves open and is committed before the first run of P1 or P2. A later correction is an entry under 'amendments' with its date and its position relative to the runs; git history keeps the order.",
        "frozen_file": {"path": "research/production_loop_20261116/mf_p1_confirm/PREDECLARED.json",
                        "sha256": sha256_file(os.path.join(NODE, "PREDECLARED.json"))},
        "base_commit": "80ba83881503c4b3253268df61c53b269740d6d3",
        "families": family_list,
        "sizing_pilot": {
            "frozen_rule": "the shipped policy alone at rtol 1e-7: a run longer than 60 s halves the span until it does not; a run shorter than 1 s doubles it, at most three times; logs kept and labelled sizing; no pilot number enters a verdict",
            "execution": "run_measurements.py phase sizing (committed at 6d87f486 before the pilot): arm S, entry route, rtol = atol = 1e-7, work partition (taskset -c 0-5,12-17), one family at a time, one run at a time; the wall is the run's own clock around the solve (wall_seconds_diagnostic_only); after a halving no doubling follows",
            "binary_sha256": BINARY_SHA256,
            "observation_not_a_verdict": "the end state of ac3d at t = 1 lies within about 3e-5 of the uniform equilibrium u = 1 (end_state_min/max of its run below); the frozen rule keeps the span [0, 1] (3.0 s). Robertson reached three doublings below 1 s and stops at [0, 320].",
        },
        "tolerances": {
            "rtol": [1e-3, 1e-4, 1e-5, 1e-6, 1e-7],
            "atol": "equal to rtol for every family (frozen). robertson_rd1d: the V06 generator carries atol_over_rtol = 1e-4, a tolerance choice of node V06 and not one of the 'rates, diffusion 1e-5 / dx^2, initial state' that the frozen family text keeps; the frozen tolerance block (atol equal to rtol) decides, so atol_over_rtol = 1.0 (mf_confirm_problems.rs::robertson_rd1d).",
            "cells": 20,
        },
        "integrator_configuration": "the configuration of the matrix-free route in V24 (frozen 'controller'): problems::adaptive of stage_clients_krylov_v17_problems.rs, unchanged: initial_step 1e-3, min_step 1e-14, max_step t1 - t0, max_attempts 1000000, AdaptiveStepConfig::default() otherwise; output schedule [t0, t1]",
        "linear_configuration": "RoutingOptions::default().matrix_free: GMRES, restart 40, budget max(maxiter, restart) = 200 Arnoldi vectors, rtol 1e-10, atol 1e-14 (stage threshold max(gamma 1e-14 rounded down, 1e-10 ||rhs||_2)), no preconditioner, zero start",
        "arms": {
            "S": "MfStageSolvePolicy::Shipped, named explicitly (driver identifier rodas5p-mf-fast-transformed-v1)",
            "P1": "MfStageSolvePolicy::ColumnTested, named explicitly (identifier at commit A: rodas5p-mf-fast-transformed-v2-candidate-p1)",
            "P2": "MfStageSolvePolicy::ColumnTestedDoubling, named explicitly; reported only",
            "entries": {
                "route": "audit off, under perf stat: S = integrate_rodas5p_mf_fast_observed_gmres_into (the shipped entry, what the router calls for S); P1, P2 = integrate_rodas5p_mf_fast_observed_with_stage_solve_policy(policy, ls_workspace = false) (what the router calls for them)",
                "router": "integrate_rodas5p_routed_observed_with_options with RoutingOptions.matrix_free_policy = the arm's policy, krylov_ls_workspace = false",
                "router_ls": "the same with krylov_ls_workspace = true (the facade's call, with the policy named)",
                "audit": "Rodas5pMfFastWorkspace with set_gmres_into(true), set_stage_solve_policy(arm), set_solve_log(true), set_stage_solve_records(true), an attempt observer with ProtectedAudit of V17 (unchanged) and the per-solve recorder of mf_confirm_measure.rs (per-solve log file, gzip, kept under logs/)",
            },
        },
        "binary": {
            "sha256": BINARY_SHA256, "path": BINARY_PATH, "built_at_commit": "956b578d",
            "rule": "every run of steps 2 to 5 (sizing, setup, consistency, commit A, P2, timing) uses this one binary; a measurement-target or driver change after this point needs a new binary, which is used only for D0 after an edit (step 6) or for runs labelled exploratory",
        },
        "reference": {
            "rule": "mf_stage_solve/PREDECLARED.json 'reference', unchanged: DOP853 rtol = atol = 1e-13 on the numpy right-hand side (reference/families.py), cross-checks DOP853 1e-11 and Radau 1e-11 with the finite-difference sparse Jacobian (jac_sparsity of families.py); if DOP853 1e-13 does not finish within 2 hours, Radau 1e-12 is the reference and Radau 1e-10 the cross-check",
            "script": "reference/make_reference.py --family <id> --out reference/<id>.json (spans from this file); its console output is kept in logs/reference/<id>.log",
            "uncertainty": "u_ref = max over the cross-checks of max_i |ref_i - other_i| / (1 + |ref_i|), per family",
            "consistency": "done before any reference (CONSISTENCY.json, logs/consistency): 20 states, every component bitwise equal; in addition the parser compares the Rust start state and f(t0, y0) (mode RHS0) with y0_hex and f0_hex of the reference file (V17 rule: 1e-12 in the max norm relative to max |f0|)",
        },
        "error_metric": "V17 primary metric, unchanged: E = max_i |y_i(t1) - ref_i| / (1 + |ref_i|) from the end state printed by the audit-off route run (hex bits); E < 10 u_ref is reference limited (flagged; matched levels below 10 u_ref are outside the compared range)",
        "matched_error": "the functions curve_value and matched of mf_stage_solve/parse_results.py (V17), imported, not re-written: arm S in the role of arm A (smallest interpolated value where segments overlap), P1 in the role of the candidate (largest); 201 points on [E_lo, E_hi] in log E with E_lo = max(min E_S, min E_P1, 10 u_ref), E_hi = min(max E_S, max E_P1); NOT_DECIDABLE with fewer than 4 completed tolerances in an arm or without an overlapping range; no extrapolation",
        "discriminators": {
            "D0": {
                "before_any_edit": "no file of the driver or the router is edited before step 6; D0 at the decision is the statement that git diff of crates/rodas5p-integrators/src since commit A is empty (checked by the parser with git) and that every timed run ends in the state of its instruction run",
                "after_an_edit": "step 6 only: run_measurements.py phase d0 with the new binary, arms S and P1 named explicitly, entries route, router, router_ls and audit on the 20 cells; every field of RESULT_KEYS (success, attempts, accepted, rejected, state_reuses, t_end_bits, output_rows, every counter, state_sha256, the full end state hex) and, for audit, the solve-log digest, residual_audit, record_audit, policy_statistics and the raw sha256 of the per-solve log, equal to commit A. The driver identifier of P1 is expected to change (candidate -> v3) and is reported, not compared.",
            },
            "V": {
                "audited": "every audit run of P1 on the 20 cells (deciding); P2's audit runs are reported and a violation there is reported as well",
                "check_1_independent_from_the_solve_log": "every 'S' line of the per-solve log: violation iff the independently rebuilt residual ||r_i||_2 is not finite or exceeds max(gamma atol_lin rounded down, rtol_lin ||rhs_i||_2); recounted by the parser from the .solves.txt.gz file",
                "check_2_driver_records": "every 'S' line: violation iff the driver's reported true residual is not finite or exceeds the threshold it used",
                "check_3_coverage": "lines of the per-solve log = linear_solves counter = the driver's records = ProtectedAudit checks; failed solves: budget_exhausted_solves + other_linear_failures (driver statistics) = linear_solve_failures; a mismatch is a violation",
                "check_4_determinism": "the audit run and the route run of the arm in the cell agree in every result field and every bit of the end state; a mismatch voids the cell (V17 rule) and V is then NOT_DECIDABLE for it",
                "cross_check": "ProtectedAudit (V17 code) failures must equal the recounted failures of check 1 (reported)",
                "budget_failures": "a solve that ends by the budget is CoreError::LinearSolve('GMRES exhausted ...'), classified by the driver; the attempt is rejected the shipped way; reported, not a violation",
                "verdict": "PASS iff 0 violations and every cell audited and deterministic; KILL by the frozen verdicts if any violation",
            },
            "B": "per cell: budget_exhausted_solves of P1 <= that of S (MfStageSolveStatistics of the audit runs; the audit and route runs are deterministic twins by check 4)",
            "A": "per cell E_P1 <= 3 E_S at equal tolerance (threshold of A6); reference-limited cells are flagged and decided the same way",
            "S": "per family the geometric mean over the five tolerances of attempts_P1 / attempts_S <= 1.05 and every cell <= 1.15 (attempts = accepted + rejected; thresholds of A6); a family with a run that did not complete is NOT_DECIDABLE",
            "E": {
                "work": "V17 instruction measure, unchanged: perf stat -e instructions:u of the route run (binary called directly) minus the median of the three SETUP runs of the family with the same binary",
                "family_part": "the V17 family statistic of P1 against S (geometric mean over 201 points of W_P1(E)/W_S(E)) must be below 1.00 in every family",
                "cell_part": "per cell: P1's interpolated instructions at the error S achieved in that cell (curve_value with max) divided by S's instructions in that cell, must be at most 1.00. If that error lies outside the matched range (outside P1's error range, or below 10 u_ref), the ratio at equal tolerance is used and the cell is labelled so (the rule of V24, PROTOCOL.json of mf_adaptive). Reported next to it: for such a cell the cheapest P1 point whose error is at most E_S (an upper bound when it exists), and whether the verdict of the cell would differ",
                "also_reported": "the smallest and largest ratio on the 201-point grid, the ratio at every S point inside the range, the ratio at equal tolerance, the same statistics for attempts and JVPs",
            },
            "W": {
                "protocol": "timing protocol of V17 with two arms S and P1 (P2 is not timed): 9 rounds per cell, order rotated by the round index (S P1, P1 S, ...), rounds 0-1 warm-ups, 2-8 the 7 repetitions, median per arm; prefix flock <lock> taskset -c 6-11; the wrapper reads /proc/loadavg immediately before and after every invocation",
                "invocations": "no invocation may hold the lock for two minutes: if 9 x (wall_S + wall_P1) of the instruction runs exceeds 90 s, the nine rounds are nine invocations of one round each; if one round exceeds 90 s, each run is its own invocation in the order of the rotation",
                "quality_rule": "authoritative iff the 1-minute load is at most 8 before and after every invocation of the measurement and the spread (max - min)/median of each arm is at most 0.10, every timed run ends in the state of its instruction run; at most two re-measurements per cell (tries 1 to 3), every measurement kept, the first that meets the rule decides, otherwise the last one is used and labelled diagnostic_only",
                "waiting": "orchestrator note of 2026-11-24: before a timing block the 1-minute load and the number of running rustc processes are read once per minute, at most 20 minutes in total for the node, until the load is at most 4 and no compiler runs; then the cells are measured as the host is and labelled by the rule",
                "cell_statistic": "per cell: P1's median wall interpolated at the error S achieved in that cell (V17 curve on the medians, max) over S's median wall in that cell, with the equal-tolerance fallback of E; the cell comparison is authoritative iff every timing cell whose median enters it is authoritative",
                "verdict": "W holds iff no authoritative cell comparison is above 1.00; cells that are not authoritative are reported and decide nothing",
            },
            "G": "only after a switch: the golden files under crates/vigilode/tests/golden/default_path are regenerated by the golden test; a script compares old and new records and writes GOLDEN_DIFF.json: every changed record must belong to the matrix-free route and its end state must differ from the old record by at most 10 (atol + rtol |y_old|) in every component and its attempts by at most 20 percent; every record of another route keeps its bytes",
            "R": "only after a switch: sentinels ppsw1_6b_contracts and ptsw2b_contracts (re-pinned, SENTINEL_REPIN.json), --test 'stage_clients_*', --test 'mf_adaptive_*' --test 'mf_confirm_*' of rodas5p-integrators, the whole rodas5p-krylov suite, the whole facade suite; clippy -D warnings on rodas5p-krylov and vigilode (all targets) and on the library and the mf_confirm targets of rodas5p-integrators; rustfmt --check on the three packages",
        },
        "verdict_computation": {
            "SWITCH": "D0, V, B, A, S, E and W are PASS",
            "KILL": "a violation of V in any audited solve of P1",
            "KEEP": "V holds and at least one of D0, B, A, S, E, W is FAIL",
            "NOT_DECIDABLE": "no FAIL, V holds, and at least one of D0, B, A, S, E, W is NOT_DECIDABLE: no default changes, the obstacle is reported (status partial)",
        },
        "runs": {
            "per_cell": ["S: route (perf), router, router_ls, audit (commit A)", "P1: route (perf), router, router_ls, audit (commit A)",
                         "P2: route (perf), audit (reported only)", "timing tries of S and P1"],
            "per_family": ["SETUP x 3 under perf and RHS0 with the binary of the runs", "the reference and its log"],
            "parallelism": "runs that are not timing runs run up to four at a time inside the work partition (counters, state bits and instructions:u of a process do not depend on it)",
            "time_limit": "30 minutes per run; a run that does not finish is NOT_COMPLETED and is not a point of a curve",
            "exclusions": "none; every declared run is kept, failed and negative ones included",
            "search_budget": "none: S, P1 and P2 are fixed by the frozen file; no parameter is tried",
        },
        "known_obstacles_found_before_any_run_of_P1_or_P2": [
            "R after a switch cannot hold inside the write list: crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs:182 (MfStageSolvePolicy::default() == Shipped), :250 (RoutingOptions::default().matrix_free_policy == Shipped), :260 (routed driver id v1) and crates/rodas5p-integrators/tests/mf_adaptive_contracts.rs:410-411 (P1's candidate identifier) pin the state that a switch changes; both files are matrix-free targets of R and outside the write list. If the measurements give SWITCH, the switch is made in its own commit, G and R are evaluated and every failing test is listed verbatim; R then fails, so the switch is taken back by a new commit (git revert) and the verdict is HOLD with this reason; the lines the orchestrator would have to change are written to OUT_OF_LIST_EDITS.json.",
            "ac3d: the pilot's end state at t = 1 lies within about 3e-5 of the uniform equilibrium (V17 withdrew ac3d at m = 8 for the same reason). The frozen span rule keeps [0, 1]. If the achieved errors of ac3d cannot be matched (fewer than two S and P1 points above 10 u_ref with an overlapping range), its E and W parts are NOT_DECIDABLE and the frozen verdict cannot be SWITCH; this is reported, no other span or family is substituted.",
        ],
        "step_6_plan_if_SWITCH": {
            "driver": "rodas5p_matrix_free_fast.rs: MfStageSolvePolicy::ColumnTested.driver_id() -> 'rodas5p-mf-fast-transformed-v3' (a constant RODAS5P_MF_FAST_P1_DRIVER_ID next to the others) and #[default] on ColumnTested; the doc comments that state the default; the workspace keeps its explicit initial value Shipped and the shipped entries keep the shipped policy (NOTES.txt step 0)",
            "router": "routing.rs: RoutingOptions::default().matrix_free_policy = MfStageSolvePolicy::ColumnTested and its doc comments",
            "lib_rs": "one re-export block after the last '// Loop L1' block if the new constant must be exported",
            "facade": "solve.rs / solution.rs only where a doc comment or a record names the identifier of the matrix-free route; facade tests only in lines that pin the identifier or the bits of the default matrix-free route, each listed with its reason in RESULTS.json",
            "sentinels": "g3 of ppsw1_6b_contracts.rs and h2 of ptsw2b_contracts.rs re-pinned with a comment naming OD-L1-15; SENTINEL_REPIN.json with old and new pins",
            "then": "D0 with the new binary, G, R; if G or R does not hold the switch is taken back (git revert) and the verdict is HOLD",
        },
        "amendments": [],
    }
    path = os.path.join(NODE, "PROTOCOL.json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(protocol, handle, indent=1, allow_nan=False)
        handle.write("\n")
    print(f"wrote {path}")
    for entry in family_list:
        print(entry["id"], entry["t_span"])


if __name__ == "__main__":
    main()
