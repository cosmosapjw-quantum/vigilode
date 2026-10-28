"""Jacobian-drift indicator gD = h ||D_4||_w / ||U_4 - y_n||_w (accepted E steps; free: D_4 is computed by the step)
per family, rtol 1e-6 / 1e-8. EXPLORATORY admission-statistic screen."""
import os, sys, json
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import run_e
out = {}
for pn in ('rot-96', 'semi-96', 'forc-96', 'hires-96', 'rob-96', 'vdp-96', 'bruss1d-50', 'bruss1d-160', 'semi-384', 'advdiff-128',
           'rot-384', 'vdp2', 'hires8'):
    for rt in (1e-4, 1e-6, 1e-8):
        d = run_e.job(pn, 'E2', rt)
        out[f'{pn} {rt:g}'] = {k: d.get(k) for k in ('att', 'gD_med', 'gD_q90', 'gD_mean')}
        print(pn, rt, out[f'{pn} {rt:g}'], flush=True)
json.dump(out, open(os.path.join(HERE, 'res/gd.json'), 'w'), indent=1)
