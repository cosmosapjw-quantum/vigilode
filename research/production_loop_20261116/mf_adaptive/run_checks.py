#!/usr/bin/env python3
"""Loop node V24: runs the suites of discriminator R (PROTOCOL.json,
discriminator_protocol.R.targets_fixed_now), one cargo process at a time,
keeps every output under logs/checks/ and writes R_SUITES.json from those
outputs. Nothing is typed in: counts and failing tests are parsed from the
logs.

usage: python3 -I run_checks.py [--only NAME,NAME] [--summary-only]
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
TARGET = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v24"
CPUS = "0-5,12-17"
CARGO = ["cargo", "+1.94.1"]
TEST = CARGO + ["test", "--locked", "-j", "4", "--release", "--no-fail-fast"]
MF_TARGETS = [
    "a1_zero_tolerance_scale", "alg01_coupled_stage_target", "alg04_alg06", "audit2_matrix_free_common_w_contracts",
    "declared_structure_routing", "int01_gcrodr_verified_reuse", "int01_posthoc_tolerance",
    "krylov_workspace_driver_contract", "matrix_free_problem_contracts", "native_reaudit_cache_contracts",
    "p0_partial_contracts", "residual_acceptance_contract", "rev03_export", "rnext01_residual_output",
    "rnext02_gmres_into_study", "rnext03_gcrodr_attribution", "rnext03_posthoc_invariant", "rnext07_model_epoch",
    "safe_recycle_policy", "shared_basis_mf_contracts", "sp01_dupfix_adoption", "spd04_ls_workspace_study",
    "spd07_mf_step_warm_start", "stage_residual_telemetry", "thread_transfer_mf_workspace",
    "time_fd_workspace_contracts", "zero_start_policy_holdout",
]
NEW_INTEGRATOR_TESTS = ["mf_adaptive_measure", "mf_adaptive_contracts", "mf_adaptive_shipped_bits", "ppsw1_6b_contracts"]


def commands():
    integrators = TEST + ["-p", "rodas5p-integrators"]
    mf = integrators[:]
    for target in MF_TARGETS:
        mf += ["--test", target]
    clippy_integrators = CARGO + ["clippy", "--locked", "-j", "4", "--release", "-p", "rodas5p-integrators", "--lib"]
    for target in NEW_INTEGRATOR_TESTS:
        clippy_integrators += ["--test", target]
    return [
        ("krylov_all", "suite", TEST + ["-p", "rodas5p-krylov"]),
        ("sentinels", "suite", integrators + ["--test", "ppsw1_6b_contracts", "--test", "ptsw2b_contracts"]),
        ("stage_clients", "suite", integrators + ["--test", "stage_clients_*"]),
        ("mf_targets", "suite", mf),
        ("integrators_lib", "suite", integrators + ["--lib"]),
        ("mf_adaptive", "suite", integrators + ["--test", "mf_adaptive_*"]),
        ("facade", "suite", TEST + ["-p", "vigilode"]),
        ("clippy_krylov_vigilode", "static", CARGO + ["clippy", "--locked", "-j", "4", "--release", "-p", "rodas5p-krylov",
                                                     "-p", "vigilode", "--all-targets", "--no-deps", "--", "-D", "warnings"]),
        ("clippy_integrators", "static", clippy_integrators + ["--no-deps", "--", "-D", "warnings"]),
        ("fmt", "static", CARGO + ["fmt", "-p", "rodas5p-krylov", "-p", "rodas5p-integrators", "-p", "vigilode", "--", "--check"]),
        ("workspace_check", "extra", CARGO + ["check", "--locked", "-j", "4", "--workspace", "--all-targets"]),
        ("loop_checker", "checker", ["python3", "-I", "research/production_loop_20261116/check_checkpoint.py"]),
    ]


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=False).stdout.strip()


def run(name, command):
    os.makedirs(LOGS, exist_ok=True)
    path = os.path.join(LOGS, name + ".log")
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = TARGET
    # Tracked files that differ from HEAD, not counting what this script itself rewrites (its logs and summary).
    own = [":!research/production_loop_20261116/mf_adaptive/logs/checks", ":!research/production_loop_20261116/mf_adaptive/R_SUITES.json"]
    dirty = [line for line in git("status", "--porcelain", "--untracked-files=no", "--", ".", *own).splitlines() if line]
    header = {"name": name, "command": command, "cpus": CPUS, "git_head": git("rev-parse", "HEAD"),
              "git_dirty_tracked": len(dirty),
              "date": time.strftime("%Y-%m-%dT%H:%M:%S%z")}
    start = time.time()
    with open(path, "w", encoding="utf-8") as log:
        log.write("# V24_CHECK_BEGIN " + json.dumps(header) + "\n")
        log.flush()
        completed = subprocess.run(["taskset", "-c", CPUS, *command], cwd=ROOT, env=env, stdout=log,
                                   stderr=subprocess.STDOUT, check=False)
        log.write("# V24_CHECK_END " + json.dumps({"exit_code": completed.returncode, "seconds": time.time() - start,
                                                   "date": time.strftime("%Y-%m-%dT%H:%M:%S%z")}) + "\n")
    print(f"{name}: exit {completed.returncode} in {time.time() - start:.0f} s", flush=True)


RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
RUNNING = re.compile(r"^\s+Running (\S+)")
FAILED = re.compile(r"^test (\S+) \.\.\. FAILED")


def summarise(name, kind):
    path = os.path.join(LOGS, name + ".log")
    if not os.path.exists(path):
        return {"name": name, "kind": kind, "present": False}
    out = {"name": name, "kind": kind, "present": True, "log": os.path.relpath(path, NODE), "binaries": 0,
           "passed": 0, "failed": 0, "ignored": 0, "failing_tests": [], "panics": []}
    current = None
    tail = []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("# V24_CHECK_BEGIN "):
                begin = json.loads(line[len("# V24_CHECK_BEGIN "):])
                out.update({"command": " ".join(begin["command"]), "git_head": begin["git_head"],
                            "git_dirty_tracked": begin["git_dirty_tracked"]})
                continue
            if line.startswith("# V24_CHECK_END "):
                end = json.loads(line[len("# V24_CHECK_END "):])
                out.update({"exit_code": end["exit_code"], "seconds": end["seconds"]})
                continue
            tail.append(line)
            match = RUNNING.match(line)
            if match:
                current = match.group(1)
            match = RESULT.match(line)
            if match:
                out["binaries"] += 1
                out["passed"] += int(match.group(2))
                out["failed"] += int(match.group(3))
                out["ignored"] += int(match.group(4))
            match = FAILED.match(line)
            if match:
                out["failing_tests"].append({"binary": current, "test": match.group(1)})
            if "panicked at" in line or line.startswith("assertion ") or line.startswith("ValueError") or line.startswith("PASS:"):
                if len(out["panics"]) < 60:
                    out["panics"].append(line[:400])
    out["last_lines"] = [line[:300] for line in tail[-6:]]
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", default="")
    parser.add_argument("--summary-only", action="store_true")
    parser.add_argument("--label", default="", help="suffix of the summary file (R_SUITES<label>.json)")
    args = parser.parse_args()
    only = [name for name in args.only.split(",") if name]
    if not args.summary_only:
        for name, _kind, command in commands():
            if only and name not in only:
                continue
            run(name, command)
    rows = [summarise(name, kind) for name, kind, _command in commands()]
    deciding = [row for row in rows if row["kind"] in ("suite", "static")]
    missing = [row["name"] for row in deciding if not row["present"]]
    red = [row["name"] for row in deciding if row["present"] and row.get("exit_code") != 0]
    failing = [f"{row['name']}: {test['binary']} {test['test']}" for row in rows for test in row.get("failing_tests", [])]
    if missing:
        verdict = "NOT_RUN"
    else:
        verdict = "PASS" if not red else "FAIL"
    summary = {
        "schema": "vigilode.loop-l1.v24.r-suites.v1",
        "node": "V24_MF_ADAPTIVE_STAGE_SOLVE",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "git_head_now": git("rev-parse", "HEAD"),
        "verdict": verdict,
        "summary": {
            "commands": len(deciding), "not_run": missing, "not_exit_0": red, "failing_tests": failing,
            "tests_passed": sum(row.get("passed", 0) for row in rows), "tests_failed": sum(row.get("failed", 0) for row in rows),
            "tests_ignored": sum(row.get("ignored", 0) for row in rows),
        },
        "rows": rows,
    }
    path = os.path.join(NODE, f"R_SUITES{args.label}.json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=1)
        handle.write("\n")
    print(json.dumps(summary["summary"], indent=1))
    print("verdict", verdict)


if __name__ == "__main__":
    sys.exit(main())
