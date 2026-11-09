#!/usr/bin/env python3
"""Exact symbolic certificate for the published PTSW2B peer-W tableau.

Requires SymPy. This checks algebraic identities only. It does not execute an
ODE, prove the full L-stability sector, or qualify a solver implementation.
"""
from __future__ import annotations

import sympy as sp


def main() -> None:
    sigma = sp.symbols("sigma", positive=True)
    r3 = sp.sqrt(3)
    gamma = (3-r3)/2
    c = sp.Matrix([3-2*r3, 1])
    one = sp.ones(2, 1)
    V0 = sp.Matrix([[1, c[i]] for i in range(2)])
    P2 = sp.Matrix([[1, 1], [0, 1]])
    D = sp.diag(1, 2)
    S = sp.diag(1, sigma)
    C = sp.diag(c[0], c[1])

    # Podhaisky (2002), equation (2.14), specialized to q=p=s=2.
    A = sp.simplify(C*V0*S*D.inv()*P2*V0.inv())
    Gamma = sp.simplify(-gamma*V0*S*P2*V0.inv())
    beta = sp.simplify(A+Gamma)
    b = sp.Matrix([[0, gamma]])
    v = sp.simplify(beta[1, :])  # stiff accuracy: v^T=e_2^T beta

    # Exact variable-step C(2), Gamma(2), B(2) identities.
    cres, gres, bres = [], [], []
    for ell in (1, 2):
        old = (c-one).applyfunc(lambda x: x**(ell-1))
        new = c.applyfunc(lambda x: x**(ell-1))
        cres.append(sp.simplify(
            A*old-sigma**(ell-1)*c.applyfunc(lambda x: x**ell)/ell
        ))
        gres.append(sp.simplify(
            Gamma*old+gamma*sigma**(ell-1)*new
        ))
        bres.append(sp.simplify(
            (b*sigma**(ell-1)*new+v*old)[0]-sigma**(ell-1)/ell
        ))
    assert all(x == sp.zeros(2, 1) for x in cres)
    assert all(x == sp.zeros(2, 1) for x in gres)
    assert all(x == 0 for x in bres)

    # Constant-step stiff-limit amplification matrix.
    beta1 = beta.subs(sigma, 1)
    v1 = v.subs(sigma, 1)
    w_inf = -1/gamma
    M_inf = sp.Matrix.vstack(
        sp.Matrix.hstack(sp.simplify(w_inf*beta1),
                         sp.simplify(w_inf*one)),
        sp.Matrix.hstack(
            sp.simplify(b*w_inf*beta1+v1),
            sp.Matrix([[sp.simplify(1+(b*w_inf*one)[0])]])
        )
    )
    lam = sp.symbols("lambda")
    assert sp.factor(M_inf.charpoly(lam).as_expr()) == lam**3
    assert sp.simplify(M_inf**3) == sp.zeros(3)
    assert sp.simplify(M_inf**2) != sp.zeros(3)

    # Source leading error coefficient.
    C3 = sp.simplify(2*gamma**2-sp.Rational(7, 2)*gamma+sp.Rational(7, 6))
    assert sp.simplify(C3-(sp.Rational(23, 12)-5*r3/4)) == 0

    # Source lower-order embedded pair, equation (4.7), specialized to PTSW2B.
    eps = sp.Rational(1, 10)
    b_emb = sp.simplify(sp.Rational(19, 20)*b)
    V1 = sp.Matrix([[1, c[i]-1] for i in range(2)])
    v_emb = sp.simplify(
        (sp.Matrix([[1, 1+eps]])*S*D.inv()-b_emb*V0*S)*V1.inv()
    )
    v_emb_expected = sp.Matrix([[
        (8*r3-11)*sigma/80,
        (11-8*r3)*sigma/80+(19*r3-17)/40
    ]])
    assert sp.simplify(b_emb-sp.Matrix([[0, 19*gamma/20]])) == sp.zeros(1, 2)
    assert sp.simplify(v_emb-v_emb_expected) == sp.zeros(1, 2)

    print("PASS: C(2), Gamma(2), B(2) vanish identically in sigma.")
    print("PASS: det(lambda I - M(infinity)) = lambda^3.")
    print("PASS: M(infinity)^3 = 0 and M(infinity)^2 != 0 (nilpotency index 3).")
    print(f"PASS: C3 = {sp.simplify(C3)}; |C3| = {sp.N(abs(C3), 14)}.")
    print("PASS: embedded b_e and v_e(sigma) match the source construction.")
    print("LIMIT: symbolic algebra only; no full sector proof, IVP integration, or runtime qualification.")


if __name__ == "__main__":
    main()
