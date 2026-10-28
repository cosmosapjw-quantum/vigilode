"""Independent Python reference for BASS/rei DAG node P04: LRS photon
transport with source-owned H/He photoionization, Case-A recombination,
signed stresses and dynamic shear.

Single source of truth: ``../MODEL.md``. The photon transport semantics
(first-order upwind, shared faces, outflow ledgers, the oracle's float
formulas for the cell averages ``c_j = 3 <mu^2>_j - 1`` and ``p_j = <P2>_j``,
the per-group Thomson kernel) follow the read-only oracle
``research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py``. Nothing is
copied from the Rust adapter.

Float discipline (so the RHS can be compared with another implementation to
~1e-14 relative):

* every RHS / f_t / diagnostic operation runs on Python floats (IEEE
  binary64, round to nearest, no FMA contraction);
* transcendental functions are the platform libm (``math.exp``, ``math.log``,
  ``math.expm1``, ``math.log1p``, ``x ** y`` = libm ``pow``) and
  ``numpy.hypot`` (= libm ``hypot``; CPython's ``math.hypot`` is a different
  algorithm and is NOT used);
* sums are plain left-to-right float sums in the order MODEL.md states,
  never ``math.fsum``; expressions are written in MODEL.md's literal operand
  order (Python evaluates ``a*b*c`` as ``(a*b)*c``).

Every choice MODEL.md leaves open is listed in ``REFERENCE_NOTES.md``.
Python allocation is intentional: this is a formula reference, not a port.
"""
from __future__ import annotations

import csv
import hashlib
import math
import sys
import threading
from bisect import bisect_right
from dataclasses import asdict, dataclass
from pathlib import Path

sys.dont_write_bytecode = True

import numpy as np  # noqa: E402
from scipy.integrate import solve_ivp  # noqa: E402
from scipy.interpolate import PchipInterpolator  # noqa: E402

NODE_DIR = Path(__file__).resolve().parents[1]
CASE_A_CSV = NODE_DIR / "source" / "owner_grackle_rates_129.csv"
CASE_A_SHA256 = "33396b071c840b40ad79d68ee626de59f12bb2cf0b1df2159627efd7fde42aec"
CASE_A_HEADER = (
    "T_K", "rr_HII_cm3_s", "rr_HeII_cm3_s", "rr_HeIII_cm3_s", "dr_HeII_cm3_s",
    "rr_cool_HII_erg_cm3_s", "rr_cool_HeII_erg_cm3_s", "rr_cool_HeIII_erg_cm3_s",
    "dr_cool_HeII_erg_cm3_s",
)
CASE_A_ROWS = 129
CASE_A_PREFIX = "CASE_A_T_DOMAIN"

# ---------------------------------------------------------------------------
# Constants (MODEL.md section 0, verbatim)
# ---------------------------------------------------------------------------

C = 29979245800.0                 # cm/s            source C9 C_CGS
KB = 8.617333262145e-5            # eV/K            source C10 KB_EV_K
EV_ERG = 1.602176634e-12          # erg/eV          source C11 rate_table.EV_ERG
BIND = (13.598434599702, 24.587389011, 54.41776)  # eV  HI, HeI, HeII (source C9)
SIGMA_T = 6.6524587321e-25        # cm^2            ours, CODATA
G = 6.67430e-8                    # cgs             ours, CODATA
# Shear coupling (ours), cm^3 eV^-1 s^-2; literal order 8*pi*G/(3*C*C)*EV_ERG.
K_D = 8 * math.pi * G / (3 * C * C) * EV_ERG

# Verner fit FIT[s] = (chi, E0, sigma0, ya, p, yw, y0, y1), source C9 experiment.py.
FIT = (
    (13.60, .4298, 5.475e4, 32.88, 2.963, 0., 0., 0.),
    (24.59, 13.61, 949.2, 1.469, 3.188, 2.039, .4434, 2.136),
    (54.42, 1.720, 1.369e4, 32.88, 2.963, 0., 0., 0.),
)
SPECIES = ("HI", "HeI", "HeII")
E_TOP_EV = 100.0                  # top of the photon grid (ln 100)
Y_HE = 0.2454                     # helium mass fraction of section 9
GL_POINTS = 16


class RateDomainError(ValueError):
    """Typed Case-A refusal; the message starts with ``CASE_A_T_DOMAIN:``."""


# ---------------------------------------------------------------------------
# Verner photoionization cross sections
# ---------------------------------------------------------------------------


def verner_sigma_scalar(species: int, E: float) -> float:
    """sigma_s(E) in cm^2, MODEL.md section 0 operation order.

    ``(x-1)**2`` is the correctly rounded product ``(x-1)*(x-1)`` (numpy's
    ``**2``), ``hypot`` is libm hypot (through numpy), powers are libm pow.
    """
    chi, E0, sigma0, ya, p, yw, y0, y1 = FIT[species]
    E = float(E)
    if E < chi:
        return 0.0
    x = E / E0 - y0
    y = float(np.hypot(x, y1))
    xm1 = x - 1.0
    return sigma0 * (xm1 * xm1 + yw * yw) * y ** (p / 2 - 5.5) * (1.0 + math.sqrt(y / ya)) ** (-p) * 1e-18


def verner_sigma(species: int, E):
    """Vectorized sigma_s(E) (numpy), same operation order as the scalar."""
    chi, E0, sigma0, ya, p, yw, y0, y1 = FIT[species]
    E = np.asarray(E, dtype=float)
    with np.errstate(all="ignore"):
        x = E / E0 - y0
        y = np.hypot(x, y1)
        xm1 = x - 1.0
        val = sigma0 * (xm1 * xm1 + yw * yw) * y ** (p / 2 - 5.5) * (1.0 + np.sqrt(y / ya)) ** (-p) * 1e-18
    return np.where(E < chi, 0.0, val)


def gauss_legendre(n=GL_POINTS):
    x, w = np.polynomial.legendre.leggauss(n)
    return [float(v) for v in x], [float(v) for v in w]


def cell_average_sigma(species: int, lo: float, hi: float, nodes=None, scalar=True) -> float:
    """sbar = (1/dq) * int_lo^hi sigma(exp(q)) dq by Gauss-Legendre in q.

    Map ``q_k = 0.5*(lo+hi) + (0.5*dq)*x_k``; ``S = sum_k w_k*sigma(exp(q_k))``
    with k ascending (leggauss order); ``integral = (0.5*dq)*S``;
    ``sbar = (1.0/dq)*integral``. A cell with ``hi <= ln(chi_s)`` is 0.0.
    """
    xs, ws = nodes if nodes is not None else gauss_legendre()
    dq = hi - lo
    if hi <= math.log(FIT[species][0]):
        return 0.0
    mid = 0.5 * (lo + hi)
    half = 0.5 * dq
    if scalar:
        acc = 0.0
        for xk, wk in zip(xs, ws):
            acc += wk * verner_sigma_scalar(species, math.exp(mid + half * xk))
    else:
        q = mid + half * np.asarray(xs)
        vals = verner_sigma(species, np.exp(q))
        acc = 0.0
        for wk, v in zip(ws, vals.tolist()):
            acc += wk * v
    integral = half * acc
    return (1.0 / dq) * integral


# ---------------------------------------------------------------------------
# PCHIP (MODEL.md section 0.1), independent of scipy
# ---------------------------------------------------------------------------


def _sign(v: float) -> float:
    """numpy.sign semantics for finite values: -1, 0 or +1."""
    return 1.0 if v > 0 else (-1.0 if v < 0 else 0.0)


def _edge(h0, h1, m0, m1):
    d = ((2 * h0 + h1) * m0 - h0 * m1) / (h0 + h1)
    if _sign(d) != _sign(m0):
        d = 0.0
    elif _sign(m0) != _sign(m1) and abs(d) > 3 * abs(m0):
        d = 3 * m0
    return d


class PchipManual:
    """Literal MODEL.md section 0.1 PCHIP on columns sharing the nodes ``x``.

    ``coef[col][k] = (c0, c1, c2, c3)`` for interval k; evaluation in the
    increasing-power order; derivative ``P' = c2 + 2*c1*s + 3*c0*(s*s)``.
    """

    def __init__(self, x, columns):
        self.x = [float(v) for v in x]
        m = len(self.x)
        if m < 3:
            raise ValueError("MODEL.md section 0.1 PCHIP needs at least 3 nodes")
        h = [self.x[k + 1] - self.x[k] for k in range(m - 1)]
        if any(not (hk > 0) for hk in h):
            raise ValueError("strictly increasing nodes required")
        self.h = h
        self.d = []
        self.coef = []
        for col in columns:
            y = [float(v) for v in col]
            if len(y) != m:
                raise ValueError("column length mismatch")
            mk = [(y[k + 1] - y[k]) / h[k] for k in range(m - 1)]
            d = [0.0] * m
            for k in range(1, m - 1):
                if _sign(mk[k]) != _sign(mk[k - 1]) or mk[k] == 0 or mk[k - 1] == 0:
                    d[k] = 0.0
                else:
                    w1 = 2 * h[k] + h[k - 1]
                    w2 = h[k] + 2 * h[k - 1]
                    whmean = (w1 / mk[k - 1] + w2 / mk[k]) / (w1 + w2)
                    d[k] = 1.0 / whmean
            d[0] = _edge(h[0], h[1], mk[0], mk[1])
            d[m - 1] = _edge(h[m - 2], h[m - 3], mk[m - 2], mk[m - 3])
            cs = []
            for k in range(m - 1):
                slope = (y[k + 1] - y[k]) / h[k]
                t = (d[k] + d[k + 1] - 2 * slope) / h[k]
                c0 = t / h[k]
                c1 = (slope - d[k]) / h[k] - t
                c2 = d[k]
                c3 = y[k]
                cs.append((c0, c1, c2, c3))
            self.d.append(d)
            self.coef.append(cs)

    def interval(self, X: float):
        """k with x_k <= X < x_{k+1}; X == x_{m-1} gives m-2; None outside/NaN."""
        x = self.x
        if not (x[0] <= X <= x[-1]):
            return None
        if X == x[-1]:
            return len(x) - 2
        return bisect_right(x, X) - 1

    def eval(self, X: float, deriv=False):
        """Values P (and P') of every column at X; None outside the nodes."""
        k = self.interval(X)
        if k is None:
            return None
        s = X - self.x[k]
        ss = s * s
        sss = ss * s
        vals = []
        ders = []
        for cs in self.coef:
            c0, c1, c2, c3 = cs[k]
            vals.append(c3 + c2 * s + c1 * ss + c0 * sss)
            if deriv:
                ders.append(c2 + 2 * c1 * s + 3 * c0 * ss)
        return (vals, ders) if deriv else vals


def pchip_manual(x, columns) -> PchipManual:
    return PchipManual(x, columns)


# ---------------------------------------------------------------------------
# Case-A table
# ---------------------------------------------------------------------------


def sha256_file(path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class CaseA:
    """Case-A rates from the source table: ``v = exp(pchip(ln T))``.

    ``backend='scipy'``: the source's ``PchipInterpolator(log T, log values,
    axis=0, extrapolate=False)`` (P' from its ``derivative()``);
    ``backend='manual'``: ``PchipManual``. Domain ``T_min <= T <= T_max``
    inclusive; anything else (or non-finite T) raises ``RateDomainError``.
    """

    COLUMNS = CASE_A_HEADER[1:]

    def __init__(self, path=CASE_A_CSV, verify_sha=True):
        path = Path(path)
        if verify_sha and sha256_file(path) != CASE_A_SHA256:
            raise ValueError(f"Case-A table sha256 mismatch: {path}")
        with path.open(newline="") as fh:
            rows = list(csv.reader(fh))
        if tuple(rows[0]) != CASE_A_HEADER:
            raise ValueError("Case-A header mismatch")
        data = [[float(v) for v in r] for r in rows[1:]]
        if len(data) != CASE_A_ROWS or any(len(r) != len(CASE_A_HEADER) for r in data):
            raise ValueError("Case-A table must have 129 rows of 9 floats")
        self.T = [r[0] for r in data]
        self.values = [[r[c] for r in data] for c in range(1, len(CASE_A_HEADER))]  # column-major
        if any(not (v > 0) or not math.isfinite(v) for col in [self.T] + self.values for v in col):
            raise ValueError("Case-A entries must be finite and positive")
        self.T_min = self.T[0]
        self.T_max = self.T[-1]
        arr = np.array(data)
        # Source semantics: numpy log of the table (equal to math.log here, tested).
        self.x_np = np.log(arr[:, 0])
        self.y_np = np.log(arr[:, 1:])
        self.x = [math.log(v) for v in self.T]
        self.ylog = [[math.log(v) for v in col] for col in self.values]
        self.pp = PchipInterpolator(self.x_np, self.y_np, axis=0, extrapolate=False)
        self.dpp = self.pp.derivative()
        self.manual = PchipManual(self.x, self.ylog)

    def check(self, T: float) -> None:
        if not math.isfinite(T) or T < self.T_min or T > self.T_max:
            raise RateDomainError(
                f"{CASE_A_PREFIX}: T={T!r} K outside [{self.T_min!r}, {self.T_max!r}] K")

    def log_values(self, T: float, backend="scipy", deriv=False):
        """(P_c) or (P_c, P'_c) for the 8 columns at X = ln T."""
        self.check(T)
        X = math.log(T)
        if backend == "scipy":
            P = [float(v) for v in self.pp(X)]
            if not deriv:
                return P
            return P, [float(v) for v in self.dpp(X)]
        if backend == "manual":
            return self.manual.eval(X, deriv=deriv)
        raise ValueError(f"unknown backend {backend!r}")

    def rates(self, T: float, backend="scipy"):
        """(rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII, cdrHeII)."""
        return [math.exp(p) for p in self.log_values(T, backend)]

    def rates_and_dT(self, T: float, backend="scipy"):
        """Values v and dv/dT = v*P'/T."""
        P, dP = self.log_values(T, backend, deriv=True)
        v = [math.exp(p) for p in P]
        return v, [vi * dpi / T for vi, dpi in zip(v, dP)]


# ---------------------------------------------------------------------------
# Grid and state layout
# ---------------------------------------------------------------------------


class Grid:
    """Aligned energy faces (three uniform segments in q = ln(E/1 eV)) and
    uniform mu cells, with the P02 photon layout plus the 12 matter slots."""

    def __init__(self, n1: int, n2: int, n3: int, nm: int):
        if min(n1, n2, n3, nm) < 1:
            raise ValueError("every segment and the angle grid need >= 1 cell")
        bounds = [math.log(FIT[0][0]), math.log(FIT[1][0]), math.log(FIT[2][0]), math.log(E_TOP_EV)]
        q = []
        for seg, ncell in enumerate((n1, n2, n3)):
            lo, hi = bounds[seg], bounds[seg + 1]
            q.append(lo)
            for k in range(1, ncell):
                q.append(lo + (hi - lo) * k / ncell)
        q.append(bounds[3])
        mu = [-1.0 + 2.0 * k / nm for k in range(nm + 1)]
        mu[0], mu[-1] = -1.0, 1.0
        self.segments = (n1, n2, n3)
        self.q_edges = q
        self.mu_edges = mu
        self.dq = [b - a for a, b in zip(q, q[1:])]
        self.dm = [b - a for a, b in zip(mu, mu[1:])]
        if min(self.dq) <= 0 or min(self.dm) <= 0:
            raise ValueError("strictly positive widths required")
        self.nq = len(self.dq)
        self.nm = len(self.dm)
        # Oracle float formulas (libm pow for **3).
        self.c = [3 * ((b ** 3 - a ** 3) / (3 * d)) - 1 for a, b, d in zip(mu, mu[1:], self.dm)]
        self.p = [((b ** 3 - b) - (a ** 3 - a)) / (2 * d) for a, b, d in zip(mu, mu[1:], self.dm)]
        self.Ebar = [math.exp(lo) * math.expm1(d) / d for lo, d in zip(q, self.dq)]
        # Layout.
        nq, nm = self.nq, self.nm
        self.n_ph = 2 + nq * (nm + 1)
        self.n = self.n_ph + 12
        self.LO = 0
        self.HI = self.n_ph - 1
        (self.IW, self.IHI, self.IHII, self.IHeI, self.IHeII, self.IHeIII, self.IU,
         self.IDm, self.IWm, self.IEesc, self.IRe, self.IS) = range(self.n_ph, self.n_ph + 12)
        self.cells = [[1 + i * (nm + 1) + j for j in range(nm)] for i in range(nq)]
        self.absorbed = [1 + i * (nm + 1) + nm for i in range(nq)]
        self.reactant = (self.IHI, self.IHeI, self.IHeII)
        self.product = (self.IHII, self.IHeII, self.IHeIII)
        # Photon energy weights: Ebar_i for cells, Ebar_0 / Ebar_{nq-1} for L_lo / L_hi, 0 for A_i.
        w = [0.0] * self.n_ph
        for i in range(nq):
            for j in range(nm):
                w[self.cells[i][j]] = self.Ebar[i]
        w[self.LO] = self.Ebar[0]
        w[self.HI] = self.Ebar[-1]
        self.photon_energy_weights = w
        nodes = gauss_legendre()
        self.sbar = [[cell_average_sigma(s, q[i], q[i + 1], nodes) for i in range(nq)] for s in range(3)]

    def cell(self, i, j):
        return self.cells[i][j]

    def index_names(self):
        names = ["L_lo"]
        for i in range(self.nq):
            names += [f"N_{i}_{j}" for j in range(self.nm)] + [f"A_{i}"]
        names += ["L_hi", "W_gamma", "N_HI", "N_HII", "N_HeI", "N_HeII", "N_HeIII", "U", "D_m",
                  "W_m", "E_esc", "R_e", "s"]
        return names

    def describe(self):
        return {"segments": list(self.segments), "nq": self.nq, "nm": self.nm, "n_ph": self.n_ph,
                "n": self.n, "q_edges": self.q_edges, "mu_edges": self.mu_edges,
                "Ebar": self.Ebar, "c": self.c, "p": self.p, "sbar": self.sbar}


# ---------------------------------------------------------------------------
# Geometry (MODEL.md section 1)
# ---------------------------------------------------------------------------


class Geometry:
    """Matter-dominated prescribed background, ``t0 = 2/(3*H0)``."""

    def __init__(self, H0: float):
        if not (math.isfinite(H0) and H0 > 0):
            raise ValueError("H0 must be finite and > 0")
        self.H0 = float(H0)
        self.t0 = 2 / (3 * self.H0)

    def H(self, t):
        return (2 / 3) / (self.t0 + t)

    def a(self, t):
        return (1 + t / self.t0) ** (2 / 3)

    def ln_a(self, t):
        return (2 / 3) * math.log1p(t / self.t0)

    def a_m3(self, t):
        return (1 + t / self.t0) ** -2

    def dH(self, t):
        H = self.H(t)
        return -1.5 * H * H

    def da_m3(self, t):
        return -3 * self.H(t) * self.a_m3(t)


# ---------------------------------------------------------------------------
# Scenario
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Params:
    nH0: float
    H0: float
    tau_m: float
    amp: float = 1.0
    xHI0: float = 0.6
    fHeI: float = 0.2
    fHeII: float = 0.5
    T0: float = 12000.0
    Dm0: float = 0.0
    s0_over_H0: float = 0.1
    photons_per_H: float = 0.05
    alpha: float = 1.5
    a2: float = 0.5


TEST_PARAMS = Params(nH0=2.4e-5, H0=3e-17, tau_m=1e14, amp=1.0, xHI0=0.6, fHeI=0.2, fHeII=0.5,
                     T0=12000.0, Dm0=0.0, s0_over_H0=0.1, photons_per_H=0.05, alpha=1.5, a2=0.5)
TEST_GRID = (4, 4, 4, 4)


class Model:
    """The P04 semi-discrete system: ``rhs``, ``jacobian``, ``partial_t``,
    ``initial_state``, diagnostics and the five linear invariants."""

    def __init__(self, params: Params, grid: Grid, case_a: CaseA | None = None, rate_backend="scipy"):
        if not (math.isfinite(params.tau_m) and params.tau_m > 0):
            raise ValueError("tau_m must be finite and > 0")
        self.params = params
        self.grid = grid
        self.geom = Geometry(params.H0)
        self.case_a = case_a if case_a is not None else CaseA()
        self.rate_backend = rate_backend
        self.n = grid.n
        self._lock = threading.Lock()
        self._domain_refusals = 0

    # -- refusal -----------------------------------------------------------

    @property
    def domain_refusals(self) -> int:
        return self._domain_refusals

    def _refuse(self, err: RateDomainError):
        with self._lock:
            self._domain_refusals += 1
        raise err

    def _state(self, y):
        yl = y.tolist() if isinstance(y, np.ndarray) else [float(v) for v in y]
        if len(yl) != self.n:
            raise ValueError(f"state dimension {len(yl)} != {self.n}")
        return yl

    def temperature(self, y) -> float:
        g = self.grid
        yl = self._state(y)
        return self._temperature(yl[g.IHI], yl[g.IHII], yl[g.IHeI], yl[g.IHeII], yl[g.IHeIII], yl[g.IU])

    @staticmethod
    def _temperature(NHI, NHII, NHeI, NHeII, NHeIII, U):
        Ne = NHII + NHeII + 2 * NHeIII
        NH = NHI + NHII
        NHe = NHeI + NHeII + NHeIII
        return U / (1.5 * KB * (NH + NHe + Ne))

    def _checked_temperature(self, T):
        try:
            self.case_a.check(T)
        except RateDomainError as err:
            self._refuse(err)
        return T

    # -- transport (the P02 operator, oracle semantics) ---------------------

    def _transport(self, h, s, vh, vs, y, add):
        """Upwind direction from (h, s), flux velocity from (vh, vs);
        accumulates through ``add(index, value)`` into the photon slots."""
        g = self.grid
        nq, nm = g.nq, g.nm
        cells, dq, dm, c, mu_e = g.cells, g.dq, g.dm, g.c, g.mu_edges
        LO, HI = g.LO, g.HI
        for j in range(nm):
            cj = c[j]
            select = -h - s * cj
            velocity = -vh - vs * cj
            for k in range(nq + 1):
                up = k - 1 if select >= 0 else k
                if 0 <= up < nq:
                    val = y[cells[up][j]] / (dq[up] * dm[j])
                else:
                    val = 0.0
                flux = velocity * val * dm[j]
                add(cells[k - 1][j] if k > 0 else LO, -flux)
                add(cells[k][j] if k < nq else HI, flux)
        for i in range(nq):
            for k in range(1, nm):
                mu = mu_e[k]
                select = -3 * s * mu * (1 - mu * mu)
                velocity = -3 * vs * mu * (1 - mu * mu)
                up = k - 1 if select >= 0 else k
                flux = velocity * (y[cells[i][up]] / (dq[i] * dm[up])) * dq[i]
                add(cells[i][k - 1], -flux)
                add(cells[i][k], flux)

    def _photon_work(self, out):
        """-(sum_i sum_j Ebar_i*out[cell] + Ebar_0*out[L_lo] + Ebar_{nq-1}*out[L_hi])."""
        g = self.grid
        acc = 0.0
        for i in range(g.nq):
            Ei = g.Ebar[i]
            for j in range(g.nm):
                acc += Ei * out[g.cells[i][j]]
        acc += g.Ebar[0] * out[g.LO]
        acc += g.Ebar[-1] * out[g.HI]
        return -acc

    def _photon_work_abs(self, out):
        g = self.grid
        acc = 0.0
        for i in range(g.nq):
            for j in range(g.nm):
                acc += abs(g.Ebar[i] * out[g.cells[i][j]])
        return acc + abs(g.Ebar[0] * out[g.LO]) + abs(g.Ebar[-1] * out[g.HI])

    # -- moments, ledgers ----------------------------------------------------

    def photon_moments(self, y):
        """(n_gamma, E_gamma, D_gamma), i ascending then j ascending."""
        g = self.grid
        yl = self._state(y)
        n_g = 0.0
        E_g = 0.0
        for i in range(g.nq):
            for j in range(g.nm):
                N = yl[g.cells[i][j]]
                n_g += N
                E_g += g.Ebar[i] * N
        return n_g, E_g, self._D_gamma(yl)

    def _D_gamma(self, yl):
        g = self.grid
        acc = 0.0
        for i in range(g.nq):
            inner = 0.0
            for j in range(g.nm):
                inner += g.p[j] * yl[g.cells[i][j]]
            acc += g.Ebar[i] * inner
        return acc

    def binding(self, y):
        g = self.grid
        yl = self._state(y)
        return BIND[0] * yl[g.IHII] + BIND[1] * (yl[g.IHeII] + yl[g.IHeIII]) + BIND[2] * yl[g.IHeIII]

    def ledgers(self, y):
        """The five invariants of MODEL.md section 7 (left-to-right sums)."""
        g = self.grid
        yl = self._state(y)
        n_g, E_g, _ = self.photon_moments(yl)
        Ne = yl[g.IHII] + yl[g.IHeII] + 2 * yl[g.IHeIII]
        sumA = 0.0
        for i in range(g.nq):
            sumA += yl[g.absorbed[i]]
        B = self.binding(yl)
        return {
            "number_ledger": n_g + yl[g.LO] + yl[g.HI] + Ne + yl[g.IRe],
            "H_ledger": yl[g.IHI] + yl[g.IHII],
            "He_ledger": yl[g.IHeI] + yl[g.IHeII] + yl[g.IHeIII],
            "photon_ledger": n_g + yl[g.LO] + yl[g.HI] + sumA,
            "energy_ledger": (E_g + g.Ebar[0] * yl[g.LO] + g.Ebar[-1] * yl[g.HI] + yl[g.IW] + yl[g.IU]
                              + B + yl[g.IEesc] + yl[g.IWm]),
        }

    def invariant_weights(self):
        """Constant weight vectors w with ledger = w . y."""
        g = self.grid
        n = self.n
        cells = [c for row in g.cells for c in row]
        num = np.zeros(n)
        num[cells] = 1.0
        num[[g.LO, g.HI, g.IHII, g.IHeII, g.IRe]] = 1.0
        num[g.IHeIII] = 2.0
        hyd = np.zeros(n)
        hyd[[g.IHI, g.IHII]] = 1.0
        hel = np.zeros(n)
        hel[[g.IHeI, g.IHeII, g.IHeIII]] = 1.0
        pho = np.zeros(n)
        pho[cells] = 1.0
        pho[[g.LO, g.HI] + g.absorbed] = 1.0
        ene = np.zeros(n)
        ene[: g.n_ph] = g.photon_energy_weights
        ene[[g.IW, g.IU, g.IEesc, g.IWm]] = 1.0
        ene[g.IHII] = BIND[0]
        ene[g.IHeII] = BIND[1]
        ene[g.IHeIII] = BIND[1] + BIND[2]
        return {"number_ledger": num, "H_ledger": hyd, "He_ledger": hel, "photon_ledger": pho,
                "energy_ledger": ene}

    def diagnostics(self, t, y):
        g = self.grid
        yl = self._state(y)
        n_g, E_g, D_g = self.photon_moments(yl)
        NH = yl[g.IHI] + yl[g.IHII]
        NHe = yl[g.IHeI] + yl[g.IHeII] + yl[g.IHeIII]
        out = {"t": float(t), "n_gamma": n_g, "E_gamma": E_g, "D_gamma": D_g,
               "T": self.temperature(yl), "x_HII": yl[g.IHII] / NH, "x_HeII": yl[g.IHeII] / NHe,
               "x_HeIII": yl[g.IHeIII] / NHe, "s_over_H": yl[g.IS] / self.geom.H(t)}
        out.update(self.ledgers(yl))
        return out

    # -- initial data (MODEL.md section 9) ------------------------------------

    def initial_state(self):
        p = self.params
        g = self.grid
        y = [0.0] * self.n
        nH0 = p.nH0
        nHe = nH0 * Y_HE / (4 * (1 - Y_HE))
        NHI = p.xHI0 * nH0
        NHII = (1 - p.xHI0) * nH0
        NHeI = p.fHeI * nHe
        NHeII = p.fHeII * nHe
        NHeIII = nHe - NHeI - NHeII
        Ne = NHII + NHeII + 2 * NHeIII
        y[g.IHI], y[g.IHII], y[g.IHeI], y[g.IHeII], y[g.IHeIII] = NHI, NHII, NHeI, NHeII, NHeIII
        y[g.IU] = 1.5 * KB * p.T0 * (nH0 + nHe + Ne)
        y[g.IDm] = p.Dm0
        y[g.IS] = p.s0_over_H0 * p.H0
        total = p.photons_per_H * nH0
        w = [[g.Ebar[i] ** (-p.alpha) * g.dq[i] * (g.dm[j] / 2) * (1 + p.a2 * g.p[j]) for j in range(g.nm)]
             for i in range(g.nq)]
        W = 0.0
        for i in range(g.nq):
            for j in range(g.nm):
                W += w[i][j]
        for i in range(g.nq):
            for j in range(g.nm):
                y[g.cells[i][j]] = total * w[i][j] / W
        return np.array(y)

    def scale_vector(self, y0):
        """Per-component magnitude used as ``atol = 1e-14*scale``: nH0 for
        every number-like slot, |U0| for every energy-like slot and D_m,
        |s0| (H0 if s0 == 0) for s."""
        g = self.grid
        p = self.params
        y0 = np.asarray(y0, float)
        nsc = p.nH0
        esc = abs(float(y0[g.IU]))
        sc = np.full(self.n, nsc)
        sc[[g.IW, g.IU, g.IDm, g.IWm, g.IEesc]] = esc
        s0 = abs(float(y0[g.IS]))
        sc[g.IS] = s0 if s0 > 0 else p.H0
        return sc

    # -- RHS ----------------------------------------------------------------

    def _eval(self, t, y, ab=None):
        g = self.grid
        p = self.params
        yl = self._state(y)
        out = [0.0] * self.n
        if ab is None:
            def add(k, v):
                out[k] += v
        else:
            def add(k, v):
                out[k] += v
                ab[k] += abs(v)
        t = float(t)
        NHI, NHII, NHeI, NHeII, NHeIII = (yl[g.IHI], yl[g.IHII], yl[g.IHeI], yl[g.IHeII], yl[g.IHeIII])
        U, Dm, s = yl[g.IU], yl[g.IDm], yl[g.IS]
        Ne = NHII + NHeII + 2 * NHeIII
        T = self._checked_temperature(self._temperature(NHI, NHII, NHeI, NHeII, NHeIII, U))
        rr = self.case_a.rates(T, self.rate_backend)
        H = self.geom.H(t)
        a_m3 = self.geom.a_m3(t)

        # Section 2: transport, then the discrete photon work, then the moments.
        self._transport(H, s, H, s, yl, add)
        out[g.IW] = self._photon_work(out)
        if ab is not None:
            ab[g.IW] = self._photon_work_abs(out)
        D_gamma = self._D_gamma(yl)

        # Section 3: Thomson, per group.
        Gam = C * SIGMA_T * (Ne * a_m3)
        for i in range(g.nq):
            row = g.cells[i]
            M0 = 0.0
            M2 = 0.0
            for j in range(g.nm):
                N = yl[row[j]]
                M0 += N
                M2 += g.p[j] * N
            for j in range(g.nm):
                add(row[j], Gam * (g.dm[j] / 2 * (M0 + 0.5 * g.p[j] * M2) - yl[row[j]]))

        # Section 4: photoionization, s outer, then i, then j.
        IU = g.IU
        for sp in range(3):
            Is, Ip = g.reactant[sp], g.product[sp]
            Ns = yl[Is]
            for i in range(g.nq):
                k = C * g.sbar[sp][i] * (Ns * a_m3)
                heat = g.Ebar[i] - BIND[sp]
                Ai = g.absorbed[i]
                for j in range(g.nm):
                    ci = g.cells[i][j]
                    R = k * yl[ci]
                    add(ci, -R)
                    add(Ai, R)
                    add(Is, -R)
                    add(Ip, R)
                    add(IU, heat * R)

        # Section 5: Case-A recombination and cooling.
        rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII, cdrHeII = rr
        ne_p = Ne * a_m3
        rH = rrHII * ne_p * NHII
        r2 = (rrHeII + drHeII) * ne_p * NHeII
        r3 = rrHeIII * ne_p * NHeIII
        add(g.IHII, -rH)
        add(g.IHI, rH)
        add(g.IHeII, -r2)
        add(g.IHeI, r2)
        add(g.IHeIII, -r3)
        add(g.IHeII, r3)
        Lam = ne_p * (NHII * cHII + NHeII * cHeII + NHeIII * cHeIII + NHeII * cdrHeII) / EV_ERG
        add(IU, -Lam)
        add(g.IEesc, BIND[0] * rH + BIND[1] * r2 + BIND[2] * r3 + Lam)
        add(g.IRe, rH + r2 + r3)

        # Section 6: matter stress, gas energy and work, shear.
        add(g.IDm, -2 * H * Dm - Dm / p.tau_m - 4 * s * U)
        a1 = 2 * H * U
        a2 = 2 * s * Dm
        add(IU, -a1 - a2)
        add(g.IWm, a1 + a2)
        out[g.IS] = -3 * H * s + K_D * p.amp * (D_gamma + Dm) * a_m3
        if ab is not None:
            ab[g.IS] = abs(-3 * H * s) + abs(K_D * p.amp * (D_gamma + Dm) * a_m3)
        return out

    def rhs(self, t, y):
        """f(t, y) (numpy array); raises RateDomainError (counted) outside the table."""
        return np.array(self._eval(t, y))

    def rhs_with_abs(self, t, y):
        """(f, a): a[k] is the sum of |every term| accumulated into slot k."""
        ab = [0.0] * self.n
        out = self._eval(t, y, ab)
        return np.array(out), np.array(ab)

    def photoionization_events(self, t, y):
        """Per channel s: list of (i, j, R, Ebar_i) with the RHS's identical float R."""
        g = self.grid
        yl = self._state(y)
        a_m3 = self.geom.a_m3(float(t))
        res = []
        for sp in range(3):
            Ns = yl[g.reactant[sp]]
            ev = []
            for i in range(g.nq):
                k = C * g.sbar[sp][i] * (Ns * a_m3)
                for j in range(g.nm):
                    ev.append((i, j, k * yl[g.cells[i][j]], g.Ebar[i]))
            res.append(ev)
        return res

    # -- Jacobian -------------------------------------------------------------

    def _transport_jacobian(self, h, s, Jt):
        """d(transport)/dN at frozen upwind (h, s); the coefficient is the RHS
        flux of a unit content, so each column equals transport(e_k)."""
        g = self.grid
        nq, nm = g.nq, g.nm
        cells, dq, dm, c, mu_e = g.cells, g.dq, g.dm, g.c, g.mu_edges
        for j in range(nm):
            vel = -h - s * c[j]
            for k in range(nq + 1):
                up = k - 1 if vel >= 0 else k
                if not (0 <= up < nq):
                    continue
                coef = vel * (1.0 / (dq[up] * dm[j])) * dm[j]
                col = cells[up][j]
                Jt[cells[k - 1][j] if k > 0 else g.LO, col] -= coef
                Jt[cells[k][j] if k < nq else g.HI, col] += coef
        for i in range(nq):
            for k in range(1, nm):
                mu = mu_e[k]
                vel = -3 * s * mu * (1 - mu * mu)
                up = k - 1 if vel >= 0 else k
                coef = vel * (1.0 / (dq[i] * dm[up])) * dq[i]
                col = cells[i][up]
                Jt[cells[i][k - 1], col] -= coef
                Jt[cells[i][k], col] += coef

    def jacobian(self, t, y):
        """Analytic dense df/dy (numpy, n x n)."""
        g = self.grid
        p = self.params
        yl = self._state(y)
        n = self.n
        t = float(t)
        NHI, NHII, NHeI, NHeII, NHeIII = (yl[g.IHI], yl[g.IHII], yl[g.IHeI], yl[g.IHeII], yl[g.IHeIII])
        U, Dm, s = yl[g.IU], yl[g.IDm], yl[g.IS]
        Ne = NHII + NHeII + 2 * NHeIII
        NH = NHI + NHII
        NHe = NHeI + NHeII + NHeIII
        Ntot = NH + NHe + Ne
        T = self._checked_temperature(U / (1.5 * KB * Ntot))
        v, dv = self.case_a.rates_and_dT(T, self.rate_backend)
        H = self.geom.H(t)
        a_m3 = self.geom.a_m3(t)
        J = np.zeros((n, n))

        # Transport block, its s column, and the W_gamma row.
        Jt = np.zeros((g.n_ph, n))
        self._transport_jacobian(H, s, Jt)
        ds_col = [0.0] * n
        def add_s(k, val):
            ds_col[k] += val
        self._transport(H, s, 0.0, 1.0, yl, add_s)
        Jt[:, g.IS] = ds_col[: g.n_ph]
        J[: g.n_ph] += Jt
        J[g.IW] = -(np.asarray(g.photon_energy_weights) @ Jt)

        # Thomson.
        gN = C * SIGMA_T * a_m3          # dGam/dN_e
        Gam = C * SIGMA_T * (Ne * a_m3)
        for i in range(g.nq):
            row = g.cells[i]
            M0 = 0.0
            M2 = 0.0
            for j in range(g.nm):
                N = yl[row[j]]
                M0 += N
                M2 += g.p[j] * N
            for j in range(g.nm):
                br = g.dm[j] / 2 * (M0 + 0.5 * g.p[j] * M2) - yl[row[j]]
                J[row[j], g.IHII] += gN * br
                J[row[j], g.IHeII] += gN * br
                J[row[j], g.IHeIII] += 2 * gN * br
                for jj in range(g.nm):
                    J[row[j], row[jj]] += Gam * (g.dm[j] / 2 * (1 + 0.5 * g.p[j] * g.p[jj]) - (1.0 if j == jj else 0.0))

        # Photoionization.
        for sp in range(3):
            Is, Ip = g.reactant[sp], g.product[sp]
            Ns = yl[Is]
            for i in range(g.nq):
                k = C * g.sbar[sp][i] * (Ns * a_m3)
                dk = C * g.sbar[sp][i] * a_m3
                heat = g.Ebar[i] - BIND[sp]
                Ai = g.absorbed[i]
                for j in range(g.nm):
                    ci = g.cells[i][j]
                    dR = dk * yl[ci]
                    for r, wgt in ((ci, -1.0), (Ai, 1.0), (Is, -1.0), (Ip, 1.0), (g.IU, heat)):
                        J[r, ci] += wgt * k
                        J[r, Is] += wgt * dR

        # Recombination and cooling, through ne_p, the species and T(U, N).
        rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII, cdrHeII = v
        d_rrHII, d_rrHeII, d_rrHeIII, d_drHeII, d_cHII, d_cHeII, d_cHeIII, d_cdrHeII = dv
        ne_p = Ne * a_m3
        al2 = rrHeII + drHeII
        d_al2 = d_rrHeII + d_drHeII
        S = NHII * cHII + NHeII * cHeII + NHeIII * cHeIII + NHeII * cdrHeII
        dS_dT = NHII * d_cHII + NHeII * d_cHeII + NHeIII * d_cHeIII + NHeII * d_cdrHeII
        dT = {g.IU: 1.0 / (1.5 * KB * Ntot)}
        for idx, nu in ((g.IHI, 1), (g.IHII, 2), (g.IHeI, 1), (g.IHeII, 2), (g.IHeIII, 3)):
            dT[idx] = -T / Ntot * nu
        dne = {g.IHII: a_m3, g.IHeII: a_m3, g.IHeIII: 2 * a_m3}
        dS_dN = {g.IHII: cHII, g.IHeII: cHeII + cdrHeII, g.IHeIII: cHeIII}
        for X in (g.IHI, g.IHII, g.IHeI, g.IHeII, g.IHeIII, g.IU):
            dTX = dT[X]
            dneX = dne.get(X, 0.0)
            drH = d_rrHII * dTX * ne_p * NHII + rrHII * dneX * NHII + (rrHII * ne_p if X == g.IHII else 0.0)
            dr2 = d_al2 * dTX * ne_p * NHeII + al2 * dneX * NHeII + (al2 * ne_p if X == g.IHeII else 0.0)
            dr3 = d_rrHeIII * dTX * ne_p * NHeIII + rrHeIII * dneX * NHeIII + (rrHeIII * ne_p if X == g.IHeIII else 0.0)
            dLam = (dneX * S + ne_p * (dS_dT * dTX + dS_dN.get(X, 0.0))) / EV_ERG
            J[g.IHII, X] -= drH
            J[g.IHI, X] += drH
            J[g.IHeII, X] += -dr2 + dr3
            J[g.IHeI, X] += dr2
            J[g.IHeIII, X] -= dr3
            J[g.IU, X] -= dLam
            J[g.IEesc, X] += BIND[0] * drH + BIND[1] * dr2 + BIND[2] * dr3 + dLam
            J[g.IRe, X] += drH + dr2 + dr3

        # Matter rows.
        J[g.IDm, g.IDm] += -2 * H - 1 / p.tau_m
        J[g.IDm, g.IS] += -4 * U
        J[g.IDm, g.IU] += -4 * s
        J[g.IU, g.IU] += -2 * H
        J[g.IU, g.IS] += -2 * Dm
        J[g.IU, g.IDm] += -2 * s
        J[g.IWm, g.IU] += 2 * H
        J[g.IWm, g.IS] += 2 * Dm
        J[g.IWm, g.IDm] += 2 * s
        KDa = K_D * p.amp * a_m3
        J[g.IS, g.IS] += -3 * H
        J[g.IS, g.IDm] += KDa
        for i in range(g.nq):
            for j in range(g.nm):
                J[g.IS, g.cells[i][j]] += KDa * g.Ebar[i] * g.p[j]
        return J

    # -- f_t ------------------------------------------------------------------

    def partial_t(self, t, y):
        """df/dt at fixed y: every term containing H(t) or a^-3(t)."""
        g = self.grid
        p = self.params
        yl = self._state(y)
        ft = [0.0] * self.n

        def add(k, val):
            ft[k] += val
        t = float(t)
        NHI, NHII, NHeI, NHeII, NHeIII = (yl[g.IHI], yl[g.IHII], yl[g.IHeI], yl[g.IHeII], yl[g.IHeIII])
        U, Dm, s = yl[g.IU], yl[g.IDm], yl[g.IS]
        Ne = NHII + NHeII + 2 * NHeIII
        T = self._checked_temperature(self._temperature(NHI, NHII, NHeI, NHeII, NHeIII, U))
        rr = self.case_a.rates(T, self.rate_backend)
        H = self.geom.H(t)
        dH = self.geom.dH(t)
        da = self.geom.da_m3(t)

        self._transport(H, s, dH, 0.0, yl, add)
        ft[g.IW] = self._photon_work(ft)
        D_gamma = self._D_gamma(yl)

        dGam = C * SIGMA_T * (Ne * da)
        for i in range(g.nq):
            row = g.cells[i]
            M0 = 0.0
            M2 = 0.0
            for j in range(g.nm):
                N = yl[row[j]]
                M0 += N
                M2 += g.p[j] * N
            for j in range(g.nm):
                add(row[j], dGam * (g.dm[j] / 2 * (M0 + 0.5 * g.p[j] * M2) - yl[row[j]]))

        for sp in range(3):
            Is, Ip = g.reactant[sp], g.product[sp]
            Ns = yl[Is]
            for i in range(g.nq):
                dk = C * g.sbar[sp][i] * (Ns * da)
                heat = g.Ebar[i] - BIND[sp]
                Ai = g.absorbed[i]
                for j in range(g.nm):
                    ci = g.cells[i][j]
                    dR = dk * yl[ci]
                    add(ci, -dR)
                    add(Ai, dR)
                    add(Is, -dR)
                    add(Ip, dR)
                    add(g.IU, heat * dR)

        rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII, cdrHeII = rr
        dne = Ne * da
        drH = rrHII * dne * NHII
        dr2 = (rrHeII + drHeII) * dne * NHeII
        dr3 = rrHeIII * dne * NHeIII
        add(g.IHII, -drH)
        add(g.IHI, drH)
        add(g.IHeII, -dr2)
        add(g.IHeI, dr2)
        add(g.IHeIII, -dr3)
        add(g.IHeII, dr3)
        dLam = dne * (NHII * cHII + NHeII * cHeII + NHeIII * cHeIII + NHeII * cdrHeII) / EV_ERG
        add(g.IU, -dLam)
        add(g.IEesc, BIND[0] * drH + BIND[1] * dr2 + BIND[2] * dr3 + dLam)
        add(g.IRe, drH + dr2 + dr3)

        add(g.IDm, -2 * dH * Dm)
        add(g.IU, -2 * dH * U)
        add(g.IWm, 2 * dH * U)
        # The K_D term contains a^-3(t): included per the governing sentence of
        # MODEL.md section 8 (its bullet list omits it; see REFERENCE_NOTES.md).
        ft[g.IS] = -3 * dH * s + K_D * p.amp * (D_gamma + Dm) * da
        return np.array(ft)


def make_model(params: Params = TEST_PARAMS, grid=TEST_GRID, case_a=None, rate_backend="scipy") -> Model:
    if not isinstance(grid, Grid):
        grid = Grid(*grid)
    return Model(params, grid, case_a, rate_backend)


# ---------------------------------------------------------------------------
# Finite-difference helpers (tests)
# ---------------------------------------------------------------------------


def _richardson(Dh, h, Dh2, h2):
    """Combine central differences with error c*h^2 (steps h, h2)."""
    return (Dh2 * (h * h) - Dh * (h2 * h2)) / (h * h - h2 * h2)


def finite_difference_jacobian(fun, t, y, steps):
    """Central differences with one Richardson step (h and h/2) per column;
    ``steps[k] > 0``. The realised steps (y +- h) - y are used."""
    y = np.array(y, dtype=float)
    n = y.size
    cols = []
    for k in range(n):
        Ds = []
        for h in (float(steps[k]), 0.5 * float(steps[k])):
            yp = y.copy()
            ym = y.copy()
            yp[k] = y[k] + h
            ym[k] = y[k] - h
            width = (yp[k] - y[k]) + (y[k] - ym[k])
            Ds.append(((np.asarray(fun(t, yp)) - np.asarray(fun(t, ym))) / width, 0.5 * width))
        (D1, h1), (D2, h2) = Ds
        cols.append(_richardson(D1, h1, D2, h2))
    return np.array(cols).T


def finite_difference_partial_t(fun, t, y, dt):
    """Central difference in t with one Richardson step (dt and dt/2)."""
    Ds = []
    for h in (float(dt), 0.5 * float(dt)):
        tp, tm = t + h, t - h
        width = (tp - t) + (t - tm)
        Ds.append(((np.asarray(fun(tp, y)) - np.asarray(fun(tm, y))) / width, 0.5 * width))
    (D1, h1), (D2, h2) = Ds
    return _richardson(D1, h1, D2, h2)


# ---------------------------------------------------------------------------
# Radau reference integration
# ---------------------------------------------------------------------------


class _JacobianRefusal(Exception):
    pass


def integrate_radau(t_span, y0, t_eval, rtol, atol_vector, params, grid=None, case_a=None):
    """scipy Radau (analytic dense Jacobian, no dense output) on the P04 model.

    ``params`` is a ``Model`` or a ``Params`` (then ``grid`` or TEST_GRID).
    A CASE_A_T_DOMAIN refusal in the RHS returns a NaN vector, which Radau's
    Newton loop treats as a failed step (step halving, the driver's
    NonFinite-as-rejection); one in the Jacobian aborts the solve. A run that
    ends unsuccessfully after refusals carries a clean failure record.
    """
    model = params if isinstance(params, Model) else make_model(params, grid or TEST_GRID, case_a)
    n = model.n
    stats = {"rhs_calls": 0, "jac_calls": 0, "refusals": 0, "last_refusal": None}

    def fun(t, y):
        stats["rhs_calls"] += 1
        try:
            return model.rhs(t, y)
        except RateDomainError as err:
            stats["refusals"] += 1
            stats["last_refusal"] = {"t": float(t), "message": str(err)}
            return np.full(n, np.nan)

    def jac(t, y):
        stats["jac_calls"] += 1
        try:
            return model.jacobian(t, y)
        except RateDomainError as err:
            stats["refusals"] += 1
            stats["last_refusal"] = {"t": float(t), "message": str(err)}
            raise _JacobianRefusal(str(err)) from err

    base = {"method": "Radau", "rtol": float(rtol), "t_span": [float(t_span[0]), float(t_span[1])]}
    try:
        sol = solve_ivp(fun, t_span, np.asarray(y0, float), method="Radau", t_eval=t_eval,
                        dense_output=False, rtol=rtol, atol=np.asarray(atol_vector, float), jac=jac)
    except _JacobianRefusal as err:
        return dict(base, success=False, status=-2, message="jacobian refused", t=np.zeros(0),
                    y=np.zeros((0, n)), nfev=stats["rhs_calls"], njev=stats["jac_calls"], nlu=None,
                    domain_refusals=stats["refusals"], rhs_calls=stats["rhs_calls"],
                    jac_calls=stats["jac_calls"],
                    failure={"kind": "NonFinite", "where": "jacobian", "message": str(err),
                             "last_refusal": stats["last_refusal"]})
    failure = None
    if not sol.success:
        failure = {"kind": "NonFinite" if stats["refusals"] else "SolverFailure", "where": "rhs",
                   "message": stats["last_refusal"]["message"] if stats["last_refusal"] else sol.message,
                   "solver_message": sol.message, "last_refusal": stats["last_refusal"],
                   "t_reached": float(sol.t[-1]) if sol.t.size else None}
    return dict(base, success=bool(sol.success), status=int(sol.status), message=sol.message,
                t=np.asarray(sol.t), y=np.asarray(sol.y).T, nfev=int(sol.nfev), njev=int(sol.njev),
                nlu=int(sol.nlu), domain_refusals=stats["refusals"], rhs_calls=stats["rhs_calls"],
                jac_calls=stats["jac_calls"], failure=failure)


def test_scenario_run(n_eval=11, rtol=1e-10, atol_factor=1e-14, params=TEST_PARAMS, grid=TEST_GRID):
    """The short reference run of test (g); returns (model, y0, result)."""
    model = make_model(params, grid)
    y0 = model.initial_state()
    t_eval = np.linspace(0.0, 1e12, n_eval)
    res = integrate_radau((0.0, 1e12), y0, t_eval, rtol, atol_factor * model.scale_vector(y0), model)
    return model, y0, res


test_scenario_run.__test__ = False  # not a unittest/pytest test


def summary(model: Model):
    return {"params": asdict(model.params), "grid": model.grid.describe(), "K_D": K_D,
            "t0": model.geom.t0, "T_domain": [model.case_a.T_min, model.case_a.T_max]}


if __name__ == "__main__":
    import json

    m = make_model()
    y0 = m.initial_state()
    rec = {"summary": summary(m), "y0": y0.tolist(), "rhs_t0": m.rhs(0.0, y0).tolist(),
           "partial_t_t0": m.partial_t(0.0, y0).tolist(), "diagnostics_t0": m.diagnostics(0.0, y0),
           "names": m.grid.index_names()}
    text = json.dumps(rec, indent=1)
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(text + "\n")
    else:
        print(text)
