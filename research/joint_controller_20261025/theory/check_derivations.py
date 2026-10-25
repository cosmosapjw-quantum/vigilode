#!/usr/bin/env python3
"""Exact-rational checks of the proved finite identities; no solver benchmark."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import random
import sys


def run():
    rng = random.Random(20261025)
    checks = {}
    count = 0
    for n in range(1, 9):
        for _ in range(50):
            a = [[F(rng.randrange(-12, 13), 4) if j < i else F(0)
                  for j in range(n)] for i in range(n)]
            b = [F(rng.randrange(-10, 11), 8) for _ in range(n)]
            u = []
            for i in range(n):
                u.append(b[i] + sum(a[i][j] * u[j] for j in range(i)))
            candidate = [u[i] + F(rng.randrange(-20, 21), 1_000_000)
                         for i in range(n)]
            rho = [abs(candidate[i] - b[i] - sum(a[i][j] * candidate[j]
                    for j in range(i))) for i in range(n)]
            bound = []
            for i in range(n):
                bound.append(rho[i] + sum(abs(a[i][j]) * bound[j]
                             for j in range(i)))
                assert abs(candidate[i] - u[i]) <= bound[i]
            w = [F(rng.randrange(0, 7), 3) for _ in range(n)]
            sigma = list(w)
            for i in reversed(range(n)):
                sigma[i] += sum(abs(a[j][i]) * sigma[j]
                                for j in range(i + 1, n))
            assert sum(w[i] * bound[i] for i in range(n)) == sum(
                sigma[i] * rho[i] for i in range(n))
            positive = [i for i in range(n) if sigma[i] > 0]
            if positive:
                tau = F(1, 100)
                allowance = {i: tau / (len(positive) * sigma[i])
                             for i in positive}
                assert sum(sigma[i] * allowance[i] for i in positive) == tau
            count += 1
    checks["triangular_bound_and_adjoint_exact"] = {"cases": count, "pass": True}

    s = 8
    saturated = [F(0)] * s
    for i in range(s):
        saturated[i] = F(1, 10**8) + (2 * saturated[i - 1] if i else 0)
    assert saturated[-1] == F(255, 10**8)
    checks["noncontractive_saturating_example"] = {
        "stages": s, "subdiagonal": 2,
        "last_error_exact": str(saturated[-1]), "pass": True}

    # Nonlinear strict-causal maps; exact finite propagation without contraction.
    count = 0
    for n in range(1, 6):
        for _ in range(15):
            base = [F(rng.randrange(-3, 4), 16) for _ in range(n)]
            a = [[F(rng.randrange(-2, 3), 16) if j < i else F(0)
                  for j in range(n)] for i in range(n)]
            target = []
            for i in range(n):
                target.append(base[i] + sum(a[i][j] * target[j]**2
                                           for j in range(i)))
            u = [F(1, 2)] * n
            for k in range(1, n + 1):
                u = [base[i] + sum(a[i][j] * u[j]**2 for j in range(i))
                     for i in range(n)]
                assert u[:k] == target[:k]
            assert u == target
            count += 1
    checks["nonlinear_finite_causal_propagation"] = {"cases": count, "pass": True}

    count = 0
    for degree in range(1, 9):
        for _ in range(50):
            q = [F(rng.randrange(-9, 10), 7) for _ in range(degree)]
            q.append(F(rng.choice([-3, -2, -1, 1, 2, 3])))
            q1 = sum(q)
            if not q1:
                continue
            p = [sum(q[j] for j in range(k + 1, degree + 1)) / q1
                 for k in range(degree)]
            lhs = [p[0]] + [p[k] - p[k - 1] for k in range(1, degree)] + [-p[-1]]
            rhs = [F(1) - q[0] / q1] + [-q[k] / q1 for k in range(1, degree + 1)]
            assert lhs == rhs
            count += 1
    checks["quotient_polynomial_identity"] = {"cases": count, "pass": True}

    count = switches = 0
    for _ in range(3000):
        la, lb = F(rng.randrange(1, 100), 10), F(rng.randrange(1, 100), 10)
        ua, ub = la + F(rng.randrange(10), 10), lb + F(rng.randrange(10), 10)
        n, t, eta = rng.randrange(1, 20), F(rng.randrange(20), 10), F(1, 10)
        guard = t + n * ua <= (1 - eta) * n * lb
        ca, cb = (la + ua) / 2, (lb + ub) / 2
        if guard:
            assert t + n * ca <= (1 - eta) * n * cb
            switches += 1
        count += 1
    checks["cost_interval_sufficiency"] = {"cases": count, "switches": switches, "pass": True}

    eps, seed, productive, probe = F(1, 20), F(3), F(0), F(0)
    admitted = 0
    for _ in range(1000):
        productive += F(rng.randrange(1, 15), 10)
        credit = seed + eps * productive - probe
        reserve = F(rng.randrange(1, 40), 10)
        if reserve <= credit:
            actual = reserve * F(rng.randrange(1, 11), 10)
            probe += actual
            admitted += 1
        assert probe <= seed + eps * productive
    checks["prepaid_probe_prefix_invariant"] = {
        "prefixes": 1000, "admitted": admitted, "pass": True,
        "productive_exact": str(productive), "probe_exact": str(probe)}

    count = 0
    for _ in range(500):
        alpha, e, gamma, target = F(rng.randrange(0, 20), 2), F(rng.randrange(1, 30), 2), F(1, 4), F(3, 4)
        if e > target * alpha:
            hmax = target / (gamma * (e - target * alpha))
            for multiplier in [F(1, 2), F(1), F(2)]:
                h = hmax * multiplier
                eta = h * gamma * e / (1 + h * gamma * alpha)
                assert (eta <= target) == (multiplier <= 1)
        else:
            for h in [F(1, 10000), F(1), F(10**8)]:
                assert h * gamma * e / (1 + h * gamma * alpha) <= target
        count += 1
    checks["damping_structure_step_radius"] = {"cases": count, "pass": True}
    return {"schema": "vigilode.joint-theory-checks.v1", "status": "pass_scoped",
            "method": "Python Fraction exact rational arithmetic; finite illustrative cases",
            "seed": 20261025, "python": sys.version.split()[0], "checks": checks,
            "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}


if __name__ == "__main__":
    result = run()
    text = json.dumps(result, indent=2) + "\n"
    if len(sys.argv) == 2:
        Path(sys.argv[1]).write_text(text)
    else:
        print(text, end="")
