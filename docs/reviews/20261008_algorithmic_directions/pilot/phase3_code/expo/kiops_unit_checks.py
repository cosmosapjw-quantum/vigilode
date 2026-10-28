"""Unit checks of kiops.py (EXPLORATORY): accuracy vs dense augmented expm, multi-output, substeps, basis reuse,
JAK bound >= actual error on 2-norm-dissipative operators (and its tightness)."""
import os, sys, math, json
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, kiops as KI
rng = np.random.default_rng(7)
res = []
def mk(n, kind, rho):
    if kind == 'sym':
        Q, _ = np.linalg.qr(rng.standard_normal((n, n))); lam = -rho * rng.random(n); return Q @ np.diag(lam) @ Q.T
    if kind == 'advdiff':   # upwind adv-diff (nonnormal, mu_2 <= 0 for this discretisation)
        dx = 1 / (n + 1); D = 0.01; a = 5.0
        A = np.diag(np.full(n, -2 * D / dx**2 - a / dx - 1)) + np.diag(np.full(n - 1, D / dx**2 + a / dx), -1) + np.diag(np.full(n - 1, D / dx**2), 1)
        return A * (rho / np.abs(np.linalg.eigvals(A)).max())
    if kind == 'rot':       # rotated 2x2 nonnormal blocks [[-s, .9 s],[0,-.35 s]] (corpus rotating-nonnormal type)
        A = np.zeros((n, n))
        for b in range(n // 2):
            s = rho * (0.2 + 0.8 * rng.random()); th = 6 * rng.random(); c, sn = math.cos(th), math.sin(th)
            R = np.array([[c, sn], [-sn, c]]); B = R.T @ np.array([[-s, 0.9 * s], [0, -0.35 * s]]) @ R
            A[2*b:2*b+2, 2*b:2*b+2] = B
        return A
    if kind == 'nonnormal_exp':   # mu_2 > 0 (not dissipative): Jordan-like
        A = -rho * np.eye(n) + rho * 3 * np.diag(np.ones(n - 1), 1); return A
for kind in ('sym', 'advdiff', 'rot'):
    for n, rho, h in ((96, 300.0, 0.02), (96, 300.0, 0.2), (200, 2000.0, 0.05)):
        A = mk(n, kind, rho); mu = float(np.linalg.eigvalsh(0.5 * (A + A.T))[-1])
        for p in (1, 2, 4):
            U = [None] + [rng.standard_normal(n) * 10.0 ** rng.uniform(-2, 2) for _ in range(p)]
            taus = [h / 4, h / 2, 0.9 * h, h] if p <= 2 else [h]
            ex = KI.dense_phi(A, taus, U)
            for tol in (1e-4, 1e-8):
                for mmax in (100, 12):
                    cnt = KI.KCnt()
                    outs, inf = KI.kiops(lambda v: A @ v, taus, U, tol, cnt, cert=(mu <= 0), mmax=mmax)
                    errs = [float(np.linalg.norm(o - e)) for o, e in zip(outs, ex)]
                    r = dict(kind=kind, n=n, rho=rho, h=h, p=p, tol=tol, mmax=mmax, mu=mu, err=max(errs), err_sum_est=inf['err_sum'],
                             ops=cnt.ops, substeps=inf['substeps'], jmax=inf['j_max'], dense=cnt.dense_flops, expms=cnt.expms,
                             bound=(max(inf['bounds']) if mu <= 0 else None),
                             bound_viol=(any(b < e * (1 - 1e-6) - 1e-15 for b, e in zip(inf['bounds'], errs)) if mu <= 0 else None),
                             ratio=([b / e if e > 0 else None for b, e in zip(inf['bounds'], errs)] if mu <= 0 else None))
                    res.append(r)
                    print(f"{kind:13s} n={n:3d} rho={rho:6.0f} h={h:5.2f} p={p} tol={tol:.0e} mmax={mmax:3d} mu={mu:9.2e} ops={cnt.ops:4d} sub={inf['substeps']:2d} jmax={inf['j_max']:3d} "
                          f"err={max(errs):.2e} est={inf['err_sum']:.2e} " + (f"bound={max(inf['bounds']):.2e} viol={r['bound_viol']} minratio={min(x for x in r['ratio'] if x is not None):.3f} errs={['%.2e' % e for e in errs]} b={['%.2e' % b for b in inf['bounds']]}" if mu <= 0 else ''), flush=True)
# basis reuse: same start vector, smaller tau -> no new JVPs needed if dims suffice
A = mk(96, 'advdiff', 300.0); U = [None, rng.standard_normal(96), rng.standard_normal(96)]
cache = {}; c1 = KI.KCnt(); o1, _ = KI.kiops(lambda v: A @ v, [0.05], U, 1e-8, c1, basis_cache=cache)
c2 = KI.KCnt(); o2, _ = KI.kiops(lambda v: A @ v, [0.01], U, 1e-8, c2, basis_cache=cache)
e2 = KI.dense_phi(A, [0.01], U)[0]
print('reuse: first ops', c1.ops, 'second ops', c2.ops, 'reused cols', c2.reused_cols, 'err', np.linalg.norm(o2[0] - e2))
res.append(dict(reuse_first_ops=c1.ops, reuse_second_ops=c2.ops, reused=c2.reused_cols, reuse_err=float(np.linalg.norm(o2[0] - e2))))
json.dump(res, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'res/test_kiops.json'), 'w'), indent=0, default=float)
vi = [r for r in res if r.get('bound_viol')]
acc = [r for r in res if 'err' in r and r['err'] > 10 * r['tol'] * max(1, 1)]
print('violations:', len(vi), ' cells with err > 10 tol:', len(acc))
