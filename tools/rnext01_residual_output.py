#!/usr/bin/env python3
"""Residual-to-output budget of the raw-U and K drivers (research node
research/rnext01_residual_output_20261003, remaining-only DAG node R-NEXT-01; see its
PREREGISTRATION.md).

For every exported one-step comparison it computes, in mpmath, the exact U and K steps from
the frozen (t, y, h) with the stored binary64 coefficients and the exact problem functions, the
exact residuals r_i = W S_i^c - b_i(S^c) of the computed stages, the stage-deviation bounds
d_i (inverse-operator bound, interval Jacobian on boxes), the output and error-norm budgets
B_X, the coefficient term, and the classification of the L-0038 item-3 failures.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-rnext01-residual-output-v1"
U_ROUND = mp.mpf(2) ** -53


def unhex(text: str):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def vec(hexes):
    return [unhex(x) for x in hexes]


# ---------------------------------------------------------------------------
# Problems: exact f, sparse J (generic over mpf / mpi), f_t. Constants are the
# binary64 values the Rust problems use.
# ---------------------------------------------------------------------------

F = lambda x: mp.mpf(float(x))


def robertson():
    k1, k2, k3 = F(0.04), F(1.0e4), F(3.0e7)
    f = lambda t, y: [-k1 * y[0] + k2 * y[1] * y[2], k1 * y[0] - k2 * y[1] * y[2] - k3 * y[1] ** 2, k3 * y[1] ** 2]
    jac = lambda t, y: [(0, 0, -k1), (0, 1, k2 * y[2]), (0, 2, k2 * y[1]), (1, 0, k1),
                        (1, 1, -k2 * y[2] - 2 * k3 * y[1]), (1, 2, -k2 * y[1]), (2, 1, 2 * k3 * y[1])]
    return f, jac, None


def van_der_pol():
    mu = F(1000.0)
    f = lambda t, y: [y[1], mu * (1 - y[0] ** 2) * y[1] - y[0]]
    jac = lambda t, y: [(0, 1, mp.mpf(1)), (1, 0, -2 * mu * y[0] * y[1] - 1), (1, 1, mu * (1 - y[0] ** 2))]
    return f, jac, None


def hires():
    c = {k: F(v) for k, v in dict(a=1.71, b=0.43, c=8.32, d=0.0007, e=8.75, g=10.03, i=0.035, j=1.12,
                                    k=1.745, l=280.0, m=0.69, o=1.81).items()}

    def f(t, y):
        return [-c['a'] * y[0] + c['b'] * y[1] + c['c'] * y[2] + c['d'],
                c['a'] * y[0] - c['e'] * y[1],
                -c['g'] * y[2] + c['b'] * y[3] + c['i'] * y[4],
                c['c'] * y[1] + c['a'] * y[2] - c['j'] * y[3],
                -c['k'] * y[4] + c['b'] * y[5] + c['b'] * y[6],
                -c['l'] * y[5] * y[7] + c['m'] * y[3] + c['a'] * y[4] - c['b'] * y[5] + c['m'] * y[6],
                c['l'] * y[5] * y[7] - c['o'] * y[6],
                -c['l'] * y[5] * y[7] + c['o'] * y[6]]

    def jac(t, y):
        return [(0, 0, -c['a']), (0, 1, c['b']), (0, 2, c['c']), (1, 0, c['a']), (1, 1, -c['e']),
                (2, 2, -c['g']), (2, 3, c['b']), (2, 4, c['i']), (3, 1, c['c']), (3, 2, c['a']), (3, 3, -c['j']),
                (4, 4, -c['k']), (4, 5, c['b']), (4, 6, c['b']), (5, 3, c['m']), (5, 4, c['a']),
                (5, 5, -c['l'] * y[7] - c['b']), (5, 6, c['m']), (5, 7, -c['l'] * y[5]),
                (6, 5, c['l'] * y[7]), (6, 6, -c['o']), (6, 7, c['l'] * y[5]),
                (7, 5, -c['l'] * y[7]), (7, 6, c['o']), (7, 7, -c['l'] * y[5])]
    return f, jac, None


def brusselator(cells=50):
    cc = mp.mpf(float(cells + 1.0) ** 2 / 50.0)

    def nb(y, i):
        ul, vl = (mp.mpf(1), mp.mpf(3)) if i == 0 else (y[2 * i - 2], y[2 * i - 1])
        ur, vr = (mp.mpf(1), mp.mpf(3)) if i + 1 == cells else (y[2 * i + 2], y[2 * i + 3])
        return ul, vl, ur, vr

    def f(t, y):
        out = []
        for i in range(cells):
            u, v = y[2 * i], y[2 * i + 1]
            ul, vl, ur, vr = nb(y, i)
            out += [1 + u * u * v - 4 * u + cc * (ul - 2 * u + ur), 3 * u - u * u * v + cc * (vl - 2 * v + vr)]
        return out

    def jac(t, y):
        entries = []
        for i in range(cells):
            u, v = y[2 * i], y[2 * i + 1]
            a, b = 2 * i, 2 * i + 1
            entries += [(a, a, 2 * u * v - 4 - 2 * cc), (a, b, u * u), (b, a, 3 - 2 * u * v), (b, b, -u * u - 2 * cc)]
            if i > 0:
                entries += [(a, a - 2, cc), (b, b - 2, cc)]
            if i + 1 < cells:
                entries += [(a, a + 2, cc), (b, b + 2, cc)]
        return entries
    return f, jac, None


def prothero_robinson():
    lam = F(-1.0e4)
    f = lambda t, y: [lam * (y[0] - mp.sin(t)) + mp.cos(t)]
    jac = lambda t, y: [(0, 0, lam)]
    ft = lambda t, y: [-lam * mp.cos(t) - mp.sin(t)]
    return f, jac, ft


def quadratic():
    a = [F(-1.0 - i) for i in range(4)]
    q = [F(-0.05 * (1 + i % 3)) for i in range(4)]
    f = lambda t, y: [a[i] * y[i] + q[i] * y[i] ** 2 for i in range(4)]
    jac = lambda t, y: [(i, i, a[i] + 2 * q[i] * y[i]) for i in range(4)]
    return f, jac, None


PROBLEMS = {"robertson": robertson, "van-der-pol-mu1000": van_der_pol, "hires": hires,
            "brusselator-1d-50": brusselator, "prothero-robinson-forced": prothero_robinson,
            "quadratic-4": quadratic}


def dense(entries, n):
    m = mp.zeros(n, n)
    for i, j, v in entries:
        m[i, j] += v
    return m


def matvec(m, v):
    n = len(v)
    return [mp.fsum(m[i, j] * v[j] for j in range(n)) for i in range(n)]


def norm2(v):
    return mp.sqrt(mp.fsum(x * x for x in v))


def frobenius_interval(entries):
    """||J||_F over a box: sqrt of the sum of squared magnitudes, combining duplicates."""
    acc = {}
    for i, j, v in entries:
        acc[(i, j)] = acc.get((i, j), mp.mpi(0)) + (v if isinstance(v, mp.ctx_iv.ivmpf) else mp.mpi(v))
    total = mp.mpf(0)
    for v in acc.values():
        mag = max(abs(mp.mpf(v.a)), abs(mp.mpf(v.b)))
        total += mag * mag
    return mp.sqrt(total) * (1 + mp.mpf("1e-30"))


def frobenius_point(entries, n):
    return mp.sqrt(mp.fsum(dense(entries, n)[i, j] ** 2 for i in range(n) for j in range(n)))


# ---------------------------------------------------------------------------


def exact_steps(problem, coeffs, t, y, h, n):
    f, jac, ft = problem
    g = coeffs["gamma"]
    s = len(coeffs["c"])
    J = dense(jac(t, y), n)
    W = mp.eye(n) - h * g * J
    ftv = ft(t, y) if ft else [mp.mpf(0)] * n
    # U form.
    U = []
    for i in range(s):
        Y = [y[q] + mp.fsum(coeffs["a"][i][j] * U[j][q] for j in range(i)) for q in range(n)]
        fi = f(t + coeffs["c"][i] * h, Y)
        rhs = [h * g * fi[q] + g * mp.fsum(coeffs["c_matrix"][i][j] * U[j][q] for j in range(i))
               + h * h * g * coeffs["gamma_rows"][i] * ftv[q] for q in range(n)]
        U.append(list(mp.lu_solve(W, mp.matrix(rhs))))
    # K form.
    K = []
    for i in range(s):
        Y = [y[q] + mp.fsum(coeffs["alpha"][i][j] * K[j][q] for j in range(i)) for q in range(n)]
        fi = f(t + coeffs["c"][i] * h, Y)
        mix = [mp.fsum(coeffs["gamma_matrix"][i][j] * K[j][q] for j in range(i)) for q in range(n)]
        jmix = matvec(J, mix)
        rhs = [h * fi[q] + h * jmix[q] + h * h * coeffs["gamma_rows"][i] * ftv[q] for q in range(n)]
        K.append(list(mp.lu_solve(W, mp.matrix(rhs))))
    return J, W, ftv, U, K


def wrms(v, scale):
    return mp.sqrt(mp.fsum((a / b) ** 2 for a, b in zip(v, scale)) / len(v))


def budget(kind, problem, coeffs, t, y, h, n, J, W, nW, ftv, S, exact, reported, atol, rtol):
    """Bounds and actual deviations for driver `kind` ('U' or 'K')."""
    f, jac, _ = problem
    g = coeffs["gamma"]
    s = len(S)
    if kind == "U":
        mix_a, out_b, emb = coeffs["a"], coeffs["b_code"], None
    else:
        mix_a, out_b, emb = coeffs["alpha"], coeffs["b"], coeffs["btilde"]
    normJ = frobenius_point(jac(t, y), n)
    d, residuals, lipschitz = [], [], []
    for i in range(s):
        Y = [y[q] + mp.fsum(mix_a[i][j] * S[j][q] for j in range(i)) for q in range(n)]
        fi = f(t + coeffs["c"][i] * h, Y)
        if kind == "U":
            b = [h * g * fi[q] + g * mp.fsum(coeffs["c_matrix"][i][j] * S[j][q] for j in range(i))
                 + h * h * g * coeffs["gamma_rows"][i] * ftv[q] for q in range(n)]
        else:
            mix = [mp.fsum(coeffs["gamma_matrix"][i][j] * S[j][q] for j in range(i)) for q in range(n)]
            jm = matvec(J, mix)
            b = [h * fi[q] + h * jm[q] + h * h * coeffs["gamma_rows"][i] * ftv[q] for q in range(n)]
        ws = matvec(W, S[i])
        r = norm2([ws[q] - b[q] for q in range(n)])
        rho = mp.fsum(abs(mix_a[i][j]) * d[j] for j in range(i))
        if rho > 0:
            box = [mp.mpi(Y[q] - rho, Y[q] + rho) for q in range(n)]
            L = frobenius_interval(jac(t + coeffs["c"][i] * h, box))
        else:
            L = mp.mpf(0)
        if kind == "U":
            di = nW * (h * g * L * rho + g * mp.fsum(abs(coeffs["c_matrix"][i][j]) * d[j] for j in range(i)) + r)
        else:
            di = nW * (h * L * rho + h * normJ * mp.fsum(abs(coeffs["gamma_matrix"][i][j]) * d[j] for j in range(i)) + r)
        d.append(di)
        residuals.append(r)
        lipschitz.append(L)
    actual_d = [norm2([S[i][q] - exact[i][q] for q in range(n)]) for i in range(s)]
    # Outputs.
    y_exact_c = [y[q] + mp.fsum(out_b[j] * S[j][q] for j in range(s)) for q in range(n)]
    y_reported = reported["y_new"]
    rho_y = norm2([y_reported[q] - y_exact_c[q] for q in range(n)])
    bound_y = mp.fsum(abs(out_b[j]) * d[j] for j in range(s)) + rho_y
    y_star = [y[q] + mp.fsum(out_b[j] * exact[j][q] for j in range(s)) for q in range(n)]
    actual_y = norm2([y_reported[q] - y_star[q] for q in range(n)])
    if kind == "U":
        e_c, e_star = S[s - 1], exact[s - 1]
        bound_e, rho_e = d[s - 1], mp.mpf(0)
    else:
        e_c = reported["error_vector"]
        e_cx = [mp.fsum(emb[j] * S[j][q] for j in range(s)) for q in range(n)]
        rho_e = norm2([e_c[q] - e_cx[q] for q in range(n)])
        bound_e = mp.fsum(abs(emb[j]) * d[j] for j in range(s)) + rho_e
        e_star = [mp.fsum(emb[j] * exact[j][q] for j in range(s)) for q in range(n)]
    # Error norm.
    s_c = [atol + rtol * max(abs(y[q]), abs(y_reported[q])) for q in range(n)]
    s_star = [atol + rtol * max(abs(y[q]), abs(y_star[q])) for q in range(n)]
    s_lb = [atol + rtol * abs(y[q]) for q in range(n)]
    err_c = reported["error_norm"]
    err_star = wrms(e_star, s_star)
    term_round = 4 * n * U_ROUND * err_c
    term_dev = bound_e / (mp.sqrt(n) * min(s_c))
    term_scale = mp.sqrt(mp.fsum(((abs(e_c[q]) + bound_e) * rtol * bound_y / (s_c[q] * s_lb[q])) ** 2
                                 for q in range(n)) / n)
    B = term_round + term_dev + term_scale
    exact_norm_of_reported = wrms(e_c, s_c)
    return {
        "d": d, "actual_d": actual_d, "residuals": residuals, "lipschitz": lipschitz, "inverse_bound": nW,
        "bound_y": bound_y, "actual_y": actual_y, "rho_y": rho_y, "y_star": y_star, "y_reported": y_reported,
        "bound_e": bound_e, "rho_e": rho_e, "err_c": err_c, "err_star": err_star, "B": B,
        "B_terms": {"rounding": term_round, "deviation": term_dev, "scale": term_scale},
        "norm_rounding_actual": abs(err_c - exact_norm_of_reported),
        "valid": bool(abs(err_c - err_star) <= B and actual_y <= bound_y
                      and all(a <= b for a, b in zip(actual_d, d))),
        "residual_share": (nW * residuals[0]) / d[-1] if d[-1] > 0 else mp.mpf(0),
    }


def s(x):
    return mp.nstr(x, 6)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--stages", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 60
    data = json.loads(args.stages.read_text())
    raw = data["coefficients"]
    coeffs = {"gamma": unhex(raw["gamma"])}
    for key in ("c", "b_code", "gamma_rows", "b", "btilde"):
        coeffs[key] = vec(raw[key])
    for key in ("a", "c_matrix", "alpha", "gamma_matrix"):
        coeffs[key] = [vec(row) for row in raw[key]]
    cache = {}
    rows = []
    validity = classification = resolved = available = True
    l0038_failures = explained = 0
    for case in data["cases"]:
        label = f'{case["problem"]}/h={float(unhex(case["h"])):g}/{case["method"]}'
        if "error" in case["u"] or "error" in case["k"]:
            rows.append({"case": label, "solver_failure": {"u": case["u"].get("error"), "k": case["k"].get("error")}})
            print(label, "solver failure", flush=True)
            continue
        problem = PROBLEMS[case["problem"]]()
        t, h = unhex(case["t"]), unhex(case["h"])
        y = vec(case["y"])
        n = len(y)
        atol, rtol = unhex(case["atol"]), unhex(case["rtol"])
        key = (case["problem"], case["h"])
        if key not in cache:
            J, W, ftv, U_star, K_star = exact_steps(problem, coeffs, t, y, h, n)
            Winv = mp.inverse(W)
            n1 = max(mp.fsum(abs(Winv[i, j]) for i in range(n)) for j in range(n))
            ninf = max(mp.fsum(abs(Winv[i, j]) for j in range(n)) for i in range(n))
            cache[key] = (J, W, ftv, U_star, K_star, mp.sqrt(n1 * ninf) * (1 + mp.mpf("1e-30")))
        J, W, ftv, U_star, K_star, nW = cache[key]
        u_rep = {"y_new": vec(case["u"]["y_new"]), "error_norm": unhex(case["u"]["error_norm"])}
        k_rep = {"y_new": vec(case["k"]["y_new"]), "error_norm": unhex(case["k"]["error_norm"]),
                 "error_vector": vec(case["k"]["error_vector"])}
        U_c = [vec(x) for x in case["u"]["stages"]]
        K_c = [vec(x) for x in case["k"]["stages"]]
        bu = budget("U", problem, coeffs, t, y, h, n, J, W, nW, ftv, U_c, U_star, u_rep, atol, rtol)
        bk = budget("K", problem, coeffs, t, y, h, n, J, W, nW, ftv, K_c, K_star, k_rep, atol, rtol)
        finite = all(mp.isfinite(x) for x in (bu["B"], bk["B"], bu["bound_y"], bk["bound_y"]))
        available &= finite
        validity &= bu["valid"] and bk["valid"]
        e_coef_err = abs(bu["err_star"] - bk["err_star"])
        e_coef_y = norm2([a - b for a, b in zip(bu["y_star"], bk["y_star"])])
        # L-0038 item-3 criterion on these values.
        worst = max(abs(a - b) / (abs(c) + mp.mpf("1e-6")) for a, b, c in zip(u_rep["y_new"], k_rep["y_new"], y))
        rel = abs(u_rep["error_norm"] - k_rep["error_norm"]) / max(k_rep["error_norm"], mp.mpf(2) ** -1022)
        failed = not (worst <= mp.mpf("1e-9") and rel <= mp.mpf("1e-6"))
        diff_err = abs(u_rep["error_norm"] - k_rep["error_norm"])
        diff_y = max(abs(a - b) for a, b in zip(u_rep["y_new"], k_rep["y_new"]))
        explains = bool(diff_err <= bu["B"] + bk["B"] + e_coef_err
                        and diff_y <= bu["bound_y"] + bk["bound_y"] + e_coef_y)
        if failed:
            l0038_failures += 1
            explained += explains
            classification &= explains

        def decision(b):
            e, B = b["err_c"], b["B"]
            return "resolved-accept" if e + B <= 1 else ("resolved-reject" if e - B > 1 else "unresolved")
        du, dk = decision(bu), decision(bk)
        resolved &= du != "unresolved" and dk != "unresolved"
        rows.append({
            "case": label, "l0038_item3_failed": failed, "l0038_worst_state": s(worst), "l0038_rel_error_norm": s(rel),
            "err_U": s(bu["err_c"]), "err_K": s(bk["err_c"]), "err_U_exact": s(bu["err_star"]), "err_K_exact": s(bk["err_star"]),
            "B_U": s(bu["B"]), "B_K": s(bk["B"]), "E_native_coefficient": s(e_coef_err),
            "diff_err": s(diff_err), "explained_ratio": s(diff_err / (bu["B"] + bk["B"] + e_coef_err))
            if bu["B"] + bk["B"] + e_coef_err > 0 else "0",
            "explained": explains if failed else None,
            "B_terms_U": {k: s(v) for k, v in bu["B_terms"].items()}, "B_terms_K": {k: s(v) for k, v in bk["B_terms"].items()},
            "bound_y": {"U": s(bu["bound_y"]), "K": s(bk["bound_y"])}, "actual_y": {"U": s(bu["actual_y"]), "K": s(bk["actual_y"])},
            "actual_err_dev": {"U": s(abs(bu["err_c"] - bu["err_star"])), "K": s(abs(bk["err_c"] - bk["err_star"]))},
            "max_residual": {"U": s(max(bu["residuals"])), "K": s(max(bk["residuals"]))},
            "inverse_bound": s(nW), "max_lipschitz": {"U": s(max(bu["lipschitz"])), "K": s(max(bk["lipschitz"]))},
            "d_last_over_actual": {"U": s(bu["d"][-1] / bu["actual_d"][-1]) if bu["actual_d"][-1] > 0 else "inf",
                                   "K": s(bk["d"][-1] / bk["actual_d"][-1]) if bk["actual_d"][-1] > 0 else "inf"},
            "decision": {"U": du, "K": dk}, "valid": {"U": bu["valid"], "K": bk["valid"]}, "finite": finite,
        })
        print(label, "failed" if failed else "ok", "explained" if explains else "-", du, dk,
              "valid" if bu["valid"] and bk["valid"] else "INVALID", flush=True)
    gate = {"1_budget_validity": validity, "2_classification": classification,
            "3_no_unresolved_decision": resolved, "4_bounds_available": available}
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "rows": rows,
              "l0038_item3_failures_completed": l0038_failures, "explained": explained,
              "gate": gate, "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"], "failures": l0038_failures, "explained": explained}))


if __name__ == "__main__":
    main()
