#!/usr/bin/env python3
"""Gate of research node research/alg03_stage_budget_guard_20261008 (ALG03; see its PREREGISTRATION.md).

Inputs: BASE.json (export_base_alg03: Legacy with budgets 200 and 2000 on D1-D4, the dense twins and the
references, on the registration commit's solver source) and RUNS.json (export_runs_alg03: arms B0, B1, B2, B2nf
and Rbig on D1-D5, the twins of D1-D4 again). Gated arm: B2 (budget 2000, stagnation guard, production fallback).

1. Neutral where the budget never binds: on every D5 cell where B0 never reaches its 200-column budget in a stage
   solve (stage_statistics.budget_exhausted == 0) and never fails a stage solve (linear_solve_failures == 0),
   B2 equals B0 bit for bit (output times and state bits, attempts, counters: the SPD07 record).
2. No livelock: B2 completes every D1-D4 cell that Rbig or the dense twin completes, and Robertson to 4e10 at all
   three rtols.
3. Failures: in every cell B2's linear-solve failures <= B0's, and 0 on brusselator-1d-160 1e-4 and on
   brusselator-1d-300 (both rtols).
4. Accuracy: in every D1-D4 cell whose twin completes and whose reference is admissible (as ALG01: exact, or the
   1e-12 vs 1e-13 dense difference <= 0.1 err(twin)), err(B2) <= 1.5 err(twin) with the ALG01 metric
   max_i |y_i - ref_i| / max(|ref_i|, 1e-10).
5. Work: on brusselator-1d-300 at 1e-4, B2's JVPs (jvp_vectors per trajectory) <= 0.80 x B0's and <= 0.80 x Rbig's.

Reported: B1 and B2nf on the same items, guard aborts and their shadow classification (false: the uncounted
continuation converged within the budget), fallback acceptances, the predictions. Counted work and endpoint
accuracy only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

SCHEMA = "vigilode-alg03-budget-guard-check-v1"
GATED = "b2"
ARMS = ("b0", "b1", "b2", "b2nf", "rbig")
RECORD_KEYS = ("ok", "success", "message", "t", "y_last", "attempts", "accepted", "rejected", "state_reuses",
               "internal_steps", "output_clipped_steps", "counters")
ZERO_FAILURE_CELLS = {("brusselator-1d-160", 1e-4), ("brusselator-1d-300", 1e-4), ("brusselator-1d-300", 1e-6)}
WORK_CELL = ("brusselator-1d-300", 1e-4)
WORK_LIMIT = 0.80
ACCURACY_FACTOR = 1.5
REFERENCE_ADMISSIBLE = 0.1


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def vec(hexes: list[str]) -> list[float]:
    return [value(h) for h in hexes]


def componentwise(y: list[float], ref: list[float]) -> float:
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = json.loads(args.base.read_text())
    runs = json.loads(args.runs.read_text())
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
    if sum(1 for k in run_rows if k[0] == "D5") != 14:
        raise SystemExit("RUNS must hold the 14 D5 cells")

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
            entry["base_legacy_200"] = {"success": succeeded(b["legacy_200"]), "failures": failures(b["legacy_200"])}
        for arm in ARMS:
            run = r["arms"][arm]
            c = run.get("counters", {})
            entry["arms"][arm] = {
                "success": succeeded(run),
                "error": componentwise(vec(run["y_last"]), ref["y"]) if (ref and succeeded(run)) else None,
                "attempts": run.get("attempts"), "accepted": run.get("accepted"), "rejected": run.get("rejected"),
                "linear_solve_failures": failures(run), "jvp_vectors": c.get("jvp_vectors"),
                "jvp_per_accepted": ratio(c.get("jvp_vectors"), run.get("accepted")),
                "inner_products": c.get("orthogonalization_inner_products"),
                "stage_statistics": run.get("stage_statistics"),
                "message": run.get("message", run.get("error")),
            }
            if entry.get("twin") and entry["arms"][arm]["error"] is not None:
                entry["arms"][arm]["error_vs_twin"] = ratio(entry["arms"][arm]["error"], twin_err)
        cells.append(entry)
    by_key = {(c["group"], c["case"], c["rtol"]): c for c in cells}
    d14 = [c for c in cells if c["group"] != "D5"]

    # 1. neutrality on D5
    neutral = []
    for c in cells:
        if c["group"] != "D5":
            continue
        k = (c["group"], c["case"], c["rtol"])
        b0, b2 = run_rows[k]["arms"]["b0"], run_rows[k]["arms"]["b2"]
        s0 = b0.get("stage_statistics") or {}
        binds = not (b0.get("ok") and s0.get("budget_exhausted", 1) == 0 and failures(b0) == 0)
        equal = record(b0) == record(b2)
        neutral.append({"case": c["case"], "rtol": c["rtol"], "budget_binds_or_aborts": binds,
                        "b2_equals_b0": equal, "stage_statistics_equal": b0.get("stage_statistics") == b2.get("stage_statistics"),
                        "pass": binds or equal})
    item1 = {"cells": neutral, "eligible": sum(1 for x in neutral if not x["budget_binds_or_aborts"]),
             "pass": all(x["pass"] for x in neutral)}

    # 2. no livelock
    livelock = []
    for c in d14:
        required = c["arms"]["rbig"]["success"] or c["twin"]["success"] or c["case"] == "robertson-4e10"
        ok = c["arms"][GATED]["success"] or not required
        livelock.append({"cell": [c["group"], c["case"], c["rtol"]], "required": required,
                         "b2_success": c["arms"][GATED]["success"], "rbig_success": c["arms"]["rbig"]["success"],
                         "twin_success": c["twin"]["success"], "pass": ok})
    item2 = {"cells": livelock, "pass": all(x["pass"] for x in livelock)}

    # 3. failures
    def failure_item(arm: str) -> dict:
        rows = []
        for c in cells:
            a, b0 = c["arms"][arm]["linear_solve_failures"], c["arms"]["b0"]["linear_solve_failures"]
            ok = a is not None and b0 is not None and a <= b0
            zero = (c["case"], c["rtol"]) in ZERO_FAILURE_CELLS and c["group"] == "D1"
            if zero:
                ok = ok and a == 0
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "failures": a, "b0_failures": b0,
                         "must_be_zero": zero, "pass": ok})
        return {"cells": rows, "pass": all(x["pass"] for x in rows), "failing": [x for x in rows if not x["pass"]]}

    # 4. accuracy
    def accuracy_item(arm: str) -> dict:
        rows, excluded = [], []
        for c in d14:
            if not c["twin"]["success"] or not c["reference_admissible"]:
                excluded.append({"cell": [c["group"], c["case"], c["rtol"]], "twin_success": c["twin"]["success"],
                                 "reference_uncertainty": c["reference_uncertainty"], "twin_error": c["twin"]["error"]})
                continue
            a = c["arms"][arm]
            ok = a["error"] is not None and a["error"] <= ACCURACY_FACTOR * c["twin"]["error"]
            rows.append({"cell": [c["group"], c["case"], c["rtol"]], "error": a["error"],
                         "twin_error": c["twin"]["error"], "ratio": ratio(a["error"], c["twin"]["error"]), "pass": ok})
        return {"cells": rows, "excluded": excluded, "pass": bool(rows) and all(x["pass"] for x in rows)}

    # 5. work
    def work_item(arm: str) -> dict:
        c = by_key[("D1", *WORK_CELL)]
        a, b0, rb = (c["arms"][x]["jvp_vectors"] for x in (arm, "b0", "rbig"))
        r0, rr = ratio(a, b0), ratio(a, rb)
        ok = c["arms"][arm]["success"] and r0 is not None and rr is not None and r0 <= WORK_LIMIT and rr <= WORK_LIMIT
        return {"case": WORK_CELL[0], "rtol": WORK_CELL[1], "jvp": a, "b0_jvp": b0, "rbig_jvp": rb,
                "ratio_vs_b0": r0, "ratio_vs_rbig": rr,
                "per_accepted_ratio_vs_b0": ratio(c["arms"][arm]["jvp_per_accepted"], c["arms"]["b0"]["jvp_per_accepted"]),
                "per_accepted_ratio_vs_rbig": ratio(c["arms"][arm]["jvp_per_accepted"], c["arms"]["rbig"]["jvp_per_accepted"]),
                "pass": bool(ok)}

    items = {"1_neutral_where_budget_never_binds": item1, "2_no_livelock": item2,
             "3_failures": failure_item(GATED), "4_accuracy": accuracy_item(GATED), "5_work": work_item(GATED)}
    gate = {k: v["pass"] for k, v in items.items()}
    verdict = "PASS" if all(gate.values()) else "FAIL"

    def guard_summary(arm: str) -> dict:
        out = []
        for c in cells:
            s = c["arms"][arm].get("stage_statistics")
            if s:
                out.append({"cell": [c["group"], c["case"], c["rtol"]],
                            **{k: s[k] for k in ("guard_contraction", "guard_overrun", "guard_false", "guard_true",
                                                 "fallback_accepted", "fallback_after_guard", "budget_exhausted",
                                                 "failed", "stall_accepted", "max_columns", "nu_max")}})
        return {"cells": out, "totals": {k: sum(x[k] for x in out) for k in (
            "guard_contraction", "guard_overrun", "guard_false", "guard_true", "fallback_accepted",
            "fallback_after_guard", "budget_exhausted", "failed", "stall_accepted")}}

    reported = {
        arm: {"3_failures": failure_item(arm), "4_accuracy": accuracy_item(arm), "5_work": work_item(arm),
              "completes": {f"{c['group']} {c['case']} {c['rtol']:g}": c["arms"][arm]["success"] for c in d14}}
        for arm in ("b1", "b2nf")
    }
    stosc = {f"{c['rtol']:g}": ratio(c["arms"][GATED]["jvp_vectors"], c["arms"]["rbig"]["jvp_vectors"])
             for c in d14 if c["case"].startswith("stosc")}
    predictions = {
        "b2nf_completes_robertson_4e10": {f"{c['rtol']:g}": c["arms"]["b2nf"]["success"] for c in d14
                                          if c["case"] == "robertson-4e10"},
        "b2_robertson_4e10_error_vs_twin": {f"{c['rtol']:g}": c["arms"][GATED].get("error_vs_twin") for c in d14
                                            if c["case"] == "robertson-4e10"},
        "b2_brusselator_300_1e-4": items["5_work"],
        "b2_vs_rbig_stosc_jvp": stosc,
        "guard_fires_on_d1": {f"{c['case']} {c['rtol']:g}": (c["arms"][GATED]["stage_statistics"] or {}).get("guard_contraction", 0)
                              + (c["arms"][GATED]["stage_statistics"] or {}).get("guard_overrun", 0)
                              for c in d14 if c["group"] == "D1"},
    }
    report = {
        "schema": SCHEMA,
        "inputs": {str(args.base): sha256(args.base), str(args.runs): sha256(args.runs)},
        "gated_arm": GATED,
        "gate": gate,
        "verdict": verdict,
        "items": items,
        "reported": {"arms": reported, "guard": {arm: guard_summary(arm) for arm in ("b2", "b2nf")},
                     "fallback": {arm: guard_summary(arm)["totals"]["fallback_accepted"] for arm in ("b0", "b1", "b2")},
                     "predictions": predictions,
                     "twins_reproduced": all(c["twin"]["equal_to_base"] for c in d14),
                     "rbig_equals_base_legacy_2000": all(c["rbig_equals_base_legacy_2000"] for c in d14)},
        "cells": cells,
        "claim_scope": "Counted work (WorkCounters) and endpoint accuracy of the opt-in matrix-free U-form research "
                       "driver; no instruction-count or wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print("verdict:", verdict)
    for k, v in gate.items():
        print(f"  {k}: {'PASS' if v else 'FAIL'}")
    for x in items["3_failures"]["failing"]:
        print("  failures fail:", x)
    for x in items["4_accuracy"]["cells"]:
        print("  accuracy:", x)
    print("  work:", items["5_work"])
    print("  livelock:", [x for x in item2["cells"] if not x["pass"]])
    print("  neutral:", [x for x in item1["cells"] if not x["pass"]], "eligible", item1["eligible"])


if __name__ == "__main__":
    main()
