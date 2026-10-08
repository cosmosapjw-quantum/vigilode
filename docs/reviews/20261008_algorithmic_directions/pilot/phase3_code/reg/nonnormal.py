"""ROCK4 stability on nonnormal operators and whether the FOV guard keeps it out (PROBE B2, EXPLORATORY).
For each frozen operator A (WRMS-scaled D J D^-1 at rtol 1e-6, and the unscaled 2-norm where noted):
  * the step sizes plain ROCK4 actually takes on the trajectory (rtol 1e-6, 1e-9): median / p90 h and stages;
  * max_{k<=K} ||R_s(hA)^k||_2 at those h, and the exact-flow growth max_{t<=K h} ||exp(tA)||_2 for comparison
    (Q = ||R^k|| / max(1, ||exp(khA)||) isolates method-induced amplification);
  * the guard's h_FOV from the m = 16 Arnoldi compression (inflated) and from the exact FOV, and ||R^k|| there.
Fault injection: power-iteration rho x 0.25 (plain ROCK4 and the switched driver) at rtol 1e-4 / 1e-6."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, json, math, time
import numpy as np
from scipy.linalg import expm
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import guard as G, rock4x, pr, swdrv

KMAX = 300


def true_fov(A, K=96):
    pts = []
    for th in np.linspace(0, 2 * np.pi, K, endpoint=False):
        M = (np.exp(-1j * th) * A + np.exp(1j * th) * A.conj().T) / 2
        w, X = np.linalg.eigh(M); x = X[:, -1]; pts.append(x.conj() @ A @ x)
    return np.array(pts)


def rmat(A, h, rho):
    n = A.shape[0]; Y = np.eye(n)
    un, _, _, s, _ = rock4x.step(lambda t, Z: A @ Z, 0.0, Y, A @ Y, h, rho)
    return un, s


def growth(A, h, rho, kmax=KMAX):
    M, s = rmat(A, h, rho)
    E = expm(h * A)
    P = np.eye(A.shape[0]); Q = np.eye(A.shape[0]); mx = 0; km = 0; qmax = 0; ex_at = 0
    for k in range(1, kmax + 1):
        P = M @ P; Q = E @ Q
        v = np.linalg.norm(P, 2); e = np.linalg.norm(Q, 2)
        if v > mx: mx, km, ex_at = v, k, e
        qmax = max(qmax, v / max(1.0, e))
        if v > 1e80: break
    return dict(h=h, s=s, maxpow=mx, k=km, exact_at_k=ex_at, Qmax=qmax)


def traj_steps(p, rt):
    c = swdrv.Cnt()
    r = rock4x.integrate(p['f'], p['span'][0], p['y0'], p['span'][1], rt, rt * p['ascale'], h0=p['h0'], cnt=c)
    hist = r['branch'].hist
    hs = np.array([q[1] for q in hist if q[0]]); ss = np.array([q[2] for q in hist if q[0]])
    return dict(rtol=rt, acc=r['acc'], rej=r['rej'], err=p['err'](r['y']), h_med=float(np.median(hs)),
                h_p90=float(np.percentile(hs, 90)), s_med=float(np.median(ss)), rho=float(r['branch'].rho))


def study(name, p, tpt, scaled=True, rt=1e-6):
    t, y = tpt
    J = p['J'](t, y).toarray()
    if scaled:
        Dw = 1.0 / (rt * p['ascale'] + rt * np.abs(y))
        A = (Dw[:, None] * J) / Dw[None, :]
    else:
        A = J
    ev = np.linalg.eigvals(A)
    W = true_fov(A)
    est, _, _, _ = rock4x.power_rho(lambda tt, z: J @ z, 0.0, np.ones(len(y)), J @ np.ones(len(y)), None)
    v0 = (Dw * p['f'](t, y)) if scaled else p['f'](t, y)
    Hm, m = G.arnoldi(lambda v: A @ v, G.start_vector(v0, 0), G.M_GUARD)      # the driver's guard start
    rz = G.ritz(Hm)
    rho_hat = max(est, 1.2 * rz['rho'])
    T = p['span'][1] - p['span'][0]
    hi = 0.8 * rock4x.HMAX_HRHO / rho_hat
    hc = G.fov_step_bound(G.inflate(G.fov_boundary(Hm), rho_hat), rho_hat, T, 1e-9 * T, hi)
    ht = G.fov_step_bound(G.inflate(W, rho_hat, kappa_im=1.0), rho_hat, T, 1e-9 * T, hi)
    out = dict(name=name, n=A.shape[0], scaled=scaled, t=t, rho=float(np.abs(ev).max()), rho_hat=float(rho_hat),
               eig_re=[float(ev.real.min()), float(ev.real.max())], eig_im=float(np.abs(ev.imag).max()),
               fov_re=[float(W.real.min()), float(W.real.max())], fov_im=float(np.abs(W.imag).max()),
               ritz_angle_stiff=rz['angle'], ritz_angle_all=rz['angle_all'], h_fov_comp=hc, h_fov_true=ht,
               henrici=float(math.sqrt(max(0.0, np.linalg.norm(A, 'fro') ** 2 - np.sum(np.abs(ev) ** 2))) / np.linalg.norm(A, 'fro')))
    return out, A, rho_hat


def main():
    res = {'operators': [], 'fault': []}
    cases = []
    pa = pr.build('advdiff-128'); cases.append(('advdiff-128 (D=0.01, a=5) @ t=0.5', pa, 0.5, True))
    pm = pr.build('advdiff-128-mild'); cases.append(('advdiff-128-mild (D=0.05, a=0.5) @ t=0.5', pm, 0.5, True))
    for nm in ('semi-96', 'semi-384', 'rot-96', 'forc-96', 'vdp-96', 'hires-96', 'rob-96'):
        cases.append((nm + ' @ mid', pr.build(nm), None, True))
    cases.append(('bruss1d-50 @ t=5 (near-normal control)', pr.build('bruss1d-50'), 5.0, True))
    for name, p, tm, sc in cases:
        t0 = time.time()
        T0, T1 = p['span']
        tmid = tm if tm is not None else 0.5 * (T0 + T1)
        # state at tmid from a tight direct run
        q = dict(p); q['span'] = (T0, tmid)
        rr = swdrv.integrate(q, 1e-10, swdrv.make_arm('direct'))
        y = np.asarray(rr['y'])
        st, A, rho_hat = study(name, p, (tmid, y), scaled=sc)
        tr = [traj_steps(p, rt) for rt in (1e-6, 1e-9)]
        st['trajectory'] = tr
        hs = sorted(set([tr[0]['h_med'], tr[0]['h_p90'], tr[1]['h_med'], st['h_fov_comp'], st['h_fov_true']]))
        st['growth'] = [growth(A, h, rho_hat) for h in hs if h > 0]
        res['operators'].append(st)
        print(f"{name}: n={st['n']} rho={st['rho']:.1f} rho_hat={rho_hat:.1f} eigRe[{st['eig_re'][0]:.1f},{st['eig_re'][1]:.2f}] |Im eig|<={st['eig_im']:.1f} "
              f"FOV Re[{st['fov_re'][0]:.1f},{st['fov_re'][1]:.2f}] |Im|<={st['fov_im']:.1f} Henrici={st['henrici']:.2f} "
              f"ritz angle stiff/all={st['ritz_angle_stiff']:.3f}/{st['ritz_angle_all']:.3f} h_FOV comp={st['h_fov_comp']:.3e} true={st['h_fov_true']:.3e}", flush=True)
        for x in tr:
            print(f"     plain ROCK4 rtol={x['rtol']:g}: acc={x['acc']} rej={x['rej']} err={x['err']:.3e} h_med={x['h_med']:.3e} h_p90={x['h_p90']:.3e} s_med={x['s_med']:.0f}", flush=True)
        for g_ in st['growth']:
            print(f"     h={g_['h']:.3e} s={g_['s']} max_k<= {KMAX} ||R^k||={g_['maxpow']:.3e} (k={g_['k']}, exact ||e^(khA)||={g_['exact_at_k']:.3e}) Qmax={g_['Qmax']:.3e}", flush=True)
        print(f"     [{time.time()-t0:.1f}s]", flush=True)
    json.dump(res, open('res/nonnormal.json', 'w'), indent=1, default=float)
    # ---------------- fault injection: power-iteration rho x 0.25
    orig = rock4x.power_rho
    def faulty(*a, **k):
        est, z, nf, vf = orig(*a, **k)
        return 0.25 * est, z, nf, vf
    origr = G.ritz
    def faulty_ritz(*a, **k):
        d = dict(origr(*a, **k)); d['rho'] *= 0.25; return d
    for nm in ('bruss1d-50', 'bruss1d-160', 'bruss2d-32', 'advdiff-128', 'semi-96'):
        p = pr.build(nm)
        for rt in (1e-4, 1e-6):
            for arm in ('rock4', 'sw'):
                row = dict(problem=nm, rtol=rt, arm=arm)
                tags = (('clean', orig, origr), ('rho x0.25', faulty, origr))
                if arm == 'sw':
                    tags = tags + (('rho x0.25 power+Krylov', faulty, faulty_ritz),)
                for tag, fn, fr in tags:
                    rock4x.power_rho = fn; swdrv.rock4x.power_rho = fn; G.ritz = fr; swdrv.G.ritz = fr
                    try:
                        r = swdrv.integrate(p, rt, swdrv.make_arm(arm), max_att=200000)
                        row[tag] = dict(err=p['err'](np.asarray(r['y'])), att=r['att'], rej=r['rej'],
                                        Mflop=r['flops']['total'] / 1e6, hand=r.get('n_handoff'), rev=r.get('n_revert'))
                    except Exception as e:
                        row[tag] = dict(fail=str(e))
                rock4x.power_rho = orig; swdrv.rock4x.power_rho = orig; G.ritz = origr; swdrv.G.ritz = origr
                res['fault'].append(row)
                print('FAULT', json.dumps(row, default=float), flush=True)
    json.dump(res, open('res/nonnormal.json', 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
