#!/usr/bin/env python3
"""Export the dynamic-range oracle of the joint phi action (re-audit R4 of 2026-10-01,
POLY-DEV-01) to ``fixtures/r4_polynomial_range_fixtures.json``.

* ``norms``: vectors whose exact Euclidean norm (``fractions.Fraction`` of the binary64
  entries, integer square root) is rounded up on its binade to ``m 2^e`` with ``m`` in
  [1/2, 1); a bound encloses the norm iff it is at least ``norm_upper``.
* ``actions``: ``F = sum_k phi_k(h A) w_k`` for ``A = diag(-1, -2)``, ``h = 0.1`` and
  amplitude-scaled ``w_k`` (exact binary64 inputs), with ``phi_k`` of the two eigenvalues
  by mpmath at 80 digits (``phi_k(z) = (phi_{k-1}(z) - 1/(k-1)!) / z``); each component of
  ``F`` is given as a binary64 interval ``[lo, hi]`` rounded outward, so
  ``max(|fused - lo|, |hi - fused|)`` is an upper estimate of the true error.

Requires mpmath. Run from the repository root.
"""

import json
import math
import struct
from fractions import Fraction as F
from pathlib import Path

import mpmath

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "fixtures/r4_polynomial_range_fixtures.json"
mpmath.mp.dps = 80
TINY = 5e-324


def hexbits(x):
    return struct.pack(">d", x).hex()


def norm_upper(vector):
    square = sum(F(x) ** 2 for x in vector)
    if square == 0:
        return dict(mantissa=0.0, exponent=0)
    shift = 1200
    scaled = square * F(2) ** (2 * shift)
    root = F(math.isqrt(scaled.numerator // scaled.denominator) + 1, 2**shift)
    assert root * root >= square
    e = root.numerator.bit_length() - root.denominator.bit_length()
    while root / F(2) ** e >= 1:
        e += 1
    while root / F(2) ** e < F(1, 2):
        e -= 1
    m = float(root / F(2) ** e)
    if F(m) < root / F(2) ** e:
        m = math.nextafter(m, math.inf)
    while True:
        lower = math.nextafter(m, 0.0)
        if lower < 0.5 or (F(lower) * F(2) ** e) ** 2 < square:
            break
        m = lower
    return dict(mantissa=m, exponent=e)


def phi(z, k):
    z = mpmath.mpf(z)
    value = mpmath.exp(z)
    for j in range(1, k + 1):
        value = (value - 1 / mpmath.factorial(j - 1)) / z
    return value


def outward(value):
    nearest = float(value)
    lo = nearest if mpmath.mpf(nearest) <= value else math.nextafter(nearest, -math.inf)
    hi = nearest if mpmath.mpf(nearest) >= value else math.nextafter(nearest, math.inf)
    return lo, hi


def action(name, vectors, h=0.1, eigenvalues=(-1.0, -2.0)):
    exact_h = mpmath.mpf(F(h).numerator) / mpmath.mpf(F(h).denominator)
    rows = []
    for i, lam in enumerate(eigenvalues):
        total = mpmath.mpf(0)
        for k, vector in enumerate(vectors):
            total += phi(exact_h * lam, k) * mpmath.mpf(F(vector[i]).numerator) / mpmath.mpf(F(vector[i]).denominator)
        rows.append(total)
    return dict(
        id=name,
        h=hexbits(h),
        eigenvalues=[hexbits(x) for x in eigenvalues],
        vectors=[[hexbits(x) for x in vector] for vector in vectors],
        exact_lo=[hexbits(outward(x)[0]) for x in rows],
        exact_hi=[hexbits(outward(x)[1]) for x in rows],
    )


def main():
    norms = []
    for vector in [
        [1e300, 1e300],
        [1e-300, 1e-300],
        [TINY, TINY],
        [1.0, TINY],
        [0.0, 0.0],
        [1.7976931348623157e308, 1.7976931348623157e308],
        [3.0, -4.0],
        [1e300, 1e-300, -2.5e-310],
    ]:
        norms.append(dict(vector=[hexbits(x) for x in vector], norm_upper=norm_upper(vector)))
    unit = [[1.0, 0.0], [0.0, 1.0], [0.5, -0.25], [0.0, 0.0], [0.125, 0.0]]
    actions = []
    for label, amplitude in [
        ("unit", 1.0),
        ("inside_window", 1e-140),
        ("tiny", 1e-300),
        ("huge", 1e300),
        ("power_of_two_tiny", 3.0 * 2.0**-600),
        ("min_subnormal", TINY),
    ]:
        vectors = [[amplitude * x for x in vector] for vector in unit]
        actions.append(action(label, vectors))
    # Mixed range: scaling the 1e300 input down loses the 1e-300 entry.
    actions.append(
        action(
            "mixed_lossy",
            [[1e300, 1e-300], [0.0, 3e-310], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
        )
    )
    OUT.write_text(
        json.dumps(
            dict(
                schema="vigilode-r4-polynomial-range-oracle-v1",
                generator="tools/r4_export_polynomial_range_fixtures.py",
                norms=norms,
                actions=actions,
            ),
            indent=1,
        )
        + "\n"
    )
    print(f"{len(norms)} norms, {len(actions)} actions -> {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
