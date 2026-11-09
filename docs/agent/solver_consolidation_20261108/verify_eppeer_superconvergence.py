#!/usr/bin/env python3
"""Exact EPPEER EPP3f4 source-coefficient/superconvergence audit.

The symbolic part uses exact rationals. The numerical part uses mpmath with
exactly initialized peer values. It is not a Fortran/Rust runtime or benchmark.

Requires: sympy, mpmath.
"""
from __future__ import annotations

import mpmath as mp
import sympy as sp


def build_source_tableau():
    sigma = sp.symbols("sigma", positive=True)
    c = sp.Matrix([0, sp.Rational(3, 4), 1])
    B = sp.Matrix([
        [0, 0, 1],
        [-sp.Rational(12163, 17521920),
         sp.Rational(12163, 7200),
         -sp.Rational(60328969, 87609600)],
        [-sp.Rational(1, 1440), sp.Rational(169, 100),
         -sp.Rational(4963, 7200)],
    ])
    s = len(c)
    V = sp.Matrix([[c[i] ** j for j in range(s)] for i in range(s)])
    Dinv = sp.diag(*[sp.Rational(1, j + 1) for j in range(s)])
    Pscaled = sp.Matrix([
        [1, 1, 1],
        [0, sp.Rational(1, 2), 1],
        [0, 0, sp.Rational(1, 3)],
    ])
    CV = sp.Matrix([[c[i] ** j for j in range(1, s + 1)]
                    for i in range(s)])
    S = sp.ones(s, 1) * sp.Matrix([[0, 0, 1]]) - B
    PA0 = sp.simplify(S * CV * Dinv * V.inv())
    PPV = sp.simplify(Pscaled * V.inv())
    Acode = sp.simplify(
        PA0 + CV * sp.diag(*[sigma ** j for j in range(1, s + 1)]) * PPV
    )
    return sigma, c, B, PA0, PPV, Acode


def moment_residual(sigma, c, B, Acode, q):
    """Return sigma^q times the source peer moment AB(q)."""
    old = c - sp.ones(len(c), 1)
    residual = (sigma * c).applyfunc(lambda x: x ** q)
    residual -= B * old.applyfunc(lambda x: x ** q)
    if q:
        residual -= q * Acode * old.applyfunc(lambda x: x ** (q - 1))
    return residual.applyfunc(sp.factor)


def to_mp_matrix(matrix):
    return mp.matrix([
        [mp.mpf(str(sp.N(matrix[i, j], mp.mp.dps))) for j in range(matrix.cols)]
        for i in range(matrix.rows)
    ])


def numeric_experiment(c, B, PA0, PPV, mesh_kind, level):
    s = len(c)
    c_mp = [mp.mpf(str(sp.N(value, mp.mp.dps))) for value in c]
    Bm, PA0m, PPVm = map(to_mp_matrix, (B, PA0, PPV))
    CVm = mp.matrix([[c_mp[i] ** j for j in range(1, s + 1)]
                     for i in range(s)])
    PE = [PPVm[s - 1, j] for j in range(s)]

    T = mp.mpf(1) / 4
    if mesh_kind == "constant":
        H = T / (level + 1)
        hs = [H] * (level + 1)
    elif mesh_kind == "abrupt":
        H = T / (level * (mp.mpf(1) / 2 + 1 + mp.mpf(1) / 3))
        hs = [v * H for _ in range(level)
              for v in (mp.mpf(1) / 2, mp.mpf(1), mp.mpf(1) / 3)]
    elif mesh_kind == "smooth":
        ratio = 1 + mp.mpf(1) / level
        h0 = T * (ratio - 1) / (ratio ** level - 1)
        hs = [h0 * ratio ** j for j in range(level)]
        H = max(hs)
    else:
        raise ValueError(mesh_kind)

    y_exact = lambda t: mp.e ** t + mp.sin(t)
    y_prime = lambda t: mp.e ** t + mp.cos(t)
    source = lambda t: y_prime(t) - y_exact(t) ** 2
    f = lambda t, y: y ** 2 + source(t)

    # Exact initial peers deliberately remove startup error from this experiment.
    Y = mp.matrix([y_exact(ci * hs[0]) for ci in c_mp])
    t = hs[0]
    h_old = hs[0]
    indicator = mp.nan
    for h_new in hs[1:]:
        ratio = h_new / h_old
        Acode = PA0m + CVm * mp.diag(
            [ratio ** j for j in range(1, s + 1)]
        ) * PPVm
        F_old = mp.matrix([
            f(t + h_old * (ci - 1), Y[j]) for j, ci in enumerate(c_mp)
        ])
        Y_new = Bm * Y + h_old * (Acode * F_old)
        t += h_new
        F_new = mp.matrix([
            f(t + h_new * ci, Y_new[j]) for j, ci in enumerate(c_mp)
        ])
        indicator = h_new * abs(sum(PE[j] * F_new[j] for j in range(s)))
        Y = Y_new
        h_old = h_new

    true_error = abs(Y[s - 1] - y_exact(t))
    return H, true_error, indicator, indicator / true_error


def observed_orders(rows):
    return [
        mp.log(a[1] / b[1]) / mp.log(a[0] / b[0])
        for a, b in zip(rows[:-1], rows[1:])
    ]


def print_table(name, rows):
    print(f"\n{name}")
    for H, err, ind, ratio in rows:
        print(
            f"  H={mp.nstr(H, 9):>12}  true_error={mp.nstr(err, 9):>12}"
            f"  PE_indicator={mp.nstr(ind, 9):>12}"
            f"  indicator/error={mp.nstr(ratio, 7)}"
        )
    print("  observed orders:", [float(mp.nstr(v, 6)) for v in observed_orders(rows)])


def main():
    mp.mp.dps = 60
    sigma, c, B, PA0, PPV, Acode = build_source_tableau()
    s = len(c)
    vT = B[-1, :]

    assert B * sp.ones(s, 1) == sp.ones(s, 1)
    assert sp.simplify(vT * B - vT) == sp.zeros(1, s)
    assert sp.simplify(B ** 2 - sp.ones(s, 1) * vT) == sp.zeros(s, s)
    assert sum(vT) == 1
    for q in range(s + 1):
        assert moment_residual(sigma, c, B, Acode, q) == sp.zeros(s, 1)

    r4 = moment_residual(sigma, c, B, Acode, s + 1)
    projection = sp.factor((vT * r4)[0])
    assert sp.simplify(projection.subs(sigma, 1)) == 0
    assert sp.simplify(projection.subs(sigma, sp.Rational(1, 2))) != 0
    slope_at_one = sp.simplify(sp.diff(projection, sigma).subs(sigma, 1))

    PE = PPV[-1, :]
    PE_moments = [
        sp.simplify((PE * c.applyfunc(lambda x: x ** q))[0])
        for q in range(s)
    ]
    assert PE_moments == [0, 0, sp.Rational(1, 3)]

    print("PASS: EPP3f4 source transform satisfies exact variable-step moments q=0..3.")
    print("PASS: B^2=1*v^T, v^T*B=v^T, v^T*1=1; the unit root is simple.")
    print("PASS: fixed-step superconvergence projection vanishes at sigma=1.")
    print("PASS: arbitrary-variable-step superconvergence condition fails identically.")
    print("  v^T*(sigma^4 AB(4)) =", projection)
    print("  derivative at sigma=1 =", slope_at_one)
    print("PASS: PE moments =", PE_moments,
          "; h*PE*F = h^3*f''/6 + O(h^4) for a smooth scalar RHS.")

    constant = [numeric_experiment(c, B, PA0, PPV, "constant", n)
                for n in (5, 10, 20, 40, 80, 160)]
    smooth = [numeric_experiment(c, B, PA0, PPV, "smooth", n)
              for n in (20, 40, 80, 160, 320, 640)]
    abrupt = [numeric_experiment(c, B, PA0, PPV, "abrupt", n)
              for n in (5, 10, 20, 40, 80, 160, 320)]

    print_table("Constant-step mesh: expected order 4", constant)
    print_table("Smooth geometric mesh sigma=1+1/N: conditional order 4", smooth)
    print_table("Abrupt periodic ratios 1/2, 1, 1/3: generic variable-step order 3", abrupt)
    print("\nNOTE: Exact starting peer values isolate the method recurrence. These are")
    print("high-precision Python experiments, not EPPEER Fortran/VigilODE runtime tests.")
    print("The PE indicator is asymptotically conservative on smooth order-4 meshes:")
    print("it remains O(h^3) while the true solution error is O(h^4).")


if __name__ == "__main__":
    main()
