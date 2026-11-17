#!/usr/bin/env python3
"""Loop node V17: executes the runs of PREDECLARED.json, one process per run,
and keeps every raw output under logs/<phase>/. Nothing is judged here;
parse_results.py reads the logs.

usage: python3 -I run_measurements.py PHASE --bin MEASURE_BINARY [options]

phases
  setup        per family: SETUP x 3 (perf), RHS0, CALIB_JVP and CALIB_RHS (perf)
  runs         per family and tolerance: A, B, C with the residual check on, and
               A, B, C, B0 with it off under perf stat -e instructions:u
  timing       per family and tolerance: the authoritative wall protocol under
               "flock <lock> taskset -c 6-11" (--try 1, 2 or 3; --cells to
               re-measure only some cells)
  exploratory  restart lengths, right Jacobi, kappa, the stiff Brusselator
  regression   V06 cells through the V06 measurement target (--v06-bin)

A log that already exists with its end marker is not repeated (a run is never
overwritten; rerun under another --try or --suffix instead).
"""
import argparse
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
SPLIT_SECONDS = 30.0
RTOLS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
TEST_ARGS = ["--ignored", "--nocapture", "--exact"]


def declared_families():
    with open(os.path.join(NODE, "PREDECLARED.json"), encoding="utf-8") as handle:
        declared = json.load(handle)
    return [entry["id"] for entry in declared["families_final"]["list"]]


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
    def __init__(self, phase, binary, test_name, force=False):
        self.phase = phase
        self.binary = binary
        self.test_name = test_name
        self.force = force
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
            return any(line.startswith("# V17_LOG_END") for line in handle)

    def run(self, name, env, perf=False, timing=False):
        """One process. Returns the log path."""
        path = os.path.join(self.directory, name + ".log")
        if self.done(name) and not self.force:
            print(f"skip {self.phase}/{name} (exists)", flush=True)
            return path
        if os.path.exists(path):
            # An interrupted run: keep it under another name.
            os.replace(path, path + f".interrupted.{int(time.time())}")
        command = [self.binary, *TEST_ARGS, self.test_name]
        perf_path = os.path.join(self.directory, name + ".perf")
        if timing:
            inner = ('printf "V17_LOAD_BEFORE %s\\n" "$(cat /proc/loadavg)"; "$0" "$@"; rc=$?; '
                     'printf "V17_LOAD_AFTER %s\\n" "$(cat /proc/loadavg)"; exit $rc')
            command = ["flock", LOCK, "taskset", "-c", CPUS_TIMING, "sh", "-c", inner, *command]
        else:
            if perf:
                command = ["perf", "stat", "-x", ",", "-e", "instructions:u", "-o", perf_path, "--", *command]
            command = ["taskset", "-c", CPUS_WORK, *command]
        full_env = dict(os.environ)
        for key in [key for key in full_env if key.startswith(("V17_", "V06_"))]:
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
            log.write("# V17_LOG_BEGIN " + json.dumps(header) + "\n")
            log.flush()
            try:
                completed = subprocess.run(command, env=full_env, stdout=log, stderr=subprocess.STDOUT,
                                           timeout=TIME_LIMIT_SECONDS, check=False)
                code = completed.returncode
            except subprocess.TimeoutExpired:
                timed_out = True
                code = -1
            log.write("# V17_LOG_END " + json.dumps({
                "exit_code": code, "timed_out": timed_out, "seconds": time.time() - start,
                "loadavg_after_exit": loadavg(), "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            }) + "\n")
        print(f"ran  {self.phase}/{name}: exit {code} in {time.time() - start:.1f} s", flush=True)
        return path


def read_run(path):
    """The V17_RUN_JSON record of a log, or None."""
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("V17_RUN_JSON "):
                return json.loads(line.split(" ", 1)[1])
    return None


def phase_setup(runner, families):
    for family in families:
        base = {"V17_FAMILY": family, "V17_LABEL": "setup"}
        for repeat in (1, 2, 3):
            runner.run(f"{family}__SETUP__r{repeat}", {**base, "V17_MODE": "SETUP"}, perf=True)
        runner.run(f"{family}__RHS0", {**base, "V17_MODE": "RHS0"})
        runner.run(f"{family}__CALIB_JVP", {**base, "V17_MODE": "CALIB_JVP"}, perf=True)
        runner.run(f"{family}__CALIB_RHS", {**base, "V17_MODE": "CALIB_RHS"}, perf=True)


def phase_runs(runner, families, rtols):
    for family in families:
        for rtol in rtols:
            base = {"V17_FAMILY": family, "V17_RTOL": rtol, "V17_MODE": "RUN", "V17_LABEL": "declared"}
            for arm in ("A", "B", "C"):
                runner.run(f"{family}__{rtol}__{arm}__audit1", {**base, "V17_ARM": arm, "V17_AUDIT": "1"})
                runner.run(f"{family}__{rtol}__{arm}__audit0", {**base, "V17_ARM": arm, "V17_AUDIT": "0"}, perf=True)
            runner.run(f"{family}__{rtol}__B0__audit0", {**base, "V17_ARM": "B0", "V17_AUDIT": "0", "V17_LABEL": "control"}, perf=True)


def phase_timing(runner, cells, attempt):
    runs_directory = os.path.join(LOGS, "runs")
    for family, rtol in cells:
        walls = []
        for arm in ("A", "B", "C"):
            record = read_run(os.path.join(runs_directory, f"{family}__{rtol}__{arm}__audit0.log"))
            if record is None or "wall_seconds_diagnostic_only" not in record:
                walls = None
                break
            walls.append(record["wall_seconds_diagnostic_only"])
        if walls is None:
            print(f"timing {family} {rtol}: the instruction runs are missing; skipped", flush=True)
            continue
        base = {"V17_FAMILY": family, "V17_RTOL": rtol, "V17_MODE": "TIMING", "V17_ARMS": "A,B,C",
                "V17_LABEL": f"timing try {attempt}"}
        if sum(walls) > SPLIT_SECONDS:
            # Long cell: nine invocations of one round each, same rotation.
            for round_index in range(9):
                runner.run(f"{family}__{rtol}__try{attempt}__round{round_index}",
                           {**base, "V17_ROUNDS": "1", "V17_ROUND_OFFSET": str(round_index)}, timing=True)
        else:
            runner.run(f"{family}__{rtol}__try{attempt}", {**base, "V17_ROUNDS": "9"}, timing=True)


def wait_for_quiet(minutes, threshold=8.0):
    """Poll the 1-minute load average once per minute, at most `minutes` times."""
    path = os.path.join(LOGS, "timing", "LOAD_WAIT.log")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    waited = 0
    with open(path, "a", encoding="utf-8") as log:
        while True:
            load = loadavg()
            log.write(f"V17_LOAD_POLL {time.strftime('%Y-%m-%dT%H:%M:%S%z')} waited_minutes={waited} loadavg={load}\n")
            log.flush()
            if float(load.split()[0]) <= threshold or waited >= minutes:
                break
            time.sleep(60)
            waited += 1
    return waited


def phase_exploratory(runner, families, rtols, groups):
    for family in families:
        base = {"V17_FAMILY": family, "V17_MODE": "RUN", "V17_LABEL": "exploratory"}
        if "restart" in groups:
            for arm in ("A", "B"):
                for restart in ("10", "20", "80"):
                    runner.run(f"{family}__1e-5__{arm}__restart{restart}__audit0",
                               {**base, "V17_RTOL": "1e-5", "V17_ARM": arm, "V17_AUDIT": "0", "V17_RESTART": restart}, perf=True)
        if "jacobi" in groups:
            for audit in ("1", "0"):
                runner.run(f"{family}__1e-5__B__jacobi__audit{audit}",
                           {**base, "V17_RTOL": "1e-5", "V17_ARM": "B", "V17_AUDIT": audit, "V17_PRECOND": "jacobi"},
                           perf=audit == "0")
        if "kappa" in groups:
            for kappa in ("1e-4", "1e-2"):
                for rtol in rtols:
                    runner.run(f"{family}__{rtol}__C__kappa{kappa}__audit0",
                               {**base, "V17_RTOL": rtol, "V17_ARM": "C", "V17_AUDIT": "0", "V17_KAPPA": kappa}, perf=True)
    if "stiff" in groups:
        base = {"V17_FAMILY": "bruss1d_stiff600", "V17_MODE": "RUN", "V17_RTOL": "1e-4", "V17_LABEL": "exploratory"}
        for repeat in (1, 2, 3):
            runner.run(f"bruss1d_stiff600__SETUP__r{repeat}",
                       {"V17_FAMILY": "bruss1d_stiff600", "V17_MODE": "SETUP", "V17_LABEL": "exploratory"}, perf=True)
        runner.run("bruss1d_stiff600__RHS0", {"V17_FAMILY": "bruss1d_stiff600", "V17_MODE": "RHS0", "V17_LABEL": "exploratory"})
        for arm in ("A", "B", "C"):
            runner.run(f"bruss1d_stiff600__1e-4__{arm}__audit1", {**base, "V17_ARM": arm, "V17_AUDIT": "1"})
            runner.run(f"bruss1d_stiff600__1e-4__{arm}__audit0", {**base, "V17_ARM": arm, "V17_AUDIT": "0"}, perf=True)


def phase_regression(runner):
    """V06 cells through the V06 measurement target, named as in shared_krylov/logs."""
    for family in ("adr2d", "grayscott2d"):
        for rtol in ("1e-4", "1e-6"):
            base = {"V06_FAMILY": family, "V06_RTOL": rtol, "V06_PRECOND": "none", "V06_LABEL": "v17 regression"}
            runner.run(f"a_{family}_{rtol}_B0_none", {**base, "V06_ARM": "B0"})
            runner.run(f"a_{family}_{rtol}_B_none_audit1", {**base, "V06_ARM": "B", "V06_AUDIT": "1"})
            runner.run(f"a_{family}_{rtol}_B_jacobi_audit1", {**base, "V06_ARM": "B", "V06_AUDIT": "1", "V06_PRECOND": "jacobi"})
    for rtol in ("1e-4", "1e-6"):
        base = {"V06_FAMILY": "adr2d", "V06_RTOL": rtol, "V06_PRECOND": "none", "V06_LABEL": "v17 regression"}
        runner.run(f"a_adr2d_{rtol}_C_none_audit1", {**base, "V06_ARM": "C", "V06_AUDIT": "1"})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=["setup", "runs", "timing", "exploratory", "regression"])
    parser.add_argument("--bin", required=True, help="the stage_clients_krylov_v17_measure test binary (regression: the V06 one)")
    parser.add_argument("--families", default="")
    parser.add_argument("--rtols", default=",".join(RTOLS))
    parser.add_argument("--cells", default="", help="timing: family:rtol,... (default: all)")
    parser.add_argument("--try", dest="attempt", type=int, default=1)
    parser.add_argument("--wait-minutes", type=int, default=0, help="timing: poll for a quiet host first")
    parser.add_argument("--groups", default="restart,jacobi,kappa,stiff")
    args = parser.parse_args()

    families = [f for f in args.families.split(",") if f] or declared_families()
    rtols = [r for r in args.rtols.split(",") if r]
    if args.phase == "regression":
        phase_regression(Runner("v06_regression", args.bin, "v06_run"))
        return
    runner = Runner(args.phase, args.bin, "v17_run")
    if args.phase == "setup":
        phase_setup(runner, families)
    elif args.phase == "runs":
        phase_runs(runner, families, rtols)
    elif args.phase == "timing":
        if args.cells:
            cells = [tuple(cell.split(":")) for cell in args.cells.split(",")]
        else:
            cells = [(family, rtol) for family in families for rtol in rtols]
        if args.wait_minutes:
            waited = wait_for_quiet(args.wait_minutes)
            print(f"waited {waited} minutes for a quiet host; loadavg {loadavg()}", flush=True)
        phase_timing(runner, cells, args.attempt)
    elif args.phase == "exploratory":
        phase_exploratory(runner, families, rtols, set(args.groups.split(",")))


if __name__ == "__main__":
    sys.exit(main())
