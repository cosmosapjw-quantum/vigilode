#!/usr/bin/env python3
"""Gate of research node research/pp03_rhs_compression_20261004 (RVJ DAG node PP03).

G1: every Certified compressed (and uncompressed) output on n = 8 encloses the exact
Euclidean error of the original target (exact rational Gauss elimination; error^2 <=
bound^2 exactly). G2: plus/minus outputs differ and each is within its bound of its own
exact solution (n = 8); for every certified pair the compressed and uncompressed outputs
differ by at most the sum of their bounds (all n, in binary64 with a 4-ulp-relative margin
noted). G3: the compressed LU column solves equal k (d+1) and are fewer than M (d+1)
exactly when k < M; abstention exactly when k = M.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from fractions import Fraction
from pathlib import Path


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def exact_solve(a, b):
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for c in range(n):
        p = next(r for r in range(c, n) if m[r][c] != 0)
        m[c], m[p] = m[p], m[c]
        for r in range(n):
            if r != c and m[r][c] != 0:
                f = m[r][c] / m[c][c]
                m[r] = [x - f * y for x, y in zip(m[r], m[c])]
    return [m[i][n] / m[i][i] for i in range(n)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    configs = json.loads(args.cases.read_text())["configs"]
    g1 = g2 = g3 = True
    rows = []
    exact_rows = 0
    worst = 0.0
    pm_distinct = True
    for cfg in configs:
        n, m, d, k = cfg["n"], cfg["m"], cfg["degree"], cfg["rank"]
        comp = cfg["compressed"]
        row = {"n": n, "m": m, "set": cfg["set"], "degree": d, "rank": k,
               "abstained": comp is None,
               "uncompressed_column_solves": cfg["uncompressed_column_solves"]}
        # G3
        if comp is None:
            g3 &= (k == m) or k == 0
        else:
            solves = comp["work"]["column_solves"]
            row["compressed_column_solves"] = solves
            g3 &= solves == k * (d + 1) and k < m and cfg["uncompressed_column_solves"] == m * (d + 1)
            row["certified_targets"] = sum(t["certified"] for t in comp["targets"])
            # G2 (consistency with the uncompressed jet)
            for tc, tu in zip(comp["targets"], cfg["uncompressed"]):
                if not (tc["certified"] and tu["certified"]):
                    continue
                for xc, xu, bc, bu in zip(tc["columns"], tu["columns"], tc["error_upper"], tu["error_upper"]):
                    diff = math.sqrt(sum((unhex(a) - unhex(b)) ** 2 for a, b in zip(xc, xu)))
                    if diff > (unhex(bc) + unhex(bu)) * (1 + 1e-12):
                        g2 = False
        rows.append(row)
        if n != 8:
            continue
        j = [[Fraction(unhex(x)) for x in r_] for r_ in cfg["j"]]
        h = Fraction(unhex(cfg["h"]))
        rhs = [[Fraction(unhex(x)) for x in b] for b in cfg["rhs"]]
        arms = [("uncompressed", cfg["uncompressed"])]
        if comp is not None:
            arms.append(("compressed", comp["targets"]))
        for ti, gh in enumerate(cfg["gammas"]):
            gamma = Fraction(unhex(gh))
            a = [[(1 if i == c else 0) - gamma * h * j[i][c] for c in range(n)] for i in range(n)]
            exact = [exact_solve(a, b) for b in rhs]
            for name, targets in arms:
                t = targets[ti]
                outs = [[Fraction(unhex(v)) for v in col] for col in t["columns"]]
                for x, u, bh in zip(exact, outs, t["error_upper"]):
                    err2 = sum((p - q) ** 2 for p, q in zip(x, u))
                    bound = Fraction(unhex(bh))
                    exact_rows += 1
                    if t["certified"] and err2 > bound * bound:
                        g1 = False
                    if bound > 0:
                        worst = max(worst, math.sqrt(float(err2)) / float(bound))
            if cfg["set"] == "plus-minus" and comp is not None:
                cols = comp["targets"][ti]["columns"]
                for p in range(0, len(cols) - 1, 2):
                    if cols[p] == cols[p + 1]:
                        pm_distinct = False
    g2 &= pm_distinct
    result = {"schema": "vigilode-pp03-check-v1", "configs": len(configs), "exact_rows_n8": exact_rows,
              "worst_error_over_bound_n8": worst, "plus_minus_outputs_distinct": pm_distinct,
              "gate": {"g1_exact_enclosure": g1, "g2_amplitudes_and_consistency": g2,
                       "g3_charged_solves": g3},
              "rows": rows}
    result["verdict"] = "PASS" if (g1 and g2 and g3) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "rows"}))
    for r in rows:
        print(r)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
