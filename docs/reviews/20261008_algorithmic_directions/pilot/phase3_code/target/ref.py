"""Extended-precision (x87 80-bit longdouble, eps 1.1e-19) RODAS5P with exact (LU) stage solves, used as
reference at rtol 1e-14 and 1e-15; the difference between the two is the stated reference uncertainty.
Coefficients parsed from the decimal snapshot at extended precision. Cross-checked against NATIVE.json
(SciPy Radau rtol 1e-13)."""
import json, sys, time, numpy as np
sys.path.insert(0, '.')
import tprobs
LD = np.longdouble
d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = LD(d['gamma']); A = np.array([[LD(x) for x in r] for r in d['A']], dtype=LD)
C = np.array([[LD(x) for x in r] for r in d['C']], dtype=LD); c = np.array([LD(x) for x in d['c']], dtype=LD)
bc = np.array([LD(x) for x in d['b_code']], dtype=LD); S = 8
# Gamma row sums in extended precision: solve (I/g - C) Gam = I by forward substitution (C strictly lower)
M = np.eye(S, dtype=LD)/g - C
Gam = np.zeros((S, S), dtype=LD)
for i in range(S):
    Gam[i] = (np.eye(S, dtype=LD)[i] - M[i, :i] @ Gam[:i])/M[i, i]
grow = Gam.sum(axis=1)

def lu(Wm):
    a = Wm.copy(); n = a.shape[0]; piv = np.arange(n)
    for k in range(n):
        p = k + int(np.argmax(np.abs(a[k:, k])))
        if p != k:
            a[[k, p]] = a[[p, k]]; piv[[k, p]] = piv[[p, k]]
        if a[k, k] == 0: continue
        a[k+1:, k] /= a[k, k]
        a[k+1:, k+1:] -= np.outer(a[k+1:, k], a[k, k+1:])
    return a, piv

def solve(F, b):
    a, piv = F; n = a.shape[0]; x = b[piv].copy()
    for i in range(1, n):
        x[i] -= a[i, :i] @ x[:i]
    for i in range(n-1, -1, -1):
        x[i] = (x[i] - a[i, i+1:] @ x[i+1:])/a[i, i]
    return x

def integrate(p, rtol, h0=1e-6):
    f, Jf = p['f'], p['J']; ft = p['ft']
    t0, tf = p['span']; t, y = LD(t0), p['y0'].astype(LD); h = LD(h0); tf = LD(tf)
    rtol = LD(rtol); atol = rtol*LD(p['ascale']); n = len(y); acc = rej = 0; last = None
    while t < tf:
        h = min(h, tf - t)
        if last is not None and h >= last: h = last*(1 - LD(1e-15))
        Jm = Jf(t, y); f0 = f(t, y); ftv = ft(t, y) if ft is not None else np.zeros(n, dtype=LD)
        hg = h*g; F = lu(np.eye(n, dtype=LD) - hg*Jm); U = np.zeros((S, n), dtype=LD)
        for i in range(S):
            fi = f0 if i == 0 else f(t + c[i]*h, y + A[i, :i] @ U[:i])
            U[i] = solve(F, hg*fi + g*(C[i, :i] @ U[:i]) + h*hg*grow[i]*ftv)
        yn = y + bc @ U
        sc = atol + rtol*np.maximum(np.abs(y), np.abs(yn))
        err = float(np.sqrt(np.mean((U[-1]/sc)**2)))
        if err <= 1:
            acc += 1; t = t + h; y = yn; last = None
            h = h*LD(min(max(0.9*err**-0.2, 0.2), 5.0) if err > 0 else 5.0)
        else:
            rej += 1; last = h; h = h*LD(min(max(0.9*err**-0.2, 0.2), 0.9))
    return y, acc, rej

if __name__ == '__main__':
    name = sys.argv[1]
    p = tprobs.PROBLEMS[name]()
    if p['ft'] is not None and name not in ('pr',):
        pass
    out = {}
    for rt in (1e-14, 1e-15):
        t0 = time.time(); y, acc, rej = integrate(p, rt)
        out[str(rt)] = dict(y=[repr(float(v)) for v in y], y_ld=[str(v) for v in y], acc=acc, rej=rej, sec=time.time()-t0)
        print(name, rt, acc, rej, f'{time.time()-t0:.0f}s', flush=True)
    y14 = np.array([LD(v) for v in out['1e-14']['y_ld']]); y15 = np.array([LD(v) for v in out['1e-15']['y_ld']])
    unc = float(np.max(np.abs(y14 - y15)/np.maximum(np.abs(y15), 1e-10)))
    out['uncertainty_componentwise'] = unc
    try:
        nat = json.load(open('/home/user/wt-speed/research/stiff_native_benchmark_20261001/NATIVE.json'))['references']
        key = {'hires': 'hires', 'robertson': 'robertson', 'vdp': 'van-der-pol-mu1000', 'bruss50': 'brusselator-1d-50'}[name]
        nv = np.array(nat[key]['final_state'])
        out['native_radau_diff'] = float(np.max(np.abs(nv - y15.astype(float))/np.maximum(np.abs(y15.astype(float)), 1e-10)))
    except Exception as e:
        out['native_radau_diff'] = None
    print(name, 'unc', unc, 'native diff', out['native_radau_diff'])
    json.dump(out, open(f'ref_{name}.json', 'w'), indent=1)
