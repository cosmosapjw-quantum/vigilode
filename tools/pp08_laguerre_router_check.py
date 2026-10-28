#!/usr/bin/env python3
"""Independent 50-digit check of the opt-in Chebyshev/Laguerre router (RVJ DAG node PP08,
research/pp08_laguerre_router_20261004).

For every case written by crates/rodas5p-core/tests/pp08_laguerre_router_study.rs it computes
the exact target F = sum_k phi_k(h A) w_k for the binary64 A, h and w_k (taken as exact reals)
by a symmetric eigendecomposition in mpmath, then evaluates the preregistered gate:
G1 every admitted routed result has ||fused - F||_2 <= bound; G2 the negative and boundary
cases; G3 the caller's counters equal the sum of the attempts' counters. It writes
RESULTS.json and refuses to overwrite it.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-pp08-laguerre-router-results-v1"
DEGREE_LIMIT = 128


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def phi(k: int, z):
    """phi_k(z) = sum_j z^j / (j + k)!, phi_0 = exp."""
    if abs(z) < mp.mpf("0.5"):
        total, term, j = mp.mpf(0), mp.mpf(1) / mp.factorial(k), 0
        while abs(term) > mp.mpf(10) ** (-mp.mp.dps - 5):
            total += term
            j += 1
            term = term * z / (j + k)
        return total
    value = mp.e ** z
    for m in range(1, k + 1):
        value = (value - 1 / mp.factorial(m - 1)) / z
    return value


def exact_target(case) -> list:
    n = case["n"]
    a = mp.matrix([[mp.mpf(unhex(x)) for x in row] for row in case["a"]])
    h = mp.mpf(unhex(case["h"]))
    w = [[mp.mpf(unhex(x)) for x in vec] for vec in case["w"]]
    eigenvalues, q = mp.eigsy(a)
    result = [mp.mpf(0)] * n
    for k in range(5):
        if all(x == 0 for x in w[k]):
            continue
        coords = [mp.fsum(q[i, j] * w[k][i] for i in range(n)) for j in range(n)]
        scaled = [phi(k, h * eigenvalues[j]) * coords[j] for j in range(n)]
        for i in range(n):
            result[i] += mp.fsum(q[i, j] * scaled[j] for j in range(n))
    return result


def distance(values_hex, exact):
    values = [mp.mpf(unhex(x)) for x in values_hex]
    return mp.sqrt(mp.fsum((v - e) ** 2 for v, e in zip(values, exact)))


def expected_choice(attempts) -> str:
    cheb, lag = attempts
    c_ok, l_ok = cheb["admission"]["admitted"], lag["admission"]["admitted"]
    if c_ok and l_ok:
        return "Laguerre" if lag["vector_products"] < cheb["vector_products"] else "Chebyshev"
    if c_ok:
        return "Chebyshev"
    if l_ok:
        return "Laguerre"
    return "Fallback"


def counters_add_up(case) -> bool:
    caller = case["caller_counters"]
    keys = set(caller)
    for attempt in case["attempts"]:
        keys |= set(attempt["work"])
    return all(
        caller.get(key, 0) == sum(a["work"].get(key, 0) for a in case["attempts"]) for key in keys
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.cases.read_text())
    mp.mp.dps = 50

    rows = []
    g1 = True
    g1_count = 0
    g2_items = {}
    g3 = True
    rule_consistent = True
    attempt_enclosure_all = True
    budget_respected = True
    declared_never = True
    above_limit_never = True
    fallback_without_output = True
    for case in data["cases"]:
        exact = exact_target(case)
        total_budget = mp.mpf(unhex(case["total_budget"]))
        attempts = []
        for attempt in case["attempts"]:
            item = {"basis": attempt["basis"], "degree": attempt["degree"],
                    "laguerre_scale": attempt["laguerre_scale"],
                    "admitted": attempt["admission"]["admitted"],
                    "vector_products": attempt["vector_products"],
                    "coefficient_setups": attempt["coefficient_setups"]}
            report = attempt["report"]
            if report is not None:
                err = distance(report["fused"], exact)
                item["actual_error"] = mp.nstr(err, 6)
                cand = report["candidate_total"]
                item["candidate_total"] = None if cand is None else mp.nstr(mp.mpf(unhex(cand["hex"])), 6)
                if not report["evidence_verified"] and attempt["admission"]["admitted"]:
                    declared_never = False
                if attempt["basis"] == "Laguerre" and attempt["degree"] > DEGREE_LIMIT \
                        and attempt["admission"]["admitted"]:
                    above_limit_never = False
            if attempt["admission"]["admitted"]:
                bound = mp.mpf(unhex(attempt["admission"]["bound"]))
                item["bound"] = mp.nstr(bound, 6)
                if bound > total_budget:
                    budget_respected = False
                ok = report is not None and distance(report["fused"], exact) <= bound
                item["enclosed"] = bool(ok)
                attempt_enclosure_all &= bool(ok)
            else:
                item["reason"] = attempt["admission"]["reason"]
            attempts.append(item)
        row = {"label": case["label"], "group": case["group"], "input": case["input"],
               "h": case["h_f64"], "total_budget": case["total_budget_f64"],
               "choice": case["choice"], "attempts": attempts,
               "admitting_bases": [a["basis"] for a in attempts if a["admitted"]]}
        if case["choice"] != expected_choice(case["attempts"]):
            rule_consistent = False
        if case["choice"] == "Fallback":
            if case["routed_fused"] is not None or case["routed_bound"] is not None:
                fallback_without_output = False
        else:
            routed_err = distance(case["routed_fused"], exact)
            routed_bound = mp.mpf(unhex(case["routed_bound"]))
            enclosed = routed_err <= routed_bound
            g1 &= bool(enclosed)
            g1_count += 1
            if routed_bound > total_budget:
                budget_respected = False
            row.update({"routed_error": mp.nstr(routed_err, 6), "routed_bound": mp.nstr(routed_bound, 6),
                        "enclosed": bool(enclosed),
                        "tightness": mp.nstr(routed_bound / routed_err, 6) if routed_err > 0 else "inf"})
        adds_up = counters_add_up(case)
        g3 &= adds_up
        row["counters_add_up"] = adds_up
        rows.append(row)
        print(row["label"], row["choice"], row.get("enclosed"), row.get("tightness"), flush=True)

    by_label = {c["label"]: c for c in data["cases"]}
    guards = data["guards"]
    declared_cases = [c for c in data["cases"] if c["group"] == "declared"]
    g2_items["declared_cases_fallback"] = bool(declared_cases) and all(
        c["choice"] == "Fallback" for c in declared_cases)
    g2_items["no_unverified_attempt_admitted"] = declared_never
    g2_items["unbounded_timing_never_admitted"] = bool(guards["unbounded_timing_execution"]) and all(
        not t["admitted_at_1e300"] and t["execution"] == "unbounded-timing"
        for t in guards["unbounded_timing_execution"])
    above = guards["degree_above_limit"]
    above_cases = [c for c in data["cases"] if c["group"] == "degree-above-limit"]
    g2_items["degree_above_limit_case_rejected"] = bool(above["found"]) and len(above_cases) == 1 and (
        not above_cases[0]["attempts"][1]["admission"]["admitted"]
        and above_cases[0]["attempts"][1]["degree"] > DEGREE_LIMIT
        and above_cases[0]["choice"] != "Laguerre")
    g2_items["no_laguerre_attempt_above_limit_admitted"] = above_limit_never
    g2_items["unmet_budget_fallback"] = by_label["unmet-budget-1e-300"]["choice"] == "Fallback"
    g2_items["fallback_has_no_output"] = fallback_without_output
    g2_items["no_admitted_bound_above_total_budget"] = budget_respected
    g2_items["invalid_total_budgets_are_errors_without_work"] = all(
        b["is_error"] and b["no_work"] for b in guards["invalid_total_budgets"])
    g2 = all(g2_items.values())

    main_rows = [r for r in rows if r["group"] == "main"]
    routed = [r for r in rows if r["choice"] != "Fallback"]
    only_one = [{"label": r["label"], "admits": r["admitting_bases"][0], "choice": r["choice"]}
                for r in rows if len(r["admitting_bases"]) == 1]
    tight = [float(mp.mpf(r["tightness"])) for r in routed if r["tightness"] != "inf"]
    tight.sort()
    summary = {
        "cases": len(rows),
        "main_cases": len(main_rows),
        "choice_counts": {k: sum(r["choice"] == k for r in rows) for k in ("Chebyshev", "Laguerre", "Fallback")},
        "main_choice_counts": {k: sum(r["choice"] == k for r in main_rows)
                               for k in ("Chebyshev", "Laguerre", "Fallback")},
        "fraction_laguerre_of_all": sum(r["choice"] == "Laguerre" for r in rows) / len(rows),
        "fraction_laguerre_of_routed": (sum(r["choice"] == "Laguerre" for r in routed) / len(routed)) if routed else None,
        "fraction_laguerre_of_main": sum(r["choice"] == "Laguerre" for r in main_rows) / len(main_rows),
        "both_admit": sum(len(r["admitting_bases"]) == 2 for r in rows),
        "only_one_admits": only_one,
        "only_chebyshev_admits": sum(o["admits"] == "Chebyshev" for o in only_one),
        "only_laguerre_admits": sum(o["admits"] == "Laguerre" for o in only_one),
        "neither_admits": sum(len(r["admitting_bases"]) == 0 for r in rows),
        "routed_tightness_min": tight[0] if tight else None,
        "routed_tightness_median": tight[len(tight) // 2] if tight else None,
        "routed_tightness_max": tight[-1] if tight else None,
        "every_admitted_attempt_encloses": attempt_enclosure_all,
        "choice_rule_consistent": rule_consistent,
    }
    gate = {
        "G1_admitted_routed_bound_encloses": g1 and g1_count > 0,
        "G2_negative_and_boundary": g2,
        "G3_accounting": g3,
    }
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps,
              "admitted_routed_cases": g1_count, "rows": rows, "g2_items": g2_items,
              "guards": guards, "summary": summary, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "g2_items": g2_items, "verdict": report["verdict"]}, indent=1))
    print(json.dumps({k: v for k, v in summary.items() if k != "only_one_admits"}, indent=1))


if __name__ == "__main__":
    main()
