#!/usr/bin/env python3
"""Build PR190_CI.json from raw GitHub API output and job logs.

Every value in the output is parsed from files under raw/ (no typed-in
results). Run: python3 -I research/production_loop_20261116/ci/collect.py
"""
import gzip
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "raw"
HEAD = "0443025895dabf350f46fd3f0cdd0cf1d7fbcf85"
BASE = "ef04bfab6926fcd4ba5f5fa2bbd8696f5e69d011"

TIMESTAMP = re.compile(r"^\d{4}-\d\d-\d\dT[\d:.]+Z ")
ANSI = re.compile(r"\x1b\[[0-9;]*m")
# The A1 job exercises the command line parser on purpose; these are not failures.
EXPECTED_CLI_ERRORS = ("invalid value 'unknown-", "unexpected argument '--profile'")


def log_lines(path):
    with gzip.open(path, "rt", errors="replace") as handle:
        for raw in handle:
            yield ANSI.sub("", TIMESTAMP.sub("", raw.rstrip("\n")))


def parse_log(path):
    if not path.exists():
        return None
    out = {
        "log": str(path.relative_to(HERE)),
        "exit_code": None,
        "failed_tests": [],
        "panic_sites": [],
        "failed_test_binaries": [],
        "error_lines": [],
        "ledger_fail_lines": 0,
        "ledger_fail_kinds": {},
        "semver_failed_lints": [],
        "semver_summary": None,
        "unsupported_rustc": [],
        "missing_headers": [],
        "runner_image": None,
        "left_right": [],
    }
    lines = list(log_lines(path))
    for index, line in enumerate(lines):
        match = re.match(r"test (\S+) \.\.\. FAILED", line)
        if match and match.group(1) not in out["failed_tests"]:
            out["failed_tests"].append(match.group(1))
        match = re.search(r"panicked at (\S+?):(\d+):\d+:", line)
        if match:
            site = f"{match.group(1)}:{match.group(2)}"
            if site not in out["panic_sites"]:
                out["panic_sites"].append(site)
            window = [item.strip() for item in lines[index + 1 : index + 4]]
            pair = [item for item in window if item.startswith(("left:", "right:"))]
            if pair and pair not in out["left_right"]:
                out["left_right"].append(pair)
        match = re.search(r"to rerun pass `(.+?)`", line)
        if match and match.group(1) not in out["failed_test_binaries"]:
            out["failed_test_binaries"].append(match.group(1))
        if re.match(r"error(\[E\d+\])?: ", line) and not any(
            token in line for token in EXPECTED_CLI_ERRORS
        ):
            if len(out["error_lines"]) < 6:
                out["error_lines"].append(line[:260])
        if line.startswith("FAIL: "):
            out["ledger_fail_lines"] += 1
            kind = "stall rule" if line.startswith("FAIL: stall rule") else (
                "missing commit" if " is missing (" in line else "other"
            )
            out["ledger_fail_kinds"][kind] = out["ledger_fail_kinds"].get(kind, 0) + 1
        match = re.match(r"--- failure (\w+):", line)
        if match:
            out["semver_failed_lints"].append(match.group(1))
        if "semver requires new" in line:
            out["semver_summary"] = line.strip()[:200]
        match = re.search(r"(\S+@\S+) requires rustc (\S+)", line)
        if match:
            item = f"{match.group(1)} requires rustc {match.group(2)}"
            if item not in out["unsupported_rustc"]:
                out["unsupported_rustc"].append(item)
        match = re.search(r"fatal error: (\S+): No such file or directory", line)
        if match and match.group(1) not in out["missing_headers"]:
            out["missing_headers"].append(match.group(1))
        match = re.match(r"Image: (\S+)", line)
        if match and out["runner_image"] is None:
            out["runner_image"] = match.group(1)
        match = re.match(r"##\[error\]Process completed with exit code (\d+)", line)
        if match:
            out["exit_code"] = int(match.group(1))
    return out


def signature(parsed):
    """A comparable description of why a job failed."""
    if parsed is None:
        return None
    return {
        "failed_tests": parsed["failed_tests"],
        "panic_sites": parsed["panic_sites"],
        "left_right": parsed["left_right"],
        "ledger_fail_kinds": parsed["ledger_fail_kinds"],
        "semver_failed_lints": parsed["semver_failed_lints"],
        "unsupported_rustc": parsed["unsupported_rustc"],
        "missing_headers": parsed["missing_headers"],
    }


def main():
    runs = json.loads((RAW / "check_runs_04430258.json").read_text())
    ours = {run["name"]: run for run in runs["check_runs"]}
    base = {}
    for row in (RAW / "jobs_base_ef04bfab.tsv").read_text().splitlines():
        run_id, job_id, name, conclusion = row.split("\t")
        base[name] = {"run_id": int(run_id), "job_id": int(job_id), "conclusion": conclusion}

    jobs = []
    for name in sorted(set(ours) | set(base)):
        head_run = ours.get(name)
        head_log = parse_log(RAW / "logs_04430258" / f"{name}.job.log.gz")
        base_log = parse_log(RAW / "logs_base_ef04bfab" / f"{name}.job.log.gz")
        head_conclusion = head_run["conclusion"] if head_run else None
        base_conclusion = base.get(name, {}).get("conclusion")
        if head_conclusion == "failure" and base_conclusion == "failure":
            same = signature(head_log) == signature(base_log)
            relation = "fails_on_both_same_signature" if same else "fails_on_both_different_signature"
        elif head_conclusion == "failure":
            relation = "fails_only_on_head"
        elif base_conclusion == "failure":
            relation = "fails_only_on_base"
        else:
            relation = "no_failure"
        jobs.append(
            {
                "job": name,
                "relation": relation,
                "head": {
                    "conclusion": head_conclusion,
                    "job_id": head_run["id"] if head_run else None,
                    "url": head_run["html_url"] if head_run else None,
                    "started_at": head_run["started_at"] if head_run else None,
                    "completed_at": head_run["completed_at"] if head_run else None,
                    "parsed_log": head_log,
                },
                "base": {
                    "conclusion": base_conclusion,
                    "job_id": base.get(name, {}).get("job_id"),
                    "parsed_log": base_log,
                },
            }
        )

    def count(side, value):
        return sum(1 for job in jobs if job[side]["conclusion"] == value)

    relations = {}
    for job in jobs:
        relations[job["relation"]] = relations.get(job["relation"], []) + [job["job"]]
    result = {
        "schema": "vigilode.loop.ci-record.v1",
        "pull_request": 190,
        "head_commit": HEAD,
        "base_commit_for_comparison": BASE,
        "base_note": "programme baseline research/unified-t02-20261110; last pull-request CI run of the unification lineage before this loop",
        "summary": {
            "head": {key: count("head", key) for key in ("success", "failure", "skipped")},
            "base": {key: count("base", key) for key in ("success", "failure", "skipped")},
            "relations": relations,
        },
        "jobs": jobs,
    }
    target = HERE / "PR190_CI.json"
    target.write_text(json.dumps(result, indent=1, allow_nan=False) + "\n")
    print(json.dumps(result["summary"], indent=1))


if __name__ == "__main__":
    main()
