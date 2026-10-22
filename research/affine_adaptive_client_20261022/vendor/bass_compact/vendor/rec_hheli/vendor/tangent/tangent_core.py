"""Analytic tangent and adjoint actions for the frozen PR147 FV owner.

The contract is collisionless CanonicalFVSpecies, L=8, f_m=0.1,
m/T_ref=30 and a 24 by 12 cell Gauss3 momentum rule.  This module keeps
the owner's 40 coordinates and treats the primordial massive quadrupole
amplitude A as an independent parameter.  It does not differentiate the
initial-condition calibration or change the underlying finite closure.

Each prepare evaluates one mass-shell energy grid.  Any number of JVPs,
VJPs, or observable JVPs then reuses its moment derivatives.  No dense
40 by 40 state Jacobian is formed.  The copied quadrature and radiation
matrices remain independent of subsequent changes to the caller's model.

Signed observables use the Frobenius convention and kappa_t=8*pi*G/c^2:
S_F=sqrt(6)*s/H, P_A,F=sqrt(6)*kappa_t*pi_A/H^2.  Since the source's
rho_crit=3*c^2*H0^2/(8*pi*G), kappa_t*rho_crit=3*H0^2.  The fourth
observable is the signed sum P_m,F+P_r,F; norms are a separate operation.
"""

from __future__ import annotations

from dataclasses import dataclass
import math

import numpy as np


_N = 9
_SIZE = 40
_SQRT6 = math.sqrt(6.0)
_KAPPA_TIME = 8.0 * math.pi * 6.67430e-11 / 299792458.0**2


def _real(value, name):
    """Reject complex and nonnumeric input before converting to binary64."""
    raw = np.asarray(value)
    if np.iscomplexobj(raw) or raw.dtype.kind not in "biuf":
        raise ValueError(f"{name} must be real numeric data")
    try:
        result = np.array(raw, dtype=np.float64, copy=True)
    except (ValueError, TypeError, OverflowError) as exc:
        raise ValueError(f"{name} cannot be represented as binary64") from exc
    if not np.all(np.isfinite(result)):
        raise ValueError(f"{name} must be finite")
    return result


def _scalar(value, name):
    result = _real(value, name)
    if result.ndim != 0:
        raise ValueError(f"{name} must be a scalar")
    return float(result)


def _readonly(value):
    result = np.array(value, dtype=np.float64, copy=True)
    result.flags.writeable = False
    return result


def _finite_result(value, name):
    if not np.all(np.isfinite(value)):
        raise FloatingPointError(f"nonfinite {name}; reduce the input scale")
    return value


def _directions(direction, d_amplitude):
    dy = _real(direction, "direction")
    vector = dy.ndim == 1
    if vector:
        if dy.shape != (_SIZE,):
            raise ValueError("direction must have shape (40,) or (40,K)")
        dy = dy[:, None]
    elif dy.ndim != 2 or dy.shape[0] != _SIZE or dy.shape[1] < 1:
        raise ValueError("direction must have shape (40,) or (40,K), K>=1")
    da = _real(d_amplitude, "d_amplitude")
    if da.ndim == 0:
        da = np.full(dy.shape[1], float(da))
    elif da.ndim != 1 or da.shape != (dy.shape[1],):
        raise ValueError("d_amplitude must be scalar or have one entry per column")
    return dy, da, vector


@dataclass(frozen=True)
class Prepared:
    """A fixed-state linearization; all public arrays are read-only copies."""

    rhs: np.ndarray
    observables_signed: np.ndarray
    _matrices: tuple
    _flows: np.ndarray
    _T: float
    _B: float
    _C: float
    _D: float
    _h_b: float
    _h_v: float
    _h_g0: float
    _h_n0: float
    _h_A: float
    _d_b: float
    _d_A: float
    _r_g2: float
    _r_n2: float
    _matter_pi: float
    _radiation_pi: float
    _observable_factor: float
    _moments: tuple
    _packets: int

    @property
    def structural_info(self):
        return {
            "state_size": _SIZE,
            "packet_count": self._packets,
            "mass_shell_grid_evaluations_per_prepare": 1,
            "mass_shell_grid_evaluations_per_action": 0,
            "dense_state_jacobian_formed": False,
            "batched_direction_axis": 1,
        }

    @property
    def moment_data(self):
        """Finite quadrature values and derivatives at fixed x and A."""
        names = (
            "rho", "pi_s", "pressure_perp", "pressure_parallel",
            "rho_b", "rho_A", "pi_b", "pi_A",
            "reference_energy_norm", "reference_energy_norm_A",
        )
        return dict(zip(names, self._moments))

    def _variations(self, dy, da):
        # h is delta(H)/H, not delta(H).  Only these four state entries
        # enter the constraint at fixed cosmological time x.
        h = (self._h_b * dy[1] + self._h_v * dy[2]
             + self._h_g0 * dy[4] + self._h_n0 * dy[22]
             + self._h_A * da)
        dm = self._d_b * dy[1] + self._d_A * da
        dr = self._r_g2 * dy[6] + self._r_n2 * dy[24]
        dbprime = self._T * dy[2] - self._B * h
        return h, dm, dr, dbprime

    def jvp(self, direction, d_amplitude=0.0):
        """Return J_y direction + F_A d_amplitude, with optional K columns."""
        dy, da, vector = _directions(direction, d_amplitude)
        h, dm, dr, dbprime = self._variations(dy, da)
        out = np.zeros_like(dy)
        out[0] = -self._T * h
        out[1] = dbprime
        out[2] = -3.0 * dy[2] + self._C * (dm + dr - self._D * h)
        # tau and u have no feedback for the fm>0 collisionless owner.
        for block, matrix in enumerate(self._matrices):
            start = 4 + block * _N
            out[start:start + _N] = (
                self._flows[block, :, None] * dbprime[None, :]
                + self._B * (matrix @ dy[start:start + _N]))
        _finite_result(out, "tangent action")
        return out[:, 0] if vector else out

    def vjp(self, cotangent):
        """Return (J_y.T @ cotangent, F_A.T @ cotangent) without a dense J."""
        lam = _real(cotangent, "cotangent")
        if lam.shape != (_SIZE,):
            raise ValueError("cotangent must have shape (40,)")
        radiation_lam = lam[4:].reshape(4, _N)
        bar_B = float(lam[1] + np.sum(radiation_lam * self._flows))
        bar_h = -self._T * lam[0] - self._B * bar_B - self._C * self._D * lam[2]
        bar_D = self._C * lam[2]
        grad = np.zeros(_SIZE)
        grad[1] = bar_h * self._h_b + bar_D * self._d_b
        grad[2] = self._T * bar_B - 3.0 * lam[2] + bar_h * self._h_v
        grad[4] = bar_h * self._h_g0
        grad[22] = bar_h * self._h_n0
        grad[6] = bar_D * self._r_g2
        grad[24] = bar_D * self._r_n2
        for block, matrix in enumerate(self._matrices):
            start = 4 + block * _N
            grad[start:start + _N] += self._B * (matrix.T @ radiation_lam[block])
        grad_A = bar_h * self._h_A + bar_D * self._d_A
        _finite_result(grad, "adjoint state action")
        _finite_result(grad_A, "adjoint amplitude action")
        return grad, float(grad_A)

    def observable_jvp(self, direction, d_amplitude=0.0):
        """Differentiate the four signed Frobenius observables at fixed x."""
        dy, da, vector = _directions(direction, d_amplitude)
        h, dm, dr, dbprime = self._variations(dy, da)
        out = np.empty((4, dy.shape[1]))
        out[0] = _SQRT6 * dbprime
        out[1] = self._observable_factor * (dm - 2.0 * self._matter_pi * h)
        out[2] = self._observable_factor * (dr - 2.0 * self._radiation_pi * h)
        out[3] = out[1] + out[2]
        _finite_result(out, "observable tangent action")
        return out[:, 0] if vector else out


class TangentOperator:
    """Snapshot the fixed CanonicalFVSpecies model and prepare tangent actions.

    Changes to the original model after construction are deliberately not
    reflected here.  Construct a new operator for a new amplitude, source,
    or owner model.  Positivity-boundary amplitudes permit formal derivatives
    of the smooth finite sums; physical parameter variations must separately
    stay inside the owner's amplitude interval [-1,2].
    """

    def __init__(self, model):
        if (getattr(model, "n", None) != _N
                or getattr(model, "ellmax", None) != 8
                or getattr(model, "ne_source", None) is not None
                or _scalar(model.fm, "mass_fraction") != 0.1
                or _scalar(model.nu_visc, "matter viscosity") != 0.0):
            raise ValueError("requires the fixed collisionless L8, fm=0.1 owner")
        momentum = model.momentum
        if (_scalar(momentum.mass, "mass/Tref") != 30.0
                or momentum.radial_cells != 24
                or momentum.angular_cells != 12
                or momentum.gauss_order != 3):
            raise ValueError("requires m/Tref=30 and 24x12 Gauss3 momentum cells")
        self.amplitude = _scalar(momentum.amplitude, "amplitude")
        if not -1.0 <= self.amplitude <= 2.0:
            raise ValueError("amplitude lies outside the positive owner family")
        self._mass = 30.0
        self._q = _readonly(_real(momentum._q, "owner q"))
        self._mu2 = _readonly(_real(momentum._mu2, "owner mu squared"))
        self._numbers = _readonly(_real(momentum._numbers, "owner packet numbers"))
        if (self._q.shape != (72, 1) or self._mu2.shape != (1, 36)
                or self._numbers.shape != (72, 36)
                or np.any(self._q <= 0.0)
                or np.any((self._mu2 < 0.0) | (self._mu2 > 1.0))
                or np.any(self._numbers < 0.0)):
            raise ValueError("owner quadrature arrays do not match the finite contract")

        # Rebuild the actual owner's A=0 rule, rather than dividing by
        # 1+A*P2 (which is unsafe at positivity-boundary amplitudes).
        zero = type(momentum)(30.0, 0.0, 24, 12, gauss_order=3)
        if not (np.array_equal(self._q, zero._q)
                and np.array_equal(self._mu2, zero._mu2)):
            raise ValueError("the owner's amplitude family changes its momentum grid")
        base_numbers = _real(zero._numbers, "A=0 packet numbers")
        p2 = (3.0 * self._mu2 - 1.0) / 2.0
        expected = base_numbers * (1.0 + self.amplitude * p2)
        if not np.allclose(self._numbers, expected, rtol=8.0*np.finfo(float).eps, atol=0.0):
            raise ValueError("packet numbers do not belong to the declared linear A family")
        self._numbers_A = _readonly(base_numbers * p2)
        self._reference = _scalar(momentum.reference_energy_norm, "reference energy norm")
        if self._reference <= 0.0:
            raise ValueError("reference energy norm must be positive")
        reference_E = np.hypot(self._mass, self._q)
        rebuilt_reference = float(np.sum(self._numbers * reference_E))
        if not math.isclose(self._reference, rebuilt_reference, rel_tol=8.0*np.finfo(float).eps):
            raise ValueError("reference energy norm is inconsistent with packet numbers")
        # Retain the finite-rule normalization derivative even if tiny.
        # An exact continuum P2 integral of zero is not substituted here.
        self._reference_A = float(np.sum(self._numbers_A * reference_E))

        energy = _real(model.energy_matrix, "energy matrix")
        number = _real(model.number_matrix, "number matrix")
        if energy.shape != (_N, _N) or number.shape != (_N, _N):
            raise ValueError("radiation matrix shape must be (9,9)")
        energy, number = _readonly(energy), _readonly(number)
        self._matrices = (energy, number, energy, number)
        source = model.source
        self._h0 = _scalar(source.h0, "H0")
        self._href = _scalar(source.href, "Href")
        self._rho_crit = _scalar(source.rhocrit_j_m3, "rho_crit [J/m^3]")
        self._aref = _scalar(source.aref, "a_ref")
        self._xmax = _scalar(source.xmax, "x_max")
        self._lambda = _scalar(source.omega_lambda, "Omega_lambda")
        self._mass_pref = _scalar(model._mass_pref, "mass prefactor")
        self._dust_omega = _scalar(model._dust_omega, "dust Omega")
        self._omega_g = _scalar(model.omega_gamma, "Omega_gamma")
        self._omega_n = _scalar(model.omega_nu, "Omega_nu")
        if min(self._h0, self._href, self._rho_crit, self._aref, self._xmax,
               self._mass_pref, self._dust_omega, self._omega_g, self._omega_n) <= 0.0:
            raise ValueError("source must have positive finite background coefficients")

    def prepare(self, x, y):
        """Evaluate the RHS, observables and their linear actions at one state."""
        x = _scalar(x, "x")
        y = _real(y, "state")
        if y.shape != (_SIZE,):
            raise ValueError("state must have shape (40,)")
        if (not 0.0 <= x <= self._xmax + 1e-12 or abs(y[1]) > 1.0
                or y[4] <= 0.0 or y[22] <= 0.0 or y[3] < -1e-12):
            raise ValueError("state lies outside the finite collisionless owner chart")
        b, v = float(y[1]), float(y[2])
        r = math.exp(-x)
        q_scaled_squared = (r * self._q) ** 2
        U = q_scaled_squared * math.exp(2.0*b) * (1.0-self._mu2)
        W = q_scaled_squared * math.exp(-4.0*b) * self._mu2
        E = np.sqrt(self._mass**2 + U + W)
        weight = (r**3 / self._reference) * self._numbers
        weight_A = (r**3 / self._reference) * (
            self._numbers_A - self._numbers * (self._reference_A / self._reference))
        pi_kernel = (2.0*W-U) / (6.0*E)
        rho = float(np.sum(weight * E))
        pi = float(np.sum(weight * pi_kernel))
        pp = float(np.sum(weight * U / (2.0*E)))
        pa = float(np.sum(weight * W / E))
        rho_b = -6.0*pi
        rho_A = float(np.sum(weight_A * E))
        # This expanded numerator avoids subtracting two positive terms
        # and makes the nonpositive pi_b sign explicit for positive weights.
        pi_b_kernel = -((2.0*U+8.0*W)*self._mass**2
                        + U*U + 14.0*U*W + 4.0*W*W) / (6.0*E**3)
        pi_b = float(np.sum(weight * pi_b_kernel))
        pi_A = float(np.sum(weight_A * pi_kernel))

        a = self._aref * math.exp(x)
        inv_a4 = a**-4
        g0, n0 = self._omega_g*inv_a4, self._omega_n*inv_a4
        rad = g0*y[4] + n0*y[22]
        dust = self._dust_omega / a**3
        h0_squared = self._h0**2
        href_squared = self._href**2
        h2 = h0_squared*(dust+self._mass_pref*rho+rad+self._lambda) + href_squared*v*v
        if not math.isfinite(h2) or h2 <= 0.0:
            raise FloatingPointError("nonphysical FV Einstein constraint")
        H = math.sqrt(h2)
        T, B = self._href/H, self._href*v/H
        C = 3.0*h0_squared/(self._href*H)
        r_g2, r_n2 = g0/3.0, n0/3.0
        matter_pi = self._mass_pref*pi
        radiation_pi = r_g2*y[6] + r_n2*y[24]
        D = matter_pi + radiation_pi
        blocks = y[4:].reshape(4, _N)
        flows = np.array([matrix @ block for matrix, block in zip(self._matrices, blocks)])
        rhs = np.empty(_SIZE)
        rhs[:4] = (T, B, -3.0*v+C*D, 0.0)
        rhs[4:] = (B*flows).reshape(-1)
        hfactor = h0_squared/(2.0*h2)
        obs_factor = _SQRT6*_KAPPA_TIME*self._rho_crit/h2
        obs = np.array([_SQRT6*B, obs_factor*matter_pi, obs_factor*radiation_pi, 0.0])
        obs[3] = obs[1]+obs[2]
        _finite_result(rhs, "owner RHS")
        _finite_result(obs, "signed observables")
        return Prepared(
            rhs=_readonly(rhs), observables_signed=_readonly(obs),
            _matrices=self._matrices, _flows=_readonly(flows),
            _T=T, _B=B, _C=C, _D=D,
            _h_b=hfactor*self._mass_pref*rho_b,
            _h_v=href_squared*v/h2,
            _h_g0=hfactor*g0, _h_n0=hfactor*n0,
            _h_A=hfactor*self._mass_pref*rho_A,
            _d_b=self._mass_pref*pi_b, _d_A=self._mass_pref*pi_A,
            _r_g2=r_g2, _r_n2=r_n2,
            _matter_pi=matter_pi, _radiation_pi=radiation_pi,
            _observable_factor=obs_factor,
            _moments=(rho, pi, pp, pa, rho_b, rho_A, pi_b, pi_A,
                      self._reference, self._reference_A),
            _packets=self._numbers.size,
        )

    def rhs(self, x, y):
        return self.prepare(x, y).rhs

    def jvp(self, x, y, direction, d_amplitude=0.0):
        return self.prepare(x, y).jvp(direction, d_amplitude)

    def vjp(self, x, y, cotangent):
        return self.prepare(x, y).vjp(cotangent)
