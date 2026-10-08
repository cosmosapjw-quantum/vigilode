"""Final analysis for probe A1: accuracy gate, costs, matched accuracy, contamination scaling, mechanism."""
import json, math, sys, numpy as np
sys.path.insert(0, '.')
import ana, tprobs
PROBS = ['hires', 'robertson', 'vdp', 'bruss50', 'pr', 'quad4']
RTS = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11]
TAG = 'rec'

def D(name):
    return ana.latest(ana.load(name, TAG))

def gate(arms, ref_arm='base', key='e_cw', thresh=1.5, out=None):
    """ratio err_arm/err_ref per cell; returns dict and prints."""
    res = {}
    for pb in PROBS:
        d = D(pb)
        for a in arms:
            for rt in RTS:
                o = d.get((a, rt)); b = d.get((ref_arm, rt))
                if o is None or b is None or 'error' in o or 'error' in b: continue
                res[(pb, a, rt)] = o[key]/b[key] if b[key] > 0 else float('inf')
    return res

def cost_ratio(pb, a, ref, rt, wkey):
    d = D(pb); o = d.get((a, rt)); b = d.get((ref, rt))
    if o is None or b is None: return None
    return o[wkey]/b[wkey]

def frontier_ratio(pb, a, ref, E, wkey='jvp'):
    d = D(pb)
    ra = [o for (k, rt), o in d.items() if k == a and 'error' not in o]
    rb = [o for (k, rt), o in d.items() if k == ref and 'error' not in o]
    wa, xa = ana.frontier(ra, E, wkey); wb, xb = ana.frontier(rb, E, wkey)
    if wa is None or wb is None: return None, True
    return wa/wb, (xa or xb)

def cheapest_ratio(pb, a, ref, E, wkey='jvp'):
    d = D(pb)
    ra = [o for (k, rt), o in d.items() if k == a and 'error' not in o]
    rb = [o for (k, rt), o in d.items() if k == ref and 'error' not in o]
    ca, cb = ana.cheapest(ra, E, wkey), ana.cheapest(rb, E, wkey)
    if ca is None or cb is None: return None
    return ca/cb

def contamination(pb, a, ref='lu'):
    """componentwise |y_a - y_ref|/(rtol*max(|y_ref|,1e-10)) at the endpoint, only when (acc,rej) identical"""
    d = D(pb); out = {}
    for rt in RTS:
        o = d.get((a, rt)); b = d.get((ref, rt))
        if o is None or b is None or 'error' in o: continue
        if (o['acc'], o['rej']) != (b['acc'], b['rej']): out[rt] = None; continue
        ya, yb = np.array(o['y']), np.array(b['y'])
        out[rt] = float(np.max(np.abs(ya - yb)/(rt*np.maximum(np.abs(yb), 1e-10))))
    return out

if __name__ == '__main__':
    pass

# ------------------------------------------------------------------------------------------------
def tableB():
    print('\n### B. base vs proj (L2 1e-10, floor g*1e-14) vs exact-solve LU control: endpoint err/rtol (componentwise)')
    print('problem  rtol    LU      base    proj   | proj mechanism: floor-bind%  max-stage med res (tol u)  max-stage med ||b||_w')
    for pb in PROBS:
        d = D(pb)
        for rt in RTS:
            l, b, p = d.get(('lu', rt)), d.get(('base', rt)), d.get(('proj', rt))
            if not (l and b and p): continue
            print(f"{pb:8s} {rt:<7g} {l['ecw_rt']:6.3f}  {b['ecw_rt']:6.3f}  {p['ecw_rt']:6.3f} | {100*p['floor_bind']/p['solves']:5.1f}%  "
                  f"{max(p['res_med']):9.1e}  {max(p['bw_med']):9.1e}  acc/rej base {b['acc']}/{b['rej']} proj {p['acc']}/{p['rej']} LU {l['acc']}/{l['rej']}")

def tableC(arms, exclude=(('robertson', 1e-11),)):
    print('\n### C. accuracy gate: max over cells of err_arm/err_base (componentwise); cells > 1.5x listed; also max vs LU')
    for a in arms:
        worst = []; worst_lu = []; fails = []
        for pb in PROBS:
            d = D(pb)
            for rt in RTS:
                if (pb, rt) in exclude: continue
                o, b, l = d.get((a, rt)), d.get(('base', rt)), d.get(('lu', rt))
                if not (o and b and l) or 'error' in o: continue
                r = o['e_cw']/b['e_cw']; rl = o['e_cw']/l['e_cw']
                worst.append((r, pb, rt)); worst_lu.append((rl, pb, rt))
                if r > 1.5: fails.append(f'{pb}@{rt:g}:{r:.2f}')
        w = max(worst); wl = max(worst_lu)
        print(f"{a:9s} max/base {w[0]:5.2f} ({w[1]}@{w[2]:g})  max/LU {wl[0]:5.2f} ({wl[1]}@{wl[2]:g})  n>1.5x base: {len(fails)}  {' '.join(fails[:8])}")

def tableD(arms, refs=('base', 'proj', 'l2c'), probs=PROBS, rts=(1e-3, 1e-6, 1e-8, 1e-10, 1e-11)):
    print('\n### D. cost at equal rtol: arm/ref ratios of JVP, orthogonalization dots, admitted flops (analytic JVP model)')
    for pb in probs:
        d = D(pb)
        for a in arms:
            line = f'{pb:8s} {a:8s}'
            for rt in rts:
                o = d.get((a, rt))
                if not o: continue
                parts = []
                for rf in refs:
                    b = d.get((rf, rt))
                    if not b: continue
                    parts.append(f"{o['jvp']/b['jvp']:.2f}/{o['dots']/max(b['dots'],1):.2f}/{o['flops']/b['flops']:.2f}")
                line += f' | {rt:g}: ' + ' '.join(parts)
            print(line)

def tableE(arms, refs=('base', 'l2c', 'proj'), Es=None, wkeys=('jvp', 'flops')):
    print('\n### E. matched accuracy: work(arm)/work(ref) at endpoint error E; frontier = log-log regression over the ladder '
          '(x = extrapolated), cheapest = harness cheapest-run-reaching-E rule')
    for pb in PROBS:
        d = D(pb)
        errs = [o['e_cw'] for (k, rt), o in d.items() if k == 'base']
        lo, hi = min(errs), max(errs)
        Egrid = Es or [10.0**float(x) for x in np.arange(math.ceil(math.log10(lo)), math.floor(math.log10(hi)) + 1)]
        for a in arms:
            for rf in refs:
                for wk in wkeys:
                    cells = []
                    for E in Egrid:
                        fr, ex = frontier_ratio(pb, a, rf, E, wk); ch = cheapest_ratio(pb, a, rf, E, wk)
                        cells.append(f"{E:.0e}: {('%.2f' % fr) if fr else '-'}{'x' if ex else ''}/{('%.2f' % ch) if ch else '-'}")
                    print(f"{pb:8s} {a:7s} vs {rf:5s} [{wk:5s}] " + '  '.join(cells))

def tableF(arms, ref='lu', probs=('hires', 'bruss50')):
    print('\n### F. contamination |y_arm - y_LU| (componentwise, rtol units) at the endpoint; "-" = different step sequence')
    for pb in probs:
        for a in arms:
            c = contamination(pb, a, ref)
            print(f"{pb:8s} {a:8s} " + ' '.join(f"{rt:g}:{('%.3f' % v) if v is not None else '-':>7s}" for rt, v in c.items()))

def ladder_table(arms):
    rows = {}
    for l in open('res/ladders.jsonl'):
        o = json.loads(l); key = (o['ladder'], o['rtol'], o['k'])
        rows.setdefault(key, {}).update({k: v for k, v in o.items() if isinstance(v, dict)})
    print('\n### H. fixed-step ladders: error ratio arm/direct-LU (rel max-norm; semilin64 also outer WRMS); PASS rule: '
          'tracking <= 3x LU + 1e-13 (diagpr128, semilin128); semilin64: WRMS <= max(3 LU, 1); pr_tight: |dWRMS| <= 1e-6 max(LU,1)')
    for key in sorted(rows):
        r = rows[key]; lu = r.get('lu')
        if not lu: continue
        cells = []
        for a in arms:
            o = r.get(a)
            if not o or 'error' in o: cells.append(f'{a}:-'); continue
            ratio = o['rel']/lu['rel']
            if key[0] in ('diagpr128', 'semilin128'):
                ok = o['rel'] <= 3*lu['rel'] + 1e-13
            elif key[0] == 'semilin64':
                ok = o['wrms'] <= max(3*lu['wrms'], 1.0)
            else:
                ok = abs(o['wrms'] - lu['wrms']) <= 1e-6*max(lu['wrms'], 1.0)
            cells.append(f"{a}:{ratio:.2f}{'' if ok else '!'}")
        print(f"{key[0]:10s} rtol={key[1]:g} k={key[2]} LU rel={lu['rel']:.2e} | " + ' '.join(cells))
    return rows

def slopes(rows, arm, ladder='semilin64', rtol=1e-6, floor=1e-12):
    errs = [rows[(ladder, rtol, k)][arm]['wrms']*rtol for k in range(3, 9) if (ladder, rtol, k) in rows and arm in rows[(ladder, rtol, k)]]
    return [math.log2(errs[i]/errs[i+1]) for i in range(len(errs) - 1) if errs[i+1] > floor], errs

def tableF2(arms, exclude=(('robertson', 1e-11),)):
    print('\n### F2. robust gate: max over cells of contamination(arm vs LU)/err_base (same step sequence only); '
          '<= 0.5 guarantees err <= 1.5x base whatever the sign; n_diff = cells with a different step sequence than LU')
    for a in arms:
        worst = (0, None, None); ndiff = 0; ncell = 0
        for pb in PROBS:
            d = D(pb)
            c = contamination(pb, a, 'lu')
            for rt, v in c.items():
                if (pb, rt) in exclude: continue
                ncell += 1
                if v is None: ndiff += 1; continue
                b = d.get(('base', rt))
                r = v*rt/b['e_cw']
                if r > worst[0]: worst = (r, pb, rt)
        print(f"{a:9s} max contamination/err_base = {worst[0]:.3f} ({worst[1]}@{worst[2]})  n_diff={ndiff}/{ncell}")
