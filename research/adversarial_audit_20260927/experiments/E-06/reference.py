#!/usr/bin/env python3
"""E-06 reference: phi_k(A) v for k=0..4 and expm(A) via one augmented (n+4)x(n+4) mpmath expm at 30 digits.
Compares against rust_out.json. Pre-registered FAIL rule: relative error (2-norm) > 1e3*eps (2.22e-13)."""
import json, sys, time
import numpy as np
import mpmath as mp
mp.mp.dps = 30
EPS = 2.220446049250313e-16
THRESH = 1e3 * EPS
cases = json.load(open("cases.json"))["cases"]
rust = {r["name"]: r for r in json.load(open("rust_out.json"))}
rows = []
t_start = time.time()
for c in cases:
    n = c["n"]; A = mp.matrix(c["a"]); v = mp.matrix(c["v"])
    p = 4
    aug = mp.zeros(n + p, n + p)
    for i in range(n):
        for j in range(n):
            aug[i, j] = A[i, j]
        aug[i, n] = v[i]
    for j in range(p - 1):
        aug[n + j, n + j + 1] = 1
    t0 = time.time()
    E = mp.expm(aug)
    Ev = (E[:n, :n]) * v  # exp(A) v (upper-left block of augmented expm equals expm(A))
    refs = [Ev] + [E[:n, n + k - 1] for k in range(1, p + 1)]
    Eexp_last = E[:n, n - 1]
    # Direct expm(A) for the k=0/expm checks (independent of augmentation).
    EA = mp.expm(A)
    EAv = EA * v
    wall = time.time() - t0
    r = rust[c["name"]]
    entry = {"name": c["name"], "n": n, "type": c["type"], "anorm": c["anorm"], "vnorm": c["vnorm"], "mp_wall_s": wall,
             "vnorm_ref_k": [], "relerr_k": [], "rust_error_k": r["phi_errors"], "aug_vs_direct_expm_v_relerr": None}
    # consistency of the two references
    num = mp.norm(Ev - EAv); den = mp.norm(EAv)
    entry["aug_vs_direct_expm_v_relerr"] = float(num / den) if den != 0 else None
    for k in range(5):
        ref = refs[k] if k > 0 else EAv
        refn = mp.norm(ref)
        entry["vnorm_ref_k"].append(float(refn))
        got = r["phi"][k]
        if got is None:
            entry["relerr_k"].append(None)
            continue
        g = mp.matrix(got)
        entry["relerr_k"].append(float(mp.norm(g - ref) / refn) if refn != 0 else float(mp.norm(g - ref)))
    if r["expm_v"] is not None:
        entry["expm_v_relerr"] = float(mp.norm(mp.matrix(r["expm_v"]) - EAv) / mp.norm(EAv))
        entry["expm_lastcol_relerr"] = float(mp.norm(mp.matrix(r["expm_last_col"]) - EA[:n, n - 1]) / mp.norm(EA[:n, n - 1]))
    else:
        entry["expm_v_relerr"] = None; entry["expm_lastcol_relerr"] = None; entry["expm_error"] = r["expm_error"]
    entry["fail_k"] = [(e is None) or (e > THRESH) for e in entry["relerr_k"]]
    rows.append(entry)
    print(f"{c['name']:36s} wall={wall:6.1f}s relerr_k={['%.2e' % e if e is not None else 'ERR' for e in entry['relerr_k']]} expm_v={entry['expm_v_relerr']}", flush=True)

# Proportionality probe: phi(A, 1e8 v) vs 1e8 * phi(A, v), and 1e-8.
prop = []
for c in cases:
    if c["vnorm"] != 1.0:
        continue
    base = rust[c["name"]]
    for s, tag in ((1e8, "1e+08"), (1e-8, "1e-08")):
        other_name = c["name"].replace("_v1", f"_v{tag}")
        if other_name not in rust:
            continue
        o = rust[other_name]
        devs = []
        for k in range(5):
            if base["phi"][k] is None or o["phi"][k] is None:
                devs.append(None); continue
            b = np.array(base["phi"][k]) * s; g = np.array(o["phi"][k])
            devs.append(float(np.linalg.norm(g - b) / np.linalg.norm(b)) if np.linalg.norm(b) != 0 else None)
        prop.append({"base": c["name"], "other": other_name, "v_factor": s, "ratio_deviation_k": devs,
                     "exactly_proportional": all(d == 0.0 for d in devs if d is not None)})
json.dump({"threshold": THRESH, "rows": rows, "proportionality": prop, "total_wall_s": time.time() - t_start}, open("reference_results.json", "w"), indent=1)
nfail = sum(any(r["fail_k"]) for r in rows)
print("cases failing (any k):", nfail, "of", len(rows))
