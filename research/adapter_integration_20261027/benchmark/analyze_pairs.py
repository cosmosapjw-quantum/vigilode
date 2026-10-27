#!/usr/bin/env python3
"""Analyze paired blocks and draw standalone scientific figures from raw data."""
import argparse
import json
import statistics as st
from pathlib import Path


def analyze(record):
    a, b = record["arms"]
    blocks = sorted({x["block"] for x in record["timings"]})
    means = {arm: [st.mean(x["seconds_per_solve"] for x in record["timings"]
                           if x["arm"] == arm and x["block"] == block)
                   for block in blocks] for arm in [a, b]}
    ratios = [y/x for x, y in zip(means[a], means[b])]
    drift = {arm: max(st.median(v[:2]), st.median(v[-2:])) /
                   min(st.median(v[:2]), st.median(v[-2:])) for arm, v in means.items()}
    spread = {arm: max(v)/min(v) for arm, v in means.items()}
    min_batch = min(x["seconds"] for x in record["timings"])
    required_batch = 0.8 * record["target_batch_seconds"]
    same_accuracy = all(v["accuracy_pass"] for v in record["initial_records"].values())
    time_ok = (len(blocks) >= 5 and max(drift.values()) <= 1.15 and max(spread.values()) <= 1.30
               and min_batch >= required_batch and record["deterministic"])
    eligible = time_ok and same_accuracy and record["timed_last_solve_accuracy_pass"] and not record["report_only"]
    reasons = []
    if max(drift.values()) > 1.15: reasons.append("early/late drift >15%")
    if max(spread.values()) > 1.30: reasons.append("block range >30%")
    if min_batch < required_batch: reasons.append(f"batch shorter than {required_batch*1000:g}ms")
    if not same_accuracy: reasons.append("at least one endpoint cap failure")
    if not record["timed_last_solve_accuracy_pass"]: reasons.append("sampled timed solve cap failure")
    if not record["deterministic"]: reasons.append("sampled outcomes not deterministic")
    if record["report_only"]: reasons.append("registered report-only tolerance cohort")
    if len(blocks) < 5: reasons.append("fewer than five blocks")
    classification = ("observed_paired_improvement" if eligible and max(ratios) < .95 else
        "observed_paired_regression" if eligible and min(ratios) > 1.05 else
        "no_clear_timing_advantage" if eligible else "timing_or_accuracy_not_qualified")
    return {"case": record["case"], "a": a, "b": b, "blocks": len(blocks),
            "b_over_a_seconds": st.median(ratios), "paired_ratios": ratios,
            "paired_ratio_range": [min(ratios), max(ratios)],
            "median_seconds": {arm: st.median(v) for arm, v in means.items()},
            "early_late_drift": drift, "block_spread": spread, "min_batch_seconds": min_batch,
            "required_minimum_batch_seconds": required_batch,
            "time_measurement_qualified": time_ok, "comparison_eligible": eligible,
            "classification": classification, "qualification_reasons": reasons,
            "accuracy_pass": same_accuracy, "report_only": record["report_only"],
            "accuracy_cap": record["accuracy_cap"],
            # Full native counters/segments remain in the immutable raw JSON.
            # Avoid copying megabytes of identical telemetry into derived tables.
            "records": {arm: {k: v for k, v in value.items() if k != "stats"}
                        for arm, value in record["initial_records"].items()}}


def plots(rows, out):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
    plt.rcParams.update({"font.size": 9, "axes.spines.top": False, "axes.spines.right": False,
                         "savefig.bbox": "tight", "font.family": "DejaVu Sans"})
    primary = [r for r in rows if r["a"] == "evidence" and r["b"] == "adaptive"]
    fig, axes = plt.subplots(1, 2, figsize=(12, 4.5))
    labels = [r["case"] for r in primary]
    y = np.arange(len(primary))
    rhs_ratios = [r["records"]["adaptive"]["rhs"]/r["records"]["evidence"]["rhs"] for r in primary]
    axes[0].barh(y, rhs_ratios, color="#4679a4")
    axes[0].set(yticks=y, yticklabels=labels, xlabel="Adaptive / evidence RHS calls", title="Charged RHS evaluations (all pilots included)")
    for i,r in enumerate(primary):
        lo, hi = r["paired_ratio_range"]; mid = r["b_over_a_seconds"]
        axes[1].errorbar(mid, i, xerr=[[mid-lo],[hi-mid]], fmt="o" if r["comparison_eligible"] else "x",
                         color="#25856a" if r["comparison_eligible"] else "#b16a35", capsize=3)
    axes[1].set(yticks=y, yticklabels=labels, xlabel="Adaptive / evidence elapsed time", title="Five balanced paired blocks (range)")
    for ax in axes: ax.axvline(1, color="black", linewidth=.8); ax.invert_yaxis()
    fig.text(.5, -.03, "Crosses: timing or accuracy did not qualify. Endpoint caps are common; attained errors may differ.", ha="center")
    save(fig,out,"policy_work_and_time")

    controls = [r for r in rows if r["a"] == "adaptive" and r["b"] not in {"adaptive", "evidence"}]
    fig, ax = plt.subplots(figsize=(10, 5))
    for i,r in enumerate(controls):
        lo, hi = r["paired_ratio_range"]; mid = r["b_over_a_seconds"]
        ax.errorbar(1/mid, i, xerr=[[1/mid-1/hi],[1/lo-1/mid]], fmt="o" if r["comparison_eligible"] else "x",
                    color="#4679a4", capsize=3)
    ax.set(yticks=range(len(controls)), yticklabels=[f"{r['case']} / {r['b']}" for r in controls],
           xlabel="Automatic adaptive / direct-control elapsed time", title="Selection overhead versus the relevant direct route")
    ax.axvline(1,color="black",linewidth=.8);ax.invert_yaxis()
    save(fig,out,"selection_overhead")

    kinds = ["committed_integration", "winning_entry_pilot_uncommitted", "losing_or_incomplete_entry_pilot",
             "uncommitted_explicit_return", "committed_explicit_return", "productive_implicit_comparator",
             "uncommitted_return_comparator", "incomplete_return_comparator", "other_uncommitted"]
    fig, ax = plt.subplots(figsize=(12, 5))
    left=np.zeros(len(primary))
    for kind in kinds:
        values=np.array([r["records"]["adaptive"]["dispositions"].get(kind,{}).get("rhs",0) for r in primary])
        if np.any(values): ax.barh(y,values,left=left,label=kind.replace("_"," ")); left+=values
    ax.set(yticks=y,yticklabels=labels,xlabel="RHS calls",title="Where the adaptive policy spends work")
    ax.invert_yaxis();ax.legend(fontsize=7,loc="upper left",bbox_to_anchor=(1,1))
    save(fig,out,"work_dispositions")

    fig,ax=plt.subplots(figsize=(10,5))
    for offset,arm,color in [(-.15,"evidence","#4679a4"),(.15,"adaptive","#c16b3c")]:
        error=[r["records"][arm]["max_component_absolute_error"]/r["accuracy_cap"] for r in primary]
        ax.scatter(error,y+offset,label=arm,color=color)
    ax.axvline(1,color="black",linewidth=.8);ax.set_xscale("log");ax.invert_yaxis()
    ax.set(yticks=y,yticklabels=labels,xlabel="Attained full-state endpoint error / declared cap",title="Full-state endpoint error against a common accuracy cap")
    ax.legend();save(fig,out,"endpoint_accuracy")

    ablations=[r for r in rows if r["a"]=="adaptive-unseeded" and r["b"]=="adaptive"]
    if ablations:
        fig,axes=plt.subplots(1,2,figsize=(11,3.8))
        yy=np.arange(len(ablations));names=[r["case"] for r in ablations]
        values=[r["records"]["adaptive"]["rhs"]/r["records"]["adaptive-unseeded"]["rhs"] for r in ablations]
        axes[0].barh(yy,values,color="#25856a")
        axes[0].set(yticks=yy,yticklabels=names,xlabel="Seeded / unseeded RHS calls",title="Winning-pilot step reuse: work")
        for i,r in enumerate(ablations):
            lo,hi=r["paired_ratio_range"];mid=r["b_over_a_seconds"]
            axes[1].errorbar(mid,i,xerr=[[mid-lo],[hi-mid]],fmt="o" if r["comparison_eligible"] else "x",
                color="#25856a" if r["comparison_eligible"] else "#b16a35",capsize=3)
        axes[1].set(yticks=yy,yticklabels=names,xlabel="Seeded / unseeded elapsed time",title="Winning-pilot step reuse: paired time")
        for ax in axes:ax.axvline(1,color="black",linewidth=.8);ax.invert_yaxis()
        save(fig,out,"entry_seed_ablation")


def save(fig,out,name):
    import matplotlib.pyplot as plt
    fig.tight_layout()
    for suffix in ["png","svg","pdf"]: fig.savefig(out/f"{name}.{suffix}",dpi=180)
    plt.close(fig)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("input",type=Path);p.add_argument("output",type=Path)
    p.add_argument("--no-plots",action="store_true");args=p.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    rows=[];failures=[]
    run=json.loads((args.input/"RUN.json").read_text())
    for r in run["runs"]:
        if r["exit"] != 0: failures.append(r);continue
        try: rows.append(analyze(json.loads((args.input/r["stdout"]).read_text())))
        except (ValueError,KeyError,ZeroDivisionError) as e: failures.append({**r,"analysis_error":str(e)})
    result={"schema":"vigilode.adapter-integration-analysis.v1","rows":rows,"failed_cells":failures,
            "runtime_pruning_rule":"require common endpoint cap, deterministic sampled state/work, qualified timing; do not drop a method for a noisy comparison",
            "native_baseline_parity":run.get("native_behavior_parity"),
            "uncertainty":"five paired block ratios with empirical range, not a confidence interval"}
    (args.output/"ANALYSIS.json").write_text(json.dumps(result,indent=2)+"\n")
    lines=["# Adapter comparisons", "", "B/A below one favors B; times include solver setup and all trial work.","",
           "| Case | A → B | RHS A → B | B/A time | Range | Qualified | Endpoint error A → B |", "|---|---|---:|---:|---|---|---:|"]
    for r in rows:
        a,b=r["a"],r["b"];ra,rb=r["records"][a],r["records"][b]
        lines.append(f"| {r['case']} | {a} → {b} | {ra['rhs']} → {rb['rhs']} | {r['b_over_a_seconds']:.3f} | {r['paired_ratio_range'][0]:.3f}–{r['paired_ratio_range'][1]:.3f} | {r['comparison_eligible']} | {ra['max_component_absolute_error']:.3g} → {rb['max_component_absolute_error']:.3g} |")
    lines += ["", "Unqualified comparisons:"]
    for r in rows:
        if r["qualification_reasons"]: lines.append(f"- {r['case']} {r['a']}/{r['b']}: {', '.join(r['qualification_reasons'])}.")
    lines += ["", f"Failed cells: {len(failures)}. See ANALYSIS.json for complete records.",
              "", "The two transition positions are new parameter holdouts within an already studied function family.",
              "The case5 matrix-free/dense pair isolates backend cost from policy overhead; neither route is silently substituted."]
    (args.output/"ANALYSIS.md").write_text("\n".join(lines)+"\n")
    if not args.no_plots and rows: plots(rows,args.output)


if __name__=="__main__":main()
