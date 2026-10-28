#!/usr/bin/env python3
"""Reproduce scalar stability diagnostics and native pilot summaries."""
import hashlib
import json
import math
import statistics
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TABLEAU = [[], [F(1,5)], [F(3,40),F(9,40)],
           [F(44,45),F(-56,15),F(32,9)],
           [F(19372,6561),F(-25360,2187),F(64448,6561),F(-212,729)],
           [F(9017,3168),F(-355,33),F(46732,5247),F(49,176),F(-5103,18656)],
           [F(35,384),F(0),F(500,1113),F(125,192),F(-2187,6784),F(11,84)]]

def stability_polynomial():
    stages = []
    for row in TABLEAU:
        p = [F(1)] + [F(0)] * 6
        for a, prior in zip(row, stages):
            for j in range(6):
                p[j+1] += a * prior[j]
        stages.append(p)
    # FSAL last stage is the endpoint; last B coefficient is zero.
    result = stages[-1]
    assert result == [F(1),F(1),F(1,2),F(1,6),F(1,24),F(1,120),F(1,600)]
    return result

def main():
    coefficients = stability_polynomial()
    def rpoly(z):
        value = 0.0
        for c in reversed(coefficients):
            value = float(c) + z * value
        return value
    lo, hi = 3.0, 4.0
    for _ in range(60):
        mid = (lo + hi) / 2
        if rpoly(-mid) > 1.0: hi = mid
        else: lo = mid
    radius = (lo + hi) / 2
    rows = []
    failures = ROOT / "research/local_frontier_native_diffsol_20261024/audit_packet_20261025/evidence/failures"
    for path in sorted(failures.glob("*/RECEIPT.json")):
        record = json.loads(path.read_text()); result = record.get("result") or {}
        stats = result.get("first_member_stats") or {}
        if not stats.get("attempts"): continue
        n, accepted = stats["attempts"], stats["accepted_steps"]
        row = dict(source=str(path.relative_to(ROOT)), source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                   case=result["case"], method=result["method"], attempts=n, accepted=accepted,
                   rejected=stats["rejected_steps"], accepted_fraction=accepted/n,
                   final_time=result["final_time"], elapsed_seconds=result["elapsed_seconds"],
                   output_clipped_steps=stats["output_clipped_steps"])
        rate = -1e6 if row["case"] == 5 else 4/(32*math.pi/1024)**2 - 16/(32*math.pi/1024)**4
        observed = row["final_time"] / accepted
        row.update(dopri_negative_real_radius=radius, limiting_linear_rate=rate,
                   predicted_stability_h=radius/abs(rate), observed_mean_accepted_h=observed,
                   ratio_to_linear_stability_limit=observed/(radius/abs(rate)),
                   interpretation="linear-mode consistency; nonlinear per-step attribution still needs trace")
        rows.append(row)
    (HERE / "STABILITY_MATCH.json").write_text(json.dumps(rows, indent=2)+"\n")
    (HERE / "STABILITY_POLYNOMIAL.json").write_text(json.dumps({
        "coefficients_exact": [str(c) for c in coefficients],
        "tableau_source": "crates/rodas5p-integrators/src/dopri5.rs DOPRI5_A and DOPRI5_B",
        "tableau_source_sha256": hashlib.sha256((ROOT / "crates/rodas5p-integrators/src/dopri5.rs").read_bytes()).hexdigest(),
        "negative_real_boundary": radius,
        "scope": "Exact rational reconstruction of source tableau; floating bisection of boundary"}, indent=2)+"\n")
    raw = [json.loads(line) for line in (HERE / "PILOT_RAW.jsonl").read_text().splitlines()]
    out = []
    for key in sorted(set((r["eps"],r["end"],r["method"]) for r in raw)):
        group = [r for r in raw if (r["eps"],r["end"],r["method"]) == key and not r["warmup"]]
        row = dict(zip(["epsilon","end","method"],key))
        row.update(median_seconds=statistics.median(r["elapsed_seconds"] for r in group),
                   **{k:group[0][k] for k in ["attempts","accepted","rejected","success","endpoint_max_abs_error",
                      "reference_kind","reference_empirical_uncertainty"]}, raw_repetitions=len(group))
        out.append(row)
    (HERE / "PILOT_SUMMARY.json").write_text(json.dumps(out,indent=2)+"\n")

if __name__ == "__main__": main()
