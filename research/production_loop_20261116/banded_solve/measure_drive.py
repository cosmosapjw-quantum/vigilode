#!/usr/bin/env python3
"""Exploratory timing of whole runs of the protected banded driver with its own kernels.

usage: measure_drive.py <runner binary> <family> <size> <budget> <index> [--kernels a,b,...]
                        [--tag v]

Writes logs/drive_<tag><index>_<family><size>.jsonl next to this script (header, one line per run,
footer). Every run is `runner drive <family> <size> <kernel> <attempts>` under the timing prefix
of the loop and `perf stat`; each round runs every kernel with 1 attempt and with <budget>
attempts. Same rounds (2 warm-ups + 7) and load records as measure.py. Tag v: the series of
node V21 stage a2 (PREDECLARED_V21.json); tag x: exploratory, decides nothing.
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
WARMUP, ROUNDS = 2, 7
EVENTS = "cycles:u,instructions:u,task-clock"


def loadavg():
    with open("/proc/loadavg") as f:
        return float(f.read().split()[0])


def cpu_jiffies(cpu):
    with open("/proc/stat") as f:
        for line in f:
            parts = line.split()
            if parts[0] == f"cpu{cpu}":
                values = [int(v) for v in parts[1:9]]
                return sum(values) - values[3] - values[4], sum(values)
    raise RuntimeError(f"cpu{cpu} not in /proc/stat")


def one_run(binary, family, size, kernel, attempts):
    cmd = ["flock", LOCK, "taskset", "-c", str(CORE), "perf", "stat", "-x,", "-e", EVENTS]
    with tempfile.NamedTemporaryFile("r", suffix=".perf") as out:
        cmd += ["-o", out.name, "--", binary, "drive", family, str(size), kernel, str(attempts)]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        perf_text = out.read()
    rec = {"kernel": kernel, "budget": attempts, "exit": proc.returncode,
           "stdout": proc.stdout.strip()}
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
    binary, family, size, budget, index = args[0], args[1], int(args[2]), int(args[3]), int(args[4])
    kernels, tag = ["indexed", "slices"], "x"
    rest = args[5:]
    while rest:
        key = rest.pop(0)
        if key == "--kernels":
            kernels = rest.pop(0).split(",")
        elif key == "--tag":
            tag = rest.pop(0)
        else:
            raise SystemExit(f"unknown option {key}")
    path = os.path.join(HERE, "logs", f"drive_{tag}{index}_{family}{size}.jsonl")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        raise SystemExit(f"{path} exists: measurements are never overwritten")
    busy0 = {c: cpu_jiffies(c) for c in (CORE, SIBLING)}
    with open(path, "w") as log:
        def emit(obj):
            log.write(json.dumps(obj, sort_keys=True) + "\n")
            log.flush()
        emit({"type": "header", "family": family, "size": size, "budget": budget, "index": index,
              "kernels": kernels, "warmup": WARMUP, "rounds": ROUNDS, "core": CORE,
              "sibling": SIBLING, "events": EVENTS, "binary": binary,
              "load_before": loadavg(), "time_start": time.time()})
        for rnd in range(WARMUP + ROUNDS):
            for kernel in kernels:
                for attempts in (1, budget):
                    rec = one_run(binary, family, size, kernel, attempts)
                    rec.update({"type": "run", "round": rnd, "warmup": rnd < WARMUP,
                                "load": loadavg()})
                    emit(rec)
        busy1 = {c: cpu_jiffies(c) for c in (CORE, SIBLING)}
        footer = {"type": "footer", "load_after": loadavg(), "time_end": time.time()}
        for c in (CORE, SIBLING):
            total = busy1[c][1] - busy0[c][1]
            footer[f"cpu{c}_busy_fraction"] = (busy1[c][0] - busy0[c][0]) / total if total else None
        emit(footer)
    print(path)


if __name__ == "__main__":
    main()
