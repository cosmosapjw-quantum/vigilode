#!/usr/bin/env python3
"""Independent exact-rational scalar source oracle for frozen f64 inputs.

Reads executed Rust CFF_ORACLE lines. Fractions reconstruct the *exact real*
meaning of represented binary64 source and trial state. No native source
function or interval implementation is imported. This checks finite endpoints
and the exact norm inequalities, not the Rust interval implementation itself.
"""
import json
import struct
import sys
from fractions import Fraction

def real(bits):
    return Fraction.from_float(struct.unpack(">d", struct.pack(">Q", int(bits)))[0])

def main(path):
    rows = []
    for line in open(path, encoding="utf8"):
        if "CFF_ORACLE=" in line:
            rows.append(json.loads(line.split("CFF_ORACLE=", 1)[1].strip()))
    assert len(rows) == 4, f"exactly four executed oracle cohorts required, got {len(rows)}"
    seen = set()
    total_components = 0
    for data in rows:
        n = int(data["n"])
        assert n in (1, 3, 12, 256) and n not in seen
        seen.add(n)
        pairs = data["source_bits"]
        xbits = data["root_bits"]
        assert len(pairs) == len(xbits) == n
        q = real(data["source_q_bits"])
        assert 0 <= q < 1
        expected_q = max(abs(real(a)) for a, _ in pairs)
        assert q == expected_q
        exact_error_sq = Fraction(0)
        exact_residual_sq = Fraction(0)
        for (a_bits, b_bits), x_bits in zip(pairs, xbits):
            a, b, x = real(a_bits), real(b_bits), real(x_bits)
            assert abs(a) <= q
            exact_xstar = b / (1 - a)
            exact_error_sq += (x - exact_xstar) ** 2
            exact_residual_sq += (a*x + b - x) ** 2
        error_upper = real(data["certified_upper_bits"])
        residual_lower = real(data["exact_residual_lower_bits"])
        residual_upper = real(data["exact_residual_upper_bits"])
        target = real(data["root_error_target_bits"])
        assert 0 <= residual_lower <= residual_upper
        assert residual_lower*residual_lower <= exact_residual_sq, f"false lower n={n}"
        assert exact_residual_sq <= residual_upper*residual_upper, f"false upper n={n}"
        assert exact_error_sq <= error_upper*error_upper, f"false certified root upper n={n}"
        assert error_upper <= target, f"outside original target n={n}"
        assert int(data["calls"]) > 0
        total_components += n
        print(f"FRACTION_N{n}=PASS calls={data['calls']} upper={float(error_upper):.17e}")
    print(f"INDEPENDENT_EXACT_FRACTION={len(rows)}/{len(rows)}_PASS components={total_components}")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python independent_fraction_oracle.py BENCH.log")
    main(sys.argv[1])
