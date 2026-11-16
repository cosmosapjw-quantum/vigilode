#!/usr/bin/env python3
"""Loop L1 node V02 (trial merge): parse raw `cargo test` logs into strict JSON.

Reads the logs written by run_gate.sh and sums the libtest "test result:" lines.
Every number in the output comes from a log line; nothing is typed in.

Usage: python3 -I parse_test_logs.py <out.json> <log> [<log> ...]
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

RESULT = re.compile(
    r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)
RUNNING = re.compile(r"^\s+Running (?:unittests )?(\S+) \((\S+)\)|^\s+Doc-tests (\S+)")


def binary_label(m: re.Match) -> str:
    """`<executable name without hash>:<source path>`, or `doc-tests <crate>`."""
    if m.group(3):
        return f"doc-tests {m.group(3)}"
    exe = m.group(2).rsplit("/", 1)[-1].rsplit("-", 1)[0]
    return f"{exe}:{m.group(1)}"
FAILED_TEST = re.compile(r"^test (\S+) \.\.\. FAILED")
FAILED_BIN = re.compile(r"^error: test failed, to rerun pass `(.+)`")


def parse(path: Path) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace")
    binaries, current = [], None
    header = {}
    failed_tests, failed_bins = [], []
    for line in text.splitlines():
        if line.startswith("# "):
            key, _, value = line[2:].partition(": ")
            header[key] = value
            continue
        m = RUNNING.match(line)
        if m:
            current = binary_label(m)
            continue
        m = FAILED_TEST.match(line)
        if m:
            failed_tests.append({"binary": current, "test": m.group(1)})
            continue
        m = FAILED_BIN.match(line)
        if m:
            failed_bins.append(m.group(1))
            continue
        m = RESULT.match(line)
        if m:
            binaries.append(
                {
                    "binary": current,
                    "status": m.group(1),
                    "passed": int(m.group(2)),
                    "failed": int(m.group(3)),
                    "ignored": int(m.group(4)),
                    "measured": int(m.group(5)),
                    "filtered_out": int(m.group(6)),
                }
            )
    return {
        "log": str(path),
        "log_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "command": header.get("cmd"),
        "head": header.get("head"),
        "toolchain": header.get("toolchain"),
        "start": header.get("start"),
        "end": header.get("end"),
        "exit_code": int(header["exit_code"]) if "exit_code" in header else None,
        "test_binaries": len(binaries),
        "passed": sum(b["passed"] for b in binaries),
        "failed": sum(b["failed"] for b in binaries),
        "ignored": sum(b["ignored"] for b in binaries),
        "filtered_out": sum(b["filtered_out"] for b in binaries),
        "failed_tests": failed_tests,
        "failed_binaries": failed_bins,
        "binaries": binaries,
    }


def main() -> int:
    out_path = Path(sys.argv[1])
    root = Path(__file__).resolve().parents[3]
    suites = []
    for arg in sys.argv[2:]:
        p = Path(arg).resolve()
        row = parse(p)
        row["log"] = p.relative_to(root).as_posix()
        suites.append(row)
    out = {
        "schema": "vigilode.loop-l1.trial-merge.test-logs.v1",
        "generator": "research/production_loop_20261116/trial_merge/parse_test_logs.py",
        "wall_time": "diagnostic_only",
        "suites": suites,
    }
    out_path.write_text(json.dumps(out, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    for s in suites:
        print(
            f"{s['log']}: exit={s['exit_code']} binaries={s['test_binaries']} "
            f"passed={s['passed']} failed={s['failed']} ignored={s['ignored']}"
        )
        for f in s["failed_tests"]:
            print(f"   FAILED {f['binary']} :: {f['test']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
