#!/usr/bin/env python3
"""Problem definitions of loop node V07 (stage a) for the independent references.

Every model is the ODE of the Rust model with the same id in
crates/rodas5p-integrators/tests/stage_clients_speculative_problems.rs; the
expressions are written in the same order of operations. The agreement of the
two definitions is checked by check_models.py on probe states printed by the
Rust measurement program.

A model is a dict:
  id, n, t_span, y0, f(t, y) -> ndarray, jac(t, y) -> ndarray (n x n),
  atol_scale (measurement atol = rtol * atol_scale) or atol_fixed,
  exact(t) -> list of mpmath numbers (closed-form models only).
"""
import math

import numpy as np

# ---------------------------------------------------------------- Robertson


def robertson():
    def f(t, y):
        return np.array([
            -0.04 * y[0] + 1.0e4 * y[1] * y[2],
            0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
            3.0e7 * y[1] * y[1],
        ])

    def jac(t, y):
        return np.array([
            [-0.04, 1.0e4 * y[2], 1.0e4 * y[1]],
            [0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1]],
            [0.0, 6.0e7 * y[1], 0.0],
        ])

    return {"id": "robertson", "n": 3, "t_span": (0.0, 40.0), "y0": [1.0, 0.0, 0.0],
            "f": f, "jac": jac, "atol_scale": 1.0e-4}


# ------------------------------------------------- Van der Pol, eps = 1e-6

VDPOL_EPS = 1.0e-6


def vdpol():
    eps = VDPOL_EPS

    def f(t, y):
        return np.array([y[1], ((1.0 - y[0] * y[0]) * y[1] - y[0]) / eps])

    def jac(t, y):
        return np.array([
            [0.0, 1.0],
            [(-2.0 * y[0] * y[1] - 1.0) / eps, (1.0 - y[0] * y[0]) / eps],
        ])

    return {"id": "vdpol-eps1e-6", "n": 2, "t_span": (0.0, 2.0), "y0": [2.0, 0.0],
            "f": f, "jac": jac, "atol_scale": 1.0}


# ---------------------------------------------------------------- Oregonator


def oregonator():
    s, q, w = 77.27, 8.375e-6, 0.161

    def f(t, y):
        return np.array([
            s * (y[1] + y[0] * (1.0 - q * y[0] - y[1])),
            (y[2] - (1.0 + y[0]) * y[1]) / s,
            w * (y[0] - y[2]),
        ])

    def jac(t, y):
        return np.array([
            [s * (1.0 - 2.0 * q * y[0] - y[1]), s * (1.0 - y[0]), 0.0],
            [-y[1] / s, -(1.0 + y[0]) / s, 1.0 / s],
            [w, 0.0, -w],
        ])

    return {"id": "oregonator", "n": 3, "t_span": (0.0, 360.0), "y0": [1.0, 2.0, 3.0],
            "f": f, "jac": jac, "atol_scale": 1.0}


# --------------------------------------------------------------------- HIRES


def hires():
    def f(t, y):
        return np.array([
            -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007,
            1.71 * y[0] - 8.75 * y[1],
            -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4],
            8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
            -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
            -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6],
            280.0 * y[5] * y[7] - 1.81 * y[6],
            -280.0 * y[5] * y[7] + 1.81 * y[6],
        ])

    def jac(t, y):
        m = np.zeros((8, 8))
        m[0, 0], m[0, 1], m[0, 2] = -1.71, 0.43, 8.32
        m[1, 0], m[1, 1] = 1.71, -8.75
        m[2, 2], m[2, 3], m[2, 4] = -10.03, 0.43, 0.035
        m[3, 1], m[3, 2], m[3, 3] = 8.32, 1.71, -1.12
        m[4, 4], m[4, 5], m[4, 6] = -1.745, 0.43, 0.43
        m[5, 3], m[5, 4], m[5, 5], m[5, 6], m[5, 7] = (
            0.69, 1.71, -280.0 * y[7] - 0.43, 0.69, -280.0 * y[5])
        m[6, 5], m[6, 6], m[6, 7] = 280.0 * y[7], -1.81, 280.0 * y[5]
        m[7, 5], m[7, 6], m[7, 7] = -280.0 * y[7], 1.81, -280.0 * y[5]
        return m

    return {"id": "hires", "n": 8, "t_span": (0.0, 321.8122),
            "y0": [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            "f": f, "jac": jac, "atol_scale": 1.0e-4}


# ----------------------------------------------------------------- Pollution

POLLUTION_K = [
    0.35, 26.6, 12300.0, 8.6e-4, 8.2e-4, 15000.0, 1.3e-4, 24000.0, 16500.0, 9000.0, 0.022,
    12000.0, 1.88, 16300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1240.0, 2.1, 5.78, 0.0474,
    1780.0, 3.12,
]
# Reactants of reaction k (0-based species), in the order of the rate product.
POLLUTION_REACTANTS = [
    (0,), (1, 3), (4, 1), (6,), (6,), (6, 5), (8,), (8, 5), (10, 1), (10, 0), (12,), (9, 1),
    (13,), (0, 5), (2,), (3,), (3,), (15,), (15,), (16, 5), (18,), (18,), (0, 3), (18, 0), (19,),
]
# Species i: the signed reactions of its right-hand side, in summation order.
POLLUTION_STOICH = [
    [(0, -1.0), (9, -1.0), (13, -1.0), (22, -1.0), (23, -1.0), (1, 1.0), (2, 1.0), (8, 1.0),
     (10, 1.0), (11, 1.0), (21, 1.0), (24, 1.0)],
    [(1, -1.0), (2, -1.0), (8, -1.0), (11, -1.0), (0, 1.0), (20, 1.0)],
    [(14, -1.0), (0, 1.0), (16, 1.0), (18, 1.0), (21, 1.0)],
    [(1, -1.0), (15, -1.0), (16, -1.0), (22, -1.0), (14, 1.0)],
    [(2, -1.0), (3, 2.0), (5, 1.0), (6, 1.0), (12, 1.0), (19, 1.0)],
    [(5, -1.0), (7, -1.0), (13, -1.0), (19, -1.0), (2, 1.0), (17, 2.0)],
    [(3, -1.0), (4, -1.0), (5, -1.0), (12, 1.0)],
    [(3, 1.0), (4, 1.0), (5, 1.0), (6, 1.0)],
    [(6, -1.0), (7, -1.0)],
    [(11, -1.0), (6, 1.0), (8, 1.0)],
    [(8, -1.0), (9, -1.0), (7, 1.0), (10, 1.0)],
    [(8, 1.0)],
    [(10, -1.0), (9, 1.0)],
    [(12, -1.0), (11, 1.0)],
    [(13, 1.0)],
    [(17, -1.0), (18, -1.0), (15, 1.0)],
    [(19, -1.0)],
    [(19, 1.0)],
    [(20, -1.0), (21, -1.0), (23, -1.0), (22, 1.0), (24, 1.0)],
    [(24, -1.0), (23, 1.0)],
]


def pollution():
    k = POLLUTION_K

    def rates(y):
        r = []
        for index, reactants in enumerate(POLLUTION_REACTANTS):
            value = k[index] * y[reactants[0]]
            if len(reactants) == 2:
                value = value * y[reactants[1]]
            r.append(value)
        return r

    def f(t, y):
        r = rates(y)
        out = np.zeros(20)
        for i, terms in enumerate(POLLUTION_STOICH):
            # The first term carries its sign; the others are added in order.
            reaction, coefficient = terms[0]
            total = coefficient * r[reaction]
            for reaction, coefficient in terms[1:]:
                total = total + coefficient * r[reaction]
            out[i] = total
        return out

    def jac(t, y):
        m = np.zeros((20, 20))
        for i, terms in enumerate(POLLUTION_STOICH):
            for reaction, coefficient in terms:
                reactants = POLLUTION_REACTANTS[reaction]
                if len(reactants) == 1:
                    m[i, reactants[0]] += coefficient * k[reaction]
                else:
                    a, b = reactants
                    m[i, a] += coefficient * k[reaction] * y[b]
                    m[i, b] += coefficient * k[reaction] * y[a]
        return m

    y0 = [0.0] * 20
    y0[1], y0[3], y0[6], y0[7], y0[8], y0[16] = 0.2, 0.04, 0.1, 0.3, 0.01, 0.007
    return {"id": "pollution", "n": 20, "t_span": (0.0, 60.0), "y0": y0,
            "f": f, "jac": jac, "atol_scale": 1.0e-6}


# ------------------------------------------------------------------------ E5


def e5():
    a, b, c, m = 7.89e-10, 1.1e7, 1.13e3, 1.0e6
    mc = m * c

    def f(t, y):
        f1 = -a * y[0] - b * y[0] * y[2]
        f2 = a * y[0] - mc * y[1] * y[2]
        f4 = b * y[0] * y[2] - c * y[3]
        return np.array([f1, f2, f2 - f4, f4])

    def jac(t, y):
        return np.array([
            [-a - b * y[2], 0.0, -b * y[0], 0.0],
            [a, -mc * y[2], -mc * y[1], 0.0],
            [a - b * y[2], -mc * y[2], -mc * y[1] - b * y[0], c],
            [b * y[2], 0.0, b * y[0], -c],
        ])

    return {"id": "e5", "n": 4, "t_span": (0.0, 1.0e13), "y0": [1.76e-3, 0.0, 0.0, 0.0],
            "f": f, "jac": jac, "atol_fixed": 1.7e-24}


# --------------------------------------------------------- 1D Brusselator


def brusselator(cells):
    c = (cells + 1.0) ** 2 / 50.0
    n = 2 * cells

    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.concatenate(([1.0], u[:-1]))
        ur = np.concatenate((u[1:], [1.0]))
        vl = np.concatenate(([3.0], v[:-1]))
        vr = np.concatenate((v[1:], [3.0]))
        out = np.empty(n)
        out[0::2] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur)
        out[1::2] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr)
        return out

    def jac(t, y):
        m = np.zeros((n, n))
        for i in range(cells):
            u, v = y[2 * i], y[2 * i + 1]
            a, b = 2 * i, 2 * i + 1
            m[a, a] = 2.0 * u * v - 4.0 - 2.0 * c
            m[a, b] = u * u
            m[b, a] = 3.0 - 2.0 * u * v
            m[b, b] = -u * u - 2.0 * c
            if i > 0:
                m[a, a - 2] = c
                m[b, b - 2] = c
            if i + 1 < cells:
                m[a, a + 2] = c
                m[b, b + 2] = c
        return m

    y0 = []
    for i in range(cells):
        x = (i + 1.0) / (cells + 1.0)
        y0.extend([1.0 + math.sin(2.0 * math.pi * x), 3.0])
    return {"id": "brusselator-1d-%d" % cells, "n": n, "t_span": (0.0, 10.0), "y0": y0,
            "f": f, "jac": jac, "atol_scale": 1.0}


# ------------------------------------- nonlinear Prothero-Robinson type

PR_L = 1.0e6


def pr_phi(t):
    """phi1, phi1', phi1'', phi2, phi2', phi2'' at t."""
    return (1.0 + 0.5 * math.sin(3.0 * t), 1.5 * math.cos(3.0 * t), -4.5 * math.sin(3.0 * t),
            math.cos(2.0 * t), -2.0 * math.sin(2.0 * t), -4.0 * math.cos(2.0 * t))


def pr_nonlinear(t_end):
    stiff = PR_L

    def f(t, y):
        p1, dp1, _, p2, dp2, _ = pr_phi(t)
        d1 = y[0] - p1
        return np.array([
            dp1 - stiff * (1.0 + y[1] * y[1]) * d1,
            dp2 - (y[1] - p2) - d1 * y[1],
        ])

    def jac(t, y):
        p1 = pr_phi(t)[0]
        d1 = y[0] - p1
        return np.array([
            [-stiff * (1.0 + y[1] * y[1]), -2.0 * stiff * y[1] * d1],
            [-y[1], -1.0 - d1],
        ])

    def exact(t):
        import mpmath as mp
        t = mp.mpf(t)
        return [1 + mp.sin(3 * t) / 2, mp.cos(2 * t)]

    return {"id": "pr-nonlinear", "n": 2, "t_span": (0.0, t_end), "y0": [1.0, 1.0],
            "f": f, "jac": jac, "atol_scale": 1.0, "exact": exact}


# ------------------------------------------------------ combustion (MOL)

COMBUSTION = {"alpha": 1.0, "delta": 20.0, "r": 5.0}


def combustion(cells, t_end):
    alpha, delta, r = COMBUSTION["alpha"], COMBUSTION["delta"], COMBUSTION["r"]
    d = r * math.exp(delta) / (alpha * delta)
    inv = float(cells) * float(cells)  # 1 / dx^2 with dx = 1 / cells

    def reaction(u):
        return d * (1.0 + alpha - u) * np.exp(-delta / u)

    def f(t, y):
        left = np.concatenate(([y[1]], y[:-1]))      # u_{-1} = u_1 (zero flux at x = 0)
        right = np.concatenate((y[1:], [1.0]))       # u_N = 1
        return inv * (left - 2.0 * y + right) + reaction(y)

    def jac(t, y):
        m = np.zeros((cells, cells))
        for i in range(cells):
            e = math.exp(-delta / y[i])
            m[i, i] = -2.0 * inv + d * e * ((1.0 + alpha - y[i]) * delta / (y[i] * y[i]) - 1.0)
            if i > 0:
                m[i, i - 1] = inv
            if i + 1 < cells:
                m[i, i + 1] = inv
        m[0, 1] = 2.0 * inv
        return m

    return {"id": "combustion-%d" % cells, "n": cells, "t_span": (0.0, t_end),
            "y0": [1.0] * cells, "f": f, "jac": jac, "atol_scale": 1.0}


# ------------------------------------ quasilinear diffusion-reaction (MOL)

NLDIFF = {"rate": 5.0, "u_left": 1.0, "u_init": 0.1}


def nldiff(cells, t_end):
    rate, u_left, u_init = NLDIFF["rate"], NLDIFF["u_left"], NLDIFF["u_init"]
    inv = float(cells) * float(cells)  # 1 / dx^2, nodes x_i = i dx, i = 1..cells, dx = 1/cells

    def f(t, y):
        left = np.concatenate(([u_left], y[:-1]))    # u_0 = u_left
        right = np.concatenate((y[1:], [y[-1]]))     # u_{N+1} = u_N (zero flux)
        wm = 0.5 * (left + y)
        wp = 0.5 * (y + right)
        flux = wp * wp * (right - y) - wm * wm * (y - left)
        return inv * flux + rate * y * (1.0 - y)

    def jac(t, y):
        m = np.zeros((cells, cells))
        for i in range(cells):
            ul = u_left if i == 0 else y[i - 1]
            ur = y[i] if i + 1 == cells else y[i + 1]
            wm = 0.5 * (ul + y[i])
            wp = 0.5 * (y[i] + ur)
            dm = y[i] - ul
            dp = ur - y[i]
            # d/du_i of wp^2 dp - wm^2 dm, with wp = (u_i + u_{i+1})/2 etc.
            diag = (wp * dp - wp * wp) - (wm * dm + wm * wm)
            lower = -(wm * dm - wm * wm)
            upper = wp * dp + wp * wp
            if i + 1 == cells:
                # u_{N+1} = u_N: dp = 0 identically and wp = u_N; the flux term vanishes.
                diag = -(wm * dm + wm * wm)
                upper = 0.0
            m[i, i] = inv * diag + rate * (1.0 - 2.0 * y[i])
            if i > 0:
                m[i, i - 1] = inv * lower
            if i + 1 < cells:
                m[i, i + 1] = inv * upper
        return m

    return {"id": "nldiff-%d" % cells, "n": cells, "t_span": (0.0, t_end),
            "y0": [u_init] * cells, "f": f, "jac": jac, "atol_scale": 1.0}


# ------------------------------------------------------- order problems


def kaps(eps, ident):
    def f(t, y):
        return np.array([
            -(2.0 + 1.0 / eps) * y[0] + y[1] * y[1] / eps,
            y[0] - y[1] * (1.0 + y[1]),
        ])

    def jac(t, y):
        return np.array([
            [-(2.0 + 1.0 / eps), 2.0 * y[1] / eps],
            [1.0, -1.0 - 2.0 * y[1]],
        ])

    def exact(t):
        import mpmath as mp
        t = mp.mpf(t)
        return [mp.exp(-2 * t), mp.exp(-t)]

    return {"id": ident, "n": 2, "t_span": (0.0, 1.5), "y0": [1.0, 1.0],
            "f": f, "jac": jac, "exact": exact}


def pr_linear():
    lam = -PR_L

    def f(t, y):
        p1, dp1 = pr_phi(t)[0], pr_phi(t)[1]
        return np.array([lam * (y[0] - p1) + dp1])

    def jac(t, y):
        return np.array([[lam]])

    def exact(t):
        import mpmath as mp
        t = mp.mpf(t)
        return [1 + mp.sin(3 * t) / 2]

    return {"id": "pr-linear", "n": 1, "t_span": (0.0, 1.5), "y0": [1.0],
            "f": f, "jac": jac, "exact": exact}


LOGISTIC = {"r": 4.0, "s": 2.0, "y1_0": 0.1}


def logistic_pair():
    r, s = LOGISTIC["r"], LOGISTIC["s"]

    def f(t, y):
        return np.array([r * y[0] * (1.0 - y[0]), -s * y[0] * y[1]])

    def jac(t, y):
        return np.array([[r * (1.0 - 2.0 * y[0]), 0.0], [-s * y[1], -s * y[0]]])

    def exact(t):
        import mpmath as mp
        t = mp.mpf(t)
        # y1 = 1 / (1 + 9 exp(-r t)), y2 = ((exp(r t) + 9) / 10)^(-s / r)
        return [1 / (1 + 9 * mp.exp(-4 * t)), ((mp.exp(4 * t) + 9) / 10) ** (-mp.mpf(1) / 2)]

    return {"id": "logistic-pair", "n": 2, "t_span": (0.0, 1.5), "y0": [0.1, 1.0],
            "f": f, "jac": jac, "exact": exact}


def adaptive_models(combustion_t_end, nldiff_t_end):
    """The adaptive stiff set, in the order of PREDECLARED.json."""
    return [
        robertson(), vdpol(), oregonator(), hires(), pollution(), e5(),
        brusselator(32), brusselator(100), pr_nonlinear(10.0),
        combustion(100, combustion_t_end), nldiff(100, nldiff_t_end),
    ]


def order_models():
    """The fixed-step order set (closed-form solutions), in the order of PREDECLARED.json."""
    return [kaps(1.0, "kaps-eps1"), logistic_pair(), kaps(1.0e-6, "kaps-eps1e-6"), pr_linear(),
            pr_nonlinear(1.5)]
