"""Independent numpy implementation of the four families of loop node V25
(mf_p1_confirm/PREDECLARED.json, families.list).

Each builder returns (rhs, jvp, y0, pattern): rhs(t, y) and jvp(t, y, v) on
flat float64 arrays, the start state, and the sparsity pattern of the
Jacobian (scipy.sparse.lil_matrix) for the implicit cross-check. The arrays
follow the layouts of the frozen file; the arithmetic is written with numpy
array operations from the equations, in the same order of floating-point
operations as the equations are written there, so that the frozen
consistency check (20 states, 1e-12 relative to 1 + |value|) compares two
implementations of one formula. Nothing here reads a Rust result.
"""
import numpy as np
from scipy.sparse import lil_matrix


def _centres(m):
    return (np.arange(m) + 0.5) / m


def _laplace_2d(w, scale):
    """m^2 (xm + xp + ym + yp - 4 w) on an [j, i] array; a neighbour outside
    the square is the cell itself (zero flux)."""
    p = np.pad(w, 1, mode="edge")
    xm, xp = p[1:-1, :-2], p[1:-1, 2:]
    ym, yp = p[:-2, 1:-1], p[2:, 1:-1]
    return scale * (xm + xp + ym + yp - 4.0 * w)


def _pattern_2d(m, species, interleaved):
    """Jacobian pattern of a 5-point stencil with `species` coupled species per cell."""
    cells = m * m
    n = species * cells
    pattern = lil_matrix((n, n))

    def index(c, s):
        return species * c + s if interleaved else s * cells + c

    for j in range(m):
        for i in range(m):
            c = i + m * j
            around = [c]
            if i > 0:
                around.append(c - 1)
            if i + 1 < m:
                around.append(c + 1)
            if j > 0:
                around.append(c - m)
            if j + 1 < m:
                around.append(c + m)
            for s in range(species):
                for q in around:
                    pattern[index(c, s), index(q, s)] = 1
                for r in range(species):
                    pattern[index(c, s), index(c, r)] = 1
    return pattern


def ac3d(m=16):
    """u_t = D lap(u) + lam (u - u^3), D = 0.1, lam = 10, unit cube, m^3 cells,
    7-point Laplacian m^2 (sum of the 6 neighbours - 6 u), zero flux; index
    i + m (j + m k)."""
    d, lam = 0.1, 10.0
    scale = float(m * m)
    c = _centres(m)
    zz, yy, xx = np.meshgrid(c, c, c, indexing="ij")  # [k, j, i]
    u0 = 0.4 * np.cos(2.0 * np.pi * xx) * np.cos(np.pi * yy) + 0.3 * np.cos(3.0 * np.pi * xx) * np.cos(np.pi * zz) + 0.05
    y0 = u0.ravel().copy()

    def laplace(w):
        p = np.pad(w, 1, mode="edge")
        xm, xp = p[1:-1, 1:-1, :-2], p[1:-1, 1:-1, 2:]
        ym, yp = p[1:-1, :-2, 1:-1], p[1:-1, 2:, 1:-1]
        zm, zp = p[:-2, 1:-1, 1:-1], p[2:, 1:-1, 1:-1]
        return scale * (xm + xp + ym + yp + zm + zp - 6.0 * w)

    def rhs(_t, y):
        u = y.reshape(m, m, m)
        return (d * laplace(u) + lam * (u - u * u * u)).ravel()

    def jvp(_t, y, v):
        u = y.reshape(m, m, m)
        w = v.reshape(m, m, m)
        return (d * laplace(w) + lam * (1.0 - 3.0 * u * u) * w).ravel()

    n = m ** 3
    pattern = lil_matrix((n, n))
    for k in range(m):
        for j in range(m):
            for i in range(m):
                cc = i + m * (j + m * k)
                pattern[cc, cc] = 1
                if i > 0:
                    pattern[cc, cc - 1] = 1
                if i + 1 < m:
                    pattern[cc, cc + 1] = 1
                if j > 0:
                    pattern[cc, cc - m] = 1
                if j + 1 < m:
                    pattern[cc, cc + m] = 1
                if k > 0:
                    pattern[cc, cc - m * m] = 1
                if k + 1 < m:
                    pattern[cc, cc + m * m] = 1
    return rhs, jvp, y0, pattern


def robertson_rd1d(cells=400):
    """Robertson kinetics in every cell of [0, 1] with diffusion 1e-5 / dx^2 on
    every species, zero gradient at both ends, interleaved (y1, y2, y3),
    x_i = (i + 1/2) dx, dx = 1/cells; start y1 = 1 - s, y2 = 0, y3 = s with
    s = sin^2(pi x)/2 (the V06 generator)."""
    dx = 1.0 / cells
    dd = 1.0e-5 / (dx * dx)
    x = (np.arange(cells) + 0.5) * dx
    s = 0.5 * np.sin(np.pi * x) ** 2
    y0 = np.empty(3 * cells)
    y0[0::3] = 1.0 - s
    y0[1::3] = 0.0
    y0[2::3] = s

    def laplace(w):
        left = np.concatenate((w[:1], w[:-1]))
        right = np.concatenate((w[1:], w[-1:]))
        return dd * (left + right - 2.0 * w)

    def rhs(_t, y):
        y1, y2, y3 = y[0::3], y[1::3], y[2::3]
        out = np.empty_like(y)
        out[0::3] = -0.04 * y1 + 1.0e4 * y2 * y3 + laplace(y1)
        out[1::3] = 0.04 * y1 - 1.0e4 * y2 * y3 - 3.0e7 * y2 * y2 + laplace(y2)
        out[2::3] = 3.0e7 * y2 * y2 + laplace(y3)
        return out

    def jvp(_t, y, v):
        y2, y3 = y[1::3], y[2::3]
        v1, v2, v3 = v[0::3], v[1::3], v[2::3]
        exchange = 1.0e4 * (y3 * v2 + y2 * v3)
        out = np.empty_like(y)
        out[0::3] = -0.04 * v1 + exchange + laplace(v1)
        out[1::3] = 0.04 * v1 - exchange - 6.0e7 * y2 * v2 + laplace(v2)
        out[2::3] = 6.0e7 * y2 * v2 + laplace(v3)
        return out

    n = 3 * cells
    pattern = lil_matrix((n, n))
    for i in range(cells):
        for a in range(3):
            row = 3 * i + a
            for b in range(3):
                pattern[row, 3 * i + b] = 1
            if i > 0:
                pattern[row, row - 3] = 1
            if i + 1 < cells:
                pattern[row, row + 3] = 1
    return rhs, jvp, y0, pattern


def schnakenberg2d(m=32):
    """u_t = du lap(u) + gam (a - u + u^2 v), v_t = dv lap(v) + gam (b - u^2 v),
    du = 0.01, dv = 0.4, gam = 100, a = 0.1, b = 0.9; m^2 cells, 5-point
    Laplacian m^2, zero flux; interleaved, index 2 (i + m j) + s."""
    du_, dv_, gam, a, b = 0.01, 0.4, 100.0, 0.1, 0.9
    scale = float(m * m)
    c = _centres(m)
    yy, xx = np.meshgrid(c, c, indexing="ij")  # [j, i]
    u0 = a + b + 0.05 * np.cos(2.0 * np.pi * xx) * np.cos(2.0 * np.pi * yy)
    v0 = b / ((a + b) * (a + b)) + 0.05 * np.sin(2.0 * np.pi * xx) * np.sin(3.0 * np.pi * yy)
    y0 = np.empty(2 * m * m)
    y0[0::2] = u0.ravel()
    y0[1::2] = v0.ravel()

    def rhs(_t, y):
        u = y[0::2].reshape(m, m)
        v = y[1::2].reshape(m, m)
        uuv = u * u * v
        out = np.empty_like(y)
        out[0::2] = (du_ * _laplace_2d(u, scale) + gam * (a - u + uuv)).ravel()
        out[1::2] = (dv_ * _laplace_2d(v, scale) + gam * (b - uuv)).ravel()
        return out

    def jvp(_t, y, d):
        u = y[0::2].reshape(m, m)
        v = y[1::2].reshape(m, m)
        du = d[0::2].reshape(m, m)
        dv = d[1::2].reshape(m, m)
        uv2 = 2.0 * u * v
        uu = u * u
        out = np.empty_like(y)
        out[0::2] = (du_ * _laplace_2d(du, scale) + gam * ((uv2 - 1.0) * du + uu * dv)).ravel()
        out[1::2] = (dv_ * _laplace_2d(dv, scale) - gam * (uv2 * du + uu * dv)).ravel()
        return out

    return rhs, jvp, y0, _pattern_2d(m, 2, interleaved=True)


def porous2d(m=48):
    """u_t = lap(u^2) on the unit square, m^2 cells, 5-point Laplacian m^2 of
    w = u^2, zero flux; index i + m j; JVP lap(2 u v)."""
    scale = float(m * m)
    c = _centres(m)
    yy, xx = np.meshgrid(c, c, indexing="ij")  # [j, i]
    u0 = 0.1 + np.exp(-50.0 * ((xx - 0.5) * (xx - 0.5) + (yy - 0.5) * (yy - 0.5)))
    y0 = u0.ravel().copy()

    def rhs(_t, y):
        u = y.reshape(m, m)
        return _laplace_2d(u * u, scale).ravel()

    def jvp(_t, y, v):
        u = y.reshape(m, m)
        w = v.reshape(m, m)
        return _laplace_2d(2.0 * u * w, scale).ravel()

    return rhs, jvp, y0, _pattern_2d(m, 1, interleaved=True)


BUILDERS = {
    "ac3d": lambda: ac3d(16),
    "robertson_rd1d": lambda: robertson_rd1d(400),
    "schnakenberg2d": lambda: schnakenberg2d(32),
    "porous2d": lambda: porous2d(48),
    # reduced instances of the contract tests (checks of this file only)
    "dev_ac3d": lambda: ac3d(4),
    "dev_robertson_rd1d": lambda: robertson_rd1d(8),
    "dev_schnakenberg2d": lambda: schnakenberg2d(6),
    "dev_porous2d": lambda: porous2d(6),
}
DECLARED = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
