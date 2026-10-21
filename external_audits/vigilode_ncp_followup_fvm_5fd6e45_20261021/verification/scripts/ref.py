"""Independent heat reference: (a) scipy.linalg.expm on dense circulant (float64),
(b) mpmath Taylor-series propagation of exp(A h) on the sparse stencil at dps=60.
Neither uses a DFT/eigendecomposition."""
import json, sys, time
import numpy as np
import scipy.linalg as sla
import mpmath as mp

C = sys.argv[1]
req = json.load(open(C + '/requests/HEAT_BASELINE_REQUEST.json'))['case']
y0 = np.array(req['initial_state'], dtype=float)
n = len(y0)
D = req['parameters']['diffusion'] * n * n
T = req['output_times']
assert len(T) == 41

# (a) scipy expm per output time directly
A = np.zeros((n, n))
for i in range(n):
    A[i, i] = -2 * D
    A[i, (i - 1) % n] += D
    A[i, (i + 1) % n] += D
ref_a = np.array([sla.expm(A * t) @ y0 for t in T])

# (b) mpmath Taylor stepping with substeps of size h = 0.025/16; step to each output time
mp.mp.dps = 60
Dm = mp.mpf(req['parameters']['diffusion']) * n * n
y = [mp.mpf(float(x)) for x in y0]


def Av(v):
    return [Dm * (v[(i - 1) % n] - 2 * v[i] + v[(i + 1) % n]) for i in range(n)]


def taylor_step(v, h):
    out = list(v)
    term = list(v)
    k = 0
    while True:
        k += 1
        term = [x * h / k for x in Av(term)]
        out = [a + b for a, b in zip(out, term)]
        if max(abs(x) for x in term) < mp.mpf(10) ** (-70):
            break
    return out


t0 = time.time()
ref_b = [[float(x) for x in y]]
tcur = mp.mpf(0)
for t in T[1:]:
    tt = mp.mpf(t)  # exact float64 value of the output time
    dt = tt - tcur
    m = 16
    for _ in range(m):
        y = taylor_step(y, dt / m)
    tcur = tt
    ref_b.append([float(x) for x in y])
ref_b = np.array(ref_b)
print('mp taylor seconds', time.time() - t0)

R = json.load(open(C + '/results/REFERENCE.json'))
ref_r = np.array(R['states'])
times_r = R['output_times']
print('ref times equal request:', times_r == T)
print('max|expm - mpTaylor|', np.max(np.abs(ref_a - ref_b)))
print('max|REFERENCE - mpTaylor|', np.max(np.abs(ref_r - ref_b)))
print('max|REFERENCE - expm|', np.max(np.abs(ref_r - ref_a)))
# relative scaled diff vs budget denominators
den = 1e-9 + 1e-6 * np.abs(ref_b)
print('max scaled |REFERENCE - mpTaylor|/den', np.max(np.abs(ref_r - ref_b) / den))
print('max scaled |expm - mpTaylor|/den', np.max(np.abs(ref_a - ref_b) / den))
np.save(sys.argv[2] + '/ref_mp.npy', ref_b)
np.save(sys.argv[2] + '/ref_expm.npy', ref_a)
