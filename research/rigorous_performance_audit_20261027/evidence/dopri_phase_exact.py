"""Exact rational stability/dispersion audit of the source DOPRI5 tableau.

No ODE campaign is rerun. Fractions and truncated formal series avoid a CAS
dependency. The theorem applies to y'=i*omega*y at constant steps, and does
not establish a tolerance law for a nonlinear adaptive gyromotion run.
"""
from fractions import Fraction as F
import json
from pathlib import Path

ORDER = 10
A = [
    [],
    [F(1, 5)],
    [F(3, 40), F(9, 40)],
    [F(44, 45), -F(56, 15), F(32, 9)],
    [F(19372, 6561), -F(25360, 2187), F(64448, 6561), -F(212, 729)],
    [F(9017, 3168), -F(355, 33), F(46732, 5247), F(49, 176), -F(5103, 18656)],
    [F(35, 384), F(0), F(500, 1113), F(125, 192), -F(2187, 6784), F(11, 84)],
]
A = [row + [F(0)] * (7 - len(row)) for row in A]
B = A[6]
BE = [F(5179, 57600), F(0), F(7571, 16695), F(393, 640),
      -F(92097, 339200), F(187, 2100), F(1, 40)]


def stability(b):
    v = [F(1)] * 7
    out = [F(1)]
    for _ in range(7):
        out.append(sum(x * y for x, y in zip(b, v)))
        v = [sum(x * y for x, y in zip(row, v)) for row in A]
    return out + [F(0)] * (ORDER + 1 - len(out))


def multiply(a, b):
    return [sum(a[j] * b[k - j] for j in range(k + 1))
            for k in range(ORDER + 1)]


def log_series(r):
    u = r.copy()
    u[0] -= 1
    p = [F(1)] + [F(0)] * ORDER
    out = [F(0)] * (ORDER + 1)
    for k in range(1, ORDER + 1):
        p = multiply(p, u)
        out = [x + F((-1) ** (k + 1), k) * y for x, y in zip(out, p)]
    return out


def terms(p):
    return {str(k): str(c) for k, c in enumerate(p) if c}


def main():
    r = stability(B)
    re = stability(BE)
    lr = log_series(r)
    diff = [a - b for a, b in zip(r, re)]
    assert r[:7] == [F(1), F(1), F(1, 2), F(1, 6), F(1, 24), F(1, 120), F(1, 600)]
    assert all(c == 0 for c in r[7:])
    assert lr[1] == 1 and all(lr[k] == 0 for k in range(2, 6))
    result = {
        "status": "EXACT_RATIONAL_DERIVATION_PASS",
        "source": "crates/rodas5p-integrators/src/dopri5.rs DOPRI5_A/B/B_EMBEDDED",
        "baseline_commit": "3064d453091af1dec7f99f0c8afabd98b447ea34",
        "arithmetic": "Python standard-library Fraction; exact formal coefficients",
        "R": terms(r), "log_R": terms(lr), "R_minus_embedded": terms(diff),
        "regime": "scalar constant-frequency oscillator, constant small positive step; exact arithmetic",
        "conclusion": "No general adaptive physical phase-error-versus-rtol law is inferred.",
        "initial_attempt": "Default Python sympy import failed (package missing); replaced with exact standard-library arithmetic; no claim of an ODE run.",
    }
    output = Path(__file__).with_name("DOPRI_PHASE_EXACT.json")
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
