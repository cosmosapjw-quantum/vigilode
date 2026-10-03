#!/usr/bin/env python3
"""50-digit check of the nonnormal exponential-action certificates (research node
research/int05_nonnormal_metric_bound_20261003, integrated DAG node INT-05).

For every case written by crates/rodas5p-core/tests/int05_nonnormal_metric_bound.rs it
computes exp(tau A) v for the binary64 A, v, tau (exact reals) with mpmath.expm at 50
digits, the true error of the candidate, and the gate of the preregistration.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-int05-nonnormal-metric-bound-v1"


def unhex(text: str):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 50
    cases = json.loads(args.cases.read_text())["cases"]
    cache = {}
    rows = []
    enclosure = True
    for case in cases:
        key = (case["family"], case["tau"])
        if key not in cache:
            a = mp.matrix([[unhex(x) for x in row] for row in case["a"]])
            v = mp.matrix([unhex(x) for x in case["v"]])
            cache[key] = mp.expm(a * unhex(case["tau"])) * v
        exact = cache[key]
        x = [unhex(h) for h in case["candidate"]]
        error = mp.sqrt(mp.fsum((x[i] - exact[i]) ** 2 for i in range(len(x))))
        bounded = case["status"] == "bounded"
        inside = (not bounded) or (mp.mpf(case["error_lower"]) <= error <= mp.mpf(case["error_upper"]))
        enclosure &= inside
        rows.append({
            "family": case["family"], "metric": case["metric"], "degree": case["degree"],
            "candidate_kind": case["candidate_kind"], "status": case["status"],
            "true_error": mp.nstr(error, 8), "error_lower": case["error_lower"], "error_upper": case["error_upper"],
            "upper_over_true": (mp.nstr(mp.mpf(case["error_upper"]) / error, 6) if bounded and error > 0 else None),
            "truncation_upper": case["truncation_upper"], "transport": case["transport"],
            "numerical_range": case["numerical_range"], "enclosed": bool(inside),
        })

    def find(family, metric, degree, kind):
        return next(r for r in rows if r["family"] == family and r["metric"] == metric
                    and r["degree"] == degree and r["candidate_kind"] == kind)

    exposed = all(
        find(f"vig-a02-k{k}", "diagonal", m, "arnoldi-near-breakdown")["error_lower"] >= 1e-2
        for k in (10, 20, 46) for m in (10, 20, 30))
    d = find("vig-a02-k46", "diagonal", 30, "own")
    e = find("vig-a02-k46", "identity", 30, "own")
    useful = (d["status"] == "bounded" and d["error_upper"] <= 1e-10
              and (e["status"] == "unbounded" or e["error_upper"] > 1.0))
    gate = {"1_enclosure": bool(enclosure), "2_counterexample_exposed": bool(exposed),
            "3_metric_useful": bool(useful), "4_typed_rejections": "contract tests int05_nonnormal_contracts"}
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "rows": rows, "gate": gate,
              "verdict": "PASS" if all(v is True for k, v in gate.items() if not k.startswith("4")) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
