"""PROBE B4 analysis: tables, matched accuracy (regression frontier + harness cheapest-run rule), N scaling.
EXPLORATORY. usage: python3 analyze.py [problems...]"""
import json, glob, math, os, sys
import numpy as np
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))


def ref_for(pname):
    fn = f'{HERE}/refs/{pname}.npz'
    if not os.path.exists(fn):
        return None, None
    z = dict(np.load(fn))
    if 'y_r13' in z:
        unc = float(np.max(np.abs(z['y_r13'] - z['y_r12'])) / np.max(np.abs(z['y_r13']))) if 'y_r12' in z else None
        if 'y_radau' in z:
            unc = max(unc or 0, float(np.max(np.abs(z['y_r13'] - z['y_radau'])) / np.max(np.abs(z['y_r13']))))
        return z['y_r13'], (unc * 0.1 if unc else None)   # r13 is ~10x better than the r12 - r13 gap
    return z['y_r12'], None


def load(pname):
    rows = {}
    fn = f'{HERE}/res/{pname}.jsonl'
    if not os.path.exists(fn): return {}
    ref, unc = (None, None)
    if not pname.startswith('semilin'):
        ref, unc = ref_for(pname)
    for line in open(fn):
        o = json.loads(line)
        if 'error' in o:
            rows[(o['rtol'], o['arm'])] = o; continue
        if 'err_rel' not in o:
            if ref is None: continue
            y = np.array(o['y']); o['err_rel'] = float(np.max(np.abs(y - ref)) / np.max(np.abs(ref)))
        o['ref_unc'] = unc if not pname.startswith('semilin') else 0.0
        o['F'] = o['flops']['total']; o['Ffd'] = o['flops_fd']['total']
        rows[(o['rtol'], o['arm'])] = o
    return rows


def arm_rows(rows, arm, minerr=0.0):
    return sorted([o for (rt, a), o in rows.items() if a == arm and 'error' not in o and o['err_rel'] > minerr],
                  key=lambda o: -o['rtol'])


def frontier(rs, E, w='F'):
    if len(rs) < 2: return None, True
    x = np.log10([o['err_rel'] for o in rs]); y = np.log10([o[w] for o in rs])
    b, a = np.polyfit(x, y, 1)
    return float(10 ** (a + b * math.log10(E))), not (x.min() <= math.log10(E) <= x.max())


def wfront(rs, E, w='F', half=1.0):
    pts = [(math.log10(o['err_rel']), math.log10(o[w])) for o in rs if abs(math.log10(o['err_rel']) - math.log10(E)) <= half]
    if len(pts) < 3: return None
    e, y = np.array(pts).T
    if e.max() - e.min() < 0.3: return None
    b, a = np.polyfit(e, y, 1)
    return float(10 ** (a + b * math.log10(E)))


def cheapest(rs, E, w='F'):
    ok = [o[w] for o in rs if o['err_rel'] <= E]
    return min(ok) if ok else None


def fmt(x, d=3):
    return 'n/a' if x is None else f'{x:.{d}g}'


def table_ladder(pname, rows, arms):
    print(f'\n### {pname}: per-rtol cells (err = max|y-ref|/max|ref| at t_end; F = total Gflop, sparse JVP model)')
    print(f"{'rtol':7s} {'arm':8s} {'att':>4s} {'rej':>3s} {'lf':>2s} {'stl':>3s} {'err':>9s} {'err/err_dir':>10s} {'GF':>8s} {'GF/dir':>7s} "
          f"{'jvp/att':>7s} {'pcs/att':>7s} {'cjvp':>5s} {'LU/acc':>6s} {'col/st':>6s} {'p95max':>6s} {'LU%':>5s} {'PC%':>5s} {'JVP%':>5s} {'orth%':>5s} {'wall':>6s}")
    for rt in sorted({k[0] for k in rows}, reverse=True):
        dref = rows.get((rt, 'direct'))
        for a in arms:
            o = rows.get((rt, a))
            if o is None: continue
            if 'error' in o:
                print(f'{rt:<7g} {a:8s} ERROR {o["error"][:80]}'); continue
            fl = o['flops']; F = fl['total']
            er = o['err_rel'] / dref['err_rel'] if dref and 'error' not in dref else None
            fr = F / dref['F'] if dref and 'error' not in dref else None
            print(f"{rt:<7g} {a:8s} {o['att']:4d} {o['rej']:3d} {o['lin_fail']:2d} {o['stalls']:3d} {o['err_rel']:9.2e} {fmt(er):>10s} {F/1e9:8.3f} {fmt(fr):>7s} "
                  f"{o['jvp']/o['att']:7.1f} {o['pcs']/o['att']:7.1f} {o['cjvp']:5d} {o['lus']/o['acc']:6.3f} {o.get('mean_cols_stage', 0):6.2f} "
                  f"{o.get('p95_maxcols', 0):6.1f} {100*(fl['lu']+fl['lusolve'])/F:5.1f} {100*fl['pc']/F:5.1f} {100*(fl['jvp']+fl['cjvp'])/F:5.1f} "
                  f"{100*fl['orth']/F:5.1f} {o['wall']:6.1f}")


def matched(pname, rows, arms, refs=('direct', 'mf'), Es=None, w='F'):
    minerr = 0.0
    unc = [o.get('ref_unc') for o in rows.values() if o.get('ref_unc')]
    if unc: minerr = 30 * max(unc)
    R = {a: arm_rows(rows, a, minerr) for a in set(arms) | set(refs)}
    allerr = [o['err_rel'] for a in R for o in R[a]]
    if not allerr: return {}
    if Es is None:
        lo, hi = math.floor(math.log10(min(allerr))), math.ceil(math.log10(max(allerr)))
        Es = [10.0 ** k for k in range(hi, lo - 1, -1)]
    out = {}
    print(f'\n### {pname}: matched accuracy, work = {w} ({"total flops" if w=="F" else w}); ratio arm/ref at endpoint error E. '
          f'R = log-log regression frontier over the ladder (x = extrapolated), Rw = local window fit, C = harness cheapest-run-reaching-E'
          + (f'; points with err < {minerr:.1e} (30x ref uncertainty) excluded' if minerr else ''))
    for rf in refs:
        if not R.get(rf): continue
        print(f'  vs {rf}:   ' + ' '.join(f'{"E="+format(E,".0e"):>22s}' for E in Es))
        for a in arms:
            if a == rf or not R.get(a): continue
            cells = []
            for E in Es:
                fa, xa = frontier(R[a], E, w); fr, xr = frontier(R[rf], E, w)
                wa, wr = wfront(R[a], E, w), wfront(R[rf], E, w)
                ca, cr = cheapest(R[a], E, w), cheapest(R[rf], E, w)
                rr = fa / fr if fa and fr else None
                rw = wa / wr if wa and wr else None
                rc = ca / cr if ca and cr else None
                out[(rf, a, E)] = dict(R=rr, x=(xa or xr), Rw=rw, C=rc)
                cells.append(f'{fmt(rr)}{"x" if (xa or xr) else " "}/{fmt(rw)}/{fmt(rc)}')
            print(f'  {a:8s}   ' + ' '.join(f'{c:>22s}' for c in cells))
    return out


PROBS = ['semilin2d-32', 'semilin2d-32x48', 'semilin2d-64', 'semilin2d-128', 'semilin2d-256',
         'bruss2d-32', 'bruss2d-64', 'bruss2d-128', 'bruss2d-256', 'bruss2d-64-a0.1', 'bruss2d-128-a0.1']
ARMS = ['direct', 'mf', 'lag', 'lin', 'dst', 'lag-rho', 'lag-w4', 'lin-w4', 'mf-m200']

if __name__ == '__main__':
    probs = sys.argv[1:] or PROBS
    allm = {}
    for pn in probs:
        rows = load(pn)
        if not rows: continue
        table_ladder(pn, rows, ARMS)
        allm[pn] = matched(pn, rows, [a for a in ARMS if a != 'direct'], refs=('direct',))
        matched(pn, rows, [a for a in ARMS if a not in ('direct', 'mf')], refs=('mf',))
