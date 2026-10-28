#!/usr/bin/env python3
"""Gate of research node research/spd05_lgmres_ls_once_20261007 (speed research node SPD05): one
least-squares solve per cycle in the augmented Arnoldi of LGMRES-into.

Reads the raw file written by `crates/rodas5p-integrators/tests/spd05_lgmres_ls_once.rs` (SPD05_OUTPUT)
and writes RESULTS.json with per-item booleans, measured ratios and a final verdict. Plain python3, no
third-party modules. An existing output is never overwritten.

Gate (PREREGISTRATION.md):
1. Identity: the candidate equals the legacy path bitwise on all 1,216 solves (solution bits, residual norm,
   iterations, matvecs, WorkCounters, carried state before and after, rollback on failure), except solves the
   legacy path aborted on an intermediate non-finite least-squares solution; each such solve is listed and
   the candidate must then succeed or fail by the final residual rule. The failure kind of every failed
   legacy solve is recorded (the kinds must account for every failure), so the exception is shown empty
   rather than assumed.
2. Output and rollback: on every failed solve the caller's output is unchanged and both states are rolled
   back (control and candidate arms).
3. Allocations per solve (after each sequence's first solve) <= 0.10 x control on every non-failing
   sequence and strictly fewer on every sequence. "Non-failing sequence" is read as a sequence of the
   regular configuration (not the forced-failure configuration max_outer 1, rtol 1e-14).
Kill: any non-failing sequence above 0.5 x control (a FAIL of item 3 in any case).
Reported, not gated: the candidate with SPD04's least-squares workspace; least-squares solves, Arnoldi
columns and cycles per solve.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

SCHEMA_IN = "vigilode-spd05-lgmres-ls-once-v1"
SCHEMA_OUT = "vigilode-spd05-lgmres-ls-once-results-v1"
EXPECTED_SOLVES = 1216
EXPECTED_FAILURES = 344
RATIO_LIMIT = 0.10
KILL_RATIO = 0.5


def check(raw: dict) -> dict:
    if raw.get("schema") != SCHEMA_IN:
        raise ValueError(f"unexpected schema {raw.get('schema')!r}")
    records = raw["records"]
    solves = sum(int(r["solves"]) for r in records)
    failures = sum(int(r["failures"]) for r in records)

    # 1. Identity.
    kinds_total: dict[str, int] = {}
    exceptions = []
    not_identical = []
    kinds_complete = True
    for r in records:
        for kind, count in r["legacy_failure_kinds"].items():
            kinds_total[kind] = kinds_total.get(kind, 0) + int(count)
        kinds_complete &= sum(int(c) for c in r["legacy_failure_kinds"].values()) == int(r["failures"])
        for e in r["exceptions"]:
            exceptions.append({"set": r["set"], "sequence": r["sequence"], **e})
        if not r["identity"]["candidate_vs_legacy"]:
            not_identical.append({"set": r["set"], "sequence": r["sequence"],
                                  "mismatches": r.get("mismatches", [])})
    exceptions_ok = all(bool(e["candidate_by_final_rule"]) for e in exceptions)
    counts_ok = solves == EXPECTED_SOLVES and failures == EXPECTED_FAILURES
    item1 = {
        "solves": solves,
        "failures": failures,
        "expected_solves": EXPECTED_SOLVES,
        "expected_failures": EXPECTED_FAILURES,
        "counts_as_registered": counts_ok,
        "legacy_failure_kinds": kinds_total,
        "failure_kinds_account_for_every_failure": kinds_complete,
        "exceptions": exceptions,
        "exceptions_by_final_rule": exceptions_ok,
        "sequences_not_identical": not_identical,
        "control_vs_legacy_all": all(r["identity"]["control_vs_legacy"] for r in records),
    }
    item1["pass"] = counts_ok and kinds_complete and exceptions_ok and not not_identical

    # 2. Output and rollback.
    arms = ("control", "candidate")
    item2 = {
        "failures_seen": failures,
        "output_kept_on_failure": all(r["output_kept_on_failure"][a] for r in records for a in arms),
        "rolled_back": all(r["rolled_back"][a] for r in records for a in arms),
    }
    item2["pass"] = failures > 0 and item2["output_kept_on_failure"] and item2["rolled_back"]

    # 3. Allocations.
    rows = []
    item3_ok = True
    killed = []
    for r in records:
        a = r["allocations"]
        control, candidate = int(a["control"]), int(a["candidate"])
        ratio = candidate / control if control else None
        failing = bool(r["failing_configuration"])
        strictly_fewer = candidate < control
        within = True if failing else (ratio is not None and ratio <= RATIO_LIMIT)
        ok = strictly_fewer and within
        if not failing and (ratio is None or ratio > KILL_RATIO):
            killed.append(r["sequence"])
        item3_ok &= ok
        per = r["allocations_per_solve"]
        rows.append({
            "set": r["set"],
            "sequence": r["sequence"],
            "failing_configuration": failing,
            "measured_solves": r["measured_solves"],
            "allocations_per_solve": per,
            "candidate_over_control": ratio,
            "candidate_ls_workspace_over_control": per.get("candidate_ls_workspace_over_control"),
            "strictly_fewer": strictly_fewer,
            "within_limit": within,
            "per_solve": r["per_solve"],
            "reported": r["reported"],
            "pass": ok,
        })
    item3 = {"ratio_limit": RATIO_LIMIT, "kill_ratio": KILL_RATIO, "killed_sequences": killed,
             "sequences": rows, "pass": item3_ok and not killed}

    reported = {
        "candidate_ls_workspace_identity_all": all(
            r["identity"]["candidate_ls_workspace_vs_legacy"] for r in records
        ),
        "candidate_ls_workspace_output_and_rollback": all(
            r["output_kept_on_failure"]["candidate_ls_workspace"]
            and r["rolled_back"]["candidate_ls_workspace"]
            for r in records
        ),
    }
    gate = {
        "1_identity": item1["pass"],
        "2_output_and_rollback": item2["pass"],
        "3_allocations": item3["pass"],
    }
    return {
        "schema": SCHEMA_OUT,
        "node": "research/spd05_lgmres_ls_once_20261007",
        "items": {"1_identity": item1, "2_output_and_rollback": item2, "3_allocations": item3},
        "reported": reported,
        "gate": gate,
        "verdict": "PASS" if all(gate.values()) else "FAIL",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--raw", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        print(f"refusing to overwrite {args.output}", file=sys.stderr)
        return 2
    raw = json.loads(args.raw.read_text())
    results = check(raw)
    results["raw"] = str(args.raw)
    args.output.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"gate": results["gate"], "verdict": results["verdict"]}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
