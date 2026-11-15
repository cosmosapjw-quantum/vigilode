#!/usr/bin/env python3
"""Independent PPSW1-6b coefficient oracle (U03, route R1).

Source: Schmitt & Weiner, SINUM 42(1) 2004, eqs. (3)/(4)/(7), order conditions
Gamma(q) (14) and AB(q) (15), beta choice (23), Table 1 (s = 6, method 6b).
The closed forms (16) Theta = V S P V^-1 and (21) B = (I + gamma E) Theta + sigma beta E,
which the Rust implementation uses, are deliberately NOT used here:

  Gamma : solve  gamma c_i^k + sum_j Gamma_ij ((c_j - 1)/sigma)^k = 0,  k = 0..5   (14)
  beta  : V^-1 beta V = F0 S D^-1                                                  (23)
  A     : A = beta - Gamma                                                         (4)
  B     : solve  c_i^k - sum_j B_ij w_j^k - k sum_j A_ij w_j^(k-1) = 0, k = 0..5,
          w_j = (c_j - 1)/sigma                                                    (15)

Vandermonde convention V_ij = c_i^(j-1) (node rows; PW04 UNRESOLVED-1 resolution).  Only (23)
uses V.  gamma is the root near 0.6194 of the Table 1 polynomial, computed here at 60 digits.
Output: coefficients_oracle.txt (one coefficient per line) and coefficients_oracle_summary.json.
"""
import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 60
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
C = [mp.mpf(-1), mp.mpf(-1) / 2, mp.mpf(0), mp.mpf(2) / 5, mp.mpf(4) / 5, mp.mpf(1)]
S = 6
# Table 1, s = 6, as printed (ascending powers).
TABLE1_S6 = [4683, -28098, 62130, -64200, 32760, -7920, 720]


def poly(g):
    return sum(mp.mpf(a) * g**k for k, a in enumerate(TABLE1_S6))


def gamma_hat():
    g = mp.findroot(poly, mp.mpf("0.6194"))
    roots = sorted(mp.re(r) for r in mp.polyroots(TABLE1_S6[::-1], maxsteps=200, extraprec=200)
                   if abs(mp.im(r)) < mp.mpf(10) ** -40)
    return g, roots


def oracle(sigma, g):
    s = mp.mpf(sigma)  # exact binary value of the f64 handed to the implementation
    w = [(c - 1) / s for c in C]
    W = mp.matrix(S, S)
    Wd = mp.matrix(S, S)  # k * w^(k-1)
    Cm = mp.matrix(S, S)
    for j in range(S):
        for k in range(S):
            W[j, k] = w[j] ** k
            Wd[j, k] = k * w[j] ** (k - 1) if k > 0 else 0
            Cm[j, k] = C[j] ** k
    Wi = mp.inverse(W)
    Gam = -g * Cm * Wi
    V = mp.matrix(S, S)
    bt = mp.matrix(S, S)
    for i in range(S):
        for j in range(S):
            V[i, j] = C[i] ** j
        if i >= 1:
            bt[i, i - 1] = s ** (i - 1) / i  # F0 S D^-1: (i+1, i) entry sigma^(i-1)/i (1-based)
    beta = V * bt * mp.inverse(V)
    A = beta - Gam
    B = (Cm - A * Wd) * Wi
    # residual audit of the defining conditions, plus the order-6 (k = 6) residual
    res = {"gamma_cond_k0_5": mp.mpf(0), "ab_cond_k0_5": mp.mpf(0), "ab_cond_k6_max": mp.mpf(0)}
    for i in range(S):
        for k in range(S + 1):
            ab = C[i] ** k - sum(B[i, j] * w[j] ** k for j in range(S)) - (
                k * sum(A[i, j] * w[j] ** (k - 1) for j in range(S)) if k else 0)
            if k < S:
                gc = g * C[i] ** k + sum(Gam[i, j] * w[j] ** k for j in range(S))
                res["gamma_cond_k0_5"] = max(res["gamma_cond_k0_5"], abs(gc))
                res["ab_cond_k0_5"] = max(res["ab_cond_k0_5"], abs(ab))
            else:
                res["ab_cond_k6_max"] = max(res["ab_cond_k6_max"], abs(ab))
    return {"B": B, "A": A, "Gamma": Gam, "beta": beta}, res


SIGMAS = [1e-9, 1e-6, 1e-3, 0.01, 0.1, 0.25, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 1.0, 1.05, 1.1]


def main():
    g, roots = gamma_hat()
    lines = [f"gamma {mp.nstr(g, 40)}"]
    summary = {"schema": "vigilode.ppsw1-oracle.v1", "route": "R1 order-condition solve",
               "dps": mp.mp.dps, "table1_s6_ascending": TABLE1_S6,
               "gamma_hat_6b": mp.nstr(g, 50), "real_roots": [mp.nstr(r, 20) for r in roots],
               "poly_at_gamma": mp.nstr(abs(poly(g)), 5), "sigmas": []}
    for sg in SIGMAS:
        mats, res = oracle(sg, g)
        mx = max(abs(m[i, j]) for m in mats.values() for i in range(S) for j in range(S))
        summary["sigmas"].append({"sigma": repr(sg), "max_abs_coefficient": mp.nstr(mx, 6),
                                  **{k: mp.nstr(v, 5) for k, v in res.items()}})
        for name, m in mats.items():
            for i in range(S):
                for j in range(S):
                    lines.append(f"{sg!r} {name} {i} {j} {mp.nstr(m[i, j], 30, min_fixed=0, max_fixed=0)}")
    (OUT / "coefficients_oracle.txt").write_text("\n".join(lines) + "\n")
    (OUT / "coefficients_oracle_summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
