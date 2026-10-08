import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{sys.argv[1] if len(sys.argv) > 1 else "dense_runs.jsonl"}')
SEEDS = ['1e-06', '1e-04', '1e-02', 'auto']
PROBS_ = ['vdp', 'hires', 'bruss50', 'robertson']
EG = {'vdp': [1e-3, 1e-4, 1e-5, 1e-6, 1e-7], 'bruss50': [1e-4, 1e-5, 1e-6, 1e-7],
      'hires': [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10], 'robertson': [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10]}
ARMS_ = sorted(set(k[1] for k in D))
W = sys.argv[2] if len(sys.argv) > 2 else 'att'
RES = {}
print(f"#### work metric = {W}")
print("== rejection fraction (pooled over ladder and seeds) [seed min, max]; err range")
for p in PROBS_:
    line = f"{p:10s}"
    for a in ['I', 'I725', 'PIc', 'H211b+cap', 'PI34+cap', 'PRED', 'PRED+cap', 'PRED725', 'PRED+gate', 'PRED+cap+gate', 'I+kobs', 'PRED+kobs']:
        f = rejfrac(D, p, a, SEEDS)
        if f[0] is not None: line += f" {a}={f[0]*100:.1f}%[{f[1]*100:.1f},{f[2]*100:.1f}]"
    print(line)
    for a in ['I', 'PRED']:
        errs = [r['err'] for s in SEEDS for r in D[(p, a, s)]]
        print(f"     {a} err range {min(errs):.2e}..{max(errs):.2e}; att total {sum(r['att'] for s in SEEDS for r in D[(p,a,s)])}")
def table(ref, arms, title):
    print(f"\n== {title}: ratio arm/{ref}; per E: R=global frontier, Rw=windowed, C=cheapest-run (median[min,max] over 4 seeds), Cp=pooled cheapest")
    for p in PROBS_:
        print(f"-- {p}")
        for a in arms:
            sc = score(D, p, a, ref, W, EG[p], SEEDS)
            RES[(p, a, ref)] = sc
            line = f"  {a:14s}"
            for E in EG[p]:
                z = sc[E]
                fl = ('x' if z['x'] else '') + ('s' if z['s'] else '')
                line += f" |{E:.0e} R{fmt(z['R'], fl)} Rw{('%.3f' % z['Rw'][0]) if z['Rw'] else ' n/a '} C{fmt(z['C'])} Cp{('%.2f' % z['Cp']) if z['Cp'] else 'n/a'}"
            print(line)
table('I', ['I725', 'PIc', 'H211b+cap', 'PI34+cap', 'PRED', 'PRED+cap', 'PRED725', 'PRED+gate', 'PRED+cap+gate', 'I+gate', 'I+kobs', 'PRED+kobs'], 'vs production I')
table('I725', ['I', 'PRED', 'PRED+cap', 'PRED725', 'PRED+gate'], 'vs set-point rival I725')
table('PRED+cap', ['PRED', 'PRED+cap+gate', 'PRED+cap85', 'PRED+cap80', 'PRED+cap75', 'PRED725', 'PRED+gate'], 'gate closure vs PRED+cap')
table('PRED', ['PRED+cap', 'PRED+kobs', 'H211b+cap', 'PI34+cap', 'PRED725'], 'vs PRED')
# geometric means over non-extrapolated E
def gm(p, a, ref, rule):
    sc = RES.get((p, a, ref))
    if sc is None: return None
    v = [sc[E][rule][0] for E in EG[p] if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']]
    return float(np.exp(np.mean(np.log(v)))) if v else None
print("\n== geometric mean over in-range E (median-over-seeds ratios), rules R / C")
for ref in ('I', 'I725', 'PRED+cap', 'PRED'):
    for p in PROBS_:
        line = f"ref={ref:8s} {p:10s}"
        for (pp, a, rr) in RES:
            if pp == p and rr == ref:
                r1, r2 = gm(p, a, ref, 'R'), gm(p, a, ref, 'C')
                line += f" {a}={r1:.3f}/{r2:.3f}" if r1 and r2 else f" {a}=n/a"
        print(line)
json.dump({f"{k[0]}|{k[1]}|{k[2]}": {str(E): v for E, v in sc.items()} for k, sc in RES.items()},
          open(f'{HERE}/scores_dense_{W}.json', 'w'), indent=0)
