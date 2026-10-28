#!/usr/bin/env python3
"""Gate runner of BASS/rei DAG node P05 (accepted-step conservative remap of
the LRS photon transport adapter, with the L3 accept-control hook, client
rebind and restart, through the RODAS5P fast banded and client drivers).

    python3 run_p05.py --out results.json [--quick] [--skip-cargo]

Builds the runner (`cargo build --profile measurement -p rodas5p-integrators
--example lrs_photon_remap_run --locked --offline -j 8`), drives it, the
independent Python references (`reference/p05_reference.py`, on top of the
read-only P02 reference and formula oracle) and the Rust contract tests, and
writes one JSON with every number, per-gate `{pass, values, threshold}` and
`verdict = PASS` iff every [C] gate R1-R7 passes and the tree is clean.
Thresholds are the frozen constants in TH. Wall times are `diagnostic_only`.

INVALID: a change under the watched paths (`crates/`, this node directory
other than the harness's own outputs, the P02 reference and harnesses, the
formula oracle, Cargo.toml, Cargo.lock, rust-toolchain.toml, .cargo/) at
start or end, a recorded input changed during the run, a stale binary, or
any runner request that timed out, failed in its environment or exited
nonzero (an incomplete run: no ledger row). `raw/` must be absent or empty
at start.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
import math  # noqa: E402
import multiprocessing  # noqa: E402
import os  # noqa: E402
import platform  # noqa: E402
import re  # noqa: E402
import subprocess  # noqa: E402
import tempfile  # noqa: E402

# One BLAS thread here and in the spawned reference workers.
for _var in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_var] = "1"

import threading  # noqa: E402
import time  # noqa: E402
import traceback  # noqa: E402
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor  # noqa: E402
from pathlib import Path  # noqa: E402

import numpy as np  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "reference"))
import p05_reference as F  # noqa: E402

R = F.R
REPO = HERE.parents[1]
EPS = float(np.finfo(float).eps)

# Approved-plan constants of P05 (R7: TH must carry exactly these).
PLAN = {"R2_per_remap_rel": 2e-15, "R2_drift_rel": 1e-11, "R3_one_cycle_ulps_max": 2, "R3_cycles": 100,
        "R3_cycles_rel": 1e-13, "R6_order_window": [0.8, 1.25], "R6_min_last_slope": 4.8}

SMOOTH = [1.15 ** k - 1.0 for k in range(1, 10)]
COARSE = [0.6, 1.2, 1.8, 2.4]

TH = {
    "R1": {"cases": {"I1": "the integrated state at t = 1.3 (static run S0) on G2",
                     "I2": "a signed version of the integrated state at t = 2.1 (every 7th cell x -0.3, every 3rd "
                           "absorbed ledger negated), refine",
                     "I3": "integrate level: hook trigger 0.5, config T, G2, rtol 1e-6, banded and client_krylov, "
                           "inject_failure_at_remap k = 1 and 3"},
           "status": "OK", "injected": "Err(Injected)", "commit_after_refusal": "Err(NothingStaged)",
           "unchanged": "the output buffer (prefilled with a sentinel), the committed edges and the epoch bitwise "
                        "after the injected propose and the NothingStaged commit",
           "work_kept": "after the refusal attempts 1, refusals 1, commits 0, overlap segments > 0; after the "
                        "normal commit attempts 2, commits 1",
           "other_mesh": "the normal commit goes to the other variant of the aligned mesh (refined if the injected "
                         "attempt was unrefined and vice versa) and equals a fresh remesher's remap bitwise",
           "epoch_after_commit": 1, "allocations": 0,
           "new_edges": "both variants bitwise equal to the reference aligned meshes of the runner's x_chi",
           "remap_parity_rel": 1e-14,
           "remap_parity_def": "per column (angular columns, absorbed ledgers): max |rust - python| / max |python|",
           "ledgers_copied": "L_lo, L_hi bitwise",
           "I3": "NUMERICAL_FAILURE with the typed Injected refusal; final (t, state, edges, epoch) bitwise equal to "
                 "the uninjected run's k-th remap record (t, y_before, old_edges, epoch k - 1); attempts and every "
                 "WorkCounters field equal to its attempts_before/counters_before; remesh work attempts k, "
                 "refusals 1, commits k - 1; the outputs before the refusal bitwise equal; the Krylov client's "
                 "factorizations equal direct_factorizations"},
    "R2": {"per_remap_rel": 2e-15, "source_constant": "MAX_RELATIVE_TOTAL_CHANGE: f64 = 2.0e-15 in remesh.rs",
           "per_column_inject_rel": 2e-15, "drift_rel": 1e-11,
           "status": "every integrate run is OK except the preregistered R4 client_exact REFUSED and the R1 I3 "
                     "injected runs (NUMERICAL_FAILURE); OK runs have 0 remesh refusals",
           "scope": "every integrate run of every gate: the runner's per-remap reports and trace, and the Python "
                    "fsum of every returned output state against fsum(y0)",
           "independent": "runs with record_remap_states (G2 hook banded/Krylov, G2 refined hook banded/Krylov, "
                          "G2 SMOOTH schedule rtol 1e-8, G3 config E hook): per remap |fsum(y_after) - "
                          "fsum(y_before)| / |fsum(y_before)| <= 2e-15 recomputed in Python, the Python remap of "
                          "y_before onto new_edges equal to y_after per column <= 1e-14, new_edges bitwise equal "
                          "to the Python aligned mesh of the reported face, old_edges equal to the previous mesh",
           "note": "the runner's own per-remap value is checked at the source (a larger change is refused); the "
                   "independent check above is the evidence"},
    "R3": {"cycles": 100, "one_cycle_ulps_max": 2, "cycles_rel": 1e-13, "zero_reference_abs": 0.0,
           "allocations_after_warmup": 0, "kernel_per_remap_rel": 2e-15, "kernel_total_drift_rel": 1e-13,
           "cases": {"C1": "the integrated state at t = 1.3, x_chi(1.3)", "C2": "the I2 signed state, x_chi(2.1)"}},
    "R4": {"arm": "hook trigger 0.5, config T, G2, rtol 1e-6; unrefined and refined (refine = true)",
           "client": {"restart": 4, "max_iterations": 48, "rtol": 1e-12, "two_level": True},
           "remap_count_equal": True, "client_epoch_equals_remaps": True, "segments": "remaps + 1",
           "final_rel_factor": 10.0,
           "final_def": "|X_krylov - X_banded| / |X_banded| <= 10 rtol for X in photons, absorbed, L_lo, L_hi",
           "refined": "final mesh n + 2 cells; client factorizations == direct_factorizations (work carried across "
                      "the rebuild)",
           "restart": "every remap of every hook run here: h_restart == control_h_next bitwise (the driver's stop "
                      "record against the accept view); restart = fresh controls: h_restart == initial_step",
           "exact_mode_refusal": "client_exact: status REFUSED with STRUCTURED_EXACT_MIXED_DIRECTION (the "
                                 "comoving energy drift -s c_j changes sign across columns)"},
    "R5": {"rtols": [1e-6, 1e-9], "attempt_ratio_min": 2.0, "schedule_K": [2, 4, 8],
           "schedule_times": "T k / (K + 1), k = 1..K", "hook_triggers": [1.0, 0.5, 0.25],
           "linears": ["banded", "client_krylov"], "outputs": [0.0, 3.0],
           "equal": "allocation counts equal at both rtols for every K and linear (positive control calibrated)",
           "affine": "schedule: A(8) - A(4) == 2 (A(4) - A(2)) exactly; hook: equal allocation increments per "
                     "remap across the triggers",
           "repeat_bitwise": "two hook runs (banded, rtol 1e-6): outputs, remaps and counters bitwise"},
    "R6": {"a": {"grids": ["G2", "G3", "G4", "G5"], "config": "E (gamma 0)", "arm": "static", "rtol": 1e-8,
                 "metrics": ["cells_l1_rel", "absorbed_rel"], "metrics_H": ["photons_rel"],
                 "monotone": "strict over the grids", "order_window": [0.8, 1.25], "order_pair": "G4->G5",
                 "exact": "P02 Characteristics: cell integrals 12x12 GL on 4x4 subcells; absorbed_total(n_t 48, "
                          "400, 48)",
                 "quadrature_refinement_factor": 1e-3, "absorbed_refinement_rel": 1e-10,
                 "metric_def": "cells: sum |N - N_ex| / sum |N_ex|; absorbed: |sum A - A_ex| / A_ex; photons [H] "
                               "(by conservation, the absorbed metric times A_ex / sum N_ex)",
                 "remap_frequency_H": "G4, config E, hook triggers 1, 0.5, 0.25: Sigma delta and the errors "
                                      "against the exact solution on each final mesh"},
           "b": {"config": "T (gamma 0.5)", "grid": "G2", "arm": "schedule", "remap_times": "SMOOTH",
                 "rtols": [1e-4, 1e-6, 1e-8], "outputs": [0.0, 1.0, 2.0, 3.0], "reference_rtol": 1e-12,
                 "reference_uncertainty_rtol": 1e-11, "reference_uncertainty_over_min_error_max": 0.25,
                 "mesh_match_rel": 4 * EPS, "strict_decrease": True,
                 "error_def": "max over outputs t > 0 of max |y - y_ref| / max |y_ref| (whole state)"},
           "c": {"config": "T", "grid": "G2", "arm": "fixed_schedule", "remap_times": "SMOOTH",
                 "N": [1, 2, 4, 8, 16, 32, 64], "outputs": [0.0, 3.0],
                 "reference": "DOP853 rtol 2.5e-14 (atol 1e-20 C); u_ref against Radau 1e-13",
                 "floor_abs_min": 1e-12, "floor_uref_factor": 100.0, "min_pre_floor_pairs": 2,
                 "min_last_slope": 4.8, "negative_control_max_last_slope": 3.5,
                 "negative_control": "COARSE schedule runs against the COARSE reference: one threshold crossing "
                                     "of a base edge (t = e^0.3125 - 1 = 0.367) inside the first segment, on the "
                                     "base mesh before any remap, makes kbar(t) C0 there; the four remapped "
                                     "segments are smooth. Every run must be OK with the steps rule",
                 "steps": "rejected 0, accepted == attempts, segments N <= attempts <= segments (N + 1), "
                          "remaps == segments - 1"},
           "d": {"arm": "hook trigger 0.5", "config": "E", "grids": ["G3", "G4", "G5"],
                 "l1_change_sum": "strictly decreasing",
                 "note": "co-refines the grid and the remap count (the trigger is in base widths); the fixed-grid "
                         "remap-frequency refinement is R6a remap_frequency_H"}},
    "R7": {"kind": "documentation meta-gate: the thresholds equal the plan literal PLAN and the constants reused "
                   "from the P02 and P02b harnesses (read from their TH by AST, read-only)",
           "plan": PLAN,
           "reused": {"R6c from P02 C5": ["floor_abs_min", "floor_uref_factor", "min_pre_floor_pairs",
                                           "min_last_slope", "negative_control_max_last_slope"],
                      "R6a from P02 C4": ["order_window", "selfcheck_refinement_factor",
                                          "selfcheck_absorbed_refinement_rel"],
                      "R6b from P02b C6": ["reference_uncertainty_over_min_error_max"]}},
    "incomplete": "any runner request with status TIMEOUT or ENVIRONMENT_FAILURE or a nonzero exit code makes the "
                  "verdict INVALID",
    "cargo": {"binaries": ["lrs_photon_remap_contracts", "accept_control_contracts", "lrs_block_solver_contracts",
                           "lrs_photon_fv_contracts", "generic_scope_contracts", "rodas5p_fast_allocations",
                           "fast_vector_atol_contracts", "fixed_step_order_contracts",
                           "client_stage_system_contracts", "partial_t_into_contracts"],
              "vigilode_binaries": ["records_contracts"],
              "gate_tests": {
                  "R1": ["r1_refusals_roll_back_bit_for_bit_and_keep_their_work",
                         "r1_a_refusal_inside_a_run_keeps_the_accepted_state_and_all_work"],
                  "R2": ["r2_one_remap_preserves_the_total_and_every_column",
                         "segmented_runs_are_deterministic_and_conserve_the_total"],
                  "R3": ["r3_refine_coarsen_cycles_are_exact_and_allocate_nothing",
                         "aligned_edges_are_increasing_with_the_face_at_x_chi"],
                  "R4": ["r4_rebind_makes_the_old_factorization_stale_and_the_rebound_client_restarts",
                         "r4_restarts_take_the_controllers_proposal_or_a_fresh_step",
                         "r4_refined_meshes_rebuild_the_client_and_keep_its_work",
                         "a_continuing_control_leaves_the_run_bit_identical",
                         "the_control_sees_every_accepted_step_once_in_order",
                         "a_stop_at_the_kth_accepted_step_returns_that_step", "a_restart_from_the_stop_reaches_tf",
                         "a_stop_at_tf_is_a_success", "a_stop_on_the_last_budgeted_attempt_is_a_client_stop",
                         "the_client_stop_record_serializes", "every_enum_variant_preserves_its_wire_representation"],
                  "R5": ["r5_remaps_allocate_nothing_and_restarts_a_constant_count",
                         "the_control_allocates_nothing",
                         "generic_solver_core_does_not_embed_physical_client_bindings"]}},
}

LO, HI = -3.0, 2.0
T_END = 3.0
GRIDS = {"G1": (16, 8), "G2": (32, 16), "G3": (64, 32), "G4": (128, 64), "G5": (256, 128)}
NQ_ONLY = [(32, 16), (64, 16), (128, 16), (256, 16)]
GEOM = R.Geometry("power_law", hh=0.5, s0=0.2, p=1.5)
ABSORBER = R.Absorber(0.5, -0.5)
GAMMA = {"E": 0.0, "T": 0.5}
KRYLOV = TH["R4"]["client"]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def sanitize(x):
    if isinstance(x, dict):
        return {str(k): sanitize(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [sanitize(v) for v in x]
    if isinstance(x, np.ndarray):
        return sanitize(x.tolist())
    if isinstance(x, (np.bool_,)):
        return bool(x)
    if isinstance(x, np.integer):
        return int(x)
    if isinstance(x, (float, np.floating)):
        v = float(x)
        return v if math.isfinite(v) else repr(v)
    return x


def finite_max(values):
    a = np.asarray([np.nan if v is None else v for v in values], dtype=float).ravel()
    if a.size == 0 or not bool(np.all(np.isfinite(a))):
        return float("nan")
    return float(np.max(a))


def finite_min(values):
    a = np.asarray([np.nan if v is None else v for v in values], dtype=float).ravel()
    if a.size == 0 or not bool(np.all(np.isfinite(a))):
        return float("nan")
    return float(np.min(a))


def le(x, th):
    return isinstance(x, (int, float)) and math.isfinite(float(x)) and float(x) <= th


def slope(e1, e2):
    return math.log2(e1 / e2) if e1 > 0 and e2 > 0 else float("nan")


def bits_equal(a, b):
    a = np.ascontiguousarray(np.asarray(a, dtype=np.float64))
    b = np.ascontiguousarray(np.asarray(b, dtype=np.float64))
    return a.shape == b.shape and bool(np.all(a.view(np.uint64) == b.view(np.uint64)))


def ulps(a, b):
    def key(x):
        i = int(np.float64(x).view(np.int64))
        return -(2 ** 63) - i if i < 0 else i
    return abs(key(a) - key(b))


def sha256_file(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def max_rel(y, ref):
    y, ref = np.asarray(y, float), np.asarray(ref, float)
    return float(np.max(np.abs(y - ref)) / np.max(np.abs(ref)))


P02_DIR = "research/bassrei_p02_fixed_grid_20261021"
P02B_HARNESS = "research/bassrei_p02b_fixed_grid_20261022/run_p02b.py"
WATCHED = tuple(p for p in ("crates", str(HERE.relative_to(REPO)), f"{P02_DIR}/reference", f"{P02_DIR}/run_p02.py",
                            P02B_HARNESS, "research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py", "Cargo.toml",
                            "Cargo.lock", "rust-toolchain.toml", ".cargo") if (REPO / p).exists())


def tree_changes(own_files, own_dirs):
    try:
        p = subprocess.run(["git", "-C", str(REPO), "status", "--porcelain", "--untracked-files=all", "--",
                            *WATCHED], capture_output=True, text=True, timeout=60)
    except Exception as e:  # pragma: no cover
        return None, f"git status unavailable: {e}"
    if p.returncode != 0:
        return None, f"git status exit {p.returncode}: {p.stderr[-500:]}"
    own_files = {Path(f).resolve() for f in own_files}
    own_dirs = [Path(d).resolve() for d in own_dirs]
    changes = []
    for line in p.stdout.splitlines():
        path = line[3:]
        if " -> " in path:
            path = path.split(" -> ")[-1]
        ap = (REPO / path.strip('"')).resolve()
        if ap in own_files or any(ap == d or d in ap.parents for d in own_dirs):
            continue
        changes.append(line)
    return changes, None


SUPPORT = "crates/rodas5p-integrators/examples/lrs_photon_fv_support"
INTEG = "crates/rodas5p-integrators"
PROVENANCE_FILES = {
    "adapter_mod_rs": f"{SUPPORT}/mod.rs",
    "remesh_rs": f"{SUPPORT}/remesh.rs",
    "block_solver_rs": f"{SUPPORT}/block_solver.rs",
    "runner_rs": f"{INTEG}/examples/lrs_photon_remap_run.rs",
    "test_lrs_photon_remap_contracts_rs": f"{INTEG}/tests/lrs_photon_remap_contracts.rs",
    "test_accept_control_contracts_rs": f"{INTEG}/tests/accept_control_contracts.rs",
    "rodas5p_fast_rs": f"{INTEG}/src/rodas5p_fast.rs",
    "client_stage_rs": f"{INTEG}/src/client_stage.rs",
    "conservative_remap_rs": f"{INTEG}/src/conservative_remap.rs",
    "integ_lib_rs": f"{INTEG}/src/lib.rs",
    "vigilode_records_rs": "crates/vigilode/src/records.rs",
    "vigilode_solve_rs": "crates/vigilode/src/solve.rs",
    "run_p05_py": str((HERE / "run_p05.py").relative_to(REPO)),
    "p05_reference_py": str((HERE / "reference" / "p05_reference.py").relative_to(REPO)),
    "test_p05_reference_py": str((HERE / "reference" / "test_p05_reference.py").relative_to(REPO)),
    "p02_reference_py": str(F.P02_REFERENCE.relative_to(REPO)),
    "oracle_py": str(R.ORACLE_PATH.relative_to(REPO)),
    "p02_harness_py": f"{P02_DIR}/run_p02.py",
    "p02b_harness_py": P02B_HARNESS,
    "cargo_lock": "Cargo.lock",
    "cargo_toml": "Cargo.toml",
    "rust_toolchain_toml": "rust-toolchain.toml",
}


def crate_sources_digest():
    rows = []
    for p in sorted((REPO / "crates").rglob("*")):
        if p.is_file() and (p.suffix == ".rs" or p.name == "Cargo.toml"):
            rows.append(f"{p.relative_to(REPO)} {sha256_file(p)}")
    rows.append(f"Cargo.toml {sha256_file(REPO / 'Cargo.toml')}")
    return {"files": len(rows), "sha256": hashlib.sha256("\n".join(rows).encode()).hexdigest()}


def provenance_hashes():
    out = {}
    for k, rel in PROVENANCE_FILES.items():
        p = REPO / rel
        out[k] = {"path": rel, "sha256": sha256_file(p) if p.is_file() else None}
    out["crate_sources"] = crate_sources_digest()
    return out


def git_info():
    def run(args):
        try:
            return subprocess.run(["git", "-C", str(REPO), *args], capture_output=True, text=True,
                                  timeout=60).stdout.strip()
        except Exception as e:  # pragma: no cover
            return f"unavailable: {e}"
    return {"head": run(["rev-parse", "HEAD"]), "status_short": run(["status", "--short"]).splitlines()}


def cargo_env():
    env = dict(os.environ)
    env["RUSTUP_TOOLCHAIN"] = "1.94.1"
    env.setdefault("CARGO_TARGET_DIR", "/home/cosmosapjw/vigilode-bassrei-target")
    env["CARGO_TARGET_DIR"] = str((REPO / env["CARGO_TARGET_DIR"]).resolve())
    return env


def rustc_version():
    try:
        return subprocess.run(["rustc", "-V"], env=cargo_env(), capture_output=True, text=True,
                              timeout=60).stdout.strip()
    except Exception as e:  # pragma: no cover
        return f"unavailable: {e}"


def build_runner(timeout_s, raw_dir):
    env = cargo_env()
    cmd = ["cargo", "build", "--profile", "measurement", "-p", "rodas5p-integrators", "--example",
           "lrs_photon_remap_run", "--locked", "--offline", "-j", "8"]
    binary = Path(env["CARGO_TARGET_DIR"]) / "measurement" / "examples" / "lrs_photon_remap_run"
    t0 = time.perf_counter()
    try:
        p = subprocess.run(cmd, cwd=str(REPO), env=env, capture_output=True, text=True, timeout=timeout_s)
        rc, log = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        rc, log = None, "TIMEOUT\n" + str(e.stderr)
    (raw_dir / "cargo_build.log").write_text(log)
    return {"command": " ".join(cmd), "exit_code": rc, "wall_s": time.perf_counter() - t0,
            "env": {"RUSTUP_TOOLCHAIN": env["RUSTUP_TOOLCHAIN"], "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"]},
            "log_file": "raw/cargo_build.log", "log_sha256": hashlib.sha256(log.encode()).hexdigest(),
            "log_tail": log[-1500:], "binary": str(binary), "binary_exists": binary.is_file()}, binary


def write_manifest(raw_dir):
    files = {}
    for p in sorted(raw_dir.rglob("*")):
        if p.is_file() and p.name != "MANIFEST.json":
            files[str(p.relative_to(raw_dir))] = sha256_file(p)
    (raw_dir / "MANIFEST.json").write_text(json.dumps({"files": files, "count": len(files), "hash": "sha256"},
                                                      indent=1, sort_keys=True) + "\n")
    return {"count": len(files), "manifest_sha256": sha256_file(raw_dir / "MANIFEST.json")}


class Log:
    def __init__(self):
        self.t0 = time.perf_counter()
        self.lock = threading.Lock()

    def __call__(self, *a):
        with self.lock:
            print(f"[{time.perf_counter() - self.t0:8.1f}s]", *a, flush=True)


LOG = Log()


class Runner:
    def __init__(self, binary, work, raw, jobs, timeout):
        self.binary, self.work, self.raw = str(binary), Path(work), Path(raw)
        self.pool = ThreadPoolExecutor(max_workers=jobs)
        self.timeout = timeout
        self.lock = threading.Lock()
        self.all_runs = {}
        self.incomplete = []

    def submit(self, tag, req, mode="timing"):
        return self.pool.submit(self._run, tag, req, mode)

    def _run(self, tag, req, mode):
        path = self.work / f"{tag}.request.json"
        path.write_text(json.dumps(req))
        cmd = [self.binary, "--request", str(path), "--measurement-mode", mode]
        t0 = time.perf_counter()
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=self.timeout, cwd=str(REPO))
            try:
                resp = json.loads(proc.stdout)
            except Exception:
                resp = {"status": "ENVIRONMENT_FAILURE", "reason": "unparseable stdout; stderr: " + proc.stderr[-2000:]}
            resp["_returncode"] = proc.returncode
        except subprocess.TimeoutExpired:
            resp = {"status": "TIMEOUT", "reason": f"runner exceeded {self.timeout:.0f}s"}
        resp["_wall_s"] = time.perf_counter() - t0
        compact_req = {k: v for k, v in req.items() if k != "initial_state"}
        if "initial_state" in req:
            compact_req["initial_state_sha256"] = hashlib.sha256(json.dumps(req["initial_state"]).encode()).hexdigest()
        compact = dict(resp)
        bad_exit = resp.get("_returncode") not in (None, 0)
        with self.lock:
            if resp.get("status") in ("TIMEOUT", "ENVIRONMENT_FAILURE") or bad_exit:
                self.incomplete.append({"tag": tag, "status": resp.get("status"),
                                        "returncode": resp.get("_returncode"),
                                        "reason": (resp.get("reason") or "")[:300]})
            self.all_runs[tag] = {"tag": tag, "status": resp.get("status"), "reason": (resp.get("reason") or "")[:300],
                                  "wall_s": resp["_wall_s"], "timing_authority": "diagnostic_only",
                                  "attempts": resp.get("attempts"), "remap_count": resp.get("remap_count"),
                                  "allocations": resp.get("allocations"), "client_ops": resp.get("client_ops")}
        (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": compact})))
        path.unlink(missing_ok=True)
        return resp


# ---------------------------------------------------------------------------
# Scenario
# ---------------------------------------------------------------------------


def mu_edges(nm):
    return np.linspace(-1.0, 1.0, nm + 1).tolist()


def base_grid(nq, nm):
    return R.Grid(F.base_edges(LO, HI, nq), mu_edges(nm), "comoving", "outflow")


def y0_of(nq, nm):
    return R.initial_state(base_grid(nq, nm))[0]


def signed_state(y_int, nq, nm):
    """A signed version of an integrated state: every 7th cell x -0.3 and
    every 3rd absorbed ledger negated (the sign split of the remap)."""
    g = base_grid(nq, nm)
    y = np.array(y_int, float)
    flat = g.cells.ravel()
    y[flat[::7]] *= -0.3
    y[g.absorbed[::3]] *= -1.0
    return y


S0_OUTPUTS = [0.0, 1.3, 2.1]


def s0_submit(runner):
    """The integrated states of R1 and R3 (static, config T, G2, rtol 1e-10)."""
    return runner.submit("S0_static_states", integrate_request("G2", "T", "static", 1e-10, S0_OUTPUTS))


def s0_states(fut):
    r = fut.result()
    if r.get("status") != "OK" or [o["t"] for o in r.get("outputs") or []] != S0_OUTPUTS:
        raise RuntimeError(f"S0 static run: {r.get('status')} {r.get('reason')}")
    nq, nm = GRIDS["G2"]
    y13 = np.array(r["outputs"][1]["state"], float)
    y21 = np.array(r["outputs"][2]["state"], float)
    return y13, signed_state(y21, nq, nm)


def layout_request(nq, nm):
    return {"q_lo": LO, "q_hi": HI, "nq": nq, "mu_edges": mu_edges(nm), "coordinate": "comoving",
            "boundary": "outflow"}


def common_request(nq, nm, config):
    return {"layout": layout_request(nq, nm), "geometry": GEOM.request(), "gamma": GAMMA[config],
            "absorber": ABSORBER.request()}


def integrate_request(gid, config, arm, rtol, outputs, linear="banded", y0=None, nm=None, **extra):
    nq, gnm = GRIDS[gid] if gid in GRIDS else gid
    nm = gnm if nm is None else nm
    req = common_request(nq, nm, config)
    req.update({"mode": "integrate", "arm": arm, "rtol": rtol, "atol_scale": rtol * 1e-6 * R.C_AMP,
                "t_span": [0.0, T_END], "output_times": list(outputs),
                "initial_state": (y0_of(nq, nm) if y0 is None else y0).tolist(), "linear": linear})
    if linear == "client_krylov":
        req["krylov"] = KRYLOV
    req.update(extra)
    return req


SAVED = {"exact_cells_static": {}, "exact_cells_hook": {}, "exact_cells_freq_G4": {}, "absorbed_exact": None,
         "segmented": {}}


def x_rust_like(t):
    return F.x_chi(GEOM, ABSORBER, t)


# ---------------------------------------------------------------------------
# Reference workers (spawned)
# ---------------------------------------------------------------------------


def w_exact_cells(edges, nm, n_sub, n_gl):
    t0 = time.perf_counter()
    g = R.Grid(edges, mu_edges(nm), "comoving", "outflow")
    ch = R.Characteristics(GEOM, ABSORBER)
    cells, info = ch.cell_integrals(T_END, g, n_sub, n_gl)
    return cells, info, time.perf_counter() - t0


def w_absorbed(n_t, n_q_sub, n_mu_sub):
    t0 = time.perf_counter()
    ch = R.Characteristics(GEOM, ABSORBER)
    ch.require_monotone_redshift(T_END)
    return ch.absorbed_total(T_END, n_t=n_t, n_q_sub=n_q_sub, n_mu_sub=n_mu_sub), time.perf_counter() - t0


def w_segmented(config, nq, nm, remap_times, outputs, method, rtol, atol_scale):
    t0 = time.perf_counter()
    oracle = R.load_oracle()
    states, meshes, info = F.segmented_reference(oracle, GEOM, GAMMA[config], ABSORBER, (LO, HI, nq), mu_edges(nm),
                                                 y0_of(nq, nm), T_END, remap_times, outputs, method, rtol,
                                                 atol_scale)
    info["wall_s"] = time.perf_counter() - t0
    return [np.asarray(s).tolist() for s in states], meshes, info


# ---------------------------------------------------------------------------
# Gates
# ---------------------------------------------------------------------------


def r1_submit(runner, s0):
    nq, nm = GRIDS["G2"]
    y13, y21s = s0
    cases = {"I1": (y13, 1.3, False), "I2": (y21s, 2.1, True)}
    futs = {}
    for cid, (y, t, refine) in cases.items():
        req = common_request(nq, nm, "T")
        req.update({"mode": "inject", "initial_state": y.tolist(), "t": t, "refine": refine})
        futs[cid] = runner.submit(f"R1_inject_{cid}", req)
    return futs, cases


def recorded_submit(runner):
    """Runs with record_remap_states for R1 I3 and R2: the G2 hook (banded,
    Krylov) and its injected copies."""
    futs = {}
    for lin in ("banded", "client_krylov"):
        futs[("rec", lin)] = runner.submit(f"R2rec_hook_{lin}", integrate_request(
            "G2", "T", "hook", 1e-6, [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0], linear=lin, trigger=0.5,
            record_remap_states=True))
        for k in (1, 3):
            futs[("inj", lin, k)] = runner.submit(f"R1_I3_hook_{lin}_inject{k}", integrate_request(
                "G2", "T", "hook", 1e-6, [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0], linear=lin, trigger=0.5,
                inject_failure_at_remap=k))
    return futs


def run_r1(futs, cases, rec):
    th = TH["R1"]
    nq, nm = GRIDS["G2"]
    base = F.base_edges(LO, HI, nq)
    vals, ok = {}, True
    for cid, (y, t, refine) in cases.items():
        r = futs[cid].result()
        v = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200]}
        if r.get("status") != "OK":
            vals[cid] = v
            ok = False
            continue
        x = float(r["x_chi"])
        edges, face = F.aligned_edges(LO, HI, nq, x, refine)
        other, oface = F.aligned_edges(LO, HI, nq, x, not refine)
        wa, w = r["work_after_refusal"], r["work"]
        after = np.asarray(r["state_after"], float)
        ref = F.remap_state(base, y, other, nm)
        par = F.column_rel_diff(after, ref, len(other) - 1, nm) if len(after) == len(ref) else [float("inf")]
        v.update({
            "x_chi": x, "x_chi_python": x_rust_like(t), "x_chi_ulps_vs_python_H": ulps(x, x_rust_like(t)),
            "face": r.get("face"), "face_python": face, "other_face": r.get("other_face"),
            "other_face_python": oface,
            "injected": r.get("injected_refusal"), "commit_after_refusal": r.get("commit_after_refusal"),
            "buffer_unchanged": r.get("buffer_unchanged"), "edges_unchanged": r.get("edges_unchanged"),
            "epoch_unchanged": r.get("epoch_unchanged"), "work_increased": r.get("work_increased"),
            "equals_fresh_remesher": r.get("equals_fresh_remesher"),
            "work_after_refusal": wa, "work": w, "epoch": r.get("epoch"),
            "inject_allocations": r.get("inject_allocations"), "normal_allocations": r.get("normal_allocations"),
            "new_edges_bitwise": bits_equal(r.get("new_edges") or [], edges),
            "other_edges_bitwise": bits_equal(r.get("other_edges") or [], other),
            "remap_parity_per_column": par, "max_remap_parity": finite_max(par),
            "ledgers_copied": bits_equal(after[[0, -1]], np.asarray(y)[[0, -1]]),
            "ledgers_nonzero": bool(np.asarray(y)[0] != 0 and np.asarray(y)[-1] != 0),
            "per_column_relative_change": r.get("per_column_relative_change")})
        good = (r.get("injected_refusal") == th["injected"] and r.get("commit_after_refusal") == th["commit_after_refusal"]
                and r.get("buffer_unchanged") is True and r.get("edges_unchanged") is True
                and r.get("epoch_unchanged") is True and r.get("work_increased") is True
                and r.get("equals_fresh_remesher") is True
                and wa["attempts"] == 1 and wa["refusals"] == 1 and wa["commits"] == 0 and wa["overlap_segments"] > 0
                and w["attempts"] == 2 and w["commits"] == 1 and r.get("epoch") == th["epoch_after_commit"]
                and r.get("inject_allocations") == th["allocations"] and r.get("normal_allocations") == th["allocations"]
                and v["new_edges_bitwise"] and v["other_edges_bitwise"] and r.get("face") == face
                and r.get("other_face") == oface and le(v["max_remap_parity"], th["remap_parity_rel"])
                and v["ledgers_copied"] and v["ledgers_nonzero"])
        v["pass"] = bool(good)
        vals[cid] = v
        ok &= good
    # I3: a refusal inside an integration
    i3 = {}
    for lin in ("banded", "client_krylov"):
        full = rec[("rec", lin)].result()
        for k in (1, 3):
            r = rec[("inj", lin, k)].result()
            v = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200]}
            good = (full.get("status") == "OK" and r.get("status") == "NUMERICAL_FAILURE"
                    and "Injected" in (r.get("reason") or "") and len(full.get("remaps") or []) >= k)
            if good:
                e, st = full["remaps"][k - 1], full["remap_states"][k - 1]
                cb = e["counters_before"]
                counters_equal = json.dumps(r.get("counters"), sort_keys=True) == json.dumps(cb, sort_keys=True)
                rw = r.get("remesh_work") or {}
                outs_r, outs_f = r.get("outputs") or [], full.get("outputs") or []
                outs_equal = len(outs_r) < len(outs_f) and all(
                    a["t"] == b["t"] and bits_equal(a["state"], b["state"]) for a, b in zip(outs_r, outs_f))
                v.update({"t": r.get("final_t"), "t_expected": e["t"],
                          "state_bitwise": bits_equal(r.get("final_state") or [], st["y_before"]),
                          "edges_bitwise": bits_equal(r.get("final_edges") or [], st["old_edges"]),
                          "epoch": r.get("final_epoch"), "attempts": r.get("attempts"),
                          "attempts_expected": e["attempts_before"], "counters_equal": counters_equal,
                          "remesh_work": rw, "outputs_before_equal": outs_equal,
                          "client_ops": r.get("client_ops")})
                good = bool(r.get("final_t") == e["t"] and v["state_bitwise"] and v["edges_bitwise"]
                            and r.get("final_epoch") == k - 1 and r.get("attempts") == e["attempts_before"]
                            and counters_equal and rw.get("attempts") == k and rw.get("refusals") == 1
                            and rw.get("commits") == k - 1 and outs_equal)
                if lin == "client_krylov":
                    ops = r.get("client_ops") or {}
                    good &= (r.get("client_epoch") == k - 1
                             and ops.get("factorizations") == (r.get("counters") or {}).get("direct_factorizations"))
            v["pass"] = bool(good)
            i3[f"{lin}_k{k}"] = v
            ok &= good
    vals["I3"] = i3
    return {"pass": bool(ok), "values": vals, "threshold": th}


def remesh_constant():
    src = (REPO / SUPPORT / "remesh.rs").read_text()
    m = re.search(r"pub const MAX_RELATIVE_TOTAL_CHANGE: f64 = ([0-9.eE+-]+);", src)
    return float(m.group(1)) if m else float("nan")


EXPECTED_NOT_OK = {"R4 client_exact": "REFUSED"}


def independent_remaps(tag, resp, nq, nm, refine):
    """Recompute every recorded remap in Python."""
    rows, ok = [], True
    states = resp.get("remap_states") or []
    remaps = resp.get("remaps") or []
    ok &= len(states) == len(remaps) and len(states) > 0
    prev = F.base_edges(LO, HI, nq)
    for e, st in zip(remaps, states):
        yb, ya = np.asarray(st["y_before"], float), np.asarray(st["y_after"], float)
        tb, ta = math.fsum(yb), math.fsum(ya)
        rel = abs(ta - tb) / abs(tb)
        x = e.get("face")
        exp_edges = F.aligned_edges(LO, HI, nq, float(x), refine)[0] if x is not None else F.base_edges(LO, HI, nq)
        ref = F.remap_state(st["old_edges"], yb, st["new_edges"], nm)
        par = F.column_rel_diff(ya, ref, len(st["new_edges"]) - 1, nm) if len(ya) == len(ref) else [float("inf")]
        row = {"t": st["t"], "fsum_rel_change": rel, "parity": finite_max(par),
               "new_edges_bitwise": bits_equal(st["new_edges"], exp_edges),
               "old_edges_continuity": bits_equal(st["old_edges"], prev)}
        prev = st["new_edges"]
        row_ok = (le(rel, TH["R2"]["per_remap_rel"]) and le(row["parity"], TH["R1"]["remap_parity_rel"])
                  and row["new_edges_bitwise"] and row["old_edges_continuity"] and st["t"] == e["t"])
        ok &= row_ok
        rows.append(row)
    return {"tag": tag, "remaps": len(rows), "max_fsum_rel_change": finite_max([r["fsum_rel_change"] for r in rows] + [0.0]),
            "max_parity": finite_max([r["parity"] for r in rows] + [0.0]),
            "all_edges_ok": all(r["new_edges_bitwise"] and r["old_edges_continuity"] for r in rows),
            "pass": bool(ok)}


def run_r2(all_traj, r1vals, recorded):
    th = TH["R2"]
    rows, ok, not_ok = [], True, []
    const = remesh_constant()
    ok &= const == th["per_remap_rel"]
    for tag, y0, resp in all_traj:
        status = resp.get("status")
        expected = next((v for k, v in EXPECTED_NOT_OK.items() if tag.startswith(k)), None)
        if tag.startswith("R1 I3"):
            expected = "NUMERICAL_FAILURE"
        if status != "OK":
            good = expected is not None and status == expected
            not_ok.append({"tag": tag, "status": status, "expected": expected, "ok": bool(good),
                           "reason": (resp.get("reason") or "")[:200]})
            ok &= good
            continue
        if expected is not None:
            not_ok.append({"tag": tag, "status": status, "expected": expected, "ok": False})
            ok = False
            continue
        tot0 = math.fsum(np.asarray(y0, float))
        rem = [e["relative_total_change"] for e in resp.get("remaps") or []]
        inv = resp.get("invariant") or {}
        py = [abs(math.fsum(np.asarray(o["state"], float)) - tot0) / abs(tot0) for o in resp.get("outputs") or []]
        refusals = (resp.get("remesh_work") or {}).get("refusals")
        r_ok = (le(finite_max(rem + [0.0]), th["per_remap_rel"]) and le(inv.get("max_relative_drift"), th["drift_rel"])
                and le(finite_max(py + [0.0]), th["drift_rel"]) and len(py) > 0 and refusals == 0)
        rows.append({"tag": tag, "remaps": len(rem), "max_per_remap_rel": finite_max(rem + [0.0]),
                     "runner_max_drift_rel": inv.get("max_relative_drift"), "python_max_drift_rel": finite_max(py + [0.0]),
                     "remesh_refusals": refusals, "pass": bool(r_ok)})
        ok &= r_ok
    ok &= len(rows) > 0 and sum(r["remaps"] for r in rows) > 0
    inj = []
    for cid, v in r1vals.items():
        if cid == "I3":
            continue
        pc = v.get("per_column_relative_change")
        inj.append(finite_max(pc) if pc else float("nan"))
    ok &= le(finite_max(inj), th["per_column_inject_rel"])
    ind = []
    for tag, resp, nq, nm, refine in recorded:
        row = independent_remaps(tag, resp, nq, nm, refine)
        ind.append(row)
        ok &= row["pass"]
    return {"pass": bool(ok), "values": {"remesh_constant": const, "runs": rows, "not_ok_runs": not_ok,
                                         "inject_per_column_max": inj,
                                         "total_remaps_checked": sum(r["remaps"] for r in rows),
                                         "independent": ind,
                                         "independent_remaps": sum(r["remaps"] for r in ind)},
            "threshold": th}


def r3_submit(runner, s0):
    nq, nm = GRIDS["G2"]
    y13, y21s = s0
    cases = {"C1": (y13, 1.3), "C2": (y21s, 2.1)}
    futs = {}
    for cid, (y, t) in cases.items():
        req = common_request(nq, nm, "T")
        req.update({"mode": "cycle", "initial_state": y.tolist(), "cycles": TH["R3"]["cycles"],
                    "x_chi": x_rust_like(t)})
        futs[cid] = runner.submit(f"R3_cycle_{cid}", req)
    return futs


def run_r3(futs):
    th = TH["R3"]
    vals, ok = {}, True
    for cid, fut in futs.items():
        r = fut.result()
        h, k = r.get("helper") or {}, r.get("kernel") or {}
        good = r.get("status") == "OK"
        for part in (h, k):
            good &= (part.get("max_ulps_one_cycle", 1 << 60) <= th["one_cycle_ulps_max"]
                     and le(part.get("max_relative_after_cycles"), th["cycles_rel"])
                     and part.get("max_abs_at_zero_reference") == th["zero_reference_abs"]
                     and part.get("allocations_after_warmup") == th["allocations_after_warmup"])
        work = k.get("remesh_work") or {}
        good &= (le(k.get("max_relative_total_change_per_remap"), th["kernel_per_remap_rel"])
                 and le(k.get("relative_total_drift_after_cycles"), th["kernel_total_drift_rel"])
                 and work.get("refusals") == 0 and work.get("commits") == 2 * th["cycles"] + 1
                 and h.get("cycles") == th["cycles"])
        vals[cid] = {"status": r.get("status"), "helper": h, "kernel": k, "pass": bool(good)}
        ok &= good
    return {"pass": bool(ok), "values": vals, "threshold": th}


R4_OUTPUTS = [0.0, 1.5, 3.0]


def r4_submit(runner):
    futs = {}
    for lin in ("banded", "client_krylov", "client_exact"):
        futs[lin] = runner.submit(f"R4_hook_{lin}", integrate_request("G2", "T", "hook", 1e-6, R4_OUTPUTS,
                                                                      linear=lin, trigger=0.5))
    for lin in ("banded", "client_krylov"):
        futs[("refined", lin)] = runner.submit(f"R4_hook_refined_{lin}", integrate_request(
            "G2", "T", "hook", 1e-6, R4_OUTPUTS, linear=lin, trigger=0.5, refine=True, record_remap_states=True))
        futs[("fresh", lin)] = runner.submit(f"R4_hook_fresh_{lin}", integrate_request(
            "G2", "T", "hook", 1e-6, R4_OUTPUTS, linear=lin, trigger=0.5, restart="fresh"))
    return futs


def pair_check(b, k, rtol):
    th = TH["R4"]
    v = {"banded": {"status": b.get("status"), "remaps": b.get("remap_count"), "segments": b.get("segments"),
                    "attempts": b.get("attempts")},
         "client_krylov": {"status": k.get("status"), "remaps": k.get("remap_count"), "segments": k.get("segments"),
                           "client_epoch": k.get("client_epoch"), "attempts": k.get("attempts"),
                           "client_ops": k.get("client_ops")}}
    if not (b.get("status") == "OK" and k.get("status") == "OK"):
        return False, v
    diffs = {q: abs(k["final_summary"][q] - b["final_summary"][q]) / abs(b["final_summary"][q])
             for q in ("photons", "absorbed", "l_lo", "l_hi")}
    v["final_rel_diff"] = diffs
    v["over_total0_H"] = {q: abs(b["final_summary"][q]) / abs((b.get("invariant") or {}).get("total0"))
                          for q in ("l_lo", "l_hi")}
    bound = th["final_rel_factor"] * rtol
    ops = k.get("client_ops") or {}
    good = bool(b["remap_count"] == k["remap_count"] and k["remap_count"] > 0
                and k.get("client_epoch") == k["remap_count"] and k.get("segments") == k["remap_count"] + 1
                and b.get("segments") == b["remap_count"] + 1 and all(le(d, bound) for d in diffs.values())
                and ops.get("factorizations") == (k.get("counters") or {}).get("direct_factorizations"))
    return good, v


def restart_check(r, fresh=False):
    rem = r.get("remaps") or []
    if fresh:
        return bool(rem) and all(e["h_restart"] == 1e-4 * T_END for e in rem)
    return bool(rem) and all(bits_equal(e["h_restart"], e["control_h_next"]) for e in rem)


def run_r4(futs):
    th = TH["R4"]
    b, k, e = (futs[x].result() for x in ("banded", "client_krylov", "client_exact"))
    rb, rk = futs[("refined", "banded")].result(), futs[("refined", "client_krylov")].result()
    fb, fk = futs[("fresh", "banded")].result(), futs[("fresh", "client_krylov")].result()
    vals = {}
    ok_u, vals["unrefined"] = pair_check(b, k, 1e-6)
    ok_r, vals["refined"] = pair_check(rb, rk, 1e-6)
    nq = GRIDS["G2"][0]
    shape_ok = all(len(r.get("final_edges") or []) == nq + 3 for r in (rb, rk))
    vals["refined"]["final_cells"] = [len(r.get("final_edges") or []) - 1 for r in (rb, rk)]
    restart_ok = all(restart_check(r) for r in (b, k, rb, rk))
    fresh_ok = all(r.get("status") == "OK" and restart_check(r, fresh=True) for r in (fb, fk))
    vals["restart"] = {"h_next_equals_control": restart_ok, "fresh_uses_initial_step": fresh_ok,
                       "h_restart_H": {"banded": [x.get("h_restart") for x in b.get("remaps") or []],
                                       "client_krylov": [x.get("h_restart") for x in k.get("remaps") or []]},
                       "fresh_control_H": {lin: {"status": r.get("status"), "attempts": r.get("attempts"),
                                                 "remaps": r.get("remap_count"),
                                                 "final_rel_diff_vs_h_next": (
                                                     {q: abs(r["final_summary"][q] - h["final_summary"][q])
                                                      / abs(h["final_summary"][q]) for q in ("photons", "absorbed")}
                                                     if r.get("status") == "OK" and h.get("status") == "OK" else None),
                                                 "h_next_attempts": h.get("attempts")}
                                           for lin, r, h in (("banded", fb, b), ("client_krylov", fk, k))}}
    refusal_ok = e.get("status") == "REFUSED" and "STRUCTURED_EXACT_MIXED_DIRECTION" in (e.get("reason") or "")
    vals["client_exact"] = {"status": e.get("status"), "reason": (e.get("reason") or "")[:300],
                            "typed_refusal": bool(refusal_ok)}
    ok = ok_u and ok_r and shape_ok and restart_ok and fresh_ok and refusal_ok
    return {"pass": bool(ok), "values": vals, "threshold": th}


def r5_submit(runner, quick):
    th = TH["R5"]
    futs = {}
    Ks = th["schedule_K"][:2] if quick else th["schedule_K"]
    for lin in th["linears"]:
        for K in Ks:
            times = [T_END * k / (K + 1) for k in range(1, K + 1)]
            for rt in th["rtols"]:
                futs[("schedule", lin, K, rt)] = runner.submit(
                    f"R5_schedule_{lin}_K{K}_rtol{rt:.0e}",
                    integrate_request("G2", "T", "schedule", rt, th["outputs"], linear=lin, remap_times=times),
                    mode="allocation")
        for trig in th["hook_triggers"]:
            futs[("hook", lin, trig)] = runner.submit(
                f"R5_hook_{lin}_trigger{trig}",
                integrate_request("G2", "T", "hook", 1e-6, th["outputs"], linear=lin, trigger=trig), mode="allocation")
    for rep in (0, 1):
        futs[("repeat", rep)] = runner.submit(f"R5_repeat{rep}", integrate_request("G2", "T", "hook", 1e-6,
                                                                                   [0.0, 1.5, 3.0], trigger=0.5))
    return futs, Ks


def run_r5(futs, Ks):
    th = TH["R5"]
    vals, ok = {"schedule": {}, "hook": {}}, True
    for lin in th["linears"]:
        A = {}
        for K in Ks:
            r1, r2 = (futs[("schedule", lin, K, rt)].result() for rt in th["rtols"])
            a1, a2 = (r.get("allocations") or {} for r in (r1, r2))
            at1, at2 = r1.get("attempts") or 0, r2.get("attempts") or 0
            good = bool(r1.get("status") == "OK" and r2.get("status") == "OK" and a1.get("calibrated")
                        and a2.get("calibrated") and a1.get("integrate") == a2.get("integrate")
                        and at2 >= th["attempt_ratio_min"] * max(at1, 1) and r1.get("remap_count") == K
                        and r2.get("remap_count") == K)
            A[K] = a1.get("integrate")
            vals["schedule"][f"{lin}_K{K}"] = {"allocations": [a1.get("integrate"), a2.get("integrate")],
                                               "attempts": [at1, at2], "remaps": [r1.get("remap_count"),
                                                                                  r2.get("remap_count")],
                                               "ok": good}
            ok &= good
        if len(Ks) == 3 and all(isinstance(A[K], int) for K in Ks):
            aff = A[8] - A[4] == 2 * (A[4] - A[2]) and A[4] > A[2]
        else:
            aff = False
        vals["schedule"][f"{lin}_affine"] = {"A": A, "per_remap": (A[4] - A[2]) / 2 if len(Ks) > 1 else None,
                                             "ok": bool(aff)}
        ok &= aff
        hk = []
        for trig in th["hook_triggers"]:
            r = futs[("hook", lin, trig)].result()
            hk.append((r.get("remap_count"), (r.get("allocations") or {}).get("integrate"), r.get("status")))
        (m1, a1, s1), (m2, a2, s2), (m3, a3, s3) = hk
        h_ok = bool(all(s == "OK" for s in (s1, s2, s3)) and m1 < m2 < m3
                    and (a2 - a1) * (m3 - m2) == (a3 - a2) * (m2 - m1))
        vals["hook"][lin] = {"remaps": [m1, m2, m3], "allocations": [a1, a2, a3],
                             "per_remap": [(a2 - a1) / (m2 - m1) if m2 != m1 else None,
                                           (a3 - a2) / (m3 - m2) if m3 != m2 else None], "ok": h_ok}
        ok &= h_ok
    p0, p1 = futs[("repeat", 0)].result(), futs[("repeat", 1)].result()

    def fp(r):
        return json.dumps({"outputs": [(o["t"], o["epoch"], o["q_edges"], o["state"]) for o in r.get("outputs") or []],
                           "remaps": r.get("remaps"), "counters": r.get("counters"),
                           "final_edges": r.get("final_edges")}, sort_keys=True)
    rep = bool(p0.get("status") == "OK" and p1.get("status") == "OK" and fp(p0) == fp(p1))
    vals["repeat_bitwise"] = rep
    ok &= rep
    return {"pass": bool(ok), "values": vals, "threshold": th}


def r6a_submit(runner, quick):
    th = TH["R6"]["a"]
    grids = th["grids"][:-1] if quick else th["grids"]
    futs = {}
    for gid in grids:
        futs[("static", gid)] = runner.submit(f"R6a_static_{gid}", integrate_request(gid, "E", "static", th["rtol"],
                                                                                     [0.0, T_END]))
        futs[("hook", gid)] = runner.submit(f"R6a_hook_{gid}", integrate_request(
            gid, "E", "hook", th["rtol"], [0.0, T_END], trigger=0.5, record_remap_states=(gid == "G3")))
    for trig in (1.0, 0.25):
        futs[("freq", trig)] = runner.submit(f"R6a_H_hook_G4_trigger{trig}", integrate_request(
            "G4", "E", "hook", th["rtol"], [0.0, T_END], trigger=trig))
    if not quick:
        for nq, nm in NQ_ONLY:
            futs[("nq_only", (nq, nm))] = runner.submit(f"R6a_H_static_nq{nq}_nm{nm}",
                                                        integrate_request((nq, nm), "E", "static", th["rtol"],
                                                                          [0.0, T_END]))
    return futs, grids


def r6a_refs(pool, grids, quick):
    jobs = {}
    for gid in grids:
        nq, nm = GRIDS[gid]
        e = F.base_edges(LO, HI, nq)
        jobs[("cells", gid, 4, 12)] = pool.submit(w_exact_cells, e, nm, 4, 12)
        jobs[("cells", gid, 2, 8)] = pool.submit(w_exact_cells, e, nm, 2, 8)
    if not quick:
        for nq, nm in NQ_ONLY:
            jobs[("cells", (nq, nm), 4, 12)] = pool.submit(w_exact_cells, F.base_edges(LO, HI, nq), nm, 4, 12)
    jobs[("absorbed", 48)] = pool.submit(w_absorbed, 48, 400, 48)
    jobs[("absorbed", 32)] = pool.submit(w_absorbed, 32, 250, 32)
    return jobs


def exact_metrics(y, edges, nm, cells_ex, absorbed_ex):
    g = R.Grid(edges, mu_edges(nm), "comoving", "outflow")
    y = np.asarray(y, float)
    n = y[g.cells]
    return {"cells_l1_rel": R.l1_rel(n, cells_ex),
            "absorbed_rel": abs(math.fsum(y[g.absorbed]) - absorbed_ex) / absorbed_ex,
            "photons_rel": abs(math.fsum(n.ravel()) - math.fsum(np.asarray(cells_ex).ravel())) /
            math.fsum(np.asarray(cells_ex).ravel()),
            "L_lo": float(y[g.lo]), "L_hi": float(y[g.hi])}


def run_r6a(futs, grids, refs, pool, quick):
    th = TH["R6"]["a"]
    a48, w48 = refs[("absorbed", 48)].result()
    a32, _ = refs[("absorbed", 32)].result()
    SAVED["absorbed_exact"] = {"n_t48": a48, "n_t32": a32}
    abs_ref = abs(a32 - a48) / abs(a48)
    rows, errs, ok = {}, {m: [] for m in th["metrics"] + th["metrics_H"]}, True
    hook_jobs = {}
    for gid in grids:
        nq, nm = GRIDS[gid]
        r = futs[("static", gid)].result()
        ex, info, tex = refs[("cells", gid, 4, 12)].result()
        ex2, _, _ = refs[("cells", gid, 2, 8)].result()
        SAVED["exact_cells_static"][gid] = {"12x12_4x4": ex, "8x8_2x2": ex2}
        row = {"status": r.get("status"), "attempts": r.get("attempts"), "wall_s": r.get("_wall_s"),
               "exact_s": tex, "quadrature_refinement_l1": R.l1_rel(ex2, ex)}
        if r.get("status") != "OK":
            ok = False
            for m in errs:
                errs[m].append(float("nan"))
        else:
            met = exact_metrics(r["outputs"][-1]["state"], r["outputs"][-1]["q_edges"], nm, ex, a48)
            row.update(met)
            for m in errs:
                errs[m].append(met[m])
            row["refinement_over_error"] = row["quadrature_refinement_l1"] / met["cells_l1_rel"]
            ok &= le(row["refinement_over_error"], th["quadrature_refinement_factor"])
        rows[gid] = row
        h = futs[("hook", gid)].result()
        if h.get("status") == "OK":
            hook_jobs[gid] = (h, pool.submit(w_exact_cells, h["final_edges"], nm, 4, 12))
    freq_jobs = {}
    if not quick:
        for trig in (1.0, 0.25):
            h = futs[("freq", trig)].result()
            if h.get("status") == "OK":
                freq_jobs[trig] = (h, pool.submit(w_exact_cells, h["final_edges"], GRIDS["G4"][1], 4, 12))
    orders, crit = {}, {}
    lo, hi = th["order_window"]
    for m, e in errs.items():
        o = [slope(a, b) for a, b in zip(e[:-1], e[1:])]
        mono = all(math.isfinite(x) for x in e) and all(a > b for a, b in zip(e[:-1], e[1:]))
        last = o[-1] if o else float("nan")
        win = math.isfinite(last) and lo <= last <= hi
        gated = m in th["metrics"]
        orders[m] = {"errors": e, "orders": o, "monotone": mono, "last_order": last, "in_window": win,
                     "gated": gated}
        if gated:
            ok &= mono and win
    ok &= le(abs_ref, th["absorbed_refinement_rel"])
    hook = {}
    for gid, (h, job) in hook_jobs.items():
        ex, _, _ = job.result()
        met = exact_metrics(h["outputs"][-1]["state"], h["final_edges"], GRIDS[gid][1], ex, a48)
        met.update({"remaps": h.get("remap_count"), "attempts": h.get("attempts"),
                    "l1_change_sum": (h.get("remesh_work") or {}).get("l1_change_sum"),
                    "h_restart_first": [x.get("h_restart") for x in (h.get("remaps") or [])[:5]]})
        hook[gid] = met
        SAVED["exact_cells_hook"][gid] = ex
    freq = {}
    for trig, (h, job) in sorted(freq_jobs.items()):
        ex, _, _ = job.result()
        met = exact_metrics(h["outputs"][-1]["state"], h["final_edges"], GRIDS["G4"][1], ex, a48)
        met.update({"remaps": h.get("remap_count"), "l1_change_sum": (h.get("remesh_work") or {}).get("l1_change_sum")})
        freq[str(trig)] = met
        SAVED["exact_cells_freq_G4"][str(trig)] = ex
    if "G4" in hook:
        freq["0.5"] = {k: hook["G4"][k] for k in ("cells_l1_rel", "absorbed_rel", "photons_rel", "remaps",
                                                   "l1_change_sum")}
    nq_only = {}
    if not quick:
        e_nq = []
        for nq, nm in NQ_ONLY:
            r = futs[("nq_only", (nq, nm))].result()
            ex, _, _ = refs[("cells", (nq, nm), 4, 12)].result()
            if r.get("status") == "OK":
                met = exact_metrics(r["outputs"][-1]["state"], r["outputs"][-1]["q_edges"], nm, ex, a48)
                nq_only[f"{nq}x{nm}"] = met
                e_nq.append(met["cells_l1_rel"])
        nq_only["cells_l1_orders"] = [slope(a, b) for a, b in zip(e_nq[:-1], e_nq[1:])]
    return {"pass": bool(ok) and not quick, "values": {"grids": rows, "orders": orders,
                                                       "absorbed_exact": a48, "absorbed_refinement_rel": abs_ref,
                                                       "absorbed_exact_s": w48, "hook_vs_exact_H": hook,
                                                       "remap_frequency_G4_H": freq, "nq_only_nm16_H": nq_only},
            "threshold": th}, hook


def r6bc_submit(runner, quick):
    tb, tc = TH["R6"]["b"], TH["R6"]["c"]
    futs = {}
    rts = tb["rtols"][:2] if quick else tb["rtols"]
    for rt in rts:
        futs[("b", "banded", rt)] = runner.submit(f"R6b_schedule_rtol{rt:.0e}",
                                                  integrate_request("G2", "T", "schedule", rt, tb["outputs"],
                                                                    remap_times=SMOOTH,
                                                                    record_remap_states=(rt == 1e-8)))
        futs[("b", "client_krylov", rt)] = runner.submit(f"R6b_H_schedule_krylov_rtol{rt:.0e}",
                                                         integrate_request("G2", "T", "schedule", rt, tb["outputs"],
                                                                           linear="client_krylov", remap_times=SMOOTH))
        futs[("b", "coarse", rt)] = runner.submit(f"R6b_H_coarse_rtol{rt:.0e}",
                                                  integrate_request("G2", "T", "schedule", rt, tb["outputs"],
                                                                    remap_times=COARSE))
    Ns = tc["N"][:5] if quick else tc["N"]
    for sched, times in (("smooth", SMOOTH), ("coarse", COARSE)):
        for N in Ns:
            futs[("c", sched, N)] = runner.submit(f"R6c_fixed_{sched}_N{N}",
                                                  integrate_request("G2", "T", "fixed_schedule", 1e-6, tc["outputs"],
                                                                    remap_times=times, fixed_steps=N))
    return futs, rts, Ns


def r6bc_refs(pool):
    tb, tc = TH["R6"]["b"], TH["R6"]["c"]
    nq, nm = GRIDS["G2"]
    jobs = {}
    for name, rt in (("ref", tb["reference_rtol"]), ("unc", tb["reference_uncertainty_rtol"])):
        jobs[("b", name)] = pool.submit(w_segmented, "T", nq, nm, SMOOTH, tb["outputs"], "Radau", rt, rt * 1e-6 * R.C_AMP)
    jobs[("b", "coarse")] = pool.submit(w_segmented, "T", nq, nm, COARSE, tb["outputs"], "Radau",
                                        tb["reference_rtol"], tb["reference_rtol"] * 1e-6 * R.C_AMP)
    for name, times, method, rt in (("smooth", SMOOTH, "DOP853", 2.5e-14), ("smooth_radau", SMOOTH, "Radau", 1e-13),
                                    ("coarse", COARSE, "DOP853", 2.5e-14), ("coarse_radau", COARSE, "Radau", 1e-13)):
        jobs[("c", name)] = pool.submit(w_segmented, "T", nq, nm, times, tc["outputs"], method, rt, 1e-20 * R.C_AMP)
    return jobs


def traj_error(resp, ref_states):
    outs = resp.get("outputs") or []
    if len(outs) != len(ref_states):
        return float("nan")
    return max(max_rel(o["state"], q) for o, q in zip(outs[1:], ref_states[1:]))


def run_r6b(futs, rts, refs, quick):
    th = TH["R6"]["b"]
    ref, ref_meshes, rinfo = refs[("b", "ref")].result()
    unc, _, _ = refs[("b", "unc")].result()
    cref, _, _ = refs[("b", "coarse")].result()
    SAVED["segmented"]["R6b_ref_radau1e-12"] = {"states": ref, "meshes": ref_meshes}
    SAVED["segmented"]["R6b_unc_radau1e-11"] = {"states": unc}
    SAVED["segmented"]["R6b_coarse_radau1e-12_H"] = {"states": cref}
    u_ref = max(max_rel(a, b) for a, b in zip(unc[1:], ref[1:]))
    errs, rows, mesh_ok, ok = [], {}, True, True
    for rt in rts:
        r = futs[("b", "banded", rt)].result()
        times_ok = [o["t"] for o in r.get("outputs") or []] == th["outputs"]
        e = traj_error(r, ref) if r.get("status") == "OK" and times_ok else float("nan")
        md = 0.0
        for o, m in zip(r.get("outputs") or [], ref_meshes):
            if len(o["q_edges"]) != len(m):
                md = float("inf")
                break
            md = max(md, max(abs(a - b) / max(abs(b), 1.0) for a, b in zip(o["q_edges"], m)))
        mesh_ok &= le(md, th["mesh_match_rel"])
        k = futs[("b", "client_krylov", rt)].result()
        c = futs[("b", "coarse", rt)].result()
        rows[f"{rt:.0e}"] = {"status": r.get("status"), "error": e, "attempts": r.get("attempts"),
                             "remaps": r.get("remap_count"), "mesh_max_rel_diff": md,
                             "client_krylov_error_H": traj_error(k, ref) if k.get("status") == "OK" else None,
                             "coarse_schedule_error_H": traj_error(c, cref) if c.get("status") == "OK" else None}
        errs.append(e)
    dec = all(math.isfinite(x) for x in errs) and all(a > b for a, b in zip(errs[:-1], errs[1:]))
    u_ok = le(u_ref, th["reference_uncertainty_over_min_error_max"] * finite_min(errs))
    ok = dec and u_ok and mesh_ok
    return {"pass": bool(ok) and not quick, "values": {"errors": errs, "rungs": rows, "strictly_decreasing": dec,
                                                       "u_ref": u_ref, "u_ref_ok": u_ok, "meshes_match": mesh_ok,
                                                       "reference": rinfo},
            "threshold": th}


def order_check(errs, floor, Ns):
    pairs = []
    for k in range(len(Ns) - 1):
        e1, e2 = errs[k], errs[k + 1]
        pre = math.isfinite(e1) and math.isfinite(e2) and e1 > floor and e2 > floor
        pairs.append({"N": [Ns[k], Ns[k + 1]], "slope": slope(e1, e2), "pre_floor": pre})
    pre = [p for p in pairs if p["pre_floor"]]
    return pairs, pre, (pre[-1]["slope"] if pre else float("nan"))


def run_r6c(futs, Ns, refs, quick):
    th = TH["R6"]["c"]
    sm, sm_meshes, sinfo = refs[("c", "smooth")].result()
    smr, _, _ = refs[("c", "smooth_radau")].result()
    co, co_meshes, _ = refs[("c", "coarse")].result()
    cor, _, _ = refs[("c", "coarse_radau")].result()
    for name, st, ms in (("R6c_smooth_dop853", sm, sm_meshes), ("R6c_smooth_radau1e-13", smr, None),
                         ("R6c_coarse_dop853", co, co_meshes), ("R6c_coarse_radau1e-13", cor, None)):
        SAVED["segmented"][name] = {"states": st, "meshes": ms}
    vals, ok = {}, True
    for name, ref, rad, sched, times, neg in (("smooth", sm, smr, "smooth", SMOOTH, False),
                                              ("coarse_negative", co, cor, "coarse", COARSE, True)):
        u = max_rel(rad[-1], ref[-1])
        floor = max(th["floor_abs_min"], th["floor_uref_factor"] * u)
        errs, steps = [], []
        segs = len(times) + 1
        steps_ok = True
        for N in Ns:
            r = futs[("c", sched, N)].result()
            if r.get("status") != "OK" or len(r.get("outputs") or []) != 2:
                errs.append(float("nan"))
                steps_ok = False
                steps.append({"N": N, "status": r.get("status")})
                continue
            errs.append(max_rel(r["outputs"][-1]["state"], ref[-1]))
            at, acc, rej = r.get("attempts"), r.get("accepted_steps"), r.get("rejected_steps")
            steps.append({"N": N, "attempts": at, "accepted": acc, "rejected": rej, "remaps": r.get("remap_count")})
            steps_ok &= bool(rej == 0 and acc == at and segs * N <= at <= segs * (N + 1)
                             and r.get("remap_count") == segs - 1)
        pairs, pre, last = order_check(errs, floor, Ns)
        if neg:
            good = steps_ok and bool(pre) and last <= th["negative_control_max_last_slope"]
        else:
            good = len(pre) >= th["min_pre_floor_pairs"] and last >= th["min_last_slope"] and steps_ok
        vals[name] = {"errors": errs, "pairs": pairs, "pre_floor_pairs": len(pre), "last_pre_floor_slope": last,
                      "floor": floor, "u_ref": u, "steps": steps, "steps_ok": steps_ok, "pass": good}
        ok &= good
    # where the threshold crosses an edge inside a segment (reference meshes)
    crossings = {}
    for name, times in (("smooth", SMOOTH), ("coarse", COARSE)):
        nq = GRIDS["G2"][0]
        edges = F.base_edges(LO, HI, nq)
        bounds = [0.0] + list(times) + [T_END]
        rows = []
        for a, b in zip(bounds[:-1], bounds[1:]):
            xa, xb = x_rust_like(a), x_rust_like(b)
            inside = [e for e in edges[1:-1] if xa < e < xb]
            rows.append({"segment": [a, b], "x_chi": [xa, xb], "edges_crossed": len(inside)})
            if b < T_END:
                edges = F.aligned_edges(LO, HI, nq, xb, False)[0]
        crossings[name] = rows
    vals["edge_crossings"] = crossings
    vals["reference"] = sinfo
    return {"pass": bool(ok) and not quick, "values": vals, "threshold": th}


def run_r6d(hook, quick):
    th = TH["R6"]["d"]
    grids = [g for g in th["grids"] if g in hook]
    s = [hook[g]["l1_change_sum"] for g in grids]
    ok = len(grids) == len(th["grids"]) and all(isinstance(x, (int, float)) for x in s) and \
        all(a > b for a, b in zip(s[:-1], s[1:]))
    return {"pass": bool(ok) and not quick, "values": {"grids": grids, "l1_change_sum": s,
                                                       "remaps": [hook[g]["remaps"] for g in grids]},
            "threshold": th}


def harness_th(rel):
    """The TH literal of another node's harness, by AST (read-only)."""
    import ast
    tree = ast.parse((REPO / rel).read_text())
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(getattr(t, "id", None) == "TH" for t in node.targets):
            return eval(compile(ast.Expression(node.value), rel, "eval"), {"__builtins__": {}, "EPS": EPS})
    raise KeyError(f"no TH in {rel}")


def run_r7():
    th = TH
    got = {"R2_per_remap_rel": th["R2"]["per_remap_rel"], "R2_drift_rel": th["R2"]["drift_rel"],
           "R3_one_cycle_ulps_max": th["R3"]["one_cycle_ulps_max"], "R3_cycles": th["R3"]["cycles"],
           "R3_cycles_rel": th["R3"]["cycles_rel"], "R6_order_window": th["R6"]["a"]["order_window"],
           "R6_min_last_slope": th["R6"]["c"]["min_last_slope"]}
    plan_ok = got == PLAN
    p02, p02b = harness_th(f"{P02_DIR}/run_p02.py"), harness_th(P02B_HARNESS)
    reused = {}
    for k in TH["R7"]["reused"]["R6c from P02 C5"]:
        reused[f"R6c.{k}"] = (th["R6"]["c"][k], p02["C5"][k])
    reused["R6a.order_window"] = (th["R6"]["a"]["order_window"], p02["C4"]["order_window"])
    reused["R6a.quadrature_refinement_factor"] = (th["R6"]["a"]["quadrature_refinement_factor"],
                                                  p02["C4"]["selfcheck_refinement_factor"])
    reused["R6a.absorbed_refinement_rel"] = (th["R6"]["a"]["absorbed_refinement_rel"],
                                             p02["C4"]["selfcheck_absorbed_refinement_rel"])
    reused["R6b.reference_uncertainty_over_min_error_max"] = (
        th["R6"]["b"]["reference_uncertainty_over_min_error_max"], p02b["C6"]["reference_uncertainty_over_min_error_max"])
    reused_ok = all(a == b for a, b in reused.values())
    return {"pass": bool(plan_ok and reused_ok),
            "values": {"thresholds": got, "plan": PLAN, "plan_equal": plan_ok,
                       "reused": {k: {"p05": a, "source": b, "equal": a == b} for k, (a, b) in reused.items()}},
            "threshold": TH["R7"]}


def run_cargo(timeout_s, raw_dir):
    env = cargo_env()
    binaries = TH["cargo"]["binaries"]
    cmds = [["cargo", "test", "-p", "rodas5p-integrators", "--release", "--locked", "--offline", "-j", "8"]
            + sum((["--test", b] for b in binaries), []) + ["--", "--test-threads=1"],
            ["cargo", "test", "-p", "vigilode", "--release", "--locked", "--offline", "-j", "8"]
            + sum((["--test", b] for b in TH["cargo"]["vigilode_binaries"]), []) + ["--", "--test-threads=1"]]
    t0 = time.perf_counter()
    out_all, err_all, rcs = "", "", []
    for cmd in cmds:
        try:
            p = subprocess.run(cmd, cwd=str(REPO), env=env, capture_output=True, text=True, timeout=timeout_s)
            rc, out, err = p.returncode, p.stdout, p.stderr
        except subprocess.TimeoutExpired as e:
            rc, out, err = None, str(e.stdout or ""), "TIMEOUT\n" + str(e.stderr or "")
        rcs.append(rc)
        out_all += out
        err_all += err
    (raw_dir / "cargo_test.log").write_text("=== stdout ===\n" + out_all + "\n=== stderr ===\n" + err_all)
    lines = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", out_all)
    passed = sum(int(m[1]) for m in lines)
    failed = sum(int(m[2]) for m in lines)
    ignored = sum(int(m[3]) for m in lines)
    names = re.findall(r"^test (\S+) \.\.\. (\S+)\s*$", out_all, flags=re.M)
    tests, dup = {}, []
    for a, b in names:
        if a in tests:
            dup.append(a)
        tests[a] = b
    expected = binaries + TH["cargo"]["vigilode_binaries"]
    running = re.findall(r"Running tests/(\w+)\.rs", err_all)
    common_ok = bool(all(rc == 0 for rc in rcs) and failed == 0 and ignored == 0 and passed > 0
                     and len(lines) == len(expected) and sorted(running) == sorted(expected)
                     and len(names) == passed and not dup and all(r == "ok" for r in tests.values()))
    gate_ok, missing = {}, {}
    for gid, exp in TH["cargo"]["gate_tests"].items():
        miss = [n for n in exp if tests.get(n) != "ok"]
        missing[gid] = miss
        gate_ok[gid] = bool(common_ok and not miss)
    return {"commands": [" ".join(c) for c in cmds], "exit_codes": rcs, "passed": passed, "failed": failed,
            "ignored": ignored, "binaries_run": running, "duplicate_test_names": dup, "all_tests_ok": common_ok,
            "tests": [{"name": a, "result": b} for a, b in names], "gate_tests_missing_or_not_ok": missing,
            "log_file": "raw/cargo_test.log", "wall_s": time.perf_counter() - t0,
            "stderr_tail": err_all[-2000:] if not common_ok else err_all[-400:]}, gate_ok


def reference_unittests(raw_dir):
    cmd = [sys.executable, "-m", "unittest", "discover", "-s", "reference", "-p", "test_*.py"]
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    try:
        p = subprocess.run(cmd, cwd=str(HERE), capture_output=True, text=True, timeout=1800, env=env)
        rc, log = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        rc, log = None, "TIMEOUT\n" + str(e.stderr)
    (raw_dir / "reference_unittests.log").write_text(log)
    return {"command": " ".join(cmd[1:]), "exit_code": rc, "tail": log.strip().splitlines()[-3:],
            "log_file": "raw/reference_unittests.log"}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True)
    ap.add_argument("--raw-dir", default=None)
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--workers", type=int, default=10)
    ap.add_argument("--timeout", type=float, default=3600.0)
    ap.add_argument("--quick", action="store_true")
    ap.add_argument("--skip-cargo", action="store_true")
    args = ap.parse_args()
    out = Path(args.out).resolve()
    raw = Path(args.raw_dir).resolve() if args.raw_dir else out.parent / "raw"
    if raw.exists() and any(raw.iterdir()):
        print(f"INVALID: raw dir {raw} not empty", file=sys.stderr)
        return 2
    raw.mkdir(parents=True, exist_ok=True)
    t_start = time.perf_counter()
    prov0 = provenance_hashes()
    git0 = git_info()
    changes0, gerr0 = tree_changes([out], [raw])
    build, binary = build_runner(3600, raw)
    LOG(f"build exit={build['exit_code']}")
    binary_sha = sha256_file(binary) if binary.is_file() else None
    results = {"node": "BASS/rei P05", "mode": "quick" if args.quick else "full", "timing_authority": "diagnostic_only",
               "thresholds": TH, "scenario_constants": {"LO": LO, "HI": HI, "T": T_END, "GRIDS": GRIDS,
                                                        "NQ_ONLY_H": NQ_ONLY, "geometry": GEOM.request(),
                                                        "absorber": ABSORBER.request(), "gamma": GAMMA,
                                                        "SMOOTH": SMOOTH, "COARSE": COARSE,
                                                        "atol_rule": "atol_scale = rtol 1e-6 C_AMP (cell rule)"},
               "build": build,
               "host": {"platform": platform.platform(), "python": platform.python_version(), "numpy": np.__version__,
                        "scipy": __import__("scipy").__version__, "cpu_count": os.cpu_count(),
                        "rustc": rustc_version(),
                        "blas_threads": {v: os.environ.get(v) for v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS",
                                                                          "MKL_NUM_THREADS")}}}
    gates = {}
    if build["exit_code"] != 0 or not binary.is_file():
        results["verdict"] = "INVALID"
        results["reason"] = "runner build failed"
    else:
        unit = reference_unittests(raw)
        LOG(f"reference unittests exit={unit['exit_code']}")
        results["reference_unittests"] = unit
        with tempfile.TemporaryDirectory() as work, \
                ProcessPoolExecutor(max_workers=args.workers, mp_context=multiprocessing.get_context("spawn")) as pool:
            runner = Runner(binary, work, raw, args.jobs, args.timeout)
            s0_f = s0_submit(runner)
            r6a_f, grids = r6a_submit(runner, args.quick)
            r6a_r = r6a_refs(pool, grids, args.quick)
            r6bc_r = r6bc_refs(pool)
            rec_f = recorded_submit(runner)
            try:
                s0 = s0_states(s0_f)
            except Exception:
                s0 = None
                gates["S0"] = {"pass": False, "error": traceback.format_exc()[-2000:]}
            if s0 is not None:
                r1_f, r1_cases = r1_submit(runner, s0)
                r3_f = r3_submit(runner, s0)
            r4_f = r4_submit(runner)
            r5_f, Ks = r5_submit(runner, args.quick)
            r6bc_f, rts, Ns = r6bc_submit(runner, args.quick)
            hook = {}

            def g_r6a():
                res, h = run_r6a(r6a_f, grids, r6a_r, pool, args.quick)
                hook.update(h)
                return res
            if s0 is None:
                def missing():
                    raise RuntimeError("S0 states missing")
                r1_fn, r3_fn = missing, missing
            else:
                def r1_fn():
                    return run_r1(r1_f, r1_cases, rec_f)

                def r3_fn():
                    return run_r3(r3_f)
            for gid, fn in (("R1", r1_fn), ("R3", r3_fn),
                            ("R4", lambda: run_r4(r4_f)), ("R5", lambda: run_r5(r5_f, Ks)),
                            ("R6a", g_r6a), ("R6b", lambda: run_r6b(r6bc_f, rts, r6bc_r, args.quick)),
                            ("R6c", lambda: run_r6c(r6bc_f, Ns, r6bc_r, args.quick)),
                            ("R6d", lambda: run_r6d(hook, args.quick)), ("R7", run_r7)):
                try:
                    gates[gid] = fn()
                except Exception:
                    gates[gid] = {"pass": False, "error": traceback.format_exc()[-3000:]}
                LOG(f"{gid} pass={gates[gid]['pass']}")
            traj = [("S0 static", y0_of(*GRIDS["G2"]), s0_f.result())]
            for key, fut in list(rec_f.items()):
                tag = f"R2rec {key[1]}" if key[0] == "rec" else f"R1 I3 {key[1]} k{key[2]}"
                traj.append((tag, y0_of(*GRIDS["G2"]), fut.result()))
            for key, fut in list(r4_f.items()):
                name = key if isinstance(key, str) else f"{key[0]} {key[1]}"
                traj.append((f"R4 {name}", y0_of(*GRIDS["G2"]), fut.result()))
            for key, fut in list(r5_f.items()):
                traj.append((f"R5 {key}", y0_of(*GRIDS["G2"]), fut.result()))
            for key, fut in list(r6a_f.items()):
                gid = "G4" if key[0] == "freq" else key[1]
                nq, nm = GRIDS[gid] if gid in GRIDS else gid
                traj.append((f"R6a {key}", y0_of(nq, nm), fut.result()))
            for key, fut in list(r6bc_f.items()):
                traj.append((f"R6bc {key}", y0_of(*GRIDS["G2"]), fut.result()))
            g2, g3 = GRIDS["G2"], GRIDS["G3"]
            recorded = [(f"R2rec {lin}", rec_f[("rec", lin)].result(), *g2, False) for lin in ("banded", "client_krylov")]
            recorded += [(f"R4 refined {lin}", r4_f[("refined", lin)].result(), *g2, True)
                         for lin in ("banded", "client_krylov")]
            recorded.append(("R6b schedule rtol 1e-8", r6bc_f[("b", "banded", 1e-8)].result(), *g2, False)
                            if ("b", "banded", 1e-8) in r6bc_f else ("R6b missing", {}, *g2, False))
            recorded.append(("R6a hook G3", r6a_f[("hook", "G3")].result(), *g3, False))
            try:
                gates["R2"] = run_r2(traj, (gates.get("R1") or {}).get("values") or {}, recorded)
            except Exception:
                gates["R2"] = {"pass": False, "error": traceback.format_exc()[-3000:]}
            LOG(f"R2 pass={gates['R2']['pass']}")
            runner.pool.shutdown(wait=True)
            results["H"] = {"all_runs": runner.all_runs, "timing_authority": "diagnostic_only"}
            results["incomplete_requests"] = runner.incomplete
            (raw / "references.json").write_text(json.dumps(sanitize(SAVED)))
        unit_ok = unit.get("exit_code") == 0
        for gid in ("R1", "R6b", "R6c"):
            if gid in gates:
                gates[gid]["reference_unittests_ok"] = unit_ok
                gates[gid]["pass"] = bool(gates[gid]["pass"] and unit_ok)
        if args.skip_cargo:
            cargo, cgate = {"skipped": True}, {}
        else:
            cargo, cgate = run_cargo(3600, raw)
            LOG(f"cargo ok={cargo['all_tests_ok']} passed={cargo['passed']}")
        results["cargo"] = cargo
        for gid, okc in cgate.items():
            if gid in gates:
                gates[gid]["pass"] = bool(gates[gid]["pass"] and okc)
                gates[gid]["cargo_tests_ok"] = okc
    prov1 = provenance_hashes()
    changes1, gerr1 = tree_changes([out], [raw])
    clean = bool(not changes0 and not changes1 and gerr0 is None and gerr1 is None and prov0 == prov1
                 and binary_sha is not None and sha256_file(binary) == binary_sha)
    incomplete = results.get("incomplete_requests") or []
    results["provenance"] = {"inputs": prov1, "binary_sha256": binary_sha, "watched": list(WATCHED),
                             "git_start": git0, "git_end": git_info(),
                             "tree_changes_start": changes0, "tree_changes_end": changes1,
                             "git_error": [gerr0, gerr1], "inputs_changed_during_run": prov0 != prov1,
                             "clean": clean}
    order = ["R1", "R2", "R3", "R4", "R5", "R6a", "R6b", "R6c", "R6d", "R7"]
    results["gates"] = gates
    results["gate_table"] = {g: bool(gates.get(g, {}).get("pass")) for g in order}
    if results.get("verdict") != "INVALID":
        if not clean:
            results["verdict"] = "INVALID"
            results["reason"] = "dirty tree, changed inputs or stale binary"
        elif incomplete:
            results["verdict"] = "INVALID"
            results["reason"] = f"incomplete run: {len(incomplete)} request(s) timed out, failed in their environment or exited nonzero"
        elif args.quick or args.skip_cargo:
            results["verdict"] = "INCONCLUSIVE"
            results["reason"] = "quick/skip-cargo mode"
        else:
            results["verdict"] = "PASS" if all(results["gate_table"].values()) else "FAIL"
    results["runtime_s"] = time.perf_counter() - t_start
    results["raw_dir"] = str(raw)
    results["raw_manifest"] = write_manifest(raw)
    out.write_text(json.dumps(sanitize(results), indent=1) + "\n")
    LOG(f"verdict {results['verdict']} gates {results['gate_table']} -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
