#!/usr/bin/env python3
"""Independent exact-rational residual oracle for represented packet curves."""

from fractions import Fraction
import json
import sys


def f64(value):
    return Fraction.from_float(float(value))


def add(a, b):
    return (a[0] + b[0], a[1] + b[1])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def mul(a, b):
    return (a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0])


def scale(a, factor):
    return (a[0] * factor, a[1] * factor)


def product(left, right):
    output = [(Fraction(0), Fraction(0)) for _ in range(len(left) + len(right) - 1)]
    for i, a in enumerate(left):
        for j, b in enumerate(right):
            output[i + j] = add(output[i + j], mul(a, b))
    return output


def complex_coefficients(raw):
    return [(f64(pair[0]), f64(pair[1])) for pair in raw]


def verify(payload):
    weights = [f64(value) for value in payload["weights"]]
    xi = [f64(value) for value in payload["xi"]]
    width = f64(payload["width"])
    chi = f64(payload["chi"])
    horizon = f64(payload["horizon"])
    modes = [complex_coefficients(mode) for mode in payload["modes"]]
    candidate = complex_coefficients(payload["candidate"])
    zero = (Fraction(0), Fraction(0))
    mode_degree = len(modes[0]) - 1
    assert all(len(mode) == mode_degree + 1 for mode in modes)
    assert len(weights) == len(xi) == len(modes)
    if payload["require_candidate_difference"]:
        assert sum(weights, Fraction(0)) != 1, "fixture must preserve exact stored weight sum"
        assert any(abs(value) > 1 for value in xi), "fixture must retain admitted spectral support"

    mean = []
    for q in range(mode_degree + 1):
        value = (Fraction(0), Fraction(0))
        for weight, mode in zip(weights, modes):
            value = add(value, scale(mode[q], weight))
        mean.append(value)

    conjugate = [(re, -im) for re, im in mean]
    mean_square = product(mean, conjugate)
    cubic = product(mean_square, mean)
    forcing = [(-chi * im, chi * re) for re, im in cubic]
    residual_stride = 3 * mode_degree + 1
    assert len(forcing) == residual_stride

    if payload["require_candidate_difference"]:
        differs_from_candidate = False
        for q, exact_mean in enumerate(mean):
            represented_candidate = candidate[q] if q < len(candidate) else zero
            differs_from_candidate |= exact_mean != represented_candidate
        assert differs_from_candidate, "fixture must distinguish candidate mean from reconstructed mean"

    checked = 0
    residual_real_sums = [Fraction(0) for _ in modes]
    for check in payload["evaluation_checks"]:
        tau = f64(check["time"]) / horizon
        for mode_index, mode in enumerate(modes):
            exact_value = (Fraction(0), Fraction(0))
            power = Fraction(1)
            for coefficient in mode:
                exact_value = add(exact_value, scale(coefficient, power))
                power *= tau
            for component, exact_component in (("re", exact_value[0]), ("im", exact_value[1])):
                lo, hi = check["modes"][mode_index][component]
                assert f64(lo) <= exact_component <= f64(hi), (
                    f"time={check['time']} mode={mode_index} component={component} "
                    f"exact={exact_component} enclosure=[{lo!r},{hi!r}]"
                )
                checked += 1
        exact_mean = (Fraction(0), Fraction(0))
        power = Fraction(1)
        for coefficient in mean:
            exact_mean = add(exact_mean, scale(coefficient, power))
            power *= tau
        for component, exact_component in (("re", exact_mean[0]), ("im", exact_mean[1])):
            lo, hi = check["mean"][component]
            assert f64(lo) <= exact_component <= f64(hi), (
                f"time={check['time']} mean component={component} "
                f"exact={exact_component} enclosure=[{lo!r},{hi!r}]"
            )
            checked += 1

    for mode_index, (mode, mode_frequency) in enumerate(zip(modes, xi)):
        omega = width * mode_frequency
        intervals = payload["residuals"][mode_index]
        assert len(intervals) == residual_stride
        for q in range(residual_stride):
            derivative = scale(mode[q + 1], Fraction(q + 1, 1) / horizon) if q < mode_degree else zero
            linear = mul((Fraction(0), omega), mode[q]) if q <= mode_degree else zero
            rhs = add(linear, forcing[q])
            exact = sub(derivative, rhs)
            for component, exact_value in (("re", exact[0]), ("im", exact[1])):
                lo, hi = intervals[q][component]
                assert f64(lo) <= exact_value <= f64(hi), (
                    f"mode={mode_index} coefficient={q} component={component} "
                    f"exact={exact_value} enclosure=[{lo!r},{hi!r}]"
                )
                checked += 1
            residual_real_sums[mode_index] += abs(exact[0])

    mean_real_sum = sum((abs(coefficient[0]) for coefficient in mean), Fraction(0))
    mean_bound = f64(payload["bounds"]["mean_magnitude_sup"][1])
    assert mean_bound >= mean_real_sum, "mean coefficient-sum bound misses exact real projection"
    weighted_real_square = sum(
        (weight * real_sum * real_sum for weight, real_sum in zip(weights, residual_real_sums)),
        Fraction(0),
    )
    residual_bound = f64(payload["bounds"]["weighted_residual_sup"][1])
    assert residual_bound * residual_bound >= weighted_real_square, (
        "weighted residual coefficient-sum bound misses exact real projection"
    )
    return checked


if __name__ == "__main__":
    print(f"exact Fraction residual and evaluation enclosures verified: {verify(json.load(sys.stdin))} components")
