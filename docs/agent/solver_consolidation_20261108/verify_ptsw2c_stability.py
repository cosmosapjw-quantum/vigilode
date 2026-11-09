#!/usr/bin/env python3
"""Independent numerical root-locus audit of the published PTSW2C method.

This is a reproducible floating-point cross-check, not an interval proof of
A(alpha)-stability. Requires NumPy, SciPy and SymPy.
"""
from __future__ import annotations

import math
import numpy as np
import sympy as sp
from scipy.optimize import minimize_scalar


def amplification_matrix(z: complex) -> np.ndarray:
    """Constant-step peer amplification for y'=lambda*y, z=h*lambda, T=J."""
    gamma = 12.0 / 25.0
    A = np.array([[-1.0 / 28.0, -27.0 / 28.0],
                  [-65.0 / 252.0, 205.0 / 252.0]])
    Gamma = np.array([[-6.0 / 35.0, -54.0 / 175.0],
                      [54.0 / 175.0, -138.0 / 175.0]])
    b = np.array([1.0 / 28.0, 27.0 / 28.0])
    e = np.ones(2)
    r = z / (1.0 - gamma * z)
    H = r * (A + Gamma)
    M = np.empty((3, 3), dtype=complex)
    M[0, 0] = 1.0 + r * np.dot(b, e)
    M[0, 1:] = b @ H
    M[1:, 0] = r * e
    M[1:, 1:] = H
    return M


def spectral_radius(z: complex) -> float:
    return float(np.max(np.abs(np.linalg.eigvals(amplification_matrix(z)))))


def boundary_z_roots(phi: float) -> np.ndarray:
    """Solve characteristic numerator P(exp(i phi), z)=0 for z."""
    w = np.exp(1j * phi)
    a2 = 2592*w**3 + 1824*w**2 - 624*w - 667
    a1 = -10800*w**3 + 1600*w**2 - 2050*w
    a0 = 11250*w**3 - 11250*w**2
    return np.roots([a2, a1, a0])


def upper_left_candidates(phi: float) -> list[complex]:
    return [complex(z) for z in boundary_z_roots(phi)
            if z.real < 0.0 and z.imag >= 0.0]


def minimum_boundary_angle(phi: float) -> float:
    candidates = upper_left_candidates(phi)
    if not candidates:
        return float("inf")
    return min(abs(float(np.angle(-z))) for z in candidates)


def minimizing_boundary_z(phi: float) -> complex:
    candidates = upper_left_candidates(phi)
    if not candidates:
        return complex(float("nan"), float("nan"))
    return min(candidates, key=lambda z: abs(float(np.angle(-z))))


def main() -> None:
    z, w = sp.symbols("z w")
    gamma = sp.Rational(12, 25)
    A = sp.Matrix([[-sp.Rational(1, 28), -sp.Rational(27, 28)],
                   [-sp.Rational(65, 252), sp.Rational(205, 252)]])
    G = sp.Matrix([[-sp.Rational(6, 35), -sp.Rational(54, 175)],
                   [sp.Rational(54, 175), -sp.Rational(138, 175)]])
    b = sp.Matrix([sp.Rational(1, 28), sp.Rational(27, 28)])
    r = z / (1 - gamma*z)
    H = r*(A+G)
    top = sp.Matrix.hstack(sp.Matrix([[1+(b.T*r*sp.ones(2, 1))[0]]]), b.T*H)
    bottom = sp.Matrix.hstack(r*sp.ones(2, 1), H)
    M = top.col_join(bottom)
    char_num = sp.factor(sp.fraction(sp.cancel(M.charpoly(w).as_expr()))[0])
    expected = (2592*w**3*z**2 - 10800*w**3*z + 11250*w**3
                + 1824*w**2*z**2 + 1600*w**2*z - 11250*w**2
                - 624*w*z**2 - 2050*w*z - 667*z**2)
    assert sp.expand(char_num - expected) == 0

    M_inf = M.applyfunc(lambda entry: sp.limit(entry, z, sp.oo))
    eig_inf = [complex(sp.N(root, 16)) for root in M_inf.eigenvals()]
    rho_inf = max(abs(root) for root in eig_inf)

    def angle_on_root_locus(phi: float) -> float:
        return minimum_boundary_angle(phi)

    phis = np.linspace(1.0e-5, math.pi-1.0e-5, 30001)
    samples = [(angle_on_root_locus(float(phi)), float(phi)) for phi in phis]
    alpha_scan, phi_scan = min(samples)
    opt = minimize_scalar(angle_on_root_locus,
                          bounds=(phi_scan-0.01, phi_scan+0.01),
                          method="bounded", options={"xatol": 1.0e-14})
    phi_star = float(opt.x)
    z_star = minimizing_boundary_z(phi_star)
    alpha_star = angle_on_root_locus(phi_star)
    w_star = np.exp(1j*phi_star)
    root_residual = abs(complex(sp.N(expected.subs({w:w_star, z:z_star}))))
    rho_star = spectral_radius(z_star)

    radii = np.logspace(-6, 8, 600)
    angles = np.linspace(0.0, math.radians(81.1), 163)
    max_sector_rho = 0.0
    for alpha in angles:
        ray_arg = math.pi - alpha
        for radius in radii:
            max_sector_rho = max(max_sector_rho,
                                 spectral_radius(radius*np.exp(1j*ray_arg)))

    z_out = 2.8373036165162113*np.exp(1j*(math.pi-math.radians(82.0)))
    rho_out = spectral_radius(z_out)

    assert abs(rho_inf-0.6796) < 1.0e-3
    assert abs(math.degrees(alpha_star)-81.16975) < 2.0e-4
    assert abs(rho_star-1.0) < 1.0e-10
    assert root_residual < 1.0e-8
    assert max_sector_rho < 1.0
    assert rho_out > 1.0

    print("PASS: exact amplification characteristic polynomial constructed.")
    print(f"PASS: stiff-limit eigenvalues={eig_inf}; rho_inf={rho_inf:.10f}.")
    print(f"PASS: unit-circle root-locus minimum sector angle={math.degrees(alpha_star):.8f} deg.")
    print(f"PASS: boundary z={z_star.real:.10f}{z_star.imag:+.10f}i, w=exp(i*{phi_star:.10f}).")
    print(f"PASS: boundary characteristic residual={root_residual:.3e}, rho(M(z*))={rho_star:.12f}.")
    print(f"PASS: coarse polar scan max rho inside 81.1-deg sector={max_sector_rho:.12f}.")
    print(f"PASS: 82-deg outside-sector witness z={z_out.real:.8f}{z_out.imag:+.8f}i has rho={rho_out:.10f}>1.")
    print("LIMITATION: this is floating-point root-locus and polar sampling, not an interval-certified sector proof.")


if __name__ == "__main__":
    main()
