#!/usr/bin/env python3
"""Execute one machine-readable suite, keeping each command's actual receipt.

Default is a dry run. --execute operates in this checkout; use the handoff's
isolated worktree first. This POSIX/WSL runner serializes native work via flock.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import signal
import math
import shutil
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--timeout", type=float, default=1800, help="caller wall-resource cap per command")
    parser.add_argument("--plan", type=Path, default=Path(__file__).with_name("DEVELOPMENT_PLAN.json"))
    args = parser.parse_args()
    plan = json.loads(args.plan.read_text())
    root = Path(__file__).resolve().parents[2]
    if args.suite not in plan["execution_suites"]:
        parser.error("unknown suite; choose " + ", ".join(plan["execution_suites"]))
    output = args.output.resolve()
    target = Path(os.environ.get("CARGO_TARGET_DIR", str(root / "target")))
    if not target.is_absolute(): target = root / target
    valgrind = os.environ.get("VIGILODE_VALGRIND", shutil.which("valgrind") or "valgrind")
    commands = [[arg.replace("{output}",str(output)).replace("{target}",str(target)).replace("{valgrind}",valgrind) for arg in command]
                for command in plan["execution_suites"][args.suite]]
    print(json.dumps({"suite":args.suite,"cwd":str(root),"commands":commands,"execute":args.execute},indent=2),flush=True)
    if not args.execute:
        return
    if not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error("timeout must be finite and positive")
    import fcntl
    output.mkdir(parents=True,exist_ok=False)
    environment = os.environ.copy()
    for key in ("CARGO_BUILD_JOBS","RAYON_NUM_THREADS","OMP_NUM_THREADS","OPENBLAS_NUM_THREADS","RUSTDOCFLAGS"):
        environment.setdefault(key,plan["environment"][key])
    runner = Path(__file__).with_name("record_command.py")
    receipts = []
    current = None
    pending_signal = None

    def forward_signal(signum, _frame):
        nonlocal pending_signal
        pending_signal = signum
        if current is not None:
            try:
                current.send_signal(signum)
            except ProcessLookupError:
                pass

    signal.signal(signal.SIGINT,forward_signal)
    signal.signal(signal.SIGTERM,forward_signal)
    with open(plan["environment"]["native_lock"],"a") as lock:
        while True:
            if pending_signal is not None:
                raise SystemExit(128 + pending_signal)
            try:
                fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                time.sleep(0.1)
        for index, command in enumerate(commands):
            if pending_signal is not None:
                raise SystemExit(128 + pending_signal)
            command_output = output / f"command_{index:02d}"
            recorded = [sys.executable,str(runner),"--output",str(command_output),"--timeout",str(args.timeout),"--input",str(args.plan.resolve()),"--",*command]
            current = subprocess.Popen(recorded,cwd=root,env=environment,start_new_session=True)
            if pending_signal is not None:
                try:
                    current.send_signal(pending_signal)
                except ProcessLookupError:
                    pass
            returncode = current.wait()
            current = None
            receipts.append({"command":command,"exit_code":returncode,"receipt":str(command_output / "RECEIPT.json")})
            (output / "SUITE_RECEIPT.json").write_text(json.dumps({"suite":args.suite,"commands":receipts},indent=2)+"\n")
            if returncode:
                raise SystemExit(returncode)
        if pending_signal is not None:
            raise SystemExit(128 + pending_signal)
    print(json.dumps({"status":"completed","suite":args.suite,"commands":len(commands)}),flush=True)


if __name__ == "__main__":
    main()
