"""2-D grid problems for PROBE B4 (PC-LAGGED-HWINDOW vs direct factorization). EXPLORATORY, not ledger authority.

Each problem is a dict with:
  f(t,y), ft(t,y) or None (autonomous), J(t,y) -> scipy.sparse.csr (exact Jacobian; the "JVP" is J@v),
  Alin(t) -> sparse csr declared stiff LINEAR part (for arm d), alin_const (bool), dst (None or DST data for
  the constant-coefficient Dirichlet Laplacian part), y0, span, ascale (atol = ascale*rtol), n, grid (nx, ny),
  b (unknowns per cell), nnzJ, frhs (flops per RHS), fft (flops per f_t), fjvp_state (flops of the analytic
  state JVP), colors (number of Curtis-Powell-Reid colors of J, greedy distance-2 coloring), exact(t) or None.

Problems
  bruss2d(N, alpha): 2-D extension of the harness brusselator-1d-N (stiff_benchmark.rs:315-385):
      u' = 1 + u^2 v - 4u + alpha*Lap u ;  v' = 3u - u^2 v + alpha*Lap v  on the unit square, N x N interior
      grid, dx = 1/(N+1), 5-point Laplacian, Dirichlet u = 1, v = 3 (as in 1-D); interleaved (u_k, v_k),
      k = i + N j (x fast). u0 = 1 + sin(2 pi x) sin(pi y), v0 = 3. span [0, 10], atol = rtol.
      alpha = 0.02 is the harness value (c = (N+1)^2/50 in 1-D). OFF-CONTRACT (not a corpus/holdout problem;
      the corpus brusselator-2d holdout is periodic with forcing and is NOT used).
  semilin2d(nx, ny): scientific-corpus-v2.1 semilinear-advection-diffusion-ramped (scientific_corpus_v2.rs:965-1082)
      on an nx x ny grid (32x48 is the on-contract n = 1536 calibration grid; N x N grids are off-contract),
      exact solution phi = exp(-t) sin(pi x) sin(pi y); atol = 0.01 rtol; span [0, 1]. Declared linear part
      A(t) = D Lap - a(t) Up - I (a(t) ramps 0.5 -> 4, so the 'linear part' is time dependent).
"""
import math
import numpy as np
import scipy.sparse as sp
from scipy.fft import dstn

import corpus_v2_copy as cv


def lap1(N):
    return sp.diags([np.ones(N - 1), -2 * np.ones(N), np.ones(N - 1)], [-1, 0, 1], format='csr')


def lap2(nx, ny):
    """x-fast index k = i + nx*j; unscaled 5-point (unit spacing)."""
    return (sp.kron(sp.identity(ny), lap1(nx)) + sp.kron(lap1(ny), sp.identity(nx))).tocsr()


def greedy_colors(J):
    """Curtis-Powell-Reid column coloring (columns sharing no row get one color), greedy natural order.
    Returns (ncolors, color array)."""
    P = (abs(J) > 0).astype(np.int8).tocsc()
    R = P.tocsr()
    n = J.shape[1]
    col = -np.ones(n, dtype=np.int64)
    for j in range(n):
        rows = P.indices[P.indptr[j]:P.indptr[j + 1]]
        forb = set()
        for r in rows:
            cs = R.indices[R.indptr[r]:R.indptr[r + 1]]
            forb.update(col[cs][col[cs] >= 0].tolist())
        k = 0
        while k in forb:
            k += 1
        col[j] = k
    return int(col.max() + 1), col


def check_coloring(J, col):
    """verify: recovering J from compressed products is exact (no two same-colored columns share a row)."""
    P = (abs(J) > 0).astype(np.int64).tocsr()
    nc = col.max() + 1
    S = sp.csr_matrix((np.ones(J.shape[1]), (np.arange(J.shape[1]), col)), shape=(J.shape[1], nc))
    M = P @ S
    return int(M.max()) <= 1


class DSTLap:
    """Fast exact solver for (I - s * c * Lap_h) on an N x N Dirichlet grid with b interleaved species
    (constant-coefficient linear part). Lap_h unscaled; c = alpha/dx^2."""
    def __init__(self, N, b, c):
        m = np.arange(1, N + 1)
        lam = -4.0 * np.sin(m * np.pi / (2 * (N + 1))) ** 2
        self.L = (lam[None, :] + lam[:, None])     # [j, i]
        self.N, self.b, self.c = N, b, c
        # flop model per apply: per species one forward + one inverse 2-D DST-I (2N 1-D DST-I of length N
        # each) plus N^2 divisions. FAVOURABLE model: a length-N DST-I costs 2.5 N log2 N (a real FFT of the same
        # length). PESSIMISTIC model (DST-I through a real FFT of length 2(N+1)): self.flops_pess.
        f1 = 2.5 * N * math.log2(N)
        self.flops = b * (2 * (2 * N * f1) + N * N)
        m2 = 2 * (N + 1)
        self.flops_pess = b * (2 * (2 * N * 2.5 * m2 * math.log2(m2)) + N * N)

    def solve(self, r, s):
        N, b = self.N, self.b
        out = np.empty_like(r)
        den = 1.0 - s * self.c * self.L
        for q in range(b):
            R = r[q::b].reshape(N, N)
            Z = dstn(R, type=1, norm='ortho')
            Z /= den
            out[q::b] = dstn(Z, type=1, norm='ortho').ravel()
        return out


def bruss2d(N, alpha=0.02, T=10.0):
    n = 2 * N * N
    dx = 1.0 / (N + 1)
    cc = alpha / dx ** 2
    x = (np.arange(N) + 1.0) * dx
    X, Y = np.meshgrid(x, x)              # [j, i]
    U0 = 1.0 + np.sin(2 * np.pi * X) * np.sin(np.pi * Y)
    y0 = np.empty(n); y0[0::2] = U0.ravel(); y0[1::2] = 3.0
    L2 = lap2(N, N)
    Lint = sp.kron(L2, sp.identity(2)).tocsr()       # interleaved, unscaled
    A = (cc * Lint).tocsr()

    def f(t, y):
        U = y[0::2].reshape(N, N); V = y[1::2].reshape(N, N)
        Up = np.pad(U, 1, constant_values=1.0); Vp = np.pad(V, 1, constant_values=3.0)
        lu = Up[1:-1, :-2] + Up[1:-1, 2:] + Up[:-2, 1:-1] + Up[2:, 1:-1] - 4.0 * U
        lv = Vp[1:-1, :-2] + Vp[1:-1, 2:] + Vp[:-2, 1:-1] + Vp[2:, 1:-1] - 4.0 * V
        uuv = U * U * V
        out = np.empty(n)
        out[0::2] = (1.0 + uuv - 4.0 * U + cc * lu).ravel()
        out[1::2] = (3.0 * U - uuv + cc * lv).ravel()
        return out

    ev = np.arange(0, n, 2); od = ev + 1

    def J(t, y):
        u = y[0::2]; v = y[1::2]
        rows = np.concatenate([ev, ev, od, od]); cols = np.concatenate([ev, od, ev, od])
        vals = np.concatenate([2 * u * v - 4.0, u * u, 3.0 - 2 * u * v, -u * u])
        R = sp.csr_matrix((vals, (rows, cols)), shape=(n, n))
        return (A + R).tocsr()

    Jt = J(0.0, y0)
    nc, col = greedy_colors(Jt)
    assert check_coloring(Jt, col)
    p = dict(name=f'bruss2d-{N}' + ('' if alpha == 0.02 else f'-a{alpha:g}'), f=f, ft=None, J=J,
             Alin=lambda t: A, alin_const=True, dst=DSTLap(N, 2, cc), y0=y0, span=(0.0, T), ascale=1.0, n=n,
             grid=(N, N), b=2, nnzJ=int(Jt.nnz), frhs=23.0 * N * N, fft=0.0, fjvp_state=24.0 * N * N,
             colors=nc, exact=None, alpha=alpha, stiff=float(8 * cc))
    return p


def semilin2d(nx, ny=None):
    ny = nx if ny is None else ny
    q = cv.semilinear_advection_diffusion_v2(nx * ny, grid=(nx, ny))
    D = 0.002
    Lap, Up = cv.semilinear_operator_matrices(nx, ny)
    I = sp.identity(nx * ny, format='csr')

    def Alin(t):
        r, _ = cv.smooth_ramp(t, 0.50, 0.08)
        adv = 0.5 + 3.5 * r
        return (D * Lap - adv * Up - I).tocsr()

    J = q.J_sparse
    Jt = J(0.0, q.y0)
    nc, col = greedy_colors(Jt)
    assert check_coloring(Jt, col)
    ii, jj = np.meshgrid(np.arange(nx), np.arange(ny))      # [j, i]
    col5 = ((ii + 2 * jj) % 5).ravel()                     # classical 5-colouring of the 5-point stencil
    if check_coloring(Jt, col5):
        nc = 5
    n = nx * ny
    hx = 1.0 / (nx + 1); hy = 1.0 / (ny + 1)
    p = dict(name=f'semilin2d-{nx}x{ny}', f=q.f, ft=q.ft, J=J, Alin=Alin, alin_const=False, dst=None,
             y0=q.y0, span=q.span, ascale=0.01, n=n, grid=(nx, ny), b=1, nnzJ=int(Jt.nnz),
             frhs=28.0 * n, fft=38.0 * n, fjvp_state=23.0 * n, colors=nc, exact=q.exact,
             stiff=float(4 * D * (1 / hx ** 2 + 1 / hy ** 2) / 2 + 4.0 * (1 / hx + 1 / hy)))
    return p


def bruss1d(cells):
    """harness brusselator-1d-N with sparse J (fidelity check against SPD07 BASE.json)."""
    cc = (cells + 1.0) ** 2 / 50.0; n = 2 * cells
    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.r_[1.0, u[:-1]]; ur = np.r_[u[1:], 1.0]; vl = np.r_[3.0, v[:-1]]; vr = np.r_[v[1:], 3.0]
        out = np.empty(n)
        out[0::2] = 1 + u * u * v - 4 * u + cc * (ul - 2 * u + ur)
        out[1::2] = 3 * u - u * u * v + cc * (vl - 2 * v + vr)
        return out
    Lint = sp.kron(lap1(cells), sp.identity(2)).tocsr()
    A = (cc * Lint).tocsr()
    ev = np.arange(0, n, 2); od = ev + 1
    def J(t, y):
        u = y[0::2]; v = y[1::2]
        rows = np.concatenate([ev, ev, od, od]); cols = np.concatenate([ev, od, ev, od])
        vals = np.concatenate([2 * u * v - 4.0, u * u, 3.0 - 2 * u * v, -u * u])
        return (A + sp.csr_matrix((vals, (rows, cols)), shape=(n, n))).tocsr()
    x = (np.arange(cells) + 1.0) / (cells + 1.0)
    y0 = np.empty(n); y0[0::2] = 1 + np.sin(2 * np.pi * x); y0[1::2] = 3.0
    Jt = J(0, y0)
    return dict(name=f'bruss1d-{cells}', f=f, ft=None, J=J, Alin=lambda t: A, alin_const=True, dst=None, y0=y0,
                span=(0.0, 10.0), ascale=1.0, n=n, grid=(cells, 1), b=2, nnzJ=int(Jt.nnz), frhs=11.0 * n,
                fft=0.0, fjvp_state=12.0 * n, colors=5, exact=None, stiff=4 * cc)


def build(name):
    """'bruss2d-N', 'bruss2d-N-a0.1', 'semilin2d-N', 'semilin2d-32x48', 'bruss1d-50'"""
    parts = name.split('-')
    if parts[0] == 'bruss2d':
        a = float(parts[2][1:]) if len(parts) > 2 else 0.02
        return bruss2d(int(parts[1]), alpha=a)
    if parts[0] == 'semilin2d':
        if 'x' in parts[1]:
            a, b = parts[1].split('x'); p = semilin2d(int(a), int(b))
        else:
            p = semilin2d(int(parts[1]))
        if len(parts) > 2 and parts[2].startswith('s'):      # badly scaled variant: atol = 10^-k rtol
            p['ascale'] = 10.0 ** (-int(parts[2][1:])); p['name'] = name
        return p
    if parts[0] == 'bruss1d':
        return bruss1d(int(parts[1]))
    raise ValueError(name)


if __name__ == '__main__':
    import sys, time
    for nm in sys.argv[1:]:
        t0 = time.time(); p = build(nm)
        J = p['J'](0.0, p['y0'])
        print(f"{nm}: n={p['n']} nnzJ={p['nnzJ']} ({p['nnzJ']/p['n']:.2f}/row) colors={p['colors']} stiff~{p['stiff']:.0f} "
              f"|f(y0)|={np.linalg.norm(p['f'](0.0, p['y0'])):.3e} build {time.time()-t0:.1f}s")
        # JVP check: J v vs FD
        v = np.random.default_rng(0).standard_normal(p['n'])
        e = 1e-7
        fd = (p['f'](0.0, p['y0'] + e * v) - p['f'](0.0, p['y0'] - e * v)) / (2 * e)
        print('   J@v vs central FD rel err', np.linalg.norm(J @ v - fd) / np.linalg.norm(fd))
        if p['dst'] is not None:
            s = 0.05 * 0.2
            r = np.random.default_rng(1).standard_normal(p['n'])
            x = p['dst'].solve(r, s)
            A = p['Alin'](0.0)
            print('   DST solve residual', np.linalg.norm(x - s * (A @ x) - r) / np.linalg.norm(r))
