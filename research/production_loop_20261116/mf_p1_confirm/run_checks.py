#!/usr/bin/env python3
"""Loop node V25: runs the checks of the task (tests, clippy, rustfmt, the
loop checker), one cargo process at a time, keeps every output under
logs/checks/ and writes CHECKS.json from those outputs (exit codes and the
test counts parsed from the logs; nothing typed in). The verdict was KEEP, so
no default changed and these checks are hygiene, not discriminator R (which
is defined only after a switch).

usage: python3 -I run_checks.py [--only NAME,NAME]
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
LOGS = os.path.join(NODE, "logs", "checks")
TARGET = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v25"
CPUS = "0-5,12-17"
CARGO = ["cargo", "+1.94.1"]
TEST = CARGO + ["test", "--locked", "-j", "4", "--release", "--no-fail-fast"]
CLIPPY = CARGO + ["clippy", "--locked", "-j", "4", "--release", "--no-deps"]


def commands():
    return [
        ("fmt", CARGO + ["fmt", "-p", "rodas5p-krylov", "-p", "rodas5p-integrators", "-p", "vigilode", "--", "--check"]),
        ("clippy_integrators_lib_and_mf_confirm", CLIPPY + ["-p", "rodas5p-integrators", "--lib", "--test", "mf_confirm_contracts",
                                                           "--test", "mf_confirm_measure", "--test", "mf_confirm_problems",
                                                           "--", "-D", "warnings"]),
        ("clippy_krylov_all_targets", CLIPPY + ["-p", "rodas5p-krylov", "--all-targets", "--", "-D", "warnings"]),
        ("clippy_vigilode_all_targets", CLIPPY + ["-p", "vigilode", "--all-targets", "--", "-D", "warnings"]),
        ("integrators_mf_sentinels_stage_clients", TEST + ["-p", "rodas5p-integrators", "--test", "mf_confirm_*", "--test", "mf_adaptive_*",
                                                           "--test", "ppsw1_6b_contracts", "--test", "ptsw2b_contracts",
                                                           "--test", "stage_clients_*"]),
        ("krylov_suite", TEST + ["-p", "rodas5p-krylov"]),
        ("facade_golden_oracle", TEST + ["-p", "vigilode", "--test", "default_path_golden"]),
        ("facade_suite", TEST + ["-p", "vigilode"]),
        ("loop_checker", ["python3", "-I", "research/production_loop_20261116/check_checkpoint.py"]),
    ]


RESULT = re.compile(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored")
FAILED = re.compile(r"^test (\S+) \.\.\. FAILED")


def summarise(text):
    passed = failed = ignored = binaries = 0
    for match in RESULT.finditer(text):
        binaries += 1
        passed += int(match.group(2))
        failed += int(match.group(3))
        ignored += int(match.group(4))
    failures = [match.group(1) for line in text.splitlines() if (match := FAILED.match(line))]
    return {"result_lines": binaries, "passed": passed, "failed": failed, "ignored": ignored, "failing_tests": failures}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", default="")
    args = parser.parse_args()
    only = [name for name in args.only.split(",") if name]
    os.makedirs(LOGS, exist_ok=True)
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = TARGET
    record_path = os.path.join(NODE, "CHECKS.json")
    record = {"schema": "vigilode.loop-l1.v25.checks.v1", "checks": {}}
    if os.path.exists(record_path):
        with open(record_path, encoding="utf-8") as handle:
            record = json.load(handle)
    for name, command in commands():
        if only and name not in only:
            continue
        full = ["taskset", "-c", CPUS, *command]
        start = time.time()
        completed = subprocess.run(full, cwd=ROOT, env=env, capture_output=True, text=True, check=False)
        text = completed.stdout + completed.stderr
        with open(os.path.join(LOGS, name + ".log"), "w", encoding="utf-8") as log:
            log.write("# V25_CHECK " + json.dumps({"name": name, "command": full, "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
                                                   "git_head": subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True,
                                                                              text=True, check=False).stdout.strip()}) + "\n")
            log.write(text)
            log.write("# V25_CHECK_END " + json.dumps({"exit_code": completed.returncode, "seconds": time.time() - start}) + "\n")
        entry = {"command": " ".join(full), "exit_code": completed.returncode, "seconds": round(time.time() - start, 1),
                 "log": os.path.relpath(os.path.join(LOGS, name + ".log"), NODE), **summarise(text)}
        record["checks"][name] = entry
        print(f"{name}: exit {completed.returncode} passed {entry['passed']} failed {entry['failed']} ignored {entry['ignored']} "
              f"in {entry['seconds']} s", flush=True)
        with open(record_path, "w", encoding="utf-8") as handle:
            json.dump(record, handle, indent=1, allow_nan=False)
            handle.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
