"""PRED x target interaction diagnostic (probe B1). For each attempt, the controller's error estimate err (from the
inexact stage chain) is compared with errx from the exact-solve chain at the same (t, y, h) (uncounted LU):
relative noise |err/errx - 1|, accept/reject decision flips (err <= 1 xor errx <= 1), and the per-step solve
contamination ||y_new - y_new^exact||_w. Appends to res/noise.jsonl."""
import json, os, sys, time, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stack, run

pname = sys.argv[1]; arms = sys.argv[2].split(','); rtols = [float(x) for x in sys.argv[3].split(',')]
for rtol in rtols:
    for an in arms:
        p = run.problem(pname); t0 = time.time()
        r = stack.integrate(p, rtol, run.ARMS[an], noise=True, max_att=6000, cap_ok=True)
        N = np.array(r['noise'])
        err, errx, dyw = N[:, 0], N[:, 1], N[:, 2]
        fin = np.isfinite(err) & (errx > 0)
        rel = np.abs(err[fin]/errx[fin] - 1.0)
        big = fin & (errx >= 0.05)                      # controller-relevant attempts (errx >= 0.05)
        relb = np.abs(err[big]/errx[big] - 1.0) if big.any() else np.array([0.0])
        absd = np.abs(err[fin] - errx[fin])              # absolute perturbation of the error estimate (tol units)
        flips = int(np.sum((err <= 1.0) != (errx <= 1.0)))
        o = dict(prob=pname, arm=an, rtol=rtol, capped=r['capped'], att=r['att'], acc=r['acc'], rej=r['rej'], lin_fail=r['lin_fail'], jvp=r['jvp'],
                 noise_med=float(np.median(rel)), noise_p90=float(np.percentile(rel, 90)), noise_max=float(rel.max()),
                 frac_gt_0p1=float(np.mean(rel > 0.1)), frac_gt_0p01=float(np.mean(rel > 0.01)), flips=flips,
                 dy_med=float(np.median(dyw)), dy_max=float(dyw.max()), n_big=int(big.sum()),
                 rb_med=float(np.median(relb)), rb_p90=float(np.percentile(relb, 90)), rb_max=float(relb.max()),
                 rb_gt_0p1=float(np.mean(relb > 0.1)), abs_med=float(np.median(absd)), abs_p90=float(np.percentile(absd, 90)),
                 abs_max=float(absd.max()), sec=time.time() - t0)
        with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'res', 'noise.jsonl'), 'a') as fo:
            fo.write(json.dumps(o) + '\n')
        print(f"{pname} {an:5s} {rtol:<8.2g} att={o['att']} rej={o['rej']} [errx>=0.05: n={o['n_big']} med={o['rb_med']:.1e} p90={o['rb_p90']:.1e} max={o['rb_max']:.1e} >0.1:{o['rb_gt_0p1']:.3f}] abs p90={o['abs_p90']:.1e} max={o['abs_max']:.1e} noise med={o['noise_med']:.1e} p90={o['noise_p90']:.1e} "
              f"max={o['noise_max']:.1e} >0.1:{o['frac_gt_0p1']:.3f} flips={flips} dy_med={o['dy_med']:.1e} dy_max={o['dy_max']:.1e}", flush=True)
