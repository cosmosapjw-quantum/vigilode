"""Guard start-vector check: compressed FOV (m=16) from the stage-1-like smooth vector D f vs a seeded random vector
vs the exact FOV, on frozen D-scaled operators. EXPLORATORY."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import guard as G, rock4x, pr, swdrv
from nonnormal import true_fov
for nm, tm in (('advdiff-128-mild', 0.5), ('advdiff-128', 0.5), ('semi-96', 0.5), ('semi-384', 0.5), ('rot-96', 0.5), ('bruss1d-50', 5.0), ('vdp-96', 0.5), ('forc-96', 0.5)):
    p = pr.build(nm); q = dict(p); q['span'] = (p['span'][0], tm)
    y = np.asarray(swdrv.integrate(q, 1e-10, swdrv.make_arm('direct'))['y'])
    J = p['J'](tm, y).toarray(); rt = 1e-6; Dw = 1/(rt*p['ascale'] + rt*np.abs(y)); A = (Dw[:, None]*J)/Dw[None, :]
    W = true_fov(A)
    est = rock4x.power_rho(lambda t, z: J @ z, 0.0, np.ones(len(y)), J @ np.ones(len(y)), None)[0]
    T = p['span'][1] - p['span'][0]
    row = []
    for tag, v0 in (('smooth Df', Dw*p['f'](tm, y)), ('random', np.random.default_rng(12345).standard_normal(len(y))),
                    ('Df+random', Dw*p['f'](tm, y)/np.linalg.norm(Dw*p['f'](tm, y)) + np.random.default_rng(12345).standard_normal(len(y))/np.sqrt(len(y)))):
        H, m = G.arnoldi(lambda v: A @ v, v0, 16)
        Wc = G.fov_boundary(H); rz = G.ritz(H); rh = max(est, 1.2*rz['rho']); hi = 0.8*rock4x.HMAX_HRHO/rh
        hc = G.fov_step_bound(G.inflate(Wc, rh), rh, T, 1e-9*T, hi)
        row.append(f"{tag}: Im<={np.abs(Wc.imag).max():.1f} ReMax={Wc.real.max():.2f} h_FOV={hc:.3e}")
    rh = est; hi = 0.8*rock4x.HMAX_HRHO/rh
    ht = G.fov_step_bound(G.inflate(W, rh, kappa_im=1.0), rh, T, 1e-9*T, hi)
    print(f"{nm}: exact FOV Im<={np.abs(W.imag).max():.1f} ReMax={W.real.max():.2f} h_FOV(true)={ht:.3e} | " + ' | '.join(row), flush=True)
