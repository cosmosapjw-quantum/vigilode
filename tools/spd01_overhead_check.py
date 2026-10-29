#!/usr/bin/env python3
"""Gate of research node research/spd01_fast_driver_overhead_20261005 (speed node SPD01).

G1 identity: every option set equals the legacy driver bitwise on every identity row (dense
   and small drivers) and on the six INT-03 banded grid cases.
G2 legacy reproduction: the legacy arms of the new binary reproduce the base export (attempts,
   accepted/rejected, counters, final-state hash) and their Ir per attempt within +-2 %.
G3 prevalidated controller: the -val arms hold exactly one AdaptiveStepConfig::validate call per
   integration (legacy: attempts + 1) and Ir per attempt falls by 120..200 on each small problem
   in both drivers.
G4 fused landing: the -land arms hold one land_capped and two step_to calls per attempt (legacy:
   three and six) and Ir per attempt falls by >= 500 on each small problem in both drivers.
G5 property test: no class A or B discrepancy with h >= 2^16 eps max(|t|, |tf|); class C counted.
Counted instructions only; no wall-time claim.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

SMALL = ["van-der-pol-mu1000", "robertson", "hires"]


def entries(profile):
    return {(e["arm"], e["problem"]): e for e in profile["entries"] if e["success"]}


def calls_of(entry, name):
    """Calls of `name` in the per-integration difference (an integer)."""
    for f, v in entry["calls"]["watched_callees"].items():
        if f.endswith(name):
            return int(round(v["calls"]))
    return 0


def per_attempt(entry, name):
    return calls_of(entry, name) / entry["attempts"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--dense", required=True, type=Path, nargs="+")
    parser.add_argument("--small", required=True, type=Path)
    parser.add_argument("--identity", required=True, type=Path)
    parser.add_argument("--banded", required=True, type=Path)
    parser.add_argument("--property", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = entries(json.loads(args.base.read_text()))
    denses = [json.loads(d.read_text()) for d in args.dense]
    dense = denses[0]
    small = json.loads(args.small.read_text())
    assert all(d["binary_sha256"] == dense["binary_sha256"] for d in denses + [small]), "one binary"
    new = {}
    for d in denses + [small]:
        new |= entries(d)
    identity = json.loads(args.identity.read_text())
    banded = json.loads(args.banded.read_text())
    prop = json.loads(args.property.read_text())

    # G1.
    g1_rows = [{"problem": r["problem"], "rtol": r["rtol"], "dense_identical": r["dense_identical"],
                "small_identical": r["small_identical"]} for r in identity["rows"]]
    g1_banded = [{"case": r["case"], **{k: r[k]["identical"] for k in ("val", "land", "ovh")},
                  "dense_ovh_identical": r.get("dense_ovh_identical")} for r in banded["rows"]]
    g1 = all(r["dense_identical"] and r["small_identical"] for r in g1_rows) and all(
        r["val"] and r["land"] and r["ovh"] and r["dense_ovh_identical"] is not False for r in g1_banded)

    # G2.
    g2_rows = []
    g2 = True
    for (arm, problem), b in base.items():
        n = new.get((arm, problem))
        if n is None:
            g2 = False
            g2_rows.append({"arm": arm, "problem": problem, "missing": True})
            continue
        same = (n["attempts"] == b["attempts"] and n["accepted_steps"] == b["accepted_steps"]
                and n["rejected_steps"] == b["rejected_steps"] and n["counters"] == b["counters"]
                and n["final_state_sha256"] == b["final_state_sha256"])
        ratio = n["ir_per_attempt"] / b["ir_per_attempt"]
        ok = same and abs(ratio - 1.0) <= 0.02 and n["callgrind_deterministic"]
        g2 &= ok
        g2_rows.append({"arm": arm, "problem": problem, "same_work": same, "base_ir_per_attempt": b["ir_per_attempt"],
                        "new_ir_per_attempt": n["ir_per_attempt"], "ratio": ratio,
                        "deterministic": n["callgrind_deterministic"], "ok": ok})

    # G3 and G4.
    g3_rows, g4_rows = [], []
    g3 = g4 = True
    ovh_rows = []
    for driver, legacy_arm in (("dense", "rodas5p-fast"), ("small", "rodas5p-fast-small")):
        for problem in SMALL:
            leg = new[(legacy_arm, problem)]
            val = new[(legacy_arm + "-val", problem)]
            land = new[(legacy_arm + "-land", problem)]
            ovh = new[(legacy_arm + "-ovh", problem)]
            att = leg["attempts"]
            validate_calls = {a: calls_of(e, "AdaptiveStepConfig::validate") for a, e in (("legacy", leg), ("val", val))}
            drop_val = leg["ir_per_attempt"] - val["ir_per_attempt"]
            ok3 = validate_calls["val"] == 1 and validate_calls["legacy"] == att + 1 and 120.0 <= drop_val <= 200.0
            g3 &= ok3
            g3_rows.append({"driver": driver, "problem": problem, "attempts": att, "validate_calls": validate_calls,
                            "legacy_ir_per_attempt": leg["ir_per_attempt"], "val_ir_per_attempt": val["ir_per_attempt"],
                            "reduction_per_attempt": drop_val, "ok": ok3})
            # Per attempt: the driver's one entry call of step_to (the initial
            # step against the span) is per integration, not per attempt.
            lc = {a: calls_of(e, "output::land_capped") for a, e in (("legacy", leg), ("land", land))}
            st = {a: calls_of(e, "output::step_to") - 1 for a, e in (("legacy", leg), ("land", land))}
            drop_land = leg["ir_per_attempt"] - land["ir_per_attempt"]
            ok4 = (lc["land"] == att and st["land"] == 2 * att and lc["legacy"] == 3 * att
                   and st["legacy"] == 6 * att and drop_land >= 500.0)
            g4 &= ok4
            g4_rows.append({"driver": driver, "problem": problem, "attempts": att, "land_capped_calls": lc,
                            "step_to_calls_minus_entry": st, "legacy_ir_per_attempt": leg["ir_per_attempt"],
                            "land_ir_per_attempt": land["ir_per_attempt"], "reduction_per_attempt": drop_land, "ok": ok4})
            ovh_rows.append({"driver": driver, "problem": problem, "legacy": leg["ir_per_attempt"],
                             "val": val["ir_per_attempt"], "land": land["ir_per_attempt"], "ovh": ovh["ir_per_attempt"],
                             "ovh_over_legacy": ovh["ir_per_attempt"] / leg["ir_per_attempt"],
                             "additive_check": (leg["ir_per_attempt"] - ovh["ir_per_attempt"]) - (drop_val + drop_land)})
    hairer = {"van-der-pol-mu1000": 2947.0, "hires": 9157.0}
    for r in ovh_rows:
        if r["problem"] in hairer:
            r["ovh_over_hairer_rodas_L0030_cross_node"] = r["ovh"] / hairer[r["problem"]]
    n400 = {a: new[(a, "brusselator-1d-200")]["ir_per_attempt"] for a in
            ("rodas5p-fast", "rodas5p-fast-val", "rodas5p-fast-land", "rodas5p-fast-ovh")
            if (a, "brusselator-1d-200") in new}

    # G5.
    g5 = prop["discrepancy_a"] == 0 and prop["discrepancy_b"] == 0
    kill = (not g1) or any(r["driver"] == "small" and r["problem"] == "van-der-pol-mu1000" and r["reduction_per_attempt"] < 400
                           for r in g4_rows)
    result = {"schema": "vigilode-spd01-check-v1",
              "gate": {"g1_identity": g1, "g2_legacy_reproduction": g2, "g3_prevalidated": g3, "g4_fused_landing": g4,
                       "g5_property": g5, "kill": kill},
              "g1_rows": g1_rows, "g1_banded": g1_banded, "g2_rows": g2_rows, "g3_rows": g3_rows, "g4_rows": g4_rows,
              "reported_ovh": ovh_rows, "reported_n400": n400, "property": prop,
              "binary_sha256": dense["binary_sha256"], "valgrind": dense["valgrind"]}
    result["verdict"] = "PASS" if (g1 and g2 and g3 and g4 and g5) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result["gate"]), result["verdict"])
    for r in ovh_rows:
        print(f"{r['driver']:5s} {r['problem']:20s} legacy {r['legacy']:9.1f} val {r['val']:9.1f} land {r['land']:9.1f} ovh {r['ovh']:9.1f} ({r['ovh_over_legacy']:.3f}x)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
