"""References for bruss2d problems (no exact solution): RODAS5P direct arm (driver 'direct', exact sparse LU
stage solves) at rtol = atol = 1e-12 and 1e-13; SciPy Radau (rtol 1e-12, atol 1e-14, sparse J) where affordable.
Writes refs/<problem>.npz with y_r12, y_r13 (if run), y_radau (if run) and a json summary."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, time
import numpy as np
from scipy.integrate import solve_ivp
import driver, probs2d

pname = sys.argv[1]; what = sys.argv[2].split(',')
p = probs2d.build(pname)
os.makedirs('refs', exist_ok=True)
fn = f'refs/{pname}.npz'
store = dict(np.load(fn)) if os.path.exists(fn) else {}
summ = {}
for w in what:
    t0 = time.time()
    if w.startswith('r1'):
        rt = float('1e-' + w[1:])
        q = dict(p); q['ascale'] = 1.0
        r = driver.integrate(q, rt, driver.make_arm('direct'), h0=1e-6, max_att=200000, wall_cap=20000)
        store[f'y_{w}'] = np.array(r['y']); summ[w] = dict(att=r['att'], rej=r['rej'], wall=time.time() - t0)
    elif w == 'radau':
        sol = solve_ivp(p['f'], p['span'], p['y0'], method='Radau', rtol=1e-12, atol=1e-14,
                        jac=lambda t, y: p['J'](t, y).tocsc())
        store['y_radau'] = sol.y[:, -1].copy(); summ[w] = dict(nfev=int(sol.nfev), nlu=int(sol.nlu), status=int(sol.status), wall=time.time() - t0)
    print(pname, w, summ[w], flush=True)
    np.savez(fn, **store)
ys = {k: v for k, v in store.items()}
keys = sorted(ys)
for i in range(len(keys)):
    for j in range(i + 1, len(keys)):
        a, b = ys[keys[i]], ys[keys[j]]
        print(f'  {keys[i]} vs {keys[j]}: max|diff|/max|y| = {np.max(np.abs(a-b))/np.max(np.abs(b)):.3e}', flush=True)
