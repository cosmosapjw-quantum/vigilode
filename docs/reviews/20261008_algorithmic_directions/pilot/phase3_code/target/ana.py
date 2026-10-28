"""Analysis helpers: references, errors, tables, matched-accuracy (frontier + cheapest-run rule)."""
import json, glob, math, sys, numpy as np
sys.path.insert(0, '.')
import tprobs, rep
LD = np.longdouble

def ref(name):
    p = tprobs.PROBLEMS[name]()
    if p['exact'] is not None:
        return np.array(p['exact'](p['span'][1]), dtype=float), 0.0
    if name == 'bruss160':   # SPD07 BASE.json reference: dense fast driver at rtol 1e-12
        import struct
        B = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
        row = [r for r in B['rows'] if r['case'] == 'brusselator-1d-160'][0]
        return np.array([struct.unpack('>d', bytes.fromhex(x))[0] for x in row['reference']['y']]), None
    d = json.load(open(f'ref_{name}.json'))
    return np.array([float(LD(v)) for v in d['1e-15']['y_ld']]), d['uncertainty_componentwise']

def load(name, tag=''):
    rows = []
    for line in open(f'res/{name}{tag}.jsonl'):
        o = json.loads(line)
        if 'error' in o: rows.append(o); continue
        r, _ = ref(name)
        e_cw, e_nw, am = rep.err_metrics(np.array(o['y']), r)
        o['e_cw'] = e_cw; o['e_nw'] = e_nw; o['argmax'] = am
        o['ecw_rt'] = e_cw/o['rtol']; o['enw_rt'] = e_nw/o['rtol']
        rows.append(o)
    return rows

def latest(rows):
    """keep the last row per (arm, rtol)"""
    d = {}
    for o in rows: d[(o['arm'], o['rtol'])] = o
    return d

def table(name, arms=None, tag='', key='ecw_rt'):
    d = latest(load(name, tag))
    rts = sorted({k[1] for k in d}, reverse=True)
    arms = arms or sorted({k[0] for k in d})
    print(f'## {name}: {key} | JVP/acc | dots/acc')
    print('rtol      ' + ' '.join(f'{a:>26s}' for a in arms))
    for rt in rts:
        cells = []
        for a in arms:
            o = d.get((a, rt))
            if o is None: cells.append(' '*26); continue
            if 'error' in o: cells.append(f'{"ERR":>26s}'); continue
            cells.append(f"{o[key]:7.3f} {o['jvp']/o['acc']:6.1f} {o['dots']/o['acc']:7.0f} {o['acc']:4d}/{o['rej']:<2d}")
        print(f'{rt:<9g} ' + ' '.join(cells))

def frontier(rows, E, wkey='jvp'):
    """log-log regression of work vs endpoint error over the ladder -> work at error E; flags extrapolation."""
    pts = [(o['e_cw'], o[wkey]) for o in rows if 'error' not in o and o['e_cw'] > 0]
    if len(pts) < 2: return None, True
    x = np.log10([p[0] for p in pts]); y = np.log10([p[1] for p in pts])
    co = np.polyfit(x, y, 1)
    w = 10**np.polyval(co, math.log10(E))
    extrap = not (min(x) <= math.log10(E) <= max(x))
    return float(w), extrap

def cheapest(rows, E, wkey='jvp'):
    ok = [o[wkey] for o in rows if 'error' not in o and o['e_cw'] <= E]
    return (min(ok) if ok else None)

if __name__ == '__main__':
    for name in sys.argv[1].split(','):
        table(name, sys.argv[2].split(',') if len(sys.argv) > 2 else None, tag=(sys.argv[3] if len(sys.argv) > 3 else ''))
