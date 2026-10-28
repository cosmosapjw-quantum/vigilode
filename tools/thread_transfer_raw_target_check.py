#!/usr/bin/env python3
"""Exact checks of the raw-stage (U) / native K target contract (research node
research/thread_transfer_mf_target_20261002, thread-transfer DAG node P0-MF-TARGET).

1. Symbolic identity (SymPy): for generic symbolic 2x2 M and J, a generic 3-stage strictly lower raw C and A,
   symbolic gamma, h, stages K, time derivative f_t and stage function values F, with Gamma = (I/gamma - C)^-1,
   alpha = A Gamma and U = Gamma K,

       r_K = (I (x) M - h Gamma (x) J) K - h F(y + alpha K) - h^2 gamma_rows (x) f_t,
       r_U = ((I - gamma C) (x) M - h gamma I (x) J) U - h gamma F(y + A U) - h^2 gamma gamma_rows (x) f_t,

   the stage arguments y + A U and y + alpha K agree identically, and r_U - gamma r_K reduces to zero.
2. Exact containment: every exact rational value of D_Gamma = (I - gamma C) S - gamma I, D_alpha = A S - alpha0,
   b_code^T S - b^T, e_s^T S - btilde^T and H_raw S - D_dense, computed with Fraction from the coefficient bits
   in fixtures/thread_transfer_raw_stage_allowance.json, lies in the corresponding Rust interval.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from fractions import Fraction
from pathlib import Path

import sympy as sp

SCHEMA = "vigilode-thread-transfer-raw-target-exact-check-v1"


def symbolic_identity() -> dict:
    s, n = 3, 2
    g, h = sp.symbols("gamma h", nonzero=True)
    M = sp.Matrix(n, n, lambda i, j: sp.Symbol(f"M{i}{j}"))
    J = sp.Matrix(n, n, lambda i, j: sp.Symbol(f"J{i}{j}"))
    C = sp.Matrix(s, s, lambda i, j: sp.Symbol(f"C{i}{j}") if j < i else 0)
    A = sp.Matrix(s, s, lambda i, j: sp.Symbol(f"A{i}{j}") if j < i else 0)
    gamma_rows = [sp.Symbol(f"g{i}") for i in range(s)]
    y = sp.Matrix(n, 1, lambda a, _: sp.Symbol(f"y{a}"))
    ft = sp.Matrix(n, 1, lambda a, _: sp.Symbol(f"ft{a}"))
    K = [sp.Matrix(n, 1, lambda a, _, i=i: sp.Symbol(f"K{i}_{a}")) for i in range(s)]
    Gamma = (sp.eye(s) / g - C).inv()
    alpha = (A * Gamma).applyfunc(sp.simplify)
    U = [sum((Gamma[i, j] * K[j] for j in range(s)), sp.zeros(n, 1)) for i in range(s)]
    arguments_equal = True
    for i in range(s):
        arg_k = y + sum((alpha[i, j] * K[j] for j in range(s)), sp.zeros(n, 1))
        arg_u = y + sum((A[i, j] * U[j] for j in range(s)), sp.zeros(n, 1))
        arguments_equal &= all(sp.simplify(e) == 0 for e in (arg_u - arg_k))
    # With equal arguments both forms evaluate F at the same point.
    F = [sp.Matrix(n, 1, lambda a, _, i=i: sp.Symbol(f"F{i}_{a}")) for i in range(s)]
    zero = True
    for i in range(s):
        r_k = M * K[i] - h * sum((Gamma[i, j] * (J * K[j]) for j in range(s)), sp.zeros(n, 1)) \
            - h * F[i] - h**2 * gamma_rows[i] * ft
        r_u = M * U[i] - g * sum((C[i, j] * (M * U[j]) for j in range(s)), sp.zeros(n, 1)) \
            - h * g * (J * U[i]) - h * g * F[i] - h**2 * g * gamma_rows[i] * ft
        zero &= all(sp.simplify(sp.together(e)) == 0 for e in (r_u - g * r_k))
    return {"stages": s, "dimension": n, "arguments_equal": bool(arguments_equal),
            "residual_identity": bool(zero), "holds": bool(arguments_equal and zero)}


def value(hex_bits: str) -> Fraction:
    number = struct.unpack(">d", bytes.fromhex(hex_bits))[0]
    if not math.isfinite(number):
        raise ValueError(hex_bits)
    return Fraction(number)


def exact_containment(allowance: dict) -> dict:
    c = allowance["coefficients"]
    gamma = value(c["gamma"])
    mat = lambda key: [[value(x) for x in row] for row in c[key]]
    vec = lambda key: [value(x) for x in c[key]]
    a, cm, gm, al, dh, dd = mat("a"), mat("c_matrix"), mat("gamma_matrix"), mat("alpha"), mat("dense_h"), mat("dense_d")
    b_code, b, btilde = vec("b_code"), vec("b"), vec("btilde")
    s = len(a)
    S = [[gm[i][j] if j < i else (gamma if j == i else Fraction(0)) for j in range(s)] for i in range(s)]
    exact = {
        "d_gamma": [[S[i][j] - gamma * sum(cm[i][k] * S[k][j] for k in range(s)) - (gamma if i == j else 0)
                     for j in range(s)] for i in range(s)],
        "d_alpha": [[sum(a[i][k] * S[k][j] for k in range(s)) - (al[i][j] if j < i else 0)
                     for j in range(s)] for i in range(s)],
        "d_output": [sum(b_code[i] * S[i][j] for i in range(s)) - b[j] for j in range(s)],
        "d_embedded": [S[s - 1][j] - btilde[j] for j in range(s)],
        "d_dense": [[sum(dh[r][k] * S[k][j] for k in range(s)) - dd[r][j] for j in range(s)]
                    for r in range(len(dh))],
    }
    report = {}
    holds = True
    for key, values in exact.items():
        enclosure = allowance["enclosures"][key]
        flat_exact = [x for row in values for x in row] if isinstance(values[0], list) else values
        flat_box = [x for row in enclosure for x in row] if isinstance(enclosure[0][0], list) else enclosure
        assert len(flat_exact) == len(flat_box)
        inside = all(value(lo) <= x <= value(hi) for x, (lo, hi) in zip(flat_exact, flat_box))
        width = max(float(value(hi) - value(lo)) for lo, hi in flat_box)
        report[key] = {"contained": inside, "entries": len(flat_exact),
                       "exact_max_abs": float(max(abs(x) for x in flat_exact)), "max_width": width}
        holds &= inside
    report["holds"] = holds
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--allowance", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    allowance = json.loads(args.allowance.read_text())
    identity = symbolic_identity()
    containment = exact_containment(allowance)
    gate = {"symbolic_identity": identity["holds"], "exact_containment": containment["holds"]}
    report = {"schema": SCHEMA, "sympy": sp.__version__, "coefficient_sha256": allowance["coefficient_sha256"],
              "symbolic": identity, "containment": containment, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
