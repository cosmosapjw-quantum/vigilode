"""PEXPRB54S4 (Luan & Ostermann 2016; tableau /home/user/wt-speed/crates/rodas5p-integrators/src/exponential.rs:907-930)
at its OWN step size with its own controller.  PROBE B3 (EXPLORATORY pilot, not ledger authority).

Non-autonomous problems are autonomised exactly: with Y = (y, t), F = (f, 1), J_A = [[J, f_t], [0, 0]],
    tau phi_1(tau J_A) F(Y) = (tau phi_1(tau J) f0 + tau^2 phi_2(tau J) f_t, tau),
    D_i = f(t + c_i h, U_i) - f0 - J (U_i - y) - c_i h f_t      (tau-component of D_i is 0),
so every phi action is on J itself (KIOPS augmentation carries f_t as the u_2 column).  Equivalent to running the
Rust step (validate_problem: autonomous only, exponential.rs:2048-2064) on the autonomised system.

Step (one Krylov basis per distinct start vector; multi-output KIOPS calls):
  call A  u = [0, f0, f_t]  at tau = h/4, h/2, 9h/10, h   -> U2 - y, phi-part of U3, U4, base = h phi1 f0 + h^2 phi2 f_t
  call B  u = [0, 0, 0, 32 D2 / h^2] at tau = h/2, 9h/10   (a32 / c3^3 = a42 / c4^3 = 32 exactly)
  call C  u3 = (b33 D3 + b43 D4) / h^2, u4 = (b34 D3 + b44 D4) / h^3 at tau = h       -> y_new = y + base + C
  call D  the same with (b - b_hat) incl. D2                                         -> error vector (embedded order 4)
  Remainders: 3 RHS + 3 JVPs.  f0, f_t, J are reused after a rejection; so is call A's first-substep basis
  (Krylov space independent of tau: unpreconditioned, same start vector).
Controller: I controller of the RODAS5P replica / Rust adaptive_exponential.rs (order 5: 0.9 err^(-1/5), clamps [0.2, 5]
accepted, [0.2, 0.9] rejected, last-rejected-h cap), err = max(time_err, phi_err) as adaptive_exponential.rs:290.
phi_err = sum of accepted-substep KIOPS estimates of calls A-C in WRMS units (scaled arm: /sqrt(n); 2-norm arm:
/(sqrt(n) min w), the F-043 conversion).
Phi tolerance per attempt (WRMS units): 'epus' (default) theta_n = Theta (h/T) min(1, e_hat/e_ref)^p (the A1 coupled
target rule, Theta 0.2, e_ref 0.5, p 6/5, e0 1e-6) or 'fixed' theta_n = theta.
"""
import math
import os
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import kiops as KI

T = dict(c2=0.25, c3=0.5, c4=0.9, a32=4.0, a42=2916 / 125, b33=18.0, b34=-60.0, b43=-250 / 81, b44=500 / 27,
         e23=64.0, e24=-60.0, e33=-8.0, e34=-285 / 8, e43=0.0, e44=125 / 8)
assert abs(T['a32'] / T['c3'] ** 3 - 32) < 1e-12 and abs(T['a42'] / T['c4'] ** 3 - 32) < 1e-12


class ECnt:
    KEYS = ('jvp_rem', 'rhs', 'ft', 'vec', 'jbuild', 'mu_checks', 'fail')

    def __init__(self):
        for k in self.KEYS:
            setattr(self, k, 0)

    def d(self):
        return {k: getattr(self, k) for k in self.KEYS}


def make_arm(**kw):
    a = dict(phi='kiops', norm='scaled', tol='epus', Theta=0.2, e_ref=0.5, p=1.2, e0=1e-6, theta=0.02,
             mmin=4, mmax=100, cert=False, Q=128, check_actual=False, reuse=True, warm_m=True, err_rule='max')
    a.update(kw)
    return a


def flops(prob, kc, ec, n_ops_scaled):
    """Admitted flop parts for an E run. kc: KIOPS counters dict, ec: ECnt dict. Per operator application F_jvp
    (+2n for the scaled form); the remainder JVPs F_jvp each."""
    n = prob['n']; Fj = prob['F_jvp']; Fr = prob['F_rhs']; Fft = prob['F_ft']
    N = n  # vector length for orth ~ n + p (p <= 4): charged at n + 2
    parts = dict(
        jvp=kc['ops'] * Fj + n_ops_scaled * 2 * n + ec['jvp_rem'] * Fj,
        aug=float(kc['aug_flops']),
        orth=2 * (N + 2) * (kc['dots'] + kc['axpys'] + kc['norms']) + (N + 2) * kc['scal'],
        out=float(kc['out_flops']),
        dense=float(kc['dense_flops']),
        rhs=ec['rhs'] * Fr + ec['ft'] * Fft,
        vec=2 * n * ec['vec'])
    parts['total'] = float(sum(parts.values()))
    parts['cert'] = float(kc['cert_flops'])
    return parts


def integrate(prob, rtol, arm, h0=None, max_att=60000, wall_cap=3000.0, log_steps=False, single=False):
    f, J_of, ft = prob['f'], prob['J'], prob['ft']
    y0 = prob['y0']; t0, tf = prob['span']; span = tf - t0
    n = len(y0); atol = rtol * prob['ascale']
    h = prob['h0'] if h0 is None else h0
    kc = KI.KCnt(); ec = ECnt()
    t, y = t0, np.array(y0, dtype=float)
    acc = rej = 0; last_rej = None; err_prev = None
    cached = False
    m_warm = {'A': 10, 'B': 10, 'C': 10, 'D': 10}
    wt0 = time.time()
    scaled = arm['norm'] == 'scaled'
    n_ops_scaled = 0
    steps = []
    cert_rows = []
    mu_log = []
    gD = []
    nonaut = ft is not None
    while t < tf - 1e-12 * max(1.0, abs(tf)):
        h = min(h, tf - t)
        if last_rej is not None and h >= last_rej:
            h = np.nextafter(t + last_rej, -np.inf) - t
        if not cached:
            J = J_of(t, y); ec.jbuild += 1
            f0 = f(t, y); ec.rhs += 1
            ftv = ft(t, y) if nonaut else None
            if nonaut: ec.ft += 1
            sc = atol + rtol * np.abs(y)
            Dw = 1.0 / sc
            cacheA = {} if arm['reuse'] else None
            cached = True
            mu = None
            if arm['cert'] or arm['check_actual']:
                Jd = J.toarray() if hasattr(J, 'toarray') else np.asarray(J)
                As = (Dw[:, None] * Jd) / Dw[None, :] if scaled else Jd
                mu = float(np.linalg.eigvalsh(0.5 * (As + As.T))[-1]); ec.mu_checks += 1
                mu2 = float(np.linalg.eigvalsh(0.5 * (Jd + Jd.T))[-1]) if scaled else mu
                mus = float(prob['mu_struct'](t, y)) if prob.get('mu_struct') is not None else None
                mu_log.append((t, mu, mu2, mus))
        # operator in the space used
        if scaled:
            def Aop(v):
                return Dw * (J @ (v / Dw))
            S = lambda x: Dw * x
            Si = lambda x: x / Dw
            wfac = math.sqrt(n)
        else:
            def Aop(v):
                return J @ v
            S = Si = (lambda x: x)
            wfac = math.sqrt(n) * float(np.min(sc))
        # phi tolerance in WRMS units
        if arm['tol'] == 'epus':
            e = arm['e0'] if err_prev is None else err_prev
            theta = arm['Theta'] * (h / span) * min(1.0, max(e, 0.0) / arm['e_ref']) ** arm['p']
        else:
            theta = arm['theta']
        theta = max(theta, 1e-14)
        tolK = theta * wfac
        ops0 = kc.ops
        info = {}
        dense = arm['phi'] == 'dense'
        if dense or arm['check_actual'] or arm['cert']:
            Ad = (J.toarray() if hasattr(J, 'toarray') else np.asarray(J))
            if scaled:
                Ad = (Dw[:, None] * Ad) / Dw[None, :]

        def call(key, U, taus, cache=None):
            Us = [None if u is None else S(u) for u in U]
            if dense:
                outs = KI.dense_phi(Ad, taus, Us)
                inf = dict(err_sum=0.0, bounds=[0.0] * len(taus), m=0, j_max=0, substeps=0)
            else:
                outs, inf = KI.kiops(Aop, taus, Us, tolK, kc, m_init=m_warm[key] if arm['warm_m'] else 10,
                                     mmin=arm['mmin'], mmax=arm['mmax'], cert=arm['cert'], Q=arm['Q'], basis_cache=cache)
                if arm['warm_m']:
                    m_warm[key] = max(arm['mmin'], inf['m'])
                if arm['check_actual'] or arm['cert']:
                    ex = KI.dense_phi(Ad, taus, Us)
                    for k in range(len(taus)):
                        act = float(np.linalg.norm(outs[k] - ex[k]))
                        cert_rows.append(dict(t=t, h=h, call=key, k=k, tau=float(taus[k]), actual=act,
                                              bound=float(inf['bounds'][k]) if arm['cert'] else None,
                                              est=float(inf['err_sum']), tol=tolK, mu=mu, norm=float(np.linalg.norm(ex[k])),
                                              jmax=inf['j_max'], substeps=inf['substeps']))
            info[key] = inf
            return [Si(o) for o in outs]

        c2, c3, c4 = T['c2'], T['c3'], T['c4']
        try:
            ynew, oD0, D4n, U4d = _stages(call, f0, ftv, nonaut, h, t, y, f, J, ec, c2, c3, c4, cacheA)
        except (FloatingPointError, OverflowError, ValueError, ZeroDivisionError, np.linalg.LinAlgError):
            rej += 1; last_rej = h; h *= 0.2; ec.fail += 1
            if scaled:
                n_ops_scaled += kc.ops - ops0
            if acc + rej > max_att:
                raise RuntimeError(f'attempt cap at t={t}')
            continue
        oD = [oD0]
        if scaled:
            n_ops_scaled += kc.ops - ops0
        ok = np.all(np.isfinite(ynew)) and np.all(np.isfinite(oD[0]))
        sc2 = atol + rtol * np.maximum(np.abs(y), np.abs(ynew))
        tim = math.sqrt(np.mean((oD[0] / sc2) ** 2)) if ok else np.inf
        ec.vec += 2
        phi_err = sum(info[k]['err_sum'] for k in ('A', 'B', 'C')) / wfac
        err = max(tim, phi_err) if arm['err_rule'] == 'max' else tim
        if not math.isfinite(err):
            err = np.inf
        if single:     # one speculative attempt (shadow use): no state update
            kd = kc.d(); ed = ec.d()
            return dict(err=err, tim=tim, phi_err=phi_err, y=ynew, h=h, flops=flops(prob, kd, ed, n_ops_scaled), kc=kd, ec=ed)
        if log_steps:
            steps.append((t, h, tim, phi_err, err <= 1.0, {k: (info[k]['j_max'], info[k]['substeps']) for k in info}, theta))
        if err <= 1.0:
            dn = math.sqrt(np.mean((U4d / sc) ** 2))
            if dn > 0:
                gD.append(h * math.sqrt(np.mean((D4n / sc) ** 2)) / dn)
            acc += 1; t = t + h; y = ynew; last_rej = None; cached = False; err_prev = err
            fac = 5.0 if err == 0 else min(max(0.9 * err ** (-0.2), 0.2), 5.0)
            h *= fac
        else:
            rej += 1; last_rej = h
            fac = min(max(0.9 * max(err, 1e-16) ** (-0.2), 0.2), 0.9) if math.isfinite(err) else 0.2
            h *= fac
        if acc + rej > max_att:
            raise RuntimeError(f'attempt cap at t={t}')
        if time.time() - wt0 > wall_cap:
            raise RuntimeError(f'wall cap at t={t} att={acc + rej}')
    kd = kc.d(); ed = ec.d()
    out = dict(att=acc + rej, acc=acc, rej=rej, y=y, t=t, kc=kd, ec=ed, n_ops_scaled=n_ops_scaled,
               flops=flops(prob, kd, ed, n_ops_scaled), wall=time.time() - wt0)
    if log_steps:
        out['steps'] = steps
    if cert_rows:
        out['cert_rows'] = cert_rows
    if mu_log:
        out['mu_log'] = mu_log
    if gD:
        out['gD_med'] = float(np.median(gD)); out['gD_q90'] = float(np.quantile(gD, 0.9)); out['gD_mean'] = float(np.mean(gD))
    return out


def _stages(call, f0, ftv, nonaut, h, t, y, f, J, ec, c2, c3, c4, cacheA):
    """One PEXPRB54S4 stage sweep: returns (y_new, error vector, ||D4||, U4 - y)."""
    with np.errstate(over='raise', invalid='raise'):
        oA = call('A', [None, f0, ftv] if nonaut else [None, f0], [c2 * h, c3 * h, c4 * h, h], cacheA)
        def rem(U, c):
            r = f(t + c * h, U) - f0 - J @ (U - y)
            ec.rhs += 1; ec.jvp_rem += 1; ec.vec += 3
            if nonaut:
                r = r - (c * h) * ftv; ec.vec += 1
            return r
        U2 = y + oA[0]; ec.vec += 1
        D2 = rem(U2, c2)
        oB = call('B', [None, None, None, 32.0 * D2 / h ** 2], [c3 * h, c4 * h])
        U3 = y + oA[1] + oB[0]; U4 = y + oA[2] + oB[1]; ec.vec += 4
        D3 = rem(U3, c3); D4 = rem(U4, c4)
        u3 = (T['b33'] * D3 + T['b43'] * D4) / h ** 2; u4 = (T['b34'] * D3 + T['b44'] * D4) / h ** 3; ec.vec += 4
        oC = call('C', [None, None, None, u3, u4], [h])
        d3 = ((T['b33'] - T['e33']) * D3 + (T['b43'] - T['e43']) * D4 - T['e23'] * D2) / h ** 2
        d4 = ((T['b34'] - T['e34']) * D3 + (T['b44'] - T['e44']) * D4 - T['e24'] * D2) / h ** 3; ec.vec += 6
        oD = call('D', [None, None, None, d3, d4], [h])
        ynew = y + oA[3] + oC[0]; ec.vec += 2
        return ynew, oD[0], D4, U4 - y
