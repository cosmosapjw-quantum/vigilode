#!/usr/bin/env python3
"""Reference for BASS/rei DAG node P03: the bordered LRS photon model.

The P02 photon operator in the physical energy coordinate (first-order
upwind on shared faces, rank-two Thomson, absorber, ledgers in the P02
layout), with three border unknowns after ``L_hi``:

* the shear ``s`` (a state): ``ds/dt = -3 H s + k (D_gamma + D_m)`` with
  ``D_gamma = sum_i w_i sum_j p_j N_ij`` and ``w_i`` the cell average of
  ``exp(q)``; the transport drifts with this ``s``;
* ``x_e`` with the Riccati law ``dx_e/dt = beta (1 - x_e) - alpha x_e^2``,
  setting the Thomson rate ``Gamma = gamma0 x_e``;
* ``D_m`` with ``dD_m/dt = -(D_m + kappa s) / tau``.

The transport is written here from the operations of the executable oracle
(``research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py``), vectorized
in the oracle's accumulation order, and is checked against the oracle; it
is not derived from the Rust adapter. ``H(t)`` comes from the P02 reference
geometry (its shear is unused).
"""
from __future__ import annotations

import math
import sys
from pathlib import Path

import numpy as np
import scipy.sparse as sp
from scipy.integrate import solve_ivp

HERE = Path(__file__).resolve().parent
NODE = HERE.parent
REPO = NODE.parents[1]
sys.path.insert(0, str(REPO / "research" / "bassrei_p02_fixed_grid_20261021" / "reference"))
import lrs_reference as R  # noqa: E402


class Bordered:
    def __init__(self, grid, geom, k, alpha, beta, kappa, tau, gamma0, absorber=None):
        if grid.coordinate != "physical":
            raise ValueError("bordered model: physical coordinate only")
        self.g, self.geom, self.absorber = grid, geom, absorber
        self.k, self.alpha, self.beta = float(k), float(alpha), float(beta)
        self.kappa, self.tau, self.gamma0 = float(kappa), float(tau), float(gamma0)
        g = grid
        self.nph = g.n
        self.n = g.n + 3
        self.IS, self.IX, self.IDM = g.n, g.n + 1, g.n + 2
        me = g.mu_edges
        # the oracle's float formulas
        self.c = np.array([3 * ((b**3 - a**3) / (3 * d)) - 1 for a, b, d in zip(me, me[1:], g.dm_list)])
        self.p2 = np.array([((b**3 - b) - (a**3 - a)) / (2 * d) for a, b, d in zip(me, me[1:], g.dm_list)])
        self.w = np.array([math.exp(g.q_edges[i]) * math.expm1(g.dq_list[i]) / g.dq_list[i] for i in range(g.nq)])
        wp = np.zeros(self.nph)
        wp[g.cells] = self.w[:, None] * self.p2[None, :]
        self.wp = wp
        if absorber is not None:
            self.kbar, _ = R.kbar_closed(g, absorber, geom, 0.0)
        else:
            self.kbar = np.zeros(g.nq)

    def H(self, t):
        h, _ = self.geom.drift(t, "physical")
        return h

    def dH(self, t):
        dh, _ = self.geom.drift_rate(t, "physical")
        return dh

    def transport(self, h, s, vh, vs, N):
        """The oracle's loops, vectorized per column and per angular face in
        the oracle's accumulation order: (cells (nq, nm), L_lo, L_hi)."""
        g = self.g
        nm = g.nm
        dq, dm = g.dq, g.dm
        dens = N / (dq[:, None] * dm[None, :])
        out = np.zeros(N.shape)
        lo = 0.0
        hi = 0.0
        closed = g.boundary == "closed"
        for j in range(nm):
            select = -h - s * self.c[j]
            velocity = -vh - vs * self.c[j]
            f = velocity * dens[:, j] * dm[j]
            if select >= 0:
                # face k (1..nq) carries cell k-1 upward; face 0 has no upstream cell
                out[:, j] -= f
                out[1:, j] += f[:-1]
                if not closed:
                    hi += f[-1]
            else:
                # face k (0..nq-1) carries cell k downward; face nq has none
                out[:, j] += f
                out[:-1, j] -= f[1:]
                if not closed:
                    lo -= f[0]
        for kk in range(1, nm):
            mu = g.mu_edges[kk]
            select = -3 * s * mu * (1 - mu * mu)
            velocity = -3 * vs * mu * (1 - mu * mu)
            up = kk - 1 if select >= 0 else kk
            f = velocity * dens[:, up] * dq
            out[:, kk - 1] -= f
            out[:, kk] += f
        return out, lo, hi

    def transport_abs(self, h, s, vh, vs, N):
        """As ``transport`` but accumulating |flux| at both slots of every
        face (the per-component scale of the accumulated terms)."""
        g = self.g
        nm = g.nm
        dq, dm = g.dq, g.dm
        dens = N / (dq[:, None] * dm[None, :])
        out = np.zeros(N.shape)
        lo = hi = 0.0
        closed = g.boundary == "closed"
        for j in range(nm):
            select = -h - s * self.c[j]
            f = np.abs((-vh - vs * self.c[j]) * dens[:, j] * dm[j])
            if select >= 0:
                out[:, j] += f
                out[1:, j] += f[:-1]
                if not closed:
                    hi += f[-1]
            else:
                out[:, j] += f
                out[:-1, j] += f[1:]
                if not closed:
                    lo += f[0]
        for kk in range(1, nm):
            mu = g.mu_edges[kk]
            select = -3 * s * mu * (1 - mu * mu)
            up = kk - 1 if select >= 0 else kk
            f = np.abs(-3 * vs * mu * (1 - mu * mu) * dens[:, up] * dq)
            out[:, kk - 1] += f
            out[:, kk] += f
        return out, lo, hi

    def partial_t_scale(self, t, y):
        """Per component, the sum of |terms| of ``partial_t``."""
        g = self.g
        y = np.asarray(y, float)
        s = y[self.IS]
        h, dh = self.H(t), self.dH(t)
        cells, lo, hi = self.transport_abs(h, s, dh, 0.0, y[g.cells])
        out = np.zeros(self.n)
        out[g.cells] = cells
        out[g.lo] = lo
        out[g.hi] = hi
        out[self.IS] = abs(3.0 * dh * s)
        return out

    def thomson(self, rate, N):
        m0 = N.sum(axis=1)
        m2 = (self.p2[None, :] * N).sum(axis=1)
        return rate * (self.g.dm[None, :] / 2 * (m0[:, None] + 0.5 * self.p2[None, :] * m2[:, None]) - N)

    def thomson_abs(self, rate, N):
        """Per entry, the sum of |terms| of ``thomson`` (its parity scale)."""
        m0 = np.abs(N).sum(axis=1)
        m2 = np.abs(self.p2[None, :] * N).sum(axis=1)
        return abs(rate) * (self.g.dm[None, :] / 2 * (m0[:, None] + 0.5 * np.abs(self.p2)[None, :] * m2[:, None])
                            + np.abs(N))

    def jacobian_scale(self, t, y):
        """Entry scales for Jacobian parity: |J| itself, except the s and x_e
        columns, which are sums with cancellation (transport inflow minus
        outflow; Thomson gain minus loss): their sums of |terms|."""
        g = self.g
        y = np.asarray(y, float)
        N = y[g.cells]
        s = y[self.IS]
        h = self.H(t)
        E = np.abs(self.jacobian(t, y).toarray())
        cells, lo, hi = self.transport_abs(h, s, 0.0, 1.0, N)
        col = np.zeros(self.n)
        col[g.cells] = cells
        col[g.lo] = lo
        col[g.hi] = hi
        E[:, self.IS] = np.maximum(E[:, self.IS], col)
        col = np.zeros(self.n)
        col[g.cells] = self.thomson_abs(self.gamma0, N)
        E[:, self.IX] = np.maximum(E[:, self.IX], col)
        return E

    def d_gamma(self, y):
        return float(self.wp @ np.asarray(y, float)[: self.nph])

    def rhs(self, t, y):
        g = self.g
        y = np.asarray(y, float)
        N = y[g.cells]
        s, xe, dm_ = y[self.IS], y[self.IX], y[self.IDM]
        h = self.H(t)
        cells, lo, hi = self.transport(h, s, h, s, N)
        gam = self.gamma0 * xe
        if gam != 0.0:
            cells = cells + self.thomson(gam, N)
        rate = self.kbar[:, None] * N
        out = np.zeros(self.n)
        out[g.cells] = cells - rate
        out[g.absorbed] = rate.sum(axis=1)
        out[g.lo] = lo
        out[g.hi] = hi
        out[self.IS] = -3.0 * h * s + self.k * (self.d_gamma(y) + dm_)
        out[self.IX] = self.beta * (1.0 - xe) - self.alpha * xe * xe
        out[self.IDM] = -(dm_ + self.kappa * s) / self.tau
        return out

    def term_scale(self, t, y):
        """Per component, the sum of the absolute values of the accumulated
        terms (the parity scale): |J_ph| |y| for the photon rows."""
        y = np.asarray(y, float)
        s, xe, dm_ = y[self.IS], y[self.IX], y[self.IDM]
        h = self.H(t)
        a = abs(self.photon_jacobian(t, h, s, self.gamma0 * xe)) @ np.abs(y[: self.nph])
        out = np.zeros(self.n)
        out[: self.nph] = a
        out[self.IS] = abs(3.0 * h * s) + abs(self.k) * (np.abs(self.wp) @ np.abs(y[: self.nph]) + abs(dm_))
        out[self.IX] = abs(self.beta * (1.0 - xe)) + abs(self.alpha * xe * xe)
        out[self.IDM] = (abs(dm_) + abs(self.kappa * s)) / self.tau
        return out

    def partial_t(self, t, y):
        g = self.g
        y = np.asarray(y, float)
        s = y[self.IS]
        h, dh = self.H(t), self.dH(t)
        cells, lo, hi = self.transport(h, s, dh, 0.0, y[g.cells])
        out = np.zeros(self.n)
        out[g.cells] = cells
        out[g.lo] = lo
        out[g.hi] = hi
        out[self.IS] = -3.0 * dh * s
        return out

    def photon_jacobian(self, t, h, s, gam):
        """Sparse photon block: the transport at the (h, s) upwind pattern from
        colored unit-vector applications (P02 reference decoding), Thomson at
        rate gam, absorber."""
        g = self.g
        nq, nm = g.nq, g.nm
        rows, cols, vals = [], [], []
        I, J_ = np.meshgrid(np.arange(nq), np.arange(nm), indexing="ij")
        for ci in range(3):
            for cj in range(3):
                on = ((I % 3) == ci) & ((J_ % 3) == cj)
                if not on.any():
                    continue
                cells, _, _ = self.transport(h, s, h, s, on.astype(float))
                di = (ci - I) % 3
                di[di == 2] = -1
                dj = (cj - J_) % 3
                dj[dj == 2] = -1
                si, sj = I + di, J_ + dj
                valid = ((di == 0) | (dj == 0)) & (si >= 0) & (si < nq) & (sj >= 0) & (sj < nm)
                nz = cells != 0
                if np.any(nz & ~valid):
                    raise AssertionError("coloring decode failed")
                sel = nz & valid
                rows.append(g.cells[sel])
                cols.append(g.cells[si[sel], sj[sel]])
                vals.append(cells[sel])
        if g.boundary == "outflow":
            for j in range(nm):
                e = np.zeros((nq, nm))
                if -h - s * self.c[j] >= 0:
                    e[nq - 1, j] = 1.0
                    _, _, v = self.transport(h, s, h, s, e)
                    rows.append([g.hi]); cols.append([g.cells[nq - 1, j]]); vals.append([v])
                else:
                    e[0, j] = 1.0
                    _, v, _ = self.transport(h, s, h, s, e)
                    rows.append([g.lo]); cols.append([g.cells[0, j]]); vals.append([v])
        m = sp.csr_matrix((np.concatenate(vals), (np.concatenate(rows).astype(int), np.concatenate(cols).astype(int))),
                          shape=(self.nph, self.nph))
        if gam != 0.0:
            tb = np.zeros((nm, nm))
            for kk in range(nm):
                e = np.zeros((1, nm))
                e[0, kk] = 1.0
                tb[:, kk] = self.thomson(gam, e)[0]
            r, c_, v_ = [], [], []
            for i in range(nq):
                rr, cc = np.meshgrid(g.cells[i], g.cells[i], indexing="ij")
                r.append(rr.ravel()); c_.append(cc.ravel()); v_.append(tb.ravel())
            m = m + sp.csr_matrix((np.concatenate(v_), (np.concatenate(r), np.concatenate(c_))), shape=(self.nph, self.nph))
        if self.absorber is not None:
            kc = np.repeat(self.kbar, nm)
            m = m + sp.csr_matrix((-kc, (g.cell_flat, g.cell_flat)), shape=(self.nph, self.nph))
            m = m + sp.csr_matrix((kc, (np.repeat(g.absorbed, nm), g.cell_flat)), shape=(self.nph, self.nph))
        return m.tocsr()

    def jacobian(self, t, y):
        g = self.g
        y = np.asarray(y, float)
        N = y[g.cells]
        s, xe = y[self.IS], y[self.IX]
        h = self.H(t)
        A = self.photon_jacobian(t, h, s, self.gamma0 * xe)
        cells, lo, hi = self.transport(h, s, 0.0, 1.0, N)
        col_s = np.zeros(self.nph)
        col_s[g.cells] = cells
        col_s[g.lo] = lo
        col_s[g.hi] = hi
        col_x = np.zeros(self.nph)
        col_x[g.cells] = self.thomson(self.gamma0, N)
        B = sp.csr_matrix(np.column_stack([col_s, col_x, np.zeros(self.nph)]))
        C = sp.csr_matrix(np.vstack([self.k * self.wp, np.zeros(self.nph), np.zeros(self.nph)]))
        D = sp.csr_matrix(np.array([[-3.0 * h, 0.0, self.k],
                                    [0.0, -self.beta - 2.0 * self.alpha * xe, 0.0],
                                    [-self.kappa / self.tau, 0.0, -1.0 / self.tau]]))
        return sp.bmat([[A, B], [C, D]]).tocsc()

    def riccati(self, x0, t):
        a, b = self.alpha, self.beta
        if a == 0.0:
            return 1.0 + (x0 - 1.0) * math.exp(-b * t)
        root = math.sqrt(b * b + 4.0 * a * b)
        r1 = 2.0 * b / (b + root)
        r2 = -(b + root) / (2.0 * a)
        c = (x0 - r1) / (x0 - r2)
        e = c * math.exp(-a * (r1 - r2) * t)
        return r1 + (r1 - r2) * e / (1.0 - e)


def ref_radau(model: Bordered, y0, times, rtol, atol):
    """Radau with the analytic sparse Jacobian, restarted at every output."""
    out = [np.array(y0, float)]
    y = np.array(y0, float)
    nfev = njev = nlu = 0
    for t0, t1 in zip(times[:-1], times[1:]):
        sol = solve_ivp(model.rhs, (t0, t1), y, method="Radau", jac=model.jacobian, rtol=rtol, atol=atol)
        if not sol.success:
            raise RuntimeError(f"Radau failed: {sol.message}")
        y = sol.y[:, -1].copy()
        out.append(y)
        nfev += sol.nfev
        njev += sol.njev
        nlu += sol.nlu
    return np.array(out), {"nfev": int(nfev), "njev": int(njev), "nlu": int(nlu)}
