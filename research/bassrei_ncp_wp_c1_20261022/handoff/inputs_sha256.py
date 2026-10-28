#!/usr/bin/env python3
"""inputs_sha256 of an NCP C1 attempt at its EXEC commit (AGENTS.md 2.7 step 2).

    python3 research/bassrei_ncp_wp_c1_20261022/handoff/inputs_sha256.py EXEC > OUT.json

Hashes `git show EXEC:<path>` for the same file set as attempt2_inputs_sha256.json
(the campaign harness, the node's PREREGISTRATION.md, the runner and library
sources the campaign builds, and Cargo.lock).
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
exec_sha = subprocess.run(["git", "rev-parse", sys.argv[1]], capture_output=True, text=True, check=True).stdout.strip()
paths = sorted(json.loads((HERE / "attempt2_inputs_sha256.json").read_text())["inputs_sha256"])
out = {}
for p in paths:
    blob = subprocess.run(["git", "show", f"{exec_sha}:{p}"], capture_output=True, check=True).stdout
    out[p] = hashlib.sha256(blob).hexdigest()
print(json.dumps({"exec_sha": exec_sha, "inputs_sha256": out}, indent=1))
