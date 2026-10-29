#!/usr/bin/env python3
"""Export the exact-rational oracle of the phi-weight transform error (re-audit R4 of
2026-10-01, ARITH-DEV-01..03) to ``fixtures/r4_transform_oracle_fixtures.json``.

For each case the stored weights ``w~_k`` are recomputed by an emulation of
``rodas5p_core::times_power`` (binary64 operations in the same order; the Rust test pins
the emulation by comparing bits with ``weight_phi_vectors``), and the transform error

    e = sum_k phi_k(hA) (h^k b_k - w~_k),   phi_k(hA) = sum_{j < n} (hA)^j / (j + k)!

(A strictly triangular, so nilpotent of index <= n; A = 0 is the case n = 1) is formed in
``fractions.Fraction`` from the exact binary64 inputs. ``error_upper`` is ``m 2^e`` with
``m`` in [1/2, 1) rounded up, so ``m 2^e >= ||e||_2`` exactly; a bound encloses the error
iff it is at least ``error_upper`` (both live on the same binade grid).

No production code is imported. Run from the repository root.
"""

import json
import math
import struct
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "fixtures/r4_transform_oracle_fixtures.json"
SUPPORTED_ORDER = 1000  # rodas5p_core::transform_bound::MAX_TRANSFORM_ORDER


def hexbits(x):
    return struct.pack(">d", x).hex()


def binary_split(x):
    m, e = math.frexp(x)
    return m, e


def binary_scale(m, e):
    if m == 0.0 or not math.isfinite(m):
        return m
    if e > 1100:
        return math.copysign(math.inf, m)
    if e < -1200:
        return math.copysign(0.0, m)
    value = m
    while e > 1000:
        value *= 2.0**1000
        e -= 1000
    if e < -1000:
        value *= 2.0**-1000
        e += 1000
    return value * 2.0**e


def binary_power(x, k):
    base_m, base_e = binary_split(abs(x))
    square_m, square_e = base_m, base_e
    acc_m, acc_e = 1.0, 0
    remaining = k
    while remaining > 0:
        if remaining & 1:
            m, e = binary_split(acc_m * square_m)
            acc_m = m
            acc_e += square_e + e
        remaining >>= 1
        if remaining > 0:
            m, e = binary_split(square_m * square_m)
            square_m = m
            square_e = 2 * square_e + e
    if acc_m == 1.0:
        return 0.5, 1
    return acc_m, acc_e


def times_power(value, scale, k):
    if k == 0:
        return value
    if value == 0.0 or scale == 0.0:
        negative = (math.copysign(1, value) < 0) ^ (math.copysign(1, scale) < 0 and k % 2 == 1)
        return -0.0 if negative else 0.0
    power_m, power_e = binary_power(scale, k)
    value_m, value_e = binary_split(abs(value))
    magnitude = binary_scale(value_m * power_m, value_e + power_e)
    negative = (value < 0) ^ (scale < 0 and k % 2 == 1)
    return -magnitude if negative else magnitude


def exp_upper(value):
    """(m, e) with m in [1/2, 1) the binary64 rounded up and m 2^e >= value >= 0."""
    if value == 0:
        return 0.0, 0
    e = value.numerator.bit_length() - value.denominator.bit_length()
    while value / F(2) ** e >= 1:
        e += 1
    while value / F(2) ** e < F(1, 2):
        e -= 1
    scaled = value / F(2) ** e
    m = float(scaled)
    if F(m) < scaled:
        m = math.nextafter(m, math.inf)
    assert F(1, 2) <= F(m) <= 1
    return m, e


def norm_upper(vector):
    square = sum(x * x for x in vector)
    if square == 0:
        return 0.0, 0
    # sqrt of a Fraction, rounded up: integer sqrt on a fine scale.
    shift = 400
    scaled = square * F(2) ** (2 * shift)
    root = math.isqrt(scaled.numerator // scaled.denominator) + 1
    upper = F(root, 2**shift)
    assert upper * upper >= square
    m, e = exp_upper(upper)
    # Step down to the smallest m on this binade with (m 2^e)^2 >= square.
    while True:
        lower = math.nextafter(m, 0.0)
        if lower < 0.5 or (F(lower) * F(2) ** e) ** 2 < square:
            break
        m = lower
    assert (F(m) * F(2) ** e) ** 2 >= square
    return m, e


def case(name, rows, h, vectors, stored_override=None):
    n = len(rows)
    a = [[F(x) for x in row] for row in rows]
    stored = [[times_power(b, h, k) for b in vector] for k, vector in enumerate(vectors)]
    if stored_override is not None:
        stored = stored_override
    hf = F(h)
    error = [F(0)] * n
    for k, (vector, weights) in enumerate(zip(vectors, stored)):
        delta = [hf**k * F(b) - F(w) for b, w in zip(vector, weights)]
        term = delta
        for j in range(max(n, 1)):
            for i in range(n):
                error[i] += term[i] / math.factorial(j + k)
            term = [hf * sum(a[i][l] * term[l] for l in range(n)) for i in range(n)]
        assert all(x == 0 for x in term), "matrix is not nilpotent of index <= n"
    m, e = norm_upper(error)
    return dict(
        id=name,
        matrix=[[hexbits(x) for x in row] for row in rows],
        h=hexbits(h),
        vectors=[[hexbits(x) for x in vector] for vector in vectors],
        stored=[[hexbits(x) for x in weights] for weights in stored],
        stored_is_source=stored_override is None,
        error_upper=dict(mantissa=m, exponent=e),
    )


def permuted(rows, vectors, perm):
    n = len(rows)
    new_rows = [[rows[perm[i]][perm[j]] for j in range(n)] for i in range(n)]
    new_vectors = [[vector[perm[i]] for i in range(n)] for vector in vectors]
    return new_rows, new_vectors


def transposed(rows):
    return [list(column) for column in zip(*rows)]


def main():
    cases = []
    # ARITH-DEV-01: the subunit nilpotent witness and its permutations/transposes.
    witness = [[0.0, 0.25], [0.0, 0.0]]
    vectors = [[0.0, 0.0], [0.0, 0.0], [0.0, 1.0]]
    for label, rows, vecs in [
        ("subunit_nilpotent", witness, vectors),
        ("subunit_nilpotent_swapped", *permuted(witness, vectors, [1, 0])),
        ("subunit_nilpotent_transposed", transposed(witness), vectors),
        ("subunit_nilpotent_transposed_swapped", *permuted(transposed(witness), vectors, [1, 0])),
    ]:
        cases.append(case(label, rows, 1000.1, vecs))
    chain = [[0.0, 0.25, 0.0], [0.0, 0.0, 0.125], [0.0, 0.0, 0.0]]
    chain_vectors = [[0.0, 0.0, 0.0], [0.0, 0.3, 0.0], [0.0, 0.0, 1.0]]
    for perm in [[0, 1, 2], [2, 1, 0], [1, 2, 0], [0, 2, 1]]:
        rows, vecs = permuted(chain, chain_vectors, perm)
        cases.append(case(f"chain3_perm{''.join(map(str, perm))}", rows, 777.7, vecs))
        cases.append(case(f"chain3_perm{''.join(map(str, perm))}_transposed", transposed(rows), 777.7, vecs))
    # The R3 lost-weight witness (h^2 b2 underflows) for both signs of h.
    for h in [1.0e-8, -1.0e-8]:
        cases.append(
            case(
                f"r3_lost_weight_{'neg' if h < 0 else 'pos'}",
                [[0.0, 1.0e308], [0.0, 0.0]],
                h,
                [[1.0e-300, 0.0], [0.0, 0.0], [0.0, 1.0e-310]],
            )
        )
    # ARITH-DEV-03: a signed stored order-0 weight (external input, not the source's).
    cases.append(case("signed_stored_order0", [[0.0]], 1.0, [[1.0]], stored_override=[[-1.0]]))
    cases.append(case("unsigned_stored_order0", [[0.0]], 1.0, [[1.0]], stored_override=[[0.5]]))
    # ARITH-DEV-02: A = 0, h = 1.1, b_p = [1] for every supported order p.
    orders = []
    for p in range(SUPPORTED_ORDER + 1):
        stored = times_power(1.0, 1.1, p)
        error = abs(F(1.1) ** p - F(stored)) / math.factorial(p)
        m, e = exp_upper(error)
        orders.append(dict(order=p, stored=hexbits(stored), error_upper=dict(mantissa=m, exponent=e)))
    OUT.write_text(
        json.dumps(
            dict(
                schema="vigilode-r4-transform-oracle-v1",
                generator="tools/r4_export_transform_oracle_fixtures.py",
                supported_order=SUPPORTED_ORDER,
                cases=cases,
                zero_operator_h=hexbits(1.1),
                zero_operator_orders=orders,
            ),
            indent=1,
        )
        + "\n"
    )
    print(f"{len(cases)} cases, {len(orders)} orders -> {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
