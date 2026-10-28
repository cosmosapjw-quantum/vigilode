#!/usr/bin/env python3
"""Gate of research node research/alg02_predictive_controller_20261008 (ALG02, algorithmic-directions
Tier 1 node N2; see its PREREGISTRATION.md).

Inputs: BASE.json (export_base: arm I on every dense cell, on the registration commit's solver
source), RUNS.json (export_runs: arms I, I725, PRED, PREDcap on every dense cell, and the reported
matrix-free cells with I and PREDcap) and the NATIVE.json references.

Scoring (every rule per seed; ratios are the median over seeds):
- R, frontier: OLS of log10(attempts) on log10(endpoint error) over the whole ladder, at E.
- C, cheapest run: the fewest attempts among runs with error <= E.
- van der Pol: E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}. Other problems: half-decade E inside both arms'
  measured error ranges. An E outside either arm's range for any seed is flagged as extrapolated
  and excluded (also at the van der Pol points: an extrapolated point cannot count toward item 2).

Gate (arm PREDcap against I), PASS iff all hold:
1. Identity: I in RUNS reproduces BASE bit for bit.
2. van der Pol gain: R ratio <= 0.90 at >= 3 of the 5 van der Pol E.
3. No regression: on hires, robertson and brusselator-1d-50 the worst in-range R ratio <= 1.06 and
   the worst in-range C ratio <= 1.10.
4. Rejections: pooled van der Pol rejection fraction (all rungs and seeds) <= 0.5 x I's.
5. Calibration: in every dense cell and seed err(PREDcap)/rtol <= max(2.0, 1.5 err(I)/rtol).

Reported, not gated: PRED, I725, the factorial reading (PREDcap beats I725 by >= 3 % in
geometric-mean R on hires or brusselator-1d-50), the matrix-free cells, per-cell equal-rtol error
ratios. Counted work only; no wall-time claim. The output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
import struct
from collections import defaultdict
from pathlib import Path

SCHEMA = "vigilode-alg02-controller-check-v1"
ERROR_FLOOR = 1.0e-10
DENSE_PROBLEMS = ("van-der-pol-mu1000", "hires", "robertson", "brusselator-1d-50")
NO_REGRESSION_PROBLEMS = ("hires", "robertson", "brusselator-1d-50")
FACTORIAL_PROBLEMS = ("hires", "brusselator-1d-50")
VDP = "van-der-pol-mu1000"
VDP_E = (1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 3.0e-7)
ARMS = ("I", "I725", "PRED", "PREDcap")
MF_ARMS = ("I", "PREDcap")
SEEDS = (1.0e-6, 1.0e-4, 1.0e-2)
HALF_DECADES = tuple(10.0 ** (-j / 2) for j in range(0, 31))
GATE2_R = 0.90
GATE2_COUNT = 3
GATE3_R = 1.06
GATE3_C = 1.10
GATE4_FACTOR = 0.5
GATE5_FLOOR = 2.0
GATE5_FACTOR = 1.5
FACTORIAL_MARGIN = 0.97


def value(hex_bits: str) -> float:
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def endpoint_error(state_hex: list[str], reference: list[float]) -> float:
    y = [value(x) for x in state_hex]
    assert len(y) == len(reference)
    return max(abs(a - r) / max(abs(r), ERROR_FLOOR) for a, r in zip(y, reference))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def gm(values):
    values = [v for v in values if v is not None]
    if not values:
        return None
    return math.exp(sum(math.log(v) for v in values) / len(values))


# ------------------------------------------------------------------------------------------ scoring
def usable(row, work):
    return (row.get("ok") and row.get("success") and row.get("error") is not None
            and row["error"] > 0.0 and math.isfinite(row["error"]) and row[work] > 0)


def fit(rows, work):
    """OLS of log10(work) on log10(error) over the whole ladder; (slope, intercept, lo, hi)."""
    pts = [(math.log10(r["error"]), math.log10(r[work])) for r in rows if usable(r, work)]
    if len(pts) < 2:
        return None
    n = len(pts)
    mx = sum(p[0] for p in pts) / n
    my = sum(p[1] for p in pts) / n
    sxx = sum((p[0] - mx) ** 2 for p in pts)
    if sxx == 0.0:
        return None
    b = sum((p[0] - mx) * (p[1] - my) for p in pts) / sxx
    a = my - b * mx
    errs = [r["error"] for r in rows if usable(r, work)]
    return b, a, min(errs), max(errs)


def frontier(f, E):
    b, a, lo, hi = f
    return 10.0 ** (a + b * math.log10(E)), not (lo <= E <= hi)


def cheapest(rows, work, E):
    ok = [r[work] for r in rows if usable(r, work) and r["error"] <= E]
    return min(ok) if ok else None


def median(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def compare(groups, problem, arm, ref, E, work="attempts", seeds=SEEDS):
    """Per-seed R and C ratios arm/ref at E; median over seeds; extrapolation flag."""
    r_ratios, c_ratios, per_seed, extrapolated = [], [], [], False
    for seed in seeds:
        ra, rr = groups[(problem, arm, seed)], groups[(problem, ref, seed)]
        fa, fr = fit(ra, work), fit(rr, work)
        if fa is None or fr is None:
            extrapolated = True
            per_seed.append({"seed": seed, "R": None, "C": None})
            continue
        va, xa = frontier(fa, E)
        vr, xr = frontier(fr, E)
        extrapolated |= xa or xr
        ca, cr = cheapest(ra, work, E), cheapest(rr, work, E)
        rr_ratio = va / vr
        cc_ratio = ca / cr if (ca is not None and cr is not None) else None
        r_ratios.append(rr_ratio)
        c_ratios.append(cc_ratio)
        per_seed.append({"seed": seed, "R": rr_ratio, "C": cc_ratio,
                         "R_arm": va, "R_ref": vr, "C_arm": ca, "C_ref": cr,
                         "extrapolated": xa or xr})
    return {
        "E": E,
        "R": median(r_ratios),
        "C": median(c_ratios),
        "R_range": [min(r_ratios), max(r_ratios)] if r_ratios else None,
        "C_range": ([min(c for c in c_ratios if c is not None), max(c for c in c_ratios if c is not None)]
                    if any(c is not None for c in c_ratios) else None),
        "extrapolated": extrapolated,
        "per_seed": per_seed,
    }


def in_range_grid(groups, problem, arms, work="attempts", seeds=SEEDS, grid=HALF_DECADES):
    """Half-decade E inside every listed arm's measured error range for every seed."""
    out = []
    for E in grid:
        ok = True
        for arm in arms:
            for seed in seeds:
                f = fit(groups[(problem, arm, seed)], work)
                if f is None or not (f[2] <= E <= f[3]):
                    ok = False
        if ok:
            out.append(E)
    return out


def table(groups, problem, arm, ref, work="attempts", seeds=SEEDS, grid=None):
    if grid is None:
        grid = VDP_E if problem == VDP else in_range_grid(groups, problem, (arm, ref), work, seeds)
    points = [compare(groups, problem, arm, ref, E, work, seeds) for E in grid]
    inr = [p for p in points if not p["extrapolated"]]
    return {
        "arm": arm, "reference": ref, "work": work,
        "points": points,
        "gm_R": gm([p["R"] for p in inr]),
        "gm_C": gm([p["C"] for p in inr]),
        "worst_R": max((p["R"] for p in inr), default=None),
        "worst_C": max((p["C"] for p in inr if p["C"] is not None), default=None),
        "in_range_E": [p["E"] for p in inr],
    }


def rejection(rows):
    att = sum(r["attempts"] for r in rows if r.get("ok"))
    rej = sum(r["rejected"] for r in rows if r.get("ok"))
    return {"attempts": att, "rejected": rej, "fraction": rej / att if att else None}


# --------------------------------------------------------------------------------------------- main
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--native", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = json.loads(args.base.read_text())
    runs = json.loads(args.runs.read_text())
    native = json.loads(args.native.read_text())["references"]

    # Recompute every dense endpoint error from the state bits and the NATIVE reference.
    error_mismatches = []
    dense = runs["dense_rows"]
    for row in list(base["rows"]) + list(dense):
        if row.get("ok") and row.get("success"):
            e = endpoint_error(row["final_state"], native[row["problem"]]["final_state"])
            if e != row["error"]:
                error_mismatches.append({"problem": row["problem"], "arm": row["arm"], "rtol": row["rtol"],
                                         "seed": row["seed"], "recorded": row["error"], "recomputed": e})

    # 1. Identity.
    key = lambda r: (r["problem"], r["rtol"], r["seed"])  # noqa: E731
    base_rows = {key(r): r for r in base["rows"]}
    run_i = {key(r): r for r in dense if r["arm"] == "I"}
    identity_diffs = [list(k) for k in sorted(set(base_rows) | set(run_i))
                      if base_rows.get(k) != run_i.get(k)]
    gate1 = {
        "item": "identity: I in RUNS reproduces BASE bit for bit",
        "base_rows": len(base_rows), "run_rows": len(run_i),
        "differing_cells": identity_diffs,
        "pass": len(base_rows) == 276 and not identity_diffs and set(base_rows) == set(run_i),
    }

    groups = defaultdict(list)
    for r in dense:
        groups[(r["problem"], r["arm"], r["seed"])].append(r)
    for k in groups:
        groups[k].sort(key=lambda r: -r["rtol"])
    cells = defaultdict(dict)
    for r in dense:
        cells[key(r)][r["arm"]] = r
    expected_cells = len(base_rows)
    complete = all(set(cells[k]) == set(ARMS) for k in cells) and len(cells) == expected_cells

    # 2. van der Pol gain.
    vdp = table(groups, VDP, "PREDcap", "I")
    passing = [p["E"] for p in vdp["points"] if not p["extrapolated"] and p["R"] is not None
               and p["R"] <= GATE2_R]
    gate2 = {
        "item": "van der Pol gain: R ratio <= 0.90 at >= 3 of the 5 E (extrapolated E excluded)",
        "points": [{"E": p["E"], "R": p["R"], "R_range": p["R_range"], "C": p["C"],
                    "extrapolated": p["extrapolated"]} for p in vdp["points"]],
        "passing_E": passing,
        "pass": len(passing) >= GATE2_COUNT,
    }

    # 3. No regression.
    gate3_problems = {}
    for problem in NO_REGRESSION_PROBLEMS:
        t = table(groups, problem, "PREDcap", "I")
        gate3_problems[problem] = {
            "in_range_E": t["in_range_E"],
            "points": [{"E": p["E"], "R": p["R"], "C": p["C"], "R_range": p["R_range"],
                        "C_range": p["C_range"]} for p in t["points"]],
            "worst_R": t["worst_R"], "worst_C": t["worst_C"], "gm_R": t["gm_R"], "gm_C": t["gm_C"],
            "pass": (bool(t["in_range_E"]) and t["worst_R"] is not None and t["worst_R"] <= GATE3_R
                     and t["worst_C"] is not None and t["worst_C"] <= GATE3_C),
        }
    gate3 = {
        "item": "no regression: worst in-range R <= 1.06 and worst in-range C <= 1.10 "
                "on hires, robertson, brusselator-1d-50",
        "problems": gate3_problems,
        "pass": all(v["pass"] for v in gate3_problems.values()),
    }

    # 4. Rejections.
    rej = {arm: {p: rejection([r for r in dense if r["problem"] == p and r["arm"] == arm])
                 for p in DENSE_PROBLEMS} for arm in ARMS}
    rej_seed = {arm: {p: {str(s): rejection(groups[(p, arm, s)])["fraction"] for s in SEEDS}
                      for p in DENSE_PROBLEMS} for arm in ARMS}
    fi, fp = rej["I"][VDP]["fraction"], rej["PREDcap"][VDP]["fraction"]
    gate4 = {
        "item": "rejections: pooled van der Pol rejection fraction of PREDcap <= 0.5 x I's",
        "I": fi, "PREDcap": fp, "ratio": fp / fi if fi else None,
        "pass": fi is not None and fp is not None and fp <= GATE4_FACTOR * fi,
    }

    # 5. Calibration.
    calibration, violations = [], []
    for k in sorted(cells):
        c = cells[k]
        i, p = c.get("I"), c.get("PREDcap")
        rtol = k[1]
        ok = bool(i and p and i.get("success") and p.get("success"))
        entry = {"problem": k[0], "rtol": rtol, "seed": k[2]}
        if ok:
            ei, ep = i["error"], p["error"]
            bound = max(GATE5_FLOOR, GATE5_FACTOR * ei / rtol)
            entry.update({"err_I_over_rtol": ei / rtol, "err_PREDcap_over_rtol": ep / rtol,
                          "bound": bound, "ratio_PREDcap_I": ep / ei if ei > 0 else None,
                          "pass": ep / rtol <= bound})
        else:
            entry.update({"pass": False, "reason": "missing or failed run"})
        calibration.append(entry)
        if not entry["pass"]:
            violations.append(entry)
    gate5 = {
        "item": "calibration: err(PREDcap)/rtol <= max(2.0, 1.5 err(I)/rtol) in every dense cell and seed",
        "cells": len(calibration), "violations": violations,
        "pass": complete and not violations and len(calibration) == expected_cells,
    }

    gates = {"1_identity": gate1, "2_vdp_gain": gate2, "3_no_regression": gate3,
             "4_rejections": gate4, "5_calibration": gate5}
    verdict = "PASS" if all(g["pass"] for g in gates.values()) and complete else "FAIL"

    # ---------------------------------------------------------------------------------- reported
    ratios = {}
    for arm, ref in (("PREDcap", "I"), ("PRED", "I"), ("I725", "I"), ("PREDcap", "I725"),
                     ("PRED", "I725"), ("PREDcap", "PRED")):
        ratios[f"{arm}/{ref}"] = {p: table(groups, p, arm, ref) for p in DENSE_PROBLEMS}
    factorial = {}
    for problem in FACTORIAL_PROBLEMS:
        t = ratios["PREDcap/I725"][problem]
        factorial[problem] = {"gm_R": t["gm_R"], "gm_C": t["gm_C"], "in_range_E": t["in_range_E"],
                              "beats_by_3pct": t["gm_R"] is not None and t["gm_R"] <= FACTORIAL_MARGIN}
    factorial_reading = {
        "question": "does PREDcap beat I725 by >= 3 % in geometric-mean R on hires or brusselator-1d-50?",
        "problems": factorial,
        "answer": any(v["beats_by_3pct"] for v in factorial.values()),
    }
    equal_rtol = {}
    for arm in ("PRED", "PREDcap", "I725"):
        per = {}
        for problem in DENSE_PROBLEMS:
            rs = [cells[k][arm]["error"] / cells[k]["I"]["error"] for k in cells
                  if k[0] == problem and cells[k].get(arm, {}).get("success")
                  and cells[k]["I"].get("success") and cells[k]["I"]["error"] > 0]
            per[problem] = {"median": median(rs), "min": min(rs) if rs else None,
                            "max": max(rs) if rs else None, "n": len(rs)}
        equal_rtol[f"{arm}/I"] = per
    err_over_rtol = {arm: {p: median([r["error"] / r["rtol"] for r in dense
                                      if r["arm"] == arm and r["problem"] == p and r.get("success")])
                           for p in DENSE_PROBLEMS} for arm in ARMS}
    ladder_totals = {arm: {p: {str(s): sum(r["attempts"] for r in groups[(p, arm, s)]) for s in SEEDS}
                           for p in DENSE_PROBLEMS} for arm in ARMS}
    failures = [{"problem": r["problem"], "arm": r["arm"], "rtol": r["rtol"], "seed": r["seed"]}
                for r in dense if not r.get("success")]

    # Matrix-free reported cells (I and PREDcap), work in JVPs and attempts.
    mf = {}
    mf_rows = runs.get("mf_rows", [])
    mf_groups = defaultdict(list)
    for r in mf_rows:
        mf_groups[(r["problem"], r["arm"], r["seed"])].append(r)
    for k in mf_groups:
        mf_groups[k].sort(key=lambda r: -r["rtol"])
    for problem in sorted({r["problem"] for r in mf_rows}):
        entry = {}
        for work in ("jvp_vectors", "attempts"):
            entry[work] = table(mf_groups, problem, "PREDcap", "I", work=work)
        entry["rejections"] = {arm: rejection([r for r in mf_rows if r["problem"] == problem
                                               and r["arm"] == arm]) for arm in MF_ARMS}
        entry["linear_solve_failures"] = {arm: sum(r.get("linear_solve_failures", 0) for r in mf_rows
                                                   if r["problem"] == problem and r["arm"] == arm)
                                          for arm in MF_ARMS}
        entry["jvp_totals"] = {arm: sum(r.get("jvp_vectors", 0) for r in mf_rows
                                        if r["problem"] == problem and r["arm"] == arm) for arm in MF_ARMS}
        entry["failed_runs"] = [{"arm": r["arm"], "rtol": r["rtol"], "seed": r["seed"]} for r in mf_rows
                                if r["problem"] == problem and not r.get("success")]
        pairs = defaultdict(dict)
        for r in mf_rows:
            if r["problem"] == problem:
                pairs[(r["rtol"], r["seed"])][r["arm"]] = r
        eq = [p["PREDcap"]["error"] / p["I"]["error"] for p in pairs.values()
              if p.get("PREDcap", {}).get("success") and p.get("I", {}).get("success") and p["I"]["error"] > 0]
        entry["equal_rtol_error_ratio_PREDcap_I"] = {"median": median(eq), "min": min(eq) if eq else None,
                                                     "max": max(eq) if eq else None}
        mf[problem] = entry

    out = {
        "schema": SCHEMA,
        "inputs": {"base": {"path": str(args.base), "sha256": sha256(args.base)},
                   "runs": {"path": str(args.runs), "sha256": sha256(args.runs)},
                   "native": {"path": str(args.native), "sha256": sha256(args.native)}},
        "dense_rows": len(dense), "complete": complete, "failed_dense_runs": failures,
        "error_recomputation_mismatches": error_mismatches,
        "gates": gates,
        "verdict": verdict,
        "reported": {
            "rejection_fractions_pooled": {arm: {p: v["fraction"] for p, v in d.items()}
                                           for arm, d in rej.items()},
            "rejection_counts": rej,
            "rejection_fractions_per_seed": rej_seed,
            "ratios": ratios,
            "factorial_reading": factorial_reading,
            "equal_rtol_error_ratios": equal_rtol,
            "median_error_over_rtol": err_over_rtol,
            "ladder_attempt_totals": ladder_totals,
            "calibration_table": calibration,
            "matrix_free": mf,
            "matrix_free_references": runs.get("mf_references"),
        },
    }
    args.output.write_text(json.dumps(out, indent=2) + "\n")
    print(f"verdict {verdict}")
    for name, g in gates.items():
        print(f"  {name}: {'pass' if g['pass'] else 'FAIL'}")


if __name__ == "__main__":
    main()
