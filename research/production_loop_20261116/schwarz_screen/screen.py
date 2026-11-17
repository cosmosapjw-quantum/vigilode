#!/usr/bin/env python3
"""V14 screen: restricted additive Schwarz on the full eight-stage RODAS5P system.

Loop L1 node V14 (2026-11-16).  Python only (numpy / scipy), single process.

Model (U-form, as crates/rodas5p-integrators/src/rodas5p_fast.rs:1160-1202):
    W = I/(h*gamma) - J
    stage i:  W U_i = f(t + c_i h, y + sum_j A_ij U_j) + sum_j (C_ij/h) U_j
                      + gamma_i h f_t                    (non-autonomous term)
    y_new = y + sum_j b_code_j U_j,   embedded error = U_8
The tableau is read from fixtures/rodas5p_coefficients_snapshot.json; gamma_i
are the row sums of inverse(I/gamma - C), as crates/rodas5p-core derives them.

Every number in RESULTS.json is computed by this script in the run that wrote
the file.  Wall time is diagnostic only.
"""
import os

for _v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(_v, "1")

import argparse
import hashlib
import json
import math
import platform
import sys
import time

import numpy as np
import scipy
import scipy.sparse as sp
from scipy.sparse.linalg import splu

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_TABLEAU = os.path.normpath(
    os.path.join(HERE, "..", "..", "..", "fixtures", "rodas5p_coefficients_snapshot.json")
)

# Frozen screen criteria (PREREGISTRATION.md row V14); never tuned here.
CONVERGED_TOL_UNITS = 0.1  # "sweeps needed": first sweep with endpoint WRMS <= 0.1
PASS_MEDIAN_SWEEPS = 2  # PASS needs median sweeps <= 2 (predictor g1)
PASS_MAX_OVERLAP_M = 4  # ... at overlap <= 4*ell
PASS_HALO_FRACTION = 1.0 / 8.0  # ... and median halo <= H/8
KILL_MEDIAN_SWEEPS = 4  # KILL: median sweeps > 4

S = 8  # stages
# SuperLU column ordering.  MMD on A^T + A halves the fill of COLAMD on these
# structurally symmetric stencil matrices (logs/probe3_splu_ordering.log).
PERMC = "MMD_AT_PLUS_A"


# --------------------------------------------------------------------------
# Tableau
# --------------------------------------------------------------------------
class Tableau:
    def __init__(self, path):
        raw = open(path, "rb").read()
        self.path = path
        self.sha256 = hashlib.sha256(raw).hexdigest()
        d = json.loads(raw)
        self.schema = d["schema_version"]
        self.gamma = float(d["gamma"])
        self.A = np.array([[float(x) for x in r] for r in d["A"]])
        self.C = np.array([[float(x) for x in r] for r in d["C"]])
        self.c = np.array([float(x) for x in d["c"]])
        self.b = np.array([float(x) for x in d["b_code"]])
        assert self.A.shape == (S, S) and self.C.shape == (S, S)
        assert np.all(np.triu(self.A) == 0.0) and np.all(np.triu(self.C) == 0.0)
        # Gamma = inverse(I/gamma - C); gamma_i = row sums (coefficients.rs:189-196).
        ginv = np.eye(S) / self.gamma - self.C
        self.Gamma = np.linalg.inv(ginv)
        self.gamma_rows = self.Gamma.sum(axis=1)
        # alpha = A_u Gamma; its row sums must reproduce the snapshot abscissae c.
        alpha = self.A @ self.Gamma
        self.c_from_alpha = alpha.sum(axis=1)


# --------------------------------------------------------------------------
# Problems: y' = L y + react(y) + source(t), blocked ordering comp*N^2 + cell,
# cell = ix + N*iy.
# --------------------------------------------------------------------------
def lap1d(N, periodic):
    e = np.ones(N)
    T = sp.diags([e[:-1], -2.0 * e, e[:-1]], [-1, 0, 1], format="lil")
    if periodic:
        T[0, N - 1] = 1.0
        T[N - 1, 0] = 1.0
    return T.tocsr()


def lap2d(N, periodic):
    T = lap1d(N, periodic)
    I = sp.identity(N, format="csr")
    return (sp.kron(I, T) + sp.kron(T, I)).tocsr()


class Problem:
    name = "?"
    ncomp = 1
    periodic = True
    has_source = False

    def _finish(self):
        self.ncell = self.N * self.N
        self.n = self.ncomp * self.ncell
        lap = lap2d(self.N, self.periodic) / (self.dx * self.dx)
        self.L = sp.block_diag([d * lap for d in self.D], format="csr")
        self.Dmax = max(self.D)

    # pointwise reaction on X of shape (ncomp, m)
    def react(self, X):
        return np.zeros_like(X)

    def react_jac(self, X):
        return None

    def source(self, t):
        return None

    def source_t(self, t):
        return None

    def react_jac_sparse(self, X):
        d = self.react_jac(X)
        m = X.shape[1]
        if d is None:
            return sp.csr_matrix((self.ncomp * m, self.ncomp * m))
        ar = np.arange(m)
        rows, cols, vals = [], [], []
        for a in range(self.ncomp):
            for b in range(self.ncomp):
                rows.append(a * m + ar)
                cols.append(b * m + ar)
                vals.append(d[a][b])
        return sp.csr_matrix(
            (np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))),
            shape=(self.ncomp * m, self.ncomp * m),
        )

    def f(self, t, y):
        out = self.L @ y + self.react(y.reshape(self.ncomp, -1)).ravel()
        s = self.source(t)
        if s is not None:
            out += s
        return out

    def jac(self, y):
        return (self.L + self.react_jac_sparse(y.reshape(self.ncomp, -1))).tocsr()

    def describe(self):
        return {}


class Heat(Problem):
    """P1: u_t = D lap(u) + s(x,y,t) on (0,1)^2, u = 0 on the boundary.

    s is a Gaussian of width w whose centre moves on a circle with period 1.
    """

    name = "P1_heat"
    ncomp = 1
    periodic = False
    has_source = True

    def __init__(self, N):
        self.N = N
        self.dx = 1.0 / (N + 1)
        self.D = [1.0]
        self.amp, self.w, self.rad, self.omega = 100.0, 0.1, 0.25, 2.0 * math.pi
        x = (np.arange(N) + 1) * self.dx
        self.X, self.Y = np.meshgrid(x, x, indexing="xy")
        self._finish()
        self.y0 = np.zeros(self.n)
        self.h0 = 1e-5
        self.t_transient = 0.25
        self.stride = 2

    def _centre(self, t):
        return (
            0.5 + self.rad * math.cos(self.omega * t),
            0.5 + self.rad * math.sin(self.omega * t),
        )

    def source(self, t):
        xc, yc = self._centre(t)
        r2 = (self.X - xc) ** 2 + (self.Y - yc) ** 2
        return (self.amp * np.exp(-r2 / (2 * self.w**2))).ravel()

    def source_t(self, t):
        xc, yc = self._centre(t)
        dxc = -self.rad * self.omega * math.sin(self.omega * t)
        dyc = self.rad * self.omega * math.cos(self.omega * t)
        r2 = (self.X - xc) ** 2 + (self.Y - yc) ** 2
        s = self.amp * np.exp(-r2 / (2 * self.w**2))
        return (s * ((self.X - xc) * dxc + (self.Y - yc) * dyc) / self.w**2).ravel()

    def describe(self):
        return {
            "equation": "u_t = D lap u + amp*exp(-|x-xc(t)|^2/(2 w^2)), Dirichlet 0, unit square",
            "D": self.D, "amp": self.amp, "w": self.w, "orbit_radius": self.rad,
            "orbit_period": 1.0, "dx": self.dx, "y0": "zero",
        }


class Brusselator(Problem):
    """P2: Hairer-Wanner 2D Brusselator (A=1, B=3.4, alpha=0.1, periodic unit square).

    u_t = 1 + u^2 v - 4.4 u + alpha lap u + f,  v_t = 3.4 u - u^2 v + alpha lap v,
    f = 5 on the disc (x-0.3)^2+(y-0.6)^2 <= 0.1^2 for t >= 1.1.
    """

    name = "P2_brusselator"
    ncomp = 2
    periodic = True
    has_source = True

    def __init__(self, N):
        self.N = N
        self.dx = 1.0 / N
        self.D = [0.1, 0.1]
        x = np.arange(N) * self.dx
        X, Y = np.meshgrid(x, x, indexing="xy")
        self._finish()
        u0 = 22.0 * Y * (1.0 - Y) ** 1.5
        v0 = 27.0 * X * (1.0 - X) ** 1.5
        self.y0 = np.concatenate([u0.ravel(), v0.ravel()])
        disc = ((X - 0.3) ** 2 + (Y - 0.6) ** 2 <= 0.1**2).ravel()
        self._src_on = np.concatenate([5.0 * disc, np.zeros(self.ncell)])
        self._src_off = np.zeros(self.n)
        self.t_on = 1.1
        self.h0 = 1e-4
        self.t_transient = 1.5
        self.stride = 2

    def react(self, X):
        u, v = X
        uuv = u * u * v
        return np.array([1.0 + uuv - 4.4 * u, 3.4 * u - uuv])

    def react_jac(self, X):
        u, v = X
        return [[2 * u * v - 4.4, u * u], [3.4 - 2 * u * v, -u * u]]

    def source(self, t):
        return self._src_on if t >= self.t_on else self._src_off

    def source_t(self, t):
        return self._src_off  # piecewise constant in time

    def describe(self):
        return {
            "equation": "Hairer-Wanner BRUSS-2D: A=1, B=3.4, alpha=0.1, periodic unit square, "
            "source 5 on a disc of radius 0.1 for t>=1.1",
            "D": self.D, "dx": self.dx, "y0": "u=22 y (1-y)^1.5, v=27 x (1-x)^1.5",
        }


class GrayScott(Problem):
    """P3: Gray-Scott on Pearson's lattice (dx = 2.5/256, Du=2e-5, Dv=1e-5), periodic.

    u_t = Du lap u - u v^2 + F (1-u),  v_t = Dv lap v + u v^2 - (F+k) v,
    F = 0.04, k = 0.06 (pattern-forming).  The periodic tile has N cells per side.
    """

    name = "P3_gray_scott"
    ncomp = 2
    periodic = True
    has_source = False

    def __init__(self, N):
        self.N = N
        self.dx = 2.5 / 256.0
        self.D = [2e-5, 1e-5]
        self.F, self.k = 0.04, 0.06
        self._finish()
        rng = np.random.default_rng(20261116)
        u = np.ones((N, N))
        v = np.zeros((N, N))
        lo, hi = N // 2 - 10, N // 2 + 10
        u[lo:hi, lo:hi] = 0.5
        v[lo:hi, lo:hi] = 0.25
        u *= 1.0 + 0.01 * (2 * rng.random((N, N)) - 1)
        v *= 1.0 + 0.01 * (2 * rng.random((N, N)) - 1)
        self.y0 = np.concatenate([u.ravel(), v.ravel()])
        self.h0 = 1e-2
        self.t_transient = 200.0
        self.stride = 3

    def react(self, X):
        u, v = X
        uvv = u * v * v
        return np.array([-uvv + self.F * (1.0 - u), uvv - (self.F + self.k) * v])

    def react_jac(self, X):
        u, v = X
        return [[-v * v - self.F, -2 * u * v], [v * v, 2 * u * v - (self.F + self.k)]]

    def describe(self):
        return {
            "equation": "Gray-Scott, Pearson lattice dx=2.5/256, periodic tile of N cells",
            "D": self.D, "F": self.F, "k": self.k, "dx": self.dx,
            "y0": "u=1, v=0; central 20x20 cells u=0.5, v=0.25; 1% multiplicative noise, "
            "numpy default_rng(20261116)",
        }


PROBLEMS = {"P1": Heat, "P2": Brusselator, "P3": GrayScott}


# --------------------------------------------------------------------------
# Exact global RODAS5P step
# --------------------------------------------------------------------------
def wrms(v, scale):
    return float(np.sqrt(np.mean((v / scale) ** 2)))


class Counters(dict):
    def bump(self, k, n=1):
        self[k] = self.get(k, 0) + n


def stage_sources(prob, tab, t, h):
    """Source at the eight stage times (n x 8) and f_t at t; None if autonomous."""
    if not prob.has_source:
        return None, None
    src = np.empty((prob.n, S))
    for i in range(S):
        src[:, i] = prob.source(t + tab.c[i] * h)
    return src, prob.source_t(t)


def exact_step(prob, tab, t, y, h, tol, ctr):
    J = prob.jac(y)
    ctr.bump("jacobian_builds")
    W = (sp.identity(prob.n, format="csr") / (h * tab.gamma) - J).tocsr()
    lu = splu(W.tocsc(), permc_spec=PERMC)
    ctr.bump("global_factorizations")
    ft = prob.source_t(t) if prob.has_source else None
    U = np.zeros((prob.n, S))
    for i in range(S):
        x = y + U[:, :i] @ tab.A[i, :i]
        rhs = prob.f(t + tab.c[i] * h, x)
        ctr.bump("rhs_evals")
        if i:
            rhs += U[:, :i] @ (tab.C[i, :i] / h)
        if ft is not None:
            rhs += (tab.gamma_rows[i] * h) * ft
        U[:, i] = lu.solve(rhs)
        ctr.bump("global_solves")
    y_new = y + U @ tab.b
    finite = bool(np.all(np.isfinite(y_new)))
    if finite:
        scale = tol + tol * np.maximum(np.abs(y), np.abs(y_new))
        err = wrms(U[:, S - 1], scale)
    else:
        scale, err = None, float("inf")
    return {"U": U, "y_new": y_new, "err": err, "scale": scale, "J": J, "W": W, "finite": finite}


def global_residuals(prob, tab, t, y, h, W, U, src, ft, scale):
    """WRMS of h*gamma*R_i with R_i the ORIGINAL global stage residual
    R_i = W U_i - f(t+c_i h, y + sum_j A_ij U_j) - sum_j (C_ij/h) U_j - gamma_i h f_t.
    Returns (max over stages, rms over stages)."""
    St = y[:, None] + U @ tab.A.T
    R = W @ U - (prob.L @ St) - U @ (tab.C.T / h)
    for i in range(S):
        R[:, i] -= prob.react(St[:, i].reshape(prob.ncomp, -1)).ravel()
    if src is not None:
        R -= src
        R -= ft[:, None] * (tab.gamma_rows * h)[None, :]
    R *= h * tab.gamma
    per = np.sqrt(np.mean((R / scale[:, None]) ** 2, axis=0))
    return float(per.max()), float(np.sqrt(np.mean(per**2)))


# --------------------------------------------------------------------------
# Subdomain geometry and the restricted additive Schwarz sweep
# --------------------------------------------------------------------------
def ext_range(a, b, delta, N, periodic):
    if periodic:
        if (b - a) + 2 * delta >= N:
            idx = np.arange(N)
        else:
            idx = np.arange(a - delta, b + delta) % N
    else:
        idx = np.arange(max(0, a - delta), min(N, b + delta))
    return idx, (idx >= a) & (idx < b)


class Sub:
    __slots__ = ("idxE", "own_pos", "own_glob", "LEE", "B", "cells", "nE")


def build_geometry(prob, P, delta):
    N, ncell, n = prob.N, prob.ncell, prob.n
    assert N % P == 0
    H = N // P
    subs = []
    seen = np.zeros(n, dtype=np.int64)
    ext_cells = 0
    for py in range(P):
        ey, oy = ext_range(py * H, (py + 1) * H, delta, N, prob.periodic)
        for px in range(P):
            ex, ox = ext_range(px * H, (px + 1) * H, delta, N, prob.periodic)
            cells = (ex[None, :] + N * ey[:, None]).ravel()
            own = (oy[:, None] & ox[None, :]).ravel()
            s = Sub()
            s.cells = cells
            s.idxE = np.concatenate([c * ncell + cells for c in range(prob.ncomp)])
            s.nE = s.idxE.size
            s.own_pos = np.flatnonzero(np.tile(own, prob.ncomp))
            s.own_glob = s.idxE[s.own_pos]
            seen[s.own_glob] += 1
            inE = np.zeros(n, dtype=bool)
            inE[s.idxE] = True
            Lrows = prob.L[s.idxE].tocsr()
            s.LEE = Lrows[:, s.idxE].tocsr()
            B = Lrows.copy()
            B.data[inE[B.indices]] = 0.0
            B.eliminate_zeros()
            s.B = B  # coupling of the block to everything outside it
            ext_cells += cells.size
            subs.append(s)
    assert np.all(seen == 1), "owned cells must partition the grid"
    return {"subs": subs, "H": H, "actual_work_factor": ext_cells / float(ncell)}


def factor_blocks(prob, tab, geom, y, h, ctr):
    lus = []
    for s in geom["subs"]:
        XE = y[s.idxE].reshape(prob.ncomp, -1)
        WEE = sp.identity(s.nE, format="csr") / (h * tab.gamma) - s.LEE - prob.react_jac_sparse(XE)
        lus.append(splu(WEE.tocsc(), permc_spec=PERMC))
        ctr.bump("local_factorizations")
    return lus


def schwarz_sweep(prob, tab, geom, lus, y, h, Uold, src, ft, ctr):
    """One Jacobi sweep: every subdomain runs the whole eight-stage chain on its
    extended block with stage values outside the block taken from Uold, then
    writes back its owned cells only."""
    # Outside values enter stage i through f (y + sum_j A_ij U_j) and through
    # -J U_i in W; both couple via the diffusion stencil only.
    Z = y[:, None] + Uold @ tab.A.T + Uold
    Unew = np.empty_like(Uold)
    for s, lu in zip(geom["subs"], lus):
        G = s.B @ Z
        yE = y[s.idxE]
        Ul = np.zeros((s.nE, S))
        for i in range(S):
            x = yE + Ul[:, :i] @ tab.A[i, :i]
            rhs = s.LEE @ x + prob.react(x.reshape(prob.ncomp, -1)).ravel() + G[:, i]
            if i:
                rhs += Ul[:, :i] @ (tab.C[i, :i] / h)
            if src is not None:
                rhs += src[s.idxE, i] + (tab.gamma_rows[i] * h) * ft[s.idxE]
            Ul[:, i] = lu.solve(rhs)
        ctr.bump("local_solves", S)
        ctr.bump("local_rhs_evals", S)
        Unew[s.own_glob, :] = Ul[s.own_pos, :]
    ctr.bump("sweeps")
    return Unew


def schwarz_sweep_reference(prob, tab, P, delta, t, y, h, Uold):
    """Independent, slow formulation used only by the self-check: global f on a
    composite stage state, global J for the exterior coupling, W sliced from the
    global matrix."""
    n = prob.n
    J = prob.jac(y)
    W = (sp.identity(n, format="csr") / (h * tab.gamma) - J).tocsr()
    ft = prob.source_t(t) if prob.has_source else np.zeros(n)
    geom = build_geometry(prob, P, delta)
    Unew = np.empty_like(Uold)
    for s in geom["subs"]:
        outE = np.ones(n)
        outE[s.idxE] = 0.0
        Jext = (J[s.idxE] @ sp.diags(outE)).tocsr()
        lu = splu(W[s.idxE][:, s.idxE].tocsc())
        Ul = np.zeros((s.nE, S))
        for i in range(S):
            X = y + Uold[:, :i] @ tab.A[i, :i]
            X[s.idxE] = y[s.idxE] + Ul[:, :i] @ tab.A[i, :i]
            rhs = prob.f(t + tab.c[i] * h, X)[s.idxE]
            if i:
                rhs += Ul[:, :i] @ (tab.C[i, :i] / h)
            rhs += (tab.gamma_rows[i] * h) * ft[s.idxE] + Jext @ Uold[:, i]
            Ul[:, i] = lu.solve(rhs)
        Unew[s.own_glob, :] = Ul[s.own_pos, :]
    return Unew


# --------------------------------------------------------------------------
# Self-checks
# --------------------------------------------------------------------------
class _Tiny(Problem):
    """Forced van der Pol (mu = 1), two unknowns, for the order check."""

    name = "tiny"
    ncomp = 2
    has_source = True

    def __init__(self):
        self.N, self.ncell, self.n = 1, 1, 2
        self.L = sp.csr_matrix((2, 2))
        self.y0 = np.array([1.0, 0.5])

    def react(self, X):
        a, b = X
        return np.array([b, (1.0 - a * a) * b - a])

    def react_jac(self, X):
        a, b = X
        return [[np.zeros_like(a), np.ones_like(a)], [-2 * a * b - 1.0, 1.0 - a * a]]

    def source(self, t):
        return np.array([0.0, 0.5 * math.cos(2.0 * t)])

    def source_t(self, t):
        return np.array([0.0, -1.0 * math.sin(2.0 * t)])


def self_checks(tab):
    from scipy.integrate import solve_ivp

    out = {}
    out["tableau_c_consistency_max_abs"] = float(np.max(np.abs(tab.c_from_alpha - tab.c)))
    out["tableau_gamma_diag_max_abs"] = float(np.max(np.abs(np.diag(tab.Gamma) - tab.gamma)))

    # (1) observed order of the U-form step on a non-autonomous nonlinear ODE
    tiny = _Tiny()
    ref = solve_ivp(lambda t, y: tiny.f(t, y), (0.0, 1.0), tiny.y0, method="DOP853",
                    rtol=1e-13, atol=1e-14).y[:, -1]
    errs, hs = [], []
    ctr = Counters()
    for nst in (8, 16, 32, 64):
        h = 1.0 / nst
        y = tiny.y0.copy()
        for k in range(nst):
            y = exact_step(tiny, tab, k * h, y, h, 1e-6, ctr)["y_new"]
        errs.append(float(np.max(np.abs(y - ref))))
        hs.append(h)
    orders = [math.log(errs[i] / errs[i + 1]) / math.log(2.0) for i in range(len(errs) - 1)]
    out["order_check"] = {"h": hs, "max_abs_error": errs, "observed_orders": orders}

    # (2) fast sweep == independent reference sweep; exact U is a fixed point
    eq = []
    for key, N, P, delta, nsteps in (("P1", 16, 2, 2, 6), ("P2", 16, 2, 2, 6),
                                     ("P3", 16, 4, 1, 6), ("P2", 16, 4, 3, 6)):
        prob = PROBLEMS[key](N)
        t, y, h = 0.0, prob.y0.copy(), 20 * prob.h0
        for _ in range(nsteps):
            st = exact_step(prob, tab, t, y, h, 1e-4, ctr)
            t, y = t + h, st["y_new"]
        if key == "P2":
            t = 1.2  # exercise the source branch
        st = exact_step(prob, tab, t, y, h, 1e-4, ctr)
        geom = build_geometry(prob, P, delta)
        lus = factor_blocks(prob, tab, geom, y, h, ctr)
        src, ft = stage_sources(prob, tab, t, h)
        U0 = np.zeros_like(st["U"])
        Ua = schwarz_sweep(prob, tab, geom, lus, y, h, U0, src, ft, ctr)
        Ua2 = schwarz_sweep(prob, tab, geom, lus, y, h, Ua, src, ft, ctr)
        Ub = schwarz_sweep_reference(prob, tab, P, delta, t, y, h, U0)
        Ub2 = schwarz_sweep_reference(prob, tab, P, delta, t, y, h, Ub)
        Ufix = schwarz_sweep(prob, tab, geom, lus, y, h, st["U"], src, ft, ctr)
        nrm = float(np.max(np.abs(st["U"])))
        eq.append({
            "problem": prob.name, "N": N, "P": P, "delta": delta,
            "fast_vs_reference_sweep1_rel": float(np.max(np.abs(Ua - Ub))) / nrm,
            "fast_vs_reference_sweep2_rel": float(np.max(np.abs(Ua2 - Ub2))) / nrm,
            "exact_is_fixed_point_rel": float(np.max(np.abs(Ufix - st["U"]))) / nrm,
            "sweep1_differs_from_exact_rel": float(np.max(np.abs(Ua - st["U"]))) / nrm,
        })
    out["sweep_equivalence"] = eq
    ok = (
        out["tableau_c_consistency_max_abs"] < 1e-12
        and min(orders[1:]) > 4.5
        and all(e["fast_vs_reference_sweep1_rel"] < 1e-9 and e["fast_vs_reference_sweep2_rel"] < 1e-9
                and e["exact_is_fixed_point_rel"] < 1e-9 for e in eq)
    )
    out["all_passed"] = bool(ok)
    return out


# --------------------------------------------------------------------------
# One (problem, N, tol) case
# --------------------------------------------------------------------------
def sig(x, digits=6):
    if x is None:
        return None
    x = float(x)
    if not math.isfinite(x):
        return None
    return float(f"{x:.{digits}e}")


def run_iteration(prob, tab, geom, lus, t, y, h, st, U0, src, ft, max_sweeps, ctr):
    scale = st["scale"]
    y_ex, err_ex, U_ex = st["y_new"], st["err"], st["U"]
    rec = {"endpoint_tol": [], "endpoint_rel_err": [], "resid_max": [], "resid_rms": [],
           "errest_diff_tol": [], "sweeps_needed": None, "converged": False, "diverged": False}

    def measure(U):
        if not np.all(np.isfinite(U)):
            return None
        e = wrms(y + U @ tab.b - y_ex, scale)
        rmax, rrms = global_residuals(prob, tab, t, y, h, st["W"], U, src, ft, scale)
        ee = wrms(U[:, S - 1] - U_ex[:, S - 1], scale)
        if not all(math.isfinite(v) for v in (e, rmax, rrms, ee)):
            return None
        return e, rmax, rrms, ee

    def push(m):
        rec["endpoint_tol"].append(sig(m[0]))
        rec["endpoint_rel_err"].append(sig(m[0] / err_ex) if err_ex > 0 else None)
        rec["resid_max"].append(sig(m[1]))
        rec["resid_rms"].append(sig(m[2]))
        rec["errest_diff_tol"].append(sig(m[3]))

    m0 = measure(U0)  # sweep 0 = the initial guess itself
    push(m0)
    U = U0
    for k in range(1, max_sweeps + 1):
        U = schwarz_sweep(prob, tab, geom, lus, y, h, U, src, ft, ctr)
        m = measure(U)
        if m is None or m[0] > 1e30:
            rec["diverged"] = True
            break
        push(m)
        if m[0] <= CONVERGED_TOL_UNITS:
            rec["sweeps_needed"] = k
            rec["converged"] = True
            break
    return rec


def run_case(prob, tab, tol, args, deadline, log):
    ctr = Counters()
    t, y, h = 0.0, prob.y0.copy(), prob.h0
    prev = None
    accepted, rejected, after_reject, since = 0, 0, False, 0
    h_hist = []
    steps = []
    geom_cache = {}
    status = "ok"
    total_steps = 0
    while len(steps) < args.samples:
        if time.time() > deadline:
            status = "aborted_time_budget"
            break
        total_steps += 1
        if total_steps > args.max_steps:
            status = "aborted_max_steps"
            break
        st = exact_step(prob, tab, t, y, h, tol, ctr)
        err = st["err"]
        if (not st["finite"]) or err > 1.0:
            rejected += 1
            fac = 0.2 if not math.isfinite(err) else min(5.0, max(0.2, 0.9 * err ** (-0.2)))
            h *= fac
            after_reject = True
            continue
        fac = 5.0 if err == 0.0 else min(5.0, max(0.2, 0.9 * err ** (-0.2)))
        if after_reject:
            fac = min(fac, 1.0)
        after_reject = False
        accepted += 1
        sampling = t >= prob.t_transient
        h_hist.append([sig(t, 9), sig(h, 9), bool(sampling)])
        if sampling:
            if since % prob.stride == 0:
                steps.append(sample_step(prob, tab, tol, t, y, h, st, prev, geom_cache, args, ctr))
                s = steps[-1]
                log(f"    sample {len(steps):2d}/{args.samples} t={t:.5g} h={h:.4g} sigma={s['sigma']:.4g} "
                    f"ell={s['ell_cells']:.3g} err={err:.3g}  "
                    + " ".join(f"P{c['P']}m{c['m']}{c['guess']}:{c['sweeps_needed']}" for c in s["configs"]))
            since += 1
        prev = (st["U"], h)
        t, y = t + h, st["y_new"]
        h *= fac
    return {
        "status": status,
        "accepted_steps": accepted,
        "rejected_steps": rejected,
        "t_final": sig(t, 9),
        "counters": dict(ctr),
        "accepted_step_history_t_h_sampling": h_hist,
        "steps": steps,
    }


def sample_step(prob, tab, tol, t, y, h, st, prev, geom_cache, args, ctr):
    ell = math.sqrt(tab.gamma * h * prob.Dmax) / prob.dx
    sigma = h * prob.Dmax / prob.dx**2
    src, ft = stage_sources(prob, tab, t, h)
    Uprev, hprev = prev
    guesses = {"g0": np.zeros_like(st["U"]), "g1": (h / hprev) * Uprev}
    out = {"t": sig(t, 9), "h": sig(h, 9), "h_prev": sig(hprev, 9), "sigma": sigma,
           "ell_cells": ell, "err_exact": st["err"], "configs": []}
    done = {}
    for P in args.P:
        H = prob.N // P
        for m in args.m:
            delta = max(1, int(math.ceil(m * ell - 1e-12)))
            key = (P, delta)
            if key not in done:
                if key not in geom_cache:
                    geom_cache[key] = build_geometry(prob, P, delta)
                geom = geom_cache[key]
                lus = factor_blocks(prob, tab, geom, y, h, ctr)
                done[key] = {g: run_iteration(prob, tab, geom, lus, t, y, h, st, U0, src, ft,
                                              args.max_sweeps, ctr)
                             for g, U0 in guesses.items()}
                done[key]["_awf"] = geom["actual_work_factor"]
                del lus
            for g in ("g0", "g1"):
                rec = dict(done[key][g])
                rec.update({
                    "P": P, "m": m, "guess": g, "H": H, "delta_cells": delta,
                    "m_ell_cells": m * ell,
                    "halo_over_H": delta / H,
                    "work_factor_nominal": (1.0 + 2.0 * delta / H) ** 2,
                    "work_factor_actual": done[key]["_awf"],
                    "block_covers_domain": bool(done[key]["_awf"] >= P * P - 1e-12),
                    "delta_cells_le_4ell": bool(delta <= 4.0 * ell + 1e-12),
                })
                out["configs"].append(rec)
    return out


# --------------------------------------------------------------------------
# Aggregation and frozen criteria
# --------------------------------------------------------------------------
def summarize_case(case, args):
    rows = []
    steps = case["steps"]
    if not steps:
        return rows
    cens = args.max_sweeps + 1
    for P in args.P:
        for m in args.m:
            for g in ("g0", "g1"):
                recs = [c for s in steps for c in s["configs"]
                        if c["P"] == P and c["m"] == m and c["guess"] == g]
                needed = [r["sweeps_needed"] for r in recs]
                filled = [cens if v is None else v for v in needed]
                med = float(np.median(filled))
                H = recs[0]["H"]
                deltas = [r["delta_cells"] for r in recs]
                med_delta = float(np.median(deltas))

                def at(key, k):
                    vals = [r[key][k] for r in recs if len(r[key]) > k and r[key][k] is not None]
                    return float(np.median(vals)) if vals else None

                def at_needed(key):
                    vals = [r[key][r["sweeps_needed"]] for r in recs
                            if r["sweeps_needed"] is not None and r[key][r["sweeps_needed"]] is not None]
                    return float(np.median(vals)) if vals else None

                contr = []
                for r in recs:
                    e = r["endpoint_tol"]
                    if len(e) >= 3 and e[1] and e[-1]:
                        contr.append((e[-1] / e[1]) ** (1.0 / (len(e) - 2)))

                row = {
                    "P": P, "m": m, "guess": g, "H": H, "n_steps": len(recs),
                    "endpoint_contraction_per_sweep_median": float(np.median(contr)) if contr else None,
                    "endpoint_contraction_definition": "(e[last]/e[1])^(1/(last-1)) per step, sweeps 1..last",
                    "sweeps_needed": needed,
                    "n_unconverged": int(sum(v is None for v in needed)),
                    "n_diverged": int(sum(r["diverged"] for r in recs)),
                    "censor_value_for_unconverged": cens,
                    "median_sweeps": med,
                    "median_is_censored": bool(med >= cens),
                    "max_sweeps": int(max(filled)),
                    "max_is_censored": bool(max(filled) >= cens),
                    "delta_cells_median": med_delta,
                    "delta_cells_min": int(min(deltas)),
                    "delta_cells_max": int(max(deltas)),
                    "ell_cells_median": float(np.median([s["ell_cells"] for s in steps])),
                    "halo_over_H_median": med_delta / H,
                    "work_factor_nominal_median": float(np.median([r["work_factor_nominal"] for r in recs])),
                    "work_factor_actual_median": float(np.median([r["work_factor_actual"] for r in recs])),
                    "n_steps_block_covers_domain": int(sum(r["block_covers_domain"] for r in recs)),
                    "n_steps_delta_cells_le_4ell": int(sum(r["delta_cells_le_4ell"] for r in recs)),
                    "endpoint_tol_median_sweep0": at("endpoint_tol", 0),
                    "endpoint_tol_median_sweep1": at("endpoint_tol", 1),
                    "endpoint_tol_median_sweep2": at("endpoint_tol", 2),
                    "endpoint_rel_err_median_sweep1": at("endpoint_rel_err", 1),
                    "endpoint_rel_err_median_sweep2": at("endpoint_rel_err", 2),
                    "resid_max_median_sweep1": at("resid_max", 1),
                    "resid_max_median_sweep2": at("resid_max", 2),
                    "per_sweep_medians_are_over": "the steps that ran that sweep (iteration stops at convergence)",
                    "endpoint_tol_median_at_needed_sweep": at_needed("endpoint_tol"),
                    "endpoint_rel_err_median_at_needed_sweep": at_needed("endpoint_rel_err"),
                    "resid_max_median_at_needed_sweep": at_needed("resid_max"),
                    "errest_diff_tol_median_at_needed_sweep": at_needed("errest_diff_tol"),
                }
                c_sweeps = med <= PASS_MEDIAN_SWEEPS
                c_overlap = m <= PASS_MAX_OVERLAP_M
                c_halo = med_delta <= PASS_HALO_FRACTION * H
                killed = med > KILL_MEDIAN_SWEEPS
                row["criteria"] = {
                    "median_sweeps_le_2": bool(c_sweeps),
                    "overlap_le_4ell_nominal": bool(c_overlap),
                    "overlap_cells_le_4ell_every_step": bool(row["n_steps_delta_cells_le_4ell"] == len(recs)),
                    "median_halo_le_H_over_8": bool(c_halo),
                    "predictor_is_g1": g == "g1",
                    "kill_median_sweeps_gt_4": bool(killed),
                }
                numeric = c_sweeps and c_overlap and c_halo
                row["meets_numeric_pass_thresholds"] = bool(numeric)
                if killed:
                    row["verdict"] = "KILL"
                elif g != "g1":
                    row["verdict"] = "REFERENCE_ONLY"  # PASS is defined for predictor g1
                elif numeric:
                    row["verdict"] = "PASS"
                else:
                    row["verdict"] = "FAIL"
                # Strict reading: the integer halo itself must not exceed 4*ell in any step.
                if row["verdict"] == "PASS" and not row["criteria"]["overlap_cells_le_4ell_every_step"]:
                    row["verdict_strict_cells"] = "FAIL"
                else:
                    row["verdict_strict_cells"] = row["verdict"]
                rows.append(row)
    return rows


def stats(vals):
    a = np.array(vals, dtype=float)
    return {"min": float(a.min()), "median": float(np.median(a)), "max": float(a.max())}


def render_markdown(res):
    L = []
    L.append("| problem | N | tol | sigma med (min-max) | ell cells med | P | H | m | halo cells med | halo/H | work (1+2d/H)^2 | "
             "g0 sweeps med/max | g1 sweeps med/max | g1 endpoint/tol after 0, 1, 2 sweeps | g1 contraction/sweep | "
             "steps with halo cells <= 4 ell | verdict g1 | g1 strict cells | g0 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")

    def sw(r):
        cens = r["censor_value_for_unconverged"]
        a = f">{cens - 1}" if r["median_is_censored"] else f"{r['median_sweeps']:g}"
        b = f">{cens - 1}" if r["max_is_censored"] else f"{r['max_sweeps']}"
        return f"{a} / {b}"

    def fm(x):
        return "n/a" if x is None else f"{x:.2g}"

    for case in res["cases"]:
        if not case.get("summary"):
            continue
        sg = case["sigma"]
        rows = case["summary"]
        for P in res["parameters"]["P"]:
            for m in res["parameters"]["m"]:
                r0 = [r for r in rows if r["P"] == P and r["m"] == m and r["guess"] == "g0"][0]
                r1 = [r for r in rows if r["P"] == P and r["m"] == m and r["guess"] == "g1"][0]
                L.append(
                    f"| {case['problem']} | {case['N']} | {case['tol']:g} | {sg['median']:.3g} ({sg['min']:.3g}-{sg['max']:.3g}) "
                    f"| {r1['ell_cells_median']:.3g} | {P} | {r1['H']} | {m} | {r1['delta_cells_median']:g} "
                    f"| {r1['halo_over_H_median']:.3g} | {r1['work_factor_nominal_median']:.3g} "
                    f"| {sw(r0)} | {sw(r1)} | {fm(r1['endpoint_tol_median_sweep0'])}, "
                    f"{fm(r1['endpoint_tol_median_sweep1'])}, {fm(r1['endpoint_tol_median_sweep2'])} "
                    f"| {fm(r1['endpoint_contraction_per_sweep_median'])} "
                    f"| {r1['n_steps_delta_cells_le_4ell']}/{r1['n_steps']} "
                    f"| {r1['verdict']} | {r1['verdict_strict_cells']} | {r0['verdict']} |"
                )
    L.append("")
    L.append("Per-sweep medians are taken over the steps that ran that sweep (the iteration stops at the "
             "first sweep with endpoint difference <= 0.1 tolerance units).")
    L.append("")
    L.append("| problem | N | tol | P | m | median sweeps <= 2 | median halo <= H/8 | start g0 (tol units) | start g1 | "
             "g1 at needed sweep: endpoint/tol | endpoint/err-estimate | max stage residual | error-estimate diff/tol | "
             "actual work factor | steps with block = whole domain |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for case in res["cases"]:
        if not case.get("summary"):
            continue
        rows = case["summary"]
        for P in res["parameters"]["P"]:
            for m in res["parameters"]["m"]:
                r0 = [r for r in rows if r["P"] == P and r["m"] == m and r["guess"] == "g0"][0]
                r1 = [r for r in rows if r["P"] == P and r["m"] == m and r["guess"] == "g1"][0]
                cr = r1["criteria"]
                L.append(
                    f"| {case['problem']} | {case['N']} | {case['tol']:g} | {P} | {m} "
                    f"| {'yes' if cr['median_sweeps_le_2'] else 'no'} | {'yes' if cr['median_halo_le_H_over_8'] else 'no'} "
                    f"| {fm(r0['endpoint_tol_median_sweep0'])} | {fm(r1['endpoint_tol_median_sweep0'])} "
                    f"| {fm(r1['endpoint_tol_median_at_needed_sweep'])} | {fm(r1['endpoint_rel_err_median_at_needed_sweep'])} "
                    f"| {fm(r1['resid_max_median_at_needed_sweep'])} | {fm(r1['errest_diff_tol_median_at_needed_sweep'])} "
                    f"| {r1['work_factor_actual_median']:.3g} | {r1['n_steps_block_covers_domain']}/{r1['n_steps']} |"
                )
    L.append("")
    L.append("Verdict per configuration (predictor g1; `Pp/m` = P x P subdomains, overlap m*ell):")
    L.append("")
    for case in res["cases"]:
        head = f"- {case['problem']} N={case['N']} tol={case['tol']:g}: "
        if not case.get("summary"):
            L.append(head + f"no verdict, status `{case['status']}`"
                     + (f", {case.get('n_sampled_steps', 0)} sampled steps" if "n_sampled_steps" in case else ""))
            continue
        g1 = [r for r in case["summary"] if r["guess"] == "g1"]
        parts = []
        for v in ("PASS", "FAIL", "KILL"):
            names = [f"P{r['P']}/m{r['m']}" for r in g1 if r["verdict"] == v]
            parts.append(f"{v}: {', '.join(names) if names else 'none'}")
        strict_lost = [f"P{r['P']}/m{r['m']}" for r in g1
                       if r["verdict"] == "PASS" and r["verdict_strict_cells"] != "PASS"]
        if strict_lost:
            parts.append("PASS only under the nominal overlap reading (integer halo > 4 ell): "
                         + ", ".join(strict_lost))
        g0k = [f"P{r['P']}/m{r['m']}" for r in case["summary"] if r["guess"] == "g0" and r["verdict"] == "KILL"]
        parts.append(f"g0 killed: {', '.join(g0k) if g0k else 'none'}")
        L.append(head + "; ".join(parts) + ".")
    L.append("")
    L.append(f"Totals over g1 configurations: {res.get('verdict_counts_g1')}; "
             f"strict-cells reading: {res.get('verdict_counts_g1_strict_cells')}. "
             f"Run status `{res.get('status')}`, wall {res.get('wall_time_s', 0):.0f} s (diagnostic only).")
    return "\n".join(L)


def write_json(path, obj):
    tmp = path + ".tmp"
    with open(tmp, "w") as fh:
        json.dump(obj, fh, indent=1, allow_nan=False)
        fh.write("\n")
    os.replace(tmp, path)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--tableau", default=DEFAULT_TABLEAU)
    ap.add_argument("--out", default=os.path.join(HERE, "RESULTS.json"))
    ap.add_argument("--cases", default="P1:64,P2:64,P3:64,P1:128,P3:128,P2:128",
                    help="comma list of problem:N, run in this order")
    ap.add_argument("--tols", default="1e-4,1e-6")
    ap.add_argument("--P", default="2,4")
    ap.add_argument("--m", default="1,2,4")
    ap.add_argument("--samples", type=int, default=20)
    ap.add_argument("--max-sweeps", type=int, default=10)
    ap.add_argument("--max-steps", type=int, default=20000)
    ap.add_argument("--budget-s", type=float, default=1380.0,
                    help="wall budget; cases not started in time are recorded as skipped")
    ap.add_argument("--render", action="store_true",
                    help="print the markdown table of an existing --out file and exit")
    args = ap.parse_args()
    if args.render:
        print(render_markdown(json.load(open(args.out))))
        return 0
    args.P = [int(x) for x in args.P.split(",")]
    args.m = [float(x) if "." in x else int(x) for x in args.m.split(",")]
    tols = [float(x) for x in args.tols.split(",")]
    case_list = [(c.split(":")[0], int(c.split(":")[1])) for c in args.cases.split(",")]

    t_start = time.time()
    deadline = t_start + args.budget_s

    def log(msg):
        print(f"[{time.time() - t_start:7.1f}s] {msg}", flush=True)

    tab = Tableau(args.tableau)
    res = {
        "schema": "vigilode-loop-l1-v14-schwarz-screen-v1",
        "node": "V14",
        "command_line": [sys.executable] + sys.argv,
        "script": {"path": os.path.abspath(__file__),
                   "sha256": hashlib.sha256(open(os.path.abspath(__file__), "rb").read()).hexdigest()},
        "cwd": os.getcwd(),
        "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(t_start)),
        "versions": {"python": platform.python_version(), "numpy": np.__version__, "scipy": scipy.__version__},
        "host": {"machine": platform.machine(), "system": platform.system(), "cpu_count": os.cpu_count(),
                 "loadavg_start": list(os.getloadavg())},
        "wall_time_is": "diagnostic_only",
        "tableau": {"path": tab.path, "sha256": tab.sha256, "schema_version": tab.schema, "gamma": tab.gamma,
                    "gamma_rows_derived": [float(v) for v in tab.gamma_rows]},
        "parameters": {
            "P": args.P, "m": args.m, "tols": tols, "samples_per_case": args.samples,
            "max_sweeps": args.max_sweeps, "budget_s": args.budget_s, "superlu_permc_spec": PERMC,
            "controller": {"norm": "WRMS, scale = atol + rtol*max(|y|,|y_new|), atol = rtol = tol",
                           "safety": 0.9, "factor_min": 0.2, "factor_max": 5.0, "exponent": -0.2,
                           "after_reject_factor_max": 1.0,
                           "note": "stand-in controller, not the protected Rust controller"},
            "delta_rule": "delta_cells = max(1, ceil(m*ell)), ell = sqrt(gamma*h*D_max)/dx",
            "iteration": "restricted additive Schwarz, Jacobi update, whole eight-stage chain per extended block",
            "guesses": {"g0": "zero stage increments",
                        "g1": "previous accepted step's stage increments times h/h_prev"},
            "frozen_criteria": {"converged_endpoint_tol_units": CONVERGED_TOL_UNITS,
                                "pass_median_sweeps_le": PASS_MEDIAN_SWEEPS,
                                "pass_overlap_le_m_ell": PASS_MAX_OVERLAP_M,
                                "pass_median_halo_le_fraction_of_H": PASS_HALO_FRACTION,
                                "kill_median_sweeps_gt": KILL_MEDIAN_SWEEPS},
        },
        "metric_definitions": {
            "endpoint_tol": "WRMS(y_new_sweep - y_new_exact), scale = tol + tol*max(|y|,|y_new_exact|); index = sweep, 0 = initial guess",
            "endpoint_rel_err": "endpoint_tol divided by the exact step's embedded error estimate WRMS(U_8)",
            "resid_max": "max over the 8 stages of WRMS(h*gamma*R_i), R_i the original global stage residual, same scale",
            "resid_rms": "root mean square over the 8 stages of the same",
            "errest_diff_tol": "WRMS(U_8_sweep - U_8_exact), same scale",
            "sweeps_needed": "first sweep with endpoint_tol <= 0.1; null if not reached within max_sweeps",
        },
        "cases": [],
        "status": "running",
    }
    log("self-checks ...")
    res["self_checks"] = self_checks(tab)
    log(f"self-checks all_passed={res['self_checks']['all_passed']} "
        f"orders={['%.3f' % o for o in res['self_checks']['order_check']['observed_orders']]}")
    if not res["self_checks"]["all_passed"]:
        res["status"] = "self_check_failed"
        res["wall_time_s"] = time.time() - t_start
        write_json(args.out, res)
        return 2

    for key, N in case_list:
        for tol in tols:
            entry = {"problem": PROBLEMS[key].name, "problem_key": key, "N": N, "tol": tol}
            if time.time() > deadline:
                entry["status"] = "skipped_time_budget"
                res["cases"].append(entry)
                log(f"SKIP {key} N={N} tol={tol:g} (time budget)")
                continue
            t0 = time.time()
            log(f"case {key} N={N} tol={tol:g}")
            prob = PROBLEMS[key](N)
            entry["problem_definition"] = prob.describe()
            entry["n_unknowns"] = prob.n
            entry["D_max"] = prob.Dmax
            entry["dx"] = prob.dx
            entry["t_transient"] = prob.t_transient
            entry["sample_stride"] = prob.stride
            entry.update(run_case(prob, tab, tol, args, deadline, log))
            if entry["steps"]:
                entry["sigma"] = stats([s["sigma"] for s in entry["steps"]])
                entry["h"] = stats([s["h"] for s in entry["steps"]])
                entry["ell_cells"] = stats([s["ell_cells"] for s in entry["steps"]])
                entry["err_exact"] = stats([s["err_exact"] for s in entry["steps"]])
            entry["n_sampled_steps"] = len(entry["steps"])
            entry["summary"] = summarize_case(entry, args) if entry["status"] == "ok" else []
            entry["wall_time_s"] = time.time() - t0
            res["cases"].append(entry)
            for r in entry["summary"]:
                log(f"  P={r['P']} m={r['m']} {r['guess']}: median={r['median_sweeps']:g} max={r['max_sweeps']} "
                    f"unconv={r['n_unconverged']} halo={r['delta_cells_median']:g} H={r['H']} -> {r['verdict']}")
            res["wall_time_s"] = time.time() - t_start
            write_json(args.out, res)

    table = []
    for c in res["cases"]:
        for r in c.get("summary", []):
            table.append({"problem": c["problem"], "N": c["N"], "tol": c["tol"], "P": r["P"], "m": r["m"],
                          "guess": r["guess"], "median_sweeps": r["median_sweeps"], "max_sweeps": r["max_sweeps"],
                          "median_is_censored": r["median_is_censored"], "max_is_censored": r["max_is_censored"],
                          "delta_cells_median": r["delta_cells_median"], "H": r["H"], "verdict": r["verdict"],
                          "verdict_strict_cells": r["verdict_strict_cells"]})
    res["verdict_table"] = table
    g1 = [r for r in table if r["guess"] == "g1"]
    res["verdict_counts_g1"] = {v: sum(r["verdict"] == v for r in g1) for v in ("PASS", "FAIL", "KILL")}
    res["verdict_counts_g1_strict_cells"] = {v: sum(r["verdict_strict_cells"] == v for r in g1)
                                             for v in ("PASS", "FAIL", "KILL")}
    g0 = [r for r in table if r["guess"] == "g0"]
    res["verdict_counts_g0"] = {v: sum(r["verdict"] == v for r in g0) for v in ("REFERENCE_ONLY", "KILL")}
    res["cases_not_completed"] = [
        {"problem": c["problem"], "N": c["N"], "tol": c["tol"], "status": c["status"]}
        for c in res["cases"] if c["status"] != "ok"]
    res["status"] = "complete" if not res["cases_not_completed"] else "partial"
    res["host"]["loadavg_end"] = list(os.getloadavg())
    res["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    res["wall_time_s"] = time.time() - t_start
    write_json(args.out, res)
    log(f"done status={res['status']} wall={res['wall_time_s']:.1f}s g1 verdicts={res['verdict_counts_g1']}")
    print(render_markdown(res))
    return 0


if __name__ == "__main__":
    sys.exit(main())
