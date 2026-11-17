#!/usr/bin/env python3
"""Assemble RECEIPT.json of node V03G from the command journal, the golden
files and the negative control. Every number is read from those files; none
is typed in."""
import collections
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GOLDEN = ROOT / "crates/vigilode/tests/golden/default_path"
TEST = ROOT / "crates/vigilode/tests/default_path_golden.rs"
REQUIRED = ["release_bless", "release_run_1", "release_run_2", "debug_run", "clippy", "rustfmt_check"]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def out(*command):
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True).stdout.strip()


def main():
    journal = [
        json.loads(line)
        for line in (HERE / "logs/commands.jsonl").read_text().splitlines()
        if line.strip()
    ]
    golden = {}
    routes = collections.Counter()
    entry_points = collections.Counter()
    outcomes = collections.Counter()
    dense_cases = 0
    n_cases = 0
    grid = {}
    for path in sorted(GOLDEN.glob("*.json")):
        document = json.loads(path.read_text())
        entry = {
            "sha256": sha256(path),
            "bytes": path.stat().st_size,
            "mtime_utc": datetime.datetime.fromtimestamp(
                path.stat().st_mtime, datetime.timezone.utc
            ).isoformat(timespec="seconds"),
        }
        if "cases" in document:
            cases = document["cases"]
            assert document["n_cases"] == len(cases), path
            entry["n_cases"] = len(cases)
            n_cases += len(cases)
            for case in cases.values():
                entry_points[case["entry_point"]] += 1
                outcomes[case["outcome"]] += 1
                dense_cases += "dense_samples" in case
                capabilities = case.get("capabilities")
                if capabilities and "route" in capabilities and case["outcome"] == "solution":
                    routes[capabilities["route"]] += 1
        else:
            for table in ("select", "capabilities"):
                rows = document[table]["rows"]
                width = document[table]["code_width"]
                cells = sum(len(codes) // width for by in rows.values() for codes in by.values())
                assert cells == document[table]["n_entries"], (path, table)
                grid[table] = {
                    "n_entries": cells,
                    "n_rows": sum(len(by) for by in rows.values()),
                    "n_distinct_outcomes": len(document[table]["legend"]),
                }
            assert grid["select"]["n_entries"] == document["n_select_entries"]
            assert grid["capabilities"]["n_entries"] == document["n_capability_entries"]
            grid["callback_entries"] = document["callback_entries"]
            entry["n_select_entries"] = grid["select"]["n_entries"]
            entry["n_capability_entries"] = grid["capabilities"]["n_entries"]
        golden[path.name] = entry

    final_hashes = {name: entry["sha256"] for name, entry in golden.items()}
    steps = {}
    for label in REQUIRED:
        runs = [record for record in journal if record["label"] == label]
        if not runs:
            steps[label] = None
            continue
        last = runs[-1]
        steps[label] = {
            "journal_seq": last["seq"],
            "command": " ".join(last["command"]),
            "env": last["env"],
            "exit_code": last["exit_code"],
            "libtest_summaries": last["libtest_summaries"],
            "bless_variable_set": last["env"].get("VIGILODE_BLESS_DEFAULT_PATH") == "1",
            "golden_files_changed_by_run": last["golden_files_changed"],
            "golden_sha256_after_equals_final": last["golden_sha256_after"] == final_hashes,
            "runs_with_this_label": len(runs),
        }

    def compared_all(label):
        # A run without the bless variable compares every golden file byte for
        # byte inside the test, so exit code 0 with no failed test means every
        # file was reproduced; the hashes show the run left the files alone.
        step = steps.get(label)
        return bool(
            step
            and not step["bless_variable_set"]
            and step["exit_code"] == 0
            and len(step["libtest_summaries"]) == 1
            and step["libtest_summaries"][0]["failed"] == 0
            and step["libtest_summaries"][0]["passed"] > 0
            and step["libtest_summaries"][0]["filtered_out"] == 0
            and not step["golden_files_changed_by_run"]
            and step["golden_sha256_after_equals_final"]
        )

    def written_by_bless():
        step = steps.get("release_bless")
        if not (step and step["bless_variable_set"] and step["exit_code"] == 0):
            return False
        run = journal[step["journal_seq"] - 1]
        return all(
            run["started_utc"] <= entry["mtime_utc"] <= run["finished_utc"]
            for entry in golden.values()
        ) and step["golden_sha256_after_equals_final"]

    negative_path = HERE / "logs/negative_control.json"
    negative = json.loads(negative_path.read_text()) if negative_path.exists() else None
    additive_path = HERE / "logs/additive_control.json"
    additive = json.loads(additive_path.read_text()) if additive_path.exists() else None
    receipt = {
        "schema": "vigilode.loop-l1.v03g-receipt.v1",
        "node": "V03G",
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "worktree": str(ROOT),
        "branch": out("git", "rev-parse", "--abbrev-ref", "HEAD"),
        "head_before_commit": out("git", "rev-parse", "HEAD"),
        "toolchain": {
            "rustc": out("rustc", "+1.94.1", "--version"),
            "cargo": out("cargo", "+1.94.1", "--version"),
        },
        "test_file": {
            "path": str(TEST.relative_to(ROOT)),
            "sha256": sha256(TEST),
            "lines": len(TEST.read_text().splitlines()),
        },
        "golden_files": golden,
        "totals": {
            "n_golden_files": len(golden),
            "n_numerical_cases": n_cases,
            "n_dense_output_cases_with_five_samples": dense_cases,
            "n_grid_select_entries": grid["select"]["n_entries"],
            "n_grid_capability_entries": grid["capabilities"]["n_entries"],
            "n_grid_rows": grid["select"]["n_rows"],
            "grid_callback_entries": grid["callback_entries"],
            "grid_distinct_select_outcomes": grid["select"]["n_distinct_outcomes"],
            "grid_distinct_capability_outcomes": grid["capabilities"]["n_distinct_outcomes"],
            "golden_bytes": sum(entry["bytes"] for entry in golden.values()),
        },
        "coverage": {
            "cases_per_entry_point": dict(sorted(entry_points.items())),
            "cases_per_outcome": dict(sorted(outcomes.items())),
            "rodas5p_solutions_per_facade_route": dict(sorted(routes.items())),
        },
        "required_steps": steps,
        "verdicts": {
            "golden_files_written_by_the_release_bless_run": written_by_bless(),
            "two_release_runs_reproduce_every_golden_file": compared_all("release_run_1")
            and compared_all("release_run_2"),
            "debug_profile_matches_release_golden_bit_for_bit": compared_all("debug_run"),
            "clippy_clean": bool(steps.get("clippy")) and steps["clippy"]["exit_code"] == 0,
            "rustfmt_clean": bool(steps.get("rustfmt_check")) and steps["rustfmt_check"]["exit_code"] == 0,
            "negative_control_detected_every_mutation": bool(negative)
            and all(r["detected"] and r["golden_file_restored"] for r in negative),
            "added_key_rule_fails_strict_and_passes_when_allowed": bool(additive)
            and additive["as_expected"],
        },
        "negative_control": negative,
        "additive_control": additive,
        "excluded_entry_points": [
            {
                "entry_point": "Dopri5Ensemble, execution_policy (ParallelExecution::rayon)",
                "reason": "worker threads and measured wall time decide the schedule; not a default route",
            },
            {
                "entry_point": "SwitchingSolver / ProductionConfig::selective_jvp / AutomaticOde with derivative permission, AutomaticOde::solve_adaptive",
                "reason": "opt-in derivative routes, outside the default-facing scope of this oracle",
            },
            {
                "entry_point": "PeerOde, ExplicitSession, Lawson and chart adapters",
                "reason": "explicit opt-in APIs, outside the default-facing scope of this oracle",
            },
        ],
        "excluded_for_non_reproducibility_among_the_cases": []
        if compared_all("release_run_1") and compared_all("release_run_2")
        else None,
        "wall_time_note": "wall_seconds_diagnostic_only in the journal is diagnostic only (shared host)",
        "journal": journal,
    }
    (HERE / "RECEIPT.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True, allow_nan=False) + "\n"
    )
    print(json.dumps({"totals": receipt["totals"], "verdicts": receipt["verdicts"]}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
