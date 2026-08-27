#!/usr/bin/env python3
"""Run exactly one K0 cell and atomically preserve its JSON and stderr."""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import tempfile
from typing import Any


def load(path: pathlib.Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: pathlib.Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    except BaseException:
        pathlib.Path(temporary).unlink(missing_ok=True)
        raise


def failure_cell(
    authority: dict[str, Any], family: str, kernel: str, kind: str, message: str
) -> dict[str, Any]:
    return {
        "schema": "vigilode-k0-stage-telemetry-cell/v1",
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
        "execution_state": "ERROR",
        **{
            key: authority[key]
            for key in (
                "repository",
                "source_pull_request",
                "source_base_sha",
                "source_base_tree",
                "execution_head_sha",
                "execution_head_tree",
                "rust_version",
                "cargo_version",
            )
        },
        "case_id": f"{family}-n320",
        "family": family,
        "kernel_arm": kernel,
        "tolerance": authority["tolerance"],
        "solver_configuration_tokens": [],
        "stages": [],
        "receipt": None,
        "error": {"kind": kind, "message": message},
        "limitations": [
            "The atomic runner preserved this execution failure; no missing cell is inferred safe.",
            "This cell remains exploratory and nonauthoritative.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--authority", required=True, type=pathlib.Path)
    parser.add_argument("--family", required=True)
    parser.add_argument("--kernel-arm", required=True)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--stderr-log", required=True, type=pathlib.Path)
    parser.add_argument("--command-record", required=True, type=pathlib.Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command and args.command[0] == "--" else args.command
    authority = load(args.authority)
    if args.family not in authority["families"] or args.kernel_arm not in authority["kernel_arms"]:
        raise SystemExit("runner family/kernel is outside the frozen campaign authority")

    scientific_exit_code = 127
    stderr = ""
    try:
        if not command:
            raise ValueError("missing scientific command after --")
        completed = subprocess.run(command, capture_output=True, text=True, check=False)
        scientific_exit_code = completed.returncode
        stderr = completed.stderr
        if completed.returncode != 0:
            payload = failure_cell(
                authority,
                args.family,
                args.kernel_arm,
                "SCIENTIFIC_COMMAND_ERROR",
                f"scientific command returned {completed.returncode}",
            )
        else:
            try:
                payload = json.loads(completed.stdout)
            except json.JSONDecodeError as error:
                payload = failure_cell(
                    authority,
                    args.family,
                    args.kernel_arm,
                    "SCIENTIFIC_OUTPUT_PARSE_ERROR",
                    str(error),
                )
                scientific_exit_code = 1
            expected_identity = {
                key: authority[key]
                for key in (
                    "repository",
                    "source_pull_request",
                    "source_base_sha",
                    "source_base_tree",
                    "execution_head_sha",
                    "execution_head_tree",
                    "rust_version",
                    "cargo_version",
                )
            }
            observed_identity = {key: payload.get(key) for key in expected_identity}
            if (
                observed_identity != expected_identity
                or payload.get("family") != args.family
                or payload.get("kernel_arm") != args.kernel_arm
            ):
                payload = failure_cell(
                    authority,
                    args.family,
                    args.kernel_arm,
                    "SCIENTIFIC_IDENTITY_MISMATCH",
                    "scientific output identity differs from the frozen campaign command",
                )
                scientific_exit_code = 1
    except Exception as error:
        stderr = f"{type(error).__name__}: {error}\n"
        payload = failure_cell(
            authority,
            args.family,
            args.kernel_arm,
            "ATOMIC_RUNNER_ERROR",
            stderr.strip(),
        )
        scientific_exit_code = 1

    write_atomic(args.output, json.dumps(payload, indent=2, sort_keys=True) + "\n")
    write_atomic(args.stderr_log, stderr)
    record = {
        "schema": "vigilode-k0-atomic-command-record/v1",
        "claim_class": "EXPLORATORY_NONAUTHORITATIVE",
        "family": args.family,
        "kernel_arm": args.kernel_arm,
        "command": command,
        "exit_code": scientific_exit_code,
        "cell_execution_state": payload["execution_state"],
        "output": str(args.output),
        "stderr_log": str(args.stderr_log),
    }
    write_atomic(args.command_record, json.dumps(record, indent=2, sort_keys=True) + "\n")
    print(
        json.dumps(
            {
                "family": args.family,
                "kernel_arm": args.kernel_arm,
                "exit_code": scientific_exit_code,
                "execution_state": payload["execution_state"],
                "stages": len(payload.get("stages", [])),
            },
            sort_keys=True,
        )
    )
    return scientific_exit_code


if __name__ == "__main__":
    raise SystemExit(main())
