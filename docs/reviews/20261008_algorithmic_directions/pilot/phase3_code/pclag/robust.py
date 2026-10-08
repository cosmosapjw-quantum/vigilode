"""Robustness tables: Krylov columns per stage vs N, vs rho = h/h_P, vs h*g*stiffness; failures, stalls,
refresh reasons. EXPLORATORY. usage: python3 robust.py"""
import json, os, sys, math
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze import load
import probs2d

g = 0.21193756319429014

FAM = {'semilin2d': ['semilin2d-32', 'semilin2d-64', 'semilin2d-128', 'semilin2d-256'],
       'bruss2d': ['bruss2d-32', 'bruss2d-64', 'bruss2d-128', 'bruss2d-256'],
       'bruss2d-a0.1': ['bruss2d-64-a0.1', 'bruss2d-128-a0.1'],
       'semilin2d-32x48': ['semilin2d-32x48'], 'semilin2d-64-s4': ['semilin2d-64-s4']}
STIFF = {}


def stiff(pn):
    if pn not in STIFF:
        STIFF[pn] = probs2d.build(pn)['stiff']
    return STIFF[pn]


def main():
    print('### A. Krylov columns per stage (mean over all stage solves) and p95 of the per-attempt max, by arm and N; '
          'failures (linear budget), stall acceptances, refresh reasons (lag/lin)')
    for fam, pns in FAM.items():
        for arm in ('mf', 'lag', 'lin', 'dst', 'lag-w4', 'lin-w4', 'lag-rho'):
            line = []
            for pn in pns:
                rows = load(pn)
                rs = [o for (rt, a), o in rows.items() if a == arm and 'error' not in o]
                if not rs: continue
                cs = np.concatenate([np.array(o['colstat'])[:, 2] for o in rs if 'colstat' in o])
                mx = np.concatenate([np.array(o['colstat'])[:, 3] for o in rs if 'colstat' in o])
                lf = sum(o['lin_fail'] for o in rs); st = sum(o['stalls'] for o in rs)
                rr = sum(o['ref_rho'] for o in rs); ri = sum(o['ref_iter'] for o in rs); rn = sum(o['ref_new'] for o in rs)
                nerr = sum(1 for (rt, a), o in rows.items() if a == arm and 'error' in o)
                line.append(f'{pn}: cols/stage {cs.mean():5.2f} p95max {np.percentile(mx, 95):5.1f} max {mx.max():4.0f} '
                            f'linfail {lf} stalls {st} err-runs {nerr}' + (f' refresh(new/rho/iter) {rn}/{rr}/{ri}' if arm.startswith(('lag', 'lin')) else ''))
            if line:
                print(f'  {arm:7s} ' + '\n          '.join(line))
    print('\n### B. lag / lin: mean columns per stage binned by rho = h/h_P (all rtols pooled)')
    bins = [0.5, 0.7, 0.9, 1.0001, 1.25, 1.6, 2.0001]
    for fam, pns in FAM.items():
        for arm in ('lag', 'lin', 'lag-w4', 'lin-w4'):
            for pn in pns:
                rows = load(pn)
                rs = [o for (rt, a), o in rows.items() if a == arm and 'error' not in o and 'colstat' in o]
                if not rs: continue
                C = np.concatenate([np.array(o['colstat']) for o in rs])
                rho = C[:, 1]; cm = C[:, 2]
                bb = [0.25, 0.5, 0.7, 0.9, 1.0001, 1.25, 1.6, 2.0001, 3.0, 4.0001] if arm.endswith('w4') else bins
                cells = []
                for lo, hi in zip(bb[:-1], bb[1:]):
                    m = (rho >= lo) & (rho < hi)
                    cells.append(f'[{lo:.2g},{hi:.2g}):{cm[m].mean():5.2f}(n={m.sum()})' if m.sum() else f'[{lo:.2g},{hi:.2g}):  -  ')
                print(f'  {pn:16s} {arm:7s} ' + ' '.join(cells))
    print('\n### C. mf / dst / lag columns per stage vs h*g*lambda_max (stiffness proxy), pooled over rtols')
    for fam, pns in FAM.items():
        for arm in ('mf', 'lag', 'lin', 'dst'):
            for pn in pns:
                rows = load(pn)
                rs = [o for (rt, a), o in rows.items() if a == arm and 'error' not in o and 'colstat' in o]
                if not rs: continue
                C = np.concatenate([np.array(o['colstat']) for o in rs])
                z = C[:, 0] * g * stiff(pn); cm = C[:, 2]
                edges = [0, 1, 10, 100, 1e3, 1e4, 1e9]
                cells = []
                for lo, hi in zip(edges[:-1], edges[1:]):
                    m = (z >= lo) & (z < hi)
                    if m.sum(): cells.append(f'[{lo:g},{hi:g}):{cm[m].mean():5.1f}(n={m.sum()})')
                print(f'  {pn:16s} {arm:4s} ' + ' '.join(cells))


if __name__ == '__main__':
    main()
