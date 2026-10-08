"""Replica fidelity: base arm vs SPD07 BASE.json gmres_into_zero counters (Rust), all 14 cells."""
import json, struct, sys, time, numpy as np
sys.path.insert(0, '.')
import rep, tprobs
B = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
h2f = lambda s: struct.unpack('>d', bytes.fromhex(s))[0]
pm = {'robertson': tprobs.robertson, 'van-der-pol-mu1000': tprobs.vdp, 'hires': tprobs.hires,
      'brusselator-1d-50': lambda: tprobs.bruss(50), 'prothero-robinson-forced': tprobs.pr_forced,
      'quadratic-4': tprobs.quad4, 'brusselator-1d-160': lambda: tprobs.bruss(160)}
ortho = sys.argv[1] if len(sys.argv) > 1 else 'mgs2'
skip160 = len(sys.argv) > 2
out = []
for row in B['rows']:
    if skip160 and row['case'] == 'brusselator-1d-160': continue
    p = pm[row['case']](); z = row['gmres_into_zero']; c = z['counters']
    t0 = time.time()
    r = rep.integrate(p, row['rtol'], rep.make_arm('base', ortho=ortho))
    yr = np.array([h2f(x) for x in z['y_last']])
    dev = float(np.max(np.abs(r['y'] - yr))/np.max(np.abs(yr)))
    o = dict(case=row['case'], rtol=row['rtol'], rust=dict(att=z['attempts'], acc=z['accepted'], rej=z['rejected'], jvp=c['jvp_vectors'],
             iters=c['linear_iterations'], matvecs=c['linear_matvecs'], dots=c['orthogonalization_inner_products'], rhs=c['rhs_evaluations']),
             py=dict(att=r['att'], acc=r['acc'], rej=r['rej'], jvp=r['jvp'], iters=r['cols'], matvecs=r['cols']+r['tres'], dots=r['dots'], rhs=r['rhs']),
             y_dev=dev)
    out.append(o)
    print(f"{row['case']:26s} {row['rtol']:g} att {z['attempts']}/{r['att']} rej {z['rejected']}/{r['rej']} jvp {c['jvp_vectors']}/{r['jvp']} ({r['jvp']/c['jvp_vectors']:.4f}) "
          f"iters {c['linear_iterations']}/{r['cols']} dots {c['orthogonalization_inner_products']}/{r['dots']} rhs {c['rhs_evaluations']}/{r['rhs']} |dy|={dev:.1e} [{time.time()-t0:.0f}s]", flush=True)
json.dump(out, open(f'fid_{ortho}.json', 'w'), indent=1)
