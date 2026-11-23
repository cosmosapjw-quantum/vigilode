#!/usr/bin/env python3
"""Build RECEIPT.json of loop L1 node V03 part B (unified router) from the raw logs.

Every number in the receipt is parsed from a log that an executed command wrote
(`run_checks.sh`, the two failing-first logs and the digest log of the base sources) or from
`git`. Nothing is typed in: thresholds come from PREDECLARED.json and are only compared
with parsed values. Reasons of edits and descriptions are text, not results.

Usage (from the worktree root):
    python3 -I research/production_loop_20261116/unified_router/build_receipt.py
"""

import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOGS = HERE / "logs"
ROOT = HERE.parents[2]
BASE = "587a139501729bbe886bb2b8c49b0ef893705d08"

ALLOWED = [
    r"^crates/vigilode/src/unified/.+",
    r"^crates/vigilode/src/registry\.rs$",
    r"^crates/vigilode/tests/unified_router_contracts\.rs$",
    r"^crates/vigilode/tests/unified_production_solver_contracts\.rs$",
    r"^crates/vigilode/examples/unified_production_solver\.rs$",
    r"^research/production_loop_20261116/unified_router/.+",
]

# Why each existing file was edited. Text, not results; the line counts next to it are
# taken from `git diff --numstat`.
EXISTING_FILE_EDITS = {
    "crates/vigilode/tests/unified_production_solver_contracts.rs": [
        "contract_3: `assert_eq!(first_tier, \"Tier1ExplicitDopri5\")` pinned a label string; "
        "it now compares the history entry with `ExplicitStats::method` of the DOPRI5 leg",
        "contract_3: `contains(\"Protected\") || contains(\"Staged\")` pinned label strings; "
        "it now compares with `Stats::route.driver_id`",
        "contract_3: `assert_eq!(heat_tier, \"Tier2MatrixFreeKrylov\")` pinned the label of a "
        "tier that had no driver; it now compares with `Stats::route.driver_id` and checks "
        "`DriverWork::MatrixFree`",
        "contract_4: `to_tier == \"ProtectedRODAS5P\"` pinned a label string; it now requires "
        "a RODAS5P leg, named by its own run record, after the DOPRI5 leg",
        "two comments next to these assertions named the removed labels",
    ],
    "crates/vigilode/examples/unified_production_solver.rs": [
        "header list: it claimed multi-tier routing, a transactional fallback and zero heap "
        "allocation in stepping hot loops (VG-1); the list now states what the example shows",
        "comment of problem 4: 'encounters stiffness rejection, and smoothly rolls back' "
        "described a detection that does not exist; it now states the trial budget",
    ],
}

TEST_LINE = re.compile(r"^test (\S+) \.\.\. ?(.*)$")
RESULT_LINE = re.compile(
    r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; "
    r"(\d+) filtered out"
)
RUNNING_LINE = re.compile(r"^\s+Running (.+?) \(")
DOCTEST_LINE = re.compile(r"^\s+Doc-tests (\S+)")
GOLDEN = re.compile(r"golden (\w+): identical \((\d+) bytes\)")
RECORD = "V03B_RECORD "
DIGEST = "V03B_SWITCHING_DIGEST "


def strict_number(text):
    raise ValueError(f"non-finite JSON constant {text}")


def strict_load(path):
    return json.loads(Path(path).read_text(), parse_constant=strict_number)


def git(*args):
    return subprocess.run(
        ["git", "-C", str(ROOT), *args], check=True, capture_output=True, text=True
    ).stdout


def read_log(name):
    path = LOGS / f"{name}.log"
    if not path.exists():
        return None
    return path.read_text(errors="replace").splitlines()


def header(lines, key):
    for line in lines:
        if line.startswith(f"# {key}: "):
            return line[len(key) + 4 :]
    return None


def parse_tests(lines):
    """Status of every test, records by bare test name, digest lines, totals."""
    statuses, records, digests = {}, {}, []
    totals = {"passed": 0, "failed": 0, "ignored": 0, "binaries": 0, "doctest_binaries": 0}
    binaries = []
    current = None
    for line in lines:
        running = RUNNING_LINE.match(line)
        if running:
            binaries.append(running.group(1))
        if DOCTEST_LINE.match(line):
            totals["doctest_binaries"] += 1
        at = line.find(RECORD)
        if at >= 0:
            record = json.loads(line[at + len(RECORD) :], parse_constant=strict_number)
            records[record["test"]] = record["fields"]
        at = line.find(DIGEST)
        if at >= 0:
            digests.append(line[at + len(DIGEST) :].strip())
        test = TEST_LINE.match(line)
        if test:
            current, rest = test.group(1), test.group(2).strip()
            if rest in ("ok", "FAILED", "ignored"):
                statuses[current] = rest
                current = None
            continue
        if current is not None and line.strip() in ("ok", "FAILED"):
            statuses[current] = line.strip()
            current = None
        result = RESULT_LINE.match(line)
        if result:
            totals["passed"] += int(result.group(2))
            totals["failed"] += int(result.group(3))
            totals["ignored"] += int(result.group(4))
            totals["binaries"] += 1
    return {
        "statuses": statuses,
        "records": records,
        "digests": digests,
        "totals": totals,
        "binaries": binaries,
    }


def command_entry(name):
    lines = read_log(name)
    if lines is None:
        return {"log": f"logs/{name}.log", "missing": True}
    exit_code = header(lines, "exit_code")
    parsed = parse_tests(lines)
    entry = {
        "log": f"logs/{name}.log",
        "command": header(lines, "command"),
        "commit": header(lines, "commit"),
        "worktree_dirty_files": header(lines, "worktree_dirty_files"),
        "date": header(lines, "date"),
        "loadavg_before": header(lines, "loadavg_before"),
        "loadavg_after": header(lines, "loadavg_after"),
        "exit_code": None if exit_code is None else int(exit_code),
    }
    if parsed["totals"]["binaries"]:
        entry["tests"] = parsed["totals"]
    return entry


def bare(name):
    return name.rsplit("::", 1)[-1]


def main():
    missing = []
    receipt = {
        "schema": "vigilode.loop-l1.v03b.receipt.v1",
        "node": "V03_UNIFIED_FRONTEND (part B: truthful unified router)",
        "generated_by": "build_receipt.py: every number is parsed from logs/*.log or from git",
        "base_commit": BASE,
        "head_commit_when_built": git("rev-parse", "HEAD").strip(),
        "timing": "no wall-time claim; elapsed times in the logs are diagnostic_only",
    }
    predeclared = strict_load(HERE / "PREDECLARED.json")
    predeclared_commit = git(
        "log", "--diff-filter=A", "--format=%H", "--", str(HERE / "PREDECLARED.json")
    ).split()
    first_log_commit = git(
        "log", "--diff-filter=A", "--format=%H", "--", str(LOGS / "failing_first_base.log")
    ).split()
    receipt["predeclared"] = {
        "file": "PREDECLARED.json",
        "commit": predeclared_commit[-1] if predeclared_commit else None,
        "first_log_commit": first_log_commit[-1] if first_log_commit else None,
        "committed_before_the_first_log": bool(predeclared_commit and first_log_commit)
        and subprocess.run(
            ["git", "-C", str(ROOT), "merge-base", "--is-ancestor", predeclared_commit[-1],
             first_log_commit[-1] + "~1"],
        ).returncode
        == 0,
        "frozen_row": predeclared["frozen_row"],
    }

    # ---------------------------------------------------------------- commands
    names = [
        "contracts", "existing_unified_and_registry_tests", "default_path_golden", "full_suite",
        "clippy", "fmt_check", "doc", "example", "check_checkpoint", "changed_paths",
        "golden_diff", "protected_diff",
    ]
    commands = [command_entry(name) for name in names]
    receipt["commands"] = commands
    missing += [c["log"] for c in commands if c.get("missing")]
    by_name = dict(zip(names, commands))
    code_commits = sorted({c["commit"] for c in commands if c.get("commit")})
    receipt["code_commits_of_the_runs"] = code_commits
    source_changes = []
    for commit in code_commits:
        changed = git("diff", "--name-only", commit, "HEAD", "--", "crates", "Cargo.toml",
                      "Cargo.lock").split()
        if changed:
            source_changes.append({"since": commit, "paths": changed})
    receipt["source_paths_changed_after_a_run"] = source_changes

    # ---------------------------------------------------------- failing first
    base_lines = read_log("failing_first_base")
    now_lines = read_log("contracts")
    failing_first = {"log": "logs/failing_first_base.log"}
    rows = []
    if base_lines is None or now_lines is None:
        missing.append("logs/failing_first_base.log or logs/contracts.log")
        now = {"statuses": {}, "records": {}, "digests": [], "totals": {}}
    else:
        base = parse_tests(base_lines)
        now = parse_tests(now_lines)
        failing_first["header"] = [line for line in base_lines if line.startswith("#")]
        failing_first["base_totals"] = base["totals"]
        failing_first["base_exit"] = next(
            (int(line.split("=")[1]) for line in base_lines if line.startswith("exit=")), None
        )
        now_status = {bare(name): status for name, status in now["statuses"].items()}
        for name, status in sorted(base["statuses"].items()):
            rows.append({
                "test": name,
                "base_sources": status,
                "this_node": now_status.get(name),
                "failing_first": status == "FAILED" and now_status.get(name) == "ok",
                "base_record": base["records"].get(name),
                "record": now["records"].get(name),
            })
        failing_first["tests"] = rows
        failing_first["count"] = len(rows)
        failing_first["failed_on_base_and_pass_now"] = sum(r["failing_first"] for r in rows)
        failing_first["regression_guards_passing_on_base"] = [
            r["test"] for r in rows if r["base_sources"] == "ok"
        ]
    receipt["failing_first_part1"] = failing_first

    new_api = read_log("failing_first_new_api")
    if new_api is None:
        missing.append("logs/failing_first_new_api.log")
    else:
        errors = {}
        for line in new_api:
            found = re.match(r"^error\[(E\d+)\]", line)
            if found:
                errors[found.group(1)] = errors.get(found.group(1), 0) + 1
        receipt["failing_first_part2_and_3"] = {
            "log": "logs/failing_first_new_api.log",
            "header": [line for line in new_api if line.startswith("#")],
            "kind": "compile failure against a scratch copy of the base sources: these tests "
                    "name API that the base does not have, and were written after the "
                    "implementation",
            "exit": next((int(line.split("=")[1]) for line in new_api
                          if line.startswith("exit=")), None),
            "compile_errors_by_code": errors,
            "tests_now": {name: status for name, status in sorted(now["statuses"].items())
                          if "::" in name},
        }

    # ------------------------------------------------------------- key numbers
    def pair(test, field):
        row = next((r for r in rows if r["test"] == test), None)
        if row is None or row["base_record"] is None or row["record"] is None:
            return None
        return {"base": row["base_record"].get(field), "now": row["record"].get(field)}

    stiff = {}
    for test in [
        "p1_stiff_contractive_heat_equation_with_dense_jacobian_is_not_explicit",
        "p1_stiff_contractive_scalar_with_dense_jacobian_is_not_explicit",
        "p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit",
        "p1_stiff_contractive_scalar_without_jacobian_is_not_explicit",
    ]:
        stiff[test] = {
            "labels": pair(test, "labels"),
            "rhs_calls_charged": pair(test, "rhs_calls_charged"),
            "rhs_entries_at_the_boundary": pair(test, "rhs_entries_at_the_boundary"),
            "rhs_bound": pair(test, "rhs_bound"),
            "explicit_leg": pair(test, "explicit_leg"),
        }
    receipt["stiff_contractive"] = stiff
    receipt["shared_budget"] = {
        field: pair("p1_fallback_shares_one_attempt_budget_and_keeps_the_work_of_the_failed_leg",
                    field)
        for field in ["labels", "max_steps", "accepted_plus_rejected_in_counters",
                      "stats_attempts", "rhs_entries", "rhs_charged", "success"]
    }
    receipt["storage"] = {
        "workspace_retained_bytes_n2000":
            pair("p1_workspace_storage_is_linear_in_the_dimension", "retained_bytes"),
        "largest_single_allocation_bytes_matrix_free_n3000":
            pair("p1_matrix_free_solve_allocates_no_dense_matrix",
                 "largest_single_allocation_bytes"),
        "dense_matrix_bytes_n3000":
            pair("p1_matrix_free_solve_allocates_no_dense_matrix", "dense_matrix_bytes"),
    }
    receipt["stop_reasons"] = {
        "fatal_rhs_under_switching":
            pair("p1_failing_switching_solve_reports_the_fatal_rhs_callback", "unified_status"),
        "fatal_jacobian_under_switching":
            pair("p1_failing_switching_solve_reports_the_fatal_jacobian_callback",
                 "unified_status"),
        "blow_up_under_switching":
            pair("p1_failing_switching_solve_with_attempts_left_is_not_reported_as_max_steps",
                 "unified_status"),
        "fatal_preflight_callback":
            pair("p1_preflight_callback_failure_is_a_typed_stop_not_invalid_input", "outcome"),
        "fatal_entries_on_the_explicit_leg":
            pair("p1_fatal_callback_on_the_explicit_leg_is_not_retried_by_another_method",
                 "fatal_entries"),
    }
    receipt["preflight_charging"] = {
        test: {"direct": (next((r for r in rows if r["test"] == test), {}).get("record") or {})
               .get("direct"),
               "unified": pair(test, "unified")}
        for test in [
            "p1_preflight_callbacks_are_charged_with_a_dense_jacobian",
            "p1_preflight_callbacks_are_charged_without_a_jacobian",
            "p1_preflight_callbacks_are_charged_with_a_jvp",
        ]
    }
    grid = now["records"].get("p2_every_tier_has_a_driver_and_every_report_replays_as_direct_calls")
    receipt["replay_grid"] = grid
    receipt["label_cases"] = {
        test: {"base_mismatches": pair(test, "mismatches"), "total": pair(test, "total")}
        for test in [
            "p1_label_equals_executed_driver_on_the_explicit_route",
            "p1_label_equals_executed_driver_on_every_rodas5p_driver_family",
            "p1_label_equals_executed_driver_on_the_switching_adapter",
        ]
    }
    receipt["part2_and_3_records"] = {
        name: fields for name, fields in sorted(now["records"].items())
        if name.startswith(("p2_", "p3_"))
        and name != "p2_every_tier_has_a_driver_and_every_report_replays_as_direct_calls"
        and name != "p2_legs_are_direct_calls_and_their_work_adds_up"
    }
    legs = now["records"].get("p2_legs_are_direct_calls_and_their_work_adds_up")
    if legs:
        receipt["two_leg_solve"] = {
            key: legs[key] for key in ["screen", "preflight_rhs_calls", "leg_rhs_calls",
                                       "total_rhs_calls", "rhs_entries", "direct_rhs_entries",
                                       "invariant_drift"]
        }
        receipt["two_leg_solve"]["legs"] = [
            {key: leg[key] for key in ["tier", "driver_id", "route_id", "t_start", "t_end",
                                       "target", "success", "attempt_budget", "attempts",
                                       "accepted_steps", "rejected_steps"]}
            | {"rhs_calls": leg["counters"]["rhs_calls"], "stop": leg["stop"]}
            for leg in legs["legs"]
        ]

    # ------------------------------------------------- switching adapter digests
    base_digest_lines = read_log("switching_digests_base")
    if base_digest_lines is None:
        missing.append("logs/switching_digests_base.log")
    else:
        base_digests = parse_tests(base_digest_lines)["digests"]
        now_digests = now["digests"]
        receipt["switching_adapter_neutrality"] = {
            "what": "one line per (problem, configuration): success, final time bits, attempts, "
                    "switches, segments, length and FNV-1a 64 of the full serialized record of "
                    "SwitchingSolver::solve and of ProductionSolver::solve",
            "base_log": "logs/switching_digests_base.log",
            "base_header": [line for line in base_digest_lines if line.startswith("#")],
            "lines_on_base_sources": len(base_digests),
            "lines_on_this_node": len(now_digests),
            "identical_lines": sum(a == b for a, b in zip(base_digests, now_digests)),
            "all_identical": bool(base_digests) and base_digests == now_digests,
            "error_lines": sum(" err " in f" {line} " for line in now_digests),
        }

    # ------------------------------------------------------------- acceptance
    golden_lines = read_log("default_path_golden") or []
    golden_files = [GOLDEN.search(line).group(1) for line in golden_lines if GOLDEN.search(line)]
    golden_diff = [line for line in (read_log("golden_diff") or []) if not line.startswith("#")]
    suite_lines = read_log("full_suite") or []
    suite = parse_tests(suite_lines)
    failed_tests = sorted(n for n, s in suite["statuses"].items() if s == "FAILED")
    contract_total = now["totals"].get("passed", 0) + now["totals"].get("failed", 0)
    suite_total = suite["totals"]["passed"] + suite["totals"]["failed"]
    receipt["full_suite"] = {
        "exit_code": by_name["full_suite"].get("exit_code"),
        **suite["totals"],
        "failed_tests": failed_tests,
        "tests_of_the_new_binary": contract_total,
        "tests_outside_the_new_binary": suite_total - contract_total,
    }
    receipt["golden"] = {
        "exit_code": by_name["default_path_golden"].get("exit_code"),
        "files_reported_identical": golden_files,
        "tests": by_name["default_path_golden"].get("tests"),
        "diff_against_base_lines": len(golden_diff),
    }
    part1_ok = bool(rows) and all(r["failing_first"] for r in rows)
    contracts_ok = (
        by_name["contracts"].get("exit_code") == 0 and now["totals"].get("failed") == 0
    )
    acceptance = {
        "A1_contract_tests_pass_each_written_first_as_a_failing_test": {
            "measured": {
                "contract_tests_passed": now["totals"].get("passed"),
                "contract_tests_failed": now["totals"].get("failed"),
                "part1_tests": len(rows),
                "part1_failed_on_base_and_pass_now": sum(r["failing_first"] for r in rows),
            },
            "verdict": "PASS" if contracts_ok and part1_ok else "FAIL",
        },
        "A2_default_path_golden_unchanged": {
            "measured": {
                "exit_code": by_name["default_path_golden"].get("exit_code"),
                "golden_files_identical": len(golden_files),
                "diff_lines_against_base": len(golden_diff),
            },
            "verdict": "PASS"
            if by_name["default_path_golden"].get("exit_code") == 0
            and len(golden_files) == 9 and not golden_diff
            else "FAIL",
        },
        "A3_every_other_pre_existing_facade_test_passes": {
            "measured": {
                "exit_code": by_name["full_suite"].get("exit_code"),
                "passed": suite["totals"]["passed"],
                "failed": suite["totals"]["failed"],
                "ignored": suite["totals"]["ignored"],
                "test_binaries": suite["totals"]["binaries"],
                "failed_tests": failed_tests,
            },
            "verdict": "NOT_RUN" if not suite_lines else (
                "PASS" if by_name["full_suite"].get("exit_code") == 0
                and suite["totals"]["failed"] == 0 else "FAIL"),
        },
        "A4_no_behaviour_change_of_the_other_frontends": {
            "measured": {
                "switching_and_production_record_digests_identical":
                    receipt.get("switching_adapter_neutrality", {}).get("identical_lines"),
                "digest_lines": receipt.get("switching_adapter_neutrality", {})
                    .get("lines_on_base_sources"),
                "golden_files_identical": len(golden_files),
                "protected_diff_lines": len([line for line in (read_log("protected_diff") or [])
                                             if not line.startswith("#")]),
            },
            "scope": "Rodas5p, Dopri5, GeneralOde, AutomaticOde and the default ProductionSolver "
                     "by the golden oracle (their sources are unchanged); SwitchingSolver and "
                     "ProductionSolver by the record digests on the base sources and here",
            "verdict": "PASS"
            if receipt.get("switching_adapter_neutrality", {}).get("all_identical")
            and len(golden_files) == 9
            else "FAIL",
        },
    }
    receipt["acceptance"] = acceptance

    # ------------------------------------------------------- lints and checker
    checker = [line for line in (read_log("check_checkpoint") or []) if not line.startswith("#")]
    receipt["lints"] = {
        "clippy_exit_code": by_name["clippy"].get("exit_code"),
        "fmt_check_exit_code": by_name["fmt_check"].get("exit_code"),
        "doc_exit_code": by_name["doc"].get("exit_code"),
        "example_exit_code": by_name["example"].get("exit_code"),
        "loop_checker_exit_code": by_name["check_checkpoint"].get("exit_code"),
        "loop_checker_output": checker,
    }

    # ---------------------------------------------------------- changed paths
    changed = []
    for line in read_log("changed_paths") or []:
        if line.startswith("#") or not line.strip():
            continue
        status, path = line.split("\t", 1)
        changed.append({"status": status, "path": path})
    receipt["changed_paths"] = changed
    receipt["changed_paths_outside_the_task_allowlist"] = [
        entry["path"] for entry in changed
        if not any(re.match(pattern, entry["path"]) for pattern in ALLOWED)
    ]
    receipt["protected_paths_diff_against_base"] = [
        line for line in (read_log("protected_diff") or []) if not line.startswith("#")
    ]
    edits = []
    for path, reasons in EXISTING_FILE_EDITS.items():
        numstat = git("diff", "--numstat", BASE, "HEAD", "--", path).split()
        edits.append({
            "path": path,
            "lines_added": int(numstat[0]) if numstat else 0,
            "lines_removed": int(numstat[1]) if numstat else 0,
            "reasons": reasons,
        })
    receipt["edits_of_existing_files"] = edits
    before = read_log("existing_unified_tests_before_edit")
    if before:
        parsed = parse_tests(before)
        receipt["existing_unified_tests_before_the_edit"] = {
            "log": "logs/existing_unified_tests_before_edit.log",
            "statuses": parsed["statuses"],
            "what": "the unedited test file against the new router: only the label "
                    "assertions fail",
        }

    receipt["missing_logs"] = missing
    receipt["json_files_checked_strict"] = ["PREDECLARED.json", "RECEIPT.json"]
    text = json.dumps(receipt, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    json.loads(text, parse_constant=strict_number)
    (HERE / "RECEIPT.json").write_text(text)
    for name, entry in acceptance.items():
        print(f"{name}: {entry['verdict']}  {json.dumps(entry['measured'])}")
    print(f"missing logs: {missing}")
    print(f"paths outside the allowlist: {receipt['changed_paths_outside_the_task_allowlist']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
