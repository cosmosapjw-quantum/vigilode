"""Serial, bounded, raw-recorded performance audit. No builds in this script.

Requires the baseline/candidate binaries prepared in tmp/bin_{baseline,candidate}.
All timing arms include their own warmup; each measured batch is run as one
process pinned to one allowed CPU. Raw outputs, qualification and paired block
ratios are saved before any reporting. Shared-host ranges are not confidence
intervals. Run from any cwd with Python 3.
"""
from pathlib import Path
import argparse
import hashlib
import json
import math
import os
import platform
import statistics
import struct
import subprocess
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
WORK = REPO.parent
OUT = HERE / "benchmarks"
TIME_BUDGET = 300.0
BLOCKS = 5
TARGET_BATCH = 0.12


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2, allow_nan=False) + "\n")


def float_bytes(value):
    if isinstance(value, list):
        return b"".join(float_bytes(v) for v in value)
    return struct.pack(">d", float(value))


def signature(row):
    return {"states_sha256": hashlib.sha256(float_bytes(row["states"])).hexdigest(),
            "times_sha256": hashlib.sha256(float_bytes(row["times"])).hexdigest(),
            "counters": row["counters"], "attempts": row["attempts"], "lu": row["lu"]}


def paired(samples, field):
    ratios = []
    for block in range(BLOCKS):
        a = [r[field] for r in samples if r["block"] == block and not r["optimized"]]
        b = [r[field] for r in samples if r["block"] == block and r["optimized"]]
        ratios.append(statistics.mean(a) / statistics.mean(b))
    return {"baseline_over_candidate": statistics.median(ratios),
            "block_ratios": ratios, "min_block_ratio": min(ratios),
            "max_block_ratio": max(ratios),
            "baseline_median_seconds": statistics.median(
                r[field] for r in samples if not r["optimized"]),
            "candidate_median_seconds": statistics.median(
                r[field] for r in samples if r["optimized"])}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cpu", type=int, default=min(os.sched_getaffinity(0)))
    args = ap.parse_args()
    if args.cpu not in os.sched_getaffinity(0):
        raise SystemExit("CPU outside allowed affinity")
    OUT.mkdir(parents=True, exist_ok=True)
    if (OUT / "SUMMARY.json").exists():
        raise SystemExit("refusing to overwrite an existing completed experiment")
    bins = {
        "dense_baseline": WORK / "tmp/bin_baseline/mode_dense_trajectory_20261027",
        "dense_candidate": WORK / "tmp/bin_candidate/mode_dense_trajectory_20261027",
        "lu": WORK / "tmp/bin_candidate/lu_workspace_bench",
        "adapter": WORK / "tmp/bin_candidate/audit_adapter_20261027",
    }
    metadata = {"cpu": args.cpu, "affinity_before": sorted(os.sched_getaffinity(0)),
                "platform": platform.platform(), "processor": platform.processor(),
                "cpuinfo": Path("/proc/cpuinfo").read_text().split("\n\n")[0],
                "cpu_max": Path("/sys/fs/cgroup/cpu.max").read_text().strip(),
                "memory_max": Path("/sys/fs/cgroup/memory.max").read_text().strip(),
                "binaries": {k: {"path": str(v), "sha256": sha(v)} for k, v in bins.items()},
                "blocks": BLOCKS, "target_batch_seconds": TARGET_BATCH,
                "max_wall_seconds": TIME_BUDGET, "concurrent_compilation": False,
                "timed_processes": 1, "clock": "Rust Instant per batch; Python monotonic orchestration",
                "baseline_commit": "3064d453091af1dec7f99f0c8afabd98b447ea34"}
    write(OUT / "ENVIRONMENT.json", metadata)
    started = time.monotonic()
    env = dict(os.environ, RAYON_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1")
    commands = []

    def run(name, command, timeout=90):
        remaining = TIME_BUDGET - (time.monotonic() - started)
        if remaining < 1:
            raise RuntimeError("experiment wall budget exhausted")
        full = ["taskset", "-c", str(args.cpu)] + [str(x) for x in command]
        t0 = time.monotonic()
        p = subprocess.run(full, cwd=REPO, env=env, capture_output=True, text=True,
                           timeout=min(timeout, remaining))
        (OUT / f"{name}.stdout.json").write_text(p.stdout)
        (OUT / f"{name}.stderr.txt").write_text(p.stderr)
        receipt = {"name": name, "command": full, "exit": p.returncode,
                   "elapsed_seconds": time.monotonic() - t0}
        commands.append(receipt)
        write(OUT / "COMMANDS.json", commands)
        if p.returncode:
            raise RuntimeError(f"{name}: nonzero exit; inspect raw receipt")
        return json.loads(p.stdout)

    summary = {"protocol": metadata, "lu": [], "dense": [], "adapter": [], "controls": []}

    for n in [32, 96, 192]:
        pilot = run(f"lu_{n}_calibration", [bins["lu"], "legacy", n, 1000])
        reps = min(10_000_000, max(1000, math.ceil(TARGET_BATCH / pilot["seconds"] * 1000)))
        samples = []
        for block in range(BLOCKS):
            order = [False, True, True, False] if block % 2 == 0 else [True, False, False, True]
            for slot, opt in enumerate(order):
                arm = "workspace" if opt else "legacy"
                row = run(f"lu_{n}_{block}_{slot}_{arm}", [bins["lu"], arm, n, reps])
                if not row["legacy_bitwise_equal"] or row["max_error"] > 2e-13:
                    raise RuntimeError("LU qualification failed")
                row.update(block=block, slot=slot, optimized=opt,
                           seconds_per_solve=row["seconds"] / row["reps"])
                samples.append(row)
        summary["lu"].append({"n": n, "reps": reps, "samples": samples,
                              "qualified": True, **paired(samples, "seconds_per_solve")})
        write(OUT / "PARTIAL_SUMMARY.json", summary)
        print(f"LU n={n}: {summary['lu'][-1]['baseline_over_candidate']:.4f}x", flush=True)

    for n in [65, 96, 192]:
        qa = run(f"dense_{n}_baseline_qualification", [bins["dense_baseline"], n, 1])
        qb = run(f"dense_{n}_candidate_qualification", [bins["dense_candidate"], n, 1])
        sa, sb = signature(qa), signature(qb)
        if sa != sb or any(r["max_grid_error"] > r["cap"] for r in [qa, qb]):
            raise RuntimeError(f"dense n={n}: exact observation/counter qualification failed")
        reps = min(10000, max(1, math.ceil(TARGET_BATCH / qa["seconds"])))
        samples = []
        for block in range(BLOCKS):
            order = [False, True, True, False] if block % 2 == 0 else [True, False, False, True]
            for slot, opt in enumerate(order):
                arm = "candidate" if opt else "baseline"
                row = run(f"dense_{n}_{block}_{slot}_{arm}", [bins[f"dense_{arm}"], n, reps])
                if signature(row) != sa or row["max_grid_error"] > row["cap"]:
                    raise RuntimeError("dense measured sample changed qualification")
                samples.append({"block": block, "slot": slot, "optimized": opt,
                                "reps": row["reps"], "seconds": row["seconds"],
                                "seconds_per_trajectory": row["seconds_per_trajectory"]})
        summary["dense"].append({"n": n, "reps": reps, "qualified": True,
                                 "bitwise_observation_and_counter_equality": True,
                                 "signature": sa, "max_grid_error": qa["max_grid_error"],
                                 "cap": qa["cap"], "samples": samples,
                                 **paired(samples, "seconds_per_trajectory")})
        write(OUT / "PARTIAL_SUMMARY.json", summary)
        print(f"Dense n={n}: {summary['dense'][-1]['baseline_over_candidate']:.4f}x", flush=True)

    for n in [1, 8, 16]:
        row = run(f"adapter_{n}_diagnostic", [bins["adapter"], .4, 1, 9999, "1e-4", 0, 0, 8, n])
        eligible = row["accepted_trajectory_equal"] and row["deterministic"]
        eligible &= all(row[arm][key] for arm in ["baseline", "optimized"]
                        for key in ["accuracy_pass", "accounting_complete", "counter_sum_pass"])
        eligible &= row["optimized"]["lower_bound_returns"] > 0
        if not eligible:
            summary["adapter"].append({"n": n, "qualified": False, "diagnostic": row,
                                       "timing": "NOT_RUN_UNQUALIFIED"})
            continue
        row = run(f"adapter_{n}_timing", [bins["adapter"], .4, 1, 9999, "1e-4", 0, BLOCKS, 8, n])
        if not row["accepted_trajectory_equal"] or not row["deterministic"]:
            raise RuntimeError("adapter timed output changed")
        summary["adapter"].append({"n": n, "qualified": True,
                                   "baseline": row["baseline"], "optimized": row["optimized"],
                                   "accepted_trajectory_equal": True,
                                   "samples": row["samples"],
                                   **paired(row["samples"], "seconds_per_solve")})
        write(OUT / "PARTIAL_SUMMARY.json", summary)
        print(f"Adapter n={n}: {summary['adapter'][-1]['baseline_over_candidate']:.4f}x", flush=True)

    control = run("adapter_sentinel_control", [bins["adapter"], .4, 1, 9999, "1e-4", 1, 0, 8, 1])
    if not control["accepted_trajectory_equal"] or control["baseline"] != control["optimized"]:
        raise RuntimeError("sentinel independence control failed")
    summary["controls"].append({"name": "implicit-first sentinel", "passed": True,
                                "full_state_and_counters_equal": True})
    summary["elapsed_seconds"] = time.monotonic() - started
    summary["status"] = "COMPLETE"
    write(OUT / "SUMMARY.json", summary)
    print(f"COMPLETE elapsed {summary['elapsed_seconds']:.2f}s", flush=True)


if __name__ == "__main__":
    main()
