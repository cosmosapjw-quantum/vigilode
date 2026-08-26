#!/usr/bin/env python3
"""Run one scientific kernel cell twice and always emit a failure-preserving envelope."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

ENVELOPE_SCHEMA = "vigilode-a1-post-a2a3-kernel-execution-cell-v1"


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def truncate(text: str, limit: int = 20_000) -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + f"\n...[truncated {len(text) - limit} characters]"


def invoke(command: list[str]) -> tuple[int, Any | None, str, str]:
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    payload = None
    parse_error = ""
    if completed.returncode == 0:
        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as error:
            parse_error = f"invalid JSON output: {error}"
    return completed.returncode, payload, completed.stderr, parse_error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--family", required=True)
    parser.add_argument("--kernel-arm", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]

    envelope: dict[str, Any] = {
        "schema": ENVELOPE_SCHEMA,
        "status": "EXPLORATORY/NONAUTHORITATIVE",
        "execution_state": "ERROR",
        "family": args.family,
        "kernel_arm": args.kernel_arm,
        "deterministic_replay": False,
        "payload_sha256_first": None,
        "payload_sha256_second": None,
        "payload": None,
        "errors": [],
    }

    try:
        if not command:
            raise ValueError("missing command after --")
        first_rc, first, first_stderr, first_parse_error = invoke(command)
        second_rc, second, second_stderr, second_parse_error = invoke(command)
        errors: list[str] = []
        if first_rc != 0:
            errors.append(f"first execution returned {first_rc}: {truncate(first_stderr)}")
        if second_rc != 0:
            errors.append(f"second execution returned {second_rc}: {truncate(second_stderr)}")
        if first_parse_error:
            errors.append(f"first {first_parse_error}")
        if second_parse_error:
            errors.append(f"second {second_parse_error}")
        if first is not None:
            envelope["payload"] = first
            envelope["payload_sha256_first"] = digest(first)
        if second is not None:
            envelope["payload_sha256_second"] = digest(second)

        if errors:
            envelope["errors"] = errors
            envelope["execution_state"] = "ERROR"
        elif canonical_bytes(first) != canonical_bytes(second):
            envelope["errors"] = [
                "two complete scientific executions produced different canonical JSON payloads"
            ]
            envelope["execution_state"] = "STOP_INVALID"
        else:
            envelope["deterministic_replay"] = True
            envelope["execution_state"] = "COMPLETE"
    except Exception as error:  # failure preservation is the primary contract here
        envelope["errors"] = [f"runner exception: {type(error).__name__}: {error}"]
        envelope["execution_state"] = "ERROR"

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(envelope, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
