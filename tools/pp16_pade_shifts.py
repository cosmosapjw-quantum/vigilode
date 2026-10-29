#!/usr/bin/env python3
"""Pade [k/k] shifts and partial-fraction weights of e^z for research node PP16
(research/pp16_complex_shift_gain_20261004).

Q_k(z) = sum_j (2k-j)! k! / ((2k)! j! (k-j)!) (-z)^j and P_k(z) = Q_k(-z) are exact
rationals. The roots p_i of Q_k are found with mpmath at 50 digits and rounded to
binary64 (component-wise; conjugate pairs are rounded as one value and its
conjugate). The shifts gamma_i = 1/p_i are computed from the rounded p_i and rounded
to binary64. With a_i = P_k(p_i)/Q_k'(p_i) (exact-root residues),
r(z) = c0 + sum_i a_i/(z - p_i) = c0 + sum_i w_i/(1 - gamma_i z), w_i = -a_i/p_i,
c0 = (-1)^k; the w_i are rounded to binary64. The rounded shifts and weights are the
exact binary inputs of the Rust exporter.
"""

from __future__ import annotations

import argparse
import json
import struct
from fractions import Fraction
from math import factorial
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-pp16-pade-shifts-v1"


def hexf(x: float) -> str:
    return struct.pack(">d", float(x)).hex()


def coefficients(k: int):
    q = [Fraction(factorial(2 * k - j) * factorial(k),
                  factorial(2 * k) * factorial(j) * factorial(k - j)) * (-1) ** j
         for j in range(k + 1)]
    p = [abs(c) for c in q]
    return p, q


def evaluate(coeffs, z):
    return mp.fsum(mp.mpf(c.numerator) / c.denominator * z ** j for j, c in enumerate(coeffs))


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def pole(root, p_re: float, p_im: float, p_coeffs, dq):
    assert mp.re(root) > 0, "Pade denominator root not in the right half plane"
    gamma = 1 / mp.mpc(p_re, p_im)
    g_re, g_im = float(mp.re(gamma)), float(mp.im(gamma))
    residue = evaluate(p_coeffs, root) / evaluate(dq, root)
    weight = -residue / root
    w_re, w_im = float(mp.re(weight)), float(mp.im(weight))
    return {
        "p_re_hex": hexf(p_re), "p_im_hex": hexf(p_im),
        "gamma_re_hex": hexf(g_re), "gamma_im_hex": hexf(g_im),
        "weight_re_hex": hexf(w_re), "weight_im_hex": hexf(w_im),
        "p_50": mp.nstr(root, 50), "p": [repr(p_re), repr(p_im)],
        "gamma": [repr(g_re), repr(g_im)], "weight": [repr(w_re), repr(w_im)],
    }


def approximant(k: int):
    p_coeffs, q_coeffs = coefficients(k)
    roots = mp.polyroots([mp.mpf(c.numerator) / c.denominator for c in reversed(q_coeffs)],
                         maxsteps=500, extraprec=200)
    tiny = mp.mpf(10) ** -40
    for root in roots:
        assert abs(evaluate(q_coeffs, root)) < tiny
    real = sorted((mp.re(r) for r in roots if abs(mp.im(r)) < tiny))
    upper = sorted((r for r in roots if mp.im(r) > tiny), key=mp.im)
    assert len(real) + 2 * len(upper) == k, "root classification failed"
    dq = [j * c for j, c in enumerate(q_coeffs)][1:]
    poles = [pole(mp.mpc(r, 0), float(r), 0.0, p_coeffs, dq) for r in real]
    for r in upper:
        p_re, p_im = float(mp.re(r)), float(mp.im(r))
        # The conjugate member is the exact conjugate of the rounded root.
        poles.append(pole(r, p_re, p_im, p_coeffs, dq))
        poles.append(pole(mp.conj(r), p_re, -p_im, p_coeffs, dq))
        a, b = poles[-2], poles[-1]
        assert a["gamma_re_hex"] == b["gamma_re_hex"] and unhex(a["gamma_im_hex"]) == -unhex(b["gamma_im_hex"])
        assert a["weight_re_hex"] == b["weight_re_hex"] and unhex(a["weight_im_hex"]) == -unhex(b["weight_im_hex"])
    c0 = (-1) ** k
    checks = {}
    for label, z in (("z=0", mp.mpc(0)), ("z=-0.7", mp.mpc(-0.7)), ("z=0.3+0.2i", mp.mpc(0.3, 0.2)),
                     ("z=-5+3i", mp.mpc(-5, 3))):
        pade = evaluate(p_coeffs, z) / evaluate(q_coeffs, z)
        rounded = c0 + mp.fsum(
            mp.mpc(unhex(pl["weight_re_hex"]), unhex(pl["weight_im_hex"]))
            / (1 - mp.mpc(unhex(pl["gamma_re_hex"]), unhex(pl["gamma_im_hex"])) * z)
            for pl in poles)
        difference = abs(pade - rounded)
        assert difference < mp.mpf(10) ** -12, (label, difference)
        checks[label] = {"pade": mp.nstr(pade, 20), "rounded_partial_fraction_abs_diff": mp.nstr(difference, 5)}
    return {
        "k": k,
        "q_coefficients": [str(c) for c in q_coeffs],
        "p_coefficients": [str(c) for c in p_coeffs],
        "c0_re_hex": hexf(float(c0)), "c0_im_hex": hexf(0.0),
        "poles": poles,
        "checks": checks,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 50
    data = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps,
            "approximants": [approximant(3), approximant(6)]}
    args.output.write_text(json.dumps(data, indent=1) + "\n")
    print(json.dumps({a["k"]: [p["gamma"] for p in a["poles"]] for a in data["approximants"]}))


if __name__ == "__main__":
    main()
