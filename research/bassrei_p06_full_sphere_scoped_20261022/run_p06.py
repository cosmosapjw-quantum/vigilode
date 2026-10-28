#!/usr/bin/env python3
"""Gate runner of BASS/rei DAG node P06 (scoped full-sphere photon transport in
triaxial Bianchi I: the (q, mu, phi) adapter, run through the RODAS5P fast
banded and dense drivers).

    python3 run_p06.py --out results.json [--quick] [--skip-cargo]

Builds the runners (`cargo build --profile measurement -p rodas5p-integrators
--example sphere_photon_fv_run --example lrs_photon_fv_run --locked --offline
-j 8`), drives them, the independent formula oracle
(`FULL_SPHERE_FVM_REFERENCE.py`, read-only) and the Rust contract tests, and
writes one JSON with every number, per-gate `{pass, values, threshold}` and
`verdict = PASS` iff every [C] gate passes and the tree is clean. Thresholds
are the frozen constants in TH. Wall times are `diagnostic_only`.

INVALID: a change under the watched inputs (`crates/`, this node directory
other than the harness's own outputs, the LRS formula oracle, the P02
reference, Cargo.toml, Cargo.lock, rust-toolchain.toml, .cargo/) at start or
end, a recorded input changed during the run, a stale binary, or any runner
request that timed out, failed in its environment or exited nonzero (an
incomplete run: no ledger row). `raw/` must be absent or empty at start.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse  # noqa: E402
import hashlib  # noqa: E402
import importlib.util  # noqa: E402
import json  # noqa: E402
import math  # noqa: E402
import os  # noqa: E402
import platform  # noqa: E402
import random  # noqa: E402
import re  # noqa: E402
import subprocess  # noqa: E402
import tempfile  # noqa: E402

# One BLAS thread here and in the oracle subprocess.
for _var in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_var] = "1"

import threading  # noqa: E402
import time  # noqa: E402
import traceback  # noqa: E402
from concurrent.futures import ThreadPoolExecutor  # noqa: E402
from pathlib import Path  # noqa: E402

import numpy as np  # noqa: E402

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
ORACLE_PATH = HERE / "FULL_SPHERE_FVM_REFERENCE.py"
LRS_ORACLE = REPO / "research" / "ncp_followup_20261021" / "BASS_REI_FVM_REFERENCE.py"


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod  # dataclasses resolve their module through sys.modules
    spec.loader.exec_module(mod)
    return mod


O = load_module("p06_full_sphere_oracle", ORACLE_PATH)
P02_REF = REPO / "research" / "bassrei_p02_fixed_grid_20261021" / "reference" / "lrs_reference.py"
R2 = load_module("p02_lrs_reference", P02_REF)  # independent P02 kbar closed form and mpmath quadrature
EPS = float(np.finfo(float).eps)
TAU = O.TAU

TH = {
    "C1": {"command": "python3 FULL_SPHERE_FVM_REFERENCE.py raw/oracle_CHECKS.json", "exit_code": 0,
           "status": "pass_scoped", "tests": 38, "failures": 0, "errors": 0, "skipped": 0,
           "golden_fixture": "the node's CHECKS.json (read by cargo a_): status pass_scoped, inputs.oracle_sha256 == "
                             "sha256(oracle), parity_fixture (incl. sha256_outputs) == the fresh run's"},
    "C2": {"cases": 160, "seed": 1610752528, "min_ok_cases": 160, "mismatches": 0, "refused": 0,
           "hex_only_differences": 0,
           "hex_bitwise": ["inputs", "grid constants (16 and B)", "drift v and dv", "face velocities",
                           "state RHS (no absorber, the runner's absorber, explicit group rates)",
                           "partial_t (vs the oracle's grid_state_partial_t with the runner's dkbar)",
                           "jvp callback output (the adapter evaluates J s through its RHS) == oracle state RHS of s",
                           "dense Jacobian (vs grid_state_jacobian_dense)",
                           "band == dense inside the band, dense == 0 outside"],
           "jacobian_action": "J_dense s (numpy, the runner's dense J) vs the oracle state RHS of s: "
                              "|d|_k <= 1e-14 (|J| |s|)_k (exact where (|J||s|)_k = 0)",
           "kbar_vs_p02_reference": "per group vs lrs_reference.kbar_closed with the sphere's float threshold "
                                    "(x = q_chi + ((ln a1 + ln a2) + ln a3)/3, rate = mean H): |d| <= 1e-14 |ref| for "
                                    "k and dk, exact where ref = 0, bitwise at exact ties",
           "kbar_mpmath_H": "per group vs lrs_reference.kbar_mpmath (40 digits; reported only)",
           "kbar_vs_lrs_adapter": "cross-check: physical coordinate bitwise; comoving |dk| <= 1e-12 max_i |k_lrs|, "
                                  "|d dk| <= 1e-12 max_i |dk_lrs| (the P02 adapter rounds the mean ln a differently)",
           "absorber_fraction": 0.5,
           "note": "every value compared by its hex (bitwise, the sign of zero included); every case must run OK"},
    "C3": {"fixed_steps": 64, "outputs": 5, "rel": 1e-13,
           "def": "max over outputs and energy groups i of max(max_j |sum_k N_ijk - N_ij|, |A_i,3D - A_i,2D|) / "
                  "max(max_j |N_ij|, |A_i,2D|), and |L_3D - L_2D| / |L_2D| for the outflow ledgers (exact where "
                  "L_2D = 0); phi-summed 3D adapter vs the P02 adapter, both fixed_banded with the same steps",
           "axisymmetry_rel": 1e-13,
           "axisymmetry_def": "max over outputs and groups i of max_j (max_k N_ijk/g_k - min_k N_ijk/g_k) / "
                              "max_j |N_ij|",
           "adaptive_H": "the same comparison for adaptive banded runs at rtol 1e-8 (reported only)"},
    "C4": {"rel": 1e-13, "def": "max over outputs and groups i of (max_jk u_i - min_jk u_i) / max_jk u_i, "
                                "u_ijk = N_ijk / (dmu_j dphi_k), isotropic geometry and data",
           "arms": ["banded rtol 1e-8", "fixed_banded 64 steps"]},
    "C5": {"levels": [[16, 8, 8], [32, 16, 16], [64, 32, 32]], "order_window": [0.75, 1.25],
           "order_pair": "finest", "strictly_decreasing": True, "rtol": 1e-7, "time_rtol": 1e-9,
           "time_uncertainty": "sum |N(rtol 1e-7) - N(rtol 1e-9)| / sum |N_exact| <= 0.02 err at every level",
           "quadrature_uncertainty": "at the coarsest level: |err(GL 3) - err(GL 5)| <= 0.25 err, and "
                                     "sum |N_exact,GL3 - N_exact,GL5| / sum |N_exact,GL5| <= 0.02 err at t = 0 and "
                                     "at T",
           "def": "sum |N_rust(T) - N_exact(T)| / sum |N_exact(T)| over cells, N_exact the tensor Gauss-Legendre "
                  "cell integrals of the exact characteristic solution (oracle exact_cell_contents)"},
    "C6": {"rtols": [1e-6, 1e-9], "attempt_ratio_min": 2.0, "arms": ["banded", "dense"],
           "dense_lu": "InPlaceZeroSkipping", "calibrated": True},
    "C7": {"repeat_bitwise": ["banded", "dense"], "def": "states and WorkCounters equal across two runs",
           "trajectory_invariant_rel": 1e-11,
           "trajectory_invariant_def": "|fsum(y(t)) - fsum(y(0))| / |fsum(y(0))| (every slot: cells, absorbed and "
                                       "outflow ledgers) at every output of every C3 (3D and 2D, both arms), C4, C6 "
                                       "and C7 trajectory (the P02 C3 / P04 C3 bound)"},
    "C8": {"status": "REFUSED", "code": "the oracle's refusal code for the same input where it has one, else "
                                        "the listed literal"},
    "incomplete": "any runner request with status TIMEOUT or ENVIRONMENT_FAILURE or a nonzero exit code makes the "
                  "run INVALID (incomplete; no ledger row), not FAIL",
    "cargo": {"binaries": ["sphere_photon_fv_contracts", "lrs_photon_fv_contracts", "generic_scope_contracts"],
              "gate_tests": {
                  "C2": ["sphere_photon_fv_contracts::a_golden_fixture_is_bitwise",
                         "sphere_photon_fv_contracts::b_dense_columns_are_unit_rhs_bitwise_and_the_band_is_exact",
                         "sphere_photon_fv_contracts::c_rhs_conserves_the_number_ledger",
                         "sphere_photon_fv_contracts::d_thomson_isotropic_state_is_stationary_and_conserves_the_"
                         "group_number",
                         "sphere_photon_fv_contracts::sanity_partial_t_matches_central_differences_away_from_kinks",
                         "sphere_photon_fv_contracts::sanity_jvp_callback_and_autonomy_flags"],
                  "C3": ["sphere_photon_fv_contracts::e_phi_summed_rhs_equals_the_lrs_adapter_on_axisymmetric_data"],
                  "C6": ["sphere_photon_fv_contracts::f_drivers_allocate_independently_of_the_step_count",
                         "sphere_photon_fv_contracts::f_callbacks_allocate_nothing",
                         "generic_scope_contracts::generic_solver_core_does_not_embed_physical_client_bindings"],
                  "C7": ["sphere_photon_fv_contracts::g_runs_are_deterministic_and_uniform_vector_atol_is_scalar"],
                  "C8": ["sphere_photon_fv_contracts::h_invalid_grids_geometries_rates_and_shapes_are_typed_"
                         "refusals"]}},
}

# C5: the smooth triaxial setup (see PREREGISTRATION.md for the smoothness argument).
CONV = {"H": [0.3, -0.1, 0.05], "T": 1.0, "q_half_width": 4.0, "phi_offset": 0.25, "sigma_q": 0.7,
        "angular": "1 + 0.3 n3 + 0.25 n3^2 + 0.3 (n1^2 - n2^2)", "coordinate": "physical", "boundary": "outflow",
        "gamma": 0.0, "absorber": None}


def conv_U0(q, n):
    s = CONV["sigma_q"]
    return math.exp(-(q * q) / (2.0 * s * s)) * (1.0 + 0.3 * n[2] + 0.25 * n[2] * n[2]
                                                 + 0.3 * (n[0] * n[0] - n[1] * n[1]))


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


def le(x, th):
    return isinstance(x, (int, float)) and math.isfinite(float(x)) and float(x) <= th


def bits_equal(a, b):
    a = np.ascontiguousarray(np.asarray(a, dtype=np.float64))
    b = np.ascontiguousarray(np.asarray(b, dtype=np.float64))
    return a.shape == b.shape and bool(np.all(a.view(np.uint64) == b.view(np.uint64)))


def sha256_file(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def hexes(v):
    return [float(z).hex() for z in v]


WATCHED = tuple(p for p in ("crates", str(HERE.relative_to(REPO)), "research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py",
                            "research/bassrei_p02_fixed_grid_20261021/reference", "Cargo.toml", "Cargo.lock",
                            "rust-toolchain.toml", ".cargo") if (REPO / p).exists())


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


EX = "crates/rodas5p-integrators/examples"
PROVENANCE_FILES = {
    "sphere_support_mod_rs": f"{EX}/sphere_photon_fv_support/mod.rs",
    "sphere_runner_rs": f"{EX}/sphere_photon_fv_run.rs",
    "sphere_contracts_rs": "crates/rodas5p-integrators/tests/sphere_photon_fv_contracts.rs",
    "lrs_support_mod_rs": f"{EX}/lrs_photon_fv_support/mod.rs",
    "lrs_runner_rs": f"{EX}/lrs_photon_fv_run.rs",
    "lrs_contracts_rs": "crates/rodas5p-integrators/tests/lrs_photon_fv_contracts.rs",
    "rodas5p_fast_rs": "crates/rodas5p-integrators/src/rodas5p_fast.rs",
    "run_p06_py": str((HERE / "run_p06.py").relative_to(REPO)),
    "oracle_py": str(ORACLE_PATH.relative_to(REPO)),
    "theory_md": str((HERE / "FULL_SPHERE_THEORY.md").relative_to(REPO)),
    "lrs_oracle_py": str(LRS_ORACLE.relative_to(REPO)),
    "p02_lrs_reference_py": str(P02_REF.relative_to(REPO)),
    "checks_json_golden_fixture": str((HERE / "CHECKS.json").relative_to(REPO)),
    "cargo_lock": "Cargo.lock",
    "cargo_toml": "Cargo.toml",
    "rust_toolchain_toml": "rust-toolchain.toml",
    "cargo_config_toml": ".cargo/config.toml",
    "cargo_config_offline_toml": ".cargo/config.offline.toml",
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


def build_runners(timeout_s, raw_dir):
    env = cargo_env()
    cmd = ["cargo", "build", "--profile", "measurement", "-p", "rodas5p-integrators", "--example",
           "sphere_photon_fv_run", "--example", "lrs_photon_fv_run", "--locked", "--offline", "-j", "8"]
    exdir = Path(env["CARGO_TARGET_DIR"]) / "measurement" / "examples"
    binaries = {"sphere": exdir / "sphere_photon_fv_run", "lrs": exdir / "lrs_photon_fv_run"}
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
            "log_tail": log[-1500:], "binaries": {k: str(v) for k, v in binaries.items()},
            "binaries_exist": all(v.is_file() for v in binaries.values())}, binaries


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
    """Runs request JSONs through a runner binary in a thread pool and keeps a
    compact copy of every exchange in raw/."""

    def __init__(self, binaries, work, raw, jobs, timeout):
        self.binaries = {k: str(v) for k, v in binaries.items()}
        self.work, self.raw = Path(work), Path(raw)
        self.pool = ThreadPoolExecutor(max_workers=jobs)
        self.timeout = timeout
        self.lock = threading.Lock()
        self.all_runs = {}
        self.incomplete = []

    def submit(self, tag, req, which="sphere", mode="timing", keep=True):
        """keep: "full" stores the request and the response whole (re-scorable); True the response whole and
        large request arrays by hash; False a slim response."""
        return self.pool.submit(self._run, tag, req, which, mode, keep)

    def _run(self, tag, req, which, mode, keep):
        path = self.work / f"{tag}.request.json"
        path.write_text(json.dumps(req))
        cmd = [self.binaries[which], "--request", str(path)]
        if mode != "timing":
            cmd += ["--measurement-mode", mode]
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
        big = ("initial_state", "atol", "states")
        if keep == "full":
            compact_req = dict(req)
        else:
            compact_req = {k: v for k, v in req.items() if k not in big}
            for k in big:
                if k in req:
                    compact_req[f"{k}_sha256"] = hashlib.sha256(json.dumps(req[k]).encode()).hexdigest()
        bad_exit = resp.get("_returncode") not in (None, 0)
        with self.lock:
            if resp.get("status") in ("TIMEOUT", "ENVIRONMENT_FAILURE") or bad_exit:
                self.incomplete.append({"tag": tag, "status": resp.get("status"), "returncode": resp.get("_returncode"),
                                        "reason": (resp.get("reason") or "")[:300]})
            self.all_runs[tag] = {"tag": tag, "runner": which, "status": resp.get("status"),
                                  "reason": (resp.get("reason") or "")[:300], "code": resp.get("code"),
                                  "wall_s": resp["_wall_s"], "timing_authority": "diagnostic_only",
                                  "counters": resp.get("counters"), "details": resp.get("details"),
                                  "allocations": resp.get("allocations")}
        if keep:  # "full" or True
            (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": resp})))
        else:
            slim = {k: v for k, v in resp.items() if k not in ("states", "dense", "band", "rhs", "jvp",
                                                                 "partial_t", "rhs_hex", "partial_t_hex")}
            (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": slim})))
        path.unlink(missing_ok=True)
        return resp


# ---------------------------------------------------------------------------
# Layout helpers (the oracle's state order)
# ---------------------------------------------------------------------------


class Lay3:
    def __init__(self, q, mu, phi):
        self.q, self.mu, self.phi = list(q), list(mu), list(phi)
        self.nq, self.nm, self.nphi = len(q) - 1, len(mu) - 1, len(phi) - 1
        self.M = self.nm * self.nphi
        self.n = 2 + self.nq * (self.M + 1)
        self.grid = O.SphereGrid(self.q, self.mu, self.phi)

    def cell(self, i, j, k):
        return 1 + i * (self.M + 1) + j * self.nphi + k

    def absorbed(self, i):
        return 1 + i * (self.M + 1) + self.M

    def layout(self, coordinate, boundary):
        return {"q_edges": self.q, "mu_edges": self.mu, "phi_edges": self.phi, "coordinate": coordinate,
                "boundary": boundary}

    def atol(self, scale):
        g = self.grid
        a = [scale] * self.n
        for i in range(self.nq):
            for j in range(self.nm):
                for k in range(self.nphi):
                    a[self.cell(i, j, k)] = scale * g.dq[i] * g.dm[j] * g.dphi[k]
        return a


class Lay2:
    def __init__(self, q, mu):
        self.q, self.mu = list(q), list(mu)
        self.nq, self.nm = len(q) - 1, len(mu) - 1
        self.n = 2 + self.nq * (self.nm + 1)

    def cell(self, i, j):
        return 1 + i * (self.nm + 1) + j

    def absorbed(self, i):
        return 1 + i * (self.nm + 1) + self.nm

    def layout(self, coordinate, boundary):
        return {"q_edges": self.q, "mu_edges": self.mu, "coordinate": coordinate, "boundary": boundary}


def integrate_req(layout, geometry, gamma, absorber, arm, y0, t_end, outputs, rtol, atol, **extra):
    req = {"layout": layout, "geometry": geometry, "gamma": gamma, "absorber": absorber, "drop_partial_t": False,
           "mode": "integrate", "arm": arm, "initial_state": list(map(float, y0)), "t_span": [0.0, t_end],
           "output_times": list(outputs), "rtol": rtol, "atol": list(map(float, atol))}
    req.update(extra)
    return req


# ---------------------------------------------------------------------------
# C1: the oracle's own checks
# ---------------------------------------------------------------------------


def run_c1(raw_dir):
    th = TH["C1"]
    out = raw_dir / "oracle_CHECKS.json"
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    t0 = time.perf_counter()
    try:
        p = subprocess.run([sys.executable, str(ORACLE_PATH), str(out)], cwd=str(HERE), env=env,
                           capture_output=True, text=True, timeout=3600)
        rc, log = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        rc, log = None, "TIMEOUT\n" + str(e.stderr)
    (raw_dir / "oracle_checks.log").write_text(log)
    rep = json.loads(out.read_text()) if out.is_file() else {}
    vals = {"exit_code": rc, "status": rep.get("status"), "tests": rep.get("tests"), "failures": rep.get("failures"),
            "errors": rep.get("errors"), "skipped": rep.get("skipped"), "inputs": rep.get("inputs"),
            "diagnostics_H": rep.get("diagnostics"), "wall_s": time.perf_counter() - t0,
            "log_file": "raw/oracle_checks.log", "report_file": "raw/oracle_CHECKS.json"}
    ok = all(vals[k] == th[k] for k in ("exit_code", "status", "tests", "failures", "errors", "skipped"))
    inp = rep.get("inputs") or {}
    ok &= inp.get("oracle_sha256") == sha256_file(ORACLE_PATH) and inp.get("lrs_oracle_sha256") == sha256_file(LRS_ORACLE)
    # The golden fixture the cargo a_ test reads is the node's CHECKS.json: it must come from this oracle and
    # carry the fixture the oracle produces now.
    node = json.loads((HERE / "CHECKS.json").read_text())
    fresh_pf = (rep.get("diagnostics") or {}).get("parity_fixture")
    node_pf = (node.get("diagnostics") or {}).get("parity_fixture")
    gf = {"status": node.get("status"), "oracle_sha256_matches": (node.get("inputs") or {}).get("oracle_sha256")
          == sha256_file(ORACLE_PATH), "parity_fixture_equal": fresh_pf is not None and node_pf == fresh_pf,
          "sha256_outputs": (node_pf or {}).get("sha256_outputs"),
          "fresh_sha256_outputs": (fresh_pf or {}).get("sha256_outputs")}
    vals["golden_fixture"] = gf
    ok &= gf["status"] == "pass_scoped" and gf["oracle_sha256_matches"] and gf["parity_fixture_equal"]
    return {"pass": bool(ok), "values": vals, "threshold": th}


# ---------------------------------------------------------------------------
# C2: bit parity with the oracle on random grids
# ---------------------------------------------------------------------------


def c2_cases():
    th = TH["C2"]
    rng = random.Random(th["seed"])
    cases = []
    for case in range(th["cases"]):
        nq, nm, nphi = rng.randint(1, 4), rng.randint(1, 5), rng.randint(1, 7)
        q = O._random_edges(rng, -2.0, 1.5, nq)
        mu = O._random_edges(rng, -1.0, 1.0, nm)
        offset = rng.choice([0.0, 0.25, 0.3, 2.9, 5.5])
        phi = O._random_phi_edges(rng, nphi, offset) if rng.random() < 0.7 else O.uniform_phi_edges(nphi, offset)
        boundary = rng.choice(["outflow", "closed"])
        coordinate = rng.choice(["physical", "comoving"])
        gamma = rng.choice([0.0, 2.5, 0.7, 1e3])
        power = rng.random() < 0.5
        hh = [rng.uniform(-1, 1) for _ in range(3)]
        if rng.random() < 0.2:
            hh[1] = hh[0]  # LRS
        t = rng.choice([0.0, 0.37, 1.3])
        absorber = None
        if rng.random() < th["absorber_fraction"]:
            q_chi = q[rng.randrange(len(q))] if rng.random() < 0.15 and coordinate == "physical" \
                else rng.uniform(q[0] - 0.2, q[-1] + 0.2)
            absorber = {"k0": rng.uniform(0.1, 3.0), "q_chi": q_chi}
        if power:
            x = 1.0 + t
            Ht = [a / x for a in hh]
            dHt = [-a / (x * x) for a in hh]
            geometry = {"kind": "power_law", "hh1": hh[0], "hh2": hh[1], "hh3": hh[2]}
        else:
            Ht, dHt = list(hh), [0.0, 0.0, 0.0]
            geometry = {"kind": "constant", "h1": hh[0], "h2": hh[1], "h3": hh[2]}
        if coordinate == "comoving":
            hm = ((Ht[0] + Ht[1]) + Ht[2]) / 3.0
            dhm = ((dHt[0] + dHt[1]) + dHt[2]) / 3.0
            V, dV = [h - hm for h in Ht], [d - dhm for d in dHt]
        else:
            V, dV = Ht, dHt
        lay = Lay3(q, mu, phi)
        states = [[rng.uniform(0.0, 2.0) for _ in range(lay.n)], [rng.uniform(-1.0, 3.0) for _ in range(lay.n)]]
        rates = [rng.choice([0.0, rng.uniform(0.0, 2.0)]) for _ in range(nq)]
        req = {"mode": "probe", "layout": lay.layout(coordinate, boundary), "geometry": geometry, "gamma": gamma,
               "absorber": absorber, "drop_partial_t": False, "t": t, "states": states, "group_rates": rates}
        lrs_req = None
        if absorber is not None:
            mean_hh = ((hh[0] + hh[1]) + hh[2]) / 3.0
            lgeom = {"kind": "power_law", "hh": mean_hh, "s0": 0.0, "p": 2.0} if power else \
                {"kind": "constant", "h": mean_hh, "s": 0.0}
            l2 = Lay2(q, [-1.0, 1.0])
            lrs_req = {"mode": "probe", "layout": l2.layout(coordinate, boundary), "geometry": lgeom, "gamma": 0.0,
                       "absorber": absorber, "drop_partial_t": False, "t": t, "states": [[0.0] * l2.n]}
        cases.append(dict(case=case, lay=lay, req=req, lrs_req=lrs_req, V=V, dV=dV, gamma=gamma, boundary=boundary,
                          coordinate=coordinate, states=states, rates=rates, absorber=absorber))
    return cases


def c2_submit(runner, cases):
    return [(runner.submit(f"C2_probe_{c['case']:03d}", c["req"], keep="full"),
             runner.submit(f"C2_lrs_kbar_{c['case']:03d}", c["lrs_req"], which="lrs", keep="full")
             if c["lrs_req"] else None) for c in cases]


class SphereMeanGeometry:
    """The sphere adapter's threshold motion, in its float order, for lrs_reference.threshold / kbar_closed /
    kbar_mpmath: ln a = ((ln a1 + ln a2) + ln a3) / 3, H = ((H1 + H2) + H3) / 3."""

    def __init__(self, geometry):
        self.geometry = geometry
        self.kind = geometry["kind"]
        if self.kind == "constant":
            self.hk = [float(geometry[k]) for k in ("h1", "h2", "h3")]
            self.h = ((self.hk[0] + self.hk[1]) + self.hk[2]) / 3.0
        else:
            self.hk = [float(geometry[k]) for k in ("hh1", "hh2", "hh3")]
            self.hh = ((self.hk[0] + self.hk[1]) + self.hk[2]) / 3.0

    def ln_a(self, t):
        t = float(t)
        la = [h * t for h in self.hk] if self.kind == "constant" else [h * math.log1p(t) for h in self.hk]
        return ((la[0] + la[1]) + la[2]) / 3.0

    def H(self, t):
        x = 1.0 + float(t)
        hs = self.hk if self.kind == "constant" else [h / x for h in self.hk]
        return ((hs[0] + hs[1]) + hs[2]) / 3.0

    def ln_a_mp(self, t):
        mp = R2.mp
        t = mp.mpf(t)
        hk = [mp.mpf(h) for h in self.hk]
        return sum(h * t for h in hk) / 3 if self.kind == "constant" else sum(hk) / 3 * mp.log(1 + t)


def run_c2(futs, cases):
    th = TH["C2"]
    rep = {"ok_cases": 0, "compared": 0, "mismatches": [], "hex_only_differences": 0, "by_quantity": {},
           "refused": [], "absorber_cases": 0, "kbar_physical_bitwise_cases": 0, "kbar_comoving_cases": 0,
           "kbar_comoving_max_rel": 0.0, "kbar_comoving_dk_max_rel": 0.0,
           "kbar_vs_p02_reference_max_rel": 0.0, "dkbar_vs_p02_reference_max_rel": 0.0, "kbar_tie_cells": 0,
           "kbar_mpmath_H": {"max_k_rel": 0.0, "max_dk_rel": 0.0, "mpmath_diff_vs_leibniz_max": 0.0, "cells": 0},
           "jacobian_action_max_ratio": 0.0}

    def note_hex(name, got, want, case):
        rep["by_quantity"][name] = rep["by_quantity"].get(name, 0) + len(want)
        rep["compared"] += len(want)
        if len(got) != len(want):
            rep["mismatches"].append({"case": case, "what": name, "len": [len(got), len(want)]})
            return
        for k, (g, w) in enumerate(zip(got, want)):
            if g != w:
                rep["mismatches"].append({"case": case, "what": f"{name}[{k}]", "rust": g, "oracle": w})

    def note_val(name, got, want, case):
        got, want = [float(v) for v in got], [float(v) for v in want]
        rep["by_quantity"][name] = rep["by_quantity"].get(name, 0) + len(want)
        rep["compared"] += len(want)
        if len(got) != len(want):
            rep["mismatches"].append({"case": case, "what": name, "len": [len(got), len(want)]})
            return
        for k, (g, w) in enumerate(zip(got, want)):
            if not g == w:
                rep["mismatches"].append({"case": case, "what": f"{name}[{k}]", "rust": g.hex(), "oracle": w.hex()})
            elif g.hex() != w.hex():
                rep["hex_only_differences"] += 1
                rep["mismatches"].append({"case": case, "what": f"{name}[{k}] (sign of zero)", "rust": g.hex(),
                                          "oracle": w.hex()})

    for (fut, lfut), c in zip(futs, cases):
        r = fut.result()
        case = c["case"]
        if r.get("status") != "OK":
            rep["refused"].append({"case": case, "status": r.get("status"), "reason": (r.get("reason") or "")[:200]})
            continue
        rep["ok_cases"] += 1
        lay, grid = c["lay"], c["lay"].grid
        V, dV, gamma, boundary = c["V"], c["dV"], c["gamma"], c["boundary"]
        ih = r["inputs_hex"]
        note_hex("inputs.q", ih["q_edges"], hexes(lay.q), case)
        note_hex("inputs.mu", ih["mu_edges"], hexes(lay.mu), case)
        note_hex("inputs.phi", ih["phi_edges"], hexes(lay.phi), case)
        gc = r["grid_constants"]
        for name in ("dq", "dm", "dphi", "m2", "t2", "p2", "r", "e", "g", "cphi", "sphi", "c2", "s2h", "wface",
                     "sn2", "om"):
            note_hex(f"const.{name}", gc[name], hexes(getattr(grid, name)), case)
        for m in range(5):
            note_hex(f"const.B{m}", gc["B"][m], hexes(grid.B[m]), case)
        note_hex("drift.v", r["drift"]["v_hex"], hexes(V), case)
        note_hex("drift.dv", r["drift"]["dv_hex"], hexes(dV), case)
        gk, vq, hd = O._velocities(grid, O._geometry(*V))
        fv = r["face_velocities"]
        note_hex("face.vq", fv["vq_hex"], hexes(vq), case)
        note_hex("face.vmu", fv["vmu_hex"],
                 hexes([-(gk[k] * grid.wface[l]) for k in range(lay.nphi) for l in range(1, lay.nm)]), case)
        note_hex("face.vphi", fv["vphi_hex"], hexes([hd * grid.sn2[l] for l in range(lay.nphi)]), case)
        kb = r["kbar"]
        kbar = [k[0] for k in kb] if c["absorber"] else None
        dkbar = [k[1] for k in kb] if c["absorber"] else None
        if c["absorber"]:
            rep["absorber_cases"] += 1
            # Independent P02 reference (closed form) at the sphere's float threshold; mpmath [H].
            geom = SphereMeanGeometry(c["req"]["geometry"])
            g2 = R2.Grid(lay.q, [-1.0, 1.0], c["coordinate"], boundary)
            ab = R2.Absorber(c["absorber"]["k0"], c["absorber"]["q_chi"])
            t = c["req"]["t"]
            kc, dkc = R2.kbar_closed(g2, ab, geom, t)
            xc, rate = R2.threshold(ab, geom, c["coordinate"], t)
            for i, (pair, k_ref, dk_ref) in enumerate(zip(kb, kc, dkc)):
                tie = lay.q[i] == xc or lay.q[i + 1] == xc
                rep["kbar_tie_cells"] += int(tie)
                for what, got, ref in (("kbar", pair[0], float(k_ref)), ("dkbar", pair[1], float(dk_ref))):
                    if tie or ref == 0.0:
                        if float(got).hex() != ref.hex():
                            rep["mismatches"].append({"case": case, "what": f"{what}_vs_p02_reference[{i}]",
                                                      "tie": tie, "rust": float(got).hex(), "reference": ref.hex()})
                        continue
                    e = abs(got - ref) / abs(ref)
                    key = f"{what}_vs_p02_reference_max_rel"
                    rep[key] = max(rep[key], e)
                    if not e <= 1e-14:
                        rep["mismatches"].append({"case": case, "what": f"{what}_vs_p02_reference[{i}]", "rel": e})
            km, dkm, ddev, _ = R2.kbar_mpmath(g2, ab, geom, t)
            hm = rep["kbar_mpmath_H"]
            for pair, a, b in zip(kb, km, dkm):
                a, b = float(a), float(b)
                hm["max_k_rel"] = max(hm["max_k_rel"], abs(pair[0] - a) / abs(a) if a else float(pair[0] != 0.0))
                hm["max_dk_rel"] = max(hm["max_dk_rel"], abs(pair[1] - b) / abs(b) if b else float(pair[1] != 0.0))
                hm["cells"] += 1
            hm["mpmath_diff_vs_leibniz_max"] = max(hm["mpmath_diff_vs_leibniz_max"], float(ddev))
            lr = lfut.result() if lfut else {}
            if lr.get("status") != "OK":
                rep["mismatches"].append({"case": case, "what": "lrs_kbar_probe", "status": lr.get("status"),
                                          "reason": (lr.get("reason") or "")[:200]})
            else:
                kl = lr["kbar"]
                if c["coordinate"] == "physical":
                    note_hex("kbar_vs_lrs", hexes([v for p in kb for v in p]), hexes([v for p in kl for v in p]), case)
                    rep["kbar_physical_bitwise_cases"] += 1
                else:
                    rep["kbar_comoving_cases"] += 1
                    ks, kl0 = np.array(kb, float), np.array(kl, float)
                    s0 = max(float(np.max(np.abs(kl0[:, 0]))), 1e-300)
                    s1 = max(float(np.max(np.abs(kl0[:, 1]))), 1e-300)
                    e0 = float(np.max(np.abs(ks[:, 0] - kl0[:, 0]))) / s0
                    e1 = float(np.max(np.abs(ks[:, 1] - kl0[:, 1]))) / s1
                    rep["kbar_comoving_max_rel"] = max(rep["kbar_comoving_max_rel"], e0)
                    rep["kbar_comoving_dk_max_rel"] = max(rep["kbar_comoving_dk_max_rel"], e1)
                    if not (e0 <= 1e-12 and e1 <= 1e-12):
                        rep["mismatches"].append({"case": case, "what": "kbar_comoving", "rel": [e0, e1]})
        for s, st in enumerate(c["states"]):
            note_hex("state_rhs", r["rhs_hex"][s], hexes(O.grid_state_rhs(grid, st, *V, gamma, kbar, boundary)), case)
            note_hex("state_rhs(group_rates)", r["rhs_group_rates_hex"][s],
                     hexes(O.grid_state_rhs(grid, st, *V, gamma, c["rates"], boundary)), case)
            note_val("partial_t", r["partial_t"][s], O.grid_state_partial_t(grid, st, V, dV, dkbar, boundary), case)
            note_val("jvp_callback", r["jvp"][s], O.grid_state_rhs(grid, st, *V, gamma, kbar, boundary), case)
        J = O.grid_state_jacobian_dense(grid, *V, gamma, kbar, boundary)
        if r["dense"] is None:
            rep["mismatches"].append({"case": case, "what": "dense missing"})
            continue
        D = r["dense"]
        # The Jacobian action: the runner's dense J times each state vs the oracle state RHS (a priori bound:
        # n u (|J||s|)_k with at most ~45 terms per row, 45 * 1.11e-16 < 1e-14).
        Dm = np.array(D, float)
        for s, st in enumerate(c["states"]):
            sv = np.array(st, float)
            js = Dm @ sv
            ref = np.array(O.grid_state_rhs(grid, st, *V, gamma, kbar, boundary), float)
            scale = np.abs(Dm) @ np.abs(sv)
            d = np.abs(js - ref)
            rep["by_quantity"]["jacobian_action"] = rep["by_quantity"].get("jacobian_action", 0) + len(ref)
            rep["compared"] += len(ref)
            bad = np.where(d > 1e-14 * scale)[0]
            pos = scale > 0
            if np.any(pos):
                rep["jacobian_action_max_ratio"] = max(rep["jacobian_action_max_ratio"],
                                                       float(np.max(d[pos] / scale[pos])))
            if bad.size:
                rep["mismatches"].append({"case": case, "what": f"jacobian_action[{s}]", "rows": bad[:10].tolist()})
        note_val("dense_jacobian", [v for row in D for v in row], [v for row in J for v in row], case)
        bw, n = r["bandwidth"], lay.n
        w = 2 * bw + 1
        band = r["band"]
        inside, outside = [], []
        want_in = []
        for i in range(n):
            for j in range(n):
                if abs(i - j) <= bw:
                    inside.append(band[i * w + (j + bw - i)])
                    want_in.append(D[i][j])
                elif D[i][j] != 0.0:
                    outside.append(D[i][j])
        note_val("band_vs_dense", inside, want_in, case)
        if outside:
            rep["mismatches"].append({"case": case, "what": "dense nonzero outside the band", "count": len(outside)})
    nmis = len(rep["mismatches"])
    rep["mismatch_count"] = nmis
    rep["mismatches"] = rep["mismatches"][:50]
    ok = rep["ok_cases"] >= th["min_ok_cases"] and nmis == th["mismatches"] and rep["absorber_cases"] > 0 \
        and len(rep["refused"]) == th["refused"] and rep["hex_only_differences"] == th["hex_only_differences"] \
        and rep["kbar_physical_bitwise_cases"] > 0 and rep["kbar_comoving_cases"] > 0
    return {"pass": bool(ok), "values": rep, "threshold": th}


# ---------------------------------------------------------------------------
# C3: phi-summed 3D == the P02 adapter along fixed-step trajectories
# ---------------------------------------------------------------------------

C3_SCEN = {
    "L1": dict(coordinate="physical", boundary="outflow", h=0.3, s=0.1, gamma=0.5, absorber={"k0": 0.5, "q_chi": -0.4},
               nq=12, nm=6, mu="uniform", nphi=4, phi_offset=0.3, phi="uniform", T=3.0),
    "L2": dict(coordinate="comoving", boundary="closed", h=0.2, s=-0.15, gamma=2.0, absorber={"k0": 1.0, "q_chi": -0.5},
               nq=10, nm=5, mu="random", nphi=7, phi_offset=1.1, phi="random", T=3.0),
}


def c3_setup(sid):
    sc = C3_SCEN[sid]
    rng = random.Random(f"P06-C3-{sid}")
    q = O.uniform_edges(-3.0, 2.0, sc["nq"])
    mu = O.uniform_mu_edges(sc["nm"]) if sc["mu"] == "uniform" else O._random_edges(rng, -1.0, 1.0, sc["nm"])
    phi = O.uniform_phi_edges(sc["nphi"], sc["phi_offset"]) if sc["phi"] == "uniform" else \
        O._random_phi_edges(rng, sc["nphi"], sc["phi_offset"])
    l3, l2 = Lay3(q, mu, phi), Lay2(q, mu)
    g = l3.grid
    y2 = [0.0] * l2.n
    y3 = [0.0] * l3.n
    for i in range(l2.nq):
        qc = 0.5 * (q[i] + q[i + 1])
        for j in range(l2.nm):
            mc = 0.5 * (mu[j] + mu[j + 1])
            v = g.dq[i] * g.dm[j] * TAU * math.exp(-(qc * qc) / (2.0 * 0.36)) * (1.0 + 0.2 * mc + 0.3 * mc * mc)
            y2[l2.cell(i, j)] = v
            for k in range(l3.nphi):
                y3[l3.cell(i, j, k)] = v * g.g[k]
    a2 = [1e-10] * l2.n
    for i in range(l2.nq):
        for j in range(l2.nm):
            a2[l2.cell(i, j)] = 1e-10 * g.dq[i] * g.dm[j] * TAU
    h, s = sc["h"], sc["s"]
    geo3 = {"kind": "constant", "h1": h - s, "h2": h - s, "h3": h + 2.0 * s}
    geo2 = {"kind": "constant", "h": h, "s": s}
    T = sc["T"]
    outs = [T * k / (TH["C3"]["outputs"] - 1) for k in range(TH["C3"]["outputs"])]
    return sc, l3, l2, y3, y2, l3.atol(1e-10), a2, geo3, geo2, outs


def c3_submit(runner):
    futs = {}
    for sid in C3_SCEN:
        sc, l3, l2, y3, y2, a3, a2, geo3, geo2, outs = c3_setup(sid)
        for arm, extra, rtol in (("fixed_banded", {"fixed_steps": TH["C3"]["fixed_steps"]}, 1e-8), ("banded", {}, 1e-8)):
            r3 = integrate_req(l3.layout(sc["coordinate"], sc["boundary"]), geo3, sc["gamma"], sc["absorber"], arm, y3,
                               sc["T"], outs, rtol, a3, **extra)
            r2 = integrate_req(l2.layout(sc["coordinate"], sc["boundary"]), geo2, sc["gamma"], sc["absorber"], arm, y2,
                               sc["T"], outs, rtol, a2, **extra)
            futs[(sid, arm)] = (runner.submit(f"C3_{sid}_{arm}_3d", r3), runner.submit(f"C3_{sid}_{arm}_2d", r2, "lrs"))
    return futs


def ratio(x, scale):
    """x / scale, with 0/0 = 0 (an exact zero must be matched exactly)."""
    if scale > 0.0:
        return x / scale
    return 0.0 if x == 0.0 else float("inf")


def lrs_deviation(l3, l2, s3, s2):
    """TH C3: per energy group, relative to that group's own scale; outflow ledgers relative to themselves."""
    g = l3.grid
    dev, axi = 0.0, 0.0
    for i in range(l2.nq):
        d_i, a_i, top = 0.0, 0.0, 0.0
        for j in range(l2.nm):
            row = [s3[l3.cell(i, j, k)] for k in range(l3.nphi)]
            n2 = s2[l2.cell(i, j)]
            d_i = max(d_i, abs(math.fsum(row) - n2))
            u = [row[k] / g.g[k] for k in range(l3.nphi)]
            a_i = max(a_i, max(u) - min(u))
            top = max(top, abs(n2))
        d_i = max(d_i, abs(s3[l3.absorbed(i)] - s2[l2.absorbed(i)]))
        dev = max(dev, ratio(d_i, max(top, abs(s2[l2.absorbed(i)]))))
        axi = max(axi, ratio(a_i, top))
    for a, b in ((s3[0], s2[0]), (s3[-1], s2[-1])):
        dev = max(dev, ratio(abs(a - b), abs(b)))
    return dev, axi


def run_c3(futs):
    th = TH["C3"]
    vals, ok = {}, True
    for sid in C3_SCEN:
        sc, l3, l2, *_ , outs = c3_setup(sid)
        row = {}
        for arm in ("fixed_banded", "banded"):
            f3, f2 = futs[(sid, arm)]
            r3, r2 = f3.result(), f2.result()
            good = r3.get("status") == "OK" and r2.get("status") == "OK" and len(r3.get("states") or []) == len(outs) \
                and len(r2.get("states") or []) == len(outs) and r3.get("times") == outs and r2.get("times") == outs
            if not good:
                row[arm] = {"status": [r3.get("status"), r2.get("status")],
                            "reason": [(r3.get("reason") or "")[:200], (r2.get("reason") or "")[:200]]}
                ok &= arm != "fixed_banded"
                continue
            devs = [lrs_deviation(l3, l2, a, b) for a, b in zip(r3["states"], r2["states"])]
            d3, d2 = r3.get("details") or {}, r2.get("details") or {}
            row[arm] = {"max_rel_dev": max(d for d, _ in devs), "max_rel_axisymmetry": max(a for _, a in devs),
                        "per_output": devs, "accepted_steps": [d3.get("accepted_steps"), d2.get("accepted_steps")],
                        "attempts": [d3.get("attempts"), d2.get("attempts")], "authority": "C" if arm == "fixed_banded" else "H"}
            if arm == "fixed_banded":
                same_steps = d3.get("accepted_steps") == d2.get("accepted_steps") == th["fixed_steps"]
                row[arm]["same_step_count"] = bool(same_steps)
                ok &= bool(same_steps and le(row[arm]["max_rel_dev"], th["rel"])
                           and le(row[arm]["max_rel_axisymmetry"], th["axisymmetry_rel"]))
        vals[sid] = row
    return {"pass": bool(ok), "values": vals, "threshold": th, "scenarios": C3_SCEN}


# ---------------------------------------------------------------------------
# C4: isotropy
# ---------------------------------------------------------------------------

C4_SCEN = {
    "I1": dict(coordinate="physical", boundary="outflow", geometry={"kind": "constant", "h1": 0.3, "h2": 0.3, "h3": 0.3},
               gamma=0.5, absorber={"k0": 0.5, "q_chi": -0.4}, nq=10, nm=6, mu="random", nphi=5, phi_offset=0.7,
               phi="random", T=3.0),
    "I2": dict(coordinate="comoving", boundary="closed",
               geometry={"kind": "power_law", "hh1": 0.5, "hh2": 0.5, "hh3": 0.5}, gamma=2.0,
               absorber={"k0": 1.0, "q_chi": -0.5}, nq=10, nm=8, mu="uniform", nphi=6, phi_offset=0.25, phi="uniform",
               T=3.0),
}


def c4_setup(sid):
    sc = C4_SCEN[sid]
    rng = random.Random(f"P06-C4-{sid}")
    q = O.uniform_edges(-3.0, 2.0, sc["nq"])
    mu = O.uniform_mu_edges(sc["nm"]) if sc["mu"] == "uniform" else O._random_edges(rng, -1.0, 1.0, sc["nm"])
    phi = O.uniform_phi_edges(sc["nphi"], sc["phi_offset"]) if sc["phi"] == "uniform" else \
        O._random_phi_edges(rng, sc["nphi"], sc["phi_offset"])
    l3 = Lay3(q, mu, phi)
    g = l3.grid
    y = [0.0] * l3.n
    for i in range(l3.nq):
        qc = 0.5 * (q[i] + q[i + 1])
        f = g.dq[i] * math.exp(-(qc * qc) / (2.0 * 0.36))
        for j in range(l3.nm):
            for k in range(l3.nphi):
                y[l3.cell(i, j, k)] = f * g.dm[j] * g.dphi[k]
    T = sc["T"]
    outs = [T * k / 4 for k in range(5)]
    return sc, l3, y, l3.atol(1e-10), outs


def c4_submit(runner):
    futs = {}
    for sid in C4_SCEN:
        sc, l3, y, a, outs = c4_setup(sid)
        for arm, extra in (("banded", {}), ("fixed_banded", {"fixed_steps": 64})):
            futs[(sid, arm)] = runner.submit(f"C4_{sid}_{arm}", integrate_req(
                l3.layout(sc["coordinate"], sc["boundary"]), sc["geometry"], sc["gamma"], sc["absorber"], arm, y,
                sc["T"], outs, 1e-8, a, **extra))
    return futs


def isotropy_defect(l3, s):
    """TH C4: per energy group, relative to that group's max."""
    g = l3.grid
    worst = 0.0
    for i in range(l3.nq):
        u = [s[l3.cell(i, j, k)] / (g.dm[j] * g.dphi[k]) for j in range(l3.nm) for k in range(l3.nphi)]
        worst = max(worst, ratio(max(u) - min(u), max(abs(v) for v in u)))
    return worst


def run_c4(futs):
    th = TH["C4"]
    vals, ok = {}, True
    for sid in C4_SCEN:
        sc, l3, y, a, outs = c4_setup(sid)
        row = {"initial_defect": isotropy_defect(l3, y)}
        for arm in ("banded", "fixed_banded"):
            r = futs[(sid, arm)].result()
            if r.get("status") != "OK" or len(r.get("states") or []) != len(outs):
                row[arm] = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200]}
                ok = False
                continue
            d = [isotropy_defect(l3, s) for s in r["states"]]
            row[arm] = {"max_defect": max(d), "per_output": d, "attempts": (r.get("details") or {}).get("attempts")}
            ok &= le(max(d), th["rel"])
        vals[sid] = row
    return {"pass": bool(ok), "values": vals, "threshold": th, "scenarios": C4_SCEN}


# ---------------------------------------------------------------------------
# C5: convergence to the exact triaxial characteristics
# ---------------------------------------------------------------------------


def c5_lay(level):
    nq, nm, nphi = level
    L = CONV["q_half_width"]
    return Lay3(O.uniform_edges(-L, L, nq), O.uniform_mu_edges(nm), O.uniform_phi_edges(nphi, CONV["phi_offset"]))


def c5_exact(level, npts):
    lay = c5_lay(level)
    A = tuple(math.exp(h * CONV["T"]) for h in CONV["H"])
    n0 = O.exact_cell_contents(lay.q, lay.mu, lay.phi, (1.0, 1.0, 1.0), conv_U0, npts)
    nT = O.exact_cell_contents(lay.q, lay.mu, lay.phi, A, conv_U0, npts)
    return n0, nT


def c5_state(lay, cells):
    y = [0.0] * lay.n
    for i in range(lay.nq):
        for c in range(lay.M):
            y[1 + i * (lay.M + 1) + c] = cells[i * lay.M + c]
    return y


def c5_cells(lay, y):
    return np.array([y[1 + i * (lay.M + 1) + c] for i in range(lay.nq) for c in range(lay.M)], float)


def c5_submit(runner, quick, raw_dir):
    th = TH["C5"]
    levels = th["levels"][:2] if quick else th["levels"]
    futs, exact = {}, {}
    H = CONV["H"]
    geo = {"kind": "constant", "h1": H[0], "h2": H[1], "h3": H[2]}
    for level in levels:
        lay = c5_lay(level)
        n0, nT = c5_exact(level, 3)
        exact[tuple(level)] = (n0, nT)
        y0 = c5_state(lay, n0)
        for rtol in (th["rtol"], th["time_rtol"]):
            req = integrate_req(lay.layout(CONV["coordinate"], CONV["boundary"]), geo, CONV["gamma"], CONV["absorber"],
                                "banded", y0, CONV["T"], [0.0, CONV["T"]], rtol, lay.atol(rtol * 1e-3))
            futs[(tuple(level), rtol)] = runner.submit(f"C5_{level[0]}x{level[1]}x{level[2]}_rtol{rtol:.0e}", req,
                                                       keep="full")
    n0q, nTq = c5_exact(levels[0], 5)
    exact[("quadrature", tuple(levels[0]))] = (n0q, nTq)
    (raw_dir / "c5_exact.json").write_text(json.dumps(sanitize(
        {f"{k[0]}_{'x'.join(map(str, k[1]))}_gl5" if k[0] == "quadrature" else f"{'x'.join(map(str, k))}_gl3":
         {"t0": v[0], "T": v[1]} for k, v in exact.items()})))
    return futs, exact, levels


def run_c5(futs, exact, levels, quick):
    th = TH["C5"]
    rows, errs, ok = [], [], True
    for level in levels:
        lev = tuple(level)
        lay = c5_lay(level)
        nex = np.array(exact[lev][1], float)
        row = {"level": level, "cells": lay.nq * lay.M}
        fvs = {}
        for rtol in (th["rtol"], th["time_rtol"]):
            r = futs[(lev, rtol)].result()
            if r.get("status") != "OK" or len(r.get("states") or []) != 2:
                row[f"rtol_{rtol:.0e}"] = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200]}
                ok = False
                continue
            fv = c5_cells(lay, r["states"][-1])
            fvs[rtol] = fv
            e = float(np.sum(np.abs(fv - nex)) / np.sum(np.abs(nex)))
            d = r.get("details") or {}
            row[f"rtol_{rtol:.0e}"] = {"error": e, "attempts": d.get("attempts"), "wall_s": r.get("_wall_s"),
                                       "invariant_drift": abs(math.fsum(r["states"][-1]) - math.fsum(r["states"][0]))
                                       / math.fsum(r["states"][0])}
        e7 = (row.get(f"rtol_{th['rtol']:.0e}") or {}).get("error")
        errs.append(e7 if e7 is not None else float("nan"))
        if len(fvs) == 2 and e7 is not None:
            tu = float(np.sum(np.abs(fvs[th["rtol"]] - fvs[th["time_rtol"]])) / np.sum(np.abs(nex)))
            row["time_uncertainty_state"] = tu
            row["time_uncertainty_ok"] = bool(tu <= 0.02 * e7)
            ok &= row["time_uncertainty_ok"]
        else:
            ok = False
        rows.append(row)
    n0q, nTq = exact[("quadrature", tuple(levels[0]))]
    n03, nT3 = exact[tuple(levels[0])]
    r = futs[(tuple(levels[0]), th["rtol"])].result()
    quad = None
    if r.get("status") == "OK" and math.isfinite(errs[0]):
        fv = c5_cells(c5_lay(levels[0]), r["states"][-1])
        nex5 = np.array(nTq, float)
        e5 = float(np.sum(np.abs(fv - nex5)) / np.sum(np.abs(nex5)))
        sT = float(np.sum(np.abs(np.array(nT3) - nex5)) / np.sum(np.abs(nex5)))
        s0 = float(np.sum(np.abs(np.array(n03) - np.array(n0q))) / np.sum(np.abs(np.array(n0q))))
        quad = {"error_gl3": errs[0], "error_gl5": e5, "difference": abs(errs[0] - e5),
                "state_difference_T": sT, "state_difference_t0": s0,
                "ok": bool(abs(errs[0] - e5) <= 0.25 * errs[0] and sT <= 0.02 * errs[0] and s0 <= 0.02 * errs[0])}
        ok &= quad["ok"]
    else:
        ok = False
    dec = all(math.isfinite(x) for x in errs) and all(a > b for a, b in zip(errs[:-1], errs[1:]))
    orders = [math.log2(a / b) for a, b in zip(errs[:-1], errs[1:])] if dec else []
    lo, hi = th["order_window"]
    ok &= dec and bool(orders) and lo <= orders[-1] <= hi
    return {"pass": bool(ok) and not quick, "values": {"rows": rows, "errors": errs, "orders": orders,
                                                       "strictly_decreasing": dec, "quadrature": quad},
            "threshold": th, "setup": CONV}


# ---------------------------------------------------------------------------
# C6 / C7: VG-1 and VG-3 on a non-autonomous triaxial scenario
# ---------------------------------------------------------------------------


def vg_setup():
    q = O.uniform_edges(-3.0, 2.0, 12)
    mu = O.uniform_mu_edges(6)
    phi = O.uniform_phi_edges(4, 0.3)
    lay = Lay3(q, mu, phi)
    g = lay.grid
    y = [0.0] * lay.n
    for i in range(lay.nq):
        qc = 0.5 * (q[i] + q[i + 1])
        for j in range(lay.nm):
            mc = 0.5 * (mu[j] + mu[j + 1])
            st = math.sqrt((1.0 - mc) * (1.0 + mc))
            for k in range(lay.nphi):
                pc = 0.5 * (phi[k] + phi[k + 1])
                n1, n2 = st * math.cos(pc), st * math.sin(pc)
                y[lay.cell(i, j, k)] = g.dq[i] * g.dm[j] * g.dphi[k] * math.exp(-(qc * qc) / (2.0 * 0.36)) \
                    * (1.0 + 0.2 * mc + 0.3 * n1 + 0.1 * n1 * n2)
    base = dict(layout=lay.layout("comoving", "outflow"),
                geometry={"kind": "power_law", "hh1": 0.5, "hh2": 0.2, "hh3": -0.1}, gamma=0.5,
                absorber={"k0": 0.5, "q_chi": -0.4})
    return lay, y, base


def c67_submit(runner):
    lay, y, base = vg_setup()
    futs = {}
    for arm in TH["C6"]["arms"]:
        for rtol in TH["C6"]["rtols"]:
            req = integrate_req(base["layout"], base["geometry"], base["gamma"], base["absorber"], arm, y, 3.0,
                                [0.0, 1.5, 3.0], rtol, lay.atol(rtol * 1e-2))
            futs[("alloc", arm, rtol)] = runner.submit(f"C6_{arm}_rtol{rtol:.0e}", req, mode="allocation")
        req = integrate_req(base["layout"], base["geometry"], base["gamma"], base["absorber"], arm, y, 3.0,
                            [0.0, 1.5, 3.0], 1e-7, lay.atol(1e-9))
        futs[("rep0", arm)] = runner.submit(f"C7_{arm}_repeat0", req)
        futs[("rep1", arm)] = runner.submit(f"C7_{arm}_repeat1", req)
    return futs


def run_c6(futs):
    th = TH["C6"]
    vals, ok = {}, True
    for arm in th["arms"]:
        r1, r2 = (futs[("alloc", arm, rt)].result() for rt in th["rtols"])
        a1, a2 = r1.get("allocations") or {}, r2.get("allocations") or {}
        d1, d2 = r1.get("details") or {}, r2.get("details") or {}
        at1, at2 = d1.get("attempts") or 0, d2.get("attempts") or 0
        good = bool(r1.get("status") == "OK" and r2.get("status") == "OK" and a1.get("calibrated")
                    and a2.get("calibrated") and a1.get("integrate") == a2.get("integrate")
                    and at2 >= th["attempt_ratio_min"] * max(at1, 1))
        if arm == "dense":
            good &= d1.get("lu") == th["dense_lu"] and d2.get("lu") == th["dense_lu"]
        vals[arm] = {"allocations": [a1.get("integrate"), a2.get("integrate")], "attempts": [at1, at2],
                     "lu": [d1.get("lu"), d2.get("lu")], "rejected_steps": [d1.get("rejected_steps"),
                                                                           d2.get("rejected_steps")], "ok": good}
        ok &= good
    return {"pass": bool(ok), "values": vals, "threshold": th}


def trajectory_drift(states):
    t0 = math.fsum(states[0])
    return max(abs(math.fsum(y) - t0) for y in states) / abs(t0)


def run_c7(futs, c3f, c4f):
    th = TH["C7"]
    vals, ok = {}, True
    for arm in th["repeat_bitwise"]:
        p0, p1 = futs[("rep0", arm)].result(), futs[("rep1", arm)].result()
        good = bool(p0.get("status") == "OK" and p1.get("status") == "OK" and bits_equal(p0["states"], p1["states"])
                    and json.dumps(p0.get("counters"), sort_keys=True) == json.dumps(p1.get("counters"), sort_keys=True))
        vals[arm] = {"bitwise": good, "attempts": (p0.get("details") or {}).get("attempts")}
        ok &= good
    runs = {}
    for (sid, arm), (f3, f2) in c3f.items():
        runs[f"C3_{sid}_{arm}_3d"], runs[f"C3_{sid}_{arm}_2d"] = f3.result(), f2.result()
    for (sid, arm), f in c4f.items():
        runs[f"C4_{sid}_{arm}"] = f.result()
    for key, f in futs.items():
        runs["C6_C7_" + "_".join(map(str, key))] = f.result()
    drifts = {}
    for tag, r in runs.items():
        st = r.get("states") or []
        if r.get("status") != "OK" or len(st) < 2:
            drifts[tag] = None
            ok = False
            continue
        drifts[tag] = trajectory_drift(st)
    worst = finite_max(list(drifts.values()))
    inv_ok = le(worst, th["trajectory_invariant_rel"])
    vals["trajectory_invariant"] = {"runs": len(drifts), "max_rel_drift": worst, "per_run": drifts, "ok": inv_ok}
    ok &= inv_ok
    return {"pass": bool(ok), "values": vals, "threshold": th}


# ---------------------------------------------------------------------------
# C8: typed refusals
# ---------------------------------------------------------------------------


def oracle_code(fn):
    try:
        fn()
    except O.FullSphereRefusal as e:
        return e.code
    return None


def c8_cases():
    lay, y, base = vg_setup()
    good_layout = base["layout"]
    q, mu, phi = lay.q, lay.mu, lay.phi
    cases = []

    def add(name, req, expected):
        cases.append((name, req, expected))

    def req_with(**kw):
        r = integrate_req(kw.get("layout", good_layout), kw.get("geometry", base["geometry"]), kw.get("gamma", base["gamma"]),
                          kw.get("absorber", base["absorber"]), "banded", kw.get("y", y), 3.0, [0.0, 3.0], 1e-6,
                          kw.get("atol", lay.atol(1e-8)))
        if "t_span" in kw:
            r["t_span"] = kw["t_span"]
            r["output_times"] = kw["t_span"]
        return r
    bad_phi = [0.0, 3.0]
    add("phi_not_full_period", req_with(layout=dict(good_layout, phi_edges=bad_phi)),
        oracle_code(lambda: O.SphereGrid(q, mu, bad_phi)))
    bad_mu = [-0.9] + mu[1:]
    add("mu_not_full_sphere", req_with(layout=dict(good_layout, mu_edges=bad_mu)),
        oracle_code(lambda: O.SphereGrid(q, bad_mu, phi)))
    bad_q = [q[1], q[0]] + q[2:]
    add("q_not_increasing", req_with(layout=dict(good_layout, q_edges=bad_q)),
        oracle_code(lambda: O.SphereGrid(bad_q, mu, phi)))
    bad_phi2 = [phi[0], phi[2], phi[1]] + phi[3:]
    add("phi_not_increasing", req_with(layout=dict(good_layout, phi_edges=bad_phi2)),
        oracle_code(lambda: O.SphereGrid(q, mu, bad_phi2)))
    add("negative_gamma", req_with(gamma=-1.0), oracle_code(lambda: O._check_rate(-1.0)))
    probe = {"mode": "probe", "layout": good_layout, "geometry": base["geometry"], "gamma": base["gamma"],
             "absorber": base["absorber"], "drop_partial_t": False, "t": 0.0, "states": [y[:-1]]}
    add("state_length_probe", probe, oracle_code(lambda: O.grid_state_rhs(lay.grid, y[:-1], 0.1, 0.2, 0.3)))
    add("power_law_domain", req_with(t_span=[-2.0, -1.5]), "geometry.domain")
    add("negative_k0", req_with(absorber={"k0": -0.5, "q_chi": -0.4}), "absorber.invalid")
    return cases


def c8_submit(runner):
    cases = c8_cases()
    return [(name, exp, runner.submit(f"C8_{name}", req, keep="full")) for name, req, exp in cases]


def run_c8(futs):
    th = TH["C8"]
    vals, ok = {}, True
    for name, exp, fut in futs:
        r = fut.result()
        good = r.get("status") == th["status"] and exp is not None and r.get("code") == exp
        vals[name] = {"status": r.get("status"), "code": r.get("code"), "expected_code": exp,
                      "reason": (r.get("reason") or "")[:200], "ok": bool(good)}
        ok &= good
    return {"pass": bool(ok), "values": vals, "threshold": th}


# ---------------------------------------------------------------------------
# Cargo contract tests
# ---------------------------------------------------------------------------


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
    # Test names repeat across binaries (the P02 and P06 contracts share sanity tests): key by binary.
    running = re.findall(r"Running tests/(\w+)\.rs", err)
    blocks = re.split(r"^running \d+ tests?\s*$", out, flags=re.M)[1:]
    names, tests, dup = [], {}, []
    for binary, block in zip(running, blocks):
        for a, b in re.findall(r"^test (\S+) \.\.\. (\S+)\s*$", block, flags=re.M):
            key = f"{binary}::{a}"
            names.append((key, b))
            if key in tests:
                dup.append(key)
            tests[key] = b
    common_ok = bool(rc == 0 and failed == 0 and ignored == 0 and passed > 0 and len(lines) == len(binaries)
                     and len(blocks) == len(binaries)
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
    build, binaries = build_runners(3600, raw)
    LOG(f"build exit={build['exit_code']}")
    bin_sha = {k: (sha256_file(v) if v.is_file() else None) for k, v in binaries.items()}
    results = {"node": "BASS/rei P06 (scoped)", "mode": "quick" if args.quick else "full",
               "timing_authority": "diagnostic_only", "thresholds": TH, "convergence_setup": CONV,
               "build": build,
               "host": {"platform": platform.platform(), "python": platform.python_version(), "numpy": np.__version__,
                        "cpu_count": os.cpu_count(), "rustc": rustc_version(),
                        "blas_threads": {v: os.environ.get(v) for v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS",
                                                                          "MKL_NUM_THREADS")}},
               "hold": ["Rust polarized transport (the (I, Q) contract is oracle-only)", "tilt/aberration", "recoil",
                        "class-B geometries", "structured sphere stage solver", "sphere AMR",
                        "live BASS/rei integration", "production", "wall-time claims"]}
    gates = {}
    if build["exit_code"] != 0 or not build["binaries_exist"]:
        results["verdict"] = "INVALID"
        results["reason"] = "runner build failed"
    else:
        with tempfile.TemporaryDirectory() as work:
            runner = Runner(binaries, work, raw, args.jobs, args.timeout)
            cases = c2_cases()
            c2f = c2_submit(runner, cases)
            c3f = c3_submit(runner)
            c4f = c4_submit(runner)
            c67f = c67_submit(runner)
            c8f = c8_submit(runner)
            c5f, c5ex, c5lev = c5_submit(runner, args.quick, raw)
            gates["C1"] = run_c1(raw)
            LOG(f"C1 pass={gates['C1']['pass']}")
            for gid, fn in (("C2", lambda: run_c2(c2f, cases)), ("C3", lambda: run_c3(c3f)),
                            ("C4", lambda: run_c4(c4f)), ("C5", lambda: run_c5(c5f, c5ex, c5lev, args.quick)),
                            ("C6", lambda: run_c6(c67f)), ("C7", lambda: run_c7(c67f, c3f, c4f)), ("C8", lambda: run_c8(c8f))):
                try:
                    gates[gid] = fn()
                except Exception:
                    gates[gid] = {"pass": False, "error": traceback.format_exc()[-3000:]}
                LOG(f"{gid} pass={gates[gid]['pass']}")
            runner.pool.shutdown(wait=True)
            results["H"] = {"all_runs": runner.all_runs, "timing_authority": "diagnostic_only"}
            results["incomplete_requests"] = runner.incomplete
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
                 and all(v is not None for v in bin_sha.values())
                 and all(sha256_file(binaries[k]) == v for k, v in bin_sha.items()))
    incomplete = results.get("incomplete_requests") or []
    results["provenance"] = {"inputs": prov1, "binary_sha256": bin_sha, "watched": list(WATCHED),
                             "git_start": git0, "git_end": git_info(),
                             "tree_changes_start": changes0, "tree_changes_end": changes1,
                             "git_error": [gerr0, gerr1], "inputs_changed_during_run": prov0 != prov1, "clean": clean}
    order = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8"]
    results["gates"] = gates
    results["gate_table"] = {g: bool(gates.get(g, {}).get("pass")) for g in order}
    if results.get("verdict") != "INVALID":
        if not clean:
            results["verdict"] = "INVALID"
            results["reason"] = "dirty tree, changed inputs or stale binary"
        elif incomplete:
            results["verdict"] = "INVALID"
            results["reason"] = (f"incomplete run: {len(incomplete)} request(s) timed out, failed in their environment "
                                 "or exited nonzero")
        elif args.quick or args.skip_cargo:
            results["verdict"] = "INCONCLUSIVE"
            results["reason"] = "quick/skip-cargo mode"
        else:
            results["verdict"] = "PASS" if all(results["gate_table"].values()) else "FAIL"
    results["runtime_s"] = time.perf_counter() - t_start
    results["raw_dir"] = str(raw)
    results["raw_manifest"] = write_manifest(raw)
    results["raw_bytes"] = sum(p.stat().st_size for p in raw.rglob("*") if p.is_file())
    out.write_text(json.dumps(sanitize(results), indent=1) + "\n")
    LOG(f"verdict {results['verdict']} gates {results['gate_table']} -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
