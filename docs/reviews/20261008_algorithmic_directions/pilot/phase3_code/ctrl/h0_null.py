"""CTRL-NULLS (g) initial step: per arm, attempts with h0 seed s vs h0 = 1e-6 (the harness default) at equal rtol
(ladder-band quantiles) and at matched accuracy (frontier R and cheapest-run C, seed-vs-seed). Also (d) early-abort
ceiling = (8-k)/8 x rejection fraction for an abort at stage k (k>=2 for any prefix feature)."""
import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/dense_runs.jsonl')
EG = {'vdp': [1e-3, 1e-4, 1e-5, 1e-6], 'bruss50': [1e-5, 1e-6], 'hires': [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9],
      'robertson': [1e-5, 1e-6, 1e-7, 1e-8, 1e-9]}
out = {}
for p in ('vdp', 'hires', 'bruss50', 'robertson'):
    for arm in ('I', 'PRED'):
        base = {round(r['rtol'], 18): r for r in D[(p, arm, '1e-06')]}
        line = f"{p:9s} {arm:5s}"
        for s in ('1e-04', '1e-02', 'auto'):
            rows = D[(p, arm, s)]
            rat = [r['att'] / base[round(r['rtol'], 18)]['att'] for r in rows]
            loose = [r['att'] / base[round(r['rtol'], 18)]['att'] for r in rows if r['rtol'] >= 0.99e-3]
            h0s = [r['h0'] for r in rows]
            # matched accuracy: frontier and cheapest-run vs the 1e-6 seed
            Rr = []; Cc = []
            for E in EG[p]:
                fa, _ = front(rows, 'att', E); fb, _ = front(D[(p, arm, '1e-06')], 'att', E); Rr.append(fa / fb)
                ca, cb = cheapest(rows, 'att', E), cheapest(D[(p, arm, '1e-06')], 'att', E)
                if ca and cb: Cc.append(ca / cb)
            line += (f" | h0={s}: equal-rtol att ratio min/med/max {min(rat):.2f}/{np.median(rat):.2f}/{max(rat):.2f} at rtol 1e-3 {loose[0]:.2f}"
                     f" (h0 range {min(h0s):.1e}..{max(h0s):.1e}); matched R {min(Rr):.3f}..{max(Rr):.3f}, C {min(Cc):.2f}..{max(Cc):.2f}")
            out[f'{p}|{arm}|{s}'] = dict(eq=[min(rat), float(np.median(rat)), max(rat)], at1e3=loose[0], R=[min(Rr), max(Rr)], C=[min(Cc), max(Cc)])
        print(line)
print("\n(d) early-abort ceiling: fraction of attempts saved if EVERY rejection were aborted after stage k (no false aborts)")
for p in ('vdp', 'hires', 'bruss50', 'robertson'):
    for arm in ('I', 'PRED', 'PRED+cap'):
        rows = [r for s in ('1e-06', '1e-04', '1e-02', 'auto') for r in D[(p, arm, s)]]
        fr = sum(r['rej'] for r in rows) / sum(r['att'] for r in rows)
        print(f"  {p:9s} {arm:9s} rej frac {fr*100:5.2f}%  ceiling k=2: {fr*6/8*100:5.2f}%  k=7 (late-stage): {fr*1/8*100:5.2f}%")
        out[f'abort|{p}|{arm}'] = dict(rej=fr, k2=fr * 6 / 8, k7=fr / 8)
json.dump(out, open(f'{HERE}/h0_null.json', 'w'), indent=1)
