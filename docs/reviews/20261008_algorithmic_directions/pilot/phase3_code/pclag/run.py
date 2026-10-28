"""Run one (problem, rtol, arm-label) cell, append a JSON line to res/<problem>.jsonl.
usage: python3 run.py <problem> <rtol> <label> [<label> ...]
labels: direct, mf, lag, lag-rho (no iteration trigger), lag-w4 (window [1/4,4]), lin, dst, mf-m200 (maxit 200)"""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, time
import numpy as np
import driver, probs2d

ARMS = {
    'direct': dict(kind='direct'),
    'mf': dict(kind='mf'),
    'mf-m200': dict(kind='mf', maxit=200),
    'lag': dict(kind='lag'),
    'lag-rho': dict(kind='lag', iter_trigger=False),
    'lag-w4': dict(kind='lag', lo=0.25, hi=4.0),
    'lag-w15': dict(kind='lag', lo=1/1.5, hi=1.5),
    'lin': dict(kind='lin'),
    'lin-w4': dict(kind='lin', lo=0.25, hi=4.0),
    'dst': dict(kind='dst'),
}
H0 = {'semilin2d': 0.01, 'bruss2d': 1e-6}

def main():
    pname = sys.argv[1]; rtol = float(sys.argv[2]); labels = sys.argv[3:]
    p = probs2d.build(pname)
    h0 = H0[pname.split('-')[0]]
    os.makedirs('res', exist_ok=True)
    for lab in labels:
        arm = driver.make_arm(**ARMS[lab])
        t0 = time.time()
        try:
            r = driver.integrate(p, rtol, arm, h0=h0)
        except Exception as e:
            r = dict(error=repr(e))
        rec = dict(problem=pname, rtol=rtol, arm=lab, n=p['n'], nnzJ=p['nnzJ'], colors=p['colors'], h0=h0, **r)
        if 'error' not in r:
            rec['flops'] = driver.flops(p, r, arm['kind'])
            rec['flops_fd'] = driver.flops(p, r, arm['kind'], jvp_model='fd')
            if p['exact'] is not None:
                ex = p['exact'](p['span'][1]); y = np.array(r['y'])
                rec['err_rel'] = float(np.max(np.abs(y - ex)) / np.max(np.abs(ex)))
                rec['err_cw'] = float(np.max(np.abs(y - ex) / np.maximum(np.abs(ex), 1e-10)))
                sc = p['ascale'] * rtol + rtol * np.abs(ex)
                rec['err_wrms_tol'] = float(np.sqrt(np.mean(((y - ex) / sc) ** 2)))
        with open(f'res/{pname}.jsonl', 'a') as fh:
            fh.write(json.dumps(rec) + '\n')
        if 'error' in r:
            print(f'{pname} {rtol:g} {lab}: ERROR {r["error"]}', flush=True)
        else:
            fl = rec['flops']
            print(f"{pname} {rtol:g} {lab:8s} att={r['att']:4d} rej={r['rej']:3d} lf={r['lin_fail']} jvp/att={r['jvp']/r['att']:6.1f} "
                  f"pcs/att={r['pcs']/r['att']:5.1f} lu/acc={r['lus']/r['acc']:.3f} cols/stage={r.get('mean_cols_stage',0):5.1f} "
                  f"Gflop={fl['total']/1e9:8.3f} [lu {fl['lu']/1e9:.3f} pc {fl['pc']/1e9:.3f} jvp {fl['jvp']/1e9:.3f} orth {fl['orth']/1e9:.3f} "
                  f"lus {fl['lusolve']/1e9:.3f}] err={rec.get('err_rel', float('nan')):.3e} wall={r['wall']:.1f}s", flush=True)

if __name__ == '__main__':
    main()
