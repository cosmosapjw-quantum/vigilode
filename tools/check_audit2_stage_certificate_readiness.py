#!/usr/bin/env python3
"""Candidate-free readiness checks for the synthetic stage certificate.

This deliberately does not call the historic authority feature, a solver
example, a real client, or a holdout surface.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "crates" / "rodas5p-integrators"
SOURCE = PACKAGE / "src" / "audit2_stage_certificate_research.rs"
TEST = PACKAGE / "tests" / "audit2_stage_certificate_contracts.rs"


def _outside_repository(path: Path) -> Path:
    resolved = path.resolve()
    try:
        resolved.relative_to(ROOT)
    except ValueError:
        return resolved
    raise ValueError(f"build state must be outside the repository: {resolved}")


def _static_contract() -> dict[str, object]:
    manifest = tomllib.loads((PACKAGE / "Cargo.toml").read_text(encoding="utf-8"))
    features = manifest.get("features", {})
    stage = features.get("audit2-stage-certificate")
    if not isinstance(stage, list):
        raise ValueError("missing audit2-stage-certificate feature")
    required = {"audit2-research", "dep:serde_json", "dep:sha2"}
    if not required.issubset(stage):
        raise ValueError("stage feature is missing a required dependency")
    if "audit2-bateman-authority" in stage:
        raise ValueError("stage feature activates forbidden authority feature")
    if features.get("default") != []:
        raise ValueError("default feature set changed")

    forbidden_source_tokens = (
        "audit2_bateman",
        "holdout",
        "Command::new",
        "std::process",
        "include_bytes!",
    )
    scanned = {}
    for path in (SOURCE, TEST):
        text = path.read_text(encoding="utf-8")
        present = [token for token in forbidden_source_tokens if token in text]
        if present:
            raise ValueError(f"forbidden execution surface in {path.name}: {present}")
        scanned[path.relative_to(ROOT).as_posix()] = path.stat().st_size

    return {
        "feature_dependencies": stage,
        "scanned_source_bytes": scanned,
        "status": "CANDIDATE_FREE_STAGE_SURFACE_VALID",
    }


def _run(argv: list[str], env: dict[str, str]) -> None:
    print(json.dumps({"argv": argv}, separators=(",", ":")), flush=True)
    subprocess.run(argv, cwd=ROOT, env=env, check=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target-dir", required=True, type=Path)
    parser.add_argument("--tmp-dir", required=True, type=Path)
    args = parser.parse_args()

    try:
        target = _outside_repository(args.target_dir)
        temporary = _outside_repository(args.tmp_dir)
        target.mkdir(parents=True, exist_ok=True)
        temporary.mkdir(parents=True, exist_ok=True)
        static = _static_contract()
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"STAGE_READINESS_INVALID: {error}", file=sys.stderr)
        return 1

    env = os.environ.copy()
    env.update(
        {
            "CARGO_TARGET_DIR": str(target),
            "TMPDIR": str(temporary),
            "PYTHONDONTWRITEBYTECODE": "1",
        }
    )
    commands = [
        [sys.executable, "tools/validate_audit2_stage_certificate_handoff.py"],
        [sys.executable, "tools/test_audit2_stage_certificate_handoff.py", "-v"],
        [
            "cargo", "test", "--locked", "-p", "rodas5p-fair-ab",
            "--test", "global_error_contracts", "--test", "output_accuracy_assessment_contracts",
        ],
        [
            "cargo", "test", "--locked", "-p", "rodas5p-integrators",
            "--features", "audit2-research",
            "--test", "audit2_structured_correction_contracts",
            "--test", "audit2_matrix_free_common_w_contracts",
            "--test", "audit2_reusable_preconditioner_transaction_contracts",
            "--test", "dense_output_v2_contracts",
            "--test", "homotopy_numerical_contracts",
        ],
        [
            "cargo", "test", "--locked", "-p", "rodas5p-integrators",
            "--features", "audit2-stage-certificate",
            "--test", "audit2_stage_certificate_contracts",
        ],
        ["cargo", "check", "--locked", "-p", "rodas5p-integrators"],
        ["cargo", "check", "--locked", "-p", "rodas5p-integrators", "--no-default-features"],
        [
            "cargo", "check", "--locked", "-p", "rodas5p-integrators",
            "--features", "audit2-stage-certificate",
        ],
    ]
    try:
        for command in commands:
            _run(command, env)
    except subprocess.CalledProcessError as error:
        print(
            json.dumps(
                {"argv": error.cmd, "exit_code": error.returncode, "status": "STAGE_READINESS_FAILED"},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return error.returncode or 1

    print(
        json.dumps(
            {
                "candidate_executions": 0,
                "commands_passed": len(commands),
                "holdout_access": "NOT_OPENED_OR_EXECUTED",
                "static": static,
                "status": "AUDIT2_STAGE_CERTIFICATE_READINESS_PASS",
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
