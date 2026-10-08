"""References: Bruss-160 (no stored reference) by SciPy Radau rtol 1e-12/1e-13 with sparse J; HIRES/Robertson
checked against NATIVE.json with Radau rtol 1e-13 (tight-rtol cells 1e-9..1e-10 need ref error << 1e-11)."""
import sys, json, time
import numpy as np
from scipy.integrate import solve_ivp
from scipy.sparse import csr_matrix
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *
out = {}
def rel(a, b): return float(np.max(np.abs(a - b) / np.maximum(np.abs(b), 1e-10)))
which = sys.argv[1]
if which == 'bruss160':
    p = bruss(160)
    sols = {}
    for rt in (1e-12, 1e-13):
        t0 = time.time()
        s = solve_ivp(p['f'], p['span'], p['y0'], method='Radau', jac=lambda t, y: csr_matrix(p['J'](t, y)), rtol=rt, atol=rt * 1e-2)
        sols[rt] = s.y[:, -1]; print('bruss160 Radau', rt, s.status, s.nfev, f'{time.time()-t0:.0f}s', flush=True)
    t0 = time.time()
    s = solve_ivp(p['f'], p['span'], p['y0'], method='BDF', jac=lambda t, y: csr_matrix(p['J'](t, y)), rtol=1e-12, atol=1e-14)
    print('bruss160 BDF 1e-12', s.status, f'{time.time()-t0:.0f}s', flush=True)
    unc = rel(sols[1e-12], sols[1e-13]); dis = rel(s.y[:, -1], sols[1e-13])
    old = np.load('/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/hyp/ctrl/ref160.npy')
    print('bruss160 unc(1e-12 vs 1e-13)', unc, 'BDF disagreement', dis, 'vs old ref160.npy (dense replica 1e-11)', rel(old, sols[1e-13]))
    np.save(f'{HERE}/ref_bruss160.npy', sols[1e-13])
    out = dict(unc=unc, bdf=dis, old=rel(old, sols[1e-13]))
else:
    p = PROBS[which]()
    t0 = time.time()
    sols = {}
    for rt in (1e-12, 1e-13):
        s = solve_ivp(p['f'], p['span'], p['y0'], method='Radau', jac=p['J'], rtol=rt, atol=rt * p['ascale'] * 1e-3)
        sols[rt] = s.y[:, -1]
    print(which, 'Radau unc', rel(sols[1e-12], sols[1e-13]), 'NATIVE vs Radau1e-13', rel(p['ref'], sols[1e-13]), f'{time.time()-t0:.0f}s')
    out = dict(unc=rel(sols[1e-12], sols[1e-13]), native=rel(p['ref'], sols[1e-13]))
json.dump(out, open(f'{HERE}/ref_{which}.json', 'w'))
