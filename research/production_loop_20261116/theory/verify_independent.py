#!/usr/bin/env python3
"""Loop L1 node V05 -- independent re-computation of the theory note (THEORY.md).

Second implementation: mpmath at 60 significant digits.  It does NOT import
verify_tableau.py.  Wherever a different algorithm was available it is used:

  * Gamma by forward substitution of the lower-triangular system;
  * stability numerator from the characteristic polynomial of beta - 1 b^T
    (Faddeev-LeVerrier), Taylor defects by power-series division;
  * |P(iy)|^2 by complex polynomial multiplication, roots by mpmath.polyroots;
  * |R(iy)| by complex linear solves of (I - z beta) k = z 1;
  * truncated sweeps by iterating the sweep recurrences themselves;
  * the 17 ROW order conditions typed out explicitly (no tree generator);
  * the inexact-Jacobian experiment in the k (untransformed ROW) form;
  * finite-grid Green's functions from closed forms / eigen-expansions;
  * Taylor coefficients of rho(kappa)^delta by a Cauchy contour integral;
  * A-stability windows by root finding instead of Sturm sequences.

It then reads RESULTS.json (data only), compares every value key written by
verify_tableau.py, writes the sections "independent_mpmath" and "agreement",
and renders RESULTS_TABLES.md.

    python3 verify_independent.py      # exit 0 iff own checks pass and all keys agree
"""
import json
import math
import os
import platform
import sys

import mpmath
from mpmath import mp, mpf, mpc

mp.dps = 60
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
OUT = os.path.join(HERE, "RESULTS.json")
TABLES_MD = os.path.join(HERE, "RESULTS_TABLES.md")
TOL_EXACT = mpf(10) ** -40
TOL_NUM = mpf(10) ** -9
N8 = 8

VAL = {}
CHECKS = []


def put(key, x):
    if key in VAL:
        raise KeyError("duplicate key " + key)
    x = mpf(x)
    if not mp.isfinite(x):
        raise ValueError("non-finite value for " + key)
    VAL[key] = mp.nstr(x, 50, min_fixed=-1, max_fixed=1) if x != 0 else "0"
    return x


def check(cid, statement, passed, detail=""):
    CHECKS.append({"id": cid, "statement": statement, "passed": bool(passed), "detail": detail})


def zeros(n, m=None):
    return [[mpf(0)] * (m or n) for _ in range(n)]


def ident(n):
    return [[mpf(1) if i == j else mpf(0) for j in range(n)] for i in range(n)]


def mm(a, b):
    return [[mp.fsum(a[i][k] * b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]


def mv(a, v):
    return [mp.fsum(a[i][k] * v[k] for k in range(len(v))) for i in range(len(a))]


def vm(v, a):
    return [mp.fsum(v[k] * a[k][j] for k in range(len(v))) for j in range(len(a[0]))]


def dotp(u, v):
    return mp.fsum(x * y for x, y in zip(u, v))


def ninf(a):
    return max(mp.fsum(abs(x) for x in r) for r in a)


def none_(a):
    return max(mp.fsum(abs(a[i][j]) for i in range(len(a))) for j in range(len(a[0])))


def ntwo(a):
    m = mp.matrix(a)
    if max(abs(x) for r in a for x in r) == 0:
        return mpf(0)
    return max(mp.svd_r(m, compute_uv=False))


def mpow(a, k):
    r = ident(len(a))
    for _ in range(k):
        r = mm(r, a)
    return r


def lower_inverse(l):
    """Inverse of a lower-triangular matrix by forward substitution (structural zeros stay exact)."""
    n = len(l)
    inv = zeros(n)
    for col in range(n):
        for i in range(col, n):
            rhs = (mpf(1) if i == col else mpf(0)) - mp.fsum(l[i][k] * inv[k][col] for k in range(col, i))
            inv[i][col] = rhs / l[i][i]
    return inv


def build(parse):
    raw = json.load(open(SNAPSHOT))
    gamma = parse(raw["gamma"])
    a = [[parse(s) for s in r] for r in raw["A"]]
    cm = [[parse(s) for s in r] for r in raw["C"]]
    c = [parse(s) for s in raw["c"]]
    b_code = [parse(s) for s in raw["b_code"]]
    ginv = [[(1 / gamma if i == j else mpf(0)) - cm[i][j] for j in range(N8)] for i in range(N8)]
    gam = lower_inverse(ginv)
    alpha = mm(a, gam)
    beta = [[alpha[i][j] + gam[i][j] for j in range(N8)] for i in range(N8)]
    nil = [[beta[i][j] - (gamma if i == j else 0) for j in range(N8)] for i in range(N8)]
    b = vm(b_code, gam)
    return {"gamma": gamma, "A": a, "C": cm, "c": c, "b_code": b_code, "Gamma": gam, "Ginv": ginv,
            "alpha": alpha, "beta": beta, "N": nil, "b": b, "bhat": list(alpha[7]),
            "gi": [mp.fsum(r) for r in gam], "ai": [mp.fsum(r) for r in alpha]}


# ------------------------------------------------------------------ polynomials (low degree first)
def pmul(p, q):
    r = [mpf(0)] * (len(p) + len(q) - 1)
    for i, x in enumerate(p):
        for j, y in enumerate(q):
            r[i + j] += x * y
    return r


def ppow(p, k):
    r = [mpf(1)]
    for _ in range(k):
        r = pmul(r, p)
    return r


def charpoly_reversed(m):
    """Coefficients of det(I - z M) (low degree first) by Faddeev-LeVerrier."""
    n = len(m)
    coeffs = [mpf(1)]
    aux = ident(n)
    for k in range(1, n + 1):
        am = mm(m, aux)
        ck = -mp.fsum(am[i][i] for i in range(n)) / k
        coeffs.append(ck)
        aux = [[am[i][j] + (ck if i == j else 0) for j in range(n)] for i in range(n)]
    return coeffs


def series_div(p, q, n):
    out = []
    for k in range(n):
        acc = (p[k] if k < len(p) else mpf(0)) - mp.fsum(q[j] * out[k - j] for j in range(1, min(k, len(q) - 1) + 1))
        out.append(acc / q[0])
    return out


def e_from_p(p, gamma, ell):
    """(1 + gamma^2 s)^ell - P(iy) P(-iy) as a polynomial in s = y^2."""
    pc = [mpc(x) * mpc(0, 1) ** k for k, x in enumerate(p)]
    pm = [mpc(x) * mpc(0, -1) ** k for k, x in enumerate(p)]
    prod = [mpc(0)] * (len(p) * 2 - 1)
    for i, x in enumerate(pc):
        for j, y in enumerate(pm):
            prod[i + j] += x * y
    mod2 = [prod[2 * j].real for j in range(len(p))]
    den = ppow([mpf(1), gamma * gamma], ell)
    n = max(len(den), len(mod2))
    return [(den[k] if k < len(den) else 0) - (mod2[k] if k < len(mod2) else 0) for k in range(n)]


def trunc_numerator(gamma, ell, deg):
    return [mp.fsum(mp.binomial(ell, j) * (-gamma) ** j / mp.factorial(k - j) for j in range(0, min(ell, k) + 1))
            for k in range(deg + 1)]


def positive_real_roots(poly):
    p = list(poly)
    while len(p) > 1 and abs(p[-1]) < mpf(10) ** -45:
        p.pop()
    if len(p) == 1:
        return []
    roots = mp.polyroots(list(reversed(p)), maxsteps=500, extraprec=400)
    return [r.real for r in roots if abs(mp.im(r)) < mpf(10) ** -20 * max(1, abs(r)) and mp.re(r) > 0]


def a_stable(gamma, ell, deg):
    e = e_from_p(trunc_numerator(gamma, ell, deg), gamma, ell)
    low = 0
    while low < len(e) and abs(e[low]) < mpf(10) ** -45:
        low += 1
    q = e[low:]
    if not q:
        return True
    if q[0] < 0:
        return False
    return len(positive_real_roots(q)) == 0


def r_direct(tb, w, z):
    m = mp.matrix([[(1 if i == j else 0) - z * tb["beta"][i][j] for j in range(N8)] for i in range(N8)])
    k = mp.lu_solve(m, mp.matrix([z] * N8))
    return 1 + mp.fsum(w[i] * k[i] for i in range(N8))


# ------------------------------------------------------------------ sections
def sec_s1(tb):
    alpha, a = tb["alpha"], tb["A"]
    low = [(abs(alpha[i][j]), i, j) for i in range(N8) for j in range(i)]
    put("S1.alpha_strict_lower_nonzero_count", sum(1 for v, _, _ in low if v > mpf(10) ** -30))
    put("S1.alpha_min_abs_entry", min(low)[0])
    for i in range(1, N8):
        put("S1.alpha_subdiag_%d_%d" % (i + 1, i), alpha[i][i - 1])
        put("S1.A_subdiag_%d_%d" % (i + 1, i), a[i][i - 1])
    put("S1.A_strict_lower_nonzero_count", sum(1 for i in range(N8) for j in range(i) if a[i][j] != 0))
    put("S1.c_minus_alpha_rowsum_max_abs", max(abs(tb["c"][i] - tb["ai"][i]) for i in range(N8)))
    # longest chain: the subdiagonal alone already forces a Hamiltonian path
    chain = 1 + sum(1 for i in range(1, N8) if abs(alpha[i][i - 1]) > mpf(10) ** -30)
    put("S1.rhs_dependency_longest_path", chain)
    check("S1", "alpha: 28 nonzero strictly-lower entries, upper part structurally zero, chain length 8",
          chain == 8 and all(alpha[i][j] == 0 for i in range(N8) for j in range(i, N8))
          and min(low)[0] > mpf(10) ** -4)


def sec_s2(tb, tbb):
    g, nil, b, beta = tb["gamma"], tb["N"], tb["b"], tb["beta"]
    put("S2.gamma", g)
    put("S2.alpha21_minus_3gamma", tb["alpha"][1][0] - 3 * g)
    put("S2.gamma21_plus_3gamma", tb["Gamma"][1][0] + 3 * g)
    put("S2.beta21", beta[1][0])
    put("S2.beta21_binary64_interpretation", tbb["beta"][1][0])
    n6, n7 = mpow(nil, 6), mpow(nil, 7)
    put("S2.N6_max_abs", max(abs(x) for r in n6 for x in r))
    put("S2.N6_nonzero_count", sum(1 for r in n6 for x in r if x != 0))
    put("S2.N7_max_abs", max(abs(x) for r in n7 for x in r))
    put("S2.N7_entry_8_1", n7[7][0])
    nd = [list(r) for r in nil]
    nd[1][0] = mpf(0)
    nd6, nd7 = mpow(nd, 6), mpow(nd, 7)
    put("S2.Ndesign6_max_abs", max(abs(x) for r in nd6 for x in r))
    check("S2.nilpotency", "N with beta21 := 0 has N^7 = 0 and N^6 != 0; represented N^7 is a 1e-21 residual",
          max(abs(x) for r in nd7 for x in r) == 0 and max(abs(x) for r in nd6 for x in r) > mpf(10) ** -6
          and max(abs(x) for r in n7 for x in r) < mpf(10) ** -18)

    def stab(w):
        mu, v = [], [mpf(1)] * N8
        for _ in range(N8):
            mu.append(dotp(w, v))
            v = mv(nil, v)
        # numerator of R = det(I - z(beta - 1 w^T)) / det(I - z beta), denominator (1 - g z)^8
        m = [[beta[i][j] - w[j] for j in range(N8)] for i in range(N8)]
        p8 = charpoly_reversed(m)
        # synthetic division P8 = (1 - g z) P7 + rem
        p7 = [mpf(0)] * 8
        p7[0] = p8[0]
        for k in range(1, 8):
            p7[k] = p8[k] + g * p7[k - 1]
        ser = series_div(p8, ppow([mpf(1), -g], 8), 9)
        defects = [ser[k] - 1 / mp.factorial(k) for k in range(1, 9)]
        rinf = p8[8] / g ** 8
        return mu, p7, p8, defects, rinf
    mu, p7, p8, defects, rinf = stab(b)
    for m_, v in enumerate(mu):
        put("S2.mu_%d" % m_, v)
    for k in range(8):
        put("S2.P_coeff_%d" % k, p7[k])
    pd = trunc_numerator(g, 7, 6)
    for k in range(7):
        put("S2.Pdesign_coeff_%d" % k, pd[k])
    put("S2.P_minus_Pdesign_max_abs", max(abs(p7[k] - pd[k]) for k in range(7)))
    put("S2.R_infinity", rinf)
    for k in range(1, 9):
        put("S2.taylor_defect_%d" % k, defects[k - 1])
    ed = e_from_p(pd, g, 7)
    for k, v in enumerate(ed):
        put("S2.Edesign_coeff_%d" % k, v)
    roots = positive_real_roots(ed[4:])
    put("S2.Edesign_positive_root_count", len(roots))
    e8 = e_from_p(p8, g, 8)
    for k, v in enumerate(e8):
        put("S2.Erepresented_coeff_%d" % k, v)
    grid = [mpf(j) * mpf(10) ** k for k in range(-6, 7) for j in (1, 2, 3, 5, 7)]
    vals = [abs(r_direct(tb, b, mpc(0, y))) ** 2 - 1 for y in grid]
    put("S2.grid_max_modR2_minus_1", max(vals))
    put("S2.grid_max_modR2_minus_1_for_y_ge_0p01", max(v for v, y in zip(vals, grid) if y >= mpf("0.01")))
    muh, p7h, p8h, defh, rinfh = stab(tb["bhat"])
    put("S2.emb_R_infinity", rinfh)
    for k in range(1, 7):
        put("S2.emb_taylor_defect_%d" % k, defh[k - 1])
    valsh = [abs(r_direct(tb, tb["bhat"], mpc(0, y))) ** 2 - 1 for y in grid]
    put("S2.emb_grid_max_modR2_minus_1", max(valsh))
    check("S2.stability", "R: linear order 6, R(inf) = 0, design numerator A-stable (no positive root of E/s^4); "
          "embedded: linear order 4, R(inf) = 0; both |R(iy)|^2 - 1 <= 1e-15 on the grid",
          max(abs(x) for x in defects[:6]) < mpf(10) ** -12 and abs(defects[6]) > mpf(10) ** -6
          and abs(rinf) < mpf(10) ** -40 and len(roots) == 0 and ed[4] > 0
          and max(abs(x) for x in defh[:4]) < mpf(10) ** -12 and abs(defh[4]) > mpf(10) ** -6
          and max(vals) < mpf(10) ** -15 and max(valsh) < mpf(10) ** -15,
          "defect7 %s, grid max %s" % (mp.nstr(defects[6], 8), mp.nstr(max(vals), 6)))
    return mu


def sec_s3(tb, mu):
    g, nil, a, cm, b, bc = tb["gamma"], tb["N"], tb["A"], tb["C"], tb["b"], tb["b_code"]
    x = [[v / g for v in r] for r in nil]
    xp = ident(N8)
    for m in range(1, 8):
        xp = mm(xp, x)
        put("S3.kform_norm1_m%d" % m, none_(xp))
        put("S3.kform_norminf_m%d" % m, ninf(xp))
        put("S3.kform_norm2_m%d" % m, ntwo(xp))
        put("S3.kform_bfunctional_norm_m%d" % m, mp.fsum(abs(v) for v in vm(b, xp)))
    put("S3.Uform_entry21_at_z_minus1", g * (-a[1][0] + cm[1][0]) / (1 + g))
    ap, cp = ident(N8), ident(N8)
    gc = [[g * v for v in r] for r in cm]
    for m in range(1, 9):
        ap, cp = mm(ap, a), mm(cp, gc)
        put("S3.Uform_stiff_norm1_m%d" % m, none_(ap))
        put("S3.Uform_stiff_norminf_m%d" % m, ninf(ap))
        put("S3.Uform_stiff_norm2_m%d" % m, ntwo(ap))
        put("S3.Uform_z0_norminf_m%d" % m, ninf(cp))
    zgrid = [-mpf(j) * mpf(10) ** k for k in range(-3, 7) for j in (1, 2, 5)]
    best = [mpf(0)] * 9
    for z0 in zgrid:
        mu_ = [[g * (z0 * a[i][j] + cm[i][j]) / (1 - g * z0) for j in range(N8)] for i in range(N8)]
        pw = ident(N8)
        for m in range(1, 9):
            pw = mm(pw, mu_)
            best[m] = max(best[m], ninf(pw))
    for m in range(1, 9):
        put("S3.Uform_grid_sup_norminf_m%d" % m, best[m])
    for eps, key in ((mpf(1), "1_1"), (mpf(1) / 2, "1_2"), (mpf(1) / 4, "1_4"), (mpf(1) / 10, "1_10"),
                     (mpf(1) / 100, "1_100")):
        sx = [[eps ** i * x[i][j] / eps ** j for j in range(N8)] for i in range(N8)]
        put("S3.scaled_norminf_eps_" + key, ninf(sx))
        put("S3.scaled_norm1_eps_" + key, none_(sx))
        put("S3.scaled_norm2_eps_" + key, ntwo(sx))
    # stiff limit of the sweeps, iterated literally
    k = [mpf(0)] * N8
    u = [mpf(0)] * N8
    rk_inf, ru_inf = [], []
    for q in range(0, 9):
        rk_inf.append(1 + dotp(b, k))
        ru_inf.append(1 + dotp(bc, u))
        put("S3.kform_Rq_infinity_q%d" % q, rk_inf[-1])
        put("S3.Uform_Rq_infinity_q%d" % q, ru_inf[-1])
        xk = mv(x, k)
        k = [-1 / g - xk[i] for i in range(N8)]
        au = mv(a, u)
        u = [-1 - au[i] for i in range(N8)]
    # identity I - M_U = gamma (I - M_k) Gamma^-1 at z = -3/7
    z0 = mpf(-3) / 7
    den = 1 - g * z0
    mu_ = [[g * (z0 * a[i][j] + cm[i][j]) / den for j in range(N8)] for i in range(N8)]
    imk = [[(1 if i == j else 0) - z0 * nil[i][j] / den for j in range(N8)] for i in range(N8)]
    rhs = mm(imk, tb["Ginv"])
    ident_err = max(abs((1 if i == j else 0) - mu_[i][j] - g * rhs[i][j]) for i in range(N8) for j in range(N8))
    check("S3.identity", "I - M_U(z) = gamma (I - M_k(z)) Gamma^-1 (z = -3/7), and M_U != M_k",
          ident_err < mpf(10) ** -45 and abs(ninf(mu_) - abs(z0 / den) * ninf(nil)) > 1,
          "max error %s" % mp.nstr(ident_err, 5))
    # sweeps on the imaginary and the negative real axis, iterated literally
    old_dps = mp.dps
    mp.dps = 25
    sup = {(form, ax, q): mpf(0) for form in "kU" for ax in "ir" for q in range(1, 9)}
    for kk in range(-30, 51):
        y = mpf(10) ** (mpf(kk) / 10)
        for ax, z0 in (("i", mpc(0, y)), ("r", mpc(-y, 0))):
            den = 1 - g * z0
            kvec = [mpc(0)] * N8
            uvec = [mpc(0)] * N8
            for q in range(1, 9):
                nk = mv(nil, kvec)
                kvec = [(z0 + z0 * nk[i]) / den for i in range(N8)]
                au, cu = mv(a, uvec), mv(cm, uvec)
                uvec = [(g * z0 * (1 + au[i]) + g * cu[i]) / den for i in range(N8)]
                sup[("k", ax, q)] = max(sup[("k", ax, q)], abs(1 + dotp(b, kvec)))
                sup[("U", ax, q)] = max(sup[("U", ax, q)], abs(1 + dotp(bc, uvec)))
    mp.dps = old_dps
    for q in range(1, 9):
        put("S3.kform_sup_abs_Rq_imag_q%d" % q, sup[("k", "i", q)])
        put("S3.Uform_sup_abs_Rq_imag_q%d" % q, sup[("U", "i", q)])
        put("S3.kform_sup_abs_Rq_negreal_q%d" % q, sup[("k", "r", q)])
        put("S3.Uform_sup_abs_Rq_negreal_q%d" % q, sup[("U", "r", q)])
    check("S3.sweeps", "truncated sweeps: sup |R_q(iy)| > 1 for q = 1..6 (k-form) and q = 1..7 (U-form); "
          "exact after 7 resp. 8 sweeps (R_q(inf) = 0)",
          all(sup[("k", "i", q)] > 1 + mpf(10) ** -6 for q in range(1, 7))
          and all(sup[("U", "i", q)] > 1 + mpf(10) ** -6 for q in range(1, 8))
          and abs(rk_inf[7]) < mpf(10) ** -12 and abs(ru_inf[8]) < mpf(10) ** -40)


def f_w(y):
    return [y[0] * y[1] - y[0] + mpf(1) / 3, y[0] * y[0] - y[1] * y[1] / 2 + y[1]]


def j_w(y):
    return [[y[1] - 1, y[0]], [2 * y[0], 1 - y[1]]]


def step_kform(tb, y0, h, t):
    """Untransformed ROW/W form: (I - h g T) k_i = h f(y0 + sum alpha_ij k_j) + h T sum_{j<i} gamma_ij k_j."""
    g = tb["gamma"]
    w = mp.matrix([[(1 if i == j else 0) - h * g * t[i][j] for j in range(2)] for i in range(2)])
    ks = []
    for i in range(N8):
        arg = [y0[d] + mp.fsum(tb["alpha"][i][j] * ks[j][d] for j in range(i)) for d in range(2)]
        fv = f_w(arg)
        comb = [mp.fsum(tb["Gamma"][i][j] * ks[j][d] for j in range(i)) for d in range(2)]
        tc = mv(t, comb)
        sol = mp.lu_solve(w, mp.matrix([h * fv[d] + h * tc[d] for d in range(2)]))
        ks.append([sol[0], sol[1]])
    y1 = [y0[d] + mp.fsum(tb["b"][i] * ks[i][d] for i in range(N8)) for d in range(2)]
    ye = [y0[d] + mp.fsum(tb["bhat"][i] * ks[i][d] for i in range(N8)) for d in range(2)]
    g7 = [y0[d] + mp.fsum(tb["alpha"][6][i] * ks[i][d] for i in range(6)) for d in range(2)]
    return y1, ye, g7


def sec_s4(tb, tbb):
    alpha, gam, ai, gi = tb["alpha"], tb["Gamma"], tb["ai"], tb["gi"]
    conds = {}
    for name, w in (("main", tb["b"]), ("emb", tb["bhat"])):
        c = {"sum_w_minus_1": mp.fsum(w) - 1,
             "sum_w_gamma_i": dotp(w, gi),
             "sum_w_alpha_i_minus_half": dotp(w, ai) - mpf(1) / 2,
             "bushy_minus_third": mp.fsum(w[i] * ai[i] ** 2 for i in range(N8)) - mpf(1) / 3,
             "w_aa_minus_sixth": dotp(w, mv(alpha, ai)) - mpf(1) / 6,
             "w_ag": dotp(w, mv(alpha, gi)),
             "w_ga": dotp(w, mv(gam, ai)),
             "w_gg": dotp(w, mv(gam, gi))}
        conds[name] = c
        for k, v in c.items():
            put("S4.%s.%s" % (name, k), v)
    put("S4.main.sum_w_gamma_i_binary64_interpretation", dotp(tbb["b"], tbb["gi"]))
    coef = {}
    for name in ("main", "emb"):
        d = conds[name]
        coef[name] = {"h2_Ef": d["sum_w_gamma_i"], "h3_JEf": d["w_ag"] + d["w_gg"],
                      "h3_EJf": d["w_ga"] + d["w_gg"], "h3_EEf": d["w_gg"]}
        for k, v in coef[name].items():
            put("S4.pert.%s.%s" % (name, k), v)
    coef["est"] = {k: coef["main"][k] - coef["emb"][k] for k in coef["main"]}
    for k, v in coef["est"].items():
        put("S4.pert.est.%s" % k, v)
    y0 = [mpf(1) / 2, mpf(1) / 3]
    e = [[mpf(1) / 5, mpf(-1) / 7], [mpf(1) / 3, mpf(1) / 11]]
    jac, f0 = j_w(y0), f_w(y0)
    t = [[jac[i][j] + e[i][j] for j in range(2)] for i in range(2)]
    ef = mv(e, f0)
    jef, ejf, eef = mv(jac, ef), mv(e, mv(jac, f0)), mv(e, ef)
    prev, orders = None, {}
    for k in range(3, 9):
        h = mpf(1) / 2 ** k
        y1, ye, _ = step_kform(tb, y0, h, t)
        y1r, yer, _ = step_kform(tb, y0, h, jac)
        dm = [y1[d] - y1r[d] for d in range(2)]
        de = [ye[d] - yer[d] for d in range(2)]
        dd = {"main": dm, "emb": de, "est": [dm[d] - de[d] for d in range(2)]}
        cur = {}
        for name in ("main", "emb", "est"):
            cf = coef[name]
            pred = [h ** 2 * cf["h2_Ef"] * ef[d] + h ** 3 * (cf["h3_JEf"] * jef[d] + cf["h3_EJf"] * ejf[d]
                                                              + cf["h3_EEf"] * eef[d]) for d in range(2)]
            cur[name] = max(abs(v) for v in dd[name])
            put("S4.exp.%s.delta_h2pow%d" % (name, k), cur[name])
            put("S4.exp.%s.delta_minus_pred_h2pow%d" % (name, k),
                max(abs(dd[name][d] - pred[d]) for d in range(2)))
            if prev is not None:
                orders[name] = mp.log(prev[name] / cur[name], 2)
        prev = cur
    for name in ("main", "emb", "est"):
        put("S4.exp.%s_observed_order" % name, orders[name])
    lag = None
    for k in range(3, 9):
        h = mpf(1) / 2 ** k
        ylag = [y0[d] - h * f0[d] for d in range(2)]
        y1, ye, _ = step_kform(tb, y0, h, j_w(ylag))
        y1r, yer, _ = step_kform(tb, y0, h, jac)
        dm = max(abs(y1[d] - y1r[d]) for d in range(2))
        dest = max(abs((y1[d] - ye[d]) - (y1r[d] - yer[d])) for d in range(2))
        put("S4.lag.main.delta_h2pow%d" % k, dm)
        put("S4.lag.est.delta_h2pow%d" % k, dest)
        if lag is not None:
            lag_orders = (mp.log(lag[0] / dm, 2), mp.log(lag[1] / dest, 2))
        lag = (dm, dest)
    put("S4.lag.main_observed_order", lag_orders[0])
    put("S4.lag.est_observed_order", lag_orders[1])
    check("S4.lag", "lagged Jacobian (E = O(h)): main perturbation O(h^4), estimate O(h^3)",
          abs(lag_orders[0] - 4) < 0.1 and abs(lag_orders[1] - 3) < 0.1,
          "orders %s %s" % (mp.nstr(lag_orders[0], 6), mp.nstr(lag_orders[1], 6)))
    check("S4", "W-order: main sum b gamma_i = 0 (residual), embedded -0.0256; one-step experiment (k-form) "
          "shows orders 3 / 2 / 2 for main / embedded / estimate",
          abs(conds["main"]["sum_w_gamma_i"]) < mpf(10) ** -12 and abs(conds["emb"]["sum_w_gamma_i"]) > mpf(10) ** -3
          and abs(orders["main"] - 3) < 0.05 and abs(orders["emb"] - 2) < 0.05 and abs(orders["est"] - 2) < 0.05,
          "orders %s %s %s" % tuple(mp.nstr(orders[n], 6) for n in ("main", "emb", "est")))


def order_conditions(alpha, beta, w, ci):
    """The 17 ROW order conditions up to order 5, written out (label, order, density, value)."""
    a1 = [mp.fsum(r) for r in alpha]
    b1 = [mp.fsum(r) for r in beta]
    had = lambda u, v: [x * y for x, y in zip(u, v)]
    pw = lambda u, k: [x ** k for x in u]
    ab1 = mv(alpha, b1)
    rows = [
        ("t", 1, 1, mp.fsum(w)),
        ("[t]", 2, 2, dotp(w, b1)),
        ("[t,t]", 3, 3, dotp(w, pw(a1, 2))),
        ("[[t]]", 3, 6, dotp(w, mv(beta, b1))),
        ("[t,t,t]", 4, 4, dotp(w, pw(a1, 3))),
        ("[t,[t]]", 4, 8, dotp(w, had(a1, ab1))),
        ("[[t,t]]", 4, 12, dotp(w, mv(beta, pw(a1, 2)))),
        ("[[[t]]]", 4, 24, dotp(w, mv(beta, mv(beta, b1)))),
        ("[t,t,t,t]", 5, 5, dotp(w, pw(a1, 4))),
        ("[t,t,[t]]", 5, 10, dotp(w, had(pw(a1, 2), ab1))),
        ("[t,[t,t]]", 5, 15, dotp(w, had(a1, mv(alpha, pw(a1, 2))))),
        ("[t,[[t]]]", 5, 30, dotp(w, had(a1, mv(alpha, mv(beta, b1))))),
        ("[[t],[t]]", 5, 20, dotp(w, pw(ab1, 2))),
        ("[[t,t,t]]", 5, 20, dotp(w, mv(beta, pw(a1, 3)))),
        ("[[t,[t]]]", 5, 40, dotp(w, mv(beta, had(a1, ab1)))),
        ("[[[t,t]]]", 5, 60, dotp(w, mv(beta, mv(beta, pw(a1, 2))))),
        ("[[[[t]]]]", 5, 120, dotp(w, mv(beta, mv(beta, mv(beta, b1))))),
    ]
    return [(lab, o, val - ci ** o / mpf(d)) for lab, o, d, val in rows]


def sec_s5(tb, tbb):
    alpha, beta, b = tb["alpha"], tb["beta"], tb["b"]
    put("S5.alpha8_minus_beta7_max_abs", max(abs(alpha[7][j] - beta[6][j]) for j in range(N8)))
    put("S5.b_minus_beta8_max_abs", max(abs(b[j] - beta[7][j]) for j in range(N8)))
    put("S5.alpha7_minus_beta6_max_abs", max(abs(alpha[6][j] - beta[5][j]) for j in range(N8)))
    orders = {}

    def analyse(name, w, ci):
        res = order_conditions(alpha, beta, w, ci)
        p = 0
        for k in range(1, 6):
            if all(abs(r) < mpf(10) ** -12 for _, o, r in res if o == k):
                p = k
            else:
                break
        for lab, _, r in res:
            put("S5.%s.residual.%s" % (name, lab), r)
        put("S5.%s.classical_order" % name, p)
        orders[name] = p
        return res
    res_b = analyse("b_main", b, mpf(1))
    analyse("g8_bhat", alpha[7], mpf(1))
    analyse("g7", alpha[6], mpf(1))
    analyse("g6", alpha[5], mpf(1))
    for i in (1, 2, 3, 4):
        analyse("g%d_at_c%d" % (i + 1, i + 1), alpha[i], tb["ai"][i])
    put("S5.b_main.max_abs_residual_17", max(abs(r) for _, _, r in res_b))
    res_bb = order_conditions(tbb["alpha"], tbb["beta"], tbb["b"], mpf(1))
    put("S5.b_main.max_abs_residual_17_binary64_interpretation", max(abs(r) for _, _, r in res_bb))
    put("S5.g6_c", tb["ai"][5])
    y0 = [mpf(1) / 2, mpf(1) / 3]
    prev, so = None, None
    for k in range(2, 7):
        h = mpf(1) / 2 ** k
        y1, g8, g7 = step_kform(tb, y0, h, j_w(y0))
        ref = step_kform(tb, y1, h, j_w(y1))
        cur = []
        for name, pt in (("g7", g7), ("g8", g8)):
            alt = step_kform(tb, y1, h, j_w(pt))
            dm = max(abs(alt[0][d] - ref[0][d]) for d in range(2))
            dest = max(abs((alt[0][d] - alt[1][d]) - (ref[0][d] - ref[1][d])) for d in range(2))
            put("S5.spec.main.delta_%s_h2pow%d" % (name, k), dm)
            put("S5.spec.est.delta_%s_h2pow%d" % (name, k), dest)
            cur += [dm, dest]
        if prev is not None:
            so = [mp.log(prev[i] / cur[i], 2) for i in range(4)]
        prev = cur
    for lab, v in zip(("g7_main", "g7_est", "g8_main", "g8_est"), so):
        put("S5.spec.%s_observed_order" % lab, v)
    check("S5.spec", "speculative Jacobian at g7 / g8: solution perturbation O(h^7) / O(h^8), estimate O(h^6) / O(h^7)",
          abs(so[0] - 7) < 0.35 and abs(so[1] - 6) < 0.35 and abs(so[2] - 8) < 0.35 and abs(so[3] - 7) < 0.35,
          "orders " + ", ".join(mp.nstr(v, 5) for v in so))
    check("S5", "orders: main 5, g8 (embedded) 4, g7 3, g6 1; identities alpha_8 = beta_7, b = beta_8, "
          "alpha_7 = beta_6 to working precision",
          orders["b_main"] == 5 and orders["g8_bhat"] == 4 and orders["g7"] == 3 and orders["g6"] == 1
          and max(abs(alpha[7][j] - beta[6][j]) for j in range(N8)) < mpf(10) ** -50, str(orders))


def spec(r, k):
    return (r + 1) / (r + 1 - min(r, mpf(k) / 8))


def sec_s6():
    put("S6.max_speedup_two_stages", spec(mpf(1) / 4, 2))
    put("S6.max_speedup_one_stage", spec(mpf(1) / 8, 1))
    put("S6.speedup_at_r_3_20", spec(mpf(3) / 20, 2))
    put("S6.speedup_at_r_11_12", spec(mpf(11) / 12, 2))
    # band from the two branch equations: 1 + r = 23/20 and (r + 1)/(r + 3/4) = 23/20
    lower = mpf(23) / 20 - 1
    upper = (1 - mpf(23) / 20 * mpf(3) / 4) / (mpf(23) / 20 - 1)
    put("S6.band_lower", lower)
    put("S6.band_upper", upper)
    samples = [mpf(j) / 1000 for j in range(0, 4001)]
    ok = all((spec(r, 2) >= mpf(23) / 20 - mpf(10) ** -50) == (lower - mpf(10) ** -50 <= r <= upper + mpf(10) ** -50)
             for r in samples)
    check("S6", "speculation band [3/20, 11/12], maxima 5/4 and 9/8",
          ok and abs(max(spec(r, 2) for r in samples) - mpf(5) / 4) < mpf(10) ** -50
          and abs(max(spec(r, 1) for r in samples) - mpf(9) / 8) < mpf(10) ** -50)


def sec_s7():
    def model(kind, n, rho, jmode, bw):
        n = mpf(n)
        rhs = rho * n
        vec = 112 * n
        if kind == "dense":
            lu, solve = 2 * n ** 3 / 3, 2 * n ** 2
            jac = 3 * rhs if jmode == "analytic3" else n * rhs
        else:
            lu, solve = n * bw ** 2, 4 * n * bw
            jac = 3 * rhs if jmode == "analytic3" else (2 * bw + 1) * rhs
        chain = 8 * (rhs + solve) + vec
        total = jac + lu + chain
        fj, flu, fch = jac / total, lu / total, chain / total
        r = (jac + lu) / chain
        return {"f_J": fj, "f_LU": flu, "f_chain": fch, "r_JLU_over_chain": r,
                "bound_parallel_J_P6": 1 / (1 - fj * mpf(5) / 6), "bound_parallel_J_Pinf": 1 / (1 - fj),
                "bound_LU_reuse_m4": 1 / (1 - (fj + flu) * mpf(3) / 4), "bound_LU_reuse_minf": 1 / fch,
                "bound_speculation_2stages": spec(r, 2)}
    for n in (8, 64, 512, 4096):
        for rho in (20, 2000):
            for jm in ("analytic3", "fd"):
                for k, v in model("dense", n, rho, jm, None).items():
                    put("S7.dense.n%d.rho%d.%s.%s" % (n, rho, jm, k), v)
    for n, bw in ((400, 2), (4096, 64), (16384, 128), (65536, 256)):
        for rho in (20, 2000):
            for jm in ("analytic3", "fd"):
                for k, v in model("banded", n, rho, jm, bw).items():
                    put("S7.banded.n%d.rho%d.%s.b%d.%s" % (n, rho, jm, bw, k), v)
    for rho in (20, 2000):
        put("S7.dense_crossover_n_rho%d" % rho, 12 + mp.sqrt(144 + 12 * rho + 168))
        put("S7.banded_crossover_b_rho%d" % rho, 16 + mp.sqrt(256 + 8 * rho + 112))
    for kbar in (5, 20):
        for jv in (20, 2000):
            for num, den in ((1, 8), (1, 4), (1, 2), (1, 1)):
                theta = mpf(num) / den
                sep = 8 * kbar * jv + 16 * kbar ** 2
                shared = 8 * theta * kbar * jv + 128 * theta ** 2 * kbar ** 2
                key = "S7.krylov.k%d.jv%d.theta_%d_%d" % (kbar, jv, num, den)
                put(key + ".speedup", sep / shared)
                put(key + ".jvp_ratio", 1 / theta)
    # crossover roots really are the zeros of LU - chain
    ok = True
    for rho in (20, 2000):
        n = 12 + mp.sqrt(144 + 12 * rho + 168)
        ok = ok and abs(2 * n ** 3 / 3 - (8 * (rho * n + 2 * n * n) + 112 * n)) < mpf(10) ** -40
        bw = 16 + mp.sqrt(256 + 8 * rho + 112)
        ok = ok and abs(bw * bw - (8 * (rho + 4 * bw) + 112)) < mpf(10) ** -40
    check("S7", "cost model re-typed; crossover sizes are zeros of LU - chain", ok)


def rho_of(sigma, kappa=0):
    a = 1 + 2 * sigma - kappa
    return (a - mp.sqrt(a * a - 4 * sigma * sigma)) / (2 * sigma)


def cauchy_coeffs(func, nterms=8, radius=mpf(1) / 2, npts=160):
    """Taylor coefficients at 0 by the trapezoid rule on a circle (func analytic for |kappa| < 1)."""
    vals = []
    for j in range(npts):
        th = 2 * mp.pi * j / npts
        vals.append(func(radius * mp.expj(th)))
    out = []
    for k in range(nterms):
        acc = mp.fsum(vals[j] * mp.expj(-2 * mp.pi * j * k / npts) for j in range(npts)) / npts
        out.append((acc / radius ** k).real)
    return out


def phi_from(kp, coeffs):
    out = zeros(N8)
    for k, cf in enumerate(coeffs):
        out = [[out[i][j] + cf * kp[k][i][j] for j in range(N8)] for i in range(N8)]
    return out


def dirichlet_green_1d(sigma, n, src):
    """Closed form of the Green's function of tridiag(-sigma, 1+2sigma, -sigma), n points, source index src."""
    rho = rho_of(sigma)
    u = lambda k: rho ** (-k) - rho ** k
    scale = sigma * (1 / rho - rho) * u(n + 1)
    return [u(min(i, src) + 1) * u(n - max(i, src)) / scale for i in range(n)]


def sec_s8(tb):
    g = tb["gamma"]
    ok = True
    for m in (1, 2, 3):
        sigma = mpf(m * (m + 1))
        rho = rho_of(sigma)
        put("S8.rho_exact_sigma%d" % (m * (m + 1)), rho)
        ok = ok and abs(rho - mpf(m) / (m + 1)) < mpf(10) ** -50
    for sig, tag in ((mpf("0.25"), "0p25"), (mpf(1), "1p0"), (mpf(10), "10p0"), (mpf(100), "100p0"),
                     (mpf(10000), "10000p0")):
        ell = mp.sqrt(sig)
        rho = rho_of(sig)
        theta = 2 * mp.asinh(1 / (2 * ell))
        put("S8.sigma_%s.rho" % tag, rho)
        put("S8.sigma_%s.theta_times_ell" % tag, theta * ell)
        put("S8.sigma_%s.rho_vs_exp_theta_abs_diff" % tag, abs(rho - mp.exp(-theta)))
        ok = ok and abs(rho - mp.exp(-theta)) < mpf(10) ** -50
    check("S8.rho", "rho = m/(m+1) for sigma = m(m+1); rho = exp(-2 asinh(1/(2 ell))) to 50 digits", ok)
    npts = 121
    cidx = npts // 2
    gf = dirichlet_green_1d(mpf(6), npts, cidx)
    dev = max(abs(gf[cidx + d + 1] / gf[cidx + d] - mpf(2) / 3) for d in range(0, 20))
    put("S8.finite1d_max_ratio_deviation", dev)
    put("S8.finite1d_G0_times_sqrt", gf[cidx] * 5)
    # 2D: Dirichlet in x (sine modes), periodic in y (Fourier modes)
    old = mp.dps
    mp.dps = 30
    nx = ny = 31
    x0 = nx // 2
    for sig in (mpf(2), mpf(6)):
        sx = [[mp.sin(p * mp.pi * (x + 1) / (nx + 1)) for x in range(nx)] for p in range(1, nx + 1)]
        lam_x = [4 * sig * mp.sin(p * mp.pi / (2 * (nx + 1))) ** 2 for p in range(1, nx + 1)]
        lam_y = [4 * sig * mp.sin(mp.pi * q / ny) ** 2 for q in range(ny)]
        cq = [[mp.cos(2 * mp.pi * q * dy / ny) for dy in range(ny)] for q in range(ny)]
        tt = [[mp.fsum(cq[q][dy] / (1 + lam_x[p] + lam_y[q]) for q in range(ny)) / ny for dy in range(ny)]
              for p in range(nx)]

        def gv(x, dy):
            return 2 * mp.fsum(sx[p][x] * sx[p][x0] * tt[p][dy % ny] for p in range(nx)) / (nx + 1)
        axis = [gv(x0 + d + 1, 0) / gv(x0 + d, 0) for d in range(0, 8)]
        diag = [gv(x0 + d + 1, d + 1) / gv(x0 + d, d) for d in range(0, 6)]
        col = [mp.fsum(gv(x0 + d, dy) for dy in range(ny)) for d in range(0, 9)]
        g1 = dirichlet_green_1d(sig, nx, x0)
        col_dev = max(abs(col[d] - g1[x0 + d]) / g1[x0 + d] for d in range(0, 9))
        key = "S8.grid2d.sigma%d" % int(sig)
        for d, v in enumerate(axis):
            put(key + ".axis_ratio_d%d" % d, v)
        for d, v in enumerate(diag):
            put(key + ".diag_ratio_d%d" % d, v)
        for d, v in enumerate(col):
            put(key + ".colsum_d%d" % d, v)
        put(key + ".colsum_vs_1d_max_rel_dev", col_dev)
        rho1 = rho_of(sig)
        rho_d2 = mp.exp(-2 * mp.acosh(1 + 1 / (4 * sig)))
        put(key + ".rho_1d", rho1)
        put(key + ".rho_diag_sq", rho_d2)
        check("S8.grid2d.sigma%d" % int(sig), "2D (eigen-expansion): y-sums equal the 1D Green's function, axis "
              "ratios < rho_1D, diagonal ratios < rho_diag^2",
              col_dev < mpf(10) ** -20 and all(v < rho1 for v in axis) and all(v < rho_d2 for v in diag))
    mp.dps = old
    # block transfer matrix
    x = [[v / g for v in r] for r in tb["N"]]
    ipx = [[(1 if i == j else 0) + x[i][j] for j in range(N8)] for i in range(N8)]
    kmat = mm(lower_inverse(ipx), x)
    kp = [ident(N8)]
    for _ in range(7):
        kp.append(mm(kp[-1], kmat))
    for m in range(1, 8):
        put("S8.K_norminf_m%d" % m, ninf(kp[m]))
    # independent validation of Phi: long local chain (H = 160, sigma = 6) against Phi(d) E0
    sigma = mpf(6)
    hmax = 160
    e0 = [mpf(i + 1) / 7 * (-1) ** i for i in range(N8)]
    es = []
    for i in range(N8):
        comb = [mp.fsum(tb["N"][i][j] * es[j][d] for j in range(i)) for d in range(hmax + 1)]
        rhs = [(sigma / g) * (comb[d - 1] - 2 * comb[d] + comb[d + 1]) for d in range(1, hmax)]
        rhs[0] += sigma * e0[i]
        nn = hmax - 1
        cp, dp = [mpf(0)] * nn, [mpf(0)] * nn
        cp[0] = -sigma / (1 + 2 * sigma)
        dp[0] = rhs[0] / (1 + 2 * sigma)
        for r in range(1, nn):
            den = (1 + 2 * sigma) + sigma * cp[r - 1]
            cp[r] = -sigma / den
            dp[r] = (rhs[r] + sigma * dp[r - 1]) / den
        sol = [mpf(0)] * nn
        sol[-1] = dp[-1]
        for r in range(nn - 2, -1, -1):
            sol[r] = dp[r] - cp[r] * sol[r + 1]
        es.append([e0[i]] + sol + [mpf(0)])
    worst = mpf(0)
    for d in (1, 2, 5, 10, 20):
        cfs = cauchy_coeffs(lambda kap: rho_of(sigma, kap) ** d)
        pred = mv(phi_from(kp, cfs), e0)
        worst = max(worst, max(abs(pred[i] - es[i][d]) for i in range(N8)))
    check("S8.transfer", "local 8-stage chain on a long interval (H = 160, sigma = 6) reproduces Phi(d) E0 "
          "with Phi from contour-integral Taylor coefficients", worst < mpf(10) ** -30,
          "max deviation %s" % mp.nstr(worst, 5))
    gam, gam_inv = tb["Gamma"], tb["Ginv"]

    def scaled_phi(sig, dist):
        rho0 = rho_of(sig)
        return phi_from(kp, cauchy_coeffs(lambda kap: mp.exp(dist * (mp.log(rho_of(sig, kap)) - mp.log(rho0)))))
    for sig in (mpf(100), mpf(10000)):
        ell = mp.sqrt(sig)
        rho = rho_of(sig)
        for cmul in (1, 2, 4, 6, 8, 12, 16):
            delta = int(mp.nint(cmul * ell))
            ph = scaled_phi(sig, delta)
            cont = phi_from(kp, cauchy_coeffs(lambda kap: mp.exp(-cmul * (mp.sqrt(1 - kap) - 1))))
            key = "S8.amp.sigma%d.c%d" % (int(sig), cmul)
            put(key + ".delta", delta)
            put(key + ".rho_pow_delta", rho ** delta)
            put(key + ".amplification_norminf", ninf(ph))
            put(key + ".amplification_b_functional", mp.fsum(abs(v) for v in vm(tb["b"], ph)))
            put(key + ".amplification_norminf_Uvariables", ninf(mm(mm(gam, ph), gam_inv)))
            put(key + ".continuum_amplification_norminf", ninf(cont))
    sig = mpf(100)
    rho = rho_of(sig)
    for m in (1, 2, 3, 4, 6):
        halo = int(mp.ceil(m * mp.sqrt(sig) - mpf(10) ** -12))
        dist = 2 * halo + 1
        for q in (1, 2, 3, 4):
            ph = scaled_phi(sig, q * dist)
            put("S8.sweeps.sigma100.m%d.q%d.norminf_U" % (m, q), ninf(mm(mm(gam, ph), gam_inv)) * rho ** (q * dist))


def sec_s9(tb):
    def coeff_poly(ell, k):
        return [mp.binomial(ell, j) * (-1) ** j / mp.factorial(k - j) for j in range(0, min(ell, k) + 1)]

    def real_pos_roots(p):
        roots = mp.polyroots(list(reversed(p)), maxsteps=500, extraprec=400)
        return sorted(r.real for r in roots if abs(mp.im(r)) < mpf(10) ** -30 and mp.re(r) > 0)
    r44 = mp.polyroots(list(reversed(coeff_poly(4, 4))), maxsteps=500, extraprec=400)
    r45 = mp.polyroots(list(reversed(coeff_poly(4, 5))), maxsteps=500, extraprec=400)
    sep = min(abs(u - v) for u in r44 for v in r45)
    check("S9.L4", "L = 4: [z^4] and [z^5] of e^z (1-gz)^4 have no common root in gamma (order 5 with "
          "R(inf) = 0 impossible)", sep > mpf(10) ** -3, "min root distance %s" % mp.nstr(sep, 6))
    r4 = [r for r in real_pos_roots(coeff_poly(4, 5)) if mpf("0.01") < r < 5]
    for i, r in enumerate(r4):
        put("S9.L4_order5_gamma_root%d" % i, r)
    st4 = [a_stable(r, 4, 4) for r in r4]
    r5 = [r for r in real_pos_roots(coeff_poly(5, 5)) if mpf("0.01") < r < 5]
    for i, r in enumerate(r5):
        put("S9.L5_gamma_root%d" % i, r)
    st5 = [a_stable(r, 5, 4) for r in r5]
    check("S9.L5", "L = 5: five admissible gamma, exactly one A-stable (0.2780538...); L = 4 order 5: none",
          len(r5) == 5 and st5 == [False, False, True, False, False] and not any(st4)
          and abs(r5[2] - mpf("0.2780538411364523")) < mpf(10) ** -12, str([mp.nstr(r, 12) for r in r5]))

    def lin_defect(gam_, ell, deg, k):
        ser = series_div(trunc_numerator(gam_, ell, deg), ppow([mpf(1), -gam_], ell), k + 1)
        return ser[k] - 1 / mp.factorial(k)
    put("S9.L5_linear_defect_z6", lin_defect(r5[2], 5, 4, 6))
    put("S9.L7_rodas5p_linear_defect_z7", lin_defect(tb["gamma"], 7, 6, 7))

    def interval(ell, deg, inside):
        step = mpf(1) / 200
        lo = hi = inside
        while a_stable(lo - step, ell, deg):
            lo -= step
        while a_stable(hi + step, ell, deg):
            hi += step
        a_, b_ = lo - step, lo
        for _ in range(34):
            mid = (a_ + b_) / 2
            if a_stable(mid, ell, deg):
                b_ = mid
            else:
                a_ = mid
        c_, d_ = hi, hi + step
        for _ in range(34):
            mid = (c_ + d_) / 2
            if a_stable(mid, ell, deg):
                c_ = mid
            else:
                d_ = mid
        return b_, c_
    lo6, hi6 = interval(6, 5, mpf(1) / 4)
    lo7, hi7 = interval(7, 6, mpf(1) / 4)
    put("S9.L6_order5_Astable_gamma_lower", lo6)
    put("S9.L6_order5_Astable_gamma_upper", hi6)
    put("S9.L7_order6_Astable_gamma_lower", lo7)
    put("S9.L7_order6_Astable_gamma_upper", hi7)
    check("S9.windows", "A-stability windows (root-finding test): L6 [0.18391465, 0.33414237], "
          "L7 [0.20408345, 0.37886489]; RODAS5P gamma inside both",
          abs(lo6 - mpf("0.18391465")) < mpf(10) ** -7 and abs(hi6 - mpf("0.33414237")) < mpf(10) ** -7
          and abs(lo7 - mpf("0.20408345")) < mpf(10) ** -7 and abs(hi7 - mpf("0.37886489")) < mpf(10) ** -7
          and max(lo6, lo7) < tb["gamma"] < min(hi6, hi7))

    def count(sizes, stiff, ties):
        s = sum(sizes)
        pairs = s * (s - 1) // 2 - sum(k * (k - 1) // 2 for k in sizes)
        if not stiff:
            return 1 + 2 * pairs + s
        tot = 1 + 2 * pairs
        before = [sum(sizes[:l]) for l, k in enumerate(sizes) for _ in range(k)]
        for t in range(ties):
            tot -= before[s - 1 - t]
        return tot
    layouts = {"rodas5p_sequential_8x1": [1] * 8, "P2_L5_1-2-2-2-1": [1, 2, 2, 2, 1],
               "P2_L6_1-2-2-2-2-1": [1, 2, 2, 2, 2, 1], "P2_L5_1-2-2-2-2_not_stiffly_accurate": [1, 2, 2, 2, 2],
               "P3_L5_1-3-3-3-1": [1, 3, 3, 3, 1]}
    for name, sizes in layouts.items():
        vals = [sum(sizes), len(sizes), count(sizes, False, 0), count(sizes, True, 0), count(sizes, True, 1),
                count(sizes, True, 2)]
        for lab, v in zip(("stages", "levels", "general", "stiffly_accurate", "sa_one_tie", "sa_two_ties"), vals):
            put("S9.count.%s.%s" % (name, lab), v)
    put("S9.conditions.ode_order5", 17)
    put("S9.conditions.rodas5p_numbered_total", 17 + 23 + 9)


# ------------------------------------------------------------------ reporting
def md_table(columns, rows):
    def fmt(v):
        if v is None:
            return "-"
        if isinstance(v, bool):
            return "yes" if v else "no"
        if isinstance(v, float):
            return "%.6g" % v
        return str(v)
    out = ["| " + " | ".join(str(c) for c in columns) + " |", "|" + "---|" * len(columns)]
    for r in rows:
        out.append("| " + " | ".join(fmt(v) for v in r) + " |")
    return "\n".join(out)


def render_tables(res):
    prim = res["primary_fraction"]
    t = prim["tables"]
    v = prim["values"]
    f = lambda key, nd=10: "%.*g" % (nd, float(v[key]))
    L = ["# V05 result tables (generated by verify_independent.py from RESULTS.json; do not edit)", "",
         "Snapshot SHA-256: `%s`" % res["snapshot"]["sha256"], "",
         "Agreement of the two implementations: %d keys compared, %d mismatches, max |diff| exact-kind %s, "
         "numerical-kind %s." % (res["agreement"]["keys_compared"], len(res["agreement"]["mismatches"]),
                                 res["agreement"]["max_abs_diff_exact_kind"],
                                 res["agreement"]["max_scaled_diff_numerical_kind"]), ""]
    L += ["## Checks of the primary script (fractions.Fraction)", "",
          md_table(["id", "tag", "passed", "statement", "detail"],
                   [[c["id"], c["tag"], c["passed"], c["statement"], c["detail"]] for c in prim["checks"]]), ""]
    L += ["## Checks of the independent script (mpmath, 60 digits)", "",
          md_table(["id", "passed", "statement", "detail"],
                   [[c["id"], c["passed"], c["statement"], c["detail"]] for c in res["independent_mpmath"]["checks"]]),
          ""]
    L += ["## S1 subdiagonals", "",
          md_table(["i,j", "alpha_ij", "A_ij (code form)"],
                   [["%d,%d" % (i + 1, i), f("S1.alpha_subdiag_%d_%d" % (i + 1, i)),
                     f("S1.A_subdiag_%d_%d" % (i + 1, i))] for i in range(1, 8)]), ""]
    L += ["## S2 stability function", "",
          md_table(["k", "P coefficient (tableau)", "P coefficient (design trunc_6[e^z(1-gz)^7])",
                    "Taylor defect of R at z^k", "E_design coefficient of s^k", "E_represented coefficient of s^k"],
                   [[k, f("S2.P_coeff_%d" % k, 17), f("S2.Pdesign_coeff_%d" % k, 17) if k < 7 else "-",
                     f("S2.taylor_defect_%d" % k, 6) if k >= 1 else "-", f("S2.Edesign_coeff_%d" % k, 6),
                     f("S2.Erepresented_coeff_%d" % k, 6)] for k in range(8)]), "",
          "gamma = %s; beta21 = %s (binary64 interpretation %s); max|N^7| = %s; R(inf) = %s; "
          "grid max of |R(iy)|^2 - 1 = %s; embedded: R(inf) = %s, defect z^5 = %s, grid max = %s." % (
              v["S2.gamma"], f("S2.beta21", 6), f("S2.beta21_binary64_interpretation", 6), f("S2.N7_max_abs", 6),
              v["S2.R_infinity"], f("S2.grid_max_modR2_minus_1", 6), v["S2.emb_R_infinity"],
              f("S2.emb_taylor_defect_5", 6), f("S2.emb_grid_max_modR2_minus_1", 6)), ""]
    for name, title in (("S3.kform_norms_N_over_gamma", "S3 k-form: norms of (N/gamma)^m"),
                        ("S3.Uform_stiff_limit_norms_A", "S3 U-form stiff limit: norms of A^m"),
                        ("S3.Uform_z0_norms_gammaC", "S3 U-form at z = 0: norms of (gamma C)^m"),
                        ("S3.Uform_sup_over_negative_real_grid", "S3 U-form: sup over z <= 0 (grid) of norminf(M_U(z)^m)"),
                        ("S3.diagonal_scaling_family", "S3 diagonal scalings D = diag(eps^i) of N/gamma"),
                        ("S3.truncated_sweep_sup_abs_Rq", "S3 truncated sweeps: sup of abs R_q")):
        L += ["## " + title, "", md_table(t[name]["columns"], t[name]["rows"]), ""]
    ts = t["S3.truncated_sweep_R_at_infinity"]
    L += ["## S3 truncated sweeps: R_q(infinity)", "",
          md_table(["q", "k-form", "U-form"], [[a[0], a[1], b_[1]] for a, b_ in zip(ts["kform"], ts["Uform"])]), ""]
    L += ["## S4 W-conditions", "",
          md_table(["quantity", "main weights b", "embedded weights (row 7 of beta)"],
                   [[k, f("S4.main." + k, 12), f("S4.emb." + k, 12)] for k in (
                       "sum_w_minus_1", "sum_w_gamma_i", "sum_w_alpha_i_minus_half", "bushy_minus_third",
                       "w_aa_minus_sixth", "w_ag", "w_ga", "w_gg")]), "",
          md_table(["perturbation coefficient", "main", "embedded", "estimate (main - embedded)"],
                   [[k, f("S4.pert.main." + k, 12), f("S4.pert.emb." + k, 12), f("S4.pert.est." + k, 12)]
                    for k in ("h2_Ef", "h3_JEf", "h3_EJf", "h3_EEf")]), "",
          t["S4.inexact_jacobian_experiment"]["problem"], "",
          md_table(t["S4.inexact_jacobian_experiment"]["columns"], t["S4.inexact_jacobian_experiment"]["rows"]), ""]
    L += [t["S4.lagged_jacobian_experiment"]["problem"], "",
          md_table(t["S4.lagged_jacobian_experiment"]["columns"], t["S4.lagged_jacobian_experiment"]["rows"]), ""]
    oc = t["S5.order_conditions"]
    names = list(oc["weights"].keys())
    L += ["## S5 order-condition residuals (17 trees)", "",
          md_table(["tree", "order", "density"] + names,
                   [[oc["trees"][i], oc["orders"][i], oc["densities"][i]]
                    + ["%.3e" % oc["weights"][n]["residuals"][i] for n in names] for i in range(len(oc["trees"]))]),
          "", md_table(["weights", "classical order"], [[n, oc["weights"][n]["order"]] for n in names]), ""]
    L += [t["S5.speculative_jacobian_experiment"]["problem"], "",
          md_table(t["S5.speculative_jacobian_experiment"]["columns"],
                   t["S5.speculative_jacobian_experiment"]["rows"]), ""]
    L += ["## S7 cost model (MODEL, flop units)", "", t["S7.cost_model"]["label"], "",
          md_table(t["S7.cost_model"]["columns"], t["S7.cost_model"]["rows"]), "",
          t["S7.shared_krylov_model"]["label"], "",
          md_table(t["S7.shared_krylov_model"]["columns"], t["S7.shared_krylov_model"]["rows"]), ""]
    L += ["## S8 locality", "", md_table(t["S8.rho_vs_ell"]["columns"], t["S8.rho_vs_ell"]["rows"]), ""]
    for sig, d in t["S8.grid2d"].items():
        L += ["2D five-point, sigma = %s: rho_1D = %.6f, rho_diag^2 = %.6f" % (sig, d["rho_1d"], d["rho_diag_squared"]),
              "", md_table(["d", "axis ratio G(d+1,0)/G(d,0)", "y-sum ratio", "diagonal ratio G(d+1,d+1)/G(d,d)"],
                           [[i, d["axis_ratios"][i], d["colsum_ratios"][i],
                             d["diag_ratios"][i] if i < len(d["diag_ratios"]) else None] for i in range(8)]), ""]
    L += ["norminf(K^m), m = 1..7, K = N beta^-1: " + ", ".join(f("S8.K_norminf_m%d" % m, 5) for m in range(1, 8)), "",
          md_table(t["S8.block_transfer_amplification"]["columns"], t["S8.block_transfer_amplification"]["rows"]), "",
          t["S8.predicted_sweep_reduction"]["label"], "",
          md_table(t["S8.predicted_sweep_reduction"]["columns"], t["S8.predicted_sweep_reduction"]["rows"]), ""]
    L += ["## S9 counting and linear stability", "",
          md_table(t["S9.free_coefficients"]["columns"], t["S9.free_coefficients"]["rows"]), "",
          "Conditions: " + json.dumps(t["S9.free_coefficients"]["conditions"]), "",
          "L = 5 admissible gamma: " + ", ".join(f("S9.L5_gamma_root%d" % i, 12) for i in range(5)),
          "; L = 4 (order 5, no L-stability): " + ", ".join(f("S9.L4_order5_gamma_root%d" % i, 12) for i in range(4)),
          "", "A-stability windows: L = 6 [%s, %s], L = 7 [%s, %s]; linear defects: L = 5 z^6 %s, RODAS5P z^7 %s." % (
              f("S9.L6_order5_Astable_gamma_lower"), f("S9.L6_order5_Astable_gamma_upper"),
              f("S9.L7_order6_Astable_gamma_lower"), f("S9.L7_order6_Astable_gamma_upper"),
              f("S9.L5_linear_defect_z6", 6), f("S9.L7_rodas5p_linear_defect_z7", 6)), ""]
    with open(TABLES_MD, "w") as fh:
        fh.write("\n".join(L) + "\n")


def main():
    tb = build(mpf)
    tbb = build(lambda s: mpf(float(s)))
    sec_s1(tb)
    mu = sec_s2(tb, tbb)
    sec_s3(tb, mu)
    sec_s4(tb, tbb)
    sec_s5(tb, tbb)
    sec_s6()
    sec_s7()
    sec_s8(tb)
    sec_s9(tb)
    own_failed = [c["id"] for c in CHECKS if not c["passed"]]

    res = json.load(open(OUT))
    prim = res.get("primary_fraction")
    if not prim or prim.get("checks_failed"):
        print("RESULTS.json has no clean primary section; run verify_tableau.py first")
        return 2
    pv, kinds = prim["values"], prim["kinds"]
    mism, missing = [], []
    max_exact, max_num, max_rel = mpf(0), mpf(0), mpf(0)
    for key in sorted(pv):
        if key not in VAL:
            missing.append(key)
            continue
        a, b = mpf(pv[key]), mpf(VAL[key])
        diff = abs(a - b)
        scale = max(1, abs(a))
        kind = kinds[key]
        if kind == "x":
            max_exact = max(max_exact, diff / scale)
            okk = diff <= TOL_EXACT * scale
        elif kind == "nr":
            max_rel = max(max_rel, diff / abs(a))
            okk = diff <= mpf(10) ** -6 * abs(a)
        else:
            max_num = max(max_num, diff / scale)
            okk = diff <= TOL_NUM * scale
        if not okk:
            mism.append({"key": key, "kind": kind, "primary": pv[key], "independent": VAL[key],
                         "abs_diff": mp.nstr(diff, 6)})
    extra = sorted(k for k in VAL if k not in pv)
    res["independent_mpmath"] = {
        "implementation": "mpmath %s at %d digits; python %s" % (mpmath.__version__, mp.dps, platform.python_version()),
        "values": VAL, "checks": CHECKS, "checks_total": len(CHECKS), "checks_failed": own_failed}
    res["agreement"] = {
        "keys_compared": len(pv) - len(missing), "keys_primary": len(pv), "keys_independent": len(VAL),
        "missing_in_independent": missing, "extra_in_independent": extra, "mismatches": mism,
        "tolerance_exact_kind": "|a-b| <= 1e-40 * max(1,|a|)",
        "tolerance_numerical_kind": "|a-b| <= 1e-9 * max(1,|a|)",
        "tolerance_relative_kind": "|a-b| <= 1e-6 * |a|",
        "max_rel_diff_relative_kind": mp.nstr(max_rel, 6),
        "max_abs_diff_exact_kind": mp.nstr(max_exact, 6), "max_scaled_diff_numerical_kind": mp.nstr(max_num, 6),
        "pass": not mism and not missing and not own_failed}
    with open(OUT, "w") as fh:
        json.dump(res, fh, indent=1, allow_nan=False, sort_keys=True)
        fh.write("\n")
    render_tables(res)
    for c in CHECKS:
        print("%-4s %-18s %s%s" % ("PASS" if c["passed"] else "FAIL", c["id"], c["statement"],
                                   (" :: " + c["detail"]) if c["detail"] else ""))
    print("keys compared: %d, missing: %d, mismatches: %d, own failed checks: %d" % (
        len(pv) - len(missing), len(missing), len(mism), len(own_failed)))
    print("max scaled diff: exact-kind %s, numerical-kind %s" % (mp.nstr(max_exact, 6), mp.nstr(max_num, 6)))
    for m_ in mism[:40]:
        print("MISMATCH", m_)
    for k in missing[:40]:
        print("MISSING", k)
    return 0 if res["agreement"]["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
