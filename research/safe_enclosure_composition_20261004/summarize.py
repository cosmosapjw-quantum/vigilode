#!/usr/bin/env python3
"""Summary of research node safe_enclosure_composition_20261004: probe check, consumer
checks and the change of the REV-02 and R-NEXT-05 bounds against the published runs."""
import json
import struct
from pathlib import Path

N = Path(__file__).resolve().parent
R = N.parent


def u(h):
    if h is None:
        return float("inf")
    return struct.unpack(">d", bytes.fromhex(h))[0] if isinstance(h, str) else float(h)


probe = json.loads((N / "PROBE_RESULTS.json").read_text())
chart = json.loads((N / "CHART_CHECK.json").read_text())
rev02 = json.loads((N / "REV02_CHECK.json").read_text())

a = json.loads((R / "rev02_nonnormal_stepping_20261003/cases.json").read_text())
b = json.loads((N / "rev02_cases.json").read_text())
arms = same = cand = 0
rels = []
for ca, cb in zip(a["cases"], b["cases"]):
    for arm in ("stepped_identity", "stepped_osborne", "single_step_identity_m30",
                "single_step_osborne_m30"):
        cx = ca[arm].get("certificate", ca[arm])
        cy = cb[arm].get("certificate", cb[arm])
        if "error_upper" not in cx:
            continue
        arms += 1
        ea, eb = u(cx["error_upper"]), u(cy["error_upper"])
        same += ea == eb
        cand += cx.get("candidate") == cy.get("candidate")
        if ea != eb and 0 < ea < float("inf"):
            rels.append((eb - ea) / ea)

pa = json.loads((R / "rnext05_chart_provider_20261003/runs.json").read_text())
pb = json.loads((N / "chart_runs.json").read_text())
points = ysame = bsame = 0
crel = []
for ra, rb in zip(pa["runs"], pb["runs"]):
    for sa, sb in zip(ra["steps"], rb["steps"]):
        for x, y in [(sa, sb)] + list(zip(sa["dense"], sb["dense"])):
            points += 1
            ysame += x["y"] == y["y"]
            ea, eb = u(x["b_phys"]), u(y["b_phys"])
            bsame += ea == eb
            if ea != eb:
                crel.append((eb - ea) / ea)

result = {
    "schema": "vigilode-safe-enclosure-results-v1",
    "probes": probe["summary"],
    "probe_max_base_under": {"c1_relative": probe["c1"]["max_base_under_relative"],
                             "c2_relative": probe["c2"]["max_base_under_relative"],
                             "c3_ulps": probe["c3"]["max_base_under_ulps"]},
    "repaired_violations_verbatim": {k: probe[k]["repaired_violations"] for k in ("c1", "c2", "c3")},
    "gate": {
        "g1_repaired_zero_violations": probe["gate_g1_repaired_zero_violations"],
        "g2_chart_check_verdict": chart["verdict"],
        "g2_rev02_enclosure": rev02["gate"]["1_enclosure"],
        "g2_rev02_directed_exponential": rev02["gate"]["2_directed_exponential"],
        "g3_published_rev02_enclosure_gate": json.loads(
            (R / "rev02_nonnormal_stepping_20261003/RESULTS.json").read_text())["gate"]["1_enclosure"],
        "g3_published_rnext05_verdict": json.loads(
            (R / "rnext05_chart_provider_20261003/RESULTS.json").read_text()).get("verdict"),
    },
    "rev02_bound_change": {"arms": arms, "identical": same, "candidates_identical": cand,
                           "relative_increase_min": min(rels) if rels else 0.0,
                           "relative_increase_max": max(rels) if rels else 0.0},
    "chart_bound_change": {"points": points, "y_identical": ysame, "bound_identical": bsame,
                           "relative_increase_min": min(crel) if crel else 0.0,
                           "relative_increase_max": max(crel) if crel else 0.0},
}
g = result["gate"]
result["verdict"] = "PASS" if (g["g1_repaired_zero_violations"] and g["g2_chart_check_verdict"] == "PASS"
                               and g["g2_rev02_enclosure"]) else "FAIL"
(N / "RESULTS.json").write_text(json.dumps(result, indent=1) + "\n")
print(json.dumps(result, indent=1))
