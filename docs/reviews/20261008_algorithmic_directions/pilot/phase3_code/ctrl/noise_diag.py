"""Noise in the controller's error estimate from inexact stage solves: per attempt, err (inexact chain, what the
controller sees) vs errx (exact LU chain at the same t, y, h). Reports quantiles of |log(err/errx)| and the
implied step-size noise for I (1/k) and PRED (min form, up to 2/k on the predictive branch)."""
import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *
from run_mf import MODES
res = {}
for name, rtols in (('bruss50', (1e-4, 1e-6)), ('hires', (1e-4, 1e-6, 1e-8, 1e-10)), ('bruss160', (1e-6,))):
    p = PROBS[name]()
    for rtol in rtols:
        for mode in ('proj', 'tf1', 'tf2', 'rel4'):
            for arm in ('I', 'PRED'):
                r = mf_integrate(p, rtol, make_ctrl(arm), diag=True, maxatt=6000, **MODES[mode])
                L = np.array([abs(math.log(e / ex)) for (_, _, e, ex) in r['rec'] if np.isfinite(e) and e > 0 and ex > 0])
                # near the accept boundary only (0.2 < errx < 5): where noise can flip accept/reject decisions
                Lb = np.array([abs(math.log(e / ex)) for (_, _, e, ex) in r['rec'] if np.isfinite(e) and 0.2 < ex < 5 and e > 0])
                flips = sum(1 for (_, _, e, ex) in r['rec'] if np.isfinite(e) and (e <= 1) != (ex <= 1))
                q = np.percentile(L, [50, 90, 99]) if len(L) else [np.nan] * 3
                key = f'{name}|{rtol:g}|{mode}|{arm}'
                res[key] = dict(att=r['att'], rej=r['rej'], jvp=r['jvp'], err=r['err'], q50=q[0], q90=q[1], q99=q[2],
                                qmax=float(L.max()) if len(L) else None, boundary_q90=float(np.percentile(Lb, 90)) if len(Lb) else None,
                                flips=flips)
                print(f"{key:28s} att={r['att']:4d} rej={r['rej']:3d} JVP={r['jvp']:7d} e/rtol={r['err']/rtol:7.3f} "
                      f"|log(err/errx)| q50={q[0]:.1e} q90={q[1]:.1e} q99={q[2]:.1e} max={res[key]['qmax']:.1e} "
                      f"boundary q90={res[key]['boundary_q90']} accept/reject flips={flips}", flush=True)
json.dump(res, open(f'{HERE}/noise_diag.json', 'w'), indent=1)
