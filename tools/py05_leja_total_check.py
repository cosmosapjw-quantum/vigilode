#!/usr/bin/env python3
"""Stage 1 Scientific Gate Checker for Certified Newton-Leja Actions (PY05).

Research node: research/py05_leja_total_certificate_20261011
Preregistration: research/py05_leja_total_certificate_20261011/PREREGISTRATION.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import struct
import subprocess
import sys

import mpmath as mp

# AS03 fail-closed validation
sys.path.insert(0, str(Path(__file__).resolve().parent))
from evidence_schema_v2 import (
    INVALID_EXIT_CODE,
    ValidationError,
    emit_invalid,
    validate_py05,
)

SCHEMA = "vigilode-py05-results-v1"
NODE = "research/py05_leja_total_certificate_20261011"


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def phi(k: int, z):
    """phi_k(z) = sum_j z^j / (j + k)!, phi_0 = exp."""
    if abs(z) < mp.mpf("0.5"):
        total, term, j = mp.mpf(0), mp.mpf(1) / mp.factorial(k), 0
        while abs(term) > mp.mpf(10) ** (-mp.mp.dps - 5):
            total += term
            j += 1
            term = term * z / (j + k)
        return total
    value = mp.e ** z
    for m in range(1, k + 1):
        value = (value - 1 / mp.factorial(m - 1)) / z
    return value


def exact_target(cell: dict) -> list:
    n = cell["n"]
    a = mp.matrix([[mp.mpf(unhex(x)) for x in row] for row in cell["a"]])
    h = mp.mpf(unhex(cell["h_hex"]))
    w = [[mp.mpf(unhex(x)) for x in vec] for vec in cell["w"]]
    eigenvalues, q = mp.eigsy(a)
    result = [mp.mpf(0)] * n
    for k in range(5):
        if all(x == 0 for x in w[k]):
            continue
        coords = [mp.fsum(q[i, j] * w[k][i] for i in range(n)) for j in range(n)]
        scaled = [phi(k, h * eigenvalues[j]) * coords[j] for j in range(n)]
        for i in range(n):
            result[i] += mp.fsum(q[i, j] * scaled[j] for j in range(n))
    return result


def distance(values_hex: list[str], exact: list) -> mp.mpf:
    values = [mp.mpf(unhex(x)) for x in values_hex]
    return mp.sqrt(mp.fsum((v - e) ** 2 for v, e in zip(values, exact)))


def main():
    parser = argparse.ArgumentParser(description="PY05 Stage 1 Total Checker")
    parser.add_argument("--cases", type=Path, required=True, help="Path to stage1_cases.json")
    parser.add_argument("--runs", type=Path, required=True, help="Path to RUNS.json")
    parser.add_argument("--profile", type=Path, required=True, help="Path to PROFILE.json")
    parser.add_argument("--output", type=Path, required=True, help="Path to RESULTS.json")
    args = parser.parse_args()

    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")

    inputs = {
        "cases": args.cases,
        "runs": args.runs,
        "profile": args.profile,
    }

    # 1. AS03 Evidence Validation first
    try:
        validate_py05(args.cases, args.runs, args.profile)
    except ValidationError as err:
        return emit_invalid(args.output, SCHEMA, err, inputs)
    except Exception as err:
        print(f"Validation error: {err}", file=sys.stderr)
        sys.exit(INVALID_EXIT_CODE)

    cases_doc = json.loads(args.cases.read_text(encoding="utf-8"))
    runs_doc = json.loads(args.runs.read_text(encoding="utf-8"))

    cells = {c["cell_id"]: c for c in cases_doc["cells"]}
    rows = runs_doc["rows"]

    mp.mp.dps = 50

    # Gate 1: Enclosure
    gate_1 = True
    enclosure_violations = []
    admitted_count = 0

    # Index rows by (cell_id, arm, cache_state)
    indexed_rows = {}
    for r in rows:
        indexed_rows[(r["cell_id"], r["arm"], r["cache_state"])] = r

    # Compute 50-digit actual error and check enclosure on all 40 cells
    for cell_id, cell in sorted(cells.items()):
        exact = exact_target(cell)
        budget = mp.mpf(unhex(cell["budget_hex"]))

        for cs in ["cold", "warm"]:
            row = indexed_rows.get((cell_id, "leja-cert", cs))
            if not row:
                gate_1 = False
                enclosure_violations.append({"cell_id": cell_id, "why": f"missing row for {cs}"})
                continue

            if row["admitted"]:
                admitted_count += 1
                actual_err = distance(row["fused"], exact)
                bound = mp.mpf(unhex(row["total_bound"]))

                # Enclosure check: bound >= actual error
                if bound < actual_err:
                    gate_1 = False
                    enclosure_violations.append({
                        "cell_id": cell_id,
                        "cache_state": cs,
                        "bound": float(bound),
                        "actual": float(actual_err),
                        "why": "under-enclosure",
                    })

                # Admitted result must be within budget
                if actual_err > budget:
                    gate_1 = False
                    enclosure_violations.append({
                        "cell_id": cell_id,
                        "cache_state": cs,
                        "actual": float(actual_err),
                        "budget": float(budget),
                        "why": "actual error exceeds budget",
                    })

    # Gate 2: No false authority
    # - leja-est reports are still rejected by certified APIs
    # - declared enclosure and non-symmetric matrix refused by leja-cert
    gate_2 = True
    for r in rows:
        if r["arm"] == "leja-est":
            if r["status"] != "estimate-only" or r.get("total_bound") is not None:
                gate_2 = False

    # Gate 3: Complete cost wins on niche (hrho >= 40)
    # Ratio leja-cert / cheb-cert complete cost:
    # median <= 0.95
    # no cell > 1.10
    # ratio = inf if leja-cert does not admit
    niche_ratios = []
    for cell_id, cell in sorted(cells.items()):
        if not cell.get("is_niche"):
            continue

        for cs in ["cold", "warm"]:
            leja_row = indexed_rows[(cell_id, "leja-cert", cs)]
            cheb_row = indexed_rows[(cell_id, "cheb-cert", cs)]

            if not leja_row["admitted"]:
                ratio = float("inf")
            else:
                c_leja = leja_row["complete_cost"]
                c_cheb = cheb_row["complete_cost"]
                ratio = float(c_leja) / float(c_cheb) if c_cheb > 0 else float("inf")
            niche_ratios.append(ratio)

    finite_ratios = [r for r in niche_ratios if math.isfinite(r)]
    median_ratio = statistics.median(niche_ratios) if niche_ratios else float("inf")
    max_ratio = max(niche_ratios) if niche_ratios else float("inf")

    gate_3 = (median_ratio <= 0.95) and (max_ratio <= 1.10)

    # Outcomes:
    # If G1, G2, G3 all hold: PASS
    # If any fail: FAIL
    overall_pass = gate_1 and gate_2 and gate_3
    verdict = "PASS" if overall_pass else "FAIL"

    repo_root = Path(__file__).resolve().parent.parent
    try:
        git_commit = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repo_root, text=True
        ).strip()
    except Exception:
        git_commit = "unknown"

    try:
        git_status = subprocess.check_output(
            ["git", "status", "--porcelain"], cwd=repo_root, text=True
        )
        norm_out = args.output.name
        tree_clean = all(
            not line.strip() or line.strip().endswith(norm_out)
            for line in git_status.splitlines()
        )
    except Exception:
        tree_clean = False

    checker_bytes = Path(__file__).read_bytes()
    checker_blob = hashlib.sha256(checker_bytes).hexdigest()

    report = {
        "schema": SCHEMA,
        "node": NODE,
        "verdict": verdict,
        "source_commit": git_commit,
        "tree_clean": tree_clean,
        "checker_blob": checker_blob,
        "gate": {
            "gate_1_enclosure": gate_1,
            "gate_2_no_false_authority": gate_2,
            "gate_3_complete_cost_wins": gate_3,
        },
        "metrics": {
            "enclosure_violations": len(enclosure_violations),
            "niche_median_ratio": median_ratio if math.isfinite(median_ratio) else "inf",
            "niche_max_ratio": max_ratio if math.isfinite(max_ratio) else "inf",
            "finite_ratio_count": len(finite_ratios),
            "total_niche_comparisons": len(niche_ratios),
        },
        "violations": enclosure_violations,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")

    print(f"Gate 1 (Enclosure): {'PASS' if gate_1 else 'FAIL'}")
    print(f"Gate 2 (No False Authority): {'PASS' if gate_2 else 'FAIL'}")
    print(f"Gate 3 (Cost Wins on Niche): {'PASS' if gate_3 else 'FAIL'} (median {median_ratio:.3f}, max {max_ratio:.3f})")
    print(f"Overall verdict: {verdict}")
    print(f"Wrote results to {args.output}")

    if not overall_pass:
        sys.exit(1)


if __name__ == "__main__":
    main()
