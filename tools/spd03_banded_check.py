#!/usr/bin/env python3
"""Gate of research node research/spd03_banded_arm_instructions_20261005 (speed node SPD03).

G1 identity: both banded arms equal v2 bitwise on the 14 CLI points, and the slices kernel equals
   the indexed kernel bitwise (outputs and BandedWork) on the six INT-03 cases.
G2 band fill: the native band fill equals the dense fill on every parity state (cargo test flag).
G3 instructions at n = 400: banded / v2 <= 0.40 on brusselator-1d-200.
G4 counted operations: BandedWork operations per attempt between n = 100 and n = 400 grow with a
   log-log slope in [0.95, 1.10].
G5 legacy reproduction: rodas5p-fast reproduces the base export on both Brusselators (+-2 % Ir).
Secondary (reported, not deciding): n = 100 ratio, n = 1000 point, Ir per counted operation,
slices / banded <= 0.95 at n = 400. Kill: banded / v2 > 0.55 at n = 400.
Counted instructions and counted banded operations only; no wall-time claim.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def entries(profile):
    return {(e["arm"], e["problem"]): e for e in profile["entries"] if e["success"]}


def ops_per_attempt(entry):
    w = entry.get("banded_work") or {}
    if not w:
        return None
    return (w["factor_operations"] + w["solve_operations"]) / entry["attempts"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--n1000", required=True, type=Path)
    parser.add_argument("--identity", required=True, type=Path)
    parser.add_argument("--fill-passed", required=True, choices=["true", "false"])
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = entries(json.loads(args.base.read_text()))
    profile = json.loads(args.profile.read_text())
    new = entries(profile)
    big = entries(json.loads(args.n1000.read_text()))
    identity = json.loads(args.identity.read_text())

    cli_rows = identity["cli_rows"]
    int03_rows = identity["int03_rows"]
    g1 = (len(cli_rows) == 14 and all(r["banded_identical"] and r["slices_identical"] for r in cli_rows)
          and len(int03_rows) == 6 and all(r["slices_equal_indexed"] for r in int03_rows))
    g2 = args.fill_passed == "true"
    table = {}
    for problem in ("brusselator-1d-50", "brusselator-1d-200"):
        v2 = new[("rodas5p-fast", problem)]
        bd = new[("rodas5p-fast-banded", problem)]
        sl = new[("rodas5p-fast-banded-slices", problem)]
        table[problem] = {
            "attempts": v2["attempts"],
            "v2_ir_per_attempt": v2["ir_per_attempt"],
            "banded_ir_per_attempt": bd["ir_per_attempt"],
            "slices_ir_per_attempt": sl["ir_per_attempt"],
            "banded_over_v2": bd["ir_per_attempt"] / v2["ir_per_attempt"],
            "slices_over_banded": sl["ir_per_attempt"] / bd["ir_per_attempt"],
            "banded_ops_per_attempt": ops_per_attempt(bd),
            "slices_ops_per_attempt": ops_per_attempt(sl),
            "banded_ir_per_op": bd["ir_per_attempt"] / ops_per_attempt(bd) if ops_per_attempt(bd) else None,
            "slices_ir_per_op": sl["ir_per_attempt"] / ops_per_attempt(sl) if ops_per_attempt(sl) else None,
            "same_work_banded": bd["attempts"] == v2["attempts"] and bd["counters"] == v2["counters"]
            and bd["final_state_sha256"] == v2["final_state_sha256"],
            "same_work_slices": sl["attempts"] == v2["attempts"] and sl["counters"] == v2["counters"]
            and sl["final_state_sha256"] == v2["final_state_sha256"],
            "deterministic": v2["callgrind_deterministic"] and bd["callgrind_deterministic"] and sl["callgrind_deterministic"],
        }
    n1000 = {}
    for arm in ("rodas5p-fast-banded", "rodas5p-fast-banded-slices"):
        e = big.get((arm, "brusselator-1d-500"))
        if e:
            n1000[arm] = {"attempts": e["attempts"], "ir_per_attempt": e["ir_per_attempt"],
                          "ops_per_attempt": ops_per_attempt(e), "deterministic": e["callgrind_deterministic"]}
    g3 = table["brusselator-1d-200"]["banded_over_v2"] <= 0.40
    o100 = table["brusselator-1d-50"]["banded_ops_per_attempt"]
    o400 = table["brusselator-1d-200"]["banded_ops_per_attempt"]
    slope = math.log(o400 / o100) / math.log(400.0 / 100.0) if o100 and o400 else None
    g4 = slope is not None and 0.95 <= slope <= 1.10
    g5 = True
    g5_rows = []
    for problem in ("brusselator-1d-50", "brusselator-1d-200"):
        b = base[("rodas5p-fast", problem)]
        n = new[("rodas5p-fast", problem)]
        same = (n["attempts"] == b["attempts"] and n["counters"] == b["counters"]
                and n["final_state_sha256"] == b["final_state_sha256"])
        ratio = n["ir_per_attempt"] / b["ir_per_attempt"]
        ok = same and abs(ratio - 1.0) <= 0.02
        g5 &= ok
        g5_rows.append({"problem": problem, "same_work": same, "ratio": ratio, "ok": ok})
    secondary = {"n100_banded_over_v2": table["brusselator-1d-50"]["banded_over_v2"],
                 "slices_over_banded_n400": table["brusselator-1d-200"]["slices_over_banded"],
                 "slices_secondary_pass": table["brusselator-1d-200"]["slices_over_banded"] <= 0.95,
                 "n1000": n1000}
    if n1000.get("rodas5p-fast-banded") and o400:
        secondary["slope_400_to_1000"] = math.log(n1000["rodas5p-fast-banded"]["ops_per_attempt"] / o400) / math.log(1000.0 / 400.0)
    kill = table["brusselator-1d-200"]["banded_over_v2"] > 0.55 or not g1 or not g2
    result = {"schema": "vigilode-spd03-check-v1",
              "gate": {"g1_identity": g1, "g2_band_fill": g2, "g3_n400_instructions": g3, "g4_operation_slope": g4,
                       "g5_legacy_reproduction": g5, "kill": kill},
              "table": table, "operation_slope_100_to_400": slope, "g5_rows": g5_rows, "secondary": secondary,
              "cli_rows": cli_rows, "int03_rows": int03_rows,
              "binary_sha256": profile["binary_sha256"], "valgrind": profile["valgrind"]}
    result["verdict"] = "PASS" if all([g1, g2, g3, g4, g5]) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result["gate"]), result["verdict"])
    for p, r in table.items():
        print(f"{p:20s} v2 {r['v2_ir_per_attempt']:12.1f} banded {r['banded_ir_per_attempt']:12.1f} ({r['banded_over_v2']:.3f}x) "
              f"slices {r['slices_ir_per_attempt']:12.1f} ({r['slices_over_banded']:.3f}x of banded) Ir/op {r['banded_ir_per_op']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
