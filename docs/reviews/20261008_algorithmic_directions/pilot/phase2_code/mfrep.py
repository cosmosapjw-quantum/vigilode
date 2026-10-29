"""Matrix-free U-form RODAS5P replica (rodas5p_matrix_free_fast.rs semantics, I controller of
r5_replica.py) with selectable inner-stopping rules. JVPs = Krylov columns + true-residual matvecs.
Modes:
  base   : production GMRES(40): full cycles, true residual between cycles, final diagnostic residual,
           threshold max(|g|*atol_lin, rtol_lin*||b||_2) in L2 (rtol_lin=1e-10, atol_lin=1e-14).
  proj   : same threshold, stop inside the cycle on the projected residual, one true residual.
  tf     : WRMS-scaled GMRES, per-stage absolute WRMS residual target eps_i (tol units) from
           tableau transfer bounds, projected stop + one true residual.
  rel    : uniform relative forcing eta*||b_i||_2 (Wang-Yu style), projected stop + one true residual.
Diagnostics per attempt: exact stage chain by dense LU -> actual dy (tol units), |err_c - err*|."""
import json, math, sys
import numpy as np
from scipy.linalg import lu_factor, lu_solve
sys.path.insert(0, '..')
d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = float(d['gamma']); A = np.array([[float(x) for x in r] for r in d['A']])
C = np.array([[float(x) for x in r] for r in d['C']]); c = np.array([float(x) for x in d['c']])
bc = np.array([float(x) for x in d['b_code']]); S = 8
Gam = np.linalg.inv(np.eye(S)/g - C); grow = Gam.sum(axis=1)
# sup over Re z <= 0 of |T_y,i|, |T_e,i| (transfer.py)
TY = np.array([1.729, 0.511, 6.246, 6.081, 3.82, 4.771, 1.011, 1.0])
TE = np.array([1.393, 2.057, 2.266, 2.16, 1.51, 1.792, 2.011, 1.0])
import os
MU_LOG = []
TAB = dict(np.load(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'tau_table.npz')))

class Budget(Exception):
    def __init__(self, jvp): self.jvp = jvp

def gmres(Wmv, b, thresh, m=40, maxit=200, full_cycles=False, x0=None, stall_ok=False):
    """Returns x, n_jvp, true_res_norm(L2 of the system given), cols. Wmv counts nothing; caller counts."""
    n = len(b); x = np.zeros(n) if x0 is None else x0.copy(); jvp = 0; cols = 0
    r = b.copy() if x0 is None else b - Wmv(x)
    if x0 is not None: jvp += 1
    rn = np.linalg.norm(r)
    first = True
    while True:
        if rn <= thresh:
            break
        if cols >= maxit:
            if stall_ok: return x, jvp, rn, cols
            raise Budget(jvp)
        k = min(m, maxit - cols, n)
        V = np.zeros((n, k+1)); H = np.zeros((k+1, k)); beta = rn; V[:, 0] = r/beta
        cs = np.zeros(k); sn = np.zeros(k); gv = np.zeros(k+1); gv[0] = beta; used = 0
        for j in range(k):
            w = Wmv(V[:, j]); jvp += 1; cols += 1
            for _ in range(2):
                hh = V[:, :j+1].T @ w; H[:j+1, j] += hh; w -= V[:, :j+1] @ hh
            hn = np.linalg.norm(w); H[j+1, j] = hn
            for i in range(j):
                t = cs[i]*H[i, j] + sn[i]*H[i+1, j]; H[i+1, j] = -sn[i]*H[i, j] + cs[i]*H[i+1, j]; H[i, j] = t
            den = math.hypot(H[j, j], H[j+1, j]); cs[j] = H[j, j]/den; sn[j] = H[j+1, j]/den
            H[j, j] = den; H[j+1, j] = 0.0; gv[j+1] = -sn[j]*gv[j]; gv[j] = cs[j]*gv[j]; used = j+1
            breakdown = hn <= 100*np.finfo(float).eps*np.linalg.norm(H[:j+2, j]) or hn == 0
            if (not full_cycles and abs(gv[j+1]) <= thresh) or breakdown:
                break
            V[:, j+1] = w/hn
        y = np.linalg.solve(np.triu(H[:used, :used]), gv[:used])
        x = x + V[:, :used] @ y
        r = b - Wmv(x); jvp += 1; rn = np.linalg.norm(r)
    if full_cycles:   # production pays a final diagnostic residual on the same x
        jvp += 1
    return x, jvp, rn, cols

def integrate(prob, rtol, mode='base', theta=0.1, eta=1e-6, rtol_lin=1e-10, h0=1e-6, diag=True,
              share=None, err_exp=0.0, u8=None, kappa8=0.1, floor8=2e-5, rho_obs=False, kmin=1, mu_cert=False, uniform=False, stall_ok=False, By_cap=0.05, fixed_h=None, mu_mode="exact", err0=1.0, err_ref=1.0):
    f, Jf, ft, y0, (t0, tf), ascale = prob
    atol = rtol*ascale
    t, y, h = t0, y0.copy(), h0; n = len(y0)
    att = acc = rej = jvp = rhs = cols = 0; last_rej = None; err_prev = err0
    lin_fail = 0; stall_acc = 0; recs = []; dev_y = []; dev_e = []; unresolved = 0; certified_rejects = 0; bnd_viol = 0; cancel = np.zeros(S); ncan = 0
    eps_share = (np.minimum(theta/(8*TY), theta/(8*TE)) if share is None else share)
    if fixed_h: h = fixed_h
    while t < tf - 1e-12:
        if fixed_h: h = min(fixed_h, tf - t)
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        att += 1
        J = Jf(t, y); f0 = f(t, y); ftv = ft(t, y); rhs += 1
        W = np.eye(n) - h*g*J
        sc = atol + rtol*np.abs(y); D = 1.0/sc
        U = np.zeros((S, n)); res_w = np.zeros(S); bnorm_w = np.zeros(S); rho = 1.0; nonfinite = False
        TYa, TEa = TY, TE; fallback = False
        if mu_cert:
            Js = (D[:, None]*J)/D[None, :]; Sy = (Js + Js.T)/2
            if mu_mode == 'exact': mu = np.linalg.eigvalsh(Sy).max()
            else: mu = np.max(np.diag(Sy) + np.sum(np.abs(Sy), axis=1) - np.abs(np.diag(Sy)))   # Gershgorin on D-symmetrized part
            a = max(0.0, h*mu); MU_LOG.append(a)
            if a >= TAB['a'][-1]: fallback = True
            else:
                k = np.searchsorted(TAB['a'], a)   # conservative: use next-larger grid point
                TYa, TEa = TAB['ty'][k], TAB['te'][k]
        if uniform:
            eps_att = np.full(S, theta/TYa.sum())
        else:
            eps_att = np.minimum(theta/(8*TYa), theta/(8*TEa))
        linfail = False
        for i in range(S):
            fi = f0 if i == 0 else f(t + c[i]*h, y + A[i, :i] @ U[:i])
            if i: rhs += 1
            b = h*g*fi + g*(C[i, :i] @ U[:i]) + h*h*g*grow[i]*ftv
            if not np.all(np.isfinite(b)) or np.linalg.norm(D*b) > 1e200:
                nonfinite = True; break
            try:
                if mode == 'base':
                    x, nj, rn, cl = gmres(lambda v: W @ v, b, max(g*1e-14, rtol_lin*np.linalg.norm(b)), full_cycles=True)
                elif mode == 'proj':
                    x, nj, rn, cl = gmres(lambda v: W @ v, b, max(g*1e-14, rtol_lin*np.linalg.norm(b)))
                elif mode == 'rel':
                    x, nj, rn, cl = gmres(lambda v: W @ v, b, max(g*1e-14, eta*np.linalg.norm(b)))
                elif mode == 'tf':
                    # scaled system (D W D^-1)(D x) = D b ; WRMS = ||.||_2/sqrt(n)
                    Ws = (D[:, None]*W)/D[None, :]
                    tgt = eps_att[i]*math.sqrt(n)*(min(1.0, err_prev/err_ref)**err_exp if err_exp else 1.0)
                    if rho_obs: tgt /= rho
                    if u8 == 'floor' and i == S-1:
                        tgt = min(tgt, floor8*math.sqrt(n))
                    if u8 == 'rel' and i == S-1:
                        tgt = max(kappa8*np.linalg.norm(D*b)/rho, floor8*math.sqrt(n))
                    if fallback: tgt = 1e-10*np.linalg.norm(D*b)
                    xs, nj, rn, cl = gmres(lambda v: Ws @ v, D*b, max(tgt, 1e-14*np.linalg.norm(D*b)), stall_ok=stall_ok)
                    x = xs/D
            except Budget as bx:
                jvp += bx.jvp; linfail = True; break
            jvp += nj; cols += cl
            U[i] = x
            r = b - W @ x
            res_w[i] = math.sqrt(np.mean((D*r)**2)); bnorm_w[i] = math.sqrt(np.mean((D*b)**2))
            uw_i = math.sqrt(np.mean((D*x)**2))
            if bnorm_w[i] > 0: rho = max(rho, uw_i/bnorm_w[i])
        if linfail:
            lin_fail += 1; rej += 1; last_rej = h; h *= 0.2; continue
        if nonfinite:
            rej += 1; last_rej = h; h *= 0.2; recs.append(dict(h=h, err=np.inf, errx=np.nan, dy=np.nan, de=np.nan, By=np.nan, Be=np.nan, viol=False, ratio=None, res=res_w, bw=bnorm_w, rho=rho)); continue
        ynew = y + bc @ U
        sc2 = atol + rtol*np.maximum(np.abs(y), np.abs(ynew))
        err = math.sqrt(np.mean((U[-1]/sc2)**2))
        if not math.isfinite(err): err = np.inf
        uw = np.sqrt(np.mean((U*D[None, :])**2, axis=1))
        cancel += np.log10(np.maximum(bnorm_w, 1e-300)/np.maximum(uw, 1e-300)); ncan += 1
        B_e = float(TEa @ res_w); B_y = float(TYa @ res_w)
        if diag:
            lu = lu_factor(W); Ux = np.zeros((S, n))
            for i in range(S):
                fi = f0 if i == 0 else f(t + c[i]*h, y + A[i, :i] @ Ux[:i])
                Ux[i] = lu_solve(lu, h*g*fi + g*(C[i, :i] @ Ux[:i]) + h*h*g*grow[i]*ftv)
            yx = y + bc @ Ux
            sx = atol + rtol*np.maximum(np.abs(y), np.abs(yx))
            errx = math.sqrt(np.mean((Ux[-1]/sx)**2))
            dy = math.sqrt(np.mean(((ynew - yx)/sc)**2))
            dev_y.append(dy); dev_e.append(abs(err - errx))
            v = (abs(err - errx) > B_e*1.0000001 + 1e-15 or dy > B_y*1.0000001 + 1e-15)
            if mode == 'tf' and v:
                bnd_viol += 1
            recs.append(dict(h=h, err=err, errx=errx, dy=dy, de=abs(err-errx), By=B_y, Be=B_e, viol=bool(v),
                             ratio=np.log10(np.maximum(bnorm_w,1e-300)/np.maximum(np.sqrt(np.mean((U*D[None,:])**2,axis=1)),1e-300)),
                             res=res_w.copy(), bw=bnorm_w.copy(), rho=rho, uw=np.sqrt(np.mean((U*D[None,:])**2,axis=1))))
        if u8 == 'rel': B_e = float(TE[:-1] @ res_w[:-1]) + res_w[-1]
        if err <= 1.0 and mode == 'tf' and err + B_e > 1.0:
            unresolved += 1
        if mode == 'tf' and err - B_e > 1.0:
            certified_rejects += 1
        if stall_ok and err <= 1.0 and (err + B_e > 1.0 or B_y > By_cap):
            err = 1.0 + 1e-9   # certified acceptance failed -> reject
        if fixed_h: err_for_ctrl = err; err = 0.0 if math.isfinite(err) else err
        if err <= 1.0:
            if fixed_h: err = err_for_ctrl
            acc += 1; t += h; y = ynew; last_rej = None; err_prev = err
            fac = 5.0 if err == 0 else min(max(0.9*err**(-0.2), 0.2), 5.0)
        else:
            rej += 1; last_rej = h
            fac = min(max(0.9*max(err, 1e-16)**(-0.2), 0.2), 0.9) if math.isfinite(err) else 0.2
        if not fixed_h: h *= fac
        if att > 20000: raise RuntimeError('attempt cap')
    return dict(att=att, acc=acc, rej=rej, jvp=jvp, cols=cols, rhs=rhs, y=y,
                dev_y_max=float(np.nanmax(dev_y)) if dev_y else None, dev_y_med=float(np.nanmedian(dev_y)) if dev_y else None,
                dev_e_max=float(np.nanmax(dev_e)) if dev_e else None, unresolved=unresolved, cert_rej=certified_rejects,
                lin_fail=lin_fail, bnd_viol=bnd_viol, cancel=cancel/max(ncan, 1), recs=recs)
