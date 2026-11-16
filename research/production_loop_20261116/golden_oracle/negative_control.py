#!/usr/bin/env python3
"""Negative control of the V03G oracle: a one-character change of a golden
file must fail exactly the test that owns the file.

Each mutation is applied to the committed golden file, the release test is
run through run_step.py (so the run is journaled with its nonzero exit code),
and the file is restored and its sha256 checked. Results go to
logs/negative_control.json.
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
GOLDEN = ROOT / "crates/vigilode/tests/golden/default_path"
TEST = [
    "cargo", "+1.94.1", "test", "--locked", "-j", "6", "--release",
    "-p", "vigilode", "--test", "default_path_golden",
]


def flip_hex(digit):
    return format(int(digit, 16) ^ 1, "x")


def mutate_state_bit(text):
    """Flip the lowest bit of the first final-state word."""
    m = re.search(r'"final_state_bits": \["([0-9a-f]{15})([0-9a-f])"', text)
    return m.start(2), m.group(2), flip_hex(m.group(2))


def mutate_counter(text):
    """Add one to the first right-hand-side call counter."""
    m = re.search(r'"rhs_calls": (\d+)', text)
    return m.start(1), m.group(1), str(int(m.group(1)) + 1)


def mutate_dense_sample(text):
    """Flip the lowest bit of the first interpolated state word."""
    m = re.search(r'"state_bits": \["([0-9a-f]{15})([0-9a-f])"', text)
    return m.start(2), m.group(2), flip_hex(m.group(2))


def mutate_grid_cell(text):
    """Replace the first decision code of the first select row."""
    section = text.index('"select"')
    m = re.compile(r'"dense_output=\d [^"]*": "(\w)').search(text, text.index('"rows"', section))
    old = m.group(1)
    return m.start(1), old, "0" if old != "0" else "1"


MUTATIONS = [
    ("final_state_bit", "rodas5p_robertson", mutate_state_bit),
    ("work_counter", "rodas5p_hires", mutate_counter),
    ("dense_sample_bit", "rodas5p_brusselator", mutate_dense_sample),
    ("explicit_counter", "dopri5", mutate_counter),
    ("grid_decision", "decision_grid", mutate_grid_cell),
]


def main():
    results = []
    for label, stem, mutate in MUTATIONS:
        path = GOLDEN / f"{stem}.json"
        stat = path.stat()
        original = path.read_bytes()
        sha = hashlib.sha256(original).hexdigest()
        text = original.decode()
        offset, old, new = mutate(text)
        mutated = text[:offset] + new + text[offset + len(old):]
        line = text.count("\n", 0, offset) + 1
        try:
            path.write_bytes(mutated.encode())
            step = subprocess.run(
                [sys.executable, str(HERE / "run_step.py"), f"negative_control_{label}", "--", *TEST],
                capture_output=True, text=True,
            )
        finally:
            # Restore the bytes and the modification time of the blessed file.
            path.write_bytes(original)
            os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        record = json.loads(step.stdout.strip().splitlines()[-1])
        log = (ROOT / record["log"]).read_text(errors="replace")
        failed = sorted(re.findall(r"^test (\w+) \.\.\. FAILED", log, flags=re.M))
        counts = {
            kind: [int(n) for n in re.findall(rf"^{kind}: (\d+)$", log, flags=re.M)]
            for kind in ("changed", "removed", "added")
        }
        restored = hashlib.sha256(path.read_bytes()).hexdigest() == sha
        results.append({
            "mutation": label,
            "golden_file": path.name,
            "line": line,
            "golden_text": old,
            "mutated_text": new,
            "journal_seq": record["seq"],
            "exit_code": record["exit_code"],
            "failed_tests": failed,
            "reported_path_counts": counts,
            "detected": record["exit_code"] != 0 and failed == [stem],
            "golden_file_restored": restored,
        })
        print(json.dumps(results[-1]))
    out = HERE / "logs" / "negative_control.json"
    out.write_text(json.dumps(results, indent=2, sort_keys=True, allow_nan=False) + "\n")
    return 0 if all(r["detected"] and r["golden_file_restored"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
