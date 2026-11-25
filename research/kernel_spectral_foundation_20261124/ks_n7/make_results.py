#!/usr/bin/env python3
"""KS-N7 results: parse the raw logs of ks_n7/logs/ and write
research/kernel_spectral_foundation_20261124/results/KS_N7_IMPLEMENTATION.json (strict JSON).

Inputs (all produced by executed runs; nothing is typed in by hand):
  logs/spatial_separable_test.log   cargo test of the spatial_separable target at the code head
  logs/spatial_separable_test_precommit.log   the same run before the code commit (kept)
  logs/clippy.log, logs/fmt_check.log, logs/verify_run.log   the other checks
  logs/smoke_<arm>_<grid>_<scope>.log   one-arm measurement smoke runs (diagnostic)
  logs/perfsmoke_<arm>_256x256_<solves>.log   the same under perf stat (diagnostic)
  logs/env.txt   toolchain and host

Verdicts: agreement with Complex and the T1/T2 bounds for PairedReal are PASS/FAIL against the
bounds stated in the tests; the timing discriminator is NOT_RUN (owner-host cell of the
orchestrator). Every timing number here is a diagnostic smoke on a loaded agent host.

Usage (repository root): python3 -I research/kernel_spectral_foundation_20261124/ks_n7/make_results.py
"""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
UNIT = HERE.parent
ROOT = UNIT.parents[1]
LOGS = HERE / "logs"
OUT = UNIT / "results" / "KS_N7_IMPLEMENTATION.json"
BOUND = 1.0e-13


def rel(path):
    return str(path.relative_to(ROOT))


def header(text, key):
    match = re.search(rf"^# {key}: (.*)$", text, re.M)
    return match.group(1).strip() if match else None


def exit_code(text, path):
    value = header(text, "exit")
    if value is None:
        raise SystemExit(f"{rel(path)}: no '# exit:' line")
    return int(value)


def tagged(text, tag):
    """JSON objects printed as `TAG {...}` (the tag may follow libtest's 'test ... ' prefix)."""
    rows = []
    for line in text.splitlines():
        position = line.find(tag + " {")
        if position >= 0:
            rows.append(json.loads(line[position + len(tag) + 1 :]))
    return rows


def read(path):
    if not path.exists():
        raise SystemExit(f"missing input {rel(path)}")
    return path.read_text()


def test_outcomes(text):
    outcomes = dict(re.findall(r"^test (\w+) \.\.\. (ok|FAILED|ignored)", text, re.M))
    summary = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text, re.M)
    return outcomes, summary


def fft_lines(cells):
    """Complex FFT lines of each length per direction (analytic, from the code structure)."""
    nx, ny, nz = (list(cells) + [1])[:3]
    rows, half = ny * nz, nx // 2 + 1
    complex_arm = {"x": rows, "y": nx * nz, "z": nx * ny if nz > 1 else 0}
    paired_arm = {"x": (rows + 1) // 2, "y": half * nz, "z": half * ny if nz > 1 else 0}
    work = lambda arm: sum(arm[a] * n for a, n in zip("xyz", (nx, ny, nz)))
    return {
        "cells": [nx, ny, nz] if nz > 1 else [nx, ny],
        "complex_lines": complex_arm,
        "paired_real_lines": paired_arm,
        "line_count_ratio": sum(paired_arm.values()) / sum(complex_arm.values()),
        "points_transformed_ratio": work(paired_arm) / work(complex_arm),
        "complex_buffer_values": nx * rows,
        "paired_real_half_values": half * rows,
    }


def main():
    env = dict(
        line.split(": ", 1) for line in read(LOGS / "env.txt").splitlines() if ": " in line
    )
    checks = []

    test_log = LOGS / "spatial_separable_test.log"
    test_text = read(test_log)
    code_head = header(test_text, "head")
    outcomes, summary = test_outcomes(test_text)
    test_exit = exit_code(test_text, test_log)
    checks.append({
        "name": "spatial_separable tests",
        "command": header(test_text, "command"),
        "exit_code": test_exit,
        "result": "; ".join(f"{s[0]}: {s[1]} passed, {s[2]} failed, {s[3]} ignored" for s in summary),
        "log": rel(test_log),
    })
    pre_log = LOGS / "spatial_separable_test_precommit.log"
    if pre_log.exists():
        pre_text = pre_log.read_text()
        _, pre_summary = test_outcomes(pre_text)
        checks.append({
            "name": "spatial_separable tests before the code commit (same code, test file not yet formatted)",
            "command": header(pre_text, "command"),
            "exit_code": exit_code(pre_text, pre_log),
            "result": "; ".join(f"{s[0]}: {s[1]} passed, {s[2]} failed, {s[3]} ignored" for s in pre_summary),
            "log": rel(pre_log),
        })
    for name, file in (("clippy", "clippy.log"), ("fmt check", "fmt_check.log"), ("verify_run", "verify_run.log")):
        path = LOGS / file
        text = read(path)
        code = exit_code(text, path)
        last = [l for l in text.splitlines() if l and not l.startswith("#")]
        checks.append({
            "name": name,
            "command": header(text, "command"),
            "exit_code": code,
            "result": "pass" if code == 0 else "fail",
            "detail": last[-1] if last else "",
            "log": rel(path),
        })

    agreement = tagged(test_text, "KS_N7_AGREEMENT")
    t1t2 = {row["transform"]: row for row in tagged(test_text, "KS_N7_T1T2")}
    default_arm = tagged(test_text, "SL_N5_T1T2")
    routes = tagged(test_text, "KS_N7_T3")
    route_pairs = tagged(test_text, "KS_N7_T3_ROUTES")
    if len(agreement) != 1 or set(t1t2) != {"paired-real", "complex"} or len(default_arm) != 1:
        raise SystemExit("test log lacks the KS_N7_AGREEMENT, KS_N7_T1T2 or SL_N5_T1T2 lines")
    agreement = agreement[0]
    default_arm = default_arm[0]

    def ok(test):
        return outcomes.get(test) == "ok" and test_exit == 0

    agreement_pass = ok("paired_real_box_transform_agrees_with_the_complex_transform") \
        and agreement["max_normalized_difference"] <= BOUND and agreement["differing_cases"] > 0
    paired = t1t2["paired-real"]
    t1t2_pass = ok("paired_real_box_solves_meet_the_t1_t2_bounds") \
        and paired["max_backward_error"] <= BOUND and paired["max_mass_defect"] <= BOUND
    route_pass = ok("paired_real_box_route_matches_the_reference_and_the_complex_route")
    selection_pass = ok("box_transform_selection_defaults_parses_and_reuses_prepared_solvers")
    default_pass = ok("separable_stage_solves_are_exact_and_mass_consistent") \
        and default_arm["max_backward_error"] <= BOUND and default_arm["max_mass_defect"] <= BOUND

    smoke = []
    for path in sorted(LOGS.glob("smoke_*.log")):
        text = path.read_text()
        rows = tagged(text, "KS_N7_MEASURE")
        if len(rows) != 1 or exit_code(text, path) != 0:
            raise SystemExit(f"{rel(path)}: expected one KS_N7_MEASURE line and exit 0")
        row = rows[0]
        row.update({"log": rel(path), "load_at_start": header(text, "load"), "label": "diagnostic"})
        smoke.append(row)
    smoke_ratios = []
    for (grid, scope) in sorted({(r["grid"], r["scope"]) for r in smoke}):
        arms = {r["transform"]: r for r in smoke if r["grid"] == grid and r["scope"] == scope}
        if {"complex", "paired-real"} <= arms.keys():
            smoke_ratios.append({
                "grid": grid,
                "scope": scope,
                "median_s_complex": arms["complex"]["median_s"],
                "median_s_paired_real": arms["paired-real"]["median_s"],
                "paired_over_complex": arms["paired-real"]["median_s"] / arms["complex"]["median_s"],
                "label": "diagnostic",
            })

    perf = []
    for path in sorted(LOGS.glob("perfsmoke_*.log")):
        text = path.read_text()
        rows = tagged(text, "KS_N7_MEASURE")
        counts = dict(
            (m.group(2), int(m.group(1)))
            for m in re.finditer(r"^(\d+),,(instructions:u|cycles:u),", text, re.M)
        )
        if len(rows) != 1 or exit_code(text, path) != 0 or set(counts) != {"instructions:u", "cycles:u"}:
            raise SystemExit(f"{rel(path)}: expected one KS_N7_MEASURE line, two perf counts and exit 0")
        perf.append({
            "transform": rows[0]["transform"],
            "grid": rows[0]["grid"],
            "solves": rows[0]["solves"],
            "median_s": rows[0]["median_s"],
            "instructions_u": counts["instructions:u"],
            "cycles_u": counts["cycles:u"],
            "log": rel(path),
            "label": "diagnostic",
        })
    marginal = {}
    for transform in ("complex", "paired-real"):
        runs = sorted((p for p in perf if p["transform"] == transform and p["grid"] == "256x256"),
                      key=lambda p: p["solves"])
        if len(runs) >= 2 and runs[-1]["solves"] > runs[0]["solves"]:
            extra = runs[-1]["solves"] - runs[0]["solves"]
            marginal[transform] = {
                "solves_difference": extra,
                "instructions_u_per_solve": (runs[-1]["instructions_u"] - runs[0]["instructions_u"]) / extra,
                "cycles_u_per_solve": (runs[-1]["cycles_u"] - runs[0]["cycles_u"]) / extra,
            }
    if set(marginal) == {"complex", "paired-real"}:
        marginal["paired_over_complex"] = {
            key: marginal["paired-real"][key] / marginal["complex"][key]
            for key in ("instructions_u_per_solve", "cycles_u_per_solve")
        }
        marginal["label"] = "diagnostic (difference of two runs per arm cancels setup; loaded agent host)"

    fft = [fft_lines((256, 256)), fft_lines((128, 128, 64))]

    result = {
        "schema": "vigilode-ks-n7-implementation-v1",
        "task": "KS-N7",
        "title": "Two-for-one real FFT on the periodic box",
        "branch": "research/stage-lattice-referee-20261124-ksn7",
        "code_commit": code_head,
        "generated_by": rel(pathlib.Path(__file__).resolve()),
        "notes": rel(HERE / "NOTES.txt"),
        "environment": {
            "toolchain": env.get("rustc"),
            "cargo": env.get("cargo"),
            "cpu": env.get("cpu"),
            "kernel": env.get("kernel"),
            "taskset": env.get("taskset"),
            "loadavg_at_env_record": env.get("loadavg"),
            "perf": env.get("perf"),
            "host_note": "agent host shared with other batch jobs (load average 10-30): every timing here is diagnostic",
        },
        "api": {
            "enum": "vigilode::spatial::separable::BoxTransform { Complex (default), PairedReal } (non_exhaustive, FromStr/Display: complex, paired-real, alias paired)",
            "setter": "SeparableStages::with_box_transform(self, BoxTransform) -> Self; getter box_transform()",
            "prepared_solver": "SeparableStages::stage_solver(z) -> impl FnMut(&mut [f64]) -> Result<(), SolveError> + Send (plans and buffers built once)",
            "default": "Complex (unchanged); only the owner-host timing cell may change it",
            "sphere": "ignores the setting (its kernel always pairs rings); test asserts identical bits",
        },
        "checks": checks,
        "agreement_with_complex": {
            "statement": "||x_paired - x_complex||_inf <= 1e-13 S/(z - c), S = ((z - c) + rho_bar)||x_complex||_inf + ||b||_inf; on the box ||W^{-1}||_inf = 1/(z - c) (M-matrix, W 1 = (z - c) 1), so the T1 bound on both residuals implies 2e-13 S/(z - c)",
            "grids": 16,
            "shifts": "(z, c) in (0.5, 0), (10, -1), (3, 1), (1e4, 0), (1e-3, 0)",
            "right_hand_sides": "three noise seeds and one with rows alternating in scale by 1e8",
            **agreement,
            "test": "crates/vigilode/tests/spatial_separable.rs::paired_real_box_transform_agrees_with_the_complex_transform",
            "verdict": "PASS" if agreement_pass else "FAIL",
        },
        "t1_t2": {
            "statement": "backward error ||b - W x||_inf / S <= 1e-13 and stage mass defect |(z - c) M(x) - M(b)| / (sum A_i S) <= 1e-13, the scales of separable_stage_solves_are_exact_and_mass_consistent",
            "paired_real": paired,
            "complex_same_cases": t1t2["complex"],
            "default_arm_existing_test_SL_N5_T1T2": default_arm,
            "default_arm_recorded_before": {"max_backward_error": 3.4e-16, "max_mass_defect": 5.1e-17,
                                            "source": "RUN.json claims K5, K4"},
            "test": "crates/vigilode/tests/spatial_separable.rs::paired_real_box_solves_meet_the_t1_t2_bounds",
            "verdict": "PASS" if t1t2_pass else "FAIL",
        },
        "route": {
            "rows": routes,
            "complex_vs_paired_final_states": route_pairs,
            "statement": "whole IVPs (12x10 box of T3, 4x6x5 box) on the separable route with PairedReal match the dense matrix-exponential reference within 1e-6 max|reference| (T3 envelope), mass defect <= 1e-12, Verified without corrections, and agree with the Complex route within the same envelope",
            "verdict": "PASS" if route_pass else "FAIL",
        },
        "selection_and_reuse": {
            "statement": "Complex is the default; names parse and print; the sphere ignores the setting (identical bits); stage_solver returns the bits of fresh solve_stage calls solve after solve and refuses like solve_stage",
            "verdict": "PASS" if selection_pass else "FAIL",
        },
        "fft_work_analytic": {
            "statement": "complex FFT lines per direction, counted from the code structure (x: packed lines; y: batched lines of the half; z: gathered lines of the half)",
            "grids": fft,
        },
        "smoke_timings": {
            "label": "diagnostic only: agent host under load, few solves; not a verdict",
            "runs": smoke,
            "ratios": smoke_ratios,
        },
        "perf_smoke": {
            "label": "diagnostic only: one arm per process under perf stat, instructions:u and cycles:u of the whole test process",
            "runs": perf,
            "marginal_per_solve": marginal,
        },
        "discriminators": [
            {"name": "agreement with Complex", "frozen_threshold": "||x_p - x_c||_inf <= 1e-13 S/(z - c) on every case",
             "measured": f"max normalized {agreement['max_normalized_difference']:.3e}, max relative {agreement['max_relative_difference']:.3e} over {agreement['cases']} cases",
             "verdict": "PASS" if agreement_pass else "FAIL"},
            {"name": "T1/T2 bounds for PairedReal", "frozen_threshold": "backward error <= 1e-13 and mass defect <= 1e-13 (T1/T2 scales)",
             "measured": f"backward {paired['max_backward_error']:.3e}, mass {paired['max_mass_defect']:.3e} over {paired['solves']} solves",
             "verdict": "PASS" if t1t2_pass else "FAIL"},
            {"name": "default Complex arm unchanged (existing T1/T2 test)", "frozen_threshold": "existing T1/T2 assertions; recorded maxima 3.4e-16 / 5.1e-17",
             "measured": f"backward {default_arm['max_backward_error']:.3e}, mass {default_arm['max_mass_defect']:.3e}",
             "verdict": "PASS" if default_pass else "FAIL"},
            {"name": "box stage solve time PairedReal / Complex at 256x256", "frozen_threshold": "<= 0.6 (PLAN.json KS-N7 acceptance; kill: no gain)",
             "measured": "not measured here: owner-host preregistered cell of the orchestrator",
             "verdict": "NOT_RUN"},
        ],
        "caveats": [
            "The timing smoke compares implementations, not the pairing alone: besides halving the FFT lines, PairedReal writes the half spectra x-mode-major during the separation (no strided gather/scatter along y) and divides without per-element integer index arithmetic, while the Complex arm's division loop computes x, y, z from the flat index by integer division and remainder for every element and its y and z transforms gather and scatter strided lines (stride nx and nx ny). A Complex arm with the same layout and loop would isolate the pairing; it was not built (the Complex arm is kept unchanged as the default and baseline).",
            "Per solve, the measurement process also copies the right-hand side (n values) outside the timed region; whole-process perf counts include it in both arms.",
            "The largest backward error of the measurement test is taken over the first and the last solve (every solve has the same input and returns the same bits).",
        ],
    }
    OUT.write_text(json.dumps(result, indent=1, allow_nan=False) + "\n")
    print(f"wrote {rel(OUT)}: agreement {result['agreement_with_complex']['verdict']}, "
          f"T1/T2 {result['t1_t2']['verdict']}, route {result['route']['verdict']}, "
          f"selection {result['selection_and_reuse']['verdict']}, timing NOT_RUN")
    failed = [d["name"] for d in result["discriminators"] if d["verdict"] == "FAIL"]
    failed += [c["name"] for c in checks if c["exit_code"] != 0]
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
