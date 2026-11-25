#!/usr/bin/env python3
"""Loop node V28, step 6 of the task (checks): runs every check once, keeps its
full output under logs/checks/<name>.log and writes CHECKS.json (exit codes and
the last lines). Nothing here changes a source file.

usage: python3 -I run_checks.py
"""
import json
import os
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
LOGS = os.path.join(NODE, "logs", "checks")
TARGET_DIR = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v28"
WORK = ["taskset", "-c", "0-5,12-17"]
CHECKS = [
    ("contract_tests", WORK + ["cargo", "+1.94.1", "test", "--locked", "-j", "4", "--release", "-p",
                               "rodas5p-integrators", "--test", "mf_budget_measure"]),
    ("clippy", WORK + ["cargo", "+1.94.1", "clippy", "--locked", "-j", "4", "--release", "--no-deps", "-p",
                       "rodas5p-integrators", "--lib", "--test", "mf_budget_measure", "--", "-D", "warnings"]),
    ("fmt", ["cargo", "+1.94.1", "fmt", "-p", "rodas5p-integrators", "-p", "vigilode", "--", "--check"]),
    ("loop_checker", ["python3", "-I", "research/production_loop_20261116/check_checkpoint.py"]),
    ("strict_json", ["python3", "-I", "-c",
                     "import json,sys\n"
                     "def reject(constant):\n"
                     "    raise ValueError(constant)\n"
                     "for path in sys.argv[1:]:\n"
                     "    with open(path, encoding='utf-8') as handle:\n"
                     "        json.load(handle, parse_constant=reject)\n"
                     "    print('strict JSON:', path)\n",
                     "research/production_loop_20261116/mf_p1_budget/PREDECLARED.json",
                     "research/production_loop_20261116/mf_p1_budget/PROTOCOL.json",
                     "research/production_loop_20261116/mf_p1_budget/RESULTS.json"]),
    ("results_regenerate", ["python3", "-I", "research/production_loop_20261116/mf_p1_budget/parse_results.py",
                            "--check"]),
]


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=False).stdout.strip()


def main():
    os.makedirs(LOGS, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=TARGET_DIR)
    head = git("rev-parse", "HEAD")
    dirty = len([line for line in git("status", "--porcelain", "--untracked-files=no").splitlines() if line])
    summary = {"schema": "vigilode.loop-l1.v28.checks.v1", "git_head": head, "git_dirty_tracked": dirty, "checks": []}
    for name, command in CHECKS:
        start = time.time()
        completed = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, check=False)
        output = completed.stdout + completed.stderr
        with open(os.path.join(LOGS, name + ".log"), "w", encoding="utf-8") as handle:
            handle.write(f"# command: {' '.join(command)}\n# git head {head}, dirty tracked {dirty}\n")
            handle.write(output)
            handle.write(f"# exit code {completed.returncode} after {time.time() - start:.1f} s\n")
        tail = [line for line in output.splitlines() if line.strip()][-3:]
        summary["checks"].append({"name": name, "command": " ".join(command), "exit_code": completed.returncode,
                                  "last_lines": tail, "log": f"logs/checks/{name}.log"})
        print(f"{name}: exit {completed.returncode}", flush=True)
    with open(os.path.join(NODE, "CHECKS.json"), "w", encoding="utf-8") as handle:
        handle.write(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    return 0 if all(check["exit_code"] == 0 for check in summary["checks"]) else 1


if __name__ == "__main__":
    sys.exit(main())
