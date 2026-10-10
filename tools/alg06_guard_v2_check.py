#!/usr/bin/env python3
"""Gate of research node research/alg06_guard_v2_20261010 (ALG06; see its PREREGISTRATION.md).

Inputs: ALG03's BASE.json (export_base_alg03 of research/alg03_stage_budget_guard_20261008: Legacy with budgets 200
and 2000 on D1-D4, the dense twins and the references, recorded on the unmodified solver source; this node has no
base export of its own) and RUNS.json (export_runs_alg06). Arms, all at budget 2000 on the CoupledGuarded2 target
with the stagnation guard, the production fallback and the Integral controller, cumulative: b2 (ALG03's switches,
reported), b3a (+ G1 effective cycle length), b3b (+ G2 floor at every confirmation), b3 (+ G3 fallback residual
charged to the error estimate; gated); rival rbig (Legacy, budget 2000); and coupled_guarded2_200 (the
CoupledGuarded2 target at budget 200, no guard, on D5 only, for item 1). Gate items (arm b3):

1. Identity.
   a. rbig equals BASE's legacy_2000 bit for bit on every D1-D4 cell (the whole SPD07 record).
   b. On every D5 cell where, in b3, neither the guard (stage_statistics guard_contraction + guard_overrun), nor
      the fallback (fallback_accepted), nor the floor (floor_accepted) ever fires, b3 equals coupled_guarded2_200
      bit for bit (output times and state bits, attempts, steps, counters: the SPD07 record).
2. No livelock: b3 completes every D1-D4 cell that rbig or the dense twin completes, and Robertson to 4e10 at all
   three rtols in any case.
3. Failures: in every cell (D1-D5) b3's linear-solve failures are <= rbig's, and 0 on brusselator-1d-160 at 1e-4
   and on brusselator-1d-300 (both rtols).
4. Accuracy: in every D1-D4 cell whose twin completes and whose reference is admissible (as ALG01/ALG03: exact, or
   the 1e-12 vs 1e-13 dense difference <= 0.1 err(twin)), err(b3) <= 1.5 err(twin), with the ALG01 metric
   max_i |y_i - ref_i| / max(|ref_i|, 1e-10).
5. Work: on brusselator-1d-300 at 1e-4, b3's JVPs per trajectory (jvp_vectors) <= 0.80 x rbig's.

Interpretations fixed before the recorded run (this checker is committed before RUNS.json exists):
- "All on the CoupledGuarded2 target": b2 is ALG03's switch set (stagnation guard with the nominal cycle length,
  production fallback, budget 2000) on the CoupledGuarded2 target, so it equals ALG03's b2 where n > 40 and differs
  (exhaustion) where n <= 40, e.g. Robertson (reported against ALG03's RUNS).
- G1: m_eff = min(restart, n, budget - used) replaces restart in the overrun prediction only; the q >= 0.98 test
  is unchanged.
- G2: the floor is tested at every true residual that misses the threshold - in-cycle confirmations and restart
  boundaries - after the threshold and (at a restart boundary) after the stall rule, before the guard and the
  budget test; acceptances the stall rule makes keep its label.
- G3: ||r_i||_WRMS = ||D r_i||_2 / sqrt(n) with the coupled target's weights D = diag(1 / (atol + rtol |y_n|)) (the
  metric the coupled target is stated in); r_i is the true residual of the accepted iterate; the charges of an
  attempt's fallback-accepted stages are summed (linearly) and added to the attempt's WRMS embedded error; the
  charged error is the attempt's error everywhere downstream (accept/reject, the Integral controller, e_hat).
- Guard aborts are classified by the shadow continuation in every guarded arm, including the aborts the fallback
  accepted (reported only; a contract test shows it changes nothing in a run).
- Item 1b eligibility uses b3's stage_statistics (guard_contraction + guard_overrun, fallback_accepted,
  floor_accepted all zero); equality is the SPD07 record. Item 3 covers all 24 cells against rbig. Item 4 keeps
  ALG03's reference admissibility rule. Item 5 is per trajectory (jvp_vectors), as worded; per accepted step is
  reported.

Reported: b2, b3a and b3b on items 2-5; guard aborts with their shadow-continuation classification (false: the
uncounted continuation without the guard converged within the budget), for aborts the fallback refused
(guard_false/true) and accepted (guard_accepted_false/true); floor and fallback acceptances; the charged error per
attempt (RUNS `charges`: t, h, error, charge, stages, charged) summarised per cell; the predictions; and the
reproduction of ALG03's RUNS (rbig, and b2 against ALG03's b2 where n > 40, where CoupledGuarded2 is
CoupledGuarded). Counted work and endpoint accuracy only; no wall-time claim. The output file is immutable.

Evidence validation (re-audit AS03, finding F104; added after the recorded run, gates unchanged): before any gate
the inputs pass tools/evidence_schema_v2.py (strict JSON, exact key/row/arm sets, unique raw keys, equal nonempty
vector lengths, finite correctly typed numbers, immutable base and twins bound to it). Rejected evidence is written
with verdict INVALID and its reasons, and the checker exits with status 2; INVALID is neither PASS nor FAIL.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
import sys

sys.dont_write_bytecode = True  # keep the historical tools directory free of __pycache__
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-alg06-guard-v2-check-v1"
GATED = "b3"
ARMS = ("b2", "b3a", "b3b", "b3", "rbig")
GUARDED = ("b2", "b3a", "b3b", "b3")
RECORD_KEYS = ("ok", "success", "message", "t", "y_last", "attempts", "accepted", "rejected", "state_reuses",
               "internal_steps", "output_clipped_steps", "counters", "error")
ZERO_FAILURE_CELLS = {("brusselator-1d-160", 1e-4), ("brusselator-1d-300", 1e-4), ("brusselator-1d-300", 1e-6)}
LIVELOCK_CASE = "robertson-4e10"
WORK_CELL = ("brusselator-1d-300", 1e-4)
WORK_LIMIT = 0.80
ACCURACY_FACTOR = 1.5
REFERENCE_ADMISSIBLE = 0.1
GUARD_KEYS = ("guard_contraction", "guard_overrun", "guard_false", "guard_true", "guard_accepted_false",
              "guard_accepted_true", "fallback_accepted", "fallback_after_guard", "floor_accepted", "stall_accepted",
              "budget_exhausted", "failed", "failed_guard", "exhaustion_solves", "solves", "charged_attempts",
              "charge_crosses_one")


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def vec(hexes: list[str]) -> list[float]:
    return [value(h) for h in hexes]


def componentwise(y: list[float], ref: list[float]) -> float:
    evidence.require_metric_pair(y, ref)  # equal nonempty lengths, finite: never a truncated or NaN-skipping max
    return max(abs(a - b) / max(abs(b), 1e-10) for a, b in zip(y, ref))


def succeeded(run: dict | None) -> bool:
    return bool(run and run.get("ok") and run.get("success"))


def failures(run: dict) -> int | None:
    return run["counters"].get("linear_solve_failures", 0) if run.get("ok") else None


def record(run: dict) -> dict:
    return {k: run.get(k) for k in RECORD_KEYS}


def ratio(a, b):
    return None if a is None or b in (None, 0) else a / b


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fires(stats: dict | None) -> dict:
    s = stats or {}
    return {"guard": s.get("guard_contraction", 0) + s.get("guard_overrun", 0),
            "fallback": s.get("fallback_accepted", 0), "floor": s.get("floor_accepted", 0)}


def charge_summary(run: dict) -> dict:
    rows = run.get("charges") or []
    charges = [r[3] for r in rows if r[3] is not None]
    crossing = [r for r in rows if r[2] is not None and r[3] is not None and r[2] <= 1.0 < r[2] + r[3]]
    return {"attempts": len(rows), "charge_sum": sum(charges), "charge_max": max(charges, default=0.0),
            "charge_median": sorted(charges)[len(charges) // 2] if charges else 0.0,
            "crosses_one": len(crossing), "charged": sum(1 for r in rows if r[5]),
            "stages": sum(r[4] for r in rows)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--alg03-runs", type=Path,
                        default=Path("research/alg03_stage_budget_guard_20261008/RUNS.json"))
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    # Evidence validation before any gate: malformed or unbound evidence is INVALID, never PASS/FAIL.
    try:
        validated = evidence.validate_alg06(args.base, args.runs, args.alg03_runs)
    except evidence.ValidationError as exc:
        raise SystemExit(evidence.emit_invalid(args.output, SCHEMA, exc, {
            "base": args.base, "runs": args.runs,
            "alg03_runs": args.alg03_runs if args.alg03_runs.exists() else None}))
    base = validated.docs["base"]
    runs = validated.docs["runs"]
    alg03 = validated.docs.get("alg03_runs")
    refs = {}
    for case, r in base["references"].items():
        y = vec(r["y"])
        unc = 0.0 if r["kind"].startswith("exact") else componentwise(vec(r["y_1e-12"]), y)
        refs[case] = {"y": y, "kind": r["kind"], "uncertainty": unc}
    key = lambda r: (r["group"], r["case"], r["rtol"])
    base_rows = {key(r): r for r in base["rows"]}
    run_rows = {key(r): r for r in runs["rows"]}
    if not set(base_rows) <= set(run_rows):
        raise SystemExit("RUNS lacks BASE cells")
    if sum(1 for k in run_rows if k[0] == "D5") != 14 or len(run_rows) != 24:
        raise SystemExit("RUNS must hold the 10 D1-D4 and the 14 D5 cells")
    for k, r in run_rows.items():
        expected = set(ARMS) | ({"coupled_guarded2_200"} if k[0] == "D5" else set())
        if set(r["arms"]) != expected:
            raise SystemExit(f"arms differ in {k}")

    cells = []
    for k in sorted(run_rows, key=lambda k: (k[0], k[1], -k[2])):
        group, case, rtol = k
        r = run_rows[k]
        entry = {"group": group, "case": case, "rtol": rtol, "dimension": r["dimension"], "arms": {}}
        ref = refs.get(case)
        twin = r.get("twin")
        twin_err = componentwise(vec(twin["y_last"]), ref["y"]) if (twin and succeeded(twin) and ref) else None
        if group != "D5":
            b = base_rows[k]
            entry["twin"] = {"success": succeeded(twin), "error": twin_err, "attempts": twin.get("attempts"),
                             "equal_to_base": twin == b["twin"]}
            entry["reference_kind"] = ref["kind"]
            entry["reference_uncertainty"] = ref["uncertainty"]
            entry["reference_admissible"] = twin_err is not None and ref["uncertainty"] <= REFERENCE_ADMISSIBLE * twin_err
            entry["rbig_equals_base_legacy_2000"] = r["arms"]["rbig"] == b["legacy_2000"]
        for arm in list(ARMS) + (["coupled_guarded2_200"] if group == "D5" else []):
            run = r["arms"][arm]
            c = run.get("counters", {})
            a = {
                "success": succeeded(run),
                "error": componentwise(vec(run["y_last"]), ref["y"]) if (ref and succeeded(run)) else None,
                "attempts": run.get("attempts"), "accepted": run.get("accepted"), "rejected": run.get("rejected"),
                "linear_solve_failures": failures(run), "local_error_failures": c.get("local_error_failures"),
                "jvp_vectors": c.get("jvp_vectors"),
                "jvp_per_accepted": ratio(c.get("jvp_vectors"), run.get("accepted")),
                "inner_products": c.get("orthogonalization_inner_products"),
                "stage_statistics": run.get("stage_statistics"),
                "message": run.get("message", run.get("error")),
            }
            if run.get("stage_statistics") is not None:
                a["charges"] = charge_summary(run)
            if entry.get("twin") and a["error"] is not None:
                a["error_vs_twin"] = ratio(a["error"], twin_err)
            entry["arms"][arm] = a
        cells.append(entry)
    by_key = {(c["group"], c["case"], c["rtol"]): c for c in cells}
    d14 = [c for c in cells if c["group"] != "D5"]

    # 1. identity
    rbig_rows = [{"cell": [c["group"], c["case"], c["rtol"]], "equal": c["rbig_equals_base_legacy_2000"]} for c in d14]
    d5 = []
    for c in cells:
        if c["group"] != "D5":
            continue
        k = (c["group"], c["case"], c["rtol"])
        b3, cg2 = run_rows[k]["arms"][GATED], run_rows[k]["arms"]["coupled_guarded2_200"]
        f = fires(b3.get("stage_statistics"))
        eligible = bool(b3.get("ok")) and b3.get("stage_statistics") is not None and not any(f.values())
        equal = record(b3) == record(cg2)
        d5.append({"case": c["case"], "rtol": c["rtol"], "b3_fires": f, "eligible": eligible, "b3_equals_cg2_200": equal,
                   "b3_budget_exhausted": (b3.get("stage_statistics") or {}).get("budget_exhausted"),
                   "cg2_200_budget_exhausted": (cg2.get("stage_statistics") or {}).get("budget_exhausted"),
                   "pass": (not eligible) or equal})
    item1 = {"rbig_equals_base": rbig_rows, "rbig_pass": len(rbig_rows) == 10 and all(x["equal"] for x in rbig_rows),
             "d5": d5, "d5_eligible": sum(1 for x in d5 if x["eligible"]),
             "d5_pass": all(x["pass"] for x in d5)}
    item1["pass"] = item1["rbig_pass"] and item1["d5_pass"]

    # 2. no livelock
    def livelock_item(arm: str) -> dict:
        rows = []
        for c in d14:
            required = c["arms"]["rbig"]["success"] or c["twin"]["success"] or c["case"] == LIVELOCK_CASE
            ok = c["arms"][arm]["success"] or not required
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "required": required,
                         "arm_success": c["arms"][arm]["success"], "rbig_success": c["arms"]["rbig"]["success"],
                         "twin_success": c["twin"]["success"], "attempts": c["arms"][arm]["attempts"], "pass": ok})
        return {"cells": rows, "pass": all(x["pass"] for x in rows), "failing": [x for x in rows if not x["pass"]]}

    # 3. failures
    def failure_item(arm: str) -> dict:
        rows = []
        for c in cells:
            a, rb = c["arms"][arm]["linear_solve_failures"], c["arms"]["rbig"]["linear_solve_failures"]
            ok = a is not None and rb is not None and a <= rb
            zero = (c["case"], c["rtol"]) in ZERO_FAILURE_CELLS and c["group"] == "D1"
            if zero:
                ok = ok and a == 0
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "failures": a, "rbig_failures": rb,
                         "must_be_zero": zero, "pass": ok})
        return {"cells": rows, "pass": all(x["pass"] for x in rows), "failing": [x for x in rows if not x["pass"]]}

    # 4. accuracy
    def accuracy_item(arm: str) -> dict:
        rows, excluded = [], []
        for c in d14:
            if not c["twin"]["success"] or not c["reference_admissible"]:
                excluded.append({"cell": [c["group"], c["case"], c["rtol"]], "twin_success": c["twin"]["success"],
                                 "reference_uncertainty": c["reference_uncertainty"], "twin_error": c["twin"]["error"],
                                 "arm_error": c["arms"][arm]["error"]})
                continue
            a = c["arms"][arm]
            ok = a["error"] is not None and a["error"] <= ACCURACY_FACTOR * c["twin"]["error"]
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "error": a["error"],
                         "twin_error": c["twin"]["error"], "ratio": ratio(a["error"], c["twin"]["error"]), "pass": ok})
        return {"cells": rows, "excluded": excluded, "pass": bool(rows) and all(x["pass"] for x in rows),
                "failing": [x for x in rows if not x["pass"]]}

    # 5. work
    def work_item(arm: str) -> dict:
        c = by_key[("D1", *WORK_CELL)]
        a, rb = c["arms"][arm]["jvp_vectors"], c["arms"]["rbig"]["jvp_vectors"]
        rr = ratio(a, rb)
        ok = c["arms"][arm]["success"] and rr is not None and rr <= WORK_LIMIT
        return {"case": WORK_CELL[0], "rtol": WORK_CELL[1], "jvp": a, "rbig_jvp": rb, "ratio_vs_rbig": rr,
                "per_accepted_ratio_vs_rbig": ratio(c["arms"][arm]["jvp_per_accepted"],
                                                    c["arms"]["rbig"]["jvp_per_accepted"]),
                "accepted": c["arms"][arm]["accepted"], "rbig_accepted": c["arms"]["rbig"]["accepted"],
                "pass": bool(ok)}

    items = {"1_identity": item1, "2_no_livelock": livelock_item(GATED), "3_failures": failure_item(GATED),
             "4_accuracy": accuracy_item(GATED), "5_work": work_item(GATED)}
    gate = {k: v["pass"] for k, v in items.items()}
    verdict = "PASS" if all(gate.values()) else "FAIL"

    # ------------------------------------------------------------- reported
    def guard_summary(arm: str) -> dict:
        out = []
        for c in cells:
            a = c["arms"][arm]
            s = a.get("stage_statistics")
            if s:
                out.append({"cell": [c["group"], c["case"], c["rtol"]], **{k2: s.get(k2, 0) for k2 in GUARD_KEYS},
                            "max_columns": s.get("max_columns"), "nu_max": s.get("nu_max"),
                            "charge_sum": s.get("charge_sum"), "charge_max": s.get("charge_max"),
                            "charges": a.get("charges")})
        return {"cells": out, "totals": {k2: sum(x[k2] for x in out) for k2 in GUARD_KEYS},
                "totals_d1_d4": {k2: sum(x[k2] for x in out if x["cell"][0] != "D5") for k2 in GUARD_KEYS}}

    reported_arms = {
        arm: {"2_no_livelock": livelock_item(arm), "3_failures": failure_item(arm),
              "4_accuracy": accuracy_item(arm), "5_work": work_item(arm)}
        for arm in ("b2", "b3a", "b3b")
    }
    for arm, v in reported_arms.items():
        v["would_pass"] = {k: v[k]["pass"] for k in ("2_no_livelock", "3_failures", "4_accuracy", "5_work")}

    def alg03_reproduction() -> dict | None:
        if alg03 is None:
            return None
        old = {key(r): r for r in alg03["rows"]}
        out = {"rbig": {"equal": 0, "differ": []}, "b2_vs_alg03_b2_n_gt_40": {"equal": 0, "differ": []},
               "b2_vs_alg03_b2_n_le_40": {"equal": 0, "differ": []}}
        for k, r in run_rows.items():
            o = old.get(k)
            if o is None:
                continue
            slot = out["rbig"]
            if record(r["arms"]["rbig"]) == record(o["arms"]["rbig"]):
                slot["equal"] += 1
            else:
                slot["differ"].append(list(k))
            slot = out["b2_vs_alg03_b2_n_gt_40" if r["dimension"] > 40 else "b2_vs_alg03_b2_n_le_40"]
            if record(r["arms"]["b2"]) == record(o["arms"]["b2"]):
                slot["equal"] += 1
            else:
                slot["differ"].append(list(k))
        return out

    robertson = [c for c in d14 if c["case"] == LIVELOCK_CASE]
    predictions = {
        "b3_brusselator_300_1e-4": items["5_work"],
        "b3_robertson_4e10_error_vs_twin": {f"{c['rtol']:g}": c["arms"][GATED].get("error_vs_twin") for c in robertson},
        "robertson_4e10_error_vs_twin_by_arm": {arm: {f"{c['rtol']:g}": c["arms"][arm].get("error_vs_twin")
                                                      for c in robertson} for arm in ARMS},
        "robertson_4e10_overrun_aborts": {arm: {f"{c['rtol']:g}": (c["arms"][arm]["stage_statistics"] or {}).get(
            "guard_overrun") for c in robertson} for arm in GUARDED},
        "robertson_4e10_contraction_aborts": {arm: {f"{c['rtol']:g}": (c["arms"][arm]["stage_statistics"] or {}).get(
            "guard_contraction") for c in robertson} for arm in GUARDED},
    }
    report = {
        "schema": SCHEMA,
        "inputs": {str(args.base): validated.sha256["base"], str(args.runs): validated.sha256["runs"],
                   **({str(args.alg03_runs): validated.sha256["alg03_runs"]} if alg03 is not None else {})},
        "gated_arm": GATED,
        "gate": gate,
        "verdict": verdict,
        "items": items,
        "reported": {"arms": reported_arms, "guard": {arm: guard_summary(arm) for arm in GUARDED},
                     "predictions": predictions,
                     "twins_reproduced": all(c["twin"]["equal_to_base"] for c in d14),
                     "alg03_runs_reproduction": alg03_reproduction()},
        "cells": cells,
        "claim_scope": "Counted work (WorkCounters) and endpoint accuracy of the opt-in matrix-free U-form research "
                       "driver; no instruction-count or wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print("verdict:", verdict)
    for k, v in gate.items():
        print(f"  {k}: {'PASS' if v else 'FAIL'}")
    print("  identity: rbig", item1["rbig_pass"], "d5 eligible", item1["d5_eligible"],
          "failing", [x for x in item1["d5"] if not x["pass"]])
    for x in items["2_no_livelock"]["failing"]:
        print("  livelock fail:", x)
    for x in items["3_failures"]["failing"]:
        print("  failures fail:", x)
    for x in items["4_accuracy"]["cells"]:
        print("  accuracy:", x["cell"], x["ratio"], "PASS" if x["pass"] else "FAIL")
    print("  work:", {k: items["5_work"][k] for k in ("ratio_vs_rbig", "per_accepted_ratio_vs_rbig", "pass")})
    for arm in ("b2", "b3a", "b3b"):
        print(f"  {arm} would pass:", reported_arms[arm]["would_pass"],
              {f"{c['rtol']:g}": c["arms"][arm].get("error_vs_twin") for c in robertson})


if __name__ == "__main__":
    main()
