#!/usr/bin/env python3
"""Gate of research node research/alg01_coupled_stage_target_20261008 (ALG01; see its PREREGISTRATION.md).

Inputs: BASE.json (export_base, on the registration commit's solver source), RUNS.json (export_runs) and
SPD07's BASE.json. Gated arm: `coupled_guarded` (CoupledGuarded). Gate items:

1. Base reproduction: `legacy` in RUNS equals BASE bit for bit on every adaptive cell and every ladder rung
   (states, attempts, counters: the whole record), and BASE's C1 rows equal SPD07's `gmres_into_zero` rows.
2. Accuracy: in every C1, C2 (except Robertson 1e-11) and C3 cell where the dense twin succeeds and the reference
   is admissible, err(arm) <= 1.5 err(twin). Error: componentwise max |y_i - ref_i| / max(|ref_i|, 1e-10) at the
   end point. Reference: the exact solution where one exists (uncertainty 0), else the dense fast driver at rtol
   1e-13, with uncertainty = the same metric between the 1e-12 and 1e-13 dense runs; a cell is admissible when
   uncertainty <= 0.1 err(twin).
3. Ladders (budget 200, the registered budget): at every rung of diagpr128 and semilin128,
   rel(arm) <= 3 rel(LU) + 1e-13 (relative max-norm max|y - ref| / max|ref| against the exact solution); on
   semilin64 at rtol 1e-6 two consecutive observed order slopes >= 4.5, slope_k = log2(e_k / e_(k+1)) of the
   relative max-norm error, observed when e_(k+1) > 1e-12 (the pilot's floor; final.py `slopes`).
4. Robustness: in every gated cell (C1, C2 except Robertson 1e-11, C3, C4 rungs at budget 200) the arm succeeds
   wherever `legacy` succeeds and has no more linear-solve failures than `legacy` (a ladder rung that fails in a
   linear solve counts one failure).
5. Work at equal rtol, JVPs (jvp_vectors) per accepted step against `legacy`: brusselator-1d-50 <= 0.40 at 1e-6
   and 1e-8 with orthogonalization inner products per accepted step <= 0.35; brusselator-1d-160 <= 0.60;
   hires, robertson, van-der-pol-mu1000 <= 0.85 at 1e-6 and 1e-8.

Everything else is reported (the other arms against the same items, the C5 frontiers, stall/fallback/nu
statistics, the pilot's ladder budget 20000, the finite-difference Brusselator-50 variant, the predictions).
Counted work and endpoint accuracy only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path

SCHEMA = "vigilode-alg01-coupled-target-check-v1"
GATED = "coupled_guarded"
ARMS = ("legacy", "proj_l2", "l2_coupled", "coupled", "coupled_guarded")
STAGED = ARMS[1:]
EXCLUDED_CELLS = {("robertson", 1e-11)}
ACCURACY_FACTOR = 1.5
REFERENCE_ADMISSIBLE = 0.1
LADDER_FACTOR = 3.0
LADDER_ALLOWANCE = 1e-13
SLOPE_MIN = 4.5
SLOPE_FLOOR = 1e-12
WORK_LIMITS = {
    "brusselator-1d-50": (0.40, 0.35),
    "brusselator-1d-160": (0.60, None),
    "hires": (0.85, None),
    "robertson": (0.85, None),
    "van-der-pol-mu1000": (0.85, None),
}
WORK_RTOLS = (1e-6, 1e-8)


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def vec(hexes: list[str]) -> list[float]:
    return [value(h) for h in hexes]


def componentwise(y: list[float], ref: list[float]) -> float:
    return max(abs(a - b) / max(abs(b), 1e-10) for a, b in zip(y, ref))


def rel_max_norm(y: list[float], ref: list[float]) -> float:
    scale = max(max(abs(v) for v in ref), 2.2250738585072014e-308)
    return max(abs(a - b) for a, b in zip(y, ref)) / scale


def succeeded(run: dict | None) -> bool:
    return bool(run and run.get("ok") and run.get("success"))


def error_of(run: dict, ref: list[float]) -> float | None:
    return componentwise(vec(run["y_last"]), ref) if succeeded(run) else None


def lin_failures(run: dict) -> int | None:
    if not run.get("ok"):
        return None
    return run["counters"].get("linear_solve_failures", 0)


def per_step(run: dict, key: str) -> float | None:
    if not succeeded(run) or not run.get("accepted"):
        return None
    return run["counters"][key] / run["accepted"]


def ratio(a: float | None, b: float | None) -> float | None:
    if a is None or b is None or b == 0:
        return None
    return a / b


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reference_info(refs: dict, case: str) -> dict:
    r = refs[case]
    y = vec(r["y"])
    if r["kind"].startswith("exact"):
        return {"y": y, "kind": r["kind"], "uncertainty": 0.0}
    return {"y": y, "kind": r["kind"], "uncertainty": componentwise(vec(r["y_1e-12"]), y)}


def ladder_failures(run: dict) -> int:
    """A rung that stops in a failed linear solve counts one linear-solve failure."""
    return 0 if run.get("ok") else int("linear solve" in run.get("error", ""))


def slopes(errors: dict[int, float | None]) -> list[dict]:
    out = []
    ks = sorted(errors)
    for k0, k1 in zip(ks, ks[1:]):
        e0, e1 = errors[k0], errors[k1]
        observed = e0 is not None and e1 is not None and e1 > SLOPE_FLOOR and e0 > 0
        out.append({"k": k0, "slope": math.log2(e0 / e1) if (e0 and e1) else None, "observed": observed})
    return out


def two_consecutive(sl: list[dict]) -> bool:
    for a, b in zip(sl, sl[1:]):
        if a["observed"] and b["observed"] and a["slope"] >= SLOPE_MIN and b["slope"] >= SLOPE_MIN:
            return True
    return False


def frontier(points: list[tuple[float, float]]) -> dict | None:
    """Least-squares fit log10(work) = a + b log10(err) over successful runs."""
    pts = [(math.log10(e), math.log10(w)) for e, w in points if e and e > 0 and w and w > 0]
    if len(pts) < 3:
        return None
    n = len(pts)
    mx = sum(p[0] for p in pts) / n
    my = sum(p[1] for p in pts) / n
    sxx = sum((p[0] - mx) ** 2 for p in pts)
    sxy = sum((p[0] - mx) * (p[1] - my) for p in pts)
    b = sxy / sxx if sxx else 0.0
    return {"a": my - b * mx, "b": b, "lo": min(p[0] for p in pts), "hi": max(p[0] for p in pts)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--spd07", type=Path,
                        default=Path("research/spd07_mf_step_warm_start_20261007/BASE.json"))
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = json.loads(args.base.read_text())
    runs = json.loads(args.runs.read_text())
    spd07 = json.loads(args.spd07.read_text())
    refs = {case: reference_info(base["references"], case) for case in base["references"]}

    key = lambda r: (r["group"], r["case"], r["rtol"])
    base_rows = {key(r): r for r in base["rows"]}
    run_rows = {key(r): r for r in runs["rows"]}
    if set(base_rows) != set(run_rows):
        raise SystemExit("BASE and RUNS cell sets differ")
    lkey = lambda r: (r["ladder"], r["rtol"], r["k"])
    base_ladders = {lkey(r): r for r in base["ladders"]}
    run_ladders = {lkey(r): r for r in runs["ladders"]}
    if set(base_ladders) != set(run_ladders):
        raise SystemExit("BASE and RUNS ladder sets differ")

    # ---------------------------------------------------------------- item 1
    identity = []
    for k in sorted(base_rows):
        b, r = base_rows[k], run_rows[k]
        identity.append({"cell": list(k), "legacy_equal": r["arms"]["legacy"] == b["legacy"],
                         "twin_equal": r["twin"] == b["twin"]})
    ladder_identity = []
    for k in sorted(base_ladders):
        b, r = base_ladders[k], run_ladders[k]
        ladder_identity.append({"rung": list(k), "legacy_equal": r["arms"]["legacy"] == b["legacy"],
                                "legacy_pilot_budget_equal": r["arms_pilot_budget"]["legacy"] == b["legacy_pilot_budget"],
                                "lu_equal": r["lu"] == b["lu"]})
    spd = {(r["case"], r["rtol"]): r["gmres_into_zero"] for r in spd07["rows"]}
    c1 = [r for r in base["rows"] if r["group"] == "C1"]
    spd07_equal = [{"case": r["case"], "rtol": r["rtol"], "equal": r["legacy"] == spd.get((r["case"], r["rtol"]))}
                   for r in c1]
    item1 = {
        "legacy_runs_equal_base": all(x["legacy_equal"] for x in identity)
        and all(x["legacy_equal"] and x["legacy_pilot_budget_equal"] for x in ladder_identity),
        "base_c1_equals_spd07": len(c1) == 14 and len(spd) == 14 and all(x["equal"] for x in spd07_equal),
    }
    item1["pass"] = all(item1.values())
    item1["twins_reproduced"] = all(x["twin_equal"] for x in identity) and all(x["lu_equal"] for x in ladder_identity)

    # ------------------------------------------------------------- cell table
    cells = []
    for k in sorted(run_rows, key=lambda k: (k[0], k[1], -k[2])):
        group, case, rtol = k
        r = run_rows[k]
        ref = refs[case]
        twin_err = error_of(r["twin"], ref["y"])
        entry = {"group": group, "case": case, "rtol": rtol, "dimension": r["dimension"],
                 "reference_kind": ref["kind"], "reference_uncertainty": ref["uncertainty"],
                 "twin": {"success": succeeded(r["twin"]), "error": twin_err,
                          "attempts": r["twin"].get("attempts")},
                 "arms": {}}
        admissible = twin_err is not None and ref["uncertainty"] <= REFERENCE_ADMISSIBLE * twin_err
        entry["reference_admissible"] = admissible
        for arm in ARMS:
            run = r["arms"][arm]
            stats = run.get("stage_statistics")
            entry["arms"][arm] = {
                "success": succeeded(run), "error": error_of(run, ref["y"]),
                "attempts": run.get("attempts"), "accepted": run.get("accepted"), "rejected": run.get("rejected"),
                "linear_solve_failures": lin_failures(run),
                "jvp_vectors": run.get("counters", {}).get("jvp_vectors"),
                "inner_products": run.get("counters", {}).get("orthogonalization_inner_products"),
                "vector_updates": run.get("counters", {}).get("orthogonalization_vector_updates"),
                "linear_iterations": run.get("counters", {}).get("linear_iterations"),
                "jvp_per_accepted": per_step(run, "jvp_vectors"),
                "inner_products_per_accepted": per_step(run, "orthogonalization_inner_products"),
                "stage_statistics": stats,
                "message": run.get("message", run.get("error")),
            }
        legacy = entry["arms"]["legacy"]
        for arm in STAGED:
            a = entry["arms"][arm]
            a["jvp_per_accepted_vs_legacy"] = ratio(a["jvp_per_accepted"], legacy["jvp_per_accepted"])
            a["inner_products_per_accepted_vs_legacy"] = ratio(a["inner_products_per_accepted"],
                                                               legacy["inner_products_per_accepted"])
            a["jvp_total_vs_legacy"] = ratio(a["jvp_vectors"], legacy["jvp_vectors"])
            a["error_vs_twin"] = ratio(a["error"], twin_err)
            a["error_vs_legacy"] = ratio(a["error"], legacy["error"])
        cells.append(entry)
    by_key = {(c["group"], c["case"], c["rtol"]): c for c in cells}
    gated_cells = [c for c in cells if c["group"] in ("C1", "C2", "C3") and (c["case"], c["rtol"]) not in EXCLUDED_CELLS]

    # ---------------------------------------------------------------- item 2
    def accuracy(arm: str) -> dict:
        rows, excluded = [], []
        for c in gated_cells:
            if not c["twin"]["success"] or not c["reference_admissible"]:
                excluded.append({"group": c["group"], "case": c["case"], "rtol": c["rtol"],
                                 "twin_success": c["twin"]["success"],
                                 "reference_uncertainty": c["reference_uncertainty"],
                                 "twin_error": c["twin"]["error"]})
                continue
            a = c["arms"][arm]
            ok = a["error"] is not None and a["error"] <= ACCURACY_FACTOR * c["twin"]["error"]
            rows.append({"group": c["group"], "case": c["case"], "rtol": c["rtol"], "error": a["error"],
                         "twin_error": c["twin"]["error"], "ratio": a["error_vs_twin"], "pass": ok})
        return {"cells": rows, "excluded": excluded, "pass": bool(rows) and all(x["pass"] for x in rows),
                "worst_ratio": max((x["ratio"] for x in rows if x["ratio"] is not None), default=None),
                "failing": [x for x in rows if not x["pass"]]}

    # ---------------------------------------------------------------- item 3
    def ladders(arm: str, field: str = "arms") -> dict:
        tracking = []
        for k in sorted(run_ladders):
            name, rtol, rung = k
            if name not in ("diagpr128", "semilin128"):
                continue
            r = run_ladders[k]
            lu, a = r["lu"], r[field][arm]
            lu_err = lu.get("rel_max_norm_error")
            a_err = a.get("rel_max_norm_error") if a.get("ok") else None
            ok = a_err is not None and lu_err is not None and a_err <= LADDER_FACTOR * lu_err + LADDER_ALLOWANCE
            tracking.append({"ladder": name, "rtol": rtol, "k": rung, "lu": lu_err, "arm": a_err,
                             "ratio": ratio(a_err, lu_err), "pass": ok, "error": a.get("error")})
        sl = {}
        for rtol in sorted({k[1] for k in run_ladders if k[0] == "semilin64"}):
            errs = {k[2]: (run_ladders[k][field][arm].get("rel_max_norm_error")
                           if run_ladders[k][field][arm].get("ok") else None)
                    for k in run_ladders if k[0] == "semilin64" and k[1] == rtol}
            lu_errs = {k[2]: run_ladders[k]["lu"].get("rel_max_norm_error")
                       for k in run_ladders if k[0] == "semilin64" and k[1] == rtol}
            s = slopes(errs)
            sl[f"{rtol:g}"] = {"errors": errs, "slopes": s, "two_consecutive_ge_4_5": two_consecutive(s),
                               "lu_slopes": slopes(lu_errs)}
        gated = sl.get("1e-06", {}).get("two_consecutive_ge_4_5", False)
        return {"tracking": tracking, "tracking_pass": bool(tracking) and all(x["pass"] for x in tracking),
                "semilin64": sl, "semilin64_slope_pass": gated,
                "pass": bool(tracking) and all(x["pass"] for x in tracking) and gated}

    # ---------------------------------------------------------------- item 4
    def robustness(arm: str) -> dict:
        rows = []
        for c in gated_cells:
            a, l = c["arms"][arm], c["arms"]["legacy"]
            ok = (a["success"] or not l["success"]) and (
                a["linear_solve_failures"] is not None and l["linear_solve_failures"] is not None
                and a["linear_solve_failures"] <= l["linear_solve_failures"])
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "arm_success": a["success"],
                         "legacy_success": l["success"], "arm_failures": a["linear_solve_failures"],
                         "legacy_failures": l["linear_solve_failures"], "pass": ok})
        for k in sorted(run_ladders):
            r = run_ladders[k]
            a, l = r["arms"][arm], r["arms"]["legacy"]
            ok = (a.get("ok") or not l.get("ok")) and ladder_failures(a) <= ladder_failures(l)
            rows.append({"cell": ["C4", *k], "arm_success": bool(a.get("ok")), "legacy_success": bool(l.get("ok")),
                         "arm_failures": ladder_failures(a), "legacy_failures": ladder_failures(l), "pass": ok})
        return {"cells": rows, "pass": all(x["pass"] for x in rows), "failing": [x for x in rows if not x["pass"]]}

    # ---------------------------------------------------------------- item 5
    def work(arm: str) -> dict:
        rows = []
        for case, (jvp_limit, dot_limit) in WORK_LIMITS.items():
            for rtol in WORK_RTOLS:
                c = by_key[("C1", case, rtol)]
                a = c["arms"][arm]
                jr, dr = a["jvp_per_accepted_vs_legacy"], a["inner_products_per_accepted_vs_legacy"]
                ok = jr is not None and jr <= jvp_limit and (dot_limit is None or (dr is not None and dr <= dot_limit))
                rows.append({"case": case, "rtol": rtol, "jvp_per_accepted_ratio": jr, "jvp_limit": jvp_limit,
                             "inner_products_per_accepted_ratio": dr, "inner_products_limit": dot_limit,
                             "jvp_total_ratio": a["jvp_total_vs_legacy"], "pass": ok})
        return {"cells": rows, "pass": all(x["pass"] for x in rows)}

    def all_items(arm: str) -> dict:
        return {"2_accuracy": accuracy(arm), "3_ladders": ladders(arm), "4_robustness": robustness(arm),
                "5_work": work(arm)}

    gated = all_items(GATED)
    gate = {
        "1_base_reproduction": item1["pass"],
        "2_accuracy": gated["2_accuracy"]["pass"],
        "3_ladders": gated["3_ladders"]["pass"],
        "4_robustness": gated["4_robustness"]["pass"],
        "5_work": gated["5_work"]["pass"],
    }
    verdict = "PASS" if all(gate.values()) else "FAIL"

    # ------------------------------------------------------------- reported
    reported_arms = {}
    for arm in ("proj_l2", "l2_coupled", "coupled"):
        items = all_items(arm)
        reported_arms[arm] = {"items": items, "would_pass": {k: v["pass"] for k, v in items.items()}}
    pilot_budget_ladders = {arm: ladders(arm, "arms_pilot_budget") for arm in ARMS}

    def frontier_report(case: str) -> dict:
        rows = [c for c in cells if c["group"] == "C5" and c["case"] == case]
        out = {"points": {}, "fits": {}, "regression_ratio_vs_legacy": {}, "cheapest_run_ratio_vs_legacy": {}}
        for arm in ARMS:
            pts = [(c["arms"][arm]["error"], c["arms"][arm]["jvp_vectors"]) for c in rows if c["arms"][arm]["success"]]
            out["points"][arm] = [{"rtol": c["rtol"], "error": c["arms"][arm]["error"],
                                   "jvp_vectors": c["arms"][arm]["jvp_vectors"],
                                   "inner_products": c["arms"][arm]["inner_products"]} for c in rows]
            out["fits"][arm] = frontier(pts)
        lf = out["fits"]["legacy"]
        legacy_pts = [(c["arms"]["legacy"]["error"], c["arms"]["legacy"]["jvp_vectors"]) for c in rows
                      if c["arms"]["legacy"]["success"]]
        for arm in STAGED:
            f = out["fits"][arm]
            if f and lf:
                lo, hi = max(f["lo"], lf["lo"]), min(f["hi"], lf["hi"])
                if lo < hi:
                    grid = [lo + (hi - lo) * i / 20 for i in range(21)]
                    logs = [(f["a"] + f["b"] * x) - (lf["a"] + lf["b"] * x) for x in grid]
                    out["regression_ratio_vs_legacy"][arm] = 10 ** (sum(logs) / len(logs))
            arm_pts = [(c["arms"][arm]["error"], c["arms"][arm]["jvp_vectors"]) for c in rows if c["arms"][arm]["success"]]
            ratios = []
            for e_target, _ in legacy_pts:
                la = [w for e, w in legacy_pts if e <= e_target]
                aa = [w for e, w in arm_pts if e <= e_target]
                if la and aa:
                    ratios.append(min(aa) / min(la))
            out["cheapest_run_ratio_vs_legacy"][arm] = (
                math.exp(sum(math.log(x) for x in ratios) / len(ratios)) if ratios else None)
        return out

    def statistics_summary(arm: str) -> dict:
        tot = {}
        nu_max = 1.0
        for c in cells:
            s = c["arms"][arm].get("stage_statistics")
            if not s:
                continue
            for k2, v in s.items():
                if k2 == "nu_max":
                    nu_max = max(nu_max, v)
                elif k2 == "max_columns":
                    tot[k2] = max(tot.get(k2, 0), v)
                else:
                    tot[k2] = tot.get(k2, 0) + v
        tot["nu_max"] = nu_max
        return tot

    def fd_report() -> list:
        out = []
        ref = refs["brusselator-1d-50"]["y"]
        for r in runs.get("fd_variant", []):
            row = {"rtol": r["rtol"], "arms": {}}
            for arm, run in r["arms"].items():
                row["arms"][arm] = {"success": succeeded(run), "error": error_of(run, ref),
                                    "attempts": run.get("attempts"), "accepted": run.get("accepted"),
                                    "linear_solve_failures": lin_failures(run),
                                    "jvp_per_accepted": per_step(run, "jvp_vectors"),
                                    "stage_statistics": run.get("stage_statistics")}
            out.append(row)
        return out

    def cguard_vs_coupled() -> dict:
        ratios = []
        for c in cells:
            if c["case"].startswith("vig") or c["group"] == "C5":
                continue
            g, p = c["arms"]["coupled_guarded"]["jvp_vectors"], c["arms"]["coupled"]["jvp_vectors"]
            if g and p and c["arms"]["coupled_guarded"]["success"] and c["arms"]["coupled"]["success"]:
                ratios.append({"cell": [c["group"], c["case"], c["rtol"]], "ratio": g / p})
        return {"cells": ratios, "max": max((x["ratio"] for x in ratios), default=None)}

    predictions = {
        "coupled_jvp_per_accepted_vs_legacy": {
            f"{case} {rtol:g}": by_key[("C1", case, rtol)]["arms"]["coupled"]["jvp_per_accepted_vs_legacy"]
            for case in WORK_LIMITS for rtol in WORK_RTOLS},
        "proj_l2_accuracy_hires_1e-9_1e-10": [x for x in reported_arms["proj_l2"]["items"]["2_accuracy"]["cells"]
                                              + reported_arms["proj_l2"]["items"]["2_accuracy"]["excluded"]
                                              if x["case"] == "hires" and x["rtol"] in (1e-9, 1e-10)],
        "coupled_accuracy_vigb_k20": [x for x in reported_arms["coupled"]["items"]["2_accuracy"]["cells"]
                                      if x["case"] == "vigb-k20"],
        "coupled_guarded_jvp_vs_coupled_outside_vig": cguard_vs_coupled(),
        "l2_coupled_semilin64_slopes": reported_arms["l2_coupled"]["items"]["3_ladders"]["semilin64"],
    }

    report = {
        "schema": SCHEMA,
        "inputs": {str(args.base): sha256(args.base), str(args.runs): sha256(args.runs),
                   str(args.spd07): sha256(args.spd07)},
        "gated_arm": GATED,
        "gate": gate,
        "verdict": verdict,
        "items": {"1_base_reproduction": {**item1, "spd07_c1": spd07_equal}, **gated},
        "identity_cells": identity,
        "identity_ladders": ladder_identity,
        "reported": {
            "arms": reported_arms,
            "ladders_pilot_budget_20000": pilot_budget_ladders,
            "frontiers": {case: frontier_report(case) for case in ("brusselator-1d-50", "hires")},
            "stage_statistics": {arm: statistics_summary(arm) for arm in STAGED},
            "fd_variant_brusselator_50": fd_report(),
            "predictions": predictions,
            "excluded_cells": [list(x) for x in EXCLUDED_CELLS],
        },
        "cells": cells,
        "claim_scope": "Counted work (WorkCounters) and endpoint accuracy of the opt-in matrix-free U-form research "
                       "driver; no instruction-count or wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print("verdict:", verdict)
    for k2, v in gate.items():
        print(f"  {k2}: {'PASS' if v else 'FAIL'}")
    for x in gated["2_accuracy"]["failing"]:
        print("  accuracy fail:", x)
    for x in gated["3_ladders"]["tracking"]:
        if not x["pass"]:
            print("  ladder fail:", x)
    print("  semilin64 slopes:", {r: [s["slope"] for s in v["slopes"]] for r, v in gated["3_ladders"]["semilin64"].items()})
    for x in gated["4_robustness"]["failing"]:
        print("  robustness fail:", x)
    for x in gated["5_work"]["cells"]:
        print(f"  work {x['case']:20s} {x['rtol']:.0e} jvp/acc {x['jvp_per_accepted_ratio']} dots/acc "
              f"{x['inner_products_per_accepted_ratio']} {'PASS' if x['pass'] else 'FAIL'}")


if __name__ == "__main__":
    main()
