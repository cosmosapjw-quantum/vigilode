#!/usr/bin/env python3
"""Loop node V24: runs research/production_loop_20261116/check_checkpoint.py
unchanged and records its verbatim outcome; then runs the same checks with
`require` replaced by a collector, in memory, so that EVERY complaint is
listed and not only the first one. Nothing of the checker, the checkpoint or
the gates is written.

usage: python3 -I checker_report.py   (from anywhere; writes CHECKER_REPORT.json)
"""
import importlib.util
import json
import os
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
CHECKER = os.path.join(NODE, "..", "check_checkpoint.py")
# What the task of node V24 allows it to edit (task text and TASK_CONTRACTS.json).
ALLOWED_BY_THE_TASK = [
    "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs",
    "crates/rodas5p-krylov/src/gmres_staged.rs",
    "crates/rodas5p-integrators/src/routing.rs",
    "crates/rodas5p-integrators/src/lib.rs",
    "crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs",
    "crates/vigilode/src/solve.rs",
    "crates/vigilode/src/solution.rs",
    "crates/vigilode/tests/default_path_golden.rs",
]
ALLOWED_PREFIXES = [
    "research/production_loop_20261116/mf_adaptive/",
    "crates/rodas5p-integrators/tests/mf_adaptive_",
    "crates/rodas5p-krylov/tests/gmres_staged_column_test_",
    "crates/vigilode/tests/golden/default_path/",
]


def allowed(path):
    return path in ALLOWED_BY_THE_TASK or any(path.startswith(prefix) for prefix in ALLOWED_PREFIXES)


def main():
    verbatim = subprocess.run([sys.executable, "-I", "research/production_loop_20261116/check_checkpoint.py"],
                              cwd=ROOT, capture_output=True, text=True, check=False)
    lines = (verbatim.stdout + verbatim.stderr).strip().splitlines()

    spec = importlib.util.spec_from_file_location("loop_checker", CHECKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    complaints = []

    def collect(condition, message):
        if not condition:
            complaints.append(message)

    module.require = collect
    printed = []
    module.print = lambda *args: printed.append(" ".join(str(arg) for arg in args))
    error = None
    try:
        module.main()
    except Exception as exception:  # a check that cannot continue without its precondition
        error = f"{type(exception).__name__}: {exception}"

    rows = []
    for message in complaints:
        path = message.split(": ", 1)[1] if ": " in message else ""
        rows.append({"complaint": message, "path": path, "path_is_allowed_by_the_task_of_V24": allowed(path)})
    changed = subprocess.run(["git", "-C", ROOT, "diff", "--name-only", "892132e5d879b5d8c2740ef7a73946fd2b1d25f9"],
                             capture_output=True, text=True, check=False).stdout.split()
    outside = [path for path in changed if not allowed(path)]
    report = {
        "schema": "vigilode.loop-l1.v24.checker-report.v1",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "git_head": subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
        "checker_unchanged_run": {"exit_code": verbatim.returncode, "last_line_verbatim": lines[-1] if lines else "", "output_tail": lines[-6:]},
        "every_complaint_with_require_collecting_in_memory": rows,
        "complaints": len(rows),
        "complaints_naming_a_path_outside_the_task": [row for row in rows if not row["path_is_allowed_by_the_task_of_V24"]],
        "collector_stopped_with": error,
        "collector_printed": printed,
        "paths_changed_since_the_base_of_the_node": len(changed),
        "changed_paths_outside_the_write_list_of_the_task": outside,
    }
    with open(os.path.join(NODE, "CHECKER_REPORT.json"), "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=1)
        handle.write("\n")
    print(json.dumps({key: report[key] for key in ("checker_unchanged_run", "complaints", "complaints_naming_a_path_outside_the_task",
                                                    "collector_stopped_with", "changed_paths_outside_the_write_list_of_the_task")}, indent=1))
    for row in rows:
        print(row["complaint"])


if __name__ == "__main__":
    sys.exit(main())
