#!/usr/bin/env python3
"""Gate runner of BASS/rei DAG node P03 (structured stage solvers of the LRS
photon transport adapter: energy-block Thomson low-rank exact solve, Krylov
energy sweep for a mixed drift, bordered Schur complement), run as clients
of the RODAS5P fast driver (library step L2).

    python3 run_p03.py --out results.json [--quick] [--skip-cargo]

Builds the runner (`cargo build --profile measurement -p rodas5p-integrators
--example lrs_photon_block_run --locked --offline -j 8`), drives it, the
Python references (P02 `lrs_reference.py`, this node's `p03_reference.py`,
the oracle) and the Rust contract tests, and writes one JSON with every
number, per-gate `{pass, values, threshold}` and `verdict = PASS` iff every
[C] gate passes and the tree is clean. Thresholds are the frozen constants
in TH. Wall times are `diagnostic_only`.

Provenance and INVALID rules as P02b: sha256 of every input and of the
built binary; INVALID if `git status --porcelain` shows a change under
`crates/` or this node directory (other than the harness's own outputs) at
start or end, or if a recorded input changed during the run. `raw/` must be
absent or empty at start; `raw/MANIFEST.json` lists the sha256 of every raw
file.
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
import p03_reference as P  # noqa: E402

R = P.R
REPO = HERE.parents[1]

TH = {
    "C1": {"scenarios": ["S1", "S2", "B1"], "rtol": 1e-6, "sample_every": {"S1": 6, "S2": 6, "B1": 5},
           "sample_max": 100, "min_samples": 50, "backward_error_max": 1e-14,
           "componentwise_backward_error_max": 1e-14, "forward_vs_lu_max": 1e-11,
           "photon_block_forward_vs_lu_max": 1e-11, "border_component_forward_vs_lu_max": 1e-11,
           "componentwise_def": "max_i |b - W x|_i / (|W| |x| + |b|)_i (Oettli-Prager)",
           "border_def": "per border unknown |x - x_LU| / |x_LU| (B1: s, x_e, D_m)",
           "sampling": "every k-th stage solve over the whole run (k from the pilot solve counts)",
           "backward_error_def": "||b - W x||_inf / (||W||_inf ||x||_inf + ||b||_inf), W = inv I - J dense",
           "forward_def": "||x_client - x_LU||_inf / ||x_LU||_inf, x_LU from a dense LU of the same W"},
    "C2": {"rtols": [1e-4, 1e-6, 1e-8], "wrms_max": 1.0, "error_ratio_window": [0.5, 2.0],
           "strict_decrease": True, "reference_rtol": {"S": 1e-14, "B": 1e-13},
           "reference_rtol_effective_note": "scipy raises rtol 1e-14 to 2.220446049250313e-14",
           "reference_uncertainty_rtol": {"S": 1e-13, "B": 1e-12},
           "reference_uncertainty_over_min_error_max": 0.25,
           "wrms_def": "max over outputs of sqrt(mean(((y_client - y_ref_arm)/(atol + rtol |y_ref_arm|))^2))",
           "error_def": "max over outputs t>0 of relative l1 vs the Radau reference",
           "ref_arm": {"S": "banded", "B": "bordered_dense"}},
    "C3": {"scenarios": ["K1", "K2"], "rtol": 1e-6,
           "exact_refusal": "status REFUSED, reason contains STRUCTURED_EXACT_MIXED_DIRECTION",
           "krylov": {"restart": 4, "max_iterations": 48, "rtol": 1e-12, "two_level": True},
           "max_iterations_per_solve": 48, "max_iterations_note": "max_arnoldi = 48: a longer solve is an "
                                                                 "Err counted in linear_solve_failures",
           "linear_solve_failures": 0, "sampled_relative_residual_max": 1e-12,
           "sampled_componentwise_backward_error_max": 1e-12, "sampled_forward_vs_lu_max": 1e-11,
           "min_samples": 50, "wrms_vs_banded_max": 1.0, "sample_every": {"K1": 35, "K2": 3}, "sample_max": 100},
    "C5": {"grid_nq": [8, 16, 32, 64, 128], "grid_nm": [2, 4, 8, 16, 32, 64], "formula_equality": "exact",
           "cost_compare_nq": [8, 32, 128], "cost_compare_nm": [8, 16, 32, 64],
           "cost_def": "one fixed step (one factorization + its stage solves): structured factor+solve "
                       "flops vs the banded driver's factor_operations+solve_operations",
           "block_below_banded": True},
    "C6": {"pairs": {"exact": "S1", "krylov": "K2", "bordered": "B1"}, "rtols": [1e-6, 1e-9],
           "attempt_ratio_min": 2.0, "allocations": "equal at both rtols (allocation mode, positive control)",
           "linear_solve_failures": "equal at both rtols"},
    "C7": {"repeat_bitwise": "states, counters and client op counts", "arms": ["S1 client_exact",
                                                                           "K2 client_krylov",
                                                                           "B1 bordered_client"]},
    "C8": {"rhs_parity_componentwise": 1e-14, "jacobian_parity_per_entry": 1e-14,
           "partial_t_parity_componentwise": 1e-14, "d_gamma_parity_rel": 1e-14, "oracle_transport_cells": "bitwise",
           "oracle_outward_rel": 1e-15, "riccati_closed_form_rel": 1e-14,
           "riccati_closed_form_note": "consistency of two copies of one formula (not independent)",
           "riccati_integrated_rel": 1e-9, "riccati_rtol": 1e-10,
           "rhs_parity_def": "max_k |f_rust,k - f_py,k| / (sum of |terms| of component k)",
           "jacobian_parity_def": "max_rc |J_rust - J_py| / max(|J_rust|, |J_py|, S_rc) (0/0 = 0): pattern and every "
                                  "entry; S = sum of |terms| in the s and x_e columns (cancelling sums), else 0",
           "partial_t_parity_def": "max_k |ft_rust,k - ft_py,k| / (sum of |terms| of component k of f_t)"},
    "cargo": {"binaries": ["lrs_block_solver_contracts", "client_stage_system_contracts",
                           "lrs_photon_fv_contracts", "partial_t_into_contracts", "rodas5p_fast_allocations",
                           "generic_scope_contracts"],
              "gate_tests": {
                  "C1": ["c1_exact_solve_matches_a_dense_lu_and_is_backward_stable"],
                  "C3": ["c3_mixed_direction_is_refused_by_the_exact_mode_and_krylov_converges"],
                  "C4": ["c4_stale_states_and_shapes_are_typed_refusals", "invalid_client_inputs_are_refused",
                         "retryable_client_errors_are_rejections_with_their_work_kept"],
                  "C5": ["c5_counted_flops_equal_the_closed_forms"],
                  "C6": ["c6_solver_calls_allocate_nothing",
                         "c6_client_runs_allocate_independently_of_the_step_count",
                         "a_client_step_allocates_nothing"],
                  "C7": ["c7_client_runs_are_deterministic_and_agree_with_the_banded_driver",
                         "banded_delegate_reproduces_the_banded_driver_bitwise",
                         "fast_runs_match_the_pre_change_golden_digests",
                         "generic_solver_core_does_not_embed_physical_client_bindings"],
                  "C8": ["c8_bordered_jacobian_schur_solve_and_riccati_law"]}},
}

PHYS = (-4.0, 2.5)
COMO = (-3.0, 2.0)
OUT6 = [3.0 * k / 6 for k in range(7)]
OUTX = [2.0 * k / 6 for k in range(7)]
BORDER = {"alpha": 2.0, "beta": 0.5, "kappa": 1.0, "tau": 0.1, "gamma0": 1e4, "k_times_dgamma0": 0.02,
          "y0_border": [0.15, 0.1, 0.0], "grid": [32, 16], "absorber": [0.5, -0.5]}


def geom(name):
    if name == "R+":
        return R.Geometry("power_law", hh=0.5, s0=0.2, p=1.5)
    if name == "X":
        return R.Geometry("constant", h=0.1, s=0.2)
    raise KeyError(name)


# scenario id -> (coordinate, geometry, gamma, absorber, T, outputs); grid G2 (32 x 16)
SCEN = {
    "S1": ("physical", "R+", 1e4, None, 3.0, OUT6),
    "S2": ("physical", "R+", 1e6, None, 3.0, OUT6),
    "K1": ("comoving", "R+", 0.5, (1e4, -0.5), 3.0, OUT6),
    "K2": ("physical", "X", 1e4, None, 2.0, OUTX),
}


def grid_of(coord, nq=32, nm=16):
    lo, hi = PHYS if coord == "physical" else COMO
    return R.uniform_grid(lo, hi, nq, nm, coord, "outflow")


def setup(sid, rtol):
    coord, gname, gamma, ab, T, outs = SCEN[sid]
    g = grid_of(coord)
    y0, tot = R.initial_state(g)
    atol = R.atol_vector(g, rtol * 1e-6, rtol * 1e-6 * tot)
    return g, y0, atol, (None if ab is None else R.Absorber(*ab))


def request(sid, arm, rtol, **extra):
    coord, gname, gamma, ab, T, outs = SCEN[sid]
    g, y0, atol, absorber = setup(sid, rtol)
    req = {"layout": g.layout_request(), "geometry": geom(gname).request(), "gamma": float(gamma),
           "absorber": None if absorber is None else absorber.request(), "mode": "integrate", "arm": arm,
           "initial_state": [float(v) for v in y0], "t_span": [0.0, T], "output_times": outs, "rtol": rtol,
           "atol": [float(v) for v in atol]}
    req.update(extra)
    return req


def bordered_model(grid=None):
    nq, nm = grid or BORDER["grid"]
    g = grid_of("physical", nq, nm)
    y0p, tot = R.initial_state(g)
    ab = R.Absorber(*BORDER["absorber"])
    args = (BORDER["alpha"], BORDER["beta"], BORDER["kappa"], BORDER["tau"], BORDER["gamma0"], ab)
    d0 = P.Bordered(g, geom("R+"), 1.0, *args).d_gamma(np.concatenate([y0p, [0.0, 0.0, 0.0]]))
    k = BORDER["k_times_dgamma0"] / d0
    m = P.Bordered(g, geom("R+"), k, *args)
    y0 = np.concatenate([y0p, BORDER["y0_border"]])
    return m, g, y0, tot, ab, k, d0


def border_atol(g, tot, rtol):
    s = rtol * 1e-6
    return np.concatenate([R.atol_vector(g, s, s * tot), [s * 0.15, s, s * 0.01]])


def border_block(ab, k):
    return {"gamma": 0.0, "absorber": ab.request(), "geometry": geom("R+").request(),
            "border": {"k": k, "alpha": BORDER["alpha"], "beta": BORDER["beta"], "kappa": BORDER["kappa"],
                       "tau": BORDER["tau"], "gamma0": BORDER["gamma0"]}}


def border_request(arm, rtol, **extra):
    m, g, y0, tot, ab, k, _ = bordered_model()
    req = {"layout": g.layout_request(), **border_block(ab, k), "mode": "integrate", "arm": arm,
           "initial_state": [float(v) for v in y0], "t_span": [0.0, 3.0], "output_times": OUT6, "rtol": rtol,
           "atol": [float(v) for v in border_atol(g, tot, rtol)]}
    req.update(extra)
    return req


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


def aligned(resp, outputs):
    times = resp.get("times") or []
    states = resp.get("states") or []
    times_ok = len(times) == len(outputs) and all(float(a) == float(b) for a, b in zip(times, outputs))
    ok = resp.get("status") == "OK" and times_ok and len(states) == len(outputs)
    return bool(ok), {"status": resp.get("status"), "reason": (resp.get("reason") or "")[:200],
                      "times_match_requested": bool(times_ok), "n_states": len(states)}


def traj_error(states, ref):
    st = np.array(states, float)
    errs = [float(np.sum(np.abs(a - b)) / np.sum(np.abs(b))) for a, b in zip(st[1:], ref[1:])]
    return finite_max(errs), errs


def wrms_max(states, ref_states, atol, rtol):
    vals = []
    for x, y in zip(np.array(states, float), np.array(ref_states, float)):
        w = atol + rtol * np.abs(y)
        vals.append(float(np.sqrt(np.mean(((x - y) / w) ** 2))))
    return finite_max(vals)


def bits_equal(a, b):
    a = np.ascontiguousarray(np.asarray(a, dtype=np.float64))
    b = np.ascontiguousarray(np.asarray(b, dtype=np.float64))
    return a.shape == b.shape and bool(np.all(a.view(np.uint64) == b.view(np.uint64)))


def sha256_file(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


WATCHED = ("crates", str(HERE.relative_to(REPO)))


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
PROVENANCE_FILES = {
    "adapter_mod_rs": f"{SUPPORT}/mod.rs",
    "block_solver_rs": f"{SUPPORT}/block_solver.rs",
    "border_rs": f"{SUPPORT}/border.rs",
    "runner_rs": "crates/rodas5p-integrators/examples/lrs_photon_block_run.rs",
    "client_stage_rs": "crates/rodas5p-integrators/src/client_stage.rs",
    "rodas5p_fast_rs": "crates/rodas5p-integrators/src/rodas5p_fast.rs",
    "test_lrs_block_solver_contracts_rs": "crates/rodas5p-integrators/tests/lrs_block_solver_contracts.rs",
    "test_client_stage_system_contracts_rs": "crates/rodas5p-integrators/tests/client_stage_system_contracts.rs",
    "run_p03_py": str((HERE / "run_p03.py").relative_to(REPO)),
    "p03_reference_py": str((HERE / "reference" / "p03_reference.py").relative_to(REPO)),
    "lrs_reference_py": "research/bassrei_p02_fixed_grid_20261021/reference/lrs_reference.py",
    "oracle_py": "research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py",
    "cargo_lock": "Cargo.lock",
}


def crate_sources_digest():
    """sha256 over the sorted (path, sha256) list of every crates/**/*.rs and Cargo.toml file."""
    rows = []
    for p in sorted((REPO / "crates").rglob("*")):
        if p.is_file() and (p.suffix == ".rs" or p.name == "Cargo.toml"):
            rows.append(f"{p.relative_to(REPO)} {sha256_file(p)}")
    rows.append(f"Cargo.toml {sha256_file(REPO / 'Cargo.toml')}")
    return {"files": len(rows), "sha256": hashlib.sha256("\n".join(rows).encode()).hexdigest()}


def git_info():
    def run(args):
        try:
            return subprocess.run(["git", "-C", str(REPO), *args], capture_output=True, text=True,
                                  timeout=60).stdout.strip()
        except Exception as e:  # pragma: no cover
            return f"unavailable: {e}"
    return {"head": run(["rev-parse", "HEAD"]), "status_short": run(["status", "--short"]).splitlines()}


def provenance_hashes():
    out = {}
    for k, rel in PROVENANCE_FILES.items():
        p = REPO / rel
        out[k] = {"path": rel, "sha256": sha256_file(p) if p.is_file() else None}
    out["crate_sources"] = crate_sources_digest()
    return out


def cargo_env():
    env = dict(os.environ)
    env["RUSTUP_TOOLCHAIN"] = "1.94.1"
    env.setdefault("CARGO_TARGET_DIR", "/home/cosmosapjw/vigilode-bassrei-target")
    env["CARGO_TARGET_DIR"] = str((REPO / env["CARGO_TARGET_DIR"]).resolve())
    return env


def build_runner(timeout_s, raw_dir):
    env = cargo_env()
    cmd = ["cargo", "build", "--profile", "measurement", "-p", "rodas5p-integrators", "--example",
           "lrs_photon_block_run", "--locked", "--offline", "-j", "8"]
    binary = Path(env["CARGO_TARGET_DIR"]) / "measurement" / "examples" / "lrs_photon_block_run"
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

    def submit(self, tag, req, mode="timing"):
        return self.pool.submit(self._run, tag, req, mode)

    def _run(self, tag, req, mode):
        path = self.work / f"{tag}.request.json"
        path.write_text(json.dumps(req))
        cmd = [self.binary, "--request", str(path)]
        if mode != "timing":
            cmd += ["--measurement-mode", mode]
        t0 = time.perf_counter()
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=self.timeout)
            try:
                resp = json.loads(proc.stdout)
            except Exception:
                resp = {"status": "ENVIRONMENT_FAILURE", "reason": "unparseable stdout; stderr: " + proc.stderr[-2000:]}
            resp["_returncode"] = proc.returncode
        except subprocess.TimeoutExpired:
            resp = {"status": "TIMEOUT", "reason": f"runner exceeded {self.timeout:.0f}s"}
        resp["_wall_s"] = time.perf_counter() - t0
        compact_req = {k: v for k, v in req.items() if k not in ("initial_state", "atol", "states")}
        for k in ("initial_state", "atol"):
            if k in req:
                compact_req[k + "_sha256"] = sha_of_list(req[k])
        if "states" in req:
            compact_req["probe_states_sha256"] = [sha_of_list(s) for s in req["states"]]
        compact_resp = {k: v for k, v in resp.items() if k != "dense"}
        summary = {"tag": tag, "arm": req.get("arm", req.get("mode")), "status": resp.get("status"),
                   "reason": (resp.get("reason") or "")[:300], "wall_s": resp["_wall_s"],
                   "timing_authority": "diagnostic_only", "counters": resp.get("counters"),
                   "details": resp.get("details"), "client_ops": resp.get("client_ops"),
                   "allocations": resp.get("allocations"),
                   "runner_elapsed_ns": (resp.get("timing") or {}).get("elapsed_ns")}
        with self.lock:
            self.all_runs[tag] = summary
        (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": compact_resp})))
        path.unlink(missing_ok=True)
        return resp


# ---------------------------------------------------------------------------
# Gates
# ---------------------------------------------------------------------------


def samples_of(resp):
    return resp.get("samples") or []


def c1_submit(runner):
    th = TH["C1"]
    def sample(sid):
        return {"every": th["sample_every"][sid], "max": th["sample_max"], "condition": False}
    futs = {sid: runner.submit(f"C1_{sid}_client_exact", request(sid, "client_exact", th["rtol"], sample=sample(sid)))
            for sid in ("S1", "S2")}
    futs["B1"] = runner.submit("C1_B1_bordered_client",
                               border_request("bordered_client", th["rtol"], sample=sample("B1")))
    futs["H_S2_textbook"] = runner.submit("H_C1_S2_client_textbook",
                                          request("S2", "client_textbook", th["rtol"], sample=sample("S2")))
    return futs


def run_c1(futs):
    th = TH["C1"]
    values, ok = {}, True
    for sid in th["scenarios"]:
        r = futs[sid].result()
        s = samples_of(r)
        eta = finite_max([x["backward_error_client"] for x in s])
        omega = finite_max([x["componentwise_backward_error_client"] for x in s])
        fwd = finite_max([x["forward_difference_vs_lu"] for x in s])
        pfwd = finite_max([x["photon_block_forward_vs_lu"] for x in s])
        bfwd = finite_max([v for x in s for v in x["border_forward_vs_lu"]]) if sid == "B1" else 0.0
        sok = bool(r.get("status") == "OK" and len(s) >= th["min_samples"] and le(eta, th["backward_error_max"])
                   and le(omega, th["componentwise_backward_error_max"]) and le(fwd, th["forward_vs_lu_max"])
                   and le(pfwd, th["photon_block_forward_vs_lu_max"])
                   and le(bfwd, th["border_component_forward_vs_lu_max"]))
        values[sid] = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200], "samples": len(s),
                       "t_range": [finite_min([x["t"] for x in s]), finite_max([x["t"] for x in s])],
                       "inv_range": [finite_min([x["inv"] for x in s]), finite_max([x["inv"] for x in s])],
                       "max_backward_error": eta, "max_componentwise_backward_error": omega,
                       "max_forward_vs_lu": fwd, "max_photon_block_forward_vs_lu": pfwd,
                       "max_border_component_forward_vs_lu": bfwd,
                       "max_backward_error_lu": finite_max([x["backward_error_lu"] for x in s]),
                       "max_componentwise_backward_error_lu": finite_max(
                           [x["componentwise_backward_error_lu"] for x in s]),
                       "direction": r.get("direction"), "pass": sok}
        ok &= sok
    h = futs["H_S2_textbook"].result()
    hs = samples_of(h)
    values["H_S2_textbook_capacitance"] = {
        "status": h.get("status"), "samples": len(hs),
        "max_backward_error": finite_max([x["backward_error_client"] for x in hs]),
        "max_forward_vs_lu": finite_max([x["forward_difference_vs_lu"] for x in hs]),
        "max_componentwise_backward_error": finite_max([x["componentwise_backward_error_client"] for x in hs]),
        "report_only": True}
    return {"pass": bool(ok), "values": values, "threshold": th}


def c2_submit(runner, quick):
    rtols = TH["C2"]["rtols"][:2] if quick else TH["C2"]["rtols"]
    futs = {}
    for sid in ("S1", "S2"):
        for rtol in rtols:
            for arm in ("banded", "client_exact"):
                futs[(sid, arm, rtol)] = runner.submit(f"C2_{sid}_{arm}_rtol{rtol:.0e}", request(sid, arm, rtol))
    for rtol in rtols:
        for arm in ("bordered_dense", "bordered_client"):
            futs[("B1", arm, rtol)] = runner.submit(f"C2_B1_{arm}_rtol{rtol:.0e}", border_request(arm, rtol))
    return futs, rtols


def c2_references(oracle):
    th = TH["C2"]
    refs = {}
    for sid in ("S1", "S2"):
        coord, gname, gamma, ab, T, outs = SCEN[sid]
        g, y0, atol, absorber = setup(sid, 1e-12)
        model = R.Model(oracle, g, geom(gname), gamma, absorber, (0.0, T))
        t0 = time.perf_counter()
        ref, info = R.ref_ivp(model, y0, outs, "Radau", th["reference_rtol"]["S"], atol)
        ref2, _ = R.ref_ivp(model, y0, outs, "Radau", th["reference_uncertainty_rtol"]["S"], atol)
        info["u_ref"] = traj_error(ref2, ref)[0]
        info["ref_s"] = time.perf_counter() - t0
        refs[sid] = (ref, info)
        LOG(f"C2 reference {sid}: {info['ref_s']:.1f}s u_ref={info['u_ref']:.2e}")
    m, g, y0, tot, ab, k, d0 = bordered_model()
    atol = border_atol(g, tot, 1e-12)
    t0 = time.perf_counter()
    ref, info = P.ref_radau(m, y0, OUT6, th["reference_rtol"]["B"], atol)
    ref2, _ = P.ref_radau(m, y0, OUT6, th["reference_uncertainty_rtol"]["B"], atol)
    info.update({"u_ref": traj_error(ref2, ref)[0], "ref_s": time.perf_counter() - t0, "k": k, "d_gamma0": d0})
    refs["B1"] = (ref, info)
    LOG(f"C2 reference B1: {info['ref_s']:.1f}s u_ref={info['u_ref']:.2e}")
    return refs


def run_c2(futs, rtols, refs, quick):
    th = TH["C2"]
    values, ok = {}, True
    for sid in ("S1", "S2", "B1"):
        ref, rinfo = refs[sid]
        kind = "B" if sid == "B1" else "S"
        ref_arm = th["ref_arm"][kind]
        client_arm = "bordered_client" if sid == "B1" else "client_exact"
        sv = {"reference": rinfo, "rungs": {}}
        errs = {ref_arm: [], client_arm: []}
        sok = True
        for rtol in rtols:
            rung = {}
            resp = {a: futs[(sid, a, rtol)].result() for a in (ref_arm, client_arm)}
            good = {a: aligned(resp[a], OUT6) for a in resp}
            if not all(g_[0] for g_ in good.values()):
                rung["alignment"] = {a: good[a][1] for a in good}
                sok = False
                for a in errs:
                    errs[a].append(float("nan"))
                sv["rungs"][f"{rtol:.0e}"] = rung
                continue
            for a in (ref_arm, client_arm):
                e, per = traj_error(resp[a]["states"], ref)
                errs[a].append(e)
                d = resp[a].get("details") or {}
                rung[a] = {"error": e, "per_output": per, "attempts": d.get("attempts"),
                           "accepted_steps": d.get("accepted_steps"), "rejected_steps": d.get("rejected_steps"),
                           "lu": d.get("lu"), "driver": d.get("driver"), "wall_s": resp[a].get("_wall_s")}
            if sid == "B1":
                _, g, _, tot, _, _, _ = bordered_model()
                atol = border_atol(g, tot, rtol)
            else:
                _, _, atol, _ = setup(sid, rtol)
            w = wrms_max(resp[client_arm]["states"], resp[ref_arm]["states"], atol, rtol)
            ratio = errs[client_arm][-1] / errs[ref_arm][-1] if errs[ref_arm][-1] > 0 else float("nan")
            lo, hi = th["error_ratio_window"]
            r_ok = bool(le(w, th["wrms_max"]) and math.isfinite(ratio) and lo <= ratio <= hi)
            rung["client_vs_ref_arm"] = {"wrms_max": w, "error_ratio": ratio, "pass": r_ok}
            sok &= r_ok
            sv["rungs"][f"{rtol:.0e}"] = rung
        for a in errs:
            e = errs[a]
            dec = all(math.isfinite(x) for x in e) and all(x > y for x, y in zip(e[:-1], e[1:]))
            sv[f"{a}_strictly_decreasing"] = bool(dec)
            sok &= dec
        emin = finite_min([x for a in errs for x in errs[a]])
        uref = rinfo.get("u_ref")
        u_ok = bool(le(uref, th["reference_uncertainty_over_min_error_max"] * emin))
        sv["reference_uncertainty"] = {"u_ref": uref, "min_error": emin, "ok": u_ok}
        sok &= u_ok
        sv["errors"] = errs
        sv["pass"] = bool(sok)
        values[sid] = sv
        ok &= sok
    return {"pass": bool(ok) and not quick, "values": values, "threshold": th}


def c3_submit(runner):
    th = TH["C3"]
    futs = {}
    for sid in th["scenarios"]:
        sample = {"every": th["sample_every"][sid], "max": th["sample_max"], "condition": False}
        futs[(sid, "exact")] = runner.submit(f"C3_{sid}_client_exact", request(sid, "client_exact", th["rtol"]))
        futs[(sid, "krylov")] = runner.submit(f"C3_{sid}_client_krylov", request(
            sid, "client_krylov", th["rtol"], krylov=th["krylov"], sample=sample))
        futs[(sid, "banded")] = runner.submit(f"C3_{sid}_banded", request(sid, "banded", th["rtol"]))
        futs[(sid, "one_level")] = runner.submit(f"H_C3_{sid}_client_krylov_one_level", request(
            sid, "client_krylov", th["rtol"], krylov=dict(th["krylov"], two_level=False)))
    return futs


def run_c3(futs):
    th = TH["C3"]
    values, ok = {}, True
    for sid in th["scenarios"]:
        outs = SCEN[sid][5]
        ex, kr, bd, one = (futs[(sid, a)].result() for a in ("exact", "krylov", "banded", "one_level"))
        refused = ex.get("status") == "REFUSED" and "STRUCTURED_EXACT_MIXED_DIRECTION" in (ex.get("reason") or "")
        kal, kinfo = aligned(kr, outs)
        bal, binfo = aligned(bd, outs)
        s = samples_of(kr)
        rel = finite_max([x["relative_residual_client"] for x in s])
        omega = finite_max([x["componentwise_backward_error_client"] for x in s])
        fwd = finite_max([x["forward_difference_vs_lu"] for x in s])
        ops = kr.get("client_ops") or {}
        lsf = (kr.get("counters") or {}).get("linear_solve_failures")
        _, _, atol, _ = setup(sid, th["rtol"])
        w = wrms_max(kr["states"], bd["states"], atol, th["rtol"]) if (kal and bal) else float("nan")
        max_it = ops.get("max_krylov_iterations")
        sok = bool(refused and kal and bal and kr.get("direction") == "Mixed"
                   and lsf == th["linear_solve_failures"] and isinstance(max_it, int)
                   and max_it <= th["max_iterations_per_solve"] and len(s) >= th["min_samples"]
                   and le(rel, th["sampled_relative_residual_max"])
                   and le(omega, th["sampled_componentwise_backward_error_max"])
                   and le(fwd, th["sampled_forward_vs_lu_max"]) and le(w, th["wrms_vs_banded_max"]))
        values[sid] = {"exact_status": ex.get("status"), "exact_reason": (ex.get("reason") or "")[:200],
                       "exact_refused_typed": bool(refused), "krylov_alignment": kinfo, "banded_alignment": binfo,
                       "direction": kr.get("direction"), "linear_solve_failures": lsf, "client_ops": ops,
                       "mean_iterations_per_solve": ops.get("krylov_iterations", 0) / max(ops.get("solves", 1), 1),
                       "samples": len(s), "t_range": [finite_min([x["t"] for x in s]), finite_max([x["t"] for x in s])],
                       "max_sampled_relative_residual": rel, "max_sampled_componentwise_backward_error": omega,
                       "max_sampled_forward_vs_lu": fwd, "wrms_vs_banded": w,
                       "H_one_level": {"status": one.get("status"), "client_ops": one.get("client_ops"),
                                       "linear_solve_failures": (one.get("counters") or {}).get("linear_solve_failures"),
                                       "report_only": True},
                       "pass": sok}
        ok &= sok
    return {"pass": bool(ok), "values": values, "threshold": th}


def c5_submit(runner):
    th = TH["C5"]
    cases = [{"nq": nq, "nm": nm} for nq in th["grid_nq"] for nm in th["grid_nm"]]
    model = {"geometry": {"kind": "constant", "h": 0.5, "s": 0.2}, "gamma": 1e4,
             "absorber": {"k0": 0.5, "q_chi": -0.5}}
    futs = {"opcount": runner.submit("C5_opcount", {"mode": "opcount", "q_range": list(PHYS), "inv": 10.0,
                                                    "cases": cases, **model})}
    for nq in th["grid_nq"]:
        for nm in th["grid_nm"]:
            g = R.uniform_grid(PHYS[0], PHYS[1], nq, nm, "physical", "outflow")
            y0, tot = R.initial_state(g)
            atol = R.atol_vector(g, 1e-12, 1e-12 * tot)
            for arm in ("fixed_banded", "fixed_client"):
                req = {"layout": g.layout_request(), **model, "mode": "integrate", "arm": arm,
                       "initial_state": [float(v) for v in y0], "t_span": [0.0, 0.05], "output_times": [0.0, 0.05],
                       "rtol": 1e-6, "atol": [float(v) for v in atol], "fixed_steps": 1}
                futs[(nq, nm, arm)] = runner.submit(f"C5_{arm}_{nq}x{nm}", req)
    return futs


def run_c5(futs):
    th = TH["C5"]
    oc = futs["opcount"].result()
    cases = oc.get("cases") or []
    eq = bool(oc.get("status") == "OK" and len(cases) == len(th["grid_nq"]) * len(th["grid_nm"])
              and all(c["factor_counted"] == c["factor_formula"] and c["solve_counted"] == c["solve_formula"]
                      and c["direction"] == "Down" for c in cases))
    cost, cmp_ok = [], True
    for nq in th["grid_nq"]:
        for nm in th["grid_nm"]:
            b = futs[(nq, nm, "fixed_banded")].result()
            c = futs[(nq, nm, "fixed_client")].result()
            gated = nq in th["cost_compare_nq"] and nm in th["cost_compare_nm"]
            row = {"nq": nq, "nm": nm, "status": [b.get("status"), c.get("status")], "gated": gated}
            if b.get("status") == "OK" and c.get("status") == "OK":
                bd, co = b.get("details") or {}, c.get("client_ops") or {}
                band = bd.get("band_factor_operations", 0) + bd.get("band_solve_operations", 0)
                blk = co.get("factor", 0) + co.get("solve", 0)
                steps_ok = bd.get("accepted_steps") == 1 and (c.get("details") or {}).get("accepted_steps") == 1
                row.update({"banded_flops": band, "block_flops": blk, "ratio_block_over_banded": blk / band,
                            "banded_solves": (b.get("counters") or {}).get("direct_solve_calls"),
                            "block_solves": co.get("solves"), "one_step_each": steps_ok})
                if gated:
                    cmp_ok &= bool(blk < band and steps_ok)
            elif gated:
                cmp_ok = False
            cost.append(row)
    return {"pass": bool(eq and cmp_ok), "values": {"formula_equality": eq, "cases": cases, "cost_map": cost,
                                                    "cost_compare_pass": bool(cmp_ok)}, "threshold": th}


def c6_submit(runner):
    th = TH["C6"]
    futs = {}
    for name, sid in th["pairs"].items():
        for rtol in th["rtols"]:
            if sid == "B1":
                req = border_request("bordered_client", rtol)
            elif name == "krylov":
                req = request(sid, "client_krylov", rtol, krylov=TH["C3"]["krylov"])
            else:
                req = request(sid, "client_exact", rtol)
            futs[(name, rtol)] = runner.submit(f"C6_{name}_rtol{rtol:.0e}_alloc", req, mode="allocation")
    return futs


def run_c6(futs):
    th = TH["C6"]
    values, ok = {}, True
    for name in th["pairs"]:
        r1, r2 = (futs[(name, rt)].result() for rt in th["rtols"])
        a1, a2 = (r.get("allocations") or {} for r in (r1, r2))
        at1, at2 = ((r.get("details") or {}).get("attempts") or 0 for r in (r1, r2))
        lsf = [(r.get("counters") or {}).get("linear_solve_failures") for r in (r1, r2)]
        sok = bool(r1.get("status") == "OK" and r2.get("status") == "OK" and a1.get("calibrated")
                   and a2.get("calibrated") and a1.get("integrate") == a2.get("integrate")
                   and at2 >= th["attempt_ratio_min"] * max(at1, 1) and lsf[0] == lsf[1])
        values[name] = {"allocations": [a1.get("integrate"), a2.get("integrate")], "attempts": [at1, at2],
                        "positive_control": [a1.get("positive_control"), a2.get("positive_control")],
                        "linear_solve_failures": lsf, "status": [r1.get("status"), r2.get("status")], "pass": sok}
        ok &= sok
    return {"pass": bool(ok), "values": values, "threshold": th}


def c7_submit(runner):
    arms = {"S1_exact": lambda: request("S1", "client_exact", 1e-6),
            "K2_krylov": lambda: request("K2", "client_krylov", 1e-6, krylov=TH["C3"]["krylov"]),
            "B1_bordered": lambda: border_request("bordered_client", 1e-6)}
    return {(name, k): runner.submit(f"C7_{name}_repeat{k}", make()) for name, make in arms.items() for k in (0, 1)}


def run_c7(futs):
    vals, ok = {}, True
    for name in ("S1_exact", "K2_krylov", "B1_bordered"):
        a, b = futs[(name, 0)].result(), futs[(name, 1)].result()
        same = bool(a.get("status") == "OK" and b.get("status") == "OK" and bits_equal(a["states"], b["states"])
                    and json.dumps(a.get("counters"), sort_keys=True) == json.dumps(b.get("counters"), sort_keys=True)
                    and json.dumps(a.get("client_ops"), sort_keys=True) == json.dumps(b.get("client_ops"),
                                                                                       sort_keys=True))
        vals[name] = same
        ok &= same
    return {"pass": bool(ok), "values": {"repeat_bitwise_states_counters_ops": vals}, "threshold": TH["C7"]}


def c8_submit(runner):
    m, g, y0, tot, ab, k, d0 = bordered_model((16, 8))
    rng = np.random.default_rng(20261022)
    states = [y0] + [y0 * (1 + 0.2 * rng.standard_normal(len(y0))) for _ in range(3)]
    states[3][m.IS] = -0.1  # the other angular upwind direction
    futs = {}
    for t in (0.0, 0.7):
        req = {"layout": g.layout_request(), **border_block(ab, k), "mode": "probe", "t": t,
               "states": [[float(v) for v in s] for s in states],
               "riccati": {"x0": BORDER["y0_border"][1], "times": OUT6}}
        futs[("probe", t)] = runner.submit(f"C8_probe_t{t}", req)
    futs["riccati_run"] = runner.submit("C8_B1_bordered_client_riccati",
                                        border_request("bordered_client", TH["C8"]["riccati_rtol"]))
    return futs, (m, states)


def run_c8(futs, ctx, oracle):
    th = TH["C8"]
    m, states = ctx
    x0 = BORDER["y0_border"][1]
    values, ok = {"probes": {}}, True
    for t in (0.0, 0.7):
        r = futs[("probe", t)].result()
        pv = {"status": r.get("status")}
        if r.get("status") != "OK":
            values["probes"][str(t)] = pv
            ok = False
            continue
        rhs_par, jac_par, ft_par, dg_par = [], [], [], []
        for s, rr, ft, dj, dg in zip(states, r["rhs"], r["partial_t"], r["dense"], r["d_gamma"]):
            f = m.rhs(t, s)
            rhs_par.append(float(np.max(np.abs(np.array(rr) - f) / np.maximum(m.term_scale(t, s), 1e-300))))
            J = m.jacobian(t, s).toarray()
            Jr = np.array(dj, float)
            den = np.maximum(np.maximum(np.abs(Jr), np.abs(J)), m.jacobian_scale(t, s))
            num = np.abs(Jr - J)
            jac_par.append(float(np.max(np.where(den > 0, num / np.where(den > 0, den, 1.0), 0.0))))
            fp = m.partial_t(t, s)
            ft_par.append(float(np.max(np.abs(np.array(ft) - fp) / np.maximum(m.partial_t_scale(t, s), 1e-300))))
            dg_par.append(abs(dg - m.d_gamma(s)) / max(abs(m.d_gamma(s)), 1e-300))
        ric = [abs(a - m.riccati(x0, tt)) / abs(m.riccati(x0, tt)) for a, tt in zip(r["riccati"], OUT6)]
        pv.update({"rhs_parity": finite_max(rhs_par), "jacobian_parity": finite_max(jac_par),
                   "partial_t_parity": finite_max(ft_par), "d_gamma_parity": finite_max(dg_par),
                   "riccati_closed_form_rel": finite_max(ric)})
        pok = bool(le(pv["rhs_parity"], th["rhs_parity_componentwise"])
                   and le(pv["jacobian_parity"], th["jacobian_parity_per_entry"])
                   and le(pv["partial_t_parity"], th["partial_t_parity_componentwise"])
                   and le(pv["d_gamma_parity"], th["d_gamma_parity_rel"])
                   and le(pv["riccati_closed_form_rel"], th["riccati_closed_form_rel"]))
        pv["pass"] = pok
        values["probes"][str(t)] = pv
        ok &= pok
    op = R.OracleOperator(oracle, m.g)
    worst, outward = 0, 0.0
    for s in states:
        for h, sh in ((0.5, s[m.IS]), (0.3, -0.12)):
            orc, out_o = op.oracle_rhs(s[: m.nph], h, sh)
            cells, lo, hi = m.transport(h, sh, h, sh, s[m.g.cells])
            worst = max(worst, int(np.count_nonzero(orc.view(np.uint64) != cells.view(np.uint64))))
            outward = max(outward, abs((lo + hi) - out_o) / max(abs(out_o), 1e-300))
    values["python_transport_vs_oracle_bit_mismatches"] = worst
    values["python_outward_vs_oracle_rel"] = outward
    ok &= worst == 0 and le(outward, th["oracle_outward_rel"])
    rr = futs["riccati_run"].result()
    al, ainfo = aligned(rr, OUT6)
    mb = bordered_model()[0]  # the integration's own (default-grid) model
    rel = (finite_max([abs(st[mb.IX] - mb.riccati(x0, tt)) / mb.riccati(x0, tt) for st, tt in zip(rr["states"], OUT6)])
           if al else float("nan"))
    values["riccati_integrated"] = {"alignment": ainfo, "max_rel": rel, "rtol": th["riccati_rtol"]}
    ok &= le(rel, th["riccati_integrated_rel"])
    return {"pass": bool(ok), "values": values, "threshold": th}


def run_cargo(timeout_s, raw_dir):
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
        rc, out, err = None, str(e.stdout or ""), "TIMEOUT\n" + str(e.stderr or "")
    (raw_dir / "cargo_test.log").write_text("=== stdout ===\n" + out + "\n=== stderr ===\n" + err)
    lines = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", out)
    passed = sum(int(m[1]) for m in lines)
    failed = sum(int(m[2]) for m in lines)
    ignored = sum(int(m[3]) for m in lines)
    names = re.findall(r"^test (\S+) \.\.\. (\S+)\s*$", out, flags=re.M)
    tests, dup = {}, []
    for a, b in names:
        if a in tests:
            dup.append(a)
        tests[a] = b
    running = re.findall(r"Running tests/(\w+)\.rs", err)
    common_ok = bool(rc == 0 and failed == 0 and ignored == 0 and passed > 0 and len(lines) == len(binaries)
                     and sorted(running) == sorted(binaries) and len(names) == passed and not dup
                     and all(r == "ok" for r in tests.values()))
    gate_ok, missing = {}, {}
    for gid, expected in TH["cargo"]["gate_tests"].items():
        miss = [n for n in expected if tests.get(n) != "ok"]
        missing[gid] = miss
        gate_ok[gid] = bool(common_ok and not miss)
    return {"command": " ".join(cmd), "exit_code": rc, "passed": passed, "failed": failed, "ignored": ignored,
            "binaries_run": running, "duplicate_test_names": dup, "all_tests_ok": common_ok,
            "tests": [{"name": a, "result": b} for a, b in names], "gate_tests_missing_or_not_ok": missing,
            "log_file": "raw/cargo_test.log", "wall_s": time.perf_counter() - t0,
            "stderr_tail": err[-2000:] if not common_ok else err[-400:]}, gate_ok


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True)
    ap.add_argument("--raw-dir", default=None)
    ap.add_argument("--jobs", type=int, default=6)
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
    results = {"node": "BASS/rei P03", "mode": "quick" if args.quick else "full",
               "timing_authority": "diagnostic_only", "thresholds": TH, "scenario_constants":
                   {"SCEN": SCEN, "BORDER": BORDER, "PHYS": PHYS, "COMO": COMO}, "build": build,
               "host": {"platform": platform.platform(), "python": platform.python_version(),
                        "numpy": np.__version__, "scipy": __import__("scipy").__version__,
                        "cpu_count": os.cpu_count()}}
    gates = {}
    if build["exit_code"] != 0 or not binary.is_file():
        results["verdict"] = "INVALID"
        results["reason"] = "runner build failed"
    else:
        oracle = R.load_oracle()
        with tempfile.TemporaryDirectory() as work:
            runner = Runner(binary, work, raw, args.jobs, args.timeout)
            c1f = c1_submit(runner)
            c2f, rtols = c2_submit(runner, args.quick)
            c3f = c3_submit(runner)
            c5f = c5_submit(runner)
            c6f = c6_submit(runner)
            c7f = c7_submit(runner)
            c8f, c8ctx = c8_submit(runner)
            refs = c2_references(oracle)
            for gid, fn in (("C1", lambda: run_c1(c1f)), ("C2", lambda: run_c2(c2f, rtols, refs, args.quick)),
                            ("C3", lambda: run_c3(c3f)), ("C5", lambda: run_c5(c5f)), ("C6", lambda: run_c6(c6f)),
                            ("C7", lambda: run_c7(c7f)), ("C8", lambda: run_c8(c8f, c8ctx, oracle))):
                try:
                    gates[gid] = fn()
                except Exception:
                    gates[gid] = {"pass": False, "error": traceback.format_exc()[-3000:]}
                LOG(f"{gid} pass={gates[gid]['pass']}")
            runner.pool.shutdown(wait=True)
            results["H"] = {"all_runs": runner.all_runs, "timing_authority": "diagnostic_only"}
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
            else:
                gates[gid] = {"pass": okc, "values": {"cargo": "see cargo"},
                              "threshold": TH["cargo"]["gate_tests"][gid]}
        if args.skip_cargo:
            gates["C4"] = {"pass": False, "values": "cargo skipped"}
    prov1 = provenance_hashes()
    changes1, gerr1 = tree_changes([out], [raw])
    clean = bool(not changes0 and not changes1 and gerr0 is None and gerr1 is None and prov0 == prov1
                 and binary_sha is not None and sha256_file(binary) == binary_sha)
    results["provenance"] = {"inputs": prov1, "binary_sha256": binary_sha, "git_start": git0, "git_end": git_info(),
                             "tree_changes_start": changes0,
                             "tree_changes_end": changes1, "git_error": [gerr0, gerr1],
                             "inputs_changed_during_run": prov0 != prov1, "clean": clean}
    order = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8"]
    results["gates"] = gates
    results["gate_table"] = {g: bool(gates.get(g, {}).get("pass")) for g in order}
    if results.get("verdict") != "INVALID":
        if not clean:
            results["verdict"] = "INVALID"
            results["reason"] = "dirty tree, changed inputs or stale binary"
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
