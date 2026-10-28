"""PROBE B5 core (exploratory replica; read-only w.r.t. /home/user/wt-speed).

Dense fast-driver replica (U form, exact J, one LU per attempt, J/f0/f_t reused after a rejection,
retry clipped strictly below the rejected trial, last step clipped to tf) = rodas5p_fast.rs semantics
(copied from hyp/ctrl/rep.py, which reproduces the Rust counters), and a matrix-free U-form replica
(rodas5p_matrix_free_fast.rs semantics, GMRES(40) x0=0, no preconditioner; copied from hyp/mfrep.py)
with pluggable step-size controllers and full admitted-cost counters.
"""
import json, math
import numpy as np
from scipy.linalg import lu_factor, lu_solve

ROOT = "/home/user/wt-speed"
_snap = json.load(open(f"{ROOT}/fixtures/rodas5p_coefficients_snapshot.json"))
g = float(_snap["gamma"])
A = np.array([[float(x) for x in r] for r in _snap["A"]])
C = np.array([[float(x) for x in r] for r in _snap["C"]])
c = np.array([float(x) for x in _snap["c"]])
bc = np.array([float(x) for x in _snap["b_code"]])
S = 8
Gam = np.linalg.inv(np.eye(S) / g - C)
grow = Gam.sum(axis=1)
REFS = json.load(open(f"{ROOT}/research/stiff_native_benchmark_20261001/NATIVE.json"))["references"]
HERE = "/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl"

# ----------------------------------------------------------------------------------------- problems
def robertson():
    f = lambda t, y: np.array([-0.04 * y[0] + 1e4 * y[1] * y[2],
                               0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] ** 2, 3e7 * y[1] ** 2])
    J = lambda t, y: np.array([[-0.04, 1e4 * y[2], 1e4 * y[1]],
                               [0.04, -1e4 * y[2] - 6e7 * y[1], -1e4 * y[1]], [0, 6e7 * y[1], 0]])
    return dict(name='robertson', f=f, J=J, ft=None, y0=np.array([1.0, 0, 0]), span=(0.0, 40.0), ascale=1e-4,
                ref=np.array(REFS["robertson"]["final_state"]), rhs_flops=12, jvp_flops=7, metric='comp')


def vdp(mu=1000.0):
    f = lambda t, y: np.array([y[1], mu * (1 - y[0] ** 2) * y[1] - y[0]])
    J = lambda t, y: np.array([[0.0, 1.0], [-2 * mu * y[0] * y[1] - 1, mu * (1 - y[0] ** 2)]])
    return dict(name='vdp', f=f, J=J, ft=None, y0=np.array([2.0, 0.0]), span=(0.0, 2000.0), ascale=1.0,
                ref=np.array(REFS["van-der-pol-mu1000"]["final_state"]), rhs_flops=4, jvp_flops=4, metric='comp')


def hires():
    def f(t, y):
        q = 280.0 * y[5] * y[7]
        return np.array([-1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007, 1.71 * y[0] - 8.75 * y[1],
                         -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4], 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
                         -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
                         -q + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6], q - 1.81 * y[6], -q + 1.81 * y[6]])

    def J(t, y):
        j = np.zeros((8, 8))
        j[0, :3] = [-1.71, 0.43, 8.32]; j[1, :2] = [1.71, -8.75]
        j[2, 2:5] = [-10.03, 0.43, 0.035]; j[3, 1:4] = [8.32, 1.71, -1.12]
        j[4, 4:7] = [-1.745, 0.43, 0.43]
        j[5, 3:8] = [0.69, 1.71, -280 * y[7] - 0.43, 0.69, -280 * y[5]]
        j[6, 5:8] = [280 * y[7], -1.81, 280 * y[5]]; j[7, 5:8] = [-280 * y[7], 1.81, -280 * y[5]]
        return j
    y0 = np.zeros(8); y0[0] = 1.0; y0[7] = 0.0057
    return dict(name='hires', f=f, J=J, ft=None, y0=y0, span=(0.0, 321.8122), ascale=1e-4,
                ref=np.array(REFS["hires"]["final_state"]), rhs_flops=6, jvp_flops=7, metric='comp')


def bruss(cells=50):
    cc = (cells + 1.0) ** 2 / 50.0; n = 2 * cells

    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.r_[1.0, u[:-1]]; ur = np.r_[u[1:], 1.0]; vl = np.r_[3.0, v[:-1]]; vr = np.r_[v[1:], 3.0]
        out = np.empty(n)
        out[0::2] = 1 + u * u * v - 4 * u + cc * (ul - 2 * u + ur)
        out[1::2] = 3 * u - u * u * v + cc * (vl - 2 * v + vr)
        return out
    ia = np.arange(cells)

    def J(t, y):
        j = np.zeros((n, n))
        u, v = y[0::2], y[1::2]; a = 2 * ia; b = a + 1
        j[a, a] = 2 * u * v - 4 - 2 * cc; j[a, b] = u * u; j[b, a] = 3 - 2 * u * v; j[b, b] = -u * u - 2 * cc
        j[a[1:], a[1:] - 2] = cc; j[b[1:], b[1:] - 2] = cc
        j[a[:-1], a[:-1] + 2] = cc; j[b[:-1], b[:-1] + 2] = cc
        return j
    x = (np.arange(cells) + 1.0) / (cells + 1.0)
    y0 = np.empty(n); y0[0::2] = 1 + np.sin(2 * np.pi * x); y0[1::2] = 3.0
    ref = REFS.get(f"brusselator-1d-{cells}", {}).get("final_state")
    if ref is None:
        try:
            ref = np.load(f"{HERE}/ref_bruss{cells}.npy")
        except FileNotFoundError:
            ref = None
    return dict(name=f'bruss{cells}', f=f, J=J, ft=None, y0=y0, span=(0.0, 10.0), ascale=1.0,
                ref=None if ref is None else np.array(ref), rhs_flops=7, jvp_flops=9, metric='comp')


def _rinv(k):
    v = k + 1; fr = 0.5; r = 0.0
    while v:
        if v & 1: r += fr
        v >>= 1; fr *= 0.5
    return 0.9 + 0.2 * r


def rotnn(n=96):
    """Corpus v2 rotating-nonnormal (scientific_corpus_v2.rs:756-893), calibration span [0,1], exact phi.
    f_t by complex step (f is analytic in t)."""
    nb = n // 2
    div_b = np.array([_rinv(b) for b in range(nb)])
    freq = np.array([(1.0 + (i % 7)) * div_b[i // 2] for i in range(n)])
    def ramp(t, cen, w): return 0.5 * (1 + np.tanh((t - cen) / w))
    def phi(t, d=0):
        if d == 0: return 0.4 * np.sin(freq * t) + 0.2 * np.cos(0.5 * freq * t)
        return 0.4 * freq * np.cos(freq * t) - 0.1 * freq * np.sin(0.5 * freq * t)
    def blocks(t):
        r = ramp(t, 0.5, 0.08); s = (20 + 480 * r) * div_b; eta = 0.1 + 0.8 * r
        th = (8 * t + 0.4 * np.sin(4 * t)) * div_b
        return s, eta, np.cos(th), np.sin(th)
    def apply(t, x):
        s, eta, cs, sn = blocks(t)
        x0, x1 = x[0::2], x[1::2]
        xr0 = cs * x0 + sn * x1; xr1 = -sn * x0 + cs * x1
        ar0 = -s * xr0 + eta * s * xr1; ar1 = -0.35 * s * xr1
        out = np.empty(n, dtype=np.result_type(x, t))
        out[0::2] = cs * ar0 - sn * ar1; out[1::2] = sn * ar0 + cs * ar1
        return out
    def f(t, y):
        ph = phi(t); nl = 40 * ramp(t, 0.6, 0.06)
        return apply(t, y - ph) + phi(t, 1) + nl * (y * y - ph * ph)
    def J(t, y):
        s, eta, cs, sn = blocks(t); nl = 40 * ramp(t, 0.6, 0.06)
        j = np.zeros((n, n))
        for b in range(nb):
            R = np.array([[cs[b], sn[b]], [-sn[b], cs[b]]]); Ab = np.array([[-s[b], eta * s[b]], [0, -0.35 * s[b]]])
            j[2 * b:2 * b + 2, 2 * b:2 * b + 2] = R.T @ Ab @ R
        j[np.arange(n), np.arange(n)] += 2 * nl * y
        return j
    def ft(t, y):
        dl = 1e-30
        return np.imag(f(t + 1j * dl, y.astype(complex))) / dl
    return dict(name='rotnn96', f=f, J=J, ft=ft, y0=phi(0.0), span=(0.0, 1.0), ascale=0.01,
                ref=phi(1.0), rhs_flops=30, jvp_flops=8, metric='norm')


PROBS = dict(robertson=robertson, vdp=vdp, hires=hires, bruss50=lambda: bruss(50), bruss160=lambda: bruss(160),
             rotnn96=rotnn)


def endpoint_error(p, y):
    ref = p['ref']
    if ref is None: return float('nan')
    if p['metric'] == 'norm':
        return float(np.max(np.abs(y - ref)) / np.max(np.abs(ref)))
    return float(np.max(np.abs(y - ref) / np.maximum(np.abs(ref), 1e-10)))


# --------------------------------------------------------------------------------------- controller
GATE_FAC = (0.2 / 0.59) ** 0.2


class Ctrl:
    """kind: I | PIc (adaptive.rs Pi as coded) | PRED (Hairer RODAS/RADAU5 Gustafsson min form) | H211b | PI34.
    s_acc: safety on accepted-step proposals (I and PRED, both PRED terms); rejections keep 0.9 (production).
    cap: first accepted factor after a rejection <= 1 (F-078 / Hairer). gate: CTRL-EXPANSIVE-GATE, proposal
    x(0.2/0.59)^(1/5) when rho1=||U_1||/||h g f0|| of the accepted attempt > 1. kobs: same-state observed exponent."""

    def __init__(self, kind="I", s_acc=0.9, cap=False, gate=False, kobs=False, k=5.0, theta=None,
                 fmin=0.2, fmax=5.0, frej=0.9, kfloor=2.0, colcap=None, colalpha=0.65, colfloor=0.5):
        self.kind, self.s, self.cap, self.gate, self.kobs, self.k = kind, s_acc, cap, gate, kobs, k
        self.colcap, self.colalpha, self.c_obs, self.ncolcap, self.colfloor = colcap, colalpha, None, 0, colfloor
        self.fmin, self.fmax, self.frej, self.kfloor = fmin, fmax, frej, kfloor
        self.theta = 0.9 ** k if theta is None else theta
        self.prev_err = None; self.prev_h = None; self.prev_ratio = 1.0
        self.hacc = None; self.erracc = None; self.last_rej = False; self.rej_pair = None
        self.nfire = 0; self.nacc = 0

    def _cl(self, x):
        return min(max(x, self.fmin), self.fmax)

    def accept(self, h, err, rho1=None):
        k, th, s = self.k, self.theta, self.s
        e = max(err, 1e-10)
        self.nacc += 1
        if err == 0.0 and self.kind in ("I", "PIc"):
            fac = self.fmax          # production: error == 0 -> max_factor
        elif self.kind == "I":
            fac = self._cl(s * e ** (-1 / k))
        elif self.kind == "PIc":
            fac = self._cl(s * e ** (-0.7 / k) * self.prev_err ** (0.4 / k)) if self.prev_err else self._cl(s * e ** (-1 / k))
        elif self.kind == "PRED":
            fac = self._cl(s * e ** (-1 / k))
            if self.hacc is not None:
                fg = self._cl(s * (h / self.hacc) * (self.erracc / e ** 2) ** (1 / k))
                fac = min(fac, fg)
            self.hacc = h; self.erracc = max(1e-2, err)
        elif self.kind == "H211b":
            if self.prev_err is None:
                fac = (th / e) ** (1 / k)
            else:
                fac = (th / e) ** (1 / (4 * k)) * (th / self.prev_err) ** (1 / (4 * k)) * self.prev_ratio ** (-0.25)
            fac = self._cl(1 + math.atan(fac - 1))
        elif self.kind == "PI34":
            fac = (th / e) ** (1 / k) if self.prev_err is None else (th / e) ** (0.7 / k) * (self.prev_err / th) ** (0.4 / k)
            fac = self._cl(fac)
        else:
            raise ValueError(self.kind)
        if self.cap and self.last_rej:
            fac = min(fac, 1.0)
        if self.gate and rho1 is not None and rho1 > 1.0:
            fac *= GATE_FAC; self.nfire += 1
        if self.colcap and self.c_obs:   # CTRL-NULLS (c) probe: Krylov-cost cap on the next step
            fc = (self.colcap / self.c_obs) ** (1.0 / self.colalpha)
            if fc < fac: fac = max(fc, self.colfloor * fac); self.ncolcap += 1   # bounded: h >= colfloor * h_acc
        if self.prev_h is not None:
            self.prev_ratio = h / self.prev_h
        self.prev_h = h; self.prev_err = max(err, 1e-16) if self.kind == "PIc" else e
        self.last_rej = False; self.rej_pair = None
        return fac

    def reject(self, h, err):
        k = self.k
        self.last_rej = True
        if not np.isfinite(err):
            self.rej_pair = None
            return self.fmin
        if self.kobs and self.rej_pair is not None:
            h0, e0 = self.rej_pair
            if e0 > err > 0 and h0 > h:
                kk = math.log(e0 / err) / math.log(h0 / h)
                k = min(max(kk, self.kfloor), self.k)
        e = max(err, 1e-16)
        fac = (self.theta / e) ** (1 / k) if self.kind in ("H211b", "PI34") else 0.9 * e ** (-1 / k)
        fac = min(max(fac, self.fmin), self.frej)
        self.rej_pair = (h, err)
        return fac

    def linfail(self):
        self.last_rej = True; self.rej_pair = None
        return self.fmin


ARMS = {
    'I':            dict(kind='I'),
    'I725':         dict(kind='I', s_acc=0.725),
    'PIc':          dict(kind='PIc'),
    'H211b+cap':    dict(kind='H211b', cap=True),
    'PI34+cap':     dict(kind='PI34', cap=True),
    'PRED':         dict(kind='PRED'),
    'PRED+cap':     dict(kind='PRED', cap=True),
    'PRED+gate':    dict(kind='PRED', gate=True),
    'PRED+cap+gate': dict(kind='PRED', cap=True, gate=True),
    'PRED725':      dict(kind='PRED', s_acc=0.725),
    'PRED+cap85':   dict(kind='PRED', cap=True, s_acc=0.85),
    'PRED+cap80':   dict(kind='PRED', cap=True, s_acc=0.80),
    'PRED+cap75':   dict(kind='PRED', cap=True, s_acc=0.75),
    'I+kobs':       dict(kind='I', kobs=True),
    'PRED+kobs':    dict(kind='PRED', kobs=True),
    'I+gate':       dict(kind='I', gate=True),
    'I85':          dict(kind='I', s_acc=0.85),
    'I80':          dict(kind='I', s_acc=0.80),
    'I65':          dict(kind='I', s_acc=0.65),
    'PRED80':       dict(kind='PRED', s_acc=0.80),
    'PRED65':       dict(kind='PRED', s_acc=0.65),
    'PRED+cc12':    dict(kind='PRED', colcap=12.0, colfloor=0.5),
    'PRED+cc20':    dict(kind='PRED', colcap=20.0, colfloor=0.5),
}


def make_ctrl(arm): return Ctrl(**ARMS[arm])


# ---------------------------------------------------------------------------------------- h0 rule
def hairer_h0(p, rtol, order=5):
    """HNW I, II.4 / DOPRI5 HINIT with iord = 5 (RODAS5P embedded error ~ h^5). Returns (h0, extra RHS)."""
    f = p['f']; t0, tf = p['span']; y0 = p['y0']; atol = rtol * p['ascale']
    f0 = f(t0, y0); sk = atol + rtol * np.abs(y0)
    rms = lambda v: math.sqrt(np.mean((v / sk) ** 2))
    d0, d1 = rms(y0), rms(f0)
    h0 = 1e-6 if (d0 < 1e-5 or d1 < 1e-5) else 0.01 * d0 / d1
    h0 = min(h0, tf - t0)
    f1 = f(t0 + h0, y0 + h0 * f0)
    d2 = rms(f1 - f0) / h0
    dm = max(d1, d2)
    h1 = max(1e-6, h0 * 1e-3) if dm <= 1e-15 else (0.01 / dm) ** (1.0 / order)
    return min(100 * h0, h1, tf - t0), 1   # f0 is reused by the first attempt; f1 is the one extra RHS


def seed_h0(p, rtol, seed):
    if seed == 'auto': return hairer_h0(p, rtol)
    return float(seed), 0


# ---------------------------------------------------------------------------------- dense replica
def dense_attempt(p, lu, f0, ftv, t, y, h):
    f = p['f']; n = len(y)
    U = np.zeros((S, n))
    with np.errstate(all="ignore"):
        for i in range(S):
            r = f0.copy() if i == 0 else f(t + c[i] * h, y + A[i, :i] @ U[:i])
            r = r + (C[i, :i] / h) @ U[:i]
            if ftv is not None:
                r = r + grow[i] * h * ftv
            if not np.all(np.isfinite(r)):
                return None, np.inf, U
            U[i] = lu_solve(lu, r)
        ynew = y + bc @ U
    return ynew, None, U


def dense_integrate(p, rtol, ctrl, h0=1e-6, maxatt=200000, h0_rhs=0, record=False):
    f, Jf = p['f'], p['J']; t0, tf = p['span']; atol = rtol * p['ascale']; n = len(p['y0'])
    t, y, h = t0, p['y0'].copy(), h0
    att = acc = rej = 0; last_rej_h = None; fresh = True; nrhs = h0_rhs; nJ = 0; rec = []
    while t < tf and att < maxatt:
        h = min(h, tf - t)
        if last_rej_h is not None and h >= last_rej_h:
            h = np.nextafter(t + last_rej_h, -np.inf) - t
        if fresh:
            J = Jf(t, y); f0 = f(t, y); nJ += 1; nrhs += 1
            ftv = p['ft'](t, y) if p['ft'] is not None else None
        att += 1; nrhs += S - 1
        with np.errstate(all="ignore"):
            lu = lu_factor(np.eye(n) / (h * g) - J, check_finite=False)
        ynew, bad, U = dense_attempt(p, lu, f0, ftv, t, y, h)
        if ynew is None:
            err = np.inf
        else:
            sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
            err = math.sqrt(np.mean((U[-1] / sc) ** 2))
            if not np.isfinite(err): err = np.inf
        if record:
            rec.append((t, h, err, err <= 1.0))
        if err <= 1.0:
            rho1 = np.linalg.norm(U[0]) / max(np.linalg.norm(h * g * f0), 1e-300) if ctrl.gate else None
            acc += 1; t += h; y = ynew; fresh = True; last_rej_h = None
            h = h * ctrl.accept(h, err, rho1)
        else:
            rej += 1; fresh = False; last_rej_h = h
            h = h * ctrl.reject(h, err)
    rhs_fl = p['rhs_flops'] * n
    flops = att * (2 * n ** 3 / 3 + n * n + S * 2 * n * n) + nrhs * rhs_fl + nJ * 3 * n
    return dict(att=att, acc=acc, rej=rej, lu=att, rhs=nrhs, jac=nJ, flops=flops, err=endpoint_error(p, y),
                fire=ctrl.nfire, nacc_ctrl=ctrl.nacc, done=bool(t >= tf), rec=rec if record else None)


# --------------------------------------------------------------------------- matrix-free replica
class Budget(Exception):
    def __init__(self, cnt): self.cnt = cnt


def gmres(Wmv, b, thresh, m=40, maxit=200, full_cycles=False):
    """mfrep.gmres with admitted-cost counters. Two-pass (CGS2, same counts as production two-pass MGS):
    column j (0-based) costs 2(j+1) inner products and 2(j+1) vector updates. 'vec' counts the other
    n-vector ops (norms, x updates, residual formation, normalisations)."""
    n = len(b); x = np.zeros(n); cnt = dict(jvp=0, ip=0, vu=0, vec=0, cols=0, cycles=0)
    r = b.copy(); rn = np.linalg.norm(r); cnt['vec'] += 1
    while True:
        if rn <= thresh:
            break
        if cnt['cols'] >= maxit:
            raise Budget(cnt)
        k = min(m, maxit - cnt['cols'], n)
        V = np.zeros((n, k + 1)); H = np.zeros((k + 1, k)); beta = rn; V[:, 0] = r / beta; cnt['vec'] += 1
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k + 1); gv[0] = beta; used = 0
        cnt['cycles'] += 1
        for j in range(k):
            w = Wmv(V[:, j]); cnt['jvp'] += 1; cnt['cols'] += 1
            for _ in range(2):
                hh = V[:, :j + 1].T @ w; H[:j + 1, j] += hh; w -= V[:, :j + 1] @ hh
            cnt['ip'] += 2 * (j + 1); cnt['vu'] += 2 * (j + 1)
            hn = np.linalg.norm(w); H[j + 1, j] = hn; cnt['vec'] += 1
            for i in range(j):
                tt = cs[i] * H[i, j] + sn[i] * H[i + 1, j]; H[i + 1, j] = -sn[i] * H[i, j] + cs[i] * H[i + 1, j]; H[i, j] = tt
            den = math.hypot(H[j, j], H[j + 1, j]); cs[j] = H[j, j] / den; sn[j] = H[j + 1, j] / den
            H[j, j] = den; H[j + 1, j] = 0.0; gv[j + 1] = -sn[j] * gv[j]; gv[j] = cs[j] * gv[j]; used = j + 1
            breakdown = hn <= 100 * np.finfo(float).eps * np.linalg.norm(H[:j + 2, j]) or hn == 0
            if (not full_cycles and abs(gv[j + 1]) <= thresh) or breakdown:
                break
            V[:, j + 1] = w / hn; cnt['vec'] += 1
        yk = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + V[:, :used] @ yk; cnt['vec'] += used
        r = b - Wmv(x); cnt['jvp'] += 1; cnt['vec'] += 1; rn = np.linalg.norm(r); cnt['vec'] += 1
    if full_cycles:
        cnt['jvp'] += 1; cnt['vec'] += 2   # production final diagnostic residual
    return x, cnt


TY = np.array([1.729, 0.511, 6.246, 6.081, 3.82, 4.771, 1.011, 1.0])
TE = np.array([1.393, 2.057, 2.266, 2.16, 1.51, 1.792, 2.011, 1.0])


def mf_integrate(p, rtol, ctrl, h0=1e-6, mode='proj', rtol_lin=1e-10, atol_lin=1e-14, theta=0.001, err_exp=1.2,
                 err0=1e-6, err_ref=0.5, floor8=2e-5, eta=1e-4, maxatt=20000, h0_rhs=0, record=False, diag=False):
    """Closed loop: stage RHS built from the inexact Krylov solutions; linear failures -> rejection x0.2,
    fed to the controller. modes: base (production full cycles + final diagnostic), proj (in-cycle projected
    exit + true-residual certification, L2 threshold max(g*atol_lin, rtol_lin*||b||)), tf (INO-FORCE-ABS
    uncertified absolute WRMS targets theta*min(1/(8TY),1/(8TE))*(min(1,err_prev/0.5))^1.2 with U8 floor),
    rel (uniform relative forcing eta*||b||, negative control)."""
    f, Jf = p['f'], p['J']; t0, tf = p['span']; atol = rtol * p['ascale']; n = len(p['y0'])
    t, y, h = t0, p['y0'].copy(), h0
    att = acc = rej = lfail = 0; last_rej_h = None; fresh = True; nrhs = h0_rhs; err_prev = err0
    tot = dict(jvp=0, ip=0, vu=0, vec=0, cols=0, cycles=0, st1_retry_jvp=0)
    eps_att = np.minimum(theta / (8 * TY), theta / (8 * TE))
    rec = []
    while t < tf and att < maxatt:
        h = min(h, tf - t)
        if last_rej_h is not None and h >= last_rej_h:
            h = np.nextafter(t + last_rej_h, -np.inf) - t
        if fresh:
            J = Jf(t, y); f0 = f(t, y); nrhs += 1
            ftv = p['ft'](t, y) if p['ft'] is not None else np.zeros(n)
            D = 1.0 / (atol + rtol * np.abs(y))
        att += 1
        W = np.eye(n) - h * g * J
        U = np.zeros((S, n)); failed = False; nonfinite = False; b0n = 1.0; cols_att = 0
        if mode == 'tf':
            Ws = (D[:, None] * W) / D[None, :]
        for i in range(S):
            if i:
                fi = f(t + c[i] * h, y + A[i, :i] @ U[:i]); nrhs += 1
            else:
                fi = f0
            b = h * g * fi + g * (C[i, :i] @ U[:i]) + h * h * g * grow[i] * ftv
            if not np.all(np.isfinite(b)):
                nonfinite = True; break
            if i == 0: b0n = np.linalg.norm(b)
            try:
                if mode in ('base', 'proj'):
                    x, cnt = gmres(lambda v: W @ v, b, max(g * atol_lin, rtol_lin * np.linalg.norm(b)),
                                   full_cycles=(mode == 'base'))
                elif mode == 'rel':
                    x, cnt = gmres(lambda v: W @ v, b, max(g * atol_lin, eta * np.linalg.norm(b)))
                elif mode == 'tf':
                    tgt = eps_att[i] * math.sqrt(n) * (min(1.0, err_prev / err_ref) ** err_exp)
                    if i == S - 1:
                        tgt = min(tgt, floor8 * math.sqrt(n))
                    Db = D * b
                    xs, cnt = gmres(lambda v: Ws @ v, Db, max(tgt, 1e-14 * np.linalg.norm(Db)))
                    x = xs / D; cnt['vec'] += 3
                else:
                    raise ValueError(mode)
            except Budget as B:
                cnt = B.cnt; failed = True
            for kk in ('jvp', 'ip', 'vu', 'vec', 'cols', 'cycles'):
                tot[kk] += cnt[kk]
            cols_att += cnt['cols']
            if not fresh and i == 0:
                tot['st1_retry_jvp'] += cnt['jvp']
            if failed: break
            U[i] = x
        if failed or nonfinite:
            rej += 1; lfail += failed; last_rej_h = h; fresh = False
            h = h * ctrl.linfail(); continue
        ynew = y + bc @ U
        sc = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1] / sc) ** 2))
        if not math.isfinite(err): err = np.inf
        if record: rec.append((t, h, err, err <= 1.0))
        if diag:   # exact stage chain at the same (t, y, h): noise in the controller's error estimate
            lu = lu_factor(W); Ux = np.zeros((S, n))
            for i in range(S):
                fi = f0 if i == 0 else f(t + c[i] * h, y + A[i, :i] @ Ux[:i])
                Ux[i] = lu_solve(lu, h * g * fi + g * (C[i, :i] @ Ux[:i]) + h * h * g * grow[i] * ftv)
            yx = y + bc @ Ux
            errx = math.sqrt(np.mean((Ux[-1] / (atol + rtol * np.maximum(np.abs(y), np.abs(yx)))) ** 2))
            rec.append((t, h, err, errx))
        if err <= 1.0:
            rho1 = np.linalg.norm(U[0]) / max(b0n, 1e-300) if ctrl.gate else None
            ctrl.c_obs = cols_att / S
            acc += 1; t += h; y = ynew; fresh = True; last_rej_h = None; err_prev = err
            h = h * ctrl.accept(h, err, rho1)
        else:
            rej += 1; fresh = False; last_rej_h = h
            h = h * ctrl.reject(h, err)
    out = dict(att=att, acc=acc, rej=rej, lfail=lfail, rhs=nrhs, err=endpoint_error(p, y), fire=ctrl.nfire, ncolcap=ctrl.ncolcap,
               done=bool(t >= tf), n=n, rec=rec if (record or diag) else None)
    out.update(tot)
    return out


def mf_flops(r, p, jvp_per_comp):
    """Admitted flop model: JVP = jvp_per_comp*n, inner product / vector update / other vector op = 2n,
    RHS = rhs_flops*n. GMRES small least-squares (O(j) per column) and stage combinations ignored."""
    n = r['n']
    return r['jvp'] * jvp_per_comp * n + (r['ip'] + r['vu'] + r['vec']) * 2 * n + r['rhs'] * p['rhs_flops'] * n
