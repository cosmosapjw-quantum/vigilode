"""Probe B1 tables (writes to stdout; tee to tables.txt)."""
import json, math, os, sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import ana, run, stack

PROBS = ['bruss50', 'bruss160', 'bruss300', 'hires', 'robertson', 'vdp', 'pr', 'quad4']
D = {pb: ana.load(pb) for pb in PROBS}
CELLS = [(pb, k) for pb in ('bruss50', 'bruss160', 'hires', 'robertson', 'vdp', 'pr', 'quad4') for k in (12, 16)] + \
        [('hires', 18), ('hires', 20), ('robertson', 18), ('robertson', 20), ('bruss160', 8), ('bruss300', 8), ('bruss300', 12)]
STACK = ['A0', 'A1', 'A2', 'A3', 'A4a', 'A4b', 'A5a', 'A5b', 'A6']
RIVALS = ['R3', 'R5', 'R3U', 'Rbig', 'C0G']
CTRL = ['C4a', 'C4b', 'C3P', 'C2P', 'C0P', 'C3G', 'C6ng', 'LU', 'LUP']
TWIN = {a: ('LUP' if run.ARMS.get(a, {}).get('ctrl', 'I') == 'PRED+cap' else 'LU') for a in run.ARMS}
TWIN['A1'] = 'LU'
for _a in ('S', 'SU', 'Sfg8', 'Sfg9', 'Sfs'): TWIN[_a] = 'LUP' if _a != 'SU' else 'LU'


def jflop(pb):
    p = run.problem(pb); n = len(p['y0'])
    return 2*p['nnzJ'] + 2*n, 2*p['nnzJ'] + 4*n, p['frhs'] + 6*n, p['frhs'] + 8*n, n


def sec(t):
    print('\n' + '='*len(t) + '\n' + t + '\n' + '='*len(t))


def t_flopmodel():
    sec('T0. JVP flop model per problem (state flops per operator application)')
    print('problem    n    analytic L2   analytic scaled(D W D^-1)   FD L2   FD scaled   frhs')
    for pb in PROBS:
        a, s, f, fs, n = jflop(pb)
        print(f'{pb:9s} {n:4d}  {a:8d}      {s:8d}                 {f:7d}  {fs:7d}   {run.problem(pb)["frhs"]}')
    print('dot/axpy/norm = 2n, scal = n, stage assembly axpy = 2n, RHS = frhs, dense LU 2n^3/3, LU solve 2n^2;'
          ' preconditioner applications = 0 and LU factorizations = 0 in every matrix-free arm.')


def t_cells(arms):
    sec('T1. Equal-rtol cells: att/acc/rej/linfail | JVP | RHS | dots | axpys | kflop | err/rtol | err/err_A0')
    for pb, k in CELLS:
        d = D[pb]; b = d.get(('A0', k))
        if not ana.ok(b): continue
        print(f'-- {pb} rtol={10**(-k/2):.3g}  (A0 err {b["e_cw"]:.3e})')
        for a in arms:
            o = d.get((a, k))
            if o is None: continue
            if 'error' in o: print(f'   {a:5s} ERROR {o["error"]}'); continue
            print(f"   {a:5s} {o['att']:5d}/{o['acc']:5d}/{o['rej']:3d}/{o['lin_fail']:3d} | {o['jvp']:8d} | {o['rhs_tot']:6d} | {o['dots']:9d} | "
                  f"{o['axpys']:9d} | {o['flops']/1e3:10.0f} | {o['ecw_rt']:7.3f} | {o['e_cw']/b['e_cw']:5.2f}"
                  + (f"  guard q/over/false {o['guard_q']}/{o['guard_over']}/{o['guard_false']}" if o.get('guard_q') or o.get('guard_over') else '')
                  + (f"  stalls {o['stalls']}" if o.get('stalls') else ''))


def contamination(d, a, k):
    o = d.get((a, k)); tw = d.get((TWIN.get(a, 'LU'), k))
    if not (ana.ok(o) and ana.ok(tw)): return None
    if (o['acc'], o['rej']) != (tw['acc'], tw['rej']): return 'seq'
    ya, yb = np.array(o['y']), np.array(tw['y'])
    return float(np.max(np.abs(ya - yb)/np.maximum(np.abs(yb), 1e-10)))


def t_gate(arms, kmin=6, kmax=20):
    sec('T2. Accuracy gate over every ladder rung (rtol 1e-3..1e-10, half-decades; Bruss-160 1e-4..1e-8; Bruss-300 1e-4..1e-7)\n'
        '    endpoint: err/err_A0 (max, #cells > 1.5x); robust: |y - y_twin|/err_A0 where the exact-solve twin (LU, or LUP for PRED arms)\n'
        '    has the same accept/reject sequence (<= 0.5 guarantees <= 1.5x whatever the sign); ctrl: err_twin/err_LU (controller effect)')
    for a in arms:
        worst = []; fails = []; rob = []; nseq = 0; ncell = 0; ctrl_fx = []
        for pb in PROBS:
            d = D[pb]
            for k in range(kmin, kmax + 1):
                o, b = d.get((a, k)), d.get(('A0', k))
                if not (ana.ok(o) and ana.ok(b)): continue
                ncell += 1
                r = o['e_cw']/b['e_cw']; worst.append((r, pb, k))
                if r > 1.5: fails.append(f'{pb}@{10**(-k/2):.2g}:{r:.2f}')
                c = contamination(d, a, k)
                if c == 'seq': nseq += 1
                elif c is not None: rob.append((c/b['e_cw'], pb, k))
                tw, lu = d.get((TWIN.get(a, 'LU'), k)), d.get(('LU', k))
                if ana.ok(tw) and ana.ok(lu): ctrl_fx.append((tw['e_cw']/lu['e_cw'], pb, k))
        if not worst: continue
        w = max(worst); rw = max(rob) if rob else (0, None, None); cw = max(ctrl_fx) if ctrl_fx else (0, None, 0)
        print(f"{a:5s} cells={ncell:3d} max err/A0 {w[0]:5.2f} ({w[1]}@{10**(-w[2]/2):.2g})  >1.5x: {len(fails):2d}  "
              f"robust max {rw[0]:.3f} ({rw[1]}@{10**(-(rw[2] or 0)/2):.2g}) diff-seq {nseq}  twin/LU max {cw[0]:.2f}")
        if fails: print('       ' + ' '.join(fails))


def t_gate2(arms, kmin=6, kmax=20):
    sec('T2b. Gate variants: (i) vs the failure-free production reference (Rbig = maxit 2000 where run, else A0; identical where A0 has\n'
        '     no linear failures); (ii) vs the exact-solve twin with the same controller (LU / LUP): err_arm/err_twin, which removes the\n'
        '     controller set-point effect; cells listed when > 1.5x')
    for a in arms:
        r1, r2 = [], []; f1, f2 = [], []
        for pb in PROBS:
            d = D[pb]
            for k in range(kmin, kmax + 1):
                o = d.get((a, k))
                ref = d.get(('Rbig', k)) if ana.ok(d.get(('Rbig', k))) else d.get(('A0', k))
                tw = d.get((TWIN.get(a, 'LU'), k))
                if not (ana.ok(o) and ana.ok(ref)): continue
                x = o['e_cw']/ref['e_cw']; r1.append(x)
                if x > 1.5: f1.append(f'{pb}@{10**(-k/2):.2g}:{x:.2f}')
                if ana.ok(tw):
                    y = o['e_cw']/tw['e_cw']; r2.append(y)
                    if y > 1.5: f2.append(f'{pb}@{10**(-k/2):.2g}:{y:.2f}')
        if not r1: continue
        print(f"{a:6s} (i) max {max(r1):6.2f} n>1.5x {len(f1):2d} {' '.join(f1[:10])}")
        print(f"{'':6s} (ii) max {max(r2) if r2 else float('nan'):6.2f} n>1.5x {len(f2):2d} {' '.join(f2[:10])}")


def t_matched(arms, refs, wkeys=('jvp', 'flops'), probs=PROBS, detail=True):
    sec('T3. Matched accuracy: work(arm)/work(ref) at endpoint error E (decades within A0\'s error range).\n'
        '    cell = frontier (log-log regression over the rtol ladder; x = extrapolated for either arm) / cheapest-run rule;\n'
        '    summary = geo-mean [range] of in-range frontier ratios | geo-mean [range] of cheapest-run ratios')
    for pb in probs:
        d = D[pb]
        if not d: continue
        Es = ana.egrid_for(pb, d)
        for a in arms:
            for rf in refs:
                if a == rf or not ana.arm_rows(d, a) or not ana.arm_rows(d, rf): continue
                for wk in wkeys:
                    m = ana.matched(d, a, rf, wk, Es)
                    fs, cs, nin, nx = ana.summarize_matched(m)
                    line = f"{pb:9s} {a:5s} vs {rf:4s} [{wk:5s}] frontier {fs:22s} cheapest {cs:22s}"
                    if detail: line += '  | ' + ana.fmt_m(m)
                    print(line)


def matched_gm(pb, a, rf, wk='jvp'):
    d = D[pb]
    if not ana.arm_rows(d, a) or not ana.arm_rows(d, rf): return None, None
    m = ana.matched(d, a, rf, wk, ana.egrid_for(pb, d))
    fr = [r for (E, r, x, c) in m if r and not x]; ch = [c for (E, r, x, c) in m if c]
    return ana.gm(fr), ana.gm(ch)


def t_attrib(chain, wkeys=('jvp', 'flops')):
    sec('T4. Attribution: increment of each layer at matched accuracy (geo-mean of in-range frontier ratios / of cheapest-run ratios)\n'
        '    and at equal rtol on the SPD07 cells (1e-6 / 1e-8)')
    for wk in wkeys:
        print(f'-- work = {wk}')
        print('problem   ' + ' '.join(f'{a+"/"+b:>22s}' for a, b in chain))
        for pb in PROBS:
            cells = []
            for a, b in chain:
                fr, ch = matched_gm(pb, a, b, wk)
                d = D[pb]; eq = []
                for k in (12, 16):
                    oa, ob = d.get((a, k)), d.get((b, k))
                    if ana.ok(oa) and ana.ok(ob): eq.append(f"{oa[wk]/ob[wk]:.2f}")
                cells.append(f"{('%.2f' % fr) if fr else '-'}/{('%.2f' % ch) if ch else '-'} ({','.join(eq) or '-'})")
            print(f'{pb:9s} ' + ' '.join(f'{c:>22s}' for c in cells))


def t_robust():
    sec('T5. Budget-binding cells (linear failures fed back): att/acc/rej | linfail | guard q/over/false | JVP | kflop | err/rtol')
    for pb, ks in (('bruss160', range(8, 17)), ('bruss300', range(8, 15))):
        d = D[pb]
        for k in ks:
            if not ana.ok(d.get(('A0', k))): continue
            print(f'-- {pb} rtol={10**(-k/2):.3g}')
            for a in ['A0', 'Rbig', 'C0G', 'A2', 'A3', 'C3G', 'A4a', 'A5a', 'C6ng', 'A6', 'C3P', 'LU']:
                o = d.get((a, k))
                if not ana.ok(o): continue
                print(f"   {a:5s} {o['att']:5d}/{o['acc']:5d}/{o['rej']:3d} | {o['lin_fail']:4d} | {o['guard_q']}/{o['guard_over']}/{o['guard_false']} | "
                      f"{o['jvp']:8d} | {o['flops']/1e3:10.0f} | {o['ecw_rt']:.3f}")


def t_noise():
    fn = os.path.join(HERE, 'res', 'noise.jsonl')
    if not os.path.exists(fn): return
    sec('T6. Controller noise from inexact solves (|err/err_exact - 1| over all attempts; flips = accept/reject decisions changed)')
    rows = [json.loads(l) for l in open(fn)]
    for o in rows:
        print(f"{o['prob']:9s} {o['arm']:5s} rtol={o['rtol']:<8.2g} att={o['att']:5d} rej={o['rej']:4d} lf={o['lin_fail']:3d} JVP={o['jvp']:7d} "
              f"noise med {o['noise_med']:.1e} p90 {o['noise_p90']:.1e} max {o['noise_max']:.1e} >10%: {o['frac_gt_0p1']:.3f} flips {o['flips']} "
              f"dy med/max {o['dy_med']:.1e}/{o['dy_max']:.1e}")


def t_fd():
    sec('T7. FD-JVP arms on Bruss-50: counts, gap statistics (tp = solver true/projected at in-cycle exits; ex = exact-J true/threshold;\n'
        '    et = exact-J true / solver (FD) true), error, FD-model kflops')
    d = D['bruss50']
    for k in range(6, 21):
        rows = [(a, d.get((a, k))) for a in ('A0f', 'A2f', 'A3f', 'A5af', 'A0f8s', 'A2f8s', 'A3fg', 'A5afg', 'A5ag', 'Sfs', 'Sfg9', 'Sfg8', 'S', 'A0')]
        rows = [(a, o) for a, o in rows if o is not None and 'error' not in o]
        if not rows: continue
        print(f'-- rtol={10**(-k/2):.3g}')
        for a, o in rows:
            g = o.get('gap') or {}
            gs = (f"tp>10 {g.get('tp_gt10')}/{g.get('n_proj_exits')} tp max {g.get('tp_max', 0):.1e} | ex>1 {g.get('ex_gt1')} ex max {g.get('ex_max', 0):.1e} | "
                  f"et med {g.get('et_med', 0):.2f} max {g.get('et_max', 0):.2e}") if g else ''
            print(f"   {a:6s} {'CAPPED ' if o.get('capped') else ''}{o['att']:5d}/{o['acc']:5d}/{o['rej']:3d} lf {o['lin_fail']:4d} stalls {o['stalls']:4d} JVP {o['jvp']:7d} "
                  f"RHS+FD {o['rhs_tot']:7d} kflopFD {o['flops_fd']/1e3:9.0f} err/rtol {o['ecw_rt'] if not o.get('capped') else float('nan'):.3f} {gs}")


if __name__ == '__main__':
    t_flopmodel()
    t_cells(STACK + ['S', 'SU'] + RIVALS + CTRL)
    t_gate(STACK + ['S', 'SU'] + RIVALS + CTRL + ['X02I', 'X02P', 'XrI', 'XrP', 'A3fg', 'A5afg', 'A5ag', 'A0f8s', 'A2f8s', 'Sfg8', 'Sfg9'])
    t_gate2(STACK + ['S', 'SU'] + RIVALS + CTRL + ['A3fg', 'A5afg', 'A5ag', 'A0f8s', 'A2f8s', 'Sfg8', 'Sfg9', 'Sfs'])
    t_attrib([('A1', 'A0'), ('A2', 'A1'), ('A3', 'A2'), ('A4a', 'A3'), ('A5a', 'A4a'), ('A6', 'A5a'), ('A5a', 'A0'), ('A5a', 'A2'),
              ('A6', 'A0'), ('A6', 'A2')])
    t_attrib([('S', 'A0'), ('S', 'A1'), ('S', 'A2'), ('S', 'Rbig'), ('S', 'R3'), ('S', 'SU'), ('S', 'A6'), ('C3P', 'A3'), ('C3G', 'A3'),
              ('SU', 'A3')])
    t_attrib([('A4b', 'A3'), ('A5b', 'A4b'), ('C4a', 'A2'), ('A3', 'R3'), ('C3P', 'A3'), ('C2P', 'A2'), ('C0P', 'A0'), ('A5a', 'R5'),
              ('R3U', 'A3'), ('C3P', 'R3U'), ('A5b', 'A0'), ('X02P', 'X02I'), ('XrP', 'XrI')])
    t_attrib([('Sfg8', 'A0f8s'), ('Sfg8', 'A2f8s'), ('Sfg8', 'S'), ('Sfg9', 'A0f8s'), ('A2f8s', 'A0f8s')], wkeys=('jvp', 'flops_fd'))
    t_matched(['A1', 'A2', 'A3', 'R3', 'A4a', 'A4b', 'A5a', 'A5b', 'A6', 'S', 'SU', 'C3P', 'R5'], ['A0', 'A2', 'Rbig'], detail=True)
    t_robust()
    t_noise()
    t_fd()
