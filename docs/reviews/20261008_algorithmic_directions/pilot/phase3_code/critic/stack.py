"""Probe B1 replica: the integrated Tier-1 stack in closed loop (EXPLORATORY pilot, not ledger authority).

Matrix-free U-form RODAS5P (rodas5p_matrix_free_fast.rs semantics as replicated by probe A1 rep.py: Zero start,
GMRES(40) with two-pass modified Gram-Schmidt, h0 = 1e-6, stage RHS from the inexact solves, linear failures and
non-finite stages fed back to the controller as rejections, every solve accepted on a true residual).
Derived from probe A1 rep.py (copied as rep_a1.py, untouched) and probe/ctrl/core.py (controller, copied below).

Arm dict fields
  kind   : 'base' (full cycles + between-cycle true residual), 'proj' (in-cycle Givens stop at L2 max(g atol_lin,
           rtol_lin ||b||), one true residual), 'l2c' (proj with rtol_lin = min(1e-10, 1e-3 rtol), atol_lin =
           1e-3 atol), 'wabs' (proj with an absolute WRMS target, GMRES on D W D^-1, A1 guard + stall rule), 'lu'
  dup    : base only: count production's duplicate final diagnostic residual (arm 0) or not (arm 1)
  tgt    : wabs only: dict(Theta=0.2 EPUS budget or None, ino=per-step budget or None) -> theta_n =
           max(Theta h/T, ino) * min(1, e_hat/0.5)^1.2; eps_i = theta_n/(8 max(tau_y,i, tau_e,i)); eps_8 <= 0.1 e_sat
  ctrl   : 'I' (production), 'PRED+cap' (Hairer/Gustafsson predictive, no growth after a rejection), 'I725'
           (uniform safety 0.725 on the I controller: the judge's cheap set-point rival), 'PRED'
  maxit  : Krylov column budget per stage solve (200 production, 2000 arm 6)
  guard  : stagnation guard at each restart: abort if q = ||r_k||/||r_{k-1}|| >= q_abort (0.98) or if the
           geometric prediction total + m*ceil(log(thr/||r_k||)/log q) exceeds maxit (KRY-BUDGET-PREDICT form)
  jvp    : 'exact' (analytic J v) or 'fd' (forward difference, sigma = sqrt(eps)(1+||y||_2)/||v||_2, cached f0;
           the in-repo semilinear_f033_ablation.rs formula, there with an uncached base RHS)
Counters: jvp (= Rust jvp_vectors), cols (linear_iterations), tres, diag, dots (orthogonalization inner products),
oaxpys (orthogonalization vector updates), axpys (all vector updates incl. ortho, x updates, residuals), norms,
scal, rhs (stage + f0 RHS; FD RHS are counted separately in fd_rhs), ft, solves, lu, lu_solves, prec (always 0),
guard_q / guard_over (guard aborts), guard_false (aborts whose uncounted shadow continuation converged within maxit),
stalls, floor_bind, roundoff_bind."""
import json, math, os, sys
import time as _time
import numpy as np
from scipy.linalg import lu_factor, lu_solve

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import coupled_target as CT

d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = float(d['gamma']); A = np.array([[float(x) for x in r] for r in d['A']])
C = np.array([[float(x) for x in r] for r in d['C']]); c = np.array([float(x) for x in d['c']])
bc = np.array([float(x) for x in d['b_code']]); S = 8
Gam = np.linalg.inv(np.eye(S)/g - C); grow = Gam.sum(axis=1)
EPS = np.finfo(float).eps
SQEPS = math.sqrt(EPS)
GUARD_LOG = None   # critic diagnostic: list to record guard aborts
WALL_CAP = None   # critic: optional wall-clock cap (s) per run; capped runs are flagged
ORTHO = 'mgs2'   # critic: 'cgs2' = vectorized block CGS2 (speed), default keeps B1's MGS2


class Budget(Exception):
    def __init__(self, why='maxit'):
        self.why = why


class Cnt:
    KEYS = ('jvp', 'cols', 'tres', 'diag', 'cycles', 'dots', 'oaxpys', 'axpys', 'scal', 'norms', 'rhs', 'ft',
            'solves', 'fd_rhs', 'lu', 'lu_solves', 'prec', 'zero_solves', 'floor_bind', 'roundoff_bind', 'stalls',
            'nonfinite', 'guard_q', 'guard_over', 'guard_false', 'maxit_fail', 'breakdowns', 'nu_cols', 'nu_flops', 'nu_max', 'fb_accepts')

    def __init__(self):
        for k in self.KEYS: setattr(self, k, 0)
        self.gap = None          # list of exit records when gap diagnostics are on

    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def gmres(Wmv, b, thr, cnt, m=40, maxit=200, full_cycles=False, stall=None, guard=False, q_abort=0.98,
          shadow=False, nu=False, fallback=None):
    """Zero-start restarted GMRES with MGS2 (Rust gmres_into loop; A1 rep.py), acceptance on the true residual
    ||b - W x||_2 <= thr. Optional A1 stall rule, stagnation guard (with an uncounted shadow continuation that
    records whether an abort was false). Raises Budget. Returns x."""
    n = len(b); x = np.zeros(n); total = 0
    first = True; rn_prev = None; bn = None; last_proj = None; last_bd = False
    K = cnt; shadow_why = None
    thr0 = thr; nu_hat = 1.0   # critic nu-guard: thr = thr0/max(1, nu_hat), nu_hat = running max 1/sigma_min(R_j)
    while True:
        if first:
            r = b.copy(); first = False; Wx = None
        else:
            Wx = Wmv(x); r = b - Wx; K.jvp += 1; K.tres += 1; K.axpys += 1
        rn = np.linalg.norm(r); K.norms += 1
        if not math.isfinite(rn):
            if shadow_why: break
            raise Budget('nonfinite')
        if rn <= thr:
            if shadow_why: cnt.guard_false += 1; break
            if cnt.gap is not None:
                cnt.gap.append((last_proj, rn, thr, last_bd, total))
            break
        if stall is not None and Wx is not None and rn_prev is not None and rn > 0.25*rn_prev:
            if bn is None: bn = np.linalg.norm(b)
            scale = bn + np.linalg.norm(x) + np.linalg.norm(x - Wx); K.norms += 2
            if rn <= stall*scale:
                if shadow_why: cnt.guard_false += 1; break
                K.stalls += 1
                if cnt.gap is not None:
                    cnt.gap.append((last_proj, rn, thr, last_bd, total))
                break
        if guard and not shadow_why and Wx is not None and rn_prev is not None:
            q = rn/rn_prev; why = None
            if q >= q_abort:
                why = 'q'
            else:
                need = math.ceil(math.log(thr/rn)/math.log(q))
                if total + m*need > maxit: why = 'over'
            if why and fallback is not None and fallback(r):
                cnt.fb_accepts += 1; break        # critic: accept what production would accept instead of failing
            if why:
                if GUARD_LOG is not None:
                    _bn = np.linalg.norm(b); _sc = _bn + np.linalg.norm(x) + (np.linalg.norm(x - Wx) if Wx is not None else 0.0)
                    GUARD_LOG.append((why, rn/thr, rn/(CT.RHO_STALL*_sc), rn/_bn, q, total))
                if why == 'q': cnt.guard_q += 1
                else: cnt.guard_over += 1
                if not shadow:
                    raise Budget(why)
                shadow_why = why; K = Cnt()       # continue uncounted to classify the abort
        rn_prev = rn
        if total >= maxit:
            if shadow_why: break
            if fallback is not None and fallback(r):
                cnt.fb_accepts += 1; break
            cnt.maxit_fail += 1
            raise Budget('maxit')
        k = min(m, maxit - total, n)
        V = np.zeros((n, k+1)); H = np.zeros((k+1, k)); beta = rn; V[:, 0] = r/beta; K.scal += 1
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k+1); gv[0] = beta; used = 0; bd = False
        for j in range(k):
            w = Wmv(V[:, j]); K.jvp += 1; K.cols += 1
            if ORTHO == 'cgs2':      # critic speed option: block classical GS, two passes (same counted dots/axpys)
                Vj = V[:, :j+1]; hcol = Vj.T @ w; w -= Vj @ hcol; h2 = Vj.T @ w; w -= Vj @ h2; hcol = hcol + h2
            else:
                hcol = np.zeros(j+1)
                for _ in range(2):
                    for i in range(j+1):
                        hh = V[:, i] @ w; hcol[i] += hh; w -= hh*V[:, i]
            K.dots += 2*(j+1); K.oaxpys += 2*(j+1); K.axpys += 2*(j+1)
            hn = np.linalg.norm(w); K.norms += 1
            H[:j+1, j] = hcol; H[j+1, j] = hn
            col_scale = math.hypot(np.linalg.norm(hcol), hn)
            breakdown = (col_scale == 0.0) or (hn <= 100*EPS*col_scale)
            for i in range(j):
                t = cs[i]*H[i, j] + sn[i]*H[i+1, j]; H[i+1, j] = -sn[i]*H[i, j] + cs[i]*H[i+1, j]; H[i, j] = t
            den = math.hypot(H[j, j], H[j+1, j]); cs[j] = H[j, j]/den; sn[j] = H[j+1, j]/den
            H[j, j] = den; H[j+1, j] = 0.0; gv[j+1] = -sn[j]*gv[j]; gv[j] = cs[j]*gv[j]; used = j+1
            if nu and abs(gv[j+1]) <= thr:   # evaluated only when the stop test would pass
                smin = np.linalg.svd(np.triu(H[:j+1, :j+1]), compute_uv=False)[-1]
                cnt.nu_cols += 1; cnt.nu_flops += 4*(j+1)**3   # charged as a dense SVD of the j x j factor
                if smin > 0 and 1.0/smin > nu_hat:
                    nu_hat = 1.0/smin; thr = thr0/nu_hat; cnt.nu_max = max(cnt.nu_max, nu_hat)
            if breakdown:
                bd = True; K.breakdowns += 1; break
            if not full_cycles and abs(gv[j+1]) <= thr:
                break
            V[:, j+1] = w/hn; K.scal += 1
        y = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + V[:, :used] @ y; K.axpys += used + 1
        total += used; K.cycles += 1; last_proj = abs(gv[used]); last_bd = bd
    if shadow_why:
        raise Budget(shadow_why)
    return x


# ------------------------------------------------------------------------------------------- controllers
class Ctrl:
    """probe/ctrl/core.py Ctrl (copied; I and PRED kinds). I reproduces rep.py / adaptive.rs exactly.
    PRED: Hairer RODAS/RADAU5 Gustafsson min form (erracc floored at 1e-2); cap: first accepted factor after a
    rejection <= 1. s_acc: safety on accepted-step proposals; rejections keep 0.9."""

    def __init__(self, kind='I', s_acc=0.9, cap=False, k=5.0, fmin=0.2, fmax=5.0, frej=0.9):
        self.kind, self.s, self.cap, self.k = kind, s_acc, cap, k
        self.fmin, self.fmax, self.frej = fmin, fmax, frej
        self.hacc = None; self.erracc = None; self.last_rej = False

    def _cl(self, x):
        return min(max(x, self.fmin), self.fmax)

    def accept(self, h, err):
        k, s = self.k, self.s
        e = max(err, 1e-10)
        if err == 0.0 and self.kind == 'I':
            fac = self.fmax
        elif self.kind == 'I':
            fac = self._cl(s*e**(-1/k))
        elif self.kind == 'PRED':
            fac = self._cl(s*e**(-1/k))
            if self.hacc is not None:
                fg = self._cl(s*(h/self.hacc)*(self.erracc/e**2)**(1/k))
                fac = min(fac, fg)
            self.hacc = h; self.erracc = max(1e-2, err)
        else:
            raise ValueError(self.kind)
        if self.cap and self.last_rej:
            fac = min(fac, 1.0)
        self.last_rej = False
        return fac

    def reject(self, h, err):
        self.last_rej = True
        if not np.isfinite(err):
            return self.fmin
        e = max(err, 1e-16)
        return min(max(0.9*e**(-1/self.k), self.fmin), self.frej)

    def linfail(self):
        self.last_rej = True
        return self.fmin


CTRLS = {'I': dict(kind='I'), 'PRED+cap': dict(kind='PRED', cap=True), 'PRED': dict(kind='PRED'),
         'I725': dict(kind='I', s_acc=0.725)}


# ------------------------------------------------------------------------------------------- stage targets
class StackTarget(CT.Target):
    """A1 coupled target with an optional INO per-step budget: theta_n = max(Theta*h/T, ino) * phi(e_hat).
    Theta=None -> INO alone (the judge's uncertified INO arm); ino=None -> A1's rule exactly."""

    def __init__(self, Theta=0.2, ino=None, **kw):
        super().__init__(Theta=(Theta if Theta is not None else 0.0), epus=True, **kw)
        self.use_epus = Theta is not None; self.ino = ino

    def step_budget(self, h, span, err_hat):
        e = self.e0 if err_hat is None else err_hat
        phi = min(1.0, max(e, 0.0)/self.e_ref)**self.p if self.p else 1.0
        b = self.Theta*h/span if self.use_epus else 0.0
        if self.ino is not None:
            b = max(b, self.ino)
        return b*phi


def make_arm(kind='base', **kw):
    a = dict(kind=kind, dup=(kind == 'base'), rtol_lin=1e-10, atol_lin=1e-14, m=40, maxit=200, ctrl='I',
             guard=False, q_abort=0.98, jvp='exact', tgt=None, stall=None, cr=1e-3, ca=1e-3)
    if kind == 'wabs':
        a['stall'] = CT.RHO_STALL
        a['tgt'] = dict(Theta=0.2, ino=None)
    a.update(kw)
    return a


def integrate(prob, rtol, arm, h0=1e-6, max_att=60000, shadow=True, gapdiag=False, noise=False, cap_ok=False):
    """Closed-loop adaptive integration. Returns counters, trajectory statistics and the endpoint state.
    gapdiag: record (projected, true, exact-J true) residuals per solve exit (FD arms); noise: exact-solve error
    estimate at every attempt (uncounted) for the controller-noise diagnostic."""
    f, ft, y0 = prob['f'], prob['ft'], prob['y0']
    t0, tf = prob['span']; span = tf - t0
    atol = rtol*prob['ascale']; n = len(y0)
    t, y, h = t0, y0.astype(float).copy(), h0
    _t_start = _time.time()
    cnt = Cnt(); att = acc = rej = lin_fail = nf_fail = 0; last_rej = None; err_prev = None
    if gapdiag: cnt.gap = []
    gaps = []; noise_rec = []
    ctrl = Ctrl(**CTRLS[arm['ctrl']])
    cached = False; assembly = 0; capped = False
    kind = arm['kind']
    tgt = StackTarget(**arm['tgt']) if kind == 'wabs' else None
    fd = arm['jvp'] == 'fd'
    while t < tf - 1e-12*max(1.0, abs(tf)):
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        att += 1
        if not cached:
            J = prob['J'](t, y); f0 = f(t, y); cnt.rhs += 1
            ftv = ft(t, y) if ft is not None else np.zeros(n)
            if ft is not None: cnt.ft += 1
            ynorm = float(np.linalg.norm(y))
            cached = True
        hg = h*g
        if fd:
            def Jv(v, t=t, y=y, f0=f0, ynorm=ynorm):
                nv = np.linalg.norm(v)
                if nv == 0.0: return np.zeros_like(v)
                sg = SQEPS*(1.0 + ynorm)/nv; cnt.fd_rhs += 1
                return (f(t, y + sg*v) - f0)/sg
            Wmv = lambda v: v - hg*Jv(v)
        else:
            Wmv = lambda v: v - hg*(J @ v)
        Wex = (lambda v: v - hg*(J @ v))
        sc = atol + rtol*np.abs(y); D = 1.0/sc
        U = np.zeros((S, n))
        eps_vec = tgt.stage_eps(h=h, span=span*arm.get('tscale', 1.0), err_hat=err_prev) if kind == 'wabs' else None  # critic: tscale = misdeclared T
        if kind == 'lu':
            LUf = lu_factor(np.eye(n) - hg*J); cnt.lu += 1
        linfail = False; nonfinite = False
        for i in range(S):
            if i == 0:
                fi = f0
            else:
                fi = f(t + c[i]*h, y + A[i, :i] @ U[:i]); cnt.rhs += 1
            b = hg*fi + g*(C[i, :i] @ U[:i]) + h*hg*grow[i]*ftv
            assembly += 2*i + 2
            if not np.all(np.isfinite(b)):
                nonfinite = True; break
            cnt.solves += 1
            ng = len(cnt.gap) if gapdiag else 0
            try:
                if kind == 'lu':
                    x = lu_solve(LUf, b); cnt.lu_solves += 1
                elif kind == 'base':
                    thr = max(g*arm['atol_lin'], arm['rtol_lin']*np.linalg.norm(b))
                    x = gmres(Wmv, b, thr, cnt, m=arm['m'], maxit=arm['maxit'], full_cycles=True, stall=arm['stall'],
                              guard=arm['guard'], q_abort=arm['q_abort'], shadow=shadow)
                    if arm['dup']:
                        cnt.jvp += 1; cnt.diag += 1; cnt.axpys += 1; cnt.norms += 1
                elif kind in ('proj', 'l2c'):
                    if kind == 'proj':
                        rl, al = arm['rtol_lin'], arm['atol_lin']
                    else:
                        rl, al = min(1e-10, arm['cr']*rtol), arm['ca']*atol
                    bn = np.linalg.norm(b)
                    if g*al > rl*bn: cnt.floor_bind += 1
                    thr = max(g*al, rl*bn)
                    x = gmres(Wmv, b, thr, cnt, m=arm['m'], maxit=arm['maxit'], guard=arm['guard'], stall=arm['stall'],
                              q_abort=arm['q_abort'], shadow=shadow)
                elif kind == 'wabs':
                    Db = D*b; Dbn = np.linalg.norm(Db)
                    thr, rb = tgt.l2_threshold(eps_vec[i], Dbn, n)
                    if rb: cnt.roundoff_bind += 1
                    Ws = lambda v: D*Wmv(v/D)
                    xs = gmres(Ws, Db, thr, cnt, m=arm['m'], maxit=arm['maxit'], stall=arm['stall'],
                               guard=arm['guard'], q_abort=arm['q_abort'], shadow=shadow, nu=arm.get('nu', False),
                               fallback=((lambda rs, D=D, bn2=float(np.linalg.norm(b)): float(np.linalg.norm(rs/D)) <= max(g*1e-14, arm.get('fb_rel', 1e-10)*bn2))
                                         if arm.get('fb') else None))
                    x = xs/D
                else:
                    raise ValueError(kind)
            except Budget:
                linfail = True; break
            if not np.all(np.isfinite(x)):
                cnt.nonfinite += 1; nonfinite = True; break
            if np.all(x == 0): cnt.zero_solves += 1
            if gapdiag and len(cnt.gap) > ng:
                # exact-J true residual of the accepted iterate, in the solver's own norm (uncounted)
                rex = b - Wex(x)
                exn = float(np.linalg.norm(D*rex)) if kind == 'wabs' else float(np.linalg.norm(rex))
                gaps.append(cnt.gap[-1] + (exn,))
            U[i] = x
        if linfail:
            lin_fail += 1; rej += 1; last_rej = h; h *= ctrl.linfail(); continue
        if nonfinite:
            nf_fail += 1; rej += 1; last_rej = h; h *= ctrl.linfail(); continue
        ynew = y + bc @ U
        sc2 = atol + rtol*np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1]/sc2)**2))
        if not math.isfinite(err): err = np.inf
        if noise:
            lu = lu_factor(np.eye(n) - hg*J); Ux = np.zeros((S, n))
            for i in range(S):
                fi = f0 if i == 0 else f(t + c[i]*h, y + A[i, :i] @ Ux[:i])
                Ux[i] = lu_solve(lu, hg*fi + g*(C[i, :i] @ Ux[:i]) + h*hg*grow[i]*ftv)
            yx = y + bc @ Ux
            errx = math.sqrt(np.mean((Ux[-1]/(atol + rtol*np.maximum(np.abs(y), np.abs(yx))))**2))
            noise_rec.append((err, errx, float(np.sqrt(np.mean(((ynew - yx)/sc)**2)))))
        if err <= 1.0:
            acc += 1; t += h; y = ynew; last_rej = None; cached = False; err_prev = err
            h *= ctrl.accept(h, err)
        else:
            rej += 1; last_rej = h; h *= ctrl.reject(h, err)
        if WALL_CAP and att % 50 == 0 and _time.time() - _t_start > WALL_CAP:
            capped = True; break
        if att > max_att:
            if not cap_ok: raise RuntimeError(f'attempt cap at t={t}')
            capped = True; break
    out = dict(att=att, acc=acc, rej=rej, lin_fail=lin_fail, nf_fail=nf_fail, y=y, t=t, assembly_axpys=assembly, capped=capped)
    out.update(cnt.d())
    if gapdiag: out['gaps'] = gaps
    if noise: out['noise'] = noise_rec
    return out


def flops(prob, r, arm, jvp_model='analytic'):
    """Admitted flop total (explicit cost model, A1 rep.flops extended):
       F_jvp(analytic) = 2 nnzJ + 2n (shift W v = v - hg Jv); F_jvp(fd) = frhs + 6n (y + s v, difference, scale,
       shift); the scaled WRMS form adds 2n per operator application (D^-1 v, D w). Dot, axpy, norm = 2n; scal = n;
       stage assembly axpy = 2n; RHS and f_t = frhs. Dense LU = 2n^3/3, LU solve = 2n^2. Preconditioner apps: 0."""
    n = len(prob['y0'])
    fd = (jvp_model == 'fd') or arm.get('jvp') == 'fd'
    Fj = (prob['frhs'] + 6*n) if fd else (2*prob['nnzJ'] + 2*n)
    if arm['kind'] == 'wabs': Fj += 2*n
    tot = r['jvp']*Fj + 2*n*(r['dots'] + r['axpys'] + r['norms']) + n*r['scal'] + r['rhs']*prob['frhs'] \
        + 2*n*r['assembly_axpys'] + r['ft']*prob['frhs'] + r['lu']*(2.0*n**3/3) + r['lu_solves']*2.0*n*n + r.get('nu_flops', 0)
    return float(tot)


def banded_lu_flops(prob, r, kl=2, ku=2):
    """Context only (declared band; SPD03-like banded direct): the exact-solve arm's counts priced with a banded LU
       (partial pivoting fill ku -> kl+ku): LU 2 n kl (kl+ku+1), solve 2n (2kl+ku+1), analytic J build 2 nnzJ per LU."""
    n = len(prob['y0'])
    return float(r['lu']*(2*n*kl*(kl + ku + 1) + 2*prob['nnzJ']) + r['lu_solves']*2*n*(2*kl + ku + 1)
                 + r['rhs']*prob['frhs'] + 2*n*r['assembly_axpys'] + r['ft']*prob['frhs'])


def err_metrics(y, ref):
    rel = np.abs(y - ref)/np.maximum(np.abs(ref), 1e-10)
    return float(rel.max()), float(np.max(np.abs(y - ref))/np.max(np.abs(ref)))
