"""Summarize preserved Round04 receipts; never rerun the solver or regenerate tables."""
import json, hashlib, re, math, statistics
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def records(name, marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0]
            for l in (root/name).read_text().splitlines() if marker in l]
reg=records("regression.log","RUN ")
wp=records("timing.log","WP ")
event=records("regression.log","EVENT ")
audit=json.loads((root/"AUDIT.json").read_text())
assert len(reg)==6 and len(wp)==15 and len(event)==1
assert audit["endpoint_cases"]==252 and audit["rational_budget_rows"]==129
assert audit["max_endpoint_error_over_bound"]<=1 and audit["minimum_endpoint_margin"]>=0
for r in reg+wp:
    m=r["metrics"];tol=r["tol"]
    assert m["bound_endpoint"]<=tol
    if r["dense"]: assert m["max_dense_upper"]<=tol
    assert all(math.isfinite(v) and 0<v for v in m["plan"]["budgets"])
    assert max(m["plan"]["planned_target_upper"])<=tol
for r in wp:
    assert len(r["samples_ms"])==5 and r["median_ms"]==statistics.median(r["samples_ms"])
counts={}
name=None
for l in (root/"final_tests.log").read_text().splitlines():
    m=re.search(r"Running tests/(\w+)\.rs",l)
    if m:name=m[1]
    m=re.search(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",l)
    if m:
        assert name is not None
        counts[name]=dict(zip(("passed","failed","ignored"),map(int,m.groups())))
assert set(counts)=={"affine_event_certificate_contracts","fast_dense_contracts","retro_causal_contracts","retro_green_contracts"}
assert sum(v["passed"] for v in counts.values())==31 and all(v["failed"]==0 for v in counts.values())
assert "test result: ok. 1 passed; 0 failed" in (root/"timing.log").read_text()
comparisons=[]
for dense,tol in [(False,1e-5),(False,1e-7),(False,1e-9),(True,1e-5),(True,1e-7)]:
    arms={r["mode"]:r for r in wp if r["dense"]==dense and r["tol"]==tol}
    assert set(arms)=={"Forward","Uniform","Retro"}
    a,b,c=[arms[k] for k in ("Forward","Uniform","Retro")]
    def rhs(r):return r["metrics"]["work"]["native"]["rhs_calls"]
    comparisons.append(dict(dense=dense,tol=tol,
        forward_over_retro=a["median_ms"]/c["median_ms"],
        uniform_over_retro=b["median_ms"]/c["median_ms"],
        rhs_forward_over_retro=rhs(a)/rhs(c),
        rhs_uniform_over_retro=rhs(b)/rhs(c),
        observed_error_ratio_retro_forward=c["metrics"]["max_observed_error"]/a["metrics"]["max_observed_error"],
        retro_end_bound=c["metrics"]["bound_endpoint"],
        retro_dense_bound=c["metrics"]["max_dense_upper"]))
sources=["crates/rodas5p-integrators/src/lib.rs",
         "crates/rodas5p-integrators/src/retro_causal.rs",
         "crates/rodas5p-integrators/tests/retro_causal_contracts.rs",
         "crates/rodas5p-integrators/src/retro_green.rs",
         "crates/rodas5p-integrators/src/retro_phi_cell.hex",
         "crates/rodas5p-integrators/src/rodas5p_fast.rs",
         "crates/rodas5p-integrators/src/rodas5p_fast_dense.rs",
         "crates/rodas5p-integrators/src/affine_event_certificate.rs","Cargo.lock"]
out={"schema":"vigilode.retro-causal.round04.v1",
     "base":(root/"BASE.txt").read_text().splitlines(),
     "authority":"diagnostic_only",
     "contract":"same declared affine IVP and target bounds; not exactly matched achieved error; timing excludes post-run report queries and event localization",
     "test_counts":counts,"timing_test_run_separately":True,
     "audit":audit,"regression_runs":reg,"timing":wp,"comparisons":comparisons,"event":event,
     "source_sha256":{p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in sources},
     "raw_sha256":{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.glob("*.log"))}}
(root/"RESULTS.json").write_text(json.dumps(out,indent=2,sort_keys=True)+"\n")
print(json.dumps({"test_counts":counts,"comparisons":comparisons},indent=2))
