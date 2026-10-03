#!/usr/bin/env python3
"""Independent 50-digit check of the Laguerre total admission (research node
research/rnext04_laguerre_admission_20261003, remaining-only DAG node R-NEXT-04).

For every case written by crates/rodas5p-core/tests/rnext04_laguerre_admission.rs it
computes the exact target F = sum_k phi_k(h A) w_k for the binary64 A, h and w_k (taken as
exact reals) by a symmetric eigendecomposition in mpmath, and checks ||fused - F||_2 <=
bound for every admitted case. It combines that with the native gate items into RESULTS.json.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-rnext04-laguerre-admission-v1"


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


def exact_target(case) -> list:
    n = case["n"]
    a = mp.matrix([[mp.mpf(unhex(x)) for x in row] for row in case["a"]])
    h = mp.mpf(unhex(case["h"]))
    w = [[mp.mpf(unhex(x)) for x in vec] for vec in case["w"]]
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.cases.read_text())
    mp.mp.dps = 50
    rows = []
    enclosed_all = True
    for case in data["cases"]:
        if "error" in case:
            rows.append({"label": case["label"], "error": case["error"]})
            continue
        exact = exact_target(case)
        fused = [mp.mpf(unhex(x)) for x in case["fused"]]
        distance = mp.sqrt(mp.fsum((f - e) ** 2 for f, e in zip(fused, exact)))
        row = {"label": case["label"], "degree": case["degree"], "branch": case["branch"],
               "normalization_shift": case["normalization_shift"],
               "admitted": case["admission"]["admitted"], "actual_error": mp.nstr(distance, 6),
               "condition_proxy": case["condition_proxy"]}
        if case["admission"]["admitted"]:
            bound = mp.mpf(unhex(case["admission"]["bound"]))
            ok = distance <= bound
            enclosed_all &= bool(ok)
            row.update({"bound": mp.nstr(bound, 6), "enclosed": bool(ok),
                        "tightness": mp.nstr(bound / distance, 6) if distance > 0 else "inf"})
        else:
            row["reason"] = case["admission"]["reason"]
        rows.append(row)
        print(row["label"], row.get("enclosed"), row.get("tightness"), flush=True)
    native = data["native_gate"]
    gate = {
        "1_coverage": native["1_coverage"],
        "2_enclosure": enclosed_all and data["admitted_cases"] > 0,
        "3_guards": native["3_guards"] and native["3_laguerre_recurrence_reports_stay_estimate_only"],
        "4_bounded_resources": native["4_bounded_resources"],
    }
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "rows": rows,
              "guards": data["guards"], "resources": data["resources"],
              "admitted_cases": data["admitted_cases"], "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
