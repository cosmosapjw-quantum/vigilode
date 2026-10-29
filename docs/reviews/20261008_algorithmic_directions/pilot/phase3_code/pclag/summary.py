"""Compact summary for the report: matched-accuracy ratios (frontier R / cheapest C) vs direct at E grid; errors at
equal rtol vs direct (tight cells); per-N columns. EXPLORATORY."""
import sys, os, math, json
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze import load, arm_rows, frontier, cheapest
import scaling
PROBS = ['semilin2d-32x48', 'semilin2d-32', 'semilin2d-64', 'semilin2d-128', 'semilin2d-256', 'semilin2d-64-s4',
         'bruss2d-32', 'bruss2d-64', 'bruss2d-128', 'bruss2d-256', 'bruss2d-64-a0.1', 'bruss2d-128-a0.1']
ES = [1e-4, 1e-6, 1e-8, 1e-10]
for w in ('F', 'T'):
    print(f'\n### matched accuracy vs direct, work={w}: R(frontier; x=extrapolated)/C(cheapest-run) at E')
    print(f"{'problem':17s} {'arm':4s} " + ' '.join(f'{"E="+format(E,".0e"):>16s}' for E in ES))
    for pn in PROBS:
        rows = load(pn)
        if not rows: continue
        for o in rows.values():
            if 'error' not in o: o['T'] = scaling.time_model(pn, o)
        unc = [o.get('ref_unc') for o in rows.values() if o.get('ref_unc')]
        minerr = 30 * max(unc) if unc else 0.0
        rd = arm_rows(rows, 'direct', minerr)
        for arm in ('mf', 'lag', 'lin', 'dst'):
            ra = arm_rows(rows, arm, minerr)
            if len(ra) < 2: continue
            if w == 'T' and any(o.get('T') is None for o in ra + rd): continue
            cells = []
            for E in ES:
                fa, xa = frontier(ra, E, w); fd, xd = frontier(rd, E, w)
                ca, cd = cheapest(ra, E, w), cheapest(rd, E, w)
                lo = min(min(o['err_rel'] for o in ra), min(o['err_rel'] for o in rd)); hi = max(max(o['err_rel'] for o in ra), max(o['err_rel'] for o in rd))
                if E < lo / 3 or E > hi * 3: cells.append(f'{"-":>16s}'); continue
                cells.append(f"{fa/fd:6.3f}{'x' if (xa or xd) else ' '}/{(ca/cd) if ca and cd else float('nan'):6.3f}  ")
            print(f'{pn:17s} {arm:4s} ' + ' '.join(f'{c:>16s}' for c in cells))
print('\n### error at equal rtol relative to direct (err_arm/err_direct), tight and loose cells')
for pn in PROBS:
    rows = load(pn)
    for rt in (1e-4, 1e-8, 1e-9, 1e-10):
        d = rows.get((rt, 'direct'))
        if not d: continue
        s = ' '.join(f"{a}:{rows[(rt, a)]['err_rel']/d['err_rel']:.3f}" for a in ('mf', 'lag', 'lin', 'dst') if (rt, a) in rows)
        extra = ''
        if 'err_cw' in d:
            extra = ' | err_cw ratio ' + ' '.join(f"{a}:{rows[(rt, a)]['err_cw']/d['err_cw']:.3f}" for a in ('mf', 'lag', 'lin') if (rt, a) in rows and 'err_cw' in rows[(rt, a)])
        print(f'{pn:17s} rtol {rt:<6g} direct err {d["err_rel"]:.3e}  {s}{extra}')
