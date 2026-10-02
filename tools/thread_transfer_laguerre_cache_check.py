#!/usr/bin/env python3
"""Exact check of the same-vector combined Laguerre adjoint bound (research node
research/thread_transfer_laguerre_cache_20261002, DAG node P2-LAGUERRE-CACHE).

For each case of fixtures/thread_transfer_laguerre_cache_cases.json: the exact recurrence t_n from
the source (X = -A, beta = 1), the native recurrence vectors t_hat_n, the stored column coefficients
c_(n,k) and scales s_k as exact rationals; the fused recurrence error
sum_k s_k sum_n c_(n,k) (t_hat_n - t_n) must have 2-norm at most the native combined bound.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from fractions import Fraction as F
from pathlib import Path


def val(h):
    x = struct.unpack(">d", bytes.fromhex(h))[0]
    if not math.isfinite(x):
        raise ValueError(h)
    return F(x)


def check(case):
    X = [[-val(x) for x in row] for row in case["matrix"]]
    n = len(X)
    w = [val(x) for x in case["source"]]
    s = [val(x) for x in case["scales"]]
    c = [[val(x) for x in row] for row in case["stored"]]
    that = [[val(x) for x in v] for v in case["vectors"]]
    m = case["degree"]
    exact = [w]
    for j in range(1, m + 1):
        k = j - 1
        xt = [sum((X[i][q] * exact[k][q] for q in range(n)), F(0)) for i in range(n)]
        prev = exact[k - 1] if k > 0 else [F(0)] * n
        exact.append([(F(2 * k + 1) * exact[k][i] - xt[i] - F(k) * prev[i]) / F(k + 1) for i in range(n)])
    fused = [sum((s[k] * c[q][k] * (that[q][i] - exact[q][i]) for q in range(m + 1) for k in range(len(s))), F(0))
             for i in range(n)]
    error2 = sum((x * x for x in fused), F(0))
    bound = val(case["combined_bound"])
    return {"degree": m, "fused_error_2norm": math.sqrt(float(error2)), "combined_bound": float(bound),
            "enclosed": error2 <= bound * bound}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    rows = [check(c) for c in json.loads(args.cases.read_text())["cases"]]
    gate = {"combination_valid": all(r["enclosed"] for r in rows)}
    report = {"schema": "vigilode-thread-transfer-laguerre-cache-exact-check-v1", "cases": rows, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
