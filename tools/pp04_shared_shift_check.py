#!/usr/bin/env python3
"""Gate of research node research/pp04_shared_shift_policy_20261004 (RVJ DAG node PP04).

G1: every Certified candidate of every method on n = 8 encloses the exact Euclidean error
(exact rational Gauss elimination on the exact binary64 inputs; error^2 <= bound^2 compared
exactly); every target is Certified or explicitly not. G2: the executed flops of M1-M3 equal
the prediction and the formulas; the executed flops of M4 equal the jet formula plus the
charged fallback work. G3: the selector's choice has the least executed flops (ties
correct) in at least 90 % of configurations.
"""

from __future__ import annotations

import argparse
import json
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


def lu(n):
    return 2 * n**3 // 3


def expected_flops(config, method, work):
    n, r, m = config["n"], config["r"], config["m"]
    nn = n * n
    k = len(set(config["gammas"]))
    if method == "individual-lu":
        return m * (2 * nn + lu(n) + r * 2 * nn)
    if method == "common-shift-lu":
        return k * (2 * nn + lu(n) + r * 2 * nn)
    if method == "hessenberg-reuse":
        return 14 * n**3 // 3 + r * 2 * nn + k * 3 * nn + m * r * 5 * nn
    # Jet: the cluster formula plus fallback (Hessenberg setup once, then per
    # fallback target the Hessenberg LU and r solves).
    total = 0
    for c in config["plan"]["clusters"]:
        d, s = c["degree"], len(c["targets"])
        total += 2 * nn + lu(n) + (d + 1) * r * 2 * nn + d * r * n + s * r * 2 * n * d
    f = work["fallback_targets"]
    if f:
        total += 14 * n**3 // 3 + r * 2 * nn + f * (3 * nn + r * 5 * nn)
    return total


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    data = json.loads(args.cases.read_text())
    configs = data["configs"]
    g1 = True
    g1_rows = g1_certified = 0
    worst_ratio = 0.0
    g2 = True
    g2_mismatch = []
    selector_correct = 0
    jet_wins = []
    abstentions = {}
    fallbacks = 0
    rejected_targets = 0
    summary = []
    for cfg in configs:
        methods = cfg["methods"]
        if not all(v["ok"] for v in methods.values()):
            g1 = False
            summary.append({"config": [cfg["n"], cfg["r"], cfg["m"], cfg["set"]], "error": "method failed"})
            continue
        executed = {k: v["work"]["flops"] for k, v in methods.items()}
        pred = dict(cfg["plan"]["predicted_flops"])
        for k, v in methods.items():
            exp = expected_flops(cfg, k, v["work"])
            if k != "shared-jet" and pred[k] != exp:
                g2 = False
                g2_mismatch.append({"config": [cfg["n"], cfg["r"], cfg["m"], cfg["set"]],
                                    "method": k, "predicted": pred[k], "formula": exp})
            if exp != executed[k]:
                g2 = False
                g2_mismatch.append({"config": [cfg["n"], cfg["r"], cfg["m"], cfg["set"]],
                                    "method": k, "expected": exp, "executed": executed[k]})
            rejected_targets += v["targets"] - v["certified_targets"]
        fallbacks += methods["shared-jet"]["work"]["fallback_targets"]
        best = min(executed.values())
        chosen = cfg["plan"]["chosen"]
        selector_correct += executed[chosen] == best
        ab = cfg["plan"]["jet_abstention"]
        abstentions[str(ab)] = abstentions.get(str(ab), 0) + 1
        others = min(executed[k] for k in executed if k != "shared-jet")
        if executed["shared-jet"] < others:
            jet_wins.append({"config": [cfg["n"], cfg["r"], cfg["m"], cfg["set"]],
                             "jet": executed["shared-jet"], "best_other": others,
                             "ratio": others / executed["shared-jet"]})
        summary.append({"config": [cfg["n"], cfg["r"], cfg["m"], cfg["set"]], "executed": executed,
                        "chosen": chosen, "abstention": ab,
                        "certified": {k: v["certified_targets"] for k, v in methods.items()},
                        "fallbacks": methods["shared-jet"]["work"]["fallback_targets"]})
        if cfg["n"] != 8:
            continue
        n = cfg["n"]
        j = [[Fraction(unhex(x)) for x in row] for row in cfg["j"]]
        h = Fraction(unhex(cfg["h"]))
        rhs = [[Fraction(unhex(x)) for x in b] for b in cfg["rhs"]]
        cache = {}
        for name, v in methods.items():
            for t in v["candidates"]:
                gamma = Fraction(unhex(t["gamma"]))
                for ci, (col, bound_h) in enumerate(zip(t["columns"], t["error_upper"])):
                    key = (t["gamma"], ci)
                    if key not in cache:
                        a = [[(1 if i == k else 0) - gamma * h * j[i][k] for k in range(n)]
                             for i in range(n)]
                        cache[key] = exact_solve(a, rhs[ci])
                    x = cache[key]
                    u = [Fraction(unhex(c)) for c in col]
                    err2 = sum((xi - ui) ** 2 for xi, ui in zip(x, u))
                    bound = Fraction(unhex(bound_h))
                    g1_rows += 1
                    if t["certified"]:
                        g1_certified += 1
                        if err2 > bound * bound:
                            g1 = False
                    if bound > 0:
                        worst_ratio = max(worst_ratio, float(err2) ** 0.5 / float(bound))
    total = len(configs)
    g3 = selector_correct >= 0.9 * total
    result = {
        "schema": "vigilode-pp04-check-v1",
        "configs": total,
        "g1_rows_n8": g1_rows,
        "g1_certified_rows_n8": g1_certified,
        "g1_worst_error_over_bound_n8": worst_ratio,
        "rejected_targets_all_methods": rejected_targets,
        "jet_fallback_targets": fallbacks,
        "g2_mismatches": g2_mismatch,
        "selector_correct": selector_correct,
        "jet_abstentions": abstentions,
        "jet_positive_margin_configs": jet_wins,
        "gate": {"g1_exact_enclosure": g1, "g2_accounting": g2, "g3_selector_90pct": g3},
        "rows": summary,
    }
    result["verdict"] = "PASS" if (g1 and g2 and g3) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: result[k] for k in ("configs", "g1_rows_n8", "g1_certified_rows_n8",
                                             "g1_worst_error_over_bound_n8", "rejected_targets_all_methods",
                                             "jet_fallback_targets", "selector_correct", "jet_abstentions",
                                             "gate", "verdict")}))
    print("jet positive margin:", len(jet_wins), "of", total)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
