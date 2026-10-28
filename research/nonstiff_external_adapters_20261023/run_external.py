#!/usr/bin/env python3
"""Run only the frozen 2026-10-23 diffsol/Julia comparison grid locally."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import time


SOURCE = "45037fc24e8f8101605f309e1fdb3a9a4e3df56d"
CASES = ["decay", "driven", "logistic", "harmonic", "rapid", "damped",
         "mixed-scales", "nonnormal", "large-vector", "stiff-control",
         "moderate-stiff"]
DIMENSIONS = {"decay": 8, "driven": 8, "logistic": 8, "harmonic": 8,
              "rapid": 8, "damped": 8, "mixed-scales": 12, "nonnormal": 8,
              "large-vector": 4096, "moderate-stiff": 12, "stiff-control": 12}
TOLERANCES = (1e-3, 1e-5, 1e-7, 1e-9)
MODES = ("diffsol", "julia")
TIMEOUT = 45.0
BUDGET = 7200.0
SEED = 20261023


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, data):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--diffsol-adapter", type=Path, required=True)
    parser.add_argument("--julia-adapter", type=Path, required=True)
    parser.add_argument("--adapter-source-commit", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    adapters = {"diffsol": args.diffsol_adapter.resolve(),
                "julia": args.julia_adapter.resolve()}
    for mode, path in adapters.items():
        if not path.is_file() or not os.access(path, os.X_OK):
            parser.error(f"{mode} adapter must be a local executable: {path}")
    if args.out.exists():
        parser.error("output directory already exists; preserve prior attempts")
    if len(args.adapter_source_commit) != 40 or any(c not in "0123456789abcdef" for c in args.adapter_source_commit):
        parser.error("adapter source commit must be a full lowercase Git SHA")
    repository = Path(__file__).resolve().parents[2]
    actual_head = subprocess.check_output(["git", "-C", str(repository), "rev-parse", "HEAD"], text=True).strip()
    if actual_head != args.adapter_source_commit:
        parser.error("adapter source commit differs from publication worktree HEAD")
    if subprocess.check_output(["git", "-C", str(repository), "status", "--porcelain=v1"], text=True).strip():
        parser.error("adapter source worktree must be clean after freeze")

    configurations = []
    groups = []
    for case in CASES:
        for dimension in ([4096, 16384] if case == "large-vector" else [DIMENSIONS[case]]):
            for rtol in TOLERANCES:
                group = []
                for mode in MODES:
                    cfg = {"id": len(configurations), "case": case, "dimension": dimension,
                           "rtol": rtol, "atol": rtol * 1e-3, "mode": mode}
                    group.append(cfg["id"])
                    configurations.append(cfg)
                groups.append(group)
    assert len(groups) == 48 and len(configurations) == 96
    args.out.mkdir(parents=True)
    child_env = {**os.environ, "VIGILODE_ADAPTER_SOURCE_COMMIT": args.adapter_source_commit}
    plan = {
        "schema": "vigilode.nonstiff-campaign.v1",
        "cohort": "external-adapters-only",
        "source_commit": SOURCE,
        "adapter_source_commit": args.adapter_source_commit,
        "adapter_source_tree": subprocess.check_output(
            ["git", "-C", str(repository), "rev-parse", "HEAD^{tree}"], text=True).strip(),
        "adapters": {mode: {"command": [str(path)], "binary_sha256": sha256(path)}
                     for mode, path in adapters.items()},
        "ncp_used": False,
        "platform": platform.platform(),
        "affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
        "budget_seconds": BUDGET,
        "per_run_timeout": TIMEOUT,
        "full": True,
        "warmups_per_configuration": 1,
        "timed_repeats": 3,
        "seed": SEED,
        "order": "randomized case/tolerance groups in every round; shuffled adapter order within each group",
        "timing_policy": "adapter-reported solve time; subprocess cap includes imports/JIT; launch recorded separately",
        "configurations": configurations,
    }
    write_json(args.out / "PLAN.json", plan)

    receipts = []
    started = time.monotonic()
    rng = random.Random(SEED)
    for phase, repeat in [("warmup", -1), ("timed", 0), ("timed", 1), ("timed", 2)]:
        order = groups.copy()
        rng.shuffle(order)
        for ids in order:
            ids = ids.copy()
            rng.shuffle(ids)
            for idx in ids:
                cfg = configurations[idx]
                stem = f"{idx:03d}_{phase}_{repeat if repeat >= 0 else 0}"
                stdout = args.out / (stem + ".stdout.json")
                stderr = args.out / (stem + ".stderr.log")
                receipt = {**cfg, "phase": phase, "repeat": repeat,
                           "stdout": stdout.name, "stderr": stderr.name}
                remaining = BUDGET - (time.monotonic() - started)
                if remaining <= 0:
                    receipt.update(status="NOT_RUN_BUDGET", exit_code=None,
                                   elapsed_process_seconds=0.0)
                else:
                    command = [str(adapters[cfg["mode"]])]
                    request = {**cfg, "source_commit": SOURCE,
                               "adapter_source_commit": args.adapter_source_commit,
                               "max_steps": 100_000}
                    receipt["command"] = command
                    receipt["request_sha256"] = hashlib.sha256(json.dumps(request).encode()).hexdigest()
                    before = time.monotonic()
                    try:
                        with stdout.open("w") as out, stderr.open("w") as err:
                            process = subprocess.run(command, input=json.dumps(request), text=True,
                                                     stdout=out, stderr=err, check=False,
                                                     timeout=min(TIMEOUT, remaining), env=child_env)
                        receipt.update(status="COMPLETED" if process.returncode == 0 else "PROCESS_FAILED",
                                       exit_code=process.returncode)
                    except subprocess.TimeoutExpired:
                        receipt.update(status="TIMEOUT", exit_code=None)
                    except OSError as error:
                        stderr.write_text(str(error) + "\n")
                        receipt.update(status="LAUNCH_FAILED", exit_code=None)
                    receipt["elapsed_process_seconds"] = time.monotonic() - before
                    for key, path in (("stdout", stdout), ("stderr", stderr)):
                        if path.exists():
                            receipt[key + "_sha256"] = sha256(path)
                receipts.append(receipt)
                write_json(args.out / "RECEIPTS.json", receipts)
                print(json.dumps({k: receipt.get(k) for k in
                                  ("id", "case", "dimension", "mode", "phase", "repeat", "status",
                                   "elapsed_process_seconds")}), flush=True)

    write_json(args.out / "CAMPAIGN.json", {
        "source_commit": SOURCE, "adapter_source_commit": args.adapter_source_commit,
        "ncp_used": False, "elapsed_seconds": time.monotonic() - started,
        "planned_configurations": 96, "planned_runs": 384,
        "completed_processes": sum(r["status"] == "COMPLETED" for r in receipts),
        "other_process_outcomes": sum(r["status"] != "COMPLETED" for r in receipts),
        "receipt_count": len(receipts), "numerical_success_is_separate": True,
    })


if __name__ == "__main__":
    main()
