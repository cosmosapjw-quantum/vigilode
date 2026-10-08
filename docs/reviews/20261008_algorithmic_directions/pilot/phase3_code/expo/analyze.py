"""PROBE B3 analysis (EXPLORATORY, not ledger authority).
Rows: E arms (res/e_runs.jsonl, res/e_extra.jsonl) + RODAS5P arms 'mf' (improved: proj-stop + coupled WRMS target + stall
rule + maxit 2000) and 'base' (SPD07 production MF) from PROBE B2 (res/reg_runs_copy.jsonl; bit-identical with my copy,
res/fid_rodas.json) and my own RODAS rows (res/e_runs.jsonl / res/e_extra.jsonl arms base/mf).
All endpoint errors are rescored from the stored final states against one reference per problem (corpus rob/hires/vdp/forc:
direct RODAS5P rtol 1e-13 (B2 refs); rot/semi/advdiff: exact; bruss1d-50: NATIVE.json; bruss1d-160: B2 refs).
Matched accuracy, two rules (as B2):
  [F] regression frontier: local linear fit of log10(flops) on log10(err) over the 5 ladder points nearest to E ('x' = E outside
      the arm's measured error range, extrapolated);
  [C] harness cheapest-run rule: cheapest run with err <= E ('NR' none reaches E).
Targets E_k = error of the improved arm 'mf' at rtol 1e-4, 1e-6, 1e-8, 1e-10 (+1e-9).  Ratios E/mf and E/base (flops), and
JVP-only ratio (operator applications + remainder JVPs vs mf JVPs) for the JVP-count view."""
import json, math, os, sys, collections
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))


def kround(rt):
    return round(math.log10(rt) * 2) / 2


def load(files=('res/e_runs.jsonl', 'res/e_extra.jsonl', 'res/e_seed.jsonl', 'res/e_ef.jsonl', 'res/e_mc.jsonl', 'res/e_riv.jsonl')):
    import run_e
    rows = []
    for l in open(os.path.join(HERE, 'res/reg_runs_copy.jsonl')):
        d = json.loads(l)
        if d.get('ok') and d['arm'] in ('base', 'mf', 'direct', 'rock4'):
            d['src'] = 'B2'; rows.append(d)
    for l in open(os.path.join(HERE, 'res/reg_runs_sw3_copy.jsonl')):     # B2 final guarded switched MF integrator
        d = json.loads(l)
        if d.get('ok') and d['arm'] == 'sw':
            d['src'] = 'B2'; rows.append(d)
    for fn in files:
        fp = os.path.join(HERE, fn)
        if os.path.exists(fp):
            for l in open(fp):
                d = json.loads(l); d['src'] = 'B3'; rows.append(d)
    # own rows supersede B2 rows for the same (problem, arm, rtol); among own rows the last one wins
    key = lambda d: (d['problem'], d['arm'], kround(d['rtol']))
    last = {}
    for i, d in enumerate(rows):
        if d['src'] == 'B3' and d.get('ok'):
            last[key(d)] = i
    rows = [d for i, d in enumerate(rows) if not (d['src'] == 'B3' and d.get('ok') and last[key(d)] != i)]
    own = {key(d) for d in rows if d['src'] == 'B3' and d.get('ok')}
    rows = [d for d in rows if not (d['src'] == 'B2' and key(d) in own)]
    D = collections.defaultdict(lambda: collections.defaultdict(list)); fails = []
    probs = {}
    for d in rows:
        if not d.get('ok'):
            fails.append(d); continue
        p = d['problem']
        if p not in probs:
            probs[p] = run_e.problem(p)
        P = probs[p]
        if 'y' in d and P['ref'] is not None:
            d['err'] = float(P['err'](np.asarray(d['y'])))
        if d.get('err') is None or not (d['err'] > 0) or not math.isfinite(d['err']):
            continue
        d['jvp_total'] = (d['kc']['ops'] + d['ec']['jvp_rem']) if 'kc' in d else d.get('jvp', 0)
        D[p][d['arm']].append(d)
    for p in D:
        for a in D[p]:
            D[p][a].sort(key=lambda r: -r['rtol'])
    return D, fails, probs


def frontier(runs, E, k=5, key=lambda r: r['flops']['total']):
    if len(runs) < 2:
        return None, True
    le = np.log10([r['err'] for r in runs]); lf = np.log10([key(r) for r in runs])
    x = math.log10(E)
    idx = np.argsort(np.abs(le - x))[:k]
    extrap = not (le.min() <= x <= le.max())
    if np.ptp(le[idx]) < 1e-9:
        return 10 ** float(np.mean(lf[idx])), True
    b, a = np.polyfit(le[idx], lf[idx], 1)
    return 10 ** (a + b * x), extrap


def cheapest(runs, E, key=lambda r: r['flops']['total']):
    ok = [key(r) for r in runs if r['err'] <= E]
    return min(ok) if ok else None


def ratio_cells(A, num, den, targets, key=lambda r: r['flops']['total']):
    out = []
    for k, E in targets:
        fn, xn = frontier(A.get(num, []), E, key=key); fd, xd = frontier(A.get(den, []), E, key=key)
        cn = cheapest(A.get(num, []), E, key=key); cd = cheapest(A.get(den, []), E, key=key)
        out.append(dict(k=k, E=E, F=(fn / fd if fn and fd else None), Fx=(xn or xd), C=(cn / cd if cn and cd else None)))
    return out


def fmt(c):
    f = 'NR' if c['F'] is None else f"{c['F']:.3f}" + ('x' if c['Fx'] else ' ')
    cc = 'NR' if c['C'] is None else f"{c['C']:.3f}"
    return f'{f:>7s}/{cc:>6s}'


ORDER = ['vdp2', 'hires8', 'rot-96', 'semi-96', 'forc-96', 'hires-96', 'rob-96', 'vdp-96', 'bruss1d-50', 'bruss1d-160', 'semi-384',
         'advdiff-128', 'rot-384', 'rot-1536', 'semi-1536', 'forc-384', 'forc-1536']


def main():
    D, fails, probs = load()
    S = {}
    print('=== failures:', len(fails))
    for f in fails[:30]:
        print('   ', f['problem'], f['arm'], f['rtol'], str(f.get('error', ''))[:160])
    for p in ORDER:
        if p not in D:
            continue
        A = D[p]
        mfr = {kround(r['rtol']): r for r in A.get('mf', [])}
        targets = [(k, mfr[float(k)]['err']) for k in (-4, -6, -8, -9, -10) if float(k) in mfr]
        if not targets:
            continue
        print(f'\n##### {p}  n={probs[p]["n"]}  err rule: {probs[p]["err_name"]}  ref: {probs[p]["ref_src"]}')
        print('  ladder (Mflop @ endpoint err) at rtol 1e-k')
        print(f'  {"arm":5s} ' + ' '.join(f'{"k=" + str(-k):>21s}' for k, _ in targets))
        for a in ('base', 'mf', 'direct', 'rock4', 'sw', 'E', 'E2', 'Ef', 'Em40', 'Em24', 'Em16', 'Em12', 'Emc', 'E2mc', 'Ed'):
            if a not in A:
                continue
            m = {kround(r['rtol']): r for r in A[a]}
            cells = []
            for k, _ in targets:
                r = m.get(float(k))
                if r is None:
                    cells.append(f'{"-":>21s}')
                elif a == 'Ed':
                    cells.append(f'{r["att"]:6d}att@{r["err"]:9.2e}  ')
                else:
                    cells.append(f'{r["flops"]["total"] / 1e6:9.3f}M@{r["err"]:9.2e}')
            print(f'  {a:5s} ' + ' '.join(f'{c:>21s}' for c in cells))
        print('  matched accuracy (targets = mf error at rtol 1e-k): ratio [F] frontier / [C] cheapest-run')
        print(f'  {"ratio":14s} ' + ' '.join(f'{"k=" + str(-k) + " E=" + format(E, ".1e"):>16s}' for k, E in targets))
        rs = {}
        for name, num, den, key in (('E/mf flops', 'E', 'mf', None), ('E/base flops', 'E', 'base', None),
                                    ('E2/mf flops', 'E2', 'mf', None), ('Ef/mf flops', 'Ef', 'mf', None),
                                    ('Em40/mf flops', 'Em40', 'mf', None), ('Em24/mf flops', 'Em24', 'mf', None),
                                    ('E2m40/mf flops', 'E2m40', 'mf', None), ('Em16/mf flops', 'Em16', 'mf', None),
                                    ('Em12/mf flops', 'Em12', 'mf', None), ('Emc/mf flops', 'Emc', 'mf', None),
                                    ('E2mc/mf flops', 'E2mc', 'mf', None), ('Emc/base flops', 'Emc', 'base', None),
                                    ('E2mc/base flops', 'E2mc', 'base', None),
                                    ('Emc/direct', 'Emc', 'direct', None), ('Emc/rock4', 'Emc', 'rock4', None),
                                    ('mf/direct', 'mf', 'direct', None), ('E/sw', 'E', 'sw', None), ('Emc/sw', 'Emc', 'sw', None),
                                    ('sw/mf', 'sw', 'mf', None), ('E/rock4', 'E', 'rock4', None), ('E/direct', 'E', 'direct', None),
                                    ('mf/base flops', 'mf', 'base', None),
                                    ('E/mf JVPs', 'E', 'mf', lambda r: r['jvp_total']),
                                    ('E/mf attempts', 'E', 'mf', lambda r: r['att']),
                                    ('Ed/mf attempts', 'Ed', 'mf', lambda r: r['att'])):
            if num not in A or den not in A:
                continue
            kw = {} if key is None else dict(key=key)
            cells = ratio_cells(A, num, den, targets, **kw)
            rs[name] = cells
            print(f'  {name:14s} ' + ' '.join(f'{fmt(c):>16s}' for c in cells))
        # flop decomposition of E at rtol 1e-6 / 1e-8
        for kk in (-6, -8):
            m = {kround(r['rtol']): r for r in A.get('E', [])}
            mm = {kround(r['rtol']): r for r in A.get('mf', [])}
            if float(kk) in m and float(kk) in mm:
                fe = m[float(kk)]['flops']; fm = mm[float(kk)]['flops']
                tot = fe['total']
                print(f'  E  parts at 1e{kk}: ' + ', '.join(f'{q} {fe[q] / tot:.2f}' for q in ('jvp', 'aug', 'orth', 'out', 'dense', 'rhs', 'vec')) +
                      f'   ops/att {m[float(kk)]["kc"]["ops"] / m[float(kk)]["att"]:.1f}, expms/att {m[float(kk)]["kc"]["expms"] / m[float(kk)]["att"]:.2f}')
                tm = fm['total']
                print(f'  mf parts at 1e{kk}: ' + ', '.join(f'{q} {fm[q] / tm:.2f}' for q in ('jvp', 'orth', 'rhs', 'asm')) +
                      f'   JVP/att {mm[float(kk)]["jvp"] / mm[float(kk)]["att"]:.1f}')
        P = probs[p]
        if 'E' in A and 'mf' in A:
            cells = ratio_cells(A, 'E', 'mf', targets, key=lambda r: r['flops']['total'] - r['flops'].get('dense', 0.0))
            rs['E(no dense)/mf'] = cells
            print(f'  {"E-dense/mf":14s} ' + ' '.join(f'{fmt(c):>16s}' for c in cells))
        for num in ('E', 'E2'):
            seeds = []
            for fac in ('0.3333', None, '3'):
                a1 = num if fac is None else f'{num}:{fac}'; a2 = 'mf' if fac is None else f'mf:{fac}'
                if a1 in A and a2 in A:
                    seeds.append(ratio_cells(A, a1, a2, targets))
            if len(seeds) == 3:
                rs[f'{num}/mf seeds'] = seeds
                cells = []
                for i in range(len(targets)):
                    Fv = [sd[i]['F'] for sd in seeds if sd[i]['F']]; Cv = [sd[i]['C'] for sd in seeds if sd[i]['C']]
                    cells.append((f'{min(Fv):.2f}-{max(Fv):.2f}' if Fv else 'NR') + '/' + (f'{min(Cv):.2f}-{max(Cv):.2f}' if Cv else 'NR'))
                print(f'  {num + "/mf 3 h0":14s} ' + ' '.join(f'{c:>16s}' for c in cells))
        def kcost(kap, P=P):
            def f(r):
                fl = r['flops']['total']
                if 'kc' in r:
                    ev = (r['kc']['ops'] + r['ec']['jvp_rem']) * P['F_jvp'] + r['ec']['rhs'] * P['F_rhs'] + r['ec']['ft'] * P['F_ft']
                else:
                    ev = r['jvp'] * P['F_jvp'] + r['rhs'] * P['F_rhs'] + r['ft'] * P['F_ft']
                return fl + (kap - 1.0) * ev
            return f
        for kap in (3.0, 10.0, 30.0):
            for num in ('E', 'E2'):
                if num in A and 'mf' in A:
                    cells = ratio_cells(A, num, 'mf', targets, key=kcost(kap))
                    rs[f'{num}/mf k{kap:g}'] = cells
                    print(f'  {num + "/mf F x" + format(kap, "g"):14s} ' + ' '.join(f'{fmt(c):>16s}' for c in cells))
        S[p] = dict(targets=targets, ratios=rs,
                    ladder={a: [dict(rtol=r['rtol'], att=r['att'], rej=r.get('rej'), err=r['err'], flops=r['flops']['total'],
                                     jvp=r['jvp_total'], parts=r['flops']) for r in A[a]] for a in A})
    json.dump(S, open(os.path.join(HERE, 'res/summary.json'), 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
