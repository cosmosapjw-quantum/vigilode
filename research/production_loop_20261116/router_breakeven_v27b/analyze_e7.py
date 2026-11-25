#!/usr/bin/env python3
"""Analysis of the V30 E7 diagnosis (node V30_ROUTER_BREAKEVEN_V27B, phase 1) on P_scalar_fd.

Reads only raw output: logs/e7/raw_95d0389c7a04/P_scalar_fd__{A,L,S,tau}.stdout.jsonl (written by
run_e7.py from the binary of e7_diag/), logs/e7/RUNS_INDEX.jsonl, the V27 raw lines of
P_scalar_fd (router_hyperparameter_free/logs/phase2/raw/) and the V27 reference
(router_hyperparameter_free/logs/phase2/references.json). Errors are computed against the closed
form y(t) = (k^2 cos t + k sin t)/(1 + k^2) + e^{-k t}/(1 + k^2), k = 1e6, with mpmath at 50 digits;
the exact flow from a point (t_a, y_a) is s(t) + (y_a - s(t_a)) e^{-k (t - t_a)},
s(t) = (k^2 cos t + k sin t)/(1 + k^2). The E7 error is the V27 one: |y - y_ref| / (atol + rtol
|y_ref|) with y_ref the V27 reference (nearest double of y(1)).

Writes E7_DIAGNOSIS.json (strict JSON). Usage: python3 -I analyze_e7.py
"""
import hashlib
import json
import math
import os
import statistics
import struct

import mpmath

mpmath.mp.dps = 50
HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "logs", "e7", "raw_95d0389c7a04")
RAW_RUN1 = os.path.join(HERE, "logs", "e7", "raw")
V27 = os.path.join(HERE, "..", "router_hyperparameter_free", "logs", "phase2")
OUT = os.path.join(HERE, "E7_DIAGNOSIS.json")
K = mpmath.mpf(10) ** 6
TOLS = {"A": (1e-6, 1e-9), "L": (1e-3, 1e-6), "S": (1e-8, 1e-11)}


def f64(hexbits):
    return struct.unpack(">d", bytes.fromhex(hexbits))[0]


def sha256(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def s(t):
    t = mpmath.mpf(t)
    return (K**2 * mpmath.cos(t) + K * mpmath.sin(t)) / (1 + K**2)


def y_exact(t):
    t = mpmath.mpf(t)
    return s(t) + mpmath.exp(-K * t) / (1 + K**2)


def flow(ta, ya, t):
    return s(t) + (mpmath.mpf(ya) - s(ta)) * mpmath.exp(-K * (mpmath.mpf(t) - mpmath.mpf(ta)))


def num(x, digits=6):
    """A finite float rounded to `digits` significant digits, or None."""
    if x is None:
        return None
    x = float(x)
    if not math.isfinite(x):
        return None
    if x == 0.0:
        return 0.0
    return float(f"{x:.{digits}e}")


def load(tol):
    return [json.loads(line) for line in open(os.path.join(RAW, f"P_scalar_fd__{tol}.stdout.jsonl"))]


def points_of(run):
    """The accepted points (t, y) of a RODAS5P run line or of a replayed handover."""
    if "points" in run:
        pts = run["points"]
    elif run.get("pilot_accepted"):
        pts = run["pilot"]["points"] + run["rest"]["points"][1:]
    else:
        return []
    return [(f64(p["t"]), f64(p["y"]), f64(p["ft_fd"])) for p in pts]


def steps_of(points):
    """Per step: end time, length, z = -k h, local error (against the exact flow from the
    step's start) and global error at its end; and the relative error of the drivers'
    finite-difference time derivative at the step's start."""
    rows = []
    for (ta, ya, fta), (tb, yb, _) in zip(points, points[1:]):
        exact_ft = -K * mpmath.sin(mpmath.mpf(ta))
        rows.append({
            "t_start": ta,
            "t_end": tb,
            "h": num(tb - ta),
            "z": num(-1e6 * (tb - ta)),
            "local_error": num(mpmath.mpf(yb) - flow(ta, ya, tb)),
            "global_error": num(mpmath.mpf(yb) - y_exact(tb)),
            "ft_fd_relative_error": num((mpmath.mpf(fta) - exact_ft) / exact_ft) if ta > 0 else None,
        })
    return rows


def final_of(run):
    if "final_y" not in run:
        return None
    return f64(run["final_t"]), f64(run["final_y"])


def main():
    ref = json.load(open(os.path.join(V27, "references.json")))["references"]["P_scalar_fd"]
    y_ref = f64(ref["y_bits"][0])
    y1 = y_exact(1)
    result = {
        "schema": "vigilode.v30.e7_diagnosis.v1",
        "node": "V30_ROUTER_BREAKEVEN_V27B, phase 1 (owner decision OD-L1-26)",
        "problem": "P_scalar_fd: y' = -1e6 (y - cos t), y(0) = 1, span [0, 1], non-autonomous, "
                   "finite-difference Jacobian and time derivative (V27 runner fixture)",
        "inputs": {
            "raw": {name: sha256(os.path.join(RAW, name)) for name in sorted(os.listdir(RAW))},
            "runs_index": "logs/e7/RUNS_INDEX.jsonl",
            "v27_reference": {"y_bits": ref["y_bits"][0], "y_50_digits": ref["y_50_digits"][0]},
        },
        "error_definition": "E7 error of V27: |y(1) - y_ref| / (atol + rtol |y_ref|) (n = 1), "
                            "y_ref = nearest double of the closed form; 1 = the tolerance unit",
    }

    # run 1 (first binary) and run 2 (second binary) must agree byte for byte on A, L, S
    result["run1_equals_run2"] = {
        tol: open(os.path.join(RAW, f"P_scalar_fd__{tol}.stdout.jsonl"), "rb").read()
        == open(os.path.join(RAW_RUN1, f"P_scalar_fd__{tol}.stdout.jsonl"), "rb").read()
        for tol in "ALS"
    }

    reproduction, variants, steps, scans, derivative = {}, {}, {}, {}, {}
    for tol, (rtol, atol) in TOLS.items():
        lines = load(tol)
        weight = atol + rtol * abs(y_ref)

        def err(y):
            return abs(y - y_ref) / weight

        by = {(j["variant"], j["exp"]): j["run"] for j in lines if j["exp"] != "scan"}
        r_fd = final_of(by[("fd", "R")])
        err_r = err(r_fd[1])
        # V27 raw lines of the same case (arm N, R, D; process 1, run 1)
        v27 = {}
        for arm in ("N", "R", "D"):
            path = os.path.join(V27, "raw", f"P_scalar_fd__{tol}__default", f"{arm}_Nbuild_p1.jsonl")
            first = json.loads(open(path).readline())
            v27[arm] = first["run"]["final_state_bits"][0]
        n_fd = by[("fd", "N")]
        d_fd = by[("fd", "D")]
        prefix = by[("fd", "DOPRI5_PREFIX")]
        t_d, y_d = f64(prefix["t_d"]), f64(prefix["y_d"])
        err_n = err(f64(n_fd["final_y"]))
        reproduction[tol] = {
            "rtol": rtol, "atol": atol,
            "D": {"success": d_fd["success"], "final_t": num(f64(d_fd["final_t"])),
                  "attempts": d_fd["attempts"], "stop": d_fd["stop"],
                  "bits_equal_v27": d_fd["final_y"] == v27["D"]},
            "R": {"final_y_bits": by[("fd", "R")]["final_y"], "error": num(err_r),
                  "abs_error": num(r_fd[1] - y_ref), "attempts": by[("fd", "R")]["attempts"],
                  "bits_equal_v27": by[("fd", "R")]["final_y"] == v27["R"]},
            "N": {"final_y_bits": n_fd["final_y"], "error": num(err_n),
                  "abs_error": num(f64(n_fd["final_y"]) - y_ref), "attempts": n_fd["attempts"],
                  "legs": n_fd["legs"], "pilots": n_fd["pilots"],
                  "bits_equal_v27": n_fd["final_y"] == v27["N"]},
            "ratio_err_N_over_err_R": num(err_n / err_r, 4),
            "E7_as_frozen": "PASS" if err_n <= 3 * err_r else "FAIL",
            "handover_replay_NR_equals_N": by[("fd", "NR")]["final_y"] == n_fd["final_y"]
            and by[("fd", "NR")]["final_t"] == n_fd["final_t"],
            "dopri5_prefix": {"t_D": num(t_d, 17), "dopri5_attempts": prefix["max_steps"],
                              "t_equals_pilot_record": prefix["t_equals_record"],
                              "H": num(f64(prefix["record_h"]), 17),
                              "y_D_minus_exact": num(mpmath.mpf(y_d) - y_exact(t_d))},
        }

        rows = []
        for (variant, exp), run in sorted(by.items()):
            if exp in ("D", "DOPRI5_PREFIX"):
                continue
            fin = final_of(run)
            if fin is None:
                rows.append({"variant": variant, "exp": exp, "final": None})
                continue
            pts = points_of(run)
            last = steps_of(pts)[-1] if len(pts) >= 2 else None
            e = err(fin[1])
            rows.append({
                "variant": variant, "exp": exp,
                "success": run.get("success"),
                "final_t_is_t1": fin[0] == 1.0,
                "final_y_bits": run["final_y"],
                "error": num(e),
                "abs_error_vs_exact": num(mpmath.mpf(fin[1]) - y1),
                "ratio_to_err_R_fd": num(e / err_r, 4),
                "E7_rule_would_pass": e <= 3 * err_r,
                "accepted_points": len(pts),
                "last_step": last,
                "replay_matches": run.get("replay_matches",
                                          run.get("rest", {}).get("replay_matches")),
            })
        variants[tol] = rows

        steps[tol] = {exp: steps_of(points_of(by[("fd", exp)]))
                      for exp in ("R", "NR", "C_cont", "C_exact_state", "C_rest_h0_user",
                                  "R_h0_H", "R_from_tD")}

        # derivative variants against fd
        fd_final = {exp: f64(run["final_y"]) for (v, exp), run in by.items()
                    if v == "fd" and "final_y" in run and exp not in ("D", "N")}
        fd_final["N"] = f64(n_fd["final_y"])
        derivative[tol] = {}
        for variant in ("ft", "ft_fdclone", "jac", "jac_ft"):
            diffs, identical, compared = [], 0, 0
            for (v, exp), run in by.items():
                if v != variant or "final_y" not in run or exp not in fd_final:
                    continue
                compared += 1
                y = f64(run["final_y"])
                identical += y == fd_final[exp]
                diffs.append(abs(y - fd_final[exp]))
            derivative[tol][variant] = {
                "experiments_compared": compared,
                "bit_identical_to_fd": identical,
                "max_abs_difference_to_fd": num(max(diffs) if diffs else None),
            }
        plus = {exp: f64(run["final_y"]) for (v, exp), run in by.items()
                if v == "ft_pert+" and "final_y" in run}
        minus = {exp: f64(run["final_y"]) for (v, exp), run in by.items()
                 if v == "ft_pert-" and "final_y" in run}
        exact_ft = {exp: f64(run["final_y"]) for (v, exp), run in by.items()
                    if v == "ft" and "final_y" in run}
        derivative[tol]["ft_relative_perturbation_1e-8"] = {
            exp: {"end_change_plus": num(plus[exp] - exact_ft[exp]),
                  "end_change_minus": num(minus[exp] - exact_ft[exp])}
            for exp in sorted(plus) if exp in minus and exp in exact_ft
        }
        fd_points = [p for exp in steps[tol] for p in steps[tol][exp]]
        rel = [abs(p["ft_fd_relative_error"]) for p in fd_points
               if p["ft_fd_relative_error"] is not None]
        derivative[tol]["ft_fd_relative_error_max_over_step_starts"] = num(max(rel) if rel else None)

        # scan of h0 for plain RODAS5P from t0
        scans[tol] = {}
        for variant in ("fd", "ft", "jac_ft"):
            entries = []
            for j in lines:
                if j["exp"] != "scan" or j["variant"] != variant:
                    continue
                run = j["run"]
                fin = final_of(run)
                st = steps_of(points_of(run))
                entries.append({
                    "j": run["scan_j"], "h0": num(f64(run["h0"])),
                    "success": run["success"],
                    "error": num(err(fin[1])) if fin else None,
                    "ratio_to_err_R_default": num(err(fin[1]) / err_r, 4) if fin else None,
                    "last_step_h": st[-1]["h"] if st else None,
                    "last_step_t_start": num(st[-1]["t_start"]) if st else None,
                    "attempts": run["attempts"],
                })
            ratios = [e["ratio_to_err_R_default"] for e in entries if e["ratio_to_err_R_default"] is not None]
            last_h = [e["last_step_h"] for e in entries if e["error"]]
            errs = [e["error"] for e in entries if e["error"]]
            fit = None
            if len(errs) >= 3:
                xs = [math.log(h) for h in last_h]
                ys = [math.log(e) for e in errs]
                mx, my = statistics.fmean(xs), statistics.fmean(ys)
                sxx = sum((x - mx) ** 2 for x in xs)
                sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
                slope = sxy / sxx
                resid = [y - (my + slope * (x - mx)) for x, y in zip(xs, ys)]
                syy = sum((y - my) ** 2 for y in ys)
                fit = {"slope_log_error_vs_log_last_step": num(slope, 4),
                       "r_squared": num(1 - sum(r * r for r in resid) / syy, 4)}
            scans[tol][variant] = {
                "runs": len(entries),
                "succeeded": sum(1 for e in entries if e["success"]),
                "ratio_min": num(min(ratios), 4) if ratios else None,
                "ratio_median": num(statistics.median(ratios), 4) if ratios else None,
                "ratio_max": num(max(ratios), 4) if ratios else None,
                "runs_with_ratio_above_3": sum(1 for r in ratios if r > 3),
                "fit_end_error_vs_last_step": fit,
                "entries": entries,
            }

    # one-step grid: local truncation error and amplification R(z) of a single step
    tau_rows = []
    for line in open(os.path.join(RAW, "P_scalar_fd__tau.stdout.jsonl")):
        j = json.loads(line)
        t, h, y = f64(j["t"]), f64(j["h"]), f64(j["y"])
        one, shifted = j["step"], j["step_shifted"]
        row = {"variant": j["variant"], "tol": j["tol"], "t": t, "h": num(h),
               "accepted": bool(one.get("success")) or (
                   "final_t" in one and f64(one["final_t"]) > t)}
        if "final_t" in one and f64(one["final_t"]) > t:
            t1, y1s = f64(one["final_t"]), f64(one["final_y"])
            row["t_end"] = t1
            row["tau"] = num(mpmath.mpf(y1s) - flow(t, y, t1))
            if "final_t" in shifted and f64(shifted["final_t"]) == t1:
                y0s = f64(shifted["start_y"])
                row["R_of_z"] = num((mpmath.mpf(f64(shifted["final_y"])) - mpmath.mpf(y1s))
                                    / (mpmath.mpf(y0s) - mpmath.mpf(y)))
                row["z"] = num(-1e6 * (t1 - t))
        tau_rows.append(row)
    loose_fd = [r for r in tau_rows if r["variant"] == "fd" and r["tol"] == "loose" and "tau" in r]
    tau_fit = {}
    for t in sorted({r["t"] for r in loose_fd}):
        sel = [r for r in loose_fd if r["t"] == t and r["h"] >= 1e-2 and r["tau"]]
        if len(sel) >= 3:
            xs = [math.log(r["h"]) for r in sel]
            ys = [math.log(abs(r["tau"])) for r in sel]
            mx, my = statistics.fmean(xs), statistics.fmean(ys)
            slope = sum((x - mx) * (yv - my) for x, yv in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
            tau_fit[f"{t:g}"] = {"points": len(sel), "slope_log_tau_vs_log_h_for_h_ge_1e-2": num(slope, 4)}
    rz = [abs(r["R_of_z"]) * abs(r["z"]) for r in loose_fd
          if r.get("R_of_z") is not None and r.get("z") and abs(r["z"]) >= 100]
    tau_variants = {}
    for variant in ("ft", "jac_ft"):
        absolute, relative = [], []
        for r in tau_rows:
            if r["variant"] != variant or r["tol"] != "loose" or r.get("tau") is None:
                continue
            twin = next((q for q in loose_fd if q["t"] == r["t"] and q["h"] == r["h"]), None)
            if twin is None or twin.get("tau") is None:
                continue
            absolute.append(abs(r["tau"] - twin["tau"]))
            if abs(twin["tau"]) >= 1e-12:
                relative.append(abs(r["tau"] - twin["tau"]) / abs(twin["tau"]))
        tau_variants[variant] = {
            "steps_compared": len(absolute),
            "max_abs_tau_difference_to_fd": num(max(absolute) if absolute else None),
            "steps_with_abs_tau_fd_ge_1e-12": len(relative),
            "max_relative_tau_difference_to_fd_there": num(max(relative) if relative else None),
        }
    result["one_step_grid"] = {
        "description": "one RODAS5P step of length h from the exact solution at t (tau = its local "
                       "truncation error against the exact flow) and from the exact solution plus "
                       "1e-8 (R(z) = the step's amplification of an initial deviation, z = -1e6 h); "
                       "tol A says whether the step is accepted at (1e-6, 1e-9); tol loose "
                       "(rtol = atol = 0.5) accepts every step",
        "tau_fit_by_start_time": tau_fit,
        "abs_R_times_abs_z_for_abs_z_ge_100": {
            "count": len(rz), "min": num(min(rz)) if rz else None,
            "median": num(statistics.median(rz)) if rz else None,
            "max": num(max(rz)) if rz else None,
        },
        "derivative_variants": tau_variants,
        "rows": tau_rows,
    }
    result["reproduction"] = reproduction
    result["variants"] = variants
    result["steps_fd"] = steps
    result["derivative_checks"] = derivative
    result["h0_scan_plain_rodas5p"] = scans

    # hypotheses, each read from the numbers above (fd variant unless stated)
    def ratio(tol, exp, variant="fd"):
        row = next(r for r in variants[tol] if r["variant"] == variant and r["exp"] == exp)
        return row["ratio_to_err_R_fd"]

    def last_h(tol, exp):
        row = next(r for r in variants[tol] if r["variant"] == "fd" and r["exp"] == exp)
        return row["last_step"]["h"] if row["last_step"] else None

    failing = [tol for tol in TOLS if reproduction[tol]["E7_as_frozen"] == "FAIL"]
    all_rows = [r for tol in TOLS for r in variants[tol] if "final_t_is_t1" in r]
    decay = {}
    for tol in TOLS:
        glob = [st["global_error"] for st in steps[tol]["NR"]]
        decay[tol] = {"y_D_minus_exact": reproduction[tol]["dopri5_prefix"]["y_D_minus_exact"],
                      "global_error_after_each_NR_step": glob}
    max_deriv = max(derivative[tol][v]["max_abs_difference_to_fd"] or 0.0
                    for tol in TOLS for v in ("ft", "jac_ft"))
    result["hypotheses"] = [
        {
            "id": "H1_handover_h0_equals_H",
            "statement": "the leg after the pilot starts with h0 = H, so RODAS5P takes another step "
                         "sequence than Rodas5p::solve from t0 with the user's h0",
            "evidence": {tol: {"N": ratio(tol, "N"), "R_h0_H (no handover, h0 = H from t0)":
                               ratio(tol, "R_h0_H"),
                               "C_rest_h0_user (handover, leg h0 = 1e-3)": ratio(tol, "C_rest_h0_user"),
                               "last_step_N": last_h(tol, "NR"), "last_step_R": last_h(tol, "R"),
                               "last_step_R_h0_H": last_h(tol, "R_h0_H")} for tol in TOLS},
            "verdict": "mechanism confirmed, not specific to the handover: the step sequence decides "
                       "the end error; Rodas5p::solve alone with h0 = H (no DOPRI5, no pilot) has "
                       "ratio %s at A; the leg with the user's h0 passes only because it repeats "
                       "R's sequence shifted by t_D + H" % ratio("A", "R_h0_H"),
        },
        {
            "id": "H2_pilot_state",
            "statement": "the state handed over (DOPRI5's y_D) carries an error that RODAS5P "
                         "carries to t1",
            "evidence": {"C_exact_state_ratio": {tol: ratio(tol, "C_exact_state") for tol in TOLS},
                         "decay_along_NR": decay,
                         "abs_R_times_abs_z_median": result["one_step_grid"][
                             "abs_R_times_abs_z_for_abs_z_ge_100"]["median"]},
            "verdict": "refuted as a carried error: R(z) ~ -12.4/z damps the prefix error below "
                       "1e-12 within five steps; the state changes the end error only by changing "
                       "the controller's early proposals and hence the step sequence (A: exact "
                       "state %s, L: %s)" % (ratio("A", "C_exact_state"), ratio("L", "C_exact_state")),
        },
        {
            "id": "H3_controller_history",
            "statement": "the restart after the pilot discards the controller's proposal",
            "evidence": {"C_cont_ratio": {tol: ratio(tol, "C_cont") for tol in TOLS}},
            "verdict": "refuted: one continuing RODAS5P run from (t_D, y_D) with h0 = H fails E7 "
                       "as well (A %s, L %s, S %s)" % (ratio("A", "C_cont"), ratio("L", "C_cont"),
                                                     ratio("S", "C_cont")),
        },
        {
            "id": "H4_finite_difference_time_derivative",
            "statement": "the drivers' central-difference f_t (relative error up to %s at the step "
                         "starts) causes the error" % max(
                             derivative[tol]["ft_fd_relative_error_max_over_step_starts"] or 0.0
                             for tol in TOLS),
            "evidence": {tol: {v: derivative[tol][v] for v in ("ft", "ft_fdclone", "jac_ft")}
                         for tol in TOLS},
            "verdict": "refuted: the exact f_t moves no end state by more than %s; the drivers' "
                       "difference supplied as a callback gives the bits of fd" % num(max_deriv),
        },
        {
            "id": "H5_finite_difference_jacobian",
            "statement": "the finite-difference Jacobian causes the error",
            "evidence": {tol: derivative[tol]["jac"] for tol in TOLS},
            "verdict": "refuted: the exact Jacobian gives the bits of fd in every experiment",
        },
        {
            "id": "H6_dense_output",
            "statement": "the end state is interpolated",
            "evidence": {"rows": len(all_rows),
                         "rows_ending_exactly_at_t1": sum(1 for r in all_rows if r["final_t_is_t1"]),
                         "dense_output_option": False},
            "verdict": "refuted: no run uses dense output; every run's last accepted step ends at "
                       "t1 exactly and the end state is that step's result",
        },
        {
            "id": "H7_error_norm",
            "statement": "the error norm differs between the arms",
            "evidence": {tol: {"err_N": reproduction[tol]["N"]["error"],
                               "err_R": reproduction[tol]["R"]["error"]} for tol in TOLS},
            "verdict": "refuted: one norm for all arms (V27 E7 definition); in it N's error is far "
                       "below the tolerance unit 1",
        },
        {
            "id": "H8_rodas5p_step_sequence_variance",
            "statement": "RODAS5P's own end error on this problem varies with the step sequence by "
                         "more than the factor 3 of E7",
            "evidence": {tol: {k: scans[tol]["fd"][k] for k in
                               ("runs", "succeeded", "ratio_min", "ratio_median", "ratio_max",
                                "runs_with_ratio_above_3", "fit_end_error_vs_last_step")}
                         for tol in TOLS},
            "verdict": "confirmed: over h0 = 10^(-6 + j/8), j = 0..40, Rodas5p::solve alone has "
                       "end errors from %s to %s times that of h0 = 1e-3 at A; %s of %s exceed 3"
                       % (scans["A"]["fd"]["ratio_min"], scans["A"]["fd"]["ratio_max"],
                          scans["A"]["fd"]["runs_with_ratio_above_3"], scans["A"]["fd"]["runs"]),
        },
    ]
    result["cause"] = (
        "On P_scalar_fd the end error of every RODAS5P run is the local truncation error of its "
        "last step: earlier errors, the DOPRI5 prefix's included, are damped by R(z) ~ -12.4/z "
        "(|z| = 1e6 h >= 18 on every step of the route's leg and of R), and the local error of a "
        "step of length h on the slow manifold grows like h^3 (one-step grid: slopes %s to %s for "
        "h >= 1e-2; finite-difference derivatives play no part). After its first steps the "
        "controller grows each step by its cap 5 because the estimate is far below tolerance, so "
        "the last step's length is set by where the ramp from h0 lands before t1, not by the "
        "tolerance (the h0 scans at A and L are identical). Rodas5p::solve from t0 with h0 = 1e-3 "
        "lands with a short last step (0.219): its error is %s times the median over h0. The FBP "
        "leg, a plain Rodas5p::solve from t_D + H with h0 = H, lands with last steps %s (A) and "
        "%s (L); the same happens without any handover (Rodas5p::solve from t0 with h0 = H: ratio "
        "%s at A). E7 failed because its reference R is a lucky step sequence and the rule has no "
        "floor; the route has no defect here. Errors in the tolerance unit: N %s (A), %s (L)."
        % (min(v["slope_log_tau_vs_log_h_for_h_ge_1e-2"] for v in tau_fit.values()),
           max(v["slope_log_tau_vs_log_h_for_h_ge_1e-2"] for v in tau_fit.values()),
           num(1 / scans["A"]["fd"]["ratio_median"], 3), last_h("A", "NR"), last_h("L", "NR"),
           ratio("A", "R_h0_H"), reproduction["A"]["N"]["error"], reproduction["L"]["N"]["error"]))
    result["route_defect"] = False
    result["failing_tolerances_reproduced"] = failing
    with open(OUT, "w") as out:
        json.dump(result, out, indent=1, sort_keys=False, allow_nan=False)
        out.write("\n")
    # a short summary on stdout
    for tol in TOLS:
        rep = reproduction[tol]
        print(tol, "ratio N/R", rep["ratio_err_N_over_err_R"], rep["E7_as_frozen"],
              "N=V27", rep["N"]["bits_equal_v27"], "R=V27", rep["R"]["bits_equal_v27"],
              "NR=N", rep["handover_replay_NR_equals_N"])
        for row in variants[tol]:
            if row["variant"] == "fd":
                print("   ", row["exp"], "ratio", row["ratio_to_err_R_fd"], "last h",
                      row["last_step"]["h"] if row["last_step"] else None)
        for variant, sc in scans[tol].items():
            print("   scan", variant, "min/median/max", sc["ratio_min"], sc["ratio_median"],
                  sc["ratio_max"], ">3:", sc["runs_with_ratio_above_3"], "of", sc["runs"],
                  "fit", sc["fit_end_error_vs_last_step"])
    print("tau fit", json.dumps(tau_fit))
    print("|R| |z|", result["one_step_grid"]["abs_R_times_abs_z_for_abs_z_ge_100"])
    print("tau variants", tau_variants)
    print("derivative", json.dumps({t: {k: v for k, v in d.items() if k != "ft_relative_perturbation_1e-8"} for t, d in derivative.items()}))


if __name__ == "__main__":
    main()
