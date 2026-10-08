import os, sys, math
sys.path.insert(0, '.'); sys.path.insert(0, 'rodas')
import numpy as np, scipy.sparse as sp, pexprb as PX
from scipy.integrate import solve_ivp
arm = PX.make_arm(phi='dense', tol='fixed', theta=1e-12, err_rule='time')
for lamv in (-1.0, -100.0):
    n = 1
    f = lambda t, y: lamv * y + np.sin(y); J = lambda t, y: sp.csr_matrix(np.array([[lamv + math.cos(y[0])]]))
    pb = dict(f=f, J=J, ft=None, y0=np.ones(n), span=(0.0, 1.0), ascale=1.0, n=n, F_jvp=1.0, F_rhs=1.0, F_ft=1.0, h0=0.1)
    s = solve_ivp(f, (0, 0.5), pb['y0'], method='Radau', rtol=1e-13, atol=1e-16)
    for h in (0.5, 0.25, 0.125):
        r = PX.integrate(dict(pb, span=(0.0, h)), 1e3, arm, h0=h)
        s = solve_ivp(f, (0, h), pb['y0'], method='DOP853', rtol=1e-14, atol=1e-16)
        print(lamv, h, r['att'], abs(r['y'][0] - s.y[0, -1]))
