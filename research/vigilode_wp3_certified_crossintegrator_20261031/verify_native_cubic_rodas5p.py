#!/usr/bin/env python3
"""WP3: source-locked achieved global error of native original RODAS5P, not a surrogate.
Input: original WP1 log/summary, WP2 exact-rational certificate/reference, native JSONL.
"""
import argparse
import csv
import hashlib
import json
import math
import struct
import sys
from fractions import Fraction as F
from pathlib import Path

def validate_records(log, summary, certpath, refdir, nativepath):
    sys.path.insert(0, str(Path(log).resolve().parent))
    import DEFECT_CERTIFIER as d
    assert hashlib.sha256(Path(log).read_bytes()).hexdigest() == d.EXPECTED_SWEEP_SHA256
    assert hashlib.sha256(Path(summary).read_bytes()).hexdigest() == d.EXPECTED_WP1_SUMMARY_SHA256
    traces, _ = d.parse_log(log)
    cert = json.loads(Path(certpath).read_text())
    records = [json.loads(t) for t in Path(nativepath).read_text().splitlines() if t.strip()]
    cases = {x["name"]:x for x in records if x.get("kind")=="WP3_CASE"}
    assert set(cases) == set(d.COHORTS), ("MISSING_OR_UNAUTHORIZED_SOURCE",set(cases))
    seen = set()
    rows, failures = [], []
    for cohort in d.COHORTS:
        baseline = traces[(cohort,2,False)]
        source = d.parse_source(baseline)
        steps = len(baseline["points"])-1
        meta = cases[cohort]
        assert meta["source_bits"] == baseline["source_bits"], ("SOURCE_BITS_MISMATCH",cohort)
        assert meta["h_bits"] == baseline["h_bits"], ("H_BITS_MISMATCH",cohort)
        assert meta["steps"] == steps, ("STEP_COUNT_CHANGED",cohort)
        assert meta["initial_state_bits"] == baseline["points"][0]["state"], ("INITIAL_SOURCE_CHANGED",cohort)
        assert meta["horizon_bits"] == d.f_to_bits(.1)
        ref = next(v for v in cert["micro_cohorts"][cohort] if v["reference_micro"]==64)
        by = (Path(refdir)/ref["reference_states_name"]).read_bytes()
        assert hashlib.sha256(by).hexdigest() == ref["reference_states_sha256"]
        width = source["n"]*8
        assert len(by) == (steps*64+1)*width
        reference = [
            struct.unpack(">"+("Q"*source["n"]),by[i*64*width:(i*64+1)*width])
            for i in range(steps+1)
        ]
        epsilon = F.from_float(ref["total_reference_plus_clock_delta_up"])
        for row in (p for p in records if p.get("kind")=="WP3_NATIVE_TRACE" and p.get("name")==cohort):
            idx=int(row["rtol_index"])
            assert idx in range(4), ("UNREGISTERED_RTOL",idx)
            key=(cohort,idx)
            assert key not in seen, ("DUPLICATE_NATIVE_RUN",key)
            seen.add(key)
            assert row["method_id"]=="RODAS5P_native_original"
            assert d.bits_to_f(row["rtol_bits"]) in [1e-4,1e-6,1e-8,1e-10]
            if not row["success"] or not row["complete_output"]:
                failures.append({"cohort":cohort,"rtol_index":idx,"status":"NATIVE_INCOMPLETE","message":row["message"]})
                continue
            assert len(row["time_bits"])==steps+1==len(row["state_bits"])
            assert row["time_bits"] == [p["time"] for p in baseline["points"]], ("CLOCK_LABEL_MISMATCH",key)
            observed_initial_gap=max(abs(d.rational(v)-d.rational(w)) for v,w in zip(row["state_bits"][0],meta["initial_state_bits"]))
            assert observed_initial_gap <= F(1,10**12), ("SAMPLED_INITIAL_STATE_MISMATCH",key)
            maxgap=F(0)
            for yy,rr in zip(row["state_bits"],reference):
                assert len(yy)==source["n"]
                assert all(math.isfinite(d.bits_to_f(v)) and d.bits_to_f(v)>=-1e-12 for v in yy)
                maxgap=max(maxgap,*(abs(d.rational(a)-d.rational(b)) for a,b in zip(yy,rr)))
            low=max(F(0),maxgap-epsilon)
            high=maxgap+epsilon
            wall_ns=sorted(row["wall_ns_raw"])
            assert len(wall_ns)==3
            rows.append({
                "cohort":cohort,"method":"native_original_RODAS5P_8stage_5th_order",
                "rtol_index":idx,"rtol":d.bits_to_f(row["rtol_bits"]),
                "atol":d.bits_to_f(row["atol_bits"]),
                "status":"SOURCE_MATCHED_COMPLETED",
                "achieved_peak_error_lower":float(low),
                "achieved_peak_error_upper":float(high),
                "reference_delta_up":float(epsilon),
                "wall_ns_median":wall_ns[1],"wall_ns_raw":wall_ns,
                "counters":row["counters"],
            })
    assert len(seen)==len(d.COHORTS)*4, ("MISSING_NATIVE_TRACE",len(seen))
    assert rows or failures
    return {
        "schema":"vigilode.wp3.native_original_rodas5p.v1",
        "complete_runs":len(rows),"incomplete_runs":len(failures),
        "work_precision":rows,"failures":failures,
        "scope":"native Rust original source and exact initial bits, WP2-certified achieved-error intervals",
        "production":"HOLD","official_vigilode_dag":"7/23"
    }

def main():
    p=argparse.ArgumentParser()
    for name in ("--log","--summary","--certificate","--reference-dir","--native","--out"):
        p.add_argument(name,required=True)
    a=p.parse_args()
    result=validate_records(a.log,a.summary,a.certificate,a.reference_dir,a.native)
    Path(a.out).write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
    print("WP3_NATIVE_VERIFIED",len(result["work_precision"]),"SUCCESSES",len(result["failures"]),"PRESERVED_FAILURES")

if __name__=="__main__":
    main()
