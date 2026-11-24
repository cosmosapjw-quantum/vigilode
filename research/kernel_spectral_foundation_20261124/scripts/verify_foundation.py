#!/usr/bin/env python3
"""Numerical checks of the claims in THEORY.md (kernel/spectral foundation run, 2026-11-24).

Builds the latitude-longitude finite-volume heat operator of
crates/vigilode/src/spatial.rs (same ring coefficients, same stencil) and checks:

  F1  A L is symmetric and negative semidefinite (discrete Hodge form, A = cell areas)
  F2  ker L = constants (dim H^0 = 1 on the connected cell graph)
  F3  the longitude DFT block-diagonalises L into tridiagonal blocks L_m that depend on m
      only through cos(2 pi m / np) (Z_np symmetry plus the reflection involution)
  F4  Thomas pivots of W_m = z I - J_m have the signs of the inertia of A W_m, so the count
      of non-positive pivots equals #{eigenvalues of J at or above z} (Sylvester)
  F5  the Gershgorin bound rho_bar = max_r 2(n_r + s_r) + 4 a_r bounds the spectral radius
  F6  the polar azimuthal rate grows like 4 nt^2 / pi^2 times the equatorial one (pole problem)
  F7  (z - c) M(W^{-1} b) = M(b) for the area mass M(u) = sum A_i u_i
  F8  two-for-one packing: one complex FFT of u + i v returns both real spectra

Writes results/FOUNDATION_CHECKS.json next to this script's parent directory.
"""
import json
import math
import pathlib
import sys

import numpy as np


def rings(nt, np_, radius, kappa):
    dt = math.pi / nt
    dp = 2.0 * math.pi / np_
    out = []
    for i in range(nt):
        sin_mid = math.sin((i + 0.5) * dt)
        area = (radius * math.sin(0.5 * dt)) * (radius * dp) * (2.0 * sin_mid)

        def theta_rate(face):
            if face == 0 or face == nt:
                return 0.0
            return kappa * (math.sin(face * dt) * dp / dt) / area

        out.append(
            dict(
                area=area,
                north=theta_rate(i),
                south=theta_rate(i + 1),
                azimuth=kappa * (dt / (sin_mid * dp)) / area,
            )
        )
    return out


def operator(nt, np_, radius=1.0, kappa=1.0):
    rs = rings(nt, np_, radius, kappa)
    n = nt * np_
    L = np.zeros((n, n))
    for r, ring in enumerate(rs):
        for x in range(np_):
            c = r * np_ + x
            for xn in ((x - 1) % np_, (x + 1) % np_):
                L[c, r * np_ + xn] += ring["azimuth"]
                L[c, c] -= ring["azimuth"]
            if r > 0:
                L[c, (r - 1) * np_ + x] += ring["north"]
                L[c, c] -= ring["north"]
            if r + 1 < nt:
                L[c, (r + 1) * np_ + x] += ring["south"]
                L[c, c] -= ring["south"]
    areas = np.repeat([ring["area"] for ring in rs], np_)
    return L, areas, rs


def mode_block(rs, m, np_, z, c):
    nt = len(rs)
    cosm = math.cos(2.0 * math.pi * m / np_)
    lower = np.zeros(nt)
    diag = np.zeros(nt)
    upper = np.zeros(nt)
    for r, ring in enumerate(rs):
        diag[r] = z - c + ring["north"] + ring["south"] + ring["azimuth"] * (2.0 - 2.0 * cosm)
        lower[r] = -ring["north"]
        upper[r] = -ring["south"]
    return lower, diag, upper


def thomas_pivots(lower, diag, upper):
    piv = np.zeros_like(diag)
    piv[0] = diag[0]
    for r in range(1, len(diag)):
        piv[r] = diag[r] - lower[r] * upper[r - 1] / piv[r - 1]
    return piv


def main():
    out_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(__file__).resolve().parents[1] / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    checks = {}
    nt, np_ = 9, 16
    L, A, rs = operator(nt, np_, radius=1.3, kappa=0.7)
    n = nt * np_
    S = np.diag(A) @ L
    sym_err = np.abs(S - S.T).max() / np.abs(S).max()
    eig_s = np.linalg.eigvalsh(0.5 * (S + S.T))
    checks["F1_area_weighted_symmetry"] = dict(rel_asymmetry=float(sym_err), max_eig_of_AL=float(eig_s.max()), ok=bool(sym_err < 1e-13 and eig_s.max() < 1e-10 * abs(eig_s.min())))
    w, V = np.linalg.eig(L)
    zero = np.abs(w) < 1e-9 * np.abs(w).max()
    kernel = V[:, zero]
    const = np.ones(n) / math.sqrt(n)
    if kernel.shape[1] == 1:
        v = np.real(kernel[:, 0])
        v /= np.linalg.norm(v)
        align = abs(v @ const)
    else:
        align = float("nan")
    checks["F2_kernel_is_constants"] = dict(kernel_dimension=int(zero.sum()), alignment_with_constants=float(align), ok=bool(zero.sum() == 1 and abs(align - 1) < 1e-10))
    # F3: block diagonalisation by the longitude DFT.
    F = np.exp(-2j * math.pi * np.outer(np.arange(np_), np.arange(np_)) / np_) / math.sqrt(np_)
    P = np.kron(np.eye(nt), F)
    Lhat = P @ L @ P.conj().T
    off = 0.0
    for r1 in range(nt):
        for r2 in range(nt):
            block = Lhat[r1 * np_:(r1 + 1) * np_, r2 * np_:(r2 + 1) * np_]
            off = max(off, np.abs(block - np.diag(np.diag(block))).max())
    block_err = 0.0
    for m in range(np_):
        lo, di, up = mode_block(rs, m, np_, 0.0, 0.0)
        Lm = -np.diag(di) - np.diag(lo[1:], -1) - np.diag(up[:-1], 1)
        got = np.array([[Lhat[r1 * np_ + m, r2 * np_ + m] for r2 in range(nt)] for r1 in range(nt)])
        block_err = max(block_err, np.abs(got - Lm).max())
        if 0 < m < np_:
            lo2, di2, up2 = mode_block(rs, np_ - m, np_, 0.0, 0.0)
            block_err = max(block_err, np.abs(di - di2).max())
    checks["F3_dft_block_diagonalisation"] = dict(max_off_mode_coupling=float(off), max_block_mismatch=float(block_err), distinct_blocks=np_ // 2 + 1, ok=bool(off < 1e-9 and block_err < 1e-9))
    # F4: inertia from Thomas pivots.
    rng = np.random.default_rng(7)
    inertia_ok = True
    trials = []
    for trial in range(40):
        c = float(rng.uniform(-5, 40))
        z = float(rng.uniform(0.1, 60))
        J = L + c * np.eye(n)
        eig_j = np.linalg.eigvals(J).real
        above = int((eig_j >= z - 1e-12).sum())
        neg = 0
        for m in range(np_):
            lo, di, up = mode_block(rs, m, np_, z, c)
            neg += int((thomas_pivots(lo, di, up) <= 0).sum())
        trials.append(dict(c=c, z=z, eigenvalues_at_or_above_z=above, nonpositive_pivots=neg))
        inertia_ok &= above == neg
    checks["F4_inertia_from_pivots"] = dict(trials=len(trials), ok=bool(inertia_ok), sample=trials[:4])
    # F5: Gershgorin bound on the spectral radius.
    rho = float(np.abs(np.linalg.eigvals(L)).max())
    rho_bar = max(2 * (r["north"] + r["south"]) + 4 * r["azimuth"] for r in rs)
    checks["F5_gershgorin_radius"] = dict(rho=rho, rho_bar=rho_bar, ratio=rho / rho_bar, ok=bool(rho <= rho_bar * (1 + 1e-12)))
    # F6: pole problem asymptotics.
    rows = []
    for nt_ in (16, 32, 64, 128, 256):
        rr = rings(nt_, 2 * nt_, 1.0, 1.0)
        ratio = rr[0]["azimuth"] / rr[nt_ // 2]["azimuth"]
        rows.append(dict(nt=nt_, polar_over_equatorial=ratio, predicted=4 * nt_ ** 2 / math.pi ** 2, quotient=ratio / (4 * nt_ ** 2 / math.pi ** 2)))
    checks["F6_pole_problem"] = dict(rows=rows, ok=bool(abs(rows[-1]["quotient"] - 1) < 1e-3))
    # F7: mass identity of one stage solve.
    c, z = -0.3, 5.0
    b = rng.normal(size=n)
    x = np.linalg.solve(z * np.eye(n) - (L + c * np.eye(n)), b)
    lhs = (z - c) * (A @ x)
    rhs = A @ b
    checks["F7_stage_mass_identity"] = dict(rel_error=float(abs(lhs - rhs) / (np.abs(A * b).sum())), ok=bool(abs(lhs - rhs) <= 1e-12 * np.abs(A * b).sum()))
    # F8: two-for-one packing.
    u = rng.normal(size=np_)
    v = rng.normal(size=np_)
    Z = np.fft.fft(u + 1j * v)
    Zr = np.conj(np.roll(Z[::-1], 1))
    U = 0.5 * (Z + Zr)
    Vv = -0.5j * (Z - Zr)
    err = max(np.abs(U - np.fft.fft(u)).max(), np.abs(Vv - np.fft.fft(v)).max())
    checks["F8_two_for_one"] = dict(max_error=float(err), ok=bool(err < 1e-12))
    checks["all_ok"] = all(v["ok"] for k, v in checks.items() if isinstance(v, dict) and "ok" in v)
    path = out_dir / "FOUNDATION_CHECKS.json"
    path.write_text(json.dumps(checks, indent=1) + "\n")
    for k, v in checks.items():
        if isinstance(v, dict):
            print(k, "ok" if v["ok"] else "FAIL")
    print("all_ok", checks["all_ok"])
    return 0 if checks["all_ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
