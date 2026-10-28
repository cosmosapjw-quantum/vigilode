#!/usr/bin/env python3
"""Separate root-schedule experiment; never pool one- and two-thread timings."""
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path


def main():
    p=argparse.ArgumentParser()
    p.add_argument("inputs",nargs="+",type=Path)
    p.add_argument("--output",required=True,type=Path)
    a=p.parse_args(); a.output.mkdir(parents=True,exist_ok=True)
    sources=[];groups=[];headers=[];failures=[]
    for file in a.inputs:
        data=file.read_bytes()
        sources.append({"path":str(file),"sha256":hashlib.sha256(data).hexdigest()})
        rows=[json.loads(s) for s in data.decode().splitlines() if s.strip()]
        hs=[r for r in rows if r.get("kind")=="header"]
        if len(hs)!=1 or hs[0].get("campaign")!="root-variants":
            raise ValueError(f"not one root-variants campaign: {file}")
        h=hs[0];headers.append(h)
        for mode in h["modes"]:
            samples=[r for r in rows if r.get("kind")=="sample" and r["mode"]==mode]
            valid=len(samples)==h["repeats"] and sorted(r["repeat"] for r in samples)==list(range(h["repeats"]))
            valid=valid and all(r["result"].get("success") is True and math.isfinite(r["elapsed_seconds"]) and r["elapsed_seconds"]>0 for r in samples)
            times=[r["elapsed_seconds"] for r in samples]
            errors=[r["result"].get("errors",{}).get("max_abs_error") for r in samples]
            errors=[x for x in errors if isinstance(x,(int,float))]
            result=samples[0]["result"] if samples else {}
            group={"case":h["case"],"threads":h["threads"],"mode":mode,"samples":len(samples),
                "passed":sum(r["result"].get("success") is True for r in samples),"speed_comparison_eligible":valid,
                "seconds_median":statistics.median(times) if times else None,
                "seconds_min":min(times,default=None),"seconds_max":max(times,default=None),
                "max_abs_error":max(errors,default=None),"first_sample_work":result.get("work"),
                "first_sample_counters":result.get("counters")}
            work=result.get("work",{})
            group["protected_route_attempts"]=work.get("fallback_attempts")
            group["post_candidate_protected_attempts"]=(work["fallback_attempts"]-work.get("candidate_skipped",0)
                if valid and "fallback_attempts" in work else None)
            groups.append(group)
            failures.extend(r for r in rows if r.get("mode")==mode and r.get("kind") in ("sample","warmup") and r["result"].get("success") is not True)
    index={(r["case"],r["threads"],r["mode"]):r for r in groups}
    if len(index)!=len(groups):raise ValueError("duplicate case/thread/method campaigns")
    def ratio(base,candidate):
        if base and candidate and base["speed_comparison_eligible"] and candidate["speed_comparison_eligible"]:
            return base["seconds_median"]/candidate["seconds_median"]
        return None
    comparisons=[]
    for case in sorted(set(r["case"] for r in groups)):
        for mode in sorted(set(r["mode"] for r in groups)):
            serial=index.get((case,1,mode));parallel=index.get((case,2,mode))
            comparisons.append({"case":case,"mode":mode,"one_thread_over_two_threads":ratio(serial,parallel),
                "serial_forward_over_two_threads":ratio(index.get((case,1,"fixed-causal")),parallel),
                "serial_dense_over_two_threads":ratio(index.get((case,1,"protected-dense")),parallel)})
    switching=[]
    for case in sorted(set(r["case"] for r in groups)):
        for threads in (1,2):
            for candidate in ("causal","picard","residual-first"):
                switching.append({"case":case,"threads":threads,"candidate":candidate,
                    "fixed_over_switching":ratio(index.get((case,threads,"fixed-"+candidate)),index.get((case,threads,"switching-"+candidate)))})
    workload_cases={h["case"] for h in headers}
    equation_families={"few-mode" if c=="expensive-few-mode" else c for c in workload_cases}
    summary={"schema":"vigilode.root-variants-summary.v1","sources":sources,"headers":headers,
        "registered_workload_cases":len(workload_cases),"distinct_equation_families":len(equation_families),
        "repetitions_are_not_independent_problems":True,
        "groups":groups,"comparisons":comparisons,"switching_comparisons":switching,"failures":failures,
        "sample_count":sum(r["samples"] for r in groups),
        "interpretation":"Exact-arithmetic finite propagation does not imply elapsed-time acceleration. Compare against serial causal forward and serial dense as well as same-method parallel speedup.",
        "timing_limit":"Five cyclic repetitions in one process per case/thread setting; amortized caller-owned reused pool, pool construction excluded; no one-shot end-to-end parallel-speed guarantee or independent-process CI.",
        "counter_semantics":"fallback_attempts includes deliberate protected calibration/cooldown; post_candidate_protected_attempts subtracts candidate_skipped only in fully successful groups.",
        "synthetic_cost":"expensive-few-mode injects 2048 sin evaluations into every RHS callback; its crossover is not a measured application speedup."}
    (a.output/"ROOT_VARIANTS.json").write_text(json.dumps(summary,indent=2)+"\n")
    fmt=lambda x:f"{x:.3f}" if x is not None else "not eligible"
    lines=["# Root-schedule and parallel-cost experiment","",summary["interpretation"],"",summary["timing_limit"],"",summary["synthetic_cost"],"",
        "| Case | Method | 1-thread / 2-thread | Serial forward / 2-thread method | Serial dense / 2-thread method |",
        "|---|---|---:|---:|---:|"]
    for r in comparisons:
        lines.append(f"| {r['case']} | {r['mode']} | {fmt(r['one_thread_over_two_threads'])} | {fmt(r['serial_forward_over_two_threads'])} | {fmt(r['serial_dense_over_two_threads'])} |")
    lines += ["","| Case | Threads | Candidate | Fixed / switching |","|---|---:|---|---:|"]
    for r in switching:lines.append(f"| {r['case']} | {r['threads']} | {r['candidate']} | {fmt(r['fixed_over_switching'])} |")
    lines += ["","| Case | Threads | Method | Passed / samples | Median ms | Maximum sampled error |","|---|---:|---|---:|---:|---:|"]
    for r in groups:
        ms=f"{1e3*r['seconds_median']:.3f}" if r["seconds_median"] is not None else "missing"
        err=f"{r['max_abs_error']:.3e}" if r["max_abs_error"] is not None else "missing"
        lines.append(f"| {r['case']} | {r['threads']} | {r['mode']} | {r['passed']}/{r['samples']} | {ms} | {err} |")
    (a.output/"ROOT_VARIANTS.md").write_text("\n".join(lines)+"\n")
    plot(summary,a.output)
    print(json.dumps({"samples":summary["sample_count"],"groups":len(groups),"failures":len(failures),"comparisons":comparisons}))


def plot(summary,output):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
    plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
    fig,axes=plt.subplots(1,2,figsize=(10.8,4.4),sharey=True)
    cases=["few-mode","expensive-few-mode","riccati"]
    x=np.arange(len(cases))
    for ax,mode,title in zip(axes,("fixed-picard","fixed-residual-first"),("Causal Picard","Residual-first correction")):
        rows={r["case"]:r for r in summary["comparisons"] if r["mode"]==mode}
        for shift,key,label,color in [(-.25,"one_thread_over_two_threads","Same method, 1 thread","#0072B2"),
            (0,"serial_forward_over_two_threads","Causal forward, 1 thread","#009E73"),
            (.25,"serial_dense_over_two_threads","Dense protected, 1 thread","#D55E00")]:
            ax.bar(x+shift,[rows[c][key] if rows[c][key] is not None else float("nan") for c in cases],.25,label=label,color=color)
        ax.axhline(1,color="#333333",linestyle="--",linewidth=1)
        ax.set_yscale("log");ax.set_xticks(x,[c.replace("-","\n") for c in cases]);ax.set_title(title)
    axes[0].set_ylabel("Serial baseline time / two-thread candidate time")
    axes[1].legend(fontsize=8,loc="upper left")
    fig.suptitle("Parallel callback-cost crossover (reused caller-owned pool)")
    fig.tight_layout()
    for suffix in ("png","svg","pdf"):fig.savefig(output/f"root_parallel_crossover.{suffix}",dpi=180)
    plt.close(fig)

if __name__=="__main__":main()
