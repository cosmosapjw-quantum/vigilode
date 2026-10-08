import sys, math; sys.path.insert(0, '.')
import numpy as np, kiops as KI
rng = np.random.default_rng(7)
n = 96; rho = 300.0; h = 0.2
Q, _ = np.linalg.qr(rng.standard_normal((n, n))); lam = -rho * rng.random(n); A = Q @ np.diag(lam) @ Q.T
for p in (1, 2):
    U = [None] + [rng.standard_normal(n) * 10.0 ** rng.uniform(-2, 2) for _ in range(p)]
    taus = [h / 4, h / 2, 0.9 * h, h]
    ex = KI.dense_phi(A, taus, U)
    for Qd in (128, 1024):
        cnt = KI.KCnt()
        outs, inf = KI.kiops(lambda v: A @ v, taus, U, 1e-4, cnt, cert=True, Q=Qd)
        print(p, Qd, [f'{np.linalg.norm(o-e):.3e}/{b:.3e}' for o, e, b in zip(outs, ex, inf['bounds'])], inf['j_max'], inf['err_sum'])
    # single-output runs per tau
    for t in taus:
        cnt = KI.KCnt(); o, inf = KI.kiops(lambda v: A @ v, [t], U, 1e-4, cnt, cert=True)
        print('  single', t, f'{np.linalg.norm(o[0]-KI.dense_phi(A,[t],U)[0]):.3e}', f"{inf['bounds'][0]:.3e}", inf['j_max'])
