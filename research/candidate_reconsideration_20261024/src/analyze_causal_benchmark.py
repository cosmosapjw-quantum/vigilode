#!/usr/bin/env python3
"""Summarize streaming benchmark JSONL. Failures remain explicit; no imputation."""
import argparse
import hashlib
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    all_groups = defaultdict(list)
    headers, sources, parse_errors = {}, [], []
    warmups = []
    for path in args.inputs:
        data = path.read_bytes()
        sources.append({"path": str(path), "sha256": hashlib.sha256(data).hexdigest()})
        for line_number, line in enumerate(data.decode().splitlines(), 1):
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                parse_errors.append({"path": str(path), "line": line_number, "error": str(error)})
                continue
            if row.get("kind") == "header":
                if row["case"] in headers:
                    raise ValueError(f"multiple campaigns for {row['case']}; analyze separately")
                headers[row["case"]] = row
            elif row.get("kind") == "sample":
                all_groups[(row["case"], row["mode"])].append(row)
            elif row.get("kind") == "warmup":
                warmups.append(row)
            else:
                parse_errors.append({"path": str(path), "line": line_number, "unrecognized_row": row})
    rows, cases = [], []
    for case, header in headers.items():
        by_mode = {}
        for mode in header["modes"]:
            samples = all_groups[(case, mode)]
            passed = [s for s in samples if s["result"].get("success") is True]
            elapsed = [s["elapsed_seconds"] for s in samples]
            complete = len(samples) == header["repeats"] and sorted(s["repeat"] for s in samples) == list(range(header["repeats"]))
            times_finite = all(math.isfinite(t) and t > 0 for t in elapsed)
            eligible = complete and len(passed) == len(samples) and times_finite
            errors = [s["result"].get("errors", {}).get("max_abs_error") for s in samples]
            errors = [e for e in errors if isinstance(e, (float, int))]
            work = samples[0]["result"].get("work", {}) if samples else {}
            counters = samples[0]["result"].get("counters", {}) if samples else {}
            protected_routes = work.get("fallback_attempts")
            after_candidate = (protected_routes - work.get("candidate_skipped", 0)
                               if eligible and protected_routes is not None else None)
            row = {"case": case, "mode": mode, "samples": len(samples), "passed": len(passed),
                   "complete_repetition_set": complete, "speed_comparison_eligible": eligible,
                   "seconds_median": statistics.median(elapsed) if elapsed else None,
                   "seconds_min": min(elapsed, default=None), "seconds_max": max(elapsed, default=None),
                   "max_abs_error": max(errors, default=None), "first_sample_work": work,
                   "protected_route_attempts": protected_routes,
                   "post_candidate_protected_attempts": after_candidate,
                   "first_sample_counters": counters,
                   "work_reproducible_across_repeats": all(s["result"].get("work", {}) == work and s["result"].get("counters", {}) == counters for s in samples),
                   "failures": [s for s in samples if s["result"].get("success") is not True]}
            by_mode[mode] = row
            rows.append(row)
        adaptive = by_mode.get("switching-causal")
        fixed = by_mode.get("fixed-causal")
        legacy = by_mode.get("legacy-switching-causal")
        protected = [r for mode, r in by_mode.items() if mode.startswith("protected-") and r["speed_comparison_eligible"]]
        best = min(protected, key=lambda r: r["seconds_median"], default=None)
        valid_adaptive = adaptive and adaptive["speed_comparison_eligible"]
        fixed_ratio = fixed["seconds_median"] / adaptive["seconds_median"] if valid_adaptive and fixed and fixed["speed_comparison_eligible"] else None
        strongest_ratio = best["seconds_median"] / adaptive["seconds_median"] if valid_adaptive and best else None
        legacy_ratio = legacy["seconds_median"] / adaptive["seconds_median"] if valid_adaptive and legacy and legacy["speed_comparison_eligible"] else None
        policy_changed_work = (legacy["first_sample_counters"] != adaptive["first_sample_counters"] or
                               any(legacy["first_sample_work"].get(k) != adaptive["first_sample_work"].get(k)
                                   for k in ("candidate_evaluations", "candidate_skipped", "candidate_failures", "fallback_attempts"))) if legacy and adaptive else None
        cases.append({"case":case,"fixed_over_adaptive":fixed_ratio,
                      "legacy_switching_over_adaptive":legacy_ratio,
                      "priced_policy_changed_routing_or_counted_work":policy_changed_work,
                      "legacy_ratio_interpretation":"changed route/work; descriptive timing comparison" if policy_changed_work else "same counted work; timing variation is not evidence of a policy speed improvement",
                      "best_protected":best["mode"] if best else None,"best_protected_over_adaptive":strongest_ratio,
                      "adaptive_work":adaptive["first_sample_work"] if adaptive else {},
                      "adaptive_passed":bool(valid_adaptive),
                      "scoped_interpretation":("lower cost than unconditional candidate" if fixed_ratio is not None and fixed_ratio > 1 else "no observed cost reduction over unconditional candidate"),
                      "competitive_with_fastest_protected":strongest_ratio is not None and strongest_ratio >= 1.0})
    models = {h.get("registered_model",h["case"]) for h in headers.values()}
    equations = {"few-mode" if m == "expensive-few-mode" else m for m in models}
    summary = {"schema":"vigilode.candidate-reconsideration-benchmark-summary.v1", "sources":sources,
               "workload_configurations":len(headers),"registered_workload_cases":len(models),
               "distinct_equation_families":len(equations),"repetitions_are_not_independent_problems":True,
               "headers":list(headers.values()),"parse_errors":parse_errors,
               "warmup_failures":[r for r in warmups if r["result"].get("success") is not True],
               "sample_count":sum(len(g) for g in all_groups.values()),"groups":rows,"cases":cases,
               "limitations":["Within-process descriptive medians, five cyclic repetitions; no independent-process confidence interval.",
                  "Exact reference sampled at 17 physical observations; no continuous-time error certificate.",
                  "Expensive-RHS case deliberately injects work, not an application measurement.",
                  "All comparators here are in-repository; no new VODE/DVODE, diffsol or Julia results.",
                  "A win over unconditional candidate does not establish a win over protected dense/banded/matrix-free methods."]}
    summary["counter_semantics"] = "Legacy fallback_attempts counts every protected-route attempt, including deliberate calibration/cooldown. post_candidate_protected_attempts subtracts candidate_skipped only for fully successful groups, where each skipped candidate dispatched the protected route."
    (args.output / "CAUSAL_BENCHMARK.json").write_text(json.dumps(summary, indent=2)+"\n")
    lines = ["# Causal candidate-reconsideration measurements", "", "All ratios are baseline time divided by adaptive time; values above one favour the adaptive path. The fastest protected method is selected from the successfully completed in-repository controls at the common sampled-error limit. Failed/incomplete groups are excluded from ratios and retained in JSON.", "",
             "| Case | Fixed/adaptive | Legacy switching/adaptive | Policy changes work? | Best protected | Best protected/adaptive | Adaptive candidate attempts/skips |", "|---|---:|---:|---|---|---:|---:|"]
    fmt = lambda x: f"{x:.3f}" if x is not None else "not eligible"
    for row in cases:
        work = row["adaptive_work"]
        lines.append(f"| {row['case']} | {fmt(row['fixed_over_adaptive'])} | {fmt(row['legacy_switching_over_adaptive'])} | {row['priced_policy_changed_routing_or_counted_work']} | {row['best_protected']} | {fmt(row['best_protected_over_adaptive'])} | {work.get('candidate_evaluations', 'NA')}/{work.get('candidate_skipped', 'NA')} |")
    lines += ["", "| Case | Method | Passes/samples | Median ms [min, max] | Maximum sampled absolute error |", "|---|---|---:|---:|---:|"]
    for row in rows:
        timing = f"{row['seconds_median']*1e3:.3f} [{row['seconds_min']*1e3:.3f}, {row['seconds_max']*1e3:.3f}]" if row["seconds_median"] is not None else "missing"
        error = f"{row['max_abs_error']:.3e}" if row["max_abs_error"] is not None else "missing"
        lines.append(f"| {row['case']} | {row['mode']} | {row['passed']}/{row['samples']} | {timing} | {error} |")
    lines += ["", "## Interpretation limits", ""] + [f"- {s}" for s in summary["limitations"]]
    (args.output / "CAUSAL_BENCHMARK.md").write_text("\n".join(lines)+"\n")
    plot(summary, args.output)
    print(json.dumps({"sample_count":summary["sample_count"],"groups":len(rows),"all_groups_eligible":all(r["speed_comparison_eligible"] for r in rows),"parse_errors":len(parse_errors),"cases":cases}))


def plot(summary, output):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
    plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
    cases = summary["cases"]
    eligible = [r for r in cases if r["fixed_over_adaptive"] and r["best_protected_over_adaptive"]]
    if not eligible:
        return
    fig, ax = plt.subplots(figsize=(10.4, 4.8))
    x = np.arange(len(eligible))
    ax.bar(x-0.25,[r["fixed_over_adaptive"] for r in eligible],0.25,label="Fixed causal / adaptive causal",color="#0072B2")
    ax.bar(x,[r.get("legacy_switching_over_adaptive") or float("nan") for r in eligible],0.25,label="Legacy switching / adaptive causal",color="#009E73")
    ax.bar(x+0.25,[r["best_protected_over_adaptive"] for r in eligible],0.25,label="Fastest protected / adaptive causal",color="#D55E00")
    ax.axhline(1,color="#333333",linestyle="--",linewidth=1)
    ax.set_yscale("log"); ax.set_ylabel("Time ratio (above 1 favours adaptive)")
    ax.set_xticks(x,[r["case"].replace("-","\n") for r in eligible])
    fig.legend(*ax.get_legend_handles_labels(),loc="upper center",bbox_to_anchor=(0.5,0.92),fontsize=8,ncol=3)
    fig.suptitle("Candidate recovery and protected-method competitiveness")
    fig.tight_layout(rect=(0,0,1,0.86))
    for suffix in ("png","svg","pdf"):
        fig.savefig(output/f"causal_timing_ratios.{suffix}",dpi=180)
    plt.close(fig)
    fig, axes = plt.subplots(1,2,figsize=(11.2,4.0))
    ax=axes[0]
    probes=[r["adaptive_work"].get("candidate_evaluations",0) for r in cases]
    skips=[r["adaptive_work"].get("candidate_skipped",0) for r in cases]
    x=np.arange(len(cases))
    ax.bar(x,probes,label="Candidate evaluated",color="#0072B2")
    ax.bar(x,skips,bottom=probes,label="Protected without candidate",color="#009E73")
    ax.set_xticks(x,[r["case"].replace("-","\n") for r in cases],fontsize=8)
    ax.set_ylabel("Trajectory attempts");ax.legend(fontsize=8);ax.set_title("Actual switching work")
    ax=axes[1]
    modes=sorted(set(r["mode"] for r in summary["groups"]))
    for mode in modes:
        rows=[r for r in summary["groups"] if r["mode"]==mode and r["speed_comparison_eligible"]]
        ax.scatter([r["seconds_median"]*1e3 for r in rows],[max(r["max_abs_error"],1e-18) for r in rows],label=mode,s=24)
    ax.axhline(1e-5,color="#333333",linestyle="--",linewidth=1)
    ax.set_xscale("log");ax.set_yscale("log");ax.set_xlabel("Complete trajectory time (ms)");ax.set_ylabel("Maximum sampled absolute error")
    ax.set_title("Observed work and accuracy");ax.legend(fontsize=7,loc="upper right",ncol=2)
    fig.tight_layout()
    for suffix in ("png","svg","pdf"):
        fig.savefig(output/f"causal_work_errors.{suffix}",dpi=180)
    plt.close(fig)


if __name__ == "__main__":
    main()
