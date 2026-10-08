"""Evaluate the judge's preregistered-style PASS/KILL lines (verify.json next_probe) on the replica data."""
import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/dense_runs.jsonl')
M = load(f'{HERE}/mf_runs.jsonl', key=('problem', 'mode', 'arm', 'seed'))
S4 = ['1e-06', '1e-04', '1e-02', 'auto']
E5 = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7]
EH = {'vdp': [1e-3, 3e-4, 1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7], 'bruss50': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7],
      'hires': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9, 3e-10, 1e-10],
      'robertson': [3e-5, 1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9, 3e-10],
      'rotnn96': [1e-5, 3e-6, 1e-6, 3e-7, 1e-7, 3e-8, 1e-8, 3e-9, 1e-9], 'bruss160': [1e-4, 3e-5, 1e-5, 3e-6, 1e-6, 3e-7]}
def inr(sc, rule):
    return {E: sc[E][rule][0] for E in sc if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']}
def gm(d): return float(np.exp(np.mean(np.log(list(d.values()))))) if d else float('nan')
res = {}
for arm in ('PRED', 'PRED+cap'):
    print(f"######## CTRL-PRED-CAP judge lines, arm = {arm} (dense attempts=LU; MF Bruss JVP and flops10)")
    sc = score(D, 'vdp', arm, 'I', 'att', E5, S4)
    nR = sum(1 for E in E5 if sc[E]['R'] and sc[E]['R'][0] <= 0.90); nC = sum(1 for E in E5 if sc[E]['C'] and sc[E]['C'][0] <= 0.90)
    print(f"  (i) vdP <= 0.90 at >=3 of 5 E: R {nR}/5 {[round(sc[E]['R'][0],3) for E in E5]} (1e-7 extrapolated={sc[1e-7]['x']}); "
          f"C {nC}/5 {[round(sc[E]['C'][0],3) if sc[E]['C'] else None for E in E5]} -> {'PASS' if nR >= 3 and nC >= 3 else 'FAIL'}")
    worst = {}
    for p in ('vdp', 'hires', 'bruss50', 'robertson', 'rotnn96'):
        s2 = score(D, p, arm, 'I', 'att', EH[p], S4)
        r, c = inr(s2, 'R'), inr(s2, 'C')
        worst[p] = (max(r.values()), max(r, key=r.get), max(c.values()) if c else None, max(c, key=c.get) if c else None, gm(r), gm(c))
    for (p, m) in (('bruss50', 'proj'), ('bruss160', 'proj')):
        for w in ('jvp', 'flops10', 'flops100'):
            s2 = score(M, p, arm, 'I', w, EH[p], S4, extra=(m,))
            r, c = inr(s2, 'R'), inr(s2, 'C')
            if r: worst[f'{p}-MF-{w}'] = (max(r.values()), max(r, key=r.get), max(c.values()) if c else None, max(c, key=c.get) if c else None, gm(r), gm(c))
    okR = all(v[0] <= 1.06 for v in worst.values()); okC = all(v[2] is None or v[2] <= 1.10 for v in worst.values())
    for k, v in worst.items():
        print(f"  (ii) {k:20s} worst R {v[0]:.3f} @E={v[1]:.0e}  worst C {v[2] if v[2] is None else round(v[2],3)} @E={v[3]}  gm R {v[4]:.3f} gm C {v[5]:.3f}")
    print(f"  (ii) no problem > 1.06 (R): {'PASS' if okR else 'FAIL'}; > 1.10 (C): {'PASS' if okC else 'FAIL'}")
    beat = {}
    for p in ('hires', 'bruss50'):
        s2 = score(D, p, arm, 'I725', 'att', EH[p], S4); beat[p] = (gm(inr(s2, 'R')), gm(inr(s2, 'C')))
    for (p, m) in (('bruss50', 'proj'), ('bruss160', 'proj')):
        s2 = score(M, p, arm, 'I725', 'jvp', EH[p], S4, extra=(m,)); r = inr(s2, 'R')
        if r: beat[f'{p}-MF-jvp'] = (gm(r), gm(inr(s2, 'C')))
    print(f"  (iii) {arm} vs I725 gm (R, C): {beat} -> {'PASS' if any(v[0] <= 0.97 for v in beat.values()) else 'FAIL'} (needs <=0.97 on HIRES or Bruss)")
    killv = all(sc[E]['R'][0] >= 0.95 for E in E5)
    print(f"  KILL lines: vdP >= 0.95 at every E: {killv}; any problem > 1.08 (R): {any(v[0] > 1.08 for v in worst.values())}")
    res[arm] = dict(worst=worst, beat=beat, nR=nR, nC=nC)
print("\n######## CTRL-EXPANSIVE-GATE closure lines (base = PRED+cap)")
s = score(D, 'vdp', 'PRED+cap+gate', 'PRED+cap', 'att', EH['vdp'], S4); g_v = (gm(inr(s, 'R')), gm(inr(s, 'C')))
print(f"  (1) vdP gate vs PRED+cap gm (R, C) = {g_v[0]:.3f}, {g_v[1]:.3f} -> need <= 0.92: {'PASS' if max(g_v) <= 0.92 else 'FAIL'}")
best = None
for u in ('PRED+cap85', 'PRED+cap80', 'PRED+cap75', 'PRED725', 'PRED65'):
    s = score(D, 'vdp', u, 'PRED+cap', 'att', EH['vdp'], S4); v = (gm(inr(s, 'R')), gm(inr(s, 'C')))
    print(f"      uniform {u:11s} vs PRED+cap gm (R, C) = {v[0]:.3f}, {v[1]:.3f}")
    if best is None or v[0] < best[1][0]: best = (u, v)
s = score(D, 'vdp', 'PRED+cap+gate', best[0], 'att', EH['vdp'], S4); gb = (gm(inr(s, 'R')), gm(inr(s, 'C')))
print(f"  (2) gate vs best uniform ({best[0]}) gm (R, C) = {gb[0]:.3f}, {gb[1]:.3f} -> need <= 0.97: {'PASS' if max(gb) <= 0.97 else 'FAIL'}")
for p in ('hires', 'bruss50', 'robertson', 'rotnn96'):
    s = score(D, p, 'PRED+cap+gate', 'PRED+cap', 'att', EH[p], S4); r = inr(s, 'R'); c = inr(s, 'C')
    print(f"  (3) {p:9s} gate vs PRED+cap gm R {gm(r):.3f} (range {min(r.values()):.3f}..{max(r.values()):.3f}) gm C {gm(c):.3f} -> within +-4%: {'yes' if 0.96 <= gm(r) <= 1.04 else 'NO'}")
rows = [r for s_ in S4 for r in D[('rotnn96', 'PRED+cap+gate', s_)]]
fr = sum(r['fire'] for r in rows) / sum(r['acc'] for r in rows)
print(f"  (4) rotating-nonnormal firing rate {fr*100:.1f}% of accepted steps -> need <= 30%: {'PASS' if fr <= 0.30 else 'FAIL'}")
print("\n######## CTRL-NULLS refutation rule: lever gives >= 10% fewer attempts at matched E on a problem without exceeding 1.05 on others")
for lever, ref in (('I+kobs', 'I'), ('PRED+kobs', 'PRED'), ('H211b+cap', 'I'), ('PI34+cap', 'I'), ('H211b+cap', 'PRED'), ('PI34+cap', 'PRED'), ('PIc', 'I')):
    best_ = []; worst_ = []
    for p in ('vdp', 'hires', 'bruss50', 'robertson'):
        s = score(D, p, lever, ref, 'att', EH[p], S4); r = inr(s, 'R')
        best_.append((p, min(r.values()))); worst_.append((p, max(r.values()), gm(r)))
    kills = [p for p, b in best_ if b <= 0.90 and all(w <= 1.05 for q, w, _ in worst_ if q != p)]
    print(f"  {lever:10s} vs {ref:5s}: best-E R {[(p, round(b,3)) for p,b in best_]}; worst-E R {[(p, round(w,3)) for p,w,_ in worst_]}; gm {[(p, round(g,3)) for p,_,g in worst_]} -> null {'REFUTED on '+str(kills) if kills else 'confirmed'}")
json.dump(res, open(f'{HERE}/judge_eval.json', 'w'), indent=1, default=str)
