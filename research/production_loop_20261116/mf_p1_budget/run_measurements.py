#!/usr/bin/env python3
"""Loop node V28: executes the runs, one process per run, and keeps every raw
output under logs/<phase>/. Nothing is judged here; parse_results.py reads
the logs.

usage: python3 -I run_measurements.py PHASE --bin MEASURE_BINARY [options]

phases (cell indices and the binary are those of PROTOCOL.json)
  cells         one CELLS process: the table of the 40 cells as the target builds them
  replay        K and V: per cell the SHADOW runs of arms S and P1 (unperturbed start); every
                stage system of the run solved again by the shipped and the P1 kernel; the
                per-system log is kept gzip-compressed next to the log
  ensemble      T: per cell and j = 1..8 the RUN runs of arms S and P1 from the same perturbed start
  reproduction  the RUN runs of arms S and P1 from the unperturbed start (j = 0), compared by the
                parser with the recordings of V25 and V24 (reported, decides nothing)

options
  --cells 0-39 | 0,5,7-9   the cells (default: all 40)
  --arms S,P1              the arms (default: both)
  --js 1-8                 ensemble: the trajectories (default: 1-8)
  --jobs N                 processes at a time inside the work partition (default 4)

A log that already exists with its end marker is not repeated (a run is never
overwritten; an interrupted log is kept under another name).
"""
import argparse
import concurrent.futures
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
LOGS = os.path.join(NODE, "logs")
CPUS_WORK = "0-5,12-17"
TIME_LIMIT_SECONDS = 1800
TEST_ARGS = ["--ignored", "--nocapture", "--exact", "v28_run"]
ARMS = ("S", "P1")
TRAJECTORIES = range(1, 9)


def loadavg():
    with open("/proc/loadavg", encoding="ascii") as handle:
        return handle.read().strip()


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=False).stdout.strip()


def protocol():
    with open(os.path.join(NODE, "PROTOCOL.json"), encoding="utf-8") as handle:
        return json.load(handle)


def cell_name(cell):
    return f"c{cell['index']:02d}__{cell['family']}__{cell['rtol_label']}"


def parse_indices(text, count):
    if not text:
        return list(range(count))
    out = []
    for part in text.split(","):
        if "-" in part:
            low, high = part.split("-")
            out.extend(range(int(low), int(high) + 1))
        elif part:
            out.append(int(part))
    return out


class Runner:
    def __init__(self, phase, binary):
        self.phase = phase
        self.binary = binary
        self.directory = os.path.join(LOGS, phase)
        os.makedirs(self.directory, exist_ok=True)
        self.identity = {
            "binary": binary,
            "binary_sha256": sha256_file(binary),
            "git_head": git("rev-parse", "HEAD"),
            "git_dirty_tracked": len([line for line in git("status", "--porcelain", "--untracked-files=no").splitlines() if line]),
        }

    def done(self, name):
        path = os.path.join(self.directory, name + ".log")
        if not os.path.exists(path):
            return False
        with open(path, encoding="utf-8", errors="replace") as handle:
            return any(line.startswith("# V28_LOG_END") for line in handle)

    def run(self, name, env, systemlog=False):
        """One process. Returns the log path."""
        path = os.path.join(self.directory, name + ".log")
        if self.done(name):
            print(f"skip {self.phase}/{name} (exists)", flush=True)
            return path
        if os.path.exists(path):
            os.replace(path, path + f".interrupted.{int(time.time())}")
        system_path = os.path.join(self.directory, name + ".systems.txt")
        env = dict(env)
        if systemlog:
            env["V28_SYSTEMLOG"] = system_path
        command = ["taskset", "-c", CPUS_WORK, self.binary, *TEST_ARGS]
        full_env = dict(os.environ)
        for key in [key for key in full_env if key.startswith(("V28_", "V25_", "V24_", "V17_", "V06_"))]:
            del full_env[key]
        full_env.update(env)
        header = dict(self.identity)
        header.update({
            "phase": self.phase, "name": name, "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "env": env, "command": command, "cpus": CPUS_WORK, "loadavg_before_launch": loadavg(),
        })
        start = time.time()
        timed_out = False
        with open(path, "w", encoding="utf-8") as log:
            log.write("# V28_LOG_BEGIN " + json.dumps(header) + "\n")
            log.flush()
            try:
                completed = subprocess.run(command, env=full_env, stdout=log, stderr=subprocess.STDOUT,
                                           timeout=TIME_LIMIT_SECONDS, check=False)
                code = completed.returncode
            except subprocess.TimeoutExpired:
                timed_out = True
                code = -1
            footer = {"exit_code": code, "timed_out": timed_out, "seconds": time.time() - start,
                      "loadavg_after_exit": loadavg(), "date": time.strftime("%Y-%m-%dT%H:%M:%S%z")}
            if systemlog and os.path.exists(system_path):
                # The per-system log, compressed reproducibly (no name, no time stamp).
                with open(system_path, "rb") as source, open(system_path + ".gz", "wb") as target:
                    with gzip.GzipFile(filename="", mode="wb", fileobj=target, compresslevel=9, mtime=0) as packed:
                        shutil.copyfileobj(source, packed)
                footer["systemlog"] = {"file": os.path.basename(system_path) + ".gz",
                                       "raw_sha256": sha256_file(system_path),
                                       "gz_sha256": sha256_file(system_path + ".gz")}
                os.remove(system_path)
            log.write("# V28_LOG_END " + json.dumps(footer) + "\n")
        print(f"ran  {self.phase}/{name}: exit {code} in {time.time() - start:.1f} s", flush=True)
        return path

    def run_many(self, jobs, workers):
        """jobs: [(name, env, systemlog)], `workers` at a time."""
        if workers <= 1:
            for name, env, systemlog in jobs:
                self.run(name, env, systemlog)
            return
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
            futures = [pool.submit(self.run, name, env, systemlog) for name, env, systemlog in jobs]
            for future in concurrent.futures.as_completed(futures):
                future.result()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["cells", "replay", "ensemble", "reproduction"])
    parser.add_argument("--bin", required=True, help="the mf_budget_measure test binary of PROTOCOL.json")
    parser.add_argument("--cells", default="")
    parser.add_argument("--arms", default=",".join(ARMS))
    parser.add_argument("--js", default="1-8")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()

    declared = protocol()
    if sha256_file(args.bin) != declared["binary"]["sha256"]:
        print("the binary is not the one of PROTOCOL.json", file=sys.stderr)
        return 2
    cells = {cell["index"]: cell for cell in declared["cells"]}
    indices = parse_indices(args.cells, len(cells))
    arms = [arm for arm in args.arms.split(",") if arm]
    runner = Runner(args.phase, args.bin)
    if args.phase == "cells":
        runner.run("cells", {"V28_MODE": "CELLS", "V28_LABEL": "cell table"})
        return 0
    jobs = []
    for index in indices:
        cell = cells[index]
        base = {"V28_CELL": str(index)}
        if args.phase == "replay":
            for arm in arms:
                jobs.append((f"{cell_name(cell)}__{arm}",
                             {**base, "V28_MODE": "SHADOW", "V28_ARM": arm, "V28_LABEL": "replay K V"}, True))
        elif args.phase == "ensemble":
            for j in parse_indices(args.js, 9):
                if j not in TRAJECTORIES:
                    raise SystemExit(f"j = {j} is not a perturbed trajectory")
                for arm in arms:
                    jobs.append((f"{cell_name(cell)}__j{j}__{arm}",
                                 {**base, "V28_MODE": "RUN", "V28_ARM": arm, "V28_J": str(j),
                                  "V28_LABEL": "ensemble T"}, False))
        elif args.phase == "reproduction":
            for arm in arms:
                jobs.append((f"{cell_name(cell)}__j0__{arm}",
                             {**base, "V28_MODE": "RUN", "V28_ARM": arm, "V28_J": "0",
                              "V28_LABEL": "reproduction j = 0"}, False))
    runner.run_many(jobs, args.jobs)
    return 0


if __name__ == "__main__":
    sys.exit(main())
