"""Why does the WRMS-scaled KIOPS arm (E) lose accuracy on vdp-96 while E2 (2-norm) and Ed (exact phi) agree?
Audit every phi action of E and E2 (actual error vs dense augmented expm, KIOPS estimate, tolerance). EXPLORATORY."""
import os, sys, json
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import numpy as np, run_e, pexprb as PX
out = {}
for pn, rt in (('vdp-96', 1e-6), ('vdp-96', 1e-4), ('rot-96', 1e-6)):
    p = run_e.problem(pn)
    for arm, kw in (('E', dict(check_actual=True)), ('E2', dict(norm='l2', check_actual=True))):
        r = PX.integrate(p, rt, PX.make_arm(**kw))
        rows = r['cert_rows']
        act = np.array([q['actual'] for q in rows]); tol = np.array([q['tol'] for q in rows]); est = np.array([q['est'] for q in rows])
        nrm = np.array([q['norm'] for q in rows])
        key = f'{pn} {rt:g} {arm}'
        mus = np.array([m[1] for m in r['mu_log']])
        s = dict(att=r['att'], rej=r['rej'], err=float(p['err'](r['y'])), n_act=len(rows),
                 frac_actual_gt_tol=float(np.mean(act > 1.4 * tol)), frac_actual_gt_10tol=float(np.mean(act > 10 * tol)),
                 max_actual_over_tol=float(np.max(act / tol)), med_actual_over_est=float(np.median(act / np.maximum(est, 1e-300))),
                 max_actual_over_est=float(np.max(act / np.maximum(est, 1e-300))),
                 frac_steps_mu_le_0=float(np.mean(mus <= 0)), mu_max=float(mus.max()))
        per = {}
        for c in 'ABCD':
            sel = np.array([q['call'] == c for q in rows])
            per[c] = dict(max_act_over_tol=float(np.max(act[sel] / tol[sel])), frac_gt_tol=float(np.mean(act[sel] > 1.4 * tol[sel])))
        s['per_call'] = per
        out[key] = s
        print(key, json.dumps(s), flush=True)
json.dump(out, open(os.path.join(HERE, 'res/diag_vdp.json'), 'w'), indent=1)
