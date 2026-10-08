"""Tight endpoint references for the corpus families without an exact solution (rob/hires/vdp/forc, n = 96):
direct RODAS5P (exact stages) at rtol 1e-12 and 1e-13 (atol = 0.01 rtol) and SciPy Radau rtol 1e-13 atol 1e-16,
compared with the stored selected_raw endpoint (tight-WRMS). EXPLORATORY."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, json, time, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pr, swdrv
from scipy.integrate import solve_ivp
out = {}
for nm in ('rob-96', 'hires-96', 'vdp-96', 'forc-96'):
    p = pr.build(nm); st = {}
    for rt in (1e-12, 1e-13):
        t0 = time.time(); r = swdrv.integrate(p, rt, swdrv.make_arm('direct'), max_att=500000)
        st[f'r{int(round(-np.log10(rt)))}'] = np.asarray(r['y']); print(nm, rt, r['att'], f'{time.time()-t0:.1f}s', flush=True)
    t0 = time.time()
    s = solve_ivp(p['f'], p['span'], p['y0'], method='Radau', rtol=1e-13, atol=1e-16, jac=lambda t, y: p['J'](t, y).tocsc())
    st['radau'] = s.y[:, -1]; print(nm, 'radau', s.status, f'{time.time()-t0:.1f}s', flush=True)
    st['stored'] = p['ref']
    cmp = {f'{a}~{b}': pr.tight_wrms(st[a], st[b]) for a in st for b in st if a < b}
    print(nm, {k: f'{v:.2e}' for k, v in cmp.items()}, flush=True)
    out[nm] = dict(cmp=cmp)
    np.savez(f'refs/{nm}-endpoint.npz', **st)
json.dump(out, open('res/corpus_refs.json', 'w'), indent=1)
