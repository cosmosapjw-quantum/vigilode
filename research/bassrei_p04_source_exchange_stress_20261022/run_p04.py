#!/usr/bin/env python3
"""Gate runner of BASS/rei DAG node P04 (source-owned H/He photoionization,
Case-A recombination, signed stresses and dynamic shear on the LRS photon
transport adapter, run through the RODAS5P fast dense driver).

    python3 run_p04.py --out results.json [--quick] [--skip-cargo]

Builds the runner (`cargo build --profile measurement -p rodas5p-integrators
--example lrs_photon_chem_run --locked --offline -j 8`), drives it, the
independent Python reference (`reference/p04_reference.py`), the source
owner's own rate code (read-only, bound by `SOURCE_BINDING.json`) and the
Rust contract tests, and writes one JSON with every number, per-gate `{pass,
values, threshold}` and `verdict = PASS` iff every [C] gate passes and the
tree is clean. Thresholds are the frozen constants in TH. Wall times are
`diagnostic_only`.

INVALID: a change under `crates/` or this node directory (other than the
harness's own outputs) at start or end, a recorded input changed during the
run, a stale binary, or a source file whose sha256 differs from
`SOURCE_BINDING.json`. `raw/` must be absent or empty at start.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse  # noqa: E402
import dataclasses  # noqa: E402
import hashlib  # noqa: E402
import importlib.util  # noqa: E402
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
from scipy.integrate import solve_ivp  # noqa: E402
from scipy.special import erf  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "reference"))
import p04_reference as P  # noqa: E402

REPO = HERE.parents[1]
TABLE = HERE / "source" / "owner_grackle_rates_129.csv"
BINDING = HERE / "SOURCE_BINDING.json"
EPS = float(np.finfo(float).eps)

TH = {
    "C1": {"verner_rel": 1e-13, "verner_zero": "exact zeros agree", "pchip_rel": 1e-12,
           "pchip_temperatures": 20001, "verner_energies": 4001,
           "refusal": "T_min, T_max accepted; next_down(T_min), next_up(T_max), 1e4 refused (CASE_A_T_DOMAIN) by the "
                      "Rust runner and by the source CaseARates",
           "derivative_rel_vs_reference": 1e-12, "table_nodes": "all 129 node temperatures are probed as well",
           "log_slope_abs": 1e-12,
           "log_slope_def": "|dv/dT_rust - dv/dT_x| T / v_x over the probed and node temperatures, x = the source "
                            "CaseARates (v exp(interp.derivative()(ln T)) / T) and the reference",
           "rec_step_limit_H": "source rec_step events / dt (Richardson, dt = 1e-6 and 5e-7 recombination times) vs "
                               "the runner's RE, E_esc and N_HII rows of an ionized photon-free state (reported only)",
           "table_sha256_matches_spec": True},
    "C2": {"states": 60, "seed": 2026102209, "invariant_rel": 1e-14,
           "invariant_rel_note": "the P02 C3 bound (1e-14 x the sum of |terms|), a priori; the plan's 1e-15 lies below "
                                 "float attainability (pilot 5.0 eps on another seed; disclosed deviation)",
           "reference_unittests": "python3 -m unittest discover -s reference -p 'test_*.py' exits 0",
           "rhs_parity_componentwise": 1e-14, "jacobian_parity": 1e-12,
           "partial_t_parity": 1e-12,
           "invariant_def": "|fsum_k w_k f_k| / sum_k |w_k| abs_k, abs_k = sum of |terms| of slot k (reference)",
           "rhs_parity_def": "max_k |f_rust,k - f_ref,k| / abs_k",
           "jacobian_parity_def": "max_rc |J_rust - J_ref|_rc v_c / sum_c |J_ref|_rc v_c, v_c = max(|y_c|, 1e-3 "
                                  "scale_c): every entry weighted by the state it multiplies",
           "partial_t_parity_def": "photon slots: max |d| / max |ft_ref| over photon slots; each matter slot: "
                                   "|d| / |ft_ref,k| (exact zero required where ft_ref,k = 0)",
           "partial_t_photon_per_slot_H": "max |d_k| / |ft_ref,k| over photon slots with |ft_ref,k| >= 1e-6 max "
                                          "(reported only)"},
    "C3": {"relative": 1e-11, "number_per_H": 5e-12, "energy_eV_per_H": 5e-11,
           "scope": "every OK trajectory of C6, C7 and C9 (all returned outputs)"},
    "C4": {"levels": [4, 8, 16, 32, 64], "order_window": [0.8, 1.25], "order_pair": "G4->G5", "monotone": True,
           "q_center": "0.5 (ln 13.60 + ln 100)", "sigma_q": 0.15, "a2": 0.5, "s_over_H": [0.1, -0.2],
           "def": "|w_disc - w_exact| / |w_exact|, w_disc = rhs[W_gamma], w_exact = H E_c + 2 s D_c of the exactly "
                  "projected smooth photon distribution (q and mu refined together)"},
    "C5": {"amps": [1e30, 1e22],
           "cancellation": "with D_m = -D_gamma (D_gamma equal bitwise to the runner's diagnostic): rate_s == (-3 H) s "
                           "bitwise, H the runner's background H, at each amp",
           "norm_sum_control": "D_m = +D_gamma: |rate_s + 3Hs| > 1e6 x 4 eps (K_D amp |D| a^-3 + |3Hs|) at each amp",
           "bitwise_pairs": ["N_HI == -N_HII (neutral gas with photons)",
                             "U == -W_m (neutral gas, no photons, D_m = -2 tau_m s U: adiabatic and shear work)",
                             "N_HI == -N_HII (ionized gas with photons: photoionization and recombination)"],
           "pairs_at": "each amp"},
    "C6": {"scenarios": ["S", "R"], "rtols": [1e-4, 1e-6, 1e-8], "strict_decrease": True, "reference_rtol": 1e-12,
           "reference_uncertainty_rtol": 1e-11, "reference_uncertainty_over_min_error_max": 0.25,
           "error_def": "max over outputs t>0 and observables of the C7 errors"},
    "C7": {"scenarios": ["N1", "S", "A", "R"], "rung_rtol": 1e-8, "B_app": 1e-6, "reference_uncertainty_max": 1e-7,
           "H_rung_rtol": 1e-6, "reference_uncertainty_def": "the C7 error of the 1e-11 reference against the 1e-12 one",
           "unfloored_H": "plain relative errors per observable (reported only)",
           "observables": {"n_gamma": "rel, floor 1e-3 n_gamma(0)", "E_gamma": "rel, floor 1e-3 E_gamma(0)",
                           "D_over_E": "abs / max(|D/E|, 1e-2), weighted by min(1, E_ref / (1e-3 E_gamma(0))): the "
                                       "plan's ratio, with the same depletion floor as n and E",
                           "D_gamma": "rel, floor 1e-3 |D_gamma(0)|", "D_m": "rel, floor 1e-3 max_t |D_m,ref|",
                           "E_esc": "rel, floor 1e-3 U(0)",
                           "x_HII": "rel, floor 1e-3", "x_HeII": "rel, floor 1e-3",
                           "x_HeIII": "rel, floor 1e-3", "U": "rel", "s_over_H": "rel, floor 1e-3 |s0/H0|"}},
    "C8": {"scenario": "HOT", "max_attempts": 3000, "status": "NUMERICAL_FAILURE with a DriverPartial",
           "partial_T_window": [0.99, 1.0], "ledger_rel": 1e-11, "domain_refusals_min": 1,
           "number_per_H": 5e-12, "energy_eV_per_H": 5e-11, "reasons": ["attempt-budget", "minimum-step"],
           "last_failure": ["non-finite", "None"],
           "note": "each CASE_A refusal allocates its message String on the rejection path (disclosed)"},
    "C9": {"alloc_scenarios": ["N1", "S"], "rtols": [1e-6, 1e-9], "attempt_ratio_min": 2.0, "lu": "InPlaceZeroSkipping",
           "repeat_bitwise": True},
    "cargo": {"binaries": ["lrs_photon_chem_contracts", "lrs_photon_fv_contracts", "generic_scope_contracts"],
              "gate_tests": {
                  "C1": ["a_case_a_pchip_reproduces_nodes_is_c1_monotone_and_refuses_outside_the_domain",
                         "b_verner_cross_sections_cell_averages_and_the_quadrature_rule"],
                  "C2": ["c_rhs_preserves_the_five_ledgers_to_rounding",
                         "d_jacobian_matches_richardson_differences_and_refills_exactly",
                         "e_partial_t_matches_richardson_differences_in_t",
                         "pins_transport_matter_rows_and_initial_data"],
                  "C8": ["h_invalid_declarations_shapes_and_temperatures_are_typed_refusals",
                         "h_a_run_leaving_the_table_ends_in_a_partial_at_the_boundary"],
                  "C9": ["f_callbacks_and_the_dense_driver_allocate_independently_of_the_step_count",
                         "g_runs_are_deterministic_and_uniform_vector_atol_is_scalar",
                         "generic_solver_core_does_not_embed_physical_client_bindings"]}},
}

GRID = (16, 16, 16, 8)
BASE = dict(nH0=2.4e-5, H0=3e-17, tau_m=1e14, amp=1.0, xHI0=0.6, fHeI=0.2, fHeII=0.5, T0=12000.0, Dm0=0.0,
            s0_over_H0=0.1, photons_per_H=0.05, alpha=1.5, a2=0.5)
SCEN = {
    "N1": dict(BASE, T=1e12),
    "S": dict(BASE, nH0=2.4e-3, T=1e13),
    "A": dict(BASE, amp=1e15, T=1e12),
    "HOT": dict(BASE, T0=20000.0, photons_per_H=2.0, alpha=0.5, T=1e12),
    "R": dict(BASE, nH0=5.0, T0=14000.0, tau_m=1e10, T=1e12),
}
N_OUT = 8


def outputs_of(sid):
    """Uniform outputs; S (all photons absorbed early) uses log-spaced ones."""
    T = SCEN[sid]["T"]
    if sid == "S":
        return [0.0] + [T * 10.0 ** (-3.5 + 0.5 * k) for k in range(N_OUT)]
    return [T * k / N_OUT for k in range(N_OUT + 1)]


def params_of(sid):
    s = dict(SCEN[sid])
    T = s.pop("T")
    return P.Params(**s), T


def model_of(sid, grid=GRID):
    p, T = params_of(sid)
    return P.make_model(p, grid), T


def request(sid, rtol, arm="dense", grid=GRID, **extra):
    p, T = params_of(sid)
    m = P.make_model(p, grid)
    y0 = m.initial_state()
    atol = [float(rtol * 1e-6 * v) for v in m.scale_vector(y0)]
    req = {"table_path": str(TABLE), "grid": dict(zip(("n1", "n2", "n3", "nm"), grid)),
           "params": {"H0": p.H0, "amp": p.amp, "tau_m": p.tau_m},
           "initial": {k: getattr(p, k) for k in ("nH0", "xHI0", "fHeI", "fHeII", "T0", "Dm0", "s0_over_H0",
                                                    "photons_per_H", "alpha", "a2")},
           "mode": "integrate", "arm": arm, "t_span": [0.0, T],
           "output_times": outputs_of(sid), "rtol": rtol, "atol": atol}
    req.update(extra)
    return req


def probe_request(sid, t, states, grid=GRID, **extra):
    req = request(sid, 1e-6, grid=grid)
    for k in ("output_times", "t_span", "rtol", "atol", "arm"):
        req.pop(k, None)
    req.update({"mode": "probe", "t": float(t), "states": [[float(v) for v in s] for s in states]})
    req.update(extra)
    return req


# ---------------------------------------------------------------------------
# Helpers (as the P03 harness)
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


def aligned(resp, outputs):
    times = resp.get("times") or []
    states = resp.get("states") or []
    times_ok = len(times) == len(outputs) and all(float(a) == float(b) for a, b in zip(times, outputs))
    ok = resp.get("status") == "OK" and times_ok and len(states) == len(outputs)
    return bool(ok), {"status": resp.get("status"), "reason": (resp.get("reason") or "")[:200],
                      "times_match_requested": bool(times_ok), "n_states": len(states)}


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
    "chem_rs": f"{SUPPORT}/chem.rs",
    "runner_rs": "crates/rodas5p-integrators/examples/lrs_photon_chem_run.rs",
    "test_lrs_photon_chem_contracts_rs": "crates/rodas5p-integrators/tests/lrs_photon_chem_contracts.rs",
    "rodas5p_fast_rs": "crates/rodas5p-integrators/src/rodas5p_fast.rs",
    "run_p04_py": str((HERE / "run_p04.py").relative_to(REPO)),
    "p04_reference_py": str((HERE / "reference" / "p04_reference.py").relative_to(REPO)),
    "model_md": str((HERE / "MODEL.md").relative_to(REPO)),
    "case_a_table_copy": str(TABLE.relative_to(REPO)),
    "source_binding_json": str(BINDING.relative_to(REPO)),
    "cargo_lock": "Cargo.lock",
}


def crate_sources_digest():
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


def source_binding():
    """Every bound source file must exist with its recorded sha256."""
    b = json.loads(BINDING.read_text())
    root = Path(b["source_root"])
    rows, ok = [], True
    for f in b["files"]:
        p = root / f["path"]
        got = sha256_file(p) if p.is_file() else None
        good = got == f["sha256"]
        ok &= good
        rows.append({"path": f["path"], "role": f["role"], "sha256": f["sha256"], "observed": got, "ok": good})
    copy_ok = sha256_file(TABLE) == b["case_a_table_sha256"]
    return bool(ok and copy_ok), {"source_root": str(root), "files": rows, "node_table_copy_matches": copy_ok}


def load_source_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod  # dataclasses resolve their module through sys.modules
    spec.loader.exec_module(mod)
    return mod


def rustc_version():
    try:
        return subprocess.run(["rustc", "-V"], env=cargo_env(), capture_output=True, text=True,
                              timeout=60).stdout.strip()
    except Exception as e:  # pragma: no cover
        return f"unavailable: {e}"


def cargo_env():
    env = dict(os.environ)
    env["RUSTUP_TOOLCHAIN"] = "1.94.1"
    env.setdefault("CARGO_TARGET_DIR", "/home/cosmosapjw/vigilode-bassrei-target")
    env["CARGO_TARGET_DIR"] = str((REPO / env["CARGO_TARGET_DIR"]).resolve())
    return env


def build_runner(timeout_s, raw_dir):
    env = cargo_env()
    cmd = ["cargo", "build", "--profile", "measurement", "-p", "rodas5p-integrators", "--example",
           "lrs_photon_chem_run", "--locked", "--offline", "-j", "8"]
    binary = Path(env["CARGO_TARGET_DIR"]) / "measurement" / "examples" / "lrs_photon_chem_run"
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
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=self.timeout, cwd=str(REPO))
            try:
                resp = json.loads(proc.stdout)
            except Exception:
                resp = {"status": "ENVIRONMENT_FAILURE", "reason": "unparseable stdout; stderr: " + proc.stderr[-2000:]}
            resp["_returncode"] = proc.returncode
        except subprocess.TimeoutExpired:
            resp = {"status": "TIMEOUT", "reason": f"runner exceeded {self.timeout:.0f}s"}
        resp["_wall_s"] = time.perf_counter() - t0
        compact_req = {k: v for k, v in req.items() if k not in ("atol", "states")}
        if "atol" in req:
            compact_req["atol_sha256"] = hashlib.sha256(json.dumps(req["atol"]).encode()).hexdigest()
        if "states" in req:
            compact_req["probe_states_sha256"] = [hashlib.sha256(json.dumps(s).encode()).hexdigest()
                                                  for s in req["states"]]
        compact_resp = dict(resp)
        if isinstance(compact_resp.get("states"), list) and compact_resp["states"] and \
                isinstance(compact_resp["states"][0], dict):
            compact_resp["states"] = [{k: v for k, v in s.items() if k != "jacobian"} for s in compact_resp["states"]]
        with self.lock:
            self.all_runs[tag] = {"tag": tag, "status": resp.get("status"), "reason": (resp.get("reason") or "")[:300],
                                  "wall_s": resp["_wall_s"], "timing_authority": "diagnostic_only",
                                  "counters": resp.get("counters"), "details": resp.get("details"),
                                  "allocations": resp.get("allocations")}
        (self.raw / f"{tag}.json").write_text(json.dumps(sanitize({"request": compact_req, "response": compact_resp})))
        path.unlink(missing_ok=True)
        return resp


# ---------------------------------------------------------------------------
# Observables and references
# ---------------------------------------------------------------------------


def observables(m, t, y):
    y = np.asarray(y, float)
    d = m.diagnostics(t, y)
    g = m.grid
    return {"n_gamma": d["n_gamma"], "E_gamma": d["E_gamma"], "D_over_E": d["D_gamma"] / d["E_gamma"],
            "D_gamma": d["D_gamma"], "D_m": float(y[g.IDm]), "E_esc": float(y[g.IEesc]),
            "x_HII": d["x_HII"], "x_HeII": d["x_HeII"], "x_HeIII": d["x_HeIII"], "U": float(y[g.IU]),
            "s_over_H": d["s_over_H"]}


def bapp_error(m, outs, states, ref, unfloored=None):
    """Max over outputs t > 0 and observables of the TH['C7'] errors; with a
    dict `unfloored`, also the plain relative errors (report only)."""
    o0 = observables(m, outs[0], ref[0])
    dm_max = max(abs(observables(m, t, r)["D_m"]) for t, r in zip(outs, ref))
    floors = {"n_gamma": 1e-3 * abs(o0["n_gamma"]), "E_gamma": 1e-3 * abs(o0["E_gamma"]), "D_over_E": 1e-2,
              "D_gamma": 1e-3 * abs(o0["D_gamma"]), "D_m": 1e-3 * dm_max, "E_esc": 1e-3 * abs(o0["U"]),
              "x_HII": 1e-3, "x_HeII": 1e-3, "x_HeIII": 1e-3, "U": 0.0, "s_over_H": 1e-3 * abs(o0["s_over_H"])}
    worst, per = 0.0, {}
    for t, y, r in zip(outs[1:], states[1:], ref[1:]):
        a, b = observables(m, t, y), observables(m, t, r)
        for k in a:
            den = max(abs(b[k]), floors[k])
            e = abs(a[k] - b[k]) / den if den > 0 else (0.0 if a[k] == b[k] else float("inf"))
            if k == "D_over_E":
                e *= min(1.0, b["E_gamma"] / (1e-3 * abs(o0["E_gamma"])))
            per[k] = max(per.get(k, 0.0), e)
            worst = max(worst, e)
            if unfloored is not None:
                u = abs(a[k] - b[k]) / abs(b[k]) if b[k] != 0 else (0.0 if a[k] == b[k] else float("inf"))
                unfloored[k] = max(unfloored.get(k, 0.0), u)
    return worst, per


def radau_restarted(sid, rtol, grid=GRID):
    """Radau with the analytic dense Jacobian, restarted at every output."""
    m, T = model_of(sid, grid)
    y = m.initial_state()
    atol = np.array([rtol * 1e-6 * v for v in m.scale_vector(y)])
    outs = outputs_of(sid)
    states, nfev = [y.copy()], 0
    t0 = time.perf_counter()
    for a, b in zip(outs[:-1], outs[1:]):
        sol = solve_ivp(m.rhs, (a, b), y, method="Radau", jac=m.jacobian, rtol=rtol, atol=atol)
        if not sol.success:
            raise RuntimeError(f"Radau {sid} {rtol}: {sol.message}")
        y = sol.y[:, -1].copy()
        states.append(y)
        nfev += sol.nfev
    return np.array(states).tolist(), {"nfev": int(nfev), "wall_s": time.perf_counter() - t0, "rtol": rtol}


# ---------------------------------------------------------------------------
# Gates
# ---------------------------------------------------------------------------


def c1_submit(runner):
    th = TH["C1"]
    ca = P.CaseA(TABLE)
    T = np.geomspace(ca.T_min, ca.T_max, th["pchip_temperatures"])
    T[0], T[-1] = ca.T_min, ca.T_max
    edge = [ca.T_min, ca.T_max, float(np.nextafter(ca.T_min, 0)), float(np.nextafter(ca.T_max, np.inf)), 1e4]
    nodes = [float(x) for x in ca.T]
    E = list(np.geomspace(13.0, 120.0, th["verner_energies"]))
    for f in P.FIT:
        E += [f[0], float(np.nextafter(f[0], 0)), float(np.nextafter(f[0], np.inf))]
    m, _ = model_of("N1", (3, 3, 3, 4))
    g = m.grid
    yr = m.initial_state().copy()  # ionized, photon-free: the species rows are recombination only
    for row in g.cells:
        for c in row:
            yr[c] = 0.0
    req = probe_request("N1", 0.0, [m.initial_state(), yr], grid=(3, 3, 3, 4),
                        temperatures=[float(x) for x in T] + nodes + edge, energies=[float(e) for e in E])
    return runner.submit("C1_probe_sources", req), (T, nodes, edge, E, yr, m)


def rec_step_limit(root, m, y, rhs, a_inv3):
    """[H] The source's own Case-A step: events / dt as dt -> 0 against the
    runner's recombination rows (proper densities = comoving x a^-3)."""
    c11 = root / "source07_c11_casea_20261015"
    sys.path.insert(0, str(c11))
    try:
        cm = load_source_module("src_c11_casea", c11 / "casea.py")
    finally:
        sys.path.remove(str(c11))
    g = m.grid
    f = a_inv3
    gas = cm.c10.Gas((y[g.IHI] + y[g.IHII]) * f, (y[g.IHeI] + y[g.IHeII] + y[g.IHeIII]) * f, y[g.IHI] * f,
                     y[g.IHeI] * f, y[g.IHeII] * f, y[g.IU] * f)
    provider = cm.CaseARates()
    t_rec = y[g.IHII] / rhs[g.IRe]
    rates = []
    for dt in (1e-6 * t_rec, 5e-7 * t_rec):
        _, info = cm.rec_step(gas, dt, provider)
        ev = np.array(info["recomb_events_cm3"], float)
        rates.append((ev / dt, info["escape_eV_cm3"] / dt))
    (e1, x1), (e2, x2) = rates
    ev, esc = 2.0 * e2 - e1, 2.0 * x2 - x1
    rust = {"RE": rhs[g.IRe] * f, "E_esc": rhs[g.IEesc] * f, "HII_recombination": -rhs[g.IHII] * f}
    src = {"RE": float(ev.sum()), "E_esc": float(esc), "HII_recombination": float(ev[0])}
    return {"t_rec_s": float(t_rec), "rust": rust, "source": src,
            "rel": {k: abs(rust[k] - src[k]) / abs(src[k]) for k in rust}, "authority": "H"}


def run_c1(fut, ctx, binding_info):
    th = TH["C1"]
    T, nodes, edge, E, yr, m1 = ctx
    r = fut.result()
    root = Path(binding_info["source_root"])
    vals = {}
    ok = r.get("status") == "OK"
    c9 = load_source_module("src_c9_experiment", root / "source07_c9_heii_vector_20261015" / "experiment.py")
    rt = load_source_module("src_c11_rate_table", root / "source07_c11_casea_20261015" / "rate_table.py")
    src_sigma = c9.cross_sections(np.array(E))
    rust_sigma = np.array(r.get("verner_sigma") or [], float)
    if rust_sigma.shape == src_sigma.shape:
        pos = src_sigma > 0
        zero_ok = bool(np.all(rust_sigma[~pos] == 0.0))
        rel = float(np.max(np.abs(rust_sigma[pos] - src_sigma[pos]) / src_sigma[pos]))
    else:
        zero_ok, rel = False, float("nan")
    vals["verner"] = {"energies": len(E), "max_rel_vs_source": rel, "zeros_agree": zero_ok,
                      "shape": [list(rust_sigma.shape), list(src_sigma.shape)]}
    ok &= zero_ok and le(rel, th["verner_rel"])
    src = rt.CaseARates(rt.TABLE)
    ref = P.CaseA(TABLE)
    ca = r.get("case_a") or []
    temps = [float(x) for x in T] + nodes
    n_main = len(temps)
    dsrc_poly = src.interp.derivative()
    prel, drel, slope_src, slope_ref, node_rel, n_ok = 0.0, 0.0, 0.0, 0.0, 0.0, 0
    for k, tk in enumerate(temps):
        row = ca[k] if k < len(ca) else {}
        if row.get("status") != "OK":
            prel = float("nan")
            break
        v = np.array(row["values"], float)
        sv = np.asarray(src(tk), float)
        rel_k = float(np.max(np.abs(v - sv) / sv))
        prel = max(prel, rel_k)
        if k >= len(T):
            node_rel = max(node_rel, rel_k)
        dv = np.array(row["dvdT"], float)
        ds = sv * np.asarray(dsrc_poly(math.log(tk)), float) / tk
        rv, rd = ref.rates_and_dT(tk)
        rv, rd = np.asarray(rv, float), np.asarray(rd, float)
        slope_src = max(slope_src, float(np.max(np.abs(dv - ds) * tk / sv)))
        slope_ref = max(slope_ref, float(np.max(np.abs(dv - rd) * tk / rv)))
        if k < len(T):
            drel = max(drel, float(np.max(np.abs(dv - rd) / np.maximum(np.abs(rd), 1e-300))))
        n_ok += 1
    vals["pchip"] = {"temperatures": len(T), "node_temperatures": len(nodes), "ok_rows": n_ok,
                     "max_rel_vs_source_CaseARates": prel, "max_rel_at_nodes_vs_source": node_rel,
                     "max_rel_dvdT_vs_reference": drel, "max_log_slope_abs_vs_source": slope_src,
                     "max_log_slope_abs_vs_reference": slope_ref, "source_table_sha256": src.sha256}
    ok &= (n_ok == n_main and le(prel, th["pchip_rel"]) and le(drel, th["derivative_rel_vs_reference"])
           and le(slope_src, th["log_slope_abs"]) and le(slope_ref, th["log_slope_abs"]))
    expect = [True, True, False, False, False]
    edges = []
    for k, (t_, acc) in enumerate(zip(edge, expect)):
        row = ca[n_main + k] if n_main + k < len(ca) else {}
        rust_ok = row.get("status") == "OK"
        rust_typed = (not rust_ok) and (row.get("message") or "").startswith("CASE_A_T_DOMAIN")
        try:
            src(t_)
            src_ok = True
        except rt.RateDomainError:
            src_ok = False
        good = (rust_ok == acc) and (src_ok == acc) and (acc or rust_typed)
        edges.append({"T": t_, "expected_accepted": acc, "rust_accepted": rust_ok, "rust_typed_refusal": rust_typed,
                      "source_accepted": src_ok, "ok": bool(good)})
        ok &= good
    vals["domain_edges"] = edges
    try:
        st = r.get("states") or []
        vals["rec_step_limit_H"] = rec_step_limit(root, m1, yr, np.array(st[1]["rhs"], float),
                                                  float(r["background"]["a_inv3"]))
    except Exception:
        vals["rec_step_limit_H"] = {"error": traceback.format_exc()[-1500:], "authority": "H"}
    table = (r.get("grid") or {}).get("table") or {}
    vals["table"] = table
    ok &= bool(table.get("sha256_matches_spec"))
    return {"pass": bool(ok), "values": vals, "threshold": th}


def random_states(rng, count):
    """Admissible states around the N1/S/A initial data: lognormal photon and
    species perturbations, T in [1.1e4, 2.2e4] K, signed s and D_m, ledgers."""
    out = []
    for k in range(count):
        sid = ("N1", "S", "A")[k % 3]
        m, T = model_of(sid)
        y = m.initial_state().copy()
        g = m.grid
        cells = [c for row in g.cells for c in row]
        y[cells] *= np.exp(0.3 * rng.standard_normal(len(cells)))
        for idx in (g.IHI, g.IHII, g.IHeI, g.IHeII, g.IHeIII):
            y[idx] *= math.exp(0.3 * rng.standard_normal())
        for idx in [g.LO, g.HI, g.IW, g.IEesc, g.IRe, g.IWm] + list(g.absorbed):
            y[idx] = abs(rng.standard_normal()) * 1e-3 * y[g.IHI]
        Ne = y[g.IHII] + y[g.IHeII] + 2 * y[g.IHeIII]
        Ntot = y[g.IHI] + y[g.IHII] + y[g.IHeI] + y[g.IHeII] + y[g.IHeIII] + Ne
        y[g.IU] = 1.5 * P.KB * rng.uniform(1.1e4, 2.2e4) * Ntot
        y[g.IS] = rng.uniform(-0.3, 0.3) * m.params.H0
        y[g.IDm] = rng.standard_normal() * 1e-2 * y[g.IU]
        out.append((sid, float(rng.uniform(0.0, T)), y))
    return out


def c2_submit(runner):
    rng = np.random.default_rng(TH["C2"]["seed"])
    states = random_states(rng, TH["C2"]["states"])
    futs = [runner.submit(f"C2_probe_{k:02d}_{sid}", probe_request(sid, t, [y])) for k, (sid, t, y) in enumerate(states)]
    return futs, states


def reference_unittests(raw_dir):
    cmd = [sys.executable, "-m", "unittest", "discover", "-s", "reference", "-p", "test_*.py"]
    try:
        p = subprocess.run(cmd, cwd=str(HERE), capture_output=True, text=True, timeout=1800)
        rc, log = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        rc, log = None, "TIMEOUT\n" + str(e.stderr)
    (raw_dir / "reference_unittests.log").write_text(log)
    tail = log.strip().splitlines()[-3:]
    return {"command": " ".join(cmd[1:]), "exit_code": rc, "tail": tail, "log_file": "raw/reference_unittests.log"}


def run_c2(futs, states, unit):
    th = TH["C2"]
    inv, par, jpar, fpar, fslot, per_state = [], [], [], [], [], []
    ok = unit.get("exit_code") == 0
    for fut, (sid, t, y) in zip(futs, states):
        r = fut.result()
        if r.get("status") != "OK" or not r.get("states"):
            ok = False
            continue
        s0 = r["states"][0]
        m, _ = model_of(sid)
        g = m.grid
        f_ref, abs_ref = m.rhs_with_abs(t, y)
        f_ref, abs_ref = np.array(f_ref, float), np.array(abs_ref, float)
        f = np.array(s0["rhs"], float)
        inv_k = [abs(math.fsum(w * f)) / max(float(np.dot(np.abs(w), abs_ref)), 1e-300)
                 for w in m.invariant_weights().values()]
        inv += inv_k
        par.append(float(np.max(np.abs(f - f_ref) / np.maximum(abs_ref, 1e-300))))
        J = np.array(s0["jacobian"], float)
        Jr = np.asarray(m.jacobian(t, y), float)
        v = np.maximum(np.abs(y), 1e-3 * m.scale_vector(m.initial_state()))
        den = np.abs(Jr) @ v
        num = np.abs(J - Jr) * v[None, :]
        jpar.append(float(np.max(num / np.maximum(den, 1e-300)[:, None])))
        jsha = hashlib.sha256(np.ascontiguousarray(J).tobytes()).hexdigest()
        ft = np.array(s0["partial_t"], float)
        fr = np.array(m.partial_t(t, y), float)
        nph = g.n_ph
        dph = float(np.max(np.abs(ft[:nph] - fr[:nph]))) / max(float(np.max(np.abs(fr[:nph]))), 1e-300)
        dm = [abs(ft[k] - fr[k]) / abs(fr[k]) if fr[k] != 0.0 else (0.0 if ft[k] == 0.0 else float("inf"))
              for k in range(nph, m.n)]
        fpar.append(max([dph] + dm))
        big = np.abs(fr[:nph]) >= 1e-6 * float(np.max(np.abs(fr[:nph])))
        fslot.append(float(np.max(np.abs(ft[:nph] - fr[:nph])[big] / np.abs(fr[:nph])[big])))
        per_state.append({"scenario": sid, "t": t, "invariant_rel": inv_k, "rhs_parity": par[-1],
                          "jacobian_parity": jpar[-1], "partial_t_parity": fpar[-1], "jacobian_f64_sha256": jsha})
    vals = {"states": len(states), "seed": th["seed"], "reference_unittests": unit,
            "max_invariant_rel": finite_max(inv), "max_rhs_parity": finite_max(par),
            "max_jacobian_parity": finite_max(jpar), "max_partial_t_parity": finite_max(fpar),
            "partial_t_photon_per_slot_H": finite_max(fslot), "per_state": per_state}
    ok &= (le(vals["max_invariant_rel"], th["invariant_rel"]) and le(vals["max_rhs_parity"], th["rhs_parity_componentwise"])
           and le(vals["max_jacobian_parity"], th["jacobian_parity"])
           and le(vals["max_partial_t_parity"], th["partial_t_parity"]))
    return {"pass": bool(ok), "values": vals, "threshold": th}


def projected_photons(g, qc, sig, a2):
    """Exact cell integrals of n = exp(-(q-qc)^2/(2 sig^2)) (1 + a2 P2(mu))."""
    qe = np.array(g.q_edges, float)
    me = np.array(g.mu_edges, float)
    Iq = sig * math.sqrt(math.pi / 2.0) * np.diff(erf((qe - qc) / (sig * math.sqrt(2.0))))
    Im = np.diff(me) + a2 * 0.5 * ((me[1:] ** 3 - me[1:]) - (me[:-1] ** 3 - me[:-1]))  # int P2 = (mu^3-mu)/2
    return Iq[:, None] * Im[None, :]


def exact_moments(g, qc, sig, a2):
    """int int e^q n and int int e^q P2 n over the grid."""
    lo, hi = g.q_edges[0], g.q_edges[-1]
    shift = qc + sig * sig
    z = (np.array([lo, hi]) - shift) / (sig * math.sqrt(2.0))
    Ieq = math.exp(qc + sig * sig / 2.0) * sig * math.sqrt(math.pi / 2.0) * float(erf(z[1]) - erf(z[0]))
    return Ieq * 2.0, Ieq * a2 * 0.4  # int(1 + a2 P2) = 2, int P2 (1 + a2 P2) = 2 a2/5


def c4_submit(runner):
    th = TH["C4"]
    qc = 0.5 * (math.log(13.60) + math.log(100.0))
    futs = {}
    for lev in th["levels"]:
        grid = (lev, lev, lev, lev)
        m, _ = model_of("N1", grid)
        g = m.grid
        y = m.initial_state().copy()
        proj = projected_photons(g, qc, th["sigma_q"], th["a2"])
        scale = float(y[[c for row in g.cells for c in row]].sum()) / float(proj.sum())
        for i in range(g.nq):
            for j in range(g.nm):
                y[g.cells[i][j]] = scale * proj[i, j]
        states = []
        for so in th["s_over_H"]:
            ys = y.copy()
            ys[g.IS] = so * m.params.H0
            states.append(ys)
        futs[lev] = (runner.submit(f"C4_probe_level{lev}", probe_request("N1", 0.0, states, grid=grid)), scale)
    return futs, qc


def run_c4(futs, qc):
    th = TH["C4"]
    errs = {so: [] for so in th["s_over_H"]}
    rows = []
    ok = True
    for lev in th["levels"]:
        fut, scale = futs[lev]
        r = fut.result()
        if r.get("status") != "OK":
            ok = False
            continue
        m, _ = model_of("N1", (lev, lev, lev, lev))
        g = m.grid
        E_c, D_c = exact_moments(g, qc, th["sigma_q"], th["a2"])
        E_c, D_c = E_c * scale, D_c * scale
        H = m.geom.H(0.0)
        for so, st in zip(th["s_over_H"], r.get("states") or []):
            w_disc = float(st["rhs"][g.IW])
            w_exact = H * E_c + 2.0 * (so * m.params.H0) * D_c
            e = abs(w_disc - w_exact) / abs(w_exact)
            errs[so].append(e)
            rows.append({"level": lev, "s_over_H": so, "w_disc": w_disc, "w_exact": w_exact, "rel": e})
    orders = {}
    lo, hi = th["order_window"]
    for so, e in errs.items():
        if len(e) != len(th["levels"]):
            ok = False
            continue
        mono = all(a > b for a, b in zip(e[:-1], e[1:]))
        o = [math.log2(a / b) for a, b in zip(e[:-1], e[1:])]
        orders[str(so)] = {"errors": e, "orders": o, "monotone": mono}
        ok &= mono and lo <= o[-1] <= hi
    return {"pass": bool(ok), "values": {"rows": rows, "orders": orders}, "threshold": th}


def c5_states():
    m, T = model_of("N1")
    g = m.grid
    y = m.initial_state().copy()
    yc = y.copy()
    _, _, D = m.photon_moments(yc)
    yc[g.IDm] = -D
    yn = y.copy()
    yn[g.IDm] = D
    nh = m.params.nH0
    nhe = nh * P.Y_HE / (4 * (1 - P.Y_HE))
    y1 = y.copy()
    y1[g.IHI], y1[g.IHII], y1[g.IHeI], y1[g.IHeII], y1[g.IHeIII] = nh, 0.0, nhe, 0.0, 0.0
    y1[g.IU] = 1.5 * P.KB * 15000.0 * (nh + nhe)
    y2 = y1.copy()
    for row in g.cells:
        for c in row:
            y2[c] = 0.0
    y2[g.IS] = 0.2 * m.params.H0
    y2[g.IDm] = -2.0 * m.params.tau_m * y2[g.IS] * y2[g.IU]  # shear work 2 s D_m != 0
    y3 = y.copy()  # the ionized initial gas with photons
    return (yc, yn, y1, y2, y3), 0.3 * T


def c5_submit(runner):
    states, t = c5_states()
    futs = {}
    for amp in TH["C5"]["amps"]:
        req = probe_request("N1", t, states)
        req["params"]["amp"] = amp
        futs[amp] = runner.submit(f"C5_probe_amp{amp:.0e}", req)
    return futs, (states, t)


def run_c5(futs, ctx):
    th = TH["C5"]
    (yc, yn, y1, y2, y3), t = ctx
    p, _ = params_of("N1")
    m = P.make_model(p, GRID)
    g = m.grid
    _, _, D = m.photon_moments(yc)
    s = float(yc[g.IS])
    vals, ok = {}, True
    for amp in th["amps"]:
        r = futs[amp].result()
        if r.get("status") != "OK" or len(r.get("states") or []) != 5:
            vals[f"{amp:.0e}"] = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200]}
            ok = False
            continue
        sc, sn, s1, s2, s3 = (np.array(st["rhs"], float) for st in r["states"])
        H, a3 = float(r["background"]["H"]), float(r["background"]["a_inv3"])
        minus_3hs = -3.0 * H * s
        d_rust = float(r["states"][0]["diagnostics"]["D_gamma"])
        coupling = P.K_D * amp * abs(D) * a3
        bound = 4 * EPS * (coupling + abs(3 * H * s))
        ctrl = abs(sn[g.IS] - minus_3hs)
        d_same = bits_equal(d_rust, D)
        canc = bits_equal(sc[g.IS], minus_3hs)
        pairs = {"N_HI_vs_minus_N_HII_neutral": bits_equal(s1[g.IHI], -s1[g.IHII]) and s1[g.IHI] != 0.0,
                 "U_vs_minus_W_m_shear": bits_equal(s2[g.IU], -s2[g.IWm]) and s2[g.IU] != 0.0,
                 "N_HI_vs_minus_N_HII_ionized": bits_equal(s3[g.IHI], -s3[g.IHII]) and s3[g.IHI] != 0.0}
        vals[f"{amp:.0e}"] = {
            "rate_s": float(sc[g.IS]), "minus_3Hs": minus_3hs, "bitwise": bool(canc),
            "D_gamma_python_equals_runner": bool(d_same), "coupling_K_D_amp_D_a3": coupling,
            "coupling_over_3Hs": coupling / abs(3 * H * s), "norm_sum_control_deviation": ctrl,
            "control_bound": 1e6 * bound,
            "reference_rate_s": float(P.make_model(dataclasses.replace(p, amp=amp), GRID).rhs(t, yc)[g.IS]),
            "bitwise_pairs": {k: bool(v) for k, v in pairs.items()},
            "pair_values": [float(s1[g.IHI]), float(s1[g.IHII]), float(s2[g.IU]), float(s2[g.IWm]),
                            float(s3[g.IHI]), float(s3[g.IHII])],
            "shear_work_2sDm": 2.0 * float(y2[g.IS] * y2[g.IDm])}
        ok &= bool(d_same and canc and ctrl > 1e6 * bound and all(pairs.values()))
    return {"pass": bool(ok), "values": vals, "threshold": th}


def c67_submit(runner, quick):
    th6, th7 = TH["C6"], TH["C7"]
    futs = {}
    rt6 = th6["rtols"][:2] if quick else th6["rtols"]
    for sid in th6["scenarios"]:
        for rtol in rt6:
            futs[(sid, rtol)] = runner.submit(f"C6_{sid}_rtol{rtol:.0e}", request(sid, rtol))
    for sid in th7["scenarios"]:
        for rtol in (th7["rung_rtol"], th7["H_rung_rtol"]):
            if (sid, rtol) not in futs:
                futs[(sid, rtol)] = runner.submit(f"C7_{sid}_rtol{rtol:.0e}", request(sid, rtol))
    return futs, rt6


def references():
    th6, th7 = TH["C6"], TH["C7"]
    jobs = {(sid, r) for sid in set(th6["scenarios"]) | set(th7["scenarios"])
            for r in (th6["reference_rtol"], th6["reference_uncertainty_rtol"])}
    out = {}
    # spawn, not fork: the runner's thread pool is live (fork after threads can deadlock)
    with ProcessPoolExecutor(max_workers=len(jobs), mp_context=multiprocessing.get_context("spawn")) as pool:
        futs = {j: pool.submit(radau_restarted, *j) for j in sorted(jobs)}
        for j, f in futs.items():
            out[j] = f.result()
            LOG(f"reference {j}: {out[j][1]['wall_s']:.1f}s")
    return out


def run_c6(futs, rt6, refs, quick):
    th = TH["C6"]
    vals, ok = {}, True
    for sid in th["scenarios"]:
        m, T = model_of(sid)
        outs = outputs_of(sid)
        ref = refs[(sid, th["reference_rtol"])][0]
        u_ref = bapp_error(m, outs, refs[(sid, th["reference_uncertainty_rtol"])][0], ref)[0]
        errs, rows = [], {}
        for rtol in rt6:
            r = futs[(sid, rtol)].result()
            al, info = aligned(r, outs)
            if not al:
                errs.append(float("nan"))
                rows[f"{rtol:.0e}"] = info
                continue
            e, per = bapp_error(m, outs, r["states"], ref)
            errs.append(e)
            d = r.get("details") or {}
            rows[f"{rtol:.0e}"] = {"error": e, "per_observable": per, "attempts": d.get("attempts"),
                                    "rejected_steps": d.get("rejected_steps"), "lu": d.get("lu")}
        dec = all(math.isfinite(x) for x in errs) and all(a > b for a, b in zip(errs[:-1], errs[1:]))
        u_ok = le(u_ref, th["reference_uncertainty_over_min_error_max"] * finite_min(errs))
        vals[sid] = {"errors": errs, "rungs": rows, "strictly_decreasing": dec, "u_ref": u_ref, "u_ref_ok": u_ok,
                     "reference": refs[(sid, th["reference_rtol"])][1]}
        ok &= dec and u_ok
    return {"pass": bool(ok) and not quick, "values": vals, "threshold": th}


def run_c7(futs, refs):
    th = TH["C7"]
    r6 = TH["C6"]
    vals, ok = {}, True
    for sid in th["scenarios"]:
        m, T = model_of(sid)
        outs = outputs_of(sid)
        ref = refs[(sid, r6["reference_rtol"])][0]
        u_ref = bapp_error(m, outs, refs[(sid, r6["reference_uncertainty_rtol"])][0], ref)[0]
        row = {"u_ref": u_ref, "u_ref_ok": le(u_ref, th["reference_uncertainty_max"])}
        s_ok = row["u_ref_ok"]
        for rtol, gated in ((th["rung_rtol"], True), (th["H_rung_rtol"], False)):
            r = futs[(sid, rtol)].result()
            al, info = aligned(r, outs)
            key = f"rtol_{rtol:.0e}" + ("" if gated else "_H")
            if not al:
                row[key] = info
                s_ok &= not gated
                continue
            unf = {}
            e, per = bapp_error(m, outs, r["states"], ref, unf)
            row[key] = {"max_error": e, "per_observable": per, "unfloored_H": unf,
                        "attempts": (r.get("details") or {}).get("attempts")}
            if gated:
                s_ok &= le(e, th["B_app"])
        row["pass"] = bool(s_ok)
        vals[sid] = row
        ok &= s_ok
    return {"pass": bool(ok), "values": vals, "threshold": th}


def ledger_drift(m, states):
    nH = m.params.nH0
    L0 = m.ledgers(np.asarray(states[0], float))
    worst_rel, num_h, en_h = 0.0, 0.0, 0.0
    for s in states[1:]:
        L = m.ledgers(np.asarray(s, float))
        for k in L0:
            worst_rel = max(worst_rel, abs(L[k] - L0[k]) / max(abs(L0[k]), 1e-300))
        num_h = max(num_h, abs(L["number_ledger"] - L0["number_ledger"]) / nH)
        en_h = max(en_h, abs(L["energy_ledger"] - L0["energy_ledger"]) / nH)
    return worst_rel, num_h, en_h


def run_c3(all_traj):
    th = TH["C3"]
    rows, ok = [], True
    for tag, sid, resp in all_traj:
        if resp.get("status") != "OK" or not resp.get("states"):
            continue
        m, _ = model_of(sid)
        rel, nh, eh = ledger_drift(m, resp["states"])
        r_ok = le(rel, th["relative"]) and le(nh, th["number_per_H"]) and le(eh, th["energy_eV_per_H"])
        rows.append({"tag": tag, "max_rel": rel, "number_per_H": nh, "energy_eV_per_H": eh, "pass": bool(r_ok)})
        ok &= r_ok
    ok &= len(rows) > 0
    return {"pass": bool(ok), "values": {"runs": rows}, "threshold": th}


def c8_submit(runner):
    return runner.submit("C8_HOT_dense", request("HOT", 1e-6, max_attempts=TH["C8"]["max_attempts"]))


def run_c8(fut):
    th = TH["C8"]
    r = fut.result()
    m, T = model_of("HOT")
    d = r.get("details") or {}
    part = d.get("partial")
    vals = {"status": r.get("status"), "reason": (r.get("reason") or "")[:200], "domain_refusals": r.get("domain_refusals"),
            "attempts": d.get("attempts"), "partial": None}
    ok = r.get("status") == "NUMERICAL_FAILURE" and isinstance(part, dict)
    if ok:
        ca = P.CaseA(TABLE)
        yp = np.array(part["y"], float)
        Tp = m.temperature(yp)
        states = list(r.get("states") or []) + [part["y"]]
        rel, nh, eh = ledger_drift(m, states)
        Ts = [m.temperature(np.array(s)) for s in states]
        lo, hi = th["partial_T_window"]
        vals["partial"] = {"t": part["t"], "reason": part.get("reason"), "last_failure": part.get("last_failure"),
                           "T_partial": Tp, "T_max": ca.T_max, "outputs_before_partial": len(states) - 1,
                           "max_T_returned": max(Ts), "ledger_max_rel": rel, "number_per_H": nh, "energy_eV_per_H": eh}
        ok = bool(part["t"] < T and (r.get("domain_refusals") or 0) >= th["domain_refusals_min"]
                  and lo * ca.T_max <= Tp <= hi * ca.T_max and all(ca.T_min <= x <= ca.T_max for x in Ts)
                  and le(rel, th["ledger_rel"]) and le(nh, th["number_per_H"]) and le(eh, th["energy_eV_per_H"])
                  and part.get("reason") in th["reasons"] and str(part.get("last_failure")) in th["last_failure"])
    return {"pass": bool(ok), "values": vals, "threshold": th}


def c9_submit(runner):
    th = TH["C9"]
    futs = {(sid, rt): runner.submit(f"C9_alloc_{sid}_rtol{rt:.0e}", request(sid, rt), mode="allocation")
            for sid in th["alloc_scenarios"] for rt in th["rtols"]}
    futs["rep0"] = runner.submit("C9_repeat0", request("N1", 1e-7))
    futs["rep1"] = runner.submit("C9_repeat1", request("N1", 1e-7))
    return futs


def run_c9(futs):
    th = TH["C9"]
    rows, alloc_ok = {}, True
    for sid in th["alloc_scenarios"]:
        r1, r2 = (futs[(sid, rt)].result() for rt in th["rtols"])
        a1, a2 = r1.get("allocations") or {}, r2.get("allocations") or {}
        d1, d2 = r1.get("details") or {}, r2.get("details") or {}
        at1, at2 = d1.get("attempts") or 0, d2.get("attempts") or 0
        lus = [d1.get("lu"), d2.get("lu")]
        good = bool(r1.get("status") == "OK" and r2.get("status") == "OK" and a1.get("calibrated")
                    and a2.get("calibrated") and a1.get("integrate") == a2.get("integrate")
                    and at2 >= th["attempt_ratio_min"] * max(at1, 1) and all(lu == th["lu"] for lu in lus))
        rows[sid] = {"allocations": [a1.get("integrate"), a2.get("integrate")], "attempts": [at1, at2],
                     "rejected_steps": [d1.get("rejected_steps"), d2.get("rejected_steps")],
                     "domain_refusals": [r1.get("domain_refusals"), r2.get("domain_refusals")], "lu": lus,
                     "ok": good}
        alloc_ok &= good
    p0, p1 = futs["rep0"].result(), futs["rep1"].result()
    rep_ok = bool(p0.get("status") == "OK" and p1.get("status") == "OK" and bits_equal(p0["states"], p1["states"])
                  and json.dumps(p0.get("counters"), sort_keys=True) == json.dumps(p1.get("counters"), sort_keys=True))
    return {"pass": bool(alloc_ok and rep_ok), "values": {"alloc": rows, "alloc_ok": alloc_ok, "repeat_bitwise": rep_ok},
            "threshold": th}


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
    bind_ok, bind_info = source_binding()
    build, binary = build_runner(3600, raw)
    LOG(f"build exit={build['exit_code']} binding ok={bind_ok}")
    binary_sha = sha256_file(binary) if binary.is_file() else None
    results = {"node": "BASS/rei P04", "mode": "quick" if args.quick else "full", "timing_authority": "diagnostic_only",
               "thresholds": TH, "scenario_constants": {"GRID": GRID, "SCEN": SCEN, "N_OUT": N_OUT,
                                                                       "outputs": {k: outputs_of(k) for k in SCEN}}, "build": build,
               "source_binding": bind_info,
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
        with tempfile.TemporaryDirectory() as work:
            runner = Runner(binary, work, raw, args.jobs, args.timeout)
            c1f, c1ctx = c1_submit(runner)
            c2f, c2st = c2_submit(runner)
            unit = reference_unittests(raw)
            c4f, qc = c4_submit(runner)
            c5f, c5ctx = c5_submit(runner)
            c67f, rt6 = c67_submit(runner, args.quick)
            c8f = c8_submit(runner)
            c9f = c9_submit(runner)
            refs = references()
            (raw / "references.json").write_text(json.dumps(sanitize(
                {f"{k[0]}_{k[1]:.0e}": {"states": v[0], "info": v[1]} for k, v in refs.items()})))
            traj = [(f"C6/C7 {k}", k[0], f.result()) for k, f in c67f.items()]
            traj += [(f"C9 alloc {sid} {rt:.0e}", sid, c9f[(sid, rt)].result())
                     for sid in TH["C9"]["alloc_scenarios"] for rt in TH["C9"]["rtols"]]
            traj += [("C9 repeat", "N1", c9f["rep0"].result())]
            for gid, fn in (("C1", lambda: run_c1(c1f, c1ctx, bind_info)), ("C2", lambda: run_c2(c2f, c2st, unit)),
                            ("C3", lambda: run_c3(traj)), ("C4", lambda: run_c4(c4f, qc)),
                            ("C5", lambda: run_c5(c5f, c5ctx)), ("C6", lambda: run_c6(c67f, rt6, refs, args.quick)),
                            ("C7", lambda: run_c7(c67f, refs)), ("C8", lambda: run_c8(c8f)), ("C9", lambda: run_c9(c9f))):
                try:
                    gates[gid] = fn()
                except Exception:
                    gates[gid] = {"pass": False, "error": traceback.format_exc()[-3000:]}
                LOG(f"{gid} pass={gates[gid]['pass']}")
            runner.pool.shutdown(wait=True)
            results["H"] = {"all_runs": runner.all_runs,
                            "references": {f"{k[0]}_{k[1]:.0e}": v[1] for k, v in refs.items()},
                            "timing_authority": "diagnostic_only"}
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
    bind_ok_end, bind_info_end = source_binding()
    clean = bool(not changes0 and not changes1 and gerr0 is None and gerr1 is None and prov0 == prov1
                 and binary_sha is not None and sha256_file(binary) == binary_sha and bind_ok and bind_ok_end
                 and bind_info_end == bind_info)
    results["provenance"] = {"inputs": prov1, "binary_sha256": binary_sha, "git_start": git0, "git_end": git_info(),
                             "tree_changes_start": changes0, "tree_changes_end": changes1,
                             "git_error": [gerr0, gerr1], "inputs_changed_during_run": prov0 != prov1,
                             "source_binding_ok": bind_ok, "source_binding_ok_end": bind_ok_end, "clean": clean}
    order = ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9"]
    results["gates"] = gates
    results["gate_table"] = {g: bool(gates.get(g, {}).get("pass")) for g in order}
    if results.get("verdict") != "INVALID":
        if not clean:
            results["verdict"] = "INVALID"
            results["reason"] = "dirty tree, changed inputs, stale binary or source binding mismatch"
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
