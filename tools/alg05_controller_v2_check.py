#!/usr/bin/env python3
"""Gate of research node research/alg05_predictive_controller_v2_20261010 (ALG05, the second test of
ALG02's predictive step-size control; see its PREREGISTRATION.md).

Inputs: BASE.json (export_base: arm I on every cell, on the test-only commit's solver source),
RUNS.json (export_runs: arms I, PREDcap and PREDcap2 on every cell) and the NATIVE.json references.
Cells: the dense fast driver on van-der-pol-mu1000, hires, robertson, brusselator-1d-50 with the
ALG02 quarter-decade ladders and the fresh initial-step seeds 3e-6, 3e-5, 3e-3.

Scoring:
- Frontier only. Per seed, an OLS fit of log10(attempts) on log10(endpoint error) over the whole
  ladder, evaluated at E; the ratio arm/I is the median over the 3 seeds of the per-seed ratios.
- In-range E: E must lie inside both arms' measured error ranges for every seed; other E are
  excluded (also at the van der Pol points: an excluded point cannot count toward item 2).
- Cheapest-run ratios (fewest attempts among runs with error <= E) are reported, not gated.

Gate (arm PREDcap2 against I), PASS iff all hold:
1. Identity: I in RUNS reproduces BASE bit for bit.
2. van der Pol gain: frontier ratio <= 0.90 at >= 3 of E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}.
3. No regression: on hires, robertson and brusselator-1d-50 the worst in-range half-decade frontier
   ratio <= 1.06.
4. Rejections: pooled van der Pol rejection fraction of PREDcap2 <= 0.5 x I's.
5. Calibration: per problem and seed, fit log10(err) = a + b log10(rtol) over the ladder for each
   arm. Over the ladder's rtol range, the median over seeds of max(fit_PREDcap2 / fit_I) <= 2.0
   (every problem); and in every PREDcap2 cell err / rtol <= 10.
6. Cap behaviour: unit tests show the two changes (err = 0 after a rejection gives factor 1; a
   sliver landing keeps the rejection flag). The checker verifies that the registered tests exist
   in crates/rodas5p-integrators/tests/alg05_predictive_capped2.rs; their passing is recorded by
   `cargo test` at the recorded commit (see the Results section).

Everything else is FAIL, with every ratio preserved. Counted work only; no wall-time claim. The
output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
import struct
from collections import defaultdict
from pathlib import Path

SCHEMA = "vigilode-alg05-controller-v2-check-v1"
ERROR_FLOOR = 1.0e-10
PROBLEMS = ("van-der-pol-mu1000", "hires", "robertson", "brusselator-1d-50")
NO_REGRESSION_PROBLEMS = ("hires", "robertson", "brusselator-1d-50")
VDP = "van-der-pol-mu1000"
VDP_E = (1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 3.0e-7)
ARMS = ("I", "PREDcap", "PREDcap2")
GATED = "PREDcap2"
REFERENCE = "I"
SEEDS = (3.0e-6, 3.0e-5, 3.0e-3)
CELLS_PER_ARM = 276
HALF_DECADES = tuple(10.0 ** (-j / 2) for j in range(0, 31))
GATE2_R = 0.90
GATE2_COUNT = 3
GATE3_R = 1.06
GATE4_FACTOR = 0.5
GATE5_FIT_RATIO = 2.0
GATE5_CELL = 10.0
UNIT_TEST_FILE = "crates/rodas5p-integrators/tests/alg05_predictive_capped2.rs"
UNIT_TESTS = ("err_zero_after_a_rejection_gives_factor_one", "a_sliver_landing_keeps_the_rejection_flag")


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


def median(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def ols(points):
    """OLS y = a + b x; (b, a) or None."""
    if len(points) < 2:
        return None
    n = len(points)
    mx = sum(p[0] for p in points) / n
    my = sum(p[1] for p in points) / n
    sxx = sum((p[0] - mx) ** 2 for p in points)
    if sxx == 0.0:
        return None
    b = sum((p[0] - mx) * (p[1] - my) for p in points) / sxx
    return b, my - b * mx


# ------------------------------------------------------------------------------------------ scoring
def usable(row, work="attempts"):
    return (row.get("ok") and row.get("success") and row.get("error") is not None
            and row["error"] > 0.0 and math.isfinite(row["error"]) and row[work] > 0)


def fit(rows, work="attempts"):
    """OLS of log10(work) on log10(error) over the whole ladder; (slope, intercept, lo, hi)."""
    pts = [(math.log10(r["error"]), math.log10(r[work])) for r in rows if usable(r, work)]
    f = ols(pts)
    if f is None:
        return None
    errs = [r["error"] for r in rows if usable(r, work)]
    return f[0], f[1], min(errs), max(errs)


def frontier(f, E):
    b, a, lo, hi = f
    return 10.0 ** (a + b * math.log10(E)), not (lo <= E <= hi)


def cheapest(rows, E, work="attempts"):
    ok = [r[work] for r in rows if usable(r, work) and r["error"] <= E]
    return min(ok) if ok else None


def compare(groups, problem, arm, ref, E, seeds=SEEDS):
    """Per-seed frontier (R, gated) and cheapest-run (C, reported) ratios arm/ref at E; medians over
    seeds; extrapolated = E outside either arm's measured error range for some seed."""
    r_ratios, c_ratios, per_seed, extrapolated = [], [], [], False
    for seed in seeds:
        ra, rr = groups[(problem, arm, seed)], groups[(problem, ref, seed)]
        fa, fr = fit(ra), fit(rr)
        if fa is None or fr is None:
            extrapolated = True
            per_seed.append({"seed": seed, "R": None, "C": None})
            continue
        va, xa = frontier(fa, E)
        vr, xr = frontier(fr, E)
        extrapolated |= xa or xr
        ca, cr = cheapest(ra, E), cheapest(rr, E)
        r_ratio = va / vr
        c_ratio = ca / cr if (ca is not None and cr is not None) else None
        r_ratios.append(r_ratio)
        c_ratios.append(c_ratio)
        per_seed.append({"seed": seed, "R": r_ratio, "C": c_ratio, "R_arm": va, "R_ref": vr,
                         "C_arm": ca, "C_ref": cr, "extrapolated": xa or xr})
    cs = [c for c in c_ratios if c is not None]
    return {
        "E": E,
        "R": median(r_ratios) if len(r_ratios) == len(seeds) else None,
        "C": median(cs) if len(cs) == len(seeds) else None,
        "R_range": [min(r_ratios), max(r_ratios)] if r_ratios else None,
        "C_range": [min(cs), max(cs)] if cs else None,
        "extrapolated": extrapolated,
        "per_seed": per_seed,
    }


def in_range_grid(groups, problem, arms, seeds=SEEDS, grid=HALF_DECADES):
    """Half-decade E inside every listed arm's measured error range for every seed."""
    out = []
    for E in grid:
        ok = True
        for arm in arms:
            for seed in seeds:
                f = fit(groups[(problem, arm, seed)])
                if f is None or not (f[2] <= E <= f[3]):
                    ok = False
        if ok:
            out.append(E)
    return out


def table(groups, problem, arm, ref, seeds=SEEDS):
    grid = VDP_E if problem == VDP else in_range_grid(groups, problem, (arm, ref), seeds)
    points = [compare(groups, problem, arm, ref, E, seeds) for E in grid]
    inr = [p for p in points if not p["extrapolated"] and p["R"] is not None]
    return {
        "arm": arm, "reference": ref,
        "points": points,
        "in_range_E": [p["E"] for p in inr],
        "excluded_E": [p["E"] for p in points if p not in inr],
        "gm_R": gm([p["R"] for p in inr]),
        "worst_R": max((p["R"] for p in inr), default=None),
        "worst_R_E": max(inr, key=lambda p: p["R"])["E"] if inr else None,
        "reported_gm_C": gm([p["C"] for p in inr]),
        "reported_worst_C": max((p["C"] for p in inr if p["C"] is not None), default=None),
    }


def rejection(rows):
    att = sum(r["attempts"] for r in rows if r.get("ok"))
    rej = sum(r["rejected"] for r in rows if r.get("ok"))
    return {"attempts": att, "rejected": rej, "fraction": rej / att if att else None}


def calibration_fit(rows):
    """OLS of log10(err) on log10(rtol) over the ladder; (b, a) or None."""
    return ols([(math.log10(r["rtol"]), math.log10(r["error"])) for r in rows if usable(r)])


def calibration(groups, problem, arm, ref, seeds=SEEDS):
    """Item 5 statistic: per seed, max over the ladder's rtol range of fit_arm(rtol)/fit_ref(rtol)
    (log-linear, so the maximum is at an end of the range); median over seeds."""
    per_seed = []
    for seed in seeds:
        ra, rr = groups[(problem, arm, seed)], groups[(problem, ref, seed)]
        fa, fr = calibration_fit(ra), calibration_fit(rr)
        rtols = sorted({r["rtol"] for r in ra} | {r["rtol"] for r in rr})
        if fa is None or fr is None or not rtols:
            per_seed.append({"seed": seed, "max_ratio": None})
            continue
        ends = (rtols[0], rtols[-1])
        ratio = {}
        for x in ends:
            lx = math.log10(x)
            ratio[x] = 10.0 ** ((fa[1] + fa[0] * lx) - (fr[1] + fr[0] * lx))
        # The ratio on the ladder points too (the maximum is at an end of the range).
        on_ladder = max(10.0 ** ((fa[1] + fa[0] * math.log10(x)) - (fr[1] + fr[0] * math.log10(x)))
                        for x in rtols)
        worst = max(ratio.values())
        per_seed.append({"seed": seed, "fit_arm": {"slope": fa[0], "intercept": fa[1]},
                         "fit_ref": {"slope": fr[0], "intercept": fr[1]},
                         "ratio_at_rtol_max": ratio[ends[1]], "ratio_at_rtol_min": ratio[ends[0]],
                         "rtol_range": [ends[0], ends[1]], "max_ratio": worst,
                         "max_ratio_on_ladder_points": on_ladder,
                         "argmax_rtol": max(ends, key=lambda x: ratio[x])})
    values = [p["max_ratio"] for p in per_seed]
    return {"per_seed": per_seed,
            "median_max_ratio": median(values) if all(v is not None for v in values) else None}


def cell_err_over_rtol(rows):
    """(max err/rtol, its cell, count > 10) over the successful rows; failures listed."""
    worst, where, above, failed = None, None, [], []
    for r in rows:
        if not usable(r):
            failed.append({"problem": r["problem"], "rtol": r["rtol"], "seed": r["seed"]})
            continue
        q = r["error"] / r["rtol"]
        if worst is None or q > worst:
            worst, where = q, {"problem": r["problem"], "rtol": r["rtol"], "seed": r["seed"]}
        if q > GATE5_CELL:
            above.append({"problem": r["problem"], "rtol": r["rtol"], "seed": r["seed"], "err_over_rtol": q})
    return {"max": worst, "argmax": where, "cells_above_10": above, "failed_cells": failed}


def unit_tests_present(repo_root: Path):
    path = repo_root / UNIT_TEST_FILE
    if not path.exists():
        return {"file": UNIT_TEST_FILE, "present": {name: False for name in UNIT_TESTS}}
    text = path.read_text()
    present = {name: bool(re.search(r"#\[test\]\s*fn\s+" + name + r"\s*\(", text)) for name in UNIT_TESTS}
    return {"file": UNIT_TEST_FILE, "sha256": sha256(path), "present": present}


def gate_items(groups, dense, gated, ref, seeds=SEEDS):
    """Items 2-5 for `gated` against `ref` (used for the gated arm and, reported, for PREDcap)."""
    vdp = table(groups, VDP, gated, ref, seeds)
    passing = [p["E"] for p in vdp["points"] if not p["extrapolated"] and p["R"] is not None
               and p["R"] <= GATE2_R]
    item2 = {
        "item": "van der Pol gain: frontier ratio <= 0.90 at >= 3 of E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7} "
                "(E outside either arm's measured error range for some seed excluded)",
        "points": [{"E": p["E"], "R": p["R"], "R_range": p["R_range"], "extrapolated": p["extrapolated"],
                    "reported_C": p["C"], "per_seed": p["per_seed"]} for p in vdp["points"]],
        "passing_E": passing,
        "pass": len(passing) >= GATE2_COUNT,
    }
    problems = {}
    for problem in NO_REGRESSION_PROBLEMS:
        t = table(groups, problem, gated, ref, seeds)
        problems[problem] = {
            "in_range_E": t["in_range_E"],
            "points": [{"E": p["E"], "R": p["R"], "R_range": p["R_range"], "reported_C": p["C"],
                        "reported_C_range": p["C_range"]} for p in t["points"]],
            "worst_R": t["worst_R"], "worst_R_E": t["worst_R_E"], "gm_R": t["gm_R"],
            "reported_worst_C": t["reported_worst_C"], "reported_gm_C": t["reported_gm_C"],
            "pass": bool(t["in_range_E"]) and t["worst_R"] is not None and t["worst_R"] <= GATE3_R,
        }
    item3 = {
        "item": "no regression: worst in-range half-decade frontier ratio <= 1.06 on hires, robertson, "
                "brusselator-1d-50",
        "problems": problems,
        "pass": all(v["pass"] for v in problems.values()),
    }
    fi = rejection([r for r in dense if r["problem"] == VDP and r["arm"] == ref])["fraction"]
    fg = rejection([r for r in dense if r["problem"] == VDP and r["arm"] == gated])["fraction"]
    item4 = {
        "item": f"rejections: pooled van der Pol rejection fraction of {gated} <= 0.5 x {ref}'s",
        ref: fi, gated: fg, "ratio": fg / fi if fi else None,
        "pass": fi is not None and fg is not None and fg <= GATE4_FACTOR * fi,
    }
    fits = {p: calibration(groups, p, gated, ref, seeds) for p in PROBLEMS}
    cells = cell_err_over_rtol([r for r in dense if r["arm"] == gated])
    n_cells = sum(1 for r in dense if r["arm"] == gated)
    fit_pass = all(v["median_max_ratio"] is not None and v["median_max_ratio"] <= GATE5_FIT_RATIO
                   for v in fits.values())
    cell_pass = (not cells["cells_above_10"] and not cells["failed_cells"] and n_cells == CELLS_PER_ARM)
    item5 = {
        "item": f"calibration: per problem, median over seeds of max over the ladder's rtol range of "
                f"fit_{gated}/fit_{ref} <= 2.0; and in every {gated} cell err/rtol <= 10",
        "fit_ratio": {p: {"median_max_ratio": v["median_max_ratio"], "pass": v["median_max_ratio"] is not None
                          and v["median_max_ratio"] <= GATE5_FIT_RATIO, "per_seed": v["per_seed"]}
                      for p, v in fits.items()},
        "cells": n_cells,
        "cell_err_over_rtol": cells,
        "fit_pass": fit_pass, "cell_pass": cell_pass,
        "pass": fit_pass and cell_pass,
    }
    return item2, item3, item4, item5


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
    repo_root = Path(__file__).resolve().parent.parent
    base = json.loads(args.base.read_text())
    runs = json.loads(args.runs.read_text())
    native = json.loads(args.native.read_text())["references"]
    dense = runs["rows"]

    # Recompute every endpoint error from the state bits and the NATIVE reference.
    error_mismatches = []
    for row in list(base["rows"]) + list(dense):
        if row.get("ok") and row.get("success"):
            e = endpoint_error(row["final_state"], native[row["problem"]]["final_state"])
            if e != row["error"]:
                error_mismatches.append({"problem": row["problem"], "arm": row["arm"], "rtol": row["rtol"],
                                         "seed": row["seed"], "recorded": row["error"], "recomputed": e})

    key = lambda r: (r["problem"], r["rtol"], r["seed"])  # noqa: E731
    cells = defaultdict(dict)
    for r in dense:
        cells[key(r)][r["arm"]] = r
    complete = (len(cells) == CELLS_PER_ARM and all(set(c) == set(ARMS) for c in cells.values())
                and len(dense) == len(ARMS) * CELLS_PER_ARM)
    failures = [{"problem": r["problem"], "arm": r["arm"], "rtol": r["rtol"], "seed": r["seed"]}
                for r in dense if not r.get("success")]

    # 1. Identity.
    base_rows = {key(r): r for r in base["rows"]}
    run_i = {key(r): r for r in dense if r["arm"] == REFERENCE}
    identity_diffs = [list(k) for k in sorted(set(base_rows) | set(run_i))
                      if base_rows.get(k) != run_i.get(k)]
    item1 = {
        "item": "identity: I in RUNS reproduces BASE bit for bit",
        "base_rows": len(base_rows), "run_rows": len(run_i), "differing_cells": identity_diffs,
        "pass": len(base_rows) == CELLS_PER_ARM and not identity_diffs and set(base_rows) == set(run_i),
    }

    groups = defaultdict(list)
    for r in dense:
        groups[(r["problem"], r["arm"], r["seed"])].append(r)
    for k in groups:
        groups[k].sort(key=lambda r: -r["rtol"])

    item2, item3, item4, item5 = gate_items(groups, dense, GATED, REFERENCE)

    # 6. Cap behaviour (unit tests).
    tests = unit_tests_present(repo_root)
    item6 = {
        "item": "cap behaviour: unit tests show err = 0 after a rejection gives factor 1 and a sliver "
                "landing keeps the rejection flag",
        "unit_tests": tests,
        "evidence": "the tests' passing is recorded by cargo test at the recorded commit (Results section)",
        "pass": all(tests["present"].values()),
    }

    gates = {"1_identity": item1, "2_vdp_gain": item2, "3_no_regression": item3,
             "4_rejections": item4, "5_calibration": item5, "6_cap_behaviour": item6}
    verdict = "PASS" if all(g["pass"] for g in gates.values()) and complete and not error_mismatches else "FAIL"

    # ---------------------------------------------------------------------------------- reported
    ratios = {}
    for arm, ref in (("PREDcap2", "I"), ("PREDcap", "I"), ("PREDcap2", "PREDcap")):
        ratios[f"{arm}/{ref}"] = {p: table(groups, p, arm, ref) for p in PROBLEMS}
    predcap_items = gate_items(groups, dense, "PREDcap", REFERENCE)
    predcap_as_gated = {"2_vdp_gain": predcap_items[0], "3_no_regression": predcap_items[1],
                        "4_rejections": predcap_items[2], "5_calibration": predcap_items[3]}

    rej = {arm: {p: rejection([r for r in dense if r["problem"] == p and r["arm"] == arm]) for p in PROBLEMS}
           for arm in ARMS}
    rej_seed = {arm: {p: {str(s): rejection(groups[(p, arm, s)])["fraction"] for s in SEEDS}
                      for p in PROBLEMS} for arm in ARMS}

    # PREDcap2 against PREDcap, cell by cell.
    diff_fields = ("attempts", "accepted", "rejected", "final_state", "t_last", "rhs_evaluations")
    differing = []
    for k in sorted(cells):
        a, b = cells[k].get("PREDcap"), cells[k].get("PREDcap2")
        if a is None or b is None:
            continue
        if any(a.get(f) != b.get(f) for f in diff_fields):
            differing.append({"problem": k[0], "rtol": k[1], "seed": k[2],
                              "PREDcap": {"attempts": a.get("attempts"), "rejected": a.get("rejected"),
                                          "error": a.get("error")},
                              "PREDcap2": {"attempts": b.get("attempts"), "rejected": b.get("rejected"),
                                           "error": b.get("error")}})
    identical_rows = sum(1 for k in cells if cells[k].get("PREDcap") is not None
                         and {f: v for f, v in cells[k]["PREDcap"].items() if f != "arm"}
                         == {f: v for f, v in cells[k].get("PREDcap2", {}).items() if f != "arm"})
    per_problem_diff = {p: sum(1 for d in differing if d["problem"] == p) for p in PROBLEMS}

    equal_rtol = {}
    for arm in ("PREDcap", "PREDcap2"):
        per = {}
        for problem in PROBLEMS:
            rs = [cells[k][arm]["error"] / cells[k]["I"]["error"] for k in cells
                  if k[0] == problem and cells[k].get(arm, {}).get("success")
                  and cells[k]["I"].get("success") and cells[k]["I"]["error"] > 0]
            per[problem] = {"median": median(rs), "min": min(rs) if rs else None,
                            "max": max(rs) if rs else None, "n": len(rs)}
        equal_rtol[f"{arm}/I"] = per
    err_over_rtol = {arm: {p: {"median": median([r["error"] / r["rtol"] for r in dense
                                                 if r["arm"] == arm and r["problem"] == p and usable(r)]),
                               "max": max((r["error"] / r["rtol"] for r in dense
                                           if r["arm"] == arm and r["problem"] == p and usable(r)), default=None),
                               "cells_above_2": sum(1 for r in dense if r["arm"] == arm and r["problem"] == p
                                                    and usable(r) and r["error"] / r["rtol"] > 2.0),
                               "cells_above_10": sum(1 for r in dense if r["arm"] == arm and r["problem"] == p
                                                     and usable(r) and r["error"] / r["rtol"] > 10.0)}
                           for p in PROBLEMS} for arm in ARMS}
    calibration_fits = {arm: {p: calibration(groups, p, arm, REFERENCE) for p in PROBLEMS}
                        for arm in ("PREDcap", "PREDcap2")}
    ladder_totals = {arm: {p: {str(s): sum(r["attempts"] for r in groups[(p, arm, s)]) for s in SEEDS}
                           for p in PROBLEMS} for arm in ARMS}

    out = {
        "schema": SCHEMA,
        "inputs": {"base": {"path": str(args.base), "sha256": sha256(args.base)},
                   "runs": {"path": str(args.runs), "sha256": sha256(args.runs)},
                   "native": {"path": str(args.native), "sha256": sha256(args.native)}},
        "gated_arm": GATED, "reference_arm": REFERENCE, "seeds": SEEDS,
        "rows": len(dense), "complete": complete, "failed_runs": failures,
        "error_recomputation_mismatches": error_mismatches,
        "gates": gates,
        "verdict": verdict,
        "reported": {
            "cheapest_run_note": "cheapest-run ratios (C) are reported, not gated",
            "ratios": ratios,
            "PREDcap_against_the_items": predcap_as_gated,
            "rejection_fractions_pooled": {arm: {p: v["fraction"] for p, v in d.items()} for arm, d in rej.items()},
            "rejection_counts": rej,
            "rejection_fractions_per_seed": rej_seed,
            "PREDcap2_vs_PREDcap": {"cells": len(cells), "identical_rows": identical_rows,
                                    "differing_cells_per_problem": per_problem_diff,
                                    "differing_cells": differing},
            "equal_rtol_error_ratios": equal_rtol,
            "error_over_rtol": err_over_rtol,
            "calibration_fits": calibration_fits,
            "ladder_attempt_totals": ladder_totals,
        },
    }
    args.output.write_text(json.dumps(out, indent=2) + "\n")
    print(f"verdict {verdict}")
    for name, g in gates.items():
        print(f"  {name}: {'pass' if g['pass'] else 'FAIL'}")


if __name__ == "__main__":
    main()
