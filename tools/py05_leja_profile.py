#!/usr/bin/env python3
"""Instruction profiler for Newton-Leja phi actions (PY05).

Preregistration: research/py05_leja_total_certificate_20261011/PREREGISTRATION.md
Measures setup and certification Ir via callgrind 2-minus-1 protocol.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

SCHEMA = "vigilode-py05-profile-v1"
NODE = "research/py05_leja_total_certificate_20261011"
EXAMPLE = "py05_leja_profile"
ROOT = Path(__file__).resolve().parent.parent

ENV = {
    "RAYON_NUM_THREADS": "1",
    "OPENBLAS_NUM_THREADS": "1",
    "DEBUGINFOD_URLS": "",
}


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.strip()


def build_example() -> Path:
    env = dict(os.environ, **ENV)
    subprocess.run(
        ["cargo", "build", "--release", "--locked", "-p", "rodas5p-core", "--example", EXAMPLE],
        cwd=ROOT,
        env=env,
        check=True,
    )
    binary = ROOT / "target" / "release" / "examples" / EXAMPLE
    if not binary.is_file():
        raise SystemExit(f"Example binary not found: {binary}")
    return binary


def ir_from_callgrind(outfile: Path) -> int:
    text = outfile.read_text()
    for line in text.splitlines():
        if line.startswith("summary:"):
            return int(line.split()[1])
    raise ValueError(f"No summary found in {outfile}")


def profile_component(binary: Path, scratch: Path, component: str, env: dict) -> dict:
    results = {}
    for reps in [1, 2]:
        out = scratch / f"{component}_{reps}.out"
        cmd = [
            "valgrind",
            "--tool=callgrind",
            f"--callgrind-out-file={out}",
            str(binary),
            component,
            str(reps),
        ]
        subprocess.run(cmd, cwd=ROOT, env=env, check=True, capture_output=True)
        results[reps] = ir_from_callgrind(out)
        out.unlink(missing_ok=True)

    delta = results[2] - results[1]
    return {
        "component": component,
        "ir_1": results[1],
        "ir_2": results[2],
        "ir_delta": delta,
    }


def main():
    parser = argparse.ArgumentParser(description="Profile PY05 setup and certification Ir")
    parser.add_argument("--output", required=True, type=Path, help="Path for PROFILE.json")
    parser.add_argument("--force", action="store_true", help="Overwrite existing output")
    args = parser.parse_args()

    if args.output.exists() and not args.force:
        raise SystemExit(f"immutable output already exists: {args.output}")

    binary = build_example()
    env = dict(os.environ, **ENV)

    valgrind_ver = subprocess.run(
        ["valgrind", "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()

    components = [
        "leja-cert-setup-cold",
        "leja-cert-setup-warm",
        "leja-cert-certification",
        "cheb-cert-setup-cold",
        "cheb-cert-setup-warm",
        "cheb-cert-certification",
        "leja-est-setup",
        "router-setup",
    ]

    scratch = Path(tempfile.mkdtemp(prefix="py05_callgrind_"))
    scratch.mkdir(parents=True, exist_ok=True)

    records = {}
    try:
        for comp in components:
            rec = profile_component(binary, scratch, comp, env)
            records[comp] = rec
            print(f"{comp}: Ir {rec['ir_delta']}")
    finally:
        for f in scratch.glob("*"):
            f.unlink(missing_ok=True)
        scratch.rmdir()

    source_commit = git("rev-parse", "HEAD")
    git_status = git("status", "--porcelain")
    norm_out = args.output.name
    tree_clean = all(
        not line.strip() or line.strip().endswith(norm_out)
        for line in git_status.splitlines()
    )

    checker_bytes = Path(__file__).read_bytes()
    checker_blob = hashlib.sha256(checker_bytes).hexdigest()

    report = {
        "schema": SCHEMA,
        "node": NODE,
        "source_commit": source_commit,
        "tree_clean": tree_clean,
        "checker_blob": checker_blob,
        "valgrind": valgrind_ver,
        "protocol": "callgrind Ir 2-minus-1",
        "components": records,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Wrote profile to {args.output}")


if __name__ == "__main__":
    main()
