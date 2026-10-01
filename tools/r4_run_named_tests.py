#!/usr/bin/env python3
"""Run the named tests of the R4 runtime remainder plan one at a time (re-audit R4 of
2026-10-01, R4-VERIFY-DEV-01).

Reads ``research/adversarial_reaudit_20261001_r4/evidence/runtime/LOCAL_REMAINDER_PLAN.json``
(20 bounded-timeout and 40 budget-skipped named tests), builds each target once, then runs
every test with ``-- --exact <name> --test-threads 1`` under a per-test wall budget. Each
test gets one of PASS, FAIL (nonzero exit before the budget), TIMEOUT (budget reached; never
counted as a pass) or NOT_FOUND (the exact filter matched no test). The plan's ignored tests
are listed, not run (one writes a tracked fixture). Output: one JSON document.

Usage: python3 tools/r4_run_named_tests.py --budget-seconds 3600 --output <path>
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "research/adversarial_reaudit_20261001_r4/evidence/runtime/LOCAL_REMAINDER_PLAN.json"


def crate_of(target):
    hits = sorted(ROOT.glob(f"crates/*/tests/{target}.rs"))
    if len(hits) != 1:
        raise SystemExit(f"target {target}: {len(hits)} test files")
    return hits[0].parent.parent.name


def run(command, budget):
    started = time.monotonic()
    try:
        done = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=budget)
        return done.returncode, done.stdout + done.stderr, time.monotonic() - started, False
    except subprocess.TimeoutExpired as expired:
        out = (expired.stdout or b"") + (expired.stderr or b"")
        if isinstance(out, bytes):
            out = out.decode(errors="replace")
        return None, out, time.monotonic() - started, True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--budget-seconds", type=float, required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    output = Path(args.output)
    if output.exists():
        raise SystemExit(f"immutable output exists: {output}")
    plan = json.loads(PLAN.read_text())
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    rows = []
    for entry in plan["targets"]:
        target = entry["target"]
        crate = crate_of(target)
        build = ["cargo", "test", "-q", "-p", crate, "--test", target, "--no-run"]
        code, log, seconds, _ = run(build, None)
        if code != 0:
            raise SystemExit(f"build of {target} failed:\n{log[-4000:]}")
        for name in entry["tests"]:
            command = ["cargo", "test", "-q", "-p", crate, "--test", target, "--", "--exact", name, "--test-threads", "1"]
            code, log, seconds, timed_out = run(command, args.budget_seconds)
            passed = re.search(r"test result: ok\. (\d+) passed", log)
            if timed_out:
                status = "TIMEOUT"
            elif code == 0 and passed and int(passed.group(1)) == 1:
                status = "PASS"
            elif code == 0:
                status = "NOT_FOUND"
            else:
                status = "FAIL"
            rows.append(dict(target=target, crate=crate, test=name, plan_status=entry["status"], status=status,
                             wall_seconds=round(seconds, 3), exit_code=code,
                             log_tail=log[-1500:] if status != "PASS" else ""))
            print(f"{status:9} {seconds:9.1f}s {target}::{name}", flush=True)
    counts = {s: sum(r["status"] == s for r in rows) for s in ("PASS", "FAIL", "TIMEOUT", "NOT_FOUND")}
    report = dict(
        schema="vigilode-r4-runtime-remainder-v1",
        execution_commit=commit,
        profile=dict(name="debug", incremental=os.environ.get("CARGO_INCREMENTAL"), debug=os.environ.get("CARGO_PROFILE_TEST_DEBUG"),
                     features="default (plan: all; the named tests need no feature; changed identity disclosed)",
                     test_threads=1, rayon_threads=os.environ.get("RAYON_NUM_THREADS")),
        budget_seconds_per_test=args.budget_seconds,
        tests=rows,
        counts=counts,
        total=len(rows),
        ignored_not_run=plan["ignored"],
        verdict="PASS" if counts["PASS"] == len(rows) else "FAIL",
    )
    output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps(dict(counts=counts, verdict=report["verdict"])))


if __name__ == "__main__":
    main()
