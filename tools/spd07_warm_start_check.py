#!/usr/bin/env python3
"""Gate of research node research/spd07_mf_step_warm_start_20261007 (speed research node SPD07;
see its PREREGISTRATION.md).

Inputs: BASE.json (export_base, on the registration commit's solver source) and RUNS.json
(export_runs). Gate on GMRES `solve_into`:

1. Base reproduction: the `zero` and `previous` GMRES-into arms and the GCRO-DR cold `previous`
   arm equal BASE.json exactly (output times and final state bits, attempts, counters, ...).
2. On brusselator-1d-50 and brusselator-1d-160 at both tolerances: `previous_step` linear_matvecs
   <= 0.85 x `previous` and <= 0.90 x `zero`, attempts within max(1, 2 %) of `previous`, zero
   linear-solve failures, final error <= 2 x `previous`'s.
3. No regression: on all 14 cases `previous_step` linear_matvecs <= 1.02 x `previous`.

Everything else is reported (the scaled arm, GCRO-DR cold arms, cycles, ratios). Counted Krylov work
only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

SCHEMA = "vigilode-spd07-warm-start-check-v1"
GATED_CASES = ("brusselator-1d-50", "brusselator-1d-160")
GUESSES = ("zero", "previous", "previous_step", "previous_step_scaled")


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def final_error(arm: dict, reference: list[str]) -> float | None:
    if not (arm.get("ok") and arm.get("success")):
        return None
    ref = [value(x) for x in reference]
    y = [value(x) for x in arm["y_last"]]
    scale = max(max(abs(v) for v in ref), 2.2250738585072014e-308)
    return max(abs(a - b) for a, b in zip(y, ref)) / scale


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
    base = {(r["case"], r["rtol"]): r for r in json.loads(args.base.read_text())["rows"]}
    runs = {(r["case"], r["rtol"]): r for r in json.loads(args.runs.read_text())["rows"]}
    if set(base) != set(runs) or len(base) != 14:
        raise SystemExit(f"case sets differ or are not 14: {sorted(base)} vs {sorted(runs)}")

    reproduction, cases = [], []
    for key in sorted(base):
        b, r = base[key], runs[key]
        prev = dict(r["gmres_into_previous"])
        prev.pop("gmres_into_cycles", None)
        zero = dict(r["gmres_into_zero"])
        zero.pop("gmres_into_cycles", None)
        rep = {
            "case": key[0], "rtol": key[1],
            "gmres_into_zero": zero == b["gmres_into_zero"],
            "gmres_into_previous": prev == b["gmres_into_previous"],
            "gcrodr_cold_previous": r["gcrodr_cold_previous"] == b["gcrodr_cold_previous"],
        }
        reproduction.append(rep)

        reference = b["reference"]["y"]
        entry = {"case": key[0], "rtol": key[1], "dimension": r["dimension"], "arms": {}}
        for solver in ("gmres_into", "gcrodr_cold"):
            for guess in GUESSES:
                arm = r[f"{solver}_{guess}"]
                c = arm.get("counters", {})
                entry["arms"][f"{solver}_{guess}"] = {
                    "ok": arm.get("ok"), "success": arm.get("success"),
                    "attempts": arm.get("attempts"), "accepted": arm.get("accepted"),
                    "rejected": arm.get("rejected"),
                    "linear_solves": c.get("linear_solves"), "linear_iterations": c.get("linear_iterations"),
                    "linear_matvecs": c.get("linear_matvecs"),
                    "inner_products": c.get("orthogonalization_inner_products"),
                    "vector_updates": c.get("orthogonalization_vector_updates"),
                    "linear_solve_failures": c.get("linear_solve_failures"),
                    "gmres_into_cycles": arm.get("gmres_into_cycles"),
                    "final_error": final_error(arm, reference),
                    "error": arm.get("error"),
                }
        arms = entry["arms"]
        for solver in ("gmres_into", "gcrodr_cold"):
            p, z = arms[f"{solver}_previous"], arms[f"{solver}_zero"]
            for guess in ("previous_step", "previous_step_scaled"):
                a = arms[f"{solver}_{guess}"]
                ok = all(x["linear_matvecs"] for x in (a, p, z))
                entry[f"{solver}_{guess}_over_previous"] = a["linear_matvecs"] / p["linear_matvecs"] if ok else None
                entry[f"{solver}_{guess}_over_zero"] = a["linear_matvecs"] / z["linear_matvecs"] if ok else None
            entry[f"{solver}_previous_over_zero"] = (
                p["linear_matvecs"] / z["linear_matvecs"] if p["linear_matvecs"] and z["linear_matvecs"] else None)
        cases.append(entry)

    def gated(entry):
        arms = entry["arms"]
        s, p = arms["gmres_into_previous_step"], arms["gmres_into_previous"]
        base_ok = base[(entry["case"], entry["rtol"])]["gmres_into_previous"].get("success") is True
        checks = {
            "base_previous_succeeded": base_ok,
            "success": s["success"] is True,
            "matvecs_vs_previous_le_0_85": entry["gmres_into_previous_step_over_previous"] is not None
            and entry["gmres_into_previous_step_over_previous"] <= 0.85,
            "matvecs_vs_zero_le_0_90": entry["gmres_into_previous_step_over_zero"] is not None
            and entry["gmres_into_previous_step_over_zero"] <= 0.90,
            "attempts_within": s["attempts"] is not None and p["attempts"] is not None
            and abs(s["attempts"] - p["attempts"]) <= max(1, 0.02 * p["attempts"]),
            "no_linear_solve_failures": s["linear_solve_failures"] == 0,
            "error_le_2x_previous": s["final_error"] is not None and p["final_error"] is not None
            and s["final_error"] <= 2.0 * p["final_error"],
        }
        return {"case": entry["case"], "rtol": entry["rtol"], "checks": checks, "pass": all(checks.values())}

    gate_cases = [gated(e) for e in cases if e["case"] in GATED_CASES]
    excluded = [g for g in gate_cases if not g["checks"]["base_previous_succeeded"]]
    regression = [{"case": e["case"], "rtol": e["rtol"],
                   "ratio": e["gmres_into_previous_step_over_previous"],
                   "ok": e["gmres_into_previous_step_over_previous"] is not None
                   and e["gmres_into_previous_step_over_previous"] <= 1.02} for e in cases]
    gate = {
        "base_reproduced": all(all(v for k, v in r.items() if k not in ("case", "rtol")) for r in reproduction),
        "fewer_matvecs_on_brusselators": len(gate_cases) == 4 and not excluded and all(g["pass"] for g in gate_cases),
        "no_regression_gt_1_02": all(r["ok"] for r in regression),
    }
    verdict = "PASS" if all(gate.values()) else "FAIL"
    report = {
        "schema": SCHEMA, "inputs": {str(args.base): sha256(args.base), str(args.runs): sha256(args.runs)},
        "reproduction": reproduction, "gated_cases": gate_cases, "excluded_from_item_2": excluded,
        "regression": regression, "cases": cases, "gate": gate, "verdict": verdict,
        "claim_scope": "Counted Krylov work of the opt-in matrix-free U-form research driver; no wall-time claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    for g in gate_cases:
        print(g["case"], g["rtol"], g["pass"], {k: v for k, v in g["checks"].items() if not v})
    for e in cases:
        print(f"{e['case']:26s} {e['rtol']:.0e} step/prev {e['gmres_into_previous_step_over_previous']:.3f} "
              f"step/zero {e['gmres_into_previous_step_over_zero']:.3f} prev/zero {e['gmres_into_previous_over_zero']:.3f}")
    print("gate", gate, "verdict", verdict)


if __name__ == "__main__":
    main()
