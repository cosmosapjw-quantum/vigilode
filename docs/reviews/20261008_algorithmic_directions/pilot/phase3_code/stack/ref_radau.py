"""References for the Brusselator cells without a stored reference (probe B1).
scipy.integrate.solve_ivp Radau with a sparse analytic Jacobian at rtol = atol = 1e-12 and 1e-13; the difference
between the two is reported as the reference uncertainty (componentwise, same metric as the error)."""
import json, sys, time, numpy as np, scipy.sparse as sp
from scipy.integrate import solve_ivp
sys.path.insert(0, '.')
import tprobs

N = int(sys.argv[1]); tols = [float(x) for x in sys.argv[2].split(',')]
p = tprobs.bruss(N); n = 2*N; cc = (N + 1.0)**2/50.0


def jac(t, y):
    u, v = y[0::2], y[1::2]; rows = []; cols = []; vals = []
    for i in range(N):
        a, b = 2*i, 2*i + 1
        rows += [a, a, b, b]; cols += [a, b, a, b]; vals += [2*u[i]*v[i] - 4 - 2*cc, u[i]**2, 3 - 2*u[i]*v[i], -u[i]**2 - 2*cc]
        if i > 0: rows += [a, b]; cols += [a - 2, b - 2]; vals += [cc, cc]
        if i + 1 < N: rows += [a, b]; cols += [a + 2, b + 2]; vals += [cc, cc]
    return sp.csc_matrix((vals, (rows, cols)), shape=(n, n))


out = {}
for tol in tols:
    for meth in ('Radau',):
        t0 = time.time()
        s = solve_ivp(p['f'], p['span'], p['y0'], method=meth, rtol=tol, atol=tol, jac=jac, jac_sparsity=None)
        y = s.y[:, -1]
        out[f'{meth}_{tol:g}'] = dict(y=[repr(float(x)) for x in y], nfev=int(s.nfev), njev=int(s.njev), nlu=int(s.nlu),
                                      status=int(s.status), sec=time.time() - t0)
        print(meth, tol, s.status, s.nfev, s.nlu, f'{time.time() - t0:.0f}s', flush=True)
keys = list(out)
if len(keys) >= 2:
    a = np.array([float(x) for x in out[keys[-2]]['y']]); b = np.array([float(x) for x in out[keys[-1]]['y']])
    out['uncertainty_componentwise'] = float(np.max(np.abs(a - b)/np.maximum(np.abs(b), 1e-10)))
    print('uncertainty', out['uncertainty_componentwise'])
json.dump(out, open(f'ref_bruss{N}_radau.json', 'w'), indent=1)
