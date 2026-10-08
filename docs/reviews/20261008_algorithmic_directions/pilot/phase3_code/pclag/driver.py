"""PROBE B4 driver: closed-loop adaptive RODAS5P (U form, I controller of hyp/mfrep.py / probe/target/rep.py)
on sparse 2-D problems, with five stage-solve arms and full admitted-cost counters. EXPLORATORY.

Arms (arm['kind']):
  direct : sparse LU of W = I - h g J every attempt (J from colored JVPs, rebuilt only on a new step, i.e.
           reused after a rejection as in rodas5p-fast), 8 exact solves.
  mf     : matrix-free U-form GMRES(40), Zero start, projected in-cycle stop + one true residual, outer-coupled
           WRMS absolute stage target (probe A1 coupled_target.py defaults: EPUS Theta 0.2, e_ref 0.5, p 6/5,
           U8 cap, RHO_FL guard, RHO_STALL stall rule), scaled form (GMRES on D W D^-1), maxit 2000.
  lag    : mf + RIGHT preconditioner P = I - h_P g J_P, J_P = J(t_n, y_n) from colored JVPs at a refresh,
           sparse LU; reused while rho = h/h_P in [lo, hi] and the previous attempt's max stage column count
           <= mmax (refresh otherwise, before the stage solves). Operator exact (JVP of the current J).
  lin    : mf + RIGHT preconditioner P = I - h_P g A(t_P), A = declared linear part, sparse LU; refreshed when
           rho leaves [lo, hi] (and, only if A is time dependent, when previous max cols > mmax).
  dst    : mf + RIGHT preconditioner P = I - h g A at the CURRENT h through a fast DST-I solver (constant-
           coefficient Dirichlet Laplacian part only; no factorization, no window).
  base / proj : SPD07 production / proj-stop L2 targets, no PC (fidelity check vs BASE.json).
All Krylov exits are confirmed by one true residual of the unpreconditioned (scaled) system b - W x.
"""
import json, math, os, sys, time
import numpy as np
import scipy.sparse as sp

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import coupled_target as CT
from lucost import SparseLU

d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = float(d['gamma']); A = np.array([[float(x) for x in r] for r in d['A']])
C = np.array([[float(x) for x in r] for r in d['C']]); c = np.array([float(x) for x in d['c']])
bc = np.array([float(x) for x in d['b_code']]); S = 8
Gam = np.linalg.inv(np.eye(S) / g - C); grow = Gam.sum(axis=1)
EPS = np.finfo(float).eps


class Budget(Exception):
    pass


class Cnt:
    KEYS = ('jvp', 'cjvp', 'pcs', 'cols', 'tres', 'diag', 'cycles', 'dots', 'axpys', 'scal', 'norms', 'rhs', 'ft',
            'solves', 'stalls', 'lus', 'lu_flops', 'lu_solve_flops', 'pc_flops', 'jbuilds', 'wassem',
            'ref_rho', 'ref_iter', 'ref_new', 'maxcols', 'roundoff_bind')
    def __init__(self):
        for k in self.KEYS: setattr(self, k, 0)
    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def gmres(Op, b, thr, cnt, m=40, maxit=2000, full_cycles=False, ortho='cgs2', stall=None):
    """Zero-start restarted GMRES on operator Op (Op counts its own JVP / PC applications).
    Returns (z, cols). In-cycle projected stop at thr unless full_cycles; acceptance on the TRUE residual."""
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
            if bn is None: bn = np.linalg.norm(b)
            scale = bn + np.linalg.norm(x) + np.linalg.norm(x - Wx); cnt.norms += 2
            if rn <= stall * scale:
                cnt.stalls += 1
                break
        rn_prev = rn
        if total >= maxit:
            raise Budget()
        k = min(m, maxit - total, n)
        V = np.zeros((k + 1, n)); H = np.zeros((k + 1, k)); beta = rn; V[0] = r / beta; cnt.scal += 1
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k + 1); gv[0] = beta; used = 0
        for j in range(k):
            w = Op(V[j]); cnt.cols += 1
            hcol = np.zeros(j + 1)
            if ortho == 'mgs2':
                for _ in range(2):
                    for i in range(j + 1):
                        hh = V[i] @ w; hcol[i] += hh; w -= hh * V[i]
            else:
                for _ in range(2):
                    hh = V[:j + 1] @ w; hcol += hh; w -= hh @ V[:j + 1]
            cnt.dots += 2 * (j + 1); cnt.axpys += 2 * (j + 1)
            hn = np.linalg.norm(w); cnt.norms += 1
            H[:j + 1, j] = hcol; H[j + 1, j] = hn
            col_scale = math.hypot(np.linalg.norm(hcol), hn)
            breakdown = (col_scale == 0.0) or (hn <= 100 * EPS * col_scale)
            for i in range(j):
                t = cs[i] * H[i, j] + sn[i] * H[i + 1, j]; H[i + 1, j] = -sn[i] * H[i, j] + cs[i] * H[i + 1, j]; H[i, j] = t
            den = math.hypot(H[j, j], H[j + 1, j]); cs[j] = H[j, j] / den; sn[j] = H[j + 1, j] / den
            H[j, j] = den; H[j + 1, j] = 0.0; gv[j + 1] = -sn[j] * gv[j]; gv[j] = cs[j] * gv[j]; used = j + 1
            if breakdown or (not full_cycles and abs(gv[j + 1]) <= thr):
                break
            V[j + 1] = w / hn; cnt.scal += 1
        y = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + y @ V[:used]; cnt.axpys += used + 1
        total += used; cnt.cycles += 1
    if full_cycles:
        Op(x); cnt.diag += 1; cnt.axpys += 1; cnt.norms += 1
    return x, total


def make_arm(kind, **kw):
    a = dict(kind=kind, m=40, maxit=2000, ortho='cgs2', lo=0.5, hi=2.0, mmax=10, iter_trigger=True,
             ordering='mmd_at_plus_a', rtol_lin=1e-10, atol_lin=1e-14, stall=CT.RHO_STALL, Theta=0.2)
    a.update(kw)
    return a


def integrate(prob, rtol, arm, h0=None, max_att=40000, wall_cap=3000.0, verbose=False):
    f, ft, y0 = prob['f'], prob['ft'], prob['y0']
    t0, tf = prob['span']; span = tf - t0
    atol = rtol * prob['ascale']; n = len(y0)
    if h0 is None: h0 = prob.get('h0', 1e-6)
    t, y, h = t0, y0.astype(float).copy(), h0
    cnt = Cnt(); att = acc = rej = lin_fail = 0; last_rej = None; err_prev = None
    cached = False; asm_axpys = 0
    kind = arm['kind']
    tgt = CT.Target(Theta=arm['Theta']) if kind in ('mf', 'lag', 'lin', 'dst') else None
    I = sp.identity(n, format='csr')
    P = None; hP = None; prev_maxcols = 0; tP = None
    colstat = []      # per-attempt (h, rho, mean cols per stage, max cols)
    wt0 = time.time()
    Fdirect = None
    while t < tf - 1e-12 * max(1.0, abs(tf)):
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        att += 1
        if not cached:
            J = prob['J'](t, y); f0 = f(t, y); cnt.rhs += 1
            ftv = ft(t, y) if ft is not None else None
            if ft is not None: cnt.ft += 1
            cached = True; newJ = True
            if kind == 'direct':
                cnt.cjvp += prob['colors']; cnt.jbuilds += 1
        hg = h * g
        Wmv = lambda v: v - hg * (J @ v)
        sc = atol + rtol * np.abs(y); D = 1.0 / sc
        U = np.zeros((S, n))
        rho_now = None
        # ---------------- per-attempt setup ----------------
        if kind == 'direct':
            W = (I - hg * J).tocsr(); cnt.wassem += 1
            Fdirect = SparseLU(W, arm['ordering']); cnt.lus += 1; cnt.lu_flops += Fdirect.fflops
        elif kind in ('lag', 'lin'):
            rho = None if hP is None else h / hP
            reason = None
            if P is None: reason = 'new'
            elif not (arm['lo'] <= rho <= arm['hi']): reason = 'rho'
            elif arm['iter_trigger'] and prev_maxcols > arm['mmax'] and (kind == 'lag' or not prob['alin_const']):
                reason = 'iter'
            if reason is not None:
                if kind == 'lag':
                    JP = J; cnt.cjvp += prob['colors']; cnt.jbuilds += 1
                else:
                    JP = prob['Alin'](t)
                M = (I - hg * JP).tocsr(); cnt.wassem += 1
                P = SparseLU(M, arm['ordering']); cnt.lus += 1; cnt.lu_flops += P.fflops
                hP = h; tP = t
                setattr(cnt, {'new': 'ref_new', 'rho': 'ref_rho', 'iter': 'ref_iter'}[reason],
                        getattr(cnt, {'new': 'ref_new', 'rho': 'ref_rho', 'iter': 'ref_iter'}[reason]) + 1)
            rho_now = h / hP
        eps_vec = tgt.stage_eps(h=h, span=span, err_hat=err_prev, n=n) if tgt is not None else None
        linfail = False; nonfinite = False; maxcols = 0; sumcols = 0
        for i in range(S):
            if i == 0:
                fi = f0
            else:
                fi = f(t + c[i] * h, y + A[i, :i] @ U[:i]); cnt.rhs += 1
            b = hg * fi + g * (C[i, :i] @ U[:i])
            if ftv is not None: b = b + h * hg * grow[i] * ftv
            asm_axpys += 2 * i + 2
            if not np.all(np.isfinite(b)):
                nonfinite = True; break
            cnt.solves += 1
            try:
                if kind == 'direct':
                    x = Fdirect.solve(b); cnt.lu_solve_flops += Fdirect.sflops
                    cols = 0
                elif kind in ('base', 'proj'):
                    def Op(v):
                        cnt.jvp += 1; return Wmv(v)
                    thr = max(g * arm['atol_lin'], arm['rtol_lin'] * np.linalg.norm(b))
                    x, cols = gmres(Op, b, thr, cnt, m=arm['m'], maxit=arm['maxit'], full_cycles=(kind == 'base'),
                                    ortho=arm['ortho'])
                else:
                    Db = D * b; Dbn = np.linalg.norm(Db)
                    thr, rb = tgt.l2_threshold(eps_vec[i], Dbn, n)
                    if rb: cnt.roundoff_bind += 1
                    cache = {}
                    if kind == 'mf':
                        def pre(z):
                            return z / D
                    elif kind in ('lag', 'lin'):
                        def pre(z, P=P):
                            cnt.pcs += 1; cnt.pc_flops += P.sflops
                            return P.solve(z / D)
                    elif kind == 'dst':
                        def pre(z):
                            cnt.pcs += 1; cnt.pc_flops += prob['dst'].flops
                            return prob['dst'].solve(z / D, hg)
                    def Op(z):
                        xx = pre(z); cache['z'] = z; cache['x'] = xx
                        cnt.jvp += 1
                        return D * Wmv(xx)
                    z, cols = gmres(Op, Db, thr, cnt, m=arm['m'], maxit=arm['maxit'], ortho=arm['ortho'], stall=arm['stall'])
                    if 'z' in cache and cache['z'] is not None and np.array_equal(cache['z'], z):
                        x = cache['x']
                    elif not np.any(z):
                        x = np.zeros(n)
                    else:
                        x = pre(z)        # (not reached for converged cycles; counted if it is)
            except Budget:
                linfail = True; break
            if not np.all(np.isfinite(x)):
                nonfinite = True; break
            U[i] = x
            maxcols = max(maxcols, cols); sumcols += cols
        prev_maxcols = maxcols
        cnt.maxcols = max(cnt.maxcols, maxcols)
        if kind not in ('direct',):
            colstat.append((h, rho_now, sumcols / S, maxcols))
        if linfail:
            lin_fail += 1; rej += 1; last_rej = h; h *= 0.2; prev_maxcols = 10 ** 9
            continue
        if nonfinite:
            rej += 1; last_rej = h; h *= 0.2; continue
        ynew = y + bc @ U
        sc2 = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1] / sc2) ** 2))
        if not math.isfinite(err): err = np.inf
        if err <= 1.0:
            acc += 1; t += h; y = ynew; last_rej = None; cached = False; err_prev = err
            fac = 5.0 if err == 0 else min(max(0.9 * err ** (-0.2), 0.2), 5.0)
            h *= fac
        else:
            rej += 1; last_rej = h
            fac = min(max(0.9 * max(err, 1e-16) ** (-0.2), 0.2), 0.9) if math.isfinite(err) else 0.2
            h *= fac
        if att > max_att: raise RuntimeError(f'attempt cap at t={t}')
        if time.time() - wt0 > wall_cap: raise RuntimeError(f'wall cap at t={t} att={att}')
        if verbose and att % 50 == 0:
            print(f'   att {att} t={t:.4g} h={h:.3e} jvp={cnt.jvp} lus={cnt.lus}', flush=True)
    out = dict(att=att, acc=acc, rej=rej, lin_fail=lin_fail, y=y.tolist(), t=t, asm_axpys=asm_axpys,
               wall=time.time() - wt0)
    out.update(cnt.d())
    if colstat:
        cs_ = np.array([(a, (b if b is not None else np.nan), m_, x_) for a, b, m_, x_ in colstat], dtype=float)
        out['mean_cols_stage'] = float(np.nanmean(cs_[:, 2])); out['p95_maxcols'] = float(np.percentile(cs_[:, 3], 95))
        out['colstat'] = cs_.tolist()
    return out


def flops(prob, r, kind, jvp_model='sparse'):
    """Admitted flop total (cost model in README):
       Krylov operator application (JVP) : F_op = 2 nnzJ + 2n (shift v - hg Jv) + 2n (WRMS scaling D^-1, D)
       colored JVP (J build)             : F_cj = 2 nnzJ                 [jvp_model 'fd': frhs + 2n for both]
       PC application                    : LU solve flops 2 nnz(L+U) + n (exact per factor), DST model
       LU factorization                  : sum_k (l_k + 2 l_k u_k) of the produced factors
       W / P assembly                    : nnzJ + n per assembly
       dot/axpy/norm 2n, scal n, RHS frhs, f_t fft, stage-RHS assembly axpys 2n."""
    n = prob['n']; nnz = prob['nnzJ']
    if jvp_model == 'sparse':
        Fop = 2 * nnz + 4 * n; Fcj = 2 * nnz
    else:
        Fop = prob['frhs'] + 4 * n; Fcj = prob['frhs'] + 2 * n
    if kind in ('base', 'proj'): Fop -= 2 * n
    parts = dict(
        jvp=r['jvp'] * Fop, cjvp=r['cjvp'] * Fcj, pc=r['pc_flops'], lu=r['lu_flops'], lusolve=r['lu_solve_flops'],
        orth=2 * n * (r['dots'] + r['axpys'] + r['norms']) + n * r['scal'], rhs=r['rhs'] * prob['frhs'] + r['ft'] * prob['fft'],
        asm=2 * n * r['asm_axpys'] + r['wassem'] * (nnz + n))
    parts['total'] = float(sum(parts.values()))
    return parts
