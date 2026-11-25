"""Parse measured work and preserve source/evidence identities."""
import json,hashlib,re,statistics
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def rows(name,marker):
    return [json.JSONDecoder().raw_decode(s.split(marker,1)[1])[0]
            for s in (root/name).read_text().splitlines() if marker in s]
kernel=rows("timing.log","KERNEL ")
driver=rows("timing.log","DRIVER_TIMING ")
assert len(kernel)==16 and len(driver)==8
for r in kernel:
    assert len(r["samples_ms"])==7 and statistics.median(r["samples_ms"])==r["median_ms"]
    assert r["relative_error"]<1e-12
for r in driver:
    assert len(r["samples_ms"])==5 and statistics.median(r["samples_ms"])==r["median_ms"]
    assert r["metrics"]["steps"]==4 and r["metrics"]["physical_bound"]<=1e-8
    assert r["metrics"]["rhs_calls"]==24
def grouped(rs,keys):
    groups={}
    for r in rs:
        groups.setdefault(tuple(r[k] for k in keys),{})[r["arm"]]=r
    for key,arms in groups.items():
        assert len(arms)==4
        old=arms["old-fft"]["median_ms"]; direct=arms["direct"]["median_ms"]
        f=arms["warp-full"]["median_ms"]; c=arms["warp-crt"]["median_ms"]
        yield dict(zip(keys,key),old_over_full=old/f,old_over_crt=old/c,direct_over_full=direct/f,direct_over_crt=direct/c,full_over_crt=f/c)
counts=[]
for name in ("final_integration.log","final_unit.log"):
    counts.extend(tuple(map(int,r)) for r in re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",(root/name).read_text()))
assert len(counts)==4 and sum(r[0] for r in counts)==11 and all(r[1]==0 for r in counts),counts
audit=json.loads((root/"EXACT_AUDIT.json").read_text())
assert audit["count"]==10
ivp=json.loads((root/"DRIVER_AUDIT.json").read_text())
assert ivp["all_reference_checks_pass"] and len(ivp["rows"])==4
alias=re.search(r"ALIAS relative=([0-9.eE+-]+) reconstructed=([0-9.eE+-]+)",(root/"regression.log").read_text())
assert alias
inputs=["crates/rodas5p-integrators/src/fourier_path_candidate.rs","crates/rodas5p-integrators/src/fourier_path_candidate/warp.rs","crates/rodas5p-integrators/src/fourier_path_certificate.rs","crates/rodas5p-integrators/tests/warp_pp06_contracts.rs","tools/pp05_fourier_check.py","tools/pp06_fft_check.py","research/pp06_fourier_fft_candidate_20261004/RESULTS.json","Cargo.lock"]
result={"schema":"vigilode.warp-pp06.coding.v1","authority":"diagnostic_only",
        "base":(root/"BASE.txt").read_text().splitlines(),
        "test_counts":{"passed":sum(r[0] for r in counts),"failed":0,"ignored":sum(r[2] for r in counts),"timing_run_separately":True},
        "kernel":kernel,"driver":driver,"kernel_comparisons":list(grouped(kernel,("k","p","sk","q"))),
        "driver_comparisons":list(grouped(driver,("omega",))),
        "exact_audit":audit,"driver_reference_audit":ivp,
        "harmful_alias_control":{"forced_single_quotient_relative_error":float(alias[1]),"crt_relative_error":float(alias[2])},
        "source_sha256":{p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in inputs},
        "raw_sha256":{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.glob("*.log"))}}
(root/"RESULTS.json").write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
print(json.dumps({"test_counts":result["test_counts"],"kernel":result["kernel_comparisons"],"driver":result["driver_comparisons"]},indent=2))
