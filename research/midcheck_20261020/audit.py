#!/usr/bin/env python3
"""Cross-check retained full solver states, work and paired time units."""
import csv,gzip,hashlib,json,math,statistics,sys
from pathlib import Path
import numpy as np
out=Path(sys.argv[1])
refs=json.loads((out/"references.json").read_text())
sel=json.loads((out/"FROZEN_MATCHED_SELECTION.json").read_text())
selection={(x["case"],x["grid"]):x for x in sel}
rows=list(csv.DictReader((out/"RESULTS.csv").open()))
pairs=json.loads((out/"PAIRS.json").read_text())
allruns=[json.loads(line) for line in gzip.open(out/"RAW.jsonl.gz","rt")]
assert allruns and len(rows)>25 and len(pairs)>=30
by={(x["case"],x["grid"],x["arm"],x["block"]):x for x in allruns}
assert len(by)==len(allruns)
def reference(case,g):
    if case=="few-mode-96":
        ts=[0.,.001]
        v=[]
        for t in ts:
            row=[]
            for i in range(96):
                a=[-1.,-20.,-100.][i%3];z=math.expm1(a*t)
                row.append(.1*(1+z)/(1-(.1/a)*z))
            v.append(row)
        return np.asarray(v),np.ones((2,96))*4e-16,ts,1.e-4
    r=refs[case+"|"+g]
    return np.asarray(r["states"]),np.asarray(r["radius"]),r["times"],(1.e-4 if case in ("robertson","hires") else 1.)
checked=0;invalid=0
for x in allruns:
    if not x["valid"]:
        invalid+=1
        assert x["E_up"] is None or x["status"]=="NOT_RERUN_FAILED_PREFLIGHT"
        continue
    y=np.asarray(x["model_states"]);v,rho,ts,scale=reference(x["case"],x["grid"])
    assert x["times"]==ts and y.shape==v.shape
    assert np.isfinite(y).all()
    denom=scale+np.abs(v)
    e=float(max(np.sqrt(np.mean(((y-v)/denom)**2,axis=1)))+
            max(np.sqrt(np.mean((rho/denom)**2,axis=1))))
    assert math.isclose(e,x["E_up"],rel_tol=5e-12,abs_tol=1e-22),(x["case"],x["arm"],e,x["E_up"])
    assert len(x["timing_ns"])==5 and statistics.median(x["timing_ns"])==x["median_ns"]
    checked+=1
for p in pairs:
    if p["valid_blocks"]!=8:continue
    ratios=[]
    for b in range(8):
        a=by[(p["case"],p["grid"],p["baseline"],b)]
        c=by[(p["case"],p["grid"],p["candidate"],b)]
        ratios.append(a["median_ns"]/c["median_ns"])
    assert ratios==p["ratios"] and statistics.median(ratios)==p["median_ratio"]
output={"status":"PASS_SCOPED_RAW_STATE_READBACK","verified_complete_rows":checked,
        "excluded_failed_rows":invalid,"verified_paired_medians":sum(p["valid_blocks"]==8 for p in pairs),
        "reference_type":"old precommitted independent Radau/LSODA or exact Riccati",
        "not_claimed":"continuous-time certification, optimal cross-backend tuning, admitted production host"}
(out/"AUDIT.json").write_text(json.dumps(output,indent=2)+"\n")
print(json.dumps(output))
