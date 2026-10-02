#!/usr/bin/env python3
"""Exact-rational stage roots of the thread-transfer R4 fixtures (research nodes
research/thread_transfer_path_action_20261002 and research/thread_transfer_radius_proposal_20261002).

Reads fixtures/thread_transfer_r4_stage_inputs.json (written by the ignored Rust test
`write_r4_stage_inputs`): the binary64 bits of J (diagonal), y, q, h, the candidate stages, gamma,
the strictly lower alpha rows and the native Gamma rows. Every value is taken as the exact rational
it encodes. The declared stage equations are

    (1 - h gamma J_aa) K_ia = h (J y - q y^2)_a + h J_aa sum_{j<i} (alpha_ij + Gamma_ij) K_ja
                              + h q_a (sum_{j<i} alpha_ij K_ja)^2,

explicit in K_i given K_j (j < i), so the root is computed exactly with Fraction. The output gives,
for every stage and component, the binary64 bracket [down, up] of the exact distance
|K_hat_ia - K*_ia| (down the largest double <= it, up the smallest double >= it), and copies the
inputs so a stale oracle is detected by the Rust tests.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = "vigilode-thread-transfer-r4-root-oracle-v1"


def value(hex_bits: str) -> Fraction:
    number = struct.unpack(">d", bytes.fromhex(hex_bits))[0]
    if not math.isfinite(number):
        raise ValueError(f"non-finite input {hex_bits}")
    return Fraction(number)


def hex_of(number: float) -> str:
    return struct.pack(">d", number).hex()


def bracket(exact: Fraction) -> list[str]:
    """Binary64 [down, up] around a nonnegative exact rational."""
    assert exact >= 0
    nearest = float(exact)  # correctly rounded
    if Fraction(nearest) == exact:
        return [hex_of(nearest), hex_of(nearest)]
    if Fraction(nearest) < exact:
        return [hex_of(nearest), hex_of(math.nextafter(nearest, math.inf))]
    return [hex_of(math.nextafter(nearest, -math.inf)), hex_of(nearest)]


def solve(fixture: dict) -> list[list[list[str]]]:
    n = fixture["dimension"]
    h = value(fixture["h"])
    gamma = value(fixture["gamma"])
    jac = [[value(x) for x in row] for row in fixture["jacobian"]]
    for a in range(n):
        for b in range(n):
            if a != b and jac[a][b] != 0:
                raise ValueError("the oracle needs a diagonal J")
    y = [value(x) for x in fixture["y"]]
    q = [value(x) for x in fixture["q"]]
    candidate = [[value(x) for x in stage] for stage in fixture["candidate"]]
    alpha = [[value(x) for x in row] for row in fixture["alpha_rows"]]
    gam = [[value(x) for x in row] for row in fixture["gamma_rows_strict_lower"]]
    s = len(alpha)
    root: list[list[Fraction]] = []
    for i in range(s):
        row = []
        for a in range(n):
            delta = sum((alpha[i][j] * root[j][a] for j in range(i)), Fraction(0))
            coupled = sum(((alpha[i][j] + gam[i][j]) * root[j][a] for j in range(i)), Fraction(0))
            right = (h * (jac[a][a] * y[a] - q[a] * y[a] * y[a])
                     + h * jac[a][a] * coupled + h * q[a] * delta * delta)
            row.append(right / (1 - h * gamma * jac[a][a]))
        root.append(row)
    return [[bracket(abs(candidate[i][a] - root[i][a])) for a in range(n)] for i in range(s)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", type=Path, default=ROOT / "fixtures/thread_transfer_r4_stage_inputs.json")
    parser.add_argument("--output", type=Path, default=ROOT / "fixtures/thread_transfer_r4_root_oracle.json")
    args = parser.parse_args()
    inputs = json.loads(args.inputs.read_text())
    fixtures = [{"dimension": f["dimension"], "stage_distance": solve(f)} for f in inputs["fixtures"]]
    report = {"schema": SCHEMA, "inputs": inputs, "fixtures": fixtures}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(f"wrote {args.output}")


if __name__ == "__main__":
    main()
