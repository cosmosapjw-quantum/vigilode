import os, sys, math
sys.path.insert(0, '.'); sys.path.insert(0, 'rodas')
import numpy as np, scipy.sparse as sp, pexprb as PX
from scipy.integrate import solve_ivp
n = 20; lam = -np.logspace(0, 4, n)
Q, _ = np.linalg.qr(np.random.default_rng(1).standard_normal((n, n))); A = Q @ np.diag(lam) @ Q.T
f = lambda t, y: A @ y + np.sin(y); J = lambda t, y: sp.csr_matrix(A + np.diag(np.cos(y)))
pb = dict(f=f, J=J, ft=None, y0=np.ones(n), span=(0.0, 1.0), ascale=1.0, n=n, F_jvp=1.0, F_rhs=1.0, F_ft=1.0, h0=0.1)
s = solve_ivp(f, (0, 1), pb['y0'], method='Radau', rtol=1e-13, atol=1e-15, jac=lambda t, y: J(t, y).toarray())
print('radau status', s.status, s.t[-1])
s2 = solve_ivp(f, (0, 1), pb['y0'], method='BDF', rtol=1e-12, atol=1e-14, jac=lambda t, y: J(t, y).toarray())
print('bdf vs radau', np.linalg.norm(s.y[:, -1] - s2.y[:, -1]))
arm = PX.make_arm(phi='dense', tol='fixed', theta=1e-12, err_rule='time')
for N in (8, 16, 32, 64):
    h = 1.0 / N; y = pb['y0'].copy(); t = 0.0; atts = 0
    for k in range(N):
        r = PX.integrate(dict(pb, y0=y, span=(t, t + h)), 1e3, arm, h0=h); y = r['y']; t += h; atts += r['att']
    print(N, atts, np.linalg.norm(y - s.y[:, -1]))
# smaller span: errors at t = 0.1
