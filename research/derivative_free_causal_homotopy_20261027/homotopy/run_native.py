#!/usr/bin/env python3
"""Run the focused native contracts and preregistered work experiment once."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRATCH = REPO.parent
ENV = os.environ.copy()
ENV.update({
    "PATH": str(SCRATCH / "tmp/rust1941/bin") + os.pathsep + ENV["PATH"],
    "CARGO_HOME": str(SCRATCH / "tmp/cargo_home"),
    "CARGO_TARGET_DIR": str(SCRATCH / "tmp/vigil_target"),
    "CARGO_BUILD_JOBS": "2",
    "RAYON_NUM_THREADS": "1",
    "OMP_NUM_THREADS": "1",
    "OPENBLAS_NUM_THREADS": "1",
})

commands = [
    ("native_contracts", ["cargo", "test", "-p", "vigilode-coupled", "--profile", "measurement", "--locked", "--offline", "--test", "rhs_homotopy_contracts", "--", "--test-threads=1"]),
    ("native_build", ["cargo", "build", "-p", "vigilode-coupled", "--profile", "measurement", "--locked", "--offline", "--example", "rhs_homotopy_bench_20261027"]),
    ("native_experiment", [str(SCRATCH / "tmp/vigil_target/measurement/examples/rhs_homotopy_bench_20261027")]),
]
receipt = {"started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(), "source": {}, "commands": []}
for relative in ["crates/vigilode-coupled/src/rhs_homotopy.rs", "crates/vigilode-coupled/tests/rhs_homotopy_contracts.rs", "crates/vigilode-coupled/examples/rhs_homotopy_bench_20261027.rs"]:
    receipt["source"][relative] = hashlib.sha256((REPO / relative).read_bytes()).hexdigest()
for name, command in commands:
    start = time.monotonic()
    completed = subprocess.run(command, cwd=REPO, env=ENV, capture_output=True, text=True, timeout=300)
    (HERE / (name + ".stdout.log")).write_text(completed.stdout)
    (HERE / (name + ".stderr.log")).write_text(completed.stderr)
    entry = {"name": name, "command": command, "exit_code": completed.returncode, "elapsed_seconds": time.monotonic()-start}
    receipt["commands"].append(entry)
    (HERE / "EXECUTION_RECEIPT.json").write_text(json.dumps(receipt, indent=2)+"\n")
    print(json.dumps(entry), flush=True)
    if completed.returncode:
        print(completed.stdout[-4000:] + completed.stderr[-4000:], flush=True)
        raise SystemExit(completed.returncode)
    if name == "native_experiment":
        data = json.loads(completed.stdout)
        (HERE / "RESULTS.json").write_text(json.dumps(data, indent=2)+"\n")
        binary = Path(command[0])
        receipt["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
receipt["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
(HERE / "EXECUTION_RECEIPT.json").write_text(json.dumps(receipt, indent=2)+"\n")
