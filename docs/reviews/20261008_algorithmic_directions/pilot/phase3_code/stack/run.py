"""Arm registry and ladder runner for probe B1. Usage: python3 run.py <problem> <arm,arm,...> <rtol,rtol,...>
Appends one JSON line per (arm, rtol) to res/<problem>.jsonl; skips pairs already present (resumable)."""
import json, math, os, sys, time
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stack, tprobs

W = lambda **kw: stack.make_arm('wabs', **kw)
ARMS = {
    # the cumulative stack
    'A0': stack.make_arm('base'),                                         # (0) SPD07 base (dup residual counted)
    # (1) = A0 minus the diagnostic counters (identical trajectory, verified in fid.py); derived in analysis
    'A2': stack.make_arm('proj'),                                         # (2) proj-stop, L2 1e-10, floor g*1e-14
    'A3': W(tgt=dict(Theta=0.2, ino=None)),                               # (3) proj + A1 coupled target (wE0.2)
    'A4a': W(tgt=dict(Theta=0.2, ino=1e-3)),                              # (4) (3) + INO per-step floor theta 0.001
    'A4b': W(tgt=dict(Theta=0.2, ino=2e-3)),                              # (4) theta 0.002
    'A5a': W(tgt=dict(Theta=0.2, ino=1e-3), ctrl='PRED+cap'),             # (5) (4a) + CTRL-PRED-CAP
    'A5b': W(tgt=dict(Theta=0.2, ino=2e-3), ctrl='PRED+cap'),             # (5) (4b) + CTRL-PRED-CAP
    'A6': W(tgt=dict(Theta=0.2, ino=1e-3), ctrl='PRED+cap', maxit=2000, guard=True),   # (6) (5a) + maxit 2000 + guard
    # rivals named by the judge / A1
    'R3': stack.make_arm('l2c'),                                          # cheap L2 coupling rival of (3)
    'R5': W(tgt=dict(Theta=0.2, ino=1e-3), ctrl='I725'),                  # uniform set-point 0.725 rival of PRED on (4a)
    'R3U': W(tgt=dict(Theta=0.2, ino=None), ctrl='I725'),                 # uniform set-point on (3)
    'Rbig': stack.make_arm('base', maxit=2000),                           # maxit 2000, no guard, on base
    # attribution controls
    'C4a': W(tgt=dict(Theta=None, ino=1e-3)),                             # INO alone (judge's T1-A arm), theta 0.001
    'C4b': W(tgt=dict(Theta=None, ino=2e-3)),                             # INO alone, theta 0.002
    'C3P': W(tgt=dict(Theta=0.2, ino=None), ctrl='PRED+cap'),             # PRED on (3) without INO
    'C2P': stack.make_arm('proj', ctrl='PRED+cap'),                       # PRED on the fixed-L2 proj target
    'C0P': stack.make_arm('base', ctrl='PRED+cap'),                       # PRED on production
    'C0G': stack.make_arm('base', maxit=2000, guard=True),                # guard + maxit 2000 on base
    'C3G': W(tgt=dict(Theta=0.2, ino=None), maxit=2000, guard=True),      # guard + maxit 2000 on (3)
    'C6ng': W(tgt=dict(Theta=0.2, ino=1e-3), ctrl='PRED+cap', maxit=2000),  # (6) without the guard
    'S': W(tgt=dict(Theta=0.2, ino=None), ctrl='PRED+cap', maxit=2000, guard=True),    # recommended: (6) without INO
    'SU': W(tgt=dict(Theta=0.2, ino=None), ctrl='I725', maxit=2000, guard=True),       # same with the set-point rival
    # deliberately loose targets (positive controls for the PRED x noise question; not stack candidates)
    'X02I': W(tgt=dict(Theta=None, ino=0.02)),                            # INO per-step 0.02, I
    'X02P': W(tgt=dict(Theta=None, ino=0.02), ctrl='PRED+cap'),           # INO per-step 0.02, PRED+cap
    'XrI': stack.make_arm('proj', rtol_lin=1e-4, max_att=6000),           # uniform relative forcing 1e-4, I (capped at 6000 attempts)
    'XrP': stack.make_arm('proj', rtol_lin=1e-4, ctrl='PRED+cap', max_att=6000),   # same, PRED+cap
    'LU': stack.make_arm('lu'),                                           # exact-solve control, I controller
    'LUP': stack.make_arm('lu', ctrl='PRED+cap'),                         # exact-solve control, PRED+cap
    # (7) finite-difference JVP variants
    'A0f': stack.make_arm('base', jvp='fd'),
    'A2f': stack.make_arm('proj', jvp='fd'),
    'A3f': W(tgt=dict(Theta=0.2, ino=None), jvp='fd'),
    'A5af': W(tgt=dict(Theta=0.2, ino=1e-3), ctrl='PRED+cap', jvp='fd'),
    'R3f': stack.make_arm('l2c', jvp='fd'),
    # FD-aware attainable-accuracy guard (exploratory extension, see report): in-cycle floor 1e-8 ||D b|| and stall
    # acceptance at 4 sqrt(eps) of the backward-error scale; production-form FD comparators at rtol_lin 1e-8
    'A3fg': W(tgt=dict(Theta=0.2, ino=None, rho_fl=1e-8), jvp='fd', stall=4*stack.SQEPS),
    'A5afg': W(tgt=dict(Theta=0.2, ino=1e-3, rho_fl=1e-8), ctrl='PRED+cap', jvp='fd', stall=4*stack.SQEPS),
    'A5ag': W(tgt=dict(Theta=0.2, ino=1e-3, rho_fl=1e-8), ctrl='PRED+cap', stall=4*stack.SQEPS),   # same guard, exact JVP
    'Sfs': W(tgt=dict(Theta=0.2, ino=None), ctrl='PRED+cap', maxit=2000, guard=True, jvp='fd', stall=4*stack.SQEPS),  # S, FD, stall-only
    'Sfg9': W(tgt=dict(Theta=0.2, ino=None, rho_fl=1e-9), ctrl='PRED+cap', maxit=2000, guard=True, jvp='fd', stall=4*stack.SQEPS),
    'Sfg8': W(tgt=dict(Theta=0.2, ino=None, rho_fl=1e-8), ctrl='PRED+cap', maxit=2000, guard=True, jvp='fd', stall=4*stack.SQEPS),
    'A0f8': stack.make_arm('base', jvp='fd', rtol_lin=1e-8),
    'A2f8': stack.make_arm('proj', jvp='fd', rtol_lin=1e-8),
    'A0e8': stack.make_arm('base', rtol_lin=1e-8),                        # exact-JVP twin of A0f8
    'A0f8s': stack.make_arm('base', jvp='fd', rtol_lin=1e-8, stall=4*stack.SQEPS),   # production form + FD stall backstop
    'A2f8s': stack.make_arm('proj', jvp='fd', rtol_lin=1e-8, stall=4*stack.SQEPS),
}
# h0 seeds for the controller comparison (h0 = 1e-5 and 1e-4 instead of 1e-6)
for _s, _h in (('s5', 1e-5), ('s4', 1e-4)):
    for _a in ('A0', 'A3', 'C3P', 'R3U', 'LU', 'LUP'):
        ARMS[_a + _s] = dict(ARMS[_a], h0=_h)


def problem(name):
    if name == 'bruss300': return tprobs.bruss(300)
    return tprobs.PROBLEMS[name]()


def gap_summary(gaps):
    """gaps: (projected, true, thr, breakdown, cols, exact_true) per accepted solve exit."""
    if not gaps: return None
    G = [x for x in gaps if x[0] is not None and not x[3]]
    tp = np.array([x[1]/max(x[0], 1e-300) for x in G]) if G else np.array([])
    ex = np.array([x[5]/x[2] for x in gaps])            # exact-J true residual / target
    et = np.array([x[5]/max(x[1], 1e-300) for x in gaps])  # exact-J true / solver true
    return dict(n=len(gaps), n_proj_exits=len(G), tp_gt2=int(np.sum(tp > 2)), tp_gt10=int(np.sum(tp > 10)),
                tp_max=float(tp.max()) if len(tp) else None, tp_med=float(np.median(tp)) if len(tp) else None,
                ex_gt1=int(np.sum(ex > 1)), ex_gt10=int(np.sum(ex > 10)), ex_max=float(ex.max()), ex_med=float(np.median(ex)),
                et_max=float(et.max()), et_med=float(np.median(et)))


if __name__ == '__main__':
    pname = sys.argv[1]; arms = sys.argv[2].split(','); rtols = [float(x) for x in sys.argv[3].split(',')]
    fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'res', f'{pname}.jsonl')
    done = set()
    if os.path.exists(fn):
        for l in open(fn):
            o = json.loads(l); done.add((o['arm'], o['rtol']))
    for rtol in rtols:
        for an in arms:
            if (an, rtol) in done: continue
            a = ARMS[an]; p = problem(pname); t0 = time.time()
            fd = a['jvp'] == 'fd'
            try:
                r = stack.integrate(p, rtol, a, h0=a.get('h0', 1e-6), gapdiag=fd, max_att=a.get('max_att', 3000 if fd else 60000),
                                    cap_ok=(fd or 'max_att' in a))
            except Exception as e:
                print(pname, an, rtol, 'ERROR', e, flush=True)
                with open(fn, 'a') as fo:
                    fo.write(json.dumps(dict(prob=pname, arm=an, rtol=rtol, error=str(e))) + '\n')
                continue
            gaps = r.pop('gaps', None)
            o = {k: (v.tolist() if isinstance(v, np.ndarray) else v) for k, v in r.items()}
            o.update(prob=pname, arm=an, rtol=rtol, sec=time.time() - t0, flops=stack.flops(p, r, a),
                     flops_fd=stack.flops(p, r, a, jvp_model='fd'))
            if a['kind'] == 'lu':
                o['flops_band'] = stack.banded_lu_flops(p, r)
            if fd: o['gap'] = gap_summary(gaps)
            with open(fn, 'a') as fo:
                fo.write(json.dumps(o) + '\n')
            print(f"{pname} {an:5s} rtol={rtol:<9.3g} att={r['att']} acc={r['acc']} rej={r['rej']} lf={r['lin_fail']} jvp={r['jvp']} "
                  f"JVP/acc={r['jvp']/r['acc']:.1f} dots={r['dots']} g={r['guard_q']}/{r['guard_over']}/{r['guard_false']} "
                  f"st={r['stalls']} [{time.time()-t0:.0f}s]", flush=True)
