#!/usr/bin/env python3
"""KS-N8: field-of-values certificate for advection-diffusion with ring-constant velocity on the
latitude-longitude grid of crates/vigilode/src/spatial.rs (THEORY.md section 10; derivation in
research/kernel_spectral_foundation_20261124/ks_n8/DERIVATION.txt).

The advection operator is flux form, centred (primary) or donor-cell upwind, with a zonal speed per
ring and a meridional speed per latitude face. On 3 small grids, with dense matrices and exact
eigen-decompositions, the script checks:

  A0  the longitude DFT block-diagonalises J into the tridiagonal mode blocks B_m of the derivation
  A1  closed form of the A-Hermitian part Ht_m and of the skew part Kt_m (L4)
  A2  mu_A(J) of the dense grid operator equals max_m lambda_max(Ht_m); the same for mu_2
  A3  Hermitian-part inertia: non-positive Thomas pivots of z I - Ht_m count the eigenvalues of the
      A-Hermitian part of J at or above z (all modes, and the half spectrum with multiplicity)
  A4  resolvent: sigma_min(Ahat W(z) Ahat^-1) >= z - mu_A for z > mu_A; Euclidean variant with mu_2
  A5  Thomas on the complex blocks W_m = z I - B_m: Re pivot >= z - lambda_max(Ht_m), multiplier
      bound, backward error of the solve
  A6  stage mass identity (z - c) M(x) = M(b) (flux form)
  A7  ||exp(tJ)||_A <= exp(t mu_A)
  A8  ||R(hJ)||_A <= sup over Re w = h mu_A of |R(w)|, R the RODAS5P stability function built from
      fixtures/rodas5p_coefficients_snapshot.json, z = 1/(h gamma)
  A9  A-normal cases (zero velocity, solid-body rotation): the count equals #{Re l >= z} and the
      resolvent norm equals 1/dist(z, spec J)
  A10 zonal flow: mu_A = c and the resolvent bound is attained
  A11 mu_A <= c + max(-div)/2 (Weyl)
  B1  every eigenvalue of J lies in the union of the K-direction polygons P_m (T5)
  B2  every eigenvalue of J lies in the union of the Gershgorin row discs of the B_m
  B3  every eigenvalue of B_m lies in P_m
  C1  alpha_F <= alpha_G and alpha_F - c <= (alpha_G - c)/2 (proven, P3 iv)
  C2  ratios of the field-of-values bounds to the Gershgorin bounds (measured; verdict D2)

Writes results/KS_N8_FOV.json (or into the directory given as the first argument). Exit status 0 only
if every check of A, B and C1 holds and every C2 metric is finite.

Usage (repository root): python3 -I research/kernel_spectral_foundation_20261124/scripts/verify_fov.py
"""
import os

for _var in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(_var, "2")

import importlib.util
import json
import math
import pathlib
import platform
import sys
import time

import numpy as np

HERE = pathlib.Path(__file__).resolve().parents[1]
ROOT = HERE.parents[1]
K_DIRECTIONS = 256
GRIDS = [
    dict(name="G1", nt=6, np=10, radius=1.0, kappa=0.05),
    dict(name="G2", nt=9, np=16, radius=1.3, kappa=0.7),
    dict(name="G3", nt=11, np=15, radius=1.0, kappa=0.01),
]
FAMILIES = ["zero", "solid", "jet", "shear", "random_zonal", "hadley_jet", "random_both"]
ZONAL = {"zero", "solid", "jet", "shear", "random_zonal"}
A_NORMAL = {"zero", "solid"}
PECLET = [0.3, 30.0]
C_FRACTIONS = [0.0, -0.05, 0.05]
Z_ABOVE = [1e-4, 1e-2, 1.0, 1e2]
Z_BELOW = [1e-3, 0.3]
T_SCALED = [0.1, 1.0, 10.0]


# ----------------------------------------------------------------------------- grid and operator
def grid_data(nt, np_, radius, kappa):
    """Ring coefficients of DiffusionGrid::sphere (spatial.rs), plus face conductances."""
    dt = math.pi / nt
    dp = 2.0 * math.pi / np_
    theta = (np.arange(nt) + 0.5) * dt
    area = np.array([(radius * math.sin(0.5 * dt)) * (radius * dp) * (2.0 * math.sin(t)) for t in theta])
    face_sin = np.array([0.0 if f in (0, nt) else math.sin(f * dt) for f in range(nt + 1)])
    cond = kappa * face_sin * dp / dt
    north = cond[:nt] / area
    south = cond[1:] / area
    azimuth = np.array([kappa * (dt / (math.sin(t) * dp)) / a for t, a in zip(theta, area)])
    return dict(nt=nt, np=np_, radius=radius, kappa=kappa, dt=dt, dp=dp, theta=theta, area=area,
                face_sin=face_sin, cond=cond, north=north, south=south, azimuth=azimuth)


def velocity(g, family, peclet, rng):
    """Zonal speed per ring and meridional speed per face, scaled to the target cell Peclet number."""
    nt = g["nt"]
    th = g["theta"]
    faces = np.arange(nt + 1) * g["dt"]
    u = np.zeros(nt)
    v = np.zeros(nt + 1)
    if family == "solid":
        u = np.sin(th)
    elif family == "jet":
        u = np.sin(th) ** 3
    elif family == "shear":
        u = np.sin(2.0 * th)
    elif family == "random_zonal":
        u = rng.normal(size=nt)
    elif family == "hadley_jet":
        u = np.sin(th) ** 3
        v = np.sin(2.0 * faces)
    elif family == "random_both":
        u = rng.normal(size=nt)
        v = rng.normal(size=nt + 1)
    v[0] = v[nt] = 0.0
    pe_u = np.abs(u) * g["radius"] * np.sin(th) * g["dp"] / (2.0 * g["kappa"])
    pe_v = np.abs(v) * g["radius"] * g["dt"] / (2.0 * g["kappa"])
    if pe_u.max() > 0:
        u = u * (peclet / pe_u.max())
    if pe_v.max() > 0:
        v = v * (peclet / pe_v.max())
    b = u * g["radius"] * g["dt"] / g["area"]
    phi = v * g["radius"] * g["face_sin"] * g["dp"]
    return u, v, b, phi


def dense_operator(g, b, phi, scheme, c):
    """J = L + c I + T as a dense real matrix, cell (r, x) at index r * np + x."""
    nt, np_ = g["nt"], g["np"]
    A = g["area"]
    n = nt * np_
    J = np.zeros((n, n))
    for r in range(nt):
        for x in range(np_):
            i = r * np_ + x
            xm = r * np_ + (x - 1) % np_
            xp = r * np_ + (x + 1) % np_
            a = g["azimuth"][r]
            J[i, xm] += a
            J[i, xp] += a
            J[i, i] -= 2.0 * a
            if r > 0:
                J[i, i - np_] += g["north"][r]
                J[i, i] -= g["north"][r]
            if r + 1 < nt:
                J[i, i + np_] += g["south"][r]
                J[i, i] -= g["south"][r]
            br = b[r]
            if scheme == "upwind":
                if br >= 0:
                    J[i, xm] += br
                    J[i, i] -= br
                else:
                    J[i, xp] -= br
                    J[i, i] += br
            else:
                J[i, xm] += 0.5 * br
                J[i, xp] -= 0.5 * br
            fin, fout = phi[r], phi[r + 1]
            if scheme == "upwind":
                if fin > 0:
                    J[i, i - np_] += fin / A[r]
                elif fin < 0:
                    J[i, i] += fin / A[r]
                if fout > 0:
                    J[i, i] -= fout / A[r]
                elif fout < 0:
                    J[i, i + np_] -= fout / A[r]
            else:
                if r > 0:
                    J[i, i - np_] += 0.5 * fin / A[r]
                J[i, i] += 0.5 * (fin - fout) / A[r]
                if r + 1 < nt:
                    J[i, i + np_] -= 0.5 * fout / A[r]
            J[i, i] += c
    return J


def mode_block(g, b, phi, scheme, c, m):
    """B_m of DERIVATION L2 (complex tridiagonal, nt x nt)."""
    nt, np_ = g["nt"], g["np"]
    A = g["area"]
    th = 2.0 * math.pi * m / np_
    sig = 4.0 * math.sin(math.pi * m / np_) ** 2
    B = np.zeros((nt, nt), dtype=complex)
    for r in range(nt):
        d = c - g["north"][r] - g["south"][r] - g["azimuth"][r] * sig - 1j * b[r] * math.sin(th)
        lo = g["north"][r]
        up = g["south"][r]
        fin, fout = phi[r], phi[r + 1]
        if scheme == "upwind":
            d += -abs(b[r]) * (1.0 - math.cos(th))
            lo += max(fin, 0.0) / A[r]
            up += -min(fout, 0.0) / A[r]
            d += (min(fin, 0.0) - max(fout, 0.0)) / A[r]
        else:
            lo += 0.5 * fin / A[r]
            up += -0.5 * fout / A[r]
            d += 0.5 * (fin - fout) / A[r]
        B[r, r] = d
        if r > 0:
            B[r, r - 1] = lo
        if r + 1 < nt:
            B[r, r + 1] = up
    return B


def closed_form_parts(g, b, phi, scheme, c, m):
    """Ht_m and Kt_m from the closed forms of DERIVATION L4."""
    nt, np_ = g["nt"], g["np"]
    A = g["area"]
    th = 2.0 * math.pi * m / np_
    sig = 4.0 * math.sin(math.pi * m / np_) ** 2
    div = (phi[1:] - phi[:-1]) / A
    H = np.zeros((nt, nt))
    K = np.zeros((nt, nt), dtype=complex)
    for r in range(nt):
        d = c - g["north"][r] - g["south"][r] - g["azimuth"][r] * sig - 0.5 * div[r]
        if scheme == "upwind":
            d -= abs(b[r]) * (1.0 - math.cos(th))
            d -= (abs(phi[r]) + abs(phi[r + 1])) / (2.0 * A[r])
        H[r, r] = d
        K[r, r] = -b[r] * math.sin(th)
        if r + 1 < nt:
            e = math.sqrt(g["north"][r + 1] * g["south"][r])
            if scheme == "upwind":
                e += abs(phi[r + 1]) / (2.0 * math.sqrt(A[r] * A[r + 1]))
            H[r, r + 1] = H[r + 1, r] = e
            k = 1j * phi[r + 1] / (2.0 * math.sqrt(A[r] * A[r + 1]))
            K[r, r + 1] = k
            K[r + 1, r] = np.conj(k)
    return H, K


# ----------------------------------------------------------------------------- linear algebra helpers
def thomas(lower, diag, upper):
    """Pivots and multipliers of a tridiagonal matrix without pivoting (lower[r] = M[r][r-1])."""
    n = len(diag)
    p = np.empty(n, dtype=diag.dtype)
    l = np.zeros(n, dtype=diag.dtype)
    p[0] = diag[0]
    for r in range(1, n):
        l[r] = lower[r] / p[r - 1]
        p[r] = diag[r] - l[r] * upper[r - 1]
    return p, l


def thomas_solve(p, l, upper, rhs):
    x = rhs.astype(complex).copy()
    n = len(p)
    for r in range(1, n):
        x[r] -= l[r] * x[r - 1]
    x[n - 1] /= p[n - 1]
    for r in range(n - 2, -1, -1):
        x[r] = (x[r] - upper[r] * x[r + 1]) / p[r]
    return x


def tridiag_parts(M):
    n = M.shape[0]
    lower = np.zeros(n, dtype=M.dtype)
    upper = np.zeros(n, dtype=M.dtype)
    for r in range(n):
        if r > 0:
            lower[r] = M[r, r - 1]
        if r + 1 < n:
            upper[r] = M[r, r + 1]
    return lower, np.diag(M).copy(), upper


def nonpositive_pivots(H, z):
    """Count of non-positive (or NaN) Thomas pivots of z I - H, H real symmetric tridiagonal."""
    lower, diag, upper = tridiag_parts(z * np.eye(H.shape[0]) - H)
    p, _ = thomas(lower.real, diag.real, upper.real)
    return int(np.sum(np.isnan(p) | (p <= 0.0)))


def expm(X):
    """Scaling and squaring with a degree-19 Taylor polynomial (||X / 2^s||_1 <= 1/4)."""
    n = X.shape[0]
    nrm = np.linalg.norm(X, 1)
    s = int(math.ceil(math.log2(nrm / 0.25))) if nrm > 0.25 else 0
    Y = X / (2.0 ** s)
    E = np.eye(n)
    term = np.eye(n)
    for k in range(1, 20):
        term = term @ Y / k
        E = E + term
    for _ in range(s):
        E = E @ E
    return E


def sigma(M):
    return np.linalg.svd(M, compute_uv=False)


# ----------------------------------------------------------------------------- RODAS5P
def parse_number(text):
    if "/" in text:
        num, den = text.split("/", 1)
        return float(num) / float(den)
    return float(text)


def rodas5p():
    raw = json.loads((ROOT / "fixtures" / "rodas5p_coefficients_snapshot.json").read_text())
    gamma = parse_number(raw["gamma"])
    A = np.array([[parse_number(s) for s in row] for row in raw["A"]])
    C = np.array([[parse_number(s) for s in row] for row in raw["C"]])
    b_code = np.array([parse_number(s) for s in raw["b_code"]])
    s = len(b_code)
    Gamma = np.linalg.inv(np.eye(s) / gamma - C)
    beta = A @ Gamma + Gamma
    b = Gamma.T @ b_code
    return dict(gamma=gamma, beta=beta, b=b, stages=s)


def r_scalar(tab, w):
    """R(w) = 1 + w b^T (I - w beta)^{-1} 1, vectorised over w."""
    w = np.asarray(w, dtype=complex)
    beta, gamma = tab["beta"], tab["gamma"]
    k = []
    for i in range(tab["stages"]):
        acc = np.ones_like(w)
        for j in range(i):
            acc = acc + w * beta[i, j] * k[j]
        k.append(acc / (1.0 - gamma * w))
    return 1.0 + w * sum(tab["b"][i] * k[i] for i in range(tab["stages"]))


def r_matrix(tab, Z):
    n = Z.shape[0]
    M = np.eye(n) - tab["gamma"] * Z
    ks = []
    for i in range(tab["stages"]):
        acc = np.eye(n)
        for j in range(i):
            acc = acc + tab["beta"][i, j] * ks[j]
        ks.append(np.linalg.solve(M, Z @ acc))
    return np.eye(n) + sum(tab["b"][i] * ks[i] for i in range(tab["stages"]))


def sup_on_line(tab, x0):
    """Lower estimate of sup_y |R(x0 + i y)| (dense sampling plus golden refinement); a check
    ||R(hJ)|| <= this value is therefore stricter than the theorem."""
    ys = np.concatenate([[0.0], np.logspace(-6, 9, 6000)])
    vals = np.abs(r_scalar(tab, x0 + 1j * ys))
    k = int(np.argmax(vals))
    best = float(vals[k])
    lo = ys[max(k - 1, 0)]
    hi = ys[min(k + 1, len(ys) - 1)]
    g = (math.sqrt(5.0) - 1.0) / 2.0
    a, bb = lo, hi
    for _ in range(80):
        c1 = bb - g * (bb - a)
        c2 = a + g * (bb - a)
        f1 = abs(complex(r_scalar(tab, x0 + 1j * c1)))
        f2 = abs(complex(r_scalar(tab, x0 + 1j * c2)))
        best = max(best, f1, f2)
        if f1 > f2:
            bb = c2
        else:
            a = c1
    return best


# ----------------------------------------------------------------------------- one case
def polygon(Ht, Kt, phis):
    """Support values h(phi_k) = lambda_max(cos phi_k Ht + sin phi_k Kt) and the polygon vertices."""
    stack = np.cos(phis)[:, None, None] * Ht[None, :, :] + np.sin(phis)[:, None, None] * Kt[None, :, :]
    h = np.linalg.eigvalsh(stack)[:, -1]
    K = len(phis)
    d = 2.0 * math.pi / K
    s = math.sin(d)
    nxt = np.roll(np.arange(K), -1)
    x = (h * np.sin(phis[nxt]) - h[nxt] * np.sin(phis)) / s
    y = (np.cos(phis) * h[nxt] - np.cos(phis[nxt]) * h) / s
    verts = x + 1j * y
    return h, verts


def violation(points, h, phis):
    """max_k (Re(e^{-i phi_k} w) - h_k) for each point w (positive: outside the polygon)."""
    if len(points) == 0:
        return np.zeros(0)
    proj = np.outer(points.real, np.cos(phis)) + np.outer(points.imag, np.sin(phis))
    return (proj - h[None, :]).max(axis=1)


def run_case(g, family, peclet, scheme, c_frac, c_scale, tab, rng_v, rng_rhs, record):
    nt, np_ = g["nt"], g["np"]
    n = nt * np_
    A = g["area"]
    c = c_frac * c_scale
    u, v, b, phi = velocity(g, family, peclet, rng_v)
    div = (phi[1:] - phi[:-1]) / A
    J = dense_operator(g, b, phi, scheme, c)
    sq = np.sqrt(np.repeat(A, np_))
    S_A = (J * sq[:, None]) / sq[None, :]
    blocks = [mode_block(g, b, phi, scheme, c, m) for m in range(np_)]
    ahat = np.sqrt(A)
    tblocks = [(B * ahat[:, None]) / ahat[None, :] for B in blocks]
    Hts = [0.5 * (Bt + Bt.conj().T) for Bt in tblocks]
    Kts = [(Bt - Bt.conj().T) / 2j for Bt in tblocks]
    out = dict(grid=g["name"], family=family, peclet=None if family == "zero" else peclet, scheme=scheme,
               c=c, n=n, max_zonal_peclet=float(np.max(np.abs(u) * g["radius"] * np.sin(g["theta"]) * g["dp"] / (2 * g["kappa"]))),
               max_meridional_peclet=float(np.max(np.abs(v) * g["radius"] * g["dt"] / (2 * g["kappa"]))),
               max_convergence=float(np.max(-div)))
    # Gershgorin (row discs of the mode blocks)
    centres = []
    radii = []
    for B in blocks:
        lower, diag, upper = tridiag_parts(B)
        centres.append(diag)
        radii.append(np.abs(lower) + np.abs(upper))
    centres = np.concatenate(centres)
    radii = np.concatenate(radii)
    rho_G = float(np.max(np.abs(centres - c) + radii))
    alpha_G = float(np.max(centres.real + radii))
    iota_G = float(np.max(np.abs(centres.imag) + radii))
    left_G = float(np.min(centres.real - radii))
    scale = max(1.0, rho_G + abs(c))
    tol = 1e-11 * scale
    # A0 block diagonalisation
    F = np.exp(-2j * math.pi * np.outer(np.arange(np_), np.arange(np_)) / np_) / math.sqrt(np_)
    P = np.kron(np.eye(nt), F)
    Jhat = P @ J @ P.conj().T
    Jhat4 = Jhat.reshape(nt, np_, nt, np_)
    diagonal = np.einsum("imjm->mij", Jhat4)
    off = Jhat4.copy()
    for m in range(np_):
        off[:, m, :, m] = 0.0
    a0_off = float(np.abs(off).max())
    a0_mis = float(max(np.abs(diagonal[m] - blocks[m]).max() for m in range(np_)))
    record("A0_block_diagonalisation", a0_off <= tol and a0_mis <= tol, max(a0_off, a0_mis) / scale)
    # A1 closed forms
    a1 = 0.0
    for m in range(np_):
        Hc, Kc = closed_form_parts(g, b, phi, scheme, c, m)
        a1 = max(a1, np.abs(Hts[m] - Hc).max(), np.abs(Kts[m] - Kc).max())
    record("A1_hermitian_part_closed_form", a1 <= tol, a1 / scale)
    Hts = [H.real for H in Hts]
    # A2 log norms
    Hd = 0.5 * (S_A + S_A.T)
    eig_hd = np.linalg.eigvalsh(Hd)
    mu_dense = float(eig_hd[-1])
    mu_modes = float(max(np.linalg.eigvalsh(H)[-1] for H in Hts))
    mu2_dense = float(np.linalg.eigvalsh(0.5 * (J + J.T))[-1])
    mu2_modes = float(max(np.linalg.eigvalsh(0.5 * (B + B.conj().T))[-1] for B in blocks))
    a2 = max(abs(mu_dense - mu_modes), abs(mu2_dense - mu2_modes))
    record("A2_log_norm_grid_equals_modes", a2 <= tol, a2 / scale)
    mu = mu_modes
    out.update(mu_A=mu, mu_2=mu2_modes, mu2_minus_muA=mu2_modes - mu)
    # spectrum
    lam = np.linalg.eigvals(J)
    alpha_true = float(lam.real.max())
    rho_true = float(np.abs(lam - c).max())
    # A11 Weyl bound and A10 zonal identity
    record("A11_weyl_upper_bound", mu <= c + 0.5 * max(0.0, float(np.max(-div))) + tol, (mu - c - 0.5 * float(np.max(-div))) / scale)
    if family in ZONAL:
        record("A10_zonal_mu_equals_c", abs(mu - c) <= tol, abs(mu - c) / scale)
    # z values
    z_above = [mu + d * scale for d in Z_ABOVE]
    spread = mu - float(eig_hd[0])
    z_below = [mu - d * spread for d in Z_BELOW] + [float(eig_hd[0]) + rng_rhs.uniform(0.0, 1.0) * spread]
    z_below = [z for z in z_below if np.min(np.abs(eig_hd - z)) > 1e-9 * scale]
    # A3 inertia of the Hermitian part
    a3_ok = True
    distinct = np_ // 2 + 1
    for z in z_above + z_below:
        dense_count = int(np.sum(eig_hd >= z))
        all_modes = sum(nonpositive_pivots(H, z) for H in Hts)
        half = sum((1 if (m == 0 or 2 * m == np_) else 2) * nonpositive_pivots(Hts[m], z) for m in range(distinct))
        a3_ok &= dense_count == all_modes == half
        if z > mu:
            a3_ok &= all_modes == 0
    record("A3_hermitian_inertia_count", a3_ok, 0.0 if a3_ok else 1.0)
    # A4 resolvent, A5 complex Thomas, A6 mass, A9 A-normal
    worst_ratio_A = math.inf
    worst_ratio_2 = math.inf
    worst_pivot = math.inf
    worst_mult = -math.inf
    worst_backward = 0.0
    worst_mass = 0.0
    attained = 0.0
    varah = []
    for z in z_above:
        W = z * np.eye(n) - J
        WA = z * np.eye(n) - S_A
        smin = float(sigma(WA)[-1])
        ratio = smin / (z - mu)
        worst_ratio_A = min(worst_ratio_A, ratio)
        record("A4_resolvent_bound_A", smin >= (z - mu) - 1e-11 * (abs(z) + scale), (z - mu - smin) / (abs(z) + scale))
        if family in ZONAL:
            attained = max(attained, abs(ratio - 1.0))
        if z > mu2_modes:
            s2 = float(sigma(W)[-1])
            worst_ratio_2 = min(worst_ratio_2, s2 / (z - mu2_modes))
            record("A4_resolvent_bound_euclidean", s2 >= (z - mu2_modes) - 1e-11 * (abs(z) + scale), (z - mu2_modes - s2) / (abs(z) + scale))
        if family in A_NORMAL:
            dist = float(np.min(np.abs(z - lam)))
            record("A9_normal_resolvent_equals_distance", abs(smin - dist) <= 1e-9 * (abs(z) + scale), abs(smin - dist) / (abs(z) + scale))
        for m in range(np_):
            lower, diag, upper = tridiag_parts(z * np.eye(nt) - blocks[m])
            p, l = thomas(lower, diag, upper)
            lam_h = float(np.linalg.eigvalsh(Hts[m])[-1])
            worst_pivot = min(worst_pivot, float(np.min(p.real)) - (z - lam_h))
            if nt > 1:
                bound = np.abs(lower[1:]) / (z - mu)
                worst_mult = max(worst_mult, float(np.max(np.abs(l[1:]) - bound)))
            rhs = rng_rhs.normal(size=nt) + 1j * rng_rhs.normal(size=nt)
            x = thomas_solve(p, l, upper, rhs)
            Wm = z * np.eye(nt) - blocks[m]
            be = np.abs(Wm @ x - rhs).max() / (np.abs(Wm).sum(axis=1).max() * np.abs(x).max() + np.abs(rhs).max())
            worst_backward = max(worst_backward, float(be))
        bvec = rng_rhs.normal(size=n)
        x = np.linalg.solve(W, bvec)
        Aw = np.repeat(A, np_)
        defect = abs((z - c) * (Aw @ x) - Aw @ bvec)
        norm = Aw.sum() * ((abs(z - c) + rho_G) * np.abs(x).max() + np.abs(bvec).max())
        worst_mass = max(worst_mass, float(defect / norm))
        rows = np.abs(W).sum(axis=1) - np.abs(np.diag(W))
        dom = float(np.min(np.abs(np.diag(W)) - rows))
        varah.append(dict(z=z, fov_bound_A=1.0 / (z - mu), true_A=1.0 / smin,
                          varah_bound_inf=(1.0 / dom) if dom > 0 else None,
                          true_inf=float(np.abs(np.linalg.inv(W)).sum(axis=1).max())))
    record("A5_complex_thomas_pivots", worst_pivot >= -1e-11 * scale, -worst_pivot / scale)
    record("A5_complex_thomas_multipliers", worst_mult <= 1e-9, worst_mult)
    record("A5_complex_thomas_backward_error", worst_backward <= 1e-13, worst_backward)
    record("A6_stage_mass_identity", worst_mass <= 1e-13, worst_mass)
    if family in ZONAL:
        record("A10_zonal_bound_attained", attained <= 1e-8, attained)
    if family in A_NORMAL:
        a9 = True
        for z in z_above + z_below:
            a9 &= int(np.sum(eig_hd >= z)) == int(np.sum(lam.real >= z - 1e-9 * scale))
        record("A9_normal_count_equals_eigenvalue_count", a9, 0.0 if a9 else 1.0)
    out.update(resolvent_ratio_A_min=worst_ratio_A, resolvent_ratio_2_min=worst_ratio_2 if math.isfinite(worst_ratio_2) else None,
               pivot_margin_min=worst_pivot / scale, thomas_backward_error=worst_backward, mass_defect=worst_mass, resolvent=varah)
    # A7 exponential
    Ahat = sq
    worst_exp = -math.inf
    for tau in T_SCALED:
        t = tau / max(rho_G, 1e-300)
        E = expm(t * J)
        nrm = float(sigma((E * Ahat[:, None]) / Ahat[None, :])[0])
        worst_exp = max(worst_exp, nrm / math.exp(t * mu) - 1.0)
    record("A7_exponential_bound", worst_exp <= 1e-9, worst_exp)
    out["exp_norm_over_bound_minus_1_max"] = worst_exp
    # A8 RODAS5P
    worst_r = -math.inf
    for z in z_above[1:]:
        if z <= 0:
            continue
        h = 1.0 / (tab["gamma"] * z)
        R = r_matrix(tab, h * J)
        nrm = float(sigma((R * Ahat[:, None]) / Ahat[None, :])[0])
        bound = sup_on_line(tab, h * mu)
        worst_r = max(worst_r, nrm / bound - 1.0)
    if math.isfinite(worst_r):
        record("A8_rodas5p_von_neumann_bound", worst_r <= 1e-9, worst_r)
    out["rodas5p_norm_over_bound_minus_1_max"] = worst_r if math.isfinite(worst_r) else None
    # B polygons and enclosures
    phis = 2.0 * math.pi * np.arange(K_DIRECTIONS) / K_DIRECTIONS
    slack = 1e-9 * scale
    rho_F = 0.0
    alpha_F = -math.inf
    iota_F = 0.0
    left_F = math.inf
    vertex_violation = 0.0
    union_violation = np.full(len(lam), math.inf)
    b3 = 0.0
    for m in range(np_):
        h, verts = polygon(Hts[m], Kts[m], phis)
        rho_F = max(rho_F, float(np.abs(verts - c).max()))
        alpha_F = max(alpha_F, float(h[0]))
        iota_F = max(iota_F, float(h[K_DIRECTIONS // 4]), float(h[3 * K_DIRECTIONS // 4]))
        left_F = min(left_F, float(-h[K_DIRECTIONS // 2]))
        vertex_violation = max(vertex_violation, float(violation(verts, h, phis).max()))
        union_violation = np.minimum(union_violation, violation(lam, h, phis))
        b3 = max(b3, float(violation(np.linalg.eigvals(blocks[m]), h, phis).max()))
    record("B0_polygon_vertices_consistent", vertex_violation <= slack, vertex_violation / scale)
    record("B1_eigenvalues_in_fov_union", float(union_violation.max()) <= slack, float(union_violation.max()) / scale)
    gersh = np.min(np.abs(lam[:, None] - centres[None, :]) - radii[None, :], axis=1)
    record("B2_eigenvalues_in_gershgorin_union", float(gersh.max()) <= slack, float(gersh.max()) / scale)
    record("B3_mode_eigenvalues_in_mode_polygon", b3 <= slack, b3 / scale)
    # C1 proven inequality, C2 ratios
    c1 = alpha_F <= alpha_G + 1e-12 * scale and (alpha_F - c) <= 0.5 * (alpha_G - c) + 1e-12 * scale
    record("C1_alpha_F_le_alpha_G", c1, (alpha_F - alpha_G) / scale)
    mu_inf = float(np.max(np.diag(J) + np.abs(J).sum(axis=1) - np.abs(np.diag(J))))
    Aw = np.repeat(A, np_)
    colw = (np.abs(J) * Aw[:, None]).sum(axis=0) / Aw - np.abs(np.diag(J))
    mu_1A = float(np.max(np.diag(J) + colw))
    nonnormal = max(float(np.linalg.norm(Bt @ Bt.conj().T - Bt.conj().T @ Bt) / max(np.linalg.norm(Bt) ** 2, 1e-300)) for Bt in tblocks)
    out.update(
        rho_F=rho_F, rho_G=rho_G, rho_true=rho_true, rho_ratio=rho_F / rho_G,
        alpha_F=alpha_F, alpha_G=alpha_G, alpha_true=alpha_true,
        alpha_excess_F=alpha_F - c, alpha_excess_G=alpha_G - c,
        iota_F=iota_F, iota_G=iota_G, iota_ratio=(iota_F / iota_G) if iota_G > 0 else None,
        left_F=left_F, left_G=left_G, left_ratio=(c - left_F) / (c - left_G) if c - left_G > 0 else None,
        grid_row_gershgorin_abscissa=mu_inf, weighted_column_log_norm=mu_1A,
        departure_from_normality=nonnormal, scale=scale,
    )
    finite = all(math.isfinite(x) for x in (rho_F, rho_G, alpha_F, alpha_G, iota_F, iota_G, left_F, left_G))
    record("C2_metrics_finite", finite, 0.0 if finite else 1.0)
    strict = rho_F < rho_G * (1.0 - 1e-9) or alpha_F < alpha_G - 1e-9 * scale
    looser = rho_F > rho_G * (1.0 + 1e-12) or alpha_F > alpha_G + 1e-12 * scale
    out.update(d2_strict=bool(strict), d2_looser=bool(looser))
    return out


def clean(obj):
    if isinstance(obj, dict):
        return {k: clean(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [clean(v) for v in obj]
    if isinstance(obj, (bool, np.bool_)):
        return bool(obj)
    if isinstance(obj, (int, np.integer)):
        return int(obj)
    if isinstance(obj, (float, np.floating)):
        return float(obj) if math.isfinite(obj) else None
    return obj


def main():
    started = time.time()
    out_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    checks = {}

    def record(name, ok, value):
        entry = checks.setdefault(name, dict(ok=True, evaluations=0, failures=0, worst=-math.inf))
        if not math.isfinite(float(value)):
            ok = False
            value = math.inf
        entry["evaluations"] += 1
        entry["ok"] = entry["ok"] and bool(ok)
        entry["failures"] += 0 if ok else 1
        entry["worst"] = max(entry["worst"], float(value))

    # ring data of verify_foundation.py (same formulas as spatial.rs); no bytecode cache in the tree
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("ks_verify_foundation", HERE / "scripts" / "verify_foundation.py")
    foundation = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(foundation)
    tab = rodas5p()
    ys = np.concatenate([np.linspace(0.0, 50.0, 20001), np.logspace(-6, 9, 4000)])
    a_stab = float(np.abs(r_scalar(tab, 1j * ys)).max())
    record("R0_rodas5p_A_stable_on_imaginary_axis_sampled", a_stab <= 1.0 + 1e-12, a_stab - 1.0)
    w = np.array([1e-2, -1e-2, 1e-2j])
    order_err = float(np.abs(r_scalar(tab, w) - np.exp(w)).max())
    record("R0_rodas5p_order_consistency", order_err <= 1e-10, order_err)
    r_inf = abs(complex(r_scalar(tab, -1e12)))
    rodas = dict(gamma=tab["gamma"], stages=tab["stages"], max_abs_R_on_imaginary_axis_sampled=a_stab,
                 max_abs_R_minus_exp_at_abs_w_0p01=order_err, abs_R_at_minus_1e12=r_inf)
    cases = []
    grids = []
    for gi, spec_g in enumerate(GRIDS):
        g = grid_data(spec_g["nt"], spec_g["np"], spec_g["radius"], spec_g["kappa"])
        g["name"] = spec_g["name"]
        ref = foundation.rings(spec_g["nt"], spec_g["np"], spec_g["radius"], spec_g["kappa"])
        rd = max(abs(r[key] - mine[i]) / max(abs(r[key]), 1e-300)
                 for i, r in enumerate(ref)
                 for key, mine in (("area", g["area"]), ("north", g["north"]), ("south", g["south"]), ("azimuth", g["azimuth"])))
        record("G0_ring_data_matches_verify_foundation", rd <= 1e-14, rd)
        c_scale = float(max(2 * (nn + ss) + 4 * aa for nn, ss, aa in zip(g["north"], g["south"], g["azimuth"])))
        grids.append(dict(name=spec_g["name"], nt=spec_g["nt"], np=spec_g["np"], radius=spec_g["radius"],
                          kappa=spec_g["kappa"], n=spec_g["nt"] * spec_g["np"], code_radius_bound=c_scale))
        rng_v = np.random.default_rng(20261125 + gi)
        rng_rhs = np.random.default_rng(1125 + gi)
        for family in FAMILIES:
            pes = [None] if family == "zero" else PECLET
            schemes = ["none"] if family == "zero" else ["centred", "upwind"]
            for pe in pes:
                state = rng_v.bit_generator.state
                for scheme in schemes:
                    for c_frac in C_FRACTIONS:
                        rng_v.bit_generator.state = state
                        res = run_case(g, family, pe if pe is not None else 0.0, "centred" if scheme == "none" else scheme,
                                       c_frac, c_scale, tab, rng_v, rng_rhs, record)
                        res["scheme"] = scheme
                        cases.append(res)
                        print(f"{res['grid']} {family:12s} pe={pe} {scheme:7s} c={res['c']:+.3e} mu_A-c={res['mu_A'] - res['c']:+.3e} "
                              f"rho_F/rho_G={res['rho_ratio']:.4f} alpha_F-c={res['alpha_excess_F']:+.3e} alpha_G-c={res['alpha_excess_G']:+.3e} "
                              f"iota={res['iota_ratio'] if res['iota_ratio'] is None else round(res['iota_ratio'], 4)} "
                              f"nonnormal={res['departure_from_normality']:.2e}", flush=True)
    # verdicts
    validity = [k for k in checks if k[0] in "ABCGR" and k != "C2_metrics_finite"]
    d1 = all(checks[k]["ok"] for k in validity)
    moving = [c for c in cases if c["family"] != "zero"]
    if not any(c["d2_strict"] for c in cases):
        d2 = "KILL"
    elif any(c["d2_looser"] for c in cases):
        d2 = "FAIL"
    elif all(c["d2_strict"] for c in moving):
        d2 = "PASS"
    else:
        d2 = "NOT_DECIDABLE"

    def stats(key, subset=None):
        vals = [c[key] for c in (subset or cases) if c.get(key) is not None and math.isfinite(c[key])]
        return dict(min=min(vals), max=max(vals), count=len(vals)) if vals else None

    zonal_cases = [c for c in cases if c["family"] in ZONAL and c["family"] != "zero"]
    divergent = [c for c in cases if c["family"] not in ZONAL]
    high_pe_centred = [c for c in cases if c["peclet"] == 30.0 and c["scheme"] == "centred"]
    summary = dict(
        cases=len(cases),
        rho_ratio_all=stats("rho_ratio"),
        rho_ratio_zero_velocity=stats("rho_ratio", [c for c in cases if c["family"] == "zero"]),
        rho_ratio_zonal=stats("rho_ratio", zonal_cases),
        rho_ratio_divergent=stats("rho_ratio", divergent),
        iota_ratio_moving=stats("iota_ratio", moving),
        left_ratio_all=stats("left_ratio"),
        alpha_excess_F_divergent=stats("alpha_excess_F", divergent),
        alpha_excess_G_divergent=stats("alpha_excess_G", divergent),
        alpha_excess_G_high_peclet_centred=stats("alpha_excess_G", high_pe_centred),
        mu2_minus_muA=stats("mu2_minus_muA"),
        resolvent_ratio_A_min=stats("resolvent_ratio_A_min"),
        grid_row_gershgorin_minus_c_high_peclet_centred=dict(
            min=min(c["grid_row_gershgorin_abscissa"] - c["c"] for c in high_pe_centred),
            max=max(c["grid_row_gershgorin_abscissa"] - c["c"] for c in high_pe_centred)),
        weighted_column_log_norm_minus_c=dict(
            min=min(c["weighted_column_log_norm"] - c["c"] for c in cases),
            max=max(c["weighted_column_log_norm"] - c["c"] for c in cases)),
        departure_from_normality=stats("departure_from_normality"),
        strict_cases=sum(1 for c in cases if c["d2_strict"]),
        looser_cases=sum(1 for c in cases if c["d2_looser"]),
        looser_case_list=[dict(grid=c["grid"], family=c["family"], peclet=c["peclet"], scheme=c["scheme"], c=c["c"],
                               rho_ratio=c["rho_ratio"], rho_true_over_rho_G=c["rho_true"] / c["rho_G"],
                               alpha_excess_ratio=(c["alpha_excess_F"] / c["alpha_excess_G"]) if c["alpha_excess_G"] > 0 else None)
                          for c in cases if c["d2_looser"]],
        rho_ratio_by_family={f: stats("rho_ratio", [c for c in cases if c["family"] == f]) for f in FAMILIES},
        rho_true_over_rho_F=dict(min=min(c["rho_true"] / c["rho_F"] for c in cases), max=max(c["rho_true"] / c["rho_F"] for c in cases)),
        alpha_excess_ratio_divergent=dict(
            min=min(c["alpha_excess_F"] / c["alpha_excess_G"] for c in divergent),
            max=max(c["alpha_excess_F"] / c["alpha_excess_G"] for c in divergent)),
        alpha_F_minus_spectral_abscissa=dict(
            zonal_max=max(c["alpha_F"] - c["alpha_true"] for c in cases if c["family"] in ZONAL),
            divergent_min=min(c["alpha_F"] - c["alpha_true"] for c in divergent),
            divergent_max=max(c["alpha_F"] - c["alpha_true"] for c in divergent)),
        mu2_minus_muA_zonal=stats("mu2_minus_muA", [c for c in cases if c["family"] in ZONAL]),
        mu2_minus_muA_divergent=stats("mu2_minus_muA", divergent),
        resolvent_bound_over_true_nearest_z=dict(
            zonal_max=max(c["resolvent"][0]["fov_bound_A"] / c["resolvent"][0]["true_A"] for c in cases if c["family"] in ZONAL),
            divergent_min=min(c["resolvent"][0]["fov_bound_A"] / c["resolvent"][0]["true_A"] for c in divergent),
            divergent_max=max(c["resolvent"][0]["fov_bound_A"] / c["resolvent"][0]["true_A"] for c in divergent)),
        z_values_where_varah_inf_bound_does_not_apply=sum(1 for c in cases for r in c["resolvent"] if r["varah_bound_inf"] is None),
        z_values_total=sum(len(c["resolvent"]) for c in cases),
    )
    result = dict(
        task="KS-N8",
        script="research/kernel_spectral_foundation_20261124/scripts/verify_fov.py",
        derivation="research/kernel_spectral_foundation_20261124/ks_n8/DERIVATION.txt",
        environment=dict(python=platform.python_version(), numpy=np.__version__, machine=platform.machine()),
        settings=dict(k_directions=K_DIRECTIONS, peclet=PECLET, c_fractions_of_code_radius_bound=C_FRACTIONS,
                      z_above_mu_A_in_scale=Z_ABOVE, z_below_mu_A_in_hermitian_spread=Z_BELOW, t_times_rho_G=T_SCALED,
                      families=FAMILIES, schemes=["centred", "upwind"]),
        rules=dict(
            D1="PASS iff every check of A, B, C1, G0 and R0 holds on all 3 grids and all cases",
            D2=("reference: row Gershgorin discs of the mode blocks B_m (THEORY 5.2). PASS iff rho_F <= rho_G (1 + 1e-12) "
                "and alpha_F <= alpha_G + 1e-12 scale in every case and rho_F < rho_G (1 - 1e-9) or alpha_F < alpha_G - 1e-9 scale "
                "in every case with non-zero velocity; FAIL iff some case is looser; KILL iff no case is strictly tighter"),
            exit="0 iff every check of A, B, C1, G0, R0 holds and every C2 metric is finite"),
        grids=grids,
        rodas5p=rodas,
        checks=checks,
        summary=summary,
        discriminators={"D1_bound_holds_on_3_grids": "PASS" if d1 else "FAIL", "D2_tighter_than_gershgorin": d2},
        cases=cases,
        seconds=time.time() - started,
    )
    all_ok = d1 and checks["C2_metrics_finite"]["ok"]
    result["all_ok"] = all_ok
    path = out_dir / "KS_N8_FOV.json"
    path.write_text(json.dumps(clean(result), indent=1, allow_nan=False) + "\n")
    for name in sorted(checks):
        e = checks[name]
        print(f"{name:52s} {'ok' if e['ok'] else 'FAIL'} evaluations={e['evaluations']} failures={e['failures']} worst={e['worst']:.3e}")
    print("summary", json.dumps(clean(summary)))
    print("D1_bound_holds_on_3_grids", "PASS" if d1 else "FAIL")
    print("D2_tighter_than_gershgorin", d2)
    print("all_ok", all_ok, "written", path)
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
