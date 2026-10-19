#!/usr/bin/env python3
"""Exact Fraction oracle for the emitted near-one PR101 quartic fixture."""

from fractions import Fraction
import math
import re
import sys


def f64(text: str) -> Fraction:
    return Fraction.from_float(float(text))


def emitted(log: str, name: str) -> str:
    match = re.search(rf"^{name}=(.*)$", log, re.MULTILINE)
    if match is None:
        raise AssertionError(f"focused log lacks {name}")
    return match.group(1)


def interval(text: str) -> tuple[Fraction, Fraction]:
    lo, hi = text.split(",")
    return f64(lo), f64(hi)


def interval_list(text: str) -> list[tuple[Fraction, Fraction]]:
    return [interval(item) for item in text.split(";")]


def add(x, y):
    return x[0] + y[0], x[1] + y[1]


def sub(x, y):
    return x[0] - y[1], x[1] - y[0]


def mul(x, y):
    corners = (x[0] * y[0], x[0] * y[1], x[1] * y[0], x[1] * y[1])
    return min(corners), max(corners)


def div(x, y):
    if y[0] <= 0 <= y[1]:
        raise AssertionError("oracle division interval contains zero")
    corners = (x[0] / y[0], x[0] / y[1], x[1] / y[0], x[1] / y[1])
    return min(corners), max(corners)


def scalar(value: Fraction):
    return value, value


def scale(x, value: Fraction):
    return mul(x, scalar(value))


def square(x):
    return mul(x, x)


def trig_interval(x: Fraction, which: str):
    polynomial = Fraction(0)
    if which == "sin":
        degrees = range(1, 33, 2)
    else:
        degrees = range(0, 33, 2)
    for degree in degrees:
        sign = -1 if ((degree - 1) // 2 if which == "sin" else degree // 2) % 2 else 1
        polynomial += sign * x**degree / factorial(degree)
    remainder = x**33 / factorial(33)
    return polynomial - remainder, polynomial + remainder


def trig_pair(x: Fraction):
    return trig_interval(x, "sin"), trig_interval(x, "cos")


def contains(box, target):
    return box[0] <= target[0] and target[1] <= box[1]


def residual_coefficients(q, delta, a):
    sine0, cosine0 = trig_pair(a)
    sine = [sine0] + [scalar(Fraction(0)) for _ in range(24)]
    cosine = [cosine0] + [scalar(Fraction(0)) for _ in range(24)]
    for k in range(24):
        divisor = Fraction(k + 1)
        sine[k + 1] = div(mul(delta, cosine[k]), scalar(divisor))
        cosine[k + 1] = scale(div(mul(delta, sine[k]), scalar(divisor)), Fraction(-1))

    zero = scalar(Fraction(0))
    error = []
    for k in range(25):
        qk = q[k] if k < 5 else zero
        error.append(sub(qk, sine[k]))
    residual = [zero for _ in range(49)]
    for k in range(25):
        residual[k] = sub(scale(error[k], Fraction(1_000_000)), cosine[k])
    for k in range(1, 5):
        derivative = div(scale(q[k], Fraction(k)), delta)
        residual[k - 1] = add(residual[k - 1], derivative)
    for left in range(25):
        for right in range(25):
            product = scale(mul(error[left], error[right]), Fraction(10))
            residual[left + right] = sub(residual[left + right], product)
    return residual, error


def coefficient_bound(coefficients):
    return sum(max(abs(lo), abs(hi)) for lo, hi in coefficients)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: pr101_fraction_oracle.py <focused-test-log>")
    log = open(sys.argv[1], encoding="utf-8").read()
    q = interval_list(emitted(log, "PR101_NEAR1_Q"))
    rust_residual = interval_list(emitted(log, "PR101_NEAR1_RPOLY"))
    delta = interval(emitted(log, "PR101_NEAR1_DELTA"))
    a = f64(emitted(log, "PR101_NEAR1_A"))
    announced_h = f64(emitted(log, "PR101_NEAR1_H"))
    if len(q) != 5 or len(rust_residual) != 49:
        raise AssertionError("wrong quartic or residual coefficient count")
    if delta[0] <= 0 or delta[0] != delta[1] or delta[0] != announced_h:
        raise AssertionError("nonpositive exact segment length")
    if a != Fraction(1023, 1024):
        raise AssertionError("the Fraction fixture does not use the frozen dyadic start")

    target_residual, error = residual_coefficients(q, delta, a)
    for index, (rust, exact) in enumerate(zip(rust_residual, target_residual)):
        if not contains(rust, exact):
            raise AssertionError(
                f"Rust residual coefficient {index} {rust} does not contain Fraction interval {exact}"
            )

    h = delta[1]
    remainder = h**25 / factorial(25)
    exact_residual_upper = (
        coefficient_bound(target_residual)
        + Fraction(1_000_000) * remainder
        + remainder
        + 20 * coefficient_bound(error) * remainder
        + 10 * remainder**2
    )
    rust_residual_upper = f64(emitted(log, "PR101_NEAR1_RESIDUAL"))
    if rust_residual_upper < exact_residual_upper:
        raise AssertionError("outward residual upper is below the exact Fraction bound")

    t = f64(emitted(log, "PR101_NEAR1_T"))
    y = f64(emitted(log, "PR101_NEAR1_Y"))
    sine, cosine = trig_pair(t)
    displacement = sub(scalar(y), sine)
    exact_rhs = add(
        add(scale(displacement, Fraction(-1_000_000)), cosine),
        scale(square(displacement), Fraction(10)),
    )
    rust_rhs = interval(emitted(log, "PR101_NEAR1_RHS"))
    if not contains(rust_rhs, exact_rhs):
        raise AssertionError(f"Rust RHS {rust_rhs} does not contain Fraction RHS {exact_rhs}")

    center, radius = (f64(value) for value in emitted(log, "PR101_NEAR1_OUTPUT").split(","))
    exact_sine = trig_interval(t, "sin")
    if center - radius > exact_sine[0] or center + radius < exact_sine[1]:
        raise AssertionError("returned local output enclosure misses Fraction sin interval")

    # Cancellation control: binary64 rounds cos(2^-30) to 1, while the exact
    # rational Taylor interval proves 1-cos(x)>0.
    tiny = Fraction(1, 2**30)
    exact_one_minus_cos = sub(scalar(Fraction(1)), trig_interval(tiny, "cos"))
    naive = 1.0 - math.cos(float(tiny))
    if not (naive == 0.0 and exact_one_minus_cos[0] > 0):
        raise AssertionError("cancellation negative control did not expose the naive zero")

    print(
        "PASS fraction_coefficients=49 rhs_and_output_enclosures=PASS "
        f"residual_upper={float(exact_residual_upper):.17e} cancellation_control=PASS"
    )


def factorial(n: int) -> int:
    value = 1
    for k in range(2, n + 1):
        value *= k
    return value


if __name__ == "__main__":
    main()
