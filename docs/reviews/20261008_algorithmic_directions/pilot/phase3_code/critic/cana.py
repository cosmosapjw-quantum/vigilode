"""Critic stress analysis: equal-rtol gates (accuracy vs base / vs the exact-solve twin / vs failure-free base),
robustness (new failures), cost at equal rtol, and matched accuracy (regression frontier + cheapest-run rule).
usage: python3 cana.py [problem ...]"""
import json, math, os, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import sprobs

SARM = os.environ.get('SARM', 'S')
SYN = lambda pn: sprobs.PROBLEMS[pn]().get('kind', 'bench') == 'synthetic' if pn in sprobs.PROBLEMS else False


def load(pn):
    fn = os.path.join(HERE, 'res', f'{pn}.jsonl'); d = {}
    if not os.path.exists(fn): return d
    for l in open(fn):
        o = json.loads(l)
        d[(o['arm'], o['rtol'])] = o
    return d


def ok(o):
    return o is not None and 'error' not in o and not o.get('capped')


def metric(o, syn):
    """rtol-independent error: synthetic = WRMS with weights (ascale + |ref|) (= e_w * rtol); bench = componentwise rel."""
    return o['e_w']*o['rtol'] if syn else o['e_cw']


def frontier(rows, E, key, syn):
    pts = [(metric(o, syn), o[key]) for o in rows if ok(o) and metric(o, syn) > 0 and o[key] > 0]
    if len(pts) < 3: return None, True
    x = np.log10([p[0] for p in pts]); y = np.log10([p[1] for p in pts])
    co = np.polyfit(x, y, 1)
    return float(10**np.polyval(co, math.log10(E))), not (min(x) <= math.log10(E) <= max(x))


def cheapest(rows, E, key, syn):
    v = [o[key] for o in rows if ok(o) and metric(o, syn) <= E]
    return min(v) if v else None


def matched(d, arm, ref, key, syn):
    ra = [o for (a, r), o in d.items() if a == arm]; rb = [o for (a, r), o in d.items() if a == ref]
    if not ra or not rb: return []
    errs = [metric(o, syn) for o in ra + rb if ok(o) and metric(o, syn) > 0]
    if not errs: return []
    lo, hi = math.floor(math.log10(min(errs))), math.ceil(math.log10(max(errs)))
    out = []
    for k in range(lo, hi + 1):
        E = 10.0**k
        fa, xa = frontier(ra, E, key, syn); fb, xb = frontier(rb, E, key, syn)
        ca, cb = cheapest(ra, E, key, syn), cheapest(rb, E, key, syn)
        if ca is None and cb is None and (xa or xb): continue
        out.append((E, fa/fb if fa and fb else None, xa or xb, ca/cb if ca and cb else None))
    return out


def fm(m):
    return '  '.join(f"{E:.0e}:{('%.2f' % r) if r else '-'}{'x' if x else ''}/{('%.2f' % c) if c else 'NR'}" for E, r, x, c in m)


def gm(v):
    v = [x for x in v if x]
    return float(np.exp(np.mean(np.log(v)))) if v else None


def summ(m):
    fr = [r for E, r, x, c in m if r and not x]; ch = [c for E, r, x, c in m if c]
    s = lambda v: f'{gm(v):.2f} [{min(v):.2f}-{max(v):.2f}]' if v else '-'
    return f'R {s(fr)} | C {s(ch)}'


def report(pn, out):
    d = load(pn)
    if not d: return
    syn = SYN(pn)
    rtols = sorted({r for (a, r) in d}, reverse=True)
    arms = sorted({a for (a, r) in d})
    out.append(f'\n##### {pn}  (table err = {"endpoint WRMS error in tolerance units, exact solution" if syn else "componentwise max rel err / rtol (B1 metric)"}; matched accuracy uses the rtol-free version)')
    hdr = f"{'rtol':>7} " + ' '.join(f'{a:>22}' for a in arms)
    out.append(hdr)
    for rt in rtols:
        cells = []
        for a in arms:
            o = d.get((a, rt))
            if o is None: cells.append(f"{'':>22}"); continue
            if 'error' in o: cells.append(f"{'ERR/skip':>22}"); continue
            e = metric(o, syn)/rt
            fl = ('C' if o.get('capped') else '') + (f"L{o['lin_fail']}" if o['lin_fail'] else '') + (f"N{o['nf_fail']}" if o['nf_fail'] else '')
            cells.append(f"{e:8.3g}|{o['att']:5d}|{o['jvp']:7d}{fl:>1}")
        out.append(f'{rt:7.0e} ' + ' '.join(f'{c:>22}' for c in cells))
    out.append('  cell = err | attempts | JVPs [C capped, Lk linear failures, Nk non-finite]; synthetic err in tol units, bench err/rtol')
    # gates
    gl = []
    for rt in rtols:
        g = lambda a: d.get((a, rt))
        S, A0, LUP, LU, Rb, C3, R3 = g(SARM), g('A0'), g('LUP'), g('LU'), g('Rbig'), g('C3G'), g('R3')
        if not (ok(S) and ok(A0)):
            if S is not None and A0 is not None:
                gl.append(f"  {rt:.0e}: S {'capped' if S.get('capped') else ('err' if 'error' in S else 'ok')} A0 {'capped' if A0.get('capped') else ('err' if 'error' in A0 else 'ok')}"
                          f" | S lf {S.get('lin_fail')} A0 lf {A0.get('lin_fail')}")
            continue
        m = lambda o: metric(o, syn) if ok(o) else float('nan')
        base = Rb if (ok(Rb) and A0['lin_fail'] > 0) else A0
        r_base = m(S)/m(base); r_lup = m(S)/m(LUP) if ok(LUP) else float('nan')
        r_c3lu = m(C3)/m(LU) if (ok(C3) and ok(LU)) else float('nan')
        r_lupu = m(LUP)/m(LU) if (ok(LUP) and ok(LU)) else float('nan')
        jr = S['jvp']/base['jvp'] if base['jvp'] else float('nan'); fr = S['flops']/base['flops']
        newf = (S['lin_fail'] > 0 and base['lin_fail'] == 0) or S.get('nf_fail', 0) > base.get('nf_fail', 0)
        flags = []
        if r_base > 1.5: flags.append('ACC>1.5x base')
        if newf: flags.append('NEW FAILURES')
        if fr > 1.0: flags.append('COST>base(flops)')
        if jr > 1.0: flags.append('JVP>base')
        gl.append(f"  {rt:.0e}: S/{'Rbig' if base is Rb else 'A0'} err {r_base:6.2f}  S/LUP {r_lup:6.2f}  C3G/LU {r_c3lu:6.2f}  LUP/LU {r_lupu:6.2f}"
                  f"  | S/base JVP {jr:5.2f} flops {fr:5.2f} | lf S {S['lin_fail']} A0 {A0['lin_fail']} guard {S['guard_q']}/{S['guard_over']}/{S['guard_false']} stalls {S['stalls']}  {' '.join(flags)}")
    out.append('  gates (equal rtol):'); out.extend(gl)
    for ref in ('A0', 'Rbig', 'R3', 'C3G', 'LU'):
        if ref not in arms: continue
        for key in ('jvp', 'flops'):
            if key == 'jvp' and ref == 'LU': continue
            m = matched(d, SARM, ref, key, syn)
            if m:
                out.append(f'  matched {SARM}/{ref:4s} [{key:5s}] {summ(m)}    {fm(m)}')


if __name__ == '__main__':
    names = sys.argv[1:] or sorted(f[:-6] for f in os.listdir(os.path.join(HERE, 'res')) if f.endswith('.jsonl'))
    out = []
    for pn in names:
        report(pn, out)
    print('\n'.join(out))
