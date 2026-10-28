"""Bounded instruction-count comparison after unresolved wall timings.

Callgrind is a deterministic emulated instruction diagnostic, not native
elapsed time. Two repetition lengths subtract command setup/serialization.
No production source or numerical target is changed.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import subprocess
import time
from run_benchmarks import signature

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]
WORK=REPO.parent
OUT=HERE/"instruction_profile"


def write(p,x):
    p.write_text(json.dumps(x,indent=2,allow_nan=False)+"\n")


def main():
    OUT.mkdir(exist_ok=True)
    if (OUT/"SUMMARY.json").exists():
        raise RuntimeError("refusing to overwrite completed instruction experiment")
    val=WORK/"tmp/valgrind_local/usr/bin/valgrind.bin"
    env=dict(os.environ,VALGRIND_LIB=str(WORK/"tmp/valgrind_local/usr/libexec/valgrind"),
             RAYON_NUM_THREADS="1",OPENBLAS_NUM_THREADS="1",OMP_NUM_THREADS="1")
    protocol={"version":"Valgrind/Callgrind 3.22.0 Ubuntu noble",
              "metric":"Ir per repetition from difference of two batch totals",
              "selected":[["lu",32,100,1000],["dense",65,2,6],["adapter",16,2,10]],
              "accuracy":"profiled arms must retain analytic cap and within-profiler output equality",
              "maximum_wall_seconds":150,"wall_time_claim":False,
              "cpu_feature_scope":"Valgrind guest; not necessarily native SIMD dispatch",
              "reason":"Native wall and process CPU timing remained inconclusive in two passes"}
    write(OUT/"PROTOCOL.json",protocol)
    start=time.monotonic(); receipts=[]; rows=[]
    for kind,n,small,large in protocol["selected"]:
        measured={}; outputs={}
        for optimized in [False,True]:
            arm="candidate" if optimized else "baseline"
            for reps in [small,large]:
                name=f"{kind}_{n}_{arm}_{reps}"
                if kind=="lu":
                    cmd=[WORK/"tmp/bin_candidate/lu_workspace_bench","workspace" if optimized else "legacy",n,reps]
                elif kind=="dense":
                    cmd=[WORK/f"tmp/bin_{arm}/mode_dense_trajectory_20261027",n,reps]
                else:
                    cmd=[WORK/"tmp/bin_candidate/audit_adapter_20261027",.4,1,9999,"1e-4",0,0,8,n,
                         "optimized" if optimized else "baseline",reps]
                raw=OUT/(name+".callgrind")
                full=[str(val),"--tool=callgrind","--cache-sim=no","--branch-sim=no",
                      "--callgrind-out-file="+str(raw)]+list(map(str,cmd))
                remaining=150-(time.monotonic()-start)
                if remaining<1: raise RuntimeError("profile budget exhausted")
                t0=time.monotonic()
                p=subprocess.run(full,cwd=REPO,env=env,text=True,capture_output=True,timeout=min(40,remaining))
                (OUT/(name+".stdout.json")).write_text(p.stdout)
                (OUT/(name+".stderr.txt")).write_text(p.stderr)
                receipts.append({"name":name,"command":full,"exit":p.returncode,
                                 "elapsed_seconds":time.monotonic()-t0})
                write(OUT/"COMMANDS.json",receipts)
                if p.returncode:
                    write(OUT/"FAILURE.json",{"status":"PROFILE_BLOCKED","name":name,"exit":p.returncode})
                    raise RuntimeError(f"{name} failed; no automatic retries")
                out=json.loads(p.stdout)
                total=int(re.search(r"^summary: (\d+)",raw.read_text(),re.M).group(1))
                if kind=="lu": valid=out["legacy_bitwise_equal"] and out["max_error"]<=2e-13
                elif kind=="dense": valid=out["max_grid_error"]<=out["cap"]
                else: valid=out["accepted_trajectory_equal"] and out["deterministic"] and all(out[a]["accuracy_pass"] for a in ["baseline","optimized"])
                if not valid: raise RuntimeError("profile accuracy qualification failed")
                measured[(optimized,reps)]=total;outputs[(optimized,reps)]=out
        if kind=="dense":
            sigs=[signature(o) for o in outputs.values()]
            if any(s!=sigs[0] for s in sigs[1:]):
                raise RuntimeError("profiled dense trajectories/counters differ")
        a=(measured[(False,large)]-measured[(False,small)])/(large-small)
        b=(measured[(True,large)]-measured[(True,small)])/(large-small)
        row={"kind":kind,"n":n,"small_reps":small,"large_reps":large,
             "totals":[{"optimized":opt,"reps":rep,"Ir":ir} for (opt,rep),ir in measured.items()],
             "baseline_Ir_per_rep":a,"candidate_Ir_per_rep":b,
             "baseline_over_candidate":a/b,"reduction_fraction":1-b/a,"accuracy_pass":True}
        rows.append(row);write(OUT/"PARTIAL_SUMMARY.json",{"rows":rows})
        print(f"{kind} n={n}: {a:.1f} -> {b:.1f} Ir/rep ({100*(1-b/a):.3f}% reduction)",flush=True)
    write(OUT/"SUMMARY.json",{"status":"COMPLETE","protocol":protocol,"rows":rows,
                              "elapsed_seconds":time.monotonic()-start})
    print("COMPLETE",flush=True)


if __name__=="__main__": main()
