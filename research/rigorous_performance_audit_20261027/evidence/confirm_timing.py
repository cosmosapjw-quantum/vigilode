"""Single prespecified long-batch confirmation; retain wall and process CPU."""
from pathlib import Path
import hashlib
import json
import math
import os
import resource
import statistics
import subprocess
import time
from run_benchmarks import signature

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
WORK = REPO.parent
OUT = HERE / "confirmation"
BLOCKS = 5
TARGET = 1.0
LIMIT = 210.0


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def summarize(samples, field):
    ratios = []
    for b in range(BLOCKS):
        a = [r[field] for r in samples if r["block"] == b and not r["optimized"]]
        c = [r[field] for r in samples if r["block"] == b and r["optimized"]]
        ratios.append(statistics.mean(a) / statistics.mean(c))
    return {"median_ratio": statistics.median(ratios), "block_ratios": ratios,
            "min_ratio": min(ratios), "max_ratio": max(ratios),
            "all_blocks_favor_candidate": min(ratios) > 1,
            "baseline_median": statistics.median(r[field] for r in samples if not r["optimized"]),
            "candidate_median": statistics.median(r[field] for r in samples if r["optimized"])}


def main():
    OUT.mkdir(exist_ok=True)
    if (OUT/"SUMMARY.json").exists():
        raise RuntimeError("refusing to overwrite a completed confirmation")
    initial = json.loads((HERE/"benchmarks/SUMMARY.json").read_text())
    binaries = {"lu": WORK/"tmp/bin_candidate/lu_workspace_bench",
                "dense_baseline": WORK/"tmp/bin_baseline/mode_dense_trajectory_20261027",
                "dense_candidate": WORK/"tmp/bin_candidate/mode_dense_trajectory_20261027",
                "adapter": WORK/"tmp/bin_candidate/audit_adapter_20261027"}
    write(OUT/"BINARIES.json", {k: {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                               for k, p in binaries.items()})
    env = dict(os.environ, RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
    start = time.monotonic()
    receipts = []
    rows = []

    def command(kind, n, opt, reps):
        if kind == "lu":
            return [binaries["lu"], "workspace" if opt else "legacy", n, reps]
        if kind == "dense":
            return [binaries["dense_candidate" if opt else "dense_baseline"], n, reps]
        return [binaries["adapter"], .4, 1, 9999, "1e-4", 0, 0, 8, n,
                "optimized" if opt else "baseline", reps]

    def run(name, kind, n, opt, reps):
        rem = LIMIT - (time.monotonic()-start)
        if rem <= 1:
            raise RuntimeError("confirmation budget exhausted")
        cmd = ["taskset", "-c", "2"] + list(map(str, command(kind,n,opt,reps)))
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        t0 = time.monotonic()
        p = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True,
                           timeout=min(30, rem))
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu = after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime
        receipt = {"name": name, "command": cmd, "exit": p.returncode,
                   "process_wall_seconds": time.monotonic()-t0,
                   "process_cpu_seconds": cpu}
        receipts.append(receipt)
        write(OUT/"COMMANDS.json", receipts)
        (OUT/f"{name}.stdout.json").write_text(p.stdout)
        (OUT/f"{name}.stderr.txt").write_text(p.stderr)
        if p.returncode:
            raise RuntimeError(f"{name} failed")
        obj = json.loads(p.stdout)
        if kind == "lu":
            valid = obj["legacy_bitwise_equal"] and obj["max_error"] <= 2e-13
        elif kind == "dense":
            ref = next(r["signature"] for r in initial["dense"] if r["n"] == n)
            valid = signature(obj) == ref and obj["max_grid_error"] <= obj["cap"]
        else:
            valid = obj["accepted_trajectory_equal"] and obj["deterministic"]
            valid &= all(obj[a][k] for a in ["baseline", "optimized"]
                         for k in ["accuracy_pass", "accounting_complete", "counter_sum_pass"])
        if not valid:
            raise RuntimeError(f"{name}: accuracy/determinism qualification failed")
        return obj, {"optimized": opt, "reps": reps,
                     "internal_wall_per_rep": obj["seconds"]/reps,
                     "process_cpu_per_rep": cpu/reps,
                     "process_wall_per_rep": receipt["process_wall_seconds"]/reps}

    for kind, n, pilot_reps in [("lu",32,10000), ("dense",65,5), ("dense",192,2), ("adapter",16,1000)]:
        pilot, _ = run(f"{kind}_{n}_calibration", kind,n,False,pilot_reps)
        reps = max(1, math.ceil(TARGET*pilot_reps/pilot["seconds"]))
        samples = []
        for block in range(BLOCKS):
            order = [False,True,True,False] if block%2==0 else [True,False,False,True]
            for slot, opt in enumerate(order):
                obj, row = run(f"{kind}_{n}_{block}_{slot}", kind,n,opt,reps)
                row.update(block=block,slot=slot)
                samples.append(row)
        item = {"kind":kind,"n":n,"qualified":True,"reps":reps,"samples":samples,
                "internal_wall":summarize(samples,"internal_wall_per_rep"),
                "process_cpu":summarize(samples,"process_cpu_per_rep"),
                "process_wall":summarize(samples,"process_wall_per_rep")}
        rows.append(item)
        write(OUT/"PARTIAL_SUMMARY.json", {"rows":rows})
        print(f"{kind} n={n}: wall {item['internal_wall']['median_ratio']:.4f}x; CPU {item['process_cpu']['median_ratio']:.4f}x",flush=True)
    result = {"status":"COMPLETE", "rows":rows, "elapsed_seconds":time.monotonic()-start,
              "first_pass_seconds":initial["elapsed_seconds"],
              "cpu_metric_scope":"complete child process user+system CPU; includes setup, diagnostic warmups and serialization",
              "protocol":"CONFIRMATION_PROTOCOL.md; no third pass"}
    write(OUT/"SUMMARY.json", result)
    print(f"COMPLETE {result['elapsed_seconds']:.2f}s",flush=True)


if __name__ == "__main__":
    main()
