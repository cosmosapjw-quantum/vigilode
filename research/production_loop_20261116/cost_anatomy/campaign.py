#!/usr/bin/env python3
"""Campaign driver of loop L1 node V04_STAGE_COST_ANATOMY.

Subcommands (run from anywhere; paths are relative to this file):

  predeclare   write PREDECLARED.json (protocol text of this file + the cell table printed by
               `runner list`; no solver is executed)
  environment  write logs/environment.json (toolchain, binary digest, host facts)
  pilot        one protected-driver run per cell with the pilot attempt budget, pinned to the
               timing core -> logs/pilot/<cell>.jsonl
  size         apply the predeclared sizing rule to the pilot logs -> SIZING.json
  identity     the three arms once per cell, compared bit for bit -> logs/identity/<cell>.jsonl
  measure      the wall protocol per cell, pinned to the timing core -> logs/measure/<cell>.jsonl
  callgrind    instruction counts per phase for the cells listed in PREDECLARED.json
               -> logs/callgrind/
  perfstat     exploratory `perf stat -e instructions:u` of single runs -> logs/perfstat/

This script executes programs and stores their raw output. It computes no result except the
attempt budgets of `size`, whose rule is fixed in PREDECLARED.json before the pilot runs.
"""
import argparse
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TARGET = Path(os.environ.get(
    "CARGO_TARGET_DIR", "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v04"))
BIN = TARGET / "release" / "v04-cost-anatomy-runner"
TIMING_CPUS = "10"
OTHER_CPUS = "0-9,12-21"

PILOT_ATTEMPTS = 12
TARGET_RUN_SECONDS = 1.5
MIN_ATTEMPTS_FULL = 20
MAX_ATTEMPTS = 1000
FULL_PROTOCOL_MAX_RUN_SECONDS = 60.0
REDUCED_ATTEMPTS = 8
REDUCED_MAX_RUN_SECONDS = 150.0
IDENTITY_ATTEMPTS = 12
CALLGRIND_EXPLORATORY_MAX_ATTEMPTS = 20
PERFSTAT_MAX_ATTEMPTS = 20

CALLGRIND_EXPLORATORY_CELLS = [
    "dense.bruss1d.n128.analytic.x1",
    "dense.radnet.n128.analytic.x1",
    "banded_1d.bruss1d.n1000.analytic.x1",
    "banded_2d.ac2d.g32.analytic.x1",
    "matrix_free_2d.ac2d.g64.jvp.x1",
]


def strict_dump(obj, path):
    text = json.dumps(obj, indent=1, ensure_ascii=False, allow_nan=False) + "\n"
    Path(path).write_text(text)


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


def loadavg():
    return [float(v) for v in Path("/proc/loadavg").read_text().split()[:3]]


def run_capture(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def runner_cells():
    out = run_capture(["taskset", "-c", OTHER_CPUS, str(BIN), "list"])
    if out.returncode != 0:
        raise SystemExit("runner list failed: " + out.stderr)
    return json.loads(out.stdout)["cells"]


def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


# ------------------------------------------------------------------------------------------------
# predeclare
# ------------------------------------------------------------------------------------------------

def predeclared_document(cells):
    return {
        "schema": "vigilode.loop_l1.v04_cost_anatomy.predeclared.v1",
        "node": "V04_STAGE_COST_ANATOMY",
        "date": "2026-11-17",
        "builder_base_commit": "0443025895dabf350f46fd3f0cdd0cf1d7fbcf85",
        "status": "written and committed before the pilot and before every measurement run",
        "question": "Mean cost per attempt of the phases of a RODAS5P step (Jacobian build, factorisation of W, eight stage solves, right-hand-side evaluations, remainder) across regimes, and the upper bound that these fractions put on every lever.",
        "frozen_discriminator": {
            "source": "research/production_loop_20261116/PREREGISTRATION.md, row V04",
            "pass": "instrumented/plain wall <= 1.02 and phases sum to 0.90-1.00 of the total at n >= 128; trajectory bit-identical to the protected driver",
            "fallback": "otherwise diagnostic_only",
            "thresholds_are_not_changed_here": True,
        },
        "what_ran_before_this_file": [
            "runner selftest (problem definitions and wrappers; no integration)",
            "runner list (cell table; no integration)",
            "development smoke test with the non-LTO smoke build: `identity --attempts 12` on eleven cells; only the identity flags, the step counts and the work counters were read, no wall time",
            "tooling check: one callgrind run of dense.hires.n8.fd.x1 (12 attempts) to verify that callgrind_annotate lists the marker symbols; the instruction numbers were masked when it was read",
            "what the smoke test showed and how it changed the grid: (1) with the finite-difference JVP 6 of 12 attempts were rejected (the product's noise is above the protected linear tolerance 1e-10), so that source is kept as one exploratory counts-only cell and is never timed; (2) the protected matrix-free driver spent exactly 40 Krylov iterations in every stage solve (its GMRES solves the least-squares problem once per restart cycle), so a 48^3 cell was dropped to bound the run time",
        ],
        "binary": {
            "package": "research/production_loop_20261116/cost_anatomy/runner (standalone, empty [workspace] table, path dependencies on crates/rodas5p-core and crates/rodas5p-integrators)",
            "profile": "release with the repository workspace's settings: opt-level 3, lto fat, codegen-units 1, panic abort",
            "toolchain": "1.94.1",
            "lock": "runner/Cargo.lock: the repository Cargo.lock pruned by one offline build; every later build uses --locked",
        },
        "arms": {
            "plain": {
                "dense": "integrate_rodas5p_fast_observed_with_atol (protected dense driver, default options; it picks its LU from the first Jacobian)",
                "banded": "integrate_rodas5p_fast_banded_observed_with_atol (protected banded driver, Indexed kernel)",
                "matrix_free": "integrate_rodas5p_mf_fast_observed_gmres_into with RoutingOptions::default().matrix_free (GMRES, restart 40, rtol 1e-10, atol 1e-14, no preconditioner, zero start, 200 Arnoldi vectors): the route the protected router takes",
            },
            "instrumented": {
                "dense": "integrate_rodas5p_fast_client_observed_with_atol with TimedStageClient(DenseLuStageClient) in the LU mode DenseLuMode::protected_choice_at(t0, y0) (evaluated once, outside every timed interval), right-hand side wrapped by a wall clock",
                "banded": "integrate_rodas5p_fast_client_observed_with_atol with TimedStageClient(BandedStageLu), right-hand side wrapped by a wall clock",
                "matrix_free": "integrate_rodas5p_mf_fast_observed_traced (gmres_into = true) with wall clocks around the right-hand side and the JVP callback and with the attempt observer; the observer cannot change the run",
            },
            "marked": "the client driver with a clock-free forwarding client and forwarding callbacks that pass through one unmangled never-inlined symbol per phase (v04_phase_linearize, v04_phase_factor, v04_phase_solve, v04_phase_rhs, v04_phase_jvp); used under callgrind only",
            "timed_interval": "the whole driver call; for the instrumented direct arms it also contains the construction and the release of the stage client, as the protected drivers build and drop their workspace inside the call",
        },
        "problems": {
            "definitions": "runner/src/problems.rs; every problem is autonomous with the identity mass matrix",
            "hires": "HIRES, n = 8, physically derived (plant-physiology kinetics). It is one of the consumed U11 configurations; it is used here as a development problem and is not called holdout",
            "bruss1d": "1D Brusselator, method of lines, species interleaved, Dirichlet data, diffusion (cells+1)^2/50, band lower = upper = 2. In dense storage (n = 32, 128, 512, 2048) its Jacobian has at most 10 % nonzeros, so the protected policy keeps the in-place zero-skipping LU at every size; in band storage n = 1e3, 1e4, 1e5",
            "radnet": "radiative-conductive lumped thermal network on a ring: all-to-all T^4 exchange (every Jacobian entry nonzero), nearest-neighbour conduction, capacities over four decades; the model is a radiosity network, its parameters come from formulas. n = 8, 32 (in-place LU), 128, 512, 2048 (faer LU by the protected policy)",
            "ac2d_ac3d": "Allen-Cahn u_t = D lap u + (u - u^3), cell-centred finite volumes, zero-flux boundaries, D = 0.01 in 2D and 0.04 in 3D; band lower = upper = m (2D) or m^2 (3D)",
            "bruss2d": "2D Brusselator, A = 1, B = 3.4, alpha = 0.02, zero-flux boundaries, species interleaved per cell; band lower = upper = 2 m",
            "expected_lu_mode": "dense cells with n <= 64: in-place; bruss1d in dense storage: in-place at every n; radnet with n >= 128: faer. The mode actually used is recorded in every log header and the protected driver's own choice in every plain run row",
            "both_lu_modes": "the protected policy offers one LU per problem; both modes appear at n in {128, 512, 2048} through the two problem families. A mode the protected driver would not pick has no protected counterpart and is not measured",
        },
        "regime_grid": {
            "cells": cells,
            "rhs_multiplier": "the same right-hand side is evaluated k times and the last result kept (runner/src/instrument.rs, `multiplied`); the multiplied function is the problem's right-hand side for both arms and for the finite-difference Jacobians",
            "jacobian_source": {
                "analytic": "dense: in-place fill of the fixed pattern; banded: band fill",
                "fd": "forward differences of the (multiplied) right-hand side inside the Jacobian callback: dense n + 1 evaluations, banded lower + upper + 2 evaluations (Curtis-Powell-Reid grouping of the band); their time is inside the Jacobian phase and their number is reported as fd_rhs_calls",
                "jvp": "analytic JVP callback (matrix-free cells)",
                "jvpfd": "exploratory, counts only: forward-difference JVP",
            },
            "excluded_a_priori": [
                {
                    "regime": "banded direct, 3D, 32^3 cells (n = 32768, lower = upper = 1024)",
                    "reason": "band storage n (2 lower + upper + 1) = 100.7e6 doubles (805 MB) for the factors plus 537 MB for the Jacobian band, and about n lower (lower + upper) = 6.9e10 multiply-subtracts per factorisation; one 8-attempt run would exceed the 150 s limit of the sizing rule by more than an order of magnitude. The 32^3 problem is measured matrix-free",
                },
                {
                    "regime": "banded direct, 2D, more than 128 x 128 cells",
                    "reason": "the contract asks for 2D up to at least 128 x 128; larger grids are measured matrix-free only (256 x 256)",
                },
            ],
        },
        "run_definition": {
            "span": "the problem's (t0, tf) of the cell table; output schedule {t0, tf}",
            "adaptive": "AdaptiveStepConfig { rtol, atol, initial_step of the cell table, min_step 1e-14, max_step tf - t0, max_attempts = attempt budget, every other field default }; the dense and banded entries get the absolute tolerance as a vector of n equal entries",
            "shortened_horizon": "a run ends at tf or when the attempt budget is used up (DriverStopReason::AttemptBudget), whichever comes first; the budget is the same for both arms, and the end time and end state are recorded",
        },
        "sizing_rule": {
            "pilot": f"one run of the plain arm per cell with an attempt budget of {PILOT_ATTEMPTS}, pinned to the timing core; c = wall / attempts. The pilot times no phase",
            "full_protocol": f"if {MIN_ATTEMPTS_FULL} c <= {FULL_PROTOCOL_MAX_RUN_SECONDS} s: attempt budget A = min({MAX_ATTEMPTS}, max({MIN_ATTEMPTS_FULL}, floor({TARGET_RUN_SECONDS} s / c))), 2 warm-ups + 7 repetitions",
            "reduced_protocol": f"else if {REDUCED_ATTEMPTS} c <= {REDUCED_MAX_RUN_SECONDS} s: A = {REDUCED_ATTEMPTS}, 1 warm-up + 3 repetitions, the row is marked reduced_protocol",
            "not_measured": f"else: the cell is not measured and is reported with its pilot numbers",
            "exploratory_cells": "cells flagged exploratory are run in pilot mode only (counts, no phase timing)",
            "output": "SIZING.json, written by `campaign.py size` from the pilot logs and committed before the first measurement run",
        },
        "wall_protocol": {
            "pinning": f"taskset -c {TIMING_CPUS} for the whole process (core 10; its SMT sibling 22 and core 11/23 are reserved for this node); builds and all non-timing runs under taskset -c {OTHER_CPUS}",
            "interleaving": "each repetition is one plain run and one instrumented run in the same process; plain first on even repetitions, instrumented first on odd ones",
            "repetitions": "2 warm-ups + 7 timed repetitions (reduced protocol: 1 + 3)",
            "load": "/proc/loadavg is recorded by the runner at the start and the end of each cell and by the campaign script before and after the process",
            "order_of_cells": "the order of the cell table; a cell that is not reached before the session ends is reported as NOT_RUN",
            "retries": "a cell whose process fails is rerun at most once; both logs are kept",
        },
        "phases": {
            "direct_drivers": {
                "jacobian": "TimedStageClient linearize_ns (finite-difference evaluations of the right-hand side included)",
                "factor": "TimedStageClient factor_ns (assembly of W and LU)",
                "solve": "TimedStageClient solve_ns (eight triangular solves per attempt)",
                "rhs": "wall clock around the driver's right-hand-side calls (f(t, y) of a new state and seven stage evaluations)",
                "remainder": "total - (jacobian + factor + solve + rhs): stage vector work, error norm, controller, output, clock overhead",
            },
            "matrix_free": {
                "rhs": "wall clock around the driver's right-hand-side calls",
                "jvp": "wall clock around the JVP callback, booked per stage",
                "solve_intervals": "per stage, the interval from the end of the right-hand-side call that precedes the stage solve (or, in an attempt that reuses f(t, y), from its first JVP) to the start of the next right-hand-side call or to the attempt observer; it contains the JVP time, the Krylov orthogonalisation, the least-squares solve and the stage vector work",
                "other": "everything else on the timeline (controller, output, attempt set-up)",
                "remainder_as_in_the_task_text": "total - (rhs + jvp): Krylov orthogonalisation + vector work + controller",
            },
        },
        "statistics": {
            "repetitions_used": "timed repetitions only",
            "per_attempt": "phase_ns / attempts of the same run; median and minimum over the repetitions. Per accepted step: phase_ns / accepted steps",
            "fractions": "phase_ns / instrumented wall of the same run; the reported fraction is the median over the repetitions",
            "sum_check": "per repetition (jacobian + factor + solve + rhs) / instrumented wall for the direct drivers and (rhs + jvp) / instrumented wall for the matrix-free driver; the cell's value is the median over the repetitions",
            "sum_check_with_solve_intervals": "matrix-free only, reported next to the sum check and not used for the label: (rhs + all solve intervals) / instrumented wall",
            "instrumented_over_plain": "median of the instrumented walls / median of the plain walls; the ratio of the minima and the median of the paired ratios are reported next to it",
            "identity": "every pair of every repetition (warm-ups included) must agree in attempts, accepted and rejected steps, reuses, success, stop reason, the bits of the end time, every bit of the end state, every WorkCounters field and the finite-difference call count",
        },
        "admissibility": {
            "n_at_least_128": "authoritative iff the full protocol was used (one pinned core, interleaved arms, 2 warm-ups + 7 repetitions, load average recorded) and (i) instrumented_over_plain <= 1.02, (ii) 0.90 <= sum_check <= 1.00, (iii) identity holds; otherwise diagnostic_only with the failed criteria listed",
            "reduced_protocol": "diagnostic_only (the wall rule needs 2 + 7); the three criteria are still evaluated and reported",
            "n_below_128": "wall numbers are diagnostic_only; the primary numbers are callgrind instruction counts, labelled instruction_counts",
            "matrix_free_note": "the label uses the sum check of the task text (rhs + jvp callbacks). The matrix-free driver has no hook around its Krylov orthogonalisation, so a row whose orthogonalisation exceeds 10 % of the wall is diagnostic_only by criterion (ii); the solve-interval accounting is reported as an extension",
        },
        "instruction_counts": {
            "tool": "valgrind 3.22 callgrind, `runner single --arm marked` and `--arm plain`, attempt budget of the cell's measurement; callgrind_annotate --inclusive=yes --threshold=100",
            "numbers": "inclusive Ir of v04_region (total of the run) and of the phase symbols; fractions = phase Ir / v04_region Ir of the marked arm; the plain arm gives the ratio marked / plain of v04_region",
            "cells": "every measured cell with n < 128",
            "exploratory_cells": CALLGRIND_EXPLORATORY_CELLS,
            "exploratory_budget": f"min(A, {CALLGRIND_EXPLORATORY_MAX_ATTEMPTS}) attempts",
            "perf_stat": f"exploratory and optional: perf stat -e instructions:u of `runner single` for the arms plain, instrumented and none (set-up only) with min(A, {PERFSTAT_MAX_ATTEMPTS}) attempts; ratio (instrumented - none) / (plain - none)",
        },
        "amdahl": {
            "inputs": "f_J, f_LU, f_S, f_R: the cell's median fractions of the instrumented wall (instruction fractions for n < 128); f_chain = f_R + f_S (eight right-hand sides and eight solves); for matrix-free cells f_S is the sum of the solve-interval fractions and f_J = f_LU = 0",
            "assumptions": "upper bounds: zero overhead, perfect scaling, unchanged step count and unchanged error",
            "a_level_structured_tableau": "S = 1 / (1 - (1 - L/8) f_chain), L in {4, 5, 6}; the present tableau has depth 8 (theory S1), bound 1.0; theory S9 excludes L = 4 for a stiffly accurate order-5 tableau",
            "b_speculative_jacobian_lu": "saving = min(f_J + f_LU, f_chain / 4), S = 1 / (1 - saving); r = (f_J + f_LU) / f_chain. Also the variant without step-size prediction (theory S6): saving = min(f_J, f_chain / 4)",
            "b_gate_list_for_V07_stage_b": "cells with 0.15 <= r <= 0.92 and plain wall per accepted step (median plain wall / accepted steps) >= 500 microseconds; each entry carries its admissibility label",
            "c_parallel_jacobian_assembly": "S = 1 / (1 - f_J (1 - 1/P)), P in {2, 4, 6}",
            "d_lagged_lu": "S = 1 / (1 - f_LU (1 - 1/k)), k in {2, 4}",
            "e_parallel_dense_lu": "S = 1 / (1 - f_LU (1 - 1/P)), P in {2, 4, 6}; dense cells only",
            "f_shared_krylov_basis": "matrix-free cells only: S = 1 / (1 - sum over stages 2..8 of the solve-interval fractions); the JVP part of that sum is reported separately",
            "g_parallel_rhs": "S = 1 / (1 - f_R (1 - 1/P)), P in {2, 4, 6}; exploratory variant with the right-hand sides inside a finite-difference Jacobian (f_R + f_J) and, matrix-free, with the JVP (f_R + f_JVP)",
        },
        "reference_solution": "not applicable: both arms are the same method on the same problem and are required to be bit-identical; no accuracy comparison between methods is made",
        "evidence_rules": [
            "numbers in SIZING.json, RESULTS.json and AMDAHL.json are parsed by scripts from the raw logs of executed runs",
            "failed and negative runs are kept",
            "strict JSON without NaN or Infinity",
        ],
        "constants": {
            "pilot_attempts": PILOT_ATTEMPTS,
            "target_run_seconds": TARGET_RUN_SECONDS,
            "min_attempts_full": MIN_ATTEMPTS_FULL,
            "max_attempts": MAX_ATTEMPTS,
            "full_protocol_max_run_seconds": FULL_PROTOCOL_MAX_RUN_SECONDS,
            "reduced_attempts": REDUCED_ATTEMPTS,
            "reduced_max_run_seconds": REDUCED_MAX_RUN_SECONDS,
            "identity_sweep_attempts": IDENTITY_ATTEMPTS,
        },
    }


def cmd_predeclare(_args):
    target = HERE / "PREDECLARED.json"
    if target.exists():
        raise SystemExit("PREDECLARED.json exists; it is not rewritten")
    strict_dump(predeclared_document(runner_cells()), target)
    print("wrote", target)


# ------------------------------------------------------------------------------------------------
# environment
# ------------------------------------------------------------------------------------------------

def cmd_environment(_args):
    def read(path):
        try:
            return Path(path).read_text().strip()
        except OSError:
            return None
    model = None
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            model = line.split(":", 1)[1].strip()
            break
    env = {
        "schema": "vigilode.loop_l1.v04_cost_anatomy.environment.v1",
        "git_head": git("rev-parse", "HEAD"),
        "git_branch": git("rev-parse", "--abbrev-ref", "HEAD"),
        "rustc": run_capture(["rustc", "+1.94.1", "--version"]).stdout.strip(),
        "cargo": run_capture(["cargo", "+1.94.1", "--version"]).stdout.strip(),
        "valgrind": run_capture(["valgrind", "--version"]).stdout.strip(),
        "perf": run_capture(["perf", "--version"]).stdout.strip(),
        "binary": str(BIN),
        "binary_sha256": sha256(BIN),
        "runner_sources_sha256": {
            str(p.relative_to(HERE)): sha256(p)
            for p in sorted((HERE / "runner").rglob("*")) if p.is_file()
        },
        "uname": run_capture(["uname", "-srm"]).stdout.strip(),
        "cpu_model": model,
        "logical_cpus": os.cpu_count(),
        "timing_cpu": TIMING_CPUS,
        "timing_cpu_thread_siblings": read("/sys/devices/system/cpu/cpu10/topology/thread_siblings_list"),
        "timing_cpu_governor": read("/sys/devices/system/cpu/cpu10/cpufreq/scaling_governor"),
        "cpufreq_boost": read("/sys/devices/system/cpu/cpufreq/boost"),
        "transparent_hugepage": read("/sys/kernel/mm/transparent_hugepage/enabled"),
        "loadavg": loadavg(),
        "unix_time": time.time(),
    }
    (HERE / "logs").mkdir(exist_ok=True)
    strict_dump(env, HERE / "logs" / "environment.json")
    print("wrote logs/environment.json")


# ------------------------------------------------------------------------------------------------
# running one runner process into a log
# ------------------------------------------------------------------------------------------------

def run_logged(cmd, log_path, extra_meta=None, prefix_cmd=None):
    """Run `cmd`, stream stdout to `log_path`, write `<log>.meta.json`; return the exit code."""
    log_path = Path(log_path)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    full = (prefix_cmd or []) + cmd
    before = loadavg()
    started = time.time()
    stderr_path = Path(str(log_path) + ".stderr.txt")
    with open(log_path, "w") as out, open(stderr_path, "w") as err:
        code = subprocess.run(full, stdout=out, stderr=err).returncode
    meta = {
        "command": full,
        "exit_code": code,
        "loadavg_before": before,
        "loadavg_after": loadavg(),
        "started_unix": started,
        "wall_seconds": time.time() - started,
        "log": str(log_path.relative_to(HERE)),
        "log_sha256": sha256(log_path),
        "binary_sha256": sha256(BIN),
    }
    if extra_meta:
        meta.update(extra_meta)
    strict_dump(meta, str(log_path) + ".meta.json")
    if stderr_path.stat().st_size == 0:
        stderr_path.unlink()
    return code


def selected(cells, only):
    if not only:
        return cells
    wanted = set(only)
    unknown = wanted - {c["id"] for c in cells}
    if unknown:
        raise SystemExit("unknown cells: " + ", ".join(sorted(unknown)))
    return [c for c in cells if c["id"] in wanted]


def read_jsonl(path):
    rows = []
    for line in Path(path).read_text().splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


# ------------------------------------------------------------------------------------------------
# pilot, size
# ------------------------------------------------------------------------------------------------

def cmd_pilot(args):
    cells = strict_load(HERE / "PREDECLARED.json")["regime_grid"]["cells"]
    for cell in selected(cells, args.cell):
        log = HERE / "logs" / "pilot" / (cell["id"] + ".jsonl")
        if log.exists() and not args.force:
            print("skip (exists)", cell["id"])
            continue
        code = run_logged(
            [str(BIN), "pilot", "--cell", cell["id"], "--attempts", str(PILOT_ATTEMPTS)],
            log, prefix_cmd=["taskset", "-c", TIMING_CPUS])
        print("pilot", cell["id"], "exit", code, flush=True)


def cmd_size(_args):
    pre = strict_load(HERE / "PREDECLARED.json")
    rows = []
    for cell in pre["regime_grid"]["cells"]:
        log = HERE / "logs" / "pilot" / (cell["id"] + ".jsonl")
        entry = {"cell": cell["id"], "n": cell["n"], "exploratory": cell["exploratory"]}
        runs = [r for r in read_jsonl(log) if r.get("type") == "pilot_run"] if log.exists() else []
        if not runs:
            entry.update({"protocol": "not_measured", "reason": "no pilot run"})
            rows.append(entry)
            continue
        run = runs[0]
        seconds_per_attempt = run["wall_ns"] * 1e-9 / run["attempts"]
        entry.update({
            "pilot_wall_ns": run["wall_ns"],
            "pilot_attempts": run["attempts"],
            "pilot_accepted": run["accepted"],
            "pilot_rejected": run["rejected"],
            "pilot_seconds_per_attempt": seconds_per_attempt,
        })
        if cell["exploratory"]:
            entry.update({"protocol": "counts_only", "reason": "exploratory cell"})
        elif MIN_ATTEMPTS_FULL * seconds_per_attempt <= FULL_PROTOCOL_MAX_RUN_SECONDS:
            budget = int(TARGET_RUN_SECONDS / seconds_per_attempt)
            budget = min(MAX_ATTEMPTS, max(MIN_ATTEMPTS_FULL, budget))
            entry.update({"protocol": "full", "attempt_budget": budget, "warmups": 2,
                          "repetitions": 7})
        elif REDUCED_ATTEMPTS * seconds_per_attempt <= REDUCED_MAX_RUN_SECONDS:
            entry.update({"protocol": "reduced", "attempt_budget": REDUCED_ATTEMPTS, "warmups": 1,
                          "repetitions": 3})
        else:
            entry.update({"protocol": "not_measured",
                          "reason": "a single 8-attempt run would exceed the limit"})
        if "attempt_budget" in entry:
            runs_total = 2 * (entry["warmups"] + entry["repetitions"])
            entry["estimated_cell_seconds"] = (
                runs_total * entry["attempt_budget"] * seconds_per_attempt)
        rows.append(entry)
    doc = {
        "schema": "vigilode.loop_l1.v04_cost_anatomy.sizing.v1",
        "generated_by": "campaign.py size, from logs/pilot/*.jsonl with the rule of PREDECLARED.json",
        "rule_constants": pre["constants"],
        "estimated_total_seconds": sum(r.get("estimated_cell_seconds", 0.0) for r in rows),
        "cells": rows,
    }
    strict_dump(doc, HERE / "SIZING.json")
    print("wrote SIZING.json; estimated total seconds", round(doc["estimated_total_seconds"]))


# ------------------------------------------------------------------------------------------------
# identity sweep, measurement
# ------------------------------------------------------------------------------------------------

def cmd_identity(args):
    cells = strict_load(HERE / "PREDECLARED.json")["regime_grid"]["cells"]
    for cell in selected(cells, args.cell):
        log = HERE / "logs" / "identity" / (cell["id"] + ".jsonl")
        if log.exists() and not args.force:
            print("skip (exists)", cell["id"])
            continue
        code = run_logged(
            [str(BIN), "identity", "--cell", cell["id"], "--attempts", str(IDENTITY_ATTEMPTS)],
            log, prefix_cmd=["taskset", "-c", OTHER_CPUS])
        print("identity", cell["id"], "exit", code, flush=True)


def cmd_measure(args):
    sizing = strict_load(HERE / "SIZING.json")
    wanted = set(args.cell or [])
    for entry in sizing["cells"]:
        if wanted and entry["cell"] not in wanted:
            continue
        if entry["protocol"] not in ("full", "reduced"):
            continue
        log = HERE / "logs" / "measure" / (entry["cell"] + ".jsonl")
        attempt = 1
        if log.exists():
            meta = strict_load(str(log) + ".meta.json")
            if meta["exit_code"] == 0 and not args.force:
                print("skip (done)", entry["cell"])
                continue
            # Keep the failed log; the single allowed rerun goes to <cell>.retry.jsonl.
            retry = HERE / "logs" / "measure" / (entry["cell"] + ".retry.jsonl")
            if retry.exists() and not args.force:
                print("skip (retry exists)", entry["cell"])
                continue
            log = retry
            attempt = 2
        code = run_logged(
            [str(BIN), "measure", "--cell", entry["cell"],
             "--attempts", str(entry["attempt_budget"]),
             "--warmups", str(entry["warmups"]), "--reps", str(entry["repetitions"])],
            log, extra_meta={"protocol": entry["protocol"], "run_attempt": attempt},
            prefix_cmd=["taskset", "-c", TIMING_CPUS])
        print("measure", entry["cell"], "exit", code, "load", loadavg(), flush=True)


# ------------------------------------------------------------------------------------------------
# callgrind, perf stat
# ------------------------------------------------------------------------------------------------

def callgrind_plan():
    pre = strict_load(HERE / "PREDECLARED.json")
    sizing = {c["cell"]: c for c in strict_load(HERE / "SIZING.json")["cells"]}
    plan = []
    for cell in pre["regime_grid"]["cells"]:
        size = sizing.get(cell["id"], {})
        if size.get("protocol") not in ("full", "reduced"):
            continue
        if cell["n"] < 128:
            plan.append((cell["id"], size["attempt_budget"], "required_n_below_128"))
        elif cell["id"] in CALLGRIND_EXPLORATORY_CELLS:
            plan.append((cell["id"],
                         min(size["attempt_budget"], CALLGRIND_EXPLORATORY_MAX_ATTEMPTS),
                         "exploratory"))
    return plan


def cmd_callgrind(args):
    out_dir = HERE / "logs" / "callgrind"
    out_dir.mkdir(parents=True, exist_ok=True)
    wanted = set(args.cell or [])
    for cell_id, budget, role in callgrind_plan():
        if wanted and cell_id not in wanted:
            continue
        for arm in ("marked", "plain"):
            stem = out_dir / f"{cell_id}.{arm}"
            annotate_path = Path(str(stem) + ".annotate.txt")
            if annotate_path.exists() and not args.force:
                print("skip (exists)", cell_id, arm)
                continue
            raw = Path(str(stem) + ".callgrind.out")
            cmd = ["valgrind", "--tool=callgrind", f"--callgrind-out-file={raw}",
                   str(BIN), "single", "--cell", cell_id, "--arm", arm,
                   "--attempts", str(budget)]
            code = run_logged(cmd, Path(str(stem) + ".stdout.jsonl"),
                              extra_meta={"role": role, "attempt_budget": budget, "arm": arm},
                              prefix_cmd=["taskset", "-c", OTHER_CPUS])
            annotate = run_capture(
                ["callgrind_annotate", "--inclusive=yes", "--threshold=100", str(raw)])
            annotate_path.write_text(annotate.stdout)
            with open(raw, "rb") as src, gzip.open(str(raw) + ".gz", "wb", mtime=0) as dst:
                shutil.copyfileobj(src, dst)
            raw.unlink()
            print("callgrind", cell_id, arm, "exit", code, "annotate", annotate.returncode,
                  flush=True)


def cmd_perfstat(args):
    sizing = strict_load(HERE / "SIZING.json")
    out_dir = HERE / "logs" / "perfstat"
    out_dir.mkdir(parents=True, exist_ok=True)
    wanted = set(args.cell or [])
    for entry in sizing["cells"]:
        if entry["protocol"] not in ("full", "reduced"):
            continue
        if wanted and entry["cell"] not in wanted:
            continue
        budget = min(entry["attempt_budget"], PERFSTAT_MAX_ATTEMPTS)
        for arm in ("none", "plain", "instrumented"):
            stat = out_dir / f"{entry['cell']}.{arm}.perf.csv"
            if stat.exists() and not args.force:
                continue
            cmd = ["perf", "stat", "-x,", "-e", "instructions:u", "-o", str(stat),
                   str(BIN), "single", "--cell", entry["cell"], "--arm", arm,
                   "--attempts", str(budget)]
            code = run_logged(cmd, out_dir / f"{entry['cell']}.{arm}.stdout.jsonl",
                              extra_meta={"attempt_budget": budget, "arm": arm},
                              prefix_cmd=["taskset", "-c", OTHER_CPUS])
            print("perfstat", entry["cell"], arm, "exit", code, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name, fn in [("predeclare", cmd_predeclare), ("environment", cmd_environment),
                     ("pilot", cmd_pilot), ("size", cmd_size), ("identity", cmd_identity),
                     ("measure", cmd_measure), ("callgrind", cmd_callgrind),
                     ("perfstat", cmd_perfstat)]:
        p = sub.add_parser(name)
        p.add_argument("--cell", action="append", help="restrict to this cell (repeatable)")
        p.add_argument("--force", action="store_true", help="rerun even if a log exists")
        p.set_defaults(fn=fn)
    args = parser.parse_args()
    if not BIN.exists():
        raise SystemExit(f"runner binary not found: {BIN}")
    args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
