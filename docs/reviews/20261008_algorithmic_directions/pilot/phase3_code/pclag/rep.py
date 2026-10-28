"""Probe A1 replica: matrix-free U-form RODAS5P (rodas5p_matrix_free_fast.rs semantics; Zero start,
GMRES(40), maxit 200, I controller of r5_replica.py / hyp/mfrep.py) with full admitted-cost counters
and selectable stage targets. Derived from hyp/mfrep.py and verify/judge/mfrep_j.py (copied; originals
untouched). Differences vs mfrep: (1) two-pass MODIFIED Gram-Schmidt (Rust kernels.rs:51-78) by
default instead of CGS2 (fixes the HIRES column-count fidelity); (2) counters for dots/axpys/scalings/
true residuals/diagnostic residuals/RHS/f_t; (3) f(t,y), f_t reused after a rejection (Rust
state_reuses) for the RHS count; (4) target arms below.

Arms (arm dict 'kind'):
  base  : full cycles, between-cycle true residual, final diagnostic residual (the duplicate);
          L2 threshold max(|g|*atol_lin, rtol_lin*||b||_2), rtol_lin=1e-10, atol_lin=1e-14 (production).
  proj  : same L2 threshold, in-cycle projected (Givens) stop, one true residual; no duplicate.
  l2c   : proj with the judge's L2 coupling: rtol_lin=min(1e-10, cr*rtol), atol_lin = ca*atol_outer.
  wabs  : proj with the outer-coupled WRMS absolute target of coupled_target.py; Krylov form
          'scaled' (GMRES on D W D^-1, projected residual == WRMS) or 'l2test' (L2 GMRES, in-cycle
          stop on ||r||_2 <= eps*sqrt(n)/max(D), true WRMS test, continue on failure).
JVP count = Krylov columns + true residuals (+ diagnostic residual for base) [Rust jvp_vectors]."""
import json, math, os, sys
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
TY = CT.TAU_Y; TE = CT.TAU_E


class Budget(Exception):
    pass


class Cnt:
    KEYS = ('jvp', 'cols', 'tres', 'diag', 'cycles', 'dots', 'axpys', 'scal', 'norms', 'rhs', 'ft', 'solves',
            'zero_solves', 'floor_bind', 'roundoff_bind', 'fail_checks', 'stalls', 'nonfinite')
    def __init__(self):
        for k in self.KEYS: setattr(self, k, 0)
    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def gmres(Wmv, b, thr, cnt, m=40, maxit=200, full_cycles=False, ortho='mgs2', true_ok=None, thr_proj=None,
          stall=None):
    """Zero-start restarted GMRES (Rust gmres_into loop). thr: acceptance threshold on the L2 norm of the
    true residual of the system given (unless true_ok is given: a callable r -> bool for the acceptance test).
    thr_proj: in-cycle projected-residual threshold (default thr); None-stop when full_cycles.
    Returns x. Raises Budget after maxit columns. Counts into cnt."""
    n = len(b); x = np.zeros(n); total = 0
    first = True; rn_prev = None; bn = None
    if thr_proj is None: thr_proj = thr
    while True:
        if first:
            r = b.copy(); first = False; Wx = None
        else:
            Wx = Wmv(x); r = b - Wx; cnt.jvp += 1; cnt.tres += 1; cnt.axpys += 1
        rn = np.linalg.norm(r); cnt.norms += 1
        if not math.isfinite(rn):
            raise Budget()
        ok = (true_ok(r) if true_ok is not None else rn <= thr)
        if ok:
            break
        # attainable-accuracy stagnation: a full cycle that did not reduce the true residual 4x, at a
        # residual within rho_stall of the backward-error scale ||b|| + ||x|| + ||x - W x|| -> accept (counted)
        if stall is not None and Wx is not None and rn_prev is not None and rn > 0.25*rn_prev:
            if bn is None: bn = np.linalg.norm(b)
            scale = bn + np.linalg.norm(x) + np.linalg.norm(x - Wx); cnt.norms += 2
            if rn <= stall*scale:
                cnt.stalls += 1
                break
        rn_prev = rn
        if total > 0 and not full_cycles:
            cnt.fail_checks += 1
        if total >= maxit:
            raise Budget()
        k = min(m, maxit - total, n)
        V = np.zeros((n, k+1)); H = np.zeros((k+1, k)); beta = rn; V[:, 0] = r/beta; cnt.scal += 1
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k+1); gv[0] = beta; used = 0
        for j in range(k):
            w = Wmv(V[:, j]); cnt.jvp += 1; cnt.cols += 1
            hcol = np.zeros(j+1)
            if ortho == 'mgs2':
                for _ in range(2):
                    for i in range(j+1):
                        hh = V[:, i] @ w; hcol[i] += hh; w -= hh*V[:, i]
            else:
                for _ in range(2):
                    hh = V[:, :j+1].T @ w; hcol += hh; w -= V[:, :j+1] @ hh
            cnt.dots += 2*(j+1); cnt.axpys += 2*(j+1)
            hn = np.linalg.norm(w); cnt.norms += 1
            H[:j+1, j] = hcol; H[j+1, j] = hn
            col_scale = math.hypot(np.linalg.norm(hcol), hn)
            breakdown = (col_scale == 0.0) or (hn <= 100*EPS*col_scale)
            for i in range(j):
                t = cs[i]*H[i, j] + sn[i]*H[i+1, j]; H[i+1, j] = -sn[i]*H[i, j] + cs[i]*H[i+1, j]; H[i, j] = t
            den = math.hypot(H[j, j], H[j+1, j]); cs[j] = H[j, j]/den; sn[j] = H[j+1, j]/den
            H[j, j] = den; H[j+1, j] = 0.0; gv[j+1] = -sn[j]*gv[j]; gv[j] = cs[j]*gv[j]; used = j+1
            if breakdown or (not full_cycles and abs(gv[j+1]) <= thr_proj):
                break
            V[:, j+1] = w/hn; cnt.scal += 1
        y = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + V[:, :used] @ y; cnt.axpys += used + 1
        total += used; cnt.cycles += 1
    if full_cycles:      # production's final diagnostic residual on the same x (the duplicate)
        cnt.jvp += 1; cnt.diag += 1; cnt.axpys += 1; cnt.norms += 1
    return x


def make_arm(kind='base', **kw):
    a = dict(kind=kind, rtol_lin=1e-10, atol_lin=1e-14, m=40, maxit=200, ortho='mgs2', form='scaled',
             cr=1e-3, ca=1e-3, rho_fl=None, stall=None)
    a.update(kw)
    return a


def integrate(prob, rtol, arm, h0=1e-6, diag=False, fixed_h=None, max_att=60000, record=False, tgt=None):
    """Closed-loop adaptive (or fixed-step) U-form integration. tgt: coupled_target.Target for 'wabs'."""
    f, ft, y0 = prob['f'], prob['ft'], prob['y0']
    t0, tf = prob['span']; span = tf - t0
    atol = rtol*prob['ascale']; n = len(y0)
    t, y, h = t0, y0.astype(float).copy(), h0
    cnt = Cnt(); att = acc = rej = lin_fail = 0; last_rej = None
    err_prev = None
    cached = False; recs = []; stage_assembly_axpys = 0
    if fixed_h: h = fixed_h
    kind = arm['kind']
    while t < tf - 1e-12*max(1.0, abs(tf)):
        if fixed_h: h = fixed_h
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        att += 1
        if not cached:
            J = prob['J'](t, y); f0 = f(t, y); cnt.rhs += 1
            ftv = ft(t, y) if ft is not None else np.zeros(n)
            if ft is not None: cnt.ft += 1
            cached = True
        hg = h*g
        Wmv = lambda v: v - hg*(J @ v)
        sc = atol + rtol*np.abs(y); D = 1.0/sc
        U = np.zeros((S, n)); res_w = np.zeros(S); bw = np.zeros(S)
        eps_vec = tgt.stage_eps(h=h, span=span, err_hat=err_prev, n=n) if kind == 'wabs' else None
        if kind == 'lu':
            LUf = lu_factor(np.eye(n) - hg*J)
        linfail = False; nonfinite = False
        for i in range(S):
            if i == 0:
                fi = f0
            else:
                fi = f(t + c[i]*h, y + A[i, :i] @ U[:i]); cnt.rhs += 1
            b = hg*fi + g*(C[i, :i] @ U[:i]) + h*hg*grow[i]*ftv
            stage_assembly_axpys += 2*i + 2
            if not np.all(np.isfinite(b)):
                nonfinite = True; break
            cnt.solves += 1
            try:
                if kind == 'lu':
                    x = lu_solve(LUf, b)
                elif kind == 'base':
                    thr = max(g*arm['atol_lin'], arm['rtol_lin']*np.linalg.norm(b))
                    x = gmres(Wmv, b, thr, cnt, m=arm['m'], maxit=arm['maxit'], full_cycles=True, ortho=arm['ortho'])
                elif kind in ('proj', 'l2c'):
                    if kind == 'proj':
                        rl, al = arm['rtol_lin'], arm['atol_lin']
                    else:
                        rl, al = min(1e-10, arm['cr']*rtol), arm['ca']*atol
                    bn = np.linalg.norm(b)
                    if g*al > rl*bn: cnt.floor_bind += 1
                    thr = max(g*al, rl*bn)
                    x = gmres(Wmv, b, thr, cnt, m=arm['m'], maxit=arm['maxit'], ortho=arm['ortho'])
                elif kind == 'wabs':
                    Db = D*b; Dbn = np.linalg.norm(Db)
                    eps = eps_vec[i]
                    thr, rb = tgt.l2_threshold(eps, Dbn, n)
                    if rb: cnt.roundoff_bind += 1
                    if arm['form'] == 'scaled':
                        Ws = lambda v: D*Wmv(v/D)
                        xs = gmres(Ws, Db, thr, cnt, m=arm['m'], maxit=arm['maxit'], ortho=arm['ortho'], stall=arm['stall'])
                        x = xs/D
                        cnt.scal += 2*0  # scaling flops are charged per JVP in the flop model
                    else:
                        tp = thr/np.max(D) if arm['form'] == 'l2test' else thr/math.sqrt(np.mean(D*D))
                        x = gmres(Wmv, b, None, cnt, m=arm['m'], maxit=arm['maxit'], ortho=arm['ortho'],
                                  true_ok=lambda r: np.linalg.norm(D*r) <= thr, thr_proj=tp, stall=arm['stall'])
                else:
                    raise ValueError(kind)
            except Budget:
                linfail = True; break
            if not np.all(np.isfinite(x)):
                cnt.nonfinite += 1; nonfinite = True; break
            if np.all(x == 0): cnt.zero_solves += 1
            U[i] = x
            if record or diag:
                r = b - Wmv(x)
                res_w[i] = math.sqrt(np.mean((D*r)**2)); bw[i] = math.sqrt(np.mean((D*b)**2))
        if linfail:
            lin_fail += 1; rej += 1; last_rej = h; h *= 0.2
            if fixed_h: raise RuntimeError('linear failure in fixed-step run')
            continue
        if nonfinite:
            rej += 1; last_rej = h; h *= 0.2; continue
        ynew = y + bc @ U
        sc2 = atol + rtol*np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1]/sc2)**2))
        if not math.isfinite(err): err = np.inf
        rec = None
        if diag:
            lu = lu_factor(np.eye(n) - hg*J); Ux = np.zeros((S, n))
            for i in range(S):
                fi = f0 if i == 0 else f(t + c[i]*h, y + A[i, :i] @ Ux[:i])
                Ux[i] = lu_solve(lu, hg*fi + g*(C[i, :i] @ Ux[:i]) + h*hg*grow[i]*ftv)
            yx = y + bc @ Ux
            sx = atol + rtol*np.maximum(np.abs(y), np.abs(yx))
            errx = math.sqrt(np.mean((Ux[-1]/sx)**2))
            dyc = (ynew - yx)/sc          # per-component contamination in tol units
            rec = dict(t=t, h=h, err=err, errx=errx, dy=float(math.sqrt(np.mean(dyc**2))), dyc=dyc,
                       By=float(TY @ res_w), res=res_w.copy(), bw=bw.copy())
        elif record:
            rec = dict(t=t, h=h, err=err, res=res_w.copy(), bw=bw.copy(), By=float(TY @ res_w))
        if rec is not None: recs.append(rec)
        acc_ok = err <= 1.0 or fixed_h
        if acc_ok:
            acc += 1; t += h; y = ynew; last_rej = None; cached = False
            if fixed_h:
                err_prev = err
            else:
                err_prev = err
                fac = 5.0 if err == 0 else min(max(0.9*err**(-0.2), 0.2), 5.0)
                h *= fac
        else:
            rej += 1; last_rej = h
            fac = min(max(0.9*max(err, 1e-16)**(-0.2), 0.2), 0.9) if math.isfinite(err) else 0.2
            h *= fac
        if att > max_att: raise RuntimeError(f'attempt cap at t={t}')
    out = dict(att=att, acc=acc, rej=rej, lin_fail=lin_fail, y=y, t=t, assembly_axpys=stage_assembly_axpys)
    out.update(cnt.d())
    if recs: out['recs'] = recs
    return out


def flops(prob, r, arm_kind, form='scaled', jvp_model='analytic'):
    """Admitted flop total with explicit cost model (see README):
       F_jvp(analytic) = 2*nnzJ + 2n (shift W v = v - hg Jv); F_jvp(fd) = frhs + 4n.
       scaled WRMS form adds 2n multiplies per operator application (D^-1 v, D w).
       dot = 2n, axpy = 2n, norm = 2n, scal = n; stage assembly axpys 2n each; RHS = frhs."""
    n = len(prob['y0'])
    Fj = (2*prob['nnzJ'] + 2*n) if jvp_model == 'analytic' else (prob['frhs'] + 4*n + 2*n)
    if arm_kind == 'wabs' and form == 'scaled': Fj += 2*n
    tot = r['jvp']*Fj + 2*n*(r['dots'] + r['axpys'] + r['norms']) + n*r['scal'] + r['rhs']*prob['frhs'] \
        + 2*n*r['assembly_axpys'] + r['ft']*prob['frhs']
    return float(tot)


def err_metrics(y, ref):
    """(componentwise max rel with 1e-10 floor [stiff_benchmark_scipy.py:156], normwise rel [rnext relative_error], argmax)"""
    rel = np.abs(y - ref)/np.maximum(np.abs(ref), 1e-10)
    nw = float(np.max(np.abs(y - ref))/np.max(np.abs(ref)))
    return float(rel.max()), nw, int(rel.argmax())
