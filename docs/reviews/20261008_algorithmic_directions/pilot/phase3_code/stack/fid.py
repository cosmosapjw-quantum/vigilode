"""Replica fidelity of the B1 stack replica (arm 0, base) against the SPD07 BASE.json Rust counters (gmres_into_zero),
all 14 cells. Also checks that arm 1 (no duplicate) equals arm 0 minus the diagnostic counters on two cells."""
import json, struct, sys, time, numpy as np
sys.path.insert(0, '.')
import stack, tprobs
B = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
h2f = lambda s: struct.unpack('>d', bytes.fromhex(s))[0]
pm = {'robertson': tprobs.robertson, 'van-der-pol-mu1000': tprobs.vdp, 'hires': tprobs.hires,
      'brusselator-1d-50': lambda: tprobs.bruss(50), 'prothero-robinson-forced': tprobs.pr_forced,
      'quadratic-4': tprobs.quad4, 'brusselator-1d-160': lambda: tprobs.bruss(160)}
out = []
for row in B['rows']:
    p = pm[row['case']](); z = row['gmres_into_zero']; c = z['counters']
    t0 = time.time()
    r = stack.integrate(p, row['rtol'], stack.make_arm('base'))
    yr = np.array([h2f(x) for x in z['y_last']])
    dev = float(np.max(np.abs(r['y'] - yr))/np.max(np.abs(yr)))
    rust = dict(att=z['attempts'], acc=z['accepted'], rej=z['rejected'], jvp=c['jvp_vectors'], iters=c['linear_iterations'],
                matvecs=c['linear_matvecs'], dots=c['orthogonalization_inner_products'],
                oaxpys=c.get('orthogonalization_vector_updates'), rhs=c['rhs_evaluations'], lin_fail=c.get('linear_solve_failures'))
    py = dict(att=r['att'], acc=r['acc'], rej=r['rej'], jvp=r['jvp'], iters=r['cols'], matvecs=r['cols'] + r['tres'], dots=r['dots'],
              oaxpys=r['oaxpys'], rhs=r['rhs'], lin_fail=r['lin_fail'])
    o = dict(case=row['case'], rtol=row['rtol'], rust=rust, py=py, y_dev=dev, sec=time.time() - t0)
    out.append(o)
    print(f"{row['case']:26s} {row['rtol']:g} " + ' '.join(f"{k} {rust[k]}/{py[k]}" for k in rust) + f" |dy|={dev:.1e} [{time.time()-t0:.0f}s]", flush=True)
json.dump(out, open('res/fidelity.json', 'w'), indent=1)
# arm 1 check
for name, rt in (('hires', 1e-6), ('bruss50', 1e-6)):
    p = tprobs.PROBLEMS[name]()
    r0 = stack.integrate(p, rt, stack.make_arm('base'))
    r1 = stack.integrate(p, rt, stack.make_arm('base', dup=False))
    same = (r0['att'], r0['acc'], r0['rej']) == (r1['att'], r1['acc'], r1['rej']) and np.array_equal(r0['y'], r1['y'])
    print(name, rt, 'arm1 identical trajectory:', same, 'jvp', r0['jvp'], r1['jvp'], 'diff', r0['jvp'] - r1['jvp'], 'solves', r0['solves'],
          'jvp per attempt saved', (r0['jvp'] - r1['jvp'])/r0['att'])
