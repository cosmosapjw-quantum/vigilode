#!/usr/bin/env python3
"""Deciding runs of V27 phase 2 (PHASE2_NOTES.txt sections 2 and 3).

Every case (problem x tolerance pair x options) runs the arms T, N, D, R in this order; each arm
in one process with --repeat 2 (runs 1 and 2) and in a fresh process with --repeat 1 (run 3),
the entry log on. D and R run from the N binary and once more from the T binary (sanity rule of
PREDECLARED arms.sanity: a disagreement is recorded and stops the evaluation). Raw stdout of
every process is kept under logs/phase2/raw/<case>/; logs/phase2/RUNS_INDEX.jsonl has one row per
process (command, UTC start time, exit code, stderr, sha256 of stdout). No duration is measured.

Usage: python3 -I run_phase2.py
"""
import hashlib
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(HERE, "logs", "phase2")
RAW = os.path.join(LOGS, "raw")
INDEX = os.path.join(LOGS, "RUNS_INDEX.jsonl")
TARGETS = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116"
BINARY = {"N": f"{TARGETS}/w3-v27/release/v27-runner", "T": f"{TARGETS}/w3-v27base/release/v27-runner"}
TASKSET = ["taskset", "-c", "0-5,12-17"]

TOLERANCES = {"A": ("1e-6", "1e-9"), "L": ("1e-3", "1e-6"), "S": ("1e-8", "1e-11")}
STIFF = ["P_scalar_fd", "lin_2000", "lin_3200", "lin_3400", "P_heat_fd", "robertson_40",
         "robertson_1e5", "hires", "orego", "vdp_1000", "flame_1e-2", "flame_1e-3", "flame_1e-4",
         "heat_64_fd", "kt_vanishing"]
STIFF_ALSO = ["P_scalar_fd", "P_heat_fd", "lin_3200", "robertson_40", "vdp_1000"]
NON_STIFF = ["osc_1", "osc_10", "osc_1000", "lotka_volterra", "lorenz", "arenstorf",
             "brusselator_2", "vdp_1", "damped_osc", "nonnormal", "lorenz96_40", "pleiades"]
CALLBACK = [("F2", "default"), ("N2", "default"), ("P_fatal", "default"),
            ("P_blowup", "default"), ("P_mass", "default"), ("P_heat_dense", "default"),
            ("P_scalar_dense", "default"), ("P_heat_jvp", "default"),
            ("P_heat_banded", "default"), ("robertson_40", "regime=stiff"),
            ("robertson_40", "regime=nonstiff"), ("osc_10", "regime=stiff"),
            ("osc_10", "regime=nonstiff"), ("robertson_40", "max_steps=500"),
            ("robertson_40", "max_steps=50")]


def cases():
    out = []
    for tol in "ALS":
        for problem in STIFF:
            if tol == "A" or problem in STIFF_ALSO:
                out.append((problem, tol, "default"))
    for tol in "ALS":
        for problem in NON_STIFF:
            out.append((problem, tol, "default"))
    for problem, tag in CALLBACK:
        out.append((problem, "A", tag))
    return out


def option_flags(tol, tag):
    rtol, atol = TOLERANCES[tol]
    flags = ["--rtol", rtol, "--atol", atol]
    if tag.startswith("regime="):
        flags += ["--regime", tag.split("=")[1]]
    elif tag.startswith("max_steps="):
        flags += ["--max-steps", tag.split("=")[1]]
    return flags


def case_id(problem, tol, tag):
    return f"{problem}|{tol}|{tag}"


def essence(line):
    run = line.get("run", {})
    keys = ("final_time_bits", "final_state_bits", "status", "success", "counters", "attempts",
            "accepted_steps", "rejected_steps", "stop", "error")
    return json.dumps({"run": {k: run.get(k) for k in keys}, "entries": line.get("entries")},
                      sort_keys=True)


def main():
    os.makedirs(RAW, exist_ok=True)
    if os.path.exists(INDEX):
        print("RUNS_INDEX.jsonl exists; refusing to run twice", file=sys.stderr)
        return 2
    seq = 0
    all_cases = cases()
    assert len(all_cases) == 76, len(all_cases)
    with open(INDEX, "a") as index:
        for problem, tol, tag in all_cases:
            cid = case_id(problem, tol, tag)
            folder = os.path.join(RAW, cid.replace("|", "__").replace("=", "-"))
            os.makedirs(folder, exist_ok=True)
            first_lines = {}
            plan = []
            for arm in ("T", "N", "D", "R"):
                build = "T" if arm == "T" else "N"
                plan.append((arm, build, 1, 2))
                plan.append((arm, build, 2, 1))
                if arm in ("D", "R"):
                    plan.append((arm, "T", 3, 1))
            for arm, build, process, repeat in plan:
                command = TASKSET + [BINARY[build], "--arm", arm, "--problem", problem,
                                     "--repeat", str(repeat)] + option_flags(tol, tag)
                start = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                done = subprocess.run(command, capture_output=True, text=True)
                name = f"{arm}_{build}build_p{process}.jsonl"
                with open(os.path.join(folder, name), "w") as handle:
                    handle.write(done.stdout)
                lines = [l for l in done.stdout.splitlines() if l.strip()]
                seq += 1
                row = {"seq": seq, "case": cid, "problem": problem, "tolerance": tol,
                       "options": tag, "arm": arm, "binary": build, "process": process,
                       "repeat": repeat, "command": " ".join(command), "start_utc": start,
                       "exit_code": done.returncode, "stderr": done.stderr,
                       "stdout_file": os.path.relpath(os.path.join(folder, name), HERE),
                       "stdout_sha256": hashlib.sha256(done.stdout.encode()).hexdigest(),
                       "lines": len(lines)}
                index.write(json.dumps(row, sort_keys=True) + "\n")
                index.flush()
                if lines:
                    try:
                        first_lines[(arm, build, process)] = json.loads(lines[0])
                    except json.JSONDecodeError:
                        pass
            # sanity: D and R of the two builds agree bit for bit
            for arm in ("D", "R"):
                a, b = first_lines.get((arm, "N", 1)), first_lines.get((arm, "T", 3))
                if a is None or b is None or essence(a) != essence(b):
                    stop = {"seq": seq + 1, "case": cid, "stop": f"sanity: arm {arm} differs "
                            "between the builds or a line is missing; evaluation stopped"}
                    index.write(json.dumps(stop, sort_keys=True) + "\n")
                    print(json.dumps(stop))
                    return 1
            print(f"{seq:4d} {cid} done", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
