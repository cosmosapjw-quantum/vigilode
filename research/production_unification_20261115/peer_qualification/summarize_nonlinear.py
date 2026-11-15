#!/usr/bin/env python3
"""U03 nonlinear evidence -> NONLINEAR_RESULTS.json, including the T01 matched-error reading.

Rules (MATCHED_ERROR_PROTOCOL.json): conservative error E = endpoint WRMS + U_ref; a point is
admissible iff status ok and U_ref <= 0.1 E; per solver the Pareto frontier is the set of
admissible points not dominated in (E, cost); comparison only inside the overlap interval
I = [max Emin, min Emax]; cost at E* by log-log interpolation between bracketing frontier
points; no extrapolation.  Output grid is {t_end} (documented limitation).

Costs (all from counters, startup included):
  rhs        total RHS callbacks
  lu         total dense LU factorizations
  cp_model   sequential critical-path units, MODEL ONLY (not measured):
             RODAS5P sequential driver: jacobian + lu + linear_solves + rhs (all sequential);
             PPSW1: startup as implemented (jacobian + lu + linear_solves + rhs, sequential)
                    + per attempt 1 jacobian + 1 lu + 1 solve phase + 1 RHS phase
                    (ideal 6-way stage parallelism; attempts = LU count in the step phase).
Wall time is single-run, diagnostic_only, and not used for ranking.
PPSW1 is an opt-in candidate; the reading is diagnostic, never a ranking/promotion claim.
"""
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent


def qual(log):
    out = {}
    for line in log.read_text().splitlines():
        if "QUAL {" in line:
            d = json.loads(line[line.index("QUAL ") + 5:])
            out[d["test"]] = d["data"]
    return out


def cost(r):
    st = r.get("startup") or {}
    sp = r.get("steps") or {}

    def g(d, k):
        return d.get(k) or 0
    rhs = g(st, "rhs_calls") + g(sp, "rhs_calls")
    lu = g(st, "lu") + g(sp, "lu")
    if r["solver"].startswith("ppsw1"):
        cp = (g(st, "jacobian_builds") + g(st, "lu") + g(st, "linear_solves") + g(st, "rhs_calls")
              + g(sp, "jacobian_builds") + g(sp, "lu") + 2 * g(sp, "lu"))
    else:
        cp = g(sp, "jacobian_builds") + g(sp, "lu") + g(sp, "linear_solves") + g(sp, "rhs_calls")
    return {"rhs": rhs, "lu": lu, "cp_model": cp}


def frontier(points, key):
    pts = sorted(points, key=lambda p: (p["E"], p["cost"][key]))
    front = []
    for p in pts:
        if not front or p["cost"][key] < front[-1]["cost"][key]:
            front.append(p)
    return front  # E increasing, cost strictly decreasing


def interp(front, key, e_star):
    for a, b in zip(front, front[1:]):
        if a["E"] <= e_star <= b["E"]:
            t = (math.log(e_star) - math.log(a["E"])) / (math.log(b["E"]) - math.log(a["E"]))
            return math.exp(math.log(a["cost"][key])
                            + t * (math.log(b["cost"][key]) - math.log(a["cost"][key])))
    # discrete attainment: cheapest frontier point with E <= E*
    ok = [p["cost"][key] for p in front if p["E"] <= e_star]
    return min(ok) if ok else None


def main():
    q = qual(HERE / "logs/u03_green.log")
    rows = list(q["n3_matched_error_sweep"])
    for r in q["n4_robertson_starter"]["caller_h0_runs"]:
        r = dict(r)
        r["solver"] = "ppsw1_caller_h0_1e-6"
        rows.append(r)
    arms = {}
    for r in rows:
        ok = r["status"] == "ok" and r.get("reference_admissible") is True
        arms.setdefault((r["problem"], r["solver"]), []).append(
            {"tol": r["tol"], "status": r["status"], "admissible": ok,
             "E": r.get("conservative_error"), "cost": cost(r) if r["status"] == "ok" else None,
             "wall_seconds_diagnostic": r.get("wall_seconds")})
    comparisons = []
    for prob in sorted({p for p, _ in arms}):
        for ps in sorted(s for (p, s) in arms if p == prob and s.startswith("ppsw1")):
            A = [x for x in arms[(prob, ps)] if x["admissible"]]
            B = [x for x in arms.get((prob, "rodas5p_sequential_direct"), []) if x["admissible"]]
            entry = {"problem": prob, "candidate": ps, "comparator": "rodas5p_sequential_direct",
                     "candidate_admissible_points": len(A), "comparator_admissible_points": len(B)}
            if not A or not B:
                entry["reading"] = "NOT_COMPARABLE_NO_ADMISSIBLE_POINTS"
                comparisons.append(entry)
                continue
            lo = max(min(x["E"] for x in A), min(x["E"] for x in B))
            hi = min(max(x["E"] for x in A), max(x["E"] for x in B))
            entry["overlap_interval"] = [lo, hi]
            entry["candidate_E_range"] = [min(x["E"] for x in A), max(x["E"] for x in A)]
            entry["comparator_E_range"] = [min(x["E"] for x in B), max(x["E"] for x in B)]
            if lo > hi:
                entry["reading"] = "NOT_COMPARABLE_NO_OVERLAP"
                comparisons.append(entry)
                continue
            stars = sorted({lo, hi, math.sqrt(lo * hi)})
            evals = []
            for key in ("rhs", "lu", "cp_model"):
                fa, fb = frontier(A, key), frontier(B, key)
                for es in stars:
                    ca, cb = interp(fa, key, es), interp(fb, key, es)
                    evals.append({"cost": key, "E_star": es, "candidate": ca, "comparator": cb,
                                  "candidate_over_comparator": (ca / cb) if (ca and cb) else None})
            entry["evaluations"] = evals
            entry["reading"] = "DIAGNOSTIC_ONLY (candidate is opt-in, single-point output grid)"
            comparisons.append(entry)
    res = {"schema": "vigilode.u03-nonlinear-results.v1",
           "rules": __doc__,
           "n1_fixed_order": q["n1_nonlinear_fixed_order"],
           "n2_jacobian_staleness": q["n2_jacobian_staleness"],
           "n4_robertson_starter_scan": q["n4_robertson_starter"]["starter_scan"],
           "n5_cap_rounding": q["n5_cap_rounding"],
           "arms": [{"problem": p, "solver": s, "points": v} for (p, s), v in sorted(arms.items())],
           "matched_error": comparisons}
    (HERE / "NONLINEAR_RESULTS.json").write_text(json.dumps(res, indent=1, allow_nan=False) + "\n")
    for c in comparisons:
        print(c["problem"], c["candidate"], c["reading"], c.get("overlap_interval"))
        for e in c.get("evaluations", []):
            r = e["candidate_over_comparator"]
            print("   %-8s E*=%.2e cand=%s comp=%s ratio=%s" % (
                e["cost"], e["E_star"], e["candidate"] and round(e["candidate"]),
                e["comparator"] and round(e["comparator"]), r and round(r, 2)))


if __name__ == "__main__":
    main()
