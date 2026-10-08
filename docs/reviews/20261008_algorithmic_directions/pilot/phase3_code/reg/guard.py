"""Nonnormality guard for the ROCK4 hand-off (PROBE B2). EXPLORATORY, not ledger authority.

GUARD = FIELD-OF-VALUES STEP BOUND on the Krylov compression of the WRMS-scaled Jacobian.

Why. Explicit stability on a nonnormal operator is governed by pseudospectra, not eigenvalues (Reddy-Trefethen).
A sufficient condition that does not need normality is Crouzeix-Palencia (2017): for any polynomial R,
    ||R(hA)||_2 <= (1 + sqrt 2) max_{z in W(hA)} |R(z)|,      W = numerical range (field of values),
and applying it to R^k gives  ||R(hA)^k|| <= (1 + sqrt 2) (max_W |R|)^k  for every k.  So if the ROCK4 stability
polynomial R_s of the degree actually used satisfies  max_{z in W(hA)} |R_s(z)| <= exp(h omega_+) (1 + delta)
(omega_+ = max(0, max Re W(A)), the growth the exact flow itself may show in this norm), the explicit propagator is
power-bounded up to the constant 2.41 and the physical growth, uniformly in k: no estimator-held stability.

What is computed (all matrix-free, JVPs only).
 * A = D J D^-1 (D = WRMS weights 1/(atol + rtol |y|)): the norm in which the error test measures.
 * The stage-1 GMRES basis of RODAS5P is K_m(D W D^-1, D b1) = K_m(A, D b1) (shift invariance, Zero start, first
   cycle), so its Hessenberg compression H_m = V_m^T A V_m is free and gives the Ritz radius used by the shadow.
   The GUARD itself runs a dedicated m_g = 16-column Arnoldi from the stage-1 direction plus a seeded random vector
   (start_vector; all columns, JVPs and orthogonalisation charged) because the pure stage-1 start was not
   conservative on 2 of 8 calibration operators; it uses W(H_m), a subset of W(A) (inner approximation).  Its boundary is traced with the support function: for K = 16 directions theta,
   x = top eigenvector of Herm(e^{-i theta} H_m), boundary point x^* H_m x.
 * Inflation against the inner approximation: imaginary parts x KAPPA_IM (1.5), real extent to the left
   extended to -rho_hat (the x1.2 power-iteration estimate that ROCK4 uses for its degree).
 * Test of a step h:  s = ROCK4 degree for (h, rho_hat);  max_k |R_s(h z_k)| <= exp(h omega_+) (1 + ln(10) h/T)
   on the inflated boundary points z_k  (the delta = ln(10) h / T term lets the extra amplification grow at
   most 10x over the whole span T, Crouzeix constant aside).  Max-modulus: the test on the boundary covers W.
 * h_FOV = largest h on a geometric grid (8 points per octave) such that every grid h' <= h passes.
Decision use: (1) the shadow cost of ROCK4 is evaluated at min(h_E, h_FOV) (a step bound, not just a veto);
(2) hand-off requires h_FOV >= predicted h_E / 2 ... see swdrv.SwitchCfg; (3) on the ROCK4 branch every rho refresh
re-runs the guard and caps h <= h_FOV; a guard violation that would cap below 0.5 h_E reverts.
Diagnostic (reported, not used for the decision): Ritz angle max |Im theta| / |Re theta| over Ritz values.
Limits: W(H_m) is an inner approximation of W(A) (heuristic, not a certificate); frozen J; Crouzeix constant.
"""
import math

import numpy as np

import rock4x

KAPPA_IM = 1.5
M_GUARD = 16
K_DIR = 16


def arnoldi(Aop, v0, m, cnt=None, start_cols=0):
    """MGS2 Arnoldi on Aop from v0. Charges JVPs/dots/axpys only for columns >= start_cols (those already paid
    by the stage-1 GMRES cycle). Returns (H square m_used x m_used, m_used)."""
    n = len(v0)
    V = np.zeros((m + 1, n)); H = np.zeros((m + 1, m))
    beta = np.linalg.norm(v0)
    if beta == 0 or not math.isfinite(beta):
        return np.zeros((0, 0)), 0
    V[0] = v0 / beta
    used = 0
    for j in range(m):
        w = Aop(V[j])
        for _ in range(2):
            for i in range(j + 1):
                hh = V[i] @ w; H[i, j] += hh; w -= hh * V[i]
        hn = np.linalg.norm(w); H[j + 1, j] = hn
        used = j + 1
        if cnt is not None and j >= start_cols:
            cnt.g_jvp += 1; cnt.dots += 2 * (j + 1); cnt.axpys += 2 * (j + 1); cnt.norms += 1; cnt.scal += 1
        if hn <= 1e-12 * max(1.0, np.abs(H[:j + 2, j]).max()):
            break
        V[j + 1] = w / hn
    return H[:used, :used].copy(), used


def start_vector(v, seed):
    """Guard Arnoldi start: the stage-1 direction D b1 (or D f0) normalised plus a seeded random vector / sqrt(n).
    Calibration (logs/fovstart.log): the pure stage-1 (smooth) start under-resolves the skew part on 2 of 8 frozen
    operators (advdiff-mild: compressed |Im W| 22 vs exact 86, h_FOV 160x too large; semi-384: 67 vs 119), the mixed
    start does not (51 / 97).  All m_g columns are therefore charged (no reuse of the stage-1 columns)."""
    n = len(v); nv = np.linalg.norm(v)
    r = np.random.default_rng(1000003 + int(seed)).standard_normal(n) / math.sqrt(n)
    return (v / nv if nv > 0 else 0.0) + r


def fov_boundary(H, K=K_DIR):
    """Boundary points of the numerical range of the square matrix H (support-function tracing)."""
    m = H.shape[0]
    if m == 0:
        return np.zeros(0, dtype=complex)
    Hc = H.astype(complex)
    pts = np.empty(K, dtype=complex)
    for k, th in enumerate(np.linspace(0.0, 2 * np.pi, K, endpoint=False)):
        M = (np.exp(-1j * th) * Hc + np.exp(1j * th) * Hc.conj().T) / 2
        w, X = np.linalg.eigh(M)
        x = X[:, -1]
        pts[k] = x.conj() @ Hc @ x
    return pts


def ritz(H, stiff_frac=0.05):
    """Ritz values of H: rho = max |theta|; angle = max |Im|/|Re| over the STIFF Ritz values |theta| >= stiff_frac*rho
    (small-modulus complex Ritz values are resolved dynamics, e.g. the Brusselator reaction oscillation, and are
    excluded); angle_all over every Ritz value (the merge note's original definition, reported only)."""
    th = np.linalg.eigvals(H) if H.shape[0] else np.zeros(0)
    if len(th) == 0:
        return dict(rho=0.0, angle=0.0, angle_all=0.0, re_min=0.0, re_max=0.0)
    rho = float(np.abs(th).max())
    a = np.abs(th.imag) / np.maximum(np.abs(th.real), 1e-300 + 1e-12 * rho)
    st = np.abs(th) >= stiff_frac * rho
    return dict(rho=rho, angle=float(np.max(a[st])), angle_all=float(np.max(a)), re_min=float(th.real.min()),
                re_max=float(th.real.max()))


def inflate(pts, rho_hat, kappa_im=KAPPA_IM, nsamp=240):
    """Inflate the inner FOV estimate (Im x kappa_im, real extent to -rho_hat), take the convex hull and sample its
    boundary densely (W is convex; max-modulus needs the whole boundary, including the added segments)."""
    from scipy.spatial import ConvexHull
    q = pts.real + 1j * kappa_im * pts.imag if len(pts) else np.zeros(0, dtype=complex)
    q = np.concatenate([q, np.array([-rho_hat + 0j, 0j if len(pts) == 0 else q[np.argmax(q.real)]])])
    P = np.c_[q.real, q.imag]
    span_im = np.ptp(P[:, 1])
    if span_im <= 1e-14 * max(1.0, np.ptp(P[:, 0])):
        x = np.linspace(P[:, 0].min(), P[:, 0].max(), nsamp)
        return x + 0j
    try:
        hull = ConvexHull(P)
    except Exception:
        return q
    V = P[hull.vertices]
    V = np.vstack([V, V[:1]])
    seg = np.hypot(np.diff(V[:, 0]), np.diff(V[:, 1])); L = seg.sum()
    out = []
    for a, b, l in zip(V[:-1], V[1:], seg):
        k = max(2, int(math.ceil(nsamp * l / L)))
        tt = np.linspace(0.0, 1.0, k, endpoint=False)
        out.append((a[0] + tt * (b[0] - a[0])) + 1j * (a[1] + tt * (b[1] - a[1])))
    return np.concatenate(out)


def rmax(zh, h, rho_hat):
    """max |R_s(z)| over scaled points zh for the ROCK4 degree chosen at (h, rho_hat)."""
    di, ms, st = rock4x.degree(h, rho_hat)
    R, _ = rock4x.stab_poly(zh, di)
    return float(np.max(np.abs(R))), ms + 4


def step_ok(pts, h, rho_hat, T):
    om = max(0.0, float(pts.real.max())) if len(pts) else 0.0
    r, s = rmax(h * pts, h, rho_hat)
    bound = math.exp(min(h * om, 50.0)) * (1.0 + math.log(10.0) * h / T)
    return r <= bound, r, bound, s


def fov_step_bound(pts, rho_hat, T, h_lo, h_hi, per_oct=8):
    """Largest grid h in [h_lo, h_hi] such that step_ok holds for all grid h' <= h (0 if h_lo fails)."""
    if h_hi <= h_lo:
        return h_hi if step_ok(pts, h_hi, rho_hat, T)[0] else 0.0
    nsteps = int(math.ceil(per_oct * math.log2(h_hi / h_lo))) + 1
    best = 0.0
    for j in range(nsteps + 1):
        h = min(h_lo * 2 ** (j / per_oct), h_hi)
        if not step_ok(pts, h, rho_hat, T)[0]:
            break
        best = h
        if h >= h_hi:
            break
    return best


def eig_flops(m, K=K_DIR):
    """flop charge of the guard's small dense work: K Hermitian eigensolves (~9 m^3) + Ritz (~25 m^3)."""
    return float(K * 9 * m ** 3 + 25 * m ** 3)
