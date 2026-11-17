#!/usr/bin/env python3
"""Run the V15 stage-client-kit checks and write RECEIPT.json from their output.

Loop L1 node V15. Every number in the receipt is parsed from a log this script
wrote under logs/ in the same run; nothing is typed in. A failing command is
recorded with its exit code and its log is kept. Wall time on this shared host
is diagnostic only; the bit identity, the work counters, the allocation counts
and the user-space instruction counts are the primary evidence.

Usage (from anywhere):  python3 make_receipt.py
Requires CARGO_TARGET_DIR to be exported by the caller.
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOGS = HERE / "logs"
TOOLCHAIN = "+1.94.1"
PACKAGE = "rodas5p-integrators"
TESTS = ["stage_clients_dense_identity", "stage_clients_alloc"]
CODE_FILES = [
    "crates/rodas5p-integrators/src/stage_clients/mod.rs",
    "crates/rodas5p-integrators/src/stage_clients/dense_lu.rs",
    "crates/rodas5p-integrators/src/stage_clients/timed.rs",
    "crates/rodas5p-integrators/tests/stage_clients_dense_identity.rs",
    "crates/rodas5p-integrators/tests/stage_clients_alloc.rs",
]
# The loop commit this node branched from (the task's base).
BASE_COMMIT = "2dc9a2c5778ac5b3ac960b0f6741aa5916122384"
BASELINE_LINT_FILE = "crates/rodas5p-integrators/src/source_owned_graph_schur.rs"
BASELINE_LINTS = 8
WALL_TEST = "diagnostic_wall_ratio_client_over_protected"
ARM_TEST = "diagnostic_single_arm_for_instruction_count"
ARM_CASES = ["brusselator-n32", "brusselator-n100", "meanfield-n100"]
ARMS = ["protected", "client", "timed"]
ARM_RUNS = 20
ARM_REPEATS = 3
JSON_LINE = re.compile(r"(STAGE_CLIENTS_[A-Z_]+_JSON) (\{.*\})\s*$")
TEST_BINARY = re.compile(r"^\s*Running (\S+) \((\S+)\)")
TEST_RESULT = re.compile(
    r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def capture(command, cwd=ROOT):
    return subprocess.run(
        command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    ).stdout.strip()


def loadavg():
    try:
        return [float(x) for x in Path("/proc/loadavg").read_text().split()[:3]]
    except (OSError, ValueError):
        return None


def run(name, command, env=None):
    """Run `command`, keep its combined output in logs/<name>.log."""
    log = LOGS / f"{name}.log"
    full_env = dict(os.environ)
    if env:
        full_env.update(env)
    load_before = loadavg()
    start = time.monotonic()
    with open(log, "w") as out:
        process = subprocess.run(
            command, cwd=ROOT, stdout=out, stderr=subprocess.STDOUT, env=full_env
        )
    seconds = time.monotonic() - start
    record = {
        "name": name,
        "command": " ".join(command),
        "cwd": str(ROOT),
        "exit_code": process.returncode,
        "log": str(log.relative_to(HERE)),
        "log_sha256": sha256(log),
        "wall_seconds_diagnostic_only": round(seconds, 3),
        "loadavg_before": load_before,
        "loadavg_after": loadavg(),
    }
    if env:
        record["env"] = env
    return record, log.read_text(errors="replace")


def test_results(text):
    """Per test binary: the counts of its `test result:` line."""
    results, current = [], None
    for line in text.splitlines():
        binary = TEST_BINARY.match(line)
        if binary:
            current = binary.group(1)
            continue
        result = TEST_RESULT.match(line)
        if result:
            results.append(
                {
                    "target": current,
                    "status": result.group(1),
                    "passed": int(result.group(2)),
                    "failed": int(result.group(3)),
                    "ignored": int(result.group(4)),
                    "filtered_out": int(result.group(6)),
                }
            )
    return results


def json_lines(text):
    rows = {}
    for line in text.splitlines():
        found = JSON_LINE.search(line)
        if found:
            rows.setdefault(found.group(1), []).append(json.loads(found.group(2)))
    return rows


def clippy_warnings(text):
    """Each warning with its primary location; summary lines are skipped."""
    warnings, lines = [], text.splitlines()
    for index, line in enumerate(lines):
        if not re.match(r"^(warning|error)(\[[^\]]+\])?: ", line):
            continue
        if re.search(r"generated \d+ warnings?|could not compile|aborting due to", line):
            continue
        location = None
        for follow in lines[index + 1 : index + 6]:
            found = re.match(r"^\s*--> (\S+?):(\d+):(\d+)", follow)
            if found:
                location = {"file": found.group(1), "line": int(found.group(2))}
                break
        warnings.append({"message": line.strip(), "location": location})
    return warnings


def instructions(text):
    """The instructions:u count of a `perf stat -x,` log."""
    for line in text.splitlines():
        fields = line.split(",")
        if len(fields) > 2 and fields[2].startswith("instructions") and fields[0].isdigit():
            return int(fields[0])
    return None


def median(values):
    ordered = sorted(values)
    return ordered[len(ordered) // 2]


def main():
    target_dir = os.environ.get("CARGO_TARGET_DIR")
    if not target_dir:
        sys.exit("CARGO_TARGET_DIR must be exported")
    LOGS.mkdir(parents=True, exist_ok=True)
    cargo = ["cargo", TOOLCHAIN]
    test_targets = [arg for test in TESTS for arg in ("--test", test)]
    base_test = cargo + ["test", "--locked", "-j", "6", "--release", "-p", PACKAGE]

    receipt = {
        "schema": "vigilode.loop_l1.v15_client_kit_receipt.v1",
        "node": "V15_STAGE_CLIENT_KIT",
        "generated_by": str(Path(__file__).resolve().relative_to(ROOT)),
        "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "evidence_policy": (
            "Every number is parsed from the logs of this run. Wall time is "
            "diagnostic_only on this shared host; bit identity, work counters, "
            "allocation counts and user-space instruction counts are primary."
        ),
        "environment": {
            "worktree": str(ROOT),
            "branch": capture(["git", "branch", "--show-current"]),
            "code_commit": capture(["git", "rev-parse", "HEAD"]),
            "base_commit": BASE_COMMIT,
            "base_is_ancestor_of_head_exit_code": subprocess.run(
                ["git", "merge-base", "--is-ancestor", BASE_COMMIT, "HEAD"], cwd=ROOT
            ).returncode,
            "code_files_uncommitted_changes": capture(
                ["git", "status", "--porcelain", "--"] + CODE_FILES
            ),
            "rustc": capture(["rustc", TOOLCHAIN, "--version"]),
            "cargo": capture(["cargo", TOOLCHAIN, "--version"]),
            "cargo_target_dir": target_dir,
            "uname": capture(["uname", "-srm"]),
            "logical_cpus": os.cpu_count(),
            "loadavg_start": loadavg(),
        },
        "code_files": [{"path": path, "sha256": sha256(ROOT / path)} for path in CODE_FILES],
        "protected_files_changed_vs_base": capture(
            [
                "git",
                "diff",
                "--name-only",
                BASE_COMMIT,
                "HEAD",
                "--",
                "fixtures",
                "crates/rodas5p-core",
                "crates/rodas5p-krylov",
                "crates/vigilode",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                "crates/rodas5p-integrators/src/rodas5p_fast.rs",
                "crates/rodas5p-integrators/src/client_stage.rs",
                "crates/rodas5p-integrators/src/lib.rs",
            ]
        ).splitlines(),
        "commands": [],
    }
    commands = receipt["commands"]

    # 1. The task's test command, exactly.
    record, text = run("01_test", base_test + test_targets)
    record["test_results"] = test_results(text)
    commands.append(record)
    tests_exit = record["exit_code"]
    receipt["tests"] = {
        "exit_code": tests_exit,
        "per_target": record["test_results"],
        "passed": sum(r["passed"] for r in record["test_results"]),
        "failed": sum(r["failed"] for r in record["test_results"]),
        "ignored": sum(r["ignored"] for r in record["test_results"]),
    }

    # 2. The same tests, serial and uncaptured, for their JSON lines.
    record, text = run(
        "02_test_nocapture",
        base_test + test_targets + ["--", "--nocapture", "--test-threads=1"],
    )
    record["test_results"] = test_results(text)
    commands.append(record)
    rows = json_lines(text)
    identity = rows.get("STAGE_CLIENTS_IDENTITY_JSON", [])
    timed = rows.get("STAGE_CLIENTS_TIMED_JSON", [])
    off_policy = rows.get("STAGE_CLIENTS_OFF_POLICY_JSON", [])
    alloc = rows.get("STAGE_CLIENTS_ALLOC_JSON", [])
    in_place_alloc = [r for r in alloc if r.get("asserted_zero", True)]
    faer_alloc = [r for r in alloc if not r.get("asserted_zero", True)]
    receipt["bit_identity"] = {
        "reference": "integrate_rodas5p_fast_observed_with_atol (default options)",
        "client_entry": "integrate_rodas5p_fast_client_observed_with_atol[_and_control]",
        "compared": (
            "attempts, accepted and rejected steps, Jacobian reuses, internal and "
            "clipped steps, the whole WorkCounters, every output time and state, "
            "every accepted (t, y) (Jacobian-callback trace of both drivers and the "
            "client driver's accept control), all as f64 bits"
        ),
        "source_exit_code": record["exit_code"],
        "rows": identity,
        "rows_count": len(identity),
        "rows_bit_identical": sum(1 for r in identity if r.get("bit_identical") is True),
        "modes": sorted({r["client_mode"] for r in identity}),
        "cases": sorted({r["case"] for r in identity}),
        "rtols": sorted({r["rtol"] for r in identity}),
        "accepted_states_compared_total": sum(r["accepted_states_compared"] for r in identity),
        "rejected_steps_total": sum(r["rejected_steps"] for r in identity),
        "jacobian_reuses_total": sum(r["jacobian_reuses"] for r in identity),
    }
    receipt["timed_wrapper"] = {
        "rows": timed,
        "rows_count": len(timed),
        "rows_result_neutral": sum(1 for r in timed if r.get("result_neutral") is True),
        "rows_one_factor_per_attempt": sum(
            1 for r in timed if r["factor_calls"] == r["attempts"]
        ),
        "rows_eight_solves_per_attempt": sum(
            1 for r in timed if r["solve_calls"] == 8 * r["attempts"]
        ),
        "rows_one_linearize_per_accepted_step": sum(
            1 for r in timed if r["linearize_calls"] == r["accepted_steps"]
        ),
    }
    receipt["off_policy_faer"] = {
        "note": (
            "The faer mode on problems where the protected policy picks the "
            "in-place LU: a valid solver, not a bit-identical one."
        ),
        "rows": off_policy,
        "rows_count": len(off_policy),
        "rows_endpoint_bit_identical": sum(
            1 for r in off_policy if r["endpoint_bit_identical"]
        ),
        "rows_same_attempts": sum(
            1 for r in off_policy if r["attempts_client"] == r["attempts_protected"]
        ),
        "max_endpoint_difference_tolerance_units": max(
            (r["endpoint_difference_tolerance_units"] for r in off_policy), default=None
        ),
    }
    receipt["allocations"] = {
        "counter": "process-global counting allocator, one test in its own binary",
        "in_place_and_timed_rows": in_place_alloc,
        "in_place_and_timed_rows_count": len(in_place_alloc),
        "in_place_steady_steps_measured_total": sum(
            r["steady_steps_measured"] for r in in_place_alloc
        ),
        "in_place_steady_allocations_total": sum(
            r["steady_allocations_total"] for r in in_place_alloc
        ),
        "in_place_run_totals": sorted({r["run_total_allocations"] for r in in_place_alloc}),
        "faer_rows_reported_not_asserted": faer_alloc,
    }

    # 3. Diagnostic wall ratio, alone.
    record, text = run(
        "03_wall_diagnostic",
        base_test
        + ["--test", TESTS[0], "--", "--ignored", "--exact", WALL_TEST]
        + ["--nocapture", "--test-threads=1"],
    )
    record["test_results"] = test_results(text)
    commands.append(record)
    wall = json_lines(text).get("STAGE_CLIENTS_WALL_JSON", [])
    receipt["wall_diagnostic"] = {
        "diagnostic_only": True,
        "note": (
            "Seven interleaved repetitions per case, arm order rotating; each "
            "repetition is the mean of runs_per_repetition runs. The host is "
            "shared and loaded (see loadavg): these ratios are not a verdict."
        ),
        "loadavg_before": record["loadavg_before"],
        "loadavg_after": record["loadavg_after"],
        "rows": wall,
        "summary": [
            {
                "case": r["case"],
                "n": r["n"],
                "client_mode": r["client_mode"],
                "repetitions": r["repetitions"],
                "client_over_protected_ratio_of_medians": r[
                    "client_over_protected_ratio_of_medians"
                ],
                "client_over_protected_median_of_ratios": r[
                    "client_over_protected_median_of_ratios"
                ],
                "client_over_protected_ratio_of_minima": r[
                    "client_over_protected_ratio_of_minima"
                ],
                "timed_over_protected_ratio_of_medians": r[
                    "timed_over_protected_ratio_of_medians"
                ],
                "timed_over_protected_ratio_of_minima": r[
                    "timed_over_protected_ratio_of_minima"
                ],
                "timed_phases_over_wall": r["last_timed_run"]["phases_over_wall"],
            }
            for r in wall
        ],
    }

    # 4. User-space instruction counts of single arms (perf stat).
    perf = shutil.which("perf")
    record, text = run(
        "04_test_binary_path", base_test + ["--test", TESTS[0], "--no-run"]
    )
    commands.append(record)
    found = re.search(r"Executable \S+ \((\S+)\)", text)
    binary = found.group(1) if found else None
    instruction = {
        "counter": "perf stat -e instructions:u (user space, child threads included)",
        "binary": binary,
        "runs": ARM_RUNS,
        "repeats": ARM_REPEATS,
        "per_run": "(instructions(runs) - instructions(0 runs)) / runs",
        "rows": [],
    }
    if perf and binary:
        for case in ARM_CASES:
            per_arm = {}
            for arm in ARMS:
                env = {"STAGE_CLIENTS_ARM": arm, "STAGE_CLIENTS_CASE": case}
                arm_command = [
                    perf, "stat", "-x,", "-e", "instructions:u", binary,
                    "--ignored", "--exact", ARM_TEST, "--nocapture", "--test-threads=1",
                ]  # fmt: skip
                record, text = run(
                    f"04_perf_{case}_{arm}_runs0",
                    arm_command,
                    env={**env, "STAGE_CLIENTS_RUNS": "0"},
                )
                commands.append(record)
                setup = instructions(text)
                counts, exits, meta = [], [record["exit_code"]], None
                for repeat in range(ARM_REPEATS):
                    record, text = run(
                        f"04_perf_{case}_{arm}_runs{ARM_RUNS}_r{repeat}",
                        arm_command,
                        env={**env, "STAGE_CLIENTS_RUNS": str(ARM_RUNS)},
                    )
                    commands.append(record)
                    exits.append(record["exit_code"])
                    counts.append(instructions(text))
                    meta = (json_lines(text).get("STAGE_CLIENTS_ARM_JSON") or [None])[0]
                ok = setup is not None and all(c is not None for c in counts)
                per_run = [(c - setup) / ARM_RUNS for c in counts] if ok else None
                per_arm[arm] = {
                    "exit_codes": exits,
                    "setup_instructions": setup,
                    "instructions": counts,
                    "instructions_per_run": per_run,
                    "instructions_per_run_median": median(per_run) if ok else None,
                    "instructions_per_run_spread_relative": (
                        (max(per_run) - min(per_run)) / median(per_run) if ok else None
                    ),
                    "arm_report": meta,
                }
            base = per_arm["protected"]["instructions_per_run_median"]
            row = {"case_key": case, "arms": per_arm}
            for arm in ("client", "timed"):
                value = per_arm[arm]["instructions_per_run_median"]
                row[f"{arm}_over_protected_instructions"] = (
                    value / base if base and value is not None else None
                )
            instruction["rows"].append(row)
        instruction["available"] = True
    else:
        instruction["available"] = False
        instruction["reason"] = "perf not found" if not perf else "test binary not found"
    receipt["instruction_counts"] = instruction

    # 5. Clippy, the task's command.
    record, text = run(
        "05_clippy",
        cargo
        + ["clippy", "--locked", "-j", "6", "--release", "-p", PACKAGE, "--lib"]
        + test_targets
        + ["--no-deps"],
    )
    commands.append(record)
    warnings = clippy_warnings(text)
    unique = {(w["message"], json.dumps(w["location"], sort_keys=True)) for w in warnings}
    baseline = [w for w in unique if BASELINE_LINT_FILE in w[1]]
    new = sorted(w for w in unique if BASELINE_LINT_FILE not in w[1])
    receipt["clippy"] = {
        "exit_code": record["exit_code"],
        "unique_warnings": len(unique),
        "baseline_file": BASELINE_LINT_FILE,
        "baseline_warnings_expected": BASELINE_LINTS,
        "baseline_warnings_found": len(baseline),
        "new_warnings": [{"message": m, "location": json.loads(l)} for m, l in new],
        "new_warnings_count": len(new),
    }

    # 6. Format check of the node's files.
    record, text = run(
        "06_rustfmt_check",
        ["rustfmt", TOOLCHAIN, "--check", "--edition", "2024"] + CODE_FILES,
    )
    commands.append(record)
    receipt["rustfmt"] = {"exit_code": record["exit_code"], "files": CODE_FILES}

    # Exploratory logs kept from before the commit (not this run's evidence).
    exploratory = []
    for log in sorted((LOGS / "exploratory").glob("*.log")):
        text = log.read_text(errors="replace")
        exploratory.append(
            {
                "log": str(log.relative_to(HERE)),
                "log_sha256": sha256(log),
                "test_results": test_results(text),
                "wall_client_over_protected_ratio_of_medians": {
                    r["case"]: r["client_over_protected_ratio_of_medians"]
                    for r in json_lines(text).get("STAGE_CLIENTS_WALL_JSON", [])
                },
            }
        )
    receipt["exploratory_logs_pre_commit"] = exploratory

    # Verdicts, computed from the parsed data above.
    identity_ok = (
        tests_exit == 0
        and receipt["bit_identity"]["rows_count"] > 0
        and receipt["bit_identity"]["rows_bit_identical"] == receipt["bit_identity"]["rows_count"]
    )
    alloc_ok = (
        tests_exit == 0
        and len(in_place_alloc) > 0
        and receipt["allocations"]["in_place_steady_allocations_total"] == 0
    )
    timed_ok = (
        tests_exit == 0
        and len(timed) > 0
        and receipt["timed_wrapper"]["rows_result_neutral"] == len(timed)
        and receipt["timed_wrapper"]["rows_one_factor_per_attempt"] == len(timed)
        and receipt["timed_wrapper"]["rows_eight_solves_per_attempt"] == len(timed)
    )
    instruction_ratios = [
        row["client_over_protected_instructions"]
        for row in instruction["rows"]
        if row.get("client_over_protected_instructions") is not None
    ]
    receipt["verdicts"] = {
        "bit_identity_through_the_trait": "PASS" if identity_ok else "FAIL",
        "timed_wrapper_neutral_and_counts": "PASS" if timed_ok else "FAIL",
        "zero_allocations_per_step_in_place": "PASS" if alloc_ok else "FAIL",
        "zero_allocations_per_step_faer": (
            "NOT_MET_BY_DESIGN: every faer factorization allocates inside "
            "LuFactorization::new, in the protected driver as well; see "
            "allocations.faer_rows_reported_not_asserted"
        ),
        "wall_at_most_1_05x_at_n_ge_64": "DIAGNOSTIC_ONLY: see wall_diagnostic.summary",
        "client_over_protected_instructions_max": (
            max(instruction_ratios) if instruction_ratios else None
        ),
        "client_over_protected_instructions_at_most_1_05": (
            ("PASS" if max(instruction_ratios) <= 1.05 else "FAIL")
            if instruction_ratios
            else "NOT_MEASURED"
        ),
        "clippy_new_warnings": receipt["clippy"]["new_warnings_count"],
        "rustfmt_check": "PASS" if receipt["rustfmt"]["exit_code"] == 0 else "FAIL",
        "protected_files_changed": len(receipt["protected_files_changed_vs_base"]),
    }
    receipt["notes"] = [
        "Faer identity is shown on the mean-field Brusselator (n = 200, half-full "
        "Jacobian): the protected policy picks faer only above order 64 with more "
        "than 10 % nonzeros, which Robertson, HIRES and the banded 1D Brusselator "
        "(N = 32, N = 100) never reach. On those the protected LU is in place; the "
        "faer client there is reported under off_policy_faer.",
        "Accepted states are compared through a recording Jacobian callback (one "
        "call per accepted state in either driver) and, for the client driver, "
        "through its accept control.",
        "exploratory/02 was run with a pre-commit variant of the faer factorization "
        "that dropped the old factors before building the new ones; the committed "
        "code builds first, as the protected driver does. Every exploratory wall "
        "ratio was taken on a loaded host and is not evidence for or against that "
        "variant.",
        "The faer path takes thousands of minor page faults per run in both "
        "drivers (wall_diagnostic.rows[*].*_minor_page_faults_per_run); the cause "
        "was not investigated in this node.",
        "TimedStageClient times the three trait phases only. RHS evaluations and "
        "the stage vector work belong to the driver, so the phases do not sum to "
        "the wall time (wall_diagnostic.summary[*].timed_phases_over_wall).",
    ]
    receipt["environment"]["loadavg_end"] = loadavg()

    out = HERE / "RECEIPT.json"
    out.write_text(json.dumps(receipt, indent=1, allow_nan=False) + "\n")
    print(json.dumps(receipt["verdicts"], indent=1))
    print(json.dumps(receipt["tests"], indent=1))
    print(f"wrote {out}")
    failed = [c["name"] for c in commands if c["exit_code"] != 0]
    if failed:
        print("commands with a nonzero exit code:", failed)
        sys.exit(1)


if __name__ == "__main__":
    main()
