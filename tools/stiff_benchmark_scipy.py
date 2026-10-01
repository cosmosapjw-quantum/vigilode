#!/usr/bin/env python3
"""SciPy arms, reference solutions and analysis of the stiff BDF/Radau benchmark.

Research node: research/stiff_bdf_radau_benchmark_20261001 (see its PREREGISTRATION.md).

Inputs: the Rust benchmark JSON (`rodas5p stiff-benchmark`).  The script

1. rebuilds the four problems in NumPy and checks the right-hand side and Jacobian against
   the samples the Rust binary exported (parity);
2. computes reference final states with SciPy Radau at rtol 1e-13 and estimates their
   uncertainty against a second solve at rtol 1e-12;
3. runs SciPy BDF and Radau (analytic dense Jacobian) over the same tolerance ladder, with
   the same warmup and repetition counts;
4. writes the SciPy rows, the references and the merged work-precision analysis.

Wall seconds are diagnostics on a shared host.  SciPy arms evaluate the right-hand side in
Python, so their wall time includes interpreter overhead per call; compare work counters
across languages, wall time only within one language.
"""

from __future__ import annotations

import os

for _var in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(_var, "1")

import argparse
import json
import math
import platform
import sys
import time
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp

SCHEMA = "vigilode-stiff-bdf-radau-benchmark-scipy-v1"
SCIPY_ARMS = ("scipy-bdf", "scipy-radau")
ERROR_FLOOR = 1.0e-10
TARGETS = (1.0e-3, 1.0e-5, 1.0e-7)
REFERENCE_RTOL = 1.0e-13
CHECK_RTOL = 1.0e-12
PARITY_LIMIT = 1.0e-13
REFERENCE_LIMIT = 1.0e-8


def robertson():
    def f(t, y):
        return np.array([
            -0.04 * y[0] + 1.0e4 * y[1] * y[2],
            0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
            3.0e7 * y[1] * y[1],
        ])

    def jac(t, y):
        return np.array([
            [-0.04, 1.0e4 * y[2], 1.0e4 * y[1]],
            [0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1]],
            [0.0, 6.0e7 * y[1], 0.0],
        ])

    return f, jac


def hires():
    def f(t, y):
        return np.array([
            -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007,
            1.71 * y[0] - 8.75 * y[1],
            -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4],
            8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
            -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
            -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6],
            280.0 * y[5] * y[7] - 1.81 * y[6],
            -280.0 * y[5] * y[7] + 1.81 * y[6],
        ])

    def jac(t, y):
        j = np.zeros((8, 8))
        j[0, 0], j[0, 1], j[0, 2] = -1.71, 0.43, 8.32
        j[1, 0], j[1, 1] = 1.71, -8.75
        j[2, 2], j[2, 3], j[2, 4] = -10.03, 0.43, 0.035
        j[3, 1], j[3, 2], j[3, 3] = 8.32, 1.71, -1.12
        j[4, 4], j[4, 5], j[4, 6] = -1.745, 0.43, 0.43
        j[5, 3], j[5, 4], j[5, 5], j[5, 6], j[5, 7] = 0.69, 1.71, -280.0 * y[7] - 0.43, 0.69, -280.0 * y[5]
        j[6, 5], j[6, 6], j[6, 7] = 280.0 * y[7], -1.81, 280.0 * y[5]
        j[7, 5], j[7, 6], j[7, 7] = -280.0 * y[7], 1.81, -280.0 * y[5]
        return j

    return f, jac


def van_der_pol(mu):
    def f(t, y):
        return np.array([y[1], mu * (1.0 - y[0] * y[0]) * y[1] - y[0]])

    def jac(t, y):
        return np.array([[0.0, 1.0], [-2.0 * mu * y[0] * y[1] - 1.0, mu * (1.0 - y[0] * y[0])]])

    return f, jac


def brusselator(cells):
    c = (cells + 1.0) ** 2 / 50.0
    n = 2 * cells

    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.concatenate(([1.0], u[:-1]))
        ur = np.concatenate((u[1:], [1.0]))
        vl = np.concatenate(([3.0], v[:-1]))
        vr = np.concatenate((v[1:], [3.0]))
        out = np.empty(n)
        out[0::2] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur)
        out[1::2] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr)
        return out

    def jac(t, y):
        j = np.zeros((n, n))
        for i in range(cells):
            u, v = y[2 * i], y[2 * i + 1]
            a, b = 2 * i, 2 * i + 1
            j[a, a] = 2.0 * u * v - 4.0 - 2.0 * c
            j[a, b] = u * u
            j[b, a] = 3.0 - 2.0 * u * v
            j[b, b] = -u * u - 2.0 * c
            if i > 0:
                j[a, a - 2] = c
                j[b, b - 2] = c
            if i + 1 < cells:
                j[a, a + 2] = c
                j[b, b + 2] = c
        return j

    return f, jac


PROBLEMS = {
    "robertson": robertson(),
    "hires": hires(),
    "van-der-pol-mu1000": van_der_pol(1.0e3),
    "brusselator-1d-50": brusselator(50),
}


def rel_diff(a, b):
    a, b = np.asarray(a, float), np.asarray(b, float)
    return float(np.max(np.abs(a - b) / np.maximum(np.abs(b), 1.0)))


def final_error(y, ref):
    y, ref = np.asarray(y, float), np.asarray(ref, float)
    return float(np.max(np.abs(y - ref) / np.maximum(np.abs(ref), ERROR_FLOOR)))


def parity(rust):
    report = {}
    for pid, samples in rust["parity_samples"].items():
        f, jac = PROBLEMS[pid]
        worst_f = worst_j = 0.0
        for s in samples:
            y = np.array(s["y"])
            worst_f = max(worst_f, rel_diff(f(s["t"], y), s["f"]))
            worst_j = max(worst_j, rel_diff(jac(s["t"], y), s["jacobian"]))
        report[pid] = {"rhs_max_rel_diff": worst_f, "jacobian_max_rel_diff": worst_j,
                       "ok": max(worst_f, worst_j) <= PARITY_LIMIT}
    return report


def solve(method, pid, problem, rtol):
    f, jac = PROBLEMS[pid]
    return solve_ivp(f, problem["t_span"], np.array(problem["y0"], float), method=method,
                     rtol=rtol, atol=rtol * problem["atol_scale"], jac=jac)


def reference(pid, problem):
    started = time.perf_counter()
    ref = solve("Radau", pid, {**problem, "atol_scale": problem["atol_scale"] * 1.0e-1}, REFERENCE_RTOL)
    check = solve("Radau", pid, {**problem, "atol_scale": problem["atol_scale"] * 1.0e-1}, CHECK_RTOL)
    lsoda = solve("LSODA", pid, {**problem, "atol_scale": problem["atol_scale"] * 1.0e-1}, CHECK_RTOL)
    if not (ref.success and check.success):
        raise SystemExit(f"reference solve failed for {pid}: {ref.message} / {check.message}")
    y = ref.y[:, -1]
    return {
        "method": f"scipy Radau rtol {REFERENCE_RTOL:g}, atol {REFERENCE_RTOL * problem['atol_scale'] * 0.1:g}",
        "final_state": y.tolist(),
        "uncertainty": final_error(check.y[:, -1], y),
        "lsoda_rtol_1e-12_difference": final_error(lsoda.y[:, -1], y) if lsoda.success else None,
        "seconds": time.perf_counter() - started,
    }


def scipy_rows(rust):
    rows = []
    for problem in rust["problems"]:
        pid = problem["id"]
        for arm in SCIPY_ARMS:
            method = "BDF" if arm == "scipy-bdf" else "Radau"
            for rtol in rust_tolerances(rust):
                for _ in range(rust["warmups"]):
                    solve(method, pid, problem, rtol)
                walls, finals, first = [], [], None
                for _ in range(rust["repetitions"]):
                    started = time.perf_counter()
                    sol = solve(method, pid, problem, rtol)
                    walls.append(time.perf_counter() - started)
                    finals.append(sol.y[:, -1].tolist() if sol.success else None)
                    if first is None:
                        first = sol
                row = {"problem": pid, "arm": arm, "rtol": rtol, "atol": rtol * problem["atol_scale"],
                       "wall_seconds": walls, "wall_median": sorted(walls)[len(walls) // 2],
                       "deterministic": all(x == finals[0] for x in finals)}
                if first.success:
                    row.update(status="completed", final_state=first.y[:, -1].tolist(),
                               accepted_steps=len(first.t) - 1,
                               counters={"rhs_evaluations": first.nfev, "jacobian_builds": first.njev,
                                         "direct_factorizations": first.nlu})
                else:
                    row.update(status="failed", message=first.message)
                print(f"{pid} {arm} rtol={rtol:g}: {row['status']} median {row['wall_median']:.4e} s",
                      file=sys.stderr)
                rows.append(row)
    return rows


def rust_tolerances(rust):
    return sorted({row["rtol"] for row in rust["rows"]}, reverse=True)


def analysis(rust, scipy_report):
    refs = scipy_report["references"]
    rows = []
    for row in rust["rows"] + scipy_report["rows"]:
        merged = {k: row.get(k) for k in ("problem", "arm", "rtol", "atol", "status", "accepted_steps",
                                           "rejected_steps", "counters", "wall_median", "deterministic")}
        if row["status"] == "completed":
            ref = refs[row["problem"]]
            err = final_error(row["final_state"], ref["final_state"])
            merged["error"] = err
            merged["reference_limited"] = err < 10.0 * ref["uncertainty"]
        rows.append(merged)
    arms = sorted({r["arm"] for r in rows}, key=lambda a: (a != "rodas5p", a))
    matched = {}
    for pid, ref in refs.items():
        matched[pid] = {}
        for target in TARGETS:
            if target < 10.0 * ref["uncertainty"]:
                matched[pid][f"{target:g}"] = "reference-limited"
                continue
            entry = {}
            for arm in arms:
                ok = [r for r in rows if r["problem"] == pid and r["arm"] == arm
                      and r.get("error") is not None and r["error"] <= target]
                if not ok:
                    entry[arm] = None
                    continue
                best = min(ok, key=lambda r: r["wall_median"])
                entry[arm] = {"wall_median": best["wall_median"], "rtol": best["rtol"],
                              "error": best["error"],
                              "rhs_evaluations": min(r["counters"]["rhs_evaluations"] for r in ok),
                              "jacobian_builds": min(r["counters"]["jacobian_builds"] for r in ok),
                              "direct_factorizations": min(r["counters"]["direct_factorizations"] for r in ok)}
            base = entry.get("rodas5p")
            for arm, value in entry.items():
                if value and base:
                    value["wall_ratio_to_rodas5p"] = value["wall_median"] / base["wall_median"]
            matched[pid][f"{target:g}"] = entry
    gate = {
        "parity": all(p["ok"] for p in scipy_report["parity"].values()),
        "references": all(r["uncertainty"] <= REFERENCE_LIMIT for r in refs.values()),
        "rust_deterministic": all(r["deterministic"] for r in rust["rows"]),
        "finite_errors": all(math.isfinite(r["error"]) for r in rows if "error" in r),
    }
    return {"rows": rows, "matched_error": matched, "validity_gate": gate,
            "verdict": "PASS" if all(gate.values()) else "FAIL"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust", required=True, type=Path)
    parser.add_argument("--scipy-output", required=True, type=Path)
    parser.add_argument("--analysis-output", required=True, type=Path)
    args = parser.parse_args()
    for path in (args.scipy_output, args.analysis_output):
        if path.exists():
            raise SystemExit(f"immutable output already exists: {path}")
    rust = json.loads(args.rust.read_text())
    scipy_report = {
        "schema": SCHEMA,
        "runtime": {"python": platform.python_version(), "numpy": np.__version__,
                    "scipy": scipy.__version__, "machine": platform.machine(),
                    "threads": {v: os.environ.get(v) for v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS")}},
        "timing_status": "DIAGNOSTIC: Python right-hand sides; compare work counters across languages",
        "parity": parity(rust),
        "references": {p["id"]: reference(p["id"], p) for p in rust["problems"]},
    }
    scipy_report["rows"] = scipy_rows(rust)
    args.scipy_output.write_text(json.dumps(scipy_report, indent=1) + "\n")
    result = analysis(rust, scipy_report)
    result["schema"] = SCHEMA.replace("scipy", "analysis")
    args.analysis_output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({"verdict": result["verdict"], "gate": result["validity_gate"]}))


if __name__ == "__main__":
    main()
