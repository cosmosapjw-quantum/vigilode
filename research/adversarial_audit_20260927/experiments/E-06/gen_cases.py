#!/usr/bin/env python3
"""E-06 case generator: matrices with ||A||_1 in {1e-3,1,1e3}, vectors with ||v||_2 in {1e-8,1,1e8}.
Types: random dense, nonnormal upper-triangular, stiff diagonal. Sizes 8 and 64. scale = 1 (A already scaled)."""
import json, numpy as np
rng = np.random.default_rng(20260927)
cases = []
for n in (8, 64):
    base = {}
    base["dense"] = rng.standard_normal((n, n))
    ut = np.triu(rng.standard_normal((n, n)), 1) * 5.0 + np.diag(-np.logspace(0, 3, n))
    base["uppertri"] = ut
    base["stiffdiag"] = np.diag(-np.logspace(0, 6, n))
    v0 = rng.standard_normal(n)
    v0 /= np.linalg.norm(v0)
    for typ, A0 in base.items():
        for anorm in (1e-3, 1.0, 1e3):
            A = A0 * (anorm / np.linalg.norm(A0, 1))
            for vnorm in (1e-8, 1.0, 1e8):
                v = v0 * vnorm
                cases.append({"name": f"n{n}_{typ}_A{anorm:g}_v{vnorm:g}", "n": n, "scale": 1.0,
                              "a": A.tolist(), "v": v.tolist(), "type": typ, "anorm": anorm, "vnorm": vnorm})
json.dump({"cases": cases}, open("cases.json", "w"))
print(len(cases), "cases")
