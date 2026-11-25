#!/usr/bin/env python3
"""Loop L1 node V16 -- prototype Rosenbrock-Wanner integrator, one generic stepper for every tableau.

Code form (the form of the protected driver, rodas5p_fast.rs: W = I/(h gamma) - J):
    U = Gamma k,  A = alpha Gamma^-1,  C = I/gamma - Gamma^-1,  m = Gamma^-T b,  e = Gamma^-T (b - bhat)
    W U_i = f(y + sum_{j<i} A_ij U_j) + sum_{j<i} (C_ij / h) U_j,   y_new = y + sum_i m_i U_i,   err = sum_i e_i U_i
Problems are autonomous (time appended as a state where needed); the Jacobian is exact, evaluated at y_n.
Stages are executed in index order, which is level order for the level-structured tableaux (sequential
execution of the levels). One Jacobian, one LU factorisation and s solves per attempt.

Controller: PREDECLARED.json, key prototype_integrator.controller (adaptive.rs:397, :273-277, :91-97;
sequential.rs:841-843; norms.rs:75).
"""
import json
import math

import numpy as np
from scipy.linalg import lu_factor, lu_solve, solve_triangular

SAFETY, MIN_FACTOR, MAX_FACTOR, REJECT_MAX_FACTOR, ESTIMATOR_ORDER = 0.9, 0.2, 5.0, 0.9, 5


class Tableau:
    def __init__(self, name, gamma, A, C, m, e, levels):
        self.name = name
        self.gamma = float(gamma)
        self.A = np.array(A, dtype=float)
        self.C = np.array(C, dtype=float)
        self.m = np.array(m, dtype=float)
        self.e = np.array(e, dtype=float)
        self.s = len(self.m)
        self.levels = levels


def from_kform(name, gamma, alpha, gam, b, bhat, levels):
    alpha = np.array(alpha, dtype=float)
    gam = np.array(gam, dtype=float)
    s = len(b)
    ginv = solve_triangular(gam, np.eye(s), lower=True)
    A = np.tril(alpha @ ginv, -1)
    C = np.tril(np.eye(s) / gamma - ginv, -1)
    m = ginv.T @ np.array(b, dtype=float)
    e = ginv.T @ (np.array(b, dtype=float) - np.array(bhat, dtype=float))
    return Tableau(name, gamma, A, C, m, e, levels)


def rodas5p(snapshot_path):
    """The protected coefficients in their own code form; error vector = U_8 (coefficients.rs:195)."""
    raw = json.load(open(snapshot_path))
    A = [[float(x) for x in r] for r in raw["A"]]
    C = [[float(x) for x in r] for r in raw["C"]]
    m = [float(x) for x in raw["b_code"]]
    e = [0.0] * 7 + [1.0]
    return Tableau("RODAS5P", float(raw["gamma"]), A, C, m, e, [[i] for i in range(1, 9)])


def from_file(path):
    d = json.load(open(path))
    return from_kform(d["id"], float(d["gamma"]), [[float(x) for x in r] for r in d["alpha"]],
                      [[float(x) for x in r] for r in d["Gamma"]], [float(x) for x in d["b"]],
                      [float(x) for x in d["bhat"]], d["levels"])


def step(tab, f, jac, y, h):
    n = y.size
    W = np.eye(n) / (h * tab.gamma) - jac(y)
    lu = lu_factor(W, check_finite=False)
    U = np.zeros((tab.s, n))
    for i in range(tab.s):
        if i == 0:
            rhs = f(y)
        else:
            rhs = f(y + tab.A[i, :i] @ U[:i]) + (tab.C[i, :i] @ U[:i]) / h
        U[i] = lu_solve(lu, rhs, check_finite=False)
    return y + tab.m @ U, tab.e @ U


def fixed(tab, prob, nsteps):
    y = np.array(prob.y0, dtype=float)
    h = prob.t_end / nsteps
    with np.errstate(all="ignore"):
        for _ in range(nsteps):
            y, _ = step(tab, prob.f, prob.jac, y, h)
    return y


def adaptive(tab, prob, rtol, atol, h0=1e-6, max_attempts=500000):
    t = 0.0
    t_end = float(prob.t_end)
    y = np.array(prob.y0, dtype=float)
    h = h0
    acc = rej = 0
    with np.errstate(all="ignore"):
        while t < t_end:
            if acc + rej >= max_attempts:
                return {"finished": False, "reason": "max_attempts", "accepted": acc, "rejected": rej, "t": t}
            last = h >= t_end - t
            h_try = (t_end - t) if last else h
            if not (h_try > 0.0) or t + h_try == t:
                return {"finished": False, "reason": "step_underflow", "accepted": acc, "rejected": rej, "t": t}
            ynew, ev = step(tab, prob.f, prob.jac, y, h_try)
            if np.all(np.isfinite(ynew)) and np.all(np.isfinite(ev)):
                scale = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
                err = math.sqrt(float(np.mean((ev / scale) ** 2)))
            else:
                err = float("inf")
            if math.isfinite(err) and err <= 1.0:
                acc += 1
                t = t_end if last else t + h_try
                y = ynew
                fac = MAX_FACTOR if err == 0.0 else min(MAX_FACTOR, max(MIN_FACTOR, SAFETY * err ** (-1.0 / ESTIMATOR_ORDER)))
            else:
                rej += 1
                if math.isfinite(err):
                    fac = min(REJECT_MAX_FACTOR, max(MIN_FACTOR, SAFETY * max(err, 1e-16) ** (-1.0 / ESTIMATOR_ORDER)))
                else:
                    fac = MIN_FACTOR
            h = h_try * fac
    return {"finished": True, "accepted": acc, "rejected": rej, "y": y}
