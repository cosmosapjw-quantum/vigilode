#!/usr/bin/env python3
"""Independently replay every saved component and analyze paired local runs.

Replay validates reported errors and work, not global accuracy against rtol.
Accuracy decisions use separately stated, tolerance-independent error caps.
"""
import argparse
import hashlib
import json
import math
import pathlib
import statistics
import struct
import tarfile
from collections import defaultdict

def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")

def initial(case, i):
    if case == "logistic": return 0.05 + 0.01 * (i % 8)
    if case in ("rapid", "damped"): return 1.0 if i % 2 == 0 else 0.0
    if case == "nonnormal": return 0.0 if i % 2 == 0 else 1.0
    return 1.0 / (1 + i % 8)

def exact(case, t, i):
    # Independent Python standard-library analytic formulas. No Rust reference
    # arrays or producer error summaries enter this computation.
    y0 = initial(case, i)
    if case == "logistic":
        rate = 0.25 + 0.1 * (i % 8)
        return 1.0 / (1.0 + (1.0 / y0 - 1.0) * math.exp(-rate * t))
    if case in ("rapid", "damped"):
        w = 100.0 + 5.0 * ((i // 2) % 4) if case == "rapid" else 10.0 + (i // 2) % 4
        envelope = math.exp(-3.0 * t) if case == "damped" else 1.0
        return envelope * (math.cos(w * t) if i % 2 == 0 else math.sin(w * t))
    if case == "nonnormal":
        return 30.0 * (math.exp(-t) - math.exp(-2.0 * t)) if i % 2 == 0 else math.exp(-2.0 * t)
    rate = 0.5 + 0.05 * (i % 8) if case == "large-vector" else (0.25, 1.0, 3.0, 0.5)[i % 4]
    return y0 * math.exp(-rate * t) + (math.sin(t) if case == "driven" else 0.0)

def replay(run, cfg, source):
    problems = []
    def require(ok, text):
        if not ok: problems.append(text)
    for key in ("case", "mode", "dimension", "rtol", "atol", "observations"):
        require(run.get(key) == cfg[key], f"configuration mismatch: {key}")
    require(run.get("source_commit") == source, "source mismatch")
    require(run.get("schema") == "vigilode.production-hardening-run.v1", "schema mismatch")
    require(run.get("success") is True, "solver did not report success")
    n, count = cfg["dimension"], cfg["observations"]
    end = 10.0 if cfg["case"] == "logistic" else 2.0 if cfg["case"] == "rapid" else 4.0
    schedule = [end * i / (count - 1) for i in range(count)]
    require(run.get("end") == end, "wrong span end")
    require(run.get("requested_times") == schedule, "producer requested schedule mismatch")
    require(run.get("times") == schedule, "returned times mismatch")
    states = run.get("states", [])
    shape_ok = len(states) == count and all(len(y) == n and all(isinstance(x, (int, float)) and math.isfinite(x) for x in y) for y in states)
    require(shape_ok, "malformed/nonfinite state")
    elapsed = run.get("elapsed_seconds")
    require(isinstance(elapsed, (float, int)) and math.isfinite(elapsed) and elapsed > 0, "invalid duration")
    absolute = normalized = discrepancy = None
    state_hash = endpoint_hash = None
    if shape_ok and run.get("times") == schedule:
        absolute, normalized = 0.0, 0.0
        state_digest = hashlib.sha256()
        for t, row in zip(schedule, states):
            state_digest.update(struct.pack(f"<{n}d", *row))
            for i, y in enumerate(row):
                ref = exact(cfg["case"], t, i)
                err = abs(y - ref)
                absolute = max(absolute, err)
                normalized = max(normalized, err / max(1.0, abs(initial(cfg["case"], i)), abs(ref)))
        state_hash = state_digest.hexdigest()
        endpoint_hash = hashlib.sha256(struct.pack(f"<{n}d", *states[-1])).hexdigest()
        for name, expected in (("max_abs_error", absolute), ("max_normalized_error", normalized)):
            actual = run.get(name)
            require(isinstance(actual, (float, int)) and math.isfinite(actual), f"nonfinite/missing {name}")
            if isinstance(actual, (float, int)) and math.isfinite(actual):
                difference = abs(actual - expected)
                discrepancy = max(discrepancy or 0.0, difference)
                require(difference <= max(5e-14, 1e-8 * expected), f"independent {name} disagreement")
        require(run.get("final_time") == end and run.get("final_state") == states[-1], "final state mismatch")
    work = run.get("work", {})
    counters = work.get("counters", {})
    attempts = work.get("attempts", -1)
    require(work.get("accepted_steps", 0) + work.get("rejected_steps", 0) == attempts, "attempt accounting")
    require(counters.get("rhs_calls") == 1 + 6 * attempts, "FSAL RHS work identity")
    require(work.get("rhs_completed") == counters.get("rhs_calls"), "incomplete successful RHS accounting")
    require(work.get("fsal_reuses") == attempts - 1, "FSAL reuse accounting")
    for name, value in counters.items():
        if any(tag in name for tag in ("jacobian", "jvp", "newton", "factorization", "linear_solve")):
            require(value == 0, f"unexpected implicit work: {name}")
    return {"passed": not problems, "problems": problems, "max_abs_error": absolute,
            "max_normalized_error": normalized, "max_report_disagreement": discrepancy,
            "elapsed_seconds": elapsed, "work": work, "states_sha256_le_f64": state_hash,
            "endpoint_sha256_le_f64": endpoint_hash}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder")
    args = parser.parse_args()
    folder = pathlib.Path(args.folder)
    plan = json.loads((folder / "PLAN.json").read_text())
    receipts = json.loads((folder / "RECEIPTS.json").read_text())
    configs = {c["id"]: c for c in plan["configurations"]}
    archive = tarfile.open(folder / "RAW_OUTPUTS.tar.gz", "r:gz")
    manifest = json.loads((folder / "RAW_MANIFEST.json").read_text())
    hashes = {row["path"]: row["sha256"] for row in manifest}
    raw_hash_checks = []
    for name, digest in hashes.items():
        data = archive.extractfile(name).read()
        raw_hash_checks.append(hashlib.sha256(data).hexdigest() == digest)
    replays = []
    for receipt in receipts:
        item = {k: receipt[k] for k in ("id", "phase", "repeat")}
        if receipt["status"] != "COMPLETED":
            item.update(passed=False, problems=[receipt["status"]])
        else:
            try:
                data = archive.extractfile(receipt["stdout"]).read()
                if hashlib.sha256(data).hexdigest() != receipt["stdout_sha256"]:
                    raise ValueError("receipt stdout hash mismatch")
                item.update(replay(json.loads(data), configs[receipt["id"]], plan["source_commit"]))
            except (ValueError, TypeError, KeyError, AttributeError) as error:
                item.update(passed=False, problems=[f"replay failure: {error}"])
        replays.append(item)
    archive.close()
    write_json(folder / "REPLAY.json", replays)
    rows = []
    for idx, cfg in configs.items():
        all_runs = [r for r in replays if r["id"] == idx]
        timed = [r for r in all_runs if r["phase"] == "timed"]
        eligible = len(timed) == plan["timed_repeats"] and all(r["passed"] for r in all_runs)
        row = {**cfg, "eligible": eligible, "replayed_runs": len(all_runs)}
        if eligible:
            duration = [r["elapsed_seconds"] for r in timed]
            row.update(median_seconds=statistics.median(duration), min_seconds=min(duration), max_seconds=max(duration),
                       max_normalized_error=max(r["max_normalized_error"] for r in timed),
                       max_abs_error=max(r["max_abs_error"] for r in timed),
                       work=timed[0]["work"],
                       repeated_states_identical=len({r["states_sha256_le_f64"] for r in all_runs}) == 1,
                       endpoint_sha256_le_f64=timed[0]["endpoint_sha256_le_f64"],
                       repeats_work_identical=all(r["work"] == timed[0]["work"] for r in all_runs))
        rows.append(row)
    groups = defaultdict(list)
    for row in rows:
        groups[(row["case"], row["dimension"])].append(row)
    density, paired, caps = [], [], []
    for (case, n), group in groups.items():
        for rtol in sorted({r["rtol"] for r in group}, reverse=True):
            selected = [r for r in group if r["rtol"] == rtol]
            dense = next(r for r in selected if r["mode"] == "dense" and r["observations"] > 5)
            sparse = next(r for r in selected if r["mode"] == "dense" and r["observations"] == 5)
            landing = next(r for r in selected if r["mode"] == "landing")
            available = dense["eligible"] and sparse["eligible"]
            keys = ("attempts", "accepted_steps", "rejected_steps", "rhs_completed", "fsal_reuses")
            density.append({"case": case, "dimension": n, "rtol": rtol,
                            "dense_config": dense["id"], "sparse_config": sparse["id"],
                            "passed": available and all(dense["work"][k] == sparse["work"][k] for k in keys)
                            and dense["work"]["counters"] == sparse["work"]["counters"]
                            and dense["endpoint_sha256_le_f64"] == sparse["endpoint_sha256_le_f64"],
                            "scope": "same adaptive trajectory work and final-state bytes; interpolation errors differ with sample grid"})
            if dense["eligible"] and landing["eligible"]:
                paired.append({"case": case, "dimension": n, "rtol": rtol, "observations": dense["observations"],
                               "landing_config": landing["id"], "dense_config": dense["id"],
                               "landing_over_dense_median": landing["median_seconds"] / dense["median_seconds"],
                               "ratio_min_max_envelope": [landing["min_seconds"] / dense["max_seconds"], landing["max_seconds"] / dense["min_seconds"]],
                               "landing_error": landing["max_normalized_error"], "dense_error": dense["max_normalized_error"],
                               "landing_rhs": landing["work"]["counters"]["rhs_calls"], "dense_rhs": dense["work"]["counters"]["rhs_calls"],
                               "equal_requested_tolerance_is_not_equal_achieved_accuracy": True})
        for cap in plan["accuracy_caps"]:
            best = {}
            for mode in ("landing", "dense"):
                candidates = [r for r in group if r["mode"] == mode and r["observations"] > 5
                              and r["eligible"] and r["max_normalized_error"] <= cap]
                best[mode] = min(candidates, key=lambda r: r["median_seconds"]) if candidates else None
            item = {"case": case, "dimension": n, "error_cap": cap,
                    "status": "COMPARABLE" if all(best.values()) else "INSUFFICIENT_SAMPLED_TOLERANCES",
                    "selected_config_ids": {mode: row["id"] if row else None for mode, row in best.items()}}
            if all(best.values()):
                a, b = best["landing"], best["dense"]
                item.update(landing_over_dense_median=a["median_seconds"] / b["median_seconds"],
                            ratio_min_max_envelope=[a["min_seconds"] / b["max_seconds"], a["max_seconds"] / b["min_seconds"]],
                            landing_error=a["max_normalized_error"], dense_error=b["max_normalized_error"])
            caps.append(item)
    summary = {"schema": "vigilode.production-hardening-summary.v1", "source_commit": plan["source_commit"],
               "configuration_count": len(rows), "run_count": len(replays), "replay_pass_count": sum(r["passed"] for r in replays),
               "raw_hash_count": len(raw_hash_checks), "raw_hashes_all_match": all(raw_hash_checks),
               "density_invariance_pass_count": sum(r["passed"] for r in density), "density_invariance_count": len(density),
               "configuration_summaries": rows, "density_invariance": density,
               "fixed_tolerance_pairs": paired, "common_error_cap_pairs": caps,
               "caveats": ["Replay PASS certifies recorded-error fidelity, not requested global tolerance.",
                           "Sparse observations are excluded from work-precision comparisons because they sample fewer times.",
                           "Three timed repetitions give observed envelopes, not confidence intervals.",
                           "No universal superiority or cross-host speed claim; external evidence is separate."]}
    write_json(folder / "SUMMARY.json", summary)
    text = ["# Paired observation-policy benchmark", "", f"Source: `{plan['source_commit']}`.", "",
            f"{summary['replay_pass_count']}/{len(replays)} full-state replays pass; {sum(raw_hash_checks)}/{len(raw_hash_checks)} raw hashes match.",
            f"Dense output work/final-state invariance: {summary['density_invariance_pass_count']}/{len(density)} pairs.", "",
            "The table chooses each policy's fastest measured configuration satisfying the same independent error cap. Missing entries mean that the sampled tolerances did not qualify both policies.", "",
            "| Model | n | Error cap | Landing / dense median time | Observed min–max ratio envelope |", "|---|---:|---:|---:|---:|"]
    for p in caps:
        if p["status"] == "COMPARABLE":
            lo, hi = p["ratio_min_max_envelope"]
            text.append(f"| {p['case']} | {p['dimension']} | {p['error_cap']:.0e} | {p['landing_over_dense_median']:.3f} | {lo:.3f}–{hi:.3f} |")
        else: text.append(f"| {p['case']} | {p['dimension']} | {p['error_cap']:.0e} | insufficient | — |")
    text += ["", "Ratios above one favor dense observation. This compares two observation policies of the current DOPRI5 implementation on the same host. It is not a comparison to a previous source version or external solvers.", "", "A successful solve only certifies that the adaptive driver reached its endpoint under its estimator. The reported analytic errors and strict-cap failures remain visible in SUMMARY.json."]
    (folder / "RESULTS.md").write_text("\n".join(text) + "\n")
    try:
        make_plots(folder, groups, paired)
        plot_status = {"status": "PASS", "figures": ["WORK_PRECISION.png", "WORK_PRECISION.pdf", "WORK_PRECISION.svg", "RHS_WORK.png", "RHS_WORK.pdf", "RHS_WORK.svg"]}
    except ImportError as error:
        plot_status = {"status": "NOT_RUN", "reason": str(error)}
    write_json(folder / "PLOT_STATUS.json", plot_status)
    print(json.dumps({k: summary[k] for k in ("configuration_count", "run_count", "replay_pass_count", "raw_hash_count", "raw_hashes_all_match", "density_invariance_pass_count", "density_invariance_count")}, indent=2))
    return 0 if all(raw_hash_checks) and all(r["passed"] for r in replays) and all(r["passed"] for r in density) else 1

def make_plots(folder, groups, paired):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    count = len(groups)
    fig, axes = plt.subplots(math.ceil(count / 2), 2, figsize=(11, 3 * math.ceil(count / 2)), squeeze=False)
    for ax, ((case, n), group) in zip(axes.flat, groups.items()):
        for mode, color, marker in (("landing", "#9e4f24", "o"), ("dense", "#235e9c", "s")):
            rows = sorted((r for r in group if r["eligible"] and r["mode"] == mode and r["observations"] > 5), key=lambda r: r["max_normalized_error"])
            if not rows: continue
            errors = [max(1e-17, r["max_normalized_error"]) for r in rows]
            times = [r["median_seconds"] for r in rows]
            ax.errorbar(errors, times, yerr=[[r["median_seconds"]-r["min_seconds"] for r in rows], [r["max_seconds"]-r["median_seconds"] for r in rows]], label=mode, color=color, marker=marker, capsize=3)
        ax.set(xscale="log", yscale="log", title=f"{case}, n={n}", xlabel="Maximum normalized analytic error", ylabel="Timed solve (s)")
        ax.grid(True, which="both", alpha=.22)
        ax.legend(fontsize=8)
    for ax in list(axes.flat)[count:]: ax.set_visible(False)
    fig.suptitle("Same-host DOPRI5 observation policies\nMedian and observed min–max, three timed repeats", fontsize=13)
    fig.tight_layout(rect=(0, 0, 1, .96))
    for ext in ("png", "pdf", "svg"): fig.savefig(folder / f"WORK_PRECISION.{ext}", dpi=170)
    plt.close(fig)
    fig, ax = plt.subplots(figsize=(11, 4.8))
    labels, landing, dense = [], [], []
    for row in paired:
        if row["rtol"] == 1e-6:
            labels.append(f"{row['case']}\nn={row['dimension']}")
            landing.append(row["landing_rhs"]); dense.append(row["dense_rhs"])
    xs = list(range(len(labels)))
    ax.bar([x-.19 for x in xs], landing, .38, label="landing", color="#9e4f24")
    ax.bar([x+.19 for x in xs], dense, .38, label="dense", color="#235e9c")
    ax.set_xticks(xs, labels, fontsize=8)
    ax.set(yscale="log", ylabel="RHS calls", title="Observation work at rtol=1e−6\n501 observations (small models), 51 (large vectors)")
    ax.grid(axis="y", alpha=.2); ax.legend()
    fig.tight_layout()
    for ext in ("png", "pdf", "svg"): fig.savefig(folder / f"RHS_WORK.{ext}", dpi=170)
    plt.close(fig)

if __name__ == "__main__":
    raise SystemExit(main())
