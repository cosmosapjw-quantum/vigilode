#!/usr/bin/env python3
"""Read-only reanalysis of frozen NCP artifacts; no solver execution.

Usage: python3 research/ncp_followup_20261021/audit_ncp_artifacts.py
The resulting NCP_AUDIT.json contains derived corrections, never rewritten inputs.
"""
from __future__ import annotations

import hashlib
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
BUNDLE = REPO / "external_audits/vigilode_ncp_intensive_120m_v1_20261021"


def read_json(name):
    return json.loads((BUNDLE / name).read_text())


def rows(name):
    return [json.loads(line) for line in (BUNDLE / name).read_text().splitlines() if line.strip()]


def measurements(rs):
    return [r for r in rs if not r.get("warmup", False)]


def exact(c, t):
    y, p, kind = c["initial_state"], c["parameters"], c["exact_kind"]
    if kind == "diagonal_exponential":
        return [a * math.exp(b * t) for a, b in zip(y, p["rates"])]
    if kind == "rotation":
        z = p["omega"] * t
        return [y[0] * math.cos(z) - y[1] * math.sin(z), y[0] * math.sin(z) + y[1] * math.cos(z)]
    if kind == "logistic":
        return [p["capacity"] / (1 + (p["capacity"] / y[0] - 1) * math.exp(-p["rate"] * t))]
    if kind == "nonnormal_triangular":
        a, d, b = p["a"], p["d"], p["b"]
        return [math.exp(a*t) * (y[0] + b*y[1]*math.expm1((d-a)*t)/(d-a)), y[1]*math.exp(d*t)]
    if kind == "periodic_heat_fourier":
        n = len(y)
        factor = math.exp(-4 * p["diffusion"] * n*n * math.sin(math.pi/n)**2 * t)
        return [a * factor for a in y]
    if kind == "damped_oscillator":
        w, z = p["omega"], p["zeta"]
        a = z*w
        if z == 1:
            b = y[1] + w*y[0]
            return [math.exp(-w*t)*(y[0]+b*t), math.exp(-w*t)*(b-w*(y[0]+b*t))]
        if z < 1:
            v = w*math.sqrt(1-z*z)
            b = (y[1]+a*y[0])/v
            return [math.exp(-a*t)*(y[0]*math.cos(v*t)+b*math.sin(v*t)), math.exp(-a*t)*((-a*y[0]+v*b)*math.cos(v*t)+(-a*b-v*y[0])*math.sin(v*t))]
        d = w*math.sqrt(z*z-1)
        r1, r2 = -a+d, -a-d
        c1 = (y[1]-r2*y[0])/(r1-r2)
        c2 = y[0]-c1
        return [c1*math.exp(r1*t)+c2*math.exp(r2*t), r1*c1*math.exp(r1*t)+r2*c2*math.exp(r2*t)]
    return None


def metrics(c, times, states, rtol=None):
    if not times or len(times) != len(states):
        raise ValueError("empty or mismatched output grid")
    n = len(c["initial_state"])
    if len(c["atol"]) != n or any(len(row) != n for row in states):
        raise ValueError("invalid state dimension")
    if not all(math.isfinite(x) for row in states for x in row):
        raise ValueError("nonfinite states")
    rtol = c["rtol"] if rtol is None else rtol
    absolute, scaled = [], []
    for t, row in zip(times, states):
        ref = exact(c, t)
        if ref is None:
            return None
        for y, z, atol in zip(row, ref, c["atol"]):
            absolute.append(abs(y-z))
            scaled.append(abs(y-z)/(atol+rtol*abs(z)))
    return {"max_abs_error": max(absolute), "max_scaled_error": max(scaled), "accuracy_pass_scaled_le_1": max(scaled) <= 1}


def main():
    manifest = read_json("ARTIFACT_SHA256SUMS.json")
    failures = []
    for item in manifest["files"]:
        path = BUNDLE / item["path"]
        if not path.resolve().is_relative_to(BUNDLE.resolve()):
            raise ValueError("manifest path escapes bundle")
        data = path.read_bytes()
        if len(data) != item["bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
            failures.append(item["path"])
    files = {str(p.relative_to(BUNDLE)) for p in BUNDLE.rglob("*") if p.is_file()}
    declared = {x["path"] for x in manifest["files"]}
    integrity = {"manifest_entries": len(manifest["files"]), "actual_files": len(files), "mismatches": failures, "unlisted_files": sorted(files-declared), "status": "PASS" if not failures and files-declared == {"ARTIFACT_SHA256SUMS.json"} else "FAIL"}
    cases = {c["case_id"]: c for c in read_json("harness/cases.json")}
    rotation_case = cases["C-rotation-w10"]
    sanity = metrics(rotation_case, [0.0], [[1.0, 0.0]])
    assert sanity["max_scaled_error"] == 0.0
    rejected = 0
    for times, states in [([], []), ([0.0, 1.0], [[1.0, 0.0]]), ([0.0], [[1.0]]), ([0.0], [[math.nan, 0.0]])]:
        try:
            metrics(rotation_case, times, states)
        except ValueError:
            rejected += 1
    assert rejected == 4
    phase_files = {
        "screening": ["RAW_RESULTS.jsonl"],
        "confirmation_original": ["CONFIRMATION_RESULTS.jsonl"],
        "confirmation_rotation_corrected": ["CONFIRMATION_ROTATION_V2.jsonl"],
        "julia_confirmation": ["JULIA_RESULTS.jsonl"],
        "throughput_main": ["THROUGHPUT_RESULTS.jsonl"] + sorted(str(p.relative_to(BUNDLE)) for p in (BUNDLE/"throughput_raw").glob("*.jsonl")),
        "throughput_rotation_corrected": ["THROUGHPUT_ROTATION_V2.jsonl"],
        "diffsol_confirmation": ["DIFFSOL_RESULTS_RECOVERED.jsonl"],
        "decay_work_precision": ["TOLERANCE_RESULTS.jsonl"],
        "rotation_work_precision": ["ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl"],
        "nonlinear_confirmation": ["NONLINEAR_CONFIRMATION_RESULTS.jsonl"],
    }
    phase_counts = {}
    for name, paths in phase_files.items():
        rr = measurements([r for path in paths for r in rows(path)])
        acc = Counter((r.get("accuracy") or {}).get("accuracy_pass_scaled_le_1") for r in rr)
        phase_counts[name] = {"attempts": len(rr), "statuses": dict(Counter(r.get("status", "UNSPECIFIED") for r in rr)), "recorded_accuracy_pass": acc[True], "recorded_accuracy_fail": acc[False], "recorded_accuracy_unknown": acc[None], "sources": paths}
    rot = measurements(rows("ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl"))
    groups = defaultdict(list)
    for r in rot:
        groups[(r["solver"], r["rtol"])].append(r)
    rotation = []
    for (solver, rtol), rr in sorted(groups.items()):
        acc = [r["accuracy"] for r in rr]
        solve_times = [(r.get("timing") or {}).get("elapsed_ns", 0)/1e9 if (r.get("timing") or {}).get("elapsed_ns") is not None else r.get("elapsed_seconds", r.get("process_wall_seconds")) for r in rr]
        endpoints = [metrics(cases[r["case_id"]], [cases[r["case_id"]]["t_span"][1]], [r["states_final"]], rtol) for r in rr if r.get("states_final") is not None]
        rotation.append({"solver": solver, "rtol": rtol, "atol": cases["C-rotation-w10"]["atol"], "measurements": len(rr), "recorded_grid_pass": sum(a["accuracy_pass_scaled_le_1"] for a in acc), "recorded_max_scaled_error": max(a["max_scaled_error"] for a in acc), "median_correct_scope_seconds": statistics.median(solve_times), "timing_scope": "native_internal_timed_solve" if solver == "vigilode" else "elapsed_seconds_or_vode_wrapper_call", "recomputed_endpoint_pass": sum(e["accuracy_pass_scaled_le_1"] for e in endpoints), "recomputed_endpoint_count": len(endpoints)})
    throughput_corrections = []
    for r in measurements(rows("THROUGHPUT_RESULTS_RECOVERED.jsonl")):
        if r["case_id"] != "C-rotation-w10":
            continue
        c = cases[r["case_id"]]
        corrected = metrics(c, c["output_times"], r["states"])
        throughput_corrections.append({"solver": r["solver"], "repetition": r["repetition"], "old_max_scaled_error": r["accuracy"]["max_scaled_error"], "corrected": corrected, "time_grid_source": "harness/cases.json"})
    independent_grid_checks = []
    for filename in ["JULIA_RESULTS.jsonl", "DIFFSOL_RESULTS_RECOVERED.jsonl"]:
        checked = mismatch = 0
        for r in measurements(rows(filename)):
            c = cases[r["case_id"]]
            value = metrics(c, c["output_times"], r["states"], r["rtol"])
            if value is None:
                continue
            checked += 1
            if value["accuracy_pass_scaled_le_1"] != r["accuracy"]["accuracy_pass_scaled_le_1"]:
                mismatch += 1
        independent_grid_checks.append({"source": filename, "checked": checked, "classification_mismatches": mismatch})
    decisions = read_json("DECISIONS.json")["decisions"]
    scope_corrections = []
    for d in decisions:
        c = cases[d["scope"]["case_id"]]
        if d["scope"]["rtol"] != c["rtol"]:
            scope_corrections.append({"solver": d["solver"], "case_id": c["case_id"], "old_rtol": d["scope"]["rtol"], "correct_rtol": c["rtol"], "atol": c["atol"], "existing_verdict": d["verdict"]})
    report = {
        "schema_version": 1, "audit_source_commit": "7a63876a97fc17f640771ec06658269947049994", "solver_source_commit": "de4c10a9f41321beef2313a3d7fb503bab35d714", "method": "Read-only artifact hashes, independent aggregation and analytic output evaluation; no new solver runs", "integrity": integrity,
        "phase_counts": phase_counts, "measurement_attempts": sum(x["attempts"] for x in phase_counts.values()),
        "rotation_work_precision_corrected": rotation, "julia_rotation_throughput_corrections": throughput_corrections,
        "independent_full_grid_checks": independent_grid_checks, "decision_scope_corrections": scope_corrections,
        "findings": [
            {"id":"NCP-A01","severity":"high","class":"reporting","evidence":"REPORT_KO.md versus ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl / WORK_PRECISION_COVERAGE.json","finding":"Report says rotation0/140 pass; raw and coverage support7/140 pass (VigilODE rtol1e-10),133fail.","action":"Use corrected derived aggregate; keep historical text and evidence unchanged."},
            {"id":"NCP-A02","severity":"high","class":"implementation","evidence":"harness/analyze_campaign.py exact_rotation caller","finding":"Julia throughput rotation reference uses invented0.01 time grid; actual request uses2*pi*i/40.","action":"Corrected metrics above; maintain failure classification at rtol1e-6 but replace inflated error."},
            {"id":"NCP-A03","severity":"high","class":"measurement_semantics","evidence":"harness/analyze_campaign.py work precision times selection","finding":"VigilODE process wall includes startup and internal warmup whereas Julia/diffsol use warmed solver timing; available native timing.elapsed_ns is ignored.","action":"Use separate cold-process and timed-solve cohorts; no speed promotion from current plot."},
            {"id":"NCP-A04","severity":"medium","class":"reporting","evidence":"DECISIONS.json; harness/cases.json","finding":"Nonnormal and mixed-nearzero decisions incorrectly label rtol1e-6; actual1e-7 and1e-8 respectively.","action":"Apply scope corrections listed above."},
            {"id":"NCP-A05","severity":"high","class":"comparison_configuration","evidence":"harness/confirmation_worker.py and SciPy v1.11.4 scipy/integrate/_ode.py vode defaults / _determine_mf_and_set_bands","finding":"VODE has no Jacobian callback, bands or with_jacobian=True, so Adams/BDF are functional iteration MF10/MF20. Metadata calling this default finite-difference Jacobian is incorrect.","action":"Keep original evidence; add explicit analytic or internal-Jacobian BDF comparison with complete derivative work before performance conclusions.","reference":"https://github.com/scipy/scipy/blob/v1.11.4/scipy/integrate/_ode.py"},
            {"id":"NCP-A06","severity":"medium","class":"measurement_semantics","evidence":"harness/julia_solver_v2.jl solveonce counters","finding":"Julia nlu is mapped to nsolve/nsolve_linear, which counts solves rather than LU factorizations.","action":"Record linear_solves separately; report nlu null unless actual factorization count available."},
            {"id":"NCP-A07","severity":"high","class":"unsupported_claim","evidence":"CONFIG.json and all campaign solver identifiers","finding":"This bundle exercises native-rodas5p-fast only, not SP02 GMRES; no evidence for claimed dense-fast/SP02 3-5x or preconditioned3-5iterations.","action":"Treat as hypotheses and measure actual dimension/preconditioner crossover."},
            {"id":"NCP-A08","severity":"medium","class":"evidence_coverage","evidence":"Native/VODE confirmation and precision rows preserve states_final but not full states","finding":"Native/VODE full sampled-grid errors cannot be independently recomputed from saved rows; endpoints can be checked, Julia/diffsol full grids are available.","action":"Persist output_times and complete states or hashed lossless array artifacts for next campaign."},
            {"id":"NCP-A09","severity":"medium","class":"metric_validation","evidence":"harness/confirmation_worker.py metrics and related copies","finding":"zip silently truncates mismatched grids/dimensions; empty output yields max error0. Existing complete-output rows do not prove the validator safe.","action":"Require nonempty exact-length finite output and terminal-time match before accuracy evaluation."}
        ],
        "interpretation":{"broad_production":"HOLD","rotation_rtol_1e_10":"recorded sampled-grid PASS; endpoints independently recomputed; full native trajectory not retained","rotation_rtol_1e_6":"scoped measured failure persists","stiffness":"local-error tolerance is not a global trajectory-error guarantee; use fixed application budget separate from internal tolerance","speed_claims":"diagnostic only; matched timing scopes and paired rerun required","largescale_FVM_readiness":"not established by n<=512 dense-adapter screening"},
        "metric_contract_checks":{"exact_initial_state": "pass", "empty_mismatched_nonfinite_rejections": rejected},
        "execution":{"status":"completed" if integrity["status"] == "PASS" else "failed","exit_code":0 if integrity["status"] == "PASS" else 1,"command":"python3 research/ncp_followup_20261021/audit_ncp_artifacts.py","solver_runs":0},
    }
    (HERE/"NCP_AUDIT.json").write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False)+"\n")
    print(json.dumps({"integrity":integrity,"attempts":report["measurement_attempts"],"rotation_pass":sum(x["recorded_grid_pass"] for x in rotation),"rotation_measurements":sum(x["measurements"] for x in rotation),"full_grid_checks":independent_grid_checks,"decision_scope_corrections":len(scope_corrections)}, ensure_ascii=False))
    if integrity["status"] != "PASS":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
