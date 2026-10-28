"""KIOPS-type Krylov phi-action engine with admitted-cost counters and a Jawecki-Auzinger-Koch defect-integral bound.
PROBE B3 (EXPLORATORY pilot, not ledger authority).

Computes, for ascending output times 0 < tau_1 < ... < tau_K,

    w(tau_k) = sum_{l=0}^{p} tau_k^l phi_l(tau_k A) u_l

with the KIOPS algorithm of Gaudreault, Rainwater & Tokman (J. Comput. Phys. 372, 2018): Arnoldi on the augmented
operator  Ahat = [[A, U_flip], [0, S]]  (n+p), S the p x p upper shift, U_flip = nu [u_p, ..., u_1]; the small
exponential of the augmented projected Hessenberg  [[tau H_j, e1], [0, 0]]  (j+1) gives both the substep output
beta V_j exp(tau H_j) e1 and Saad's first-term error estimate  err = beta h_{j+1,j} |tau e_j^T phi_1(tau H_j) e1|;
error-per-unit-step test omega = tau_end err / (tau tol) <= delta = 1.4; adaptive (m, tau) by the KIOPS rules
(order / k estimates, mmin, mmax, gamma); substeps (time stepping of the augmented exponential); basis reuse on a
rejected substep; multiple outputs from one basis.

Deviations from the published kiops.m (stated):
  * full orthogonalisation, two-pass classical Gram-Schmidt (CGS2) instead of IOP(2) (the operators here are
    nonnormal); counted as 2(j+1) inner products + 2(j+1) vector updates per column.
  * happy breakdown only when h_{j+1,j} <= 100 eps ||A v_j|| (KIOPS uses h < tol, which can hide a large error,
    cf. VIG-A02); otherwise the ordinary estimate decides.
  * multi-output in the first substep uses the squaring chain exp(tau Hhat / 4)^2^k when tau/2 and tau/4 are
    requested (exact identity, saves expms); other outputs use one extra expm each (as kiops.m).

COST MODEL (flops; the caller adds F_jvp per operator application):
  augmented operator: 2 p n (U_flip times the augmented tail) ;  orthogonalisation: 2 (n+p) per inner product and
  per vector update, 2 (n+p) per norm, (n+p) per scaling ;  output assembly beta V_j y: 2 j n per output ;
  small dense exponentials: Higham (2005) scaling-and-squaring Pade model, (2 pi_m + 8/3 + 2 s) N^3 with
  (m, pi_m) in {(3,2),(5,3),(7,4),(9,5),(13,6)} chosen by ||M||_1 and s squarings (expm_cost below);
  matrix squarings 2 N^3.

JAK BOUND (cert=True). For a substep the Krylov approximation z(s) = beta V_j exp(s H_j) e1 of exp(s Ahat) vhat has
defect z' - Ahat z = -c(s) v_{j+1}, c(s) = beta h_{j+1,j} e_j^T exp(s H_j) e1.  Splitting the error e_hat = [e; a]
and v_{j+1} = [q; r]:  a' = S a + c r,  e' = A e + U_flip a + c q, e(0)=a(0)=0, so with mu(A) <= 0 in the norm used
    ||e(tau)|| <= int_0^tau ( |c(s)| ||q|| + ||U_flip a(s)|| ) ds          (Jawecki, Auzinger & Koch, BIT 60 (2020),
                                                                           extended to the KIOPS augmentation)
and substep errors add (exp(s A) contractive; the augmented tail is reset exactly at each substep).  [x; a] solves
the small linear ODE with G = [[H_j, 0], [beta h r e_j^T, S]]; it is evaluated on a uniform grid (Q points, exact
propagation by expm(G tau/Q)) and integrated by composite Simpson.  'Rigorous' up to that quadrature and rounding.
"""
import math

import numpy as np
from scipy.linalg import expm

EPS = np.finfo(float).eps
_PADE = [(3, 1.495585217958292e-2, 2), (5, 2.539398330063230e-1, 3), (7, 9.504178996162932e-1, 4),
         (9, 2.097847961257068, 5)]
_TH13 = 5.371920351148152


def expm_cost(M):
    """(flops, squarings) of Higham-2005 scaling and squaring for the N x N matrix M (1-norm selection)."""
    N = M.shape[0]
    a = float(np.abs(M).sum(axis=0).max()) if N else 0.0
    base = N * N  # 1-norm
    for deg, th, prods in _PADE:
        if a <= th:
            return (2 * prods + 8.0 / 3.0) * N ** 3 + base, 0
    s = max(0, int(math.ceil(math.log2(a / _TH13)))) if a > 0 else 0
    return (2 * 6 + 8.0 / 3.0 + 2 * s) * N ** 3 + base, s


class KCnt:
    """Counters for one engine (accumulated over calls)."""
    KEYS = ('ops', 'dots', 'axpys', 'norms', 'scal', 'aug_flops', 'out_flops', 'dense_flops', 'expms', 'substeps',
            'sub_rejects', 'calls', 'reused_cols', 'cert_flops', 'happy')

    def __init__(self):
        for k in self.KEYS:
            setattr(self, k, 0)

    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def _expm_counted(M, cnt):
    f, _ = expm_cost(M)
    cnt.dense_flops += f; cnt.expms += 1
    return expm(M)


def kiops(Aop, tau_out, U, tol, cnt, m_init=10, mmin=4, mmax=100, cert=False, Q=128, basis_cache=None,
          record=None):
    """Aop(v): operator application (the caller counts its F_jvp); tau_out ascending positive; U list of p+1 arrays
    (U[0] may be None = 0); tol absolute (2-norm of the space Aop acts in).  Returns (outs, info) with outs[k] =
    w(tau_out[k]); info: m (final m for warm start), err_sum (sum of accepted substep estimates), bounds (list per
    output of the JAK bound, if cert), substeps, j_max, rej.
    basis_cache: dict; if it holds a first-substep basis for the same operator and start vector (u's identical; only
    tau changes), the basis is reused (Krylov space independent of tau)."""
    cnt.calls += 1
    p = len(U) - 1
    n = None
    for u in U:
        if u is not None:
            n = len(u); break
    tau_out = np.asarray(tau_out, dtype=float)
    K = len(tau_out)
    u0 = U[0] if U[0] is not None else np.zeros(n)
    if p == 0:
        p = 1; U = [u0, np.zeros(n)]
    Um = np.array([U[l] if U[l] is not None else np.zeros(n) for l in range(1, p + 1)])   # p x n (u_1..u_p)
    # trivial input
    if not np.any(u0) and not np.any(Um):
        return [np.zeros(n) for _ in range(K)], dict(m=m_init, err_sum=0.0, bounds=[0.0] * K, substeps=0, j_max=0, rej=0)
    N = n + p
    normU = float(np.abs(Um).sum(axis=1).max())     # ||[u_1..u_p]||_1 (max column abs sum)
    cnt.norms += p
    if normU > 0:
        ex = math.ceil(math.log2(normU)); nu = 2.0 ** (-ex); mu = 2.0 ** ex
    else:
        nu = mu = 1.0
    Uflip = nu * Um[::-1]          # rows: u_p, ..., u_1   (column k of kiops' u_flip = row k here)
    cnt.scal += p
    if cert:
        WtW = Uflip @ Uflip.T      # p x p Gram matrix for ||Uflip^T a||
        cnt.cert_flops += 2 * p * p * n

    def aug_op(v):
        top = Aop(v[:n])
        top += v[n:] @ Uflip       # sum_k v_aug[k] * Uflip[k]
        cnt.ops += 1; cnt.aug_flops += 2 * p * n
        out = np.empty(N); out[:n] = top
        out[n:N - 1] = v[n + 1:]; out[N - 1] = 0.0
        return out

    tau_end = float(tau_out[-1])
    tau_now = 0.0
    tau = tau_end
    gamma, gamma_mmax = (0.2, 0.1) if tau_end > 1 else (0.9, 0.6)
    delta = 1.4
    mmax = max(1, min(mmax, N))
    mmin = min(mmin, mmax)
    m = max(mmin, min(int(m_init), mmax))
    oldm = -1; oldtau = float('nan'); omega = float('nan'); oldomega = float('nan')
    orderold = True; kestold = True
    l = 0
    w = u0.astype(float).copy()
    outs = [None] * K
    bounds = [0.0] * K
    acc_bound = 0.0
    V = np.zeros((mmax + 1, N)); H = np.zeros((mmax + 1, mmax + 1))
    j = 0; ireject = 0; err_sum = 0.0; substeps = 0; j_max = 0; nrej = 0
    beta = None
    first_sub = True
    reuse = None
    if basis_cache is not None and basis_cache.get('V') is not None:
        reuse = basis_cache
    while tau_now < tau_end * (1 - 1e-15):
        if j == 0:
            H[:, :] = 0.0
            aug = np.zeros(p)
            for k in range(1, p):
                i = p - k
                aug[k - 1] = (tau_now ** i) / math.factorial(i) * mu
            aug[p - 1] = mu
            beta = math.sqrt(float(w @ w) + float(aug @ aug)); cnt.norms += 1
            V[0, :n] = w / beta; V[0, n:] = aug / beta; cnt.scal += 1
            if first_sub and reuse is not None and abs(reuse['beta'] - beta) <= 1e-14 * beta and \
                    np.array_equal(reuse['v0'], V[0]):
                jr = reuse['j']
                V[:jr + 1] = reuse['V'][:jr + 1]; H[:jr + 1, :jr] = reuse['H'][:jr + 1, :jr]
                j = jr; cnt.reused_cols += jr
            reuse = None
        happy = False
        # Arnoldi (CGS2, full)
        while j < m:
            j += 1
            wv = aug_op(V[j - 1])
            sc0 = math.sqrt(float(wv @ wv)); cnt.norms += 1
            hc = V[:j] @ wv; wv -= hc @ V[:j]
            hc2 = V[:j] @ wv; wv -= hc2 @ V[:j]
            hc += hc2
            cnt.dots += 2 * j; cnt.axpys += 2 * j
            H[:j, j - 1] = hc
            nrm = math.sqrt(float(wv @ wv)); cnt.norms += 1
            if nrm == 0.0 or nrm <= 100 * EPS * sc0:
                happy = True
                H[j, j - 1] = 0.0
                break
            H[j, j - 1] = nrm
            V[j] = wv / nrm; cnt.scal += 1
        j_max = max(j_max, j)
        if first_sub and basis_cache is not None and tau_now == 0.0:
            basis_cache.update(V=V[:j + 1].copy(), H=H[:j + 1, :j + 1].copy(), j=j, beta=beta, v0=V[0].copy())
        # augmented small matrix [[tau H_j, e1], [0, 0]]
        nrm = H[j, j - 1]
        Hh = np.zeros((j + 1, j + 1)); Hh[:j, :j] = H[:j, :j]; Hh[0, j] = 1.0
        # squaring-chain multi-output in the first full substep
        pending = [k for k in range(l, K) if tau_out[k] < tau_now + tau * (1 - 1e-15)]
        F = None; chain = {}
        if tau_now == 0.0 and abs(tau - tau_end) <= 1e-15 * tau_end:
            want = {round(float(tau_out[k] / tau), 12) for k in pending}
            if 0.25 in want and 0.5 in want:
                F4 = _expm_counted(tau / 4 * Hh, cnt)
                F2 = F4 @ F4; F = F2 @ F2; cnt.dense_flops += 4 * (j + 1) ** 3
                chain = {0.25: F4, 0.5: F2}
            elif 0.5 in want:
                F2 = _expm_counted(tau / 2 * Hh, cnt)
                F = F2 @ F2; cnt.dense_flops += 2 * (j + 1) ** 3
                chain = {0.5: F2}
        if F is None:
            F = _expm_counted(tau * Hh, cnt)
        if happy:
            omega = 0.0; err = 0.0
            m_new = m; tau_new = min(tau_end - (tau_now + tau), tau)
            if tau_new <= 0: tau_new = tau
        else:
            err = abs(beta * nrm * F[j - 1, j])
            if not math.isfinite(err) or not math.isfinite(beta):
                raise FloatingPointError('kiops: non-finite Krylov quantities (attempt fails, caller rejects)')
            oldomega = omega
            omega = tau_end * err / (tau * tol)
            if m == oldm and tau != oldtau and ireject >= 1:
                order = max(1.0, math.log(omega / oldomega) / math.log(tau / oldtau)) if omega > 0 and oldomega > 0 else j / 4
                orderold = False
            elif orderold or ireject == 0:
                orderold = True; order = j / 4
            else:
                orderold = True
            if m != oldm and tau == oldtau and ireject >= 1:
                kest = max(1.1, (omega / oldomega) ** (1.0 / (oldm - m))) if omega > 0 and oldomega > 0 else 2.0
                kestold = False
            elif kestold or ireject == 0:
                kestold = True; kest = 2.0
            else:
                kestold = True
            remaining = (tau_end - tau_now) if omega > delta else (tau_end - (tau_now + tau))
            same_tau = min(remaining, tau)
            tau_opt = tau * (gamma / omega) ** (1.0 / order) if omega > 0 else 5 * tau
            tau_opt = min(remaining, max(tau / 5, min(5 * tau, tau_opt)))
            m_opt = math.ceil(j + math.log(omega / gamma) / math.log(kest)) if omega > 0 else mmin
            m_opt = max(mmin, min(mmax, max(math.floor(3 / 4 * m), min(m_opt, math.ceil(4 / 3 * m)))))
            if j == mmax:
                if omega > delta:
                    m_new = j; tau_new = tau * (gamma_mmax / omega) ** (1.0 / order)
                    tau_new = min(tau_end - tau_now, max(tau / 5, tau_new))
                else:
                    tau_new = tau_opt; m_new = m
            else:
                m_new = m_opt; tau_new = same_tau
        if omega <= delta:
            substeps += 1; nrej += ireject
            # JAK defect integral for this substep (cert)
            if cert:
                Bfun = _jak_setup(H, j, beta, nrm, V[j] if not happy else np.zeros(N), n, p, WtW, tau, Q, cnt)
            nextT = tau_now + tau
            for k in range(l, K):
                if tau_out[k] < nextT * (1 - 1e-15):
                    tp = tau_out[k] - tau_now
                    key = round(float(tp / tau), 12) if tau_now == 0.0 else None
                    if key in chain:
                        y = chain[key][:j, 0]
                    else:
                        F2 = _expm_counted(tp * H[:j, :j], cnt); y = F2[:, 0]
                    outs[k] = beta * (y @ V[:j, :n]); cnt.out_flops += 2 * j * n
                    if cert:
                        bounds[k] = acc_bound + Bfun(tp)
                    l = k + 1
            w = beta * (F[:j, 0] @ V[:j, :n]); cnt.out_flops += 2 * j * n
            if cert:
                acc_bound += Bfun(tau)
            tau_now = tau_now + tau
            j = 0; ireject = 0
            err_sum += err
            first_sub = False
        else:
            ireject += 1; cnt.sub_rejects += 1
        oldtau = tau; tau = tau_new
        oldm = m; m = m_new
        if record is not None:
            record.append((j_max, tau, omega))
    outs[K - 1] = w
    if cert:
        bounds[K - 1] = acc_bound
    cnt.substeps += substeps
    return outs, dict(m=m, err_sum=err_sum, bounds=bounds, substeps=substeps, j_max=j_max, rej=nrej, beta=beta)


def _jak_setup(H, j, beta, hn, vnext, n, p, WtW, tau, Q, cnt):
    """Returns B(t) = int_0^t (|c(s)| ||q|| + ||Uflip^T a(s)||) ds for t in (0, tau] (Simpson on a uniform grid)."""
    q = vnext[:n]; r = vnext[n:]
    qn = math.sqrt(float(q @ q))
    G = np.zeros((j + p, j + p))
    G[:j, :j] = H[:j, :j]
    G[j:, j - 1] = beta * hn * r
    if p > 1:
        G[j:j + p - 1, j + 1:j + p] = np.eye(p - 1)
    if Q % 2: Q += 1
    ds = tau / Q
    E = expm(ds * G)
    f, _ = expm_cost(ds * G); cnt.cert_flops += f
    z = np.zeros(j + p); z[0] = 1.0
    g = np.empty(Q + 1)
    for k in range(Q + 1):
        c = beta * hn * z[j - 1]
        a = z[j:]
        g[k] = abs(c) * qn + math.sqrt(max(0.0, float(a @ WtW @ a)))
        z = E @ z
    cnt.cert_flops += Q * 2 * (j + p) ** 2
    # cumulative Simpson at even nodes, linear interpolation in between
    cum = np.zeros(Q + 1)
    for k in range(2, Q + 1, 2):
        cum[k] = cum[k - 2] + ds / 3 * (g[k - 2] + 4 * g[k - 1] + g[k])
    for k in range(1, Q, 2):
        cum[k] = cum[k - 1] + ds / 2 * (g[k - 1] + g[k])

    def B(t):
        x = t / ds
        k = min(Q, int(math.floor(x)))
        if k >= Q:
            return float(cum[Q])
        fr = x - k
        return float(cum[k] + fr * (cum[k + 1] - cum[k]))
    return B


def dense_phi(Adense, tau_out, U):
    """Exact (dense) w(tau) = sum_l tau^l phi_l(tau A) u_l via the (n+p) augmented exponential (oracle; not counted)."""
    p = len(U) - 1
    n = Adense.shape[0]
    u0 = U[0] if U[0] is not None else np.zeros(n)
    if p == 0:
        return [expm(t * Adense) @ u0 for t in tau_out]
    Um = np.array([U[l] if U[l] is not None else np.zeros(n) for l in range(1, p + 1)])
    N = n + p
    M = np.zeros((N, N)); M[:n, :n] = Adense
    for k in range(p):
        M[:n, n + k] = Um[p - 1 - k]
    if p > 1:
        M[n:N - 1, n + 1:N] = np.eye(p - 1)
    v = np.zeros(N); v[:n] = u0; v[N - 1] = 1.0
    return [(expm(t * M) @ v)[:n] for t in tau_out]
