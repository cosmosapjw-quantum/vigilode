"""Reference for bruss1d-160: direct RODAS5P (sparse LU, exact stages) at rtol = atol = 1e-12 and 1e-13; compared
with hyp/ctrl/ref160.npy (dense RODAS 1e-11) and SciPy Radau (rtol 1e-12, atol 1e-14). EXPLORATORY."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, time, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pr, swdrv
from scipy.integrate import solve_ivp
p = pr.build('bruss1d-160')
store = {}
for rt in (1e-12, 1e-13):
    t0 = time.time(); r = swdrv.integrate(p, rt, swdrv.make_arm('direct'), max_att=400000)
    store[f'y_r{int(round(-np.log10(rt)))}'] = r['y']; print('direct', rt, r['att'], f'{time.time()-t0:.1f}s', flush=True)
t0 = time.time()
s = solve_ivp(p['f'], p['span'], p['y0'], method='Radau', rtol=1e-12, atol=1e-14, jac=lambda t, y: p['J'](t, y).tocsc())
store['y_radau'] = s.y[:, -1]; print('radau', s.status, s.nfev, f'{time.time()-t0:.1f}s', flush=True)
store['y_ref160_old'] = np.load('refs/bruss160_ref.npy')
np.savez('refs/bruss1d-160.npz', **store)
k = sorted(store)
for i in range(len(k)):
    for j in range(i+1, len(k)):
        print(f'{k[i]} vs {k[j]}: maxrel {pr.maxrel(store[k[i]], store[k[j]]):.3e}')
