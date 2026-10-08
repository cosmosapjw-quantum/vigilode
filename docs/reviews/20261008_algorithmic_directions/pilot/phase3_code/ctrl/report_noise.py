"""Noise arm: does PRED's advantage survive loose stage solves? Same 2 seeds (1e-06, auto) in every mode."""
import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
M = load(f'{HERE}/mf_runs.jsonl', key=('problem', 'mode', 'arm', 'seed'))
S2 = ['1e-06', 'auto']
EH = {'bruss50': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7], 'bruss160': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7],
      'hires': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9, 3e-10]}
def inr(sc, rule):
    return {E: sc[E][rule][0] for E in sc if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']}
def gm(d): return float(np.exp(np.mean(np.log(list(d.values()))))) if d else float('nan')
out = {}
for p in ('bruss50', 'bruss160', 'hires'):
    print(f"==== {p} (seeds {S2})")
    for m in ('proj', 'tf1', 'tf2', 'rel4'):
        arms = [a for a in ('I', 'I725', 'PRED', 'PRED+cap') if all((p, m, a, s) in M for s in S2)]
        if 'I' not in arms: continue
        line = f"  {m:5s}"
        for a in arms:
            rows = [r for s in S2 for r in M[(p, m, a, s)]]
            att = sum(r['att'] for r in rows); rej = sum(r['rej'] for r in rows); lf = sum(r['lfail'] for r in rows)
            er = np.array([r['err'] / r['rtol'] for r in rows]); nd = sum(not r['done'] for r in rows)
            line += f" | {a}: rej {100*rej/att:.1f}% lfail {lf} err/rtol med {np.median(er):.2f} max {er.max():.3g}{' UNFINISHED '+str(nd) if nd else ''}"
            out[f'{p}|{m}|{a}|stats'] = dict(rej=rej / att, lfail=lf, med=float(np.median(er)), mx=float(er.max()), unfinished=nd, jvp=sum(r['jvp'] for r in rows))
        print(line)
        for w in ('jvp', 'flops10', 'flops100'):
            line = f"        {w:8s} gm R/C vs I:"
            for a in arms:
                if a == 'I': continue
                sc = score(M, p, a, 'I', w, EH[p], S2, extra=(m,)); r, c = inr(sc, 'R'), inr(sc, 'C')
                wr = max(r.values()) if r else float('nan')
                line += f"  {a}={gm(r):.3f}/{gm(c):.3f} (worst-E R {wr:.3f}, nE {len(r)})"
                out[f'{p}|{m}|{a}|{w}'] = dict(R=gm(r), C=gm(c), worstR=wr, nE=len(r))
            print(line)
        if 'I725' in arms:
            sc = score(M, p, 'PRED', 'I725', 'jvp', EH[p], S2, extra=(m,)); r, c = inr(sc, 'R'), inr(sc, 'C')
            print(f"        jvp      PRED vs I725 gm R/C {gm(r):.3f}/{gm(c):.3f}")
    # cross-mode: PRED/I ratio change and arm-wise mode increments
json.dump(out, open(f'{HERE}/noise_arm.json', 'w'), indent=1)
