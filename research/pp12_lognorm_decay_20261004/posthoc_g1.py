#!/usr/bin/env python3
"""Post-hoc diagnostic (after the recorded run) for the G1 rows of research node
pp12_lognorm_decay_20261004 whose 50-digit check failed by < 1e-47: recompute the largest
eigenvalue of the exact 2x2 symmetric part in closed form (exact rationals, then a
100-digit square root) and compare with mu_up."""
import json
import struct
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

N = Path(__file__).resolve().parent
mp.mp.dps = 100


def fr(t):
    return F(struct.unpack(">d", bytes.fromhex(t))[0])


data = json.loads((N / "cases.json").read_text())
out = []
for c in data["cases"]:
    if len(c["v"]) != 2:
        continue
    a = [[fr(x) for x in r] for r in c["a"]]
    for key in ("chosen", "other"):
        x = c[key]
        d = [F(1), F(1)] if x["metric"] == "identity" else [fr(t) for t in c["osborne"]]
        s = [[(d[i] * a[i][j] / d[j] + d[j] * a[j][i] / d[i]) / 2 for j in range(2)] for i in range(2)]
        tr, det = s[0][0] + s[1][1], s[0][0] * s[1][1] - s[0][1] * s[1][0]
        disc = tr * tr / 4 - det
        lam = mp.mpf(tr.numerator) / (2 * tr.denominator) + mp.sqrt(mp.mpf(disc.numerator) / disc.denominator)
        mu = fr(x["mu_up"])
        exact_equal = disc == ((mu - tr / 2) ** 2) and mu - tr / 2 >= 0
        out.append({"label": c["label"], "certificate": key, "metric": x["metric"], "mu_up": str(mu),
                    "lambda_max_100_digits": mp.nstr(lam, 40), "mu_minus_lambda_100": mp.nstr(mp.mpf(mu.numerator) / mu.denominator - lam, 5),
                    "mu_equals_lambda_exactly": bool(exact_equal)})
(N / "POSTHOC_G1.json").write_text(json.dumps(out, indent=1) + "\n")
print(json.dumps(out, indent=1))
