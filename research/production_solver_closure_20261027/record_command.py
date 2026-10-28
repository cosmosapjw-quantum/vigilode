#!/usr/bin/env python3
"""Record a POSIX command, terminate its whole process group on timeout,
and refuse successful validation if its source/input identity changed.
"""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import signal
import shutil
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--input", type=Path, action="append", default=[])
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("supply a command and finite positive wall-resource timeout")
    if os.name != "posix": parser.error("use POSIX/WSL or an equivalent process-group runner")
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    executable = shutil.which(command[0])

    def snapshot():
        paths = [root / x for x in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
        paths += list((root / "crates").rglob("*.rs"))
        paths += list((root / "crates").rglob("Cargo.toml"))
        paths += list(Path(__file__).parent.rglob("*.py"))
        paths += list((Path(__file__).parent / "configs").glob("*.json"))
        paths += [p.resolve() for p in args.input]
        if executable: paths.append(Path(executable).resolve())
        return {str(p):sha(p) for p in sorted(set(paths))}

    before = snapshot()
    (output / "SOURCE_INPUTS.json").write_text(json.dumps(before, indent=2) + "\n")
    receipt = {
        "schema":"vigilode.recorded-process-group.v1", "command":command, "cwd":str(root),
        "started_utc":dt.datetime.now(dt.timezone.utc).isoformat(), "timeout_seconds":args.timeout,
        "git_head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip(),
        "host":platform.node(), "platform":platform.platform(),
        "environment":{k:os.environ.get(k) for k in (
            "CARGO_HOME","CARGO_TARGET_DIR","CARGO_BUILD_JOBS","CARGO_INCREMENTAL",
            "RUSTFLAGS","RUSTDOCFLAGS","RAYON_NUM_THREADS","OMP_NUM_THREADS",
            "OPENBLAS_NUM_THREADS","MKL_NUM_THREADS","VALGRIND_LIB")},
        "timeout":False,"interrupted_signal":None,"process_group_termination":None,
        "resolved_executable":executable,
    }
    start = time.monotonic()
    pending_signal = None

    def interrupt(signum, _frame):
        nonlocal pending_signal
        pending_signal = signum

    with (output / "stdout.log").open("wb") as stdout, (output / "stderr.log").open("wb") as stderr:
        signal.signal(signal.SIGINT,interrupt)
        signal.signal(signal.SIGTERM,interrupt)
        process = None
        try:
            process = subprocess.Popen(command,cwd=root,stdout=stdout,stderr=stderr,start_new_session=True)
            receipt["process_group_id"] = process.pid
            receipt["status"] = "running"
            (output / "RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
            while pending_signal is None:
                remaining = args.timeout - (time.monotonic() - start)
                if remaining <= 0:
                    receipt["timeout"] = True
                    break
                try:
                    actual_exit = process.wait(timeout=min(remaining,0.1))
                    break
                except subprocess.TimeoutExpired:
                    continue
        except OSError as error:
            receipt["launch_error"] = str(error)
            actual_exit = 127
        if receipt["timeout"] or pending_signal is not None:
            receipt["interrupted_signal"] = pending_signal
            # Preserve the cleanup/receipt transaction under a repeated signal.
            signal.signal(signal.SIGINT,signal.SIG_IGN)
            signal.signal(signal.SIGTERM,signal.SIG_IGN)
            termination = {"sigterm_sent":False,"sigkill_sent":False}
            actual_exit = None
            if process is not None:
                try:
                    os.killpg(process.pid,signal.SIGTERM); termination["sigterm_sent"] = True
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    pass
                # Always address surviving grandchildren even if the direct parent
                # already exited on SIGTERM. Only this newly created group is killed.
                try:
                    os.killpg(process.pid,signal.SIGKILL); termination["sigkill_sent"] = True
                except ProcessLookupError:
                    pass
                actual_exit = process.wait()
            receipt["process_group_termination"] = termination
        elif process is not None:
            # A successful direct child must not leave its still-running group
            # behind after the suite lock is released. Helpers must not detach.
            try:
                os.killpg(process.pid,signal.SIGKILL)
                receipt["remaining_group_cleanup"] = "sigkill_sent"
            except ProcessLookupError:
                receipt["remaining_group_cleanup"] = "group_absent"
    receipt["wall_seconds"] = time.monotonic() - start
    receipt["actual_command_exit_code"] = actual_exit
    after = snapshot()
    receipt["source_unchanged_during_command"] = before == after
    receipt["source_changes"] = sorted(k for k in set(before) | set(after) if before.get(k) != after.get(k))
    wrapper_exit = 124 if receipt["timeout"] else (128 + receipt["interrupted_signal"] if receipt["interrupted_signal"] else actual_exit)
    if wrapper_exit == 0 and before != after:
        wrapper_exit = 2
        receipt["status"] = "stale_source"
    else:
        receipt["status"] = "completed" if wrapper_exit == 0 else "failed"
    receipt["exit_code"] = wrapper_exit
    for name in ("stdout.log","stderr.log","SOURCE_INPUTS.json"):
        receipt[name + "_sha256"] = sha(output / name)
    (output / "RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
    print(json.dumps({k:receipt[k] for k in ("status","exit_code","actual_command_exit_code","wall_seconds","source_unchanged_during_command")}),flush=True)
    raise SystemExit(wrapper_exit)


if __name__ == "__main__":
    main()
