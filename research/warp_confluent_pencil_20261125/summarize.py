import hashlib,json,re,statistics
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def rows(name,marker):
    return [json.JSONDecoder().raw_decode(line.split(marker,1)[1])[0]
            for line in (root/name).read_text().splitlines() if marker in line]
wp=rows("work_precision.log","WARP_WP ")
cache=rows("prepared_cache.log","WARP_CACHE ")
assert len(wp)==24 and len(cache)==6
for r in wp+cache:
    assert len(r["samples_ms"])==5
    assert statistics.median(r["samples_ms"])==r["median_ms"]
    assert r.get("bound",r.get("max_bound"))<=r["tol"]
comparisons=[]
for n in (32,128,512,1024):
    for tol in (1e-10,2e-12):
        t={r["arm"]:r for r in wp if r["n"]==n and r["tol"]==tol}
        assert len(t)==3
        a,d,p=[t[k] for k in ("warp-elimination","direct-three-modal-lu","full-coefficient-picard")]
        comparisons.append({"n":n,"tol":tol,"direct_over_warp":d["median_ms"]/a["median_ms"],
                            "picard_over_warp":p["median_ms"]/a["median_ms"],
                            "warp_center_solves":a["warp_center_solves"],"picard_center_solves":3*(p["picard_levels"]+1)})
c=[]
for n in (128,512,1024):
    a,d=[next(r for r in cache if r["n"]==n and r["arm"]==k) for k in ("warp-prepared","three-lu-prepared")]
    c.append({"n":n,"warp_over_direct":a["median_ms"]/d["median_ms"]})
counts=[tuple(map(int,m)) for m in re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",(root/"final_tests.log").read_text())]
assert len(counts)==5 and sum(t[0] for t in counts)==84 and all(t[1]==0 for t in counts)
audit=json.loads((root/"EXACT_AUDIT.json").read_text())
assert audit["count"]==19 and audit["underbounds"]==0
inputs=["crates/rodas5p-core/src/lib.rs","crates/rodas5p-core/src/warp_cubic.rs","crates/rodas5p-core/tests/warp_cubic_contracts.rs","Cargo.lock"]
result={"schema":"vigilode.warp-cubic.coding.v1","base":(root/"BASE.txt").read_text().splitlines(),
        "authority":"diagnostic_only","test_counts":{"passed":sum(t[0] for t in counts),"failed":0,"ignored_timing_tests":sum(t[2] for t in counts),"timing_tests_run_separately":2},
        "exact_audit_cases":audit["count"],"work_precision":wp,"comparisons":comparisons,
        "prepared_cache":cache,"prepared_cache_comparisons":c,
        "source_sha256":{name:hashlib.sha256((repo/name).read_bytes()).hexdigest() for name in inputs},
        "raw_sha256":{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(root.glob("*.log"))}}
(root/"RESULTS.json").write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
print("SUMMARY",len(wp),len(cache),result["test_counts"],"exact",audit["count"])
