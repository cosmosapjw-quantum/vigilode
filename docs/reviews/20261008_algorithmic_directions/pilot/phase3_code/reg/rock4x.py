"""ROCK4 (Abdulle, SISC 23 (2002) 2041-2054) for PROBE B2 (REG-EXPLICIT-TRIAGE). EXPLORATORY, not ledger authority.

Coefficients (ms, fpa, fpb, fpbe, recf) are parsed from OrdinaryDiffEq.jl lib/OrdinaryDiffEqStabilizedRK
rkc_tableaus_rock4.jl (local copy rkc_tableaus_rock4.jl), which transcribes the data block of Abdulle's rock4.f.
rock4.f itself could not be fetched (www.unige.ch is not on the egress allowlist), so the coefficient integrity
is checked independently in selftest(): the stability polynomial of every one of the 50 degrees must satisfy
R_s(z) = exp(z) + O(z^5) and the embedded one  R^_s(z) = exp(z) + O(z^4), and a non-autonomous nonlinear
problem must converge with order 4.

Degree formula and step control as in rock4.f / OrdinaryDiffEq perform_step!(ROCK4ConstantCache):
  mdeg = floor(sqrt((3 + h*rho)/0.353)) + 1 ; if mdeg > 152: h = 0.8*(152^2*0.353 - 3)/rho
  mdeg = max(mdeg, 5) - 4 -> smallest tabulated ms >= mdeg; total f-evals per step = ms + 4 (FSAL: ms + 4 new,
  the first stage reuses f(t_n, y_n)).
Difference to the earlier hyp/regime/rock4.py port: the Chebyshev and finishing stages are evaluated at their
internal times t + c_i h (the OrdinaryDiffEq time recurrence t_i = h mu - nu t_{i-1} - kappa t_{i-2}); the old port
used t for every stage, which is wrong for non-autonomous problems (every corpus-v2 family is non-autonomous).
Controller: rock4.f (fac = (1/err)^(1/4), Gustafsson predictive factor after a successful step, facmax 5 at start
then 2, facmax 1 after a rejection, safety 0.8, fac >= 0.1). rho: nonlinear power iteration x 1.2 (OrdinaryDiffEq
maxeig!), refreshed every 25 accepted steps and after every rejection.

Work counting (Cnt fields, used by swdrv.flops): e_f  f-evaluations inside ROCK4 steps, e_pow power-iteration
f-evaluations, e_vec explicit vector flops (recurrences, finishing stages, error norm, power iteration updates).
"""
import math
import os
import re

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
_src = open(os.path.join(HERE, 'rkc_tableaus_rock4.jl')).read()
_num = re.compile(r'-?\d+\.\d*(?:e-?\d+)?')


def _block(name, nxt):
    s = _src.index(name + ' = ['); e = _src.index(nxt, s)
    return _src[s:e]


def _tuples(txt, k):
    vals = [float(x) for x in _num.findall(txt)]
    assert len(vals) % k == 0
    return np.array(vals).reshape(-1, k)


MS = [int(x) for x in re.findall(r'\d+', _src[_src.index('ms = ('):_src.index('fpa = [')].split('(', 1)[1])]
FPA = _tuples(_block('fpa', 'fpb = ['), 6)
FPB = _tuples(_block('fpb', 'fpbe = ['), 4)
FPBE = _tuples(_block('fpbe', 'recf = ['), 5)
RECF = np.array([float(x) for x in _num.findall(_src[_src.index('recf = ['):_src.index('_fpa = map')])])
assert len(MS) == 50 and FPA.shape == (50, 6) and FPB.shape == (50, 4) and FPBE.shape == (50, 5)
STARTS = np.cumsum([0] + [2 * m - 1 for m in MS[:-1]])
assert len(RECF) == STARTS[-1] + 2 * MS[-1] - 1
HMAX_HRHO = 152 ** 2 * 0.353 - 3.0          # largest h*rho admitted by the degree formula


def degree(h, rho):
    """(deg_index, ms, recf_start) for step h and spectral-radius estimate rho."""
    mdeg = int(math.floor(math.sqrt((3.0 + h * rho) / 0.353))) + 1
    mdeg = min(max(mdeg, 0), 152)
    mdeg = max(mdeg, 5) - 4
    for i, m in enumerate(MS):
        if m >= mdeg:
            return i, m, int(STARTS[i])
    raise ValueError(mdeg)


def stages(h, rho):
    return degree(h, rho)[1] + 4


def step_flops_vec(ms, n):
    """explicit vector flops of one ROCK4 step with ms Chebyshev stages: first stage 2n, each further recurrence
    stage 5n, finishing procedure 33n, error norm 7n."""
    return float(n * (2 + 5 * (ms - 1) + 33 + 7))


def step(f, t, y, f0, h, rho, deg=None):
    """One ROCK4 step. Returns (y_new, err_vec, n_f, total_stages, f(t+h, y_new))."""
    di, ms, st = degree(h, rho) if deg is None else deg
    nf = 0
    c1 = RECF[st]
    um2 = y
    um1 = y + (h * c1) * f0
    u = um1
    cm1, cm2 = c1, 0.0
    for i in range(2, ms + 1):
        mu, ka = RECF[st + (i - 2) * 2 + 1], RECF[st + (i - 2) * 2 + 2]
        nu = -1.0 - ka
        fu = f(t + cm1 * h, um1); nf += 1
        u = (h * mu) * fu - nu * um1 - ka * um2
        cn = mu - nu * cm1 - ka * cm2
        um2, um1 = um1, u
        cm2, cm1 = cm1, cn
    tc = t + cm1 * h
    a21, a31, a32, a41, a42, a43 = h * FPA[di]
    B1, B2, B3, B4 = h * FPB[di]
    Bh = h * (FPBE[di][:4] - FPB[di]); Bh5 = h * FPBE[di][4]
    k1 = f(tc, u); nf += 1
    s3 = u + a31 * k1; s4 = u + a41 * k1; un = u + B1 * k1; ev = Bh[0] * k1
    k2 = f(tc + a21, u + a21 * k1); nf += 1
    s3 = s3 + a32 * k2; s4 = s4 + a42 * k2; un = un + B2 * k2; ev = ev + Bh[1] * k2
    k3 = f(tc + a31 + a32, s3); nf += 1
    s4 = s4 + a43 * k3; un = un + B3 * k3; ev = ev + Bh[2] * k3
    k4 = f(tc + a41 + a42 + a43, s4); nf += 1
    un = un + B4 * k4; ev = ev + Bh[3] * k4
    k5 = f(t + h, un); nf += 1
    ev = ev + Bh5 * k5
    return un, ev, nf, ms + 4, k5


def stab_poly(z, ms_index):
    """(R(z), R_hat(z)) of the degree with index ms_index for complex array z (step h = 1 on y' = z y)."""
    z = np.asarray(z, dtype=complex)
    ms = MS[ms_index]; st = int(STARTS[ms_index])
    f = lambda t, y: z * y
    y = np.ones_like(z)
    un, ev, _, _, _ = step(f, 0.0, y, z * y, 1.0, None, deg=(ms_index, ms, st))
    return un, un - ev


def power_rho(f, t, y, f0, zprev, maxiter=50):
    """Nonlinear power method (OrdinaryDiffEq maxeig!, non-RKC branch): safety 1.2, 5% convergence test.
    Returns (rho_est, z - y, n_f, vec_flops)."""
    n = len(y); nf = 0; vf = 0.0
    z = zprev if zprev is not None else f0
    un = np.linalg.norm(y); zn = np.linalg.norm(z); vf += 4 * n
    pert = np.spacing(un) if un > 0 else 1e-300
    dz = un * math.sqrt(pert) if un > 0 else pert
    z = y + (dz / zn) * z if zn > 0 else y + dz
    vf += 2 * n
    est = 0.0
    for it in range(1, maxiter + 1):
        fz = f(t, z); nf += 1
        tmp = fz - f0; Dn = np.linalg.norm(tmp); vf += 3 * n
        prev = est; est = Dn / dz * 1.2
        if it >= 2 and abs(prev - est) < est * 0.05:
            return est, z - y, nf, vf + n
        z = y + (dz / Dn) * tmp if Dn > 0 else -z
        vf += 2 * n
    return est, z - y, nf, vf


class Branch:
    """ROCK4 state machine; one call of attempt() = one attempted step (so a switched driver can monitor it)."""

    def __init__(self, f, t, y, f0, h, rtol, atol, cnt, rho_every=25, rho_fixed=None, nf_f0=0):
        self.f, self.t, self.y, self.f0, self.h = f, t, y.copy(), f0, h
        self.rtol, self.atol, self.cnt = rtol, atol, cnt
        self.rho_every = rho_every; self.rho_fixed = rho_fixed
        self.zprev = None; self.rho = None; self.since = 10 ** 9; self.rho_floor = 0.0
        self.errp = None; self.hp = None; self.facmax = 5.0; self.last_reject = False
        self.acc = self.rej = 0; self.stage_hist = []; self.n = len(y)
        self.hist = []          # (accepted?, h, stages, err)

    def refresh_rho(self):
        if self.rho_fixed is not None:
            self.rho = self.rho_fixed(self.t, self.y) if callable(self.rho_fixed) else float(self.rho_fixed)
            return
        est, self.zprev, k, vf = power_rho(self.f, self.t, self.y, self.f0, self.zprev)
        self.rho = max(est, self.rho_floor); self.cnt.e_pow += k; self.cnt.e_vec += vf; self.since = 0

    def attempt(self, tf, hcap=None):
        if self.rho is None or self.since >= self.rho_every:
            self.refresh_rho()
        h = min(self.h, tf - self.t)
        if hcap is not None:
            h = min(h, hcap)
        if h * self.rho > HMAX_HRHO:
            h = 0.8 * HMAX_HRHO / self.rho
        yn, ev, k, s, fn = step(self.f, self.t, self.y, self.f0, h, self.rho)
        c = self.cnt; c.e_f += k; c.e_vec += step_flops_vec(s - 4, self.n); c.e_steps += 1
        self.stage_hist.append(s)
        sc = self.atol + self.rtol * np.maximum(np.abs(self.y), np.abs(yn))
        err = math.sqrt(np.mean((ev / sc) ** 2))
        if not math.isfinite(err) or not np.all(np.isfinite(yn)):
            err = 1e10
        fac = (1.0 / max(err, 1e-16)) ** 0.25
        if err <= 1.0:
            self.acc += 1
            if self.errp is not None and not self.last_reject:
                facp = (self.errp ** 0.25) * fac * fac * (h / self.hp); fac = min(fac, facp)
            fac = min(self.facmax, max(0.1, 0.8 * fac))
            self.t += h; self.y = yn; self.f0 = fn; self.hp = h; self.errp = max(err, 1e-4); self.since += 1
            self.facmax = 2.0; self.last_reject = False
            self.h = h * fac
            ok = True
        else:
            self.rej += 1; self.last_reject = True; self.facmax = 1.0
            self.h = h * max(0.1, 0.8 * fac)
            self.since = 10 ** 9          # re-estimate rho after a rejection (rock4.f practice)
            ok = False
        self.hist.append((ok, h, s, err))
        return ok, h, s, err


class _C:
    def __init__(self):
        self.e_f = self.e_pow = self.e_steps = 0; self.e_vec = 0.0


def integrate(f, t0, y0, tf, rtol, atol, h0=1e-6, rho_every=25, rho_fixed=None, max_steps=10 ** 6, cnt=None):
    """Plain ROCK4 run (control arm). f(t0, y0) is counted as one f-evaluation."""
    cnt = cnt if cnt is not None else _C()
    f0 = f(t0, y0); cnt.e_f += 1
    b = Branch(f, t0, y0, f0, h0, rtol, atol, cnt, rho_every=rho_every, rho_fixed=rho_fixed)
    while b.t < tf - 1e-12 * max(1.0, abs(tf)):
        b.attempt(tf)
        if b.acc + b.rej > max_steps:
            raise RuntimeError(f'ROCK4 step cap at t={b.t}')
    return dict(y=b.y, acc=b.acc, rej=b.rej, nf=cnt.e_f + cnt.e_pow, nf_power=cnt.e_pow, vec=cnt.e_vec,
                mean_stages=float(np.mean(b.stage_hist)), max_stages=int(max(b.stage_hist)), branch=b)


def selftest(verbose=True):
    """(1) order of R_s and R_hat_s for every degree; (2) consistency c_end + sum B = 1 (y' = 1);
    (3) order 4 on a non-autonomous nonlinear scalar problem with a fixed degree."""
    out = {}
    worst5, emb = 0.0, 0.0
    for i in range(50):
        zs = np.array([1e-1, -1e-1, 1e-1j, -2e-1 + 1e-1j])
        R, Rh = stab_poly(zs, i)
        R2, _ = stab_poly(zs / 2, i)
        p5 = math.log2(np.max(np.abs(R - np.exp(zs))) / max(np.max(np.abs(R2 - np.exp(zs / 2))), 1e-300))
        worst5 = max(worst5, abs(p5 - 5))
        z3 = np.array([5e-2, -5e-2, 5e-2j])
        _, Rh3 = stab_poly(z3, i)
        emb = max(emb, float(np.max(np.abs(Rh3 - np.exp(z3)) / np.abs(z3) ** 4)))   # embedded: O(z^4)
    worst4 = emb
    out['stab_poly_order_dev_R'] = worst5; out['embedded_coeff_max'] = worst4
    # real-axis stability length check: |R_s(x)| <= 1 on [-0.35 ms^2 * 0.95, 0]
    lens = []
    for i in (0, 10, 30, 49):
        ms = MS[i]
        x = -np.linspace(0, 0.353 * (ms + 4) ** 2 - 3, 4000)
        R, _ = stab_poly(x, i)
        bad = np.where(np.abs(R) > 1 + 1e-9)[0]
        lens.append((ms + 4, float(-x[bad[0]]) if len(bad) else float(-x[-1]), 0.353 * (ms + 4) ** 2 - 3))
    out['real_stab_len'] = lens
    # (2) consistency
    cons = []
    for i in range(50):
        ms = MS[i]; st = int(STARTS[i])
        un, ev, _, _, _ = step(lambda t, y: np.ones_like(y), 0.0, np.zeros(1), np.ones(1), 1.0, None, deg=(i, ms, st))
        cons.append(abs(un[0] - 1.0))
    out['consistency_max'] = float(max(cons))
    # (3) non-autonomous nonlinear convergence (fixed degree index 3 and 20)
    lam = -3.0
    phi = lambda t: math.sin(3 * t) + 0.5 * math.cos(t)
    dphi = lambda t: 3 * math.cos(3 * t) - 0.5 * math.sin(t)
    fn = lambda t, y: lam * (y - phi(t)) + dphi(t) - (y - phi(t)) ** 2 + 0.3 * np.sin(t) * (y - phi(t))
    orders = {}
    for i in (3, 20):
        ms = MS[i]; st = int(STARTS[i]); errs = []
        for N in (20, 40, 80):
            h = 1.0 / N; y = np.array([phi(0.0) + 0.1]); t = 0.0
            for _ in range(N):
                y, _, _, _, _ = step(fn, t, y, fn(t, y), h, None, deg=(i, ms, st)); t += h
            errs.append(y[0])
        # reference with N = 640
        h = 1.0 / 640; yr = np.array([phi(0.0) + 0.1]); t = 0.0
        for _ in range(640):
            yr, _, _, _, _ = step(fn, t, yr, fn(t, yr), h, None, deg=(i, ms, st)); t += h
        e = [abs(x - yr[0]) for x in errs]
        orders[ms + 4] = (math.log2(e[0] / e[1]), math.log2(e[1] / e[2]))
    out['nonauto_orders'] = orders
    if verbose:
        print('ROCK4 selftest:', out)
    return out


if __name__ == '__main__':
    selftest()
