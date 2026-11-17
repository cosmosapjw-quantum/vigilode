#!/usr/bin/env python3
"""Loop L1 node V16 -- test problems of PREDECLARED.json in autonomous form with exact Jacobians."""
import math

import numpy as np


class Problem:
    def __init__(self, pid, y0, t_end, f, jac, a_scale=1.0, nstate=None, mp_rhs=None, note=""):
        self.id = pid
        self.y0 = np.array(y0, dtype=float)
        self.t_end = float(t_end)
        self.f = f
        self.jac = jac
        self.a_scale = a_scale
        self.nstate = nstate if nstate is not None else len(y0)     # components that count in the error
        self.mp_rhs = mp_rhs                                        # (mp module) -> F(t, y) for mpmath.odefun
        self.note = note


# ------------------------------------------------------------------------------------ order check (smooth)
def bruss_ode():
    def f(y):
        return np.array([1.0 + y[0] * y[0] * y[1] - 4.0 * y[0], 3.0 * y[0] - y[0] * y[0] * y[1]])

    def jac(y):
        return np.array([[2.0 * y[0] * y[1] - 4.0, y[0] * y[0]], [3.0 - 2.0 * y[0] * y[1], -y[0] * y[0]]])

    def mp_rhs(mp):
        return lambda t, y: [1 + y[0] * y[0] * y[1] - 4 * y[0], 3 * y[0] - y[0] * y[0] * y[1]]

    return Problem("BRUSS_ODE", [1.5, 3.0], 2.0, f, jac, mp_rhs=mp_rhs)


def vdp_mu2():
    def f(y):
        return np.array([y[1], 2.0 * (1.0 - y[0] * y[0]) * y[1] - y[0]])

    def jac(y):
        return np.array([[0.0, 1.0], [-4.0 * y[0] * y[1] - 1.0, 2.0 * (1.0 - y[0] * y[0])]])

    def mp_rhs(mp):
        return lambda t, y: [y[1], 2 * (1 - y[0] * y[0]) * y[1] - y[0]]

    return Problem("VDP_MU2", [2.0, 0.0], 2.0, f, jac, mp_rhs=mp_rhs)


def pendulum_forced():
    def f(y):
        return np.array([y[1], -math.sin(y[0]) + 0.5 * math.cos(2.0 * y[2]), 1.0])

    def jac(y):
        return np.array([[0.0, 1.0, 0.0], [-math.cos(y[0]), 0.0, -math.sin(2.0 * y[2])], [0.0, 0.0, 0.0]])

    def mp_rhs(mp):
        return lambda t, y: [y[1], -mp.sin(y[0]) + mp.cos(2 * t) / 2]

    return Problem("PENDULUM_FORCED", [1.0, 0.0, 0.0], 2.0, f, jac, nstate=2, mp_rhs=mp_rhs)


# ------------------------------------------------------------------------------------- pay-off screen (stiff)
def rober():
    def f(y):
        return np.array([-0.04 * y[0] + 1e4 * y[1] * y[2],
                         0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1],
                         3e7 * y[1] * y[1]])

    def jac(y):
        return np.array([[-0.04, 1e4 * y[2], 1e4 * y[1]],
                         [0.04, -1e4 * y[2] - 6e7 * y[1], -1e4 * y[1]],
                         [0.0, 6e7 * y[1], 0.0]])

    return Problem("ROBER", [1.0, 0.0, 0.0], 1e5, f, jac, a_scale=1e-6)


def vdpol():
    eps = 1e-6

    def f(y):
        return np.array([y[1], ((1.0 - y[0] * y[0]) * y[1] - y[0]) / eps])

    def jac(y):
        return np.array([[0.0, 1.0], [(-2.0 * y[0] * y[1] - 1.0) / eps, (1.0 - y[0] * y[0]) / eps]])

    return Problem("VDPOL", [2.0, -0.66], 2.0, f, jac, a_scale=1.0)


def burgers(n=100, nu=0.05):
    dx = 1.0 / (n + 1)
    x = np.linspace(dx, 1.0 - dx, n)
    y0 = np.sin(2.0 * math.pi * x) + 0.5 * np.sin(math.pi * x)
    idx = np.arange(n)

    def f(u):
        up = np.concatenate((u[1:], [0.0]))
        um = np.concatenate(([0.0], u[:-1]))
        return nu * (up - 2.0 * u + um) / (dx * dx) - u * (up - um) / (2.0 * dx)

    def jac(u):
        up = np.concatenate((u[1:], [0.0]))
        um = np.concatenate(([0.0], u[:-1]))
        J = np.zeros((n, n))
        J[idx, idx] = -2.0 * nu / (dx * dx) - (up - um) / (2.0 * dx)
        J[idx[:-1], idx[:-1] + 1] = nu / (dx * dx) - u[:-1] / (2.0 * dx)
        J[idx[1:], idx[1:] - 1] = nu / (dx * dx) + u[1:] / (2.0 * dx)
        return J

    return Problem("BURGERS", y0, 1.0, f, jac, a_scale=1.0)


# ------------------------------------------------------------------------- development (consumed U11 configs)
def hires():
    def f(y):
        return np.array([
            -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007,
            1.71 * y[0] - 8.75 * y[1],
            -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4],
            8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
            -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
            -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6],
            280.0 * y[5] * y[7] - 1.81 * y[6],
            -280.0 * y[5] * y[7] + 1.81 * y[6]])

    def jac(y):
        J = np.zeros((8, 8))
        J[0, 0], J[0, 1], J[0, 2] = -1.71, 0.43, 8.32
        J[1, 0], J[1, 1] = 1.71, -8.75
        J[2, 2], J[2, 3], J[2, 4] = -10.03, 0.43, 0.035
        J[3, 1], J[3, 2], J[3, 3] = 8.32, 1.71, -1.12
        J[4, 4], J[4, 5], J[4, 6] = -1.745, 0.43, 0.43
        J[5, 3], J[5, 4], J[5, 5], J[5, 6], J[5, 7] = 0.69, 1.71, -280.0 * y[7] - 0.43, 0.69, -280.0 * y[5]
        J[6, 5], J[6, 6], J[6, 7] = 280.0 * y[7], -1.81, 280.0 * y[5]
        J[7, 5], J[7, 6], J[7, 7] = -280.0 * y[7], 1.81, -280.0 * y[5]
        return J

    return Problem("HIRES", [1.0, 0, 0, 0, 0, 0, 0, 0.0057], 321.8122, f, jac, a_scale=1e-4,
                   note="consumed U11 configuration, development use only")


def orego():
    def f(y):
        return np.array([77.27 * (y[1] + y[0] * (1.0 - 8.375e-6 * y[0] - y[1])),
                         (y[2] - (1.0 + y[0]) * y[1]) / 77.27,
                         0.161 * (y[0] - y[2])])

    def jac(y):
        return np.array([[77.27 * (1.0 - 2.0 * 8.375e-6 * y[0] - y[1]), 77.27 * (1.0 - y[0]), 0.0],
                         [-y[1] / 77.27, -(1.0 + y[0]) / 77.27, 1.0 / 77.27],
                         [0.161, 0.0, -0.161]])

    return Problem("OREGO", [1.0, 2.0, 3.0], 360.0, f, jac, a_scale=1e-6,
                   note="consumed U11 configuration, development use only")


# ------------------------------------------------------------------------------- exploratory: Prothero-Robinson
def prothero_robinson(lam=-1e6):
    def f(y):
        return np.array([lam * (y[0] - (math.sin(y[1]) + 2.0)) + math.cos(y[1]), 1.0])

    def jac(y):
        return np.array([[lam, -lam * math.cos(y[1]) - math.sin(y[1])], [0.0, 0.0]])

    p = Problem("PROTHERO_ROBINSON", [2.0, 0.0], 1.0, f, jac, nstate=1)
    p.exact = lambda t: math.sin(t) + 2.0
    return p


ORDER_PROBLEMS = {"BRUSS_ODE": bruss_ode, "VDP_MU2": vdp_mu2, "PENDULUM_FORCED": pendulum_forced}
STIFF_PROBLEMS = {"ROBER": rober, "VDPOL": vdpol, "BURGERS": burgers, "HIRES": hires, "OREGO": orego}
