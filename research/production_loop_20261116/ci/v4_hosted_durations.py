#!/usr/bin/env python3
"""Hosted step and command durations behind the time budgets of CI v4 (node V33, owner decision
OD-L1-29).

Inputs (raw, unedited): v4_logs/hosted/jobs_<run id>.json, the GitHub API job listings
(`gh api repos/<repo>/actions/runs/<run id>/jobs`), and v4_logs/hosted/job_<job id>.log.gz, the
job logs (`gh api repos/<repo>/actions/jobs/<job id>/logs`, gzip -9 -n). Runs: 45414722991
(Production CI v3 at 7012e9d5: jobs test, test-cli-fair-ab), 45414722588 (Ignored long tests v3 at
7012e9d5), 45378440571 (Ignored long tests v2 at a66215e7), 45378440406 (Production CI v2 at
a66215e7). These are the runs of CI triage sections 12.2 and 12.3.

Output: v4_logs/hosted_durations.json (strict JSON). Every number is computed here from the
inputs; nothing is typed in.
"""

import glob
import gzip
import json
import os
import re
import sys
from datetime import datetime

HERE = os.path.dirname(os.path.abspath(__file__))
HOSTED = os.path.join(HERE, "v4_logs", "hosted")
OUT = os.path.join(HERE, "v4_logs", "hosted_durations.json")
JOBS = ("test", "test-cli-fair-ab", "ignored-tests")
LINE = re.compile(r"^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d+))?Z ?(.*)$")
FLAGS = re.compile(r" --message-format=\S+| --locked| --no-fail-fast| --profile \S+| -- --ignored.*| -- .*")


def when(text):
    """Seconds since the epoch of an API time ('2026-11-24T14:29:53Z')."""
    return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").timestamp() if text else None


def stamp(day, fraction):
    base = datetime.strptime(day, "%Y-%m-%dT%H:%M:%S").timestamp()
    return base + (int(fraction[:6].ljust(6, "0")) / 1e6 if fraction else 0.0)


def listings():
    out = []
    for path in sorted(glob.glob(os.path.join(HOSTED, "jobs_*.json"))):
        run = os.path.basename(path)[5:-5]
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
        for job in data.get("jobs", []):
            if job["name"] not in JOBS:
                continue
            steps = []
            for s in job.get("steps", []):
                a, b = when(s.get("started_at")), when(s.get("completed_at"))
                steps.append({"number": s["number"], "name": s["name"], "conclusion": s.get("conclusion"),
                              "seconds": None if a is None or b is None else b - a})
            a, b = when(job.get("started_at")), when(job.get("completed_at"))
            out.append({"run": run, "job": job["id"], "name": job["name"], "conclusion": job.get("conclusion"),
                        "started_at": job.get("started_at"), "completed_at": job.get("completed_at"),
                        "minutes": None if a is None or b is None else round((b - a) / 60, 2), "steps": steps})
    return out


def log_facts(path):
    """Per cargo command of the run step: start, seconds until the next command (or the end of the
    step) and whether the step ended inside it; the silent tail after the last output line."""
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()
    rows = []
    for line in lines:
        m = LINE.match(line)
        if m:
            rows.append((stamp(m.group(1), m.group(2)), m.group(3)))
    run_start = next((t for t, text in rows if text.startswith("##[group]Run python3 -u tools/ci/select_tests.py run")
                      or text.startswith("##[group]Run cargo test")), None)
    cmds = [(i, t, text) for i, (t, text) in enumerate(rows) if text.startswith("+ cargo test")]
    end_index = None
    for i, (t, text) in enumerate(rows):
        if cmds and i > cmds[-1][0] and (text.startswith("##[error]") or text.startswith("Post job cleanup.")
                                         or text.startswith("##[group]Run actions/upload-artifact")):
            end_index = i
            break
    end = rows[end_index][0] if end_index is not None else None
    cancelled = end_index is not None and "canceled" in rows[end_index][1]
    out = []
    for k, (i, t, text) in enumerate(cmds):
        nxt = cmds[k + 1][1] if k + 1 < len(cmds) else end
        name = FLAGS.sub("", text[len("+ cargo test"):]).strip()
        out.append({"command": name, "build": "--no-run" in text,
                    "seconds": None if nxt is None else round(nxt - t, 1),
                    "cut_by_the_end_of_the_step": k + 1 == len(cmds) and cancelled})
    tail = None
    if end_index is not None and not cancelled:
        last_output = max(t for t, text in rows[:end_index] if text.strip())
        tail = round(end - last_output, 1)
    tests = [c for c in out if not c["build"] and c["seconds"] is not None and not c["cut_by_the_end_of_the_step"]]
    seconds = sorted(c["seconds"] for c in tests)

    def share_below(limit):
        return round(sum(1 for s in seconds if s < limit) / len(seconds), 4) if seconds else None
    return {"log": os.path.basename(path), "run_step_started": run_start, "commands": len(out),
            "test_commands_finished": len(tests), "build_commands": sum(1 for c in out if c["build"]),
            "build_seconds": round(sum(c["seconds"] or 0 for c in out if c["build"]), 1),
            "test_seconds": round(sum(seconds), 1), "step_ended_by_cancellation": cancelled,
            "silent_tail_after_last_output_seconds": tail,
            "longest_finished_test_commands": sorted(tests, key=lambda c: -c["seconds"])[:6],
            "cut_command": next((c for c in out if c["cut_by_the_end_of_the_step"]), None),
            "share_of_finished_test_commands_below": {f"{m} min": share_below(60 * m) for m in (5, 10, 20, 30)}}


def main():
    jobs = listings()
    logs = {os.path.basename(p)[4:-7]: log_facts(p) for p in sorted(glob.glob(os.path.join(HOSTED, "job_*.log.gz")))}
    after = [s for j in jobs for s in j["steps"]
             if s["conclusion"] == "success" and (s["name"].startswith("Keep the outcome") or s["name"].startswith("Post ")
                                                   or s["name"] == "Complete job")]
    setup = [s for j in jobs for s in j["steps"] if s["name"] == "Set up job"]
    before_run = []
    for j in jobs:
        total = 0.0
        for s in j["steps"]:
            if s["name"].startswith("Run the selected"):
                before_run.append({"job": j["job"], "name": j["name"], "run": j["run"], "seconds": total})
                break
            total += s["seconds"] or 0
    tails = [v["silent_tail_after_last_output_seconds"] for v in logs.values()
             if v["silent_tail_after_last_output_seconds"] is not None]
    result = {
        "schema": "vigilode.loop.ci-v4-hosted-durations.v1",
        "inputs": sorted(os.path.basename(p) for p in glob.glob(os.path.join(HOSTED, "*"))),
        "jobs": jobs,
        "logs": logs,
        "margin_inputs": {
            "set_up_job_seconds_max": max(s["seconds"] for s in setup),
            "steps_after_the_run_step_seconds_max": max(s["seconds"] for s in after),
            "steps_after_the_run_step": sorted({f"{s['name']}: {s['seconds']:.0f} s" for s in after}),
            "steps_before_the_run_step_seconds": before_run,
            "measurement_tail_seconds_max": max(tails) if tails else None,
        },
    }
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, sort_keys=True, allow_nan=False)
        f.write("\n")
    m = result["margin_inputs"]
    print(f"jobs: {len(jobs)}; logs: {len(logs)}")
    for j in jobs:
        print(f"  run {j['run']} job {j['job']} {j['name']}: {j['conclusion']}, {j['minutes']} min")
    for name, v in logs.items():
        print(f"  log {name}: {v['commands']} commands ({v['build_commands']} builds, {v['test_commands_finished']} "
              f"finished tests), tail {v['silent_tail_after_last_output_seconds']} s, cut {v['cut_command']}")
        for c in v["longest_finished_test_commands"][:3]:
            print(f"     {c['seconds']:8.1f} s  {c['command']}")
        print(f"     share of finished test commands below: {v['share_of_finished_test_commands_below']}")
    print(f"set up job max {m['set_up_job_seconds_max']} s; steps after the run step max "
          f"{m['steps_after_the_run_step_seconds_max']} s; measurement tail max {m['measurement_tail_seconds_max']} s")
    for b in m["steps_before_the_run_step_seconds"]:
        print(f"  before the run step: {b['name']} (run {b['run']}): {b['seconds']:.0f} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
