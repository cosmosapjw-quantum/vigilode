"""PROBE B3: offline test of a SHADOW ADMISSION RULE for E (EXPLORATORY).
Run the improved RODAS5P arm (mf) closed loop; at every accepted, accuracy-limited R step (err_R >= (0.9/5)^5) evaluate one
speculative E attempt (arm E2 unless given) from the same (t_n, y_n) with R's h (pexprb single=True) and record
    C_E / C_R      flops of the E attempt / flops of the accepted R attempt (incl. its own Krylov/orth/RHS; J/f0 shared,
                   E's f0/J evaluation is counted on the E side, so the ratio is conservative for E)
    err_E / err_R  embedded error estimates at the same h
    kappa          ||y_E - y_R||_w / max(err_E, err_R): estimator-consistency check (the difference of the two solutions
                   is a computable proxy of the larger true local error; kappa >> 1 means an estimator underestimates)
Predicted own-h cost ratio (order-5 I controllers on both sides): rho_hat = (C_E/C_R) * (err_E/err_R)^(1/5).
Rule under test: admit E when median(rho_hat) <= 0.8 and median(kappa) <= 2 over the window (here: the whole run)."""
import os, sys, json, math
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import numpy as np
import run_e, swdrv, pexprb as PX

E_SET = (0.9 / 5.0) ** 5


def shadow(pn, rt, earm='E2', every=1, max_steps=400):
    p = run_e.problem(pn)
    cnt = swdrv.Cnt()
    t0, tf = p['span']
    R = swdrv.Rodas(p, rt, swdrv.make_arm('mf'), cnt, t0, p['y0'], p['h0'])
    rows = []; acc = 0
    while R.t < tf - 1e-12 * max(1.0, abs(tf)):
        t_b, y_b = R.t, R.y.copy()
        f_before = swdrv.flops(p, cnt)['total']
        ok, h, err = R.attempt(tf)
        cR = swdrv.flops(p, cnt)['total'] - f_before
        if not ok:
            continue
        acc += 1
        if err < E_SET or acc % every:
            continue
        q = dict(p); q['span'] = (t_b, t_b + h); q['y0'] = y_b
        e = PX.integrate(q, rt, PX.make_arm(**run_e.EARMS[earm]), h0=h, single=True)
        atol = rt * p['ascale']
        w = atol + rt * np.maximum(np.abs(y_b), np.abs(R.y))
        dif = float(np.sqrt(np.mean(((e['y'] - R.y) / w) ** 2)))
        rows.append(dict(t=t_b, h=h, errR=err, errE=e['err'], cR=cR, cE=e['flops']['total'],
                         kappa=dif / max(err, e['err'], 1e-300)))
        if len(rows) >= max_steps:
            break
    if not rows:
        return dict(n=0)
    cr = np.array([r['cE'] / r['cR'] for r in rows]); er = np.array([r['errE'] / r['errR'] for r in rows])
    rho = cr * er ** 0.2; kap = np.array([r['kappa'] for r in rows])
    return dict(n=len(rows), med_cost_ratio=float(np.median(cr)), med_err_ratio=float(np.median(er)),
                med_rho_hat=float(np.median(rho)), q25_rho=float(np.quantile(rho, 0.25)), q75_rho=float(np.quantile(rho, 0.75)),
                med_kappa=float(np.median(kap)), q90_kappa=float(np.quantile(kap, 0.9)), frac_kappa_gt2=float(np.mean(kap > 2)),
                admit=bool(np.median(rho) <= 0.8 and np.median(kap) <= 2.0))


def main():
    probs = sys.argv[1].split(',') if len(sys.argv) > 1 else ['rot-96', 'semi-96', 'forc-96', 'hires-96', 'rob-96', 'vdp-96',
                                                               'bruss1d-50', 'bruss1d-160', 'semi-384', 'advdiff-128', 'vdp2', 'hires8']
    earm = sys.argv[2] if len(sys.argv) > 2 else 'E2'
    fn = os.path.join(HERE, f'res/shadow_{earm}.json')
    out = json.load(open(fn)) if os.path.exists(fn) else {}
    for pn in probs:
        for rt in (1e-6, 1e-8):
            k = f'{pn} {rt:g}'
            if k in out:
                continue
            s = shadow(pn, rt, earm)
            out[k] = s
            print(k, earm, json.dumps(s), flush=True)
            json.dump(out, open(fn, 'w'), indent=1)


if __name__ == '__main__':
    main()
