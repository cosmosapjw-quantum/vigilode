#!/usr/bin/env python3
"""KS-N7b results: parse the raw logs of ks_n7b/logs/ and write
research/kernel_spectral_foundation_20261124/results/KS_N7B_RESULTS.json (strict JSON).

Inputs, all produced by executed runs (nothing is typed in by hand). Each log is written by
ks_n7b/run_logged.sh and starts with '# command:' and '# head:' and ends with '# exit:'.
  logs/env.txt                                   toolchain and host
  logs/spatial_separable_base_failing_first.log  the test target at the failing-first test commit
  logs/spatial_separable_after_record.log        the test target after the record commit (F5)
  logs/spatial_separable_after_default.log       the test target after the default commit
  logs/facade_suite.log                          cargo test -p vigilode --release --no-fail-fast
  logs/doc.log                                   RUSTDOCFLAGS='-D warnings' cargo doc (CI docs job)
  logs/clippy_vigilode.log, logs/clippy_integrators.log, logs/fmt_check.log, logs/verify_run.log
  logs/public_api_diff.log                       git diff of crates/vigilode/src from the base
  logs/scope_name_status.log                     git diff --name-status from the base
  logs/failing_first_scope.log                   git diff --stat base..failing-first commit, crates/
  logs/commits.log                               git log base..head (hash and subject)

The thresholds are the ones stated in ks_n7b/NOTES.txt sections 4 and 5, committed before any
run. There is no timing here: the performance verdict is the owner-host cell KS-N7-T.

Usage (repository root): python3 -I research/kernel_spectral_foundation_20261124/ks_n7b/make_results.py
"""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
UNIT = HERE.parent
ROOT = UNIT.parents[1]
LOGS = HERE / "logs"
OUT = UNIT / "results" / "KS_N7B_RESULTS.json"
BOUND = 1.0e-13
ALLOWED_FILES = (
    "crates/vigilode/src/spatial/separable.rs",
    "crates/vigilode/src/spatial.rs",
    "crates/vigilode/tests/spatial_separable.rs",
)
ALLOWED_PREFIXES = (
    "research/kernel_spectral_foundation_20261124/ks_n7b/",
    "research/kernel_spectral_foundation_20261124/results/KS_N7B_",
)
FAILING_FIRST = "the_run_record_names_the_box_transform_that_ran"
DEFAULT_TEST = "the_box_defaults_to_the_paired_real_transform"
SELECTION_TEST = "box_transform_selection_defaults_parses_and_reuses_prepared_solvers"
AGREEMENT_TEST = "paired_real_box_transform_agrees_with_the_complex_transform"
T1T2_TESTS = ("separable_stage_solves_are_exact_and_mass_consistent", "paired_real_box_solves_meet_the_t1_t2_bounds")
BOX_IVP_TESTS = (
    "separable_route_conserves_mass_and_matches_the_reference",
    "paired_real_box_route_matches_the_reference_and_the_complex_route",
    FAILING_FIRST,
    DEFAULT_TEST,
)
BASE_TAGS = ("SL_N5_T1T2", "SL_N5_T3", "SL_N5_T5", "KS_N7_AGREEMENT", "KS_N7_T1T2", "KS_N7_T3", "KS_N7_T3_ROUTES")


def rel(path):
    return str(path.relative_to(ROOT))


def read(path):
    if not path.exists():
        raise SystemExit(f"missing input {rel(path)}")
    return path.read_text()


def header(text, key):
    match = re.search(rf"^# {key}: (.*)$", text, re.M)
    return match.group(1).strip() if match else None


def exit_code(text, path):
    value = header(text, "exit")
    if value is None:
        raise SystemExit(f"{rel(path)}: no '# exit:' line")
    return int(value)


def body(text):
    """The raw output of a log without its header and trailer lines."""
    return [line for line in text.splitlines() if not line.startswith("# ")]


def tagged_raw(text, tag):
    """Sorted raw `TAG {...}` strings (the tag may follow libtest's 'test ... ' prefix)."""
    rows = []
    for line in text.splitlines():
        position = line.find(tag + " {")
        if position >= 0:
            rows.append(line[position:])
    return sorted(rows)


def tagged(text, tag):
    return [json.loads(row[len(tag) + 1:]) for row in tagged_raw(text, tag)]


def test_outcomes(text):
    outcomes = dict(re.findall(r"^test (\w+) \.\.\. (ok|FAILED|ignored)", text, re.M))
    summary = [
        {"result": s[0], "passed": int(s[1]), "failed": int(s[2]), "ignored": int(s[3])}
        for s in re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text, re.M)
    ]
    return outcomes, summary


def summary_text(summary):
    return "; ".join(f"{s['result']}: {s['passed']} passed, {s['failed']} failed, {s['ignored']} ignored" for s in summary)


def test_log(name):
    path = LOGS / name
    text = read(path)
    outcomes, summary = test_outcomes(text)
    return {
        "path": path,
        "text": text,
        "head": header(text, "head"),
        "dirty": header(text, "dirty"),
        "command": header(text, "command"),
        "exit": exit_code(text, path),
        "outcomes": outcomes,
        "summary": summary,
    }


def check_entry(name, log, result, **extra):
    entry = {
        "name": name,
        "command": log["command"],
        "head": log["head"],
        "exit_code": log["exit"],
        "result": result,
        "log": rel(log["path"]),
    }
    entry.update(extra)
    return entry


def plain_log(file):
    path = LOGS / file
    text = read(path)
    return {
        "path": path,
        "text": text,
        "head": header(text, "head"),
        "command": header(text, "command"),
        "exit": exit_code(text, path),
    }


def main():
    env = dict(line.split(": ", 1) for line in read(LOGS / "env.txt").splitlines() if ": " in line)
    base = test_log("spatial_separable_base_failing_first.log")
    record = test_log("spatial_separable_after_record.log")
    default = test_log("spatial_separable_after_default.log")
    facade = test_log("facade_suite.log")

    # Failing-first test on the base: FAILED by an assertion, every other test ok.
    panic = re.search(
        rf"^thread '{FAILING_FIRST}'(?: \(\d+\))? panicked at (\S+):\n(assertion[^\n]*)\n\s*left: ([^\n]*)\n\s*right: ([^\n]*)",
        base["text"],
        re.M,
    )
    others_ok = all(v in ("ok", "ignored") for k, v in base["outcomes"].items() if k != FAILING_FIRST)
    ff_scope_text = read(LOGS / "failing_first_scope.log")
    ff_scope_files = re.findall(r"^ (\S+)\s+\|", ff_scope_text, re.M)
    failing_first = {
        "test": f"crates/vigilode/tests/spatial_separable.rs::{FAILING_FIRST}",
        "commit": base["head"],
        "dirty_tracked_files": base["dirty"],
        "crates_changed_since_base": ff_scope_files,
        "outcome": base["outcomes"].get(FAILING_FIRST),
        "exit_code": base["exit"],
        "summary": summary_text(base["summary"]),
        "panic_location": panic.group(1) if panic else None,
        "assertion": panic.group(2).split(": {")[0] if panic else None,
        "left": panic.group(3).strip() if panic else None,
        "right": panic.group(4).strip() if panic else None,
        "other_tests_ok": others_ok,
        "log": rel(base["path"]),
    }
    failing_first_pass = (
        base["exit"] != 0
        and failing_first["outcome"] == "FAILED"
        and panic is not None
        and failing_first["assertion"].startswith("assertion")
        and failing_first["left"] == "Null"
        and failing_first["right"] == '"complex"'
        and others_ok
        and ff_scope_files == ["crates/vigilode/tests/spatial_separable.rs"]
        and base["dirty"] == "0"
    )

    # The record: the failing-first test passes after the record commit and after the default one.
    expected_record = {"box_12x10": ["complex", "paired-real"], "sphere_7x10": None}
    records = {}
    for name, log in (("after_record", record), ("after_default", default)):
        rows = tagged(log["text"], "KS_N7B_RECORD")
        records[name] = {
            "commit": log["head"],
            "outcome": log["outcomes"].get(FAILING_FIRST),
            "KS_N7B_RECORD": rows[0] if len(rows) == 1 else None,
        }
    record_pass = all(
        r["outcome"] == "ok" and r["KS_N7B_RECORD"] == expected_record for r in records.values()
    ) and record["exit"] == 0 and default["exit"] == 0

    # The default.
    default_rows = tagged(default["text"], "KS_N7B_DEFAULT")
    default_line = default_rows[0] if len(default_rows) == 1 else {}
    default_pass = (
        default["outcomes"].get(DEFAULT_TEST) == "ok"
        and default["outcomes"].get(SELECTION_TEST) == "ok"
        and default_line.get("default") == "paired-real"
        and default_line.get("default_record") == "paired-real"
        and default_line.get("complex_record") == "complex"
        and "sphere_record" in default_line
        and default_line["sphere_record"] is None
        and default_line.get("default_equals_paired_bits") is True
    )

    # The base run's tagged lines, unchanged by the record commit and by the default commit.
    identity = {}
    for tag in BASE_TAGS:
        lines = {name: tagged_raw(log["text"], tag) for name, log in (("base", base), ("after_record", record), ("after_default", default))}
        identity[tag] = {
            "lines": len(lines["base"]),
            "identical_after_record": lines["after_record"] == lines["base"],
            "identical_after_default": lines["after_default"] == lines["base"],
        }
    agreement = tagged(default["text"], "KS_N7_AGREEMENT")
    t1t2 = {row["transform"]: row for row in tagged(default["text"], "KS_N7_T1T2")}
    sl_t1t2 = tagged(default["text"], "SL_N5_T1T2")
    if len(agreement) != 1 or set(t1t2) != {"paired-real", "complex"} or len(sl_t1t2) != 1:
        raise SystemExit("the default log lacks the KS_N7_AGREEMENT, KS_N7_T1T2 or SL_N5_T1T2 lines")
    agreement, sl_t1t2 = agreement[0], sl_t1t2[0]
    within_bounds = (
        agreement["max_normalized_difference"] <= BOUND
        and all(row["max_backward_error"] <= BOUND and row["max_mass_defect"] <= BOUND for row in t1t2.values())
        and sl_t1t2["max_backward_error"] <= BOUND
        and sl_t1t2["max_mass_defect"] <= BOUND
    )
    unchanged_pass = (
        all(identity[tag]["identical_after_record"] and identity[tag]["identical_after_default"]
            for tag in ("KS_N7_AGREEMENT", "KS_N7_T1T2"))
        and all(default["outcomes"].get(test) == "ok" for test in (AGREEMENT_TEST,) + T1T2_TESTS)
        and within_bounds
    )

    # Whole IVPs on box grids: the tests pass; the default run's report equals the explicit
    # PairedReal route line of KS-N7 on the T3 box (Verified, the default mode).
    t3_default = [r for r in tagged(default["text"], "SL_N5_T3") if r["case"] == "box-12x10"]
    ks_t3 = [r for r in tagged(default["text"], "KS_N7_T3") if r["case"] == "box-12x10"]
    t3_comparison = []
    for row in t3_default:
        if row["mode"] == "shipped":
            continue
        arms = {r["transform"]: r for r in ks_t3 if r["mode"] == row["mode"]}
        t3_comparison.append({
            "mode": row["mode"],
            "default_route": {k: row[k] for k in ("error", "attempts", "solves")},
            "equals_paired_real_line": all(row[k] == arms["paired-real"][k] for k in ("error", "attempts", "solves")),
            "equals_complex_line": all(row[k] == arms["complex"][k] for k in ("error", "attempts", "solves")),
        })
    verified_paired = [r for r in ks_t3 if r["transform"] == "paired-real" and r["mode"].startswith("Verified")]
    verified_complex = [r for r in ks_t3 if r["transform"] == "complex" and r["mode"].startswith("Verified")]
    report_matches_paired = bool(verified_paired) and default_line.get("max_mass_defect_default") == verified_paired[0]["max_mass_defect"] \
        and default_line.get("max_backward_error_default") == verified_paired[0]["max_backward_error"]
    report_matches_complex = bool(verified_complex) and default_line.get("max_mass_defect_default") == verified_complex[0]["max_mass_defect"] \
        and default_line.get("max_backward_error_default") == verified_complex[0]["max_backward_error"]
    box_ivp_pass = (
        all(default["outcomes"].get(test) == "ok" for test in BOX_IVP_TESTS)
        and len(t3_comparison) == 2
        and all(c["equals_paired_real_line"] for c in t3_comparison)
        and report_matches_paired
    )

    # The facade suite.
    facade_failed = re.findall(r"^test (\S+) \.\.\. FAILED", facade["text"], re.M)
    totals = {key: sum(s[key] for s in facade["summary"]) for key in ("passed", "failed", "ignored")}
    binaries = re.findall(r"^\s+(?:Running|Doc-tests) (\S+)", facade["text"], re.M)
    # Every test binary (and the doc-tests, which print a merged and a standalone block) must
    # end with at least one result block: a crashed binary would leave its section without one.
    sections = re.split(r"^\s+(?:Running|Doc-tests) \S.*$", facade["text"], flags=re.M)[1:]
    blocks = [len(re.findall(r"^test result: ", section, re.M)) for section in sections]
    facade_pass = (
        facade["exit"] == 0
        and totals["failed"] == 0
        and not facade_failed
        and len(sections) == len(binaries)
        and all(count >= 1 for count in blocks)
    )

    checks = [
        check_entry("spatial_separable at the failing-first test commit (expected to fail)", base, summary_text(base["summary"])),
        check_entry("spatial_separable after the record commit", record, summary_text(record["summary"])),
        check_entry("spatial_separable after the default commit", default, summary_text(default["summary"])),
        check_entry(
            "facade suite (--no-fail-fast)",
            facade,
            f"{totals['passed']} passed, {totals['failed']} failed, {totals['ignored']} ignored; "
            f"{len(binaries)} sections (test binaries and doc-tests), {len(facade['summary'])} result blocks",
            failed_tests=facade_failed,
            sections=len(binaries),
            sections_without_result=sum(1 for count in blocks if count == 0),
        ),
    ]
    other = {}
    for name, file in (
        ("rustdoc -D warnings (CI docs job)", "doc.log"),
        ("clippy vigilode --all-targets", "clippy_vigilode.log"),
        ("clippy rodas5p-integrators --lib", "clippy_integrators.log"),
        ("fmt check", "fmt_check.log"),
        ("verify_run", "verify_run.log"),
    ):
        log = plain_log(file)
        lines = [l for l in body(log["text"]) if l.strip()]
        warnings = sum(1 for l in lines if l.startswith("warning") or l.startswith("error"))
        other[name] = log
        checks.append(check_entry(
            name, log, "pass" if log["exit"] == 0 else "fail",
            detail=lines[-1] if lines else "", warning_or_error_lines=warnings,
        ))
    checks_pass = all(c["exit_code"] == 0 for c in checks[1:])

    # Scope and public API.
    scope_text = read(LOGS / "scope_name_status.log")
    changed = re.findall(r"^([AMDR]\d*)\t(\S+)(?:\t(\S+))?$", scope_text, re.M)
    paths = [m[2] or m[1] for m in changed]
    outside = [p for p in paths if p not in ALLOWED_FILES and not p.startswith(ALLOWED_PREFIXES)]
    api_text = read(LOGS / "public_api_diff.log")
    api_lines = [l for l in body(api_text) if re.match(r"^[-+](?![-+])", l)]
    api_relevant = [l for l in api_lines if re.search(r"\bpub\b|#\[default\]|#\[derive|#\[serde|#\[non_exhaustive", l)]
    commits = [line.split(" ", 1) for line in body(read(LOGS / "commits.log")) if re.match(r"^[0-9a-f]{40} ", line)]

    result = {
        "schema": "vigilode-ks-n7b-results-v1",
        "task": "KS-N7b",
        "title": "PairedReal as the default box transform, after recording the transform in the run record (review F5 of KS-N7)",
        "branch": "research/stage-lattice-referee-20261124-kschain",
        "generated_by": rel(pathlib.Path(__file__).resolve()),
        "notes": rel(HERE / "NOTES.txt"),
        "commits": [{"commit": c[0], "subject": c[1]} for c in commits],
        "code_commits": {
            "failing_first_test": base["head"],
            "record": record["head"],
            "default": default["head"],
        },
        "environment": {
            "toolchain": env.get("rustc"),
            "cargo": env.get("cargo"),
            "cpu": env.get("cpu"),
            "kernel": env.get("kernel"),
            "taskset": env.get("taskset"),
            "cargo_target_dir": env.get("cargo_target_dir"),
            "cargo_semver_checks": env.get("cargo_semver_checks"),
        },
        "design": {
            "choice": "SeparableReport::box_transform: Option<BoxTransform> (Some on the periodic box, None on a sphere grid), written by SeparableStages::client from the kernel it builds; BoxTransform derives Serialize (kebab case: complex, paired-real)",
            "why": [
                "The client id of DriverWork::SeparableClient is the constant SEPARABLE_STAGE_CLIENT_ID written in crates/vigilode/src/solve.rs:530, outside this task's paths; a constant cannot depend on the run, so a second client id needs a solve.rs edit.",
                "The SeparableReport is the only per-run value that separable.rs hands to a Solution (StageLinearSystem has no identifier; the driver's client is dropped).",
                "Decision KS-N7-T and the KS-N7 protocol name the report first; the report is typed; the client id keeps naming the client (same certificates for both transforms).",
            ],
            "api_change": {
                "semver_tool": "cargo-semver-checks not installed (env.txt); public-API changes reviewed by hand from logs/public_api_diff.log",
                "major": "SeparableReport gains the public field box_transform (all fields public, not non_exhaustive): downstream struct literals that list every field, and exhaustive destructuring patterns, stop compiling; literals with ..Default::default() keep compiling (the field defaults to None). The type was added at a0ec98ba on this research branch and is unreleased (not on main); the workspace has no such literal or pattern.",
                "minor": "BoxTransform implements serde::Serialize (additive).",
                "behavior": "BoxTransform::default() is PairedReal instead of Complex: a box built without with_box_transform runs the paired transform (same linear map, different rounding; K17 bounds the difference). The serialized run record gains the key report.box_transform.",
                "unchanged": "No item removed or renamed; no signature changed; crates/vigilode/src/spatial.rs exports unchanged; SEPARABLE_STAGE_CLIENT_ID unchanged.",
                "diff_lines": api_relevant,
                "diff_log": rel(LOGS / "public_api_diff.log"),
            },
            "alternative_left_open": "A non-breaking second client id for the paired box solve: a const and a getter in separable.rs plus solve.rs:530 writing stages.client_id(); it can be added to the report field or replace it (orchestrator's choice; solve.rs is outside this task's paths).",
        },
        "scope": {
            "changed_paths": paths,
            "outside_allowed_paths": outside,
            "log": rel(LOGS / "scope_name_status.log"),
        },
        "failing_first": failing_first,
        "record": records,
        "default": {
            "KS_N7B_DEFAULT": default_line,
            "test": f"crates/vigilode/tests/spatial_separable.rs::{DEFAULT_TEST}",
            "selection_test": SELECTION_TEST,
        },
        "unchanged_lines": identity,
        "bounds": {
            "statement": "agreement ||x_p - x_c||_inf <= 1e-13 S/(z - c); T1 backward error <= 1e-13; T2 mass defect <= 1e-13 (scales of separable_stage_solves_are_exact_and_mass_consistent)",
            "KS_N7_AGREEMENT": agreement,
            "KS_N7_T1T2": t1t2,
            "SL_N5_T1T2_default_arm": sl_t1t2,
        },
        "box_ivp": {
            "tests": {test: default["outcomes"].get(test) for test in BOX_IVP_TESTS},
            "SL_N5_T3_box_12x10_against_KS_N7_T3": t3_comparison,
            "default_report_equals_paired_real_route_line": report_matches_paired,
            "default_report_equals_complex_route_line": report_matches_complex,
            "caveat": "At the printed precision the T3 box lines of both transforms agree (error, attempts, solves), so the T3 comparison alone does not tell the arms apart; the report values (max mass defect and backward error) and the bitwise equality of the default and explicit PairedReal runs do.",
        },
        "checks": checks,
        "discriminators": [
            {"name": "failing-first test fails on the base by assertion",
             "frozen_threshold": "at the failing-first commit (crates/ changed only in the test file): the new test FAILED by an assertion (left Null, right \"complex\"), every other test ok, exit != 0",
             "measured": f"{failing_first['outcome']}: {failing_first['assertion']} (left {failing_first['left']}, right {failing_first['right']}); others ok: {others_ok}; exit {base['exit']}",
             "verdict": "PASS" if failing_first_pass else "FAIL"},
            {"name": "the run record distinguishes the transforms",
             "frozen_threshold": "serialized report.box_transform = complex / paired-real on the T3 box and null on the T3 sphere, after the record commit and after the default commit",
             "measured": json.dumps({k: v["KS_N7B_RECORD"] for k, v in records.items()}),
             "verdict": "PASS" if record_pass else "FAIL"},
            {"name": "PairedReal is the default; Complex stays selectable",
             "frozen_threshold": "BoxTransform::default() and separable_stages().box_transform() are PairedReal; a default run records paired-real (typed and serialized) and returns the explicit PairedReal bits; explicit Complex records complex; a sphere records None",
             "measured": json.dumps({k: default_line.get(k) for k in ("default", "default_record", "complex_record", "sphere_record", "default_equals_paired_bits")}),
             "verdict": "PASS" if default_pass else "FAIL"},
            {"name": "T1/T2 and the agreement test unchanged",
             "frozen_threshold": "KS_N7_AGREEMENT and KS_N7_T1T2 lines identical to the base run; every bound <= 1e-13",
             "measured": f"agreement max normalized {agreement['max_normalized_difference']:.3e} over {agreement['cases']} cases; PairedReal backward {t1t2['paired-real']['max_backward_error']:.3e}, mass {t1t2['paired-real']['max_mass_defect']:.3e}; default arm T1/T2 {sl_t1t2['max_backward_error']:.3e} / {sl_t1t2['max_mass_defect']:.3e}; identical lines: {all(identity[t]['identical_after_default'] for t in ('KS_N7_AGREEMENT', 'KS_N7_T1T2'))}",
             "verdict": "PASS" if unchanged_pass else "FAIL"},
            {"name": "whole-IVP tests on box grids pass",
             "frozen_threshold": "T3, the KS-N7 route test and the two KS-N7b tests ok; SL_N5_T3 box-12x10 equals the KS_N7_T3 paired-real line of the same mode",
             "measured": json.dumps({"tests_ok": all(default["outcomes"].get(t) == "ok" for t in BOX_IVP_TESTS), "t3_equals_paired_line": [c["equals_paired_real_line"] for c in t3_comparison], "report_equals_paired_route_line": report_matches_paired}),
             "verdict": "PASS" if box_ivp_pass else "FAIL"},
            {"name": "facade suite",
             "frozen_threshold": "cargo test -p vigilode --release --no-fail-fast: exit 0, 0 failed",
             "measured": f"{totals['passed']} passed, {totals['failed']} failed, {totals['ignored']} ignored; {len(binaries)} sections, each with a result block: {all(count >= 1 for count in blocks)}",
             "verdict": "PASS" if facade_pass else "FAIL"},
            {"name": "docs, lint, format, run verifier",
             "frozen_threshold": "exit 0 each",
             "measured": "; ".join(f"{c['name']}: exit {c['exit_code']}" for c in checks[4:]),
             "verdict": "PASS" if all(c["exit_code"] == 0 for c in checks[4:]) else "FAIL"},
            {"name": "scope",
             "frozen_threshold": "only the task's allowed paths changed",
             "measured": f"{len(paths)} paths changed, {len(outside)} outside",
             "verdict": "PASS" if not outside else "FAIL"},
        ],
        "open_issues": [
            "RUN.json stage KS-N7 and claim K17 say Complex stays the default; the orchestrator updates them (this task may not edit RUN.json). THEORY.md does not name the box default.",
            "The default-arm T1/T2 line SL_N5_T1T2 (claims K5, K4) after the default change is "
            + ("identical to the base run" if identity["SL_N5_T1T2"]["identical_after_default"] else "changed")
            + f": backward {sl_t1t2['max_backward_error']:.3e}, mass {sl_t1t2['max_mass_defect']:.3e}.",
            "The non-breaking second client id is left to the orchestrator (needs solve.rs:530).",
        ],
    }
    OUT.write_text(json.dumps(result, indent=1, allow_nan=False) + "\n")
    verdicts = {d["name"]: d["verdict"] for d in result["discriminators"]}
    print(f"wrote {rel(OUT)}: " + "; ".join(f"{k}: {v}" for k, v in verdicts.items()))
    return 0 if all(v == "PASS" for v in verdicts.values()) and checks_pass else 1


if __name__ == "__main__":
    sys.exit(main())
