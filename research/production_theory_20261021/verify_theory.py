#!/usr/bin/env python3
"""Exact verification of the numerical claims of docs/plan/PRODUCTION_PLAN_20261020.md section 5.

Run: python3 research/production_theory_20261021/verify_theory.py --out research/production_theory_20261021/RESULTS.json
Claims and gates are fixed in PREREGISTRATION.md (committed before this script was run).
"""
import argparse, itertools, json, platform, sys
from fractions import Fraction
import mpmath as mp
import numpy as np
import sympy as sp

SEED = 20261021
mp.mp.dps = 50
SNAP = "fixtures/rodas5p_coefficients_snapshot.json"


def load():
    d = json.load(open(SNAP))
    R = sp.Rational
    return (R(d["gamma"]),
            sp.Matrix(8, 8, lambda i, j: R(d["A"][i][j])),
            sp.Matrix(8, 8, lambda i, j: R(d["C"][i][j])),
            sp.Matrix(1, 8, [R(x) for x in d["b_code"]]))


def f6(x):
    return float(sp.N(x, 20))


def build_polys(g, A, C, b):
    w = sp.Symbol("w")
    T = sp.Matrix(8, 8, lambda i, j: sp.expand(g * C[i, j] * w + A[i, j] * (w - 1)))
    S = sp.eye(8)
    P = sp.eye(8)
    for _ in range(7):
        P = (P * T).applyfunc(sp.expand)
        S = S + P
    assert (P * T).applyfunc(sp.expand) == sp.zeros(8, 8)  # T^8 = 0
    p = (b * S).applyfunc(sp.expand)            # b^T (I-T)^-1  (polynomials in w)
    m = (p * w).applyfunc(sp.expand)            # times w
    return w, p, m


def claim1(g, polys_m, w):
    ms = [sp.Poly(polys_m[0, j], w) for j in range(8)]
    at1 = [ms[j].eval(1) for j in range(8)]
    coefsum = [sum(abs(c) for c in ms[j].all_coeffs()) for j in range(8)]
    claimed_abs = [0.577, 0.237, 6.246, 5.966, 3.729, 4.568, 1.011, 1.000]
    claimed_cs = [100.456, 15.401, 24.467, 18.847, 13.655, 14.583, 1.011, 1.000]
    obs_abs = [abs(f6(x)) for x in at1]
    obs_cs = [f6(x) for x in coefsum]
    ok_a = all(round(o, 3) == c for o, c in zip(obs_abs, claimed_abs)) and abs(sum(obs_abs) - 23.335) <= 1e-3
    ok_b = all(round(o, 3) == c for o, c in zip(obs_cs, claimed_cs)) and abs(sum(obs_cs) - 189.418) <= 1e-3
    zs = {}
    claimed_z = {"-1": 17.819, "-10": 3.825, "-100": 0.125}
    for zz in [-1, -10, -100, sp.Rational(-1, 10), -3, -1000]:
        wv = 1 / (1 - g * sp.Integer(zz) if not isinstance(zz, sp.Rational) else 1 - g * zz)
        vals = [abs(f6(ms[j].eval(wv))) for j in range(8)]
        zs[str(zz)] = {"abs_m": vals, "sum": sum(vals)}
    ok_c = all(abs(zs[k]["sum"] - v) <= 1e-3 for k, v in claimed_z.items())
    return {
        "a_values_at_z0": {"signed_m": [f6(x) for x in at1], "abs_m": obs_abs, "sum_abs": sum(obs_abs),
                           "claimed_abs": claimed_abs, "claimed_sum": 23.335, "pass": bool(ok_a)},
        "b_coefficient_sums": {"observed": obs_cs, "sum": sum(obs_cs), "claimed": claimed_cs,
                               "claimed_sum": 189.418, "pass": bool(ok_b)},
        "c_scalar_exact_sums": {"by_z": zs, "claimed_sums": claimed_z, "pass": bool(ok_c)},
        "pass": bool(ok_a and ok_b and ok_c),
    }


def claim2(g, p, w):
    polys = [sp.Poly(p[0, j], w) for j in range(8)]
    coeffs = [[mp.mpf(sp.Rational(c).p) / mp.mpf(sp.Rational(c).q) for c in pl.all_coeffs()] for pl in polys]
    gm = mp.mpf(g.p) / mp.mpf(g.q)
    ts = np.linspace(-4, 8, 400)
    max_abs, min_abs_sum, max_abs_sum = (-1, None), (None, None), (-1, None)
    min_sum_entry = None
    max_signed_sum = None; min_signed_sum = None
    max_entry_val = mp.mpf(0); max_entry_at = None
    for t in ts:
        zabs = mp.mpf(10) ** mp.mpf(float(t))
        wv = 1 / (1 + gm * zabs)
        vals = [mp.polyval(c, wv) for c in coeffs]
        sa = sum(abs(v) for v in vals); ss = sum(vals)
        me = max(abs(v) for v in vals)
        if me > max_entry_val: max_entry_val, max_entry_at = me, float(t)
        if max_abs_sum[1] is None or sa > max_abs_sum[0]: max_abs_sum = (sa, float(t))
        if min_abs_sum[0] is None or sa < min_abs_sum[0]: min_abs_sum = (sa, float(t))
        max_signed_sum = ss if max_signed_sum is None or ss > max_signed_sum else max_signed_sum
        min_signed_sum = ss if min_signed_sum is None or ss < min_signed_sum else min_signed_sum
    ok = (max_entry_val <= mp.mpf("6.25")) and (min_abs_sum[0] >= 1) and (max_abs_sum[0] <= mp.mpf("23.34"))
    return {
        "grid": "z = -10^t, t = linspace(-4, 8, 400), mpmath 50 digits on exact-rational polynomial coefficients",
        "max_abs_entry": float(max_entry_val), "max_abs_entry_at_t": max_entry_at,
        "max_abs_sum": float(max_abs_sum[0]), "max_abs_sum_at_t": max_abs_sum[1],
        "min_abs_sum": float(min_abs_sum[0]), "min_abs_sum_at_t": min_abs_sum[1],
        "signed_sum_min": float(min_signed_sum), "signed_sum_max": float(max_signed_sum),
        "gate": "max |entry| <= 6.25 and abs-sum in [1, 23.34]", "pass": bool(ok),
    }


def claim3(g):
    x = sp.Symbol("x", positive=True)
    R = x / ((1 + 2 * g * x) * (1 - sp.exp(-x)))
    lim0 = sp.limit(R, x, 0)
    liminf = sp.limit(R, x, sp.oo)
    dlogR = sp.simplify(sp.diff(sp.log(R), x))
    dR = sp.diff(R, x)
    fR = sp.lambdify(x, R, "mpmath")
    fd = sp.lambdify(x, dR, "mpmath")
    gm = mp.mpf(g.p) / mp.mpf(g.q)
    upper = 1 / (2 * gm)
    n = 10 ** 4
    mn_d = None; ok = True; rmin = None; rmax = None
    for k in range(n):
        xv = mp.mpf(10) ** (mp.mpf(-8) + mp.mpf(12) * k / (n - 1))
        d = fd(xv); r = fR(xv)
        if mn_d is None or d < mn_d[0]: mn_d = (d, xv)
        rmin = r if rmin is None or r < rmin else rmin
        rmax = r if rmax is None or r > rmax else rmax
        if not (d > 0): ok = False
        if not (1 - mp.mpf(10) ** -30 <= r <= upper): ok = False
    # extreme points: tail of dR/dx*x relative, reported only
    sym_ok = (sp.simplify(lim0 - 1) == 0) and (sp.simplify(liminf - 1 / (2 * g)) == 0)
    return {
        "method": "grid check of dR/dx > 0 on 10^4 log-spaced points x in [1e-8, 1e4], mpmath 50 digits; NOT a symbolic proof",
        "R_in_terms_of_x": "x/((1+2*gamma*x)*(1-exp(-x))), x=-z/2",
        "limit_x_to_0": str(lim0), "limit_x_to_inf": str(liminf), "upper_1_over_2gamma": float(upper),
        "limits_ok": bool(sym_ok),
        "min_dR_dx_on_grid": float(mn_d[0]), "min_dR_dx_at_x": float(mn_d[1]),
        "R_min_on_grid": float(rmin), "R_max_on_grid": float(rmax),
        "grid_derivative_positive_and_bounds_ok": bool(ok),
        "pass": bool(ok and sym_ok),
    }


def claim4(g):
    lam, h, ho = sp.symbols("lam h ho", real=True)
    P = 1 - ho * g * lam
    W = 1 - h * g * lam
    E = sp.simplify(1 - W / P)
    plan_T = (1 - h / ho) * (1 / P - 1)
    sign_rel = sp.simplify(plan_T + E)   # 0 means plan_T == -(I - W/P)
    same_rel = sp.simplify(plan_T - E)
    # symbolic: |E| = |1-h/ho| * t/(1+t), t = ho*g*|lam| >= 0
    t = sp.Symbol("t", nonnegative=True)
    ratio_form = sp.simplify(E.subs(lam, -t / (ho * g)) / (1 - h / ho))  # should equal -t/(1+t)... magnitude t/(1+t)
    exact_ok = True; worst = Fraction(0)
    ratios = [Fraction(k, 20) for k in range(1, 81)]   # h/ho in (0.05, 4]
    tvals = [Fraction(0)] + [Fraction(10) ** e for e in range(-6, 13)] + [Fraction(k, 7) for k in range(1, 50)]
    gf = Fraction(g.p, g.q)
    for r in ratios:
        for tv in tvals:
            # lambda = -tv/(ho*g), ho=1
            lamv = -tv / gf
            Pv = 1 - lamv * gf
            Wv = 1 - r * gf * lamv
            lhs = abs(1 - Wv / Pv)
            rhs = 2 * abs(1 - r)
            if lhs > rhs: exact_ok = False
            if abs(1 - r) > 0:
                worst = max(worst, lhs / abs(1 - r))
    # random symmetric NSD 3x3, 2-norm
    rng = np.random.default_rng(SEED)
    worst_mat = 0.0
    for _ in range(2000):
        Q = rng.standard_normal((3, 3)); J = -(Q @ Q.T) * 10 ** rng.uniform(-3, 4)
        hold = 10 ** rng.uniform(-3, 1); r = rng.uniform(0.05, 4); hnew = r * hold
        gf_ = float(g)
        Pm = np.eye(3) - hold * gf_ * J; Wm = np.eye(3) - hnew * gf_ * J
        lhs = np.linalg.norm(np.eye(3) - np.linalg.solve(Pm, Wm), 2)
        if abs(1 - r) > 1e-9: worst_mat = max(worst_mat, lhs / (2 * abs(1 - r)))
    return {
        "exact_error_ratio_1_minus_W_over_P": str(E),
        "sup_ratio_error_over_abs_1_minus_h_over_hold_on_exact_grid": float(worst),
        "bound_with_factor_2_holds_on_exact_rational_grid": bool(exact_ok),
        "grid_size": len(ratios) * len(tvals),
        "symbolic": "|1 - W/P| = |1 - h/h_old| * t/(1+t) with t = h_old*gamma*|lam| >= 0, hence <= |1 - h/h_old| < 2|1 - h/h_old|",
        "ratio_form_E_over_(1-h/ho)": str(ratio_form),
        "plan_identity_T_equals_I_minus_Pinv_W": bool(same_rel == 0),
        "plan_identity_T_equals_minus_(I_minus_Pinv_W)": bool(sign_rel == 0),
        "plan_remark": "T = (1-h/ho)(P^-1 - I) equals P^-1 W - I, the negative of I - P^-1 W; norms agree so the bound is unaffected",
        "extra_random_symmetric_nsd_3x3_worst_ratio_to_bound": worst_mat,
        "pass": bool(exact_ok and sp.simplify(E.subs(lam, -t / (ho * g)) ** 2 - ((1 - h / ho) * t / (1 + t)) ** 2) == 0),
    }


def claim5():
    a = [4096, 1, 1, 1]; q = Fraction(1, 2); B = Fraction(1, 16)
    S = sum(a)
    kc = next(k for k in range(0, 100) if S * q ** k <= B)
    prev = S * q ** (kc - 1)
    alloc = (18, 6, 6, 6)
    lhs = sum(Fraction(ai) * q ** ki for ai, ki in zip(a, alloc))
    # exhaustive integer optimum, integers scaled by 2^40
    K = 41
    best = None; nbest = 0
    for ks in itertools.product(range(K), repeat=4):
        if sum(a[i] * 2 ** (40 - ks[i]) for i in range(4)) <= 2 ** 40 // 16:
            tot = sum(ks)
            if best is None or tot < best[0]: best = (tot, ks); nbest = 1
            elif tot == best[0]: nbest += 1
    ok = (kc == 17 and 4 * kc == 68 and prev > B and S * q ** kc <= B and lhs == B and sum(alloc) == 36)
    return {
        "sum_a": S, "common_k": kc, "common_total": 4 * kc,
        "common_residual_at_k-1": str(prev), "common_residual_at_k": str(S * q ** kc),
        "common_k_minus_1_exceeds_B": bool(prev > B),
        "alloc": list(alloc), "alloc_lhs_exact": str(lhs), "alloc_equals_B_exactly": bool(lhs == B), "alloc_total": sum(alloc),
        "exhaustive_integer_optimum_total": best[0], "exhaustive_optimum_example": list(best[1]),
        "exhaustive_optimum_count_ordered": nbest, "pass": bool(ok),
    }


def claim6(g, A, C):
    rng = np.random.default_rng(SEED)
    R = sp.Rational
    J0 = sp.Matrix(3, 3, [R(int(v), 8) for v in rng.integers(-40, 40, 9)])
    f0 = sp.Matrix(3, 1, [R(int(v), 8) for v in rng.integers(-40, 40, 3)])
    h = R(1, 10)
    W = sp.eye(3) - h * g * J0
    detW = W.det()
    lam, th = sp.symbols("lam th")
    U = [sp.Matrix(sp.symbols(f"u{i}_0:3")) for i in range(8)]
    allU = [c for ui in U for c in ui]

    def Nf(x):
        return sp.Matrix([x[0] * x[1], sp.sin(x[2]) * x[0], x[1] ** 2 + sp.exp(x[0] / 3) - 1])

    def Hsys(nonlin):
        eta = th + lam * (1 - th)
        rows = []
        for i in range(8):
            xi = sp.zeros(3, 1); cs = sp.zeros(3, 1)
            for j in range(i):
                xi += A[i, j] * U[j]; cs += C[i, j] * U[j]
            Hi = W * U[i] - eta * (g * cs + h * g * J0 * xi) - h * g * f0
            if nonlin: Hi -= lam * h * g * Nf(xi)
            rows.append(Hi)
        return sp.Matrix.vstack(*rows)

    out = {"J0": [[str(J0[i, j]) for j in range(3)] for i in range(3)], "h": "1/10", "detW": str(detW),
           "detW_float": float(detW)}
    cases = [(1, 0), (1, R(1, 2)), (1, 1), (R(1, 2), 0), (R(1, 2), R(1, 2)), (0, 1)]
    # exact rational check for the linear part (N = 0)
    H_lin = Hsys(False)
    Jlin = H_lin.jacobian(allU)
    lin_res = []
    for (lv, tv) in cases:
        Jn = Jlin.subs({lam: lv, th: tv})
        d = Jn.det(method="berkowitz")
        diag_ok = all(Jn[3 * i:3 * i + 3, 3 * i:3 * i + 3] == W for i in range(8))
        up_ok = all(Jn[3 * i:3 * i + 3, 3 * j:3 * j + 3] == sp.zeros(3, 3) for i in range(8) for j in range(i + 1, 8))
        lin_res.append({"lambda": str(lv), "theta": str(tv), "exact_det_equals_detW^8": bool(sp.simplify(d - detW ** 8) == 0),
                        "diag_blocks_equal_W": bool(diag_ok), "upper_blocks_zero": bool(up_ok)})
    out["linear_exact_rational"] = lin_res
    # nonlinear: symbolic 24x24 Jacobian, evaluated at random points in 50-digit arithmetic
    H = Hsys(True)
    Jac = H.jacobian(allU)
    nl_res = []
    ok_all = all(r["exact_det_equals_detW^8"] and r["diag_blocks_equal_W"] and r["upper_blocks_zero"] for r in lin_res)
    for (lv, tv) in cases:
        Jc = Jac.subs({lam: lv, th: tv})
        fJ = sp.lambdify(allU, Jc, "mpmath")
        worst = mp.mpf(0); diag_err = mp.mpf(0)
        for trial in range(3):
            pt = [mp.mpf(float(v)) for v in rng.uniform(-1.5, 1.5, 24)]
            Mx = mp.matrix(fJ(*pt))
            d = mp.det(Mx)
            ref = mp.mpf(sp.Rational(detW ** 8).p) / mp.mpf(sp.Rational(detW ** 8).q)
            worst = max(worst, abs(d - ref) / abs(ref))
            for i in range(8):
                for a_ in range(3):
                    for b_ in range(3):
                        wv = W[a_, b_]
                        diag_err = max(diag_err, abs(Mx[3 * i + a_, 3 * i + b_] - mp.mpf(wv.p) / mp.mpf(wv.q)))
        ok = worst < mp.mpf(10) ** -30 and diag_err < mp.mpf(10) ** -40
        ok_all = ok_all and ok
        nl_res.append({"lambda": str(lv), "theta": str(tv), "max_rel_err_det_vs_detW^8": float(worst),
                       "max_abs_err_diag_blocks_vs_W": float(diag_err), "pass": bool(ok)})
    out["nonlinear_numeric_50_digits"] = nl_res
    out["pass"] = bool(ok_all)
    return out


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out", required=True)
    args = ap.parse_args()
    g, A, C, b = load()
    w, p, m = build_polys(g, A, C, b)
    res = {
        "schema": "vigilode-production-theory-verification-v1",
        "node": "production_theory_20261021", "seed": SEED,
        "versions": {"python": platform.python_version(), "sympy": sp.__version__, "mpmath": mp.__version__,
                     "numpy": np.__version__},
        "gamma": str(g),
        "claim1_T1_numbers": claim1(g, m, w),
        "claim2_preconditioned_sensitivity": claim2(g, p, w),
        "claim3_lemma3_R": claim3(g),
        "claim4_corollary2a": claim4(g),
        "claim5_T9_example": claim5(),
        "claim6_stage_homotopy_det": claim6(g, A, C),
    }
    res["m_polynomials_in_w_degree"] = [int(sp.Poly(m[0, j], w).degree()) for j in range(8)]
    res["summary"] = {k: res[k]["pass"] for k in res if k.startswith("claim")}
    json.dump(res, open(args.out, "w"), indent=1, sort_keys=True, default=str)
    print(json.dumps(res["summary"], indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
