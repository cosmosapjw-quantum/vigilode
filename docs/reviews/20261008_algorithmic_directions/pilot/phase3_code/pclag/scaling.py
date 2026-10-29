"""N scaling of matched-accuracy cost ratios (arm / direct), crossover extrapolation, and the kappa = F_LU / S
rule. EXPLORATORY. usage: python3 scaling.py [w]   (w = F (flops, default), Ffd (FD-JVP flop model), T (time model))"""
import json, math, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze import load, arm_rows, frontier, cheapest

W = sys.argv[1] if len(sys.argv) > 1 else 'F'
FAMS = {'bruss2d (alpha 0.02)': [(32, 'bruss2d-32'), (64, 'bruss2d-64'), (128, 'bruss2d-128'), (256, 'bruss2d-256')],
        'bruss2d (alpha 0.1)': [(64, 'bruss2d-64-a0.1'), (128, 'bruss2d-128-a0.1')],
        'semilin2d': [(32, 'semilin2d-32'), (64, 'semilin2d-64'), (128, 'semilin2d-128'), (256, 'semilin2d-256')]}
ES = [1e-5, 1e-6, 1e-7, 1e-8]
MB = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'microbench.json'))) \
    if os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'microbench.json')) else {}


def time_model(pn, o):
    """T = counts x per-op single-thread wall times (microbench.json)."""
    m = MB.get(pn)
    if m is None: return None
    arm = o['arm']
    lin = arm.startswith('lin')
    t = 0.0
    nfac = o['lus']
    t += nfac * (m['t_facl'] if lin else m['t_fac'])
    nsol = (o['solves'] if arm == 'direct' else 0) + (o['pcs'] if arm.startswith(('lag', 'lin')) else 0)
    t += nsol * (m['t_soll'] if lin else m['t_sol'])
    if arm == 'dst': t += o['pcs'] * m['t_dst']
    t += o['jvp'] * m['t_op'] + o['cjvp'] * m['t_spmv']
    t += (o['dots'] + o['axpys'] + o['norms']) * m['t_vec'] + (o['rhs'] + o['ft']) * m['t_rhs'] + o['asm_axpys'] * m['t_vec']
    return t


def ratios(pn, arm, w):
    rows = load(pn)
    if not rows: return None
    for o in rows.values():
        if 'error' not in o:
            o['T'] = time_model(pn, o)
    unc = [o.get('ref_unc') for o in rows.values() if o.get('ref_unc')]
    minerr = 30 * max(unc) if unc else 0.0
    ra, rd = arm_rows(rows, arm, minerr), arm_rows(rows, 'direct', minerr)
    if len(ra) < 2 or len(rd) < 2: return None
    if w == 'T' and any(o.get('T') is None for o in ra + rd): return None
    out = []
    for E in ES:
        fa, xa = frontier(ra, E, w); fd, xd = frontier(rd, E, w)
        ca, cd = cheapest(ra, E, w), cheapest(rd, E, w)
        out.append((E, fa / fd, xa or xd, (ca / cd) if ca and cd else None))
    return out


def main():
    print(f'### matched-accuracy cost ratio arm/direct vs N, work = {W}; per E: R (frontier; x = extrapolated) / C (cheapest-run)')
    summary = {}
    for fam, lst in FAMS.items():
        for arm in ('lag', 'lin', 'dst', 'mf', 'lag-w4', 'lin-w4', 'lag-rho'):
            pts = []
            for N, pn in lst:
                r = ratios(pn, arm, W)
                if r is None: continue
                gmR = math.exp(np.mean([math.log(x[1]) for x in r]))
                pts.append((N, gmR, r))
            if not pts: continue
            print(f'  {fam:20s} {arm:7s} ' + ' | '.join(
                f'N={N}: gm {g_:.3f} [' + ' '.join(f'{x[1]:.2f}{"x" if x[2] else ""}/{x[3]:.2f}' if x[3] else f'{x[1]:.2f}{"x" if x[2] else ""}/-' for x in r) + ']'
                for N, g_, r in pts))
            if len(pts) >= 2:
                Ns = np.array([p[0] for p in pts], float); gs = np.array([p[1] for p in pts])
                b, a = np.polyfit(np.log2(Ns), np.log2(gs), 1)      # log2 ratio = a + b log2 N
                n1 = 2 ** (-a / b) if b < 0 else float('inf')
                n07 = 2 ** ((math.log2(0.7) - a) / b) if b < 0 else float('inf')
                print(f'  {"":20s} {"":7s} fit: ratio ~ N^{b:.2f} (x{2**b:.2f} per doubling); ratio = 1 at N ~ {n1:.0f}; ratio = 0.7 at N ~ {n07:.0f}'
                      + ('  (extrapolated beyond measured N)' if max(n1, n07) > Ns.max() else ''))
                summary[(fam, arm)] = dict(slope=b, N1=n1, N07=n07, pts=[(p[0], p[1]) for p in pts])
    return summary


if __name__ == '__main__':
    main()
