#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import os
import random
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BINARY = ROOT / "target" / "release" / "rodas5p"
RESULT_CSV = ROOT / "research" / "homotopy_order_policy_v06" / "results" / "THREAD_SCALING.csv"
RESULT_JSON = ROOT / "research" / "homotopy_order_policy_v06" / "results" / "THREAD_SCALING.json"
THREADS = (1, 2, 4)
CPU_LIST = {1: "0", 2: "0-1", 4: "0-3"}
REPETITIONS = 5
SEED = 20260808

env = os.environ.copy()
for name in [
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
]:
    env[name] = "1"


def run_one(threads: int, phase: str, repetition: int) -> dict[str, object]:
    command = [
        "taskset",
        "-c",
        CPU_LIST[threads],
        str(BINARY),
        "homotopy-order-policy-screen",
        "--profile",
        "canonical",
        "--threads",
        str(threads),
        "--output",
        "/dev/null",
    ]
    print(f"START phase={phase} repetition={repetition} threads={threads}", flush=True)
    start = time.perf_counter()
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    elapsed = time.perf_counter() - start
    print(f"DONE phase={phase} repetition={repetition} threads={threads} elapsed={elapsed:.6f}", flush=True)
    if completed.returncode != 0:
        raise SystemExit(f"command failed with {completed.returncode}: {' '.join(command)}")
    return {
        "phase": phase,
        "repetition": repetition,
        "threads": threads,
        "cpu_list": CPU_LIST[threads],
        "elapsed_seconds": elapsed,
        "returncode": completed.returncode,
    }


rows: list[dict[str, object]] = []
for threads in THREADS:
    rows.append(run_one(threads, "warmup", 0))

rng = random.Random(SEED)
for repetition in range(1, REPETITIONS + 1):
    order = list(THREADS)
    rng.shuffle(order)
    for threads in order:
        rows.append(run_one(threads, "timed", repetition))

RESULT_CSV.parent.mkdir(parents=True, exist_ok=True)
with RESULT_CSV.open("w", newline="", encoding="utf-8") as handle:
    writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
RESULT_JSON.write_text(
    json.dumps(
        {
            "schema": "rodas5p-homotopy-order-policy-thread-scaling-v1",
            "seed": SEED,
            "repetitions": REPETITIONS,
            "rows": rows,
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
print(RESULT_CSV)
print(RESULT_JSON)
