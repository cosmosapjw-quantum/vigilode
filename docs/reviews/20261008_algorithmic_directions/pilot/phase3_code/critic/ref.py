"""Radau references (rtol 1e-12 and 1e-13; uncertainty = componentwise difference) for the stress problems without
an exact solution. Integrates piecewise across declared switch times (the solver under test is NOT told about them)."""
import json, sys, time, os
import numpy as np
from scipy.integrate import solve_ivp
import sprobs
name = sys.argv[1]
p = sprobs.problem(name)
t0, tf = p['span']
cuts = [t0] + [s for s in p.get('switches', []) if t0 < s < tf] + [tf]
out = {}
for rt in (1e-12, 1e-13):
    at = rt*p['ascale']*1e-4 if name == 'robL' else rt*p['ascale']*1e-2
    y = p['y0'].astype(float).copy(); T0 = time.time(); nfev = 0
    for a, b in zip(cuts[:-1], cuts[1:]):
        mid = 0.5*(a + b)
        if 'switches' in p:
            fm = (lambda t, y, m=mid: p['f'](min(max(t, a), b) if False else t, y))
        sol = solve_ivp(lambda t, y: p['f'](t if (a < t < b) else mid, y), (a, b), y, method='Radau', rtol=rt, atol=at,
                        jac=lambda t, y: p['J'](t, y))
        assert sol.success, sol.message
        y = sol.y[:, -1]; nfev += sol.nfev
    out[rt] = y
    print(name, rt, 'nfev', nfev, f'{time.time()-T0:.1f}s', flush=True)
y12, y13 = out[1e-12], out[1e-13]
unc_cw = float(np.max(np.abs(y12 - y13)/np.maximum(np.abs(y13), 1e-10)))
unc_nw = float(np.max(np.abs(y12 - y13))/np.max(np.abs(y13)))
json.dump(dict(y=y13.tolist(), y12=y12.tolist(), unc=unc_cw, unc_nw=unc_nw), open(os.path.join('refs', f'{name}.json'), 'w'))
print(name, 'unc_cw', unc_cw, 'unc_nw', unc_nw)
