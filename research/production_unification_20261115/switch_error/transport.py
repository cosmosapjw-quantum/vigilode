#!/usr/bin/env python3
"""U10: componentwise error transport across PPSW1 <-> one-step switches (independent backend).

Linear Prothero-Robinson test y' = lam (y - phi) + phi', exact solution phi, W-matrix T = lam.
Peer stage-history errors e_n in R^6 obey exactly e_{n+1} = M_n e_n + delta_n with
M_n = (B + z (A + Gamma)) / (1 - gamma z), z = h lam, and delta_n the defect of one step from
exact history. Hence |e_{n+1}| <= |M_n| |e_n| + |delta_n| componentwise (WP7-style positive
majorant G_n = |M_n|). The monomial-basis majorant |V^-1 M_n V| is reported for comparison.
Switch maps:
  T_PR (Peer -> one-step Radau IIA(2)): select stage 6 (gain 1 on e_6, defect 0)
  T_RP (one-step -> Peer): PPSW1 Radau IIA(5) starter over 2 h0, affine in y_n:
       history = a * y_n + s, error = a * e_y + d_T (gain a, defect d_T measured from exact data)
Outputs transport_results.json. Coefficients: peer_qualification/oracle_coefficients.py (R1).
"""
import json
import sys
from pathlib import Path

import mpmath as mp
import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "peer_qualification"))
import oracle_coefficients as oc  # noqa: E402

mp.mp.dps = 40
GAM, _ = oc.gamma_hat()
G = float(GAM)
C = np.array([-1.0, -0.5, 0.0, 0.4, 0.8, 1.0])
V = np.vander(C, 6, increasing=True)
VI = np.linalg.inv(V)
_cache = {}


def coeffs(sigma):
    if sigma not in _cache:
        m, _ = oc.oracle(sigma, GAM)
        _cache[sigma] = {k: np.array([[float(v[i, j]) for j in range(6)] for i in range(6)])
                         for k, v in m.items()}
    return _cache[sigma]


def phi(t):
    return np.sin(2 * t) + 0.5 * np.exp(t / 3)


def dphi(t):
    return 2 * np.cos(2 * t) + np.exp(t / 3) / 6


def peer_step(Y, t, h, h_prev, lam):
    """Stage history Y (6,) at nodes t + h_prev (c - 1) -> new history at t + h c."""
    k = coeffs(float(h / h_prev))
    tp = t + h_prev * (C - 1)
    F = lam * (Y - phi(tp)) + dphi(tp)
    z = h * lam
    rhs = k["B"] @ Y + h * (k["A"] @ F) + z * (k["Gamma"] @ Y)
    return rhs / (1 - G * z)


def peer_matrix(z, sigma):
    k = coeffs(float(sigma))
    return (k["B"] + z * (k["A"] + k["Gamma"])) / (1 - G * z)


# Radau IIA(2), stiffly accurate, order 3
RA = np.array([[5 / 12, -1 / 12], [3 / 4, 1 / 4]])
RC = np.array([1 / 3, 1.0])


def forcing(lam):
    return lambda s: dphi(s) - lam * phi(s)  # y' = lam y + g


def radau2_step(y, t, h, lam):
    g = forcing(lam)
    U = np.linalg.solve(np.eye(2) - h * lam * RA, y * np.ones(2) + h * RA @ g(t + RC * h))
    return U[1]


def radau2_gain(z):
    return np.linalg.solve(np.eye(2) - z * RA, np.ones(2))[1]


def radau5():
    p = np.polynomial.legendre.Legendre.basis(5) - np.polynomial.legendre.Legendre.basis(4)
    c = np.sort(np.real((p.roots() + 1) / 2))
    c[-1] = 1.0
    lag = []
    for j in range(5):
        num = np.poly1d([1.0])
        for k in range(5):
            if k != j:
                num = num * np.poly1d([1.0, -c[k]]) / (c[j] - c[k])
        lag.append(np.polyint(num))
    a = np.array([[lag[j](c[i]) for j in range(5)] for i in range(5)])
    return c, a, lag


RC5, RA5, RL5 = radau5()


def starter(y, t, h0, lam, with_forcing=True):
    """PPSW1 starter: Radau IIA(5) collocation over 2 h0; history at t + h0 (1 + c)."""
    H = 2 * h0
    g = forcing(lam) if with_forcing else (lambda s: 0.0 * s)
    U = np.linalg.solve(np.eye(5) - H * lam * RA5, y * np.ones(5) + H * RA5 @ g(t + RC5 * H))
    f = lam * U + g(t + RC5 * H)
    hist = np.empty(6)
    for k, ck in enumerate(C):
        th = (1 + ck) / 2
        if k == 0:
            hist[k] = y
        elif k == 5:
            hist[k] = U[4]
        else:
            hist[k] = y + H * sum(f[j] * RL5[j](th) for j in range(5))
    return hist


def violates(e, b):
    return bool(np.any(np.abs(e) > np.abs(b) * (1 + 1e-9) + 1e-14))


def switch_experiment(lam, h, n1, n2, n3):
    """Peer (n1 steps) -> Radau IIA(2) (n2 steps) -> Peer via starter (n3 steps); constant h."""
    rows, viol = [], 0
    t = 0.0
    Y = starter(phi(t), t, h, lam)
    e = Y - phi(t + h * (1 + C))
    b = np.abs(e)
    t, hp = t + 2 * h, h
    q_nodal, q_mono = [], []
    for _ in range(n1):
        M = peer_matrix(h * lam, h / hp)
        delta = peer_step(phi(t + hp * (C - 1)), t, h, hp, lam) - phi(t + h * C)
        Y = peer_step(Y, t, h, hp, lam)
        t, hp = t + h, h
        e = Y - phi(t + h * (C - 1))
        b = np.abs(M) @ b + np.abs(delta)
        q_nodal.append(np.abs(M).sum(1).max())
        q_mono.append(np.abs(VI @ M @ V).sum(1).max())
        viol += violates(e, b)
        rows.append({"phase": "peer1", "t": t, "err6": float(abs(e[5])), "bound6": float(b[5])})
    y, by = Y[5], b[5]
    for _ in range(n2):
        delta = radau2_step(phi(t), t, h, lam) - phi(t + h)
        y = radau2_step(y, t, h, lam)
        t = t + h
        ey = y - phi(t)
        by = abs(radau2_gain(h * lam)) * by + abs(delta)
        viol += violates(np.array([ey]), np.array([by]))
        rows.append({"phase": "onestep", "t": t, "err6": float(abs(ey)), "bound6": float(by)})
    a = starter(1.0, t, h, lam, with_forcing=False)
    d_t = starter(phi(t), t, h, lam) - phi(t + h * (1 + C))
    Y = starter(y, t, h, lam)
    e = Y - phi(t + h * (1 + C))
    b = np.abs(a) * by + np.abs(d_t)
    viol += violates(e, b)
    t, hp = t + 2 * h, h
    for _ in range(n3):
        M = peer_matrix(h * lam, 1.0)
        delta = peer_step(phi(t + hp * (C - 1)), t, h, hp, lam) - phi(t + h * C)
        Y = peer_step(Y, t, h, hp, lam)
        t, hp = t + h, h
        e = Y - phi(t + h * (C - 1))
        b = np.abs(M) @ b + np.abs(delta)
        viol += violates(e, b)
        rows.append({"phase": "peer2", "t": t, "err6": float(abs(e[5])), "bound6": float(b[5])})
    final = rows[-1]
    return {"lambda": lam, "h": h, "steps": [n1, n2, n3], "violations": viol,
            "final_error": final["err6"], "final_bound": final["bound6"],
            "bound_over_error": final["bound6"] / max(final["err6"], 1e-300),
            "q_nodal_max": float(max(q_nodal)), "q_monomial_max": float(max(q_mono)),
            "transfer_one_step_to_peer": {"gain": a.tolist(), "defect": d_t.tolist()},
            "trace_every_10": rows[::10]}


def nonnormal_transient():
    out = []
    for z in [-0.01, -0.1, -1.0, -10.0, -100.0, -1e4]:
        M = peer_matrix(z, 1.0)
        rho = max(abs(np.linalg.eigvals(M)))
        powers = [np.abs(np.linalg.matrix_power(M, k)).sum(1).max() for k in range(1, 61)]
        out.append({"z": z, "rho": float(rho), "norm_inf_M": float(powers[0]),
                    "max_k_norm_inf_Mk": float(max(powers)), "k_at_max": int(np.argmax(powers) + 1),
                    "norm_inf_M60": float(powers[-1])})
    rng = np.random.default_rng(20261116)
    var = []
    for lam_h in [-0.1, -1.0, -10.0]:
        sig = rng.uniform(0.5, 1.1, 30)
        h, prod, maj, rhos = 1.0, np.eye(6), 1.0, 1.0
        for s in sig:
            h *= s
            M = peer_matrix(lam_h * h, float(s))
            prod = M @ prod
            maj *= np.abs(M).sum(1).max()
            rhos *= max(abs(np.linalg.eigvals(M)))
        var.append({"z_reference": lam_h, "norm_inf_product": float(np.abs(prod).sum(1).max()),
                    "product_of_majorant_norms": float(maj), "product_of_spectral_radii": float(rhos)})
    return out, var


def windowed_majorants():
    """Effective per-step rate of the k-step majorant |M^k| (sigma = 1): a block certificate
    contracts iff ||(|M^k|)||_inf < 1 for some window k; the first such k is reported."""
    out = []
    for z in [-0.01, -0.1, -1.0, -10.0, -100.0, -1e4]:
        M = peer_matrix(z, 1.0)
        rates, first = {}, None
        for k in (1, 5, 10, 20, 40, 80):
            q = np.abs(np.linalg.matrix_power(M, k)).sum(1).max()
            rates[str(k)] = float(q)
            if first is None and q < 1.0:
                first = k
        out.append({"z": z, "norm_inf_abs_Mk": rates, "first_contracting_window": first})
    return out


def main():
    sw = [switch_experiment(lam, 0.01, 40, 20, 40) for lam in (-1.0, -50.0, -1e4)]
    tr, var = nonnormal_transient()
    win = windowed_majorants()
    res = {"schema": "vigilode.u10-transport.v1", "gamma": G,
           "switch_experiments": sw, "nonnormal_transient_sigma1": tr, "variable_sigma_products": var,
           "windowed_majorants_sigma1": win}
    (HERE / "transport_results.json").write_text(json.dumps(res, indent=1, allow_nan=False) + "\n")
    for s in sw:
        print("SWITCH", s["lambda"], "viol", s["violations"],
              "err %.2e bound %.2e ratio %.1e q_nodal %.1f q_mono %.2f"
              % (s["final_error"], s["final_bound"], s["bound_over_error"], s["q_nodal_max"],
                 s["q_monomial_max"]))
    for r in tr:
        print("TRANSIENT", r)
    for r in var:
        print("VARSIGMA", r)
    for r in win:
        print("WINDOW", r)


if __name__ == "__main__":
    main()
