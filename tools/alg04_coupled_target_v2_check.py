#!/usr/bin/env python3
"""Gate of research node research/alg04_coupled_target_v2_20261010 (ALG04; see its PREREGISTRATION.md).

Inputs: ALG01's BASE.json (export_base of research/alg01_coupled_stage_target_20261008, recorded on the unmodified
solver source; this node has no base export of its own) and RUNS.json (export_runs_alg04). Arms: legacy, dup_fix,
proj_l2, l2_coupled, coupled_guarded (ALG01 rule, reported) and coupled_guarded2 (gated). Adaptive cells at budget
200, the C4 ladders at budget 20,000 for every arm. Gate items (arm coupled_guarded2):

1. Identity.
   a. `legacy` in RUNS equals BASE's `legacy` bit for bit on every adaptive cell (the whole SPD07 record: output
      times and state bits, attempts, steps, counters) and on every ladder rung: RUNS `arms.legacy` (budget 20,000)
      equals BASE `legacy_pilot_budget` (the same budget) and RUNS `legacy_budget_200` equals BASE `legacy`.
   b. `dup_fix` equals `legacy` bit for bit in states, attempts, accepted and rejected steps: adaptive cells compare
      ok, success, message, t, y_last, attempts, accepted, rejected; ladder rungs compare ok, steps, y_last,
      max_step_error_estimate, rel_max_norm_error, failed_step and error.
   c. `dup_fix`'s JVPs (jvp_vectors) equal `legacy`'s minus `legacy`'s diagnostic_matvecs (the diagnostic residuals
      counted), and `dup_fix` counts no diagnostic residual, on every adaptive cell and ladder rung. (Whether every
      other counter is equal is reported.)
2. Accuracy: in every C1, C2 (except Robertson 1e-11) and C3 cell where the dense twin succeeds and the reference
   is admissible, err(arm) <= 1.5 err(twin). Error, reference and admissibility as ALG01: componentwise
   max |y_i - ref_i| / max(|ref_i|, 1e-10) at the end point; the exact solution (uncertainty 0) or the dense driver
   at rtol 1e-13 with uncertainty = the same metric between its 1e-12 and 1e-13 runs; admissible when the
   uncertainty is <= 0.1 err(twin).
3. Ladders (budget 20,000): at every rung of diagpr128 and semilin128, rel(arm) <= 3 rel(LU) + 1e-13 (relative
   max-norm against the exact solution); on semilin64 at rtol 1e-6 two consecutive observed order slopes >= 4.5
   (slope_k = log2(e_k / e_(k+1)), observed when e_(k+1) > 1e-12, as ALG01).
4. Robustness: in every gated cell (C1, C2 except Robertson 1e-11, C3, and the C4 rungs at budget 20,000) the arm
   succeeds wherever `legacy` succeeds and has no more linear-solve failures (a rung that stops in a failed linear
   solve counts one failure).
5. The target's own work gain: over brusselator-1d-50 and brusselator-1d-160 at 1e-6 and 1e-8 (C1), the geometric
   mean of coupled_guarded2 / proj_l2 JVPs (jvp_vectors) per accepted step is <= 0.85, and each cell's ratio is
   <= 1.0.
6. Small n no worse than the attribution arm: on hires, robertson and van-der-pol-mu1000 at 1e-6 and 1e-8 (C1),
   coupled_guarded2's JVPs per accepted step are <= 1.10 x dup_fix's.

Interpretations fixed before the recorded run (this checker is committed before RUNS.json exists):
- CoupledGuarded2's small-system exhaustion (n <= restart = 40): an in-cycle confirmation is taken only at a column
  that ends the Krylov space (happy breakdown, projected residual <= 16 eps ||D b||, or the cycle's last column);
  the nu-guard code is ALG01's unchanged, so nu is evaluated at every column whose projected residual meets the
  (tightened) threshold outside the confirmation gap (used >= next_check), running maximum, also between the first
  would-pass column and the end of the space. For n > 40 CoupledGuarded2 is CoupledGuarded bit for bit.
- DupFix runs through the same GMRES-into stage solve as Legacy (the stage-target entry point and the ladder rungs
  both use GMRES into) with only the final diagnostic true residual skipped.
- Item 1 compares the record fields listed above; "JVPs" are jvp_vectors. Items 5 and 6 use the C1 cells and JVPs
  per accepted step (jvp_vectors / accepted). Item 4 covers C1, C2 without Robertson 1e-11, C3 and the 18 rungs at
  budget 20,000; C5 is not gated. The reference admissibility rule is ALG01's, applied to item 2 only.

Everything else is reported: every arm against items 2-6, the C5 frontiers, the stall/nu statistics and the
nu-guard SVD flops (stage_statistics.nu_flops, not in the counters), the attribution ratios (each arm against
legacy and dup_fix), the predictions, and whether the pre-existing staged arms reproduce ALG01's RUNS. Counted work
and endpoint accuracy only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path

SCHEMA = "vigilode-alg04-coupled-target-v2-check-v1"
GATED = "coupled_guarded2"
ARMS = ("legacy", "dup_fix", "proj_l2", "l2_coupled", "coupled_guarded", "coupled_guarded2")
NON_LEGACY = ARMS[1:]
EXCLUDED_CELLS = {("robertson", 1e-11)}
ACCURACY_FACTOR = 1.5
REFERENCE_ADMISSIBLE = 0.1
LADDER_FACTOR = 3.0
LADDER_ALLOWANCE = 1e-13
SLOPE_MIN = 4.5
SLOPE_FLOOR = 1e-12
TARGET_CELLS = [("brusselator-1d-50", 1e-6), ("brusselator-1d-50", 1e-8),
                ("brusselator-1d-160", 1e-6), ("brusselator-1d-160", 1e-8)]
TARGET_GEOMEAN_LIMIT = 0.85
TARGET_CELL_LIMIT = 1.0
SMALL_CELLS = [(case, rtol) for case in ("hires", "robertson", "van-der-pol-mu1000") for rtol in (1e-6, 1e-8)]
SMALL_LIMIT = 1.10
RECORD_KEYS = ("ok", "success", "message", "t", "y_last", "attempts", "accepted", "rejected", "state_reuses",
               "internal_steps", "output_clipped_steps", "counters", "error")
DUP_FIX_ADAPTIVE_KEYS = ("ok", "success", "message", "t", "y_last", "attempts", "accepted", "rejected")
DUP_FIX_LADDER_KEYS = ("ok", "steps", "y_last", "max_step_error_estimate", "rel_max_norm_error", "failed_step",
                       "error")
DIAGNOSTIC_COUNTERS = {"diagnostic_matvecs", "jvp_calls", "jvp_vectors", "linear_matvec_vectors"}


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def vec(hexes: list[str]) -> list[float]:
    return [value(h) for h in hexes]


def componentwise(y: list[float], ref: list[float]) -> float:
    return max(abs(a - b) / max(abs(b), 1e-10) for a, b in zip(y, ref))


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


def geomean(xs: list[float]) -> float | None:
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs and all(x > 0 for x in xs) else None


def record(run: dict) -> dict:
    return {k: run.get(k) for k in RECORD_KEYS}


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
    """Least-squares fit log10(work) = a + b log10(err) over successful runs (ALG01)."""
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


def dup_fix_counters(legacy: dict, dup: dict) -> dict:
    lc, dc = legacy.get("counters"), dup.get("counters")
    if lc is None or dc is None:
        return {"present": False, "jvp_identity": lc is None and dc is None, "other_counters_equal": lc == dc}
    diagnostic = lc.get("diagnostic_matvecs", 0)
    jvp_ok = dc["jvp_vectors"] == lc["jvp_vectors"] - diagnostic and dc.get("diagnostic_matvecs", 0) == 0
    others = sorted(k for k in set(lc) | set(dc) if k not in DIAGNOSTIC_COUNTERS and lc.get(k) != dc.get(k))
    return {"present": True, "legacy_jvp": lc["jvp_vectors"], "dup_fix_jvp": dc["jvp_vectors"],
            "legacy_diagnostic": diagnostic, "jvp_identity": jvp_ok, "other_counters_differing": others,
            "other_counters_equal": not others}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--alg01-runs", type=Path,
                        default=Path("research/alg01_coupled_stage_target_20261008/RUNS.json"))
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = json.loads(args.base.read_text())
    runs = json.loads(args.runs.read_text())
    alg01 = json.loads(args.alg01_runs.read_text()) if args.alg01_runs.exists() else None
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
    for r in runs["rows"]:
        if set(r["arms"]) != set(ARMS):
            raise SystemExit(f"arms differ in {key(r)}")
    for r in runs["ladders"]:
        if set(r["arms"]) != set(ARMS):
            raise SystemExit(f"ladder arms differ in {lkey(r)}")

    # ---------------------------------------------------------------- item 1
    identity = []
    for k in sorted(base_rows):
        b, r = base_rows[k], run_rows[k]
        legacy, dup = r["arms"]["legacy"], r["arms"]["dup_fix"]
        identity.append({
            "cell": list(k),
            "legacy_equals_base": legacy == b["legacy"],
            "dup_fix_equals_legacy": all(dup.get(f) == legacy.get(f) for f in DUP_FIX_ADAPTIVE_KEYS),
            "dup_fix_counters": dup_fix_counters(legacy, dup),
            "twin_equal": r["twin"] == b["twin"],
        })
    ladder_identity = []
    for k in sorted(base_ladders):
        b, r = base_ladders[k], run_ladders[k]
        legacy, dup = r["arms"]["legacy"], r["arms"]["dup_fix"]
        ladder_identity.append({
            "rung": list(k),
            "legacy_20000_equals_base_pilot_budget": legacy == b["legacy_pilot_budget"],
            "legacy_200_equals_base": r["legacy_budget_200"] == b["legacy"],
            "dup_fix_equals_legacy": all(dup.get(f) == legacy.get(f) for f in DUP_FIX_LADDER_KEYS),
            "dup_fix_counters": dup_fix_counters(legacy, dup),
            "lu_equal": r["lu"] == b["lu"],
        })
    item1 = {
        "legacy_reproduces_base": all(x["legacy_equals_base"] for x in identity)
        and all(x["legacy_20000_equals_base_pilot_budget"] and x["legacy_200_equals_base"] for x in ladder_identity),
        "dup_fix_equals_legacy": all(x["dup_fix_equals_legacy"] for x in identity + ladder_identity),
        "dup_fix_jvp_identity": all(x["dup_fix_counters"]["jvp_identity"] for x in identity + ladder_identity),
    }
    item1["pass"] = all(item1.values())
    item1["dup_fix_other_counters_equal"] = all(x["dup_fix_counters"]["other_counters_equal"]
                                                for x in identity + ladder_identity)
    item1["twins_and_lu_reproduced"] = all(x["twin_equal"] for x in identity) and all(
        x["lu_equal"] for x in ladder_identity)
    item1["cells"] = len(identity)
    item1["rungs"] = len(ladder_identity)

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
        entry["reference_admissible"] = twin_err is not None and ref["uncertainty"] <= REFERENCE_ADMISSIBLE * twin_err
        for arm in ARMS:
            run = r["arms"][arm]
            c = run.get("counters", {})
            entry["arms"][arm] = {
                "success": succeeded(run), "error": error_of(run, ref["y"]),
                "attempts": run.get("attempts"), "accepted": run.get("accepted"), "rejected": run.get("rejected"),
                "linear_solve_failures": lin_failures(run),
                "jvp_vectors": c.get("jvp_vectors"),
                "inner_products": c.get("orthogonalization_inner_products"),
                "vector_updates": c.get("orthogonalization_vector_updates"),
                "linear_iterations": c.get("linear_iterations"),
                "diagnostic_matvecs": c.get("diagnostic_matvecs"),
                "jvp_per_accepted": per_step(run, "jvp_vectors"),
                "inner_products_per_accepted": per_step(run, "orthogonalization_inner_products"),
                "stage_statistics": run.get("stage_statistics"),
                "message": run.get("message", run.get("error")),
            }
        legacy, dup = entry["arms"]["legacy"], entry["arms"]["dup_fix"]
        for arm in ARMS:
            a = entry["arms"][arm]
            a["jvp_per_accepted_vs_legacy"] = ratio(a["jvp_per_accepted"], legacy["jvp_per_accepted"])
            a["jvp_per_accepted_vs_dup_fix"] = ratio(a["jvp_per_accepted"], dup["jvp_per_accepted"])
            a["inner_products_per_accepted_vs_legacy"] = ratio(a["inner_products_per_accepted"],
                                                               legacy["inner_products_per_accepted"])
            a["error_vs_twin"] = ratio(a["error"], twin_err)
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
                                 "twin_error": c["twin"]["error"], "arm_error": c["arms"][arm]["error"],
                                 "arm_ratio": c["arms"][arm]["error_vs_twin"]})
                continue
            a = c["arms"][arm]
            ok = a["error"] is not None and a["error"] <= ACCURACY_FACTOR * c["twin"]["error"]
            rows.append({"group": c["group"], "case": c["case"], "rtol": c["rtol"], "error": a["error"],
                         "twin_error": c["twin"]["error"], "ratio": a["error_vs_twin"], "pass": ok})
        return {"cells": rows, "excluded": excluded, "pass": bool(rows) and all(x["pass"] for x in rows),
                "worst_ratio": max((x["ratio"] for x in rows if x["ratio"] is not None), default=None),
                "failing": [x for x in rows if not x["pass"]]}

    # ---------------------------------------------------------------- item 3
    def ladders(arm: str) -> dict:
        tracking = []
        for k in sorted(run_ladders):
            name, rtol, rung = k
            if name not in ("diagpr128", "semilin128"):
                continue
            r = run_ladders[k]
            lu, a = r["lu"], r["arms"][arm]
            lu_err = lu.get("rel_max_norm_error")
            a_err = a.get("rel_max_norm_error") if a.get("ok") else None
            ok = a_err is not None and lu_err is not None and a_err <= LADDER_FACTOR * lu_err + LADDER_ALLOWANCE
            tracking.append({"ladder": name, "rtol": rtol, "k": rung, "lu": lu_err, "arm": a_err,
                             "ratio": ratio(a_err, lu_err), "pass": ok, "error": a.get("error"),
                             "jvp_vectors": (a.get("counters") or {}).get("jvp_vectors")})
        sl = {}
        for rtol in sorted({k[1] for k in run_ladders if k[0] == "semilin64"}):
            errs = {k[2]: (run_ladders[k]["arms"][arm].get("rel_max_norm_error")
                           if run_ladders[k]["arms"][arm].get("ok") else None)
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
    def target_gain(arm: str, rival: str = "proj_l2") -> dict:
        rows = []
        for case, rtol in TARGET_CELLS:
            c = by_key[("C1", case, rtol)]
            r = ratio(c["arms"][arm]["jvp_per_accepted"], c["arms"][rival]["jvp_per_accepted"])
            rows.append({"case": case, "rtol": rtol, "ratio": r,
                         "arm_jvp_per_accepted": c["arms"][arm]["jvp_per_accepted"],
                         "rival_jvp_per_accepted": c["arms"][rival]["jvp_per_accepted"],
                         "cell_pass": r is not None and r <= TARGET_CELL_LIMIT})
        rs = [x["ratio"] for x in rows]
        gm = geomean(rs) if all(x is not None for x in rs) else None
        return {"rival": rival, "cells": rows, "geomean": gm, "geomean_limit": TARGET_GEOMEAN_LIMIT,
                "cell_limit": TARGET_CELL_LIMIT,
                "pass": gm is not None and gm <= TARGET_GEOMEAN_LIMIT and all(x["cell_pass"] for x in rows)}

    # ---------------------------------------------------------------- item 6
    def small_n(arm: str) -> dict:
        rows = []
        for case, rtol in SMALL_CELLS:
            c = by_key[("C1", case, rtol)]
            r = c["arms"][arm]["jvp_per_accepted_vs_dup_fix"]
            rows.append({"case": case, "rtol": rtol, "ratio_vs_dup_fix": r,
                         "ratio_vs_legacy": c["arms"][arm]["jvp_per_accepted_vs_legacy"],
                         "arm_jvp_per_accepted": c["arms"][arm]["jvp_per_accepted"],
                         "dup_fix_jvp_per_accepted": c["arms"]["dup_fix"]["jvp_per_accepted"],
                         "pass": r is not None and r <= SMALL_LIMIT})
        return {"cells": rows, "limit": SMALL_LIMIT, "pass": all(x["pass"] for x in rows)}

    def all_items(arm: str) -> dict:
        return {"2_accuracy": accuracy(arm), "3_ladders": ladders(arm), "4_robustness": robustness(arm),
                "5_target_gain": target_gain(arm), "6_small_n": small_n(arm)}

    gated = all_items(GATED)
    gate = {
        "1_identity": item1["pass"],
        "2_accuracy": gated["2_accuracy"]["pass"],
        "3_ladders": gated["3_ladders"]["pass"],
        "4_robustness": gated["4_robustness"]["pass"],
        "5_target_gain": gated["5_target_gain"]["pass"],
        "6_small_n": gated["6_small_n"]["pass"],
    }
    verdict = "PASS" if all(gate.values()) else "FAIL"

    # ------------------------------------------------------------- reported
    reported_arms = {}
    for arm in ("dup_fix", "proj_l2", "l2_coupled", "coupled_guarded"):
        items = all_items(arm)
        reported_arms[arm] = {"items": items, "would_pass": {k: v["pass"] for k, v in items.items()}}

    def frontier_report(case: str) -> dict:
        rows = [c for c in cells if c["group"] == "C5" and c["case"] == case]
        out = {"points": {}, "fits": {}, "regression_ratio_vs_legacy": {}, "cheapest_run_ratio_vs_legacy": {},
               "regression_ratio_vs_dup_fix": {}, "cheapest_run_ratio_vs_dup_fix": {}}
        for arm in ARMS:
            pts = [(c["arms"][arm]["error"], c["arms"][arm]["jvp_vectors"]) for c in rows if c["arms"][arm]["success"]]
            out["points"][arm] = [{"rtol": c["rtol"], "error": c["arms"][arm]["error"],
                                   "jvp_vectors": c["arms"][arm]["jvp_vectors"],
                                   "inner_products": c["arms"][arm]["inner_products"]} for c in rows]
            out["fits"][arm] = frontier(pts)
        for rival in ("legacy", "dup_fix"):
            rf = out["fits"][rival]
            rival_pts = [(c["arms"][rival]["error"], c["arms"][rival]["jvp_vectors"]) for c in rows
                         if c["arms"][rival]["success"]]
            for arm in ARMS:
                if arm == rival:
                    continue
                f = out["fits"][arm]
                if f and rf:
                    lo, hi = max(f["lo"], rf["lo"]), min(f["hi"], rf["hi"])
                    if lo < hi:
                        grid = [lo + (hi - lo) * i / 20 for i in range(21)]
                        logs = [(f["a"] + f["b"] * x) - (rf["a"] + rf["b"] * x) for x in grid]
                        out[f"regression_ratio_vs_{rival}"][arm] = 10 ** (sum(logs) / len(logs))
                arm_pts = [(c["arms"][arm]["error"], c["arms"][arm]["jvp_vectors"]) for c in rows
                           if c["arms"][arm]["success"]]
                ratios = []
                for e_target, _ in rival_pts:
                    ra = [w for e, w in rival_pts if e <= e_target]
                    aa = [w for e, w in arm_pts if e <= e_target]
                    if ra and aa:
                        ratios.append(min(aa) / min(ra))
                out[f"cheapest_run_ratio_vs_{rival}"][arm] = geomean(ratios)
        return out

    def statistics_summary(arm: str, groups: tuple[str, ...]) -> dict:
        tot = {}
        nu_max = 1.0
        for c in cells:
            if c["group"] not in groups:
                continue
            s = c["arms"][arm].get("stage_statistics")
            if not s:
                continue
            for k2, v in s.items():
                if k2 == "nu_max":
                    nu_max = max(nu_max, v)
                elif k2 in ("max_columns", "charge_max"):
                    tot[k2] = max(tot.get(k2, 0), v)
                else:
                    tot[k2] = tot.get(k2, 0) + v
        tot["nu_max"] = nu_max
        return tot

    def per_cell_statistics(arm: str) -> list:
        out = []
        for c in cells:
            s = c["arms"][arm].get("stage_statistics")
            if s:
                out.append({"cell": [c["group"], c["case"], c["rtol"]], "dimension": c["dimension"],
                            **{k2: s.get(k2) for k2 in ("solves", "converged", "stall_accepted", "fallback_accepted",
                                                         "failed", "exhaustion_solves", "nu_evaluations",
                                                         "nu_tightened", "nu_max", "nu_flops", "columns",
                                                         "max_columns", "confirmations", "failed_confirmations")}})
        return out

    def attribution() -> list:
        """JVPs per accepted step of every arm against legacy and dup_fix on the C1 cells."""
        out = []
        for c in cells:
            if c["group"] != "C1":
                continue
            out.append({"case": c["case"], "rtol": c["rtol"], "dimension": c["dimension"],
                        "vs_legacy": {a: c["arms"][a]["jvp_per_accepted_vs_legacy"] for a in NON_LEGACY},
                        "vs_dup_fix": {a: c["arms"][a]["jvp_per_accepted_vs_dup_fix"] for a in ARMS
                                       if a != "dup_fix"}})
        return out

    def nu_flops_vs_counted() -> list:
        """Orthogonalization inner products + vector updates (+ nu-guard SVD flops for the guarded arms), per
        accepted step, against legacy, on the C1 cells (the ALG01 correction's flop comparison)."""
        out = []
        for c in cells:
            if c["group"] != "C1":
                continue
            l = c["arms"]["legacy"]
            lf = (l["inner_products"] or 0) + (l["vector_updates"] or 0)
            row = {"case": c["case"], "rtol": c["rtol"]}
            for arm in ("dup_fix", "proj_l2", "coupled_guarded", "coupled_guarded2"):
                a = c["arms"][arm]
                s = a["stage_statistics"] or {}
                f = (a["inner_products"] or 0) + (a["vector_updates"] or 0) + s.get("nu_flops", 0)
                row[arm] = ratio(ratio(f, a["accepted"]), ratio(lf, l["accepted"]))
            out.append(row)
        return out

    def alg01_reproduction() -> dict | None:
        """The pre-existing arms (legacy, proj_l2, l2_coupled, coupled_guarded) against ALG01's RUNS, and
        coupled_guarded2 against ALG01's coupled_guarded where n > 40 (reported)."""
        if alg01 is None:
            return None
        a_rows = {key(r): r for r in alg01["rows"]}
        a_ladders = {lkey(r): r for r in alg01["ladders"]}
        out = {arm: {"equal": 0, "differ": []} for arm in ("legacy", "proj_l2", "l2_coupled", "coupled_guarded")}
        out["coupled_guarded2_vs_alg01_coupled_guarded_n_gt_40"] = {"equal": 0, "differ": []}
        out["ladders_20000"] = {arm: {"equal": 0, "differ": []}
                                for arm in ("legacy", "proj_l2", "l2_coupled", "coupled_guarded")}
        for k, r in run_rows.items():
            old = a_rows.get(k)
            if old is None:
                continue
            for arm in ("legacy", "proj_l2", "l2_coupled", "coupled_guarded"):
                if record(r["arms"][arm]) == record(old["arms"][arm]):
                    out[arm]["equal"] += 1
                else:
                    out[arm]["differ"].append(list(k))
            if r["dimension"] > 40:
                slot = out["coupled_guarded2_vs_alg01_coupled_guarded_n_gt_40"]
                if record(r["arms"]["coupled_guarded2"]) == record(old["arms"]["coupled_guarded"]):
                    slot["equal"] += 1
                else:
                    slot["differ"].append(list(k))
        for k, r in run_ladders.items():
            old = a_ladders.get(k)
            if old is None:
                continue
            for arm in out["ladders_20000"]:
                fields = ("ok", "steps", "y_last", "max_step_error_estimate", "counters", "error", "failed_step")
                same = all(r["arms"][arm].get(f) == old["arms_pilot_budget"][arm].get(f) for f in fields)
                slot = out["ladders_20000"][arm]
                if same:
                    slot["equal"] += 1
                else:
                    slot["differ"].append(list(k))
        return out

    predictions = {
        "item5_target_gain": gated["5_target_gain"],
        "item2_vig1b_k20": [x for x in gated["2_accuracy"]["cells"] + gated["2_accuracy"]["excluded"]
                            if x["case"] == "vig1b-k20"],
        "item6_hires": [x for x in gated["6_small_n"]["cells"] if x["case"] == "hires"],
    }

    report = {
        "schema": SCHEMA,
        "inputs": {str(args.base): sha256(args.base), str(args.runs): sha256(args.runs),
                   **({str(args.alg01_runs): sha256(args.alg01_runs)} if alg01 is not None else {})},
        "gated_arm": GATED,
        "gate": gate,
        "verdict": verdict,
        "items": {"1_identity": item1, **gated},
        "identity_cells": identity,
        "identity_ladders": ladder_identity,
        "reported": {
            "arms": reported_arms,
            "frontiers": {case: frontier_report(case) for case in ("brusselator-1d-50", "hires")},
            "stage_statistics": {arm: {"gated_cells": statistics_summary(arm, ("C1", "C2", "C3")),
                                       "all_adaptive": statistics_summary(arm, ("C1", "C2", "C3", "C5"))}
                                 for arm in ("proj_l2", "l2_coupled", "coupled_guarded", "coupled_guarded2")},
            "stage_statistics_per_cell": {arm: per_cell_statistics(arm)
                                          for arm in ("coupled_guarded", "coupled_guarded2")},
            "attribution_c1": attribution(),
            "orthogonalization_and_svd_flops_per_accepted_vs_legacy_c1": nu_flops_vs_counted(),
            "alg01_runs_reproduction": alg01_reproduction(),
            "predictions": predictions,
            "excluded_cells": [list(x) for x in EXCLUDED_CELLS],
        },
        "cells": cells,
        "claim_scope": "Counted work (WorkCounters) and endpoint accuracy of the opt-in matrix-free U-form research "
                       "driver; no instruction-count or wall-time claim. The nu-guard SVD flops are reported, not "
                       "counted.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print("verdict:", verdict)
    for k2, v in gate.items():
        print(f"  {k2}: {'PASS' if v else 'FAIL'}")
    print("  identity:", {k2: v for k2, v in item1.items() if not isinstance(v, (list, dict))})
    for x in gated["2_accuracy"]["failing"]:
        print("  accuracy fail:", x)
    print("  accuracy worst ratio:", gated["2_accuracy"]["worst_ratio"])
    for x in gated["3_ladders"]["tracking"]:
        if not x["pass"]:
            print("  ladder fail:", x)
    print("  semilin64 slopes:", {r: [s["slope"] for s in v["slopes"]] for r, v in gated["3_ladders"]["semilin64"].items()})
    for x in gated["4_robustness"]["failing"]:
        print("  robustness fail:", x)
    t = gated["5_target_gain"]
    print("  target gain geomean:", t["geomean"], [(x["case"], x["rtol"], x["ratio"]) for x in t["cells"]])
    for x in gated["6_small_n"]["cells"]:
        print(f"  small n {x['case']:20s} {x['rtol']:.0e} vs dup_fix {x['ratio_vs_dup_fix']} "
              f"{'PASS' if x['pass'] else 'FAIL'}")


if __name__ == "__main__":
    main()
