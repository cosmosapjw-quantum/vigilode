#!/usr/bin/env python3
"""Independent 50-digit check of the Newton-Leja phi-action candidate (RVJ DAG node PP10,
research/pp10_leja_candidate_20261004).

For every case written by crates/rodas5p-core/tests/pp10_leja_study.rs it computes the exact
target F = sum_k phi_k(h A) w_k for the binary64 A, h and w_k (taken as exact reals) with the
PP08 reference itself (tools/pp08_laguerre_router_check.py: exact_target, 50 digits, mpmath
symmetric eigendecomposition), then evaluates the preregistered gate:

G1 every Leja report is EstimateOnly (reason LEJA_TOTAL_NOT_CERTIFIED), and no certified API
   (admit_total_error, admit_laguerre_total, route_admission) admits the Leja result put into the
   case's certified Chebyshev report;
G2 ||fused_Leja - F||_2 <= 10 x the requested tolerance in at least 90 % of the cases (a Leja
   action error counts as a miss); every case with actual error > the Leja estimate is listed.

Reported (no gate): Leja vs Chebyshev operator vector products and coefficient setups. The
Chebyshev run uses the same tolerance as its truncation budget, so the comparison is "at matched
actual error" in this sense, fixed before the run: the product ratio is summarized over the cases
where both actual errors are at most the requested tolerance (both met it); the cases where only
one met it are listed, and the ratio of the actual errors is reported beside it. Also reported:
failures (action errors, cap reached, actual error above the tolerance and above 10x it).

Precondition (reported): every case's A, h and w_k equal, bit for bit, the PP08 case with the same
label in research/pp08_laguerre_router_20261004/cases.json.

Writes RESULTS.json and refuses to overwrite it.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pp08_laguerre_router_check import distance, exact_target, unhex  # noqa: E402

SCHEMA = "vigilode-pp10-leja-results-v1"
REASON = "LEJA_TOTAL_NOT_CERTIFIED"
G2_FACTOR = 10
G2_FRACTION = 0.9


def fixtures_match(cases, pp08_path: Path):
    pp08 = {c["label"]: c for c in json.loads(pp08_path.read_text())["cases"]}
    mismatches = []
    for case in cases:
        ref = pp08.get(case["label"])
        if ref is None or ref["group"] != "main":
            mismatches.append({"label": case["label"], "why": "no PP08 main case with this label"})
            continue
        for key in ("n", "a", "h", "input", "w"):
            if case[key] != ref[key]:
                mismatches.append({"label": case["label"], "why": f"{key} differs"})
    return mismatches


def stats(values):
    if not values:
        return None
    values = sorted(values)
    return {"min": values[0], "median": statistics.median(values), "max": values[-1], "count": len(values)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pp08-cases", type=Path,
                        default=Path("research/pp08_laguerre_router_20261004/cases.json"))
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.cases.read_text())
    mp.mp.dps = 50

    mismatches = fixtures_match(data["cases"], args.pp08_cases)
    rows = []
    g1_reports = 0
    g1 = True
    g1_failures = []
    within = 0
    under_reports = []
    failures = {"leja_errors": [], "leja_cap_reached": [], "leja_above_tolerance": [],
                "leja_above_10x_tolerance": [], "chebyshev_errors": [], "chebyshev_above_tolerance": []}
    for case in data["cases"]:
        exact = exact_target(case)
        tol = mp.mpf(unhex(case["tolerance"]))
        leja, cheb = case["leja"], case["chebyshev"]
        row = {"label": case["label"], "n": case["n"], "h": case["h_f64"], "input": case["input"],
               "tolerance": case["tolerance_f64"]}
        if "error" in leja:
            failures["leja_errors"].append({"label": case["label"], "error": leja["error"]})
            row["leja_error"] = leja["error"]
        else:
            g1_reports += 1
            status = leja["total_error"]
            ok = status["status"] == "estimate-only" and status["reason"].startswith(REASON)
            apis = case["certified_apis_on_leja"]
            # None: no carrier (the Chebyshev action failed), so not tested in this case; the
            # contract test pp10_leja_contracts.rs covers the APIs independently.
            apis_rejected = None if apis is None else all(not v["admitted"] for v in apis.values())
            if not ok or apis_rejected is False:
                g1 = False
                g1_failures.append({"label": case["label"], "status_ok": ok, "apis_rejected": apis_rejected})
            err = distance(leja["fused"], exact)
            est = mp.mpf(unhex(leja["estimate"]))
            if err <= G2_FACTOR * tol:
                within += 1
            else:
                failures["leja_above_10x_tolerance"].append(case["label"])
            if err > tol:
                failures["leja_above_tolerance"].append(case["label"])
            if not leja["converged"]:
                failures["leja_cap_reached"].append(case["label"])
            if err > est:
                under_reports.append({"label": case["label"], "actual_error": mp.nstr(err, 6),
                                      "estimate": mp.nstr(est, 6), "tolerance": case["tolerance_f64"],
                                      "ratio_actual_over_estimate": mp.nstr(err / est, 6) if est > 0 else "inf"})
            row.update({"leja_degree": leja["degree"], "leja_converged": leja["converged"],
                        "leja_actual_error": mp.nstr(err, 6), "leja_estimate": mp.nstr(est, 6),
                        "leja_within_10x": bool(err <= G2_FACTOR * tol), "leja_within_tol": bool(err <= tol),
                        "leja_under_reports": bool(err > est),
                        "leja_vector_products": leja["vector_products"],
                        "leja_coefficient_setups": leja["coefficient_setups"],
                        "leja_status_estimate_only": ok, "certified_apis_reject": apis_rejected})
            row["_leja_err"] = err
        if "error" in cheb:
            failures["chebyshev_errors"].append({"label": case["label"], "error": cheb["error"]})
            row["chebyshev_error"] = cheb["error"]
        else:
            cerr = distance(cheb["fused"], exact)
            if cerr > tol:
                failures["chebyshev_above_tolerance"].append(case["label"])
            row.update({"chebyshev_degree": cheb["degree"], "chebyshev_actual_error": mp.nstr(cerr, 6),
                        "chebyshev_within_tol": bool(cerr <= tol),
                        "chebyshev_vector_products": cheb["vector_products"],
                        "chebyshev_coefficient_setups": cheb["coefficient_setups"]})
            row["_cheb_err"] = cerr
        rows.append(row)
        print(row["label"], row.get("leja_degree"), row.get("leja_actual_error"), row.get("leja_estimate"),
              row.get("chebyshev_degree"), row.get("chebyshev_actual_error"), flush=True)

    total = len(rows)
    fraction = within / total if total else 0.0
    # Products at matched actual error (both met the requested tolerance).
    both, only_leja, only_cheb, neither = [], [], [], []
    for row in rows:
        if "_leja_err" not in row or "_cheb_err" not in row:
            continue
        lw, cw = row["leja_within_tol"], row["chebyshev_within_tol"]
        entry = {"label": row["label"], "leja_products": row["leja_vector_products"],
                 "chebyshev_products": row["chebyshev_vector_products"],
                 "leja_actual_error": row["leja_actual_error"],
                 "chebyshev_actual_error": row["chebyshev_actual_error"]}
        (both if lw and cw else only_leja if lw else only_cheb if cw else neither).append(entry)
        cp = row["chebyshev_vector_products"]
        row["products_ratio_leja_over_chebyshev"] = row["leja_vector_products"] / cp if cp else None
        row["actual_error_ratio_leja_over_chebyshev"] = (
            mp.nstr(row["_leja_err"] / row["_cheb_err"], 6) if row["_cheb_err"] > 0 else "inf")
    ratios = [e["leja_products"] / e["chebyshev_products"] for e in both if e["chebyshev_products"] > 0]
    by_tol = {}
    for t in sorted({r["tolerance"] for r in rows}):
        sub = [e for e in both if next(r for r in rows if r["label"] == e["label"])["tolerance"] == t]
        rr = [e["leja_products"] / e["chebyshev_products"] for e in sub if e["chebyshev_products"] > 0]
        by_tol[f"{t:e}"] = {"both_met": len(sub), "ratio": stats(rr),
                            "leja_fewer": sum(e["leja_products"] < e["chebyshev_products"] for e in sub),
                            "equal": sum(e["leja_products"] == e["chebyshev_products"] for e in sub),
                            "leja_more": sum(e["leja_products"] > e["chebyshev_products"] for e in sub)}
    err_ratio = [float(r["_leja_err"] / r["_cheb_err"]) for r in rows
                 if "_leja_err" in r and "_cheb_err" in r and r["_cheb_err"] > 0]
    for row in rows:
        row.pop("_leja_err", None)
        row.pop("_cheb_err", None)
    setups = {"leja": sorted({r.get("leja_coefficient_setups") for r in rows if "leja_coefficient_setups" in r}),
              "chebyshev": sorted({r.get("chebyshev_coefficient_setups") for r in rows
                                   if "chebyshev_coefficient_setups" in r})}
    summary = {
        "cases": total,
        "leja_reports": g1_reports,
        "within_10x_tolerance": within,
        "fraction_within_10x_tolerance": fraction,
        "under_report_count": len(under_reports),
        "products_at_matched_error": {
            "both_met_tolerance": len(both), "only_leja_met": only_leja, "only_chebyshev_met": only_cheb,
            "neither_met": neither, "ratio_leja_over_chebyshev": stats(ratios),
            "leja_fewer": sum(e["leja_products"] < e["chebyshev_products"] for e in both),
            "equal": sum(e["leja_products"] == e["chebyshev_products"] for e in both),
            "leja_more": sum(e["leja_products"] > e["chebyshev_products"] for e in both),
            "by_tolerance": by_tol,
        },
        "actual_error_ratio_leja_over_chebyshev": stats(err_ratio),
        "coefficient_setups_per_case": setups,
        "failure_counts": {k: len(v) for k, v in failures.items()},
    }
    gate = {
        "G1_estimate_only_never_admitted": g1 and g1_reports > 0,
        "G2_actual_within_10x_tolerance_in_90pct": fraction >= G2_FRACTION,
    }
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps,
              "preconditions": {"fixtures_match_pp08": not mismatches, "fixture_mismatches": mismatches},
              "rows": rows, "g1_failures": g1_failures, "under_reports": under_reports,
              "failures": failures, "summary": summary, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"preconditions": report["preconditions"], "gate": gate, "verdict": report["verdict"]},
                     indent=1))
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
