"""Fixed-step contract ladders (A1 ladders.py transcription; A1 rep.py fixed-step driver) for the stacked targets:
A3 (A1 rule), A4a/A4b (A1 rule with INO per-step floor 0.001/0.002), C4a (INO alone), vs direct LU. PRED/guard are
inert at fixed step. Appends to res/ladders_b1.jsonl."""
import json, sys, math, time, numpy as np
sys.path.insert(0, '.')
import rep_a1 as rep, tprobs, stack, coupled_target as CT
ARMS = {'A3': dict(Theta=0.2, ino=None), 'A4a': dict(Theta=0.2, ino=1e-3), 'A4b': dict(Theta=0.2, ino=2e-3), 'C4a': dict(Theta=None, ino=1e-3)}
def runone(prob, rtol, steps, an):
    p = dict(prob); p['ascale'] = 1e-2
    h = (p['span'][1] - p['span'][0])/steps
    if an == 'lu':
        r = rep.integrate(p, rtol, rep.make_arm('lu'), fixed_h=h)
    else:
        a = rep.make_arm('wabs', stall=CT.RHO_STALL); a['maxit'] = 20000
        r = rep.integrate(p, rtol, a, tgt=stack.StackTarget(**ARMS[an]), fixed_h=h)
    ex = p['exact'](p['span'][1])
    rel = float(np.max(np.abs(r['y'] - ex))/np.max(np.abs(ex)))
    sc = 1e-2*rtol + rtol*np.abs(ex)
    return dict(rel=rel, wrms=float(math.sqrt(np.mean(((r['y'] - ex)/sc)**2))), jvp=r['jvp'])
LAD = {'diagpr128': (lambda: tprobs.diag_pr(128, 1e6), [1e-6], range(3, 6)),
       'semilin128': (lambda: tprobs.semilin(128, 0.02, 3.0, -1.0, 10.0), [1e-6], range(3, 6)),
       'semilin64': (lambda: tprobs.semilin(64, 0.05, 0.5, -1.0, 0.5), [1e-4, 1e-6], range(3, 9))}
for L, (mk, rtols, ks) in LAD.items():
    for rtol in rtols:
        for k in ks:
            row = dict(ladder=L, rtol=rtol, k=k)
            for an in ['lu'] + list(ARMS):
                t0 = time.time(); row[an] = runone(mk(), rtol, 1 << k, an); row[an]['sec'] = time.time() - t0
            lu = row['lu']['rel']
            print(L, rtol, k, f"LU rel {lu:.2e} | " + ' '.join(f"{an}: {row[an]['rel']/lu:.2f}x jvp {row[an]['jvp']}" for an in ARMS), flush=True)
            with open('res/ladders_b1.jsonl', 'a') as fo: fo.write(json.dumps(row) + '\n')
