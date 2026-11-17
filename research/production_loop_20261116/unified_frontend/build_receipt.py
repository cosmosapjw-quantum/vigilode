#!/usr/bin/env python3
"""Build RECEIPT.json of loop L1 node V03 part A from the raw logs under logs/.

Every number in the receipt is parsed from a log that an executed command wrote
(`run_checks.sh` and the two failing-first logs). Nothing is typed in: thresholds come from
PREDECLARED.json and from the frozen row, and are only compared with parsed values.

Usage (from the worktree root):
    python3 -I research/production_loop_20261116/unified_frontend/build_receipt.py
"""

import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOGS = HERE / "logs"
ROOT = HERE.parents[2]

GRID_ENTRIES = 221184  # PREDECLARED.json D1_decision_equality.grid.entries
EXTENSION_ENTRIES = 331776  # PREDECLARED.json D1_decision_equality.phase_B.extension_grid

ALLOWED = [
    r"^crates/vigilode/src/selection\.rs$",
    r"^crates/vigilode/src/selection/.+",
    r"^crates/vigilode/src/solve\.rs$",
    r"^crates/vigilode/tests/unified_frontend_contracts\.rs$",
    r"^crates/vigilode/tests/unified_method_capabilities\.rs$",
    r"^crates/vigilode/tests/unified_method_sessions\.rs$",
    r"^research/production_loop_20261116/unified_frontend/.+",
]

TEST_LINE = re.compile(r"^test (\S+) \.\.\. ?(.*)$")
RESULT_LINE = re.compile(
    r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; "
    r"(\d+) filtered out"
)
RUNNING_LINE = re.compile(r"^\s+Running (.+?) \(")
DOCTEST_LINE = re.compile(r"^\s+Doc-tests (\S+)")
QUAL = re.compile(r"QUAL (\{.*\})\s*$")
GOLDEN = re.compile(r"golden (\w+): identical \((\d+) bytes\)")


def strict_number(text):
    raise ValueError(f"non-finite JSON constant {text}")


def parse_log(name):
    """Header fields, test results, per-test status and QUAL records of one log."""
    path = LOGS / f"{name}.log"
    if not path.exists():
        return None
    header, body = {}, []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if raw.startswith("# ") and ": " in raw:
            key, value = raw[2:].split(": ", 1)
            header[key] = value
        elif raw.startswith("# "):
            header.setdefault("notes", []).append(raw[2:])
        else:
            body.append(raw)
    binaries, statuses, qual, golden = [], {}, {}, {}
    current_binary, pending = None, None
    for line in body:
        match = RUNNING_LINE.match(line)
        if match:
            current_binary = match.group(1)
            continue
        match = DOCTEST_LINE.match(line)
        if match:
            current_binary = f"doc-tests {match.group(1)}"
            continue
        match = QUAL.search(line)
        if match:
            record = json.loads(match.group(1), parse_constant=strict_number)
            qual.setdefault(record["test"], []).append(record["data"])
        for stem, size in GOLDEN.findall(line):
            golden[stem] = int(size)
        match = TEST_LINE.match(line)
        if match:
            test, rest = match.group(1), match.group(2).strip()
            if rest in ("ok", "FAILED") or rest.startswith("ignored"):
                statuses[test] = "ignored" if rest.startswith("ignored") else rest
                pending = None
            else:
                pending = test
            continue
        if pending and line.strip() in ("ok", "FAILED"):
            statuses[pending] = line.strip()
            pending = None
            continue
        match = RESULT_LINE.match(line)
        if match:
            binaries.append(
                {
                    "binary": current_binary,
                    "result": match.group(1),
                    "passed": int(match.group(2)),
                    "failed": int(match.group(3)),
                    "ignored": int(match.group(4)),
                    "filtered_out": int(match.group(6)),
                }
            )
    exit_code = header.get("exit_code")
    return {
        "log": f"logs/{name}.log",
        "command": header.get("command"),
        "commit": header.get("commit"),
        "date": header.get("date"),
        "loadavg_before": header.get("loadavg_before"),
        "loadavg_after": header.get("loadavg_after"),
        "worktree_dirty_files": header.get("worktree_dirty_files"),
        "exit_code": int(exit_code) if exit_code is not None else None,
        "binaries": binaries,
        "tests": {
            "passed": sum(b["passed"] for b in binaries),
            "failed": sum(b["failed"] for b in binaries),
            "ignored": sum(b["ignored"] for b in binaries),
            "binaries": len(binaries),
        },
        "failed_tests": sorted(t for t, s in statuses.items() if s == "FAILED"),
        "_statuses": statuses,
        "_qual": qual,
        "_golden": golden,
        "_body": body,
    }


def public(record):
    return {k: v for k, v in record.items() if not k.startswith("_")}


def one(record, test):
    """The single QUAL record of `test` in a parsed log."""
    rows = record["_qual"].get(test, [])
    if len(rows) != 1:
        raise SystemExit(f"{record['log']}: expected one QUAL record of {test}, found {len(rows)}")
    return rows[0]


def verdict(condition):
    return "PASS" if condition else "FAIL"


def main():
    names = [
        "contracts",
        "existing_registry_tests",
        "default_path_golden_run1",
        "default_path_golden_run2",
        "full_suite",
        "clippy",
        "fmt_check",
        "doc",
        "check_checkpoint",
        "changed_paths",
        "golden_diff",
        "protected_diff",
        "failing_first_peer_defects",
        "peer_defects_after_fix",
        "decision_grid_debug_profile",
    ]
    logs = {name: parse_log(name) for name in names}
    missing = [name for name, record in logs.items() if record is None]
    if missing:
        raise SystemExit(f"missing logs: {missing}")
    contracts = logs["contracts"]
    passed = lambda test: contracts["_statuses"].get(test) == "ok"  # noqa: E731

    # ---------------------------------------------------------------- D1
    grid = one(contracts, "d1_decision_grid")
    extension = one(contracts, "d1_phase_b_extension_grid")
    d1_pass = (
        passed("d1_route_general_equals_select_on_the_golden_decision_grid")
        and grid["entries"] == GRID_ENTRIES
        and grid["golden_file_entries"] == GRID_ENTRIES
        and grid["equal_to_select"] == GRID_ENTRIES
        and grid["equal_to_golden_file"] == GRID_ENTRIES
        and grid["callback_entries"] == 0
    )
    phase_b = (
        passed("d1_phase_b_select_equals_the_reference_body_on_the_extension_grid")
        and extension["entries"] == EXTENSION_ENTRIES
        and extension["select_equal_to_reference"] == EXTENSION_ENTRIES
        and extension["general_selection_equal_to_reference"] == EXTENSION_ENTRIES
        and extension["callback_entries"] == 0
        and grid["equal_to_reference_body"] == GRID_ENTRIES
    )
    debug = logs["decision_grid_debug_profile"]

    # ---------------------------------------------------------------- D2
    adapter_tests = {
        "d2_adapter_rodas5p": "d2_rodas5p_adapter_is_bitwise_the_direct_call_on_every_facade_route",
        "d2_adapter_dopri5_general": "d2_dopri5_adapter_and_general_plan_are_bitwise_the_direct_calls",
        "d2_adapter_bdf": "d2_bdf_adapter_is_bitwise_the_direct_call",
        "d2_adapter_radau": "d2_radau_adapter_is_bitwise_the_direct_call",
        "d2_adapter_pexprb": "d2_pexprb_adapter_is_bitwise_the_direct_call",
        "d2_adapter_lagged": "d2_lagged_rodas5p_adapter_is_bitwise_the_direct_call",
        "d2_adapter_ppsw1": "d2_ppsw1_adapter_is_bitwise_the_direct_call",
        "d2_adapter_lawson": "d2_lawson_adapter_is_bitwise_the_direct_call_and_needs_its_declaration",
    }
    adapters = []
    for key, test in adapter_tests.items():
        data = one(contracts, key)
        cases = data["cases"]
        ok = passed(test) and bool(cases) and all(c.get("bitwise_equal") is True for c in cases)
        adapters.append(
            {
                "method": data["method"],
                "test": test,
                "test_status": contracts["_statuses"].get(test),
                "cases": len(cases),
                "cases_bitwise_equal": sum(1 for c in cases if c.get("bitwise_equal") is True),
                "routes": sorted({c["route"] for c in cases}),
                "verdict": verdict(ok),
                "case_rows": cases,
            }
        )
    d2_pass = all(a["verdict"] == "PASS" for a in adapters)

    # ---------------------------------------------------------------- D3
    purity = one(contracts, "d3_route_purity")
    solve_refusals = one(contracts, "d3_solve_refusals")
    refusal_matrix = one(contracts, "d3_explicit_refusal_matrix")
    d3_pass = (
        passed("d3_route_is_pure_for_every_choice_over_the_extension_grid")
        and passed("d3_solve_refuses_malformed_input_before_any_callback")
        and passed("d3_explicit_refusals_are_typed")
        and purity["callback_entries"] == 0
        and purity["route_calls"] > 0
        and solve_refusals["callback_entries"] == 0
        and solve_refusals["methods_checked"] == solve_refusals["available_methods"]
        and grid["callback_entries"] == 0
        and extension["callback_entries"] == 0
    )

    # ---------------------------------------------------------------- D4
    run1, run2 = logs["default_path_golden_run1"], logs["default_path_golden_run2"]
    golden_diff_lines = [line for line in logs["golden_diff"]["_body"] if line.strip()]
    d4_pass = (
        run1["exit_code"] == 0
        and run2["exit_code"] == 0
        and run1["tests"]["failed"] == 0
        and run2["tests"]["failed"] == 0
        and len(run1["_golden"]) == 9
        and run1["_golden"] == run2["_golden"]
        and not golden_diff_lines
    )

    # ------------------------------------------------------- PeerOde defects
    before, after = logs["failing_first_peer_defects"], contracts
    defect_tests = sorted(t for t in before["_statuses"] if t.startswith("cod0"))
    defects = []
    for test in defect_tests:
        defects.append(
            {
                "test": test,
                "base_sources": before["_statuses"].get(test),
                "after_fix": after["_statuses"].get(test),
            }
        )
    forcing = {}
    for key in ("cod02_p3", "cod02_p3b"):
        forcing[key] = {
            "base_sources": {
                k: one(before, key)[k]
                for k in ("success", "h0", "abs_error", "error_in_tolerance_units")
            },
            "after_fix": {
                k: one(after, key)[k]
                for k in (
                    "success",
                    "h0",
                    "abs_error",
                    "error_in_tolerance_units",
                    "accepted_peer_steps",
                    "rejected_peer_steps",
                    "startup_jacobian_builds",
                )
            },
        }
    digest_keys = ("jacobian", "threads")
    base_digests = {
        tuple(row[k] for k in digest_keys): row for row in one(before, "peer_baseline_digests")
    }
    digest_rows = []
    for row in one(after, "peer_baseline_digests"):
        base = base_digests[tuple(row[k] for k in digest_keys)]
        digest_rows.append(
            {
                "jacobian": row["jacobian"],
                "threads": row["threads"],
                "trajectory_fnv1a64_base": base["trajectory_fnv1a64"],
                "trajectory_fnv1a64_after_fix": row["trajectory_fnv1a64"],
                "trajectory_unchanged": base["trajectory_fnv1a64"] == row["trajectory_fnv1a64"]
                and base["h0_bits"] == row["h0_bits"]
                and base["n_outputs"] == row["n_outputs"],
                "startup_jacobian_builds": [
                    base["startup_jacobian_builds"],
                    row["startup_jacobian_builds"],
                ],
                "startup_rhs_calls": [base["startup_rhs_calls"], row["startup_rhs_calls"]],
                "step_rhs_calls": [base["step_rhs_calls"], row["step_rhs_calls"]],
            }
        )
    defects_fixed = (
        all(d["base_sources"] == "FAILED" and d["after_fix"] == "ok" for d in defects)
        and len(defects) == 8
    )

    # ------------------------------------------------------- suite and lints
    suite = logs["full_suite"]
    changed = []
    for line in logs["changed_paths"]["_body"]:
        parts = line.split("\t")
        if len(parts) == 2:
            changed.append({"status": parts[0], "path": parts[1]})
    outside = [c["path"] for c in changed if not any(re.match(p, c["path"]) for p in ALLOWED)]
    edited_existing_tests = [
        c["path"]
        for c in changed
        if c["path"].endswith(("unified_method_capabilities.rs", "unified_method_sessions.rs"))
    ]
    protected_diff_lines = [line for line in logs["protected_diff"]["_body"] if line.strip()]
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()

    receipt = {
        "schema": "vigilode.loop-l1.v03a.receipt.v1",
        "node": "V03_UNIFIED_FRONTEND (part A)",
        "generated_by": "build_receipt.py: every number is parsed from logs/*.log",
        "base_commit": "c468478e73123a4625c6d20f7459a9f79cbf33de",
        "head_commit_when_built": head,
        "code_commit_of_the_runs": contracts["commit"],
        "predeclared": "PREDECLARED.json (commit 00d32764, before any run)",
        "timing": "no wall-time claim; elapsed times in the logs are diagnostic_only",
        "phase_reached": "B" if (d1_pass and phase_b and d4_pass) else "A",
        "discriminators": {
            "D1_decision_equality": {
                "frozen": "100% decision equality of route(General) and GeneralOde::select on an exhaustive grid with zero callbacks",
                "threshold": f"{GRID_ENTRIES} of {GRID_ENTRIES} entries, 0 callback entries",
                "measured": {k: grid[k] for k in sorted(grid) if k != "differing_examples"},
                "differing_examples": grid["differing_examples"],
                "verdict": verdict(d1_pass),
            },
            "D1_phase_B": {
                "condition": "registry-derived selection equals the retained old body on the golden grid and on the extension grid, zero callbacks, golden oracle unchanged",
                "threshold": f"{EXTENSION_ENTRIES} of {EXTENSION_ENTRIES} extension entries and {GRID_ENTRIES} of {GRID_ENTRIES} golden entries",
                "measured": {
                    "extension_grid": {
                        k: extension[k]
                        for k in sorted(extension)
                        if k not in ("differing_examples", "outcome_counts")
                    },
                    "extension_outcome_counts": extension["outcome_counts"],
                    "golden_grid_equal_to_reference_body": grid["equal_to_reference_body"],
                    "debug_profile_run": {
                        "exit_code": debug["exit_code"],
                        "passed": debug["tests"]["passed"],
                        "failed": debug["tests"]["failed"],
                        "meaning": "debug assertions on: the fallback to the reference body was never taken",
                    },
                },
                "differing_examples": extension["differing_examples"],
                "verdict": verdict(phase_b),
            },
            "D2_adapter_bitwise_equality": {
                "frozen": "each adapter bitwise equal to the direct integrator call",
                "measured": {
                    "adapters": len(adapters),
                    "adapters_equal": sum(1 for a in adapters if a["verdict"] == "PASS"),
                    "cases": sum(a["cases"] for a in adapters),
                    "cases_bitwise_equal": sum(a["cases_bitwise_equal"] for a in adapters),
                },
                "verdict": verdict(d2_pass),
            },
            "D3_refusal_before_any_callback": {
                "frozen": "refusal before any callback",
                "measured": {
                    "route_calls": purity["route_calls"],
                    "route_plans": purity["plans"],
                    "route_refusals": purity["refusals"],
                    "route_callback_entries": purity["callback_entries"],
                    "rodas5p_plans_equal_to_the_route_that_runs": [
                        purity["rodas5p_admissions_equal_to_capabilities"],
                        purity["rodas5p_admissions_compared_with_capabilities"],
                    ],
                    "rodas5p_capability_query_names_another_driver_than_runs": purity[
                        "rodas5p_capability_query_names_another_driver_than_runs"
                    ],
                    "refused_solve_calls": solve_refusals["refused_solve_calls"],
                    "solve_methods_checked": [
                        solve_refusals["methods_checked"],
                        solve_refusals["available_methods"],
                    ],
                    "solve_callback_entries": solve_refusals["callback_entries"],
                    "typed_refusal_rows": refusal_matrix["rows"],
                },
                "verdict": verdict(d3_pass),
            },
            "D4_default_route_golden": {
                "frozen": "default-route golden unchanged",
                "measured": {
                    "run1": {"exit_code": run1["exit_code"], **run1["tests"]},
                    "run2": {"exit_code": run2["exit_code"], **run2["tests"]},
                    "golden_files_identical_bytes": run1["_golden"],
                    "two_runs_report_the_same_files": run1["_golden"] == run2["_golden"],
                    "git_diff_of_golden_and_oracle_against_base": golden_diff_lines,
                },
                "verdict": verdict(d4_pass),
            },
        },
        "adapters": adapters,
        "peer_defects": {
            "source": "audit findings COD-02 and COD-04",
            "reproductions": defects,
            "all_failed_on_base_and_pass_after_fix": defects_fixed,
            "failing_first_log": public(before),
            "forcing_cases": forcing,
            "other_cases_after_fix": {
                key: one(after, key)
                for key in ("cod02_p5", "cod04a_p1", "cod04b_p2", "cod04c_p4", "cod04cd_p6", "cod04e_p7")
            },
            "other_cases_on_base": {
                key: before["_qual"].get(key, [None])[0]
                for key in ("cod02_p5", "cod04a_p1", "cod04b_p2", "cod04c_p4", "cod04cd_p6", "cod04e_p7")
            },
            "typed_assertions": {
                key: one(after, key)
                for key in (
                    "peer_typed_recoverable_jacobian",
                    "peer_typed_fatal_rhs",
                    "peer_starter_record",
                )
            },
            "u07_trajectory_regression_exploratory": digest_rows,
        },
        "auto": {
            "ranking": one(contracts, "auto_ranking"),
            "fallback": one(contracts, "auto_fallback"),
        },
        "view_typed_failures": one(contracts, "view_typed_failures"),
        "executed_route_vs_capability_query": one(contracts, "executed_route_vs_capability_query"),
        "cod03_audit_cases": one(contracts, "cod03_audit_cases"),
        "method_catalog": one(contracts, "method_catalog"),
        "commands": [
            public(logs[name])
            for name in names
            if name not in ("changed_paths", "golden_diff", "protected_diff")
        ],
        "full_suite": {
            "exit_code": suite["exit_code"],
            **suite["tests"],
            "failed_tests": suite["failed_tests"],
            "all_pre_existing_tests_pass": suite["exit_code"] == 0 and suite["tests"]["failed"] == 0,
        },
        "lints": {
            "clippy_exit_code": logs["clippy"]["exit_code"],
            "fmt_check_exit_code": logs["fmt_check"]["exit_code"],
            "doc_exit_code": logs["doc"]["exit_code"],
            "loop_checker_exit_code": logs["check_checkpoint"]["exit_code"],
            "loop_checker_output": [l for l in logs["check_checkpoint"]["_body"] if l.strip()],
        },
        "changed_paths": changed,
        "changed_paths_outside_the_task_allowlist": outside,
        "protected_paths_diff_against_base": protected_diff_lines,
        "edits_of_existing_tests": [
            {"path": path, "reason": "see REPORT_KO.md"} for path in edited_existing_tests
        ],
    }
    text = json.dumps(receipt, indent=1, ensure_ascii=False, allow_nan=False) + "\n"
    json.loads(text, parse_constant=strict_number)
    (HERE / "RECEIPT.json").write_text(text, encoding="utf-8")
    summary = {
        "phase_reached": receipt["phase_reached"],
        **{k: v["verdict"] for k, v in receipt["discriminators"].items()},
        "peer_defects_fixed": defects_fixed,
        "full_suite": receipt["full_suite"],
        "lints": {k: v for k, v in receipt["lints"].items() if k.endswith("exit_code")},
        "outside_allowlist": outside,
    }
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
