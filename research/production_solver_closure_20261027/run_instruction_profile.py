#!/usr/bin/env python3
"""Profile frozen native arms; subtract startup with two repetition counts.

Run under the shared native measurement lock. Example:
  python3 run_instruction_profile.py --binary /path/cubic_production_bench \
    --output profile --arms generic scalar --repetitions 20 40 \
    --valgrind /path/valgrind.bin -- --arm '{arm}' '{repetitions}'
No shell interprets the command template. Ir is an instruction count in the
Valgrind guest, not native elapsed time or a CPU-independent performance model.
"""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import time


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--arms", required=True, nargs="+")
    parser.add_argument("--repetitions", required=True, nargs=2, type=int)
    parser.add_argument("--valgrind", default="valgrind")
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    small, large = args.repetitions
    if not 0 < small < large:
        parser.error("repetitions must be two increasing positive integers")
    if not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("timeout must be finite and positive")
    if len(set(args.arms)) != len(args.arms):
        parser.error("arm names must be unique")
    template = args.arguments[1:] if args.arguments[:1] == ["--"] else args.arguments
    if not any("{arm}" in x for x in template) or not any("{repetitions}" in x for x in template):
        parser.error("template must contain {arm} and {repetitions}")
    if not all(re.fullmatch(r"[A-Za-z0-9_-]+", arm) for arm in args.arms):
        parser.error("arm names must be simple identifiers")
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = Path(__file__).resolve().parents[2]
    source_paths = sorted((root / "crates").rglob("*.rs"))
    source_before = {str(p.relative_to(root)): digest(p) for p in source_paths}
    (output / "SOURCE_INPUTS.json").write_text(json.dumps(source_before, indent=2) + "\n")
    version = subprocess.run([args.valgrind, "--version"], text=True, capture_output=True, check=False)
    if version.returncode != 0:
        failure = {"status": "failed", "class": "environment", "phase": "profiler-preflight",
                   "command": [args.valgrind, "--version"], "exit_code": version.returncode,
                   "stdout": version.stdout, "stderr": version.stderr,
                   "VALGRIND_LIB": os.environ.get("VALGRIND_LIB"), "native_profiles_executed": 0}
        (output / "PREFLIGHT_FAILURE.json").write_text(json.dumps(failure, indent=2) + "\n")
        raise SystemExit(version.returncode)
    protocol = {
        "schema": "vigilode.instruction-profile.v1",
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "binary": str(binary), "binary_sha256": digest(binary),
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "host": platform.node(), "platform": platform.platform(),
        "arms": args.arms, "repetitions": [small, large], "argument_template": template,
        "metric": "(Ir(large)-Ir(small))/(large-small)",
        "qualification": "frozen native executable asserts each complete solve; scientific audit is a separate receipt",
        "scope": "guest instruction count; startup subtracted; not native wall time",
        "valgrind_version": version.stdout.strip(),
        "environment": {k: os.environ.get(k) for k in ("VALGRIND_LIB", "RAYON_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS")},
    }
    (output / "PROTOCOL.json").write_text(json.dumps(protocol, indent=2) + "\n")
    runs = []
    for arm in args.arms:
        for count in (small, large):
            stem = f"{arm}_{count}"
            callgrind = output / f"{stem}.callgrind"
            command = [args.valgrind, "--tool=callgrind", "--cache-sim=no", "--branch-sim=no",
                       f"--callgrind-out-file={callgrind}", str(binary)]
            command += [x.format(arm=arm, repetitions=count) for x in template]
            started = time.monotonic()
            with (output / f"{stem}.stdout").open("wb") as stdout, (output / f"{stem}.stderr").open("wb") as stderr:
                # Inherit the outer recorder's group: cancellation must reach
                # Valgrind and the native program before the suite lock releases.
                process = subprocess.Popen(command, cwd=root, stdout=stdout, stderr=stderr)
                try:
                    code = process.wait(timeout=args.timeout)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                    code = 124
            record = {"arm": arm, "repetitions": count, "command": command,
                      "exit_code": code, "wall_seconds": time.monotonic() - started}
            if code == 0 and callgrind.exists():
                values = re.findall(r"^summary: (\d+)", callgrind.read_text(), re.M)
                if len(values) != 1:
                    record["parse_failure"] = f"expected one Ir summary, got {values}"
                else:
                    record["instructions"] = int(values[0])
                    record["callgrind_sha256"] = digest(callgrind)
            runs.append(record)
            (output / "RUNS.json").write_text(json.dumps(runs, indent=2) + "\n")
            print(json.dumps({k: record[k] for k in ("arm", "repetitions", "exit_code", "wall_seconds")}), flush=True)
            if "instructions" not in record:
                raise SystemExit(code or 1)
    slopes = {}
    for arm in args.arms:
        pair = [r for r in runs if r["arm"] == arm]
        slopes[arm] = (pair[1]["instructions"] - pair[0]["instructions"]) / (large - small)
    source_after = {str(p.relative_to(root)): digest(p) for p in sorted((root / "crates").rglob("*.rs"))}
    summary = {
        "status": "completed", "instructions_per_repetition": slopes,
        "ratio_to_first_arm": {k: v / slopes[args.arms[0]] for k, v in slopes.items()},
        "binary_unchanged": digest(binary) == protocol["binary_sha256"],
        "source_changes_during_profile": sorted(k for k in set(source_before) | set(source_after) if source_before.get(k) != source_after.get(k)),
        "interpretation": protocol["scope"],
    }
    (output / "SUMMARY.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary), flush=True)
    if not summary["binary_unchanged"] or summary["source_changes_during_profile"]:
        raise SystemExit("source or binary changed while profiling")


if __name__ == "__main__":
    main()
