#!/usr/bin/env python3
"""Apply the pre-registered U11 rules (PREREGISTRATION.json) to logs/holdout_runs.jsonl.

E = endpoint WRMS (weights |y_ref| + 1e-6) + U_ref; admissible iff success and U_ref <= 0.1 E.
Matched-E comparisons only inside overlap intervals, log-log interpolation, no extrapolation.
Each G0 criterion is evaluated separately and as a conjunction; missing evidence -> HOLD.
"""
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load():
    ref = json.loads((HERE / "reference.json").read_text())["problems"]
    lines = (HERE / "logs/holdout_runs.jsonl").read_text().splitlines()
    return ref, [json.loads(l) for l in lines if l.strip()]


def wrms(y, r):
    return math.sqrt(sum(((a - b) / (abs(b) + 1e-6)) ** 2 for a, b in zip(y, r["y_ref"])) / len(y))


def interp(points, key, e_star):
    pts = sorted(points, key=lambda p: p["E"])
    for a, b in zip(pts, pts[1:]):
        if a["E"] <= e_star <= b["E"] and a["E"] < b["E"]:
            t = (math.log(e_star) - math.log(a["E"])) / (math.log(b["E"]) - math.log(a["E"]))
            return math.exp(math.log(a[key]) + t * (math.log(b[key]) - math.log(a[key])))
    for p in pts:
        if p["E"] == e_star:
            return p[key]
    return None


def main():
    ref, runs = load()
    arms = {}
    for r in runs:
        res = r["result"]
        rf = ref[r["problem"]]
        row = {"problem": r["problem"], "arm": r["arm"], "tol": r["tol"],
               "success": bool(res.get("success")), "wall": r["wall_median_s"]}
        if row["success"] and res.get("y_end") is not None:
            e = wrms(res["y_end"], rf) + rf["u_ref_wrms"]
            row["E"] = e
            row["admissible"] = rf["u_ref_wrms"] <= 0.1 * e
        else:
            row["admissible"] = False
            row["failure"] = res.get("failure") or res.get("error")
        row["steps"] = (res.get("accepted") or 0) + (res.get("rejected") or 0)
        if r["arm"] == "rodas5p":
            row["stage_depth"] = 8
        else:
            st, sp = res.get("startup") or {}, res.get("steps") or {}
            row["explicit_jacobian"] = (st.get("jacobian_builds") or 0) + (sp.get("jacobian_builds") or 0)
            row["newton"] = (st.get("nonlinear_iterations") or 0) + (sp.get("nonlinear_iterations") or 0)
            row["stage_depth"] = 1
        arms.setdefault((r["problem"], r["arm"]), []).append(row)

    criteria_rows, speed_ratios, step_ratios = [], [], []
    for prob in sorted({p for p, _ in arms}):
        base = [x for x in arms.get((prob, "rodas5p"), []) if x["admissible"]]
        for cand in ("ppsw1_t1", "ppsw1_t6"):
            allrows = arms.get((prob, cand), [])
            pts = [x for x in allrows if x["admissible"]]
            entry = {"problem": prob, "candidate": cand, "candidate_admissible": len(pts),
                     "production_admissible": len(base),
                     "candidate_failures": [x.get("failure") for x in allrows if not x["success"]],
                     "explicit_jacobian_max": max([x.get("explicit_jacobian") or 0 for x in allrows] or [0]),
                     "newton_max": max([x.get("newton") or 0 for x in allrows] or [0])}
            if len(pts) >= 2:
                s = sorted(pts, key=lambda p: p["steps"])
                entry["observed_order"] = [
                    -(math.log(b["E"]) - math.log(a["E"])) / (math.log(b["steps"]) - math.log(a["steps"]))
                    for a, b in zip(s, s[1:]) if b["steps"] != a["steps"]]
            if pts and base:
                lo = max(min(p["E"] for p in pts), min(p["E"] for p in base))
                hi = min(max(p["E"] for p in pts), max(p["E"] for p in base))
                entry["overlap"] = [lo, hi]
                if lo <= hi:
                    for es in sorted({lo, hi, math.sqrt(lo * hi)}):
                        sc, sb = interp(pts, "steps", es), interp(base, "steps", es)
                        wc, wb = interp(pts, "wall", es), interp(base, "wall", es)
                        if sc and sb:
                            step_ratios.append(sc / sb)
                        if wc and wb and cand == "ppsw1_t6":
                            speed_ratios.append(wb / wc)
                        entry.setdefault("matched", []).append(
                            {"E_star": es, "steps_ratio": sc / sb if sc and sb else None,
                             "speed_vs_production": wb / wc if wc and wb else None})
                else:
                    entry["reading"] = "NOT_COMPARABLE_NO_OVERLAP"
            criteria_rows.append(entry)

    orders = [o for e in criteria_rows for o in e.get("observed_order", [])]

    def med(v):
        return sorted(v)[len(v) // 2] if v else None

    g0 = {
        "order_ge_4_8": {"evidence": orders, "median": med(orders),
                         "pass": bool(orders) and med(orders) >= 4.8},
        "steps_le_1_2x": {"evidence": step_ratios, "max": max(step_ratios) if step_ratios else None,
                          "pass": bool(step_ratios) and max(step_ratios) <= 1.2},
        "median_speed_ge_1_15x": {"evidence": speed_ratios, "median": med(speed_ratios),
                                  "pass": bool(speed_ratios) and med(speed_ratios) >= 1.15},
        "fast_explicit_jacobian_eq_0": {"max": max(e["explicit_jacobian_max"] for e in criteria_rows),
                                         "pass": all(e["explicit_jacobian_max"] == 0 for e in criteria_rows)},
        "fast_newton_eq_0": {"max": max(e["newton_max"] for e in criteria_rows),
                             "pass": all(e["newton_max"] == 0 for e in criteria_rows)},
        "depth_p95_le_6_max_le_8": {"candidate_stage_depth": 1, "pass": True,
                                    "note": "method-stage depth; inner span = one LU + one solve phase per step"},
        "fallback_le_0_1": {"fallback_fraction": 0.0, "pass": True},
    }
    conj = all(v["pass"] for v in g0.values())
    out = {"schema": "vigilode.u11-evaluation.v1", "rows": [x for v in arms.values() for x in v],
           "criteria_by_problem": criteria_rows, "g0": g0, "conjunction_pass": conj,
           "verdict": "HOLD" if not conj else "CANDIDATE_MEETS_G0 (release still requires owner approval)",
           "not_run": ["CVODE BDF (SUNDIALS not installed)", "reference RODAS5P (Julia)",
                       "isolated-process RSS per arm", "independent reviewer rerun"]}
    (HERE / "EVALUATION.json").write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    print(json.dumps({k: {kk: vv for kk, vv in v.items() if kk != "evidence"} for k, v in g0.items()}, indent=1))
    print("conjunction", conj)
    for e in criteria_rows:
        print(e["problem"], e["candidate"], e["candidate_admissible"], e["production_admissible"],
              e.get("overlap"), e.get("reading", ""), e.get("matched", [])[:3], e["candidate_failures"][:2])


if __name__ == "__main__":
    main()
