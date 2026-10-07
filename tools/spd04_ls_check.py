#!/usr/bin/env python3
"""Gate of research node research/spd04_ls_workspace_20261007 (speed research node SPD04):
allocation-free small least squares for the Krylov `into` paths.

Reads the raw file written by
`crates/rodas5p-integrators/tests/spd04_ls_workspace_study.rs` (SPD04_OUTPUT) and writes RESULTS.json
with per-item booleans, measured ratios and a final verdict. Plain python3, no third-party modules.
An existing output is never overwritten.

Gate (PREREGISTRATION.md):
1. Contract: `solve_into` equals `small::least_squares` bit for bit (solution or error text) on every
   contract system (>= 1,000 seeded Hessenberg systems, m = 1..64) and reuse sequence (the raw file carries
   the harness's replication of the contract test's systems; the registered cargo test is run separately).
2. Identity: on all 408 solves of the 56 rnext02 families the candidate equals the legacy solve and the
   control bitwise; on the six L-0038 driver runs the candidate equals the control bitwise.
3. Exact allocation removal: per family, candidate = control - 11 x (least-squares solves), except
   allocations of workspace growth. Checked per solve: every solve without a reported growth (control
   workspace, candidate workspace or least-squares workspace) must satisfy the equality exactly; solves
   with growth are listed per family.
4. Driver: allocations per attempt <= 0.10 x control on the five non-HIRES L-0038 problems; HIRES reported.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

SCHEMA_IN = "vigilode-spd04-ls-workspace-v1"
SCHEMA_OUT = "vigilode-spd04-ls-workspace-results-v1"
EXPECTED_FAMILIES = 56
EXPECTED_SOLVES = 408
EXPECTED_PROBLEMS = {
    "robertson",
    "van-der-pol-mu1000",
    "hires",
    "brusselator-1d-50",
    "prothero-robinson-forced",
    "quadratic-4",
}
REPORTED_ONLY_PROBLEM = "hires"
DRIVER_RATIO_LIMIT = 0.10
MIN_CONTRACT_SYSTEMS = 1000


def check(raw: dict) -> dict:
    if raw.get("schema") != SCHEMA_IN:
        raise ValueError(f"unexpected schema {raw.get('schema')!r}")
    per_ls = int(raw["allocations_per_least_squares"])

    # 1. Contract.
    contract = raw["contract"]
    item1 = {
        "systems": contract["systems"],
        "reuse_sequences": contract["reuse_sequences"],
        "reuse_solves": contract["reuse_solves"],
        "nonfinite_inputs": contract["nonfinite_inputs"],
        "identical": bool(contract["identical"]),
        "growth_only_above_previous_maximum": bool(contract["growth_only_above_previous_maximum"]),
        "first_mismatch": contract.get("first_mismatch"),
    }
    item1["pass"] = (
        item1["identical"]
        and contract["systems"] >= MIN_CONTRACT_SYSTEMS
        and contract["reuse_sequences"] >= 1
        and contract["nonfinite_inputs"] >= 1
    )

    # 2. Identity.
    families = raw["families"]
    counts = raw["counts"]
    total_solves = sum(int(f["solves"]) for f in families)
    failing_families = [
        f["family"]
        for f in families
        if not (f["identity"]["candidate_vs_legacy"] and f["identity"]["candidate_vs_control"])
    ]
    runs = raw["driver_runs"]
    problems = {r["problem"] for r in runs}
    failing_runs = [r["problem"] for r in runs if not r["identical"]]
    counts_ok = (
        len(families) == EXPECTED_FAMILIES
        and total_solves == EXPECTED_SOLVES
        and problems == EXPECTED_PROBLEMS
        and len(runs) == len(EXPECTED_PROBLEMS)
    )
    item2 = {
        "families": len(families),
        "solves": total_solves,
        "expected_families": EXPECTED_FAMILIES,
        "expected_solves": EXPECTED_SOLVES,
        "driver_problems": sorted(problems),
        "counts_as_registered": counts_ok,
        "failures": counts.get("failures"),
        "excluded_states": counts.get("excluded_states"),
        "families_not_identical": failing_families,
        "driver_runs_not_identical": failing_runs,
        "control_vs_legacy_all": all(f["identity"]["control_vs_legacy"] for f in families),
    }
    item2["pass"] = counts_ok and not failing_families and not failing_runs

    # 3. Exact allocation removal.
    family_rows = []
    item3_ok = True
    for f in families:
        exact_solves, growth_solves, violations = 0, [], []
        for s in f["per_solve"]:
            grew = any(bool(v) for v in s["growth"].values())
            a = s["allocations"]
            expected = a["control"] - per_ls * int(s["least_squares_solves"])
            if grew:
                growth_solves.append(
                    {"index": s["index"], "growth": s["growth"], "control": a["control"],
                     "candidate": a["candidate"], "control_minus_11_per_least_squares": expected}
                )
            elif a["candidate"] == expected:
                exact_solves += 1
            else:
                violations.append(
                    {"index": s["index"], "control": a["control"], "candidate": a["candidate"],
                     "expected": expected}
                )
        ok = not violations
        item3_ok &= ok
        control = f["allocations"]["control"]
        family_rows.append({
            "family": f["family"],
            "least_squares_solves": f["least_squares_solves"]["candidate_workspace"],
            "allocations": f["allocations"],
            "allocations_per_solve": f["allocations_per_solve"],
            "candidate_over_control": (f["allocations"]["candidate"] / control) if control else None,
            "family_exact_removal": bool(f["exact_removal"]),
            "exact_solves": exact_solves,
            "growth_solves": growth_solves,
            "violations": violations,
            "pass": ok,
        })
    item3 = {
        "allocations_per_least_squares": per_ls,
        "families_with_growth": [r["family"] for r in family_rows if r["growth_solves"]],
        "families_failing": [r["family"] for r in family_rows if not r["pass"]],
        "pass": item3_ok,
    }

    # 4. Driver allocations.
    driver_rows = []
    item4_ok = True
    for r in runs:
        ratio = r["allocations_per_attempt"]["ratio"]
        gated = r["problem"] != REPORTED_ONLY_PROBLEM
        ok = (ratio <= DRIVER_RATIO_LIMIT) if gated else True
        item4_ok &= ok
        driver_rows.append({
            "problem": r["problem"],
            "gated": gated,
            "attempts": r["attempts"],
            "allocations_per_attempt": r["allocations_per_attempt"],
            "ratio": ratio,
            "pass": ok,
        })
    item4 = {"ratio_limit": DRIVER_RATIO_LIMIT, "reported_only": REPORTED_ONLY_PROBLEM,
             "runs": driver_rows, "pass": item4_ok}

    gate = {
        "1_contract": item1["pass"],
        "2_identity": item2["pass"],
        "3_exact_allocation_removal": item3["pass"],
        "4_driver_allocations": item4["pass"],
    }
    return {
        "schema": SCHEMA_OUT,
        "node": "research/spd04_ls_workspace_20261007",
        "items": {
            "1_contract": item1,
            "2_identity": item2,
            "3_exact_allocation_removal": item3,
            "4_driver_allocations": item4,
        },
        "families": family_rows,
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
