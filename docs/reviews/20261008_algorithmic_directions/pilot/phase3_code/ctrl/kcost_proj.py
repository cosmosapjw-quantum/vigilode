"""CTRL-NULLS (c) re-check under the new Krylov baseline (proj-stop, and INO tf1): Krylov cost per unit time vs
step size at accepted states of the Bruss-160 trajectory. Full 8-stage closed-loop attempt at h' = s*h."""
import sys, math, json
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *
N = int(sys.argv[1]); rtol = float(sys.argv[2])
p = bruss(N); n = 2 * N; f, Jf = p['f'], p['J']; atol = rtol
r = dense_integrate(p, rtol, make_ctrl('I'), record=True)
acc = [(t, h) for (t, h, e, a) in r['rec'] if a]
y = p['y0'].copy(); states = []
for (t0, h) in acc:
    states.append((t0, y.copy(), h))
    J = Jf(t0, y); f0 = f(t0, y)
    lu = lu_factor(np.eye(n) / (h * g) - J)
    y, _, U = dense_attempt(p, lu, f0, None, t0, y, h)
def attempt_cost(t, y, h, mode, theta=0.001):
    J = Jf(t, y); f0 = f(t, y); W = np.eye(n) - h * g * J; D = 1.0 / (atol + rtol * np.abs(y))
    Ws = (D[:, None] * W) / D[None, :]; eps = np.minimum(theta / (8 * TY), theta / (8 * TE))
    U = np.zeros((S, n)); tot = dict(jvp=0, ip=0, vu=0, vec=0)
    for i in range(S):
        fi = f0 if i == 0 else f(t + c[i] * h, y + A[i, :i] @ U[:i])
        b = h * g * fi + g * (C[i, :i] @ U[:i])
        if mode == 'proj':
            x, cnt = gmres(lambda v: W @ v, b, max(g * 1e-14, 1e-10 * np.linalg.norm(b)))
        else:
            tgt = eps[i] * math.sqrt(n) * (min(1.0, 0.59 / 0.5) ** 1.2)
            if i == S - 1: tgt = min(tgt, 2e-5 * math.sqrt(n))
            xs, cnt = gmres(lambda v: Ws @ v, D * b, max(tgt, 1e-14 * np.linalg.norm(D * b))); x = xs / D
        U[i] = x
        for k in tot: tot[k] += cnt[k]
    fl = tot['jvp'] * 10 * n + (tot['ip'] + tot['vu'] + tot['vec']) * 2 * n
    return tot['jvp'], fl
SC = [0.35, 0.5, 0.7, 1.0, 1.4]
idx = np.linspace(len(states) // 6, len(states) - 2, 8).astype(int)
res = {}
for mode in ('proj', 'tf1'):
    tj = np.zeros(len(SC)); tf_ = np.zeros(len(SC))
    for k in idx:
        t0, y, h = states[k]
        row = [attempt_cost(t0, y, s * h, mode) for s in SC]
        jv = np.array([q[0] for q in row]); fl = np.array([q[1] for q in row])
        print(f"{mode} t={t0:6.3f} h={h:.3e} JVP/attempt at s={SC}: {jv.tolist()}", flush=True)
        tj += (jv / np.array(SC)) / (jv[SC.index(1.0)]); tf_ += (fl / np.array(SC)) / fl[SC.index(1.0)]
    tj /= len(idx); tf_ /= len(idx)
    print(f"{mode}: mean relative JVP per unit time {dict(zip(SC, tj.round(3)))}; flops per unit time {dict(zip(SC, tf_.round(3)))}", flush=True)
    res[mode] = dict(jvp=tj.tolist(), flops=tf_.tolist())
json.dump(dict(scales=SC, res=res, N=N, rtol=rtol), open(f'{HERE}/kcost_proj_{N}_{rtol:g}.json', 'w'))
