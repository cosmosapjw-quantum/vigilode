"""Markdown tables for the B1 report (compact versions of tables.py)."""
import math, os, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import ana, tables as T

D = T.D
NM = {'bruss50': 'Bruss-50', 'bruss160': 'Bruss-160', 'bruss300': 'Bruss-300', 'hires': 'HIRES', 'robertson': 'Robertson',
      'vdp': 'vdP', 'pr': 'PR-forced', 'quad4': 'quad-4'}
CELLS = T.CELLS


def rt(k): return f'{10**(-k/2):.0e}' if k % 2 == 0 else f'{10**(-k/2):.2g}'


def cell_table(arms, key):
    print(f'\n| cell | ' + ' | '.join(arms) + ' |')
    print('|---|' + '---|'*len(arms))
    for pb, k in CELLS:
        d = D[pb]; b = d.get(('A0', k))
        if not ana.ok(b): continue
        row = []
        for a in arms:
            o = d.get((a, k))
            if not ana.ok(o): row.append('-'); continue
            if key == 'jvp_err':
                row.append(f"{o['jvp']} / {o['ecw_rt']:.3g}")
            elif key == 'counts':
                row.append(f"{o['att']}/{o['acc']}/{o['rej']}/{o['lin_fail']}")
            elif key == 'ops':
                row.append(f"{o['rhs_tot']} / {o['dots']} / {o['axpys']}")
            elif key == 'flops':
                row.append(f"{o['flops']/1e6:.3g}")
            elif key == 'ratio':
                row.append(f"{o['jvp']/b['jvp']:.3f} / {o['flops']/b['flops']:.3f} / {o['e_cw']/b['e_cw']:.2f}")
        print(f'| {NM[pb]} {rt(k)} | ' + ' | '.join(row) + ' |')


def matched_table(pairs, wkeys=('jvp', 'flops'), probs=None):
    probs = probs or ['bruss50', 'bruss160', 'bruss300', 'hires', 'robertson', 'vdp', 'pr', 'quad4']
    print('\n| problem | ' + ' | '.join(f'{a}/{b} {w}' for a, b in pairs for w in wkeys) + ' |')
    print('|---|' + '---|'*(len(pairs)*len(wkeys)))
    for pb in probs:
        d = D[pb]
        if not d: continue
        row = []
        for a, b in pairs:
            for w in wkeys:
                if not ana.arm_rows(d, a) or not ana.arm_rows(d, b): row.append('-'); continue
                m = ana.matched(d, a, b, w, ana.egrid_for(pb, d))
                fs, cs, nin, nx = ana.summarize_matched(m)
                row.append(f'{fs} / {cs}' + (f' ({nx}x)' if nx else ''))
        print(f'| {NM[pb]} | ' + ' | '.join(row) + ' |')


if __name__ == '__main__':
    which = sys.argv[1]
    if which == 'cells':
        for key in ('jvp_err', 'counts', 'ops', 'flops', 'ratio'):
            print(f'\n### {key}')
            cell_table(sys.argv[2].split(','), key)
    elif which == 'matched':
        pairs = [tuple(x.split('/')) for x in sys.argv[2].split(',')]
        wk = tuple(sys.argv[3].split(',')) if len(sys.argv) > 3 else ('jvp', 'flops')
        matched_table(pairs, wk)
