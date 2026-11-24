"""Evaluate endpoint WRMS and accepted-step ratios on measured overlap only."""
import json
import math
import statistics
from pathlib import Path

here = Path(__file__).resolve().parent
holdout = here.parents[1] / "production_unification_20261115/production_holdout"
reference = json.loads((holdout / "reference.json").read_text())["problems"]
native = [json.loads(line) for line in (here / "raw.jsonl").read_text().splitlines()]
julia_file = here / "julia.jsonl"
julia = [json.loads(line) for line in julia_file.read_text().splitlines()] if julia_file.exists() else []
arms = ["rodas5p_dense", "cvode_bdf_dense"]
tols = {10.0**-k for k in range(4, 9)}
expected = {(p, a, t) for p in reference for a in arms for t in tols}
assert len(native) == len(expected) and {(r["problem"],r["arm"],r["rtol"]) for r in native} == expected
if julia:
    jj = {(p, "julia_rodas5p", t) for p in reference for t in tols | {1e-13}}
    assert len(julia) == len(jj) and {(r["problem"],r["arm"],r["rtol"]) for r in julia} == jj

rows = []
for raw in native + julia:
    ref = reference[raw["problem"]]
    err = [(v-y)/(abs(y)+1e-6) for v,y in zip(raw["result"]["y_end"],ref["y_ref"],strict=True)]
    wrms = math.sqrt(sum(x*x for x in err)/len(err))
    E = wrms + ref["u_ref_wrms"]
    row = dict(problem=raw["problem"], arm=raw["arm"], rtol=raw["rtol"], E=E,
               reference_uncertainty=ref["u_ref_wrms"], admissible=ref["u_ref_wrms"]<=0.1*E,
               accepted=raw["result"]["accepted"])
    assert math.isfinite(E) and row["accepted"] > 0
    if "wall_s" in raw:
        assert len(raw["wall_s"]) == 7 and min(raw["wall_s"]) > 0
        row["wall_s"] = statistics.median(raw["wall_s"])
    rows.append(row)

def interpolate(points, error, key):
    for p in points:
        if p["E"] == error:
            return p[key]
    for p,q in zip(points,points[1:]):
        if p["E"] < error < q["E"]:
            f = math.log(error/p["E"])/math.log(q["E"]/p["E"])
            return p[key] * (q[key]/p[key])**f
    raise ValueError("target outside observed overlap")

comparisons = []
for name in reference:
    sets = [sorted((r for r in rows if r["problem"]==name and r["arm"]==a and r["admissible"]),
                   key=lambda r:r["E"]) for a in arms]
    if not all(sets):
        comparisons.append(dict(problem=name,status="no_admissible_points"))
        continue
    lo = max(s[0]["E"] for s in sets)
    hi = min(s[-1]["E"] for s in sets)
    if lo > hi:
        comparisons.append(dict(problem=name,status="no_overlap"))
        continue
    for E in sorted({lo,math.sqrt(lo*hi),hi}):
        left,right = sets
        comparisons.append(dict(problem=name,E=E,
            accepted_rodas_over_cvode=interpolate(left,E,"accepted")/interpolate(right,E,"accepted"),
            diagnostic_wall_rodas_over_cvode=interpolate(left,E,"wall_s")/interpolate(right,E,"wall_s")))
out = dict(schema="vigilode.u11.external.v1",rows=rows,comparisons=comparisons,
           ratio_direction="protected Rust RODAS5P / native CVODE BDF (accepted steps or wall)",
           wall_status="diagnostic_only_shared_host",julia_cases=len(julia),
           scope="autonomous endpoint only; no dense-output, event or general speed claim")
(here / "results.json").write_text(json.dumps(out,indent=2,allow_nan=False)+"\n")
print(json.dumps(comparisons,indent=1))
