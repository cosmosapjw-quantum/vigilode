#!/usr/bin/env python3
"""Build the pinned local suite; retain failed offline dependency/build output."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from campaign import REPO, canonical, digest, json_new, source_identity, utc


EXAMPLES = {
    "local_bottleneck_suite": "vigilode",
    "reconsideration_bench": "vigilode-coupled",
    "conditional_chart_sweep": "rodas5p-integrators",
    "conditional_continuation_sweep": "rodas5p-integrators",
    "conditioned_chart_bench": "rodas5p-integrators",
    "fixed_point_continuation_bench": "rodas5p-integrators",
    "invariant_module_bench": "rodas5p-core",
    "structured_residual_sweep": "rodas5p-core",
}

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--expected-commit", required=True)
    p.add_argument("--expected-tree", required=True)
    p.add_argument("--output", required=True, type=Path)
    p.add_argument("--target-dir", required=True, type=Path)
    p.add_argument("--example", choices=list(EXAMPLES), default="local_bottleneck_suite")
    p.add_argument("--online-locked", action="store_true", help="explicitly permit Cargo to fetch locked dependencies")
    a = p.parse_args()
    identity = source_identity(REPO, a.expected_commit, a.expected_tree)
    a.output.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(a.target_dir.resolve()), CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0",
               CARGO_PROFILE_MEASUREMENT_CODEGEN_UNITS="1", CARGO_PROFILE_MEASUREMENT_DEBUG="1",
               RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    cmd = ["cargo", "build", "--locked", "--profile", "measurement", "-p", EXAMPLES[a.example], "--example", a.example]
    if not a.online_locked:
        cmd.append("--offline")
    versions = {}
    for tool in ("cargo", "rustc"):
        result = subprocess.run([tool, "--version"], capture_output=True, text=True)
        versions[tool] = {"stdout": result.stdout, "stderr": result.stderr, "exit_code": result.returncode}
    started = utc()
    with (a.output / "stdout.log").open("xb") as out, (a.output / "stderr.log").open("xb") as err:
        result = subprocess.run(cmd, cwd=REPO, env=env, stdout=out, stderr=err)
    source_after = source_identity(REPO, allow_dirty=True)
    source_changed = source_after != identity
    effective_exit = result.returncode if not source_changed else 2
    binary = a.target_dir.resolve() / ("measurement/examples/" + a.example)
    receipt = {"schema_version": 1, "source": identity, "started_utc": started, "finished_utc": utc(),
               "command": cmd, "exit_code": effective_exit, "cargo_exit_code": result.returncode, "source_after": source_after, "source_changed_during_build": source_changed, "versions": versions,
               "cargo_lock_sha256": digest(REPO / "Cargo.lock"), "binary": str(binary),
               "binary_sha256": digest(binary) if effective_exit == 0 and binary.is_file() else None,
               "stdout_sha256": digest(a.output / "stdout.log"), "stderr_sha256": digest(a.output / "stderr.log"),
               "profile": "measurement, codegen-units=1, debug=1; all campaign methods share this binary",
               "online_locked_explicitly_enabled": a.online_locked}
    json_new(a.output / "BUILD_RECEIPT.json", receipt)
    print(canonical(receipt))
    return effective_exit


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as e:
        print(canonical({"status": "BLOCKED", "error": str(e)}), file=sys.stderr)
        raise SystemExit(2)
