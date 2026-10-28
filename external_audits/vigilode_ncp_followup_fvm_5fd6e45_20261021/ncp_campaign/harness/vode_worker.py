"""SciPy VODE (DVODE) heat worker: 2 warmups + N measured in-process solves.

MF20: archived arm (BDF, with_jacobian=False, no jac -> functional iteration).
MF21: BDF chord/Newton with user-supplied analytic dense Jacobian.
MF22: BDF chord/Newton with VODE-internal finite-difference dense Jacobian.
Usage: python3 vode_worker.py MF REQUEST WARMUPS MEASUREMENTS
"""
import json
import sys
import time

import numpy as np
import scipy
from scipy.integrate import ode

MF = int(sys.argv[1])
case = json.load(open(sys.argv[2]))['case']
WARM, NMEAS = int(sys.argv[3]), int(sys.argv[4])
assert case['model'] == 'periodic_heat' and MF in (20, 21, 22)
y0 = np.array(case['initial_state'], dtype=float)
n = len(y0)
D = case['parameters']['diffusion'] * n * n
J = np.zeros((n, n))
for i in range(n):
    J[i, i] -= 2 * D
    J[i, (i - 1) % n] += D
    J[i, (i + 1) % n] += D
nfev = {'rhs': 0, 'jac': 0}


def f(t, y):
    nfev['rhs'] += 1
    # same operation order as the archived list RHS: D*((y[i-1]-2*y[i])+y[i+1])
    return D * (np.roll(y, 1) - 2 * y + np.roll(y, -1))


def jac(t, y):
    nfev['jac'] += 1
    return J


def solve_once():
    nfev['rhs'] = nfev['jac'] = 0
    t0 = time.perf_counter_ns()
    r = ode(f, jac if MF == 21 else None)
    r.set_integrator('vode', method='bdf', with_jacobian=(MF in (21, 22)), rtol=case['rtol'],
                     atol=case['atol'][0], nsteps=100000)
    r.set_initial_value(y0, case['t_span'][0])
    times, states, ok = [case['t_span'][0]], [y0.tolist()], True
    for ti in case['output_times'][1:]:
        yy = r.integrate(ti)
        if not r.successful():
            ok = False
            break
        times.append(float(r.t))
        states.append(yy.tolist())
    el = time.perf_counter_ns() - t0
    integ = r._integrator
    iw = np.asarray(integ.iwork)
    mf_used = int(integ.call_args[-1])
    counters = {'NST': int(iw[10]), 'NFE': int(iw[11]), 'NJE': int(iw[12]), 'NLU': int(iw[18]),
                'NNI': int(iw[19]), 'NCFN': int(iw[20]), 'NETF': int(iw[21]),
                'python_rhs_calls': nfev['rhs'], 'python_jac_calls': nfev['jac']}
    return {'elapsed_ns': el, 'ok': ok and r.successful(), 'istate': int(getattr(integ, 'istate', 0)),
            'times': times, 'states': states, 'counters': counters, 'mf_used': mf_used}


warm = [solve_once() for _ in range(WARM)]
meas = []
for i in range(NMEAS):
    m = solve_once()
    m['repetition'] = i + 1
    meas.append(m)
print(json.dumps({'status': 'OK' if all(m['ok'] for m in meas) else 'NUMERICAL_FAILURE',
                  'solver': 'scipy.integrate.ode vode (DVODE)', 'scipy_version': scipy.__version__,
                  'numpy_version': np.__version__, 'mf_requested': MF,
                  'mf_used': sorted({m['mf_used'] for m in warm + meas}),
                  'internal_rtol': case['rtol'], 'internal_atol_scalar': case['atol'][0],
                  'timing_scope': 'in-process ode construction + integrate over output grid',
                  'warmup_elapsed_ns': [w['elapsed_ns'] for w in warm], 'measurements': meas},
                 allow_nan=False))
