#!/usr/bin/env python3
"""Loop L1 node V05 -- primary verification of the theory note (THEORY.md).

Exact rational arithmetic (fractions.Fraction) on the decimal strings of
fixtures/rodas5p_coefficients_snapshot.json.  The derived tableau is rebuilt
exactly as crates/rodas5p-core/src/coefficients.rs (lines 168-212) does:

    Gamma = (I/gamma - C)^-1,  alpha = A*Gamma,  beta = alpha + Gamma,
    N = beta - gamma*I,        b = Gamma^T * b_code.

Standard library only.  Writes the section "primary_fraction" of RESULTS.json
(next to this file) and resets the other sections; verify_independent.py then
adds its own section and the agreement report.

    python3 verify_tableau.py          # exit 0 iff every check passes

Value keys: every scalar is stored as a decimal string.  kind "x" = exact
rational arithmetic (rounded to 50 significant digits only for printing),
kind "n" = floating-point / iterative (numerical).
"""
import hashlib
import json
import math
import os
import platform
import re
import sys
from decimal import Decimal, getcontext
from fractions import Fraction as F

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SNAPSHOT = os.path.join(ROOT, "fixtures", "rodas5p_coefficients_snapshot.json")
COEFF_RS = os.path.join(ROOT, "crates", "rodas5p-core", "src", "coefficients.rs")
OUT = os.path.join(HERE, "RESULTS.json")
SIG = 50

VALUES = {}
KINDS = {}
CHECKS = []
TABLES = {}


def dec(x):
    """Decimal string with SIG significant digits (exact input: Fraction/int)."""
    if isinstance(x, float):
        if not math.isfinite(x):
            raise ValueError("non-finite float")
        return repr(x)
    x = F(x)
    if x == 0:
        return "0"
    getcontext().prec = SIG
    return str(Decimal(x.numerator) / Decimal(x.denominator))


def sci(x):
    return "%.6e" % float(x)


def put(key, x, kind="x"):
    if key in VALUES:
        raise KeyError("duplicate key " + key)
    if kind == "x" and isinstance(x, float):
        raise TypeError("exact key with float value: " + key)
    VALUES[key] = dec(x)
    KINDS[key] = kind
    return x


def check(cid, tag, statement, passed, detail=""):
    CHECKS.append({"id": cid, "tag": tag, "statement": statement,
                   "passed": bool(passed), "detail": detail})
    return passed


# ----------------------------------------------------------------- linear algebra
def eye(n, one=F(1), zero=F(0)):
    return [[one if i == j else zero for j in range(n)] for i in range(n)]


def matmul(a, b):
    n, m, p = len(a), len(b), len(b[0])
    return [[sum(a[i][k] * b[k][j] for k in range(m)) for j in range(p)] for i in range(n)]


def matvec(a, v):
    return [sum(a[i][k] * v[k] for k in range(len(v))) for i in range(len(a))]


def vecmat(v, a):
    return [sum(v[k] * a[k][j] for k in range(len(v))) for j in range(len(a[0]))]


def transpose(a):
    return [list(r) for r in zip(*a)]


def madd(a, b, sb=1):
    return [[a[i][j] + sb * b[i][j] for j in range(len(a[0]))] for i in range(len(a))]


def mscale(a, s):
    return [[s * x for x in r] for r in a]


def dot(u, v):
    return sum(x * y for x, y in zip(u, v))


def inverse(a):
    """Gauss-Jordan with exact pivots (works for Fraction and float)."""
    n = len(a)
    m = [list(a[i]) + [type(a[0][0])(1 if i == j else 0) for j in range(n)] for i in range(n)]
    for col in range(n):
        piv = max(range(col, n), key=lambda r: abs(m[r][col]))
        if m[piv][col] == 0:
            raise ZeroDivisionError("singular matrix")
        m[col], m[piv] = m[piv], m[col]
        p = m[col][col]
        m[col] = [x / p for x in m[col]]
        for r in range(n):
            if r != col and m[r][col] != 0:
                fct = m[r][col]
                m[r] = [x - fct * y for x, y in zip(m[r], m[col])]
    return [row[n:] for row in m]


def matpow(a, k):
    r = eye(len(a), type(a[0][0])(1), type(a[0][0])(0))
    for _ in range(k):
        r = matmul(r, a)
    return r


def norm_inf(a):
    return max(sum(abs(x) for x in r) for r in a)


def norm_one(a):
    return norm_inf(transpose(a))


def max_abs(a):
    return max(abs(x) for r in a for x in r)


def norm_two(a):
    """Spectral norm: cyclic Jacobi eigenvalue iteration on A^T A (floats)."""
    af = [[float(x) for x in r] for r in a]
    scale = max(abs(x) for r in af for x in r)
    if scale == 0.0:
        return 0.0
    af = [[x / scale for x in r] for r in af]
    g = matmul(transpose(af), af)
    n = len(g)
    for _ in range(100):
        off = sum(g[i][j] ** 2 for i in range(n) for j in range(n) if i != j)
        if off < 1e-60:
            break
        for p in range(n - 1):
            for q in range(p + 1, n):
                if abs(g[p][q]) < 1e-300:
                    continue
                theta = (g[q][q] - g[p][p]) / (2.0 * g[p][q])
                t = math.copysign(1.0, theta) / (abs(theta) + math.sqrt(theta * theta + 1.0))
                c = 1.0 / math.sqrt(t * t + 1.0)
                s = t * c
                for k in range(n):
                    gkp, gkq = g[k][p], g[k][q]
                    g[k][p] = c * gkp - s * gkq
                    g[k][q] = s * gkp + c * gkq
                for k in range(n):
                    gpk, gqk = g[p][k], g[q][k]
                    g[p][k] = c * gpk - s * gqk
                    g[q][k] = s * gpk + c * gqk
    return scale * math.sqrt(max(max(g[i][i] for i in range(n)), 0.0))


# ----------------------------------------------------------------- polynomials (low degree first)
def ptrim(p):
    p = list(p)
    while len(p) > 1 and p[-1] == 0:
        p.pop()
    return p


def padd(p, q, sq=1):
    n = max(len(p), len(q))
    return [(p[i] if i < len(p) else 0) + sq * (q[i] if i < len(q) else 0) for i in range(n)]


def pmul(p, q):
    r = [0 * p[0]] * (len(p) + len(q) - 1)
    for i, x in enumerate(p):
        if x == 0:
            continue
        for j, y in enumerate(q):
            r[i + j] = r[i + j] + x * y
    return r


def ppow(p, k):
    r = [p[0] * 0 + 1]
    for _ in range(k):
        r = pmul(r, p)
    return r


def pshift(p, k):
    return [p[0] * 0] * k + list(p)


def peval(p, x):
    r = p[-1]
    for cf in reversed(p[:-1]):
        r = r * x + cf
    return r


def pderiv(p):
    return [i * p[i] for i in range(1, len(p))] or [p[0] * 0]


def prem(a, b):
    a, b = ptrim(a), ptrim(b)
    a = list(a)
    while len(a) >= len(b) and any(x != 0 for x in a):
        fct = a[-1] / b[-1]
        sh = len(a) - len(b)
        for i, y in enumerate(b):
            a[i + sh] = a[i + sh] - fct * y
        a.pop()
        if not a:
            a = [F(0)]
            break
    return ptrim(a)


def sign_changes(vals):
    s = [1 if v > 0 else -1 for v in vals if v != 0]
    return sum(1 for i in range(1, len(s)) if s[i] != s[i - 1])


def sturm_count_positive(p):
    """Number of distinct real roots in (0, +inf); requires p(0) != 0."""
    p = ptrim(p)
    if p[0] == 0:
        raise ValueError("p(0) == 0")
    seq = [p, ptrim(pderiv(p))]
    while True:
        if len(seq[-1]) == 1 and seq[-1][0] == 0:
            seq.pop()
            break
        r = prem(seq[-2], seq[-1])
        if all(x == 0 for x in r):
            break
        seq.append([-x for x in r])
    at0 = [q[0] for q in seq]
    atinf = [q[-1] for q in seq]
    return sign_changes(at0) - sign_changes(atinf)


def exp_series(n):
    return [F(1, math.factorial(k)) for k in range(n + 1)]


def e_poly(p, gamma, ell):
    """Coefficients in s=y^2 of (1+gamma^2 s)^ell - |P(i y)|^2."""
    re = [p[2 * j] * (-1) ** j for j in range((len(p) + 1) // 2)]
    im = [p[2 * j + 1] * (-1) ** j for j in range(len(p) // 2)] or [p[0] * 0]
    mod2 = padd(pmul(re, re), pshift(pmul(im, im), 1))
    return padd(ppow([p[0] * 0 + 1, gamma * gamma], ell), mod2, -1)


def pade_like_numerator(gamma, ell, deg):
    """trunc_deg[ exp(z) * (1-gamma z)^ell ]."""
    full = pmul(exp_series(deg), ppow([F(1), -gamma], ell))
    return full[:deg + 1]


def a_stable(gamma, ell, deg):
    """A-stability of trunc_deg[e^z (1-gamma z)^ell]/(1-gamma z)^ell, gamma>0 rational."""
    e = ptrim(e_poly(pade_like_numerator(gamma, ell, deg), gamma, ell))
    low = 0
    while low < len(e) and e[low] == 0:
        low += 1
    if low == len(e):
        return True
    q = e[low:]
    if q[0] < 0:
        return False
    if len(q) == 1:
        return True
    return sturm_count_positive(q) == 0


# ----------------------------------------------------------------- rooted trees
def forests(n, _memo={}):
    if n in _memo:
        return _memo[n]
    if n == 0:
        res = {()}
    else:
        res = set()
        for k in range(1, n + 1):
            for t in trees(k):
                for f in forests(n - k):
                    res.add(tuple(sorted((t,) + f)))
    _memo[n] = res
    return res


def trees(n):
    return sorted(forests(n - 1))


def t_order(t):
    return 1 + sum(t_order(ch) for ch in t)


def t_density(t):
    r = t_order(t)
    for ch in t:
        r *= t_density(ch)
    return r


def t_label(t):
    return "t" if t == () else "[" + ",".join(t_label(ch) for ch in t) + "]"


def t_phi(t, alpha, beta):
    s = len(alpha)
    if t == ():
        return [F(1)] * s
    m = beta if len(t) == 1 else alpha
    out = [F(1)] * s
    for ch in t:
        v = matvec(m, t_phi(ch, alpha, beta))
        out = [x * y for x, y in zip(out, v)]
    return out


# ----------------------------------------------------------------- tableau
def build(parse):
    raw = json.load(open(SNAPSHOT))
    n = len(raw["c"])
    gamma = parse(raw["gamma"])
    a = [[parse(s) for s in r] for r in raw["A"]]
    cm = [[parse(s) for s in r] for r in raw["C"]]
    c = [parse(s) for s in raw["c"]]
    b_code = [parse(s) for s in raw["b_code"]]
    ginv = madd(mscale(eye(n), 1 / gamma), cm, -1)
    gam = inverse(ginv)
    alpha = matmul(a, gam)
    beta = madd(alpha, gam)
    nil = madd(beta, mscale(eye(n), gamma), -1)
    b = matvec(transpose(gam), b_code)
    return {"n": n, "gamma": gamma, "A": a, "C": cm, "c": c, "b_code": b_code,
            "Gamma": gam, "alpha": alpha, "beta": beta, "N": nil, "b": b,
            "bhat": list(alpha[n - 1]),
            "gamma_rows": [sum(r) for r in gam], "alpha_rows": [sum(r) for r in alpha]}


def stability_data(tb, w):
    """mu_m = w^T N^m 1 and the numerators of R(z) = 1 + z w^T (I - z beta)^-1 1."""
    n, gamma, nil = tb["n"], tb["gamma"], tb["N"]
    ones = [F(1)] * n
    mu, v = [], list(ones)
    for _ in range(n):
        mu.append(dot(w, v))
        v = matvec(nil, v)
    q1 = [F(1), -gamma]
    p7 = ppow(q1, 7)
    for m in range(7):
        p7 = padd(p7, pshift(pmul([mu[m]], ppow(q1, 6 - m)), m + 1))
    p8 = ppow(q1, 8)
    for m in range(8):
        p8 = padd(p8, pshift(pmul([mu[m]], ppow(q1, 7 - m)), m + 1))
    taylor, v = [], list(ones)
    for k in range(1, 9):
        taylor.append(dot(w, v) - F(1, math.factorial(k)))
        v = matvec(tb["beta"], v)
    rinf = 1 + sum(mu[m] * (-1 / gamma) ** (m + 1) for m in range(n))
    return {"mu": mu, "P7": p7, "P8": p8, "taylor_defect": taylor, "Rinf": rinf}


# ----------------------------------------------------------------- sections
def section_s1(tb):
    n, alpha, a = tb["n"], tb["alpha"], tb["A"]
    upper_zero = all(alpha[i][j] == 0 for i in range(n) for j in range(i, n))
    low = [(abs(alpha[i][j]), i + 1, j + 1) for i in range(n) for j in range(i)]
    nz = sum(1 for v, _, _ in low if v != 0)
    mn = min(low)
    put("S1.alpha_strict_lower_nonzero_count", nz)
    put("S1.alpha_min_abs_entry", mn[0])
    TABLES["S1.alpha_min_abs_entry_position"] = [mn[1], mn[2]]
    for i in range(1, n):
        put("S1.alpha_subdiag_%d_%d" % (i + 1, i), alpha[i][i - 1])
        put("S1.A_subdiag_%d_%d" % (i + 1, i), a[i][i - 1])
    a_nz = sum(1 for i in range(n) for j in range(i) if a[i][j] != 0)
    put("S1.A_strict_lower_nonzero_count", a_nz)
    put("S1.c_minus_alpha_rowsum_max_abs",
        max(abs(tb["c"][i] - tb["alpha_rows"][i]) for i in range(n)))
    # longest path of the dependency DAG "evaluation j feeds the argument of evaluation i"
    depth = [1] * n
    for i in range(n):
        for j in range(i):
            if alpha[i][j] != 0:
                depth[i] = max(depth[i], depth[j] + 1)
    put("S1.rhs_dependency_longest_path", max(depth))
    check("S1.a", "exact", "alpha is strictly lower triangular (diagonal and upper part exactly zero)",
          upper_zero)
    check("S1.b", "exact", "all 28 strictly-lower entries of alpha are nonzero (also of the code matrix A)",
          nz == 28 and a_nz == 28, "min |alpha_ij| = %s at (%d,%d)" % (sci(mn[0]), mn[1], mn[2]))
    check("S1.c", "exact", "the RHS dependency DAG has longest path 8 (total order)", max(depth) == 8)


def section_s2(tb, tb_bin):
    n, gamma, nil, b = tb["n"], tb["gamma"], tb["N"], tb["b"]
    put("S2.gamma", gamma)
    put("S2.alpha21_minus_3gamma", tb["alpha"][1][0] - 3 * gamma)
    put("S2.gamma21_plus_3gamma", tb["Gamma"][1][0] + 3 * gamma)
    put("S2.beta21", tb["beta"][1][0])
    put("S2.beta21_binary64_interpretation", tb_bin["beta"][1][0])
    check("S2.a", "exact", "alpha21 = 3*gamma exactly in the represented tableau",
          tb["alpha"][1][0] == 3 * gamma)
    check("S2.b", "zero by design within representation",
          "beta21 = alpha21 + gamma21 = 0 by design", abs(tb["beta"][1][0]) < F(1, 10 ** 15),
          "residual " + sci(tb["beta"][1][0]))
    n6, n7 = matpow(nil, 6), matpow(nil, 7)
    put("S2.N6_max_abs", max_abs(n6))
    put("S2.N6_nonzero_count", sum(1 for r in n6 for x in r if x != 0))
    put("S2.N7_max_abs", max_abs(n7))
    put("S2.N7_entry_8_1", n7[7][0])
    only81 = all(n7[i][j] == 0 for i in range(n) for j in range(n) if (i, j) != (7, 0))
    nd = [list(r) for r in nil]
    nd[1][0] = F(0)
    nd6, nd7 = matpow(nd, 6), matpow(nd, 7)
    put("S2.Ndesign6_max_abs", max_abs(nd6))
    check("S2.c", "exact", "N^7 has at most the entry (8,1), which equals the product of the subdiagonal",
          only81 and n7[7][0] == math.prod(nil[i + 1][i] for i in range(7)))
    check("S2.d", "exact", "with beta21 := 0 the matrix N has nilpotency index exactly 7 (N^6 != 0, N^7 = 0)",
          max_abs(nd7) == 0 and max_abs(nd6) != 0)
    check("S2.e", "zero by design within representation", "N^7 = 0 (nilpotency index 7)",
          max_abs(n7) < F(1, 10 ** 12), "residual max|N^7| = " + sci(max_abs(n7)))

    sd = stability_data(tb, b)
    for m, v in enumerate(sd["mu"]):
        put("S2.mu_%d" % m, v)
    p7, p8 = sd["P7"], sd["P8"]
    diff = padd(p8, pmul([F(1), -gamma], p7), -1)
    check("S2.f", "exact", "P8(z) = (1-gamma z) P7(z) + mu_7 z^8 (exact numerator identity)",
          all(diff[k] == 0 for k in range(8)) and diff[8] == sd["mu"][7])
    ok = True
    for z0 in (F(-1), F(1, 3), F(-1000), F(7, 2)):
        m = madd(eye(n), mscale(tb["beta"], z0), -1)
        r_direct = 1 + z0 * dot(b, matvec(inverse(m), [F(1)] * n))
        ok = ok and r_direct == peval(p8, z0) / (1 - gamma * z0) ** 8
    check("S2.g", "exact", "R(z) = 1 + z b^T (I - z beta)^-1 1 equals P8(z)/(1-gamma z)^8 at 4 rational points", ok)
    for k in range(8):
        put("S2.P_coeff_%d" % k, p7[k])
    pd = pade_like_numerator(gamma, 7, 6)
    for k in range(7):
        put("S2.Pdesign_coeff_%d" % k, pd[k])
    resid = max(abs(p7[k] - pd[k]) for k in range(7))
    put("S2.P_minus_Pdesign_max_abs", resid)
    put("S2.R_infinity", sd["Rinf"])
    b_is_beta8 = all(b[j] == tb["beta"][7][j] for j in range(n))
    check("S2.h", "exact", "R(infinity) = 0 exactly (b equals row 8 of beta exactly)",
          sd["Rinf"] == 0 and b_is_beta8)
    check("S2.i", "zero by design within representation",
          "deg P <= 6: coefficient of z^7 of the numerator over (1-gamma z)^7 vanishes",
          abs(p7[7]) < F(1, 10 ** 12), "residual " + sci(p7[7]))
    check("S2.j", "zero by design within representation",
          "P = trunc_6[e^z (1-gamma z)^7], i.e. P is fixed by gamma alone",
          resid < F(1, 10 ** 12), "max coefficient residual " + sci(resid))
    for k in range(1, 9):
        put("S2.taylor_defect_%d" % k, sd["taylor_defect"][k - 1])
    d = sd["taylor_defect"]
    check("S2.k", "zero by design within representation",
          "R(z) - exp(z) = O(z^7): Taylor defects of orders 1..6 vanish",
          max(abs(x) for x in d[:6]) < F(1, 10 ** 12),
          "max residual " + sci(max(abs(x) for x in d[:6])))
    check("S2.l", "exact", "the z^7 Taylor defect is nonzero (linear order is exactly 6)",
          abs(d[6]) > F(1, 10 ** 6), "defect_7 = " + sci(d[6]))
    # design stability function: exact A-stability via Sturm
    ed = e_poly(pd, gamma, 7)
    for k, v in enumerate(ed):
        put("S2.Edesign_coeff_%d" % k, v)
    low_zero = all(ed[k] == 0 for k in range(4))
    cubic = ed[4:]
    roots_pos = sturm_count_positive(cubic)
    put("S2.Edesign_positive_root_count", roots_pos)
    check("S2.m", "exact",
          "design R_d = trunc_6[e^z(1-gz)^7]/(1-gz)^7: E_d(s) = s^4 * cubic, cubic has no positive root "
          "and positive value at 0, hence |R_d(iy)| <= 1 for all real y (A- and L-stable)",
          low_zero and roots_pos == 0 and cubic[0] > 0)
    # represented tableau: exact E polynomial and exact grid
    e8 = e_poly(p8, gamma, 8)
    for k, v in enumerate(e8):
        put("S2.Erepresented_coeff_%d" % k, v)
    grid = [F(j) * F(10) ** k for k in range(-6, 7) for j in (1, 2, 3, 5, 7)]
    worst, worst_y = None, None
    worst_ge, worst_ge_y = None, None
    for y in grid:
        s = y * y
        val = -peval(e8, s) / (1 + gamma * gamma * s) ** 8  # |R(iy)|^2 - 1
        if worst is None or val > worst:
            worst, worst_y = val, y
        if y >= F(1, 100) and (worst_ge is None or val > worst_ge):
            worst_ge, worst_ge_y = val, y
    put("S2.grid_max_modR2_minus_1", worst)
    put("S2.grid_max_modR2_minus_1_for_y_ge_0p01", worst_ge)
    TABLES["S2.grid"] = {"points": len(grid), "y_min": dec(grid[0]), "y_max": dec(max(grid)),
                         "argmax_y": dec(worst_y), "argmax_y_ge_0p01": dec(worst_ge_y)}
    check("S2.n", "numerical",
          "represented R: |R(iy)|^2 - 1 <= 1e-15 on the rational grid (65 points, exact evaluation); a positive "
          "maximum is the representation residual of the coefficients of s^1..s^3 of E",
          worst < F(1, 10 ** 15), "max = %s at y = %s" % (sci(worst), dec(worst_y)))
    # embedded
    sh = stability_data(tb, tb["bhat"])
    put("S2.emb_R_infinity", sh["Rinf"])
    for k in range(1, 7):
        put("S2.emb_taylor_defect_%d" % k, sh["taylor_defect"][k - 1])
    e8h = e_poly(sh["P8"], gamma, 8)
    worst_h = max(-peval(e8h, y * y) / (1 + gamma * gamma * y * y) ** 8 for y in grid)
    put("S2.emb_grid_max_modR2_minus_1", worst_h)
    check("S2.o", "exact", "embedded: R_hat(infinity) = 0 exactly", sh["Rinf"] == 0)
    check("S2.p", "zero by design within representation",
          "embedded stability function has linear order exactly 4 (defects 1..4 vanish, defect 5 does not)",
          max(abs(x) for x in sh["taylor_defect"][:4]) < F(1, 10 ** 12)
          and abs(sh["taylor_defect"][4]) > F(1, 10 ** 6))
    check("S2.q", "numerical", "embedded, represented: |R_hat(iy)|^2 - 1 <= 1e-15 on the same grid",
          worst_h < F(1, 10 ** 15), "max = " + sci(worst_h))
    return sd


def section_s3(tb, sd):
    n, gamma, nil, a, cm = tb["n"], tb["gamma"], tb["N"], tb["A"], tb["C"]
    x = mscale(nil, 1 / gamma)
    rows = []
    xp = eye(n)
    for m in range(1, 8):
        xp = matmul(xp, x)
        n1, ni, n2 = norm_one(xp), norm_inf(xp), norm_two(xp)
        put("S3.kform_norm1_m%d" % m, n1)
        put("S3.kform_norminf_m%d" % m, ni)
        put("S3.kform_norm2_m%d" % m, n2, "n")
        put("S3.kform_bfunctional_norm_m%d" % m, sum(abs(v) for v in vecmat(tb["b"], xp)))
        rows.append([m, float(n1), n2, float(ni)])
    TABLES["S3.kform_norms_N_over_gamma"] = {"columns": ["m", "norm1", "norm2", "norminf"], "rows": rows}
    plan = [37, 24, 23, 20, 6.4, 2.3, 0]
    ok = all(abs(rows[m][3] - plan[m]) <= 0.5 if m < 4 else abs(rows[m][3] - plan[m]) <= 0.06
             for m in range(7))
    check("S3.a", "exact", "plan values ||(N/gamma)^m||_inf = 37, 24, 23, 20, 6.4, 2.3, 0 reproduce after rounding",
          ok, ", ".join("%.4g" % r[3] for r in rows))
    # U-form sweep of THEORY_R6 Theorem 1: M_U(z) = gamma (z A + C)/(1 - gamma z)
    ginv = madd(mscale(eye(n), 1 / gamma), cm, -1)
    ident_ok = True
    similar_norm_equal = True
    for z0 in (F(-1), F(-1000), F(1, 3)):
        den = 1 - gamma * z0
        mu_ = mscale(madd(mscale(a, z0), cm), gamma / den)
        mk = mscale(nil, z0 / den)
        rhs = madd(eye(n), mscale(matmul(madd(eye(n), mk, -1), ginv), gamma), -1)
        ident_ok = ident_ok and mu_ == rhs
        similar_norm_equal = similar_norm_equal and norm_inf(mu_) == norm_inf(mk)
    check("S3.b", "exact", "I - M_U(z) = gamma (I - M_k(z)) Gamma^-1 with M_k = z N/(1-gamma z), "
          "M_U = gamma (z A + C)/(1-gamma z)", ident_ok)
    check("S3.c", "exact", "REFUTED as literally posed: the R6 (U-variable) sweep matrix is not z/(1-gamma z) N "
          "(norms differ at the tested points); the formula holds for the k-variable sweep", not similar_norm_equal)
    m21 = [gamma * (z0 * a[1][0] + cm[1][0]) / (1 - gamma * z0) for z0 in (F(-1), F(-10 ** 6), F(1, 7))]
    put("S3.Uform_entry21_at_z_minus1", m21[0])
    check("S3.d", "zero by design within representation",
          "M_U(z)[2,1] = -3 for every z, hence the U-form sweep matrix has nilpotency index 8, not 7",
          all(abs(v + 3) < F(1, 10 ** 12) for v in m21))
    rows_a, rows_c = [], []
    ap, cp = eye(n), eye(n)
    gc = mscale(cm, gamma)
    for m in range(1, 9):
        ap, cp = matmul(ap, a), matmul(cp, gc)
        put("S3.Uform_stiff_norm1_m%d" % m, norm_one(ap))
        put("S3.Uform_stiff_norminf_m%d" % m, norm_inf(ap))
        put("S3.Uform_stiff_norm2_m%d" % m, norm_two(ap), "n")
        put("S3.Uform_z0_norminf_m%d" % m, norm_inf(cp))
        rows_a.append([m, float(norm_one(ap)), norm_two(ap), float(norm_inf(ap))])
        rows_c.append([m, float(norm_inf(cp))])
    TABLES["S3.Uform_stiff_limit_norms_A"] = {"columns": ["m", "norm1", "norm2", "norminf"], "rows": rows_a}
    TABLES["S3.Uform_z0_norms_gammaC"] = {"columns": ["m", "norminf"], "rows": rows_c}
    zgrid = [-F(j) * F(10) ** k for k in range(-3, 7) for j in (1, 2, 5)]
    rows_sup = []
    for m in range(1, 9):
        best, arg = None, None
        for z0 in zgrid:
            mu_ = mscale(madd(mscale(a, z0), cm), gamma / (1 - gamma * z0))
            v = norm_inf(matpow(mu_, m))
            if best is None or v > best:
                best, arg = v, z0
        put("S3.Uform_grid_sup_norminf_m%d" % m, best)
        rows_sup.append([m, float(best), float(arg)])
    TABLES["S3.Uform_sup_over_negative_real_grid"] = {"columns": ["m", "sup norminf", "argmax z"],
                                                      "rows": rows_sup, "grid_points": len(zgrid)}
    # diagonal scalings: D = diag(eps^i), the infimum of ||D X D^-1|| is 0
    rows_d = []
    for eps in (F(1), F(1, 2), F(1, 4), F(1, 10), F(1, 100)):
        d = [eps ** i for i in range(n)]
        sx = [[d[i] * x[i][j] / d[j] for j in range(n)] for i in range(n)]
        key = "%d_%d" % (eps.numerator, eps.denominator)
        put("S3.scaled_norminf_eps_" + key, norm_inf(sx))
        put("S3.scaled_norm1_eps_" + key, norm_one(sx))
        put("S3.scaled_norm2_eps_" + key, norm_two(sx), "n")
        rows_d.append([float(eps), float(norm_one(sx)), norm_two(sx), float(norm_inf(sx)), float(eps ** -(n - 1))])
    TABLES["S3.diagonal_scaling_family"] = {
        "columns": ["eps", "norm1", "norm2", "norminf", "cond(D)"], "rows": rows_d}
    check("S3.e", "exact", "REFUTED: min over diagonal scalings of ||D (N/gamma) D^-1|| is not bounded away "
          "from zero (infimum 0, strictly lower triangular); the m=1 norm is scaling dependent",
          rows_d[-1][3] < 1.0 and rows_d[0][3] > 30.0)
    # scaling-invariant quantity: stability function of the truncated sweep at infinity
    rq, ru = [], []
    for q in range(0, 9):
        val = 1 + sum(sd["mu"][m] * (-1 / gamma) ** (m + 1) for m in range(min(q, 8)))
        put("S3.kform_Rq_infinity_q%d" % q, val)
        rq.append([q, float(val)])
    ones = [F(1)] * n
    u = [F(0)] * n
    for q in range(0, 9):
        val = 1 + dot(tb["b_code"], u)
        put("S3.Uform_Rq_infinity_q%d" % q, val)
        ru.append([q, float(val)])
        u = [-(o + w) for o, w in zip(ones, matvec(a, u))]
    TABLES["S3.truncated_sweep_R_at_infinity"] = {"columns": ["q sweeps", "R_q(inf)"],
                                                   "kform": rq, "Uform": ru}
    check("S3.f", "exact", "truncated sweeps at infinity: |R_q(inf)| > 1 for q in {1,2,3,5,6} (k-form) and "
          "q = 1..6 (U-form); |R_4(inf)| < 1 (k-form) and |R_7(inf)| < 1 (U-form); exact (0) after 7 resp. 8 sweeps",
          all(abs(rq[q][1]) > 1 for q in (1, 2, 3, 5, 6)) and abs(rq[4][1]) < 1
          and all(abs(ru[q][1]) > 1 for q in range(1, 7)) and abs(ru[7][1]) < 1
          and abs(rq[7][1]) < 1e-9 and ru[8][1] == 0.0,
          "k-form " + ", ".join("%.4g" % r[1] for r in rq) + " | U-form " + ", ".join("%.4g" % r[1] for r in ru))
    # sup over the imaginary axis and the negative real axis (floats)
    b_f = [float(v) for v in tb["b_code"]]
    a_f = [[float(v) for v in r] for r in a]
    c_f = [[float(v) for v in r] for r in cm]
    g_f = float(gamma)
    mu_f = [float(v) for v in sd["mu"]]
    ygrid = [10.0 ** (k / 10.0) for k in range(-30, 51)]
    rows_i = []
    for q in range(1, 9):
        best_k = best_u = best_kr = best_ur = 0.0
        for y in ygrid:
            for z0, real in ((complex(0.0, y), False), (complex(-y, 0.0), True)):
                w = z0 / (1 - g_f * z0)
                rk = 1 + sum(mu_f[m] * w ** (m + 1) for m in range(q))
                uvec = [0j] * n
                for _ in range(q):
                    au = [sum(a_f[i][j] * uvec[j] for j in range(n)) for i in range(n)]
                    cu = [sum(c_f[i][j] * uvec[j] for j in range(n)) for i in range(n)]
                    uvec = [(g_f * z0 * (1 + au[i]) + g_f * cu[i]) / (1 - g_f * z0) for i in range(n)]
                ruv = 1 + sum(b_f[i] * uvec[i] for i in range(n))
                if real:
                    best_kr, best_ur = max(best_kr, abs(rk)), max(best_ur, abs(ruv))
                else:
                    best_k, best_u = max(best_k, abs(rk)), max(best_u, abs(ruv))
        put("S3.kform_sup_abs_Rq_imag_q%d" % q, best_k, "n")
        put("S3.Uform_sup_abs_Rq_imag_q%d" % q, best_u, "n")
        put("S3.kform_sup_abs_Rq_negreal_q%d" % q, best_kr, "n")
        put("S3.Uform_sup_abs_Rq_negreal_q%d" % q, best_ur, "n")
        rows_i.append([q, best_k, best_kr, best_u, best_ur])
    check("S3.g", "numerical", "no truncated sweep count is A-stable: sup_y |R_q(iy)| > 1 on the grid for "
          "q = 1..6 (k-form) and q = 1..7 (U-form); q = 4 (k-form) is marginal",
          all(r[1] > 1.0 + 1e-6 for r in rows_i[:6]) and all(r[3] > 1.0 + 1e-6 for r in rows_i[:7])
          and abs(rows_i[6][1] - 1.0) < 1e-3 and abs(rows_i[7][3] - 1.0) < 1e-3,
          "k-form " + ", ".join("%.4g" % r[1] for r in rows_i) + " | U-form "
          + ", ".join("%.4g" % r[3] for r in rows_i))
    TABLES["S3.truncated_sweep_sup_abs_Rq"] = {
        "columns": ["q", "k-form sup abs R_q(iy)", "k-form sup abs R_q(-x)", "U-form sup abs R_q(iy)", "U-form sup abs R_q(-x)"],
        "rows": rows_i, "grid": "y, x = 10^(k/10), k = -30..50"}


W_F = [lambda y: [y[0] * y[1] - y[0] + F(1, 3), y[0] * y[0] - y[1] * y[1] / 2 + y[1]],
       lambda y: [[y[1] - 1, y[0]], [2 * y[0], 1 - y[1]]]]
W_Y0 = [F(1, 2), F(1, 3)]
W_E = [[F(1, 5), F(-1, 7)], [F(1, 3), F(1, 11)]]


def w_step_uform(tb, h, tmat, ystart=None):
    """One step in the code (U) form with an arbitrary matrix T in place of the Jacobian.

    Returns (y_new, y_embedded = g8, g7)."""
    n, gamma, a, cm = tb["n"], tb["gamma"], tb["A"], tb["C"]
    y0 = W_Y0 if ystart is None else ystart
    w = madd(eye(2), mscale(tmat, h * gamma), -1)
    det = w[0][0] * w[1][1] - w[0][1] * w[1][0]
    us = []
    for i in range(n):
        g = [y0[d] + sum(a[i][j] * us[j][d] for j in range(i)) for d in range(2)]
        fv = W_F[0](g)
        r = [h * gamma * fv[d] + gamma * sum(cm[i][j] * us[j][d] for j in range(i)) for d in range(2)]
        us.append([(r[0] * w[1][1] - w[0][1] * r[1]) / det, (w[0][0] * r[1] - w[1][0] * r[0]) / det])
    g7 = [y0[d] + sum(a[n - 2][j] * us[j][d] for j in range(n - 2)) for d in range(2)]
    y_emb = [y0[d] + sum(a[n - 1][j] * us[j][d] for j in range(n - 1)) for d in range(2)]
    y_new = [y0[d] + sum(tb["b_code"][j] * us[j][d] for j in range(n)) for d in range(2)]
    return y_new, y_emb, g7


def section_s4(tb, tb_bin):
    n, alpha, gam, b, bh = tb["n"], tb["alpha"], tb["Gamma"], tb["b"], tb["bhat"]
    ai, gi = tb["alpha_rows"], tb["gamma_rows"]

    def wconds(w):
        return {
            "sum_w_minus_1": sum(w) - 1,
            "sum_w_gamma_i": dot(w, gi),
            "sum_w_alpha_i_minus_half": dot(w, ai) - F(1, 2),
            "bushy_minus_third": sum(w[i] * ai[i] ** 2 for i in range(n)) - F(1, 3),
            "w_aa_minus_sixth": dot(w, matvec(alpha, ai)) - F(1, 6),
            "w_ag": dot(w, matvec(alpha, gi)),
            "w_ga": dot(w, matvec(gam, ai)),
            "w_gg": dot(w, matvec(gam, gi)),
        }
    wb, wh = wconds(b), wconds(bh)
    for name, d in (("main", wb), ("emb", wh)):
        for k, v in d.items():
            put("S4.%s.%s" % (name, k), v)
    put("S4.main.sum_w_gamma_i_binary64_interpretation", dot(tb_bin["b"], tb_bin["gamma_rows"]))
    chain = dot(b, matvec(tb["beta"], [x + y for x, y in zip(ai, gi)])) - F(1, 6)
    check("S4.a", "exact", "ROW chain condition = sum of the four coloured W-trees",
          chain == wb["w_aa_minus_sixth"] + wb["w_ag"] + wb["w_ga"] + wb["w_gg"])
    check("S4.b", "zero by design within representation",
          "main weights: sum b_i gamma_i = 0 and sum b_i alpha_i = 1/2 (W-order 2)",
          abs(wb["sum_w_gamma_i"]) < F(1, 10 ** 12) and abs(wb["sum_w_alpha_i_minus_half"]) < F(1, 10 ** 12),
          "residual " + sci(wb["sum_w_gamma_i"]))
    check("S4.c", "exact", "main weights fail the order-3 W-trees (defects of size 1e-2), W-order is exactly 2",
          min(abs(wb["w_ag"]), abs(wb["w_ga"]), abs(wb["w_gg"])) > F(1, 1000),
          "w_ag %s, w_ga %s, w_gg %s" % (sci(wb["w_ag"]), sci(wb["w_ga"]), sci(wb["w_gg"])))
    check("S4.d", "exact", "embedded weights fail the order-2 W condition: sum bhat_i gamma_i != 0 (W-order 1)",
          abs(wh["sum_w_gamma_i"]) > F(1, 1000), "sum bhat_i gamma_i = " + sci(wh["sum_w_gamma_i"]))
    # leading perturbation coefficients for T = J + E
    coef = {}
    for name, d in (("main", wb), ("emb", wh)):
        coef[name] = {"h2_Ef": d["sum_w_gamma_i"], "h3_JEf": d["w_ag"] + d["w_gg"],
                      "h3_EJf": d["w_ga"] + d["w_gg"], "h3_EEf": d["w_gg"]}
        for k, v in coef[name].items():
            put("S4.pert.%s.%s" % (name, k), v)
    for k in coef["main"]:
        put("S4.pert.est.%s" % k, coef["main"][k] - coef["emb"][k])
    # exact one-step experiment (2-dimensional polynomial ODE, fixed E)
    jac = W_F[1](W_Y0)
    f0 = W_F[0](W_Y0)
    tmat = madd(jac, W_E)
    ef = matvec(W_E, f0)
    jef = matvec(jac, ef)
    ejf = matvec(W_E, matvec(jac, f0))
    eef = matvec(W_E, ef)
    rows = []
    prev = None
    for k in range(3, 9):
        h = F(1, 2 ** k)
        y1, ye, _ = w_step_uform(tb, h, tmat)
        y1r, yer, _ = w_step_uform(tb, h, jac)
        dm = [y1[d] - y1r[d] for d in range(2)]
        de = [ye[d] - yer[d] for d in range(2)]
        dest = [dm[d] - de[d] for d in range(2)]
        res = {}
        for name, dv in (("main", dm), ("emb", de), ("est", dest)):
            if name == "est":
                cf = {kk: coef["main"][kk] - coef["emb"][kk] for kk in coef["main"]}
            else:
                cf = coef[name]
            pred = [h ** 2 * cf["h2_Ef"] * ef[d] + h ** 3 * (cf["h3_JEf"] * jef[d] + cf["h3_EJf"] * ejf[d]
                                                              + cf["h3_EEf"] * eef[d]) for d in range(2)]
            res[name] = (max(abs(v) for v in dv), max(abs(dv[d] - pred[d]) for d in range(2)))
            put("S4.exp.%s.delta_h2pow%d" % (name, k), res[name][0])
            put("S4.exp.%s.delta_minus_pred_h2pow%d" % (name, k), res[name][1])
        row = [k]
        for name in ("main", "emb", "est"):
            if prev is None:
                row += [float(res[name][0]), None, float(res[name][1] / h ** 4)]
            else:
                row += [float(res[name][0]), math.log2(float(prev[name][0] / res[name][0])),
                        float(res[name][1] / h ** 4)]
        rows.append(row)
        prev = res
    TABLES["S4.inexact_jacobian_experiment"] = {
        "problem": "y' = (y1 y2 - y1 + 1/3, y1^2 - y2^2/2 + y2), y0 = (1/2, 1/3), "
                   "T = J(y0) + E, E = [[1/5,-1/7],[1/3,1/11]], h = 2^-k",
        "columns": ["k", "main |delta|", "main order", "main |delta-pred|/h^4",
                    "emb |delta|", "emb order", "emb |delta-pred|/h^4",
                    "est |delta|", "est order", "est |delta-pred|/h^4"],
        "rows": rows}
    last = rows[-1]
    put("S4.exp.main_observed_order", last[2], "n")
    put("S4.exp.emb_observed_order", last[5], "n")
    put("S4.exp.est_observed_order", last[8], "n")
    check("S4.e", "numerical", "exact one-step experiment: perturbation by a fixed Jacobian error is O(h^3) in the "
          "main solution, O(h^2) in the embedded solution and in the error estimate; the h^2/h^3 prediction "
          "leaves an O(h^4) remainder",
          abs(last[2] - 3) < 0.05 and abs(last[5] - 2) < 0.05 and abs(last[8] - 2) < 0.05
          and all(rows[-1][i] < 2 * rows[-2][i] + 1e-30 for i in (3, 6, 9)),
          "orders %.4f, %.4f, %.4f" % (last[2], last[5], last[8]))


    # lagged Jacobian: T = J(y0 - h f(y0)), i.e. E = O(h)
    rows_l, prev = [], None
    for k in range(3, 9):
        h = F(1, 2 ** k)
        ylag = [W_Y0[d] - h * f0[d] for d in range(2)]
        y1, ye, _ = w_step_uform(tb, h, W_F[1](ylag))
        y1r, yer, _ = w_step_uform(tb, h, jac)
        dm = max(abs(y1[d] - y1r[d]) for d in range(2))
        dest = max(abs((y1[d] - ye[d]) - (y1r[d] - yer[d])) for d in range(2))
        put("S4.lag.main.delta_h2pow%d" % k, dm)
        put("S4.lag.est.delta_h2pow%d" % k, dest)
        rows_l.append([k, float(dm), None if prev is None else math.log2(float(prev[0] / dm)),
                       float(dest), None if prev is None else math.log2(float(prev[1] / dest))])
        prev = (dm, dest)
    TABLES["S4.lagged_jacobian_experiment"] = {
        "problem": "same ODE; T = J(y0 - h f(y0)) (a Jacobian that is one step old), h = 2^-k",
        "columns": ["k", "main |delta|", "main order", "est |delta|", "est order"], "rows": rows_l}
    put("S4.lag.main_observed_order", rows_l[-1][2], "n")
    put("S4.lag.est_observed_order", rows_l[-1][4], "n")
    check("S4.f", "numerical", "a Jacobian lagged by O(h) perturbs the main solution by O(h^4) per step and the "
          "error estimate by O(h^3): the lagged method has classical order 3, not 5",
          abs(rows_l[-1][2] - 4) < 0.1 and abs(rows_l[-1][4] - 3) < 0.1,
          "orders %.4f, %.4f" % (rows_l[-1][2], rows_l[-1][4]))


def section_s5(tb, tb_bin):
    n, alpha, beta, b = tb["n"], tb["alpha"], tb["beta"], tb["b"]
    id1 = max(abs(alpha[7][j] - beta[6][j]) for j in range(n))
    id2 = max(abs(b[j] - beta[7][j]) for j in range(n))
    id3 = max(abs(alpha[6][j] - beta[5][j]) for j in range(n))
    put("S5.alpha8_minus_beta7_max_abs", id1)
    put("S5.b_minus_beta8_max_abs", id2)
    put("S5.alpha7_minus_beta6_max_abs", id3)
    check("S5.a", "exact", "alpha_8 = beta_7, b = beta_8, alpha_7 = beta_6 hold exactly in the represented tableau",
          id1 == 0 and id2 == 0 and id3 == 0)
    all_trees = [t for k in range(1, 6) for t in trees(k)]
    counts = [len(trees(k)) for k in range(1, 6)]
    check("S5.b", "exact", "rooted trees of order 1..5: 1, 1, 2, 4, 9 (17 in total)", counts == [1, 1, 2, 4, 9])
    phis = [(t, t_order(t), t_density(t), t_phi(t, alpha, beta)) for t in all_trees]
    phis_bin = [(t, t_order(t), t_density(t), t_phi(t, tb_bin["alpha"], tb_bin["beta"])) for t in all_trees]
    table = {"trees": [t_label(t) for t in all_trees], "orders": [o for _, o, _, _ in phis],
             "densities": [d for _, _, d, _ in phis], "weights": {}}
    orders = {}
    tol = F(1, 10 ** 12)

    def analyse(name, w, ci):
        res = [dot(w, ph) - F(ci) ** o / d for (_, o, d, ph) in phis]
        p = 0
        for k in range(1, 6):
            if all(abs(r) < tol for r, (_, o, _, _) in zip(res, phis) if o == k):
                p = k
            else:
                break
        by_order = [max(abs(r) for r, (_, o, _, _) in zip(res, phis) if o == k) for k in range(1, 6)]
        for (t, _, _, _), r in zip(phis, res):
            put("S5.%s.residual.%s" % (name, t_label(t)), r)
        put("S5.%s.classical_order" % name, p)
        table["weights"][name] = {"order": p, "max_abs_residual_by_tree_order": [float(v) for v in by_order],
                                  "residuals": [float(r) for r in res]}
        orders[name] = p
        return res
    res_b = analyse("b_main", b, 1)
    analyse("g8_bhat", alpha[7], 1)
    analyse("g7", alpha[6], 1)
    analyse("g6", alpha[5], 1)
    for i in (1, 2, 3, 4):
        analyse("g%d_at_c%d" % (i + 1, i + 1), alpha[i], tb["alpha_rows"][i])
    res_b_bin = [dot(tb_bin["b"], ph) - F(1, d) for (_, o, d, ph) in phis_bin]
    put("S5.b_main.max_abs_residual_17", max(abs(r) for r in res_b))
    put("S5.b_main.max_abs_residual_17_binary64_interpretation", max(abs(r) for r in res_b_bin))
    TABLES["S5.order_conditions"] = table
    check("S5.c", "zero by design within representation", "main weights satisfy all 17 ROW conditions (order 5)",
          orders["b_main"] == 5, "max residual " + sci(max(abs(r) for r in res_b)))
    check("S5.d", "zero by design within representation",
          "g8 (weights alpha_8 = beta_7, the embedded solution) has classical order exactly 4", orders["g8_bhat"] == 4)
    check("S5.e", "zero by design within representation",
          "g7 (weights alpha_7 = beta_6) has classical order exactly 3", orders["g7"] == 3)
    put("S5.g6_c", tb["alpha_rows"][5])
    TABLES["S5.stage_argument_orders"] = orders
    # two-step experiment: the Jacobian of step n+1 is taken at g7 or g8 of step n instead of y_{n+1}
    rows_s, prev = [], None
    cap = 10 ** 40
    for k in range(2, 7):
        h = F(1, 2 ** k)
        y1, g8, g7 = w_step_uform(tb, h, W_F[1](W_Y0))
        y1 = [v.limit_denominator(cap) for v in y1]
        g8 = [v.limit_denominator(cap) for v in g8]
        g7 = [v.limit_denominator(cap) for v in g7]
        ref = w_step_uform(tb, h, W_F[1](y1), y1)
        cur = []
        for name, pt in (("g7", g7), ("g8", g8)):
            alt = w_step_uform(tb, h, W_F[1](pt), y1)
            dm = max(abs(alt[0][d] - ref[0][d]) for d in range(2))
            dest = max(abs((alt[0][d] - alt[1][d]) - (ref[0][d] - ref[1][d])) for d in range(2))
            put("S5.spec.main.delta_%s_h2pow%d" % (name, k), dm, "nr")
            put("S5.spec.est.delta_%s_h2pow%d" % (name, k), dest, "nr")
            cur += [dm, dest]
        row = [k]
        for i, v in enumerate(cur):
            row += [float(v), None if prev is None else math.log2(float(prev[i] / v))]
        rows_s.append(row)
        prev = cur
    TABLES["S5.speculative_jacobian_experiment"] = {
        "problem": "same ODE as S4; step n from y0 with the exact Jacobian, step n+1 from y1 with the Jacobian "
                   "evaluated at g7 or g8 of step n; difference to the step with J(y1); h = 2^-k; "
                   "y1, g7, g8 rounded to rationals with denominators <= 1e40",
        "columns": ["k", "g7 main |delta|", "order", "g7 est |delta|", "order",
                    "g8 main |delta|", "order", "g8 est |delta|", "order"], "rows": rows_s}
    last = rows_s[-1]
    for lab, idx in (("g7_main", 2), ("g7_est", 4), ("g8_main", 6), ("g8_est", 8)):
        put("S5.spec.%s_observed_order" % lab, last[idx], "n")
    check("S5.f", "numerical", "speculative Jacobian: evaluating J of step n+1 at g7 (g8) of step n perturbs the "
          "solution by O(h^7) (O(h^8)) and the error estimate by O(h^6) (O(h^7))",
          abs(last[2] - 7) < 0.35 and abs(last[4] - 6) < 0.35 and abs(last[6] - 8) < 0.35 and abs(last[8] - 7) < 0.35,
          "orders g7 %.3f / %.3f, g8 %.3f / %.3f" % (last[2], last[4], last[6], last[8]))
    return orders


def spec_speedup(r, k):
    """Step speed-up when J+LU of the next step overlaps the last k of 8 stages (r = T_JLU/T_chain)."""
    return (r + 1) / (r + 1 - min(r, F(k, 8)))


def section_s6():
    put("S6.max_speedup_two_stages", spec_speedup(F(1, 4), 2))
    put("S6.max_speedup_one_stage", spec_speedup(F(1, 8), 1))
    put("S6.speedup_at_r_3_20", spec_speedup(F(3, 20), 2))
    put("S6.speedup_at_r_11_12", spec_speedup(F(11, 12), 2))
    grid = [F(j, 240) for j in range(0, 240 * 4 + 1)]
    band = [r for r in grid if spec_speedup(r, 2) >= F(23, 20)]
    put("S6.band_lower", min(band))
    put("S6.band_upper", max(band))
    contiguous = len(band) == int((max(band) - min(band)) * 240) + 1
    mx2 = max(spec_speedup(r, 2) for r in grid)
    mx1 = max(spec_speedup(r, 1) for r in grid)
    check("S6.a", "exact", "speed-up <= 5/4 (two stages) and <= 9/8 (stage 8 only), attained at r = 1/4 resp. 1/8",
          mx2 == F(5, 4) and mx1 == F(9, 8) and spec_speedup(F(1, 4), 2) == F(5, 4)
          and spec_speedup(F(1, 8), 1) == F(9, 8))
    check("S6.b", "exact", "speed-up >= 23/20 exactly for r in [3/20, 11/12] (grid of step 1/240 on [0,4])",
          min(band) == F(3, 20) and max(band) == F(11, 12) and contiguous)
    TABLES["S6.band"] = {"lower": "3/20", "upper": "11/12", "upper_decimal": float(F(11, 12))}


def cost_model(kind, n, rho, jmode, bw=None, kbar=None, theta=None):
    """Flop-unit cost model of one accepted step.  Returns shares and lever bounds (Fractions)."""
    rhs = F(rho) * n
    vec = F(112) * n                      # 2*(nnz(A)+nnz(C)) axpy flops per step
    if kind == "dense":
        lu, solve = F(2, 3) * n ** 3, F(2) * n ** 2
        jac = {"analytic3": 3 * rhs, "fd": n * rhs}[jmode]
    elif kind == "banded":
        lu, solve = F(n) * bw ** 2, F(4) * n * bw
        jac = {"analytic3": 3 * rhs, "fd": (2 * bw + 1) * rhs}[jmode]
    else:
        raise ValueError(kind)
    chain = 8 * (rhs + solve) + vec
    total = jac + lu + chain
    fj, flu, fch = jac / total, lu / total, chain / total
    r = (jac + lu) / chain
    return {"f_J": fj, "f_LU": flu, "f_chain": fch, "r_JLU_over_chain": r,
            "bound_parallel_J_P6": 1 / (1 - fj * (1 - F(1, 6))),
            "bound_parallel_J_Pinf": 1 / (1 - fj),
            "bound_LU_reuse_m4": 1 / (1 - (fj + flu) * (1 - F(1, 4))),
            "bound_LU_reuse_minf": 1 / fch,
            "bound_speculation_2stages": spec_speedup(r, 2)}


def section_s7():
    rows = []
    cases = []
    for n in (8, 64, 512, 4096):
        for rho in (20, 2000):
            for jm in ("analytic3", "fd"):
                cases.append(("dense", n, rho, jm, None))
    for n, bw in ((400, 2), (4096, 64), (16384, 128), (65536, 256)):
        for rho in (20, 2000):
            for jm in ("analytic3", "fd"):
                cases.append(("banded", n, rho, jm, bw))
    for kind, n, rho, jm, bw in cases:
        m = cost_model(kind, n, rho, jm, bw)
        key = "S7.%s.n%d.rho%d.%s%s" % (kind, n, rho, jm, "" if bw is None else ".b%d" % bw)
        for k, v in m.items():
            put(key + "." + k, v)
        rows.append([kind, n, bw, rho, jm] + [round(float(m[k]), 4) for k in (
            "f_J", "f_LU", "f_chain", "r_JLU_over_chain", "bound_parallel_J_P6", "bound_LU_reuse_m4",
            "bound_LU_reuse_minf", "bound_speculation_2stages")])
    TABLES["S7.cost_model"] = {
        "label": "MODEL (flop units), not a measurement; to be confronted with V04",
        "columns": ["kind", "n", "half-bandwidth", "rho = RHS flops per unknown", "Jacobian mode",
                    "f_J", "f_LU", "f_chain", "(J+LU)/chain", "parallel-J bound P=6", "LU-reuse bound m=4",
                    "LU-reuse bound m=inf", "speculation (2 stages)"],
        "rows": rows}
    # closed-form crossovers LU = chain (cheap Jacobian neglected): exact rational discriminants
    for rho in (20, 2000):
        disc_d = F(144 + 12 * rho + 168)
        disc_b = F(256 + 8 * rho + 112)
        put("S7.dense_crossover_n_rho%d" % rho, 12 + math.sqrt(disc_d), "n")
        put("S7.banded_crossover_b_rho%d" % rho, 16 + math.sqrt(disc_b), "n")
    ok = True
    for rho in (20, 2000):
        for n in (10, 100, 1000):
            lhs = F(2, 3) * n ** 3 - (8 * (F(rho) * n + 2 * n * n) + 112 * n)
            ok = ok and (lhs > 0) == (n * n - 24 * n - 12 * rho - 168 > 0)
            for bw in (4, 40, 400):
                lhs = F(n) * bw * bw - (8 * (F(rho) * n + 4 * n * bw) + 112 * n)
                ok = ok and (lhs > 0) == (bw * bw - 32 * bw - 8 * rho - 112 > 0)
    check("S7.a", "exact", "within the model, LU > chain iff n^2 - 24n - 12rho - 168 > 0 (dense) "
          "resp. b^2 - 32b - 8rho - 112 > 0 (banded)", ok)
    # matrix-free: shared basis model
    rows_k = []
    for kbar in (5, 20):
        for jv in (20, 2000):          # JVP cost per unknown
            for theta in (F(1, 8), F(1, 4), F(1, 2), F(1)):
                sep = 8 * kbar * F(jv) + 16 * kbar ** 2
                shared = 8 * theta * kbar * F(jv) + 128 * theta ** 2 * kbar ** 2
                key = "S7.krylov.k%d.jv%d.theta_%d_%d" % (kbar, jv, theta.numerator, theta.denominator)
                put(key + ".speedup", sep / shared)
                put(key + ".jvp_ratio", 1 / theta)
                rows_k.append([kbar, jv, float(theta), float(1 / theta), round(float(sep / shared), 4)])
    TABLES["S7.shared_krylov_model"] = {
        "label": "MODEL: 8 solves of kbar iterations each vs one shared basis of dimension 8*theta*kbar; "
                 "orthogonalisation 2*K^2*n flops, JVP = jv*n flops",
        "columns": ["kbar", "JVP flops per unknown", "theta", "JVP ratio", "Krylov-part speed-up"],
        "rows": rows_k}
    check("S7.b", "exact", "within the model the shared basis reduces the orthogonalisation cost only if "
          "theta < 1/sqrt(8) (128 theta^2 kbar^2 < 16 kbar^2)", F(1, 8) < F(1, 3) and 128 * F(1, 3) ** 2 < 16 <
          128 * F(2, 5) ** 2)


def series_sqrt(d, s0, nterms):
    """Power series of sqrt(d(x)) given s0 = sqrt(d[0])."""
    s = [s0]
    for k in range(1, nterms):
        dk = d[k] if k < len(d) else 0
        acc = dk - sum(s[j] * s[k - j] for j in range(1, k))
        s.append(acc / (2 * s0))
    return s


def series_mul(p, q, nterms):
    r = [p[0] * 0] * nterms
    for i in range(min(len(p), nterms)):
        for j in range(min(len(q), nterms - i)):
            r[i + j] = r[i + j] + p[i] * q[j]
    return r


def series_exp(v, nterms):
    """exp of a series with v[0] = 0."""
    e = [v[0] * 0 + 1]
    for k in range(1, nterms):
        e.append(sum(j * v[j] * e[k - j] for j in range(1, k + 1)) / k)
    return e


def rho_series(sigma, s0, nterms=8):
    """rho(kappa): decaying root of sigma (rho + 1/rho) = 1 + 2 sigma - kappa, as a series in kappa."""
    d = [1 + 4 * sigma, -2 * (1 + 2 * sigma), sigma * 0 + 1]
    sq = series_sqrt(d, s0, nterms)
    a = [1 + 2 * sigma, sigma * 0 - 1] + [sigma * 0] * (nterms - 2)
    return [(a[k] - sq[k]) / (2 * sigma) for k in range(nterms)]


def phi_matrix(kmat_pows, coeffs):
    n = len(kmat_pows[0])
    out = [[coeffs[0] * 0 for _ in range(n)] for _ in range(n)]
    for k, cf in enumerate(coeffs):
        out = madd(out, mscale(kmat_pows[k], cf))
    return out


def thomas(sub, diag, sup, rhs):
    n = len(rhs)
    cp, dp = [None] * n, [None] * n
    cp[0] = sup / diag
    dp[0] = rhs[0] / diag
    for i in range(1, n):
        den = diag - sub * cp[i - 1]
        cp[i] = sup / den
        dp[i] = (rhs[i] - sub * dp[i - 1]) / den
    x = [None] * n
    x[-1] = dp[-1]
    for i in range(n - 2, -1, -1):
        x[i] = dp[i] - cp[i] * x[i + 1]
    return x


def cg_2d(nx, ny, sigma, x0, y0, iters=4000):
    """Green's function of I - sigma*Laplace_5pt, Dirichlet in x, periodic in y (floats, CG)."""
    def op(u):
        out = [0.0] * (nx * ny)
        for x in range(nx):
            for y in range(ny):
                i = x * ny + y
                acc = 4.0 * u[i] - u[x * ny + (y + 1) % ny] - u[x * ny + (y - 1) % ny]
                if x > 0:
                    acc -= u[i - ny]
                if x < nx - 1:
                    acc -= u[i + ny]
                out[i] = u[i] + sigma * acc
        return out
    bvec = [0.0] * (nx * ny)
    bvec[x0 * ny + y0] = 1.0
    u = [0.0] * (nx * ny)
    r = list(bvec)
    p = list(r)
    rr = sum(v * v for v in r)
    for _ in range(iters):
        ap = op(p)
        al = rr / sum(x * y for x, y in zip(p, ap))
        u = [x + al * y for x, y in zip(u, p)]
        r = [x - al * y for x, y in zip(r, ap)]
        rr_new = sum(v * v for v in r)
        if rr_new < 1e-40:
            break
        p = [x + (rr_new / rr) * y for x, y in zip(r, p)]
        rr = rr_new
    return u


def section_s8(tb):
    n, gamma = tb["n"], tb["gamma"]
    # --- 1D: exact characteristic root and Green's function for sigma = m(m+1)
    ok_root = ok_green = True
    for m in (1, 2, 3):
        sigma = F(m * (m + 1))
        rho = F(m, m + 1)
        ok_root = ok_root and sigma * rho ** 2 - (1 + 2 * sigma) * rho + sigma == 0
        g = [rho ** d / (2 * m + 1) for d in range(0, 12)]
        ok_green = ok_green and (1 + 2 * sigma) * g[0] - 2 * sigma * g[1] == 1
        ok_green = ok_green and all((1 + 2 * sigma) * g[d] - sigma * (g[d - 1] + g[d + 1]) == 0
                                    for d in range(1, 11))
        put("S8.rho_exact_sigma%d" % (m * (m + 1)), rho)
    check("S8.a", "exact", "1D: rho = (1 + 2 sigma - sqrt(1 + 4 sigma))/(2 sigma) solves "
          "sigma rho^2 - (1 + 2 sigma) rho + sigma = 0; for sigma = m(m+1), rho = m/(m+1)", ok_root)
    check("S8.b", "exact", "1D lattice Green's function G_d = rho^|d| / sqrt(1 + 4 sigma) satisfies W G = delta",
          ok_green)
    rows = []
    for sig in (0.25, 1.0, 10.0, 100.0, 10000.0):
        ell = math.sqrt(sig)
        rho = (1 + 2 * sig - math.sqrt(1 + 4 * sig)) / (2 * sig)
        theta = 2 * math.asinh(1 / (2 * ell))
        key = "S8.sigma_%s" % repr(sig).replace(".", "p")
        put(key + ".rho", rho, "n")
        put(key + ".theta_times_ell", theta * ell, "n")
        put(key + ".rho_vs_exp_theta_abs_diff", abs(rho - math.exp(-theta)), "n")
        rows.append([sig, ell, rho, math.exp(-1 / ell), theta * ell])
    TABLES["S8.rho_vs_ell"] = {"columns": ["sigma = ell^2", "ell", "rho", "exp(-1/ell)", "ell * ln(1/rho)"],
                               "rows": rows}
    check("S8.c", "numerical", "rho = exp(-theta), theta = 2 asinh(1/(2 ell)); ell*theta -> 1 as ell grows "
          "(decay length ell cells)", all(abs(VALUES_F(k)) < 1e-12 for k in VALUES if k.endswith("abs_diff"))
          and abs(rows[-1][4] - 1) < 1e-4)
    # finite grid, exact: Dirichlet tridiagonal Green's function, sigma = 6 (rho = 2/3)
    sigma, npts = F(6), 121
    cidx = npts // 2
    rhs = [F(0)] * npts
    rhs[cidx] = F(1)
    g = thomas(-sigma, 1 + 2 * sigma, -sigma, rhs)
    ratios = [g[cidx + d + 1] / g[cidx + d] for d in range(0, 20)]
    dev = max(abs(r - F(2, 3)) for r in ratios)
    put("S8.finite1d_max_ratio_deviation", dev)
    put("S8.finite1d_G0_times_sqrt", g[cidx] * 5)
    check("S8.d", "exact", "finite Dirichlet grid (121 points, sigma = 6, exact rationals): successive ratios of "
          "the Green's function equal rho = 2/3 up to image terms below 1e-12", dev < F(1, 10 ** 12),
          "max deviation " + sci(dev))
    # --- 2D five-point: Dirichlet in x, periodic in y
    nx = ny = 31
    x0, y0 = nx // 2, 0
    rows2 = {}
    for sig in (2.0, 6.0):
        u = cg_2d(nx, ny, sig, x0, y0)
        gv = lambda x, y: u[x * ny + (y % ny)]
        axis = [gv(x0 + d + 1, y0) / gv(x0 + d, y0) for d in range(0, 8)]
        diag = [gv(x0 + d + 1, y0 + d + 1) / gv(x0 + d, y0 + d) for d in range(0, 6)]
        col = [sum(gv(x0 + d, y) for y in range(ny)) for d in range(0, 9)]
        rhs1 = [0.0] * nx
        rhs1[x0] = 1.0
        g1 = thomas(-sig, 1 + 2 * sig, -sig, rhs1)
        col_dev = max(abs(col[d] - g1[x0 + d]) / g1[x0 + d] for d in range(0, 9))
        rho1 = (1 + 2 * sig - math.sqrt(1 + 4 * sig)) / (2 * sig)
        rho_diag2 = math.exp(-2 * math.acosh(1 + 1 / (4 * sig)))
        key = "S8.grid2d.sigma%d" % int(sig)
        for d, v in enumerate(axis):
            put(key + ".axis_ratio_d%d" % d, v, "n")
        for d, v in enumerate(diag):
            put(key + ".diag_ratio_d%d" % d, v, "n")
        for d, v in enumerate(col):
            put(key + ".colsum_d%d" % d, v, "n")
        put(key + ".colsum_vs_1d_max_rel_dev", col_dev, "n0")
        put(key + ".rho_1d", rho1, "n")
        put(key + ".rho_diag_sq", rho_diag2, "n")
        rows2[str(int(sig))] = {"rho_1d": rho1, "axis_ratios": axis, "rho_diag_squared": rho_diag2,
                                "diag_ratios": diag, "colsum_ratios": [col[d + 1] / col[d] for d in range(8)]}
        check("S8.e.sigma%d" % int(sig), "numerical",
              "2D five-point (31x31, Dirichlet x, periodic y): y-sums of the Green's function equal the 1D "
              "Green's function; axis ratios G(d+1,0)/G(d,0) < rho_1D; diagonal ratios < rho_diag^2",
              col_dev < 1e-10 and all(v < rho1 for v in axis) and all(v < rho_diag2 for v in diag),
              "colsum dev %.2e, axis %.4f..%.4f vs %.4f" % (col_dev, axis[0], axis[-1], rho1))
    TABLES["S8.grid2d"] = rows2
    # --- eight-stage block transfer matrix Phi(d) = rho(K)^d, K = (I + X)^-1 X, X = N/gamma
    x = mscale(tb["N"], 1 / gamma)
    kmat = matmul(inverse(madd(eye(n), x)), x)
    kp = [eye(n)]
    for _ in range(7):
        kp.append(matmul(kp[-1], kmat))
    for m in range(1, 8):
        put("S8.K_norminf_m%d" % m, norm_inf(kp[m]))
    sigma = F(6)
    rs = rho_series(sigma, F(5))
    pw = [[F(1)] + [F(0)] * 7]
    hmax = 12
    for _ in range(2 * hmax + 1):
        pw.append(series_mul(pw[-1], rs, 8))
    phi = [phi_matrix(kp, pw[d]) for d in range(2 * hmax + 2)]
    ok_rec = all(madd(madd(mscale(phi[d], 1 + 2 * sigma), mscale(madd(phi[d - 1], phi[d + 1]), sigma), -1),
                      matmul(kmat, phi[d]), -1) == [[F(0)] * n for _ in range(n)]
                 for d in range(1, 2 * hmax))
    ok_semi = matmul(phi[3], phi[5]) == phi[8] and phi[0] == eye(n)
    check("S8.f", "exact", "Phi(d) = rho(K)^d satisfies the block difference equation "
          "(1+2 sigma) Phi(d) - sigma (Phi(d-1) + Phi(d+1)) = K Phi(d), Phi(0) = I, Phi(a) Phi(b) = Phi(a+b) "
          "(sigma = 6, exact)", ok_rec and ok_semi)
    # direct simulation of the local eight-stage error chain on [0, H] with stage-error data E0 at d = 0
    e0 = [F(i + 1, 7) * (-1) ** i for i in range(n)]
    nil = tb["N"]
    es = []
    for i in range(n):
        comb = [sum(nil[i][j] * (es[j][d] if 0 < d < hmax else (e0[j] if d == 0 else F(0)))
                    for j in range(i)) for d in range(0, hmax + 1)]
        rhs = [(sigma / gamma) * (comb[d - 1] - 2 * comb[d] + comb[d + 1]) for d in range(1, hmax)]
        rhs[0] = rhs[0] + sigma * e0[i]
        sol = thomas(-sigma, 1 + 2 * sigma, -sigma, rhs)
        es.append([e0[i]] + sol + [F(0)])
    inv_term = inverse(madd(eye(n), phi[2 * hmax], -1))
    ok_sim = True
    for d in range(0, hmax + 1):
        pred = matvec(matmul(madd(phi[d], phi[2 * hmax - d], -1), inv_term), e0)
        ok_sim = ok_sim and all(pred[i] == es[i][d] for i in range(n))
    check("S8.g", "exact", "direct simulation of the local 8-stage chain (linear diffusion, k-form, sigma = 6, "
          "H = 12, Dirichlet stage-error data) equals (Phi(d) - Phi(2H-d)) (I - Phi(2H))^-1 E0 exactly", ok_sim)
    # amplification table (floats) for sigma = 100 (ell = 10) and 10^4 (ell = 100)
    kp_f = [[[float(v) for v in r] for r in m] for m in kp]
    x_f = [[float(v) for v in r] for r in x]
    b_f = [float(v) for v in tb["b"]]
    gam_f = [[float(v) for v in r] for r in tb["Gamma"]]
    gam_inv_f = [[float(v) for v in r] for r in inverse(tb["Gamma"])]
    rows_a = []
    for sig in (100.0, 10000.0):
        ell = math.sqrt(sig)
        rs_f = rho_series(sig, math.sqrt(1 + 4 * sig))
        rho = rs_f[0]
        for cmul in (1, 2, 4, 6, 8, 12, 16):
            delta = int(round(cmul * ell))
            # coefficients of (rho(kappa)/rho)^delta via exp(delta * log-series) to avoid underflow
            ratio = [v / rho for v in rs_f]
            lg = [0.0] * 8
            for k in range(1, 8):
                lg[k] = (ratio[k] - sum(j * lg[j] * ratio[k - j] for j in range(1, k)) / k)
            coeffs = series_exp([delta * v for v in lg], 8)
            ph = phi_matrix(kp_f, coeffs)
            amp = norm_inf(ph)
            amp_b = sum(abs(v) for v in vecmat(b_f, ph))
            amp_u = norm_inf(matmul(matmul(gam_f, ph), gam_inv_f))
            # continuum limit exp(-c sqrt(1-kappa)) / exp(-c)
            sq = series_sqrt([1.0, -1.0], 1.0, 8)
            cont = series_exp([0.0] + [-cmul * v for v in sq[1:]], 8)
            amp_c = norm_inf(phi_matrix(kp_f, cont))
            key = "S8.amp.sigma%d.c%d" % (int(sig), cmul)
            put(key + ".delta", delta)
            put(key + ".rho_pow_delta", rho ** delta, "n")
            put(key + ".amplification_norminf", amp, "n")
            put(key + ".amplification_b_functional", amp_b, "n")
            put(key + ".amplification_norminf_Uvariables", amp_u, "n")
            put(key + ".continuum_amplification_norminf", amp_c, "n")
            rows_a.append([sig, cmul, delta, rho ** delta, amp, amp * rho ** delta, amp_b, amp_c, amp_u,
                           amp_u * rho ** delta])
    TABLES["S8.block_transfer_amplification"] = {
        "columns": ["sigma", "c = delta/ell", "delta (cells)", "rho^delta", "norminf(Phi(delta)) / rho^delta",
                    "norminf(Phi(delta))", "norm1(b^T Phi(delta)) / rho^delta", "continuum norminf(exp(-c((beta/gamma)^-1/2 - I)))",
                    "U variables: norminf(Gamma Phi Gamma^-1) / rho^delta", "U variables: norminf(Gamma Phi Gamma^-1)"],
        "rows": rows_a}
    # predicted worst-case reduction of the stage-error vector (U variables) after q parallel sweeps for a
    # one-sided halo of ceil(m*ell) cells: distance between the two artificial boundaries D = 2*halo + 1
    rows_s = []
    sig = 100.0
    ell = math.sqrt(sig)
    rs_f = rho_series(sig, math.sqrt(1 + 4 * sig))
    ratio = [v / rs_f[0] for v in rs_f]
    lg = [0.0] * 8
    for k in range(1, 8):
        lg[k] = (ratio[k] - sum(j * lg[j] * ratio[k - j] for j in range(1, k)) / k)
    for m in (1, 2, 3, 4, 6):
        halo = int(math.ceil(m * ell - 1e-12))
        dist = 2 * halo + 1
        row = [m, halo, dist, rs_f[0] ** dist]
        for q in (1, 2, 3, 4):
            coeffs = series_exp([q * dist * v for v in lg], 8)
            ph = phi_matrix(kp_f, coeffs)
            val = norm_inf(matmul(matmul(gam_f, ph), gam_inv_f)) * rs_f[0] ** (q * dist)
            put("S8.sweeps.sigma100.m%d.q%d.norminf_U" % (m, q), val, "n")
            row.append(val)
        rows_s.append(row)
    TABLES["S8.predicted_sweep_reduction"] = {
        "label": "MODEL (two half-line subdomains, linear constant-coefficient diffusion, sigma = 100, ell = 10): "
                 "worst-case infinity-norm reduction of the eight-stage error vector (U variables) after q sweeps",
        "columns": ["m (halo = ceil(m*ell))", "halo cells", "D = 2*halo + 1", "rho^D (asymptotic per sweep)",
                    "q=1", "q=2", "q=3", "q=4"],
        "rows": rows_s}
    a8 = [r for r in rows_a if r[0] == 100.0 and r[1] == 8][0]
    a4 = [r for r in rows_a if r[0] == 100.0 and r[1] == 4][0]
    check("S8.h", "numerical", "the 8-stage block transfer matrix has spectral radius rho^delta but a norm "
          "larger by a polynomial transient factor (non-normal growth through powers of K = N beta^-1)",
          a8[4] > 1.0 and a4[4] > 1.0,
          "sigma=100: c=4 factor %.4g, c=8 factor %.4g, ||Phi|| = %.3g, %.3g" % (a4[4], a8[4], a4[5], a8[5]))
    return rows_a


def VALUES_F(key):
    return float(VALUES[key])


def section_s9(tb):
    def coeff_poly(ell, k):
        """[z^k] e^z (1 - g z)^ell as a polynomial in g."""
        return [F((-1) ** j * math.comb(ell, j), math.factorial(k - j)) if j <= k else F(0)
                for j in range(0, min(ell, k) + 1)]
    # L = 4: order 5 with R(inf) = 0 needs [z^4] = [z^5] = 0 simultaneously
    p4, p5 = ptrim(coeff_poly(4, 4)), ptrim(coeff_poly(4, 5))
    aa, bb = p5, p4
    while not (len(bb) == 1 and bb[0] == 0):
        aa, bb = bb, prem(aa, bb)
    gcd_const = len(aa) == 1 and aa[0] != 0
    check("S9.a", "exact", "L = 4 levels: order 5 together with R(infinity) = 0 is impossible "
          "(the two conditions on gamma have no common root: polynomial gcd is constant)", gcd_const)

    def real_roots(p, lo, hi, steps=20000):
        out = []
        xs = [lo + (hi - lo) * F(j, steps) for j in range(steps + 1)]
        vals = [peval(p, v) for v in xs]
        for j in range(steps):
            if vals[j] == 0:
                out.append(xs[j])
            elif vals[j] * vals[j + 1] < 0:
                a_, b_ = xs[j], xs[j + 1]
                fa = vals[j]
                for _ in range(140):
                    mid = (a_ + b_) / 2
                    fm = peval(p, mid)
                    if fm == 0:
                        a_ = b_ = mid
                        break
                    if fa * fm < 0:
                        b_ = mid
                    else:
                        a_, fa = mid, fm
                out.append((a_ + b_) / 2)
        return out
    # L = 4 without L-stability: [z^5] = 0
    r4 = real_roots(p5, F(1, 100), F(5))
    rows4 = []
    for i, g in enumerate(r4):
        st = a_stable(g, 4, 4)
        put("S9.L4_order5_gamma_root%d" % i, g, "n")
        rows4.append([float(g), st])
    check("S9.b", "numerical", "L = 4 levels, order 5 without R(inf)=0: none of the admissible gamma is A-stable",
          len(r4) >= 1 and not any(r[1] for r in rows4), str(rows4))
    # L = 5: order 5 and R(inf) = 0  <=>  [z^5] e^z (1-gz)^5 = 0
    q5 = ptrim(coeff_poly(5, 5))
    r5 = real_roots(q5, F(1, 100), F(5))
    rows5 = []
    for i, g in enumerate(r5):
        st = a_stable(g, 5, 4)
        put("S9.L5_gamma_root%d" % i, g, "n")
        rows5.append([float(g), st])
    stable5 = [r[0] for r in rows5 if r[1]]
    check("S9.c", "numerical", "L = 5 levels: order 5 with R(inf) = 0 forces gamma to one of 5 real roots; "
          "exactly one is A-stable (gamma = 0.27805...)",
          len(r5) == 5 and len(stable5) == 1 and abs(stable5[0] - 0.2780538411) < 1e-9, str(rows5))
    g5 = [g for g in r5 if abs(float(g) - 0.2780538411) < 1e-6][0]
    # error constants of the linear stability function
    def lin_defect(gam_, ell, deg, k):
        p = pade_like_numerator(gam_, ell, deg)
        inv = [F(math.comb(j + ell - 1, ell - 1)) * gam_ ** j for j in range(k + 1)]
        ser = series_mul(p, inv, k + 1)
        return ser[k] - F(1, math.factorial(k))
    put("S9.L5_linear_defect_z6", lin_defect(g5, 5, 4, 6), "n")
    put("S9.L7_rodas5p_linear_defect_z7", lin_defect(tb["gamma"], 7, 6, 7))

    def interval(ell, deg, inside):
        grid = [F(j, 200) for j in range(4, 201)]
        st = {g: a_stable(g, ell, deg) for g in grid}
        lo = hi = inside
        while lo - F(1, 200) in st and st[lo - F(1, 200)]:
            lo -= F(1, 200)
        while hi + F(1, 200) in st and st[hi + F(1, 200)]:
            hi += F(1, 200)
        a_, b_ = lo - F(1, 200), lo
        for _ in range(34):
            mid = (a_ + b_) / 2
            if a_stable(mid, ell, deg):
                b_ = mid
            else:
                a_ = mid
        c_, d_ = hi, hi + F(1, 200)
        for _ in range(34):
            mid = (c_ + d_) / 2
            if a_stable(mid, ell, deg):
                c_ = mid
            else:
                d_ = mid
        return b_, c_, sum(1 for g in grid if st[g])
    lo6, hi6, _ = interval(6, 5, F(1, 4))
    lo7, hi7, _ = interval(7, 6, F(1, 4))
    put("S9.L6_order5_Astable_gamma_lower", lo6, "n")
    put("S9.L6_order5_Astable_gamma_upper", hi6, "n")
    put("S9.L7_order6_Astable_gamma_lower", lo7, "n")
    put("S9.L7_order6_Astable_gamma_upper", hi7, "n")
    check("S9.d", "numerical", "L = 6 levels (order >= 5, R(inf) = 0): A-stable for gamma in "
          "[0.18391465, 0.33414237]; L = 7 (order >= 6): [0.20408345, 0.37886489]; RODAS5P gamma lies in both",
          abs(float(lo6) - 0.18391465) < 1e-7 and abs(float(hi6) - 0.33414237) < 1e-7
          and abs(float(lo7) - 0.20408345) < 1e-7 and abs(float(hi7) - 0.37886489) < 1e-7
          and lo7 < tb["gamma"] < hi7 and lo6 < tb["gamma"] < hi6,
          "L6 [%.8f, %.8f], L7 [%.8f, %.8f]" % (float(lo6), float(hi6), float(lo7), float(hi7)))

    # counting of free coefficients
    def count(sizes, stiff, ties):
        s = sum(sizes)
        pairs = sum(sizes[i] * sizes[j] for i in range(len(sizes)) for j in range(i))
        if not stiff:
            return 1 + 2 * pairs + s
        tot = 1 + 2 * pairs
        lvl = [l for l, sz in enumerate(sizes) for _ in range(sz)]
        for t in range(ties):
            row = s - 1 - t
            tot -= sum(1 for j in range(s) if lvl[j] < lvl[row])
        return tot
    layouts = {"rodas5p_sequential_8x1": [1] * 8, "P2_L5_1-2-2-2-1": [1, 2, 2, 2, 1],
               "P2_L6_1-2-2-2-2-1": [1, 2, 2, 2, 2, 1], "P2_L5_1-2-2-2-2_not_stiffly_accurate": [1, 2, 2, 2, 2],
               "P3_L5_1-3-3-3-1": [1, 3, 3, 3, 1]}
    rows = []
    for name, sizes in layouts.items():
        vals = [sum(sizes), len(sizes), count(sizes, False, 0), count(sizes, True, 0), count(sizes, True, 1),
                count(sizes, True, 2)]
        for lab, v in zip(("stages", "levels", "general", "stiffly_accurate", "sa_one_tie", "sa_two_ties"), vals):
            put("S9.count.%s.%s" % (name, lab), v)
        rows.append([name] + vals)
    TABLES["S9.free_coefficients"] = {
        "columns": ["layout", "stages", "levels", "general (gamma, alpha, gamma_ij, b)",
                    "stiffly accurate (b = beta_s)", "+ alpha_s = beta_(s-1)", "+ alpha_(s-1) = beta_(s-2)"],
        "rows": rows,
        "conditions": {"ode_order5_trees": 17, "index1_dae_additional_table3_inferred": 23,
                       "table4_additional": 9, "rodas5p_numbered_total": 49,
                       "embedded_order4_ode_trees": 8}}
    put("S9.conditions.ode_order5", 17)
    put("S9.conditions.rodas5p_numbered_total", 49)
    check("S9.e", "exact", "free coefficients: RODAS5P structure (stiffly accurate, two embedded ties) 44; "
          "two-processor L=5 stiffly accurate with one embedded tie: 44; L=6: 74",
          rows[0][6] == 44 and rows[1][5] == 44 and rows[2][5] == 74, str(rows))


def main():
    raw_bytes = open(SNAPSHOT, "rb").read()
    digest = hashlib.sha256(raw_bytes).hexdigest()
    pinned = re.search(r'RODAS5P_COEFFICIENT_SNAPSHOT_SHA256: &str =\s*"([0-9a-f]{64})"', open(COEFF_RS).read())
    tb = build(F)
    tb_bin = build(lambda s: F(float(s)))
    check("S0.a", "exact", "snapshot SHA-256 equals the digest pinned in coefficients.rs",
          pinned is not None and pinned.group(1) == digest, digest)
    section_s1(tb)
    sd = section_s2(tb, tb_bin)
    section_s3(tb, sd)
    section_s4(tb, tb_bin)
    section_s5(tb, tb_bin)
    section_s6()
    section_s7()
    section_s8(tb)
    section_s9(tb)
    failed = [c["id"] for c in CHECKS if not c["passed"]]
    result = {
        "schema": "vigilode-loop-l1-v05-theory-results-v1",
        "node": "V05",
        "commands": ["python3 research/production_loop_20261116/theory/verify_tableau.py",
                     "python3 research/production_loop_20261116/theory/verify_independent.py"],
        "snapshot": {"path": "fixtures/rodas5p_coefficients_snapshot.json", "sha256": digest},
        "primary_fraction": {
            "implementation": "fractions.Fraction on the decimal strings; python " + platform.python_version(),
            "value_format": "decimal strings, %d significant digits; kinds: x exact, n numerical, "
                            "n0 numerical quantity that is zero up to rounding, nr numerical compared "
                            "relatively" % SIG,
            "values": VALUES, "kinds": KINDS, "tables": TABLES, "checks": CHECKS,
            "checks_total": len(CHECKS), "checks_failed": failed},
        "independent_mpmath": None,
        "agreement": None,
    }
    with open(OUT, "w") as fh:
        json.dump(result, fh, indent=1, allow_nan=False, sort_keys=True)
        fh.write("\n")
    for c in CHECKS:
        print("%-4s %-16s [%s] %s%s" % ("PASS" if c["passed"] else "FAIL", c["id"], c["tag"], c["statement"],
                                        (" :: " + c["detail"]) if c["detail"] else ""))
    print("values: %d, checks: %d, failed: %d -> %s" % (len(VALUES), len(CHECKS), len(failed), OUT))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
