#!/usr/bin/env python3
"""Fresh same-host full-solve native comparisons + fixed-h coupled arms.

Uses SOURCE-PINNED old independent references and accuracy-frozen tolerances.
Reports measured achieved accuracy, and refuses a matched-error rank unless
each arm passes its own target and observed error ratio <=2.
"""
import csv,gzip,hashlib,json,math,random,statistics,subprocess,sys
from pathlib import Path
import numpy as np

binary=Path(sys.argv[1]).resolve()
archive=Path(sys.argv[2]).resolve()
out=Path(sys.argv[3]).resolve();out.mkdir(parents=True,exist_ok=True)
frozen=json.loads((archive/"FROZEN_MATCHED_SELECTION.json").read_text())
refs=json.loads((archive/"references.json").read_text())
selection={(x["case"],x["grid"]):x for x in frozen}
records=[];rng=random.Random(20261020)
scopes=["robertson","hires","van-der-pol-mu1000","brusselator-1d-50","brusselator-1d-200"]
cases=[(case,g) for case in scopes for g in ("endpoint","uniform101")]
cases.append(("few-mode-96","endpoint"))

def sha(path):
    h=hashlib.sha256()
    with path.open("rb") as f:
        for b in iter(lambda:f.read(2**20),b""): h.update(b)
    return h.hexdigest()
def arms(case):
    if case=="few-mode-96":return ["coupled-eager","coupled-lazy","vigilode-fast","diffsol-bdf","netlib-dvode"]
    if case.startswith("brusselator"):return ["vigilode-fast","vigilode-banded","diffsol-bdf","netlib-dvode"]
    return ["vigilode-fast","diffsol-bdf","netlib-dvode"]
def tolerance(case,g,arm):
    if case=="few-mode-96":return 1.e-6
    return selection[(case,g)]["matched"][arm]["rtol"]
def refcase(case,g):
    if case=="few-mode-96":
        ts=[0.,.001]; v=[]
        for t in ts:
            y=[]
            for i in range(96):
                a=[-1.,-20.,-100.][i%3]
                em=math.expm1(a*t)
                y.append(.1*(1+em)/(1-(.1/a)*em))
            v.append(y)
        return np.array(v),np.ones((2,96))*4e-16,ts,1.e-4
    data=refs[f"{case}|{g}"]
    return np.asarray(data["states"]),np.asarray(data["radius"]),data["times"],(1.e-4 if case in ("robertson","hires") else 1.)
def call(case,g,arm,warm,reps):
    args=[str(binary),"bench",case,arm,format(tolerance(case,g,arm),".17g"),g,"1",str(warm),str(reps)]
    try:
        x=subprocess.run(args,capture_output=True,text=True,timeout=35)
        if x.returncode: return {"status":"process_failure","error":x.stderr[-1500:],"command":args,"exit":x.returncode}
        r=json.loads(x.stdout)
        r["command"]=args;r["exit"]=0
        return r
    except subprocess.TimeoutExpired:
        return {"status":"timeout","error":"35s ceiling","command":args}
def assess(r,case,g):
    v,radius,ts,scale=refcase(case,g); tol=tolerance(case,g,r.get("arm","vigilode-fast"))
    ans={"structurally_valid":False,"E_up":None,"R_up":None,"reference_fraction":None}
    if r.get("status")!="ok" or r.get("deterministic") is not True:return ans
    if r.get("times")!=ts:return ans
    try:y=np.asarray(r["states"],dtype=float)
    except (ValueError,TypeError):return ans
    if y.shape!=v.shape or not np.isfinite(y).all():return ans
    w=(scale+np.abs(v));raw=np.max(np.linalg.norm((y-v)/w,axis=1)/math.sqrt(v.shape[1]))
    er=np.max(np.linalg.norm(radius/w,axis=1)/math.sqrt(v.shape[1]))
    e=float(raw+er)
    return {"structurally_valid":True,"E_up":e,"R_up":e/tol,"reference_fraction":float(er)}
def one(case,g,arm,b):
    r=call(case,g,arm,1,5)
    r["arm"]=arm
    m=assess(r,case,g)
    times=r.get("elapsed_ns") or []
    valid=m["structurally_valid"] and len(times)==5 and all(isinstance(t,int) and t>0 for t in times)
    ent={"case":case,"grid":g,"arm":arm,"block":b,"rtol":tolerance(case,g,arm),
         "status":r.get("status"),"valid":bool(valid),"E_up":m["E_up"],"R_up":m["R_up"],
         "reference_fraction":m["reference_fraction"],"median_ns":float(statistics.median(times)) if valid else None,
         "command":r.get("command"),"timing_ns":times,"model_states":r.get("states"),"times":r.get("times"),
         "work":r.get("counters"),"error":r.get("error")}
    return ent

preflight=[]
for case,g in cases:
    for arm in arms(case):
        r=one(case,g,arm,-1);preflight.append(r)
        if r["status"]!="ok":print("preflight non-success",case,g,arm,r["status"],flush=True)
(out/"PREFLIGHT.json").write_text(json.dumps(preflight,indent=2,allow_nan=False)+"\n")
# Retain every failed configuration, but avoid repeatedly executing a known failure
# or violating the CPU/time budget with unbounded futile attempts.
eligible={(r["case"],r["grid"],r["arm"]):r["valid"] for r in preflight}
with gzip.open(out/"RAW.jsonl.gz","wt",compresslevel=6) as f:
    for b in range(8):
        shuffled=cases.copy();rng.shuffle(shuffled)
        for case,g in shuffled:
            aa=arms(case)
            if b%2:aa=aa[::-1]
            aa=aa[b%len(aa):]+aa[:b%len(aa)]
            for arm in aa:
                if eligible[(case,g,arm)]:
                    r=one(case,g,arm,b)
                else:
                    r=dict(next(x for x in preflight if (x["case"],x["grid"],x["arm"])==(case,g,arm)))
                    r["block"]=b;r["status"]="NOT_RERUN_FAILED_PREFLIGHT";r["valid"]=False;r["median_ns"]=None
                records.append(r);f.write(json.dumps(r,allow_nan=False,separators=(",",":"))+"\n");f.flush()
        print("completed comparison block",b+1,flush=True)

# For each case/grid: time ratio only among complete passes, and only declare
# matched-achieved if ratio of observed worst errors <= 2 (not just equal rtol).
rows=[];pairs=[]
for case,g in cases:
    for arm in arms(case):
        r=[x for x in records if (x["case"],x["grid"],x["arm"])==(case,g,arm)]
        good=[x for x in r if x["valid"]]
        target=1.e-6 if case=="few-mode-96" else selection[(case,g)]["anchor"]
        eligible_error= len(good)==8 and max(x["E_up"] for x in good)<=max(target,1e-15)*1.1
        rows.append({"case":case,"grid":g,"arm":arm,"valid_blocks":len(good),
                     "median_ms":statistics.median(x["median_ns"] for x in good)/1e6 if len(good)==8 else None,
                     "E_up_max":max(x["E_up"] for x in good) if good else None,
                     "R_up_max":max(x["R_up"] for x in good) if good else None,
                     "target":target,"achieved_pass":eligible_error})
    # pair against VigilODE fast on legacy, and eager/lazy on few-mode
    aa=arms(case)
    combos=(list((aa[0],a) for a in aa[1:]) if case!="few-mode-96" else
        [("coupled-eager","coupled-lazy"),("vigilode-fast","coupled-lazy"),
         ("diffsol-bdf","coupled-lazy"),("netlib-dvode","coupled-lazy"),
         ("netlib-dvode","diffsol-bdf"),("vigilode-fast","diffsol-bdf")])
    for baseline,candidate in combos:
        base=[x for x in records if (x["case"],x["grid"],x["arm"])==(case,g,baseline)]
        cand=[x for x in records if (x["case"],x["grid"],x["arm"])==(case,g,candidate)]
        okay=len(base)==len(cand)==8 and all(x["valid"] and y["valid"] for x,y in zip(base,cand))
        ratios=[x["median_ns"]/y["median_ns"] for x,y in zip(base,cand)] if okay else []
        target=1e-6 if case=="few-mode-96" else selection[(case,g)]["anchor"]
        achieved=([x["E_up"] for x in base+cand] if okay else [])
        matched=bool(okay and max(achieved)<=1.1*target and
                     max(achieved)/max(min(achieved),np.finfo(float).tiny)<=2.0)
        pairs.append({"case":case,"grid":g,"baseline":baseline,"candidate":candidate,
                      "valid_blocks":len(ratios),"median_ratio":statistics.median(ratios) if okay else None,
                      "observed_error_ratio":max(achieved)/max(min(achieved),np.finfo(float).tiny) if okay else None,
                      "matched_achieved_pass":matched,
                      "scope":"MATCHED_ACHIEVED" if matched else "DIAGNOSTIC_ONLY_NOT_MATCHED",
                      "ratios":ratios})
with (out/"RESULTS.csv").open("w",newline="") as f:
    w=csv.DictWriter(f,fieldnames=rows[0].keys());w.writeheader();w.writerows(rows)
with (out/"PAIRS.json").open("w") as f:json.dump(pairs,f,indent=2,allow_nan=False)
summary={"source_binary_sha256":sha(binary),"existing_strict_source":"9c6a6d9145795e35b978652a567b806ddc84a6bd",
         "legacy_cases":len(cases)-1,"new_common_case":"few-mode-96",
         "preflight_attempts":len(preflight),"timing_cells":len(records),
         "valid_timing_cells":sum(x["valid"] for x in records),
         "matched_pairs":sum(x["matched_achieved_pass"] for x in pairs),
         "diagnostic_pairs":sum(x["scope"]!="MATCHED_ACHIEVED" for x in pairs),
         "no_automated_production_speed_claim":True,
         "reference_type":"frozen Radau+LSODA empirical for legacy; Riccati exact formula for common case"}
(out/"SUMMARY.json").write_text(json.dumps(summary,indent=2)+"\n")
print(json.dumps(summary))
