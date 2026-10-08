"""Shared pieces for the Krylov-reuse probes (read-only w.r.t. repo)."""
import json, math
import numpy as np
from scipy.linalg import lu_factor, lu_solve

ROOT = "/home/user/wt-speed"
snap = json.load(open(f"{ROOT}/fixtures/rodas5p_coefficients_snapshot.json"))
g = float(snap["gamma"])
A = np.array([[float(x) for x in r] for r in snap["A"]])
C = np.array([[float(x) for x in r] for r in snap["C"]])
c = np.array([float(x) for x in snap["c"]])
bc = np.array([float(x) for x in snap["b_code"]])
S = len(c)
Gam = np.linalg.inv(np.eye(S) / g - C)
grow = Gam.sum(axis=1)


def bruss(cells):
    cc = (cells + 1.0) ** 2 / 50.0; n = 2 * cells
    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.r_[1.0, u[:-1]]; ur = np.r_[u[1:], 1.0]; vl = np.r_[3.0, v[:-1]]; vr = np.r_[v[1:], 3.0]
        out = np.empty(n)
        out[0::2] = 1 + u * u * v - 4 * u + cc * (ul - 2 * u + ur)
        out[1::2] = 3 * u - u * u * v + cc * (vl - 2 * v + vr)
        return out
    def J(t, y):
        j = np.zeros((n, n))
        for i in range(cells):
            u, v = y[2 * i], y[2 * i + 1]; a, b = 2 * i, 2 * i + 1
            j[a, a] = 2 * u * v - 4 - 2 * cc; j[a, b] = u * u; j[b, a] = 3 - 2 * u * v; j[b, b] = -u * u - 2 * cc
            if i > 0: j[a, a - 2] = cc; j[b, b - 2] = cc
            if i + 1 < cells: j[a, a + 2] = cc; j[b, b + 2] = cc
        return j
    x = (np.arange(cells) + 1.0) / (cells + 1.0)
    y0 = np.empty(n); y0[0::2] = 1 + np.sin(2 * np.pi * x); y0[1::2] = 3.0
    return dict(f=f, J=J, y0=y0, span=(0.0, 10.0), ascale=1.0, auto=True, n=n, name=f"bruss-{cells}")


def heat(n, nu=1.0):
    dx = 1.0 / (n + 1); L = (np.diag(-2 * np.ones(n)) + np.diag(np.ones(n - 1), 1) + np.diag(np.ones(n - 1), -1)) * nu / dx**2
    x = np.arange(1, n + 1) * dx
    y0 = np.sin(np.pi * x) + 0.3 * np.sin(5 * np.pi * x) + 0.1 * x * (1 - x)
    return dict(f=lambda t, y: L @ y, J=lambda t, y: L, y0=y0, span=(0.0, 0.1), ascale=1.0, auto=True, n=n, name=f"heat-{n}")


def trajectory(prob, rtol, h0=1e-6, maxatt=100000):
    """Dense-LU replica of rodas5p-fast (I controller). Returns list of attempts."""
    f, Jf, y0 = prob["f"], prob["J"], prob["y0"]
    t0, tf = prob["span"]; atol = rtol * prob["ascale"]
    t, y, h = t0, y0.copy(), h0
    out = []; last_rej = None
    while t < tf and len(out) < maxatt:
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        J = Jf(t, y); f0 = f(t, y); n = len(y)
        W = np.eye(n) - h * g * J
        lu = lu_factor(W)
        U = np.zeros((S, n)); Bs = np.zeros((S, n))
        for i in range(S):
            fi = f0 if i == 0 else f(t + c[i] * h, y + A[i, :i] @ U[:i])
            b = h * g * fi + g * (C[i, :i] @ U[:i])
            Bs[i] = b; U[i] = lu_solve(lu, b)
        ynew = y + bc @ U
        sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1] / sc) ** 2))
        acc = err <= 1.0
        out.append(dict(t=t, y=y.copy(), h=h, J=J, W=W, B=Bs, U=U, err=err, acc=acc))
        if acc:
            t += h; y = ynew; last_rej = None
            fac = 5.0 if err == 0 else min(max(0.9 * err ** (-0.2), 0.2), 5.0)
        else:
            last_rej = h; fac = min(max(0.9 * max(err, 1e-16) ** (-0.2), 0.2), 0.9)
        h *= fac
    return out


def gmres_restarted(Aop, b, tol, restart=40, maxit=200, x0=None, nmax=None):
    """Production-like: full cycles of `restart` columns (or n), true residual between cycles.
    Returns (jvps, converged). JVP accounting: columns + one true residual per cycle end
    (+1 loop-top residual if x0 given)."""
    n = len(b); m = min(restart, n)
    x = np.zeros(n) if x0 is None else x0.copy()
    jv = 0
    r = b.copy() if x0 is None else b - Aop(x)
    if x0 is not None: jv += 1
    thr = tol * np.linalg.norm(b)
    cols = 0
    while np.linalg.norm(r) > thr and cols < maxit:
        beta = np.linalg.norm(r)
        V = np.zeros((n, m + 1)); H = np.zeros((m + 1, m)); V[:, 0] = r / beta
        k = 0
        for j in range(m):
            w = Aop(V[:, j]); jv += 1; cols += 1
            for _ in range(2):
                hh = V[:, :j + 1].T @ w; H[:j + 1, j] += hh; w -= V[:, :j + 1] @ hh
            H[j + 1, j] = np.linalg.norm(w); k = j + 1
            if H[j + 1, j] <= 100 * 2.2e-16 * np.linalg.norm(H[:j + 2, j]):
                break
            V[:, j + 1] = w / H[j + 1, j]
        e1 = np.zeros(k + 1); e1[0] = beta
        yk = np.linalg.lstsq(H[:k + 1, :k], e1, rcond=None)[0]
        x = x + V[:, :k] @ yk
        r = b - Aop(x); jv += 1
    return jv, np.linalg.norm(r) <= thr


def gmres_full_cols(Aop, b, tol, nmax):
    """Columns of full GMRES to reach projected residual <= tol*|b| (x0=0)."""
    n = len(b); beta = np.linalg.norm(b)
    V = np.zeros((n, nmax + 1)); H = np.zeros((nmax + 1, nmax)); V[:, 0] = b / beta
    from scipy.linalg import qr
    for j in range(nmax):
        w = Aop(V[:, j])
        for _ in range(2):
            hh = V[:, :j + 1].T @ w; H[:j + 1, j] += hh; w -= V[:, :j + 1] @ hh
        H[j + 1, j] = np.linalg.norm(w)
        e1 = np.zeros(j + 2); e1[0] = beta
        yk, res, *_ = np.linalg.lstsq(H[:j + 2, :j + 1], e1, rcond=None)
        rr = np.linalg.norm(e1 - H[:j + 2, :j + 1] @ yk)
        if rr <= tol * beta or H[j + 1, j] <= 1e-14 * np.linalg.norm(H[:j + 2, j]):
            return j + 1
        V[:, j + 1] = w / H[j + 1, j]
    return None


class SharedSpace:
    """GCR/GCRO-style minimal residual with ALL previous (z, Wz) pairs of the same operator kept
    (exact images: each image is one charged JVP of the same W). C orthonormal, W Z = C."""
    def __init__(self, n):
        self.Z = np.zeros((n, 0)); self.C = np.zeros((n, 0))
    def load(self, Z, Cimg):
        # (Z, W Z) raw pairs -> orthonormalize images, transform Z alike
        q, r = np.linalg.qr(Cimg)
        keep = np.abs(np.diag(r)) > 1e-13 * max(1e-300, np.abs(np.diag(r)).max()) if r.size else []
        q = q[:, keep]; r = r[np.ix_(keep, keep)] if r.size else r
        # Z_new = Z[:,keep] R^{-1} (approx; dropping dependent columns)
        Zk = Z[:, keep]
        self.C = q; self.Z = np.linalg.solve(r.T, Zk.T).T if len(keep) and keep.any() else Zk
    def solve(self, Wop, b, tol, M=None, maxdir=2000):
        """Returns x, jvps (new directions), plus 1 true residual JVP charged by caller."""
        thr = tol * np.linalg.norm(b)
        x = self.Z @ (self.C.T @ b)
        r = b - self.C @ (self.C.T @ b)
        r = r - self.C @ (self.C.T @ r)
        jv = 0
        while np.linalg.norm(r) > thr and jv < maxdir:
            z = r / np.linalg.norm(r) if M is None else M(r)
            w = Wop(z); jv += 1
            for _ in range(2):
                hh = self.C.T @ w; w = w - self.C @ hh; z = z - self.Z @ hh
            nw = np.linalg.norm(w)
            if nw <= 1e-14 * np.linalg.norm(z):
                break
            w /= nw; z /= nw
            self.C = np.column_stack([self.C, w]); self.Z = np.column_stack([self.Z, z])
            a = w @ r; x = x + a * z; r = r - a * w
        return x, jv
