"""Distributional gate for chaotic (discontinuous-forcing) problems: pool the 3 h0 seeds x rtol ladder per arm family.
Reports per family the median / geometric mean / p90 / max of err (tol units or cw/rtol) and per-cell paired ratios."""
import math, os, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import cana
SEEDS = ('', 's5', 's4')
for pn in sys.argv[1:]:
    d = cana.load(pn); syn = cana.SYN(pn)
    print(f'== {pn} ({"WRMS tol units" if syn else "cw rel/rtol"}; pooled over h0 seeds 1e-6/1e-5/1e-4 x rtol)')
    fam = {}
    for (a, r), o in d.items():
        if not cana.ok(o): continue
        base = a[:-2] if a[-2:] in ('s5', 's4') else a
        fam.setdefault(base, []).append(cana.metric(o, syn)/r)
    for a in ('A0', 'S', 'C3G', 'LU', 'LUP'):
        v = np.array(fam.get(a, []))
        if len(v):
            print(f'  {a:4s} n={len(v):2d} median {np.median(v):.3g} gmean {math.exp(np.mean(np.log(v))):.3g} p90 {np.percentile(v, 90):.3g} max {v.max():.3g}')
    for num, den in (('S', 'A0'), ('S', 'LUP'), ('C3G', 'LU'), ('LUP', 'LU'), ('LU', 'A0')):
        rat = []; jr = []
        for sfx in SEEDS:
            for (a, r), o in d.items():
                if a != num + sfx or not cana.ok(o): continue
                q = d.get((den + sfx, r))
                if cana.ok(q):
                    rat.append(cana.metric(o, syn)/cana.metric(q, syn))
                    if q['jvp']: jr.append(o['jvp']/q['jvp'])
        if rat:
            rat = np.array(rat)
            s = f'  {num}/{den}: cells {len(rat)} gmean {math.exp(np.mean(np.log(rat))):.2f} median {np.median(rat):.2f} >1.5x in {int(np.sum(rat > 1.5))} max {rat.max():.2f} min {rat.min():.3f}'
            if jr: s += f' | JVP ratio gmean {math.exp(np.mean(np.log(jr))):.2f} [{min(jr):.2f}-{max(jr):.2f}]'
            print(s)
