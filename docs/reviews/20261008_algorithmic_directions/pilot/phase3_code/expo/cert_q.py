"""Quadrature-size sensitivity of the JAK certificate (cost vs validity), rot-96 / forc-96 rtol 1e-6, arm Ec. EXPLORATORY."""
import os, sys, json
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import run_e, pexprb as PX, cert
out = {}
for pn in ('rot-96', 'forc-96'):
    for Q in (8, 16, 32, 128):
        p = dict(run_e.problem(pn))
        r = PX.integrate(p, 1e-6, PX.make_arm(norm='l2', cert=True, Q=Q))
        s = cert.summarize(r['cert_rows'], r['mu_log'])
        s.update(Q=Q, cert_overhead=r['flops']['cert'] / r['flops']['total'], att=r['att'])
        s.pop('worst', None)
        out[f'{pn} Q={Q}'] = s
        print(pn, Q, {k: s[k] for k in ('violations_raw', 'violations_above_floor', 'n_above_floor', 'cert_overhead')}, s['ratio_quantiles'], flush=True)
json.dump(out, open(os.path.join(HERE, 'res/cert_q.json'), 'w'), indent=1)
