#!/usr/bin/env python3
"""The fresh problem set of V30 (node V30_ROUTER_BREAKEVEN_V27B), one source for the draft
PREDECLARED.json (draft_predeclared.py) and for the definition checks (check_fresh_set.py).

Nothing here runs a VigilODE driver. Each problem carries its text definition (what the runner of
phase 2 implements), y0, span, Jacobian kind, autonomy, tolerance pairs, reference, the reason it
is in the set, and numpy / mpmath callables used only by check_fresh_set.py (closed-form residuals,
an independent scipy integration, a stiffness estimate).
"""
import math

import mpmath
import numpy as np

mpmath.mp.dps = 50
ALL3 = ["A", "L", "S"]
TIGHT = "Rodas5p::solve at rtol 1e-12, atol 1e-14 (N build, default h0 and max_steps; once more " \
        "with max_steps 10000000 if that run does not succeed), as in V27"


def householder(v):
    v = np.asarray(v, dtype=float)
    return np.eye(len(v)) - 2.0 * np.outer(v, v) / v.dot(v)


def mp_householder(v):
    n = len(v)
    vv = sum(mpmath.mpf(x) ** 2 for x in v)
    return mpmath.matrix([[(1 if i == j else 0) - 2 * mpmath.mpf(v[i]) * v[j] / vv for j in range(n)]
                          for i in range(n)])


def linear_exact(matrix_rows, y0, forcing=None):
    """t -> exp(M t) y0 (+ steady part for constant forcing c: y* = -M^{-1} c), mpmath."""
    M = mpmath.matrix(matrix_rows)
    y0 = mpmath.matrix(y0)
    if forcing is None:
        return lambda t: list(mpmath.expm(M * mpmath.mpf(t)) * y0)
    c = mpmath.matrix(forcing)
    ystar = -(mpmath.inverse(M) * c)
    return lambda t: list(ystar + mpmath.expm(M * mpmath.mpf(t)) * (y0 - ystar))


# ----------------------------------------------------------------------------- matrices
A3 = [[-1e4, 100, -10, 1], [0, -1e3, 10, -10], [0, 0, -1, 10], [0, 0, 0, -0.1]]
B1 = [[-1, 1, 0, 0], [-100, -1, 0, 0], [0, 0, -100, 1], [0, 0, -1e4, -100]]
LATE_B = [[-0.5, 5, 0, 0], [-5, -0.5, 0, 0], [0, 0, -2000, 0], [0, 0, 0, -500]]
LATE_V = [1, 2, 3, 4]
LIN20_V = list(range(1, 21))
CHAIN_K = [1e4, 1e3, 100, 10, 1, 0.1]
PRE = [[-1e5, 2e5, 0], [1e5, -2e5 - 1, 0], [0, 1, 0]]


def late_matrix_np():
    q = householder(LATE_V)
    return q @ np.array(LATE_B, dtype=float) @ q


def late_matrix_mp():
    q = mp_householder(LATE_V)
    return q * mpmath.matrix(LATE_B) * q


def lin20_np():
    q = householder(LIN20_V)
    lam = np.array([10.0 ** (4.0 * i / 19.0) for i in range(20)])
    return -(q @ np.diag(lam) @ q)


def lin20_mp():
    q = mp_householder(LIN20_V)
    lam = mpmath.diag([mpmath.mpf(10) ** (mpmath.mpf(4) * i / 19) for i in range(20)])
    return -(q * lam * q)


def chain_rows():
    k = CHAIN_K
    rows = [[0.0] * 6 for _ in range(6)]
    for i in range(6):
        rows[i][i] = -k[i]
        if i > 0:
            rows[i][i - 1] = k[i - 1]
    return rows


# ----------------------------------------------------------------------------- PDE grids
AC_N = 100
AC_DX = 2.0 / (AC_N + 1)
AC_X = -1.0 + AC_DX * np.arange(1, AC_N + 1)
BU_N = 80
BU_DX = 1.0 / (BU_N + 1)
BU_X = BU_DX * np.arange(1, BU_N + 1)


def ac_rhs(t, u):
    up = np.concatenate(([-1.0], u, [1.0]))
    return 0.01 * (up[:-2] - 2 * up[1:-1] + up[2:]) / AC_DX ** 2 + u - u ** 3


def burgers_rhs(t, u):
    up = np.concatenate(([0.0], u, [0.0]))
    return (-(up[2:] ** 2 - up[:-2] ** 2) / (4 * BU_DX)
            + 0.02 * (up[:-2] - 2 * up[1:-1] + up[2:]) / BU_DX ** 2)


def fput_rhs(t, y):
    q, p = y[:15], y[15:]
    qq = np.concatenate(([0.0], q, [0.0]))
    right, left = qq[2:] - qq[1:-1], qq[1:-1] - qq[:-2]
    return np.concatenate((p, right - left + 0.25 * (right ** 2 - left ** 2)))


KUR_W = np.array([-1.0 + 2.0 * i / 49.0 for i in range(50)])


def kuramoto_rhs(t, th):
    return KUR_W + np.sin(th[None, :] - th[:, None]).sum(axis=1) / 50.0


def kepler(e):
    def rhs(t, y):
        r3 = (y[0] ** 2 + y[1] ** 2) ** 1.5
        return [y[2], y[3], -y[0] / r3, -y[1] / r3]

    def exact(t):
        t = mpmath.mpf(t)
        E = mpmath.findroot(lambda E: E - e * mpmath.sin(E) - t, t)
        d = 1 - e * mpmath.cos(E)
        s = mpmath.sqrt(1 - mpmath.mpf(e) ** 2)
        return [mpmath.cos(E) - e, s * mpmath.sin(E), -mpmath.sin(E) / d, s * mpmath.cos(E) / d]
    return rhs, exact


KEP05 = kepler(mpmath.mpf("0.5"))
KEP09 = kepler(mpmath.mpf("0.9"))


def problems():
    """Every problem of the fresh set, in the order of PREDECLARED.json."""
    P = []

    def add(**kw):
        P.append(kw)

    # ======================================================== stiff or late stiff (FD)
    add(id="pr_sin2t", group="stiff_or_late_stiff", kind="stiff from t0", n=1, autonomous=False,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = -1e4 (y - sin(2 t)) + 2 cos(2 t)",
        y0="[0]", span=[0.0, 5.0], rho=1e4,
        reference="closed form y(t) = sin(2 t)",
        reason="Prothero-Robinson test: stiff relaxation (lambda = -1e4) onto a smooth forced slow "
               "manifold, non-autonomous with the drivers' finite-difference time derivative; the "
               "class of P_scalar_fd but with a closed form and DOPRI5 feasible (about 1.5e4 "
               "attempts); E9 instance",
        rhs=lambda t, y: [-1e4 * (y[0] - math.sin(2 * t)) + 2 * math.cos(2 * t)],
        y0v=[0.0], exact=lambda t: [mpmath.sin(2 * mpmath.mpf(t))])
    add(id="kaps_1e6", group="stiff_or_late_stiff", kind="stiff from t0", n=2, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y1' = -(1e6 + 2) y1 + 1e6 y2^2, y2' = y1 - y2 - y2^2",
        y0="[1, 1]", span=[0.0, 2.0], rho=1e6,
        reference="closed form y1 = exp(-2 t), y2 = exp(-t)",
        reason="Kaps' singular perturbation problem (epsilon = 1e-6): nonlinear, stiff from t0 on "
               "its slow manifold; DOPRI5 infeasible within max_steps (about 6e5 attempts at its "
               "stability limit)",
        rhs=lambda t, y: [-(1e6 + 2) * y[0] + 1e6 * y[1] ** 2, y[0] - y[1] - y[1] ** 2],
        y0v=[1.0, 1.0], exact=lambda t: [mpmath.exp(-2 * mpmath.mpf(t)), mpmath.exp(-mpmath.mpf(t))])
    add(id="a3_linear4", group="stiff_or_late_stiff", kind="stiff from t0", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = M y, M = [[-1e4, 100, -10, 1], [0, -1e3, 10, -10], [0, 0, -1, 10], "
                   "[0, 0, 0, -0.1]] (rows)",
        y0="[1, 1, 1, 1]", span=[0.0, 20.0], rho=1e4,
        reference="closed form expm(M t) y0 (mpmath, 50 digits)",
        reason="linear stiff system with real eigenvalues over five decades and non-normal "
               "upper-triangular coupling (the form of Enright-Hull-Lindberg class A3); E9 instance",
        rhs=lambda t, y: list(np.array(A3) @ np.asarray(y)),
        y0v=[1.0, 1.0, 1.0, 1.0], exact=linear_exact(A3, [1, 1, 1, 1]))
    add(id="b1_linear4", group="stiff_or_late_stiff", kind="stiff from t0", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y1' = -y1 + y2, y2' = -100 y1 - y2, y3' = -100 y3 + y4, y4' = -1e4 y3 - 100 y4",
        y0="[1, 0, 1, 0]", span=[0.0, 20.0], rho=1.5e2,
        reference="closed form expm(M t) y0 (mpmath, 50 digits)",
        reason="complex eigenvalues -1 +- 10i (slowly damped oscillation) and -100 +- 100i (damped "
               "fast mode; the form of Enright-Hull-Lindberg class B): mild stiffness, the grey "
               "zone of the break-even rule",
        rhs=lambda t, y: list(np.array(B1, dtype=float) @ np.asarray(y)),
        y0v=[1.0, 0.0, 1.0, 0.0], exact=linear_exact(B1, [1, 0, 1, 0]))
    add(id="chem_d4", group="stiff_or_late_stiff", kind="stiff from t0", n=3, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y1' = -0.013 y1 - 1000 y1 y3, y2' = -2500 y2 y3, "
                   "y3' = -0.013 y1 - 1000 y1 y3 - 2500 y2 y3",
        y0="[1, 1, 0]", span=[0.0, 50.0], rho=3.5e3,
        reference=TIGHT,
        reason="nonlinear chemical kinetics stiff from t0 (Gear's problem, the form of "
               "Enright-Hull-Lindberg class D4); carries the declared-regime and budget variants",
        rhs=lambda t, y: [-0.013 * y[0] - 1000 * y[0] * y[2], -2500 * y[1] * y[2],
                          -0.013 * y[0] - 1000 * y[0] * y[2] - 2500 * y[1] * y[2]],
        y0v=[1.0, 1.0, 0.0], exact=None)
    add(id="c1_nonlinear4", group="stiff_or_late_stiff", kind="stiff from t0 (mild)", n=4,
        autonomous=True, jacobian="finite-difference", tolerances=["A"],
        definition="y1' = -y1 + y2^2 + y3^2 + y4^2, y2' = -10 y2 + 10 (y3^2 + y4^2), "
                   "y3' = -40 y3 + 40 y4^2, y4' = -100 y4 + 2",
        y0="[1, 1, 1, 1]", span=[0.0, 5.0], rho=1e2,
        reference=TIGHT,
        reason="nonlinear coupling with real eigenvalues -1 to -100 (the form of "
               "Enright-Hull-Lindberg class C): mild stiffness; the span ends while y1 still "
               "relaxes (exp(-5))",
        rhs=lambda t, y: [-y[0] + y[1] ** 2 + y[2] ** 2 + y[3] ** 2, -10 * y[1] + 10 * (y[2] ** 2 + y[3] ** 2),
                          -40 * y[2] + 40 * y[3] ** 2, -100 * y[3] + 2],
        y0v=[1.0, 1.0, 1.0, 1.0], exact=None)
    add(id="lin20_dense", group="stiff_or_late_stiff", kind="stiff from t0", n=20, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y' = M y, M = -Q diag(lambda_1..lambda_20) Q, lambda_i = 10^(4 (i - 1)/19), "
                   "Q = I - 2 v v^T / (v^T v), v = (1, 2, ..., 20)",
        y0="[1, 1, ..., 1] (20 ones)", span=[0.0, 10.0], rho=1e4,
        reference="closed form Q expm(-diag(lambda) t) Q y0 (mpmath, 50 digits)",
        reason="dense symmetric linear stiff system, n = 20 (b = 15), eigenvalues 1 to 1e4 evenly "
               "spread in log scale; every finite-difference Jacobian costs 21 calls",
        rhs=lambda t, y: list(lin20_np() @ np.asarray(y)),
        y0v=[1.0] * 20, exact=None, exact_mp=lambda: (lin20_mp(), [1] * 20))
    add(id="ac_100", group="stiff_or_late_stiff", kind="stiff from t0 (mild), late fast transition",
        n=100, autonomous=True, jacobian="finite-difference", tolerances=["A"],
        definition="Allen-Cahn: u_i' = 0.01 (u_{i-1} - 2 u_i + u_{i+1}) / dx^2 + u_i - u_i^3, "
                   "i = 1..100, dx = 2/101, x_i = -1 + i dx, u_0 = -1, u_101 = 1",
        y0="u_i(0) = 0.53 x_i + 0.47 sin(-1.5 pi x_i)", span=[0.0, 40.0], rho=1.05e2,
        reference=TIGHT,
        reason="large-n reaction-diffusion (after Trefethen, Spectral Methods in MATLAB, program "
               "34, here by finite differences): metastable plateau, then a fast transition near "
               "t = 38 (three interfaces become one, checked with scipy); the span ends in the "
               "relaxation after it; b = 166",
        rhs=ac_rhs, y0v=list(0.53 * AC_X + 0.47 * np.sin(-1.5 * np.pi * AC_X)), exact=None)
    add(id="burgers_80", group="stiff_or_late_stiff", kind="stiff from t0 (mild)", n=80,
        autonomous=True, jacobian="finite-difference", tolerances=["A"],
        definition="viscous Burgers by central differences: u_i' = -(u_{i+1}^2 - u_{i-1}^2)/(4 dx) "
                   "+ 0.02 (u_{i-1} - 2 u_i + u_{i+1}) / dx^2, i = 1..80, dx = 1/81, x_i = i dx, "
                   "u_0 = u_81 = 0",
        y0="u_i(0) = sin(2 pi x_i) + 0.5 sin(pi x_i)", span=[0.0, 1.0], rho=5.3e2,
        reference=TIGHT,
        reason="nonlinear advection-diffusion with a steepening front (cell Peclet number below "
               "1), non-symmetric Jacobian, n = 80 (b = 113): pilots are expensive in operations",
        rhs=burgers_rhs, y0v=list(np.sin(2 * np.pi * BU_X) + 0.5 * np.sin(np.pi * BU_X)), exact=None)
    add(id="pendulum_tracker", group="stiff_or_late_stiff", kind="stiff from t0 (very)", n=3,
        autonomous=True, jacobian="finite-difference", tolerances=["A"],
        definition="y1' = y2, y2' = -sin(y1), y3' = -1e6 (y3 - cos(y1))",
        y0="[1, 0, cos(1)]", span=[0.0, 10.0], rho=1e6,
        reference=TIGHT,
        reason="a nonlinear pendulum with a very stiff component slaved to it: DOPRI5 infeasible "
               "(about 3e6 attempts at its stability limit), while RODAS5P's steps stay "
               "accuracy-limited by the pendulum up to t1",
        rhs=lambda t, y: [y[1], -math.sin(y[0]), -1e6 * (y[2] - math.cos(y[0]))],
        y0v=[1.0, 0.0, math.cos(1.0)], exact=None)
    add(id="late_linear4", group="stiff_or_late_stiff", kind="late stiff", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = M y, M = Q B Q, B = [[-0.5, 5, 0, 0], [-5, -0.5, 0, 0], [0, 0, -2000, 0], "
                   "[0, 0, 0, -500]] (rows), Q = I - 2 v v^T / (v^T v), v = (1, 2, 3, 4)",
        y0="[1, 1, 1, 1]", span=[0.0, 30.0], rho=2e3,
        reference="closed form Q expm(B t) Q y0 (mpmath, 50 digits)",
        reason="late stiffness without a chemistry: a slowly damped oscillation (-0.5 +- 5i) "
               "first limits DOPRI5 by accuracy; once it has decayed (amplitude exp(-0.5 t)) "
               "DOPRI5 is limited by the eigenvalue -2000; the onset time depends on the tolerance",
        rhs=lambda t, y: list(late_matrix_np() @ np.asarray(y)),
        y0v=[1.0, 1.0, 1.0, 1.0], exact=None, exact_mp=lambda: (late_matrix_mp(), [1, 1, 1, 1]))
    add(id="fhn_relax", group="stiff_or_late_stiff", kind="late stiff, alternating", n=2,
        autonomous=True, jacobian="finite-difference", tolerances=ALL3,
        definition="FitzHugh-Nagumo relaxation oscillator: y1' = (y1 - y1^3/3 - y2 + 0.5) / 1e-3, "
                   "y2' = y1 + 0.7 - 0.8 y2",
        y0="[0, 0]", span=[0.0, 19.5], rho=3e3,
        reference=TIGHT,
        reason="relaxation oscillation (period about 2.02): stiffness appears on each slow branch "
               "and disappears at the jumps; starts off the slow manifold; t1 = 19.5 lies in the "
               "middle of a slow phase (jumps near 18.91 and 20.18, checked with scipy)",
        rhs=lambda t, y: [(y[0] - y[0] ** 3 / 3 - y[1] + 0.5) / 1e-3, y[0] + 0.7 - 0.8 * y[1]],
        y0v=[0.0, 0.0], exact=None)
    add(id="pr_ramp", group="stiff_or_late_stiff", kind="late stiff (onset in time)", n=1,
        autonomous=False, jacobian="finite-difference", tolerances=["A"],
        definition="y' = -k(t) (y - cos(3 t)) - 3 sin(3 t), k(t) = 2e4 t^2 / (1 + t^2)",
        y0="[1]", span=[0.0, 4.0], rho=1.9e4,
        reference="closed form y(t) = cos(3 t)",
        reason="stiffness that grows smoothly from k(0) = 0 to k(4) = 1.88e4 (property B2: one "
               "onset), closed form, non-autonomous with the finite-difference time derivative",
        rhs=lambda t, y: [-(2e4 * t * t / (1 + t * t)) * (y[0] - math.cos(3 * t)) - 3 * math.sin(3 * t)],
        y0v=[1.0], exact=lambda t: [mpmath.cos(3 * mpmath.mpf(t))])
    add(id="decay_chain6", group="stiff_or_late_stiff", kind="stiff from t0", n=6, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y1' = -k1 y1, y_i' = k_{i-1} y_{i-1} - k_i y_i (i = 2..6), "
                   "k = (1e4, 1e3, 100, 10, 1, 0.1)",
        y0="[1, 0, 0, 0, 0, 0]", span=[0.0, 20.0], rho=1e4,
        reference="closed form expm(M t) y0 (mpmath, 50 digits)",
        reason="linear decay chain (lower bidiagonal), stiff from t0, with a slow tail through "
               "the last members; n = 6 (b = 5)",
        rhs=lambda t, y: list(np.array(chain_rows()) @ np.asarray(y)),
        y0v=[1.0, 0, 0, 0, 0, 0], exact=linear_exact(chain_rows(), [1, 0, 0, 0, 0, 0]))
    add(id="pre_equilibrium", group="stiff_or_late_stiff", kind="stiff from t0 (very)", n=3,
        autonomous=True, jacobian="finite-difference", tolerances=["A"],
        definition="y1' = -1e5 y1 + 2e5 y2, y2' = 1e5 y1 - 2e5 y2 - y2, y3' = y2",
        y0="[1, 0, 0]", span=[0.0, 10.0], rho=3e5,
        reference="closed form expm(M t) y0 (mpmath, 50 digits)",
        reason="fast reversible pre-equilibrium A <-> B followed by a slow drain B -> C: DOPRI5 "
               "infeasible (about 9e5 attempts at its stability limit); slow, non-trivial dynamics "
               "up to t1 (y1 + y2 about exp(-t/3))",
        rhs=lambda t, y: list(np.array(PRE, dtype=float) @ np.asarray(y)),
        y0v=[1.0, 0.0, 0.0], exact=linear_exact(PRE, [1, 0, 0]))

    # ================================================================== non-stiff (FD)
    add(id="detest_a2", group="non_stiff", kind="non-stiff", n=1, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = -y^3 / 2", y0="[1]", span=[0.0, 20.0],
        reference="closed form y = 1 / sqrt(1 + t)",
        reason="scalar nonlinear decay, a short cheap run (DETEST A2): pilot overhead on short runs",
        rhs=lambda t, y: [-y[0] ** 3 / 2], y0v=[1.0],
        exact=lambda t: [1 / mpmath.sqrt(1 + mpmath.mpf(t))])
    add(id="detest_a3", group="non_stiff", kind="non-stiff", n=1, autonomous=False,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = y cos(t)", y0="[1]", span=[0.0, 20.0],
        reference="closed form y = exp(sin t)",
        reason="non-autonomous non-stiff (DETEST A3): every pilot pays the finite-difference time "
               "derivative",
        rhs=lambda t, y: [y[0] * math.cos(t)], y0v=[1.0],
        exact=lambda t: [mpmath.exp(mpmath.sin(mpmath.mpf(t)))])
    add(id="logistic", group="non_stiff", kind="non-stiff", n=1, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y' = (y / 4) (1 - y / 20)", y0="[1]", span=[0.0, 20.0],
        reference="closed form y = 20 / (1 + 19 exp(-t/4))",
        reason="smooth logistic growth (DETEST A4), an unstable start and a stable end",
        rhs=lambda t, y: [y[0] / 4 * (1 - y[0] / 20)], y0v=[1.0],
        exact=lambda t: [20 / (1 + 19 * mpmath.exp(-mpmath.mpf(t) / 4))])
    add(id="detest_b3", group="non_stiff", kind="non-stiff", n=3, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y1' = -y1, y2' = y1 - y2^2, y3' = y2^2", y0="[1, 0, 0]", span=[0.0, 20.0],
        reference=TIGHT,
        reason="nonlinear chain with a linear invariant y1 + y2 + y3 = 1 (DETEST B3)",
        rhs=lambda t, y: [-y[0], y[0] - y[1] ** 2, y[1] ** 2], y0v=[1.0, 0.0, 0.0], exact=None)
    add(id="spiral_b4", group="non_stiff", kind="non-stiff", n=3, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="r = sqrt(y1^2 + y2^2); y1' = -y2 - y1 y3 / r, y2' = y1 - y2 y3 / r, y3' = y1 / r",
        y0="[3, 0, 0]", span=[0.0, 20.0],
        reference="closed form y1 = (2 + cos t) cos t, y2 = (2 + cos t) sin t, y3 = sin t",
        reason="rotation with a radial modulation (DETEST B4); derived closed form (r = 2 + cos t, "
               "angle t)",
        rhs=lambda t, y: (lambda r: [-y[1] - y[0] * y[2] / r, y[0] - y[1] * y[2] / r, y[0] / r])(
            math.hypot(y[0], y[1])),
        y0v=[3.0, 0.0, 0.0],
        exact=lambda t: (lambda t: [(2 + mpmath.cos(t)) * mpmath.cos(t), (2 + mpmath.cos(t)) * mpmath.sin(t),
                                    mpmath.sin(t)])(mpmath.mpf(t)))
    add(id="rigid_body", group="non_stiff", kind="non-stiff", n=3, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="Euler's rigid body: y1' = y2 y3, y2' = -y1 y3, y3' = -0.51 y1 y2",
        y0="[0, 1, 1]", span=[0.0, 20.0],
        reference="closed form (sn, cn, dn)(t | m = 0.51) (mpmath ellipfun)",
        reason="conservative periodic motion (DETEST B5) with a closed form",
        rhs=lambda t, y: [y[1] * y[2], -y[0] * y[2], -0.51 * y[0] * y[1]], y0v=[0.0, 1.0, 1.0],
        exact=lambda t: [mpmath.ellipfun(k, mpmath.mpf(t), m=mpmath.mpf("0.51")) for k in ("sn", "cn", "dn")])
    add(id="cascade10", group="non_stiff", kind="non-stiff", n=10, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y1' = -y1, y_i' = y_{i-1} - y_i (i = 2..10)",
        y0="[1, 0, ..., 0] (e_1, n = 10)", span=[0.0, 20.0],
        reference="closed form y_i = t^(i-1) exp(-t) / (i-1)!",
        reason="linear cascade, n = 10 (b = 8), a repeated eigenvalue (DETEST C1)",
        rhs=lambda t, y: [-y[0]] + [y[i - 1] - y[i] for i in range(1, 10)], y0v=[1.0] + [0.0] * 9,
        exact=lambda t: [mpmath.mpf(t) ** i * mpmath.exp(-mpmath.mpf(t)) / mpmath.factorial(i)
                         for i in range(10)])
    add(id="kepler_e05", group="non_stiff", kind="non-stiff", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="two-body problem: r = sqrt(y1^2 + y2^2); y1' = y3, y2' = y4, y3' = -y1 / r^3, "
                   "y4' = -y2 / r^3; eccentricity 0.5",
        y0="[0.5, 0, 0, sqrt(3)]", span=[0.0, 20.0],
        reference="closed form by Kepler's equation E - 0.5 sin E = t: y1 = cos E - 0.5, "
                  "y2 = sqrt(0.75) sin E, y3 = -sin E / (1 - 0.5 cos E), "
                  "y4 = sqrt(0.75) cos E / (1 - 0.5 cos E)",
        reason="periodic orbit with a speed ratio 3 (DETEST D3); carries the declared-regime "
               "variants",
        rhs=KEP05[0], y0v=[0.5, 0.0, 0.0, math.sqrt(3.0)], exact=KEP05[1])
    add(id="kepler_e09", group="non_stiff", kind="non-stiff", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="two-body problem as kepler_e05 with eccentricity 0.9",
        y0="[0.1, 0, 0, sqrt(19)]", span=[0.0, 20.0],
        reference="closed form by Kepler's equation E - 0.9 sin E = t: y1 = cos E - 0.9, "
                  "y2 = sqrt(0.19) sin E, y3 = -sin E / (1 - 0.9 cos E), "
                  "y4 = sqrt(0.19) cos E / (1 - 0.9 cos E)",
        reason="highly eccentric orbit (DETEST D5): step sizes vary by a factor of about 19^(3/2) "
               "between pericentre and apocentre, a stress test for false handovers near apocentre",
        rhs=KEP09[0], y0v=[0.1, 0.0, 0.0, math.sqrt(19.0)], exact=KEP09[1])
    add(id="henon_heiles", group="non_stiff", kind="non-stiff", n=4, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="y1' = y3, y2' = y4, y3' = -y1 - 2 y1 y2, y4' = -y2 - y1^2 + y2^2",
        y0="[0, 0.1, 0.5, 0] (energy 0.1297 < 1/6, bounded)", span=[0.0, 50.0],
        reference=TIGHT,
        reason="Hamiltonian with mixed regular and chaotic motion (Henon-Heiles)",
        rhs=lambda t, y: [y[2], y[3], -y[0] - 2 * y[0] * y[1], -y[1] - y[0] ** 2 + y[1] ** 2],
        y0v=[0.0, 0.1, 0.5, 0.0], exact=None)
    add(id="fput_30", group="non_stiff", kind="non-stiff", n=30, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="Fermi-Pasta-Ulam-Tsingou alpha chain, 15 masses, fixed ends: y = (q_1..q_15, "
                   "p_1..p_15), q_i' = p_i, p_i' = (q_{i+1} - 2 q_i + q_{i-1}) + 0.25 ((q_{i+1} - "
                   "q_i)^2 - (q_i - q_{i-1})^2), q_0 = q_16 = 0",
        y0="q_i = sin(pi i / 16), p_i = 0", span=[0.0, 100.0],
        reference=TIGHT,
        reason="large-n non-stiff oscillatory lattice, n = 30 (b = 25), purely imaginary linear "
               "spectrum up to 2i",
        rhs=fput_rhs, y0v=[math.sin(math.pi * i / 16) for i in range(1, 16)] + [0.0] * 15,
        exact=None)
    add(id="duffing_e3", group="non_stiff", kind="non-stiff", n=2, autonomous=False,
        jacobian="finite-difference", tolerances=ALL3,
        definition="forced Duffing: y1' = y2, y2' = y1 - y1^3 / 6 - 2 sin(2.78535 t)",
        y0="[0, 0]", span=[0.0, 20.0],
        reference=TIGHT,
        reason="forced nonlinear oscillator (DETEST E3), non-autonomous",
        rhs=lambda t, y: [y[1], y[0] - y[0] ** 3 / 6 - 2 * math.sin(2.78535 * t)], y0v=[0.0, 0.0],
        exact=None)
    add(id="kuramoto_50", group="non_stiff", kind="non-stiff", n=50, autonomous=True,
        jacobian="finite-difference", tolerances=ALL3,
        definition="Kuramoto oscillators: y_i' = w_i + (1/50) sum_j sin(y_j - y_i), "
                   "w_i = -1 + 2 (i - 1)/49, i = 1..50",
        y0="y_i = 2 pi (i - 1) / 50", span=[0.0, 20.0],
        reference=TIGHT,
        reason="large-n non-stiff with a dense Jacobian (all-to-all coupling), n = 50 (b = 52)",
        rhs=kuramoto_rhs, y0v=[2 * math.pi * i / 50 for i in range(50)], exact=None)

    # ===================================================== callbacks, mass, start rule
    add(id="cb_probe", group="callback_budget_declared", kind="callback", n=1, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y' = -y (1 + y^2); the right-hand side returns a fatal error whenever y > 1",
        y0="[1]", span=[0.0, 1.0],
        reference="closed form y = 1 / sqrt(2 exp(2 t) - 1)",
        reason="the F2 property on a new problem: any call above y0 at t0 (an off-trajectory "
               "probe) fails fatally; the break-even route makes none (E8)",
        rhs=lambda t, y: [-y[0] * (1 + y[0] ** 2)], y0v=[1.0],
        exact=lambda t: [1 / mpmath.sqrt(2 * mpmath.exp(2 * mpmath.mpf(t)) - 1)])
    add(id="cb_column", group="callback_budget_declared", kind="callback", n=2, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y1' = 0, y2' = -1500 (y2 - y1); fatal error whenever y1 > 2 (only a "
                   "finite-difference column reaches it)",
        y0="[2, 0]", span=[0.0, 1.0],
        reference="closed form y1 = 2, y2 = 2 (1 - exp(-1500 t))",
        reason="the N2 property on a new problem: owner decision D1 ends the solve at the first "
               "pilot (E8; the exact set excluded from E3)",
        rhs=lambda t, y: [0.0, -1500 * (y[1] - y[0])], y0v=[2.0, 0.0],
        exact=lambda t: [mpmath.mpf(2), 2 * (1 - mpmath.exp(-1500 * mpmath.mpf(t)))])
    add(id="cb_time", group="callback_budget_declared", kind="callback", n=1, autonomous=True,
        jacobian="finite-difference", tolerances=["A"],
        definition="y' = 1 - y^2; the right-hand side returns a fatal error with the message "
                   "'logger offline' whenever t > 0.4 (declared autonomous: f does not depend on t)",
        y0="[0]", span=[0.0, 1.0],
        reference="none (no arm reaches t1)",
        reason="the P_fatal property on a new problem: the typed stop CallbackFailed{Rhs, "
               "'logger offline'} (E8)",
        rhs=lambda t, y: [1 - y[0] ** 2], y0v=[0.0], exact=None)
    add(id="cb_blowup", group="callback_budget_declared", kind="declared fact (derivative)", n=1,
        autonomous=True, jacobian="dense callback: J = [2 y]", tolerances=["A"],
        definition="y' = 1 + y^2 (tan t, blows up at t = pi/2)", y0="[0]", span=[0.0, 2.0],
        reference="none (no arm reaches t1)",
        reason="the P_blowup property on a new problem: a failure under a declared fact; N must "
               "equal T bit for bit (E3 b)",
        rhs=lambda t, y: [1 + y[0] ** 2], y0v=[0.0], exact=None)
    add(id="cb_mass", group="callback_budget_declared", kind="declared fact (mass matrix)", n=3,
        autonomous=True, jacobian="dense callback: J = [[-1, 0, 2], [1, -3, 0], [1, 1, 1]]",
        tolerances=["A"],
        definition="mass matrix M = diag(1, 1, 0): y1' = -y1 + 2 y3, y2' = y1 - 3 y2, "
                   "0 = y1 + y2 + y3 - 1 (index 1, consistent y0)",
        y0="[1, 0, 0]", span=[0.0, 5.0],
        reference="closed form: z = (y1, y2) solves z' = [[-3, -2], [1, -3]] z + (2, 0), "
                  "z = z* + expm(A t)(z0 - z*), z* = (6/11, 2/11); y3 = 1 - y1 - y2",
        reason="the P_mass property on a new problem: a mass matrix decides the route (DOPRI5 "
               "not admitted)",
        rhs=None, y0v=[1.0, 0.0, 0.0],
        exact=lambda t: (lambda z: [z[0], z[1], 1 - z[0] - z[1]])(
            linear_exact([[-3, -2], [1, -3]], [1, 0], forcing=[2, 0])(t)))
    add(id="sr_chem_dense", group="callback_budget_declared", kind="declared fact (start rule)",
        n=3, autonomous=True,
        jacobian="dense callback: J = [[-0.013 - 1000 y3, 0, -1000 y1], [0, -2500 y3, -2500 y2], "
                 "[-0.013 - 1000 y3, -2500 y3, -1000 y1 - 2500 y2]]",
        tolerances=["A"], definition="chem_d4 with its exact dense Jacobian", y0="[1, 1, 0]",
        span=[0.0, 50.0], reference=TIGHT + " (of chem_d4)",
        reason="start rule D2 with a dense Jacobian", rhs=None, y0v=[1.0, 1.0, 0.0], exact=None)
    add(id="sr_pr_dense_ft", group="callback_budget_declared", kind="declared fact (start rule)",
        n=1, autonomous=False,
        jacobian="dense callback J = [-1e4] and time derivative 2e4 cos(2 t) - 4 sin(2 t)",
        tolerances=["A"], definition="pr_sin2t with its exact Jacobian and time derivative",
        y0="[0]", span=[0.0, 5.0], reference="closed form y(t) = sin(2 t)",
        reason="start rule D2 with a dense Jacobian and a time-derivative callback", rhs=None,
        y0v=[0.0], exact=lambda t: [mpmath.sin(2 * mpmath.mpf(t))])
    add(id="sr_ac_jvp", group="callback_budget_declared", kind="declared fact (start rule)",
        n=100, autonomous=True,
        jacobian="JVP callback: (J v)_i = 0.01 (v_{i-1} - 2 v_i + v_{i+1}) / dx^2 + (1 - 3 u_i^2) v_i, "
                 "v_0 = v_101 = 0",
        tolerances=["A"], definition="ac_100 with an exact Jacobian-vector product",
        y0="as ac_100", span=[0.0, 40.0], reference=TIGHT + " (of ac_100)",
        reason="start rule D2 with a JVP (the matrix-free RODAS5P driver)", rhs=None,
        y0v=None, exact=None)
    add(id="sr_ac_band", group="callback_budget_declared", kind="declared fact (start rule)",
        n=100, autonomous=True,
        jacobian="banded callback (lower 1, upper 1): diagonal -0.02 / dx^2 + 1 - 3 u_i^2, "
                 "off-diagonals 0.01 / dx^2",
        tolerances=["A"], definition="ac_100 with its exact band Jacobian",
        y0="as ac_100", span=[0.0, 40.0], reference=TIGHT + " (of ac_100)",
        reason="start rule D2 with a band Jacobian callback (the banded RODAS5P driver)", rhs=None,
        y0v=None, exact=None)
    return P


def break_even_length(n):
    """b of the route (crates/vigilode/src/unified/breakeven.rs), in integers."""
    callbacks = -(-(n + 11) // 6)
    operations = -(-(2 * n * n + 54 * n + 384) // 156)
    return max(callbacks, operations)
