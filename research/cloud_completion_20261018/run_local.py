#!/usr/bin/env python3
"""Run the completed cloud implementation and retain reproducible local receipts.

No third-party Python packages. Rust 1.94.1 + locked Cargo dependencies required.
This runner never changes tolerances or turns an empirical timing into promotion.
"""
import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("check", "smoke", "bench", "all"), default="check")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--online", action="store_true", help="allow Cargo dependency downloads")
    parser.add_argument("--profile", choices=("measurement", "release"), default="measurement")
    parser.add_argument("--modes", type=int, default=32)
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()
    if args.modes <= 0 or args.modes & (args.modes - 1) or args.repeats <= 0:
        parser.error("modes must be a positive power of two, repeats positive")
    output = args.output.resolve()
    if output.exists() and any(output.iterdir()):
        parser.error("output directory must be empty; preserve earlier attempts")
    output.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    env["LEJA_ENCLOSURE_RUNS"] = str(output / "leja_runs.json")
    flags = ["--locked"] + ([] if args.online else ["--offline"])
    records = []

    def capture(command):
        p = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True)
        return {"exit_code": p.returncode, "stdout": p.stdout.strip(), "stderr": p.stderr.strip()}

    metadata = {
        "schema_version": 1, "mode": args.mode,
        "source": capture(["git", "rev-parse", "HEAD"]),
        "worktree": capture(["git", "status", "--porcelain"]),
        "rustc": capture(["rustc", "-Vv"]), "cargo": capture(["cargo", "-V"]),
        "platform": platform.platform(), "python": sys.version,
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "cargo_target_dir": env.get("CARGO_TARGET_DIR", "repository default"),
        "rustflags": env.get("RUSTFLAGS", ""),
        "ld_library_path_set": bool(env.get("LD_LIBRARY_PATH")),
        "promotion": "REQUIRES_SCOPE_REVIEW; timing is diagnostic",
    }
    (output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")

    def run(name, command):
        start = time.monotonic()
        with (output / (name + ".stdout")).open("wb") as stdout, (output / (name + ".stderr")).open("wb") as stderr:
            try:
                process = subprocess.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
                code = process.returncode
            except OSError as error:
                stderr.write(str(error).encode())
                code = 127
        record = {"name": name, "command": command, "exit_code": code,
                  "wall_seconds": time.monotonic() - start,
                  "stdout": name + ".stdout", "stderr": name + ".stderr"}
        records.append(record)
        (output / "executions.json").write_text(json.dumps(records, indent=2) + "\n")
        print(f"{name}: exit {code}", flush=True)
        return code == 0

    if args.mode in ("check", "all"):
        run("all_targets", ["cargo", "check", *flags, "--workspace", "--all-targets"])
        run("core_contracts", ["cargo", "test", *flags, "-p", "rodas5p-core", "--test", "leja_enclosed_contract",
                              "--test", "leja_total_contract", "--test", "leja_setup_work_contract"])
        run("leja_unit", ["cargo", "test", *flags, "-p", "rodas5p-core", "--lib", "cloud_"])
        run("integrator_contracts", ["cargo", "test", *flags, "-p", "rodas5p-integrators",
            "--test", "affine_transport_contract", "--test", "production_audit_packet_verified",
            "--test", "production_audit_packet_curve_output", "--test", "production_audit_pr101_cloud",
            "--test", "a05_dense_global_output", "--test", "collective_rodas5p_contracts", "--", "--nocapture"])
        run("allocation_probe", ["cargo", "test", *flags, "-p", "rodas5p-integrators",
            "--test", "collective_rodas5p_allocation_contract", "--", "--nocapture"])
        if (output / "leja_runs.json").is_file():
            run("leja_fraction", [sys.executable, "research/cloud_completion_20261018/leja_fraction_oracle.py",
                str(output / "leja_runs.json"), "--output", str(output / "leja_fraction.json")])
        run("pr101_fraction", [sys.executable, "crates/rodas5p-integrators/tests/pr101_cloud_fraction_oracle.py",
            str(output / "integrator_contracts.stdout")])
    if args.mode in ("smoke", "all"):
        run("packet_run", ["cargo", "run", *flags, "-p", "rodas5p-integrators", "--example",
                           "packet_verified_run", "--", "16", "32"])
        run("pr101_run", ["cargo", "run", *flags, "-p", "rodas5p-integrators", "--example",
                          "pr101_verified_run"])
    if args.mode in ("bench", "all"):
        built = run("comparison_build", ["cargo", "build", *flags, "--profile", args.profile,
            "-p", "rodas5p-integrators", "--example", "collective_structured_compare"])
        if built:
            # cargo run does not include compilation in the runner's per-arm timing.
            run("structured_comparison", ["cargo", "run", *flags, "--profile", args.profile,
                "-p", "rodas5p-integrators", "--example", "collective_structured_compare", "--",
                str(args.modes), str(args.repeats)])
    success = bool(records) and all(row["exit_code"] == 0 for row in records)
    numeric = [row for row in records if row["name"] != "allocation_probe"]
    receipt = {"schema_version": 1, "success": success,
               "nonallocation_commands_success": bool(numeric) and all(row["exit_code"] == 0 for row in numeric),
               "executions": records,
               "scientific_scope": "See DECISION.json; successful process alone does not establish accuracy or speed.",
               "return_to_review": ["environment.json", "executions.json", "*.stdout", "*.stderr", "*.json"]}
    (output / "RESULT.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
