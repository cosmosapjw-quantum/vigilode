"""E-side fidelity (EXPLORATORY). No Rust counters exist for PEXPRB54S4 at its own h on these problems, so:
 (a) order screen: fixed-step global error slopes with exact (dense) phi on a stiff non-commuting semilinear problem
     (the Rust g2 screen reports slopes 4.19/4.39/4.92 for |lambda|h 100..6.25) and on a non-autonomous one
     (checks the autonomisation through f_t);
 (b) the earlier Python replica hyp/regime/expo.py (dense phi, own h): Bruss-50 rtol 1e-6 87 acc / 13 rej, err 1.85e-6
     (repro:BC reproduced it) -> my dense-phi arm with the same controller and h0 = 1e-6;
 (c) my KIOPS arm vs my dense-phi arm on the same cell (phi-error contamination)."""
import os, sys, math, json
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import numpy as np, scipy.sparse as sp
import pexprb as PX, pr
out = {}
# (a) order screens with fixed steps (dense phi)
def fixed(prob, h, phi='dense'):
    f, J_of, ft = prob['f'], prob['J'], prob['ft']
    arm = PX.make_arm(phi=phi, tol='fixed', theta=1e-12)
    p = dict(prob); p['h0'] = h
    # huge rtol so every step is accepted; constant h (controller growth capped by span)
    r = PX.integrate(dict(p, ascale=1.0), 1e6, arm, h0=h)
    return r
n = 20
lam = -np.logspace(0, 4, n)
Q, _ = np.linalg.qr(np.random.default_rng(1).standard_normal((n, n)))
Aq = Q @ np.diag(lam) @ Q.T
def mkprob(L, nonaut):
    A = L
    f = (lambda t, y: A @ y + np.sin(y) + (np.cos(3 * t) if nonaut else 0.0))
    J = (lambda t, y: sp.csr_matrix(A + np.diag(np.cos(y))))
    ft = (lambda t, y: -3 * np.sin(3 * t) * np.ones(n)) if nonaut else None
    return dict(f=f, J=J, ft=ft, y0=np.ones(n), span=(0.0, 1.0), ascale=1.0, n=n, F_jvp=1.0, F_rhs=1.0, F_ft=1.0, h0=0.1)
from scipy.integrate import solve_ivp
for nonaut in (False, True):
    pb = mkprob(Aq, nonaut)
    # start on the slow manifold (an initial layer reduces any one-step method to O(h) globally): y(0.5) from ones
    jac = lambda t, y: pb['J'](t, y).toarray()
    y05 = solve_ivp(pb['f'], (0, 0.5), pb['y0'], method='Radau', rtol=1e-13, atol=1e-15, jac=jac).y[:, -1]
    pb = dict(pb, y0=y05)
    ref = solve_ivp(lambda t, y: pb['f'](t + 0.5, y), (0, 1), y05, method='Radau', rtol=1e-13, atol=1e-15,
                    jac=lambda t, y: pb['J'](t + 0.5, y).toarray()).y[:, -1]
    f_sh = pb['f']; J_sh = pb['J']; ft_sh = pb['ft']
    pb = dict(pb, f=lambda t, y: f_sh(t + 0.5, y), J=lambda t, y: J_sh(t + 0.5, y),
              ft=(None if ft_sh is None else (lambda t, y: ft_sh(t + 0.5, y))))
    errs = []
    for N in (4, 8, 16, 32, 64):
        h = 1.0 / N
        # force constant steps: integrate with controller disabled -> emulate by err_rule 'none'
        arm = PX.make_arm(phi='dense', tol='fixed', theta=1e-12, err_rule='time')
        pp = dict(pb, h0=h)
        # use a manual loop: big rtol accepts all, factor capped at 5 -> to keep h fixed, run with span stepping
        y = pb['y0'].copy(); t = 0.0
        for k in range(N):
            q = dict(pb, y0=y, span=(t, t + h), h0=h)
            r = PX.integrate(q, 1e3, arm, h0=h)
            y = r['y']; t += h
        errs.append(float(np.linalg.norm(y - ref)))
    sl = [math.log2(errs[i] / errs[i + 1]) for i in range(len(errs) - 1)]
    print('order screen', 'nonaut' if nonaut else 'aut', ['%.2e' % e for e in errs], 'slopes', ['%.2f' % s for s in sl], flush=True)
    out[f'order_{"nonaut" if nonaut else "aut"}'] = dict(errs=errs, slopes=sl, h=[1 / N for N in (4, 8, 16, 32, 64)], start='y(0.5) on the slow manifold', stiff_ratio_max=float(-lam.min()))
# (b) Bruss-50 vs expo.py
p = pr.build('bruss1d-50')
for phi in ('dense', 'kiops'):
    arm = PX.make_arm(phi=phi, err_rule=('time' if phi == 'dense' else 'max'))
    r = PX.integrate(p, 1e-6, arm, h0=1e-6)
    e = p['err'](r['y'])
    print('bruss50 1e-6', phi, 'acc', r['acc'], 'rej', r['rej'], 'err %.3e' % e, 'Mflop %.3f' % (r['flops']['total'] / 1e6) if phi == 'kiops' else '', r['kc']['ops'] if phi == 'kiops' else '', flush=True)
    out[f'bruss50_{phi}'] = dict(acc=r['acc'], rej=r['rej'], err=e, flops=r['flops'], kc=r['kc'], ec=r['ec'])
out['expo_py_reference'] = dict(acc=87, rej=13, err=1.85e-6, source='verify/repro/v_expo_b50.log')
json.dump(out, open(os.path.join(HERE, 'res/fid_expo.json'), 'w'), indent=1, default=float)
