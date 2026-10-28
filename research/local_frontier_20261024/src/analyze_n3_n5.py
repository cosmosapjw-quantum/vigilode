#!/usr/bin/env python3
"""Summarize preserved native measurements; never launch or filter solver runs."""
from collections import Counter
import hashlib
import json
from pathlib import Path
from statistics import median

PKG = Path(__file__).resolve().parents[1]
ROOT = PKG.parents[1]
RUNS = PKG / "evidence/runs"
OUT = PKG / "analysis"
OWN_SOURCE = [
    "crates/vigilode-coupled/src/switching.rs",
    "crates/vigilode-coupled/src/adaptive.rs",
    "crates/vigilode-coupled/tests/switching_contracts.rs",
    "crates/vigilode-coupled/examples/reconsideration_bench.rs",
]


def read_campaign(name):
    path = RUNS / (name + ".stdout")
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.startswith("{")]
    header = next(r for r in rows if r["kind"] == "header")
    samples = [r for r in rows if r["kind"] == "sample"]
    receipt = json.loads((RUNS / (name + ".receipt.json")).read_text())
    assert hashlib.sha256(path.read_bytes()).hexdigest() == receipt["stdout_sha256"]
    modes = {}
    for mode in header["modes"]:
        subset = [r for r in samples if r["mode"] == mode]
        durations = [r["elapsed_seconds"] for r in subset]
        first = subset[0]["result"]
        statuses, phases = Counter(), Counter()
        coordinates = {"inspected": 0, "copied": 0}
        for encoded, count in first["work"].get("switch_decisions", {}).items():
            info = json.loads(encoded)
            if not info:
                continue
            statuses[info.get("reference_status", "absent")] += count
            phases[info.get("phase", "absent")] += count
            for key in coordinates:
                coordinates[key] += count * info.get("reference_state_coordinates_" + key, 0)
        modes[mode] = {
            "elapsed_seconds": durations,
            "median_seconds": median(durations),
            "min_seconds": min(durations),
            "max_seconds": max(durations),
            "all_success": all(r["result"]["success"] for r in subset),
            "max_abs_error": max(r["result"]["errors"]["max_abs_error"] for r in subset),
            "all_counters_identical_across_repeats": all(r["result"]["counters"] == first["counters"] for r in subset),
            "representative_counters": first["counters"],
            "representative_routing": {k: v for k, v in first["work"].items() if k != "switch_decisions"},
            "representative_reference_status_counts": dict(statuses),
            "representative_phase_counts": dict(phases),
            "representative_guard_coordinates": coordinates,
        }
    return {
        "run": name,
        "receipt": str((RUNS / (name + ".receipt.json")).relative_to(PKG)),
        "stdout": str(path.relative_to(PKG)),
        "exit_code": receipt["exit_code"],
        "measured_binary": receipt["measured_binary"],
        "source_sha256": {p: receipt["source_sha256"][p] for p in OWN_SOURCE},
        "source_still_matches_at_analysis": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == receipt["source_sha256"][p] for p in OWN_SOURCE},
        "sample_count": len(samples),
        "success_count": sum(r["result"]["success"] for r in samples),
        "max_abs_error": max(r["result"]["errors"]["max_abs_error"] for r in samples),
        "header": header,
        "modes": modes,
    }


def main():
    case_ids = ["few-mode", "expensive-few-mode", "broad", "riccati", "nonnormal", "oscillatory",
                "changing-rate", "few-mode-basis1", "nonlinear-changing-rank", "nonlinear-step-regime"]
    cases = [read_campaign("n3-final-" + case) for case in case_ids]
    for case in cases:
        modes = case["modes"]
        guard = modes["guarded-switching-causal"]["median_seconds"]
        protected = min((m for m in modes if m.startswith("protected-")), key=lambda m: modes[m]["median_seconds"])
        case["guard_comparison"] = {
            "definition": "comparator median / guarded median; above one favors guard",
            "priced_transfer_speedup": modes["switching-causal"]["median_seconds"] / guard,
            "fixed_causal_speedup": modes["fixed-causal"]["median_seconds"] / guard,
            "legacy_unpriced_speedup": modes["legacy-switching-causal"]["median_seconds"] / guard,
            "strongest_protected": protected,
            "strongest_protected_speedup": modes[protected]["median_seconds"] / guard,
        }
    n3 = {
        "schema": "vigilode.n3-local-cost-reference.v1",
        "status": "OPT_IN_VALIDITY_GUARD_NO_GENERAL_SPEED_PROMOTION",
        "default_reference_guard": None,
        "total_timed_samples": sum(c["sample_count"] for c in cases),
        "total_successful_samples": sum(c["success_count"] for c in cases),
        "max_abs_error": max(c["max_abs_error"] for c in cases),
        "error_cap": 1e-5,
        "tests": {"round2_passing_tests": 88, "final_curved_model_test": 1,
                  "receipts": ["evidence/runs/n3-round2-tests.receipt.json", "evidence/runs/n3-final-model-tests.receipt.json"]},
        "limits": ["Three repetitions per mode in one process are descriptive, not independent process confidence intervals.",
                   "Mean work weights are routing proxies; no verified wall-time enclosure or unconditional 25% theorem.",
                   "Guard checks locality, not a Lipschitz or stiffness certificate; calibration can reset cooldown.",
                   "All native trajectories use analytical JVP callbacks; this experiment does not validate a fully derivative-free speed claim.",
                   "Reference epoch None cannot report unannounced model mutation.",
                   "Dense/band comparators are much faster at these small registered dimensions."],
        "cases": cases,
    }
    n5_cases = [read_campaign("n5-final-meanfield-" + str(t)) for t in [1, 2]]
    comparison = {}
    for mode in n5_cases[0]["modes"]:
        one, two = (c["modes"][mode] for c in n5_cases)
        comparison[mode] = {
            "same_route_one_over_two_thread_median": one["median_seconds"] / two["median_seconds"],
            "equal_work_counters": one["representative_counters"] == two["representative_counters"],
            "single_forward_over_two_thread_median": n5_cases[0]["modes"]["fixed-causal"]["median_seconds"] / two["median_seconds"],
            "single_dense_over_two_thread_median": n5_cases[0]["modes"]["protected-dense"]["median_seconds"] / two["median_seconds"],
        }
    n5 = {
        "schema": "vigilode.n5-expensive-callback-parallelism.v1",
        "status": "SPECIALIZED_STAGE_BATCH_SPEEDUP_NO_END_TO_END_PROMOTION",
        "total_timed_samples": sum(c["sample_count"] for c in n5_cases),
        "total_successful_samples": sum(c["success_count"] for c in n5_cases),
        "max_abs_error": max(c["max_abs_error"] for c in n5_cases),
        "error_cap": 1e-5,
        "model": "65-state autonomous augmentation of nonlinear nonseparable weighted all-pair phase coupling with curved manufactured clock forcing",
        "callback_cost": "Actual nonlinear interaction and manufactured forcing evaluated inside every RHS; no burn loop.",
        "comparison": comparison,
        "limits": ["One process per thread count, three cyclic repetitions; process-order and host scheduling effects not eliminated.",
                   "Pool construction is outside solve timer and reported separately.",
                   "JVP is analytical; no fully derivative-free claim.",
                   "This exercises existing stage batches, not proof of useful homotopy subdivision or faster sequential time stepping.",
                   "No new parallel dispatcher was installed; local hardware sweep remains required.",
                   "All methods use identical physical observation times and error cap; errors are below the cap, not matched exactly."],
        "campaigns": n5_cases,
    }
    for name, data in [("N3_REGIME_COST.json", n3), ("N5_PARALLEL_CALLBACKS.json", n5)]:
        (OUT / name).write_text(json.dumps(data, indent=2, allow_nan=False) + "\n")
    tables = ["<!-- Generated by src/analyze_n3_n5.py; median of three cyclic samples. -->\n",
              "| Case | Guard (ms) | Priced transfer / guard | Fixed causal / guard | Best protected / guard |", "|---|---:|---:|---:|---:|"]
    for ident, case in zip(case_ids, cases):
        c = case["guard_comparison"]
        tables.append(f"| {ident} | {1e3*case['modes']['guarded-switching-causal']['median_seconds']:.3f} | {c['priced_transfer_speedup']:.3f} | {c['fixed_causal_speedup']:.3f} | {c['strongest_protected_speedup']:.3f} ({c['strongest_protected']}) |")
    tables += ["", "| Meanfield route | 1 worker (ms) | 2 workers (ms) | Same route speedup | RHS / JVP |", "|---|---:|---:|---:|---:|"]
    for mode, comp in comparison.items():
        one, two = (c["modes"][mode] for c in n5_cases)
        count = one["representative_counters"]
        tables.append(f"| {mode} | {1e3*one['median_seconds']:.3f} | {1e3*two['median_seconds']:.3f} | {comp['same_route_one_over_two_thread_median']:.3f} | {count['rhs_calls']} / {count['jvp_calls']} |")
    (OUT / "N3_N5_TIMING_TABLES.md").write_text("\n".join(tables) + "\n")
    print(json.dumps({"N3": [n3['total_successful_samples'], n3['total_timed_samples']], "N5": [n5['total_successful_samples'], n5['total_timed_samples']], "max_errors": [n3['max_abs_error'], n5['max_abs_error']]}))


if __name__ == "__main__":
    main()
