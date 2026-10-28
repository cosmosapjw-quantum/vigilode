"""Full-sphere triaxial Bianchi-I photon transport formula oracle (BASS/rei DAG node P06, scoped).

Independent pure-Python float oracle. It is not a live solver port and is not installed into BASS/rei.
The theory, the proofs and the bit-parity contract are in FULL_SPHERE_THEORY.md (same directory).

Variables: q = ln(E/E_ref), mu = n_3 = cos(theta), phi = atan2(n_2, n_1) and the cell contents
N_ijk = int_cell U dq dmu dphi, U = a^3 kappa E^3 f with a^3 = a_1 a_2 a_3. Geometry
ds^2 = -dt^2 + sum_k a_k^2 dx_k^2 with prescribed H_k = d ln a_k/dt, given per call.

Conventions (the modelling choices of FULL_SPHERE_THEORY.md section 6):

* cells-only vectors are ordered ``contents[i*M + j*nphi + k]`` with M = nm*nphi (energy group i
  outermost, then mu cell j, then phi cell k);
* mu_edges[0] == -1.0 and mu_edges[-1] == 1.0 exactly;
* phi_edges[0] = phi0 with 0 <= phi0 < TAU and phi_edges[-1] == phi0 + TAU exactly (TAU = 2*pi as a
  float). The last edge is the periodic image of the first, so there are nphi phi faces: face l sits
  at phi_edges[l] and separates the cells (l-1) mod nphi and l;
* first-order upwind on shared faces with the exact face average of the normal velocity. The tie
  rule is: v >= 0 takes the lower-index cell (for phi, the cell (l-1) mod nphi);
* q boundaries are either zero inflow with outflow ledgers ("outflow") or no flux ("closed"). The
  mu = +-1 faces carry no flux, because the velocity vanishes there.

Python allocation is intentional: this is a formula oracle, not a hot loop.
Run: ``python3 FULL_SPHERE_FVM_REFERENCE.py CHECKS.json``.
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import platform
import random
import sys
import time
import unittest
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True

TAU = 2.0 * math.pi
#: Weights w_m of the unnormalised real quadrupole basis b_m: (1/2) P2(n.n') = sum_m w_m b_m(n) b_m(n').
QUAD_WEIGHTS = (0.5, 1.5, 1.5, 0.375, 1.5)
QUAD_WEIGHTS_EXACT = (Fraction(1, 2), Fraction(3, 2), Fraction(3, 2), Fraction(3, 8), Fraction(3, 2))
BOUNDARIES = ("outflow", "closed")

NODE_DIR = Path(__file__).resolve().parent
REPO = NODE_DIR.parents[1]
LRS_ORACLE_PATH = REPO / "research" / "ncp_followup_20261021" / "BASS_REI_FVM_REFERENCE.py"

DIAG: dict = {}


# ---------------------------------------------------------------------------
# Typed refusals
# ---------------------------------------------------------------------------

class FullSphereRefusal(ValueError):
    """Typed refusal. ``code`` is a stable, machine-readable reason (theory section 9)."""

    def __init__(self, code: str, message: str):
        super().__init__(f"{code}: {message}")
        self.code = code


class GridRefusal(FullSphereRefusal):
    pass


class StateRefusal(FullSphereRefusal):
    pass


class GeometryRefusal(FullSphereRefusal):
    pass


class RateRefusal(FullSphereRefusal):
    pass


class BoundaryRefusal(FullSphereRefusal):
    pass


class CharacteristicRefusal(FullSphereRefusal):
    pass


class PolarizationRefusal(FullSphereRefusal):
    pass


def _is_real(x) -> bool:
    return isinstance(x, (int, float)) and not isinstance(x, bool)


def _finite_reals(values, refusal, code, what):
    try:
        vals = list(values)
    except TypeError:
        raise refusal(code, f"{what} must be a sequence of real numbers") from None
    out = []
    for x in vals:
        if not _is_real(x):
            raise refusal(code, f"{what} must contain real numbers only")
        x = float(x)
        if not math.isfinite(x):
            raise refusal(code, f"{what} must be finite")
        out.append(x)
    return out


def _edges(edges, axis):
    """Validated float edges and widths of one axis."""
    try:
        vals = list(edges)
    except TypeError:
        raise GridRefusal(f"{axis}.not_a_sequence", "edges must be a sequence") from None
    if len(vals) < 2:
        raise GridRefusal(f"{axis}.too_few_edges", "at least two edges are required")
    out = []
    for x in vals:
        if not _is_real(x):
            raise GridRefusal(f"{axis}.not_real", "edges must be real numbers")
        x = float(x)
        if not math.isfinite(x):
            raise GridRefusal(f"{axis}.nonfinite_edge", "edges must be finite")
        out.append(x)
    widths = [b - a for a, b in zip(out, out[1:])]
    for d in widths:
        if not (math.isfinite(d) and d > 0.0):
            raise GridRefusal(f"{axis}.width", "strictly increasing edges with finite widths required")
        if d < sys.float_info.min:
            raise GridRefusal(f"{axis}.subnormal_width", "every width must be a normal float")
    return out, widths


# ---------------------------------------------------------------------------
# Exact cell averages (closed forms; theory section 5.2 and 7.3)
# ---------------------------------------------------------------------------

def _mu_poly(a, b):
    """Polynomial mu-cell constants; generic over float and Fraction (same expressions).

    Returns (d, m2, t2, p2, ea, eb): width, <mu^2>, <1-mu^2>, <P2>, 1-a^2, 1-b^2.
    For floats ``x/2`` equals ``0.5*x`` and ``x/3`` equals ``x/3.0`` bit for bit.
    """
    d = b - a
    m2 = (a * a + a * b + b * b) / 3
    ea = (1 - a) * (1 + a)
    eb = (1 - b) * (1 + b)
    t2 = (ea + eb) / 2 + (d * d) / 6
    p2 = m2 - t2 / 2
    return d, m2, t2, p2, ea, eb


def mu_cell_constants(a: float, b: float):
    """(d, m2, t2, p2, r) for the mu cell [a, b]: exact cell averages of mu^2, 1-mu^2, P2(mu)
    and mu*sqrt(1-mu^2), in cancellation-free closed forms."""
    d, m2, t2, p2, ea, eb = _mu_poly(a, b)
    sa = math.sqrt(ea)
    sb = math.sqrt(eb)
    den = sa + sb
    # den == 0 only for the single cell [-1, 1], where the average is 0 by symmetry.
    r = (a + b) * ((ea + sa * sb) + eb) / (3.0 * den) if den > 0.0 else 0.0
    return d, m2, t2, p2, r


def phi_cell_constants(alpha: float, beta: float):
    """(delta, g, cphi, sphi, c2, s2h) for the phi cell [alpha, beta].

    g = delta/TAU; exact cell averages of cos(phi), sin(phi), cos(2 phi) and sin(phi)cos(phi),
    written with half-angle products (no difference of nearly equal sines).
    """
    delta = beta - alpha
    g = delta / TAU
    half = 0.5 * delta
    sinc_half = math.sin(half) / half
    mid = 0.5 * (alpha + beta)
    cphi = math.cos(mid) * sinc_half
    sphi = math.sin(mid) * sinc_half
    total = alpha + beta
    sinc = math.sin(delta) / delta
    c2 = math.cos(total) * sinc
    s2h = 0.5 * (math.sin(total) * sinc)
    return delta, g, cphi, sphi, c2, s2h


def mu_face_weight(mu: float) -> float:
    """w(mu) = mu (1 - mu^2), written as mu*((1-mu)*(1+mu))."""
    return mu * ((1.0 - mu) * (1.0 + mu))


def uniform_edges(lo: float, hi: float, n: int):
    if not isinstance(n, int) or n < 1:
        raise GridRefusal("grid.cell_count", "n must be a positive integer")
    return [lo + (hi - lo) * j / n for j in range(n)] + [hi]


def uniform_mu_edges(n: int):
    return uniform_edges(-1.0, 1.0, n)


def uniform_phi_edges(n: int, offset: float = 0.0):
    if not isinstance(n, int) or n < 1:
        raise GridRefusal("grid.cell_count", "n must be a positive integer")
    return [offset + TAU * j / n for j in range(n)] + [offset + TAU]


class SphereGrid:
    """Validated (q, mu, phi) grid holding every precomputed constant of the parity contract."""

    def __init__(self, q_edges, mu_edges, phi_edges):
        self.q, self.dq = _edges(q_edges, "q")
        self.mu, self.dm = _edges(mu_edges, "mu")
        self.phi, self.dphi = _edges(phi_edges, "phi")
        if self.mu[0] != -1.0 or self.mu[-1] != 1.0:
            raise GridRefusal("mu.not_full_sphere", "mu edges must start at -1.0 and end at 1.0 exactly")
        phi0 = self.phi[0]
        if not (0.0 <= phi0 < TAU) or self.phi[-1] != phi0 + TAU:
            raise GridRefusal("phi.not_periodic_cover",
                              "phi edges need 0 <= phi0 < TAU and phi[-1] == phi0 + TAU exactly")
        self.nq, self.nm, self.nphi = len(self.dq), len(self.dm), len(self.dphi)
        nm, nphi = self.nm, self.nphi
        self.M = nm * nphi
        muc = [mu_cell_constants(a, b) for a, b in zip(self.mu, self.mu[1:])]
        self.m2 = [c[1] for c in muc]
        self.t2 = [c[2] for c in muc]
        self.p2 = [c[3] for c in muc]
        self.r = [c[4] for c in muc]
        self.e = [d / 2.0 for d in self.dm]
        phc = [phi_cell_constants(a, b) for a, b in zip(self.phi, self.phi[1:])]
        self.g = [c[1] for c in phc]
        self.cphi = [c[2] for c in phc]
        self.sphi = [c[3] for c in phc]
        self.c2 = [c[4] for c in phc]
        self.s2h = [c[5] for c in phc]
        # Interior mu faces l = 1..nm-1 (index 0 unused: the mu = -1 face carries no flux).
        self.wface = [0.0] + [mu_face_weight(self.mu[l]) for l in range(1, nm)]
        # Periodic phi faces l = 0..nphi-1 at phi_edges[l].
        self.sn2 = [math.sin(2.0 * self.phi[l]) for l in range(nphi)]
        B = [[0.0] * self.M for _ in range(5)]
        for j in range(nm):
            for k in range(nphi):
                c = j * nphi + k
                B[0][c] = self.p2[j]
                B[1][c] = self.r[j] * self.cphi[k]
                B[2][c] = self.r[j] * self.sphi[k]
                B[3][c] = self.t2[j] * self.c2[k]
                B[4][c] = self.t2[j] * self.s2h[k]
        self.B = B
        self.Aw = [[w * b for b in Bm] for w, Bm in zip(QUAD_WEIGHTS, B)]
        self.om = [self.e[j] * self.g[k] for j in range(nm) for k in range(nphi)]

    def cell(self, i: int, j: int, k: int) -> int:
        """Index into the cells-only vector."""
        return i * self.M + j * self.nphi + k


class StateLayout:
    """P02 layout extended to the sphere: [L_lo, (N_i,0..N_i,M-1, A_i) for i in 0..nq, L_hi]."""

    LO = 0

    def __init__(self, nq: int, nm: int, nphi: int):
        self.nq, self.nm, self.nphi = nq, nm, nphi
        self.M = nm * nphi
        self.n = 2 + nq * (self.M + 1)
        self.hi = self.n - 1
        self.bandwidth = self.M + 1

    def cell(self, i: int, j: int, k: int) -> int:
        return 1 + i * (self.M + 1) + j * self.nphi + k

    def absorbed(self, i: int) -> int:
        return 1 + i * (self.M + 1) + self.M


# ---------------------------------------------------------------------------
# Transport (theory section 5)
# ---------------------------------------------------------------------------

def _geometry(H1, H2, H3):
    if not (_is_real(H1) and _is_real(H2) and _is_real(H3)):
        raise GeometryRefusal("geometry.not_real", "H1, H2, H3 must be real numbers")
    H1, H2, H3 = float(H1), float(H2), float(H3)
    if not (math.isfinite(H1) and math.isfinite(H2) and math.isfinite(H3)):
        raise GeometryRefusal("geometry.nonfinite", "H1, H2, H3 must be finite")
    hbar = 0.5 * (H1 + H2)
    hd = 0.5 * (H1 - H2)
    if not (math.isfinite(hbar) and math.isfinite(hd)):
        raise GeometryRefusal("geometry.overflow", "(H1 +- H2)/2 overflows")
    return H1, H2, H3, hbar, hd


def _velocities(grid: SphereGrid, geo):
    """(gk, vq, hd): mu-face factor H3 - <H_phi>_k, q velocity per column, phi factor (H1-H2)/2."""
    _, _, H3, hbar, hd = geo
    hp = [hbar + hd * c for c in grid.c2]
    gk = [H3 - h for h in hp]
    vq = [-(H3 * grid.m2[j] + grid.t2[j] * hp[k]) for j in range(grid.nm) for k in range(grid.nphi)]
    return gk, vq, hd


def _check_boundary(boundary):
    if boundary not in BOUNDARIES:
        raise BoundaryRefusal("boundary.invalid", "boundary must be 'outflow' or 'closed'")
    return boundary == "closed"


def _transport(grid: SphereGrid, x, geo, sel_geo, closed: bool):
    """The three face stages in the contract order (theory section 6.3)."""
    nq, nm, nphi, M = grid.nq, grid.nm, grid.nphi, grid.M
    dq, dm, dphi = grid.dq, grid.dm, grid.dphi
    gk, vq, hd = _velocities(grid, geo)
    gks, vqs, hds = _velocities(grid, sel_geo)
    rhs = [0.0] * (nq * M)
    out_lo = 0.0
    out_hi = 0.0
    # Stage Q: energy faces l = 0..nq of every angular column (j, k).
    for j in range(nm):
        for k in range(nphi):
            jk = j * nphi + k
            v = vq[jk]
            sel = vqs[jk]
            for l in range(nq + 1):
                if closed and (l == 0 or l == nq):
                    continue
                up = l - 1 if sel >= 0.0 else l
                val = x[up * M + jk] / dq[up] if 0 <= up < nq else 0.0
                flux = v * val
                if l > 0:
                    rhs[(l - 1) * M + jk] -= flux
                else:
                    out_lo -= flux
                if l < nq:
                    rhs[l * M + jk] += flux
                else:
                    out_hi += flux
    # Stage MU: interior mu faces l = 1..nm-1 for every (i, k).
    w = grid.wface
    for i in range(nq):
        base = i * M
        for k in range(nphi):
            for l in range(1, nm):
                v = -(gk[k] * w[l])
                sel = -(gks[k] * w[l])
                up = l - 1 if sel >= 0.0 else l
                flux = v * (x[base + up * nphi + k] / dm[up])
                rhs[base + (l - 1) * nphi + k] -= flux
                rhs[base + l * nphi + k] += flux
    # Stage PHI: periodic phi faces l = 0..nphi-1 for every (i, j).
    sn2 = grid.sn2
    for i in range(nq):
        for j in range(nm):
            base = i * M + j * nphi
            for l in range(nphi):
                v = hd * sn2[l]
                sel = hds * sn2[l]
                lo = l - 1 if l > 0 else nphi - 1
                up = lo if sel >= 0.0 else l
                flux = v * (x[base + up] / dphi[up])
                rhs[base + lo] -= flux
                rhs[base + l] += flux
    return rhs, out_lo, out_hi


def _select_geometry(select, geo):
    if select is None:
        return geo
    try:
        s = list(select)
    except TypeError:
        raise GeometryRefusal("geometry.select", "select must be (H1, H2, H3)") from None
    if len(s) != 3:
        raise GeometryRefusal("geometry.select", "select must be (H1, H2, H3)")
    try:
        return _geometry(*s)
    except GeometryRefusal as exc:
        raise GeometryRefusal("geometry.select", str(exc)) from None


def grid_number_rhs(grid: SphereGrid, contents, H1, H2, H3, boundary="outflow", *, select=None):
    closed = _check_boundary(boundary)
    geo = _geometry(H1, H2, H3)
    sel_geo = _select_geometry(select, geo)
    x = _finite_reals(contents, StateRefusal, "state.nonfinite", "contents")
    if len(x) != grid.nq * grid.M:
        raise StateRefusal("state.length", "contents must have nq*nm*nphi entries")
    return _transport(grid, x, geo, sel_geo, closed)


def sphere_number_rhs(q_edges, mu_edges, phi_edges, contents, H1, H2, H3, boundary="outflow", *,
                      select=None):
    """Conservative first-order upwind transport. Returns (rhs, outward_lo, outward_hi).

    ``outward_lo``/``outward_hi`` are the number rates leaving through the low/high energy boundary,
    so ``sum(rhs) + outward_lo + outward_hi == 0`` in exact arithmetic. ``select=(H1s, H2s, H3s)``
    takes the upwind direction from another geometry while the flux velocities use (H1, H2, H3);
    with select = H(t) and velocities dH/dt this is the exact f_t of a time-dependent geometry.
    The contents may be negative (linear JVP use): the operator is linear, so JVP = RHS.
    """
    grid = SphereGrid(q_edges, mu_edges, phi_edges)
    return grid_number_rhs(grid, contents, H1, H2, H3, boundary, select=select)


def sphere_number_jvp(q_edges, mu_edges, phi_edges, direction, H1, H2, H3, boundary="outflow"):
    """Exact derivative with respect to the contents for prescribed geometry."""
    return sphere_number_rhs(q_edges, mu_edges, phi_edges, direction, H1, H2, H3, boundary)[0]


def grid_transport_cfl(grid: SphereGrid, H1, H2, H3, boundary="outflow"):
    """Sufficient forward-Euler positivity step of the transport alone (1/max outgoing rate)."""
    closed = _check_boundary(boundary)
    geo = _geometry(H1, H2, H3)
    gk, vq, hd = _velocities(grid, geo)
    nq, nm, nphi = grid.nq, grid.nm, grid.nphi
    maxloss = 0.0
    for i in range(nq):
        for j in range(nm):
            for k in range(nphi):
                v = vq[j * nphi + k]
                if v >= 0.0:
                    open_face = not (closed and i == nq - 1)
                else:
                    open_face = not (closed and i == 0)
                loss = abs(v) / grid.dq[i] if open_face else 0.0
                if j >= 1:
                    vlo = -(gk[k] * grid.wface[j])
                    if vlo < 0.0:
                        loss += -vlo / grid.dm[j]
                if j + 1 <= nm - 1:
                    vhi = -(gk[k] * grid.wface[j + 1])
                    if vhi >= 0.0:
                        loss += vhi / grid.dm[j]
                vl = hd * grid.sn2[k]
                if vl < 0.0:
                    loss += -vl / grid.dphi[k]
                vu = hd * grid.sn2[(k + 1) % nphi]
                if vu >= 0.0:
                    loss += vu / grid.dphi[k]
                maxloss = max(maxloss, loss)
    return 1.0 / maxloss if maxloss > 0.0 else math.inf


def sphere_transport_cfl(q_edges, mu_edges, phi_edges, H1, H2, H3, boundary="outflow"):
    return grid_transport_cfl(SphereGrid(q_edges, mu_edges, phi_edges), H1, H2, H3, boundary)


# ---------------------------------------------------------------------------
# Rank-6 Thomson (theory section 7)
# ---------------------------------------------------------------------------

def _check_rate(rate):
    if not _is_real(rate) or not math.isfinite(float(rate)) or float(rate) < 0.0:
        raise RateRefusal("rate.invalid", "Thomson rate must be finite and >= 0")
    return float(rate)


def _thomson6_group(grid: SphereGrid, group, rate: float):
    """One energy group, contract order: moments in cell order, then S, then the gain."""
    B, Aw, om, M = grid.B, grid.Aw, grid.om, grid.M
    m0 = 0.0
    mom = [0.0] * 5
    for c in range(M):
        n = group[c]
        m0 += n
        for m in range(5):
            mom[m] += B[m][c] * n
    out = [0.0] * M
    for c in range(M):
        s = m0
        for m in range(5):
            s += Aw[m][c] * mom[m]
        out[c] = rate * (om[c] * s - group[c])
    return out


def thomson6_rhs(mu_edges, phi_edges, contents, rate):
    """Unpolarized cold elastic Thomson on one energy group of the full sphere (rank 6 + diagonal).

    contents[j*nphi + k] are cell contents; rate = c sigma_T n_e. Returns dN/dt.
    """
    grid = SphereGrid([0.0, 1.0], mu_edges, phi_edges)
    rate = _check_rate(rate)
    x = _finite_reals(contents, StateRefusal, "state.nonfinite", "contents")
    if len(x) != grid.M:
        raise StateRefusal("state.length", "contents must have nm*nphi entries")
    return _thomson6_group(grid, x, rate)


def thomson6_factors(mu_edges, phi_edges):
    """Low-rank factors (U, V) with T + Gamma I = Gamma U V^T, U, V of shape M x 6 (lists of columns).

    U_0 = om, V_0 = 1, U_{m+1} = om * w_m B_m, V_{m+1} = B_m. (For solvers; the RHS uses the
    S form of _thomson6_group, which is not bit-identical to U V^T N.)
    """
    grid = SphereGrid([0.0, 1.0], mu_edges, phi_edges)
    U = [list(grid.om)] + [[o * a for o, a in zip(grid.om, Aw)] for Aw in grid.Aw]
    V = [[1.0] * grid.M] + [list(Bm) for Bm in grid.B]
    return U, V


# ---------------------------------------------------------------------------
# State-level RHS in the P02 layout (theory section 8)
# ---------------------------------------------------------------------------

def _check_kbar(kbar, nq, code="absorber.invalid"):
    vals = _finite_reals(kbar, RateRefusal, code, "absorption rates")
    if len(vals) != nq or any(v < 0.0 for v in vals):
        raise RateRefusal(code, "one finite nonnegative absorption rate per energy group required")
    return vals


def grid_state_rhs(grid: SphereGrid, state, H1, H2, H3, rate=0.0, kbar=None, boundary="outflow", *,
                   select=None):
    """f(t, y) on the state [L_lo, (N_i, A_i) per group, L_hi]: transport, Thomson, absorber."""
    lay = StateLayout(grid.nq, grid.nm, grid.nphi)
    closed = _check_boundary(boundary)
    geo = _geometry(H1, H2, H3)
    sel_geo = _select_geometry(select, geo)
    rate = _check_rate(rate)
    y = _finite_reals(state, StateRefusal, "state.nonfinite", "state")
    if len(y) != lay.n:
        raise StateRefusal("state.length", "state must have 2 + nq*(nm*nphi + 1) entries")
    ks = None if kbar is None else _check_kbar(kbar, grid.nq)
    M = grid.M
    x = [y[1 + i * (M + 1) + c] for i in range(grid.nq) for c in range(M)]
    rhs, lo, hi = _transport(grid, x, geo, sel_geo, closed)
    out = [0.0] * lay.n
    out[StateLayout.LO] = lo
    out[lay.hi] = hi
    for i in range(grid.nq):
        for c in range(M):
            out[1 + i * (M + 1) + c] = rhs[i * M + c]
    if rate != 0.0:
        for i in range(grid.nq):
            gain = _thomson6_group(grid, x[i * M:(i + 1) * M], rate)
            for c in range(M):
                out[1 + i * (M + 1) + c] += gain[c]
    if ks is not None:
        for i in range(grid.nq):
            k = ks[i]
            absorbed = 0.0
            for c in range(M):
                rr = k * y[1 + i * (M + 1) + c]
                out[1 + i * (M + 1) + c] -= rr
                absorbed += rr
            out[lay.absorbed(i)] += absorbed
    return out


def grid_state_jacobian_entries(grid: SphereGrid, H1, H2, H3, rate, kbar, boundary, add):
    """Template of the Rust ``jacobian_entries``: every nonzero contribution through
    ``add(row, col, value)`` in the RHS's own loop order, so that a dense accumulation from +0.0
    reproduces ``grid_state_rhs`` of each unit vector bit for bit (theory section 8.3)."""
    lay = StateLayout(grid.nq, grid.nm, grid.nphi)
    closed = _check_boundary(boundary)
    geo = _geometry(H1, H2, H3)
    rate = _check_rate(rate)
    ks = None if kbar is None else _check_kbar(kbar, grid.nq)
    gk, vq, hd = _velocities(grid, geo)
    nq, nm, nphi, M = grid.nq, grid.nm, grid.nphi, grid.M
    for j in range(nm):
        for k in range(nphi):
            v = vq[j * nphi + k]
            for l in range(nq + 1):
                if closed and (l == 0 or l == nq):
                    continue
                up = l - 1 if v >= 0.0 else l
                if not 0 <= up < nq:
                    continue
                coef = v * (1.0 / grid.dq[up])
                col = lay.cell(up, j, k)
                add(lay.cell(l - 1, j, k) if l > 0 else StateLayout.LO, col, -coef)
                add(lay.cell(l, j, k) if l < nq else lay.hi, col, coef)
    for i in range(nq):
        for k in range(nphi):
            for l in range(1, nm):
                v = -(gk[k] * grid.wface[l])
                up = l - 1 if v >= 0.0 else l
                coef = v * (1.0 / grid.dm[up])
                col = lay.cell(i, up, k)
                add(lay.cell(i, l - 1, k), col, -coef)
                add(lay.cell(i, l, k), col, coef)
    for i in range(nq):
        for j in range(nm):
            for l in range(nphi):
                v = hd * grid.sn2[l]
                lo = l - 1 if l > 0 else nphi - 1
                up = lo if v >= 0.0 else l
                coef = v * (1.0 / grid.dphi[up])
                col = lay.cell(i, j, up)
                add(lay.cell(i, j, lo), col, -coef)
                add(lay.cell(i, j, l), col, coef)
    if rate != 0.0:
        for i in range(nq):
            base = 1 + i * (M + 1)
            for r in range(M):
                for c in range(M):
                    s = 1.0
                    for m in range(5):
                        s += grid.Aw[m][r] * grid.B[m][c]
                    add(base + r, base + c, rate * (grid.om[r] * s - (1.0 if r == c else 0.0)))
    if ks is not None:
        for i in range(nq):
            for c in range(M):
                cell = 1 + i * (M + 1) + c
                add(cell, cell, -ks[i])
                add(lay.absorbed(i), cell, ks[i])


def grid_state_jacobian_dense(grid: SphereGrid, H1, H2, H3, rate=0.0, kbar=None, boundary="outflow"):
    lay = StateLayout(grid.nq, grid.nm, grid.nphi)
    J = [[0.0] * lay.n for _ in range(lay.n)]

    def add(r, c, v):
        J[r][c] += v

    grid_state_jacobian_entries(grid, H1, H2, H3, rate, kbar, boundary, add)
    return J


def parity_fixture():
    """Fixed inputs for Rust bit-parity tests (every input is reproducible from its literal)."""
    q = [-1.0, -0.25, 0.5, 1.75]
    mu = [-1.0, -0.375, 0.25, 1.0]
    phi = uniform_phi_edges(4, 0.25)
    grid = SphereGrid(q, mu, phi)
    lay = StateLayout(grid.nq, grid.nm, grid.nphi)
    x = [1.0 + (c % 7) * 0.125 - (c % 3) * 0.0625 for c in range(grid.nq * grid.M)]
    y = [0.0] * lay.n
    for i in range(grid.nq):
        for c in range(grid.M):
            y[1 + i * (grid.M + 1) + c] = x[i * grid.M + c]
    return dict(q=q, mu=mu, phi=phi, contents=x, state=y, H=(0.3, -0.1, 0.05), select=(-0.2, 0.4, 0.1),
                rate=2.5, kbar=[0.2, 0.0, 0.7])


def parity_fixture_outputs():
    fx = parity_fixture()
    grid = SphereGrid(fx["q"], fx["mu"], fx["phi"])
    H = fx["H"]
    hexs = lambda v: [float(z).hex() for z in v]  # noqa: E731
    out = {}
    for b in BOUNDARIES:
        rhs, lo, hi = grid_number_rhs(grid, fx["contents"], *H, b)
        out[f"transport_{b}"] = hexs(rhs) + [lo.hex(), hi.hex()]
    rhs, lo, hi = grid_number_rhs(grid, fx["contents"], *H, "outflow", select=fx["select"])
    out["transport_select"] = hexs(rhs) + [lo.hex(), hi.hex()]
    out["thomson6_group0"] = hexs(_thomson6_group(grid, fx["contents"][:grid.M], fx["rate"]))
    out["state_rhs"] = hexs(grid_state_rhs(grid, fx["state"], *H, fx["rate"], fx["kbar"]))
    out["constants"] = {name: hexs(getattr(grid, name)) for name in
                        ("dq", "dm", "dphi", "m2", "t2", "p2", "r", "e", "g", "cphi", "sphi", "c2", "s2h",
                         "wface", "sn2", "om")}
    out["constants"]["B"] = [hexs(Bm) for Bm in grid.B]
    out["inputs"] = {"q": hexs(fx["q"]), "mu": hexs(fx["mu"]), "phi": hexs(fx["phi"]),
                     "H": hexs(H), "select": hexs(fx["select"]), "rate": float(fx["rate"]).hex(),
                     "kbar": hexs(fx["kbar"]), "contents_formula": "1 + (c % 7)*0.125 - (c % 3)*0.0625"}
    digest = hashlib.sha256()
    for key in ("transport_outflow", "transport_closed", "transport_select", "thomson6_group0", "state_rhs"):
        digest.update(("\n".join(out[key]) + "\n").encode())
    out["sha256_outputs"] = digest.hexdigest()
    return out


def grid_state_partial_t(grid: SphereGrid, state, H, dH, dkbar=None, boundary="outflow"):
    """f_t at fixed state: transport velocities from dH/dt, upwind pattern from H(t); absorber
    rate derivatives dkbar (the Thomson rate is constant here)."""
    lay = StateLayout(grid.nq, grid.nm, grid.nphi)
    out = grid_state_rhs(grid, state, dH[0], dH[1], dH[2], 0.0, None, boundary, select=H)
    if dkbar is not None:
        # Rate derivatives may have either sign: finite, one per energy group.
        ks = _finite_reals(dkbar, RateRefusal, "absorber.invalid", "absorption rate derivatives")
        if len(ks) != grid.nq:
            raise RateRefusal("absorber.invalid", "one rate derivative per energy group required")
        y = [float(v) for v in state]
        M = grid.M
        for i in range(grid.nq):
            k = ks[i]
            absorbed = 0.0
            for c in range(M):
                rr = k * y[1 + i * (M + 1) + c]
                out[1 + i * (M + 1) + c] -= rr
                absorbed += rr
            out[lay.absorbed(i)] += absorbed
    return out


# ---------------------------------------------------------------------------
# Exact characteristics (theory section 4)
# ---------------------------------------------------------------------------

def _amplification(A):
    try:
        vals = list(A)
    except TypeError:
        raise CharacteristicRefusal("characteristic.amplification", "A must be (A1, A2, A3)") from None
    if len(vals) != 3 or not all(_is_real(a) for a in vals):
        raise CharacteristicRefusal("characteristic.amplification", "A must be three real numbers")
    vals = [float(a) for a in vals]
    if not all(math.isfinite(a) and a > 0.0 for a in vals):
        raise CharacteristicRefusal("characteristic.amplification", "A_k must be finite and > 0")
    return vals


def direction(mu: float, phi: float):
    if not (_is_real(mu) and _is_real(phi)) or not (math.isfinite(mu) and math.isfinite(phi)) \
            or not (-1.0 <= mu <= 1.0):
        raise CharacteristicRefusal("characteristic.direction", "need finite phi and -1 <= mu <= 1")
    s = math.sqrt((1.0 - mu) * (1.0 + mu))
    return (s * math.cos(phi), s * math.sin(phi), mu)


def angles(n):
    return n[2], math.atan2(n[1], n[0]) % TAU


def _char_dir(n, A):
    """(q shift q0 - q, n0, Liouville factor) for direction n and amplifications A."""
    x, y, z = A[0] * n[0], A[1] * n[1], A[2] * n[2]
    s2 = x * x + y * y + z * z
    root = math.sqrt(s2)
    return 0.5 * math.log(s2), (x / root, y / root, z / root), (A[0] * A[1] * A[2]) / (s2 * root)


def characteristic_map(q, mu, phi, A):
    """Back-propagate (q, mu, phi) at time t to (q0, mu0, phi0) at t0 and return the factor
    prod(A_k) (E/E0)^3, with A_k = exp(int_{t0}^{t} H_k dt)."""
    A = _amplification(A)
    if not (_is_real(q) and math.isfinite(q)):
        raise CharacteristicRefusal("characteristic.direction", "q must be finite")
    shift, n0, factor = _char_dir(direction(mu, phi), A)
    mu0, phi0 = angles(n0)
    return q + shift, mu0, phi0, factor


def exact_U(q, mu, phi, A, U0):
    """U(t, q, mu, phi) = prod(A_k) (E/E0)^3 U0(q0, n0) for the collisionless transport."""
    A = _amplification(A)
    shift, n0, factor = _char_dir(direction(mu, phi), A)
    return factor * U0(q + shift, n0)


_GL_CACHE: dict = {}


def gauss_legendre(n: int):
    """Gauss-Legendre nodes and weights on [-1, 1] (Newton on P_n; pure Python)."""
    if n in _GL_CACHE:
        return _GL_CACHE[n]
    if not isinstance(n, int) or n < 1:
        raise CharacteristicRefusal("characteristic.quadrature", "npts must be a positive integer")
    xs, ws = [], []
    for i in range(1, n + 1):
        x = math.cos(math.pi * (i - 0.25) / (n + 0.5))
        for _ in range(100):
            p0, p1 = 1.0, x
            for k in range(2, n + 1):
                p0, p1 = p1, ((2 * k - 1) * x * p1 - (k - 1) * p0) / k
            dp = n * (x * p1 - p0) / (x * x - 1.0)
            dx = p1 / dp
            x -= dx
            if abs(dx) <= 1e-16:
                break
        p0, p1 = 1.0, x
        for k in range(2, n + 1):
            p0, p1 = p1, ((2 * k - 1) * x * p1 - (k - 1) * p0) / k
        dp = n * (x * p1 - p0) / (x * x - 1.0)
        xs.append(x)
        ws.append(2.0 / ((1.0 - x * x) * dp * dp))
    _GL_CACHE[n] = (xs, ws)
    return xs, ws


def exact_cell_integral(q_lo, q_hi, mu_lo, mu_hi, phi_lo, phi_hi, A, U0, npts=3):
    """Tensor Gauss-Legendre integral of the exact U over one cell, in (q, theta, phi).

    The substitution mu = cos(theta) makes the integrand smooth up to the poles. U0(q, n) takes a
    unit vector n. With A = (1, 1, 1) this projects the initial data.
    """
    A = _amplification(A)
    xs, ws = gauss_legendre(npts)
    th_a, th_b = math.acos(mu_hi), math.acos(mu_lo)
    hq, cq = 0.5 * (q_hi - q_lo), 0.5 * (q_hi + q_lo)
    ht, ct = 0.5 * (th_b - th_a), 0.5 * (th_b + th_a)
    hp, cp = 0.5 * (phi_hi - phi_lo), 0.5 * (phi_hi + phi_lo)
    total = 0.0
    for xt, wt in zip(xs, ws):
        th = ct + ht * xt
        st, cth = math.sin(th), math.cos(th)
        for xp, wp in zip(xs, ws):
            ph = cp + hp * xp
            n = (st * math.cos(ph), st * math.sin(ph), cth)
            shift, n0, factor = _char_dir(n, A)
            acc = 0.0
            for xq, wq in zip(xs, ws):
                acc += wq * U0(cq + hq * xq + shift, n0)
            total += wt * wp * st * factor * acc
    return total * hq * ht * hp


def exact_cell_contents(q_edges, mu_edges, phi_edges, A, U0, npts=3):
    """Exact cell contents in the cells-only order (i, j, k) -> i*M + j*nphi + k."""
    grid = SphereGrid(q_edges, mu_edges, phi_edges)
    out = []
    for i in range(grid.nq):
        for j in range(grid.nm):
            for k in range(grid.nphi):
                out.append(exact_cell_integral(grid.q[i], grid.q[i + 1], grid.mu[j], grid.mu[j + 1],
                                               grid.phi[k], grid.phi[k + 1], A, U0, npts))
    return out


# ---------------------------------------------------------------------------
# Polarization contract: axisymmetric (I, Q) Thomson, Chandrasekhar kernel (theory section 10)
# ---------------------------------------------------------------------------

def _axial_grid(mu_edges):
    mu, dm = _edges(mu_edges, "mu")
    if mu[0] != -1.0 or mu[-1] != 1.0:
        raise GridRefusal("mu.not_full_sphere", "mu edges must start at -1.0 and end at 1.0 exactly")
    return mu, dm


def _pol_iq(d, p, I, Q, rate):
    """(I, Q) moment form; generic over float and Fraction (x/2 == 0.5*x for floats)."""
    i0 = 0
    i2 = 0
    q0 = 0
    q2 = 0
    for dj, pj, Ij, Qj in zip(d, p, I, Q):
        i0 += Ij
        i2 += pj * Ij
        q0 += Qj
        q2 += pj * Qj
    i0, i2, q0, q2 = i0 / 2, i2 / 2, q0 / 2, q2 / 2
    pi = (q0 - i2) - q2
    dI = [rate * ((dj * i0 - (dj * pj) / 2 * pi) - Ij) for dj, pj, Ij in zip(d, p, I)]
    dQ = [rate * ((dj * (1 - pj)) / 2 * pi - Qj) for dj, pj, Qj in zip(d, p, Q)]
    return dI, dQ


def _pol_lr(d, m2, t2, Il, Ir, rate, three_eighths):
    """Cell-averaged Chandrasekhar kernel in (I_l, I_r); O(n^2) cross-check form."""
    n = len(d)
    dIl, dIr = [], []
    for j in range(n):
        jl = 0
        jr = 0
        for jj in range(n):
            jl += (2 * t2[j] * t2[jj] + m2[j] * m2[jj]) * Il[jj] + m2[j] * Ir[jj]
            jr += m2[jj] * Il[jj] + Ir[jj]
        dIl.append(rate * (three_eighths * d[j] * jl - Il[j]))
        dIr.append(rate * (three_eighths * d[j] * jr - Ir[j]))
    return dIl, dIr


def polarized_thomson_iq_rhs(mu_edges, I, Q, rate):
    """Axisymmetric polarized cold Thomson on one group: (dI/dt, dQ/dt) for cell contents (I, Q).

    Q is referred to the meridian plane (Q = I_l - I_r); U = V = 0 by axial symmetry.
    Continuum form: dI/dt = G[I0 - P2(mu) Pi/2 - I], dQ/dt = G[(1 - P2(mu)) Pi/2 - Q],
    Pi = Q0 - I2 - Q2, X_l = (1/2) int P_l X dmu.
    """
    mu, dm = _axial_grid(mu_edges)
    rate = _check_rate(rate)
    Iv = _finite_reals(I, PolarizationRefusal, "polarization.nonfinite", "I")
    Qv = _finite_reals(Q, PolarizationRefusal, "polarization.nonfinite", "Q")
    if len(Iv) != len(dm) or len(Qv) != len(dm):
        raise PolarizationRefusal("polarization.length", "I and Q need one entry per mu cell")
    p = [_mu_poly(a, b)[3] for a, b in zip(mu, mu[1:])]
    return _pol_iq(dm, p, Iv, Qv, rate)


def polarized_thomson_lr_rhs(mu_edges, Il, Ir, rate):
    mu, dm = _axial_grid(mu_edges)
    rate = _check_rate(rate)
    consts = [_mu_poly(a, b) for a, b in zip(mu, mu[1:])]
    return _pol_lr(dm, [c[1] for c in consts], [c[2] for c in consts], list(Il), list(Ir), rate, 0.375)


def coherency_eigenvalues(I, Q, U=0.0, V=0.0):
    """Eigenvalues (I - |P|)/2, (I + |P|)/2 of rho = (1/2)[[I+Q, U-iV], [U+iV, I-Q]]."""
    P = math.sqrt(Q * Q + U * U + V * V)
    return 0.5 * (I - P), 0.5 * (I + P)


def componentwise_clip(I, Q, U=0.0, V=0.0):
    """The WRONG limiter: clip every Stokes component at zero (kept as a negative control)."""
    return max(I, 0.0), max(Q, 0.0), max(U, 0.0), max(V, 0.0)


def coherency_limit(I, Q, U=0.0, V=0.0):
    """Number-conserving realizability limiter: scale the polarization vector to |P| <= I.

    Keeps I (number and energy). There is no number-conserving local repair for I < 0: refuse.
    """
    if not all(_is_real(v) and math.isfinite(v) for v in (I, Q, U, V)):
        raise PolarizationRefusal("polarization.nonfinite", "Stokes parameters must be finite")
    if I < 0.0:
        raise PolarizationRefusal("polarization.negative_intensity",
                                  "I < 0 has no number-conserving local repair; reject the step")
    P = math.sqrt(Q * Q + U * U + V * V)
    if P <= I:
        return I, Q, U, V
    scale = I / P  # P > I >= 0
    # Rounding-aware: step the scale down until the evaluated |P'| <= I holds in floating point.
    while True:
        q, u, v = Q * scale, U * scale, V * scale
        if math.sqrt(q * q + u * u + v * v) <= I:
            return I, q, u, v
        scale = math.nextafter(scale, 0.0)


# ===========================================================================
# Checks
# ===========================================================================

def sha256_file(path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_lrs_oracle():
    """Import the P01 LRS oracle read-only by file path (no bytecode written)."""
    prev = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        spec = importlib.util.spec_from_file_location("bass_rei_lrs_oracle_readonly", str(LRS_ORACLE_PATH))
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
    finally:
        sys.dont_write_bytecode = prev
    return mod


def _random_edges(rng, lo, hi, n, min_frac=0.25):
    w = [min_frac + (1.0 - min_frac) * rng.random() for _ in range(n)]
    tot = sum(w)
    edges = [lo]
    acc = 0.0
    for x in w[:-1]:
        acc += x
        edges.append(lo + (hi - lo) * acc / tot)
    edges.append(hi)
    return edges


def _random_phi_edges(rng, n, offset):
    w = [0.25 + 0.75 * rng.random() for _ in range(n)]
    tot = sum(w)
    edges = [offset]
    acc = 0.0
    for x in w[:-1]:
        acc += x
        edges.append(offset + TAU * acc / tot)
    edges.append(offset + TAU)
    return edges


class _Dual:
    """Dual numbers over Fraction for exact derivatives."""

    __slots__ = ("a", "b")

    def __init__(self, a, b=0):
        self.a = Fraction(a)
        self.b = Fraction(b)

    @staticmethod
    def lift(x):
        return x if isinstance(x, _Dual) else _Dual(x, 0)

    def __add__(self, o):
        o = _Dual.lift(o)
        return _Dual(self.a + o.a, self.b + o.b)

    __radd__ = __add__

    def __sub__(self, o):
        o = _Dual.lift(o)
        return _Dual(self.a - o.a, self.b - o.b)

    def __rsub__(self, o):
        return _Dual.lift(o) - self

    def __neg__(self):
        return _Dual(-self.a, -self.b)

    def __mul__(self, o):
        o = _Dual.lift(o)
        return _Dual(self.a * o.a, self.a * o.b + self.b * o.a)

    __rmul__ = __mul__

    def __truediv__(self, o):
        o = _Dual.lift(o)
        return _Dual(self.a / o.a, (self.b * o.a - self.a * o.b) / (o.a * o.a))

    def __rtruediv__(self, o):
        return _Dual.lift(o) / self

    def sqrt(self):
        f = self.a
        rn, rd = math.isqrt(f.numerator), math.isqrt(f.denominator)
        if rn * rn != f.numerator or rd * rd != f.denominator:
            raise ArithmeticError("real part is not a rational square")
        r = Fraction(rn, rd)
        return _Dual(r, self.b / (2 * r))


def _sphere_point(u, v):
    """Rational point on S^2 (inverse stereographic projection)."""
    u, v = Fraction(u), Fraction(v)
    den = u * u + v * v + 1
    return (2 * u / den, 2 * v / den, (u * u + v * v - 1) / den)


def _circle_point(tau):
    tau = Fraction(tau)
    den = 1 + tau * tau
    return (1 - tau * tau) / den, 2 * tau / den


def _exact_drift(mu, c, s, H1, H2, H3):
    hphi = H1 * c * c + H2 * s * s
    qdot = -(H1 * (1 - mu * mu) * c * c + H2 * (1 - mu * mu) * s * s + H3 * mu * mu)
    mudot = -mu * (1 - mu * mu) * (H3 - hphi)
    phidot = (H1 - H2) * s * c
    return qdot, mudot, phidot


def _basis_exact(n):
    x, y, z = n
    return ((3 * z * z - 1) / 2, x * z, y * z, x * x - y * y, x * y)


def _rel(a, b):
    return abs(a - b) / max(abs(b), 1e-300)


class A_ExactAlgebra(unittest.TestCase):
    """(a) Fractions: drift from momenta, Liouville identity, LRS reduction, kernel identities."""

    def test_a1_drift_from_geodesic_momenta_exact(self):
        rng = random.Random(101)
        count = 0
        for _ in range(40):
            u = Fraction(rng.randint(-9, 9), rng.randint(1, 7))
            v = Fraction(rng.randint(-9, 9), rng.randint(1, 7))
            if u == 0 and v == 0:
                continue
            p = _sphere_point(u, v)
            lam = Fraction(rng.randint(1, 5), rng.randint(1, 5))
            p = tuple(lam * x for x in p)  # |p| = lam: rational
            H = [Fraction(rng.randint(-20, 20), rng.randint(1, 11)) for _ in range(3)]
            t = _Dual(0, 1)
            P = [p[k] / (1 + H[k] * t) for k in range(3)]  # a_k = 1 + H_k t: (da_k/dt)/a_k = H_k at t = 0
            E = (P[0] * P[0] + P[1] * P[1] + P[2] * P[2]).sqrt()
            nvec = [Pk / E for Pk in P]
            qdot = E.b / E.a
            n1, n2, n3 = (x.a for x in nvec)
            rho2 = n1 * n1 + n2 * n2
            mudot = nvec[2].b
            phidot = (n1 * nvec[1].b - n2 * nvec[0].b) / rho2
            S = H[0] * n1 * n1 + H[1] * n2 * n2 + H[2] * n3 * n3
            for k in range(3):
                self.assertEqual(nvec[k].b, -nvec[k].a * (H[k] - S))
            hphi = (H[0] * n1 * n1 + H[1] * n2 * n2) / rho2
            self.assertEqual(qdot, -S)
            self.assertEqual(mudot, -n3 * (1 - n3 * n3) * (H[2] - hphi))
            self.assertEqual(phidot, (H[0] - H[1]) * n1 * n2 / rho2)
            count += 1
        DIAG["a1_geodesic_points"] = count

    def test_a2_liouville_identity_exact(self):
        rng = random.Random(102)
        basis = [(1, 0, 0), (0, 1, 0), (0, 0, 1)]
        unisolvent_mu = (Fraction(0), Fraction(1, 2))  # distinct mu^2
        unisolvent_tau = (Fraction(0), Fraction(1))  # phi = 0, pi/2: distinct cos(2 phi)
        pts = [(m, t) for m in unisolvent_mu for t in unisolvent_tau]
        pts += [(Fraction(rng.randint(-12, 12), 12), Fraction(rng.randint(-15, 15), rng.randint(1, 6)))
                for _ in range(40)]
        pts += [(Fraction(-1), Fraction(2)), (Fraction(1), Fraction(-3, 5))]
        Hs = [tuple(Fraction(x) for x in b) for b in basis]
        Hs += [tuple(Fraction(rng.randint(-30, 30), rng.randint(1, 13)) for _ in range(3)) for _ in range(6)]
        checked = 0
        for H1, H2, H3 in Hs:
            for mu, tau in pts:
                c, s = _circle_point(tau)
                qdot, _, _ = _exact_drift(mu, c, s, H1, H2, H3)
                hphi = H1 * c * c + H2 * s * s
                m = _Dual(mu, 1)
                dmu = (-(m * (1 - m * m)) * (H3 - hphi)).b
                T = _Dual(tau, 1)
                C = (1 - T * T) / (1 + T * T)
                Sn = 2 * T / (1 + T * T)
                dphi = ((H1 - H2) * Sn * C).b / (Fraction(2) / (1 + tau * tau))
                self.assertEqual(dmu, -(H3 - hphi) * (1 - 3 * mu * mu))
                self.assertEqual(dphi, (H1 - H2) * (c * c - s * s))
                self.assertEqual((H1 + H2 + H3) + 3 * qdot + dmu + dphi, 0)
                checked += 1
        DIAG["a2_liouville_points"] = checked

    def test_a3_lrs_drift_reduction_exact(self):
        rng = random.Random(103)
        for _ in range(60):
            H = Fraction(rng.randint(-20, 20), rng.randint(1, 9))
            sh = Fraction(rng.randint(-20, 20), rng.randint(1, 9))
            mu = Fraction(rng.randint(-10, 10), 10)
            c, s = _circle_point(Fraction(rng.randint(-9, 9), rng.randint(1, 5)))
            qdot, mudot, phidot = _exact_drift(mu, c, s, H - sh, H - sh, H + 2 * sh)
            self.assertEqual(phidot, 0)
            self.assertEqual(mudot, -3 * sh * mu * (1 - mu * mu))
            self.assertEqual(qdot, -H - sh * (3 * mu * mu - 1))
            self.assertEqual((H - sh) + (H - sh) + (H + 2 * sh), 3 * H)

    def test_a4_quadrupole_addition_theorem_exact(self):
        rng = random.Random(104)
        for _ in range(60):
            n = _sphere_point(Fraction(rng.randint(-9, 9), rng.randint(1, 5)),
                              Fraction(rng.randint(-9, 9), rng.randint(1, 5)))
            m = _sphere_point(Fraction(rng.randint(-9, 9), rng.randint(1, 5)),
                              Fraction(rng.randint(-9, 9), rng.randint(1, 5)))
            self.assertEqual(sum(x * x for x in n), 1)
            lhs = sum(w * a * b for w, a, b in zip(QUAD_WEIGHTS_EXACT, _basis_exact(n), _basis_exact(m)))
            cos = sum(a * b for a, b in zip(n, m))
            self.assertEqual(lhs, (3 * cos * cos - 1) / 4)
        self.assertEqual(QUAD_WEIGHTS, tuple(float(w) for w in QUAD_WEIGHTS_EXACT))

    def test_a5_discrete_mu_moments_exact(self):
        rng = random.Random(105)
        for _ in range(30):
            nm = rng.randint(1, 9)
            cuts = sorted({Fraction(rng.randint(-99, 99), 100) for _ in range(nm - 1)})
            edges = [Fraction(-1)] + cuts + [Fraction(1)]
            sp2 = sm2 = st2 = 0
            for a, b in zip(edges, edges[1:]):
                d, m2, t2, p2, ea, eb = _mu_poly(a, b)
                self.assertEqual(d * m2, (b ** 3 - a ** 3) / 3)
                self.assertEqual(d * t2, d - (b ** 3 - a ** 3) / 3)
                self.assertEqual(d * p2, ((b ** 3 - b) - (a ** 3 - a)) / 2)
                self.assertEqual(m2 + t2, 1)
                self.assertTrue(t2 > 0 and ea >= 0 and eb >= 0)
                sp2 += d * p2
                sm2 += d * m2
                st2 += d * t2
            self.assertEqual(sp2, 0)
            self.assertEqual(sm2, Fraction(2, 3))
            self.assertEqual(st2, Fraction(4, 3))

    def test_a6_polarized_iq_equals_chandrasekhar_lr_exact(self):
        rng = random.Random(106)
        for _ in range(25):
            nm = rng.randint(1, 7)
            cuts = sorted({Fraction(rng.randint(-49, 49), 50) for _ in range(nm - 1)})
            edges = [Fraction(-1)] + cuts + [Fraction(1)]
            consts = [_mu_poly(a, b) for a, b in zip(edges, edges[1:])]
            d = [c[0] for c in consts]
            m2 = [c[1] for c in consts]
            t2 = [c[2] for c in consts]
            p = [c[3] for c in consts]
            n = len(d)
            I = [Fraction(rng.randint(0, 40), rng.randint(1, 9)) for _ in range(n)]
            Q = [Fraction(rng.randint(-40, 40), rng.randint(1, 9)) for _ in range(n)]
            rate = Fraction(rng.randint(1, 9), rng.randint(1, 4))
            dI, dQ = _pol_iq(d, p, I, Q, rate)
            Il = [(a + b) / 2 for a, b in zip(I, Q)]
            Ir = [(a - b) / 2 for a, b in zip(I, Q)]
            dl, dr = _pol_lr(d, m2, t2, Il, Ir, rate, Fraction(3, 8))
            for j in range(n):
                self.assertEqual(dI[j], dl[j] + dr[j])
                self.assertEqual(dQ[j], dl[j] - dr[j])
            self.assertEqual(sum(dI), 0)
            iso = [dj for dj in d]
            dIi, dQi = _pol_iq(d, p, iso, [0] * n, rate)
            self.assertEqual(dIi, [0] * n)
            self.assertEqual(dQi, [0] * n)


class B_Conservation(unittest.TestCase):
    """(b) Number conservation with ledgers, state layout, linearity, positivity."""

    @staticmethod
    def _cases(seed, count):
        rng = random.Random(seed)
        geoms = [(0.3, -0.1, 0.05), (-0.2, 0.4, 0.1), (0.1, 0.1, -0.3), (0.0, 0.0, 0.0),
                 (0.5, 0.2, 0.9), (-0.7, -0.1, -0.4), (0.25, 0.25, 0.25)]
        for c in range(count):
            nq, nm, nphi = rng.randint(1, 5), rng.randint(1, 5), rng.randint(1, 6)
            q = _random_edges(rng, -2.0, 1.5, nq)
            mu = _random_edges(rng, -1.0, 1.0, nm)
            phi = _random_phi_edges(rng, nphi, rng.choice([0.0, 0.3, 2.9, 5.5]))
            H = geoms[c % len(geoms)] if c < len(geoms) else tuple(rng.uniform(-1, 1) for _ in range(3))
            x = [rng.uniform(-1.0, 3.0) for _ in range(nq * nm * nphi)]
            yield q, mu, phi, H, x

    def test_b1_transport_number_ledger(self):
        worst = 0.0
        n = 0
        for q, mu, phi, H, x in self._cases(201, 60):
            for boundary in BOUNDARIES:
                rhs, lo, hi = sphere_number_rhs(q, mu, phi, x, *H, boundary)
                total = math.fsum(rhs) + lo + hi
                scale = math.fsum(abs(v) for v in rhs) + abs(lo) + abs(hi)
                if scale == 0.0:
                    self.assertEqual(total, 0.0)
                else:
                    worst = max(worst, abs(total) / scale)
                    self.assertLessEqual(abs(total), 1e-14 * scale)
                if boundary == "closed":
                    self.assertEqual((lo, hi), (0.0, 0.0))
                n += 1
        DIAG["b1_cases"] = n
        DIAG["b1_max_relative_sum_residual"] = worst

    def test_b2_state_layout_ledgers_jacobian_band(self):
        rng = random.Random(202)
        q = [-1.0, -0.2, 0.5, 1.4]
        mu = [-1.0, -0.4, 0.3, 1.0]
        phi = _random_phi_edges(rng, 4, 0.7)
        grid = SphereGrid(q, mu, phi)
        lay = StateLayout(grid.nq, grid.nm, grid.nphi)
        idx = sorted(lay.cell(i, j, k) for i in range(3) for j in range(3) for k in range(4))
        ledgers = sorted([0, lay.hi] + [lay.absorbed(i) for i in range(3)])
        self.assertEqual(sorted(idx + ledgers), list(range(lay.n)))
        self.assertEqual(lay.n, 2 + 3 * (12 + 1))
        H = (0.35, -0.15, 0.1)
        kbar = [0.2, 0.0, 0.7]
        y = [rng.uniform(0.0, 2.0) for _ in range(lay.n)]
        out = grid_state_rhs(grid, y, *H, rate=2.5, kbar=kbar)
        scale = math.fsum(abs(v) for v in out)
        self.assertLessEqual(abs(math.fsum(out)), 1e-14 * scale)
        cells = [y[1 + i * 13 + c] for i in range(3) for c in range(12)]
        _, lo, hi = grid_number_rhs(grid, cells, *H)
        self.assertEqual(out[0], lo)
        self.assertEqual(out[lay.hi], hi)
        for i in range(3):
            expect = 0.0
            for c in range(12):
                expect += kbar[i] * y[1 + i * 13 + c]
            self.assertEqual(out[lay.absorbed(i)], expect)
        maxoff = 0
        colsum = 0.0
        for c in range(lay.n):
            e = [0.0] * lay.n
            e[c] = 1.0
            col = grid_state_rhs(grid, e, *H, rate=2.5, kbar=kbar)
            colsum = max(colsum, abs(math.fsum(col)) / max(1.0, math.fsum(abs(v) for v in col)))
            for r, v in enumerate(col):
                if v != 0.0:
                    maxoff = max(maxoff, abs(r - c))
        self.assertEqual(maxoff, lay.bandwidth)
        self.assertLessEqual(colsum, 1e-14)
        DIAG["b2_bandwidth"] = {"M": grid.M, "observed": maxoff, "contract": lay.bandwidth}

    def test_b3_linearity_jvp_and_select(self):
        rng = random.Random(203)
        worst = 0.0
        for q, mu, phi, H, x in self._cases(204, 25):
            yv = [rng.uniform(-2.0, 2.0) for _ in x]
            ra = sphere_number_rhs(q, mu, phi, x, *H)[0]
            rb = sphere_number_jvp(q, mu, phi, yv, *H)
            rab = sphere_number_rhs(q, mu, phi, [a + 2.0 * b for a, b in zip(x, yv)], *H)[0]
            scale = max(1e-300, max(abs(v) for v in ra + rb + rab))
            err = max(abs(z - a - 2.0 * b) for z, a, b in zip(rab, ra, rb)) / scale
            worst = max(worst, err)
            self.assertLessEqual(err, 1e-14)
            # The operator is linear in the velocity geometry for a fixed upwind pattern.
            H2 = tuple(rng.uniform(-1, 1) for _ in range(3))
            r1 = sphere_number_rhs(q, mu, phi, x, *H, select=H)[0]
            r2 = sphere_number_rhs(q, mu, phi, x, *H2, select=H)[0]
            comb = tuple(0.7 * a - 1.3 * b for a, b in zip(H, H2))
            r3 = sphere_number_rhs(q, mu, phi, x, *comb, select=H)[0]
            scale = max(1e-300, max(abs(v) for v in r1 + r2 + r3))
            self.assertLessEqual(max(abs(c - 0.7 * a + 1.3 * b) for a, b, c in zip(r1, r2, r3)) / scale,
                                 1e-13)
        # f_t on a linear geometry path equals a central difference of the state RHS.
        grid = SphereGrid([-1.0, 0.0, 0.8, 1.5], [-1.0, -0.3, 0.4, 1.0], uniform_phi_edges(5, 0.4))
        lay = StateLayout(3, 3, 5)
        y = [rng.uniform(0.0, 2.0) for _ in range(lay.n)]
        H0, dH = (0.3, -0.2, 0.05), (0.1, 0.07, -0.04)
        kb, dkb = [0.3, 0.1, 0.05], [0.05, -0.02, 0.01]
        ft = grid_state_partial_t(grid, y, H0, dH, dkb)
        h = 1e-3
        Hp = tuple(a + h * b for a, b in zip(H0, dH))
        Hm = tuple(a - h * b for a, b in zip(H0, dH))
        fp = grid_state_rhs(grid, y, *Hp, 1.0, [a + h * b for a, b in zip(kb, dkb)])
        fm = grid_state_rhs(grid, y, *Hm, 1.0, [a - h * b for a, b in zip(kb, dkb)])
        fd = [(a - b) / (2 * h) for a, b in zip(fp, fm)]
        scale = max(abs(v) for v in ft)
        self.assertLessEqual(max(abs(a - b) for a, b in zip(ft, fd)) / scale, 1e-10)
        DIAG["b3_max_linearity_residual"] = worst

    def test_b4_euler_positivity_under_cfl(self):
        rng = random.Random(205)
        worst = 0.0
        for q, mu, phi, H, x in self._cases(206, 40):
            x = [abs(v) * (rng.random() < 0.7) for v in x]
            for boundary in BOUNDARIES:
                dt = 0.999 * sphere_transport_cfl(q, mu, phi, *H, boundary)
                if not math.isfinite(dt):
                    continue
                rhs, _, _ = sphere_number_rhs(q, mu, phi, x, *H, boundary)
                scale = max(max(x), 1e-300)
                low = min(a + dt * b for a, b in zip(x, rhs)) / scale
                worst = min(worst, low)
                self.assertGreaterEqual(low, -1e-14)
                # With Thomson rate G: dt (1/cfl + G) <= 1 suffices (gain kernel >= 0).
                G = 3.0
                grid = SphereGrid(q, mu, phi)
                dt2 = 0.999 / (1.0 / sphere_transport_cfl(q, mu, phi, *H, boundary) + G)
                lay = StateLayout(grid.nq, grid.nm, grid.nphi)
                y = [0.0] * lay.n
                for i in range(grid.nq):
                    for c in range(grid.M):
                        y[1 + i * (grid.M + 1) + c] = x[i * grid.M + c]
                f = grid_state_rhs(grid, y, *H, rate=G, boundary=boundary)
                for i in range(grid.nq):
                    for c in range(grid.M):
                        s = 1 + i * (grid.M + 1) + c
                        self.assertGreaterEqual((y[s] + dt2 * f[s]) / scale, -1e-14)
        DIAG["b4_min_scaled_euler_content"] = worst

    def test_b5_isotropic_expansion_and_frozen_geometry(self):
        q = [-1.0, 0.2, 0.9, 2.0]
        mu = [-1.0, -0.7, 0.1, 0.8, 1.0]
        phi = uniform_phi_edges(5, 1.1)
        grid = SphereGrid(q, mu, phi)
        w = [grid.dm[j] * grid.dphi[k] * (1.0 + 0.0) for j in range(4) for k in range(5)]
        x = [nq * a for nq in (1.0, 3.0, 2.0) for a in w]
        rhs, _, _ = grid_number_rhs(grid, x, 0.1, 0.1, 0.1)
        for i in range(3):
            ratios = [rhs[i * 20 + c] / w[c] for c in range(20)]
            self.assertLessEqual(max(ratios) - min(ratios), 1e-14 * max(map(abs, ratios)))
        frozen, lo, hi = grid_number_rhs(grid, x, 0.0, 0.0, 0.0)
        self.assertEqual(frozen, [0.0] * 60)
        self.assertEqual((lo, hi), (0.0, 0.0))

    def test_b6_jacobian_columns_bitwise_and_parity_fixture(self):
        """J[:, c] == state_rhs(e_c) bit for bit, by the entries template; record golden hex."""
        rng = random.Random(207)
        configs = [(parity_fixture()["q"], parity_fixture()["mu"], parity_fixture()["phi"], (0.3, -0.1, 0.05),
                    2.5, [0.2, 0.0, 0.7], "outflow"),
                   ([-1.0, 0.5], [-1.0, 1.0], [0.0, TAU], (0.1, -0.3, 0.2), 0.0, None, "closed"),
                   (_random_edges(rng, -1, 1, 2), _random_edges(rng, -1, 1, 3), _random_phi_edges(rng, 2, 4.0),
                    (-0.4, 0.2, 0.3), 1e3, [0.5, 0.1], "outflow"),
                   (_random_edges(rng, -1, 1, 3), _random_edges(rng, -1, 1, 2), _random_phi_edges(rng, 5, 0.0),
                    (0.2, 0.2, -0.1), 0.7, None, "closed")]
        compared = 0
        for q, mu, phi, H, rate, kbar, boundary in configs:
            grid = SphereGrid(q, mu, phi)
            lay = StateLayout(grid.nq, grid.nm, grid.nphi)
            J = grid_state_jacobian_dense(grid, *H, rate, kbar, boundary)
            for c in range(lay.n):
                e = [0.0] * lay.n
                e[c] = 1.0
                col = grid_state_rhs(grid, e, *H, rate, kbar, boundary)
                for r in range(lay.n):
                    self.assertEqual(J[r][c].hex(), col[r].hex(), (r, c))
                    compared += 1
        first = parity_fixture_outputs()
        second = parity_fixture_outputs()
        self.assertEqual(first, second)
        DIAG["b6_bitwise_jacobian_entries_compared"] = compared
        DIAG["parity_fixture"] = first


class C_LRSReduction(unittest.TestCase):
    """(c) H1 = H2: the phi-summed 3D transport is the P01/P02 LRS operator."""

    @classmethod
    def setUpClass(cls):
        cls.lrs = load_lrs_oracle()

    @staticmethod
    def _phi_grids():
        rng = random.Random(301)
        return [[0.0, TAU], uniform_phi_edges(2), uniform_phi_edges(3, 0.4),
                _random_phi_edges(rng, 7, 1.1), uniform_phi_edges(16), _random_phi_edges(rng, 5, 6.0)]

    def test_c1_phi_summed_transport_equals_lrs_oracle(self):
        rng = random.Random(302)
        worst = 0.0
        cases = 0
        geoms = [(0.1, 0.07), (-0.2, 0.05), (0.03, -0.3), (0.4, 0.4), (0.0, 0.0), (-0.1, -0.08)]
        for phi in self._phi_grids():
            for (H, s) in geoms:
                nq, nm = rng.randint(1, 5), rng.randint(1, 6)
                q = _random_edges(rng, -2.0, 2.0, nq)
                mu = _random_edges(rng, -1.0, 1.0, nm)
                grid = SphereGrid(q, mu, phi)
                n2 = [rng.uniform(0.0, 5.0) for _ in range(nq * nm)]
                x = [n2[i * nm + j] * grid.g[k] for i in range(nq) for j in range(nm) for k in range(grid.nphi)]
                for boundary in BOUNDARIES:
                    r2, out2 = self.lrs.lrs_number_rhs(q, mu, n2, H, s, boundary)
                    r3, lo, hi = grid_number_rhs(grid, x, H - s, H - s, H + 2 * s, boundary)
                    summed = [math.fsum(r3[i * grid.M + j * grid.nphi + k] for k in range(grid.nphi))
                              for i in range(nq) for j in range(nm)]
                    scale = max([abs(v) for v in r2] + [abs(out2)])
                    if scale == 0.0:
                        self.assertEqual(max(map(abs, summed)), 0.0)
                        continue
                    err = max(max(abs(a - b) for a, b in zip(summed, r2)), abs(lo + hi - out2)) / scale
                    worst = max(worst, err)
                    self.assertLessEqual(err, 1e-14)
                    cases += 1
        DIAG["c1_cases"] = cases
        DIAG["c1_max_relative_error"] = worst

    def test_c2_lrs_preserves_axisymmetry(self):
        rng = random.Random(303)
        worst = 0.0
        for phi in self._phi_grids():
            q = _random_edges(rng, -1.0, 1.0, 3)
            mu = _random_edges(rng, -1.0, 1.0, 4)
            grid = SphereGrid(q, mu, phi)
            n2 = [rng.uniform(0.5, 3.0) for _ in range(12)]
            x = [n2[i * 4 + j] * grid.g[k] for i in range(3) for j in range(4) for k in range(grid.nphi)]
            r3, _, _ = grid_number_rhs(grid, x, 0.12, 0.12, -0.2)
            scale = max(abs(v / grid.g[c % grid.nphi]) for c, v in enumerate(r3))
            for i in range(3):
                for j in range(4):
                    vals = [r3[grid.cell(i, j, k)] / grid.g[k] for k in range(grid.nphi)]
                    worst = max(worst, (max(vals) - min(vals)) / scale)
        self.assertLessEqual(worst, 1e-14)
        DIAG["c2_max_axisymmetry_defect"] = worst

    def test_c3_theta_only_shortcut_not_closed(self):
        """Axisymmetric data under triaxial shear: the phi-sum agrees with a theta-only operator at
        first order in time, but the axisymmetric subspace is not invariant and the second time
        derivative differs by exactly hd^2 C (fixed upwind pattern)."""
        rng = random.Random(304)
        q = _random_edges(rng, -1.5, 1.5, 4)
        mu = _random_edges(rng, -1.0, 1.0, 5)
        phi = _random_phi_edges(rng, 8, 0.2)
        grid = SphereGrid(q, mu, phi)
        nq, nm, nphi = grid.nq, grid.nm, grid.nphi
        n2 = [rng.uniform(0.5, 3.0) for _ in range(nq * nm)]
        x = [n2[i * nm + j] * grid.g[k] for i in range(nq) for j in range(nm) for k in range(nphi)]
        H3 = 0.4
        rec = {}
        for delta in (0.02, 0.01):
            H1, H2 = 0.1 + delta, 0.1 - delta
            geo = _geometry(H1, H2, H3)
            gk, vq, hd = _velocities(grid, geo)
            self.assertTrue(all(g > 0 for g in gk) and all(v < 0 for v in vq) and hd > 0)
            hbar = geo[3]
            s = (H3 - hbar) / 3.0
            Hiso = hbar + s
            y1, _, _ = grid_number_rhs(grid, x, H1, H2, H3)
            y2, _, _ = grid_number_rhs(grid, y1, H1, H2, H3)
            r2a, _ = self.lrs.lrs_number_rhs(q, mu, n2, Hiso, s)
            r2b, _ = self.lrs.lrs_number_rhs(q, mu, r2a, Hiso, s)
            sum1 = [math.fsum(y1[i * grid.M + j * nphi + k] for k in range(nphi)) for i in range(nq) for j in range(nm)]
            sum2 = [math.fsum(y2[i * grid.M + j * nphi + k] for k in range(nphi)) for i in range(nq) for j in range(nm)]
            first = max(abs(a - b) for a, b in zip(sum1, r2a)) / max(map(abs, r2a))
            second = max(abs(a - b) for a, b in zip(sum2, r2b))
            nonaxi = max(abs(y1[grid.cell(i, j, k)] / grid.g[k] - sum1[i * nm + j])
                         for i in range(nq) for j in range(nm) for k in range(nphi))
            rec[delta] = (first, second, nonaxi, max(map(abs, r2b)))
        f02, s02, n02, sc = rec[0.02]
        f01, s01, n01, _ = rec[0.01]
        self.assertLessEqual(max(f02, f01), 1e-14)
        self.assertGreater(s02 / sc, 1e-6)
        self.assertLess(abs(s02 / s01 - 4.0), 1e-6)
        self.assertLess(abs(n02 / n01 - 2.0), 1e-6)
        DIAG["c3_theta_only"] = {"first_derivative_rel_gap": [f02, f01],
                                 "second_derivative_gap": [s02, s01], "ratio_second": s02 / s01,
                                 "nonaxisymmetric_part": [n02, n01], "ratio_nonaxi": n02 / n01}


class D_Thomson(unittest.TestCase):
    """(d) Rank-6 Thomson: closed-form averages, conservation, isotropic state, positivity."""

    def test_d1_cell_average_closed_forms_vs_mpmath(self):
        import mpmath as mp
        mp.mp.dps = 40
        mu_cells = [(-1.0, -0.9), (-0.3, 0.2), (1.0 - 1e-6, 1.0), (-1.0, -1.0 + 1e-6),
                    (0.3, 0.3 + 1e-9), (-1.0, 1.0), (-0.2, 0.7), (0.999, 1.0)]
        worst = 0.0
        for a, b in mu_cells:
            d, m2, t2, p2, r = mu_cell_constants(a, b)
            A, Bm = mp.mpf(a), mp.mpf(b)
            W = Bm - A
            ex = [mp.quad(lambda u: u * u, [A, Bm]) / W, mp.quad(lambda u: 1 - u * u, [A, Bm]) / W,
                  mp.quad(lambda u: (3 * u * u - 1) / 2, [A, Bm]) / W,
                  mp.quad(lambda u: u * mp.sqrt(1 - u * u), [A, Bm]) / W]
            for got, want in zip((m2, t2, p2, r), ex):
                worst = max(worst, float(abs(mp.mpf(got) - want)))
        phi_cells = [(0.0, 1e-7), (3.0, 3.0 + 1e-9), (6.0, TAU), (0.0, TAU), (1.0, 4.0), (0.3, 0.3 + TAU / 7)]
        naive = 0.0
        for a, b in phi_cells:
            _, g, cphi, sphi, c2, s2h = phi_cell_constants(a, b)
            A, Bm = mp.mpf(a), mp.mpf(b)
            W = Bm - A
            ex = [(mp.sin(Bm) - mp.sin(A)) / W, (mp.cos(A) - mp.cos(Bm)) / W,
                  (mp.sin(2 * Bm) - mp.sin(2 * A)) / (2 * W), (mp.sin(Bm) ** 2 - mp.sin(A) ** 2) / (2 * W)]
            for got, want in zip((cphi, sphi, c2, s2h), ex):
                worst = max(worst, float(abs(mp.mpf(got) - want)))
            naive = max(naive, float(abs(mp.mpf((math.sin(b) - math.sin(a)) / (b - a)) - ex[0])))
        self.assertLessEqual(worst, 1e-14)
        DIAG["d1_max_abs_cell_average_error"] = worst
        DIAG["d1_naive_difference_quotient_error_cos"] = naive

    def test_d2_group_number_conservation(self):
        rng = random.Random(402)
        worst = 0.0
        for _ in range(60):
            nm, nphi = rng.randint(1, 7), rng.randint(1, 9)
            mu = _random_edges(rng, -1.0, 1.0, nm)
            phi = _random_phi_edges(rng, nphi, rng.uniform(0.0, 6.0))
            x = [rng.uniform(-1.0, 4.0) for _ in range(nm * nphi)]
            G = rng.uniform(0.1, 1e6)
            rhs = thomson6_rhs(mu, phi, x, G)
            err = abs(math.fsum(rhs)) / (G * math.fsum(abs(v) for v in x))
            worst = max(worst, err)
            self.assertLessEqual(err, 1e-14)
        DIAG["d2_max_relative_number_residual"] = worst

    def test_d3_isotropic_state_stationary(self):
        rng = random.Random(403)
        worst = 0.0
        for _ in range(40):
            nm, nphi = rng.randint(1, 8), rng.randint(1, 10)
            mu = _random_edges(rng, -1.0, 1.0, nm)
            phi = _random_phi_edges(rng, nphi, rng.uniform(0.0, 6.0))
            grid = SphereGrid([0.0, 1.0], mu, phi)
            iso = [grid.dm[j] * grid.dphi[k] for j in range(nm) for k in range(nphi)]
            G = rng.uniform(0.5, 50.0)
            rhs = thomson6_rhs(mu, phi, iso, G)
            err = max(map(abs, rhs)) / (G * max(iso))
            worst = max(worst, err)
            self.assertLessEqual(err, 1e-14)
        DIAG["d3_max_isotropic_residual"] = worst

    def test_d4_discrete_kernel_is_cell_averaged_continuum_kernel(self):
        import numpy as np
        rng = random.Random(404)
        mu = _random_edges(rng, -1.0, 1.0, 3)
        phi = _random_phi_edges(rng, 4, 0.3)
        grid = SphereGrid([0.0, 1.0], mu, phi)
        x, w = np.polynomial.legendre.leggauss(10)
        dirs, wts = [], []
        for j in range(grid.nm):
            ta, tb = math.acos(grid.mu[j + 1]), math.acos(grid.mu[j])
            for k in range(grid.nphi):
                pa, pb = grid.phi[k], grid.phi[k + 1]
                th = 0.5 * (tb + ta) + 0.5 * (tb - ta) * x
                ph = 0.5 * (pb + pa) + 0.5 * (pb - pa) * x
                TH, PH = np.meshgrid(th, ph, indexing="ij")
                WW = np.outer(w, w) * 0.25 * (tb - ta) * (pb - pa) * np.sin(TH)
                n = np.stack([np.sin(TH) * np.cos(PH), np.sin(TH) * np.sin(PH), np.cos(TH)], -1).reshape(-1, 3)
                dirs.append(n)
                wts.append(WW.reshape(-1) / (grid.dm[j] * grid.dphi[k]))
        M = grid.M
        worst = 0.0
        kmin = math.inf
        for c in range(M):
            for cc in range(M):
                cos = dirs[c] @ dirs[cc].T
                K = 1.0 + 0.5 * (1.5 * cos * cos - 0.5)
                quad = float(wts[c] @ K @ wts[cc])
                disc = 1.0 + sum(QUAD_WEIGHTS[m] * grid.B[m][c] * grid.B[m][cc] for m in range(5))
                worst = max(worst, abs(quad - disc))
                kmin = min(kmin, disc)
        self.assertLessEqual(worst, 1e-13)
        self.assertGreaterEqual(kmin, 0.75 - 1e-15)
        DIAG["d4_kernel_vs_quadrature_max_abs"] = worst
        DIAG["d4_min_discrete_kernel"] = kmin

    def test_d5_euler_positivity_dt_gamma_le_one(self):
        rng = random.Random(405)
        for _ in range(60):
            nm, nphi = rng.randint(1, 6), rng.randint(1, 8)
            mu = _random_edges(rng, -1.0, 1.0, nm)
            phi = _random_phi_edges(rng, nphi, rng.uniform(0.0, 6.0))
            x = [rng.uniform(0.0, 3.0) * (rng.random() < 0.5) for _ in range(nm * nphi)]
            G = rng.uniform(0.1, 100.0)
            rhs = thomson6_rhs(mu, phi, x, G)
            for frac in (0.3, 1.0):
                dt = frac / G
                low = min(a + dt * b for a, b in zip(x, rhs))
                self.assertGreaterEqual(low, -1e-14 * max(max(x), 1e-300))

    def test_d6_full_period_and_pole_sums_vanish(self):
        """The algebraic conservation identity: sum_c Omega_c B_m,c = 0 (exact edges, 50 digits)."""
        import mpmath as mp
        mp.mp.dps = 50
        worst = mp.mpf(0)
        for nphi, off in ((1, 0), (3, mp.mpf("0.4")), (7, mp.mpf("1.3")), (12, 0)):
            edges = [off + 2 * mp.pi * k / nphi for k in range(nphi + 1)]
            sums = [mp.mpf(0)] * 4
            for a, b in zip(edges, edges[1:]):
                dl = b - a
                h = dl / 2
                mid = (a + b) / 2
                sinc_h = mp.sin(h) / h
                sinc = mp.sin(dl) / dl
                vals = (mp.cos(mid) * sinc_h, mp.sin(mid) * sinc_h, mp.cos(a + b) * sinc,
                        mp.sin(a + b) * sinc / 2)
                sums = [s + dl * v for s, v in zip(sums, vals)]
            worst = max([worst] + [abs(s) for s in sums])
        self.assertLess(worst, mp.mpf("1e-45"))
        DIAG["d6_max_period_sum"] = float(worst)


class E_Rank(unittest.TestCase):
    """(e) T + Gamma I = Gamma diag(om) V D V^T has rank 6 on generic grids."""

    @staticmethod
    def _matrix(mu, phi, G):
        import numpy as np
        grid = SphereGrid([0.0, 1.0], mu, phi)
        M = grid.M
        T = np.zeros((M, M))
        for c in range(M):
            e = [0.0] * M
            e[c] = 1.0
            T[:, c] = thomson6_rhs(mu, phi, e, G)
        return grid, T

    @staticmethod
    def _rank(A):
        import numpy as np
        s = np.linalg.svd(A, compute_uv=False)
        tol = s[0] * A.shape[0] * np.finfo(float).eps * 10
        return int((s > tol).sum()), s

    def test_e1_rank_six_generic(self):
        import numpy as np
        rng = random.Random(501)
        cases = [(_random_edges(rng, -1.0, 1.0, 4), _random_phi_edges(rng, 5, 0.3)),
                 (uniform_mu_edges(3), uniform_phi_edges(6)),
                 (_random_edges(rng, -1.0, 1.0, 6), _random_phi_edges(rng, 8, 2.0)),
                 (uniform_mu_edges(4), uniform_phi_edges(3, 0.1))]
        rep = []
        for mu, phi in cases:
            G = 7.0
            grid, T = self._matrix(mu, phi, G)
            self.assertGreaterEqual(grid.M, 12)
            rank, s = self._rank(T + G * np.eye(grid.M))
            self.assertEqual(rank, 6)
            rep.append({"nm": grid.nm, "nphi": grid.nphi, "sigma6": float(s[5]),
                        "sigma7": float(s[6]) if len(s) > 6 else 0.0})
        DIAG["e1_rank_six"] = rep

    def test_e2_low_rank_factorization(self):
        import numpy as np
        rng = random.Random(502)
        mu = _random_edges(rng, -1.0, 1.0, 5)
        phi = _random_phi_edges(rng, 6, 1.0)
        G = 3.0
        grid, T = self._matrix(mu, phi, G)
        U, V = thomson6_factors(mu, phi)
        Um, Vm = np.array(U).T, np.array(V).T
        recon = G * (Um @ Vm.T) - G * np.eye(grid.M)
        err = float(np.max(np.abs(recon - T)) / G)
        self.assertLessEqual(err, 1e-14)
        DIAG["e2_factor_reconstruction"] = err

    def test_e3_degenerate_symmetric_grids(self):
        """Harmonics whose cell averages vanish identically drop the rank (predicted, then checked)."""
        import numpy as np
        rep = {}
        for name, mu, phi, expect in (("nphi4_uniform", uniform_mu_edges(4), uniform_phi_edges(4), 5),
                                      ("nphi2_uniform", uniform_mu_edges(4), uniform_phi_edges(2), 3),
                                      ("nphi1", uniform_mu_edges(5), [0.0, TAU], 2),
                                      ("nm2_symmetric", uniform_mu_edges(2), uniform_phi_edges(6), 5)):
            G = 2.0
            grid, T = self._matrix(mu, phi, G)
            rank, s = self._rank(T + G * np.eye(grid.M))
            self.assertEqual(rank, expect, name)
            rep[name] = rank
        DIAG["e3_degenerate_ranks"] = rep


class F_ThomsonLRS(unittest.TestCase):
    """(f) phi-summed rank-6 Thomson on axisymmetric data equals the LRS rank-2 Thomson."""

    def test_f1_phi_summed_thomson_equals_lrs_rank2(self):
        lrs = load_lrs_oracle()
        rng = random.Random(601)
        worst = 0.0
        quad = 0.0
        for phi in C_LRSReduction._phi_grids():
            for _ in range(8):
                nm = rng.randint(1, 8)
                mu = _random_edges(rng, -1.0, 1.0, nm)
                grid = SphereGrid([0.0, 1.0], mu, phi)
                n2 = [rng.uniform(0.0, 5.0) for _ in range(nm)]
                x = [n2[j] * grid.g[k] for j in range(nm) for k in range(grid.nphi)]
                G = rng.uniform(0.1, 1e4)
                r3 = thomson6_rhs(mu, phi, x, G)
                summed = [math.fsum(r3[j * grid.nphi + k] for k in range(grid.nphi)) for j in range(nm)]
                r2 = lrs.thomson_rhs(mu, n2, G)
                err = max(abs(a - b) for a, b in zip(summed, r2)) / (G * max(n2))
                worst = max(worst, err)
                self.assertLessEqual(err, 1e-14)
                for m in range(1, 5):
                    mom = math.fsum(grid.B[m][c] * x[c] for c in range(grid.M))
                    quad = max(quad, abs(mom) / max(n2))
        DIAG["f1_max_relative_error"] = worst
        DIAG["f1_max_nonaxisymmetric_moment"] = quad


def _H_t(t, c, dd, w):
    return [c[k] / (1.0 + t) + dd[k] * math.cos(w[k] * t) for k in range(3)]


def _lnA_t(t, c, dd, w):
    return [c[k] * math.log1p(t) + dd[k] * math.sin(w[k] * t) / w[k] for k in range(3)]


def smooth_U0(q, n):
    """Smooth positive test data on the sphere (all l <= 2 shapes, no axial symmetry)."""
    return math.exp(-(q * q) / 0.5) * (1.0 + 0.4 * n[0] + 0.3 * n[0] * n[1] - 0.2 * n[2] + 0.25 * n[2] * n[2])


def _assemble_transport_csr(grid, H, boundary):
    """Exact sparse matrix of the oracle transport by 27-colour probing (nphi % 3 == 0)."""
    import scipy.sparse as sp
    nq, nm, nphi, M = grid.nq, grid.nm, grid.nphi, grid.M
    if nphi % 3:
        raise ValueError("colouring needs nphi % 3 == 0")
    n = nq * M
    rows, cols, vals = [], [], []
    for ci in range(3):
        for cj in range(3):
            for ck in range(3):
                x = [0.0] * n
                members = []
                for i in range(ci, nq, 3):
                    for j in range(cj, nm, 3):
                        for k in range(ck, nphi, 3):
                            c = grid.cell(i, j, k)
                            x[c] = 1.0
                            members.append((i, j, k, c))
                if not members:
                    continue
                y, _, _ = grid_number_rhs(grid, x, *H, boundary)
                for i, j, k, c in members:
                    stencil = {c}
                    if i > 0:
                        stencil.add(grid.cell(i - 1, j, k))
                    if i < nq - 1:
                        stencil.add(grid.cell(i + 1, j, k))
                    if j > 0:
                        stencil.add(grid.cell(i, j - 1, k))
                    if j < nm - 1:
                        stencil.add(grid.cell(i, j + 1, k))
                    stencil.add(grid.cell(i, j, (k - 1) % nphi))
                    stencil.add(grid.cell(i, j, (k + 1) % nphi))
                    for r in stencil:
                        if y[r] != 0.0:
                            rows.append(r)
                            cols.append(c)
                            vals.append(y[r])
    return sp.csr_matrix((vals, (rows, cols)), shape=(n, n))


class G_Characteristics(unittest.TestCase):
    """(g) Exact triaxial characteristics vs RK45 rays; FV convergence trend."""

    def test_g1_closed_form_vs_rk45_rays(self):
        from scipy.integrate import solve_ivp
        c, dd, w = (0.4, -0.15, 0.2), (0.1, 0.25, -0.2), (1.3, 0.7, 2.1)
        tf = 1.7
        A = [math.exp(v) for v in _lnA_t(tf, c, dd, w)]

        def f(t, y):
            q, mu, phi, _ = y
            H1, H2, H3 = _H_t(t, c, dd, w)
            cp, sp_ = math.cos(phi), math.sin(phi)
            hphi = H1 * cp * cp + H2 * sp_ * sp_
            one = 1.0 - mu * mu
            return [-(H1 * one * cp * cp + H2 * one * sp_ * sp_ + H3 * mu * mu),
                    -mu * one * (H3 - hphi), (H1 - H2) * sp_ * cp,
                    (H3 - hphi) * (1.0 - 3.0 * mu * mu) - (H1 - H2) * (cp * cp - sp_ * sp_)]

        rng = random.Random(701)
        pts = [(rng.uniform(-1, 1), rng.uniform(-0.99, 0.99), rng.uniform(0, TAU)) for _ in range(16)]
        pts += [(0.2, 0.999, 0.3), (-0.4, -0.9995, 5.9), (0.0, 0.0, 0.0), (0.7, 0.5, TAU - 1e-3)]
        worst = 0.0
        for q, mu, phi in pts:
            sol = solve_ivp(f, (tf, 0.0), [q, mu, phi, 0.0], method="RK45", rtol=1e-12, atol=1e-14)
            self.assertTrue(sol.success)
            q0o, mu0o, phi0o, L0 = sol.y[:, -1]
            q0, mu0, phi0, factor = characteristic_map(q, mu, phi, A)
            dphi = (phi0o - phi0 + math.pi) % TAU - math.pi
            errs = (abs(q0o - q0), abs(mu0o - mu0), abs(dphi), abs(math.log(factor) + L0))
            worst = max(worst, max(errs))
            self.assertLessEqual(max(errs), 1e-9)
        DIAG["g1_max_ray_error"] = worst

    def test_g2_closed_form_jacobian_is_liouville_factor(self):
        rng = random.Random(702)
        A = (1.35, 0.8, 1.1)
        worst = 0.0
        for _ in range(20):
            q, mu, phi = rng.uniform(-1, 1), rng.uniform(-0.95, 0.95), rng.uniform(0, TAU)
            h = 1e-6
            J = []
            for d in range(3):
                pt_p, pt_m = [q, mu, phi], [q, mu, phi]
                pt_p[d] += h
                pt_m[d] -= h
                fp = characteristic_map(*pt_p, A)
                fm = characteristic_map(*pt_m, A)
                col = [(fp[0] - fm[0]) / (2 * h), (fp[1] - fm[1]) / (2 * h),
                       (((fp[2] - fm[2] + math.pi) % TAU) - math.pi) / (2 * h)]
                J.append(col)
            det = (J[0][0] * (J[1][1] * J[2][2] - J[2][1] * J[1][2])
                   - J[1][0] * (J[0][1] * J[2][2] - J[2][1] * J[0][2])
                   + J[2][0] * (J[0][1] * J[1][2] - J[1][1] * J[0][2]))
            factor = characteristic_map(q, mu, phi, A)[3]
            worst = max(worst, _rel(det, factor))
            self.assertLessEqual(_rel(det, factor), 1e-7)
        DIAG["g2_max_rel_jacobian_vs_factor"] = worst

    def test_g3_quadrature_helper(self):
        import numpy as np
        for n in (1, 2, 3, 5, 8):
            xs, ws = gauss_legendre(n)
            xr, wr = np.polynomial.legendre.leggauss(n)
            self.assertLessEqual(max(abs(a - b) for a, b in zip(sorted(xs), sorted(xr))), 1e-15)
            self.assertLessEqual(max(abs(a - b) for a, b in zip(sorted(ws), sorted(wr))), 1e-14)
        one = (1.0, 1.0, 1.0)
        a, b, al, be = -0.6, 0.35, 0.4, 2.2
        got = exact_cell_integral(0.0, 0.5, a, b, al, be, one, lambda q, n: 1.0, 10)
        self.assertLess(_rel(got, 0.5 * (b - a) * (be - al)), 1e-13)
        got = exact_cell_integral(0.0, 0.5, a, b, al, be, one, lambda q, n: n[2] * n[2], 10)
        self.assertLess(_rel(got, 0.5 * (b ** 3 - a ** 3) / 3 * (be - al)), 1e-13)
        got = exact_cell_integral(0.0, 0.5, a, b, al, be, one, lambda q, n: n[0] * n[1], 10)
        want = 0.5 * ((b - b ** 3 / 3) - (a - a ** 3 / 3)) * (math.sin(be) ** 2 - math.sin(al) ** 2) / 2
        self.assertLess(_rel(got, want), 1e-12)
        # Contents are conserved by the exact collisionless map (whole-line support).
        q = uniform_edges(-4.0, 4.0, 16)
        mu = uniform_mu_edges(4)
        phi = uniform_phi_edges(6, 0.25)
        n0 = exact_cell_contents(q, mu, phi, one, smooth_U0, 8)
        n1 = exact_cell_contents(q, mu, phi, (math.exp(0.3), math.exp(-0.1), math.exp(0.05)), smooth_U0, 8)
        DIAG["g3_exact_total_relative_drift"] = abs(math.fsum(n1) - math.fsum(n0)) / math.fsum(n0)
        self.assertLess(DIAG["g3_exact_total_relative_drift"], 1e-6)

    def test_g4_first_order_convergence_trend(self):
        from scipy.sparse.linalg import expm_multiply
        import numpy as np
        H = (0.3, -0.1, 0.05)
        T = 1.0
        A = tuple(math.exp(h * T) for h in H)
        levels = [(12, 6, 6), (24, 12, 12), (48, 24, 24)]
        report = []
        for nq, nm, nphi in levels:
            t0 = time.perf_counter()
            q = uniform_edges(-3.0, 3.0, nq)
            mu = uniform_mu_edges(nm)
            phi = uniform_phi_edges(nphi, 0.25)
            grid = SphereGrid(q, mu, phi)
            mat = _assemble_transport_csr(grid, H, "outflow")
            rng = random.Random(nq)
            probe = [rng.uniform(-1, 1) for _ in range(grid.nq * grid.M)]
            ref = grid_number_rhs(grid, probe, *H)[0]
            got = mat @ np.array(probe)
            self.assertLessEqual(float(np.max(np.abs(got - np.array(ref)))) / max(map(abs, ref)), 1e-14)
            n0 = np.array(exact_cell_contents(q, mu, phi, (1.0, 1.0, 1.0), smooth_U0, 3))
            nex = np.array(exact_cell_contents(q, mu, phi, A, smooth_U0, 3))
            nfv = expm_multiply(mat * T, n0)
            err = float(np.sum(np.abs(nfv - nex)) / np.sum(np.abs(nex)))
            report.append({"nq": nq, "nm": nm, "nphi": nphi, "cells": grid.nq * grid.M,
                           "rel_L1_error": err, "wall_s": time.perf_counter() - t0})
        for a, b in zip(report, report[1:]):
            b["observed_order"] = math.log2(a["rel_L1_error"] / b["rel_L1_error"])
        DIAG["g4_convergence"] = {"H": H, "T": T, "U0": "exp(-q^2/0.5)(1+0.4n1+0.3n1n2-0.2n3+0.25n3^2)",
                                  "q_domain": [-3.0, 3.0], "phi_offset": 0.25, "boundary": "outflow",
                                  "time_integration": "scipy expm_multiply of the probed oracle matrix",
                                  "quadrature": "3-point Gauss-Legendre in (q, theta, phi) per cell",
                                  "timing_authority": "diagnostic_only", "levels": report}
        errs = [r["rel_L1_error"] for r in report]
        self.assertTrue(errs[0] > errs[1] > errs[2])


class H_Polarization(unittest.TestCase):
    """(h) Axisymmetric polarized Thomson: coherency positivity contract and clipping counterexample."""

    def _state(self, rng, nm):
        mu = _random_edges(rng, -1.0, 1.0, nm)
        d = [b - a for a, b in zip(mu, mu[1:])]
        I = [dj * rng.uniform(0.0, 2.0) * (rng.random() < 0.85) for dj in d]
        rho = [rng.choice([-1.0, 1.0, 0.0, rng.uniform(-1, 1), rng.uniform(-1, 1)]) for _ in d]
        Q = [r * i for r, i in zip(rho, I)]
        return mu, I, Q

    def test_h1_iq_form_matches_lr_kernel_and_unpolarized_limit(self):
        lrs = load_lrs_oracle()
        rng = random.Random(801)
        worst = 0.0
        for _ in range(40):
            nm = rng.randint(1, 9)
            mu, I, Q = self._state(rng, nm)
            G = rng.uniform(0.5, 20.0)
            dI, dQ = polarized_thomson_iq_rhs(mu, I, Q, G)
            Il = [(a + b) / 2 for a, b in zip(I, Q)]
            Ir = [(a - b) / 2 for a, b in zip(I, Q)]
            dl, dr = polarized_thomson_lr_rhs(mu, Il, Ir, G)
            scale = G * max(max(I), 1e-300)
            worst = max(worst, max(abs(a - (b + c)) for a, b, c in zip(dI, dl, dr)) / scale,
                        max(abs(a - (b - c)) for a, b, c in zip(dQ, dl, dr)) / scale)
            dI0, _ = polarized_thomson_iq_rhs(mu, I, [0.0] * nm, G)
            ref = lrs.thomson_rhs(mu, I, G)
            worst = max(worst, max(abs(a - b) for a, b in zip(dI0, ref)) / scale)
        self.assertLessEqual(worst, 1e-14)
        # Stokes contract: the eigenvalues of the 2x2 coherency matrix.
        import numpy as np
        for _ in range(50):
            I, Q, U, V = rng.uniform(0, 2), rng.uniform(-2, 2), rng.uniform(-2, 2), rng.uniform(-2, 2)
            rho = 0.5 * np.array([[I + Q, U - 1j * V], [U + 1j * V, I - Q]])
            ev = np.linalg.eigvalsh(rho)
            lo, hi = coherency_eigenvalues(I, Q, U, V)
            self.assertLess(max(abs(ev[0] - lo), abs(ev[1] - hi)), 1e-14)
        DIAG["h1_max_form_difference"] = worst

    def test_h2_number_conservation_and_isotropic_stationary(self):
        rng = random.Random(802)
        for _ in range(40):
            nm = rng.randint(1, 9)
            mu, I, Q = self._state(rng, nm)
            G = rng.uniform(0.5, 1e5)
            dI, _ = polarized_thomson_iq_rhs(mu, I, Q, G)
            self.assertLessEqual(abs(math.fsum(dI)), 1e-14 * G * math.fsum(map(abs, I + Q)))
            d = [b - a for a, b in zip(mu, mu[1:])]
            dIi, dQi = polarized_thomson_iq_rhs(mu, d, [0.0] * nm, G)
            self.assertLessEqual(max(map(abs, dIi + dQi)), 1e-14 * G * max(d))

    def test_h3_euler_positivity_dt_gamma_le_one(self):
        rng = random.Random(803)
        worst = 0.0
        trials = 0
        for _ in range(300):
            nm = rng.randint(1, 9)
            mu, I, Q = self._state(rng, nm)
            G = rng.uniform(0.5, 50.0)
            dI, dQ = polarized_thomson_iq_rhs(mu, I, Q, G)
            for frac in (0.25, 0.5, 0.9, 1.0):
                dt = frac / G
                In = [a + dt * b for a, b in zip(I, dI)]
                Qn = [a + dt * b for a, b in zip(Q, dQ)]
                low = min(a - abs(b) for a, b in zip(In, Qn)) / max(max(I), 1e-300)
                worst = min(worst, low)
                self.assertGreaterEqual(low, -1e-14)
                trials += 1
        DIAG["h3_trials"] = trials
        DIAG["h3_min_scaled_I_minus_absQ"] = worst

    def test_h4_beyond_the_bound_euler_can_violate(self):
        mu = uniform_mu_edges(6)
        d = [b - a for a, b in zip(mu, mu[1:])]
        worst = 0.0
        for j0 in range(6):
            I = [0.0] * 6
            Q = [0.0] * 6
            I[j0] = Q[j0] = d[j0]  # fully polarized, I = |Q|
            dI, dQ = polarized_thomson_iq_rhs(mu, I, Q, 1.0)
            In = [a + 1.5 * b for a, b in zip(I, dI)]
            Qn = [a + 1.5 * b for a, b in zip(Q, dQ)]
            worst = min(worst, min(a - abs(b) for a, b in zip(In, Qn)) / d[j0])
        self.assertLess(worst, -1e-3)
        DIAG["h4_dt_gamma_1p5_min_scaled_I_minus_absQ"] = worst

    def test_h5_componentwise_clipping_is_wrong(self):
        # (1) Non-realizable but componentwise nonnegative: clipping is a no-op, PSD stays violated.
        I, Q = 0.5, 0.9
        clipped = componentwise_clip(I, Q)
        self.assertEqual(clipped[:2], (I, Q))
        self.assertLess(coherency_eigenvalues(*clipped)[0], 0.0)
        # (2) Negative intensity: clipping creates number (ledger broken) and stays non-realizable.
        I, Q = -0.1, 0.05
        clipped = componentwise_clip(I, Q)
        self.assertGreater(clipped[0], I)
        self.assertLess(coherency_eigenvalues(*clipped)[0], 0.0)
        # (3) Admissible state with signed Q: clipping changes the physical (I_l, I_r) state.
        I, Q = 1.0, -0.75
        self.assertGreaterEqual(coherency_eigenvalues(I, Q)[0], 0.0)
        Ic, Qc = componentwise_clip(I, Q)[:2]
        self.assertEqual(((I + Q) / 2, (I - Q) / 2), (0.125, 0.875))
        self.assertEqual(((Ic + Qc) / 2, (Ic - Qc) / 2), (0.5, 0.5))
        # (4) On an actual over-long Euler step (dt G = 1.5), clipping fails one of the relations.
        mu = uniform_mu_edges(6)
        d = [b - a for a, b in zip(mu, mu[1:])]
        I = [0.0] * 6
        Q = [0.0] * 6
        I[2] = d[2]
        Q[2] = -d[2]
        I[4] = 0.5 * d[4]
        Q[4] = 0.5 * d[4]
        dI, dQ = polarized_thomson_iq_rhs(mu, I, Q, 1.0)
        In = [a + 1.5 * b for a, b in zip(I, dI)]
        Qn = [a + 1.5 * b for a, b in zip(Q, dQ)]
        clipped = [componentwise_clip(a, b)[:2] for a, b in zip(In, Qn)]
        psd_fail = any(coherency_eigenvalues(a, b)[0] < -1e-15 for a, b in clipped)
        number_fail = abs(math.fsum(c[0] for c in clipped) - math.fsum(In)) > 1e-12
        self.assertTrue(psd_fail or number_fail)
        DIAG["h5_euler_clip"] = {"psd_violated_after_clip": psd_fail, "number_changed_by_clip": number_fail,
                                 "number_change": math.fsum(c[0] for c in clipped) - math.fsum(In)}

    def test_h6_coherency_limiter(self):
        rng = random.Random(806)
        for _ in range(200):
            I = rng.uniform(0.0, 2.0)
            P = [rng.uniform(-3, 3) for _ in range(3)]
            out = coherency_limit(I, *P)
            self.assertEqual(out[0], I)
            self.assertGreaterEqual(coherency_eigenvalues(*out)[0], 0.0)
            if math.sqrt(sum(p * p for p in P)) <= I:
                self.assertEqual(out, (I, *P))
        with self.assertRaises(PolarizationRefusal) as cm:
            coherency_limit(-0.1, 0.05)
        self.assertEqual(cm.exception.code, "polarization.negative_intensity")


class I_Refusals(unittest.TestCase):
    """(i) Every typed refusal."""

    Q = [0.0, 0.5, 1.0]
    MU = [-1.0, 0.0, 1.0]
    PHI = uniform_phi_edges(3)

    def _refuse(self, exc, code, fn, *args, **kw):
        with self.assertRaises(exc) as cm:
            fn(*args, **kw)
        self.assertEqual(cm.exception.code, code)
        return code

    def _x(self, grid_n=2 * 2 * 3):
        return [1.0] * grid_n

    def test_i1_grid_refusals(self):
        codes = []
        R = sphere_number_rhs
        x = self._x()
        bad_q = [([], "q.too_few_edges"), ([0.0], "q.too_few_edges"), ([0.0, math.nan], "q.nonfinite_edge"),
                 ([0.0, math.inf], "q.nonfinite_edge"), ([0.0, 0.0], "q.width"), ([1.0, 0.0], "q.width"),
                 ([-1e308, 1e308], "q.width"), ([0.0, 5e-324], "q.subnormal_width"),
                 ([0.0, True], "q.not_real"), (None, "q.not_a_sequence")]
        for q, code in bad_q:
            codes.append(self._refuse(GridRefusal, code, R, q, self.MU, self.PHI, x, 0.1, 0.0, 0.0))
        bad_mu = [([-1.0, 0.5], "mu.not_full_sphere"), ([-1.0000001, 1.0], "mu.not_full_sphere"),
                  ([-1.0, 1.0, 0.5], "mu.width"), ([-0.9, 1.0], "mu.not_full_sphere")]
        for mu, code in bad_mu:
            codes.append(self._refuse(GridRefusal, code, R, self.Q, mu, self.PHI, x, 0.1, 0.0, 0.0))
        bad_phi = [([0.0, math.pi], "phi.not_periodic_cover"), ([0.0, TAU + 1e-12], "phi.not_periodic_cover"),
                   ([-0.1, -0.1 + TAU], "phi.not_periodic_cover"), ([TAU, 2 * TAU], "phi.not_periodic_cover"),
                   ([0.0, 3.0, 2.0, TAU], "phi.width"), ([0.0, 3.0, TAU + 1.0], "phi.not_periodic_cover"),
                   ([0.0, math.nan, TAU], "phi.nonfinite_edge")]
        for phi, code in bad_phi:
            codes.append(self._refuse(GridRefusal, code, R, self.Q, self.MU, phi, [1.0] * 4, 0.1, 0.0, 0.0))
        # Positive controls: offsets with phi[-1] == phi0 + TAU are accepted.
        sphere_number_rhs(self.Q, self.MU, [0.1, 3.0, 0.1 + TAU], [1.0] * 8, 0.1, 0.2, 0.0)
        sphere_number_rhs(self.Q, self.MU, [0.0, TAU], [1.0] * 4, 0.1, 0.2, 0.0)
        codes.append(self._refuse(GridRefusal, "grid.cell_count", uniform_phi_edges, 0))
        DIAG.setdefault("i_codes", []).extend(codes)

    def test_i2_state_geometry_boundary_rate_refusals(self):
        codes = []
        R = sphere_number_rhs
        x = self._x()
        codes.append(self._refuse(StateRefusal, "state.length", R, self.Q, self.MU, self.PHI, x[:-1], 0.1, 0, 0))
        codes.append(self._refuse(StateRefusal, "state.nonfinite", R, self.Q, self.MU, self.PHI,
                                  x[:-1] + [math.nan], 0.1, 0, 0))
        codes.append(self._refuse(StateRefusal, "state.nonfinite", R, self.Q, self.MU, self.PHI,
                                  x[:-1] + [math.inf], 0.1, 0, 0))
        codes.append(self._refuse(GeometryRefusal, "geometry.nonfinite", R, self.Q, self.MU, self.PHI, x,
                                  math.nan, 0, 0))
        codes.append(self._refuse(GeometryRefusal, "geometry.nonfinite", R, self.Q, self.MU, self.PHI, x,
                                  0.1, 0, -math.inf))
        codes.append(self._refuse(GeometryRefusal, "geometry.overflow", R, self.Q, self.MU, self.PHI, x,
                                  1.7e308, 1.7e308, 0))
        codes.append(self._refuse(GeometryRefusal, "geometry.not_real", R, self.Q, self.MU, self.PHI, x,
                                  "0.1", 0, 0))
        codes.append(self._refuse(GeometryRefusal, "geometry.select", R, self.Q, self.MU, self.PHI, x,
                                  0.1, 0, 0, select=(0.1, 0.0)))
        codes.append(self._refuse(GeometryRefusal, "geometry.select", R, self.Q, self.MU, self.PHI, x,
                                  0.1, 0, 0, select=(0.1, math.nan, 0.0)))
        codes.append(self._refuse(BoundaryRefusal, "boundary.invalid", R, self.Q, self.MU, self.PHI, x,
                                  0.1, 0, 0, "periodic"))
        T = thomson6_rhs
        for rate in (-1.0, math.nan, math.inf, None):
            codes.append(self._refuse(RateRefusal, "rate.invalid", T, self.MU, self.PHI, [1.0] * 6, rate))
        codes.append(self._refuse(StateRefusal, "state.length", T, self.MU, self.PHI, [1.0] * 5, 1.0))
        grid = SphereGrid(self.Q, self.MU, self.PHI)
        lay = StateLayout(2, 2, 3)
        y = [0.0] * lay.n
        codes.append(self._refuse(StateRefusal, "state.length", grid_state_rhs, grid, y[:-1], 0.1, 0, 0))
        codes.append(self._refuse(RateRefusal, "absorber.invalid", grid_state_rhs, grid, y, 0.1, 0, 0, 0.0,
                                  [0.1]))
        codes.append(self._refuse(RateRefusal, "absorber.invalid", grid_state_rhs, grid, y, 0.1, 0, 0, 0.0,
                                  [0.1, -0.2]))
        codes.append(self._refuse(RateRefusal, "absorber.invalid", grid_state_rhs, grid, y, 0.1, 0, 0, 0.0,
                                  [0.1, math.nan]))
        DIAG.setdefault("i_codes", []).extend(codes)

    def test_i3_characteristic_and_polarization_refusals(self):
        codes = []
        C = characteristic_map
        for A in ((1.0, 0.0, 1.0), (1.0, -1.0, 1.0), (1.0, math.inf, 1.0), (1.0, 1.0), None):
            codes.append(self._refuse(CharacteristicRefusal, "characteristic.amplification", C, 0.0, 0.1, 0.2, A))
        codes.append(self._refuse(CharacteristicRefusal, "characteristic.direction", C, 0.0, 1.5, 0.2, (1, 1, 1)))
        codes.append(self._refuse(CharacteristicRefusal, "characteristic.direction", C, 0.0, 0.1, math.nan,
                                  (1, 1, 1)))
        codes.append(self._refuse(CharacteristicRefusal, "characteristic.direction", C, math.inf, 0.1, 0.2,
                                  (1, 1, 1)))
        codes.append(self._refuse(CharacteristicRefusal, "characteristic.quadrature", gauss_legendre, 0))
        P = polarized_thomson_iq_rhs
        codes.append(self._refuse(RateRefusal, "rate.invalid", P, self.MU, [1.0, 1.0], [0.0, 0.0], -2.0))
        codes.append(self._refuse(PolarizationRefusal, "polarization.length", P, self.MU, [1.0], [0.0, 0.0], 1.0))
        codes.append(self._refuse(PolarizationRefusal, "polarization.nonfinite", P, self.MU, [1.0, math.nan],
                                  [0.0, 0.0], 1.0))
        codes.append(self._refuse(GridRefusal, "mu.not_full_sphere", P, [-1.0, 0.9], [1.0], [0.0], 1.0))
        codes.append(self._refuse(PolarizationRefusal, "polarization.negative_intensity", coherency_limit,
                                  -1e-300, 0.0))
        codes.append(self._refuse(PolarizationRefusal, "polarization.nonfinite", coherency_limit, 1.0, math.nan))
        DIAG.setdefault("i_codes", []).extend(codes)


CHECK_CLASSES = (A_ExactAlgebra, B_Conservation, C_LRSReduction, D_Thomson, E_Rank, F_ThomsonLRS,
                 G_Characteristics, H_Polarization, I_Refusals)


class _Recorder(unittest.TextTestResult):
    def __init__(self, *a, **k):
        super().__init__(*a, **k)
        self.records = []
        self._t0 = {}

    def startTest(self, test):
        self._t0[test.id()] = time.perf_counter()
        super().startTest(test)

    def _rec(self, test, status, detail=""):
        dt = time.perf_counter() - self._t0.get(test.id(), time.perf_counter())
        self.records.append({"id": test.id().split(".", 1)[-1], "status": status, "wall_s": round(dt, 3),
                             "detail": detail})

    def addSuccess(self, test):
        super().addSuccess(test)
        self._rec(test, "pass")

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._rec(test, "fail", self._exc_info_to_string(err, test))

    def addError(self, test, err):
        super().addError(test, err)
        self._rec(test, "error", self._exc_info_to_string(err, test))

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._rec(test, "skip", reason)


def _versions():
    out = {"python": platform.python_version(), "libc": "-".join(platform.libc_ver()),
           "machine": platform.machine()}
    for name in ("numpy", "scipy", "mpmath"):
        try:
            out[name] = __import__(name).__version__
        except Exception as exc:  # pragma: no cover - environment report only
            out[name] = f"unavailable: {exc}"
    return out


def main(argv):
    suite = unittest.TestSuite()
    loader = unittest.defaultTestLoader
    for cls in CHECK_CLASSES:
        suite.addTests(loader.loadTestsFromTestCase(cls))
    t0 = time.perf_counter()
    result = unittest.TextTestRunner(verbosity=2, resultclass=_Recorder).run(suite)
    wall = time.perf_counter() - t0
    if len(argv) > 1:
        payload = {
            "status": "pass_scoped" if result.wasSuccessful() else "fail",
            "tests": result.testsRun,
            "failures": len(result.failures),
            "errors": len(result.errors),
            "skipped": len(result.skipped),
            "scope": ("formula oracle: triaxial Bianchi I full-sphere (q, mu, phi) first-order FV transport, "
                      "rank-6 unpolarized elastic Thomson, axisymmetric (I, Q) polarized Thomson contract; "
                      "no live BASS/rei port, no Rust adapter, no production performance"),
            "checks": result.records,
            "diagnostics": DIAG,
            "inputs": {"oracle_sha256": sha256_file(__file__),
                       "lrs_oracle_path": str(LRS_ORACLE_PATH.relative_to(REPO)),
                       "lrs_oracle_sha256": sha256_file(LRS_ORACLE_PATH)},
            "environment": _versions(),
            "wall_s": round(wall, 2),
            "timing_authority": "diagnostic_only",
        }
        Path(argv[1]).write_text(json.dumps(payload, indent=2, default=float) + "\n")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
