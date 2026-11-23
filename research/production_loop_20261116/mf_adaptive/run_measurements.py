#!/usr/bin/env python3
"""Loop node V24: executes the runs of PROTOCOL.json, one process per run, and
keeps every raw output under logs/<phase>/. Nothing is judged here;
parse_results.py reads the logs.

usage: python3 -I run_measurements.py PHASE --bin MEASURE_BINARY [options]

phases
  record_a     commit A, the driver before the edit: per family SETUP x 3
               (perf); per cell (20 deciding + 10 development) arm S through
               the entries route (perf), route_ls and audit
  setup        per family: SETUP x 3 (perf) and RHS0, with the binary of the runs
  d0           after the edit: arm S through route (perf), route_ls, audit and
               policy_shipped on the 30 cells (also the arm S of every comparison)
  dev          P1 and P2 on the 10 development cells: audit and route (perf)
  runs         arm N (--arm P1 or P2) on the 20 deciding cells: audit and route (perf)
  timing       per deciding cell the wall protocol under
               "flock <lock> taskset -c 6-11" (--arm, --try 1 or 2, --cells)
  exploratory  --arms A,B,...: the named arms on the deciding grid: audit and
               route (perf)

A log that already exists with its end marker is not repeated (a run is never
overwritten; an interrupted log is kept under another name).
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
import time

NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
LOGS = os.path.join(NODE, "logs")
LOCK = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock"
CPUS_WORK = "0-5,12-17"
CPUS_TIMING = "6-11"
TIME_LIMIT_SECONDS = 1800
SPLIT_SECONDS = 8.0
RTOLS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
DECIDING = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"]
DEVELOPMENT = ["dev06_bruss1d", "dev06_adr2d"]
TEST_ARGS = ["--ignored", "--nocapture", "--exact", "v24_run"]


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
            return any(line.startswith("# V24_LOG_END") for line in handle)

    def run(self, name, env, perf=False, timing=False):
        """One process. Returns the log path."""
        path = os.path.join(self.directory, name + ".log")
        if self.done(name):
            print(f"skip {self.phase}/{name} (exists)", flush=True)
            return path
        if os.path.exists(path):
            # An interrupted run: keep it under another name.
            os.replace(path, path + f".interrupted.{int(time.time())}")
        command = [self.binary, *TEST_ARGS]
        perf_path = os.path.join(self.directory, name + ".perf")
        if timing:
            inner = ('printf "V24_LOAD_BEFORE %s\\n" "$(cat /proc/loadavg)"; "$0" "$@"; rc=$?; '
                     'printf "V24_LOAD_AFTER %s\\n" "$(cat /proc/loadavg)"; exit $rc')
            command = ["flock", LOCK, "taskset", "-c", CPUS_TIMING, "sh", "-c", inner, *command]
        else:
            if perf:
                command = ["perf", "stat", "-x", ",", "-e", "instructions:u", "-o", perf_path, "--", *command]
            command = ["taskset", "-c", CPUS_WORK, *command]
        full_env = dict(os.environ)
        for key in [key for key in full_env if key.startswith(("V24_", "V17_", "V06_"))]:
            del full_env[key]
        full_env.update(env)
        header = dict(self.identity)
        header.update({
            "phase": self.phase, "name": name, "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "env": env, "perf": perf, "timing": timing, "command": command,
            "cpus": CPUS_TIMING if timing else CPUS_WORK, "loadavg_before_launch": loadavg(),
        })
        start = time.time()
        timed_out = False
        with open(path, "w", encoding="utf-8") as log:
            log.write("# V24_LOG_BEGIN " + json.dumps(header) + "\n")
            log.flush()
            try:
                completed = subprocess.run(command, env=full_env, stdout=log, stderr=subprocess.STDOUT,
                                           timeout=TIME_LIMIT_SECONDS, check=False)
                code = completed.returncode
            except subprocess.TimeoutExpired:
                timed_out = True
                code = -1
            log.write("# V24_LOG_END " + json.dumps({
                "exit_code": code, "timed_out": timed_out, "seconds": time.time() - start,
                "loadavg_after_exit": loadavg(), "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            }) + "\n")
        print(f"ran  {self.phase}/{name}: exit {code} in {time.time() - start:.1f} s", flush=True)
        return path

    def run_many(self, jobs, workers):
        """jobs: [(name, env, perf)], run `workers` at a time (never timing runs)."""
        if workers <= 1:
            for name, env, perf in jobs:
                self.run(name, env, perf=perf)
            return
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
            futures = [pool.submit(self.run, name, env, perf) for name, env, perf in jobs]
            for future in concurrent.futures.as_completed(futures):
                future.result()


def read_run(path):
    """The V24_RUN_JSON record of a log, or None."""
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("V24_RUN_JSON "):
                return json.loads(line.split(" ", 1)[1])
    return None


def setup_jobs(families, label, rhs0):
    jobs = []
    for family in families:
        base = {"V24_FAMILY": family, "V24_LABEL": label}
        for repeat in (1, 2, 3):
            jobs.append((f"{family}__SETUP__r{repeat}", {**base, "V24_MODE": "SETUP"}, True))
        if rhs0:
            jobs.append((f"{family}__RHS0", {**base, "V24_MODE": "RHS0"}, False))
    return jobs


def cell_jobs(families, rtols, arm, entries, label):
    """entries: [(entry, perf)]."""
    jobs = []
    for family in families:
        for rtol in rtols:
            base = {"V24_FAMILY": family, "V24_RTOL": rtol, "V24_MODE": "RUN", "V24_ARM": arm, "V24_LABEL": label}
            for entry, perf in entries:
                jobs.append((f"{family}__{rtol}__{arm}__{entry}", {**base, "V24_ENTRY": entry}, perf))
    return jobs


def phase_timing(runner, cells, arm, attempt):
    for family, rtol in cells:
        walls = []
        for directory, name in (("d0", f"{family}__{rtol}__S__route"), ("runs", f"{family}__{rtol}__{arm}__route")):
            record = read_run(os.path.join(LOGS, directory, name + ".log"))
            if record is None or "wall_seconds_diagnostic_only" not in record:
                walls = None
                break
            walls.append(record["wall_seconds_diagnostic_only"])
        if walls is None:
            print(f"timing {family} {rtol}: the instruction runs are missing; skipped", flush=True)
            continue
        base = {"V24_FAMILY": family, "V24_RTOL": rtol, "V24_MODE": "TIMING", "V24_ARMS": f"S,{arm}",
                "V24_LABEL": f"timing try {attempt}"}
        if sum(walls) > SPLIT_SECONDS:
            # Long cell: nine invocations of one round each, same rotation.
            for round_index in range(9):
                runner.run(f"{family}__{rtol}__try{attempt}__round{round_index}",
                           {**base, "V24_ROUNDS": "1", "V24_ROUND_OFFSET": str(round_index)}, timing=True)
        else:
            runner.run(f"{family}__{rtol}__try{attempt}", {**base, "V24_ROUNDS": "9"}, timing=True)


def wait_for_quiet(minutes, threshold=8.0):
    """Poll the 1-minute load average once per minute, at most `minutes` times."""
    path = os.path.join(LOGS, "timing", "LOAD_WAIT.log")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    waited = 0
    with open(path, "a", encoding="utf-8") as log:
        while True:
            load = loadavg()
            log.write(f"V24_LOAD_POLL {time.strftime('%Y-%m-%dT%H:%M:%S%z')} waited_minutes={waited} loadavg={load}\n")
            log.flush()
            if float(load.split()[0]) <= threshold or waited >= minutes:
                break
            time.sleep(60)
            waited += 1
    return waited


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["record_a", "setup", "d0", "dev", "runs", "timing", "exploratory"])
    parser.add_argument("--bin", required=True, help="the mf_adaptive_measure test binary")
    parser.add_argument("--families", default="")
    parser.add_argument("--rtols", default=",".join(RTOLS))
    parser.add_argument("--arm", default="", help="runs, timing: the adaptive arm (P1 or P2)")
    parser.add_argument("--arms", default="", help="exploratory: arm labels")
    parser.add_argument("--cells", default="", help="timing: family:rtol,... (default: all deciding cells)")
    parser.add_argument("--try", dest="attempt", type=int, default=1)
    parser.add_argument("--wait-minutes", type=int, default=0, help="timing: poll for a quiet host first")
    parser.add_argument("--jobs", type=int, default=1, help="processes at a time (never for timing)")
    args = parser.parse_args()

    rtols = [r for r in args.rtols.split(",") if r]
    chosen = [f for f in args.families.split(",") if f]
    runner = Runner(args.phase, args.bin)
    if args.phase == "record_a":
        families = chosen or DECIDING + DEVELOPMENT
        runner.run_many(setup_jobs(families, "commit A", False), args.jobs)
        runner.run_many(cell_jobs(families, rtols, "S", [("route", True), ("route_ls", False), ("audit", False)],
                                  "commit A"), args.jobs)
    elif args.phase == "setup":
        families = chosen or DECIDING + DEVELOPMENT
        deciding = [f for f in families if f in DECIDING]
        runner.run_many(setup_jobs(deciding, "setup", True) + setup_jobs([f for f in families if f not in DECIDING],
                                                                        "setup", False), args.jobs)
    elif args.phase == "d0":
        families = chosen or DECIDING + DEVELOPMENT
        runner.run_many(cell_jobs(families, rtols, "S", [("route", True), ("route_ls", False), ("audit", False),
                                                         ("policy_shipped", False)], "after the edit"), args.jobs)
    elif args.phase == "dev":
        families = chosen or DEVELOPMENT
        for arm in ("P1", "P2"):
            runner.run_many(cell_jobs(families, rtols, arm, [("audit", False), ("route", True)], "development"), args.jobs)
    elif args.phase == "runs":
        if args.arm not in ("P1", "P2"):
            sys.exit("runs needs --arm P1 or --arm P2 (CHOICE.json)")
        families = chosen or DECIDING
        runner.run_many(cell_jobs(families, rtols, args.arm, [("audit", False), ("route", True)], "declared"), args.jobs)
    elif args.phase == "timing":
        if args.arm not in ("P1", "P2"):
            sys.exit("timing needs --arm P1 or --arm P2 (CHOICE.json)")
        families = chosen or DECIDING
        if args.cells:
            cells = [tuple(cell.split(":")) for cell in args.cells.split(",")]
        else:
            cells = [(family, rtol) for family in families for rtol in rtols]
        if args.wait_minutes:
            waited = wait_for_quiet(args.wait_minutes)
            print(f"waited {waited} minutes for a quiet host; loadavg {loadavg()}", flush=True)
        phase_timing(runner, cells, args.arm, args.attempt)
    elif args.phase == "exploratory":
        families = chosen or DECIDING
        for arm in [a for a in args.arms.split(",") if a]:
            runner.run_many(cell_jobs(families, rtols, arm, [("audit", False), ("route", True)], "exploratory"), args.jobs)


if __name__ == "__main__":
    sys.exit(main())
