#!/usr/bin/env python3
"""Timing harness of loop L1 node V19, stage a. Protocol: PREDECLARED.json, key "protocol".

usage: measure.py <runner binary> <n> <measurement index> [--lu LU] [--arms A,B,...] [--patterns R]

Writes logs/measure_m<index>_n<n>[_lu<LU>].jsonl next to this script: a header line, one line
per run and a footer line. Every run is one process under the timing prefix of the loop
(lock file, one pinned core) and under `perf stat`. Nothing is computed here: build_results.py
derives every reported number from these lines.
"""
import json
import os
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
LOCK = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock"
CORE, SIBLING = 9, 21
ARMS = ["A0", "A1", "B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8", "B2", "B3", "C16", "CV16", "BD"]
PATTERNS = {1000: 20000, 10000: 2000, 100000: 200, 1000000: 20}
WARMUP, ROUNDS = 2, 7
EVENTS = "cycles:u,instructions:u,task-clock"


def loadavg():
    with open("/proc/loadavg") as f:
        return float(f.read().split()[0])


def cpu_jiffies(cpu):
    """(busy, total) jiffies of one logical CPU from /proc/stat."""
    with open("/proc/stat") as f:
        for line in f:
            parts = line.split()
            if parts[0] == f"cpu{cpu}":
                values = [int(v) for v in parts[1:9]]
                total = sum(values)
                idle = values[3] + values[4]
                return total - idle, total
    raise RuntimeError(f"cpu{cpu} not in /proc/stat")


def one_run(binary, arm, n, patterns, solves, lu):
    cmd = ["flock", LOCK, "taskset", "-c", str(CORE), "perf", "stat", "-x,", "-e", EVENTS]
    with tempfile.NamedTemporaryFile("r", suffix=".perf") as out:
        cmd += ["-o", out.name, "--", binary, "bench", arm, str(n), str(patterns), str(solves)]
        if lu is not None:
            cmd.append(str(lu))
        t0 = time.time()
        proc = subprocess.run(cmd, capture_output=True, text=True)
        wall = time.time() - t0
        perf_text = out.read()
    rec = {"arm": arm, "patterns": patterns, "solves": solves, "exit": proc.returncode,
           "wall_s": wall, "stdout": proc.stdout.strip()}
    for line in perf_text.splitlines():
        fields = line.split(",")
        if len(fields) < 3:
            continue
        event = fields[2]
        if event in ("cycles:u", "instructions:u"):
            rec[event.split(":")[0]] = int(fields[0])
        elif event.startswith("task-clock"):
            rec["task_clock_ms"] = float(fields[0])
    return rec


def main():
    args = sys.argv[1:]
    binary, n, index = args[0], int(args[1]), int(args[2])
    lu, arms, patterns = None, list(ARMS), None
    rest = args[3:]
    while rest:
        key = rest.pop(0)
        if key == "--lu":
            lu = int(rest.pop(0))
        elif key == "--arms":
            arms = rest.pop(0).split(",")
        elif key == "--patterns":
            patterns = int(rest.pop(0))
        else:
            raise SystemExit(f"unknown option {key}")
    if lu is not None and lu != 2:
        arms = [a for a in arms if a != "B3"]
    if patterns is None:
        patterns = PATTERNS[n]
    name = f"measure_m{index}_n{n}" + (f"_lu{lu}" if lu is not None else "") + ".jsonl"
    path = os.path.join(HERE, "logs", name)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        raise SystemExit(f"{path} exists: measurements are never overwritten")
    busy0 = {c: cpu_jiffies(c) for c in (CORE, SIBLING)}
    with open(path, "w") as log:
        def emit(obj):
            log.write(json.dumps(obj, sort_keys=True) + "\n")
            log.flush()
        emit({"type": "header", "n": n, "lu": lu, "index": index, "arms": arms,
              "patterns": patterns, "warmup": WARMUP, "rounds": ROUNDS, "core": CORE,
              "sibling": SIBLING, "events": EVENTS, "binary": binary,
              "load_before": loadavg(), "time_start": time.time()})
        for rnd in range(WARMUP + ROUNDS):
            for arm in arms:
                for pat, sol in ((0, 0), (patterns, 0), (patterns, 8)):
                    rec = one_run(binary, arm, n, pat, sol, lu)
                    rec.update({"type": "run", "round": rnd, "warmup": rnd < WARMUP,
                                "load": loadavg()})
                    emit(rec)
        busy1 = {c: cpu_jiffies(c) for c in (CORE, SIBLING)}
        frac = {}
        for c in (CORE, SIBLING):
            total = busy1[c][1] - busy0[c][1]
            frac[f"cpu{c}_busy_fraction"] = (busy1[c][0] - busy0[c][0]) / total if total else None
        footer = {"type": "footer", "load_after": loadavg(), "time_end": time.time()}
        footer.update(frac)
        emit(footer)
    print(path)


if __name__ == "__main__":
    main()
