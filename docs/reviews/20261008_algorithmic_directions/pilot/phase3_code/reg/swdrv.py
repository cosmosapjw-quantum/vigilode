"""PROBE B2 driver (EXPLORATORY, not ledger authority): closed-loop RODAS5P (U form, I controller of
hyp/mfrep.py / probe/target/rep.py / probe/pclag/driver.py, Rust rodas5p_matrix_free_fast.rs semantics) with
selectable stage-solve arms, a plain ROCK4 arm, and the SWITCHED integrator (improved MF RODAS5P <-> ROCK4).

Arms (arm['kind']):
  base   : SPD07 production MF: GMRES(40) Zero start, maxit 200, full cycles, true residual between cycles, final
           diagnostic residual, L2 threshold max(g*1e-14, 1e-10*||b||_2).
  mf     : improved MF arm (probe A1 / T1-A): GMRES(40) Zero start on the WRMS-scaled system D W D^-1, projected
           in-cycle stop + one true residual, outer-coupled absolute stage target (coupled_target.py defaults:
           EPUS Theta 0.2, e_ref 0.5, p 6/5, U8 cap, RHO_FL guard, RHO_STALL stall rule), maxit 2000.
  direct : sparse LU of W = I - h g J every attempt (J from CPR-coloured JVPs, rebuilt only on a new step, reused
           after a rejection), 8 exact solves; LU/solve flops exact for the produced fill (lucost.py).
  rock4  : plain ROCK4 (rock4x.py), the control arm for attribution.
  swd    : SWITCHED with the DIRECT arm as the RODAS side (shadow rho from a Gershgorin bound of the assembled J;
           everything else as 'sw'); added by the probe as the attribution / recommended variant.
  sw     : SWITCHED: 'mf' by default; hand-off to ROCK4 when the shadow cost-per-unit-time ratio >= 5 for 3
           consecutive accepted ACCURACY-LIMITED steps (err >= (0.9/5)^5, i.e. growth not saturated) AND the FOV guard (guard.py) admits the predicted step (shadow back-off: after a
           ratio < 2.5 the next shadow is skipped for 1, 2, 4, ..., 64 accepted steps); revert on rejection
           rate > 15% (window of 20 ROCK4 attempts), realised cost per unit time over that window > 0.5x the RODAS
           reference (median shadow c_R of the triggering steps), guard h_FOV = 0 or stiff-Ritz-angle growth.
           Options: guard (True/False), kappa (ROCK4 internal tolerance factor), ratio, consec.
All Krylov exits are confirmed by a true residual; linear failures and non-finite stages are fed back to the
controller as rejections (h *= 0.2).  Counters -> flops() with the explicit cost model of pr.py.
"""
import math
import os
import sys
import time

import numpy as np
import scipy.sparse as sp

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import coupled_target as CT
import guard as G
import rock4x
from lucost import SparseLU

import json
_d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = float(_d['gamma']); A_ = np.array([[float(x) for x in r] for r in _d['A']])
C_ = np.array([[float(x) for x in r] for r in _d['C']]); c_ = np.array([float(x) for x in _d['c']])
bc = np.array([float(x) for x in _d['b_code']]); S = 8
grow = np.linalg.inv(np.eye(S) / g - C_).sum(axis=1)
EPS = np.finfo(float).eps
E_SETTLE = (0.9 / 5.0) ** 5      # 1.89e-4: below it the I controller's growth factor saturates at 5


class Budget(Exception):
    pass


class Cnt:
    KEYS = ('jvp', 'cjvp', 'cols', 'tres', 'diag', 'cycles', 'dots', 'axpys', 'scal', 'norms', 'rhs', 'ft',
            'solves', 'stalls', 'lus', 'lu_flops', 'lu_solve_flops', 'wassem', 'asm_axpys', 'jbuilds',
            'roundoff_bind', 'e_f', 'e_pow', 'e_vec', 'e_steps', 'sh_f', 'sh_vec', 'sh_n', 'g_jvp', 'g_eig', 'g_n',
            'scaled_ops')

    def __init__(self):
        for k in self.KEYS:
            setattr(self, k, 0)

    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def flops(prob, c):
    """Admitted flop total (dict of parts). c: Cnt or dict.
    Krylov op = F_jvp + 2n (shift v - hg Jv) [+ 2n WRMS scaling D^-1, D for the scaled form];
    guard Arnoldi column = F_jvp + 2n (scaling); coloured JVP = F_jvp (+ nnz scatter in wassem);
    dot/axpy/norm 2n, scal n, stage-RHS assembly axpys 2n, W assembly nnz + n, LU / LU-solve flops exact (fill),
    ROCK4 / shadow / power f-evals F_rhs each + their explicit vector flops, guard small dense work g_eig."""
    if not isinstance(c, dict):
        c = c.d()
    n = prob['n']; nnz = prob['nnzJ']; Fj = prob['F_jvp']; Fr = prob['F_rhs']; Fft = prob['F_ft']
    parts = dict(
        jvp=c['jvp'] * (Fj + 2 * n) + c['scaled_ops'] * 2 * n,
        cjvp=c['cjvp'] * Fj,
        lu=float(c['lu_flops']), lusolve=float(c['lu_solve_flops']),
        orth=2 * n * (c['dots'] + c['axpys'] + c['norms']) + n * c['scal'],
        rhs=c['rhs'] * Fr + c['ft'] * Fft,
        asm=2 * n * c['asm_axpys'] + c['wassem'] * (nnz + n),
        rock4=(c['e_f'] + c['e_pow']) * Fr + c['e_vec'],
        shadow=c['sh_f'] * Fr + c['sh_vec'],
        guard=c['g_jvp'] * (Fj + 2 * n) + c['g_eig'])
    parts['total'] = float(sum(parts.values()))
    return parts


def gmres(Op, b, thr, cnt, m=40, maxit=200, full_cycles=False, stall=None, hess=None):
    """Zero-start restarted GMRES, MGS2 (Rust kernels.rs). Op counts its own applications. In-cycle projected stop
    at thr unless full_cycles; acceptance on the TRUE residual. hess: dict -> receives raw first-cycle Hessenberg."""
    n = len(b); x = np.zeros(n); total = 0
    first = True; rn_prev = None; bn = None
    while True:
        if first:
            r = b.copy(); first = False; Wx = None
        else:
            Wx = Op(x); r = b - Wx; cnt.tres += 1; cnt.axpys += 1
        rn = np.linalg.norm(r); cnt.norms += 1
        if not math.isfinite(rn):
            raise Budget()
        if rn <= thr:
            break
        if stall is not None and Wx is not None and rn_prev is not None and rn > 0.25 * rn_prev:
            if bn is None:
                bn = np.linalg.norm(b)
            scale = bn + np.linalg.norm(x) + np.linalg.norm(x - Wx); cnt.norms += 2
            if rn <= stall * scale:
                cnt.stalls += 1
                break
        rn_prev = rn
        if total >= maxit:
            raise Budget()
        k = min(m, maxit - total, n)
        V = np.zeros((k + 1, n)); H = np.zeros((k + 1, k)); beta = rn; V[0] = r / beta; cnt.scal += 1
        rawH = np.zeros((k + 1, k)) if (hess is not None and total == 0) else None
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k + 1); gv[0] = beta; used = 0
        for j in range(k):
            w = Op(V[j]); cnt.cols += 1
            hcol = np.zeros(j + 1)
            for _ in range(2):
                for i in range(j + 1):
                    hh = V[i] @ w; hcol[i] += hh; w -= hh * V[i]
            cnt.dots += 2 * (j + 1); cnt.axpys += 2 * (j + 1)
            hn = np.linalg.norm(w); cnt.norms += 1
            H[:j + 1, j] = hcol; H[j + 1, j] = hn
            if rawH is not None:
                rawH[:j + 1, j] = hcol; rawH[j + 1, j] = hn
            col_scale = math.hypot(np.linalg.norm(hcol), hn)
            breakdown = (col_scale == 0.0) or (hn <= 100 * EPS * col_scale)
            for i in range(j):
                t = cs[i] * H[i, j] + sn[i] * H[i + 1, j]; H[i + 1, j] = -sn[i] * H[i, j] + cs[i] * H[i + 1, j]; H[i, j] = t
            den = math.hypot(H[j, j], H[j + 1, j]); cs[j] = H[j, j] / den; sn[j] = H[j + 1, j] / den
            H[j, j] = den; H[j + 1, j] = 0.0; gv[j + 1] = -sn[j] * gv[j]; gv[j] = cs[j] * gv[j]; used = j + 1
            if breakdown or (not full_cycles and abs(gv[j + 1]) <= thr):
                break
            V[j + 1] = w / hn; cnt.scal += 1
        if rawH is not None:
            hess['H'] = rawH[:used, :used].copy(); hess['m'] = used
        y = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + y @ V[:used]; cnt.axpys += used + 1
        total += used; cnt.cycles += 1
    if full_cycles:
        Op(x); cnt.diag += 1; cnt.axpys += 1; cnt.norms += 1
    return x, total


def make_arm(kind, **kw):
    a = dict(kind=kind, m=40, maxit=(200 if kind == 'base' else 2000), rtol_lin=1e-10, atol_lin=1e-14,
             stall=CT.RHO_STALL, Theta=0.2, ordering='mmd_at_plus_a',
             guard=True, kappa=1.0, ratio=5.0, consec=3, rej_rate=0.15, win=20, cost_revert=0.5, angle_revert=True)
    a.update(kw)
    return a


class Rodas:
    """Closed-loop RODAS5P attempt engine (state: t, y, h, last_rej, err_prev, cached J/f0/ft)."""

    def __init__(self, prob, rtol, arm, cnt, t, y, h):
        self.p, self.rtol, self.arm, self.cnt = prob, rtol, arm, cnt
        self.atol = rtol * prob['ascale']; self.n = len(y)
        self.t, self.y, self.h = t, y.copy(), h
        self.last_rej = None; self.err_prev = None; self.cached = False
        self.kind = {'sw': 'mf', 'swd': 'direct'}.get(arm['kind'], arm['kind'])
        self.tgt = CT.Target(Theta=arm['Theta']) if self.kind == 'mf' else None
        self.I = sp.identity(self.n, format='csr')
        self.att = self.acc = self.rej = self.lin_fail = 0
        self.span = prob['span'][1] - prob['span'][0]
        self.hess = None; self.stage1_b = None; self.D = None; self.f0 = None

    def reset(self, t, y, h):
        self.t, self.y, self.h = t, y.copy(), h
        self.last_rej = None; self.cached = False

    def attempt(self, tf):
        p, cnt = self.p, self.cnt
        h = min(self.h, tf - self.t)
        if self.last_rej is not None and h >= self.last_rej:
            h = np.nextafter(self.t + self.last_rej, -np.inf) - self.t
        self.att += 1
        t, y = self.t, self.y
        if not self.cached:
            self.J = p['J'](t, y); self.f0 = p['f'](t, y); cnt.rhs += 1
            self.ftv = p['ft'](t, y) if p['ft'] is not None else None
            if p['ft'] is not None:
                cnt.ft += 1
            self.cached = True
            if self.kind == 'direct':
                cnt.cjvp += p['colors']; cnt.jbuilds += 1
        J, f0, ftv = self.J, self.f0, self.ftv
        hg = h * g
        Wmv = lambda v: v - hg * (J @ v)
        sc = self.atol + self.rtol * np.abs(y); D = 1.0 / sc
        n = self.n
        U = np.zeros((S, n))
        if self.kind == 'direct':
            W = (self.I - hg * J).tocsr(); cnt.wassem += 1
            F = SparseLU(W, self.arm['ordering']); cnt.lus += 1; cnt.lu_flops += F.fflops
        eps_vec = self.tgt.stage_eps(h=h, span=self.span, err_hat=self.err_prev, n=n) if self.tgt else None
        linfail = nonfinite = False
        hess = {} if self.arm['kind'] == 'sw' else None
        for i in range(S):
            fi = f0 if i == 0 else p['f'](t + c_[i] * h, y + A_[i, :i] @ U[:i])
            if i:
                cnt.rhs += 1
            b = hg * fi + g * (C_[i, :i] @ U[:i])
            if ftv is not None:
                b = b + h * hg * grow[i] * ftv
            cnt.asm_axpys += 2 * i + 2
            if not np.all(np.isfinite(b)):
                nonfinite = True; break
            cnt.solves += 1
            try:
                if self.kind == 'direct':
                    x = F.solve(b); cnt.lu_solve_flops += F.sflops
                elif self.kind == 'base':
                    def Op(v):
                        cnt.jvp += 1; return Wmv(v)
                    thr = max(g * self.arm['atol_lin'], self.arm['rtol_lin'] * np.linalg.norm(b))
                    x, _ = gmres(Op, b, thr, cnt, m=self.arm['m'], maxit=self.arm['maxit'], full_cycles=True)
                else:   # mf (scaled WRMS form)
                    Db = D * b; Dbn = np.linalg.norm(Db)
                    thr, rb = self.tgt.l2_threshold(eps_vec[i], Dbn, n)
                    if rb:
                        cnt.roundoff_bind += 1
                    def Op(z):
                        cnt.jvp += 1; cnt.scaled_ops += 1
                        return D * Wmv(z / D)
                    z, _ = gmres(Op, Db, thr, cnt, m=self.arm['m'], maxit=self.arm['maxit'], stall=self.arm['stall'],
                                 hess=(hess if i == 0 else None))
                    x = z / D
            except Budget:
                linfail = True; break
            if not np.all(np.isfinite(x)):
                nonfinite = True; break
            U[i] = x
            if i == 0:
                self.stage1_b = b
        self.D = D
        self.hess = hess
        if linfail or nonfinite:
            self.rej += 1; self.last_rej = h; self.h = h * 0.2
            if linfail:
                self.lin_fail += 1
            return False, h, np.inf
        ynew = y + bc @ U
        sc2 = self.atol + self.rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1] / sc2) ** 2))
        if not math.isfinite(err):
            err = np.inf
        if err <= 1.0:
            self.acc += 1; self.t = t + h; self.y = ynew; self.last_rej = None; self.cached = False; self.err_prev = err
            fac = 5.0 if err == 0 else min(max(0.9 * err ** (-0.2), 0.2), 5.0)
            self.h = h * fac
            self.prev_t, self.prev_y, self.prev_f0 = t, y, f0
            return True, h, err
        self.rej += 1; self.last_rej = h
        fac = min(max(0.9 * max(err, 1e-16) ** (-0.2), 0.2), 0.9) if math.isfinite(err) else 0.2
        self.h = h * fac
        return False, h, err


def _rock_step_cost(prob, s):
    return s * prob['F_rhs'] + rock4x.step_flops_vec(s - 4, prob['n'])


def integrate(prob, rtol, arm, h0=None, max_att=60000, wall_cap=2400.0, log=False):
    f, y0 = prob['f'], prob['y0']
    t0, tf = prob['span']; span = tf - t0
    h0 = prob['h0'] if h0 is None else h0
    cnt = Cnt(); wt0 = time.time()
    kind = arm['kind']
    n = prob['n']
    events = []
    if kind == 'rock4':
        atol = rtol * prob['ascale']
        f0 = f(t0, y0); cnt.e_f += 1
        B = rock4x.Branch(f, t0, y0, f0, h0, rtol * arm['kappa'], atol * arm['kappa'], cnt)
        while B.t < tf - 1e-12 * max(1.0, abs(tf)):
            B.attempt(tf)
            if B.acc + B.rej > max_att:
                raise RuntimeError(f'attempt cap at t={B.t}')
            if time.time() - wt0 > wall_cap:
                raise RuntimeError(f'wall cap at t={B.t}')
        out = dict(att=B.acc + B.rej, acc=B.acc, rej=B.rej, lin_fail=0, y=B.y, t=B.t,
                   mean_stages=float(np.mean(B.stage_hist)), max_stages=int(max(B.stage_hist)),
                   rock_frac_time=1.0, n_handoff=0, n_revert=0)
        out.update(cnt.d()); out['flops'] = flops(prob, cnt); out['wall'] = time.time() - wt0
        return out
    R = Rodas(prob, rtol, arm, cnt, t0, y0, h0)
    if kind not in ('sw', 'swd'):
        while R.t < tf - 1e-12 * max(1.0, abs(tf)):
            R.attempt(tf)
            if R.att > max_att:
                raise RuntimeError(f'attempt cap at t={R.t}')
            if time.time() - wt0 > wall_cap:
                raise RuntimeError(f'wall cap at t={R.t} att={R.att}')
        out = dict(att=R.att, acc=R.acc, rej=R.rej, lin_fail=R.lin_fail, y=R.y, t=R.t, rock_frac_time=0.0,
                   n_handoff=0, n_revert=0)
        out.update(cnt.d()); out['flops'] = flops(prob, cnt); out['wall'] = time.time() - wt0
        return out
    # ------------------------------------------------------------------ switched integrator
    kap = arm['kappa']; atolE = rtol * prob['ascale'] * kap; rtolE = rtol * kap
    mode = 'R'; consec = 0; cool = 0; nrev = 0; nhand = 0
    rho_pow = None; zprev = None
    last_cost_mark = flops(prob, cnt)['total']
    shadow_log = []          # per accepted RODAS step: (t, h, ratio or None, reason)
    t_rock = 0.0; att_R = att_E = 0
    B = None; cR_ref = None; hR_last = None; ang0 = None; hcap = None; guard_age = 0; win = []
    ang_hist = []; hfov_hist = []
    cR_win = []; g_cost = 0.0; fl_lastg = 0.0; skip = 0; backoff = 0
    while True:
        t_now = R.t if mode == 'R' else B.t
        if t_now >= tf - 1e-12 * max(1.0, abs(tf)):
            break
        if time.time() - wt0 > wall_cap:
            raise RuntimeError(f'wall cap at t={t_now}')
        if R.att + att_E > max_att:
            raise RuntimeError(f'attempt cap at t={t_now}')
        if mode == 'R':
            ok, h, err = R.attempt(tf); att_R += 1
            if not ok:
                continue
            fl = flops(prob, cnt)['total']; C_R = fl - last_cost_mark; last_cost_mark = fl
            hR_last = h
            if cool > 0:
                cool -= 1; shadow_log.append((R.t, h, None, 'cool')); consec = 0
                continue
            if skip > 0:          # shadow back-off after clearly unfavourable ratios (< ratio/2)
                skip -= 1; shadow_log.append((R.t, h, None, 'backoff')); consec = 0
                continue
            if R.t >= tf - 1e-12 * max(1.0, abs(tf)):
                break
            # settle rule: compare costs only on accuracy-limited RODAS steps (controller growth not saturated,
            # err >= (0.9/5)^5); during the start-up ramp both methods run at tiny h and the ratio is not representative
            if arm.get('settle', True) and err < E_SETTLE:
                shadow_log.append((R.t, h, None, 'unsettled')); consec = 0
                continue
            # --- free information: stage-1 Hessenberg of D W D^-1 -> compression of D J D^-1
            Hd = R.hess or {}
            m1 = Hd.get('m', 0)
            hg = h * g
            if m1 >= 1:
                HJ = (np.eye(m1) - Hd['H']) / hg
                rz = G.ritz(HJ)
            elif R.kind == 'direct':
                # direct RODAS side: no Krylov basis; Gershgorin bound of the assembled J (upper bound, nnz flops)
                rz = dict(rho=float(abs(R.J).sum(axis=1).max()), angle=0.0)
                cnt.sh_vec += prob['nnzJ'] + n
            else:
                HJ = np.zeros((0, 0)); rz = dict(rho=0.0, angle=0.0)
            rho_est = max(rz['rho'], rho_pow or 0.0)
            rho_hat = 1.2 * rho_est if rho_est > 0 else None
            hR_star = h * min(10.0, 0.9 * max(err, 1e-16) ** (-0.2))
            cR = C_R / hR_star
            if rho_hat is None:
                shadow_log.append((R.t, h, None, 'no-rho')); consec = 0; continue
            hcap152 = 0.8 * rock4x.HMAX_HRHO / rho_hat
            # prescreen (free): best case ROCK4 at h_opt = min(4 h_R*, cap)
            h_opt = min(4 * hR_star, hcap152)
            cE_opt = _rock_step_cost(prob, rock4x.stages(h_opt, rho_hat)) / h_opt
            if cR / cE_opt < arm['ratio']:
                shadow_log.append((R.t, h, cR / cE_opt, 'prescreen')); consec = 0
                if cR / cE_opt < 0.5 * arm['ratio']:
                    backoff = min(64, max(1, 2 * backoff)); skip = backoff
                continue
            # shadow ROCK4 step from the accepted step's start point at h_R (charged)
            hs = min(h, hcap152)
            ys, ev, k, s, _ = rock4x.step(f, R.prev_t, R.prev_y, R.prev_f0, hs, rho_hat)
            cnt.sh_f += k; cnt.sh_vec += rock4x.step_flops_vec(s - 4, n); cnt.sh_n += 1
            last_cost_mark = flops(prob, cnt)['total']
            scE = atolE + rtolE * np.maximum(np.abs(R.prev_y), np.abs(ys))
            errE = math.sqrt(np.mean((ev / scE) ** 2)) if np.all(np.isfinite(ys)) else np.inf
            if not math.isfinite(errE):
                shadow_log.append((R.t, h, 0.0, 'shadow-nonfinite')); consec = 0; continue
            hE_star = min(hs * min(10.0, 0.8 * max(errE, 1e-16) ** (-0.25)), hcap152)
            sE = rock4x.stages(hE_star, rho_hat)
            cE = (_rock_step_cost(prob, sE) * 1.05) / hE_star
            ratio = cR / cE
            shadow_log.append((R.t, h, ratio, 'shadow'))
            consec = consec + 1 if ratio >= arm['ratio'] else 0
            if ratio < 0.5 * arm['ratio']:
                backoff = min(64, max(1, 2 * backoff)); skip = backoff
            else:
                backoff = 0
            cR_win = (cR_win + [cR])[-3:] if consec else []
            if consec < arm['consec']:
                continue
            # --- hand-off decision: real power iteration + FOV guard (charged)
            f_hand = f(R.t, R.y); cnt.rhs += 1
            est, zprev, kpw, vf = rock4x.power_rho(f, R.t, R.y, f_hand, zprev)
            cnt.e_pow += kpw; cnt.e_vec += vf; rho_pow = est / 1.2
            rho_hat = est
            hcap152 = 0.8 * rock4x.HMAX_HRHO / rho_hat
            hfov = None; reason = 'handoff'; rho_floor = 0.0
            if arm['guard']:
                Dg = R.D
                Jc = R.J
                Aop = lambda v: Dg * (Jc @ (v / Dg))
                Hg, mg = G.arnoldi(Aop, G.start_vector(Dg * R.stage1_b, cnt.g_n), G.M_GUARD, cnt=cnt, start_cols=0)
                cnt.g_eig += G.eig_flops(mg); cnt.g_n += 1
                rho_floor = 1.2 * G.ritz(Hg)['rho']          # Krylov lower bound x 1.2 floors the power estimate
                rho_hat = max(rho_hat, rho_floor)
                hcap152 = 0.8 * rock4x.HMAX_HRHO / rho_hat
                g_cost = G.eig_flops(mg) + mg * (prob['F_jvp'] + 2 * n) + 2 * n * sum(4 * (j + 1) + 1 for j in range(mg))
                Wb = G.fov_boundary(Hg)
                pts = G.inflate(Wb, rho_hat)
                hfov = G.fov_step_bound(pts, rho_hat, span, 1e-8 * span, hcap152)
                rzg = G.ritz(Hg)
                ang0 = rzg['angle']
                hE_eff = min(hE_star, hfov)
                if hE_eff <= 0:
                    reason = 'guard-veto(h_FOV=0)'
                else:
                    sE2 = rock4x.stages(hE_eff, rho_hat)
                    cE2 = (_rock_step_cost(prob, sE2) * 1.05) / hE_eff
                    if cR / cE2 < arm['ratio']:
                        reason = f'guard-veto(ratio {cR / cE2:.2f} at h_FOV)'
                hfov_hist.append((R.t, hfov, ang0))
            else:
                hE_eff = hE_star
            last_cost_mark = flops(prob, cnt)['total']
            if reason != 'handoff':
                events.append(dict(t=R.t, ev=reason, hR=h, hE=hE_star, hfov=hfov, ratio=ratio))
                consec = 0; cool = 10; continue
            # hand off
            nhand += 1; mode = 'E'; consec = 0
            cR_ref = float(np.median(cR_win)) if cR_win else cR
            B = rock4x.Branch(f, R.t, R.y, f_hand, hE_eff, rtolE, atolE, cnt)
            B.rho = rho_hat; B.zprev = zprev; B.since = 0
            B.rho_floor = rho_floor if arm['guard'] else 0.0
            hcap = hfov if arm['guard'] else None
            guard_age = 0; win = []; t_hand = R.t; fl_hand = flops(prob, cnt)['total']; fl_lastg = fl_hand
            events.append(dict(t=R.t, ev='handoff', hR=h, hE=hE_eff, hfov=hfov, ratio=ratio, rho_hat=rho_hat,
                               angle=ang0))
            continue
        # ------------------------------------------------ ROCK4 branch
        fl0 = flops(prob, cnt)['total']; tb0 = B.t
        ok, h, s, err = B.attempt(tf, hcap=hcap); att_E += 1
        guard_age += 1
        fl1 = flops(prob, cnt)['total']
        win.append((ok, fl1 - fl0, (B.t - tb0)))
        if ok:
            t_rock += h
        why = None
        if len(win) >= arm['win']:
            w10 = win[-arm['win']:]
            rr = sum(1 for q in w10 if not q[0]) / float(len(w10))
            dt = sum(q[2] for q in w10); cf = sum(q[1] for q in w10)
            if rr > arm['rej_rate']:
                why = f'rejection rate {rr:.2f}'
            elif dt > 0 and cf / dt > arm['cost_revert'] * cR_ref:
                why = f'cost {cf / dt:.3e} > {arm["cost_revert"]}x RODAS {cR_ref:.3e}'
            elif dt == 0:
                why = 'no progress'
        # guard refresh on a work budget: when the branch has spent >= 20x the last guard cost since the last check
        # (guard overhead <= 5% of the branch work), checked at accepted steps
        br_work = flops(prob, cnt)['total'] - fl_lastg
        if why is None and arm['guard'] and B.t < tf and ok and br_work >= 20.0 * g_cost:
            guard_age = 0
            Jc = prob['J'](B.t, B.y)
            Dg = 1.0 / (atolE + rtolE * np.abs(B.y))
            Aop = lambda v: Dg * (Jc @ (v / Dg))
            Hg, mg = G.arnoldi(Aop, G.start_vector(Dg * B.f0, cnt.g_n), G.M_GUARD, cnt=cnt, start_cols=0)
            cnt.g_eig += G.eig_flops(mg); cnt.g_n += 1
            Wb = G.fov_boundary(Hg)
            B.rho_floor = 1.2 * G.ritz(Hg)['rho']; B.rho = max(B.rho or 0.0, B.rho_floor)
            rhoh = B.rho
            hcap152 = 0.8 * rock4x.HMAX_HRHO / rhoh
            hfov = G.fov_step_bound(G.inflate(Wb, rhoh), rhoh, span, 1e-8 * span, hcap152)
            ang = G.ritz(Hg)['angle']
            hfov_hist.append((B.t, hfov, ang))
            hcap = hfov
            fl_lastg = flops(prob, cnt)['total']
            if arm['angle_revert'] and ang > max(0.3, 3.0 * (ang0 or 0.0)):
                why = f'Ritz angle growth {ang0:.3f} -> {ang:.3f}'
            elif hfov <= 0:
                why = 'guard h_FOV = 0'
        if why is not None:
            nrev += 1; mode = 'R'
            events.append(dict(t=B.t, ev='revert', why=why, rock_acc=B.acc, rock_rej=B.rej))
            R.reset(B.t, B.y, min(hR_last if hR_last else B.h, tf - B.t) if B.t < tf else B.h)
            R.err_prev = None
            cool = 10 * 2 ** nrev; consec = 0
            last_cost_mark = flops(prob, cnt)['total']
            continue
    yend = R.y if mode == 'R' else B.y
    out = dict(att=R.att + att_E, att_R=R.att, att_E=att_E, acc=R.acc + (B.acc if B else 0), rej=R.rej + (B.rej if B else 0),
               lin_fail=R.lin_fail, y=yend, t=(R.t if mode == 'R' else B.t), rock_frac_time=t_rock / span,
               n_handoff=nhand, n_revert=nrev, events=events,
               shadow_summary=_shadow_summary(shadow_log), hfov_hist=hfov_hist,
               mean_stages=(float(np.mean(B.stage_hist)) if B and B.stage_hist else None))
    out.update(cnt.d()); out['flops'] = flops(prob, cnt); out['wall'] = time.time() - wt0
    return out


def _shadow_summary(log):
    rs = np.array([q[2] for q in log if q[3] == 'shadow' and q[2] is not None])
    pre = np.array([q[2] for q in log if q[3] == 'prescreen' and q[2] is not None])
    return dict(n_acc=len(log), n_shadow=int(len(rs)), n_prescreen_skip=int(len(pre)),
                ratio_median=(float(np.median(rs)) if len(rs) else None),
                frac_ge5=(float(np.mean(rs >= 5)) if len(rs) else 0.0),
                prescreen_median=(float(np.median(pre)) if len(pre) else None))
