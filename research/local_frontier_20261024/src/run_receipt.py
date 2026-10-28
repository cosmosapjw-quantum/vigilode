#!/usr/bin/env python3
"""Execute a supplied command once; retain streams, identity, exit and elapsed time."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", required=True)
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or Path(args.name).name != args.name:
        parser.error("provide a command and a plain receipt name")
    package = Path(__file__).resolve().parents[1]
    repo = package.parents[1]
    output = package / "evidence" / "runs"
    output.mkdir(exist_ok=True)
    receipt_path = output / f"{args.name}.receipt.json"
    if receipt_path.exists():
        parser.error("receipt already exists; use a new revision name")
    paths = [repo / x for x in subprocess.check_output(
        ["git", "ls-files", "crates"], cwd=repo, text=True).splitlines()]
    paths += list((repo / "crates").glob("*/src/*.rs"))
    paths += list((repo / "crates").glob("*/tests/*.rs"))
    paths += list((repo / "crates").glob("*/examples/*.rs"))
    sources = {str(p.relative_to(repo)): sha(p) for p in sorted(set(paths)) if p.is_file()}
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    executable = shutil.which(command[0])
    binary = Path(executable) if executable else repo / command[0]
    binary_identity = {"path": str(binary), "sha256": sha(binary)} if binary.is_file() else None
    then = time.monotonic()
    timed_out = False
    with (output / f"{args.name}.stdout").open("wb") as stdout, (output / f"{args.name}.stderr").open("wb") as stderr:
        proc = subprocess.Popen(command, cwd=repo, stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            code = proc.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            import signal
            timed_out = True
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
            code = 124
    receipt = {
        "schema_version": 1, "name": args.name, "command": command,
        "cwd": str(repo), "started_utc": started, "elapsed_seconds": time.monotonic() - then,
        "exit_code": code, "timed_out": timed_out,
        "executable": binary_identity,
        "source_sha256": sources,
        "environment": {k: os.environ.get(k) for k in ["CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_CODEGEN_UNITS", "CARGO_PROFILE_TEST_CODEGEN_UNITS", "CARGO_PROFILE_MEASUREMENT_CODEGEN_UNITS", "RAYON_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"]},
        "stdout_sha256": sha(output / f"{args.name}.stdout"),
        "stderr_sha256": sha(output / f"{args.name}.stderr"),
    }
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({k: receipt[k] for k in ["name", "exit_code", "timed_out", "elapsed_seconds"]}))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
