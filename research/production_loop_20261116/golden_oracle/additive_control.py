#!/usr/bin/env python3
"""Control of the added-key rule of the V03G oracle.

One key is deleted from a golden file, so the run holds a key the golden file
lacks. Without VIGILODE_DEFAULT_PATH_ALLOW_ADDED_KEYS the test must fail and
report exactly one added path; with the variable set to 1 it must pass. The
file is restored after each run. Results go to logs/additive_control.json.
"""
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PATH = ROOT / "crates/vigilode/tests/golden/default_path/dopri5.json"
TEST = [
    "cargo", "+1.94.1", "test", "--locked", "-j", "6", "--release",
    "-p", "vigilode", "--test", "default_path_golden",
]
ALLOW = "VIGILODE_DEFAULT_PATH_ALLOW_ADDED_KEYS=1"


def main():
    stat = PATH.stat()
    original = PATH.read_bytes()
    sha = hashlib.sha256(original).hexdigest()
    text = original.decode()
    m = re.search(r'^ *"n_outputs": \d+,\n', text, flags=re.M)
    mutated = text[: m.start()] + text[m.end():]
    json.loads(mutated)
    results = []
    for label, env in (("additive_control_strict", []), ("additive_control_allowed", [ALLOW])):
        try:
            PATH.write_bytes(mutated.encode())
            step = subprocess.run(
                [sys.executable, str(HERE / "run_step.py"), label, *env, "--", *TEST],
                capture_output=True, text=True,
            )
        finally:
            PATH.write_bytes(original)
            os.utime(PATH, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        record = json.loads(step.stdout.strip().splitlines()[-1])
        log = (ROOT / record["log"]).read_text(errors="replace")
        results.append({
            "run": label,
            "golden_file": PATH.name,
            "deleted_golden_line": m.group(0).strip(),
            "allow_added_keys": bool(env),
            "journal_seq": record["seq"],
            "exit_code": record["exit_code"],
            "failed_tests": sorted(re.findall(r"^test (\w+) \.\.\. FAILED", log, flags=re.M)),
            "reported_path_counts": {
                kind: [int(n) for n in re.findall(rf"^{kind}: (\d+)$", log, flags=re.M)]
                for kind in ("changed", "removed", "added")
            },
            "golden_file_restored": hashlib.sha256(PATH.read_bytes()).hexdigest() == sha,
        })
        print(json.dumps(results[-1]))
    strict, allowed = results
    ok = (
        strict["exit_code"] != 0
        and strict["failed_tests"] == ["dopri5"]
        and strict["reported_path_counts"] == {"changed": [0], "removed": [0], "added": [1]}
        and allowed["exit_code"] == 0
        and all(r["golden_file_restored"] for r in results)
    )
    out = HERE / "logs" / "additive_control.json"
    out.write_text(
        json.dumps({"as_expected": ok, "runs": results}, indent=2, sort_keys=True, allow_nan=False) + "\n"
    )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
