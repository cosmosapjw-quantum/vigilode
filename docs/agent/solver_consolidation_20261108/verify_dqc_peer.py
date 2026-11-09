#!/usr/bin/env python3
"""Exact symbolic checks for the published 4-stage variable-step DQC EPP method.

Requires SymPy. All tableau entries are rational functions of the step ratio.
Run: python3 verify_dqc_peer.py
"""
from __future__ import annotations

import sympy as sp


def moment_residuals(
    A: sp.Matrix, B: sp.Matrix, c: sp.Matrix,
    theta: sp.Symbol, max_order: int,
) -> list[sp.Matrix]:
    """Return defect moments in the t_(n+1) expansion."""
    s = len(c)
    one = sp.ones(s, 1)
    old = (c - one) / theta
    out: list[sp.Matrix] = []
    for ell in range(max_order + 1):
        cterm = c.applyfunc(lambda x: x**ell)
        bterm = B * old.applyfunc(lambda x: x**ell)
        aterm = (
            sp.zeros(s, 1) if ell == 0
            else ell * A * old.applyfunc(lambda x: x ** (ell - 1))
        )
        out.append((cterm - bterm - aterm).applyfunc(sp.factor))
    return out


def main() -> None:
    theta = sp.symbols("theta", positive=True)
    c = sp.Matrix([0, sp.Rational(1, 4), sp.Rational(1, 2), 1])
    b_row = sp.Matrix([[
        sp.Rational(1, 6), sp.Rational(1, 2),
        sp.Rational(1, 6), sp.Rational(1, 6),
    ]])
    B = sp.ones(4, 1) * b_row

    A = sp.Matrix([
        [
            (1 - 24*theta + 12*theta**2)/(96*theta),
            sp.Rational(5, 16)/theta,
            sp.Rational(1, 2),
            (29 - 24*theta - 12*theta**2)/(96*theta),
        ],
        [
            (-39 + 37*theta + 62*theta**2 + 50*theta**3)/(192*theta),
            (2*theta + 5)/(16*theta),
            (41 - 55*theta - 92*theta**2 - 50*theta**3)/(96*theta),
            (17 + 97*theta + 122*theta**2 + 50*theta**3)/(192*theta),
        ],
        [
            (1 - 30*theta)/(96*theta),
            (4*theta + 5)/(16*theta),
            sp.Rational(1, 4),
            (29 + 30*theta)/(96*theta),
        ],
        [
            (1 - 42*theta - 36*theta**2)/(96*theta),
            (8*theta + 5)/(16*theta),
            sp.Rational(1, 8),
            (29 + 78*theta + 36*theta**2)/(96*theta),
        ],
    ])

    # Fixed B is a rank-one projector: a simple unit eigenvalue and zeros.
    assert B * B == B
    assert B * sp.ones(4, 1) == sp.ones(4, 1)
    assert B.rank() == 1

    base = moment_residuals(A, B, c, theta, 3)
    assert base[0] == sp.zeros(4, 1)
    assert base[1] == sp.zeros(4, 1)
    beta3 = sp.Matrix([sp.Rational(1, 40)] * 4)
    assert base[2] == sp.Matrix([
        sp.Rational(1, 4), -sp.Rational(1, 4),
        sp.Rational(1, 4), sp.Rational(1, 4),
    ])
    assert sp.simplify(B * base[2]) == sp.zeros(4, 1)
    assert sp.simplify(B * base[3]) == sp.zeros(4, 1)
    assert sp.simplify(A * base[2]) == sp.zeros(4, 1)

    # Published embedded formula, with the same B and peer nodes.
    V0 = sp.Matrix([[c[i]**j for j in range(4)] for i in range(4)])
    V1 = sp.Matrix([[(c[i] - 1)**j for j in range(4)] for i in range(4)])
    C = sp.diag(*list(c))
    D = sp.diag(1, 2, 3, 4)
    S = sp.diag(1, theta, theta**2, theta**3)
    e4t = sp.Matrix([[0, 0, 0, 1]])

    def a_emb(beta: sp.Matrix) -> sp.Matrix:
        return sp.simplify(
            (C * V0 * D.inv() - sp.Rational(1, 4) * beta * e4t)
            * S * V1.inv()
            - (1/theta) * B * (C - sp.eye(4)) * V1 * D.inv() * V1.inv()
        )

    Aemb3 = a_emb(beta3)
    Aemb4 = a_emb(sp.zeros(4, 1))
    emb3 = moment_residuals(Aemb3, B, c, theta, 4)
    emb4 = moment_residuals(Aemb4, B, c, theta, 5)
    for ell in range(3):
        assert emb3[ell] == sp.zeros(4, 1)
    assert emb3[3] == beta3
    for ell in range(5):
        assert emb4[ell] == sp.zeros(4, 1)
    assert sp.simplify(B * base[2]) == sp.zeros(4, 1)

    print("PASS: B is a rank-one projector (eigenvalues 1,0,0,0).")
    print("PASS: base EPP order-2 moments AB(0)=AB(1)=0.")
    print("PASS: AB(2)=", list(base[2]))
    print("PASS: DQC identities B AB(2)=B AB(3)=A(theta) AB(2)=0 exactly.")
    print("PASS: order-3 embedded moments AB_emb(0..2)=0; AB_emb(3)=beta.")
    print("PASS: order-4 embedded moments AB_emb(0..4)=0.")
    print("PASS: embedded-pair condition B_emb AB_base(2)=0.")


if __name__ == "__main__":
    main()
