"""Set-point x controller factorial (dense, attempts=LU): {I, PRED} x safety {0.9, 0.85, 0.80, 0.725, 0.65}.
Geometric mean over in-range E of the median-over-seeds frontier ratio (R) and cheapest-run ratio (C), vs I."""
import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/dense_runs.jsonl')
SEEDS = ['1e-06', '1e-04', '1e-02', 'auto']
EG = {'vdp': [1e-3, 3e-4, 1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7], 'bruss50': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7],
      'hires': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9, 3e-10, 1e-10],
      'robertson': [3e-5, 1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9, 3e-10]}
GRID = {'I': ['I', 'I85', 'I80', 'I725', 'I65'], 'PRED': ['PRED', 'PRED+cap85', 'PRED80', 'PRED725', 'PRED65']}
SAF = [0.9, 0.85, 0.80, 0.725, 0.65]
out = {}
def gm(sc, rule, Es):
    v = [sc[E][rule][0] for E in Es if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']]
    return float(np.exp(np.mean(np.log(v)))) if v else float('nan'), len(v)
def worst(sc, rule, Es):
    v = [sc[E][rule][0] for E in Es if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']]
    return max(v) if v else float('nan')
for p in ('vdp', 'hires', 'bruss50', 'robertson'):
    print(f"== {p}: gm R / gm C / worst-E R  (vs I, half-decade in-range E grid)")
    for kind in ('I', 'PRED'):
        line = f"  {kind:5s}"
        for s, a in zip(SAF, GRID[kind]):
            if a == 'I': line += f" | s={s}: 1.000/1.000/1.000"; continue
            sc = score(D, p, a, 'I', 'att', EG[p], SEEDS)
            r, n = gm(sc, 'R', EG[p]); cc, m = gm(sc, 'C', EG[p]); w = worst(sc, 'R', EG[p])
            line += f" | s={s}: {r:.3f}/{cc:.3f}/{w:.3f}"
            out[f'{p}|{a}'] = dict(R=r, C=cc, worstR=w, nR=n, nC=m)
        print(line)
json.dump(out, open(f'{HERE}/factorial.json', 'w'), indent=1)
