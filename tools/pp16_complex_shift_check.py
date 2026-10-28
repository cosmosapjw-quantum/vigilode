#!/usr/bin/env python3
"""Independent oracle of research node PP16 (research/pp16_complex_shift_gain_20261004).

For every case written by crates/rodas5p-core/tests/pp16_complex_shift_study.rs it
solves the exact binary target (I - gamma h J) x = b: exact complex Fraction Gauss
elimination for n <= 6, mpmath at 50 digits for n = 16, 24. It compares each reported
bound (single shifts, jet columns, partial-fraction terms and outputs) with the actual
Euclidean error, checks that every negative control failed closed, and evaluates the
preregistered prediction G3 (actual error > ||r||_2, computed exactly, among the random
complex shifts). No archived code is used.
"""

from __future__ import annotations

import argparse
import json
import struct
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-pp16-complex-shift-check-v1"
EXACT_MAX_N = 6


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def fx(text: str) -> F:
    return F(unhex(text))


def fxs(values):
    return [fx(v) for v in values]


# Complex Fractions as (re, im) pairs.
def cadd(a, b):
    return (a[0] + b[0], a[1] + b[1])


def csub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def cmul(a, b):
    return (a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0])


def cdiv(a, b):
    d = b[0] * b[0] + b[1] * b[1]
    return ((a[0] * b[0] + a[1] * b[1]) / d, (a[1] * b[0] - a[0] * b[1]) / d)


def abs2(a):
    return a[0] * a[0] + a[1] * a[1]


def system(j, h, gamma):
    n = len(j)
    return [[csub((F(int(i == k)), F(0)), cmul(gamma, (h * j[i][k], F(0)))) for k in range(n)]
            for i in range(n)]


def exact_solve(a, b):
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for k in range(n):
        pivot = next((i for i in range(k, n) if m[i][k] != (0, 0)), None)
        if pivot is None:
            raise ValueError("singular exact oracle target")
        m[k], m[pivot] = m[pivot], m[k]
        for i in range(k + 1, n):
            if m[i][k] != (0, 0):
                factor = cdiv(m[i][k], m[k][k])
                m[i] = [csub(x, cmul(factor, y)) for x, y in zip(m[i], m[k])]
    x = [None] * n
    for i in reversed(range(n)):
        s = m[i][n]
        for k in range(i + 1, n):
            s = csub(s, cmul(m[i][k], x[k]))
        x[i] = cdiv(s, m[i][i])
    return x


def to_mp(q: F):
    return mp.mpf(q.numerator) / q.denominator


def mp_solve(a, b):
    n = len(b)
    am = mp.matrix(n, n)
    for i in range(n):
        for k in range(n):
            am[i, k] = mp.mpc(to_mp(a[i][k][0]), to_mp(a[i][k][1]))
    bm = mp.matrix([mp.mpc(to_mp(v[0]), to_mp(v[1])) for v in b])
    x = mp.lu_solve(am, bm)
    return [x[i] for i in range(n)]


class Solver:
    """Exact (n <= 6) or 50-digit solutions, cached per (operator, gamma, b)."""

    def __init__(self, operators):
        self.operators = operators
        self.cache = {}

    def solve(self, op, gamma, b):
        key = (op, gamma, tuple(b))
        if key not in self.cache:
            j, h, n = self.operators[op]
            a = system(j, h, gamma)
            self.cache[key] = (exact_solve(a, b) if n <= EXACT_MAX_N else mp_solve(a, b), a)
        return self.cache[key]


def error_measures(n, x, u):
    """Squared error, exact (Fraction) for n <= 6 else mpf."""
    if n <= EXACT_MAX_N:
        return sum(abs2(csub(ui, xi)) for ui, xi in zip(u, x))
    return mp.fsum(abs(mp.mpc(to_mp(ui[0]), to_mp(ui[1])) - xi) ** 2 for ui, xi in zip(u, x))


def as_mp(value):
    return to_mp(value) if isinstance(value, F) else value


def check_column(solver, op, gamma, b, u, cert, tol, label, family):
    j, h, n = solver.operators[op]
    x, a = solver.solve(op, gamma, b)
    error_sq = error_measures(n, x, u)
    residual = []
    for i in range(n):
        r = b[i]
        for k in range(n):
            r = csub(r, cmul(a[i][k], u[k]))
        residual.append(r)
    residual_sq = sum(abs2(r) for r in residual)
    bound = fx(cert["error_upper"])
    residual_upper = fx(cert["residual_l2_upper"])
    gain_upper = fx(cert["gain_upper"])
    tol_value = fx(cert["absolute_tolerance"])
    assert tol_value == tol
    if n <= EXACT_MAX_N:
        enclosed = error_sq <= bound * bound
        above_residual = error_sq > residual_sq
    else:
        enclosed = error_sq <= to_mp(bound) ** 2
        above_residual = error_sq > to_mp(residual_sq)
    gain_ok = gain_upper * gain_upper * gamma[0] * gamma[0] >= abs2(gamma) and gamma[0] > 0
    residual_ok = residual_sq <= residual_upper * residual_upper
    certified = cert["status"] == "Certified"
    safe = (not certified) or (bound <= tol and (error_sq <= tol * tol if n <= EXACT_MAX_N
                                                   else error_sq <= to_mp(tol) ** 2))
    error = mp.sqrt(as_mp(error_sq))
    return {
        "id": label, "family": family, "n": n, "status": cert["status"],
        "gamma": [float(gamma[0]), float(gamma[1])],
        "error_upper": float(bound), "actual_error": mp.nstr(error, 8),
        "residual_l2_exact": mp.nstr(mp.sqrt(to_mp(residual_sq)), 8),
        "gain_upper": float(gain_upper),
        "upper_over_actual": (mp.nstr(to_mp(bound) / error, 6) if error > 0 else None),
        "error_over_residual": (mp.nstr(error / mp.sqrt(to_mp(residual_sq)), 6) if residual_sq > 0 else None),
        "enclosed": bool(enclosed), "gain_enclosed": bool(gain_ok), "residual_enclosed": bool(residual_ok),
        "safe_acceptance": bool(safe), "actual_error_above_residual": bool(above_residual),
        "oracle": "exact-fraction" if n <= EXACT_MAX_N else "mpmath-50",
    }


def complex_vector(re_hex, im_hex):
    return [(r, i) for r, i in zip(fxs(re_hex), fxs(im_hex))]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--g4-contract-and-lint-passed", action="store_true",
                        help="set only when fmt, clippy and the contract tests passed in the same command chain")
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 50
    data = json.loads(args.cases.read_text())
    assert data["schema"] == "vigilode-pp16-complex-shift-cases-v1"
    tol = fx(data["absolute_tolerance"])
    operators = {}
    dissipative = True
    for op in data["operators"]:
        n = op["n"]
        flat = fxs(op["j_row_major"])
        j = [flat[i * n:(i + 1) * n] for i in range(n)]
        for i in range(n):
            dissipative &= j[i][i] + sum(abs((j[i][k] + j[k][i]) / 2) for k in range(n) if k != i) <= 0
        operators[op["id"]] = (j, fx(op["h"]), n)
    solver = Solver(operators)

    rows = []
    for case in data["singles"]:
        gamma = (fx(case["gamma_re"]), fx(case["gamma_im"]))
        rows.append(check_column(solver, case["operator"], gamma, complex_vector(case["b_re"], case["b_im"]),
                                 complex_vector(case["u_re"], case["u_im"]), case["certificate"], tol,
                                 case["id"], case["family"]))
    jet_radius_ok = True
    for jet in data["jets"]:
        gamma0 = fx(jet["gamma0"])
        rhs = [complex_vector(r, i) for r, i in zip(jet["rhs_re"], jet["rhs_im"])]
        for t, cand in enumerate(jet["candidates"]):
            gamma = (fx(cand["gamma_re"]), fx(cand["gamma_im"]))
            z = ((gamma[0] - gamma0) / gamma0, gamma[1] / gamma0)
            upper = fx(cand["z_modulus_upper"])
            jet_radius_ok &= abs2(z) < 1 and abs2(z) <= upper * upper and upper < 1
            for c, (b, ur, ui, cert) in enumerate(zip(rhs, cand["columns_re"], cand["columns_im"],
                                                      cand["certificates"])):
                rows.append(check_column(solver, jet["operator"], gamma, b, complex_vector(ur, ui), cert, tol,
                                         f"{jet['id']}_t{t}_c{c}", "jet"))
    pf_rows = []
    for pf in data["partial_fractions"]:
        op = pf["operator"]
        n = operators[op][2]
        b = complex_vector(pf["b_re"], pf["b_im"])
        c0 = (fx(pf["c0_re"]), fx(pf["c0_im"]))
        y = complex_vector(pf["output_re"], pf["output_im"])
        target_exact = [cmul(c0, bi) for bi in b] if n <= EXACT_MAX_N else None
        target_mp = [mp.mpc(to_mp(v[0]), to_mp(v[1])) for v in (cmul(c0, bi) for bi in b)]
        y_of_u = [cmul(c0, bi) for bi in b]
        for t, term in enumerate(pf["terms"]):
            gamma = (fx(term["gamma_re"]), fx(term["gamma_im"]))
            w = (fx(term["weight_re"]), fx(term["weight_im"]))
            u = complex_vector(term["u_re"], term["u_im"])
            rows.append(check_column(solver, op, gamma, b, u, term["certificate"], tol,
                                     f"{pf['id']}_term{t}", f"{pf['family']}_pf_term"))
            x, _ = solver.solve(op, gamma, b)
            y_of_u = [cadd(acc, cmul(w, ui)) for acc, ui in zip(y_of_u, u)]
            if n <= EXACT_MAX_N:
                target_exact = [cadd(acc, cmul(w, xi)) for acc, xi in zip(target_exact, x)]
            else:
                wm = mp.mpc(to_mp(w[0]), to_mp(w[1]))
                target_mp = [acc + wm * xi for acc, xi in zip(target_mp, x)]
        bound = fx(pf["error_upper"])
        radius_sq = sum(abs2(csub(yi, vi)) for yi, vi in zip(y, y_of_u))
        radius_ok = radius_sq <= fx(pf["output_radius_l2_upper"]) ** 2
        if n <= EXACT_MAX_N:
            error_sq = sum(abs2(csub(yi, ti)) for yi, ti in zip(y, target_exact))
            enclosed = error_sq <= bound * bound
        else:
            error_sq = mp.fsum(abs(mp.mpc(to_mp(yi[0]), to_mp(yi[1])) - ti) ** 2 for yi, ti in zip(y, target_mp))
            enclosed = error_sq <= to_mp(bound) ** 2
        error = mp.sqrt(as_mp(error_sq))
        certified = pf["status"] == "Certified"
        safe = (not certified) or (bound <= tol and as_mp(error_sq) <= to_mp(tol) ** 2)
        pf_rows.append({
            "id": pf["id"], "family": pf["family"], "n": n, "status": pf["status"],
            "error_upper": float(bound), "actual_error": mp.nstr(error, 8),
            "upper_over_actual": (mp.nstr(to_mp(bound) / error, 6) if error > 0 else None),
            "output_radius_l2_upper": unhex(pf["output_radius_l2_upper"]),
            "weighted_term_error_upper": unhex(pf["weighted_term_error_upper"]),
            "enclosed": bool(enclosed), "radius_enclosed": bool(radius_ok), "safe_acceptance": bool(safe),
            "oracle": "exact-fraction" if n <= EXACT_MAX_N else "mpmath-50",
        })

    controls = data["negative_controls"]
    control_failures = [c["id"] for c in controls if not c["rejected"] or c["outcome"] == "Certified"]

    def ratio_stats(selected):
        ratios = [mp.mpf(r["upper_over_actual"]) for r in selected if r["upper_over_actual"] is not None]
        if not ratios:
            return None
        ratios.sort()
        return {"min": mp.nstr(ratios[0], 6), "median": mp.nstr(ratios[len(ratios) // 2], 6),
                "max": mp.nstr(ratios[-1], 6), "count": len(ratios),
                "zero_actual_error": sum(r["upper_over_actual"] is None for r in selected)}

    families = sorted({r["family"] for r in rows})
    summary = {fam: {"rows": sum(r["family"] == fam for r in rows),
                     "certified": sum(r["family"] == fam and r["status"] == "Certified" for r in rows),
                     "upper_over_actual": ratio_stats([r for r in rows if r["family"] == fam])}
               for fam in families}
    summary["partial_fraction"] = {"rows": len(pf_rows),
                                   "certified": sum(r["status"] == "Certified" for r in pf_rows),
                                   "upper_over_actual": ratio_stats(pf_rows)}
    single_failures = [r["id"] for r in rows if not (r["enclosed"] and r["gain_enclosed"]
                                                     and r["residual_enclosed"] and r["safe_acceptance"])]
    pf_failures = [r["id"] for r in pf_rows if not (r["enclosed"] and r["radius_enclosed"] and r["safe_acceptance"])]
    g3 = [{"id": r["id"], "n": r["n"], "gamma": r["gamma"], "actual_error": r["actual_error"],
           "residual_l2_exact": r["residual_l2_exact"], "error_over_residual": r["error_over_residual"],
           "error_upper": r["error_upper"]}
          for r in rows if r["family"] == "random" and r["actual_error_above_residual"]]
    gate = {
        "G1_bounds_enclose_actual_error": bool(rows) and bool(pf_rows) and not single_failures
        and not pf_failures and dissipative and jet_radius_ok,
        "G2_negative_controls_fail_closed": bool(controls) and not control_failures,
        "G3_prediction_observed": bool(g3),
        "G4_contract_tests_fmt_clippy": bool(args.g4_contract_and_lint_passed),
    }
    report = {
        "schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "exact_max_n": EXACT_MAX_N,
        "cases_input": str(args.cases), "absolute_tolerance": float(tol),
        "operators_dissipative_exact": bool(dissipative), "jet_radius_exact_below_one": bool(jet_radius_ok),
        "single_column_rows": len(rows), "partial_fraction_rows": len(pf_rows),
        "negative_control_count": len(controls), "negative_control_failures": control_failures,
        "single_failures": single_failures, "partial_fraction_failures": pf_failures,
        "summary": summary,
        "G3_random_rows": sum(r["family"] == "random" for r in rows),
        "G3_cases_actual_error_above_residual": g3,
        "gate": gate,
        "verdict": "PASS" if (gate["G1_bounds_enclose_actual_error"] and gate["G2_negative_controls_fail_closed"]
                              and gate["G4_contract_tests_fmt_clippy"]) else "FAIL",
        "rows": rows, "partial_fraction_rows_detail": pf_rows, "negative_controls": controls,
    }
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({k: report[k] for k in ("single_column_rows", "partial_fraction_rows", "negative_control_count",
                                              "negative_control_failures", "single_failures",
                                              "partial_fraction_failures", "summary", "gate", "verdict")}, indent=1))
    print(f"G3 cases: {len(g3)} of {report['G3_random_rows']} random rows")


if __name__ == "__main__":
    main()
