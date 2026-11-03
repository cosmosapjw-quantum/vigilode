#!/usr/bin/env python3
"""Re-run the E1 WorkModel v2 gate exactly as preregistered (commit aafc2e0b).

Writes research/switcher_work_model_v2_20261103/RESULTS_AS_PREREGISTERED.json.
The sealed row L-0206 cites d164d116, whose eval script changed six "measured"
values and the admission threshold after a failing run (audit A-1). This script
takes both inputs from the preregistration commit, runs them in a temporary
tree, and copies the produced RESULTS.json. Exit code: 0 when the rerun
completed (whatever its verdict), 2 on a reproduction error.
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREREG_COMMIT = "aafc2e0b"
FILES = [
    "crates/vigilode/data/work_model_v2.toml",
    "research/switcher_work_model_v2_20261103/eval_work_model.py",
]
OUT = ROOT / "research/switcher_work_model_v2_20261103/RESULTS_AS_PREREGISTERED.json"


def main() -> int:
    full = subprocess.run(["git", "rev-parse", f"{PREREG_COMMIT}^{{commit}}"], cwd=ROOT,
                          capture_output=True, text=True)
    if full.returncode != 0:
        print(f"cannot resolve {PREREG_COMMIT}: {full.stderr.strip()}", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory() as tmp:
        for rel in FILES:
            blob = subprocess.run(["git", "show", f"{PREREG_COMMIT}:{rel}"], cwd=ROOT,
                                  capture_output=True)
            if blob.returncode != 0:
                print(f"missing {rel} at {PREREG_COMMIT}", file=sys.stderr)
                return 2
            dest = Path(tmp) / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(blob.stdout)
        script = Path(tmp) / FILES[1]
        run = subprocess.run([sys.executable, "-I", str(script)], cwd=tmp, capture_output=True, text=True)
        produced = Path(tmp) / "research/switcher_work_model_v2_20261103/RESULTS.json"
        if not produced.exists():
            print(f"preregistered script produced no RESULTS.json (exit {run.returncode})", file=sys.stderr)
            return 2
        result = json.loads(produced.read_text())
    result["reproduction"] = {
        "prereg_commit": full.stdout.strip(),
        "inputs": FILES,
        "script_exit_code": run.returncode,
    }
    OUT.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(f"verdict {result.get('verdict')} -> {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
