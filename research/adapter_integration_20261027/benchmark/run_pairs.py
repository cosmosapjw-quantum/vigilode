#!/usr/bin/env python3
"""Run the registered comparisons serially; never compiles or contacts NCP."""
import argparse
import hashlib
import json
import os
import platform
import subprocess
import time
from pathlib import Path

CASES = ["rotation", "decay", "case5", "stiff-forced", "relaxation",
         "transition-39", "transition-79", "gyro-tight", "gyro-default"]
CONTROLS = {c: ("dopri" if c in {"rotation", "decay", "gyro-tight", "gyro-default"}
                else "rodas") for c in CASES}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cases", nargs="+", choices=CASES, default=CASES)
    parser.add_argument("--blocks", type=int, default=5)
    args = parser.parse_args()
    binary = args.binary.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    repo = Path(__file__).resolve().parents[3]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    env = os.environ.copy()
    env.update(RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1",
               VIGILODE_SOURCE_COMMIT=commit)
    pairs = []
    for case in args.cases:
        pairs.extend([(case, "evidence", "adaptive"), (case, "adaptive", CONTROLS[case])])
        if case in {"case5", "stiff-forced", "relaxation", "transition-39", "transition-79"}:
            pairs.append((case, "adaptive-unseeded", "adaptive"))
        if case == "case5":
            pairs.extend([(case, "adaptive", "rodas-dense"), (case, "rodas", "rodas-dense"),
                          (case, "adaptive-no-startup-retry", "adaptive")])
    manifest = {
        "schema": "vigilode.adapter-integration-run.v1", "commit": commit,
        "binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "source_sha256": hashlib.sha256((repo / "crates/vigilode/examples/adapter_integration_pilot.rs").read_bytes()).hexdigest(),
        "source_dirty": subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True),
        "host": platform.platform(), "processor": platform.processor(), "cpu_count": os.cpu_count(),
        "thread_environment": {k: env[k] for k in ["RAYON_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS"]},
        "pairs": pairs, "blocks": args.blocks, "runs": [],
        "prior_native_source": "596dcdfae8b95d24f27b57975d9f4626267b8149",
        "prior_native_tip": "e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824",
        "native_behavior_parity": "NOT_ASSERTED_BY_THIS_RUNNER; requires separate fixture/config/counter comparison",
        "same_binary_comparison": True, "concurrent_timed_cells": 1,
    }
    for case, a, b in pairs:
        stem = f"{case}__{a}__{b}"
        command = [str(binary), case, a, b, str(args.blocks)]
        started = time.monotonic()
        try:
            result = subprocess.run(command, text=True, capture_output=True, env=env, timeout=60)
            stdout, stderr, code = result.stdout, result.stderr, result.returncode
        except subprocess.TimeoutExpired as exc:
            stdout = exc.stdout or b""
            stderr = exc.stderr or b""
            stdout = stdout.decode() if isinstance(stdout, bytes) else stdout
            stderr = stderr.decode() if isinstance(stderr, bytes) else stderr
            code = "TIMEOUT"
        (output / f"{stem}.stdout.json").write_text(stdout)
        (output / f"{stem}.stderr.log").write_text(stderr)
        manifest["runs"].append({"case": case, "a": a, "b": b, "exit": code,
            "wall_seconds": time.monotonic()-started, "command": command,
            "stdout": f"{stem}.stdout.json", "stderr": f"{stem}.stderr.log"})
        (output / "RUN.json").write_text(json.dumps(manifest, indent=2)+"\n")
        print(stem, code, flush=True)


if __name__ == "__main__":
    main()
