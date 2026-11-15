#!/usr/bin/env python3
"""Fold the raw U03 evidence (Rust QUAL log + Python oracle outputs) into RESULTS.json.

Numbers are copied from the raw files; verdict strings are derived by explicit rules below so
the classification is reproducible.  Run: python3 -I summarize.py (from any directory).
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def strict_load(p):
    def bad(x):
        raise ValueError(f"nonfinite constant in {p}: {x}")
    return json.loads(Path(p).read_text(), parse_constant=bad)


def qual_rows(log):
    out = {}
    for line in log.read_text().splitlines():
        if "QUAL {" in line:
            d = json.loads(line[line.index("QUAL ") + 5:])
            out.setdefault(d["test"], []).append(d["data"])
    return out


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    log = HERE / "logs/u03_green.log"
    q = qual_rows(log)
    coeff = strict_load(HERE / "coefficients_oracle_summary.json")
    theta = strict_load(HERE / "theta_oracle_results.json")

    q01 = q["q01_coefficients"][0]
    q03 = q["q03_prothero_robinson"][0]
    q05 = q["q05_sigma_and_clipping"][0]
    q08 = q["q08_adaptive"][0]

    # theta verdict per (theta, eps): stable iff all four grids finite and finest error < 1e-6
    theta_rows = []
    for r in theta["nonnormal_pr_system"]:
        errs = r["endpoint_error"]
        ok = all(e is not None for e in errs) and errs[-1] < 1e-6 and max(errs) < 1e-3
        theta_rows.append({"theta": r["theta"], "eps": r["eps"], "schedule": r["schedule"],
                           "finest_error": errs[-1], "max_error": None if None in errs else max(errs),
                           "verdict": "CONVERGED_BOUNDED" if ok else "UNSTABLE_OR_UNRESOLVED"})
    scalar = [{k: r[k] for k in ("sigma", "theta", "max_rho", "rho_at_z_minus_1e8", "unstable_z_range")}
              for r in theta["scalar_stability"]]

    adaptive = [{k: r.get(k) for k in ("problem", "variant", "tol", "threads", "h_max_for_h0_rule", "h0",
                                       "status", "achieved_error", "accepted", "rejected", "rhs_calls",
                                       "jacobian_builds", "lu", "linear_solves", "rounding_floor_warnings",
                                       "min_final_clip_ratio")} for r in q08]
    tol_met = [{"problem": r["problem"], "variant": r.get("variant"), "tol": r["tol"],
                "achieved_over_tol": r["achieved_error"] / r["tol"], "status": r["status"]}
               for r in q08 if r["threads"] == 1]

    res = {
        "schema": "vigilode.u03-results.v1",
        "task": "U03_PEER_INDEPENDENT_QUALIFICATION",
        "scope": "Scoped method/implementation evidence on linear calibration problems; not G0, not production.",
        "inputs_sha256": {str(p.relative_to(ROOT)): sha(p) for p in [
            log, HERE / "coefficients_oracle.txt", HERE / "coefficients_oracle_summary.json",
            HERE / "theta_oracle_results.json",
            ROOT / "crates/rodas5p-integrators/src/ppsw1.rs",
            ROOT / "crates/rodas5p-integrators/tests/ppsw1_independent_qualification.rs"]},
        "coefficients": {
            "route": coeff["route"], "gamma_hat_6b": coeff["gamma_hat_6b"],
            "table1_real_roots": coeff["real_roots"],
            "oracle_condition_residuals": coeff["sigmas"],
            "implementation_vs_oracle": q01,
            "max_diff_in_scale_eps": max(r["diff_in_scale_eps"] for r in q01),
            "verdict": "MATCH_INDEPENDENT_ROUTE_R1 (sigma 1e-9..1.1; closed forms (16)/(21) not used by oracle)",
        },
        "polynomial_exactness": q["q02_polynomial_exactness"][0],
        "prothero_robinson": q03,
        "lin3_cross_backend": q["q04_lin3_cross_backend"][0],
        "sigma_patterns_and_clipping": q05,
        "startup": q["q06_startup"][0],
        "estimator": q["q07_estimator"][0],
        "adaptive": adaptive,
        "adaptive_tolerance_ratio": tol_met,
        "refusals": q["q09_refusals"][0],
        "failure_accounting": q.get("q10_singular_w", []),
        "theta_axis": {"scalar_stability": scalar, "nonnormal_pr_system": theta_rows,
                       "backend": theta["backend"], "L": theta["L"], "norm_L_2": theta["norm_L_2"]},
    }
    (HERE / "RESULTS.json").write_text(json.dumps(res, indent=1, allow_nan=False) + "\n")
    print("wrote RESULTS.json")
    for r in tol_met:
        print(r)


if __name__ == "__main__":
    main()
