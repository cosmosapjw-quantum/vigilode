#!/usr/bin/env python3
"""Exact Fraction ODE-Taylor reference for the frozen unequal-frequency A01 input."""
from fractions import Fraction as F
from math import factorial
import hashlib
import json
import sys


def q(x):
    return F.from_float(float(x))


def complex_pair(raw):
    return q(raw[0]), q(raw[1])


ZERO = (F(0), F(0))


def add(a, b):
    return a[0] + b[0], a[1] + b[1]


def sub(a, b):
    return a[0] - b[0], a[1] - b[1]


def mul(a, b):
    return a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]


def scale(a, c):
    return a[0] * c, a[1] * c


def iconj(a):
    return a[0], -a[1]


def irotate(a):
    return -a[1], a[0]


def magnitude_l1(a):
    return abs(a[0]) + abs(a[1])


def polynomial_product(a, b):
    output = [ZERO for _ in range(len(a) + len(b) - 1)]
    for j, left in enumerate(a):
        for k, right in enumerate(b):
            output[j + k] = add(output[j + k], mul(left, right))
    return output


def cubic_coefficient(mean, n):
    result = ZERO
    for j in range(n + 1):
        for k in range(n - j + 1):
            result = add(result, mul(mul(mean[j], iconj(mean[k])), mean[n - j - k]))
    return result


def reference(initial, xi, weights, width, chi, h, order):
    n_modes = len(initial)
    coefficients = [[initial[j]] for j in range(n_modes)]
    mean = [sum_weighted((coefficients[j][0] for j in range(n_modes)), weights)]
    for n in range(order):
        force = scale(irotate(cubic_coefficient(mean, n)), chi) if chi else ZERO
        next_modes = []
        for j in range(n_modes):
            linear = scale(irotate(coefficients[j][n]), width * xi[j])
            next_modes.append(scale(add(linear, force), F(1, n + 1)))
        for j, value in enumerate(next_modes):
            coefficients[j].append(value)
        mean.append(sum_weighted(next_modes, weights))
    energy = sum((w * (z[0] * z[0] + z[1] * z[1]) for w, z in zip(weights, initial)), F(0))
    assert energy <= min(weights), "initial energy leaves unit mode tube"
    for mode in coefficients:
        assert sum((magnitude_l1(c) * h ** k for k, c in enumerate(mode)), F(0)) <= 1, "candidate coefficient tube"
    mean_cubic = polynomial_product(polynomial_product(mean, [iconj(x) for x in mean]), mean)
    assert len(mean_cubic) == 3 * order + 1
    rho = F(0)
    zero_count = 0
    for j, mode in enumerate(coefficients):
        sum_abs = F(0)
        for k in range(3 * order + 1):
            derivative = scale(mode[k + 1], F(k + 1)) if k < order else ZERO
            linear = scale(irotate(mode[k]), width * xi[j]) if k <= order else ZERO
            nonlinear = scale(irotate(mean_cubic[k]), chi)
            residual = sub(derivative, add(linear, nonlinear))
            if k < order:
                assert residual == ZERO, ("nonzero Taylor residual", j, k)
                zero_count += 1
            sum_abs += magnitude_l1(residual) * h ** k
        rho = max(rho, sum_abs)
    return coefficients, rho, zero_count, energy


def sum_weighted(values, weights):
    result = ZERO
    for value, weight in zip(values, weights):
        result = add(result, scale(value, weight))
    return result


def evaluate(coefficients, t):
    return [sum_poly(mode, t) for mode in coefficients]


def sum_poly(coefficients, t):
    result = ZERO
    for coefficient in reversed(coefficients):
        result = add(scale(result, t), coefficient)
    return result


def exp_plus(x):
    assert 0 <= x <= 1
    total = sum((x ** j / factorial(j) for j in range(81)), F(0))
    return total + (x ** 81 / factorial(81)) / (1 - x / 82)


def box(center, radius):
    return ((center[0] - radius, center[0] + radius),
            (center[1] - radius, center[1] + radius))


def boxes_intersect(a, b):
    return all(max(a[c][0], b[c][0]) <= min(a[c][1], b[c][1]) for c in (0, 1))


def disk_contains(raw_center, raw_radius, enclosure):
    center = complex_pair(raw_center)
    radius = q(raw_radius)
    if radius < 0:
        return False
    dr = max(abs(enclosure[0][0] - center[0]), abs(enclosure[0][1] - center[0]))
    di = max(abs(enclosure[1][0] - center[1]), abs(enclosure[1][1] - center[1]))
    return dr * dr + di * di <= radius * radius


def rational_trig(x):
    assert abs(x) <= 1
    cosine = sum(((-1) ** j * x ** (2 * j) / factorial(2 * j) for j in range(17)), F(0))
    sine = sum(((-1) ** j * x ** (2 * j + 1) / factorial(2 * j + 1) for j in range(17)), F(0))
    dc = abs(x) ** 34 / factorial(34)
    ds = abs(x) ** 35 / factorial(35)
    return (cosine - dc, cosine + dc), (sine - ds, sine + ds)


def interval_scale(a, s):
    terms = (a[0] * s, a[1] * s)
    return min(terms), max(terms)


def trig_orbit(z, phase):
    c, s = rational_trig(phase)
    r1, r2 = interval_scale(c, z[0]), interval_scale(s, -z[1])
    i1, i2 = interval_scale(s, z[0]), interval_scale(c, z[1])
    return ((r1[0] + r2[0], r1[1] + r2[1]),
            (i1[0] + i2[0], i1[1] + i2[1]))


def verify(payload):
    assert payload["allocation_calibration_clone_allocations"] >= 3, "allocator positive control failed"
    xi = [q(x) for x in payload["xi"]]
    weights = [q(w) for w in payload["weights"]]
    center_initial = [complex_pair(z) for z in payload["initial_center"]]
    width, h = q(payload["width"]), q(payload["horizon"])
    assert len(xi) == len(weights) == len(center_initial) == 3
    assert payload["carrier_metadata_only"] == 1.25 and payload["kappa"] == 0
    assert xi == [F(-3, 4), F(1, 8), 1 + F(1, 2 ** 49)]
    assert weights == [F(1, 8), F(3, 8), F(1, 2) + F(1, 2 ** 42)]
    assert h == F(1, 32)
    assert [q(t) for t in payload["times"]] == [h * j / 4 for j in range(5)]
    assert [q(arm["chi"]) for arm in payload["arms"]] == [F(0), F(3, 8), F(-3, 8), F(3, 8)]
    assert [arm["uncertain"] for arm in payload["arms"]] == [False, False, False, True]
    eta = q(payload["eta"])
    assert eta == F(1, 2 ** 30)
    expected_perturb = [(eta, F(0)), (F(0), -eta), (-eta, F(0))]
    total_disks = total_zeros = 0
    largest_eps40 = largest_eps48 = F(0)
    negative = None
    for arm_index, arm in enumerate(payload["arms"]):
        chi = q(arm["chi"])
        initial = [complex_pair(z) for z in arm["initial_true"]]
        expected = [add(z, d) for z, d in zip(center_initial, expected_perturb)] if arm_index == 3 else center_initial
        assert initial == expected, "initial-state identity"
        refs = {}
        for order in (40, 48):
            refs[order] = reference(initial, xi, weights, width, chi, h, order)
            total_zeros += refs[order][2]
        assert all(refs[40][0][j] == refs[48][0][j][:41] for j in range(3)), "shared coefficient identity"
        s = sum(weights, F(0))
        lipschitz = max(abs(width * x) for x in xi) + 6 * abs(chi) * s ** 3
        assert len(arm["outputs"]) == 5
        for output_index, output in enumerate(arm["outputs"]):
            t = q(output["time"])
            assert t == h * output_index / 4
            certified = {}
            for order, threshold in ((40, F(1, 2 ** 120)), (48, F(1, 2 ** 160))):
                modes, rho, _, _ = refs[order]
                eps = exp_plus(lipschitz * t) * t * rho
                assert eps <= threshold, ("reference threshold", arm_index, order, output_index)
                values = evaluate(modes, t)
                certified[order] = ([box(value, eps) for value in values],
                                    box(sum_weighted(values, weights), s * eps), eps)
            boxes40, mean40, eps40 = certified[40]
            boxes48, mean48, eps48 = certified[48]
            assert eps48 <= eps40, "reference radius monotonicity"
            largest_eps40, largest_eps48 = max(largest_eps40, eps40), max(largest_eps48, eps48)
            assert all(boxes_intersect(a, b) for a, b in zip(boxes40, boxes48))
            assert boxes_intersect(mean40, mean48)
            assert len(output["modes"]) == len(output["mode_radii"]) == 3
            for j in range(3):
                assert disk_contains(output["modes"][j], output["mode_radii"][j], boxes48[j]), ("mode disk", arm_index, output_index, j)
                total_disks += 1
            assert disk_contains(output["mean"], output["mean_radius"], mean48), ("mean disk", arm_index, output_index)
            total_disks += 1
            if chi == 0:
                trig_modes = [trig_orbit(z, width * x * t) for z, x in zip(initial, xi)]
                for j, trig_box in enumerate(trig_modes):
                    assert boxes_intersect(trig_box, boxes48[j]), "independent trig reference disagreement"
                    assert disk_contains(output["modes"][j], output["mode_radii"][j], trig_box), "trig disk"
            if arm_index == 1 and output_index == 4:
                moved = list(output["modes"][0]); moved[0] += 1.0
                assert not disk_contains(moved, output["mode_radii"][0], boxes48[0]), "moved center accepted"
                assert not disk_contains(output["modes"][0], 0.0, boxes48[0]), "zero radius accepted"
                negative = "both rejected"
    assert negative is not None
    return {"status": "PASS", "arms": 4, "orders": [40, 48], "requested_times": 5,
            "disk_checks": total_disks, "exact_zero_residual_coefficients": total_zeros,
            "allocation_calibration_clone_allocations": payload["allocation_calibration_clone_allocations"],
            "largest_epsilon40_float": float(largest_eps40),
            "largest_epsilon48_float": float(largest_eps48),
            "negative_endpoint_controls": negative}


if __name__ == "__main__":
    raw = open(sys.argv[1], "rb").read()
    payload = json.loads(raw)
    result = verify(payload)
    result["input_sha256"] = hashlib.sha256(raw).hexdigest()
    print(json.dumps(result, sort_keys=True))
