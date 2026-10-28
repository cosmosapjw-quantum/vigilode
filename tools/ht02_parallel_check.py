#!/usr/bin/env python3
"""Checker and report generator for research node HT02 (Windowed Homotopy Parallel Pilot).

Supports:
- Branch A (--abstain-only): Invoked when HT01 decided ABSTAIN. Reads HT01 RESULTS.json,
  verifies the decision, and writes RESULTS.json with decision ABSTAIN_NOT_RUN and
  verdict INCONCLUSIVE without executing solver runs.
- Branch B: Evaluates parallel homotopy runs from RUNS.json when a client was selected.

Usage:
    python3 tools/ht02_parallel_check.py --ht01 research/ht01_homotopy_client_contract_20261011/RESULTS.json --abstain-only --output research/ht02_homotopy_parallel_pilot_20261011/RESULTS.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

try:
    import evidence_schema_v2 as evidence
except ImportError:
    evidence = None

SCHEMA_HT01 = "vigilode-ht01-results-v1"
SCHEMA_HT02 = "vigilode-ht02-results-v1"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def emit_invalid(reason: str) -> int:
    print(f"INVALID: {reason}", file=sys.stderr)
    return 2


def emit_fail(reason: str) -> int:
    print(f"FAIL: {reason}", file=sys.stderr)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ht01", required=True, type=Path, help="Path to HT01 RESULTS.json")
    parser.add_argument("--abstain-only", action="store_true", help="Execute Branch A (HT01 ABSTAIN)")
    parser.add_argument("--runs", type=Path, help="Path to HT02 RUNS.json (Branch B)")
    parser.add_argument("--output", required=True, type=Path, help="Path to HT02 RESULTS.json output")
    args = parser.parse_args()

    if not args.ht01.exists():
        return emit_invalid(f"HT01 results file not found: {args.ht01}")

    raw_bytes = args.ht01.read_bytes()

    # Strict JSON parse
    if evidence is not None and hasattr(evidence, "loads_strict"):
        try:
            ht01_data = evidence.loads_strict(raw_bytes)
        except Exception as exc:
            return emit_invalid(f"HT01 RESULTS.json failed strict parsing: {exc}")
    else:
        try:
            ht01_data = json.loads(raw_bytes.decode("utf-8"))
        except Exception as exc:
            return emit_invalid(f"HT01 RESULTS.json invalid JSON: {exc}")

    if ht01_data.get("schema") != SCHEMA_HT01:
        return emit_invalid(f"Unexpected HT01 schema: {ht01_data.get('schema')}, expected {SCHEMA_HT01}")

    ht01_sha256 = sha256_file(args.ht01)
    ht01_decision = ht01_data.get("decision")
    ht01_verdict = ht01_data.get("verdict")

    if args.abstain_only:
        # Enforce Branch A prerequisites
        if ht01_decision != "ABSTAIN":
            return emit_invalid(f"--abstain-only specified, but HT01 decision is '{ht01_decision}' (expected 'ABSTAIN')")
        if ht01_verdict != "PASS":
            return emit_fail(f"HT01 verdict is '{ht01_verdict}', expected 'PASS'")

        # Extract candidate failing criteria from HT01
        failing_criteria: dict[str, list[str]] = {}
        for row in ht01_data.get("rows", []):
            cid = row.get("id")
            if cid:
                failing_criteria[cid] = row.get("failing_criteria", [])

        results = {
            "schema": SCHEMA_HT02,
            "node": "HT02",
            "branch": "Branch A: HT01 decided ABSTAIN",
            "decision": "ABSTAIN_NOT_RUN",
            "verdict": "INCONCLUSIVE",
            "ht01_results_sha256": ht01_sha256,
            "ht01_decision": ht01_decision,
            "candidate_failing_criteria": failing_criteria,
            "runs": 0,
            "wall_time_speedup": None,
            "claim": (
                "INCONCLUSIVE: HT01 decided ABSTAIN (no candidate met expensive client contract preconditions); "
                "Branch A executed; nothing built or run; no evidence about homotopy parallelism."
            ),
        }

        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
        print(f"PASS: Branch A executed (verdict: INCONCLUSIVE, decision: ABSTAIN_NOT_RUN; wrote {args.output})")
        return 0

    # Branch B (when HT01 selected a client)
    return emit_fail("Branch B requires HT01 SELECTED(client) and committed PREREGISTRATION_HT02B.md")


if __name__ == "__main__":
    sys.exit(main())
