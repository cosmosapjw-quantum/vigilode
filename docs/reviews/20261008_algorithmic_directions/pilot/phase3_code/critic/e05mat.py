"""E-05 operator replica: A = Q (D + N) Q^T, n = 256, same LCG/Householder construction and seed as
research/adversarial_audit_20260927/harness/src/bin/e05_krylov_stress.rs (read-only worktree a49f7e4)."""
import numpy as np
M64 = (1 << 64) - 1

class Lcg:
    def __init__(self, s): self.s = s
    def next_f64(self):
        self.s = (self.s*6364136223846793005 + 1442695040888963407) & M64
        return (self.s >> 11)/float(1 << 53)
    def uniform(self): return 2.0*self.next_f64() - 1.0

def random_orthogonal(n, rng):
    q = np.eye(n)
    for k in range(n):
        v = np.zeros(n)
        for i in range(k, n): v[i] = rng.uniform()
        nv = np.sqrt(np.sum(v*v))
        if nv == 0: continue
        v /= nv
        qv = q @ v
        q -= 2.0*np.outer(qv, v)
    return q

def build_a(n, s, dmax, rng):
    q = random_orthogonal(n, rng)
    inner = np.zeros((n, n)); diag = np.empty(n)
    for i in range(n):
        d = -(10.0**((i/(n - 1))*np.log10(dmax)))
        inner[i, i] = d; diag[i] = d
        for j in range(i + 1, n):
            inner[i, j] = s*rng.uniform()
    return q @ inner @ q.T, diag, q, inner

def e05_mats(n=256, scales=(0.0, 1.0, 10.0, 100.0)):
    rng = Lcg(20260927)
    b = np.array([rng.uniform() for _ in range(n)]); b /= np.linalg.norm(b)
    out = {}
    for s in scales:
        a, d, q, inner = build_a(n, s, 1e4, rng)
        out[s] = (a, q, inner)
    return out, b

if __name__ == '__main__':
    import time, scipy.linalg as sl
    t0 = time.time(); M, b = e05_mats(); print('built', time.time() - t0)
    for s, (a, q, inner) in M.items():
        ev = np.linalg.eigvals(a); H = (a + a.T)/2; mu2 = np.linalg.eigvalsh(H).max()
        dep = np.linalg.norm(np.triu(inner, 1))/np.linalg.norm(inner)
        g = []
        for t in (1e-4, 1e-3, 1e-2, 0.1, 1.0, 3.0):
            g.append(np.linalg.norm(sl.expm(t*a), 2))
        print(f's={s}: max Re ev {ev.real.max():.3g} min {ev.real.min():.3g} mu2 {mu2:.3g} ||A|| {np.linalg.norm(a,2):.3g} '
              f'Henrici rel {dep:.3g} ||e^tA|| at t=1e-4..3: ' + ' '.join(f'{x:.3g}' for x in g), flush=True)
