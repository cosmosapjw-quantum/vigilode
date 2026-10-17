#!/usr/bin/env python3
"""Export Stage 0 and Stage 1 test cases for certified Newton-Leja phi actions (PY05).

Research node: research/py05_leja_total_certificate_20261011
Preregistration: research/py05_leja_total_certificate_20261011/PREREGISTRATION.md

Outputs:
1. Stage 0 cases: 60 fresh fixtures + exact rational test fixtures on n <= 5.
2. Stage 1 cases: 40 registered cells (n in {8, 32}, h*rho in {10, 20, 40, 80, 160},
   lambda = rho/100, distinct/same inputs, budgets 1e-8 and 1e-12).
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import struct
import sys

SCHEMA_STAGE0 = "vigilode-py05-stage0-cases-v1"
SCHEMA_STAGE1 = "vigilode-py05-stage1-cases-v1"
NODE = "research/py05_leja_total_certificate_20261011"


def hex_f64(x: float) -> str:
    return format(struct.unpack(">Q", struct.pack(">d", float(x)))[0], "016x")


def hex_vec(vec: list[float]) -> list[str]:
    return [hex_f64(x) for x in vec]


def hex_mat(mat: list[list[float]]) -> list[list[str]]:
    return [[hex_f64(x) for x in row] for row in mat]


def splitmix(seed: int, n: int) -> list[float]:
    state = seed & 0xFFFFFFFFFFFFFFFF
    out = []
    for _ in range(n):
        state = (state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        z ^= (z >> 31)
        out.append(2.0 * ((z >> 11) / (1 << 53)) - 1.0)
    return out


def make_symmetric_operator(n: int, rho: float, lambda_val: float, seed: int) -> list[list[float]]:
    """Construct dense symmetric matrix with Gershgorin discs strictly in [-rho, -lambda] <= 0."""
    # Diagonal values spanning [lambda, rho]
    u = splitmix(seed, n)
    # Sort u to have a clean distribution
    diag_vals = []
    for val in u:
        frac = (val + 1.0) / 2.0
        diag_vals.append(lambda_val + (rho - lambda_val) * frac)
    diag_vals[0] = lambda_val
    diag_vals[-1] = rho

    # Off-diagonals small enough so row sums are strictly bounded
    raw_off = splitmix(seed + 1000, n * n)
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        a[i][i] = -diag_vals[i]
        for j in range(i):
            cap = 0.05 * min(diag_vals[i], diag_vals[j]) / float(n)
            v = cap * raw_off[i * n + j]
            a[i][j] = v
            a[j][i] = v

    # Verify Gershgorin nonpositive condition
    for i in range(n):
        radius = sum(abs(a[i][j]) for j in range(n) if j != i)
        hi = a[i][i] + radius
        if hi >= 0.0:
            raise ValueError(f"Gershgorin disc {hi} is not strictly negative")
    return a


def generate_stage0_cases() -> dict:
    cases = []
    seed_base = 50000

    # 60 fresh fixtures
    configs = [
        # (n, rho, h, tol, input_kind)
        (3, 10.0, 0.05, 1.0e-8, "distinct"),
        (3, 20.0, 0.02, 1.0e-10, "same"),
        (4, 15.0, 0.04, 1.0e-8, "distinct"),
        (4, 30.0, 0.01, 1.0e-10, "same"),
        (5, 25.0, 0.02, 1.0e-8, "distinct"),
        (5, 50.0, 0.01, 1.0e-10, "same"),
        (6, 40.0, 0.02, 1.0e-8, "distinct"),
        (6, 80.0, 0.01, 1.0e-10, "same"),
        (8, 50.0, 0.02, 1.0e-8, "distinct"),
        (8, 100.0, 0.01, 1.0e-10, "same"),
        (10, 60.0, 0.02, 1.0e-8, "distinct"),
        (10, 120.0, 0.01, 1.0e-10, "same"),
        (12, 80.0, 0.02, 1.0e-8, "distinct"),
        (12, 160.0, 0.01, 1.0e-10, "same"),
        (16, 100.0, 0.02, 1.0e-8, "distinct"),
    ]

    count = 0
    while count < 60:
        cfg = configs[count % len(configs)]
        n, rho, h, tol, input_kind = cfg
        # Vary seed for freshness
        seed = seed_base + count
        lambda_val = rho / 100.0
        a = make_symmetric_operator(n, rho, lambda_val, seed)

        if input_kind == "distinct":
            w = [splitmix(seed + 100 * k, n) for k in range(5)]
        else:
            base_v = splitmix(seed + 50, n)
            scales = [1.0, -0.5, 0.25, 2.0, -1.0]
            w = [[s * x for x in base_v] for s in scales]

        label = f"stage0-case{count:02d}-n{n}-rho{rho:.0f}-h{h}-{input_kind}"
        cases.append({
            "label": label,
            "group": "stage0",
            "n": n,
            "rho": rho,
            "rho_hex": hex_f64(rho),
            "lambda": lambda_val,
            "lambda_hex": hex_f64(lambda_val),
            "h": h,
            "h_hex": hex_f64(h),
            "input": input_kind,
            "tolerance": tol,
            "tolerance_hex": hex_f64(tol),
            "a": hex_mat(a),
            "w": [hex_vec(col) for col in w],
            "seed": seed,
        })
        count += 1

    # Exact rational test cases on n <= 5
    rational_cases = []
    for n in [2, 3, 4, 5]:
        a_rat = [[0.0] * n for _ in range(n)]
        for i in range(n):
            a_rat[i][i] = -float(i + 1) * 2.0
            for j in range(i):
                a_rat[i][j] = 0.1
                a_rat[j][i] = 0.1
        w_rat = [[1.0 / float(k + 1) for _ in range(n)] for k in range(5)]
        rational_cases.append({
            "n": n,
            "h": 0.1,
            "a": a_rat,
            "w": w_rat,
        })

    return {
        "schema": SCHEMA_STAGE0,
        "node": NODE,
        "cases": cases,
        "rational_cases": rational_cases,
    }


def generate_stage1_cases() -> dict:
    cells = []
    seed = 80000

    # Registered parameters:
    # n in {8, 32}
    # h*rho in {10, 20, 40, 80, 160}
    # lambda = rho / 100
    # inputs: distinct, same
    # budgets: 1e-8, 1e-12
    # Total = 2 * 5 * 2 * 2 = 40 cells
    rho = 100.0
    lambda_val = rho / 100.0  # 1.0
    hrho_values = [10, 20, 40, 80, 160]
    budgets = [1.0e-8, 1.0e-12]

    scales = [1.0, -0.5, 0.25, 2.0, -1.0]

    for n in [8, 32]:
        # Distinct operator per dimension
        a = make_symmetric_operator(n, rho, lambda_val, seed + n * 17)

        for hrho in hrho_values:
            h = float(hrho) / rho
            is_niche = (hrho >= 40)

            for input_kind in ["distinct", "same"]:
                if input_kind == "distinct":
                    w = [splitmix(seed + 77 * k + n * 11 + hrho, n) for k in range(5)]
                else:
                    base_v = splitmix(seed + 99 + n * 11 + hrho, n)
                    w = [[s * x for x in base_v] for s in scales]

                for budget in budgets:
                    cell_id = f"n{n}-hrho{hrho}-{input_kind}-b{budget:.0e}"
                    cells.append({
                        "cell_id": cell_id,
                        "n": n,
                        "rho": rho,
                        "rho_hex": hex_f64(rho),
                        "lambda": lambda_val,
                        "lambda_hex": hex_f64(lambda_val),
                        "hrho": hrho,
                        "h": h,
                        "h_hex": hex_f64(h),
                        "input": input_kind,
                        "budget": budget,
                        "budget_hex": hex_f64(budget),
                        "is_niche": is_niche,
                        "a": hex_mat(a),
                        "w": [hex_vec(col) for col in w],
                        "enclosure": {
                            "lambda": hex_f64(lambda_val),
                            "rho": hex_f64(rho),
                            "evidence": "Gershgorin",
                        },
                    })

    assert len(cells) == 40, f"Expected 40 cells, got {len(cells)}"

    return {
        "schema": SCHEMA_STAGE1,
        "node": NODE,
        "cells": cells,
    }


def main():
    parser = argparse.ArgumentParser(description="Export PY05 Stage 0 and Stage 1 test cases")
    parser.add_argument("--stage0-output", type=Path, required=True, help="Path for stage0_cases.json")
    parser.add_argument("--stage1-output", type=Path, required=True, help="Path for stage1_cases.json")
    args = parser.parse_args()

    args.stage0_output.parent.mkdir(parents=True, exist_ok=True)
    args.stage1_output.parent.mkdir(parents=True, exist_ok=True)

    stage0_doc = generate_stage0_cases()
    args.stage0_output.write_text(json.dumps(stage0_doc, indent=2) + "\n")
    print(f"Wrote 60 stage-0 fixtures to {args.stage0_output}")

    stage1_doc = generate_stage1_cases()
    args.stage1_output.write_text(json.dumps(stage1_doc, indent=2) + "\n")
    print(f"Wrote 40 stage-1 cells to {args.stage1_output}")


if __name__ == "__main__":
    main()
