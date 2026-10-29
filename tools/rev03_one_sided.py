#!/usr/bin/env python3
"""One-sided acceptance gate of research node research/rev03_one_sided_acceptance_20261003
(review DAG node REV-03), evaluated from the budgets of tools/rnext01_residual_output.py."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "vigilode-rev03-one-sided-acceptance-v1"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--budgets", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.budgets.read_text())
    validity = available = True
    accepts = unresolved_accepts = 0
    rejects = {"resolved": 0, "unresolved": 0}
    rows = []
    solver_failures = 0
    for row in data["rows"]:
        if "solver_failure" in row:
            solver_failures += 1
            rows.append({"case": row["case"], "solver_failure": row["solver_failure"]})
            continue
        validity &= row["valid"]["U"] and row["valid"]["K"]
        available &= row["finite"]
        for driver in ("U", "K"):
            err = float(row["err_" + driver])
            decision = row["decision"][driver]
            if err <= 1.0:
                accepts += 1
                unresolved_accepts += decision != "resolved-accept"
            else:
                rejects["resolved" if decision == "resolved-reject" else "unresolved"] += 1
        rows.append({"case": row["case"], "err": {"U": row["err_U"], "K": row["err_K"]},
                     "B": {"U": row["B_U"], "K": row["B_K"]}, "decision": row["decision"],
                     "l0038_item3_failed": row["l0038_item3_failed"]})
    gate = {"1_budget_validity": bool(validity),
            "2_one_sided_rule": bool(unresolved_accepts == 0 and accepts >= 10),
            "3_bounds_available": bool(available)}
    report = {"schema": SCHEMA, "rows": rows, "acceptances": accepts, "unresolved_acceptances": unresolved_accepts,
              "rejections": rejects, "solver_failures": solver_failures, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({k: report[k] for k in ("acceptances", "unresolved_acceptances", "rejections",
                                             "solver_failures", "gate", "verdict")}))


if __name__ == "__main__":
    main()
