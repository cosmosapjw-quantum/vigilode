#!/usr/bin/env python3
"""Validate source/artifact bindings, not mathematical truth or runtime speed."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
UNIT = Path(__file__).resolve().parent


def main():
    errors = []
    required = ["REPORT_KO.md", "HANDOFF_CODEX_KO.md", "SOURCE_IDENTITY.json", "DECISION.json",
                "LOCAL_EXECUTION.json", "RESULTS.json", "VALIDATION.json", "RUN_RECORD.json",
                "REVIEW.json", "REVIEW.md", "PACKET.md", "LEJA.md", "DENSE.md",
                "STRUCTURED.md", "PARALLEL_THEORY.md", "run_local.py"]
    for name in required:
        if not (UNIT / name).is_file():
            errors.append("missing: " + name)
    identity = json.loads((UNIT / "SOURCE_IDENTITY.json").read_text())
    for filename, expected in identity["source_sha256"].items():
        p = ROOT / filename
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest() != expected:
            errors.append("source mismatch: " + filename)
    if hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest() != identity["cargo_lock_sha256"]:
        errors.append("Cargo.lock changed")
    decision = json.loads((UNIT / "DECISION.json").read_text())
    if decision["measured_speed_promoted"] or decision["allocation_measurement_valid_here"]:
        errors.append("unsupported measured authority")
    validation = json.loads((UNIT / "VALIDATION.json").read_text())
    if not validation["runner_check"]["nonallocation_commands_success"]:
        errors.append("nonallocation execution unresolved")
    for p in UNIT.rglob("*.json"):
        # This process's redirected receipt is incomplete until stdout closes.
        if p == UNIT / "evidence/package_check.json":
            continue
        try:
            json.loads(p.read_text())
        except (ValueError, UnicodeError) as error:
            errors.append(str(p.relative_to(UNIT)) + ": " + str(error))
    result = {"schema_version": 1, "validation_scope": "artifact_structure_and_source_binding",
              "source_files": len(identity["source_sha256"]), "required_files": len(required),
              "passed": not errors, "errors": errors}
    print(json.dumps(result, indent=2))
    return bool(errors)


if __name__ == "__main__":
    raise SystemExit(main())
