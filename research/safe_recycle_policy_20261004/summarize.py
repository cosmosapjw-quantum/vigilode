#!/usr/bin/env python3
"""Gate evaluation of research node safe_recycle_policy_20261004 (RUNS.json against
BASE_LEGACY.json)."""
import json
from pathlib import Path

N = Path(__file__).resolve().parent
base = json.loads((N / "BASE_LEGACY.json").read_text())["rows"]
runs = json.loads((N / "RUNS.json").read_text())["rows"]
L, R, C = ("gcrodr-recycle-legacy-v1", "gcrodr-recycle-refresh-after-update-v1", "gcrodr-cold-v1")


def ops(c):
    return c["linear_matvecs"] + c["diagnostic_matvecs"] + c["recycle_refresh_matvecs"]


g1 = len(base) == len(runs) and all(
    b["case"] == r["case"] and b["rtol"] == r["rtol"] and b["legacy"] == r[L] and r["default"] == r[L]
    for b, r in zip(base, runs))
g2 = g4 = True
rows = []
for r in runs:
    entry = {"case": r["case"], "rtol": r["rtol"]}
    for key in (L, R, C):
        x = r[key]
        c = x["counters"]
        entry[key] = {"success": x["success"], "attempts": x["attempts"], "rejected": x["rejected"],
                      "linear_solve_failures": c["linear_solve_failures"],
                      "shifted_operator_applications": ops(c),
                      "recycle_update_refreshes": c.get("recycle_update_refreshes", 0),
                      "recycle_updates": c["recycle_updates"]}
    if entry[R]["recycle_updates"] > 0 and entry[R]["recycle_update_refreshes"] == 0:
        g2 = False
    if entry[L]["recycle_update_refreshes"] or entry[C]["recycle_update_refreshes"]:
        g2 = False
    lf, ff = entry[L]["linear_solve_failures"], entry[R]["linear_solve_failures"]
    if ff > lf or (lf > 0 and not ff < lf):
        g4 = False
    rows.append(entry)
result = {"schema": "vigilode-safe-recycle-results-v1", "rows": rows,
          "gate": {"g1_legacy_bitwise_base_and_default": g1, "g2_refresh_counted": g2,
                   "g2_g3_contract_tests": "safe_recycle_policy::{refreshed_stage_solve_equals_traced_rev01c_call, cold_stage_solve_leaves_carried_state}",
                   "g4_refresh_fewer_failures": g4}}
result["verdict"] = "PASS" if (g1 and g2 and g4) else "FAIL"
(N / "RESULTS.json").write_text(json.dumps(result, indent=1) + "\n")
print(result["verdict"], result["gate"])
