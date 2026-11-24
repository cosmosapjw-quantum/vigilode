#!/usr/bin/env python3
"""Independent re-derivation of closed-form claims made by the 2026-11-24 unified referee run.

Every check recomputes the quantity from the repository inputs or from first principles and
does not import any script of `research/unified_referee_20261124`. Exit status 0 means that
every check passed; the JSON written to the path given as the first argument holds the
values.

Checks
  C1  six distinct stage clocks among the eight RODAS5P abscissae (tableau snapshot)
  C2  non-normal W example: ||W^-1 r||_inf / ||r||_inf = 250 (exact rationals)
  C3  backward-Euler fold for u' = u^2: fold at (u, tau) = (2, 1/2), no real root at tau = 1
  C4  S^2 finite-volume colatitude operator: the alternating vector is an exact eigenvector
      with eigenvalue 2 kappa cot(dtheta/2) / (R^2 dtheta), which equals the Gershgorin bound,
      so the area-weighted spectrum of P^-1 A (P = I - sigma L_phi) has extreme ratio
      1 + 2 sigma kappa cot(dtheta/2) / (R^2 dtheta)
  C5  manufactured scalar problem x(t) = 1/4 + sin t with N(t, y) = -80 y - y^3: the forcing
      g(t) = cos t + 80 x + x^3 makes x an exact solution (sympy)
"""
from __future__ import annotations

import json
import math
import pathlib
import sys
from fractions import Fraction

import numpy as np
import sympy as sp

ROOT = pathlib.Path(__file__).resolve().parents[3]
out: dict[str, object] = {"schema": "vigilode.referee2.prior-claims.v1", "checks": {}}
failed: list[str] = []


def record(name: str, ok: bool, **values: object) -> None:
    out["checks"][name] = {"pass": bool(ok), **values}
    if not ok:
        failed.append(name)


# ----------------------------------------------------------------------------------- C1
snap = json.loads((ROOT / "fixtures/rodas5p_coefficients_snapshot.json").read_text())
c_values = [float(x) for x in snap["c"]]
distinct = sorted(set(c_values))
record(
    "C1_six_distinct_clocks",
    len(c_values) == 8 and len(distinct) == 6 and c_values[5:] == [1.0, 1.0, 1.0],
    abscissae=c_values,
    distinct=len(distinct),
)

# ----------------------------------------------------------------------------------- C2
J = [[Fraction(-1), Fraction(1000)], [Fraction(0), Fraction(-1)]]
W = [[1 - J[0][0], -J[0][1]], [-J[1][0], 1 - J[1][1]]]
r = [Fraction(0), Fraction(1, 1000)]
x2 = r[1] / W[1][1]
x1 = (r[0] - W[0][1] * x2) / W[0][0]
ratio = max(abs(x1), abs(x2)) / max(abs(r[0]), abs(r[1]))
record("C2_nonnormal_amplification", (x1, x2, ratio) == (Fraction(1, 4), Fraction(1, 2000), 250),
       x=[str(x1), str(x2)], ratio=str(ratio))

# ----------------------------------------------------------------------------------- C3
u, tau, t = sp.symbols("u tau t", real=True)
H = u - 1 - tau / 2 * u**2
fold = sp.solve([H, sp.diff(H, u)], [u, tau], dict=True)
disc = sp.discriminant(H, u)
real_roots_at_1 = sp.solveset(H.subs(tau, 1), u, domain=sp.S.Reals)
transversal = (sp.diff(H, u, 2) * sp.diff(H, tau)).subs({u: 2, tau: sp.Rational(1, 2)})
ode_value = (1 / (1 - t)).subs(t, sp.Rational(1, 2))
record(
    "C3_backward_euler_fold",
    fold == [{u: 2, tau: sp.Rational(1, 2)}]
    and sp.simplify(disc - (1 - 2 * tau)) == 0
    and real_roots_at_1 == sp.S.EmptySet
    and transversal != 0
    and ode_value == 2,
    fold=str(fold), discriminant=str(disc), transversality=str(transversal),
)


# ----------------------------------------------------------------------------------- C4
def colatitude_operator(n_theta: int, kappa: float, radius: float) -> tuple[np.ndarray, np.ndarray]:
    """Finite-volume colatitude diffusion with exact cell areas and zero polar flux.

    Cell j covers [j dtheta, (j + 1) dtheta]; its area per unit longitude is
    R^2 (cos theta_{j-1/2} - cos theta_{j+1/2}); the face length is R sin theta_{j+1/2}.
    """
    dth = math.pi / n_theta
    faces = np.array([j * dth for j in range(n_theta + 1)])
    area = radius**2 * (np.cos(faces[:-1]) - np.cos(faces[1:]))
    L = np.zeros((n_theta, n_theta))
    for j in range(n_theta - 1):
        cond = kappa * radius * math.sin(faces[j + 1]) / (radius * dth)
        L[j, j] -= cond / area[j]
        L[j, j + 1] += cond / area[j]
        L[j + 1, j + 1] -= cond / area[j + 1]
        L[j + 1, j] += cond / area[j + 1]
    return L, area


c4_rows = []
c4_ok = True
for n_theta, kappa, radius, sigma in [(8, 1.0, 1.0, 0.3), (32, 0.7, 2.5, 4.0), (96, 3.0, 0.4, 1e-2)]:
    L, area = colatitude_operator(n_theta, kappa, radius)
    dth = math.pi / n_theta
    predicted = 2 * kappa / (math.tan(dth / 2) * radius**2 * dth)
    alt = np.array([(-1.0) ** j for j in range(n_theta)])
    eig_residual = float(np.max(np.abs(-L @ alt - predicted * alt)) / predicted)
    gershgorin = float(np.max(np.sum(np.abs(L), axis=1)))
    # symmetrise with the area weights to obtain the real spectrum
    S = np.diag(np.sqrt(area)) @ (-L) @ np.diag(1 / np.sqrt(area))
    spectrum = np.linalg.eigvalsh((S + S.T) / 2)
    top = float(spectrum[-1])
    bottom = float(spectrum[0])
    # The m = 0 block of P^-1 A is I - sigma L_theta; every other longitudinal mode has a
    # larger positive diagonal in P, so the extreme ratio is attained by the zonal block.
    ratio = (1 + sigma * top) / (1 + sigma * max(bottom, 0.0))
    row = {
        "n_theta": n_theta, "kappa": kappa, "radius": radius, "sigma": sigma,
        "predicted_top": predicted, "computed_top": top, "gershgorin": gershgorin,
        "alternating_eigen_residual": eig_residual, "null_eigenvalue": bottom,
        "ratio": ratio, "predicted_ratio": 1 + sigma * predicted,
    }
    c4_rows.append(row)
    c4_ok &= (
        eig_residual < 1e-12
        and abs(top - predicted) < 1e-10 * predicted
        and abs(gershgorin - predicted) < 1e-10 * predicted
        and abs(bottom) < 1e-9 * predicted
        and abs(ratio - (1 + sigma * predicted)) < 1e-9 * ratio
    )
record("C4_sphere_longitude_line_ratio", c4_ok, rows=c4_rows)

# ----------------------------------------------------------------------------------- C5
x = sp.Rational(1, 4) + sp.sin(t)
N = -80 * x - x**3
g = sp.cos(t) + 80 * x + x**3
record("C5_manufactured_solution", sp.simplify(sp.diff(x, t) - (N + g)) == 0)

out["failed"] = failed
target = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else None
text = json.dumps(out, indent=1, sort_keys=True)
if target:
    target.write_text(text + "\n")
print(text)
sys.exit(1 if failed else 0)
