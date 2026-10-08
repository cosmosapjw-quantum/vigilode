"""Fixed-step contract ladders (transcribed from inner_forcing_fixed_step_ladder_contracts.rs:164-193 and
fixed_step_order_contracts.rs:315-383): each arm's inexact solves vs direct LU on the same fixed-h ladder.
Fixed step: every step accepted, err_prev = the step's own embedded estimate (as in the Rust forcing arm)."""
import json, sys, math, time, numpy as np
sys.path.insert(0, '.')
import rep, tprobs, arms

def run(prob, rtol, steps, arm_name):
    p = dict(prob); p['ascale'] = 1e-2               # all contract ladders use atol = 1e-2 * rtol
    if arm_name == 'lu':
        a, tg = rep.make_arm('lu'), None
    else:
        a, tg = arms.arm(arm_name)
    a = dict(a); a['maxit'] = 20000
    h = (p['span'][1] - p['span'][0])/steps
    r = rep.integrate(p, rtol, a, tgt=tg, fixed_h=h)
    ex = p['exact'](p['span'][1])
    rel = float(np.max(np.abs(r['y'] - ex))/np.max(np.abs(ex)))
    sc = 1e-2*rtol + rtol*np.abs(ex)
    wr = float(math.sqrt(np.mean(((r['y'] - ex)/sc)**2)))
    return dict(rel=rel, wrms=wr, jvp=r['jvp'], dots=r['dots'], rbind=r['roundoff_bind'])

LADDERS = {
    'pr_tight':  (lambda: tprobs.pr_forced(), [1e-10], range(2, 8)),
    'diagpr128': (lambda: tprobs.diag_pr(128, 1e6), [1e-6], range(3, 6)),
    'semilin128': (lambda: tprobs.semilin(128, 0.02, 3.0, -1.0, 10.0), [1e-6], range(3, 6)),
    'semilin64': (lambda: tprobs.semilin(64, 0.05, 0.5, -1.0, 0.5), [1e-4, 1e-6], range(3, 9)),
}
if __name__ == '__main__':
    which = sys.argv[1].split(','); armlist = sys.argv[2].split(',')
    out = []
    for L in which:
        mk, rtols, ks = LADDERS[L]
        for rtol in rtols:
            for k in ks:
                row = dict(ladder=L, rtol=rtol, k=k)
                for an in armlist:
                    t0 = time.time()
                    p = mk()
                    if L == 'pr_tight': p['span'] = (0.0, 1.0)
                    try:
                        row[an] = run(p, rtol, 1 << k, an)
                    except Exception as e:
                        row[an] = dict(error=str(e))
                    row[an]['sec'] = time.time() - t0
                out.append(row)
                print(L, rtol, k, ' '.join(f"{an}: rel={row[an].get('rel', float('nan')):.3e} wrms={row[an].get('wrms', float('nan')):.3e} jvp={row[an].get('jvp')}" for an in armlist), flush=True)
                with open('res/ladders.jsonl', 'a') as fo: fo.write(json.dumps(row) + '\n')
