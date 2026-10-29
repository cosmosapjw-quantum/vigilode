"""CTRL-NULLS kept residual: catastrophe-only abort |Y_i|_inf > 1e6 * max(1, |y|_inf) at stage i.
Replay on the Rust-exported single-attempt stage data of R-NEXT-01 (L-0049) and REV-03 (L-0062) (U form:
Y_i = y + sum_{j<i} A_ij U_j; the K-form stages are mapped with the same test on the stage argument), plus the
adaptive dense replica ladders (every attempt of I and PRED on the 4 problems, rtol 1e-3..1e-7, h0 1e-6 and 1e-2)."""
import json, struct, sys, math
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *
def unhex(s): return struct.unpack('>d', bytes.fromhex(s))[0]
THR = 1e6
out = {}
for tag, path in (('L-0049', 'rnext01_residual_output_20261003'), ('L-0062', 'rev03_one_sided_acceptance_20261003')):
    d = json.load(open(f'{ROOT}/research/{path}/stages.json'))
    rows = []
    for cs in d['cases']:
        h = unhex(cs['h']); y = np.array([unhex(v) for v in cs['y']])
        for drv in ('u',):
            if 'error' in cs[drv]: continue
            U = np.array([[unhex(v) for v in st] for st in cs[drv]['stages']])
            Y = np.array([y + A[i, :i] @ U[:i] for i in range(S)])
            grow_ = np.max(np.abs(Y), axis=1) / max(1.0, np.max(np.abs(y)))
            ynew = np.array([unhex(v) for v in cs[drv]['y_new']]); err = unhex(cs[drv]['error_norm'])
            first = next((i for i in range(S) if grow_[i] > THR or not np.isfinite(grow_[i])), None)
            rows.append(dict(case=f"{cs['problem']}/h={h:g}/{cs['method']}", err=err, maxgrow=float(np.nanmax(grow_)),
                             abort_stage=first, ynew_grow=float(np.max(np.abs(ynew)) / max(1.0, np.max(np.abs(y))))))
    acc = [r for r in rows if r['err'] <= 1]; rejb = [r for r in rows if r['err'] > 1e3]; rejn = [r for r in rows if 1 < r['err'] <= 1e3]
    print(f"{tag}: {len(rows)} U-driver attempts; accepted {len(acc)}: aborted {sum(r['abort_stage'] is not None for r in acc)} "
          f"(max stage growth among accepted {max(r['maxgrow'] for r in acc):.3g}); blown-up rejections (err>1e3) {len(rejb)}: "
          f"aborted {sum(r['abort_stage'] is not None for r in rejb)} at stages {[r['abort_stage'] for r in rejb]} "
          f"(min growth {min([r['maxgrow'] for r in rejb], default=float('nan')):.3g}); ordinary rejections {len(rejn)}: aborted {sum(r['abort_stage'] is not None for r in rejn)}")
    for r in rejb: print('   ', r)
    out[tag] = rows
# adaptive ladders: any false abort on accepted attempts? max stage growth over all attempts
def ladder_scan(name, arm, rtol, h0):
    p = PROBS[name](); f, Jf = p['f'], p['J']; t0, tf = p['span']; atol = rtol * p['ascale']; n = len(p['y0'])
    t, y, h = t0, p['y0'].copy(), h0; ctrl = make_ctrl(arm); last = None; fresh = True; att = 0; mg_acc = 0.0; mg_rej = 0.0; ab_acc = ab_rej = 0
    while t < tf and att < 100000:
        h = min(h, tf - t)
        if last is not None and h >= last: h = np.nextafter(t + last, -np.inf) - t
        if fresh: J = Jf(t, y); f0 = f(t, y)
        att += 1
        lu = lu_factor(np.eye(n) / (h * g) - J); ynew, bad, U = dense_attempt(p, lu, f0, None, t, y, h)
        Y = np.array([y + A[i, :i] @ U[:i] for i in range(S)])
        gr = float(np.nanmax(np.max(np.abs(Y), axis=1)) / max(1.0, np.max(np.abs(y))))
        if ynew is None: err = np.inf
        else:
            err = math.sqrt(np.mean((U[-1] / (atol + rtol * np.maximum(np.abs(y), np.abs(ynew)))) ** 2))
        if err <= 1:
            mg_acc = max(mg_acc, gr); ab_acc += gr > THR
            t += h; y = ynew; fresh = True; last = None; h *= ctrl.accept(h, err)
        else:
            mg_rej = max(mg_rej, gr); ab_rej += gr > THR
            fresh = False; last = h; h *= ctrl.reject(h, err)
    return att, mg_acc, mg_rej, ab_acc, ab_rej
tot = dict(att=0, ab_acc=0, ab_rej=0, mg_acc=0.0)
for name in ('robertson', 'hires', 'vdp', 'bruss50'):
    for arm in ('I', 'PRED'):
        for rtol in (1e-3, 1e-5, 1e-7):
            for h0 in (1e-6, 1e-2):
                att, ma, mr, aa, ar = ladder_scan(name, arm, rtol, h0)
                tot['att'] += att; tot['ab_acc'] += aa; tot['ab_rej'] += ar; tot['mg_acc'] = max(tot['mg_acc'], ma)
                print(f"  {name:9s} {arm:4s} {rtol:.0e} h0={h0:.0e} att={att} max growth acc={ma:.3g} rej={mr:.3g} aborts acc/rej={aa}/{ar}", flush=True)
print('adaptive totals', tot)
out['adaptive'] = tot
json.dump(out, open(f'{HERE}/catastrophe.json', 'w'), indent=1, default=float)
