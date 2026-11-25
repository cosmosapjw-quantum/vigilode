"""Parse observed paired solver timings; no extrapolation of work-precision data."""
import hashlib
import json
import math
import re
from pathlib import Path

root = Path(__file__).resolve().parent
rows = [json.loads(line.split("CSAS_WP ", 1)[1])
        for line in (root / "work_precision.log").read_text().splitlines()
        if "CSAS_WP " in line]
assert len(rows) == 10
arms = {}
for row in rows:
    arms.setdefault(row["arm"], {})[row["rtol"]] = row
baseline = arms["shipped-gmres"]
candidate = arms["verified-fourier-upwind"]
assert baseline.keys() == candidate.keys()
pairs = []
for tol in sorted(baseline, reverse=True):
    a, b = baseline[tol], candidate[tol]
    assert a["accepted"] == b["accepted"] and a["rejected"] == b["rejected"]
    assert a["rhs"] == b["rhs"]
    assert b["linear_iterations"] == 0
    assert len(a["samples_ms"]) == len(b["samples_ms"]) == 7
    pairs.append({"rtol": tol, "speed_ratio": a["median_ms"]/b["median_ms"],
                  "absolute_error_difference": abs(a["error"]-b["error"]),
                  "relative_error_difference": abs(a["error"]-b["error"])/a["error"],
                  "jvp_ratio": a["jvp"]/b["jvp"]})

def interpolate(arm, target):
    points = sorted((r["error"],r["median_ms"]) for r in arm.values())
    for (e0,t0),(e1,t1) in zip(points,points[1:]):
        if e0 <= target <= e1:
            w = math.log(target/e0)/math.log(e1/e0)
            return math.exp((1-w)*math.log(t0)+w*math.log(t1))
    return None

matched=[]
for target in (1e-9,1e-8,1e-7,1e-6,1e-5):
    a,b=interpolate(baseline,target),interpolate(candidate,target)
    assert a is not None and b is not None
    matched.append({"target_common_endpoint_error":target,"gmres_ms_interpolated":a,
                    "fourier_ms_interpolated":b,"speed_ratio":a/b})
regression = (root/"regression.log").read_text()
counts = [tuple(map(int,m)) for m in re.findall(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",regression)]
assert len(counts)==3 and all(f==0 for p,f,i in counts)
result={"schema":"vigilode.csas-upwind.coding.v1","timing_authority":"diagnostic_only",
        "base":(root/"BASE.txt").read_text().splitlines(),
        "scope":"original periodic-advdiff-n256 stencil, endpoint, rtol=atol, 7 paired repeats",
        "test_counts":{"passed":sum(c[0] for c in counts),"failed":sum(c[1] for c in counts),
                       "ignored":sum(c[2] for c in counts)},
        "pairs":pairs,"matched_error_interpolation":matched,
        "max_absolute_error_difference":max(p["absolute_error_difference"] for p in pairs),
        "max_relative_error_difference":max(p["relative_error_difference"] for p in pairs),
        "rows":rows,
        "raw_sha256":{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(root.glob("*.log"))}}
(root/"RESULTS.json").write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
print(json.dumps({k:result[k] for k in ("test_counts","pairs","matched_error_interpolation",
    "max_absolute_error_difference","max_relative_error_difference")},indent=2))
