import sys, json, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/mf_runs.jsonl', key=('problem', 'mode', 'arm', 'seed'))
SEEDS = sorted(set(k[3] for k in D))
EG = {'bruss50': [1e-4, 1e-5, 1e-6, 1e-7], 'bruss160': [1e-4, 1e-5, 1e-6, 1e-7],
      'hires': [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10]}
RES = {}
ONLY = sys.argv[1].split(',') if len(sys.argv) > 1 else None
for (p, m) in sorted(set((k[0], k[1]) for k in D)):
    if ONLY and f'{p}:{m}' not in ONLY: continue
    arms = sorted(set(k[2] for k in D if k[0] == p and k[1] == m))
    seeds = sorted(set(k[3] for k in D if k[0] == p and k[1] == m))
    print(f"\n######## {p} mode={m} seeds={seeds}")
    for a in arms:
        rows = [r for s in seeds for r in D.get((p, m, a, s), [])]
        if not rows: continue
        att = sum(r['att'] for r in rows); rej = sum(r['rej'] for r in rows); lf = sum(r['lfail'] for r in rows)
        jv = sum(r['jvp'] for r in rows); ip = sum(r['ip'] for r in rows); cols = sum(r['cols'] for r in rows)
        st1 = sum(r['st1_retry_jvp'] for r in rows); nd = sum(not r['done'] for r in rows)
        er = [r['err'] / r['rtol'] for r in rows]
        print(f"  {a:14s} rej {rej}/{att} ({100*rej/att:.1f}%) linfail {lf} JVP {jv} ip {ip} cols/solve {cols/(8*att):.1f} "
              f"st1-retry JVP share {st1/jv*100:.2f}% err/rtol median {np.median(er):.2f} max {max(er):.1f} unfinished {nd} "
              f"err range {min(r['err'] for r in rows):.1e}..{max(r['err'] for r in rows):.1e}")
    for ref in ('I', 'I725'):
        if ref not in arms: continue
        for w in ('jvp', 'flops10', 'flops100', 'att'):
            print(f"  -- ratio vs {ref}, work={w}")
            for a in arms:
                if a == ref: continue
                sc = score(D, p, a, ref, w, EG[p], seeds, extra=(m,))
                RES[(p, m, a, ref, w)] = sc
                line = f"    {a:12s}"
                for E in EG[p]:
                    z = sc[E]; fl = ('x' if z['x'] else '') + ('s' if z['s'] else '')
                    line += f" |{E:.0e} R{fmt(z['R'], fl)} C{fmt(z['C'])} Cp{('%.2f' % z['Cp']) if z['Cp'] else 'n/a'}"
                print(line)
def gm(sc, Es, rule):
    v = [sc[E][rule][0] for E in Es if sc[E][rule] is not None and not sc[E]['x'] and not sc[E]['s']]
    return float(np.exp(np.mean(np.log(v)))) if v else None
print("\n== geometric means over in-range E (R / C)")
for (p, m, a, ref, w), sc in sorted(RES.items()):
    r1, r2 = gm(sc, EG[p], 'R'), gm(sc, EG[p], 'C')
    print(f"  {p:9s} {m:5s} {w:8s} {a:12s} vs {ref:5s}: {r1 if r1 is None else round(r1,3)} / {r2 if r2 is None else round(r2,3)}")
json.dump({"|".join(k): {str(E): v for E, v in sc.items()} for k, sc in RES.items()}, open(f'{HERE}/scores_mf.json', 'w'), indent=0)
