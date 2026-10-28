#!/usr/bin/env python3
"""Export the independent oracle fixtures of the joint phi polynomial actions (re-audit R3,
POLY-01/POLY-02) to ``fixtures/r3_polynomial_oracle_fixtures.json``.

* ``actions``: the ten inputs of the R3 probe
  (``research/adversarial_reaudit_20261001_r3/polynomial/joint_polynomial_probe.py``) with its
  Decimal-120 reference (exact orthogonal Q, scalar phi recurrence), all binary64 values as
  hex, plus the probe's Chebyshev and Laguerre degrees.
* ``chebyshev_coefficients`` / ``laguerre_coefficients``: c_{n,k} by 40-digit mpmath
  quadrature of the defining integrals (Bessel ``besseli`` and the Laguerre generating
  function), not by the series the Rust code uses.

Requires numpy and mpmath. Run from the repository root.
"""

import json
import math
from pathlib import Path

import mpmath
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
PROBE = ROOT / "research/adversarial_reaudit_20261001_r3/polynomial/joint_polynomial_probe.py"
OUT = ROOT / "fixtures/r3_polynomial_oracle_fixtures.json"


def load_probe():
    source = PROBE.read_text()
    namespace = {}
    # The SciPy-free helpers only: the reference and the degree rules.
    head = source.split("def admissible")[0]
    head = head.replace("import scipy\n", "").replace("from scipy.special import ive\n", "")
    exec(head, namespace)
    exec("def degree_cheb" + source.split("def degree_cheb")[1].split("def coefficients")[0], namespace)
    return namespace


def actions(probe):
    hadamard = np.array([[1, 1, 1, 1], [1, -1, 1, -1], [1, 1, -1, -1], [1, -1, -1, 1]], dtype=float) / 2
    families = [
        ("diagonal24", -np.geomspace(0.1, 100.0, 24), np.eye(24), [0.001, 0.01, 0.1, 1.0]),
        ("hadamard4", -np.array([0.125, 1.0, 8.0, 64.0]), hadamard, [1e-12, 0.01, 0.1, 1.0]),
        ("semidefinite4", -np.array([0.0, 0.125, 1.0, 8.0]), hadamard, [0.1, 1.0]),
    ]
    rows = []
    for name, eigs, q, steps in families:
        a = (q * eigs) @ q.T
        n = len(eigs)
        w = np.stack([np.cos(np.arange(n) * (0.23 + 0.11 * k)) + 0.2 * (k + 1) for k in range(5)], axis=1)
        w = w / np.linalg.norm(w, axis=0) * np.array([1.0, -1.0, 0.5, -0.25, 0.125])
        for h in steps:
            columns, fused = probe["reference"](eigs, q, w, h)
            lam, rho = -float(max(eigs)), -float(min(eigs))
            factor = sum(np.linalg.norm(w[:, k]) / math.factorial(k) for k in range(5))
            cheb_degree, _ = probe["degree_cheb"](h, lam, rho, factor)
            lag = probe["degree_lag"](h, rho, factor)
            rows.append(
                dict(
                    family=name,
                    h=float(h).hex(),
                    lam=lam.hex(),
                    rho=rho.hex(),
                    A=[[float(x).hex() for x in row] for row in a],
                    W=[[float(x).hex() for x in row] for row in w],
                    ref_columns=[[float(x).hex() for x in row] for row in columns],
                    ref_fused=[float(x).hex() for x in fused],
                    probe_chebyshev_degree=cheb_degree,
                    probe_laguerre_degree=lag[0],
                    probe_laguerre_scale=lag[2],
                )
            )
    return rows


def chebyshev_coefficients():
    mpmath.mp.dps = 40
    rows = []
    for h, lam, rho in [("0.75", "0.5", "8"), ("0.0625", "0", "64"), ("1", "0.125", "8")]:
        hh, ll, rr = (mpmath.mpf(x) for x in (h, lam, rho))
        # The shift and half-width are exact in binary64 for these inputs.
        a = -hh * (rr + ll) / 2
        b = hh * (rr - ll) / 2
        for n in (0, 1, 2, 5, 10, 17):
            weight = 1 if n == 0 else 2
            values = [weight * mpmath.exp(a) * mpmath.besseli(n, b)]
            for k in range(1, 5):
                integral = mpmath.quad(
                    lambda u: mpmath.exp(u * a) * mpmath.besseli(n, u * b) * (1 - u) ** (k - 1), [0, 1]
                )
                values.append(weight * integral / mpmath.factorial(k - 1))
            rows.append(dict(h=h, lam=lam, rho=rho, n=n, c=[mpmath.nstr(v, 30) for v in values]))
    return rows


def laguerre_coefficients():
    mpmath.mp.dps = 40
    rows = []
    for h, rho, scale in [("0.3", "8", "4"), ("0.01", "100", "16"), ("1", "8", "1")]:
        hh, rr, ll = (mpmath.mpf(x) for x in (h, rho, scale))
        # beta = rho / L is exact in binary64 for these inputs.
        a = hh * (rr / ll)
        q = a / (1 + a)
        for n in (0, 1, 3, 9, 20):
            values = [(1 - q) * q**n]
            for k in range(1, 5):
                integral = mpmath.quad(
                    lambda u: (1 - u) ** (k - 1) / (1 + a * u) * (a * u / (1 + a * u)) ** n, [0, 1]
                )
                values.append(integral / mpmath.factorial(k - 1))
            rows.append(dict(h=h, rho=rho, scale=scale, n=n, c=[mpmath.nstr(v, 30) for v in values]))
    return rows


def main():
    probe = load_probe()
    payload = dict(
        schema="vigilode-r3-polynomial-oracle-fixtures-v1",
        source="tools/r3_export_polynomial_oracle_fixtures.py",
        observed_target=probe["OBSERVED_TARGET"],
        truncation_budget=probe["TRUNCATION_BUDGET"],
        actions=actions(probe),
        chebyshev_coefficients=chebyshev_coefficients(),
        laguerre_coefficients=laguerre_coefficients(),
    )
    OUT.write_text(json.dumps(payload, indent=1) + "\n")
    print(f"wrote {OUT.relative_to(ROOT)}: {len(payload['actions'])} actions")


if __name__ == "__main__":
    main()
