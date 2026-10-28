#!/usr/bin/env python3
"""Gate runner of BASS/rei DAG node P02 (fixed-grid LRS Bianchi-I photon
transport adapter run through real RODAS5P stages).

    python3 run_p02.py --out results.json [--quick] [--skip-cargo]

Builds the Rust JSON runner itself (`cargo build --profile measurement -p
rodas5p-integrators --example lrs_photon_fv_run --locked --offline -j 8`;
command, exit code and log recorded) and drives that binary, the
independent Python references of `reference/lrs_reference.py` and the Rust
contract tests, and writes one JSON with every number, per-gate `{pass,
values, threshold}` and `verdict = PASS` iff every [C] gate passes and the
tree is clean. Thresholds are the frozen constants below. Wall times are
`diagnostic_only`.

Provenance: sha256 of the adapter, runner, test, harness, reference, oracle
and Cargo.lock files and of the built runner binary, and every argument,
are recorded. The verdict is INVALID if `git status --porcelain` shows any
change under `crates/` or this node directory other than the harness's own
outputs (`--out`, the raw directory), at start or at end, or if a recorded
input changed during the run.

Raw compact outputs (request parameters, final states, counters; full
trajectories only for n <= 600) go to `--raw-dir` (default: `raw/` next to
`--out`), which must be absent or empty at start; `MANIFEST.json` there
lists the sha256 of every raw file.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
import math  # noqa: E402
import os  # noqa: E402
import platform  # noqa: E402
import re  # noqa: E402
import shutil  # noqa: E402
import subprocess  # noqa: E402
import tempfile  # noqa: E402
import threading  # noqa: E402
import time  # noqa: E402
import traceback  # noqa: E402
from concurrent.futures import ThreadPoolExecutor  # noqa: E402
from pathlib import Path  # noqa: E402

import numpy as np  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "reference"))
import lrs_reference as R  # noqa: E402

REPO = HERE.parents[1]
EPS = R.EPS

# ---------------------------------------------------------------------------
# Frozen thresholds (fixed before any result; never tuned afterwards)
# ---------------------------------------------------------------------------
TH = {
    "C1": {"min_fixtures": 8, "transport_cells": "bitwise",
           # |L_lo + L_hi - oracle outward| / boundary_flux_scale
           "ledger_rel": 4 * EPS,
           # |L_lo - sum_lo|, |L_hi - sum_hi| / boundary_flux_scale, sums of the independent
           # per-boundary face accounting (OracleOperator._ledger_part rows applied to y)
           "ledger_per_boundary_rel": 4 * EPS,
           "thomson_abs": "4*nm*eps*max|N|", "thomson_abs_factor": 4.0,
           "jvp_wiring_bitwise": "bitwise (wiring check: the JVP callback at states[0] is the RHS of "
                                 "the direction; the operator is linear)",
           "partial_t_rel": 1e-14,
           # PT fixtures, all rows incl. ledgers: max|r - A y| / max|A y|, max|J - A| / max|A|
           "rhs_vs_python_A_rel": 1e-14, "dense_vs_python_A_rel": 1e-14,
           "kbar_rel": 1e-14, "dkbar_rel": 1e-14,
           "closed_kbar_vs_mpmath_rel": 1e-14, "closed_dkbar_vs_mpmath_rel": 1e-14,
           "mpmath_diff_vs_leibniz_rel": 1e-10,
           "kbar_tie_rule": "branch follows the threshold motion (right-sided derivative): rate>0 lo==x "
                            "straddles, rate<0 hi==x straddles; one-sided forward mp.diff at ties",
           "tie_fixtures_require_a_tie": True},
    # Linear-solve roundoff on the stage system W = I/(h gamma) - J makes the invariant drift
    # scale like eps * stiffness * T (factor 1 since the pre-freeze review).
    "C3": {"invariant_rel_base": 1e-11, "stiff_factor": 1.0,
           "bound": "1e-11 + 1*eps*max(gamma, k0)*T per run",
           "scope": "adaptive runs of [C] gates (tags C4_*, C6_*, C7_*); H_* runs report only",
           "status_required": "OK"},
    "C4": {"order_window": [0.8, 1.25], "order_window_pair": "G4->G5 (finest)",
           "monotone": "G1>G2>G3>G4>G5",
           "orders_strictly_increasing": "order(G2->G3) < order(G3->G4) < order(G4->G5)",
           "selfcheck_conservation_rel": 1e-10, "selfcheck_refinement_factor": 1e-3,
           "selfcheck_pde_rel": 1e-5, "selfcheck_ic_quadrature_l1_rel": 1e-13,
           "selfcheck_absorbed_refinement_rel": 1e-10},
    "C5": {"floor_abs_min": 1e-12, "floor_uref_factor": 100.0, "min_pre_floor_pairs": 2,
           "min_last_slope": 4.8, "negative_control_max_last_slope": 3.5},
    "C6": {"strict_decrease": True, "banded_vs_dense_wrms_max": 1.0, "error_ratio_window": [0.5, 2.0],
           "dense_vs_banded_label": "consistency_expected_bit_identical (gated)",
           "fixed_banded_vs_fixed_dense_max_rel": 1e-10, "fixed_steps": 64,
           "fixed_banded_details": "accepted_steps == attempts == 64, rejected_steps == 0",
           "reference_rtol": 1e-13, "reference_uncertainty_over_min_error_max": 0.25,
           "output_times": "runner times == requested outputs exactly, len(states) == len(reference)"},
    "C7": {"m2_abs_factor": 1e-8, "m0_rel": 1e-13,
           "output_times": "runner times == requested outputs exactly, len(states) == len(outputs)"},
    "cargo": {"exit_code": 0, "failed": 0, "ignored": 0, "all_tests_ok": True,
              "binaries": ["lrs_photon_fv_contracts", "partial_t_into_contracts"],
              "gate_tests": {
                  "C2": ["c2_dense_columns_are_unit_rhs_and_the_band_is_exact"],
                  "C8": ["c8_drivers_allocate_independently_of_the_step_count", "c8_callbacks_allocate_nothing",
                         "a_non_autonomous_fast_step_with_analytic_ft_allocates_nothing"],
                  "C9": ["c9_runs_are_deterministic_and_uniform_vector_atol_is_scalar",
                         "fast_runs_match_the_pre_change_golden_digests"],
                  "C10": ["c10_invalid_declarations_and_shapes_are_typed_refusals"]}},
}

# ---------------------------------------------------------------------------
# Scenario constants
# ---------------------------------------------------------------------------
PHYS_DOMAIN = (-4.0, 2.5)
COMO_DOMAIN = (-3.0, 2.0)
GRIDS = {"G1": (16, 8), "G2": (32, 16), "G3": (64, 32), "G4": (128, 64), "G5": (256, 128)}


def geom_of(name):
    if name == "R+":
        return R.Geometry("power_law", hh=0.5, s0=0.2, p=1.5)
    if name == "R-":
        return R.Geometry("power_law", hh=0.5, s0=-0.12, p=1.5)
    if name == "C+":
        return R.Geometry("constant", h=0.5, s=0.2)
    raise KeyError(name)


T_END = {"R+": 3.0, "R-": 3.0, "C+": 2.0}

C4_SCENARIOS = [
    # (id, geometry, absorber (k0, q_chi) | None, coordinate)
    ("R+", "R+", None, "physical"),
    ("R-", "R-", None, "physical"),
    ("C+", "C+", None, "physical"),
    ("R+abs", "R+", (0.5, -0.5), "physical"),
    ("R+abs-com", "R+", (0.5, -0.5), "comoving"),
]


def grid_for(gid, coordinate, boundary="outflow"):
    nq, nm = GRIDS[gid]
    lo, hi = PHYS_DOMAIN if coordinate == "physical" else COMO_DOMAIN
    return R.uniform_grid(lo, hi, nq, nm, coordinate, boundary)


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


def sha_of_list(v):
    return hashlib.sha256(json.dumps([float(a) for a in v]).encode()).hexdigest()


def bits_equal(a, b):
    a = np.ascontiguousarray(np.asarray(a, dtype=np.float64))
    b = np.ascontiguousarray(np.asarray(b, dtype=np.float64))
    if a.shape != b.shape:
        return False, -1, -1
    nb = int(np.count_nonzero(a.view(np.uint64) != b.view(np.uint64)))
    nv = int(np.count_nonzero(a != b))
    return nb == 0, nb, nv


def slope(e1, e2):
    return math.log2(e1 / e2) if e1 > 0 and e2 > 0 else float("nan")


def finite_max(values):
    """max over a list for gates: NaN unless the list is nonempty and every
    entry is finite (so `finite_max(v) <= th` fails on NaN/inf/empty)."""
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
    """x <= th for a finite x only."""
    return x is not None and math.isfinite(float(x)) and float(x) <= th


def aligned(resp, outputs):
    """Status OK, runner times equal to the requested outputs exactly and one
    state per output (checked before any error is computed)."""
    times = resp.get("times") or []
    states = resp.get("states") or []
    times_ok = len(times) == len(outputs) and all(float(a) == float(b) for a, b in zip(times, outputs))
    ok = resp.get("status") == "OK" and times_ok and len(states) == len(outputs)
    return bool(ok), {"status": resp.get("status"), "times_match_requested": bool(times_ok),
                      "n_times": len(times), "n_states": len(states), "n_requested": len(outputs)}


def git_head():
    try:
        return subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"], capture_output=True, text=True,
                              timeout=30).stdout.strip()
    except Exception as e:  # pragma: no cover
        return f"unavailable: {e}"


def git_status_short():
    try:
        return subprocess.run(["git", "-C", str(REPO), "status", "--short"], capture_output=True, text=True,
                              timeout=30).stdout.splitlines()
    except Exception as e:  # pragma: no cover
        return [f"unavailable: {e}"]


WATCHED = ("crates", str(HERE.relative_to(REPO)))


def tree_changes(own_files, own_dirs):
    """`git status --porcelain` lines (untracked files listed one by one)
    under crates/ and this node directory, except the harness's own outputs.
    Returns (changes, error)."""
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


PROVENANCE_FILES = {
    "adapter_mod_rs": "crates/rodas5p-integrators/examples/lrs_photon_fv_support/mod.rs",
    "runner_rs": "crates/rodas5p-integrators/examples/lrs_photon_fv_run.rs",
    "test_lrs_photon_fv_contracts_rs": "crates/rodas5p-integrators/tests/lrs_photon_fv_contracts.rs",
    "test_partial_t_into_contracts_rs": "crates/rodas5p-integrators/tests/partial_t_into_contracts.rs",
    "test_partial_t_into_support_mod_rs": "crates/rodas5p-integrators/tests/partial_t_into_support/mod.rs",
    "run_p02_py": str((HERE / "run_p02.py").relative_to(REPO)),
    "lrs_reference_py": str((HERE / "reference" / "lrs_reference.py").relative_to(REPO)),
    "oracle_py": str(R.ORACLE_PATH.relative_to(REPO)),
    "cargo_lock": "Cargo.lock",
}


def provenance_hashes():
    out = {}
    for k, rel in PROVENANCE_FILES.items():
        p = REPO / rel
        out[k] = {"path": rel, "sha256": R.sha256_file(p) if p.is_file() else None}
    return out


def cargo_env():
    env = dict(os.environ)
    env["RUSTUP_TOOLCHAIN"] = "1.94.1"
    env.setdefault("CARGO_TARGET_DIR", "/home/cosmosapjw/vigilode-bassrei-target")
    env["CARGO_TARGET_DIR"] = str((REPO / env["CARGO_TARGET_DIR"]).resolve())
    return env


def build_runner(timeout_s, raw_dir):
    """Build the runner with the measurement profile; returns (info, binary)."""
    env = cargo_env()
    cmd = ["cargo", "build", "--profile", "measurement", "-p", "rodas5p-integrators", "--example",
           "lrs_photon_fv_run", "--locked", "--offline", "-j", "8"]
    binary = Path(env["CARGO_TARGET_DIR"]) / "measurement" / "examples" / "lrs_photon_fv_run"
    t0 = time.perf_counter()
    try:
        p = subprocess.run(cmd, cwd=str(REPO), env=env, capture_output=True, text=True, timeout=timeout_s)
        rc, log = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        rc = None
        log = "TIMEOUT\n" + ((e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or ""))
    (raw_dir / "cargo_build.log").write_text(log)
    info = {"command": " ".join(cmd), "cwd": str(REPO),
            "env": {"RUSTUP_TOOLCHAIN": env["RUSTUP_TOOLCHAIN"], "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"]},
            "exit_code": rc, "wall_s": time.perf_counter() - t0, "log_file": "raw/cargo_build.log",
            "log_sha256": hashlib.sha256(log.encode()).hexdigest(), "log_tail": log[-2000:],
            "binary": str(binary), "binary_exists": binary.is_file()}
    return info, binary


def write_manifest(raw_dir):
    files = {}
    for p in sorted(raw_dir.rglob("*")):
        if p.is_file() and p.name != "MANIFEST.json":
            files[str(p.relative_to(raw_dir))] = R.sha256_file(p)
    body = {"files": files, "count": len(files), "hash": "sha256"}
    (raw_dir / "MANIFEST.json").write_text(json.dumps(body, indent=1, sort_keys=True) + "\n")
    return {"count": len(files), "manifest_sha256": R.sha256_file(raw_dir / "MANIFEST.json")}


class Log:
    def __init__(self):
        self.t0 = time.perf_counter()
        self.lock = threading.Lock()

    def __call__(self, *a):
        with self.lock:
            print(f"[{time.perf_counter() - self.t0:8.1f}s]", *a, flush=True)

    def elapsed(self):
        return time.perf_counter() - self.t0


LOG = Log()


# ---------------------------------------------------------------------------
# Rust runner
# ---------------------------------------------------------------------------


class Runner:
    def __init__(self, binary, work, raw, jobs, timeout):
        self.binary = str(binary)
        self.work = Path(work)
        self.raw = Path(raw)
        self.pool = ThreadPoolExecutor(max_workers=jobs)
        self.timeout = timeout
        self.lock = threading.Lock()
        self.adaptive = {}  # tag -> response (for C3)
        self.adaptive_req = {}  # tag -> (gamma, k0, T) for the C3 bound
        self.all_runs = {}  # tag -> compact summary
        self.min_cells = {}

    def submit(self, tag, req, mode="timing", timeout=None, cells=None):
        return self.pool.submit(self._run, tag, req, mode, timeout or self.timeout, cells)

    def _run(self, tag, req, mode, timeout, cells):
        path = self.work / f"{tag}.request.json"
        path.write_text(json.dumps(req))
        cmd = [self.binary, "--request", str(path)]
        if mode != "timing":
            cmd += ["--measurement-mode", mode]
        t0 = time.perf_counter()
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
            wall = time.perf_counter() - t0
            try:
                resp = json.loads(proc.stdout)
            except Exception:
                resp = {"status": "ENVIRONMENT_FAILURE",
                        "reason": "unparseable runner stdout; stderr tail: " + proc.stderr[-2000:]}
            resp["_returncode"] = proc.returncode
        except subprocess.TimeoutExpired:
            wall = time.perf_counter() - t0
            resp = {"status": "TIMEOUT", "reason": f"runner exceeded {timeout:.0f}s"}
        resp["_wall_s"] = wall
        self._record(tag, req, resp, cells)
        path.unlink(missing_ok=True)
        return resp

    def _record(self, tag, req, resp, cells):
        n = len(req.get("initial_state", [])) or resp.get("n") or 0
        compact_req = {k: v for k, v in req.items() if k not in ("initial_state", "atol", "states")}
        for k in ("initial_state", "atol"):
            if k in req:
                compact_req[k + "_sha256"] = sha_of_list(req[k])
                compact_req[k + "_len"] = len(req[k])
        if "states" in req:
            compact_req["probe_states_sha256"] = [sha_of_list(s) for s in req["states"]]
        compact_resp = {k: v for k, v in resp.items() if k != "states"}
        states = resp.get("states") or []
        if req.get("mode") == "probe":
            compact_resp = resp
        elif n <= 600:
            compact_resp["states"] = states
        else:
            compact_resp["final_state"] = states[-1] if states else None
            compact_resp["states_omitted"] = f"n={n} > 600: final state only"
        summary = {"tag": tag, "arm": req.get("arm", req.get("mode")), "n": n, "status": resp.get("status"),
                   "reason": (resp.get("reason") or "")[:300], "wall_s": resp.get("_wall_s"),
                   "timing_authority": "diagnostic_only", "counters": resp.get("counters"),
                   "details": resp.get("details"), "allocations": resp.get("allocations")}
        if states and cells is not None:
            arr = np.array([[np.nan if v is None else v for v in s] for s in states], float)
            cm = arr[:, cells]
            summary["min_cell_content"] = float(np.nanmin(cm))
            summary["min_cell_over_max_cell"] = float(np.nanmin(cm) / np.nanmax(np.abs(cm)))
        with self.lock:
            self.all_runs[tag] = summary
            if req.get("mode") != "probe" and req.get("arm") in ("dense", "banded", "mf_identity"):
                self.adaptive[tag] = resp
                ab = req.get("absorber") or {}
                self.adaptive_req[tag] = (float(req.get("gamma", 0.0)), float(ab.get("k0", 0.0)),
                                          float(req["t_span"][1] - req["t_span"][0]))
        (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": compact_resp})))


def integrate_request(grid, geom, gamma, absorber, arm, y0, t_end, outputs, rtol, atol, fixed_steps=None,
                      drop_partial_t=False):
    req = {"layout": grid.layout_request(), "geometry": geom.request(), "gamma": float(gamma),
           "absorber": None if absorber is None else absorber.request(), "drop_partial_t": bool(drop_partial_t),
           "mode": "integrate", "arm": arm, "initial_state": [float(v) for v in y0], "t_span": [0.0, float(t_end)],
           "output_times": [float(t) for t in outputs], "rtol": float(rtol), "atol": [float(v) for v in atol]}
    if fixed_steps is not None:
        req["fixed_steps"] = int(fixed_steps)
    return req


def final_state(resp):
    st = resp.get("states") or []
    if resp.get("status") != "OK" or not st:
        return None
    return np.array(st[-1], float)


# ---------------------------------------------------------------------------
# C1: oracle parity via probe
# ---------------------------------------------------------------------------

C1_GRIDS = {
    "oracle": ([0.0, 0.2, 0.9, 2.0], [-1.0, -0.7, 0.1, 0.8, 1.0]),
    "u16x8": (np.linspace(-4.0, 2.5, 17).tolist(), np.linspace(-1.0, 1.0, 9).tolist()),
    "nonuniform": ([-1.0, -0.6, -0.5, 0.1, 0.15, 0.9, 1.3, 2.0], [-1.0, -0.8, -0.3, 0.05, 0.4, 0.9, 1.0]),
}

C1_TRANSPORT = [
    # (name, grid, coordinate, boundary, h, s)
    ("T01", "oracle", "physical", "outflow", 0.1, 0.07),
    ("T02", "oracle", "physical", "closed", 0.1, 0.07),
    ("T03", "oracle", "physical", "outflow", -0.3, 0.2),
    ("T04", "u16x8", "physical", "outflow", 0.5, 0.2),
    ("T05", "u16x8", "physical", "outflow", 0.5, -0.12),
    ("T06", "u16x8", "physical", "closed", -0.4, 0.3),
    ("T07", "u16x8", "physical", "outflow", 0.0, 0.2),
    ("T08", "nonuniform", "physical", "outflow", 0.2, -0.35),
    ("T09", "nonuniform", "physical", "closed", -0.1, -0.05),
    ("T10", "nonuniform", "physical", "outflow", 0.3, 0.6),
    ("T11", "u16x8", "comoving", "outflow", 0.5, 0.2),
    ("T12", "nonuniform", "comoving", "outflow", -0.2, -0.4),
]

C1_THOMSON = [("TH01", "oracle"), ("TH02", "u16x8"), ("TH03", "nonuniform")]

# Edge q_8 of the u16x8 grid: at t = 0 (ln a(0) = 0 exactly) the comoving threshold sits exactly on it.
C1_TIE_EDGE = C1_GRIDS["u16x8"][0][8]

C1_PARTIAL = [
    # (name, grid, coordinate, boundary, (hh, s0, p), t, gamma, absorber)
    ("PT01", "u16x8", "physical", "outflow", (0.5, 0.2, 1.5), 0.9, 0.5, None),
    ("PT02", "u16x8", "physical", "outflow", (0.5, 0.2, 1.5), 0.9, 0.5, (0.5, -0.5)),
    ("PT03", "u16x8", "comoving", "outflow", (0.5, 0.2, 1.5), 0.7, 0.5, (2.0, -0.5)),
    ("PT04", "nonuniform", "comoving", "closed", (0.4, -0.3, 2.5), 1.3, 0.0, (1.0, 0.2)),
    ("PT05", "oracle", "physical", "outflow", (-0.2, 0.15, 0.5), 0.4, 0.0, (3.0, 0.5)),
    # exact threshold ties at the probe time: rate = H(0) > 0 (lo == x straddles) and < 0 (hi == x)
    ("PT06", "u16x8", "comoving", "outflow", (0.5, 0.2, 1.5), 0.0, 0.5, (2.0, C1_TIE_EDGE)),
    ("PT07", "u16x8", "comoving", "outflow", (-0.4, 0.2, 1.5), 0.0, 0.5, (2.0, C1_TIE_EDGE)),
]
C1_TIE_FIXTURES = ("PT06", "PT07")


def probe_states(grid, rng):
    meas = grid.measure()
    s0 = np.zeros(grid.n)
    s0[grid.cells] = rng.uniform(0.2, 1.8, (grid.nq, grid.nm)) * meas * 1e3
    s0[grid.ledgers] = rng.uniform(0, 5, grid.ledgers.size)
    d1 = rng.standard_normal(grid.n)
    d2 = np.zeros(grid.n)
    d2[grid.cells] = rng.standard_normal((grid.nq, grid.nm)) * meas
    return [s0, d1, d2]


def probe_request(grid, geom, gamma, absorber, t, states):
    return {"mode": "probe", "layout": grid.layout_request(), "geometry": geom.request(), "gamma": float(gamma),
            "absorber": None if absorber is None else absorber.request(), "drop_partial_t": False, "t": float(t),
            "states": [[float(v) for v in s] for s in states]}


def boundary_flux_scale(grid, y, H, s):
    """sum_j |boundary flux_j| (independent; normalizes the ledger check)."""
    if grid.boundary == "closed":
        return 0.0
    tot = 0.0
    for j, c in enumerate(grid.c):
        v = -H - s * c
        i = grid.nq - 1 if v >= 0 else 0
        tot += abs(v * y[grid.cells[i, j]] / grid.dq_list[i])
    return tot


def wedge_split_points(g, H, s):
    """Points sharing the upwind pattern of (H, s) for the L_H / L_s split.
    The pattern-preserving set is an open wedge of directions; the solve uses
    two directions at 1/4 and 3/4 of the wedge (well conditioned) and is
    verified at (H, s) itself and at the wedge midpoint."""
    if H == 0.0:
        return [(0.0, s), (0.0, 0.8 * s), (0.0, 1.15 * s)]
    r = math.hypot(H, s)
    th0 = math.atan2(s, H)
    pat = R.sign_pattern(g, H, s)
    step = 1e-4
    lo = hi = th0
    while lo - step > th0 - math.pi and R.sign_pattern(g, r * math.cos(lo - step), r * math.sin(lo - step)) == pat:
        lo -= step
    while hi + step < th0 + math.pi and R.sign_pattern(g, r * math.cos(hi + step), r * math.sin(hi + step)) == pat:
        hi += step
    if hi - lo < 4 * step:
        return None
    pts = [(H, s)]
    for f in (0.25, 0.75, 0.5):
        th = lo + f * (hi - lo)
        pts.append((r * math.cos(th), r * math.sin(th)))
    # transport_split solves with points[0] and the best-conditioned partner
    # and verifies the rest; put the two wedge points first.
    return [pts[1], pts[2], pts[0], pts[3]]


def boundary_sums(op, g, y, H, s):
    """Independent per-boundary sums: OracleOperator._ledger_part rows (face
    accounting) applied to y, each product rounded once, summed exactly."""
    rows, cols, vals = op._ledger_part(H, s)
    lo_terms = [v * float(y[c]) for r_, c, v in zip(rows, cols, vals) if r_ == g.lo]
    hi_terms = [v * float(y[c]) for r_, c, v in zip(rows, cols, vals) if r_ == g.hi]
    return math.fsum(lo_terms), math.fsum(hi_terms)


def c1_submit(runner, rng):
    """Submit every C1 probe (before the long runs); returns (futures, meta)."""
    futs, meta = [], []
    for name, gk, coord, bnd, h, s in C1_TRANSPORT:
        g = R.Grid(*C1_GRIDS[gk], coordinate=coord, boundary=bnd)
        st = probe_states(g, rng)
        geom = R.Geometry("constant", h=h, s=s)
        futs.append(runner.submit(f"C1_{name}", probe_request(g, geom, 0.0, None, 0.37, st)))
        meta.append(("transport", name, g, geom, st, dict(h=h, s=s)))
    for name, gk in C1_THOMSON:
        g = R.Grid(*C1_GRIDS[gk])
        st = probe_states(g, rng)
        geom = R.Geometry("constant", h=0.0, s=0.0)
        futs.append(runner.submit(f"C1_{name}", probe_request(g, geom, 1.0, None, 0.0, st)))
        meta.append(("thomson", name, g, geom, st, {}))
    for name, gk, coord, bnd, (hh, s0, p), t, gamma, ab in C1_PARTIAL:
        g = R.Grid(*C1_GRIDS[gk], coordinate=coord, boundary=bnd)
        st = probe_states(g, rng)
        geom = R.Geometry("power_law", hh=hh, s0=s0, p=p)
        absorber = None if ab is None else R.Absorber(*ab)
        futs.append(runner.submit(f"C1_{name}", probe_request(g, geom, gamma, absorber, t, st)))
        meta.append(("partial", name, g, geom, st, dict(t=t, gamma=gamma, absorber=absorber)))
    return futs, meta


def rel_vec(a, ref):
    out = []
    for x, y in zip(a, ref):
        x, y = float(x), float(y)
        if not (math.isfinite(x) and math.isfinite(y)):
            out.append(float("nan"))
        elif y == 0:
            out.append(0.0 if x == 0 else float("inf"))
        else:
            out.append(abs(x - y) / abs(y))
    return out


def run_c1(oracle, futs, meta):
    T1 = TH["C1"]
    fixtures = {}
    ok_all = True
    for fut, (kind, name, g, geom, st, extra) in zip(futs, meta):
        resp = fut.result()
        rec = {"kind": kind, "grid": g.describe(), "geometry": geom.request(), "status": resp.get("status")}
        ok = resp.get("status") == "OK" and len(resp.get("rhs") or []) == len(st) \
            and len(resp.get("jvp") or []) == len(st) and len(resp.get("partial_t") or []) == len(st)
        if not ok:
            rec["reason"] = resp.get("reason")
            fixtures[name] = {**rec, "pass": False}
            ok_all = False
            continue
        rhs = [np.array(v, float) for v in resp["rhs"]]
        jvp = [np.array(v, float) for v in resp["jvp"]]
        pt = [np.array(v, float) for v in resp["partial_t"]]
        # Wiring check: the JVP callback at states[0] applied to each direction is the RHS of that
        # direction bit for bit (the operator is linear).
        jb = [bits_equal(j, r) for j, r in zip(jvp, rhs)]
        rec["jvp_wiring_bitwise"] = all(b[0] for b in jb)
        rec["jvp_wiring_bit_mismatches"] = [b[1] for b in jb]
        ok &= rec["jvp_wiring_bitwise"]
        if kind == "transport":
            H = 0.0 if g.coordinate == "comoving" else extra["h"]
            s = extra["s"]
            op = R.OracleOperator(oracle, g)
            per = []
            for y, r in zip(st, rhs):
                o_rhs, outward = op.oracle_rhs(y, H, s)
                be, nb, nv = bits_equal(r[g.cells].ravel(), o_rhs.ravel())
                led = r[g.lo] + r[g.hi]
                scale = boundary_flux_scale(g, y, H, s)
                ref_lo, ref_hi = boundary_sums(op, g, y, H, s)
                if g.boundary == "closed":
                    exact = r[g.lo] == 0 and r[g.hi] == 0 and outward == 0 and ref_lo == 0 and ref_hi == 0
                    lrel = 0.0 if exact else float("inf")
                    lo_rel = hi_rel = lrel
                elif scale > 0:
                    lrel = abs(led - outward) / scale
                    lo_rel = abs(r[g.lo] - ref_lo) / scale
                    hi_rel = abs(r[g.hi] - ref_hi) / scale
                else:
                    lrel = 0.0 if led == outward else float("inf")
                    lo_rel = 0.0 if r[g.lo] == ref_lo else float("inf")
                    hi_rel = 0.0 if r[g.hi] == ref_hi else float("inf")
                p_ok = be and le(lrel, T1["ledger_rel"]) and le(lo_rel, T1["ledger_per_boundary_rel"]) \
                    and le(hi_rel, T1["ledger_per_boundary_rel"]) and bool(np.all(r[g.absorbed] == 0))
                per.append({"cells_bitwise": be, "bit_mismatches": nb, "value_mismatches": nv,
                            "ledger_rel": lrel, "oracle_outward": outward, "L_lo": r[g.lo], "L_hi": r[g.hi],
                            "ref_L_lo_face_sum": ref_lo, "ref_L_hi_face_sum": ref_hi,
                            "ledger_lo_rel": lo_rel, "ledger_hi_rel": hi_rel, "boundary_flux_scale": scale,
                            "absorbed_rows_zero": bool(np.all(r[g.absorbed] == 0)), "pass": bool(p_ok)})
            rec["states"] = per
            rec["oracle_H_used"] = H
            ok &= all(p["pass"] for p in per)
        elif kind == "thomson":
            per = []
            for y, r in zip(st, rhs):
                ref = np.zeros((g.nq, g.nm))
                for i in range(g.nq):
                    ref[i] = oracle.thomson_rhs(g.mu_edges, [float(v) for v in y[g.cells[i]]], 1.0)
                err = finite_max(np.abs(r[g.cells] - ref))
                bound = T1["thomson_abs_factor"] * g.nm * EPS * finite_max(np.abs(y[g.cells]))
                per.append({"max_abs_err": err, "bound": bound, "pass": le(err, bound),
                            "ledger_rows_zero": bool(np.all(r[g.ledgers] == 0))})
            rec["states"] = per
            ok &= all(p["pass"] and p["ledger_rows_zero"] for p in per)
        else:
            t = extra["t"]
            absorber = extra["absorber"]
            H, s = geom.drift(t, g.coordinate)
            pts = wedge_split_points(g, H, s)
            if pts is None:
                rec["error"] = "no pattern-preserving split points"
                fixtures[name] = {**rec, "pass": False}
                ok_all = False
                continue
            model = R.Model(oracle, g, geom, extra["gamma"], absorber, split_points=pts)
            dA = model.dA(t)
            A = model.A(t)
            per = []
            for y, r, p in zip(st, rhs, pt):
                ref = dA @ y
                denom = finite_max(np.abs(ref))
                prel = (float('nan') if not math.isfinite(denom)
                        else finite_max(np.abs(p - ref)) / denom if denom > 0 else finite_max(np.abs(p)))
                ay = A @ y
                ay_max = finite_max(np.abs(ay))
                arel = finite_max(np.abs(r - ay)) / ay_max if math.isfinite(ay_max) and ay_max > 0 else float('nan')
                per.append({"partial_t_rel": prel, "rhs_vs_python_A_rel": arel,
                            "pass": le(prel, T1["partial_t_rel"]) and le(arel, T1["rhs_vs_python_A_rel"])})
            rec["split"] = model.split_info
            rec["states"] = per
            ok &= all(p["pass"] for p in per)
            if resp.get("dense") is not None:
                dense = np.array(resp["dense"], float)
                Ad = A.toarray()
                if dense.shape == Ad.shape:
                    rec["dense_vs_python_A_rel"] = finite_max(np.abs(dense - Ad)) / finite_max(np.abs(Ad))
                else:
                    rec["dense_vs_python_A_rel"] = float("nan")
                ok &= le(rec["dense_vs_python_A_rel"], T1["dense_vs_python_A_rel"])
                bw = int(resp["bandwidth"])
                width = 2 * bw + 1
                band = np.array(resp["band"], float)
                bd = np.zeros_like(dense)
                for rr in range(g.n):
                    for cc in range(max(0, rr - bw), min(g.n, rr + bw + 1)):
                        bd[rr, cc] = band[rr * width + (cc + bw - rr)]
                rec["band_vs_dense_bitwise_info"] = bits_equal(bd, dense)[0]
                rec["dense_outside_band_zero_info"] = bool(np.all(dense[np.abs(np.subtract.outer(np.arange(g.n), np.arange(g.n))) > bw] == 0))
            else:
                rec["dense_vs_python_A_rel"] = "dense matrix missing"
                ok = False
            if absorber is not None:
                kb = np.array(resp["kbar"], float)
                kc, dkc = R.kbar_closed(g, absorber, geom, t)
                km, dkm, ddev, mp_ties = R.kbar_mpmath(g, absorber, geom, t)
                kmf = np.array([float(v) for v in km])
                dkmf = np.array([float(v) for v in dkm])
                xc, rate = R.threshold(absorber, geom, g.coordinate, t)
                cats = R.kbar_categories(g, xc, rate)
                ties = [i for i, (l_, h_) in enumerate(zip(g.qe[:-1], g.qe[1:])) if l_ == xc or h_ == xc]
                rk = rel_vec(kb[:, 0], kmf)
                rdk = rel_vec(kb[:, 1], dkmf)
                ck = finite_max(rel_vec(kc, kmf))
                cdk = finite_max(rel_vec(dkc, dkmf))
                rec["kbar"] = {"threshold_grid_coord": xc, "threshold_rate": rate, "categories": cats,
                               "tie_cells": ties, "tie_cells_mp": mp_ties,
                               "rust_vs_mpmath_rel": rk, "rust_dk_vs_mpmath_rel": rdk,
                               "max_rust_vs_mpmath_rel": finite_max(rk), "max_rust_dk_vs_mpmath_rel": finite_max(rdk),
                               "closed_vs_mpmath_rel": ck, "closed_dk_vs_mpmath_rel": cdk,
                               "mpmath_diff_vs_leibniz_rel": ddev,
                               "has_straddle": "straddle" in cats, "has_above": "above" in cats}
                ok &= le(finite_max(rk), T1["kbar_rel"]) and le(finite_max(rdk), T1["dkbar_rel"])
                ok &= le(ck, T1["closed_kbar_vs_mpmath_rel"]) and le(cdk, T1["closed_dkbar_vs_mpmath_rel"])
                ok &= le(ddev, T1["mpmath_diff_vs_leibniz_rel"])
                ok &= ("straddle" in cats) and ("above" in cats)
                if name in C1_TIE_FIXTURES:
                    rec["kbar"]["tie_required"] = True
                    ok &= len(ties) > 0 and ties == mp_ties
        rec["pass"] = bool(ok)
        ok_all &= bool(ok)
        fixtures[name] = rec
    n_fix = len(fixtures)
    passed = ok_all and n_fix >= T1["min_fixtures"] and n_fix == len(meta)

    def states_of(kind_):
        return [p for f in fixtures.values() if f["kind"] == kind_ for p in f.get("states", [])]
    summary = {
        "fixtures": n_fix,
        "fixtures_passed": sum(1 for f in fixtures.values() if f.get("pass")),
        "max_transport_bit_mismatches": finite_max([p["bit_mismatches"] for p in states_of("transport")]),
        "max_ledger_rel": finite_max([p["ledger_rel"] for p in states_of("transport")]),
        "max_ledger_per_boundary_rel": finite_max([finite_max([p["ledger_lo_rel"], p["ledger_hi_rel"]])
                                                   for p in states_of("transport")]),
        "max_thomson_err_over_bound": finite_max([p["max_abs_err"] / p["bound"] if p["bound"] > 0 else float("nan")
                                                  for p in states_of("thomson")]),
        "max_partial_t_rel": finite_max([p["partial_t_rel"] for p in states_of("partial")]),
        "max_rhs_vs_python_A_rel": finite_max([p["rhs_vs_python_A_rel"] for p in states_of("partial")]),
        "max_dense_vs_python_A_rel": finite_max([f.get("dense_vs_python_A_rel") if isinstance(
            f.get("dense_vs_python_A_rel"), float) else float("nan") for f in fixtures.values()
            if f["kind"] == "partial"]),
        "max_kbar_rel": finite_max([f["kbar"]["max_rust_vs_mpmath_rel"] for f in fixtures.values() if "kbar" in f]),
        "max_dkbar_rel": finite_max([f["kbar"]["max_rust_dk_vs_mpmath_rel"] for f in fixtures.values()
                                     if "kbar" in f]),
        "max_closed_kbar_rel": finite_max([f["kbar"]["closed_vs_mpmath_rel"] for f in fixtures.values()
                                           if "kbar" in f]),
        "max_closed_dkbar_rel": finite_max([f["kbar"]["closed_dk_vs_mpmath_rel"] for f in fixtures.values()
                                            if "kbar" in f]),
        "max_mpmath_diff_vs_leibniz_rel": finite_max([f["kbar"]["mpmath_diff_vs_leibniz_rel"]
                                                      for f in fixtures.values() if "kbar" in f]),
        "tie_fixtures": {k: fixtures[k].get("kbar", {}).get("tie_cells") for k in C1_TIE_FIXTURES if k in fixtures},
        "all_jvp_wiring_bitwise": all(f.get("jvp_wiring_bitwise", False) for f in fixtures.values()),
    }
    return {"pass": bool(passed), "values": {"summary": summary, "fixtures": fixtures}, "threshold": T1}


# ---------------------------------------------------------------------------
# C4: spatial order vs exact characteristics
# ---------------------------------------------------------------------------


def c4_submit(runner, grids):
    futs = {}
    order = sorted(grids, key=lambda g: -GRIDS[g][0])  # largest first
    for gid in order:
        for sid, gname, ab, coord in C4_SCENARIOS:
            g = grid_for(gid, coord)
            y0, tot = R.initial_state(g)
            atol = R.atol_vector(g, 1e-14, 1e-14 * tot)
            absorber = None if ab is None else R.Absorber(*ab)
            req = integrate_request(g, geom_of(gname), 0.0, absorber, "banded", y0, T_END[gname],
                                    [0.0, T_END[gname]], 1e-10, atol)
            futs[(sid, gid)] = runner.submit(f"C4_{sid}_{gid}", req, cells=g.cell_flat)
    return futs


def exact_for(sid, gid, n_sub, n_gl, cache):
    key = (sid, gid, n_sub, n_gl)
    if key not in cache:
        _, gname, ab, coord = next(s for s in C4_SCENARIOS if s[0] == sid)
        g = grid_for(gid, coord)
        ch = R.Characteristics(geom_of(gname), None if ab is None else R.Absorber(*ab))
        t0 = time.perf_counter()
        cache[key] = (ch.cell_integrals(T_END[gname], g, n_sub, n_gl), time.perf_counter() - t0)
    return cache[key]


def c4_selfchecks(sid):
    _, gname, ab, coord = next(s for s in C4_SCENARIOS if s[0] == sid)
    geom = geom_of(gname)
    T = T_END[gname]
    absorber = None if ab is None else R.Absorber(*ab)
    ch = R.Characteristics(geom, absorber)
    out = {}
    if absorber is not None:
        out["monotone_redshift_min"] = ch.require_monotone_redshift(T)
        a32 = ch.absorbed_total(T, n_t=32)
        a48 = ch.absorbed_total(T, n_t=48, n_q_sub=400, n_mu_sub=48)
        out["absorbed_exact"] = a48
        out["absorbed_quadrature_refinement_rel"] = abs(a32 - a48) / abs(a48)
    else:
        a48 = 0.0
    # (1) conservation of the characteristic map on an extended domain
    ext = R.uniform_grid(-10.0, 6.0, 160, 16, coord)
    eT, _ = ch.cell_integrals(T, ext, 4, 12)
    total_exact = R.C_AMP * math.sqrt(R.Q_WIDTH2 * math.pi) * 2.0
    out["extended_total_T"] = float(eT.sum())
    out["total_continuum_exact"] = total_exact
    out["conservation_rel"] = abs(float(eT.sum()) + a48 - total_exact) / total_exact
    # (4) t = 0 quadrature reproduces the exact initial cell integrals
    g1 = grid_for("G2", coord)
    n0, _ = R.initial_cells(g1)
    e0, _ = ch.cell_integrals(0.0, g1, 2, 8)
    out["ic_quadrature_vs_exact_l1_rel"] = R.l1_rel(e0, n0)
    # (3) pointwise PDE residual of the exact solution (central differences, d = 1e-4)
    rng = np.random.default_rng(11)
    lo, hi = (PHYS_DOMAIN if coord == "physical" else COMO_DOMAIN)
    shift = float(geom.ln_a(0.5 * T)) if coord == "comoving" else 0.0
    q = rng.uniform(-2.5, 0.5, 400)
    mu = rng.uniform(-0.98, 0.98, 400)
    t = 0.5 * T
    d = 1e-4

    def U(tt, qq, mm):
        lnR, pref = ch.mu_factors(tt, mm)
        Gf = ch.G_full(tt, mm) if absorber is not None else None
        return ch.U(tt, qq, mm, lnR, pref, Gf)
    u = U(t, q, mu)
    ut = (U(t + d, q, mu) - U(t - d, q, mu)) / (2 * d)
    uq = (U(t, q + d, mu) - U(t, q - d, mu)) / (2 * d)
    Hh, s = float(geom.H(t)), float(geom.S(t))
    vq = -Hh - s * (3 * mu**2 - 1)

    def vm(m):
        return -3 * s * m * (1 - m * m)
    fm = (vm(mu + d) * U(t, q, mu + d) - vm(mu - d) * U(t, q, mu - d)) / (2 * d)
    k = 0.0
    if absorber is not None:
        k = np.where(q >= absorber.q_chi, absorber.k0 * np.exp(-3 * (q - absorber.q_chi)), 0.0)
    res = ut + vq * uq + fm + k * u
    sc = np.abs(ut) + np.abs(vq * uq) + np.abs(fm) + np.abs(k * u)
    m = np.ones_like(q, bool) if absorber is None else (np.abs(q - absorber.q_chi) > 2e-3)
    out["pde_residual_max_rel_sanity"] = finite_max(np.abs(res[m]) / sc[m])
    out["pde_residual_note"] = (f"physical q points, t={t}, central differences d={d}; "
                                f"bound TH['C4']['selfcheck_pde_rel'] = {TH['C4']['selfcheck_pde_rel']:g}")
    del lo, hi, shift
    return out


def run_c4(runner, futs, grids, quick, cache):
    values = {}
    ok = True
    for sid, gname, ab, coord in C4_SCENARIOS:
        if not any((sid, g) in futs for g in grids):
            continue
        sv = {"geometry": geom_of(gname).request(), "absorber": ab, "coordinate": coord, "T": T_END[gname],
              "grids": {}}
        LOG(f"C4 self-checks {sid}")
        sc = c4_selfchecks(sid)
        sv["selfchecks"] = sc
        errs = []
        for gid in grids:
            if (sid, gid) not in futs:
                continue
            g = grid_for(gid, coord)
            (base, binfo), tb = exact_for(sid, gid, 2, 8, cache)
            (ref, rinfo), tr = exact_for(sid, gid, 4, 12, cache)
            resp = futs[(sid, gid)].result()
            y = final_state(resp)
            gv = {"status": resp.get("status"), "wall_s": resp.get("_wall_s"),
                  "quadrature": {"base": "8x8 GL on 2x2 subcells", "ref": "12x12 GL on 4x4 subcells",
                                 "refinement_l1_rel": R.l1_rel(base, ref), "base_s": tb, "ref_s": tr,
                                 "ref_points": rinfo["points"], "kink_corrections": rinfo["kink_corrections"]}}
            if y is None:
                gv["error"] = "run failed"
                errs.append(float("nan"))
            else:
                e = R.l1_rel(y[g.cells], ref)
                gv["l1_rel_error"] = e
                gv["refinement_over_error"] = gv["quadrature"]["refinement_l1_rel"] / e
                y0, tot0 = R.initial_state(g)
                gv["fv_ledgers"] = {"L_lo": y[g.lo], "L_hi": y[g.hi], "absorbed": float(np.sum(y[g.absorbed]))}
                gv["exact_grid_total_T"] = float(ref.sum())
                gv["exact_leakage_plus_absorbed"] = tot0 - float(ref.sum())
                gv["fv_leakage_plus_absorbed"] = float(y[g.lo] + y[g.hi] + np.sum(y[g.absorbed]))
                gv["accepted_steps"] = (resp.get("details") or {}).get("accepted_steps")
                errs.append(e)
            sv["grids"][gid] = gv
        sv["errors"] = errs
        run_grids = [gid for gid in grids if (sid, gid) in futs]
        err_of = dict(zip(run_grids, errs))
        orders = {f"{a}->{b}": slope(err_of[a], err_of[b]) for a, b in zip(run_grids[:-1], run_grids[1:])}
        sv["observed_orders"] = orders
        mono = all(math.isfinite(e) for e in errs) and all(a > b for a, b in zip(errs[:-1], errs[1:]))
        # Strictly increasing orders over the three finest pairs only (G1->G2 is pre-asymptotic).
        rising_pairs = ["G2->G3", "G3->G4", "G4->G5"]
        rising_vals = [orders.get(k, float("nan")) for k in rising_pairs]
        rising = all(math.isfinite(o) for o in rising_vals) and \
            all(a < b for a, b in zip(rising_vals[:-1], rising_vals[1:]))
        if "G4->G5" in orders:
            last_pair = "G4->G5"
        else:
            last_pair = list(orders)[-1] if orders else None  # quick mode: informational only
        last = orders.get(last_pair, float("nan")) if last_pair else float("nan")
        lo_, hi_ = TH["C4"]["order_window"]
        in_window = math.isfinite(last) and lo_ <= last <= hi_
        refine_ok = all(le(sv["grids"][gid].get("refinement_over_error", float("inf")),
                           TH["C4"]["selfcheck_refinement_factor"]) for gid in sv["grids"])
        min_err = finite_min(errs)
        sv["refinement_max_over_min_error_strict"] = finite_max(
            [sv["grids"][gid]["quadrature"]["refinement_l1_rel"] for gid in sv["grids"]]) / min_err
        cons_ok = le(sc["conservation_rel"], TH["C4"]["selfcheck_conservation_rel"])
        pde_ok = le(sc["pde_residual_max_rel_sanity"], TH["C4"]["selfcheck_pde_rel"])
        ic_ok = le(sc["ic_quadrature_vs_exact_l1_rel"], TH["C4"]["selfcheck_ic_quadrature_l1_rel"])
        absorbed_ok = ab is None or le(sc.get("absorbed_quadrature_refinement_rel"),
                                       TH["C4"]["selfcheck_absorbed_refinement_rel"])
        sv["criteria"] = {"monotone": mono, "orders_strictly_increasing": rising,
                          "orders_strictly_increasing_pairs": dict(zip(rising_pairs, rising_vals)),
                          "last_order_pair": last_pair, "last_order": last, "last_order_in_window": in_window,
                          "selfcheck_refinement_ok": refine_ok, "selfcheck_conservation_ok": cons_ok,
                          "selfcheck_pde_ok": pde_ok, "selfcheck_ic_quadrature_ok": ic_ok,
                          "selfcheck_absorbed_refinement_ok": absorbed_ok}
        sv["pass"] = bool(mono and (rising or quick) and in_window and refine_ok and cons_ok and pde_ok
                          and ic_ok and absorbed_ok)
        ok &= sv["pass"]
        values[sid] = sv
    passed = bool(ok) and not quick
    return {"pass": passed, "values": values, "threshold": TH["C4"],
            "note": "quick mode: G1-G2 only, not a gate result" if quick else None}


# ---------------------------------------------------------------------------
# C5: nonstiff fixed-step order (VG-2)
# ---------------------------------------------------------------------------


def c5_configs(quick):
    cfgs = [("C+_g0", "C+", 0.0, False), ("C+_g0.5", "C+", 0.5, False),
            ("R+_g0", "R+", 0.0, False), ("R+_g0.5", "R+", 0.5, False),
            ("NEG_R+_g0_drop_ft", "R+", 0.0, True)]
    if quick:
        cfgs = [c for c in cfgs if c[0] in ("C+_g0", "R+_g0.5", "NEG_R+_g0_drop_ft")]
    Ns = [8, 16, 32, 64] if quick else [8, 16, 32, 64, 128, 256]
    return cfgs, Ns


def c5_submit(runner, quick):
    cfgs, Ns = c5_configs(quick)
    g = grid_for("G2", "physical")
    y0, tot = R.initial_state(g)
    atol = R.atol_vector(g, 1e-6 * 1e-6, 1e-6 * 1e-6 * tot)
    futs = {}
    for cid, gname, gamma, drop in cfgs:
        for N in Ns:
            req = integrate_request(g, geom_of(gname), gamma, None, "fixed_dense", y0, T_END[gname],
                                    [0.0, T_END[gname]], 1e-6, atol, fixed_steps=N, drop_partial_t=drop)
            futs[(cid, N)] = runner.submit(f"C5_{cid}_N{N}", req, cells=g.cell_flat)
    return futs


def c5_references(oracle, quick):
    cfgs, _ = c5_configs(quick)
    g = grid_for("G2", "physical")
    y0, tot = R.initial_state(g)
    atol_ref = R.atol_vector(g, 1e-20, 1e-20 * tot)
    refs = {}
    for gname, gamma in sorted({(c[1], c[2]) for c in cfgs}):
        T = T_END[gname]
        model = R.Model(oracle, g, geom_of(gname), gamma, None, (0.0, T))
        t0 = time.perf_counter()
        dop, dinfo = R.ref_ivp(model, y0, [0.0, T], "DOP853", 2.5e-14, atol_ref)
        info = {"split": model.split_info, "dop853": {"rtol": 2.5e-14, **dinfo}}
        if gname == "C+":
            ex = R.ref_expm(model, y0, [0.0, T])[-1]
            ref = ex
            info["primary"] = "expm_multiply"
            info["u_ref"] = R.max_rel(dop[-1], ex)
            info["u_ref_def"] = "max|DOP853(2.5e-14) - expm| / max|expm|"
        else:
            rad, rinfo = R.ref_ivp(model, y0, [0.0, T], "Radau", 1e-13, atol_ref)
            ref = dop[-1]
            info["primary"] = "DOP853 rtol 2.5e-14"
            info["radau"] = {"rtol": 1e-13, **rinfo}
            info["u_ref"] = R.max_rel(rad[-1], dop[-1])
            info["u_ref_def"] = "max|Radau(1e-13) - DOP853(2.5e-14)| / max|DOP853|"
        info["ref_s"] = time.perf_counter() - t0
        refs[(gname, gamma)] = (ref, info)
        LOG(f"C5 reference {gname} gamma={gamma}: u_ref={info['u_ref']:.3e}")
    return refs


def run_c5(futs, refs, quick):
    cfgs, Ns = c5_configs(quick)
    values = {}
    ok = True
    for cid, gname, gamma, drop in cfgs:
        ref, info = refs[(gname, gamma)]
        floor = finite_max([TH["C5"]["floor_abs_min"], TH["C5"]["floor_uref_factor"] * info["u_ref"]])
        errs, inv = [], []
        for N in Ns:
            resp = futs[(cid, N)].result()
            y = final_state(resp)
            if y is None:
                errs.append(float("nan"))
                inv.append(None)
            else:
                errs.append(R.max_rel(y, ref))
                y0 = np.array(resp["states"][0], float)
                inv.append(abs(y.sum() - y0.sum()) / y0.sum())
        pairs = []
        for k in range(len(Ns) - 1):
            e1, e2 = errs[k], errs[k + 1]
            pre = math.isfinite(e1) and math.isfinite(e2) and e1 > floor and e2 > floor
            pairs.append({"N": [Ns[k], Ns[k + 1]], "slope": slope(e1, e2), "pre_floor": pre})
        pre = [p for p in pairs if p["pre_floor"]]
        last = pre[-1]["slope"] if pre else float("nan")
        if drop:
            p_ok = bool(pre) and last <= TH["C5"]["negative_control_max_last_slope"]
        else:
            p_ok = len(pre) >= TH["C5"]["min_pre_floor_pairs"] and last >= TH["C5"]["min_last_slope"]
        values[cid] = {"geometry": gname, "gamma": gamma, "drop_partial_t": drop, "T": T_END[gname],
                       "N": Ns, "errors": errs, "pairs": pairs, "floor": floor, "u_ref": info["u_ref"],
                       "reference": {k: v for k, v in info.items() if k != "split"},
                       "split_deviation": (info.get("split") or {}).get("max_split_deviation_rel"),
                       "pre_floor_pairs": len(pre), "last_pre_floor_slope": last,
                       "fixed_step_invariant_rel_info": inv, "pass": bool(p_ok)}
        ok &= p_ok
    return {"pass": bool(ok) and not quick, "values": values, "threshold": TH["C5"],
            "note": "quick mode subset" if quick else None}


# ---------------------------------------------------------------------------
# C6: stiff variant
# ---------------------------------------------------------------------------

C6_OUT = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]


def c6_scenarios(quick):
    sc = [("S1_R+_phys_g1e4", "physical", 1e4, None),
          ("S2_R+_phys_g1e6", "physical", 1e6, None),
          ("S3_R+_com_abs1e4_g0.5", "comoving", 0.5, (1e4, -0.5))]
    rt = [1e-4, 1e-6] if quick else [1e-4, 1e-6, 1e-8]
    return (sc[:1] if quick else sc), rt


def c6_setup(sid, coord, gamma, ab, rtol):
    g = grid_for("G2", coord)
    y0, tot = R.initial_state(g)
    atol = R.atol_vector(g, rtol * 1e-6, rtol * 1e-6 * tot)
    return g, y0, tot, atol, (None if ab is None else R.Absorber(*ab))


def c6_submit(runner, quick):
    scs, rts = c6_scenarios(quick)
    futs = {}
    geom = geom_of("R+")
    for sid, coord, gamma, ab in scs:
        for rtol in rts + [1e-12]:
            g, y0, tot, atol, absorber = c6_setup(sid, coord, gamma, ab, rtol)
            arms = ["dense"] if rtol == 1e-12 else ["dense", "banded"]
            for arm in arms:
                req = integrate_request(g, geom, gamma, absorber, arm, y0, 3.0, C6_OUT, rtol, atol)
                futs[(sid, arm, rtol)] = runner.submit(f"C6_{sid}_{arm}_rtol{rtol:.0e}", req, cells=g.cell_flat)
    # fixed-step banded vs dense (N = 64, gamma 1e4 scenario)
    sid, coord, gamma, ab = scs[0]
    g, y0, tot, atol, absorber = c6_setup(sid, coord, gamma, ab, 1e-6)
    n_fixed = TH["C6"]["fixed_steps"]
    for arm in ("fixed_dense", "fixed_banded"):
        req = integrate_request(g, geom, gamma, absorber, arm, y0, 3.0, [0.0, 3.0], 1e-6, atol,
                                fixed_steps=n_fixed)
        futs[(sid, arm, n_fixed)] = runner.submit(f"C6_{sid}_{arm}_N{n_fixed}", req, cells=g.cell_flat)
    return futs


def c6_references(oracle, quick):
    scs, _ = c6_scenarios(quick)
    refs = {}
    for sid, coord, gamma, ab in scs:
        g, y0, tot, atol, absorber = c6_setup(sid, coord, gamma, ab, 1e-12)
        model = R.Model(oracle, g, geom_of("R+"), gamma, absorber, (0.0, 3.0))
        t0 = time.perf_counter()
        rad, info = R.ref_ivp(model, y0, C6_OUT, "Radau", TH["C6"]["reference_rtol"], atol)
        info["ref_s"] = time.perf_counter() - t0
        info["split"] = model.split_info
        refs[sid] = (rad, info)
        LOG(f"C6 reference {sid}: Radau {info['ref_s']:.1f}s nfev={info['nfev']}")
    return refs


def traj_error(states, ref):
    """Per-output relative l1 errors (t > 0) and their NaN-safe max; callers
    check `aligned` first (equal times, one state per reference row)."""
    st = np.array(states, float)
    errs = [float(np.sum(np.abs(a - b)) / np.sum(np.abs(b))) for a, b in zip(st[1:], ref[1:])]
    return finite_max(errs), errs


def run_c6(futs, refs, quick):
    scs, rts = c6_scenarios(quick)
    values = {}
    ok = True
    for sid, coord, gamma, ab in scs:
        ref, rinfo = refs[sid]
        sv = {"coordinate": coord, "gamma": gamma, "absorber": ab, "outputs": C6_OUT,
              "reference": {k: v for k, v in rinfo.items() if k != "split"},
              "split_deviation": (rinfo.get("split") or {}).get("max_split_deviation_rel"), "rungs": {}}
        crit_ok = len(ref) == len(C6_OUT)
        r12 = futs[(sid, "dense", 1e-12)].result()
        a12, al12 = aligned(r12, C6_OUT)
        sv["vigilode_dense_rtol1e-12_alignment"] = al12
        if a12 and len(ref) == len(C6_OUT):
            sv["vigilode_dense_rtol1e-12_vs_radau"] = traj_error(r12["states"], ref)[0]
        else:
            sv["vigilode_dense_rtol1e-12_vs_radau"] = f"run failed or misaligned: {r12.get('status')} " \
                                                      f"{r12.get('reason')} {al12}"
        errs = {"dense": [], "banded": []}
        for rtol in rts:
            g, y0, tot, atol, _ = c6_setup(sid, coord, gamma, ab, rtol)
            rung = {}
            ends = {}
            trajs = {}
            for arm in ("dense", "banded"):
                resp = futs[(sid, arm, rtol)].result()
                good, al = aligned(resp, C6_OUT)
                if not good or len(ref) != len(C6_OUT):
                    rung[arm] = {"status": resp.get("status"), "reason": resp.get("reason"), "alignment": al}
                    errs[arm].append(float("nan"))
                    crit_ok = False
                    continue
                e, per = traj_error(resp["states"], ref)
                errs[arm].append(e)
                ends[arm] = np.array(resp["states"][-1], float)
                trajs[arm] = np.array(resp["states"], float)
                d = resp.get("details") or {}
                rung[arm] = {"error": e, "per_output": per, "alignment": al,
                             "accepted_steps": d.get("accepted_steps"),
                             "rejected_steps": d.get("rejected_steps"), "lu": d.get("lu"),
                             "counters": resp.get("counters"), "wall_s": resp.get("_wall_s")}
            if "dense" in ends and "banded" in ends:
                w = atol + rtol * np.maximum(np.abs(ends["dense"]), np.abs(ends["banded"]))
                wrms = float(np.sqrt(np.mean(((ends["banded"] - ends["dense"]) / w) ** 2)))
                ratio = errs["dense"][-1] / errs["banded"][-1]
                lo, hi = TH["C6"]["error_ratio_window"]
                c_ok = le(wrms, TH["C6"]["banded_vs_dense_wrms_max"]) and math.isfinite(ratio) and lo <= ratio <= hi
                # Both arms solve the same stage systems (dense vs banded LU): labelled
                # consistency_expected_bit_identical, a consistency check (gated), not independent
                # accuracy evidence.
                rung["consistency_expected_bit_identical"] = {
                    "banded_vs_dense_endpoint_wrms": wrms, "dense_over_banded_error": ratio,
                    "trajectories_bit_identical_info": bits_equal(trajs["dense"], trajs["banded"])[0],
                    "pass": bool(c_ok)}
                crit_ok &= c_ok
            else:
                rung["consistency_expected_bit_identical"] = {"pass": False, "error": "an arm failed"}
                crit_ok = False
            sv["rungs"][f"{rtol:.0e}"] = rung
        for arm in ("dense", "banded"):
            e = errs[arm]
            dec = all(math.isfinite(x) for x in e) and all(a > b for a, b in zip(e[:-1], e[1:]))
            sv[f"{arm}_strictly_decreasing"] = dec
            crit_ok &= dec
        sv["errors"] = errs
        uref = sv["vigilode_dense_rtol1e-12_vs_radau"]
        emin = finite_min([x for arm in errs for x in errs[arm]])
        uref_ok = isinstance(uref, float) and le(uref, TH["C6"]["reference_uncertainty_over_min_error_max"] * emin)
        sv["reference_uncertainty"] = {"u_ref": uref, "min_error": emin, "ok": uref_ok,
                                       "def": "traj error of VigilODE dense rtol 1e-12 vs Radau reference"}
        crit_ok &= uref_ok
        sv["pass"] = bool(crit_ok)
        ok &= crit_ok
        values[sid] = sv
    sid = scs[0][0]
    n_fixed = TH["C6"]["fixed_steps"]
    fd = futs[(sid, "fixed_dense", n_fixed)].result()
    fb = futs[(sid, "fixed_banded", n_fixed)].result()
    fd_al_ok, fd_al = aligned(fd, [0.0, 3.0])
    fb_al_ok, fb_al = aligned(fb, [0.0, 3.0])
    fdd, fbd = fd.get("details") or {}, fb.get("details") or {}
    fd_steps_ok = fdd.get("fixed_steps") == n_fixed
    fb_steps_ok = (fbd.get("accepted_steps") == n_fixed and fbd.get("rejected_steps") == 0
                   and fbd.get("attempts") == n_fixed)
    fixed = {"scenario": sid, "N": n_fixed, "status": [fd.get("status"), fb.get("status")],
             "label": "consistency_expected_bit_identical",
             "alignment": {"fixed_dense": fd_al, "fixed_banded": fb_al},
             "fixed_dense_steps_ok": fd_steps_ok, "fixed_banded_steps_ok": fb_steps_ok,
             "fixed_dense_details": fdd, "fixed_banded_details": fbd}
    if fd_al_ok and fb_al_ok:
        yd, yb = np.array(fd["states"][-1], float), np.array(fb["states"][-1], float)
        fixed["endpoint_max_rel_diff"] = R.max_rel(yb, yd)
        fixed["endpoint_max_rel_diff_def"] = "max|y_banded - y_dense| / max|y_dense|"
        fixed["endpoint_bit_identical_info"] = bits_equal(yb, yd)[0]
        fixed["pass"] = bool(fd_steps_ok and fb_steps_ok and
                             le(fixed["endpoint_max_rel_diff"], TH["C6"]["fixed_banded_vs_fixed_dense_max_rel"]))
    else:
        fixed["pass"] = False
    ok &= fixed["pass"]
    values["fixed_banded_vs_fixed_dense"] = fixed
    return {"pass": bool(ok) and not quick, "values": values, "threshold": TH["C6"],
            "note": "quick mode subset" if quick else None}


# ---------------------------------------------------------------------------
# C7: Thomson discrete eigenvalue
# ---------------------------------------------------------------------------

C7_GAMMA = 1e4
C7_OUT = [0.0, 1.0 / C7_GAMMA, 2.0 / C7_GAMMA, 5.0 / C7_GAMMA, 10.0 / C7_GAMMA]


def c7_submit(runner):
    g = grid_for("G2", "physical")
    y0, tot = R.initial_state(g)
    atol = R.atol_vector(g, 1e-20, 1e-20 * tot)
    req = integrate_request(g, R.Geometry("constant", h=0.0, s=0.0), C7_GAMMA, None, "banded", y0, C7_OUT[-1],
                            C7_OUT, 1e-10, atol)
    return runner.submit("C7_thomson_G2", req, cells=g.cell_flat)


def run_c7(fut):
    g = grid_for("G2", "physical")
    resp = fut.result()
    good, al = aligned(resp, C7_OUT)
    if not good:
        return {"pass": False, "values": {"status": resp.get("status"), "reason": resp.get("reason"),
                                          "alignment": al}, "threshold": TH["C7"]}
    st = np.array(resp["states"], float)
    p = g.p2_exact
    lam2 = -C7_GAMMA * (1.0 - float(np.sum(g.dm * p * p)) / 4.0)
    N = st[:, g.cells]  # (K, nq, nm)
    M0 = N.sum(axis=2)
    M2 = (N * p[None, None, :]).sum(axis=2)
    m2max = float(np.max(np.abs(M2[0])))
    dev2 = []
    drift0 = []
    for k, t in enumerate(C7_OUT):
        pred = M2[0] * math.exp(lam2 * t)
        dev2.append(float(np.max(np.abs(M2[k] - pred))))
        drift0.append(float(np.max(np.abs(M0[k] - M0[0]) / np.abs(M0[0]))))
    bound = TH["C7"]["m2_abs_factor"] * m2max
    max_dev2, max_drift0 = finite_max(dev2), finite_max(drift0)
    ok = math.isfinite(bound) and le(max_dev2, bound) and le(max_drift0, TH["C7"]["m0_rel"])
    return {"pass": bool(ok), "values": {"lambda2": lam2, "gamma_t": [t * C7_GAMMA for t in C7_OUT],
                                         "alignment": al, "max_abs_M2_dev": max_dev2,
                                         "max_rel_M0_drift": max_drift0,
                                         "max_abs_M2_dev_per_output": dev2, "M2_bound": bound,
                                         "max_rel_M0_drift_per_output": drift0, "max_abs_M2_0": m2max,
                                         "M2_decay_observed_over_predicted_at_end": float(
                                             np.sum(M2[-1]) / np.sum(M2[0] * math.exp(lam2 * C7_OUT[-1]))),
                                         "details": resp.get("details"), "counters": resp.get("counters")},
            "threshold": {**TH["C7"], "m2_bound_def": "1e-8 * max_i |M2_i(0)| (per group)",
                          "m0_def": "per-group |M0_i(t) - M0_i(0)| / |M0_i(0)|"}}


# ---------------------------------------------------------------------------
# C3: invariant over all adaptive runs
# ---------------------------------------------------------------------------


C3_GATE_PREFIXES = ("C4_", "C6_", "C7_")


def run_c3(runner):
    """Invariant drift of every adaptive run of a [C] gate (status OK
    required); H_* (and any other) adaptive runs are reported only."""
    per, report_only = {}, {}
    ok = True
    for tag, resp in sorted(runner.adaptive.items()):
        st = resp.get("states") or []
        gamma, k0, T = runner.adaptive_req[tag]
        bound = TH["C3"]["invariant_rel_base"] + TH["C3"]["stiff_factor"] * EPS * finite_max([gamma, k0]) * T
        if st:
            arr = np.array([[np.nan if v is None else v for v in s] for s in st], float)
            s0 = float(np.sum(arr[0]))
            d = finite_max(np.abs(arr.sum(axis=1) - s0)) / abs(s0) if s0 != 0 else float("nan")
        else:
            d = float("nan")
        entry = {"status": resp.get("status"), "drift": d, "bound": bound, "outputs": len(st),
                 "gamma": gamma, "k0": k0, "T": T}
        if not tag.startswith(C3_GATE_PREFIXES):
            report_only[tag] = entry
            continue
        entry["pass"] = bool(resp.get("status") == "OK" and le(d, bound))
        per[tag] = entry
        ok &= entry["pass"]
    worst = finite_max([e["drift"] for e in per.values()])
    return {"pass": bool(ok) and len(per) > 0,
            "values": {"runs": len(per), "max_drift": worst, "per_run": per,
                       "report_only": {"runs": len(report_only), "per_run": report_only,
                                       "note": "H_* runs are not part of the C3 gate"}},
            "threshold": TH["C3"]}


# ---------------------------------------------------------------------------
# [H] report-only runs
# ---------------------------------------------------------------------------


def h_submit(runner):
    futs = {}
    g = grid_for("G2", "physical")
    y0, tot = R.initial_state(g)
    rtol = 1e-6
    atol = R.atol_vector(g, rtol * 1e-6, rtol * 1e-6 * tot)
    req = integrate_request(g, geom_of("R+"), 0.5, None, "mf_identity", y0, 3.0, [0.0, 3.0], rtol, atol)
    futs["mf"] = runner.submit("H_mf_identity_R+_G2_g0.5", req, cells=g.cell_flat)
    for arm in ("banded", "dense"):
        req = integrate_request(g, geom_of("R+"), 0.5, None, arm, y0, 3.0, [0.0, 3.0], rtol, atol)
        futs[f"alloc_{arm}"] = runner.submit(f"H_alloc_{arm}_R+_G2_g0.5", req, mode="allocation",
                                             cells=g.cell_flat)
    return futs


def run_h(runner, hfuts, c5refs):
    out = {}
    mf = hfuts["mf"].result()
    rec = {"status": mf.get("status"), "reason": (mf.get("reason") or "")[:300], "counters": mf.get("counters"),
           "details": mf.get("details"), "wall_s": mf.get("_wall_s"), "timing_authority": "diagnostic_only"}
    y = final_state(mf)
    if y is not None and ("R+", 0.5) in c5refs:
        rec["endpoint_max_rel_error_vs_dop853"] = R.max_rel(y, c5refs[("R+", 0.5)][0])
    out["mf_identity_R+_G2_g0.5_rtol1e-6"] = rec
    for arm in ("banded", "dense"):
        r = hfuts[f"alloc_{arm}"].result()
        out[f"allocation_mode_{arm}"] = {"status": r.get("status"), "allocations": r.get("allocations"),
                                         "note": "runner counts the whole second integrate call incl. problem "
                                                 "build and output vectors (not a hot-loop count; VG-1 lives in "
                                                 "the Rust contract tests)"}
    mins = {t: s.get("min_cell_content") for t, s in runner.all_runs.items() if s.get("min_cell_content") is not None}
    if mins:
        tmin = min(mins, key=lambda k: mins[k])
        out["positivity"] = {"global_min_cell_content": mins[tmin], "at_run": tmin,
                             "runs_with_negative_cells": sorted(t for t, v in mins.items() if v < 0),
                             "min_cell_over_max_cell_per_run": {t: s.get("min_cell_over_max_cell")
                                                                for t, s in runner.all_runs.items()
                                                                if s.get("min_cell_over_max_cell") is not None}}
    out["wall_times_s"] = {t: s.get("wall_s") for t, s in runner.all_runs.items()}
    out["timing_authority"] = "diagnostic_only"
    out["all_counters"] = {t: {"counters": s.get("counters"), "details": s.get("details")}
                           for t, s in runner.all_runs.items() if s.get("arm") != "probe"}
    return out


# ---------------------------------------------------------------------------
# Rust contract tests (C2, C8, C9, C10)
# ---------------------------------------------------------------------------


def run_cargo(timeout_s, raw_dir):
    """Rust contract tests of both binaries, sequential (deterministic
    output order); per-test 'test NAME ... RESULT' lines are parsed."""
    env = cargo_env()
    binaries = TH["cargo"]["binaries"]
    cmd = ["cargo", "test", "-p", "rodas5p-integrators", "--release", "--locked", "--offline", "-j", "8"]
    for b in binaries:
        cmd += ["--test", b]
    cmd += ["--", "--test-threads=1"]
    t0 = time.perf_counter()
    try:
        p = subprocess.run(cmd, cwd=str(REPO), env=env, capture_output=True, text=True, timeout=timeout_s)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        rc = None
        out = (e.stdout or b"").decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
        err = "TIMEOUT\n" + ((e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or ""))
    wall = time.perf_counter() - t0
    (raw_dir / "cargo_test.log").write_text("=== stdout ===\n" + out + "\n=== stderr ===\n" + err)
    lines = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", out)
    passed = sum(int(m[1]) for m in lines)
    failed = sum(int(m[2]) for m in lines)
    ignored = sum(int(m[3]) for m in lines)
    names = re.findall(r"^test (\S+) \.\.\. (\S+)\s*$", out, flags=re.M)
    tests = {}
    duplicates = []
    for a, b in names:
        if a in tests:
            duplicates.append(a)
        tests[a] = b
    running = re.findall(r"Running tests/(\w+)\.rs", err)
    common_ok = (rc == 0 and failed == 0 and ignored == 0 and passed > 0 and len(lines) == len(binaries)
                 and sorted(running) == sorted(binaries) and len(names) == passed and not duplicates
                 and all(r == "ok" for r in tests.values()))
    gate_ok = {}
    gate_missing = {}
    for gid, expected in TH["cargo"]["gate_tests"].items():
        missing = [n for n in expected if tests.get(n) != "ok"]
        gate_missing[gid] = missing
        gate_ok[gid] = bool(common_ok and not missing)
    info = {"command": " ".join(cmd), "cwd": str(REPO),
            "env": {"RUSTUP_TOOLCHAIN": env["RUSTUP_TOOLCHAIN"], "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"]},
            "exit_code": rc, "passed": passed, "failed": failed, "ignored": ignored,
            "result_lines": len(lines), "binaries_run": running, "duplicate_test_names": duplicates,
            "all_tests_ok": bool(common_ok), "tests": [{"name": a, "result": b} for a, b in names],
            "gate_tests_missing_or_not_ok": gate_missing, "log_file": "raw/cargo_test.log",
            "wall_s": wall, "stderr_tail": err[-3000:] if not common_ok else err[-600:]}
    return info, gate_ok


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bin", default=None,
                    help="optional; must equal the runner the harness builds (the harness always builds it)")
    ap.add_argument("--out", required=True, help="results JSON")
    ap.add_argument("--quick", action="store_true", help="development smoke subset (verdict QUICK_SMOKE_ONLY)")
    ap.add_argument("--raw-dir", default=None,
                    help="raw output directory, absent or empty at start (default: raw/ next to --out)")
    ap.add_argument("--jobs", type=int, default=6, help="parallel runner processes")
    ap.add_argument("--skip-cargo", action="store_true", help="do not run the Rust contract tests (gates fail)")
    ap.add_argument("--cargo", action="store_true", help="run the Rust contract tests also in --quick")
    ap.add_argument("--run-timeout", type=float, default=3600.0, help="per runner call timeout (s)")
    ap.add_argument("--cargo-timeout", type=float, default=3600.0, help="cargo build / cargo test timeout (s)")
    args = ap.parse_args()

    out_path = Path(args.out).resolve()
    raw_dir = Path(args.raw_dir).resolve() if args.raw_dir else out_path.parent / "raw"
    if raw_dir.exists() and (not raw_dir.is_dir() or any(raw_dir.iterdir())):
        sys.exit(f"refusing: raw directory {raw_dir} exists and is not empty")
    quick = args.quick
    own_files, own_dirs = [out_path], [raw_dir]
    changes_start, git_err_start = tree_changes(own_files, own_dirs)
    hashes_start = provenance_hashes()
    raw_dir.mkdir(parents=True, exist_ok=True)

    results = {
        "node": "BASS/rei P02 fixed-grid LRS photon transport adapter",
        "mode": "quick" if quick else "full",
        "timing_authority": "diagnostic_only",
    }
    LOG("building the runner (measurement profile)")
    build, binary = build_runner(args.cargo_timeout, raw_dir)
    LOG(f"build exit {build['exit_code']} -> {binary}")
    refusal = None
    if build["exit_code"] != 0 or not binary.is_file():
        refusal = "runner build failed"
    elif args.bin is not None and Path(args.bin).resolve() != binary.resolve():
        refusal = f"--bin {Path(args.bin).resolve()} is not the built runner {binary.resolve()}"
    oracle = R.load_oracle()
    results["inputs"] = {
        "repo": str(REPO), "git_head": git_head(), "git_status_short": git_status_short(),
        "args": {k: (str(v) if isinstance(v, Path) else v) for k, v in vars(args).items()},
        "argv": sys.argv,
        "out": str(out_path), "raw_dir": str(raw_dir),
        "build": build,
        "runner_binary": str(binary), "runner_sha256": R.sha256_file(binary) if binary.is_file() else None,
        "files_sha256": hashes_start,
        "oracle_path": str(R.ORACLE_PATH.relative_to(REPO)), "oracle_sha256": hashes_start["oracle_py"]["sha256"],
        "reference_sha256": hashes_start["lrs_reference_py"]["sha256"],
        "run_p02_sha256": hashes_start["run_p02_py"]["sha256"],
        "python": sys.version.split()[0], "numpy": np.__version__,
        "scipy": __import__("scipy").__version__, "mpmath": R.mp.__version__,
        "platform": platform.platform(), "jobs": args.jobs,
    }
    results["thresholds"] = TH
    results["scenario_constants"] = {"C": R.C_AMP, "U0": "C exp(-(q+0.5)^2/0.18) (1 + 0.5 P2(mu))",
                                     "physical_domain": PHYS_DOMAIN, "comoving_domain": COMO_DOMAIN,
                                     "grids": GRIDS, "geometries": {k: geom_of(k).request()
                                                                    for k in ("R+", "R-", "C+")}, "T": T_END}
    gates = {}
    if refusal is None:
        work = Path(tempfile.mkdtemp(prefix="p02_requests_"))
        runner = Runner(binary, work, raw_dir, args.jobs, args.run_timeout)
        try:
            rng = np.random.default_rng(20261021)
            c4_grids = ["G1", "G2"] if quick else ["G1", "G2", "G3", "G4", "G5"]
            LOG("submitting runner calls (C1 probes first)")
            c1f, c1meta = c1_submit(runner, rng)
            c4f = c4_submit(runner, c4_grids)
            c6f = c6_submit(runner, quick)
            c5f = c5_submit(runner, quick)
            c7f = c7_submit(runner)
            hf = h_submit(runner)
            LOG("C1 probes")
            gates["C1"] = run_c1(oracle, c1f, c1meta)
            LOG(f"C1 pass={gates['C1']['pass']}")
            geo = {k: R.geometry_self_check(geom_of(k), T_END[k]) for k in ("R+", "R-", "C+")}
            results["reference_selfchecks"] = {"geometry_closed_forms": geo}
            LOG("C5 references")
            c5refs = c5_references(oracle, quick)
            LOG("C6 references")
            c6refs = c6_references(oracle, quick)
            LOG("C4 exact quadratures")
            cache = {}
            for gid in c4_grids:
                for sid, *_ in C4_SCENARIOS:
                    exact_for(sid, gid, 2, 8, cache)
                    exact_for(sid, gid, 4, 12, cache)
                LOG(f"  exact {gid} done")
            gates["C4"] = run_c4(runner, c4f, c4_grids, quick, cache)
            LOG(f"C4 pass={gates['C4']['pass']}")
            gates["C5"] = run_c5(c5f, c5refs, quick)
            LOG(f"C5 pass={gates['C5']['pass']}")
            gates["C6"] = run_c6(c6f, c6refs, quick)
            LOG(f"C6 pass={gates['C6']['pass']}")
            gates["C7"] = run_c7(c7f)
            LOG(f"C7 pass={gates['C7']['pass']}")
            results["H"] = run_h(runner, hf, c5refs)
            gates["C3"] = run_c3(runner)
            LOG(f"C3 pass={gates['C3']['pass']}")
        except Exception:
            results["harness_error"] = traceback.format_exc()
            LOG("HARNESS ERROR\n" + results["harness_error"])
        finally:
            runner.pool.shutdown(wait=True)
            shutil.rmtree(work, ignore_errors=True)

        cargo_gate = {}
        if quick and not args.cargo:
            cargo = {"pass": False, "skipped": "quick mode (pass --cargo to run)"}
        elif args.skip_cargo:
            cargo = {"pass": False, "skipped": "--skip-cargo"}
        else:
            LOG("cargo contract tests")
            cargo, cargo_gate = run_cargo(args.cargo_timeout, raw_dir)
            LOG(f"cargo all_tests_ok={cargo['all_tests_ok']} passed={cargo.get('passed')} "
                f"failed={cargo.get('failed')} ignored={cargo.get('ignored')}")
        results["cargo"] = cargo
        for gid in ("C2", "C8", "C9", "C10"):
            gates[gid] = {"pass": bool(cargo_gate.get(gid, False)),
                          "values": {"source": "Rust contract tests "
                                               f"({' + '.join(TH['cargo']['binaries'])}), see results['cargo']",
                                     "expected_tests": TH["cargo"]["gate_tests"][gid],
                                     "missing_or_not_ok": (cargo.get("gate_tests_missing_or_not_ok") or {}).get(gid),
                                     "all_tests_ok": cargo.get("all_tests_ok")},
                          "threshold": TH["cargo"]}
    else:
        results["refused"] = refusal
        LOG(f"REFUSED: {refusal}")

    order = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10"]
    gates = {k: gates.get(k, {"pass": False, "values": "not run", "threshold": None}) for k in order}
    results["gates"] = gates
    results["gate_table"] = {k: bool(v["pass"]) for k, v in gates.items()}

    changes_end, git_err_end = tree_changes(own_files, own_dirs)
    hashes_end = provenance_hashes()
    runner_sha_end = R.sha256_file(binary) if binary.is_file() else None
    changed_inputs = [k for k in hashes_start if hashes_start[k] != hashes_end[k]]
    if runner_sha_end != results["inputs"]["runner_sha256"]:
        changed_inputs.append("runner_binary")
    clean = (git_err_start is None and git_err_end is None and not changes_start and not changes_end
             and not changed_inputs)
    results["provenance"] = {
        "watched": list(WATCHED), "excluded_own_outputs": [str(out_path), str(raw_dir) + "/"],
        "tree_changes_start": changes_start, "tree_changes_end": changes_end,
        "git_error": [e for e in (git_err_start, git_err_end) if e],
        "inputs_changed_during_run": changed_inputs, "clean": bool(clean),
    }
    all_pass = all(v["pass"] for v in gates.values()) and "harness_error" not in results and refusal is None
    if not clean or refusal is not None:
        results["verdict"] = "INVALID"
        results["verdict_reason"] = refusal or "tree not clean or inputs changed (see provenance)"
    else:
        results["verdict"] = "QUICK_SMOKE_ONLY" if quick else ("PASS" if all_pass else "FAIL")
    results["runtime_s"] = LOG.elapsed()
    results["raw_dir"] = str(raw_dir)
    results["raw_manifest"] = write_manifest(raw_dir)
    out_path.write_text(json.dumps(sanitize(results), indent=1) + "\n")
    LOG(f"verdict {results['verdict']} gates {results['gate_table']} -> {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
