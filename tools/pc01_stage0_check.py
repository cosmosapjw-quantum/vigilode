#!/usr/bin/env python3
"""Stage 0 Feasibility and Verification Checker for PC01 (Linear-part right preconditioner).

Research node: research/pc01_right_preconditioner_20261011
Preregistration: research/pc01_right_preconditioner_20261011/PREREGISTRATION.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from evidence_schema_v2 import (
    emit_invalid,
    validate_pc01_stage0,
    PC01_NODE,
    PC01_STAGE0_SCHEMA,
    ValidationError,
)

REPO_ROOT = Path(__file__).resolve().parent.parent


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description="PC01 Stage 0 Checker")
    parser.add_argument("--contracts", type=Path, required=True, help="Path to STAGE0_CONTRACTS.json")
    parser.add_argument("--output", type=Path, required=True, help="Path to STAGE0.json")
    args = parser.parse_args()

    # Step 1: Validate contracts via evidence_schema_v2
    try:
        evidence = validate_pc01_stage0(args.contracts)
    except ValidationError as err:
        sys.exit(emit_invalid(args.output, PC01_STAGE0_SCHEMA, err, {"contracts": args.contracts}))

    contracts_doc = evidence.docs["contracts"]
    prereqs = contracts_doc["prerequisites"]

    # Step 2: Evaluate F1, F2, F3, F4, R
    required_keys = ("F1", "F2", "F3", "F4", "R")
    for k in required_keys:
        if k not in prereqs:
            print(f"FAIL: missing prerequisite {k} in contracts", file=sys.stderr)
            sys.exit(1)

    evaluated_prereqs = {}
    for k in required_keys:
        item = prereqs[k]
        met = bool(item.get("passed", False))
        test = item.get("test", "")
        desc = item.get("description", "")
        details = item.get("details", {})
        evidence_str = f"{test}: {desc}" if test else desc
        evaluated_prereqs[k] = {
            "met": met,
            "evidence": evidence_str,
            "details": details,
        }

    # Step 3: Determine decision and verdict per PREREGISTRATION lines 77-87
    all_met = all(evaluated_prereqs[k]["met"] for k in required_keys)
    if all_met:
        decision = "FEASIBLE"
        verdict = "PASS"
        claim = "decision FEASIBLE"
    else:
        # Determine specific HOLD reason
        if not evaluated_prereqs["F1"]["met"]:
            hold_reason = "HOLD(no sparse kernel)"
        elif not evaluated_prereqs["F2"]["met"]:
            hold_reason = "HOLD(residual contract)"
        elif not evaluated_prereqs["F3"]["met"]:
            hold_reason = "HOLD(linear part)"
        elif not evaluated_prereqs["F4"]["met"]:
            hold_reason = "HOLD(problem)"
        elif not evaluated_prereqs["R"]["met"]:
            hold_reason = "HOLD(reference)"
        else:
            hold_reason = "HOLD"
        decision = hold_reason
        verdict = "PASS"
        claim = f"decision {decision}"

    # Build Stage 0 numeric report
    gated_cells_count = int(evaluated_prereqs["R"]["details"].get("gated_cells_count", 6))
    reference_limited_count = int(evaluated_prereqs["R"]["details"].get("reference_limited_count", 6))
    ref_limited_fraction = float(reference_limited_count) / float(gated_cells_count) if gated_cells_count > 0 else 1.0

    report = {
        "schema": PC01_STAGE0_SCHEMA,
        "node": PC01_NODE,
        "verdict": verdict,
        "decision": decision,
        "claim": claim,
        "prerequisites": evaluated_prereqs,
        "numeric_summary": {
            "prerequisites_total": len(required_keys),
            "prerequisites_met": sum(1 for k in required_keys if evaluated_prereqs[k]["met"]),
            "prerequisites_unmet": sum(1 for k in required_keys if not evaluated_prereqs[k]["met"]),
            "gated_cells_count": gated_cells_count,
            "reference_limited_count": reference_limited_count,
            "reference_limited_fraction": ref_limited_fraction,
            "reference_limited_percent": ref_limited_fraction * 100.0,
        },
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, sort_keys=True)
        f.write("\n")

    print(f"PC01 Stage 0 complete: verdict={verdict}, decision={decision}")
    print(f"Output written to {args.output}")


if __name__ == "__main__":
    main()
