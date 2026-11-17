#!/usr/bin/env python3
"""Loop L1 node V16 -- independent re-verification of a candidate tableau from its coefficient file.

    python3 -I verify_candidate.py candidates/D6T_r01.json [more files] > logs/verify_<name>.jsonl

Shares no code with the generator, the search or the refinement (standard library and mpmath only). The 17
order conditions are typed out; exact arithmetic is fractions.Fraction on the decimal strings of the file;
binary64 arithmetic is Python floats of the same strings. Checks: PREDECLARED.json, key algebraic_found_checks.
Exit code 0 iff every file passes every algebraic check.
"""
import json
import os
import sys
from fractions import Fraction as Fr

from mpmath import mp, mpc, mpf

mp.dps = 50
TOL64 = 1e-12
TOLX = Fr(1, 10 ** 25)
GRID_TOL = 1e-20
DEGENERATE = 1e-6
BOUND = 100


def emit(**kw):
    print(json.dumps(kw, allow_nan=False), flush=True)


def mv(m, v):
    return [sum((m[i][j] * v[j] for j in range(len(v))), v[0] * 0) for i in range(len(m))]


def had(u, v):
    return [x * y for x, y in zip(u, v)]


def dot(u, v):
    return sum((x * y for x, y in zip(u, v)), v[0] * 0)


def typed_conditions(al, be, w):
    """The 17 conditions of order <= 5 (branching vertex: alpha; singly-branched vertex: beta with diagonal)."""
    one = [w[0] * 0 + 1] * len(w)
    c = mv(al, one)
    b1 = mv(be, one)
    c2 = had(c, c)
    c3 = had(c2, c)
    c4 = had(c3, c)
    ab1 = mv(al, b1)
    b2 = mv(be, b1)
    b3 = mv(be, b2)
    b4 = mv(be, b3)
    cab1 = had(c, ab1)
    bc2 = mv(be, c2)
    return [
        (1, "sum b_i = 1", dot(w, one), Fr(1)),
        (2, "b.beta.1 = 1/2", dot(w, b1), Fr(1, 2)),
        (3, "b.c^2 = 1/3", dot(w, c2), Fr(1, 3)),
        (3, "b.beta^2.1 = 1/6", dot(w, b2), Fr(1, 6)),
        (4, "b.c^3 = 1/4", dot(w, c3), Fr(1, 4)),
        (4, "b.(c*alpha.beta.1) = 1/8", dot(w, cab1), Fr(1, 8)),
        (4, "b.beta.c^2 = 1/12", dot(w, bc2), Fr(1, 12)),
        (4, "b.beta^3.1 = 1/24", dot(w, b3), Fr(1, 24)),
        (5, "b.c^4 = 1/5", dot(w, c4), Fr(1, 5)),
        (5, "b.(c^2*alpha.beta.1) = 1/10", dot(w, had(c2, ab1)), Fr(1, 10)),
        (5, "b.(c*alpha.c^2) = 1/15", dot(w, had(c, mv(al, c2))), Fr(1, 15)),
        (5, "b.(c*alpha.beta^2.1) = 1/30", dot(w, had(c, mv(al, b2))), Fr(1, 30)),
        (5, "b.(alpha.beta.1)^2 = 1/20", dot(w, had(ab1, ab1)), Fr(1, 20)),
        (5, "b.beta.c^3 = 1/20", dot(w, mv(be, c3)), Fr(1, 20)),
        (5, "b.beta.(c*alpha.beta.1) = 1/40", dot(w, mv(be, cab1)), Fr(1, 40)),
        (5, "b.beta^2.c^2 = 1/60", dot(w, mv(be, bc2)), Fr(1, 60)),
        (5, "b.beta^4.1 = 1/120", dot(w, b4), Fr(1, 120)),
    ]


# ------------------------------------------------------------------------------- polynomials, low degree first
def padd(p, q):
    n = max(len(p), len(q))
    return [(p[i] if i < len(p) else 0) + (q[i] if i < len(q) else 0) for i in range(n)]


def pmul(p, q):
    r = [Fr(0)] * (len(p) + len(q) - 1)
    for i, x in enumerate(p):
        for j, y in enumerate(q):
            r[i + j] += x * y
    return r


def ppow(p, k):
    r = [Fr(1)]
    for _ in range(k):
        r = pmul(r, p)
    return r


def trim(p):
    p = list(p)
    while len(p) > 1 and p[-1] == 0:
        p.pop()
    return p


def prem(a, b):
    a = trim(a)
    b = trim(b)
    while len(a) >= len(b) and any(x != 0 for x in a):
        f = a[-1] / b[-1]
        sh = len(a) - len(b)
        for i, x in enumerate(b):
            a[i + sh] -= f * x
        a = trim(a[:-1]) if len(a) > 1 else [Fr(0)]
    return a


def sturm_positive_roots(p):
    """Number of distinct roots of p in (0, +inf); p(0) != 0 is required."""
    p = trim(p)
    assert p[0] != 0
    chain = [p, trim([i * p[i] for i in range(1, len(p))])]
    while len(chain[-1]) > 1 or chain[-1][0] != 0:
        r = prem(chain[-2], chain[-1])
        if all(x == 0 for x in r):
            break
        chain.append([-x for x in r])

    def sign_at_zero_plus(q):
        for x in q:
            if x != 0:
                return 1 if x > 0 else -1
        return 0

    def sign_at_inf(q):
        return 1 if q[-1] > 0 else (-1 if q[-1] < 0 else 0)

    def changes(signs):
        s = [x for x in signs if x != 0]
        return sum(1 for i in range(len(s) - 1) if s[i] != s[i + 1])

    return changes([sign_at_zero_plus(q) for q in chain]) - changes([sign_at_inf(q) for q in chain])


def e_polynomial(P, gamma, L):
    """E(s) = (1 + gamma^2 s)^L - |P(i y)|^2 with s = y^2, as coefficients in s."""
    re = [Fr(0)] * (len(P) // 2 + 1)      # sum over even j of p_j (-1)^(j/2) s^(j/2)
    im = [Fr(0)] * (len(P) // 2 + 1)      # y * sum over odd j of p_j (-1)^((j-1)/2) s^((j-1)/2)
    for j, pj in enumerate(P):
        if j % 2 == 0:
            re[j // 2] += pj * (-1) ** (j // 2)
        else:
            im[(j - 1) // 2] += pj * (-1) ** ((j - 1) // 2)
    mod2 = padd(pmul(re, re), [Fr(0)] + pmul(im, im))
    q2 = ppow([Fr(1), gamma * gamma], L)
    return trim(padd(q2, [-x for x in mod2]))


def numerator(be, w, gamma, L):
    """P with R(z) = 1 + z w^T (I - z beta)^-1 1 = P(z) / (1 - gamma z)^L (uses N = beta - gamma I, N^L = 0)."""
    s = len(w)
    N = [[be[i][j] - (gamma if i == j else 0) for j in range(s)] for i in range(s)]
    v = [Fr(1)] * s
    mus = []
    for _ in range(L):
        mus.append(dot(w, v))
        v = mv(N, v)
    tail = dot(w, v)                       # w^T N^L 1, must be 0
    P = ppow([Fr(1), -gamma], L)
    for m, mu in enumerate(mus):
        term = pmul([Fr(0)] * (m + 1) + [mu], ppow([Fr(1), -gamma], L - 1 - m))
        P = padd(P, term)
    return P, tail, any(x != 0 for row in mat_pow(N, L) for x in row)


def mat_pow(M, k):
    s = len(M)
    R = [[Fr(1) if i == j else Fr(0) for j in range(s)] for i in range(s)]
    for _ in range(k):
        R = [[sum(R[i][t] * M[t][j] for t in range(s)) for j in range(s)] for i in range(s)]
    return R


def design_numerator(gamma, L):
    """trunc_{L-1}[exp(z) (1 - gamma z)^L]."""
    ex = [Fr(1)]
    for k in range(1, L + 1):
        ex.append(ex[-1] / k)
    return pmul(ex, ppow([Fr(1), -gamma], L))[:L]


def grid_max(be_fr, w_fr):
    """max over y = 10^(k/8), k = -48..56, of |R(iy)| - 1 with R from the tableau (50-digit complex solves)."""
    s = len(w_fr)
    be = [[mpf(x.numerator) / mpf(x.denominator) for x in r] for r in be_fr]
    w = [mpf(x.numerator) / mpf(x.denominator) for x in w_fr]
    worst = mpf(-1)
    at = None
    for k in range(-48, 57):
        y = mpf(10) ** (mpf(k) / 8)
        z = mpc(0, y)
        x = []
        for i in range(s):
            acc = mpc(1)
            for j in range(i):
                acc += z * be[i][j] * x[j]
            x.append(acc / (1 - z * be[i][i]))
        R = 1 + z * sum(w[i] * x[i] for i in range(s))
        d = abs(R) - 1
        if d > worst:
            worst, at = d, y
    return float(worst), float(at)


def verify(path):
    d = json.load(open(path))
    s = d["stages"]
    levels = d["levels"]
    L = len(levels)
    level_of = {st - 1: li for li, lv in enumerate(levels) for st in lv}
    gamma = Fr(d["gamma"])
    al = [[Fr(x) for x in r] for r in d["alpha"]]
    gm = [[Fr(x) for x in r] for r in d["Gamma"]]
    b = [Fr(x) for x in d["b"]]
    bh = [Fr(x) for x in d["bhat"]]
    be = [[al[i][j] + gm[i][j] for j in range(s)] for i in range(s)]
    out = {"kind": "verify", "id": d["id"], "file": os.path.basename(path), "formulation": d["formulation"],
           "stages": s, "levels": levels, "depth": L, "gamma": float(gamma)}

    # structure
    pattern_ok = all((al[i][j] == 0 and (gm[i][j] == 0 or i == j)) for i in range(s) for j in range(s)
                     if not level_of[j] < level_of[i]) and all(gm[i][i] == gamma for i in range(s))
    widths = [len(lv) for lv in levels]
    out["level_pattern_exact"] = bool(pattern_ok)
    out["level_widths"] = widths
    out["two_processor_layout"] = bool(max(widths) <= 2)
    digits = min(len(x.replace("-", "").replace(".", "").lstrip("0")) for r in d["alpha"] + d["Gamma"] for x in r
                 if Fr(x) != 0)
    out["min_significant_digits"] = digits

    # order 5, exact and binary64
    ex = typed_conditions(al, be, b)
    res_exact = [v - rhs for (_, _, v, rhs) in ex]
    alf = [[float(x) for x in r] for r in d["alpha"]]
    gmf = [[float(x) for x in r] for r in d["Gamma"]]
    bef = [[alf[i][j] + gmf[i][j] for j in range(s)] for i in range(s)]
    bf = [float(x) for x in d["b"]]
    bhf = [float(x) for x in d["bhat"]]
    f64 = typed_conditions(alf, bef, bf)
    res64 = [v - float(rhs) for (_, _, v, rhs) in f64]
    out["order5_max_abs_residual_exact"] = float(max(abs(x) for x in res_exact))
    out["order5_max_abs_residual_binary64"] = max(abs(x) for x in res64)
    out["order5_conditions"] = [{"order": o, "condition": name, "residual_exact": float(r), "residual_binary64": r64}
                                for (o, name, _, _), r, r64 in zip(ex, res_exact, res64)]
    ok_order = max(abs(x) for x in res_exact) <= TOLX and max(abs(x) for x in res64) <= TOL64

    # stiff accuracy and R(infinity)
    stiff = all(b[j] == be[s - 1][j] for j in range(s))
    x = []
    for i in range(s):
        x.append((1 - sum(be[i][j] * x[j] for j in range(i))) / be[i][i])
    r_inf = 1 - dot(b, x)
    out["stiffly_accurate_exact"] = bool(stiff)
    out["R_infinity_exact"] = float(r_inf)
    ok_stiff = stiff and r_inf == 0

    # A-stability
    P, tail, npow = numerator(be, b, gamma, L)
    Pd = design_numerator(gamma, L)
    diff = max(abs((P[i] if i < len(P) else 0) - (Pd[i] if i < len(Pd) else 0)) for i in range(L + 1))
    E = e_polynomial(Pd, gamma, L)
    low = 0
    while E[low] == 0:
        low += 1
    Ered = E[low:]
    roots = sturm_positive_roots(Ered)
    gmax, gat = grid_max(be, b)
    out["nilpotency_N_power_L_nonzero"] = bool(npow)
    out["stability_numerator"] = [float(v) for v in P]
    out["numerator_minus_design_max"] = float(diff)
    out["E_polynomial_lowest_power_of_s"] = low
    out["E_polynomial_reduced_coefficients"] = [float(v) for v in Ered]
    out["E_sturm_roots_in_positive_s"] = roots
    out["E_leading_coefficient_positive"] = bool(Ered[-1] > 0)
    out["grid_max_abs_R_minus_1"] = gmax
    out["grid_argmax_y"] = gat
    ok_astab = (roots == 0 and Ered[-1] > 0 and Ered[0] > 0 and diff <= TOLX and gmax <= GRID_TOL and gamma > 0
                and not npow)
    out["A_stable"] = bool(ok_astab)

    # embedded
    exh = typed_conditions(al, be, bh)
    reh = [v - rhs for (_, _, v, rhs) in exh]
    f64h = typed_conditions(alf, bef, bhf)
    re64h = [v - float(rhs) for (_, _, v, rhs) in f64h]
    e4x = max(abs(r) for (o, _, _, _), r in zip(exh, reh) if o <= 4)
    e464 = max(abs(r) for (o, _, _, _), r in zip(f64h, re64h) if o <= 4)
    e5 = max(abs(r) for (o, _, _, _), r in zip(exh, reh) if o == 5)
    out["embedded_order4_max_abs_residual_exact"] = float(e4x)
    out["embedded_order4_max_abs_residual_binary64"] = e464
    out["embedded_order5_max_abs_residual"] = float(e5)
    out["embedded_order5_residuals"] = [float(r) for (o, _, _, _), r in zip(exh, reh) if o == 5]
    out["embedded_differs_from_main_max"] = float(max(abs(x - y) for x, y in zip(b, bh)))
    ok_emb = e4x <= TOLX and e464 <= TOL64 and float(e5) >= DEGENERATE
    out["embedded_R_infinity"] = float(1 - dot(bh, x))
    ghmax, ghat = grid_max(be, bh)
    out["embedded_grid_max_abs_R_minus_1"] = ghmax
    out["embedded_grid_argmax_y"] = ghat
    out["embedded_A_stable_on_grid"] = bool(ghmax <= GRID_TOL)

    # W condition
    gi = [sum(r) for r in gm]
    wx = dot(b, gi)
    w64 = sum(bf[i] * sum(gmf[i]) for i in range(s))
    out["W_sum_b_gamma_i_exact"] = float(wx)
    out["W_sum_b_gamma_i_binary64"] = w64
    ok_w = abs(wx) <= TOLX and abs(w64) <= TOL64

    # coefficient bound (k-form)
    cmax = max([abs(gamma)] + [abs(v) for r in al for v in r] + [abs(v) for r in gm for v in r]
               + [abs(v) for v in b] + [abs(v) for v in bh])
    out["max_abs_coefficient"] = float(cmax)
    ok_bound = cmax <= BOUND

    # exploratory: abscissae, linear defects
    c = mv(al, [Fr(1)] * s)
    out["c"] = [float(v) for v in c]
    v = [Fr(1)] * s
    taylor = []
    fact = 1
    for k in range(1, 8):
        fact *= k
        taylor.append(float(dot(b, v) - Fr(1, fact)))
        v = mv(be, v)
    out["stability_taylor_defects_z1_to_z7"] = taylor
    out["checks"] = {"order5": bool(ok_order), "stiff_accuracy_R_inf_0": bool(ok_stiff), "A_stable": bool(ok_astab),
                     "embedded_order4": bool(ok_emb), "W": bool(ok_w), "coefficient_bound_100": bool(ok_bound),
                     "level_pattern": bool(pattern_ok and max(widths) <= 2)}
    out["algebraic_pass"] = bool(all(out["checks"].values()))
    return out


def main():
    emit(kind="header", script="verify_candidate.py", mp_dps=mp.dps, tolerances={"binary64": TOL64, "exact": 1e-25,
         "grid": GRID_TOL, "degenerate_estimator_below": DEGENERATE, "coefficient_bound": BOUND})
    allok = True
    for path in sys.argv[1:]:
        out = verify(path)
        allok = allok and out["algebraic_pass"]
        emit(**out)
    return 0 if allok else 1


if __name__ == "__main__":
    sys.exit(main())
