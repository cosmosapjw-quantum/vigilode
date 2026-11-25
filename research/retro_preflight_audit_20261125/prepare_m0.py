#!/usr/bin/env python3
"""Read historical measurements and lock inputs only; never invokes a solver."""
import argparse, hashlib, json, math, pathlib, shutil
P=pathlib.Path
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def verify_runs(base,filename):
    rows={}
    for line in (base/filename).read_text().splitlines():
        digest,rel=line.split(maxsplit=1);rel=rel.removeprefix("*")
        if not rel.startswith("run/"):continue
        p=(base/rel).resolve()
        assert base.resolve() in p.parents and p.is_file()
        assert sha(p)==digest,(p,digest)
        rows[rel]=digest
    assert rows
    return rows
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--base",type=P,required=True);p.add_argument("--expanded",type=P,required=True);p.add_argument("--out",type=P,required=True);args=p.parse_args()
    a,b,o=args.base,args.expanded,args.out
    old=verify_runs(a,"RAW_SHA256SUMS");new=verify_runs(b,"INPUT_RUN_SHA256SUMS")
    c=json.loads((a/"COMPARISON.json").read_bytes());s=json.loads((b/"SUMMARY.json").read_bytes())
    assert c["original_rows"]==c["new_rows"]==c["common_keys"]==1032
    assert s["total_fresh_rows"]==2076 and s["fresh_failure"]==46 and s["fresh_success"]==2030 and s["original_key_replays"]==504
    cases=[];corpora=[]
    for source,raw,name in [("cross_solver_v1_valid.json","cross_solver_v1_rust.json","canonical"),("adversarial_spatial_battery_v3.json","spatial_battery_v3_rust.json","spatial")]:
        src=a/"harness"/source;rawfile=a/"run"/raw
        doc=json.loads(rawfile.read_bytes());corpus=json.loads(src.read_bytes())
        assert doc["corpus"]==corpus
        target=o/(name+".corpus.json");assert not target.exists()
        shutil.copyfile(src,target);assert sha(target)==sha(src)
        corpora.append({"cohort":name,"historical_source":str(src),"copied_file":target.name,"sha256":sha(target),"error_rtol":corpus["error_rtol"],"grid_points":corpus["grid_points"],"reference_policy":corpus["reference"]})
        byid={r["problem"]:r for r in doc["references"]}
        for spec in corpus["problems"]:
            r=byid[spec["id"]];states=r["states"];levels=r["independent_protocol"]["levels"]
            assert len(states)==101 and all(len(x)==spec["n"] and all(math.isfinite(y) for y in x) for x in states)
            cases.append({"cohort":name,"problem":spec["id"],"spec":spec,"historical_disputed":r["disputed"],"historical_reference_status":r["status"],"old_ladder_status":r["independent_protocol"]["status"],"old_levels":levels,"reference_array_canonical_json_sha256":hashlib.sha256(json.dumps(states,separators=(",",":"),allow_nan=False).encode()).hexdigest(),"new_adjudication":"NOT_RUN"})
    assert len(cases)==14 and sum(x["historical_disputed"] for x in cases)==12
    lock={"schema":"vigilode.m0.preparation.v1","status":"INPUTS_LOCKED_NOT_RUN","baseline_summary_sha256":sha(a/"COMPARISON.json"),"expanded_summary_sha256":sha(b/"SUMMARY.json"),"baseline_raw":old,"expanded_raw":new,"corpora":corpora,"cases":cases,"historical_reference_disputes_preserved":12,"no_new_reference_solve":True,"no_matched_error_reclassification":True}
    target=o/"M0_INPUT_LOCK.json";assert not target.exists();target.write_text(json.dumps(lock,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({"old_raw_files":len(old),"expanded_raw_files":len(new),"cases":len(cases),"disputed":12,"lock_sha256":sha(target),"state":lock["status"]}))
if __name__=="__main__":main()
