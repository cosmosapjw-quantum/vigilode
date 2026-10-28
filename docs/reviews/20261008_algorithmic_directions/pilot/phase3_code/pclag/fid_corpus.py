"""Fidelity: driver 'direct' arm on the on-contract corpus semilinear n=1536 (32x48) grid vs recorded Rust
v2 dense-arm counters (from probe/corpus/replica_dense_arm_n1536.json, read-only), h0 = span/100."""
import json
import numpy as np
import driver, probs2d
R = json.load(open('/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/corpus/replica_dense_arm_n1536.json'))
p = probs2d.build('semilin2d-32x48')
out = {}
for rt in (1e-4, 1e-6, 1e-8):
    key = [k for k in R if 'semilinear' in k and f'rtol-{rt:.0e}'.replace('e-0', 'e-') in k][0]
    rust = R[key]['rust']
    for kind in ('direct', 'mf', 'lag'):
        r = driver.integrate(p, rt, driver.make_arm(kind), h0=0.01)
        ex = p['exact'](p['span'][1]); y = np.array(r['y'])
        e = float(np.max(np.abs(y - ex)) / np.max(np.abs(ex)))
        line = dict(case=key, kind=kind, rust_att=rust['attempts'], rust_rej=rust['rejected'], att=r['att'], rej=r['rej'],
                    acc=r['acc'], err_rel=e, jvp=r['jvp'], lus=r['lus'], pcs=r['pcs'], lin_fail=r['lin_fail'])
        print(json.dumps(line), flush=True); out[f'{key}-{kind}'] = line
json.dump(out, open('fid_corpus.json', 'w'), indent=1)
