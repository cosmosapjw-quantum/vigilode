#!/usr/bin/env python3
"""Loop node V25: executes the runs, one process per run, and keeps every raw
output under logs/<phase>/. Nothing is judged here; parse_results.py reads
the logs.

usage: python3 -I run_measurements.py PHASE --bin MEASURE_BINARY [options]

phases
  sizing       the sizing pilot of PREDECLARED.json (families.spans_rule): arm S
               alone, entry route, rtol = atol = 1e-7, from the provisional span:
               a run longer than 60 s halves the span and runs again, a run
               shorter than 1 s doubles it (at most three doublings) and runs
               again; otherwise the span is final. Logs labelled sizing.
  setup        per family (final spans of PROTOCOL.json): SETUP x 3 under perf
               and RHS0
  record_a     commit A: arms S and P1, each named explicitly, on the 20 cells:
               route (perf), router, router_ls, audit (with the per-solve log)
  p2           arm P2 (reported only) on the 20 cells: route (perf), audit
  timing       per cell the wall protocol under "flock <lock> taskset -c 6-11"
               (arms S and P1, --try 1, 2 or 3, --cells)
  d0           after an edit of the driver or the router: arms S and P1 through
               the entries of record_a (and --entries) with the new binary
  exploratory  --arms and --entries on the 20 cells

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
LOCK = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock"
CPUS_WORK = "0-5,12-17"
CPUS_TIMING = "6-11"
TIME_LIMIT_SECONDS = 1800
RTOLS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
PROVISIONAL_END = {"ac3d": 1.0, "robertson_rd1d": 40.0, "schnakenberg2d": 1.0, "porous2d": 0.1}
TEST_ARGS = ["--ignored", "--nocapture", "--exact", "v25_run"]
# Sizing pilot (PREDECLARED.json, families.spans_rule).
SIZING_RTOL = "1e-7"
SIZING_MAX_SECONDS = 60.0
SIZING_MIN_SECONDS = 1.0
SIZING_MAX_DOUBLINGS = 3
SIZING_MAX_RUNS = 12
# Timing: no invocation may hold the lock for two minutes.
INVOCATION_BUDGET_SECONDS = 90.0


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


def final_spans():
    with open(os.path.join(NODE, "PROTOCOL.json"), encoding="utf-8") as handle:
        protocol = json.load(handle)
    return {family["id"]: family["t_span"][1] for family in protocol["families"]}


def end_text(value):
    return repr(float(value))


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
            return any(line.startswith("# V25_LOG_END") for line in handle)

    def run(self, name, env, perf=False, timing=False, solvelog=False):
        """One process. Returns the log path."""
        path = os.path.join(self.directory, name + ".log")
        if self.done(name):
            print(f"skip {self.phase}/{name} (exists)", flush=True)
            return path
        if os.path.exists(path):
            os.replace(path, path + f".interrupted.{int(time.time())}")
        command = [self.binary, *TEST_ARGS]
        perf_path = os.path.join(self.directory, name + ".perf")
        solve_path = os.path.join(self.directory, name + ".solves.txt")
        env = dict(env)
        if solvelog:
            env["V25_SOLVELOG"] = solve_path
        if timing:
            inner = ('printf "V25_LOAD_BEFORE %s\\n" "$(cat /proc/loadavg)"; "$0" "$@"; rc=$?; '
                     'printf "V25_LOAD_AFTER %s\\n" "$(cat /proc/loadavg)"; exit $rc')
            command = ["flock", LOCK, "taskset", "-c", CPUS_TIMING, "sh", "-c", inner, *command]
        else:
            if perf:
                command = ["perf", "stat", "-x", ",", "-e", "instructions:u", "-o", perf_path, "--", *command]
            command = ["taskset", "-c", CPUS_WORK, *command]
        full_env = dict(os.environ)
        for key in [key for key in full_env if key.startswith(("V25_", "V24_", "V17_", "V06_"))]:
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
            log.write("# V25_LOG_BEGIN " + json.dumps(header) + "\n")
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
            if solvelog and os.path.exists(solve_path):
                # The per-solve log, compressed reproducibly (no name, no time stamp).
                with open(solve_path, "rb") as source, open(solve_path + ".gz", "wb") as target:
                    with gzip.GzipFile(filename="", mode="wb", fileobj=target, compresslevel=9, mtime=0) as packed:
                        shutil.copyfileobj(source, packed)
                footer["solvelog"] = {"file": os.path.basename(solve_path) + ".gz",
                                      "raw_sha256": sha256_file(solve_path),
                                      "gz_sha256": sha256_file(solve_path + ".gz")}
                os.remove(solve_path)
            log.write("# V25_LOG_END " + json.dumps(footer) + "\n")
        print(f"ran  {self.phase}/{name}: exit {code} in {time.time() - start:.1f} s", flush=True)
        return path

    def run_many(self, jobs, workers):
        """jobs: [(name, env, perf, solvelog)], `workers` at a time (never timing runs)."""
        if workers <= 1:
            for name, env, perf, solvelog in jobs:
                self.run(name, env, perf=perf, solvelog=solvelog)
            return
        with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
            futures = [pool.submit(self.run, name, env, perf, False, solvelog) for name, env, perf, solvelog in jobs]
            for future in concurrent.futures.as_completed(futures):
                future.result()


def read_run(path):
    """The V25_RUN_JSON record of a log, or None."""
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("V25_RUN_JSON "):
                return json.loads(line.split(" ", 1)[1])
    return None


def phase_sizing(runner, families):
    for family in families:
        end = PROVISIONAL_END[family]
        doublings = 0
        halved = False
        for index in range(SIZING_MAX_RUNS):
            name = f"{family}__T{end_text(end)}__S__route"
            env = {"V25_FAMILY": family, "V25_T1": end_text(end), "V25_RTOL": SIZING_RTOL, "V25_MODE": "RUN",
                   "V25_ARM": "S", "V25_ENTRY": "route", "V25_LABEL": f"sizing pilot run {index}"}
            record = read_run(runner.run(name, env))
            wall = (record or {}).get("wall_seconds_diagnostic_only")
            ok = bool(record and record.get("completed") and record.get("success"))
            print(f"sizing {family}: end {end} wall {wall} completed {ok}", flush=True)
            if wall is None:
                print(f"sizing {family}: no wall; stopped", flush=True)
                break
            if wall > SIZING_MAX_SECONDS:
                end = end / 2.0
                halved = True
                continue
            if wall < SIZING_MIN_SECONDS and doublings < SIZING_MAX_DOUBLINGS and not halved:
                end = end * 2.0
                doublings += 1
                continue
            break
        with open(os.path.join(runner.directory, f"{family}__FINAL.txt"), "w", encoding="utf-8") as handle:
            handle.write(json.dumps({"family": family, "final_end": end, "doublings": doublings, "halved": halved}) + "\n")


def setup_jobs(families, spans, label):
    jobs = []
    for family in families:
        base = {"V25_FAMILY": family, "V25_T1": end_text(spans[family]), "V25_LABEL": label}
        for repeat in (1, 2, 3):
            jobs.append((f"{family}__SETUP__r{repeat}", {**base, "V25_MODE": "SETUP"}, True, False))
        jobs.append((f"{family}__RHS0", {**base, "V25_MODE": "RHS0"}, False, False))
    return jobs


def cell_jobs(families, spans, rtols, arm, entries, label):
    """entries: [(entry, perf)]; the audit entry writes the per-solve log."""
    jobs = []
    for family in families:
        for rtol in rtols:
            base = {"V25_FAMILY": family, "V25_T1": end_text(spans[family]), "V25_RTOL": rtol, "V25_MODE": "RUN",
                    "V25_ARM": arm, "V25_LABEL": label}
            for entry, perf in entries:
                jobs.append((f"{family}__{rtol}__{arm}__{entry}", {**base, "V25_ENTRY": entry}, perf, entry == "audit"))
    return jobs


def phase_timing(runner, cells, spans, attempt):
    for family, rtol in cells:
        walls = []
        for name in (f"{family}__{rtol}__S__route", f"{family}__{rtol}__P1__route"):
            record = read_run(os.path.join(LOGS, "record_a", name + ".log"))
            if record is None or "wall_seconds_diagnostic_only" not in record:
                walls = None
                break
            walls.append(record["wall_seconds_diagnostic_only"])
        if walls is None:
            print(f"timing {family} {rtol}: the instruction runs are missing; skipped", flush=True)
            continue
        base = {"V25_FAMILY": family, "V25_T1": end_text(spans[family]), "V25_RTOL": rtol, "V25_MODE": "TIMING",
                "V25_ARMS": "S,P1", "V25_LABEL": f"timing try {attempt}"}
        if 9 * sum(walls) <= INVOCATION_BUDGET_SECONDS:
            runner.run(f"{family}__{rtol}__try{attempt}", {**base, "V25_ROUNDS": "9"}, timing=True)
        elif sum(walls) <= INVOCATION_BUDGET_SECONDS:
            # Long cell: nine invocations of one round each, same rotation.
            for round_index in range(9):
                runner.run(f"{family}__{rtol}__try{attempt}__round{round_index}",
                           {**base, "V25_ROUNDS": "1", "V25_ROUND_OFFSET": str(round_index)}, timing=True)
        else:
            # Very long cell: one run per invocation, in the order of the rotation.
            for round_index in range(9):
                order = ["S", "P1"] if round_index % 2 == 0 else ["P1", "S"]
                for position, arm in enumerate(order):
                    runner.run(f"{family}__{rtol}__try{attempt}__round{round_index}__pos{position}",
                               {**base, "V25_ARMS": arm, "V25_ROUNDS": "1", "V25_ROUND_OFFSET": str(round_index),
                                "V25_POSITION": str(position)}, timing=True)


def wait_for_quiet(minutes, threshold=4.0):
    """Poll once per minute, at most `minutes` times, for a 1-minute load of at
    most `threshold` and no running compiler (orchestrator note of 2026-11-24)."""
    path = os.path.join(LOGS, "timing", "LOAD_WAIT.log")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    waited = 0
    with open(path, "a", encoding="utf-8") as log:
        while True:
            load = loadavg()
            compilers = subprocess.run(["pgrep", "-c", "-x", "rustc"], capture_output=True, text=True, check=False).stdout.strip() or "0"
            log.write(f"V25_LOAD_POLL {time.strftime('%Y-%m-%dT%H:%M:%S%z')} waited_minutes={waited} loadavg={load} rustc={compilers}\n")
            log.flush()
            if (float(load.split()[0]) <= threshold and int(compilers) == 0) or waited >= minutes:
                break
            time.sleep(60)
            waited += 1
    return waited


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["sizing", "setup", "record_a", "p2", "timing", "d0", "exploratory"])
    parser.add_argument("--bin", required=True, help="the mf_confirm_measure test binary")
    parser.add_argument("--families", default="")
    parser.add_argument("--rtols", default=",".join(RTOLS))
    parser.add_argument("--arms", default="", help="exploratory, d0: arm labels")
    parser.add_argument("--entries", default="", help="exploratory, d0: entry labels")
    parser.add_argument("--cells", default="", help="timing: family:rtol,... (default: all cells)")
    parser.add_argument("--try", dest="attempt", type=int, default=1)
    parser.add_argument("--wait-minutes", type=int, default=0, help="timing: poll for a quiet host first")
    parser.add_argument("--jobs", type=int, default=1, help="processes at a time (never for timing)")
    args = parser.parse_args()

    rtols = [r for r in args.rtols.split(",") if r]
    families = [f for f in args.families.split(",") if f] or FAMILIES
    runner = Runner(args.phase, args.bin)
    if args.phase == "sizing":
        phase_sizing(runner, families)
        return 0
    spans = final_spans()
    if args.phase == "setup":
        runner.run_many(setup_jobs(families, spans, "setup"), args.jobs)
    elif args.phase == "record_a":
        entries = [("route", True), ("router", False), ("router_ls", False), ("audit", False)]
        for arm in ("S", "P1"):
            runner.run_many(cell_jobs(families, spans, rtols, arm, entries, "commit A"), args.jobs)
    elif args.phase == "p2":
        runner.run_many(cell_jobs(families, spans, rtols, "P2", [("route", True), ("audit", False)], "P2 reported only"),
                        args.jobs)
    elif args.phase == "timing":
        if args.cells:
            cells = [tuple(cell.split(":")) for cell in args.cells.split(",")]
        else:
            cells = [(family, rtol) for family in families for rtol in rtols]
        if args.wait_minutes:
            waited = wait_for_quiet(args.wait_minutes)
            print(f"waited {waited} minutes for a quiet host; loadavg {loadavg()}", flush=True)
        phase_timing(runner, cells, spans, args.attempt)
    elif args.phase == "d0":
        names = [e for e in args.entries.split(",") if e] or ["route", "router", "router_ls", "audit"]
        entries = [(entry, entry == "route") for entry in names]
        for arm in [a for a in (args.arms or "S,P1").split(",") if a]:
            runner.run_many(cell_jobs(families, spans, rtols, arm, entries, "after the edit"), args.jobs)
    elif args.phase == "exploratory":
        names = [e for e in args.entries.split(",") if e] or ["route", "audit"]
        entries = [(entry, entry == "route") for entry in names]
        for arm in [a for a in args.arms.split(",") if a]:
            runner.run_many(cell_jobs(families, spans, rtols, arm, entries, "exploratory"), args.jobs)
    return 0


if __name__ == "__main__":
    sys.exit(main())
