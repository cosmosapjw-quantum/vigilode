import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/dense_runs.jsonl')
SEEDS = ['1e-06', '1e-04', '1e-02', 'auto']
p = 'rotnn96'
arms = sorted(set(k[1] for k in D if k[0] == p))
print("rotating-nonnormal n=96 (corpus v2 calibration family, [0,1], atol=0.01 rtol, rtol 1e-4..1e-8 quarter-decade, 4 h0 seeds)")
for a in arms:
    rows = [r for s in SEEDS for r in D[(p, a, s)]]
    fire = sum(r['fire'] for r in rows); acc = sum(r['acc'] for r in rows)
    per = [r['fire'] / r['acc'] for r in rows if r['acc']]
    print(f"  {a:14s} att {sum(r['att'] for r in rows):6d} rej {sum(r['rej'] for r in rows):4d} ({100*sum(r['rej'] for r in rows)/sum(r['att'] for r in rows):.2f}%) "
          f"gate fires {fire}/{acc} accepted = {100*fire/max(acc,1):.1f}% (per-run min/median/max {100*min(per):.0f}/{100*np.median(per):.0f}/{100*max(per):.0f}%) "
          f"err range {min(r['err'] for r in rows):.1e}..{max(r['err'] for r in rows):.1e}")
EG = [1e-5, 1e-6, 1e-7, 1e-8, 1e-9]
for ref, alist in (('I', ['I725', 'PRED', 'PRED+cap', 'PRED725', 'PRED+gate', 'I+gate']), ('PRED+cap', ['PRED+cap+gate', 'PRED+cap80', 'PRED725'])):
    print(f"-- vs {ref}")
    for a in alist:
        sc = score(D, p, a, ref, 'att', EG, SEEDS)
        v = [sc[E]['R'][0] for E in EG if sc[E]['R'] and not sc[E]['x']]
        print(f"  {a:14s}" + ''.join(f" |{E:.0e} R{fmt(sc[E]['R'], 'x' if sc[E]['x'] else '')} C{fmt(sc[E]['C'])}" for E in EG)
              + f"  gm(R,in-range)={np.exp(np.mean(np.log(v))) if v else float('nan'):.3f}")
