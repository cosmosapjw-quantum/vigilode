"""PROBE B2 analysis (EXPLORATORY). Reads res/runs.jsonl (+ res/runs_sw2.jsonl, which supersedes the 'sw' rows
of the main file: same arm with the final guard start vector), writes res/summary.json and prints tables.

Matched accuracy, two rules:
  (F) regression frontier: local linear regression of log10(flops) on log10(err) over the 5 ladder points of the arm
      nearest (in log err) to the target E; flagged 'x' when E lies outside the arm's measured error range
      (extrapolation).
  (C) harness cheapest-run rule: cheapest run of the arm with err <= E ('NR' when no run of the arm reaches E).
Targets E_k = endpoint error of the DIRECT arm at rtol 1e-4, 1e-6, 1e-8, 1e-10 (what the strongest cheap rival
delivers at those tolerances)."""
import json, math, os, sys, collections
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ARMS = ['base', 'mf', 'direct', 'rock4', 'sw', 'swng', 'swd', 'swd-r2', 'sw-ns', 'swd-ns', 'swd-r2-ns']


def load():
    rows = [json.loads(l) for l in open(os.path.join(HERE, 'res/runs.jsonl'))]
    pb = os.path.join(HERE, 'res/runs_b64.jsonl')
    if os.path.exists(pb):
        rows += [json.loads(l) for l in open(pb)]
    # switched arms: runs_sw3 (settle rule, primary) supersede the main-file rows; runs_sw2 (no settle rule) are kept
    # as the sensitivity arms '<arm>-ns'
    p2 = os.path.join(HERE, 'res/runs_sw2.jsonl')
    p3 = os.path.join(HERE, 'res/runs_sw3.jsonl')
    sw_arms = ('sw', 'swng', 'swd', 'swd-r2')
    rows = [r for r in rows if r['arm'] not in sw_arms]
    if os.path.exists(p3):
        rows += [json.loads(l) for l in open(p3)]
    if os.path.exists(p2):
        for l in open(p2):
            r = json.loads(l); r['arm'] = r['arm'] + '-ns'; rows.append(r)
    # vdp-96 tight cells: the stored reference is off by 8.8e-6 tight units at the endpoint (res/corpus_refs.json),
    # so rtol <= 1e-8 was rerun with final states saved (res/runs_vdp.jsonl) and is rescored against the
    # direct-RODAS rtol 1e-13 reference (agrees with Radau 1e-13 to 2.2e-7)
    pv = os.path.join(HERE, 'res/runs_vdp.jsonl')
    if os.path.exists(pv):
        import pr as _pr
        refv = np.load(os.path.join(HERE, 'refs/vdp-96-endpoint.npz'))['r13']
        new = [json.loads(l) for l in open(pv)]
        for r in new:
            if r.get('ok') and 'y' in r:
                r['err_stored_ref'] = r['err']; r['err'] = _pr.tight_wrms(np.asarray(r['y']), refv)
        keys = {(r['problem'], r['arm'], round(math.log10(r['rtol']) * 2) / 2) for r in new}
        rows = [r for r in rows if (r['problem'], r['arm'], round(math.log10(r['rtol']) * 2) / 2) not in keys] + new
    D = collections.defaultdict(lambda: collections.defaultdict(list))
    fails = []
    import pr
    cm = json.load(open(os.path.join(HERE, 'res/costmodel.json')))
    COL = {}
    for r in rows:
        # post-hoc J-build correction: runs made before the structural colouring fix used colours at y0
        if r.get('ok') and r.get('jbuilds', 0) > 0:
            if r['problem'] not in COL:
                COL[r['problem']] = pr.build(r['problem'])['colors']
            used = r['cjvp'] / r['jbuilds']
            dc = COL[r['problem']] - used
            if abs(dc) > 1e-9:
                add = dc * r['jbuilds'] * cm[r['problem']]['F_jvp']
                r['flops']['cjvp'] += add; r['flops']['total'] += add; r['colors_fix'] = dc
    for r in rows:
        if not r.get('ok'):
            fails.append(r); continue
        if r.get('err') is None or not (r['err'] > 0) or not math.isfinite(r['err']):
            continue
        D[r['problem']][r['arm']].append(r)
    for p in D:
        for a in D[p]:
            D[p][a].sort(key=lambda r: -r['rtol'])
    return D, fails


def frontier(runs, E, k=5):
    if len(runs) < 2:
        return None, True
    le = np.log10([r['err'] for r in runs]); lf = np.log10([r['flops']['total'] for r in runs])
    x = math.log10(E)
    idx = np.argsort(np.abs(le - x))[:k]
    if np.ptp(le[idx]) < 1e-9:
        return 10 ** float(np.mean(lf[idx])), True
    b, a = np.polyfit(le[idx], lf[idx], 1)
    extrap = not (le.min() <= x <= le.max())
    return 10 ** (a + b * x), extrap


def cheapest(runs, E):
    ok = [r['flops']['total'] for r in runs if r['err'] <= E]
    return (min(ok) if ok else None)


def fmt_ratio(num, den, xn=False, xd=False):
    if num is None or den is None:
        return '   NR  '
    s = f'{num / den:7.3f}'
    if xn or xd:
        s += 'x'
    else:
        s += ' '
    return s


def main():
    D, fails = load()
    order = ['semi-96', 'semi-384', 'forc-96', 'rot-96', 'hires-96', 'rob-96', 'vdp-96', 'bruss1d-50', 'bruss1d-160',
             'bruss2d-32', 'bruss2d-64', 'advdiff-128']
    summ = {}
    print('=== failures:', len(fails))
    for f in fails[:20]:
        print('   ', f['problem'], f['arm'], f['rtol'], f.get('error', '')[:150])
    for p in order:
        if p not in D:
            continue
        A = D[p]
        dr = {round(math.log10(r['rtol']) * 2) / 2: r for r in A.get('direct', [])}
        targets = []
        for k in (-4, -6, -8, -10):
            if float(k) in dr:
                targets.append((k, dr[float(k)]['err']))
        S = dict(targets=targets, rows={})
        print(f'\n##### {p}  (n={A["direct"][0]["n"] if A.get("direct") else "?"}, err rule: {next(iter(A.values()))[0]["err_name"]})')
        # ladder table
        print(f'  {"arm":6s} ' + ' '.join(f'{"rtol=1e" + str(k):>22s}' for k, _ in targets))
        for a in ARMS:
            if a not in A:
                continue
            m = {round(math.log10(r['rtol']) * 2) / 2: r for r in A[a]}
            cells = []
            for k, _ in targets:
                r = m.get(float(k))
                cells.append(f'{r["flops"]["total"] / 1e6:9.3f}M@{r["err"]:9.2e}' + (f'h{r.get("n_handoff", 0)}' if a.startswith('sw') else '  ') if r else f'{"-":>22s}')
            print(f'  {a:6s} ' + ' '.join(f'{c:>22s}' for c in cells))
        # matched-accuracy ratios
        print(f'  matched accuracy at E = direct-arm error at rtol 1e-k   [F] local-regression frontier ratio ("x" = extrapolated), [C] cheapest-run ratio')
        hdr = f'  {"ratio":16s} ' + ' '.join(f'{"k=" + str(-k) + " E=" + format(E, ".1e"):>20s}' for k, E in targets)
        print(hdr)
        pairs = [('sw/direct', 'sw', 'direct'), ('sw/mf', 'sw', 'mf'), ('sw/base', 'sw', 'base'), ('rock4/direct', 'rock4', 'direct'),
                 ('swng/direct', 'swng', 'direct'), ('sw/rock4', 'sw', 'rock4'), ('swd/direct', 'swd', 'direct'),
                 ('swd-r2/direct', 'swd-r2', 'direct'), ('mf/direct', 'mf', 'direct'), ('base/direct', 'base', 'direct'),
                 ('rock4/mf', 'rock4', 'mf'), ('sw-ns/direct', 'sw-ns', 'direct'), ('swd-r2-ns/direct', 'swd-r2-ns', 'direct')]
        for lab, a1, a2 in pairs:
            if a1 not in A or a2 not in A:
                continue
            cells = []; rec = []
            for k, E in targets:
                f1, x1 = frontier(A[a1], E); f2, x2 = frontier(A[a2], E)
                c1 = cheapest(A[a1], E); c2 = cheapest(A[a2], E)
                cells.append(f'F{fmt_ratio(f1, f2, x1, x2)} C{fmt_ratio(c1, c2)}')
                rec.append(dict(k=k, E=E, F=(f1 / f2 if f1 and f2 else None), F_extrap=bool(x1 or x2),
                                C=(c1 / c2 if c1 and c2 else None), C1=c1, C2=c2, F1=f1, F2=f2))
            S['rows'][lab] = rec
            print(f'  {lab:16s} ' + ' '.join(f'{c:>20s}' for c in cells))
        # trigger statistics
        S['trigger'] = {}
        for swa in ('sw', 'swd', 'swd-r2', 'sw-ns', 'swd-r2-ns'):
            if swa not in A:
                continue
            sw = A[swa]
            fired = [r for r in sw if r.get('n_handoff', 0) > 0]
            revs = collections.Counter()
            for r in sw:
                for e in r.get('events', []):
                    if e.get('ev') == 'revert':
                        w = e['why']
                        cat = ('cost' if w.startswith('cost') else 'rejection-rate' if w.startswith('rejection')
                               else 'ritz-angle' if w.startswith('Ritz') else 'guard' if w.startswith('guard') else w)
                        revs['revert:' + cat] += 1
                    elif e.get('ev', '').startswith('guard-veto'):
                        revs['guard-veto'] += 1
            rf = [r.get('rock_frac_time', 0) for r in sw]
            sr = [r.get('shadow_summary', {}).get('ratio_median') for r in sw]
            sr = [x for x in sr if x is not None]
            S['trigger'][swa] = dict(n_rtol=len(sw), n_fired=len(fired), rock_frac_median=float(np.median(rf)),
                                     rock_frac=[round(x, 3) for x in rf], reverts=dict(revs),
                                     handoffs=[r.get('n_handoff', 0) for r in sw],
                                     shadow_ratio_median=(float(np.median(sr)) if sr else None))
            print(f'  trigger[{swa}]: fired at {len(fired)}/{len(sw)} rtols; time on ROCK4 median {np.median(rf):.2f} '
                  f'(per rtol {", ".join(f"{x:.2f}" for x in rf)}); shadow-ratio median {np.median(sr) if sr else float("nan"):.2f}; reverts/vetoes {dict(revs)}')
        if 'sw' in A:
            eq = []
            mm = {r['rtol']: r for r in A.get('mf', [])}
            for r in A['sw']:
                if r['rtol'] in mm:
                    eq.append(r['err'] / mm[r['rtol']]['err'])
            if eq:
                S['eq_rtol_err_ratio_sw_mf'] = float(np.median(eq))
                print(f'  equal-rtol accuracy err(sw)/err(mf): median {np.median(eq):.2f} (range {min(eq):.2f}..{max(eq):.2f})')
        # guard cost share
        if 'sw' in A:
            gs = [r['flops']['guard'] / r['flops']['total'] for r in A['sw']]
            sh = [r['flops']['shadow'] / r['flops']['total'] for r in A['sw']]
            S['guard_share_median'] = float(np.median(gs)); S['shadow_share_median'] = float(np.median(sh))
            print(f'  overhead shares in sw: guard median {np.median(gs):.3f}, shadow median {np.median(sh):.3f}')
        summ[p] = S
    json.dump(summ, open(os.path.join(HERE, 'res/summary.json'), 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
