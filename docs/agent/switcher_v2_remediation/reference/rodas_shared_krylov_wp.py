"""Prototype: RODAS5P (repository tableau, transformed form) with matrix-free stage solves.

Two linear-solver modes with the *same* minimal-residual algorithm and stopping rule:
  per_stage : a fresh Krylov basis for every stage (zero start, GMRES(inf)-equivalent)
  shared    : one basis per step, shared by all 8 stages (single shifted operator W)
plus 'direct' (dense LU) as a reference.

Counts: JVP applications (matvecs), RHS calls, accepted/rejected steps, global error.
This is a research prototype for evidence only; it is not the repository's driver.
"""
import json
import sys
import numpy as np
from scipy.integrate import solve_ivp

TAB = json.load(open(sys.argv[1] if len(sys.argv) > 1 else "rodas5p_coefficients_snapshot.json"))
A = np.array(TAB["A"], float)
C = np.array(TAB["C"], float)
B = np.array(TAB["b_code"], float)
G = float(TAB["gamma"])
S = 8


class Counter:
    def __init__(self):
        self.jvp = 0
        self.rhs = 0


class MinRes:
    """Minimal residual over a growing basis Q with W Q kept as a thin QR (P, R)."""

    def __init__(self, n, jvp, inv_hg, kmax):
        self.n, self.jvp, self.inv_hg, self.kmax = n, jvp, inv_hg, kmax
        self.Q = np.zeros((n, kmax))
        self.P = np.zeros((n, kmax))
        self.R = np.zeros((kmax, kmax))
        self.k = 0

    def _add(self, v):
        # orthonormalise the new domain vector against Q (two MGS passes)
        for _ in range(2):
            v = v - self.Q[:, : self.k] @ (self.Q[:, : self.k].T @ v)
        nv = np.linalg.norm(v)
        if nv == 0.0 or self.k >= self.kmax:
            return False
        q = v / nv
        w = self.inv_hg * q - self.jvp(q)  # one JVP per basis vector
        col = np.zeros(self.k + 1)
        for _ in range(2):
            proj = self.P[:, : self.k].T @ w
            w = w - self.P[:, : self.k] @ proj
            col[: self.k] += proj
        col[self.k] = np.linalg.norm(w)
        if col[self.k] == 0.0:
            return False
        self.Q[:, self.k] = q
        self.P[:, self.k] = w / col[self.k]
        self.R[: self.k + 1, self.k] = col
        self.k += 1
        return True

    def solve(self, b, eta, sc=None, hg=None):
        bn = np.linalg.norm(b)
        if bn == 0.0:
            return np.zeros_like(b)
        def small(r):
            if sc is None:
                return np.linalg.norm(r) <= eta * bn
            return hg * np.sqrt(np.mean((r / sc) ** 2)) <= eta
        while True:
            if self.k:
                y = np.linalg.solve(self.R[: self.k, : self.k], self.P[:, : self.k].T @ b)
                x = self.Q[:, : self.k] @ y
                r = b - self.P[:, : self.k] @ (self.P[:, : self.k].T @ b)
            else:
                x, r = np.zeros_like(b), b.copy()
            if small(r) or not self._add(r):
                return x


def rodas_solve(f, jac_vec, jac_dense, y0, T, rtol, atol, mode, eta, kmax=400, h0=1e-4, budget=False):
    cnt = Counter()
    n = y0.size
    t, y, h = 0.0, y0.copy(), h0
    acc = rej = 0
    while t < T:
        h = min(h, T - t)
        inv_hg = 1.0 / (h * G)
        jv = lambda v, yy=y: (cnt.__setattr__("jvp", cnt.jvp + 1), jac_vec(yy, v))[1]
        if mode == "direct":
            W = inv_hg * np.eye(n) - jac_dense(y)
            lu = np.linalg.inv(W)
            solver = None
        elif mode == "shared":
            solver = MinRes(n, jv, inv_hg, kmax)
        U = np.zeros((S, n))
        scy = atol + rtol * np.abs(y)
        for i in range(S):
            Y = y + A[i, :i] @ U[:i]
            cnt.rhs += 1
            rhs = f(Y) + (C[i, :i] @ U[:i]) / h
            if mode == "direct":
                U[i] = lu @ rhs
            elif mode == "shared":
                U[i] = solver.solve(rhs, eta, scy if budget else None, h * G)
            else:  # per_stage
                U[i] = MinRes(n, jv, inv_hg, kmax).solve(rhs, eta, scy if budget else None, h * G)
        ynew = y + B @ U
        sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = np.sqrt(np.mean((U[-1] / sc) ** 2))
        if err <= 1.0:
            t, y = t + h, ynew
            acc += 1
        else:
            rej += 1
        h = h * min(5.0, max(0.2, 0.9 * max(err, 1e-10) ** (-1.0 / 5.0)))
    return y, dict(accepted=acc, rejected=rej, jvp=cnt.jvp, rhs=cnt.rhs)


# ---------------- problems -----------------
def heat(N=128, nu=1.0):
    dx = 1.0 / (N + 1)
    x = np.linspace(dx, 1 - dx, N)
    L = (np.diag(-2 * np.ones(N)) + np.diag(np.ones(N - 1), 1) + np.diag(np.ones(N - 1), -1)) * nu / dx**2
    f = lambda y: L @ y
    return f, (lambda y, v: L @ v), (lambda y: L), np.sin(np.pi * x) + 0.5 * np.sin(7 * np.pi * x), 0.05


def allen_cahn(N=128, eps=0.02):
    dx = 1.0 / (N + 1)
    x = np.linspace(dx, 1 - dx, N)
    L = (np.diag(-2 * np.ones(N)) + np.diag(np.ones(N - 1), 1) + np.diag(np.ones(N - 1), -1)) * eps / dx**2
    f = lambda y: L @ y + y - y**3
    jv = lambda y, v: L @ v + (1 - 3 * y**2) * v
    jd = lambda y: L + np.diag(1 - 3 * y**2)
    return f, jv, jd, 0.8 * np.sin(3 * np.pi * x) + 0.1 * np.cos(11 * np.pi * x), 2.0


def brusselator(M=64, alpha=0.1):
    # interleaved (u_1, v_1, u_2, v_2, ...), Dirichlet u=1, v=3 at both ends
    dx = 1.0 / (M + 1)
    x = np.linspace(dx, 1 - dx, M)
    c = alpha / dx**2

    def f(y):
        u, v = y[0::2], y[1::2]
        up = np.concatenate(([1.0], u, [1.0]))
        vp = np.concatenate(([3.0], v, [3.0]))
        du = 1 + u * u * v - 4 * u + c * (up[:-2] - 2 * u + up[2:])
        dv = 3 * u - u * u * v + c * (vp[:-2] - 2 * v + vp[2:])
        out = np.empty_like(y)
        out[0::2], out[1::2] = du, dv
        return out

    def jv(y, w):
        u, v = y[0::2], y[1::2]
        a, b = w[0::2], w[1::2]
        ap = np.concatenate(([0.0], a, [0.0]))
        bp = np.concatenate(([0.0], b, [0.0]))
        du = (2 * u * v - 4) * a + u * u * b + c * (ap[:-2] - 2 * a + ap[2:])
        dv = (3 - 2 * u * v) * a - u * u * b + c * (bp[:-2] - 2 * b + bp[2:])
        out = np.empty_like(w)
        out[0::2], out[1::2] = du, dv
        return out

    def jd(y):
        n = y.size
        return np.column_stack([jv(y, e) for e in np.eye(n)])

    y0 = np.empty(2 * M)
    y0[0::2] = 1 + np.sin(2 * np.pi * x)
    y0[1::2] = 3.0
    return f, jv, jd, y0, 5.0


PROBLEMS = {"heat": heat, "allen_cahn": allen_cahn, "brusselator": brusselator}

if __name__ == "__main__":
    out = []
    theta = 1e-3
    for name, mk in PROBLEMS.items():
        f, jv, jd, y0, T = mk()
        ref = solve_ivp(lambda t, y: f(y), (0, T), y0, method="Radau", rtol=1e-12, atol=1e-13,
                        jac=lambda t, y: jd(y)).y[:, -1]
        for rtol in (1e-4, 1e-5, 1e-6, 1e-7, 1e-8):
            atol = rtol * 1e-2
            row = {"problem": name, "n": int(y0.size), "rtol": rtol, "theta": theta}
            for mode in ("direct", "per_stage", "shared"):
                y, st = rodas_solve(f, jv, jd, y0, T, rtol, atol, mode, theta, budget=True)
                st["err"] = float(np.max(np.abs(y - ref)) / max(1.0, np.max(np.abs(ref))))
                row[mode] = st
            out.append(row)
            print(json.dumps(row), flush=True)
    json.dump(out, open("rodas_wp_results.json", "w"), indent=1)
