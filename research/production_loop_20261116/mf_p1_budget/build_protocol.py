#!/usr/bin/env python3
"""Loop node V28: writes PROTOCOL.json, the protocol choices that the frozen
PREDECLARED.json leaves open, before any run of a cell of this node.

Inputs: the frozen PREDECLARED.json of this node, the protocols of V25
(mf_p1_confirm/PROTOCOL.json) and V24 (mf_adaptive/PROTOCOL.json, deciding
grid) for the cells, the source files of the measurement target and of the
generators (code excerpts and digests), the measurement binary (hash). No
number of a cell is typed here: sizes and spans are read from the two
protocols.

usage: python3 -I build_protocol.py --bin BINARY --built-at COMMIT
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
LOOP = os.path.dirname(NODE)
ROOT = os.path.abspath(os.path.join(LOOP, "..", ".."))
TESTS = os.path.join(ROOT, "crates", "rodas5p-integrators", "tests")
FROZEN_SHA256_A9 = "fce3ba98b0878107f0eb58d6b4663cd64601066b3e5d4d342c0a8245834467b2"
RTOL_LABELS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
V25_FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
V24_FAMILIES = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"]
TARGET = "crates/rodas5p-integrators/tests/mf_budget_measure.rs"
GENERATOR_FILES = [
    TARGET,
    "crates/rodas5p-integrators/tests/mf_confirm_problems.rs",
    "crates/rodas5p-integrators/tests/stage_clients_krylov_v17_problems.rs",
    "crates/rodas5p-integrators/tests/stage_clients_krylov_problems.rs",
]


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=True).stdout.strip()


def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def excerpt(relative, start_marker, count_blocks=1):
    """The lines of `relative` from the first line that contains `start_marker`
    to the end of the `count_blocks`-th top-level item that follows (a line
    "}" or "};" at column 0), with their line numbers."""
    with open(os.path.join(ROOT, relative), encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    start = next(k for k, line in enumerate(lines) if start_marker in line)
    # Include the doc comment above the item.
    while start > 0 and lines[start - 1].startswith("///"):
        start -= 1
    end, seen = start, 0
    for k in range(start, len(lines)):
        if lines[k] in ("}", "};") or (lines[k].startswith("const ") and lines[k].endswith(";")):
            seen += 1
            if seen == count_blocks:
                end = k
                break
    return {"file": relative, "lines": [start + 1, end + 1], "code": "\n".join(lines[start:end + 1])}


def cells():
    v25 = load(os.path.join(LOOP, "mf_p1_confirm", "PROTOCOL.json"))
    v24 = load(os.path.join(LOOP, "mf_adaptive", "PROTOCOL.json"))
    v25_families = {family["id"]: family for family in v25["families"]}
    v24_families = {family["id"]: family for family in v24["deciding_grid"]["families"]}
    out = []
    for node, families, declared in (("V25", V25_FAMILIES, v25_families), ("V24", V24_FAMILIES, v24_families)):
        for family in families:
            source = declared[family]
            for label in RTOL_LABELS:
                index = len(out)
                if node == "V25":
                    generator = (f"with_end(problems::declared(\"{family}\")?, {source['t_span'][1]!r}) "
                                 f"[{source['generator']}]")
                    reference = f"mf_p1_confirm/reference/{family}.json"
                    recorded = {
                        "S": f"mf_p1_confirm/logs/record_a/{family}__{label}__S__audit.log",
                        "P1": f"mf_p1_confirm/logs/record_a/{family}__{label}__P1__audit.log",
                    }
                else:
                    generator = f"problems::v17::declared(\"{family}\")? [{source['generator']}]"
                    reference = source["reference"]
                    recorded = {
                        "S": f"mf_adaptive/logs/d0/{family}__{label}__S__audit.log",
                        "P1": f"mf_adaptive/logs/exploratory/{family}__{label}__P1__audit.log",
                    }
                out.append({
                    "index": index, "node": node, "family": family, "rtol_label": label, "rtol": float(label),
                    "atol": float(label), "n": source["n"], "t_span": source["t_span"], "generator": generator,
                    "source_protocol": f"{'mf_p1_confirm' if node == 'V25' else 'mf_adaptive'}/PROTOCOL.json",
                    "reference_for_the_error_report": reference,
                    "reference_sha256": sha256_file(os.path.join(LOOP, reference)),
                    "recorded_runs_for_the_reproduction_check": recorded,
                })
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True)
    parser.add_argument("--built-at", required=True)
    args = parser.parse_args()
    frozen = os.path.join(NODE, "PREDECLARED.json")
    frozen_sha = sha256_file(frozen)
    if frozen_sha != FROZEN_SHA256_A9:
        raise SystemExit("PREDECLARED.json differs from the file frozen by addendum A9")
    built_at = git("rev-parse", args.built_at)
    changed = git("diff", "--name-only", built_at, "HEAD", "--", "crates")
    if changed:
        raise SystemExit(f"sources changed since the binary was built: {changed}")
    protocol = {
        "schema": "vigilode.loop-l1.v28-protocol.v1",
        "node": "V28_MF_P1_BUDGET_FOLLOWUP",
        "written": time.strftime("%Y-%m-%dT%H:%M:%S%z") + " by build_protocol.py, after the plumbing check on "
                   "development instances (logs/plumbing) and before any run of a cell of this node",
        "role": "PREDECLARED.json of this directory is frozen and is not edited. This file fixes every protocol "
                "choice it leaves open and is committed before the first run of a cell. A later correction is an "
                "entry under 'amendments' with its date and its position relative to the runs; git history keeps "
                "the order.",
        "frozen_file": {"path": "research/production_loop_20261116/mf_p1_budget/PREDECLARED.json",
                        "sha256": frozen_sha, "sha256_in_addendum_A9": FROZEN_SHA256_A9},
        "base_commit": git("rev-parse", "e36fc6f4"),
        "notes_commit": git("rev-parse", "be0ec4ce"),
        "target_commit": built_at,
        "cells_rule": "the 40 cells of PREDECLARED.json cells.rule in a fixed order: index 0-19 the V25 cells "
                      "(ac3d, robertson_rd1d, schnakenberg2d, porous2d; final spans of mf_p1_confirm/PROTOCOL.json), "
                      "index 20-39 the V24 cells (fhn3d, cusp1d, burgers2d, bruss1d; spans of the V17 generators, "
                      "mf_adaptive/PROTOCOL.json deciding_grid), within a family rtol 1e-3, 1e-4, 1e-5, 1e-6, "
                      "1e-7 with atol = rtol. The target derives family, span and tolerance from V28_CELL alone "
                      "(Cell::of of mf_budget_measure.rs); size and span below are read from the two protocols and "
                      "the CELLS mode (first logged process, logs/cells) prints what the target builds, which the "
                      "parser compares with this table.",
        "cells": cells(),
        "integrator_configuration": "problems::adaptive(family, rtol) (= stage_clients_krylov_v17_problems.rs::adaptive, "
                                    "used by V24 and V25): atol = rtol * atol_over_rtol (1 for every family of the 40 "
                                    "cells), initial_step 1e-3, min_step 1e-14, max_step t1 - t0, max_attempts "
                                    "1000000, otherwise AdaptiveStepConfig::default(); output schedule [t0, t1]",
        "linear_configuration": "RoutingOptions::default().matrix_free: GMRES, restart 40, budget max(maxiter, restart) "
                                "= 200 Arnoldi vectors, rtol 1e-10, atol 1e-14 (stage threshold max(gamma 1e-14 "
                                "rounded down, 1e-10 ||rhs||_2)), no preconditioner, zero start; unchanged by this node",
        "arms": {
            "S": "MfStageSolvePolicy::Shipped, named explicitly (identifier rodas5p-mf-fast-transformed-v1)",
            "P1": "MfStageSolvePolicy::ColumnTested, named explicitly (identifier before any switch: "
                  "rodas5p-mf-fast-transformed-v2-candidate-p1)",
        },
        "entries": {
            "RUN": "Rodas5pMfFastWorkspace::new(problem, linear), set_gmres_into(true), set_stage_solve_policy(arm), "
                   "integrate_observed (no observer, no solve log): the settings of the policy entry "
                   "integrate_rodas5p_mf_fast_observed_with_stage_solve_policy that change a run (V25 check 4 found "
                   "its audit runs, the same workspace with log, records and observer, bitwise equal to the route "
                   "runs); budget-exhausting solves = stage_solve_policy_statistics().budget_exhausted_solves after "
                   "the run, also after a run that ends with an error",
            "SHADOW": "the RUN workspace with set_solve_log(true) and an attempt observer (Shadow::observe of "
                      "mf_budget_measure.rs) from the unperturbed start; the observer changes nothing in the run "
                      "(contract test the_replay_reproduces_the_driver_on_development_problems; reproduction check "
                      "below)",
        },
        "perturbation": {
            "frozen": "PREDECLARED.json measurements.T_ensemble: y0_i <- y0_i (1 + 1e-8 xi_ij), xi_ij in [-1, 1] from "
                      "splitmix64 seeded by (cell index, j, i), mapped linearly; M = 8, relative size 1e-8",
            "seed": "seed = (cell << 48) | (j << 32) | i with the cell index 0..39 of this table, j = 1..8 and the "
                    "component index i = 0..n-1 (injective for cell, j < 2^16 and i < 2^32; asserted)",
            "draw": "xi = the first output z of SplitMix64 started at state = seed (state += 0x9e3779b97f4a7c15, "
                    "then z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9, z = (z ^ (z >> 27)) * 0x94d049bb133111eb, "
                    "z ^ (z >> 31), wrapping), mapped linearly onto [-1, 1): xi = 2 ((z >> 11) * 2^-53) - 1",
            "start": "y0_i * (1.0 + 1.0e-8 * xi) in f64 in this order of operations; a zero component stays zero; "
                     "j = 0 is the unperturbed start (no draw); S and P1 of a pair start from the bitwise same state "
                     "(the log records start_sha256)",
            "generator_code": [
                excerpt("crates/rodas5p-integrators/tests/mf_confirm_problems.rs", "pub struct SplitMix64", 1),
                excerpt(TARGET, "const PERTURBATION: f64", 2),
                excerpt(TARGET, "fn perturbation_seed(", 1),
                excerpt(TARGET, "fn perturbed_start(", 1),
            ],
        },
        "generator_sources_sha256": {path: sha256_file(os.path.join(ROOT, path)) for path in GENERATOR_FILES},
        "cell_code": excerpt(TARGET, "impl Cell {", 1),
        "binary": {
            "path": os.path.abspath(args.bin),
            "sha256": sha256_file(args.bin),
            "built_at_commit": built_at,
            "build": "CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v28 taskset -c "
                     "0-5,12-17 cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test "
                     "mf_budget_measure --no-run; the test executable copied to the path above",
            "rule": "every run of steps 2 and 3 and of the reproduction check uses this one binary (the runner "
                    "refuses another); a target or driver change after this commit needs a new binary, which is "
                    "used only for D0 after an edit (step 6)",
        },
        "commands": {
            "runner": "python3 -I research/production_loop_20261116/mf_p1_budget/run_measurements.py PHASE --bin "
                      "<binary> [--cells ...] [--arms S,P1] [--js 1-8] [--jobs 4]",
            "phases_in_order": [
                "cells: V28_MODE=CELLS (one process, logs/cells/cells.log)",
                "replay (step 2, K and V): per cell and arm S, P1: V28_MODE=SHADOW V28_CELL=<index> V28_ARM=<arm> "
                "V28_SYSTEMLOG=<log>.systems.txt; the runner gzips the per-system log reproducibly (mtime 0, no "
                "name) and records its raw and gz sha256 in the log footer; logs/replay/c<index>__<family>__<rtol>__<arm>.log",
                "ensemble (step 3, T): per cell, j = 1..8 and arm S, P1: V28_MODE=RUN V28_CELL=<index> V28_ARM=<arm> "
                "V28_J=<j>; logs/ensemble/c<index>__<family>__<rtol>__j<j>__<arm>.log",
                "reproduction (after step 3; reported, decides nothing): V28_MODE=RUN V28_J=0, arms S and P1; "
                "logs/reproduction/c<index>__<family>__<rtol>__j0__<arm>.log",
            ],
            "process": "taskset -c 0-5,12-17 <binary> --ignored --nocapture --exact v28_run with only the V28_* "
                       "variables of the run set (every V28_/V25_/V24_/V17_/V06_ variable of the caller removed); "
                       "log header with binary sha256, git head, dirty count, date, environment, load; footer with "
                       "exit code, time-out flag, seconds",
            "parser": "python3 -I research/production_loop_20261116/mf_p1_budget/parse_results.py (writes RESULTS.json "
                      "and renders REPORT_KO.md; RESULTS.json carries no time stamp and regenerates identically)",
        },
        "discriminators": {
            "K": {
                "frozen": "there is no replayed stage system on which P1 exhausts the Krylov budget and S does not "
                          "(subset), in any cell, along either trajectory",
                "systems": "every stage system of the unperturbed run of S and of the unperturbed run of P1 in every "
                           "cell: the completed solves of every attempt and, for an attempt that failed before all "
                           "eight solves completed, the system of the stage at which it failed (the rule of the V25 "
                           "replay). The right-hand side is rebuilt from the driver's U form with the stage values "
                           "of the attempt; the operator is I - h gamma J(t, y) with J the problem's JVP at (t, y)",
                "kernels": "each system is solved again from the zero start by the shipped kernel "
                           "(solve_gmres_into_with_accounting, ResidualAccounting::DEFAULT, identity preconditioner, "
                           "GmresCapacity::unbounded, GmresIntoOptions::default) and by the P1 kernel "
                           "(StagedGmresWorkspace::solve_column_tested_into, first cycle = restart 40, "
                           "ResidualAccounting::DEFAULT), both with GmresConfig {restart 40, max_arnoldi 200, rtol "
                           "1e-10, atol raw_absolute_residual_budget(1e-14, gamma)}, the configuration the driver "
                           "hands its kernel",
                "classification": "Replay::of (mf_confirm_measure.rs, unchanged): an iterate -> its column count; "
                                  "Err(LinearSolve(text)) with text starting 'GMRES exhausted' -> budget (-1); any "
                                  "other error -> other (-2)",
                "counted": "a K-system is a per-system line 'Y' whose P1 code is -1 and whose shipped code is not -1. "
                           "K PASS iff there is no K-system in the 80 replay runs; FAIL iff there is one. Every system "
                           "on which the two kernels differ in their class (solved, budget, other) is listed in "
                           "RESULTS.json with cell, trajectory, attempt, stage, h and both codes",
                "coverage": "decides NOT_DECIDABLE (not FAIL) when no K-system is found: every replay run present "
                            "with exit code 0 and its V28_SHADOW_JSON; the per-system log present with the raw and gz "
                            "sha256 of the footer; the lines recounted by the parser equal the summary (attempts, "
                            "systems, per-kernel solved, budget, other, p1_budget_shipped_not); rebuild_failures 0; "
                            "the driver's own kernel reproduces the driver on every replayed system "
                            "(replay_differs_from_driver 0: same column count for a completed solve, budget for a "
                            "budget failure, other for any other failure); the driver ran to t1 (completed); "
                            "no write error",
            },
            "V": {
                "frozen": "0 violations of the stage threshold by a completed P1 solve in the replay",
                "counted": "every line 'Y' with a P1 code >= 0: violation iff the residual "
                           "||rhs - (U - h gamma J U)||_2 of the returned iterate, formed by the target with its own "
                           "JVP and buffers (the formula of ProtectedAudit of V17), is not finite or exceeds the stage "
                           "threshold max(gamma 1e-14 rounded down, 1e-10 ||rhs||_2) formed by the target; recounted "
                           "by the parser from the bits in the per-system logs. Also counted as violations: the P1 "
                           "kernel's reported residual not finite or above the same threshold. Reported, not a "
                           "violation: a P1 kernel threshold whose bits differ from the target's threshold",
                "verdict": "PASS iff 0 violations over the 80 replay runs and the coverage of K holds; FAIL iff any "
                           "violation (frozen verdict KILL); NOT_DECIDABLE iff no violation and the coverage fails",
            },
            "T": {
                "frozen": "over the 320 pairs, d = exhausting solves of P1 minus those of S; with W = number of pairs "
                          "with d > 0 and n = number of pairs with d != 0, the one-sided exact binomial probability "
                          "P(X >= W), X ~ Bin(n, 1/2), is at least 0.05 (if n = 0, T holds)",
                "pairs": "cell index 0..39 x j = 1..8; the S run and the P1 run of a pair (ensemble logs)",
                "d": "policy_statistics.budget_exhausted_solves of the P1 run minus that of the S run (the driver's "
                     "count, rodas5p_matrix_free_fast.rs:1747-1756, as in rule B of V24 and V25); a run that ends "
                     "with an error enters with the statistics it reports",
                "probability": "p = sum_{k=W..n} C(n, k) / 2^n in exact rational arithmetic (fractions.Fraction); "
                               "PASS iff p >= 1/20; n = 0: PASS",
                "coverage": "NOT_DECIDABLE iff a run of a pair is missing, was stopped by the time limit, or has no "
                            "policy_statistics, or the two runs of a pair do not start from the same state "
                            "(start_sha256) or a perturbed start equals the unperturbed start",
            },
            "report_without_rule": {
                "totals": "budget-exhausting solves per family and arm: over the ensemble (j = 1..8), over the "
                          "unperturbed driver runs of the replay, and over the reproduction runs (j = 0)",
                "attempts_ratio_per_pair": "attempts(P1) / attempts(S) of the two runs of a pair (attempts of the "
                                           "result = accepted + rejected)",
                "error_ratio_per_pair": "E(P1) / E(S) with E = max_i |y_i(t1) - ref_i| / (1 + |ref_i|) (V17 metric) "
                                        "against the unperturbed reference of the family (reference_for_the_error_report "
                                        "of the cell table); the reference is not corrected for the perturbation of the "
                                        "start (relative 1e-8), which is stated next to the numbers",
                "replay": "per cell and trajectory: systems, per-kernel solved / budget / other, columns of the solved "
                          "systems, largest residual over threshold of each kernel",
            },
        },
        "verdict_computation": {
            "SWITCH": "K, V and T PASS",
            "KILL": "V FAIL",
            "KEEP": "V PASS and K or T FAIL",
            "NOT_DECIDABLE": "otherwise (no FAIL that decides, at least one NOT_DECIDABLE): no default changes, the "
                             "obstacle is reported (status partial)",
        },
        "reproduction_check": {
            "what": "reported, decides nothing: the driver run inside each SHADOW run and the RUN runs with j = 0 must "
                    "reproduce the recorded runs of V25 (record_a audit logs, both arms) and V24 (S: d0 audit logs; P1: "
                    "exploratory audit logs) in state_sha256, attempts, accepted, rejected and, where recorded, "
                    "policy_statistics.budget_exhausted_solves",
            "if_not": "a mismatch is reported as a deviation of the cell from its declared protocol, next to the verdict",
        },
        "runs": {
            "count": {"cells": 1, "replay": 80, "ensemble": 640, "reproduction": 80},
            "order": "cells, replay (step 2; commit), ensemble (step 3; committed as it goes), reproduction, parser "
                     "(step 4)",
            "parallelism": "up to 4 processes at a time inside the work partition taskset -c 0-5,12-17 (counters, "
                           "state bits and statistics of a process do not depend on it); no wall-time measurement "
                           "(wall_seconds_diagnostic_only is a diagnostic)",
            "time_limit": "30 minutes per run; a run that does not finish is kept and counted as missing",
            "exclusions": "none; every declared run is kept, failed and negative ones included",
            "search_budget": "none: S and P1 are fixed, no parameter is tried",
        },
        "step_6_plan_if_SWITCH": {
            "as_frozen": "step 6 of V25 unchanged (mf_p1_confirm/PREDECLARED.json verdicts.SWITCH and discriminators "
                         "D0, G, R; mf_p1_confirm/PROTOCOL.json step_6_plan_if_SWITCH)",
            "driver": "rodas5p_matrix_free_fast.rs: a constant RODAS5P_MF_FAST_P1_DRIVER_ID = "
                      "\"rodas5p-mf-fast-transformed-v3\" next to the others, MfStageSolvePolicy::ColumnTested."
                      "driver_id() -> it, #[default] on ColumnTested, the doc comments that state the default; the "
                      "workspace keeps its explicit initial value Shipped and the shipped entries keep the shipped "
                      "policy (V25 NOTES step 0)",
            "router": "routing.rs: RoutingOptions::default().matrix_free_policy = MfStageSolvePolicy::ColumnTested and "
                      "its doc comments",
            "lib_rs": "one re-export block after the last '// Loop L1' block if the new constant must be exported",
            "out_of_list_lines_of_V25": "the four lines of mf_p1_confirm/OUT_OF_LIST_EDITS.json "
                                        "(mf_adaptive_shipped_bits.rs:182, :250, :260 and mf_adaptive_contracts.rs:411) "
                                        "follow the new default; they are in the write list of this node",
            "facade": "crates/vigilode/src/{solve.rs,solution.rs} only where the matrix-free identifier is named; facade "
                      "test lines that pin the matrix-free identifier or bits, each listed with its reason in "
                      "RESULTS.json",
            "sentinels": "g3 of ppsw1_6b_contracts.rs and h2 of ptsw2b_contracts.rs re-pinned with a comment naming "
                         "OD-L1-23; SENTINEL_REPIN.json with old and new pins",
            "D0": "with the binaries built from the edited sources, on two cells per family (rtol 1e-3 and 1e-7, 16 "
                  "cells): arms S and P1 named explicitly through the measurement targets of the node that declared "
                  "the cell (V25: mf_confirm_measure.rs entries route, router, router_ls, audit; V24: "
                  "mf_adaptive_measure.rs entries route, route_ls and audit, and policy_shipped for S), logs under "
                  "logs/d0 of this node; every result field of the earlier node's comparison (V25 RESULT_KEYS and the "
                  "audit blocks; V24 the same fields) equal to its recorded run; the identifier of P1 is expected to "
                  "change and is reported, not compared",
            "G": "the golden files under crates/vigilode/tests/golden/default_path regenerated by the golden test; "
                 "GOLDEN_DIFF.json by a script: every changed record belongs to the matrix-free route, its end state "
                 "differs from the old record by at most 10 (atol + rtol |y_old|) in every component and its attempts "
                 "by at most 20 percent; every record of another route keeps its bytes (bounds of addendum A6)",
            "R": "the integrator targets mf_* and stage_clients_*, both sentinels, the whole rodas5p-krylov suite, the "
                 "facade golden test and the whole facade suite (cargo +1.94.1 test --locked -j 4 --release -p "
                 "vigilode --no-fail-fast); clippy -D warnings on the library of rodas5p-integrators, the mf_budget "
                 "target and vigilode (all targets); rustfmt --check",
            "then": "if G or R does not hold, the switch is taken back by a new commit (git revert) and the verdict is "
                    "HOLD with the reason",
        },
        "known_obstacles_found_before_any_run": [
            "None that blocks the frozen discriminators. The obstacle of V25 (four lines of V24 tests outside its write "
            "list) is inside the write list of this node.",
            "The reproduction check compares with runs of V24 (log headers: git heads 7fb23b63 for d0, 10ea5b31 for "
            "the exploratory P1 runs) and V25 (binary built at 956b578d); since 10ea5b31 the only commits that touch "
            "rodas5p-core, rodas5p-krylov, the driver, the generators or Cargo.lock are 956b578d (the V25 generators, "
            "new file) and 1d421b17 (a doc comment of the driver), so equal bits are expected; a difference is "
            "reported, not explained away.",
        ],
        "amendments": [],
    }
    path = os.path.join(NODE, "PROTOCOL.json")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(protocol, indent=1, ensure_ascii=False, allow_nan=False) + "\n")
    print(f"wrote {os.path.relpath(path, ROOT)}: {len(protocol['cells'])} cells, binary {protocol['binary']['sha256']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
