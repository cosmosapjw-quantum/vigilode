#!/usr/bin/env python3
"""Run one command of node V03G and append its record to logs/commands.jsonl.

usage: run_step.py <label> [NAME=VALUE ...] -- <command> [args ...]

The command runs in the worktree root with CARGO_TARGET_DIR fixed to the
node's target directory. Its combined output goes to logs/<seq>_<label>.log.
The record holds the command, the extra environment, the exit code, the
libtest summary lines parsed from the output and the sha256 of every golden
file before and after the run. Wall time is diagnostic only.
"""
import datetime
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOGS = HERE / "logs"
GOLDEN = ROOT / "crates/vigilode/tests/golden/default_path"
TARGET = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/v03g"
SUMMARY = re.compile(
    r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out"
)


def golden_hashes():
    if not GOLDEN.is_dir():
        return {}
    return {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(GOLDEN.glob("*.json"))
    }


def now():
    return datetime.datetime.now(datetime.timezone.utc)


def main(argv):
    if "--" not in argv or len(argv) < 3:
        sys.exit(__doc__)
    split = argv.index("--")
    label, extra = argv[0], argv[1:split]
    command = argv[split + 1 :]
    env_extra = dict(item.split("=", 1) for item in extra)
    LOGS.mkdir(parents=True, exist_ok=True)
    journal = LOGS / "commands.jsonl"
    seq = 1
    if journal.exists():
        seq += sum(1 for line in journal.read_text().splitlines() if line.strip())
    log = LOGS / f"{seq:02d}_{label}.log"
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = TARGET
    env.update(env_extra)
    before = golden_hashes()
    started = now()
    with log.open("wb") as sink:
        exit_code = subprocess.run(
            command, cwd=ROOT, env=env, stdout=sink, stderr=subprocess.STDOUT
        ).returncode
    finished = now()
    after = golden_hashes()
    text = log.read_text(errors="replace")
    summaries = [
        {
            "result": m.group(1),
            "passed": int(m.group(2)),
            "failed": int(m.group(3)),
            "ignored": int(m.group(4)),
            "measured": int(m.group(5)),
            "filtered_out": int(m.group(6)),
        }
        for m in SUMMARY.finditer(text)
    ]
    record = {
        "seq": seq,
        "label": label,
        "command": command,
        "env": {"CARGO_TARGET_DIR": TARGET, **env_extra},
        "cwd": str(ROOT),
        "exit_code": exit_code,
        "libtest_summaries": summaries,
        "identical_golden_lines": sorted(
            re.findall(r"^golden (\w+): identical", text, flags=re.M)
        ),
        "blessed_lines": sorted(
            pathlib.Path(p).name for p in re.findall(r"^blessed (\S+) ", text, flags=re.M)
        ),
        "golden_sha256_before": before,
        "golden_sha256_after": after,
        "golden_files_changed": before != after,
        "started_utc": started.isoformat(timespec="seconds"),
        "finished_utc": finished.isoformat(timespec="seconds"),
        "wall_seconds_diagnostic_only": round((finished - started).total_seconds(), 1),
        "log": str(log.relative_to(ROOT)),
    }
    with journal.open("a") as sink:
        sink.write(json.dumps(record, sort_keys=True, allow_nan=False) + "\n")
    print(json.dumps({k: record[k] for k in ("seq", "label", "exit_code", "libtest_summaries", "golden_files_changed", "wall_seconds_diagnostic_only", "log")}))
    return exit_code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
