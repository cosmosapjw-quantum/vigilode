import sys
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from analyze import *
HERE = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl'
D = load(f'{HERE}/dense_runs.jsonl'); S4 = ['1e-06', '1e-04', '1e-02', 'auto']
EG = {'vdp': [1e-3, 1e-4, 1e-5, 1e-6, 3e-7], 'hires': [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10],
      'bruss50': [3e-5, 1e-5, 3e-6, 1e-6, 3e-7], 'robertson': [1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 3e-10], 'rotnn96': [1e-6, 1e-7, 1e-8, 1e-9]}
def f(z, key):
    v = z[key]
    if v is None: return 'n/a'
    return f"{v[0]:.2f}[{v[1]:.2f}-{v[2]:.2f}]" + ('x' if (key == 'R' and z['x']) else '')
for ref, arms in (('I', ['I725', 'PRED', 'PRED+cap', 'PRED725']), ('I725', ['PRED'])):
    for p in EG:
        for a in arms:
            if (p, a, '1e-06') not in D: continue
            sc = score(D, p, a, ref, 'att', EG[p], S4)
            print(f"{p:9s} {a:9s}/{ref:4s} R: " + ' '.join(f"{E:.0e}:{f(sc[E],'R')}" for E in EG[p]))
            print(f"{'':9s} {'':9s}      C: " + ' '.join(f"{E:.0e}:{f(sc[E],'C')}" for E in EG[p]))
