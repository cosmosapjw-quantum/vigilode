#!/usr/bin/env python3
"""Run the registered follow-up pairs serially with one binary (in-process ABBA blocks).

`--identity` runs every packet pair with one block only: its records are compared
with the packet raw, and its timings are never analysed.
"""
import argparse
import hashlib
import json
import os
import platform
import subprocess
import time
from pathlib import Path

PACKET_CASES = ["rotation", "decay", "case5", "stiff-forced", "relaxation",
                "transition-39", "transition-79", "gyro-tight", "gyro-default"]
CONTROLS = {c: ("dopri" if c in {"rotation", "decay", "gyro-tight", "gyro-default"} else "rodas")
            for c in PACKET_CASES}
# Registered timed pairs (PREREGISTRATION.md, M3).
FOLLOWUP = [
    ("stale-return", "adaptive", "adaptive-sentinel"),
    ("stale-return", "adaptive-sentinel", "dopri"),
    ("stale-return", "adaptive-sentinel", "rodas"),
    ("case5", "adaptive", "adaptive-sentinel"),
    ("stiff-forced", "adaptive", "adaptive-sentinel"),
    ("relaxation", "adaptive", "adaptive-sentinel"),
    ("transition-39", "adaptive", "adaptive-sentinel"),
    ("case5", "adaptive", "rodas"),
    ("case5", "adaptive", "rodas-dense"),
    ("case5", "rodas", "rodas-dense"),
    ("transition-39", "adaptive", "rodas"),
    ("transition-79", "adaptive", "rodas"),
    ("rotation", "adaptive", "dopri"),
    ("gyro-tight", "adaptive", "dopri"),
]


def packet_pairs():
    pairs = []
    for case in PACKET_CASES:
        pairs.extend([(case, "evidence", "adaptive"), (case, "adaptive", CONTROLS[case])])
        if case in {"case5", "stiff-forced", "relaxation", "transition-39", "transition-79"}:
            pairs.append((case, "adaptive-unseeded", "adaptive"))
        if case == "case5":
            pairs.extend([(case, "adaptive", "rodas-dense"), (case, "rodas", "rodas-dense"),
                          (case, "adaptive-no-startup-retry", "adaptive")])
    return pairs


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("binary", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--identity", action="store_true")
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    repo = Path(__file__).resolve().parents[3]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    env = os.environ.copy()
    env.update(RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1",
               VIGILODE_SOURCE_COMMIT=commit)
    pairs, blocks = (packet_pairs(), 1) if a.identity else (FOLLOWUP, 5)
    manifest = {"schema": "vigilode.adapter-followup-run.v1", "commit": commit,
                "binary_sha256": hashlib.sha256(a.binary.read_bytes()).hexdigest(),
                "source_dirty": subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True),
                "host": platform.platform(), "cpu_count": os.cpu_count(), "blocks": blocks,
                "mode": "identity-only (timings not analysed)" if a.identity else "registered timed pairs",
                "timing_authority": "diagnostic_only", "runs": []}
    for case, x, y in pairs:
        stem = f"{case}__{x}__{y}"
        started = time.monotonic()
        try:
            r = subprocess.run([str(a.binary), case, x, y, str(blocks)], text=True,
                               capture_output=True, env=env, timeout=120)
            out, err, code = r.stdout, r.stderr, r.returncode
        except subprocess.TimeoutExpired as e:
            out, err, code = str(e.stdout or ""), str(e.stderr or ""), "TIMEOUT"
        (a.output / f"{stem}.stdout.json").write_text(out)
        (a.output / f"{stem}.stderr.log").write_text(err)
        manifest["runs"].append({"case": case, "a": x, "b": y, "exit": code,
                                 "wall_seconds": time.monotonic() - started})
        (a.output / "RUN.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print(stem, code, flush=True)


if __name__ == "__main__":
    main()
