"""Critic stress runner (EXPLORATORY). usage: python3 crun.py <problem> <arm,arm,...> <rtol,rtol,...> [wall_cap_s] [ortho]
Appends one JSON line per (arm, rtol) to res/<problem>.jsonl; resumable. Arms reuse probe B1's definitions
(stack.py copied unchanged apart from the CGS2 speed option, the misdeclared-T option and a wall cap)."""
import json, os, sys, time
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import stack, sprobs

W = lambda **kw: stack.make_arm('wabs', **kw)
S_KW = dict(tgt=dict(Theta=0.2, ino=None), ctrl='PRED+cap', maxit=2000, guard=True)
ARMS = {
    'A0': stack.make_arm('base'),                                      # SPD07 production
    'A2': stack.make_arm('proj'),                                      # proj-stop, L2 1e-10 (no coupled target)
    'R3': stack.make_arm('l2c'),                                       # cheap L2 coupling rival of the target
    'Rbig': stack.make_arm('base', maxit=2000),                        # budget rival
    'C3G': W(tgt=dict(Theta=0.2, ino=None), maxit=2000, guard=True),   # S without PRED (I controller)
    'S': W(**S_KW),                                                    # recommended stack (B1 ARMS['S'])
    'SU': W(tgt=dict(Theta=0.2, ino=None), ctrl='I725', maxit=2000, guard=True),
    # critic post-hoc exploratory remedy: tighten each stage threshold by nu_hat = running max 1/sigma_min(R_j)
    # (a lower bound of ||(D W D^-1)^-1||_2 from the GMRES Givens factor); inert when nu_hat <= 1
    'Snu': W(nu=True, **S_KW),
    'Snus': W(nu=True, **dict(S_KW, stall=None)),   # nu-guard and no stall rule
    'Sns': W(**dict(S_KW, stall=None)),              # S without the stall rule (control)
    # critic post-hoc remedy 2: before a guard/maxit abort, accept the iterate if it meets production's own
    # acceptance (unscaled L2 ||b - W x|| <= max(g 1e-14, 1e-10 ||b||)); never fails a solve production accepts
    'Sfb': W(fb=True, **S_KW), 'Sfbnu': W(fb=True, nu=True, **S_KW),
    'Sfg8fb': W(tgt=dict(Theta=0.2, ino=None, rho_fl=1e-8), ctrl='PRED+cap', maxit=2000, guard=True, jvp='fd',
                stall=4*stack.SQEPS, fb=True, fb_rel=1e-8),   # FD-aware stack + fallback to the FD production-form rule
    'LU': stack.make_arm('lu'), 'LUP': stack.make_arm('lu', ctrl='PRED+cap'),
    # misdeclared integration span T in the EPUS target (T/100: e.g. per-output-interval calls; 100 T: overestimate)
    'S_T01': W(tscale=0.01, **S_KW), 'S_T100': W(tscale=100.0, **S_KW),
    # finite-difference JVPs
    'A0f': stack.make_arm('base', jvp='fd'),
    'Sf': W(jvp='fd', **S_KW),
    'Sfg8': W(tgt=dict(Theta=0.2, ino=None, rho_fl=1e-8), ctrl='PRED+cap', maxit=2000, guard=True, jvp='fd',
              stall=4*stack.SQEPS),
    'A0f8s': stack.make_arm('base', jvp='fd', rtol_lin=1e-8, stall=4*stack.SQEPS),
}
for _s, _h in (('s5', 1e-5), ('s4', 1e-4)):
    for _a in ('A0', 'S', 'LU', 'LUP', 'C3G'):
        ARMS[_a + _s] = dict(ARMS[_a], h0=_h)


def errs(p, y, rtol, ref):
    y = np.asarray(y, float)
    ecw = float(np.max(np.abs(y - ref)/np.maximum(np.abs(ref), 1e-10)))
    enw = float(np.max(np.abs(y - ref))/np.max(np.abs(ref)))
    sc = rtol*p['ascale'] + rtol*np.abs(ref)
    ew = float(np.sqrt(np.mean(((y - ref)/sc)**2)))
    return ecw, enw, ew


if __name__ == '__main__':
    pname = sys.argv[1]; arms = sys.argv[2].split(','); rtols = [float(x) for x in sys.argv[3].split(',')]
    stack.WALL_CAP = float(sys.argv[4]) if len(sys.argv) > 4 else 1800.0
    stack.ORTHO = sys.argv[5] if len(sys.argv) > 5 else 'cgs2'
    fn = os.path.join(HERE, 'res', f'{pname}.jsonl')
    done = set()
    if os.path.exists(fn):
        for l in open(fn):
            o = json.loads(l); done.add((o['arm'], o['rtol']))
    p = sprobs.problem(pname)
    ref, unc = sprobs.reference(pname, p)
    for rtol in rtols:
        for an in arms:
            if (an, rtol) in done: continue
            a = ARMS[an]; t0 = time.time()
            fd = a['jvp'] == 'fd'
            try:
                r = stack.integrate(p, rtol, a, h0=a.get('h0', 1e-6), gapdiag=fd, max_att=a.get('max_att', 20000),
                                    cap_ok=True)
            except Exception as e:
                print(pname, an, rtol, 'ERROR', repr(e), flush=True)
                with open(fn, 'a') as fo:
                    fo.write(json.dumps(dict(prob=pname, arm=an, rtol=rtol, error=repr(e))) + '\n')
                continue
            gaps = r.pop('gaps', None)
            o = {k: (v.tolist() if isinstance(v, np.ndarray) else v) for k, v in r.items()}
            ecw, enw, ew = errs(p, r['y'], rtol, ref)
            o.update(prob=pname, arm=an, rtol=rtol, sec=time.time() - t0, flops=stack.flops(p, r, a),
                     flops_fd=stack.flops(p, r, a, jvp_model='fd'), e_cw=ecw, e_nw=enw, e_w=ew, ref_unc=unc,
                     ortho=stack.ORTHO, n=len(p['y0']))
            if fd and gaps:
                G = [g for g in gaps if g[0] is not None and not g[3]]
                tp = np.array([g[1]/max(g[0], 1e-300) for g in G]) if G else np.array([0.0])
                ex = np.array([g[5]/g[2] for g in gaps])
                o['gap'] = dict(n=len(gaps), tp_gt10=int(np.sum(tp > 10)), tp_max=float(tp.max()),
                                ex_gt10=int(np.sum(ex > 10)), ex_max=float(ex.max()), ex_med=float(np.median(ex)))
            with open(fn, 'a') as fo:
                fo.write(json.dumps(o) + '\n')
            print(f"{pname} {an:6s} rtol={rtol:<8.2g} att={r['att']} acc={r['acc']} rej={r['rej']} lf={r['lin_fail']} "
                  f"nf={r['nf_fail']} jvp={r['jvp']} dots={r['dots']} Mflop={o['flops']/1e6:.3g} e_cw={ecw:.3g} e_w={ew:.3g} "
                  f"g={r['guard_q']}/{r['guard_over']}/{r['guard_false']} st={r['stalls']} cap={r['capped']} t_end={r['t']:.4g} "
                  f"[{time.time()-t0:.0f}s]", flush=True)
