"""Calibration of the FOV guard: compressed (Arnoldi, m=24) vs true numerical range on test operators;
h_FOV (compressed+inflated) vs h_FOV (true FOV) and max_k ||R_s(hA)^k||_2 at those h. EXPLORATORY."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, math, json, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import guard, rock4x, corpus_v2 as cv, probs2d
def advdiff_A(n=128, D=0.01, a=5.0, r=-1.0):
    dx = 1.0/(n+1)
    return (np.diag((-2*D/dx**2 + r - a/dx)*np.ones(n)) + np.diag((D/dx**2 + a/dx)*np.ones(n-1), -1) + np.diag((D/dx**2)*np.ones(n-1), 1))
def true_fov(A, K=144):
    pts = []
    for th in np.linspace(0, 2*np.pi, K, endpoint=False):
        M = (np.exp(-1j*th)*A + np.exp(1j*th)*A.conj().T)/2
        w, X = np.linalg.eigh(M); x = X[:, -1]; pts.append(x.conj() @ A @ x)
    return np.array(pts)
def power_norms(M, kmax=400):
    P = np.eye(M.shape[0]); mx = 0; km = 0
    for k in range(1, kmax+1):
        P = M @ P; v = np.linalg.norm(P, 2)
        if v > mx: mx, km = v, k
        if v > 1e60: break
    return mx, km
def rock_mat(A, h, rho):
    n = A.shape[0]; Y = np.eye(n)
    un, _, _, s, _ = rock4x.step(lambda t, Z: A @ Z, 0.0, Y, A @ Y, h, rho)
    return un, s
cases = []
cases.append(('advdiff128-a5', advdiff_A(), np.sin(np.pi*np.arange(1,129)/129), 1.0))
cases.append(('advdiff128-a0.5-D0.05', advdiff_A(D=0.05, a=0.5), np.sin(np.pi*np.arange(1,129)/129), 1.0))
pb = probs2d.build('bruss1d-50'); y0 = pb['y0']; rt = 1e-6; Dw = 1/(rt + rt*np.abs(y0))
Jb = pb['J'](0, y0).toarray(); cases.append(('bruss50@y0 (D-scaled)', (Dw[:,None]*Jb)/Dw[None,:], Dw*pb['f'](0,y0), 10.0))
for fam in cv.CALIBRATION_FAMILIES:
    p = cv.build(fam, 96); r = p.reference(); tm = 0.5*(p.span[0]+p.span[1]); k = np.argmin(np.abs(r['times']-tm)); y = r['states'][k]
    Dw = 1/(0.01*rt + rt*np.abs(y)); J = p.J(tm, y)
    cases.append((fam+'@mid', (Dw[:,None]*J)/Dw[None,:], Dw*p.f(tm, y), p.span[1]-p.span[0]))
out = {}
for name, A, v0, T in cases:
    ev = np.linalg.eigvals(A); rho = np.abs(ev).max()
    W = true_fov(A)
    Hm, m = guard.arnoldi(lambda v: A @ v, v0, guard.M_GUARD)
    Hr, mr = guard.arnoldi(lambda v: A @ v, np.random.default_rng(0).standard_normal(len(v0)), guard.M_GUARD)
    Wc = guard.fov_boundary(Hm); Wr = guard.fov_boundary(Hr)
    # power estimate as ROCK4 would do on the linear map
    est, _, _, _ = rock4x.power_rho(lambda t, z: A @ z, 0.0, np.ones(len(v0)), A @ np.ones(len(v0)), None)
    pts_c = guard.inflate(Wc, est); pts_t = guard.inflate(W, est, kappa_im=1.0)
    hi = 0.8*rock4x.HMAX_HRHO/est
    hc = guard.fov_step_bound(pts_c, est, T, 1e-7*T, hi)
    ht = guard.fov_step_bound(pts_t, est, T, 1e-7*T, hi)
    rows = []
    for h in sorted(set([hc, ht] + [x for x in (hc*2, hc*4, ht*2, ht*4) if x <= hi])):
        if h <= 0: continue
        M, s = rock_mat(A, h, est)
        mx, km = power_norms(M)
        rows.append(dict(h=h, hrho=h*est, stages=s, maxpow=mx, k=km))
    rz = guard.ritz(Hm)
    out[name] = dict(n=A.shape[0], rho=rho, rho_hat=est, eig_re=[float(ev.real.min()), float(ev.real.max())], eig_im=float(np.abs(ev.imag).max()),
                     fov_true=dict(re=[float(W.real.min()), float(W.real.max())], im=float(np.abs(W.imag).max())),
                     fov_comp=dict(m=m, re=[float(Wc.real.min()), float(Wc.real.max())], im=float(np.abs(Wc.imag).max())),
                     fov_comp_rand=dict(re=[float(Wr.real.min()), float(Wr.real.max())], im=float(np.abs(Wr.imag).max())),
                     ritz=rz, h_fov_comp=hc, h_fov_true=ht, h_cap152=hi, pow=rows)
    o = out[name]
    print(f"{name}: n={o['n']} rho={rho:.1f} rho_hat={est:.1f} eigRe[{o['eig_re'][0]:.1f},{o['eig_re'][1]:.2f}] |Im eig|<={o['eig_im']:.1f} | "
          f"FOV true Re[{o['fov_true']['re'][0]:.1f},{o['fov_true']['re'][1]:.2f}] |Im|<={o['fov_true']['im']:.1f} | comp(m={m}) Re[{o['fov_comp']['re'][0]:.1f},{o['fov_comp']['re'][1]:.2f}] |Im|<={o['fov_comp']['im']:.1f} "
          f"| rand Im<={o['fov_comp_rand']['im']:.1f} | ritz angle={rz['angle']:.3f} | h_FOV comp={hc:.3e} true={ht:.3e} cap152={hi:.3e}", flush=True)
    for r_ in rows:
        print(f"     h={r_['h']:.3e} h*rho_hat={r_['hrho']:.1f} s={r_['stages']} max_k ||R^k||={r_['maxpow']:.3e} (k={r_['k']})", flush=True)
json.dump(out, open('res/fovtest.json', 'w'), indent=1, default=float)
