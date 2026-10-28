"""Probe B1 analysis: references, errors, equal-rtol tables, accuracy gates, contamination, matched accuracy
(regression frontier + harness cheapest-run rule), attribution, PRED interaction, robustness and FD tables."""
import json, math, os, struct, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import stack, run, tprobs
LD = np.longdouble
_REF = {}


def ref(name):
    if name in _REF: return _REF[name]
    p = run.problem(name)
    if p['exact'] is not None:
        r = (np.array(p['exact'](p['span'][1]), dtype=float), 0.0)
    elif name in ('bruss160', 'bruss300'):
        d = json.load(open(os.path.join(HERE, f'ref_{name}_radau.json')))
        r = (np.array([float(x) for x in d['Radau_1e-13']['y']]), d['uncertainty_componentwise'])
    else:
        d = json.load(open(os.path.join(HERE, f'ref_{name}.json')))
        r = (np.array([float(LD(v)) for v in d['1e-15']['y_ld']]), d['uncertainty_componentwise'])
    _REF[name] = r
    return r


def hk(rtol):
    """half-decade key: 12 for 1e-6, 13 for 3.16e-7, ..."""
    return int(round(-2*math.log10(rtol)))


def load(name):
    fn = os.path.join(HERE, 'res', f'{name}.jsonl')
    if not os.path.exists(fn): return {}
    d = {}
    yr, _ = ref(name)
    p = run.problem(name)
    for line in open(fn):
        o = json.loads(line)
        if 'error' in o:
            d[(o['arm'], hk(o['rtol']))] = o; continue
        y = np.array(o['y'])
        o['e_cw'], o['e_nw'] = stack.err_metrics(y, yr)
        o['ecw_rt'] = o['e_cw']/o['rtol']
        o['rhs_tot'] = o['rhs'] + o.get('fd_rhs', 0)
        d[(o['arm'], hk(o['rtol']))] = o
        if o['arm'] == 'A0':      # derived arm (1): identical trajectory, minus the duplicate diagnostic residual
            o1 = dict(o); o1['arm'] = 'A1'
            for k in ('jvp', 'axpys', 'norms'): o1[k] = o[k] - o['diag']
            o1['diag'] = 0
            a1 = stack.make_arm('base', dup=False)
            o1['flops'] = stack.flops(p, o1, a1); o1['flops_fd'] = stack.flops(p, o1, a1, jvp_model='fd')
            d[('A1', hk(o['rtol']))] = o1
    return d


def ok(o):
    return o is not None and 'error' not in o and not o.get('capped')


def frontier(rows, E, wkey='jvp'):
    pts = [(o['e_cw'], o[wkey]) for o in rows if ok(o) and o['e_cw'] > 0 and o[wkey] > 0]
    if len(pts) < 3: return None, True
    x = np.log10([p[0] for p in pts]); y = np.log10([p[1] for p in pts])
    co = np.polyfit(x, y, 1)
    return float(10**np.polyval(co, math.log10(E))), not (min(x) <= math.log10(E) <= max(x))


def cheapest(rows, E, wkey='jvp'):
    v = [o[wkey] for o in rows if ok(o) and o['e_cw'] <= E]
    return min(v) if v else None


def arm_rows(d, arm):
    return [o for (a, k), o in d.items() if a == arm]


def egrid(d, ref_arm='A0', step=1.0):
    errs = [o['e_cw'] for o in arm_rows(d, ref_arm) if ok(o) and o['e_cw'] > 0]
    if not errs: return []
    lo, hi = math.log10(min(errs)), math.log10(max(errs))
    k0, k1 = math.ceil(lo/step), math.floor(hi/step)
    return [10**(k*step) for k in range(k0, k1 + 1)]


def matched(d, arm, refarm, wkey='jvp', Es=None):
    """list of (E, frontier ratio, extrapolated?, cheapest ratio)"""
    Es = Es or egrid(d, 'A0')
    ra, rb = arm_rows(d, arm), arm_rows(d, refarm)
    out = []
    for E in Es:
        fa, xa = frontier(ra, E, wkey); fb, xb = frontier(rb, E, wkey)
        ca, cb = cheapest(ra, E, wkey), cheapest(rb, E, wkey)
        fr = fa/fb if (fa and fb) else None
        ch = ca/cb if (ca and cb) else None
        out.append((E, fr, xa or xb, ch))
    return out


def gm(xs):
    xs = [x for x in xs if x]
    return float(np.exp(np.mean(np.log(xs)))) if xs else None


def summarize_matched(m):
    """geo-mean and range of in-range frontier ratios; range of cheapest ratios"""
    fr = [r for (E, r, x, c) in m if r and not x]
    frx = [r for (E, r, x, c) in m if r]
    ch = [c for (E, r, x, c) in m if c]
    f = lambda v: (f'{gm(v):.2f} [{min(v):.2f}-{max(v):.2f}]' if v else '-')
    return f(fr), f(ch), len(fr), len(frx) - len(fr)


def fmt_m(m):
    return '  '.join(f"{E:.0e}:{('%.2f' % r) if r else '-'}{'x' if x else ''}/{('%.2f' % c) if c else '-'}" for E, r, x, c in m)


def egrid_for(pb, d):
    """decade grid; half-decade grid for the narrow-range Bruss-160/300 ladders"""
    return egrid(d, 'A0', step=(0.5 if pb in ('bruss160', 'bruss300') else 1.0))
