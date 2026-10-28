"""Independent Python references for BASS/rei DAG node P02 (fixed-grid LRS
Bianchi-I photon transport adapter run through RODAS5P stages).

Nothing here is ported from the Rust adapter. The pieces are written from
the physics or taken from the read-only formula oracle
`research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py` (loaded by file
path through importlib, never copied or modified, no bytecode written):

* state layout `[L_lo, (N_i0..N_i(nm-1), A_i) for i in 0..nq, L_hi]`
  (`cell(i,j) = 1 + i (nm+1) + j`, `absorbed(i) = 1 + i (nm+1) + nm`);
* exact initial cell integrals of
  `U0(q, mu) = C exp(-(q + 0.5)^2 / 0.18) (1 + 0.5 P2(mu))`, `C = 4.1e8`
  (separable: erf/erfc in q at 40 digits, polynomial in mu);
* closed-form geometry (`H, s, dH, ds, ln a, int s, ln A, ln B`);
* the semi-discrete operator `A(t)` as a scipy.sparse matrix assembled from
  the oracle: for a fixed upwind sign pattern the oracle transport is
  `L(H, s) = H L_H + s L_s`; `L_H`, `L_s` come from two oracle evaluations
  (unit vectors applied through a provably collision-free 3x3 coloring) and
  the split is verified at further points; per-boundary ledger rows come
  from an independent face accounting and are checked against the oracle
  `outward`; Thomson blocks from `thomson_rhs` on unit vectors; absorber
  cell averages `kbar(i, t)` / `dkbar/dt` in closed form and by mpmath
  quadrature;
* `dA/dt` with the upwind pattern of `A(t)`;
* ODE references: `expm_multiply` (autonomous), DOP853 and Radau (analytic
  sparse Jacobian) restarted at every output time;
* the exact collisionless (and absorbing) continuum solution along
  characteristics with tensor Gauss-Legendre cell integrals on subcells,
  explicit splitting at the two absorption kinks, and conservation checks.

Python allocation is intentional (formula references, not a solver port).
"""
from __future__ import annotations

import hashlib
import importlib.util
import math
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True

import mpmath as mp  # noqa: E402
import numpy as np  # noqa: E402
import scipy.sparse as sp  # noqa: E402
from scipy.integrate import solve_ivp  # noqa: E402
from scipy.sparse.linalg import expm_multiply  # noqa: E402

C_AMP = 4.1e8
Q_CENTER = -0.5
Q_WIDTH2 = 0.18  # 2 sigma^2 of the energy Gaussian
EPS = float(np.finfo(float).eps)

NODE_DIR = Path(__file__).resolve().parents[1]
REPO = NODE_DIR.parents[1]
ORACLE_PATH = REPO / "research" / "ncp_followup_20261021" / "BASS_REI_FVM_REFERENCE.py"


def sha256_file(path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_oracle(path=ORACLE_PATH):
    """Import the oracle read-only by file path (no bytecode written)."""
    prev = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        spec = importlib.util.spec_from_file_location("bass_rei_fvm_oracle_readonly", str(path))
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
    finally:
        sys.dont_write_bytecode = prev
    return mod


# ---------------------------------------------------------------------------
# Grid and layout
# ---------------------------------------------------------------------------


class Grid:
    """Energy x angle finite-volume grid with the P02 state layout."""

    def __init__(self, q_edges, mu_edges, coordinate="physical", boundary="outflow"):
        if coordinate not in ("physical", "comoving") or boundary not in ("outflow", "closed"):
            raise ValueError("bad coordinate/boundary")
        self.q_edges = [float(x) for x in q_edges]
        self.mu_edges = [float(x) for x in mu_edges]
        if self.mu_edges[0] != -1.0 or self.mu_edges[-1] != 1.0:
            raise ValueError("full sphere required")
        self.coordinate = coordinate
        self.boundary = boundary
        # Same float subtraction as the oracle's _grid.
        self.dq_list = [b - a for a, b in zip(self.q_edges, self.q_edges[1:])]
        self.dm_list = [b - a for a, b in zip(self.mu_edges, self.mu_edges[1:])]
        if min(self.dq_list) <= 0 or min(self.dm_list) <= 0:
            raise ValueError("positive widths required")
        self.qe = np.array(self.q_edges)
        self.me = np.array(self.mu_edges)
        self.dq = np.array(self.dq_list)
        self.dm = np.array(self.dm_list)
        self.nq = len(self.dq_list)
        self.nm = len(self.dm_list)
        self.n = 2 + self.nq * (self.nm + 1)
        i = np.arange(self.nq)[:, None]
        j = np.arange(self.nm)[None, :]
        self.cells = 1 + i * (self.nm + 1) + j  # (nq, nm)
        self.absorbed = 1 + np.arange(self.nq) * (self.nm + 1) + self.nm
        self.lo = 0
        self.hi = self.n - 1
        self.ledgers = np.concatenate([[0], self.absorbed, [self.n - 1]]).astype(int)
        self.cell_flat = self.cells.ravel()
        # The oracle's float formula for c_j = 3 <mu^2>_j - 1 (sign selection).
        self.c = [3 * ((b**3 - a**3) / (3 * d)) - 1 for a, b, d in zip(self.mu_edges, self.mu_edges[1:], self.dm_list)]
        # Exact cell averages of P2 from the exact rationals of the float edges.
        p2 = []
        for a, b in zip(self.mu_edges, self.mu_edges[1:]):
            fa, fb = Fraction(a), Fraction(b)
            p2.append(float(((fb**3 - fb) - (fa**3 - fa)) / (2 * (fb - fa))))
        self.p2_exact = np.array(p2)

    def measure(self):
        return self.dq[:, None] * self.dm[None, :]

    def to_oracle(self, y):
        return [float(v) for v in np.asarray(y)[self.cell_flat]]

    def layout_request(self):
        return {"q_edges": self.q_edges, "mu_edges": self.mu_edges,
                "coordinate": self.coordinate, "boundary": self.boundary}

    def describe(self):
        return {"nq": self.nq, "nm": self.nm, "n": self.n, "q_range": [self.q_edges[0], self.q_edges[-1]],
                "coordinate": self.coordinate, "boundary": self.boundary,
                "uniform_q": bool(np.allclose(self.dq, self.dq[0], rtol=0, atol=0)),
                "uniform_mu": bool(np.allclose(self.dm, self.dm[0], rtol=0, atol=0))}


def uniform_grid(q_lo, q_hi, nq, nm, coordinate="physical", boundary="outflow"):
    return Grid(np.linspace(q_lo, q_hi, nq + 1).tolist(), np.linspace(-1.0, 1.0, nm + 1).tolist(),
                coordinate, boundary)


# ---------------------------------------------------------------------------
# Initial condition (exact cell integrals)
# ---------------------------------------------------------------------------


def _gauss_q_integral_mp(a, b):
    """int_a^b exp(-(q + 0.5)^2 / 0.18) dq at the working mp precision."""
    s = mp.sqrt(mp.mpf(Q_WIDTH2))
    za = (mp.mpf(a) - mp.mpf(Q_CENTER)) / s
    zb = (mp.mpf(b) - mp.mpf(Q_CENTER)) / s
    if za >= 0:
        d = mp.erfc(za) - mp.erfc(zb)
    elif zb <= 0:
        d = mp.erfc(-zb) - mp.erfc(-za)
    else:
        d = mp.erf(zb) - mp.erf(za)
    return s * mp.sqrt(mp.pi) / 2 * d


def _mu_integral_mp(a, b):
    """int_a^b (1 + 0.5 P2(mu)) dmu, P2 = (3 mu^2 - 1) / 2 (exact polynomial)."""
    a, b = mp.mpf(a), mp.mpf(b)
    return (b - a) + mp.mpf(1) / 4 * ((b**3 - b) - (a**3 - a))


def initial_cells(grid: Grid):
    """Exact (40-digit, single rounding) cell integrals of U0; at t = 0 the
    comoving coordinate equals q, so the same formula applies."""
    with mp.workdps(40):
        iq = [_gauss_q_integral_mp(a, b) for a, b in zip(grid.q_edges, grid.q_edges[1:])]
        im = [_mu_integral_mp(a, b) for a, b in zip(grid.mu_edges, grid.mu_edges[1:])]
        c = mp.mpf(C_AMP)
        n0 = np.array([[float(c * x * y) for y in im] for x in iq])
        total = float(c * mp.fsum(iq) * mp.fsum(im))
    return n0, total


def initial_state(grid: Grid):
    n0, total = initial_cells(grid)
    y0 = np.zeros(grid.n)
    y0[grid.cells] = n0
    return y0, total


def atol_vector(grid: Grid, cell_scale, ledger_value):
    """atol_i = cell_scale * C * |cell| for cells, ledger_value for ledgers."""
    a = np.zeros(grid.n)
    a[grid.cells] = cell_scale * C_AMP * grid.measure()
    a[grid.ledgers] = ledger_value
    return a


# ---------------------------------------------------------------------------
# Geometry
# ---------------------------------------------------------------------------


class Geometry:
    """`constant {h, s}` or `power_law {hh, s0, p}`: H = hh/(1+t), s = s0 (1+t)^-p."""

    def __init__(self, kind, **kw):
        self.kind = kind
        if kind == "constant":
            self.h = float(kw["h"])
            self.s = float(kw["s"])
        elif kind == "power_law":
            self.hh = float(kw["hh"])
            self.s0 = float(kw["s0"])
            self.p = float(kw["p"])
            if self.p == 1.0:
                raise ValueError("p != 1 required")
        else:
            raise ValueError(kind)

    def request(self):
        if self.kind == "constant":
            return {"kind": "constant", "h": self.h, "s": self.s}
        return {"kind": "power_law", "hh": self.hh, "s0": self.s0, "p": self.p}

    def H(self, t):
        t = np.asarray(t, float)
        return np.full_like(t, self.h) if self.kind == "constant" else self.hh / (1.0 + t)

    def S(self, t):
        t = np.asarray(t, float)
        return np.full_like(t, self.s) if self.kind == "constant" else self.s0 * (1.0 + t) ** (-self.p)

    def dH(self, t):
        t = np.asarray(t, float)
        return np.zeros_like(t) if self.kind == "constant" else -self.hh / (1.0 + t) ** 2

    def dS(self, t):
        t = np.asarray(t, float)
        return np.zeros_like(t) if self.kind == "constant" else -self.p * self.s0 * (1.0 + t) ** (-self.p - 1.0)

    def ln_a(self, t):
        t = np.asarray(t, float)
        return self.h * t if self.kind == "constant" else self.hh * np.log1p(t)

    def int_s(self, t):
        t = np.asarray(t, float)
        if self.kind == "constant":
            return self.s * t
        return self.s0 * np.expm1((1.0 - self.p) * np.log1p(t)) / (1.0 - self.p)

    def lnA(self, t):
        """ln A = int (H - s): transverse scale factor."""
        return self.ln_a(t) - self.int_s(t)

    def lnB(self, t):
        """ln B = int (H + 2 s): axial scale factor."""
        return self.ln_a(t) + 2.0 * self.int_s(t)

    def ln_a_mp(self, t):
        t = mp.mpf(t)
        return mp.mpf(self.h) * t if self.kind == "constant" else mp.mpf(self.hh) * mp.log(1 + t)

    def drift(self, t, coordinate):
        """(H_eff, s): comoving coordinates drop H from the energy drift."""
        h = 0.0 if coordinate == "comoving" else float(self.H(t))
        return h, float(self.S(t))

    def drift_rate(self, t, coordinate):
        dh = 0.0 if coordinate == "comoving" else float(self.dH(t))
        return dh, float(self.dS(t))


def geometry_self_check(geom: Geometry, T):
    """Closed forms vs quadrature / finite differences (relative)."""
    from scipy.integrate import quad

    out = {}
    worst = 0.0
    for t in np.linspace(0.1, T, 5):
        la = quad(lambda x: float(geom.H(x)), 0, t, epsabs=0, epsrel=1e-13, limit=200)[0]
        ls = quad(lambda x: float(geom.S(x)), 0, t, epsabs=0, epsrel=1e-13, limit=200)[0]
        e1 = abs(la - float(geom.ln_a(t))) / max(abs(la), 1e-300)
        e2 = abs(ls - float(geom.int_s(t))) / max(abs(ls), 1e-300) if ls != 0 else abs(float(geom.int_s(t)))
        d = 1e-5
        fdh = (float(geom.H(t + d)) - float(geom.H(t - d))) / (2 * d)
        fds = (float(geom.S(t + d)) - float(geom.S(t - d))) / (2 * d)
        e3 = abs(fdh - float(geom.dH(t))) / max(abs(float(geom.dH(t))), 1e-12) if geom.kind != "constant" else abs(fdh)
        e4 = abs(fds - float(geom.dS(t))) / max(abs(float(geom.dS(t))), 1e-12) if geom.kind != "constant" else abs(fds)
        worst = max(worst, e1, e2)
        out[f"t={t:.3g}"] = {"ln_a_vs_quad": e1, "int_s_vs_quad": e2, "dH_vs_fd": e3, "ds_vs_fd": e4}
    return {"per_time": out, "max_closed_form_vs_quad": worst}


# ---------------------------------------------------------------------------
# Absorber
# ---------------------------------------------------------------------------


class Absorber:
    """k(E) = k0 (E / chi)^-3 for E >= chi, else 0; chi = exp(q_chi) (E_ref = 1)."""

    def __init__(self, k0, q_chi):
        self.k0 = float(k0)
        self.q_chi = float(q_chi)

    def request(self):
        return {"k0": self.k0, "q_chi": self.q_chi}


def threshold(absorber, geom, coordinate, t):
    """Threshold position in grid coordinates and its rate."""
    if coordinate == "comoving":
        return absorber.q_chi + float(geom.ln_a(t)), float(geom.H(t))
    return absorber.q_chi, 0.0


def tie_masks(lo, hi, xc, rate):
    """(below, above, straddle) cell masks with the tie rule: at an exact tie
    the branch follows the threshold's motion (the branch a cell is in for
    t' > t, i.e. the right-sided time derivative). rate > 0: lo == xc
    straddles; rate < 0: hi == xc straddles (value 0); rate == 0: ties count
    as below (hi == xc) / above (lo == xc). Values are continuous."""
    lo, hi = np.asarray(lo), np.asarray(hi)
    below = (hi < xc) if rate < 0 else (hi <= xc)
    above = (lo > xc) if rate > 0 else (lo >= xc)
    return below, above, (~below) & (~above)


def kbar_categories(grid: Grid, xc, rate):
    below, above, _ = tie_masks(grid.qe[:-1], grid.qe[1:], xc, rate)
    return ["below" if b else ("above" if a else "straddle") for b, a in zip(below, above)]


def kbar_closed(grid: Grid, absorber, geom, t):
    """Own closed form of the cell average of k over the energy cell (density
    uniform in the cell coordinate) and its time derivative (Leibniz), with
    the tie rule of `tie_masks` (right-sided derivative at an exact tie)."""
    if absorber is None:
        return np.zeros(grid.nq), np.zeros(grid.nq)
    xc, rate = threshold(absorber, geom, grid.coordinate, t)
    lo, hi, dx = grid.qe[:-1], grid.qe[1:], grid.dq
    k = np.zeros(grid.nq)
    dk = np.zeros(grid.nq)
    k0 = absorber.k0
    _, above, strad = tie_masks(lo, hi, xc, rate)
    # above: (k0 / (3 dx)) [e^{-3(lo-xc)} - e^{-3(hi-xc)}] = (k0/(3dx)) e^{-3(lo-xc)} (-expm1(-3 dx))
    ka = k0 / (3.0 * dx[above]) * np.exp(-3.0 * (lo[above] - xc)) * (-np.expm1(-3.0 * dx[above]))
    k[above] = ka
    dk[above] = 3.0 * rate * ka  # k proportional to exp(3 xc)
    # straddle: (k0 / (3 dx)) [1 - e^{-3(hi-xc)}]; d/dt = -(k0/dx) rate e^{-3(hi-xc)}
    w = hi[strad] - xc
    k[strad] = k0 / (3.0 * dx[strad]) * (-np.expm1(-3.0 * w))
    dk[strad] = -(k0 / dx[strad]) * rate * np.exp(-3.0 * w)
    return k, dk


def kbar_mpmath(grid: Grid, absorber, geom, t, dps=40):
    """kbar by mpmath quadrature from the definition; dkbar both by mp.diff
    of that quadrature and by the mp Leibniz rule (returned with their
    mutual deviation). The Leibniz rule applies the tie rule of `tie_masks`
    with the exact mp threshold; at an exact tie (lo == xc or hi == xc in mp)
    mp.diff is the one-sided forward difference in time (the direction the
    threshold moves into), elsewhere the central one. Every cell is checked
    (no edge-distance skip); returns (k, dk, dev, ties)."""
    out_k, out_dk, dev, ties = [], [], 0.0, []
    with mp.workdps(dps):
        def xchi_of(tt):
            base = mp.mpf(absorber.q_chi)
            return base + geom.ln_a_mp(tt) if grid.coordinate == "comoving" else base

        def kb(i, tt):
            lo, hi = mp.mpf(grid.q_edges[i]), mp.mpf(grid.q_edges[i + 1])
            xc = xchi_of(tt)
            if hi <= xc:
                return mp.mpf(0)
            f = lambda x: mp.mpf(absorber.k0) * mp.exp(-3 * (x - xc)) if x >= xc else mp.mpf(0)  # noqa: E731
            pts = [lo, xc, hi] if lo < xc < hi else [lo, hi]
            return mp.quad(f, pts) / (hi - lo)

        tt = mp.mpf(t)
        if grid.coordinate == "comoving":
            if geom.kind == "constant":
                rate = mp.mpf(geom.h)
            else:
                rate = mp.mpf(geom.hh) / (1 + tt)
        else:
            rate = mp.mpf(0)
        for i in range(grid.nq):
            k = kb(i, tt)
            lo, hi = mp.mpf(grid.q_edges[i]), mp.mpf(grid.q_edges[i + 1])
            xc = xchi_of(tt)
            below = (hi < xc) if rate < 0 else (hi <= xc)
            above = (lo > xc) if rate > 0 else (lo >= xc)
            if below:
                lb = mp.mpf(0)
            else:
                a = lo if above else xc
                integ = mp.quad(lambda x: mp.mpf(absorber.k0) * mp.exp(-3 * (x - xc)), [a, hi]) if a < hi \
                    else mp.mpf(0)
                lb = (3 * rate * integ - (0 if above else mp.mpf(absorber.k0) * rate)) / (hi - lo)
            tie = lo == xc or hi == xc
            if tie:
                ties.append(i)
                d = mp.diff(lambda s: kb(i, s), tt, direction=1)
            else:
                d = mp.diff(lambda s: kb(i, s), tt)
            if not mp.isfinite(d):
                dev = float("nan")
            elif lb != 0:
                dev = max(dev, float(abs(d - lb) / abs(lb)))
            else:
                dev = max(dev, float(abs(d)))
            out_k.append(k)
            out_dk.append(lb)
    return out_k, out_dk, dev, ties


# ---------------------------------------------------------------------------
# Operator assembled from the oracle
# ---------------------------------------------------------------------------


def sign_pattern(grid: Grid, H, s):
    """Upwind selections exactly as the oracle computes them (float ops)."""
    qsel = tuple((-H - s * c) >= 0 for c in grid.c)
    msel = tuple((-3 * s * mu * (1 - mu * mu)) >= 0 for mu in grid.mu_edges[1:-1])
    return qsel, msel


class OracleOperator:
    def __init__(self, oracle, grid: Grid):
        self.oracle = oracle
        self.grid = grid
        self.oracle_calls = 0

    def oracle_rhs(self, y, H, s):
        self.oracle_calls += 1
        g = self.grid
        rhs, outward = self.oracle.lrs_number_rhs(g.q_edges, g.mu_edges, g.to_oracle(y), float(H), float(s), g.boundary)
        return np.array(rhs).reshape(g.nq, g.nm), outward

    def _cells_part(self, H, s, colored=True):
        """Cell rows of the transport matrix from oracle applications to unit
        vectors. With `colored`, 9 vectors switch on cells with (i mod 3,
        j mod 3) fixed: every row's 5-point neighbourhood then contains at
        most one switched-on cell, so each nonzero row value is the single
        unit-vector entry (bit-identical; checked against plain unit vectors
        on small grids)."""
        g = self.grid
        nq, nm = g.nq, g.nm
        rows, cols, vals = [], [], []
        if not colored:
            for i in range(nq):
                for j in range(nm):
                    e = np.zeros(g.n)
                    e[g.cells[i, j]] = 1.0
                    r, _ = self.oracle_rhs(e, H, s)
                    nz = np.nonzero(r)
                    rows.append(g.cells[nz])
                    cols.append(np.full(len(nz[0]), g.cells[i, j]))
                    vals.append(r[nz])
        else:
            I, J = np.meshgrid(np.arange(nq), np.arange(nm), indexing="ij")
            for ci in range(3):
                for cj in range(3):
                    on = ((I % 3) == ci) & ((J % 3) == cj)
                    if not on.any():
                        continue
                    e = np.zeros(g.n)
                    e[g.cells[on]] = 1.0
                    r, _ = self.oracle_rhs(e, H, s)
                    di = (ci - I) % 3
                    di[di == 2] = -1
                    dj = (cj - J) % 3
                    dj[dj == 2] = -1
                    si, sj = I + di, J + dj
                    valid = ((di == 0) | (dj == 0)) & (si >= 0) & (si < nq) & (sj >= 0) & (sj < nm)
                    nzm = r != 0
                    if np.any(nzm & ~valid):
                        raise AssertionError("coloring decode: value without a unique source")
                    sel = nzm & valid
                    rows.append(g.cells[sel])
                    cols.append(g.cells[si[sel], sj[sel]])
                    vals.append(r[sel])
        rows = np.concatenate(rows) if rows else np.zeros(0, int)
        cols = np.concatenate(cols) if cols else np.zeros(0, int)
        vals = np.concatenate(vals) if vals else np.zeros(0)
        return rows, cols, vals

    def _ledger_part(self, H, s):
        """Independent face accounting of the energy-boundary outflow."""
        g = self.grid
        rows, cols, vals = [], [], []
        if g.boundary == "closed":
            return rows, cols, vals
        for j, c in enumerate(g.c):
            v = -H - s * c
            if v >= 0:  # face nq is fed by cell nq-1 into L_hi
                rows.append(g.hi)
                cols.append(g.cells[g.nq - 1, j])
                vals.append(v / g.dq_list[g.nq - 1])
            else:  # face 0 is fed by cell 0 into L_lo (outward = -flux > 0)
                rows.append(g.lo)
                cols.append(g.cells[0, j])
                vals.append(-v / g.dq_list[0])
        return rows, cols, vals

    def transport(self, H, s, colored=True):
        g = self.grid
        r1, c1, v1 = self._cells_part(H, s, colored)
        r2, c2, v2 = self._ledger_part(H, s)
        rows = np.concatenate([r1, np.array(r2, int)])
        cols = np.concatenate([c1, np.array(c2, int)])
        vals = np.concatenate([v1, np.array(v2, float)])
        m = sp.csr_matrix((vals, (rows, cols)), shape=(g.n, g.n))
        m.sum_duplicates()
        return m

    def thomson_block(self):
        """nm x nm rate-1 Thomson matrix from oracle thomson_rhs on unit vectors."""
        g = self.grid
        t1 = np.zeros((g.nm, g.nm))
        for k in range(g.nm):
            e = [0.0] * g.nm
            e[k] = 1.0
            t1[:, k] = self.oracle.thomson_rhs(g.mu_edges, e, 1.0)
        return t1

    def thomson(self):
        g = self.grid
        t1 = self.thomson_block()
        rows, cols, vals = [], [], []
        for i in range(g.nq):
            rr, cc = np.meshgrid(g.cells[i], g.cells[i], indexing="ij")
            rows.append(rr.ravel())
            cols.append(cc.ravel())
            vals.append(t1.ravel())
        return sp.csr_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))), shape=(g.n, g.n))


def sparse_max_abs(m):
    m = m.tocsr()
    return float(np.max(np.abs(m.data))) if m.nnz else 0.0


def transport_split(op: OracleOperator, points):
    """L_H, L_s from oracle transports at (H, s) points sharing one upwind
    pattern; verified at every other point. Returns (L_H|None, L_s, info)."""
    g = op.grid
    pats = [sign_pattern(g, h, s) for h, s in points]
    if any(p != pats[0] for p in pats):
        raise ValueError("upwind sign pattern changes over the scenario")
    mats = [op.transport(h, s) for h, s in points]
    all_h0 = all(h == 0.0 for h, _ in points)
    if all_h0:
        h0, s0 = points[0]
        L_H = None
        L_s = mats[0] / s0
        used = [0]
    else:
        h_a, s_a = points[0]
        best, kb = 0.0, None
        for k, (h, s) in enumerate(points[1:], start=1):
            det = h_a * s - h * s_a
            if abs(det) > best:
                best, kb = abs(det), k
        if kb is None or best == 0.0:
            raise ValueError("split needs two independent (H, s) points")
        h_b, s_b = points[kb]
        det = h_a * s_b - h_b * s_a
        L_H = (s_b * mats[0] - s_a * mats[kb]) / det
        L_s = (h_a * mats[kb] - h_b * mats[0]) / det
        used = [0, kb]
    dev = 0.0
    checked = 0
    for k, ((h, s), m) in enumerate(zip(points, mats)):
        rec = (s * L_s) if L_H is None else (h * L_H + s * L_s)
        d = sparse_max_abs(m - rec) / max(sparse_max_abs(m), 1e-300)
        if k not in used:
            checked += 1
        dev = max(dev, d)
    for m in (L_H, L_s):
        if m is not None:
            m.eliminate_zeros()
    return L_H, L_s, {"points": [list(p) for p in points], "used_for_solve": used,
                      "verified_points": checked, "max_split_deviation_rel": dev}


class Model:
    """A(t) = transport(t) + gamma Th + Abs(t) in the P02 layout."""

    def __init__(self, oracle, grid: Grid, geom: Geometry, gamma=0.0, absorber=None,
                 t_range=(0.0, 1.0), split_points=None):
        self.grid = grid
        self.geom = geom
        self.gamma = float(gamma)
        self.absorber = absorber
        self.op = OracleOperator(oracle, grid)
        coord = grid.coordinate
        self.split_info = None
        if geom.kind == "constant":
            h, s = geom.drift(0.0, coord)
            self.M_const = self.op.transport(h, s)
            self.L_H = self.L_s = None
        else:
            self.M_const = None
            if split_points is None:
                ts = np.linspace(t_range[0], t_range[1], 13)
                order = [0, len(ts) - 1] + list(range(1, len(ts) - 1))
                split_points = [geom.drift(float(ts[k]), coord) for k in order]
            self.L_H, self.L_s, self.split_info = transport_split(self.op, split_points)
        self.Th = self.op.thomson() if self.gamma != 0.0 else None
        if absorber is not None:
            g = grid
            rows = np.concatenate([g.cell_flat, np.repeat(g.absorbed, g.nm)])
            cols = np.concatenate([g.cell_flat, g.cell_flat])
            self._abs_rows, self._abs_cols = rows, cols
            self._abs_group = np.concatenate([np.repeat(np.arange(g.nq), g.nm)] * 2)
            self._abs_sign = np.concatenate([-np.ones(g.nq * g.nm), np.ones(g.nq * g.nm)])

    def transport_matrix(self, t):
        if self.M_const is not None:
            return self.M_const
        h, s = self.geom.drift(t, self.grid.coordinate)
        return (s * self.L_s) if self.L_H is None else (h * self.L_H + s * self.L_s)

    def _abs_matrix(self, kvec):
        n = self.grid.n
        return sp.csr_matrix((self._abs_sign * kvec[self._abs_group], (self._abs_rows, self._abs_cols)), shape=(n, n))

    def A(self, t):
        m = self.transport_matrix(t)
        if self.Th is not None:
            m = m + self.gamma * self.Th
        if self.absorber is not None:
            k, _ = kbar_closed(self.grid, self.absorber, self.geom, t)
            m = m + self._abs_matrix(k)
        return m.tocsr()

    def dA(self, t):
        n = self.grid.n
        if self.M_const is not None:
            m = sp.csr_matrix((n, n))
        else:
            dh, ds = self.geom.drift_rate(t, self.grid.coordinate)
            m = (ds * self.L_s) if self.L_H is None else (dh * self.L_H + ds * self.L_s)
        if self.absorber is not None:
            _, dk = kbar_closed(self.grid, self.absorber, self.geom, t)
            m = m + self._abs_matrix(dk)
        return m.tocsr()

    def autonomous(self):
        return self.geom.kind == "constant" and (self.absorber is None or self.grid.coordinate == "physical")

    def f(self, t, y):
        return self.A(t) @ y

    def jac(self, t, y):
        return self.A(t).tocsc()


# ---------------------------------------------------------------------------
# ODE references (restarted at every output time: no dense-output interpolation)
# ---------------------------------------------------------------------------


def ref_expm(model: Model, y0, times):
    a = model.A(0.0).tocsc()
    out = [np.array(y0, float)]
    y = np.array(y0, float)
    for t0, t1 in zip(times[:-1], times[1:]):
        y = expm_multiply(a * (t1 - t0), y)
        out.append(y)
    return np.array(out)


def ref_ivp(model: Model, y0, times, method, rtol, atol):
    out = [np.array(y0, float)]
    y = np.array(y0, float)
    nfev = njev = nlu = 0
    for t0, t1 in zip(times[:-1], times[1:]):
        kw = {}
        if method in ("Radau", "BDF"):
            kw["jac"] = model.jac
        sol = solve_ivp(model.f, (t0, t1), y, method=method, rtol=rtol, atol=atol, **kw)
        if not sol.success:
            raise RuntimeError(f"{method} failed: {sol.message}")
        y = sol.y[:, -1].copy()
        out.append(y)
        nfev += sol.nfev
        nlu += getattr(sol, "nlu", 0)
        njev += getattr(sol, "njev", 0)
    return np.array(out), {"nfev": int(nfev), "njev": int(njev), "nlu": int(nlu)}


# ---------------------------------------------------------------------------
# Exact continuum solution along characteristics
# ---------------------------------------------------------------------------


def leggauss(n):
    return np.polynomial.legendre.leggauss(n)


class Characteristics:
    """Exact U(t, q, mu) of the collisionless LRS transport in physical q,
    optionally attenuated by exp(-tau) of the absorber along the ray.

    U = a^3 (E/E0)^3 U0(q0, mu0), E0 = E sqrt(A^2 (1-mu^2) + B^2 mu^2),
    mu0 = B E mu / E0, a^3 = A^2 B (A, B, a at t, relative to t = 0);
    E(t')^2 = E^2 [(A(t)/A(t'))^2 (1-mu^2) + (B(t)/B(t'))^2 mu^2].
    """

    def __init__(self, geom: Geometry, absorber=None, n_tau=24, bisect_iter=64):
        self.geom = geom
        self.absorber = absorber
        self.n_tau = n_tau
        self.bisect_iter = bisect_iter
        self.xt, self.wt = leggauss(n_tau)

    def require_monotone_redshift(self, T):
        """The absorber logic needs E(t') strictly decreasing along every ray:
        H - s > 0 and H + 2s > 0 on [0, T]."""
        ts = np.linspace(0.0, T, 2001)
        m = float(np.min(np.minimum(self.geom.H(ts) - self.geom.S(ts), self.geom.H(ts) + 2 * self.geom.S(ts))))
        if not m > 0:
            raise ValueError("absorber reference requires monotone redshift along rays")
        return m

    def mu_factors(self, t, mu):
        g = self.geom
        lnA, lnB, lna = float(g.lnA(t)), float(g.lnB(t)), float(g.ln_a(t))
        mu = np.asarray(mu, float)
        mu2 = mu * mu
        r2 = math.exp(2 * lnA) * (1.0 - mu2) + math.exp(2 * lnB) * mu2
        lnR = 0.5 * np.log(r2)
        mu0 = math.exp(lnB) * mu / np.sqrt(r2)
        ang = 1.0 + 0.25 * (3.0 * mu0 * mu0 - 1.0)
        pref = np.exp(3.0 * lna - 3.0 * lnR) * ang
        return lnR, pref

    def _W(self, t, tp, mu2):
        g = self.geom
        al = np.exp(2.0 * (float(g.lnA(t)) - g.lnA(tp)))
        be = np.exp(2.0 * (float(g.lnB(t)) - g.lnB(tp)))
        return al * (1.0 - mu2) + be * mu2

    def G_full(self, t, mu):
        mu2 = np.asarray(mu, float) ** 2
        acc = np.zeros_like(mu2)
        for x, w in zip(self.xt, self.wt):
            tp = 0.5 * t * (x + 1.0)
            acc += 0.5 * t * w * self._W(t, tp, mu2) ** -1.5
        return acc

    def _tstar(self, t, gq, mu2):
        """Bisection for E(t*) = chi on [0, t] (g(t') decreasing)."""
        lo = np.zeros_like(gq)
        hi = np.full_like(gq, t)
        for _ in range(self.bisect_iter):
            mid = 0.5 * (lo + hi)
            g = gq + 0.5 * np.log(self._W(t, mid, mu2))
            pos = g >= 0
            lo = np.where(pos, mid, lo)
            hi = np.where(pos, hi, mid)
        return 0.5 * (lo + hi)

    def _G_partial(self, t, ts, mu2):
        acc = np.zeros_like(ts)
        for x, w in zip(self.xt, self.wt):
            tp = 0.5 * ts * (x + 1.0)
            acc += 0.5 * ts * w * self._W(t, tp, mu2) ** -1.5
        return acc

    def tau(self, t, q, mu, lnR, Gf):
        a = self.absorber
        shape = np.broadcast_shapes(np.shape(q), np.shape(mu))
        gT = np.broadcast_to(np.asarray(q) - a.q_chi, shape)
        g0 = gT + lnR
        tau = np.zeros(shape)
        full = gT >= 0
        if full.any():
            gf = np.broadcast_to(Gf, shape)
            tau[full] = a.k0 * np.exp(-3.0 * gT[full]) * gf[full]
        part = (~full) & (g0 > 0)
        if part.any():
            gq = gT[part]
            mu2 = np.broadcast_to(np.asarray(mu) ** 2, shape)[part]
            ts = self._tstar(t, gq, mu2)
            tau[part] = a.k0 * np.exp(-3.0 * gq) * self._G_partial(t, ts, mu2)
        return tau

    def U(self, t, q, mu, lnR, pref, Gf=None):
        q0 = np.asarray(q) + lnR
        val = C_AMP * np.exp(-((q0 - Q_CENTER) ** 2) / Q_WIDTH2) * pref
        if self.absorber is not None and self.absorber.k0 != 0.0:
            val = val * np.exp(-self.tau(t, q, mu, lnR, Gf))
        return val

    def cell_integrals(self, t, grid: Grid, n_sub, n_gl, max_points=2_000_000):
        """Tensor Gauss-Legendre (n_gl per subcell, n_sub x n_sub subcells)
        cell integrals at time t; comoving cells map x -> q = x - ln a(t).
        With an absorber, each angular node's energy integral is recomputed
        with breakpoints at the two kinks of exp(-tau): q = q_chi (t* = t) and
        q = q_chi - ln R(mu) (t* = 0)."""
        shift = float(self.geom.ln_a(t)) if grid.coordinate == "comoving" else 0.0
        xg, wg = leggauss(n_gl)
        loc = ((np.arange(n_sub)[:, None] + (xg[None, :] + 1.0) / 2.0) / n_sub).ravel()
        wloc = np.tile(wg / (2.0 * n_sub), n_sub)
        P = loc.size
        qpts = grid.qe[:-1, None] + grid.dq[:, None] * loc[None, :]
        qw = grid.dq[:, None] * wloc[None, :]
        mpts = grid.me[:-1, None] + grid.dm[:, None] * loc[None, :]
        mw = (grid.dm[:, None] * wloc[None, :]).ravel()
        mu_flat = mpts.ravel()
        M = mu_flat.size
        lnR, pref = self.mu_factors(t, mu_flat)
        absorbing = self.absorber is not None and self.absorber.k0 != 0.0
        Gf = self.G_full(t, mu_flat) if absorbing else None
        res = np.zeros((grid.nq, grid.nm))
        chunk = max(1, int(max_points // (P * M)))
        for i0 in range(0, grid.nq, chunk):
            i1 = min(grid.nq, i0 + chunk)
            qq = (qpts[i0:i1] - shift).ravel()[:, None]
            u = self.U(t, qq, mu_flat[None, :], lnR[None, :], pref[None, :], None if Gf is None else Gf[None, :])
            wsum = (u * mw[None, :]).reshape(i1 - i0, P, grid.nm, P).sum(axis=3)
            res[i0:i1] = np.einsum("cpn,cp->cn", wsum, qw[i0:i1])
        n_corr = 0
        if absorbing:
            n_corr = self._kink_corrections(t, grid, shift, loc, wloc, mu_flat, mw, lnR, pref, Gf, P, res)
        return res, {"points": int(grid.nq * grid.nm * P * P), "kink_corrections": int(n_corr)}

    def _kink_corrections(self, t, grid, shift, loc, wloc, mu_flat, mw, lnR, pref, Gf, P, res):
        qc = self.absorber.q_chi
        M = mu_flat.size
        k1 = np.full(M, qc + shift)
        k2 = qc - lnR + shift
        def cell_of(x):
            ci = np.searchsorted(grid.qe, x, side="right") - 1
            ok = (ci >= 0) & (ci < grid.nq)
            cic = np.clip(ci, 0, grid.nq - 1)
            ok &= (grid.qe[cic] < x) & (x < grid.qe[cic + 1])
            return cic, ok
        c1, ok1 = cell_of(k1)
        c2, ok2 = cell_of(k2)
        same = ok1 & ok2 & (c1 == c2)
        m_list, c_list, a_list, b_list = [], [], [], []
        idx = np.arange(M)
        # both kinks in one cell
        m_list.append(idx[same]); c_list.append(c1[same])
        a_list.append(np.minimum(k1, k2)[same]); b_list.append(np.maximum(k1, k2)[same])
        for c, ok, k in ((c1, ok1, k1), (c2, ok2, k2)):
            sel = ok & ~same
            m_list.append(idx[sel]); c_list.append(c[sel]); a_list.append(k[sel]); b_list.append(k[sel])
        mm = np.concatenate(m_list)
        cc = np.concatenate(c_list)
        ba = np.concatenate(a_list)
        bb = np.concatenate(b_list)
        if mm.size == 0:
            return 0
        lo = grid.qe[cc]
        hi = grid.qe[cc + 1]
        segs = [(lo, ba), (ba, bb), (bb, hi)]
        qs, ws = [], []
        for a, b in segs:
            L = (b - a)[:, None]
            qs.append(a[:, None] + L * loc[None, :])
            ws.append(L * wloc[None, :])
        qs.append(lo[:, None] + (hi - lo)[:, None] * loc[None, :])
        ws.append(-(hi - lo)[:, None] * wloc[None, :])
        qall = np.concatenate(qs, axis=1) - shift
        wall = np.concatenate(ws, axis=1)
        step = max(1, 400_000 // qall.shape[1])
        for s0 in range(0, mm.size, step):
            s1 = min(mm.size, s0 + step)
            m = mm[s0:s1]
            u = self.U(t, qall[s0:s1], mu_flat[m][:, None], lnR[m][:, None], pref[m][:, None], Gf[m][:, None])
            corr = np.sum(u * wall[s0:s1], axis=1)
            np.add.at(res, (cc[s0:s1], m // P), mw[m] * corr)
        return mm.size

    def absorbed_total(self, T, n_t=32, q_span=5.0, n_q_sub=250, n_mu_sub=32, n_gl=8):
        """int_0^T dt int_{q >= q_chi} int dmu k(E) U(t, q, mu): rays above the
        threshold at t have always been above (monotone redshift), so tau is
        the full-interval integral there."""
        a = self.absorber
        xg, wg = leggauss(n_gl)
        loc = ((np.arange(n_q_sub)[:, None] + (xg[None, :] + 1) / 2) / n_q_sub).ravel()
        wq = np.tile(wg / (2 * n_q_sub), n_q_sub) * q_span
        q = a.q_chi + q_span * loc
        locm = ((np.arange(n_mu_sub)[:, None] + (xg[None, :] + 1) / 2) / n_mu_sub).ravel()
        mu = -1.0 + 2.0 * locm
        wm = np.tile(wg / (2 * n_mu_sub), n_mu_sub) * 2.0
        xt, wt = leggauss(n_t)
        total = 0.0
        for x, w in zip(xt, wt):
            t = 0.5 * T * (x + 1)
            lnR, pref = self.mu_factors(t, mu)
            Gf = self.G_full(t, mu)
            u = self.U(t, q[:, None], mu[None, :], lnR[None, :], pref[None, :], Gf[None, :])
            rate = a.k0 * np.exp(-3.0 * (q - a.q_chi))[:, None] * u
            total += 0.5 * T * w * float(wq @ rate @ wm)
        return total


def l1_rel(y, ref):
    y, ref = np.asarray(y), np.asarray(ref)
    return float(np.sum(np.abs(y - ref)) / np.sum(np.abs(ref)))


def max_rel(y, ref):
    y, ref = np.asarray(y), np.asarray(ref)
    return float(np.max(np.abs(y - ref)) / np.max(np.abs(ref)))
