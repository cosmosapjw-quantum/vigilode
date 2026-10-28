#!/usr/bin/env python3
"""Independent rational Taylor enclosure for exact binary64 equal-frequency flows."""
from fractions import Fraction as F
from math import factorial
import json
import sys


def q(x):
    return F.from_float(float(x))


def pair(raw):
    return q(raw[0]), q(raw[1])


def add(a, b):
    return a[0] + b[0], a[1] + b[1]


def sub(a, b):
    return a[0] - b[0], a[1] - b[1]


def scale(a, s):
    return a[0] * s, a[1] * s


def trig(x):
    # Taylor through sin degree 33 and cos degree 32. The next Taylor
    # remainders are bounded by |x|^35/35! and |x|^34/34! respectively.
    assert abs(x) <= 1
    sine = sum(((-1) ** k * x ** (2 * k + 1) / factorial(2 * k + 1) for k in range(17)), F(0))
    cosine = sum(((-1) ** k * x ** (2 * k) / factorial(2 * k) for k in range(17)), F(0))
    ds = abs(x) ** 35 / factorial(35)
    dc = abs(x) ** 34 / factorial(34)
    return (cosine - dc, cosine + dc), (sine - ds, sine + ds)


def rotate(z, phase):
    c, s = trig(phase)
    # General signed rational interval product, independent of Rust intervals.
    def product(v, box):
        terms = (v * box[0], v * box[1])
        return min(terms), max(terms)
    def plus(a, b):
        return a[0] + b[0], a[1] + b[1]
    def negative(a):
        return -a[1], -a[0]
    re = plus(product(z[0], c), negative(product(z[1], s)))
    im = plus(product(z[0], s), product(z[1], c))
    return re, im


def add_box(a, b):
    return (a[0][0] + b[0][0], a[0][1] + b[0][1]), (a[1][0] + b[1][0], a[1][1] + b[1][1])


def encloses_disk(raw_center, raw_radius, exact):
    center = pair(raw_center)
    radius = q(raw_radius)
    assert radius >= 0
    dr = max(abs(exact[0][0] - center[0]), abs(exact[0][1] - center[0]))
    di = max(abs(exact[1][0] - center[1]), abs(exact[1][1] - center[1]))
    assert dr * dr + di * di <= radius * radius, (dr, di, radius)


def verify(payload):
    t = q(payload["time"])
    chi = q(payload["chi"])
    omega = q(payload["width"]) * q(payload["xi"])
    weights = [q(w) for w in payload["weights"]]
    initial = [pair(z) for z in payload["initial"]]
    s = sum(weights, F(0))
    m0 = (sum((w * z[0] for w, z in zip(weights, initial)), F(0)),
          sum((w * z[1] for w, z in zip(weights, initial)), F(0)))
    m02 = m0[0] * m0[0] + m0[1] * m0[1]
    p0 = omega * t
    p1 = (omega + chi * s * m02) * t
    m_exact = rotate(m0, p1)
    encloses_disk(payload["mean"], payload["mean_radius"], m_exact)
    for j, z in enumerate(initial):
        shifted = sub(z, scale(m0, 1 / s))
        exact = add_box(rotate(shifted, p0), rotate(scale(m0, 1 / s), p1))
        encloses_disk(payload["modes"][j], payload["mode_radii"][j], exact)
    return len(initial) + 1


if __name__ == "__main__":
    count = sum(verify(case) for case in json.load(sys.stdin)["cases"])
    print(f"Fraction Taylor disk containments: {count}")
