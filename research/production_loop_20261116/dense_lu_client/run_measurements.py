#!/usr/bin/env python3
"""Runs the measurements of loop L1 node V18_DENSE_LU_CLIENT.

Usage (from the worktree root, CARGO_TARGET_DIR exported):

    python3 -I research/production_loop_20261116/dense_lu_client/run_measurements.py <stage> ...

Stages: build, links, tests, identity, reference, faults, wall, wp, extra, perf, phases.

The protocol (cells, tolerances, caps) is read from PREDECLARED.json. This script starts
programs, stores their raw output under logs/ and appends one line per invocation to
logs/MANIFEST.jsonl. It computes no result: parse_results.py does that from the logs. The only
numbers it looks at are the ones the timing quality rule needs (load average, spread within an
arm) to decide whether an invocation is measured again.
"""

import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOGS = HERE / "logs"
MANIFEST = LOGS / "MANIFEST.jsonl"
RUNNER_DIR = HERE / "runner"
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ""))
EXPECTED_TARGET = Path("/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v18")
LOCK = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock"
BINARY = TARGET / "release" / "v18-dense-lu-runner"
LINK_DIR = TARGET / "hl"
VARIANTS = (93, 108, 124)
NOT_TIMING = ["taskset", "-c", "0-5,12-17"]
TIMING = ["flock", LOCK, "taskset", "-c", "6-11"]
CLEAN_ENV = ["env", "-i", "PATH=/usr/bin:/bin", "LC_ALL=C"]
LOAD_LIMIT = 8.0
SPREAD_LIMIT = 0.10
MAX_TRIES = 3
WAIT_BUDGET_S = 20 * 60
# A later measurement round (V18_ROUND=2, ...) keeps the logs of the earlier ones: its timing
# logs carry the suffix .round<N> and it has a wait budget of its own. Round 1 is the node's run.
ROUND = int(os.environ.get("V18_ROUND", "1"))
ROUND_SUFFIX = "" if ROUND == 1 else f".round{ROUND}"
WAIT_STATE = LOGS / f"WAIT_BUDGET{ROUND_SUFFIX}.json"

PREDECLARED = json.loads((HERE / "PREDECLARED.json").read_text())
CELLS = PREDECLARED["cells"]


def load1():
    return float(Path("/proc/loadavg").read_text().split()[0])


def loadavg():
    return [float(v) for v in Path("/proc/loadavg").read_text().split()[:3]]


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def link(length):
    prefix = f"{LINK_DIR}/v18r-len{length:03d}-"
    path = prefix + "x" * (length - len(prefix))
    assert len(path) == length, (path, len(path))
    return path


def rtol_text(rtol):
    return f"{rtol:.0e}".replace("e-0", "e-")


def record(entry):
    with open(MANIFEST, "a") as handle:
        handle.write(json.dumps(entry, sort_keys=True) + "\n")


def run(stage, name, argv, cwd=ROOT, env=None, extra=None):
    """One program run: stdout to logs/<name>, stderr to logs/<name>.err, one manifest line."""
    LOGS.mkdir(exist_ok=True)
    out_path = LOGS / name
    err_path = LOGS / (name + ".err")
    entry = {
        "stage": stage,
        "log": name,
        "argv": argv,
        "cwd": str(cwd),
        "loadavg_before": loadavg(),
        "start_unix": time.time(),
    }
    with open(out_path, "w") as out, open(err_path, "w") as err:
        code = subprocess.run(argv, cwd=cwd, stdout=out, stderr=err, env=env).returncode
    entry.update(
        {
            "exit_code": code,
            "end_unix": time.time(),
            "loadavg_after": loadavg(),
            "stderr_log": name + ".err" if err_path.stat().st_size else None,
        }
    )
    if not err_path.stat().st_size:
        err_path.unlink()
    if extra:
        entry.update(extra)
    record(entry)
    print(f"[{stage}] {name}: exit {code}, {entry['end_unix'] - entry['start_unix']:.1f} s", flush=True)
    return code


def runner(stage, name, length, command, cell, rtol, options, prefix, extra=None):
    spec = CELLS[cell]
    argv = (
        prefix
        + CLEAN_ENV
        + [link(length), command, "--cell", cell, "--t-end", repr(spec["t_span"][1]), "--rtol", repr(rtol)]
        + ["--max-attempts", str(spec["max_attempts"])]
        + options
    )
    info = {"runner_sha256": sha256(BINARY), "link_length": length, "cell": cell, "rtol": rtol}
    if extra:
        info.update(extra)
    return run(stage, name, argv, extra=info)


def waited():
    if WAIT_STATE.exists():
        return json.loads(WAIT_STATE.read_text())["waited_s"]
    return 0.0


def wait_for_quiet():
    """The bounded wait of the timing quality rule: one check per minute, 20 minutes per node."""
    total = waited()
    while load1() > LOAD_LIMIT and total < WAIT_BUDGET_S:
        print(f"  load {load1():.2f} > {LOAD_LIMIT}: waiting ({total:.0f} s of {WAIT_BUDGET_S} used)", flush=True)
        time.sleep(60)
        total += 60
        state = json.loads(WAIT_STATE.read_text()) if WAIT_STATE.exists() else {}
        state.update({"waited_s": total, "limit_s": WAIT_BUDGET_S})
        WAIT_STATE.write_text(json.dumps(state) + "\n")


def meets_quality_rule(name):
    """Load before and after at most 8 and the spread of every arm at most 10 %."""
    walls, before, after = {}, None, None
    for line in (LOGS / name).read_text().splitlines():
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        if row["type"] == "header":
            before = row["loadavg_before"][0]
        elif row["type"] == "footer":
            after = row["loadavg_after"][0]
        elif row["type"] == "run" and row["kind"] == "timed":
            walls.setdefault(row["arm"], []).append(row["wall_ns"])
    if before is None or after is None or not walls:
        return False
    spreads = [(max(v) - min(v)) / statistics.median(v) for v in walls.values()]
    return before <= LOAD_LIMIT and after <= LOAD_LIMIT and max(spreads) <= SPREAD_LIMIT


def measure(stage, base, length, cell, rtol, arms, options=()):
    """One more try of a timing invocation.

    Nothing is run when an earlier try of this round meets the quality rule or when the three
    tries of the re-measurement rule exist. Every try is kept. Calling a timing stage again
    therefore re-measures exactly the invocations that still miss the rule.
    """
    names = [f"{base}{ROUND_SUFFIX}.try{k}.jsonl" for k in range(1, MAX_TRIES + 1)]
    done = [name for name in names if (LOGS / name).exists()]
    if any(meets_quality_rule(name) for name in done):
        print(f"  {base}: an earlier try meets the quality rule", flush=True)
        return
    if len(done) == MAX_TRIES:
        print(f"  {base}: three tries exist, none meets the quality rule", flush=True)
        return
    attempt = len(done) + 1
    name = names[attempt - 1]
    wait_for_quiet()
    code = runner(
        stage,
        name,
        length,
        "measure",
        cell,
        rtol,
        ["--arms", ",".join(arms), "--warmups", "2", "--reps", "7"] + list(options),
        TIMING,
        {"try": attempt, "round": ROUND, "arms": arms},
    )
    verdict = "meets" if code == 0 and meets_quality_rule(name) else "does not meet"
    print(f"  {name}: {verdict} the quality rule", flush=True)


def stage_build():
    assert TARGET == EXPECTED_TARGET, f"CARGO_TARGET_DIR must be {EXPECTED_TARGET}"
    code = run(
        "build",
        "build_runner.log",
        NOT_TIMING + ["cargo", "+1.94.1", "build", "--locked", "-j", "4", "--release"],
        cwd=RUNNER_DIR,
    )
    if code == 0:
        record({"stage": "build", "binary": str(BINARY), "runner_sha256": sha256(BINARY)})


def stage_links():
    LINK_DIR.mkdir(exist_ok=True)
    for length in VARIANTS:
        path = Path(link(length))
        if path.is_symlink():
            path.unlink()
        path.symlink_to(BINARY)
        record({"stage": "links", "link": str(path), "length": len(str(path)), "target": str(BINARY)})
        print(f"[links] {path} ({len(str(path))} characters)", flush=True)


def stage_tests():
    cargo = NOT_TIMING + ["cargo", "+1.94.1", "test", "--locked", "-j", "4", "--release", "-p", "rodas5p-integrators"]
    tail = ["--", "--nocapture", "--test-threads=1"]
    run("tests", "test_identity.log", cargo + ["--test", "stage_clients_dense_lu_reuse_identity"] + tail)
    run("tests", "test_alloc.log", cargo + ["--test", "stage_clients_dense_lu_reuse_alloc"] + tail)
    run(
        "tests",
        "test_identity_n2048.log",
        cargo + ["--test", "stage_clients_dense_lu_reuse_identity"] + tail + ["--ignored"],
    )


def stage_identity():
    arms = "kit,reuse,threaded2,threaded4,threaded6"
    for cell, spec in CELLS.items():
        for rtol in spec["rtols"]:
            name = f"identity.{cell}.rtol{rtol_text(rtol)}.jsonl"
            runner("identity", name, 93, "identity", cell, rtol, ["--arms", arms], NOT_TIMING)


def stage_reference():
    for cell in ("skewnet.n512", "skewnet.n2048"):
        runner("reference", f"reference_inputs.{cell}.jsonl", 93, "reference-inputs", cell, 1.0e-6, [], NOT_TIMING)


def stage_faults():
    plan = []
    for cell in ("meanfield.n512", "skewnet.n512"):
        for length in VARIANTS:
            plan += [(cell, 1.0e-6, length, arm) for arm in ("protected", "kit", "reuse", "threaded6")]
    for cell in ("meanfield.n128", "skewnet.n128"):
        plan += [(cell, 1.0e-6, 93, arm) for arm in ("protected", "kit", "reuse")]
    plan += [("skewnet.n2048", 1.0e-5, 93, arm) for arm in ("protected", "kit", "reuse")]
    plan += [("meanfield.n2048", 1.0e-6, 93, arm) for arm in ("protected", "kit", "reuse")]
    for cell, rtol, length, arm in plan:
        name = f"faults.{cell}.rtol{rtol_text(rtol)}.len{length}.{arm}.jsonl"
        runner("faults", name, length, "faults", cell, rtol, ["--arms", arm, "--runs", "3"], NOT_TIMING, {"arm": arm})


def stage_wall():
    for cell in ("skewnet.n512", "meanfield.n512"):
        for length in VARIANTS:
            measure("wall", f"wall.{cell}.rtol1e-6.len{length}", length, cell, 1.0e-6, ["protected", "reuse"])


def stage_wp():
    threads = ["threaded2", "threaded4", "threaded6"]
    for rtol in CELLS["skewnet.n512"]["rtols"]:
        measure(
            "wp",
            f"wp.skewnet.n512.rtol{rtol_text(rtol)}",
            93,
            "skewnet.n512",
            rtol,
            ["protected", "reuse"] + threads,
            ["--emit-end-state"],
        )
    for rtol in CELLS["skewnet.n2048"]["rtols"]:
        measure(
            "wp",
            f"wp.skewnet.n2048.rtol{rtol_text(rtol)}",
            93,
            "skewnet.n2048",
            rtol,
            ["protected"] + threads,
            ["--emit-end-state"],
        )


def stage_extra():
    threads = ["protected", "threaded2", "threaded4", "threaded6"]
    measure("extra", "extra.meanfield.n512.rtol1e-6.threads", 93, "meanfield.n512", 1.0e-6, threads)
    measure("extra", "extra.meanfield.n2048.rtol1e-6.threads", 93, "meanfield.n2048", 1.0e-6, threads)
    measure("extra", "extra.skewnet.n2048.rtol1e-4.reuse", 93, "skewnet.n2048", 1.0e-4, ["protected", "reuse"])


PHASES_BINARY = TARGET / "release" / "v18-dense-lu-phases"
# Exploratory (not in PREDECLARED.json): phase times of the client arms. n = 1024 is not a
# declared cell, so the time span is given here.
PHASE_CELLS = (
    ("skewnet.n512", 2.0, 1.0e-6),
    ("meanfield.n512", 10.0, 1.0e-6),
    ("skewnet.n1024", 1.0, 1.0e-5),
    ("skewnet.n2048", 1.0, 1.0e-4),
)


def stage_phases():
    for cell, t_end, rtol in PHASE_CELLS:
        name = f"phases.{cell}.rtol{rtol_text(rtol)}{ROUND_SUFFIX}.jsonl"
        wait_for_quiet()
        argv = (
            TIMING
            + CLEAN_ENV
            + [str(PHASES_BINARY), "--cell", cell, "--t-end", repr(t_end), "--rtol", repr(rtol)]
            + ["--warmups", "1", "--reps", "5"]
        )
        run(
            "phases",
            name,
            argv,
            extra={"cell": cell, "rtol": rtol, "round": ROUND, "phases_sha256": sha256(PHASES_BINARY)},
        )


def stage_perf():
    for cell in ("skewnet.n512", "meanfield.n512"):
        for arm in ("none", "protected", "kit", "reuse"):
            base = f"perf.{cell}.rtol1e-6.{arm}"
            spec = CELLS[cell]
            argv = (
                NOT_TIMING
                + ["perf", "stat", "-x", ",", "-e", "instructions:u,minor-faults", "-o", str(LOGS / (base + ".perf.csv"))]
                + ["--"]
                + CLEAN_ENV
                + [link(93), "single", "--cell", cell, "--t-end", repr(spec["t_span"][1]), "--rtol", "1e-06"]
                + ["--max-attempts", str(spec["max_attempts"]), "--arms", arm, "--runs", "3"]
            )
            run("perf", base + ".jsonl", argv, extra={"cell": cell, "arm": arm, "perf_csv": base + ".perf.csv"})


STAGES = {
    "build": stage_build,
    "links": stage_links,
    "tests": stage_tests,
    "identity": stage_identity,
    "reference": stage_reference,
    "faults": stage_faults,
    "wall": stage_wall,
    "wp": stage_wp,
    "extra": stage_extra,
    "perf": stage_perf,
    "phases": stage_phases,
}


def main():
    if len(sys.argv) < 2 or any(stage not in STAGES for stage in sys.argv[1:]):
        sys.exit(f"usage: run_measurements.py <{'|'.join(STAGES)}> ...")
    assert TARGET == EXPECTED_TARGET, f"CARGO_TARGET_DIR must be {EXPECTED_TARGET}"
    LOGS.mkdir(exist_ok=True)
    for stage in sys.argv[1:]:
        record({"stage": stage, "event": "stage_start", "unix": time.time(), "loadavg": loadavg()})
        STAGES[stage]()
        record({"stage": stage, "event": "stage_end", "unix": time.time(), "loadavg": loadavg()})


if __name__ == "__main__":
    main()
