"""Job runner for PROBE B3 (EXPLORATORY). Usage: python3 run_e.py <jobs-file> <out.jsonl> [workers]
jobs-file lines: problem arm rtol.  E arms (pexprb.py):
  E   : KIOPS, WRMS-scaled operator (D J D^-1), EPUS phi tolerance (Theta 0.2), err = max(time, phi)
  E2  : KIOPS, unscaled 2-norm operator (certificate-eligible norm), tolerance via F-043 (sqrt(n) min w)
  Ed  : exact dense phi (oracle; attribution of step counts / errors to the method, not the Krylov layer)
  Ef  : as E with a fixed per-step phi tolerance theta = 0.02 (control for the EPUS rule)
  Ec  : E2 + JAK defect-integral bounds + actual phi errors (dense) + mu_2(J) per step  (certification audit)
  Esc : E  + actual phi errors + mu_2(D J D^-1) per step (scaled-norm certificate applicability)
  mf, base : RODAS5P arms of PROBE B2 (swdrv.py copy; reproduces B2 rows bit-for-bit, res/fid_rodas.json)
Each result line holds counters, flop parts, endpoint error (rescored against the tight references), final y."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, time, traceback
from concurrent.futures import ProcessPoolExecutor, as_completed
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))

EARMS = {'E': dict(), 'E2': dict(norm='l2'), 'Ed': dict(phi='dense'), 'Ef': dict(tol='fixed', theta=0.02),
         'Ec': dict(norm='l2', cert=True), 'Esc': dict(check_actual=True), 'Ef5': dict(tol='fixed', theta=0.2),
         'Em': dict(mmin=10), 'Em40': dict(mmax=40), 'Em24': dict(mmax=24), 'E2m40': dict(norm='l2', mmax=40), 'Em16': dict(mmax=16), 'Em12': dict(mmax=12)}
_PC = {}


def harness(pname):
    """SPD07 harness small problems (hyp/probs.py definitions; 80-bit references of probe A1, max-rel 1e-10 floor).
    F_rhs / F_jvp: hand counts (vdP mu=1000: 8 / 9 flops; HIRES: 45 / 60 flops)."""
    import pr, scipy.sparse as sp
    T = '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/target/'
    if pname == 'vdp2':
        mu = 1000.0
        f = lambda t, y: np.array([y[1], mu * (1 - y[0] ** 2) * y[1] - y[0]])
        Jd = lambda t, y: np.array([[0.0, 1.0], [-2 * mu * y[0] * y[1] - 1, mu * (1 - y[0] ** 2)]])
        y0 = np.array([2.0, 0.0]); span = (0.0, 2000.0); asc = 1.0; Fr, Fj = 8.0, 9.0; refn = 'vdp'
    elif pname == 'hires8':
        def f(t, y):
            q = 280.0 * y[5] * y[7]
            return np.array([-1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007, 1.71 * y[0] - 8.75 * y[1],
                             -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4], 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3],
                             -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6],
                             -q + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6], q - 1.81 * y[6], -q + 1.81 * y[6]])
        def Jd(t, y):
            j = np.zeros((8, 8))
            j[0, :3] = [-1.71, 0.43, 8.32]; j[1, :2] = [1.71, -8.75]
            j[2, 2:5] = [-10.03, 0.43, 0.035]; j[3, 1:4] = [8.32, 1.71, -1.12]
            j[4, 4:7] = [-1.745, 0.43, 0.43]
            j[5, 3:8] = [0.69, 1.71, -280 * y[7] - 0.43, 0.69, -280 * y[5]]
            j[6, 5:8] = [280 * y[7], -1.81, 280 * y[5]]; j[7, 5:8] = [-280 * y[7], 1.81, -280 * y[5]]
            return j
        y0 = np.zeros(8); y0[0] = 1.0; y0[7] = 0.0057; span = (0.0, 321.8122); asc = 1e-4; Fr, Fj = 45.0, 60.0; refn = 'hires'
    else:
        raise ValueError(pname)
    R = json.load(open(T + f'ref_{refn}.json'))
    ref = np.array([float(x) for x in R['1e-15']['y_ld']])
    n = len(y0)
    J = lambda t, y: sp.csr_matrix(Jd(t, y))
    return dict(name=pname, family='harness', f=f, J=J, ft=None, y0=y0, span=span, ascale=asc, n=n,
                nnzJ=int(np.count_nonzero(Jd(0.0, y0 + 0.1))), colors=n, h0=1e-6, F_rhs=Fr, F_jvp=Fj, F_ft=0.0, ref=ref,
                ref_src='80-bit RODAS5P 1e-15 (probe A1)', err=(lambda y, ref=ref: pr.maxrel(y, ref)),
                err_name='max rel (1e-10 floor) endpoint', kind='harness')


def problem(pname):
    import pr
    if pname in _PC:
        return _PC[pname]
    p = harness(pname) if pname in ('vdp2', 'hires8') else pr.build(pname)
    short = pname.split('-')[0]
    fn = os.path.join(HERE, 'rodas', 'refs', f'{pname}-endpoint.npz')
    if p['kind'] == 'corpus' and os.path.exists(fn):
        ref = np.load(fn)['r13']; p['ref'] = ref; p['ref_src'] = 'direct RODAS5P rtol 1e-13 (B2 refs)'
        p['err'] = (lambda y, ref=ref: pr.tight_wrms(y, ref))
    _PC[pname] = p
    return p


def job(pname, arm, rt):
    p = problem(pname)
    t0 = time.time()
    base_arm, h0 = arm, None
    if ':' in arm:                     # 'E:3' / 'mf:0.3333': h0 seed = factor * default h0
        base_arm, fac = arm.split(':'); h0 = p['h0'] * float(fac)
    try:
        if base_arm in ('Emc', 'E2mc'):   # cost-balanced Krylov cap: 25 m^3 (small expm) ~ m F_jvp + 2 m^2 n (Arnoldi)
            import pexprb as PX, math as _m
            n_, Fj_ = p['n'], p['F_jvp']
            cap = int(min(100, max(12, _m.ceil((2 * n_ + _m.sqrt(4 * n_ * n_ + 100 * Fj_)) / 50))))
            kw = dict(mmax=cap) if base_arm == 'Emc' else dict(mmax=cap, norm='l2')
            r = PX.integrate(p, rt, PX.make_arm(**kw), h0=h0); r['mmax_rule'] = cap
        elif base_arm in EARMS:
            import pexprb as PX
            r = PX.integrate(p, rt, PX.make_arm(**EARMS[base_arm]), h0=h0)
        else:
            import swdrv
            r = swdrv.integrate(p, rt, swdrv.make_arm(base_arm), h0=h0)
    except Exception as e:
        return dict(problem=pname, arm=arm, rtol=rt, ok=False, error=f'{type(e).__name__}: {e}', wall=time.time() - t0)
    y = np.asarray(r.pop('y'))
    err = p['err'](y) if p['ref'] is not None else None
    out = dict(problem=pname, arm=arm, rtol=rt, ok=True, err=err, err_name=p['err_name'], n=p['n'], ref_src=p['ref_src'])
    if p['n'] <= 512:
        out['y'] = y.tolist()
    for k, v in r.items():
        if k in ('events', 'hfov_hist'):
            continue
        if isinstance(v, (np.floating, np.integer)):
            v = v.item()
        out[k] = v
    out['wall'] = time.time() - t0
    return out


def main():
    jobs = [l.split() for l in open(sys.argv[1]) if l.strip() and not l.startswith('#')]
    outp = sys.argv[2]; nw = int(sys.argv[3]) if len(sys.argv) > 3 else 2
    done = set()
    if os.path.exists(outp):
        for l in open(outp):
            try:
                d = json.loads(l); done.add((d['problem'], d['arm'], float(d['rtol'])))
            except Exception:
                pass
    todo = [(a, b, float(c)) for a, b, c in jobs if (a, b, float(c)) not in done]
    print(f'{len(todo)} jobs ({len(done)} done)', flush=True)
    with ProcessPoolExecutor(max_workers=nw) as ex, open(outp, 'a') as fo:
        futs = {ex.submit(job, *j): j for j in todo}
        for fu in as_completed(futs):
            j = futs[fu]
            try:
                d = fu.result()
            except Exception:
                d = dict(problem=j[0], arm=j[1], rtol=j[2], ok=False, error=traceback.format_exc()[-500:])
            fo.write(json.dumps(d, default=float) + '\n'); fo.flush()
            if d.get('ok'):
                print(f"{d['problem']:12s} {d['arm']:5s} {d['rtol']:.2e} att={d['att']:6d} rej={d.get('rej', 0):4d} Mflop={d['flops']['total']/1e6:10.3f} "
                      f"err={d['err']:.3e} wall={d['wall']:.1f}s", flush=True)
            else:
                print('FAIL', j, d.get('error'), flush=True)


if __name__ == '__main__':
    main()
