#!/usr/bin/env python3
"""SciPy's double-precision VODE wrapper, with real Python callback costs."""
import argparse
import json
import math
from pathlib import Path
import time
from campaign import canonical, digest
from external_models import rhs, exact, verify

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--contract',required=True,type=Path)
    p.add_argument('--method',choices=['vode-adams','vode-bdf'],required=True)
    p.add_argument('--rtol',type=float,default=1e-6)
    p.add_argument('--atol',type=float,default=1e-9)
    p.add_argument('--max-steps',type=int,default=200000)
    p.add_argument('--error-limit',type=float,default=1e-4)
    a=p.parse_args()
    if not all(math.isfinite(x) and x>0 for x in [a.rtol,a.atol,a.error_limit]) or a.max_steps<1:
        raise ValueError('invalid numerical limits')
    c=json.loads(a.contract.read_text()); matched=verify(c)
    import scipy
    from scipy.integrate import ode
    calls=0
    def f(t,y):
        nonlocal calls
        calls+=1
        return rhs(c,t,y)
    start=time.perf_counter()
    solver=ode(f).set_integrator('vode',method=a.method.split('-')[1],rtol=a.rtol,atol=a.atol,nsteps=a.max_steps)
    solver.set_initial_value(c['initial_state'],c['t_start'])
    out=solver.integrate(c['t_end'])
    seconds=time.perf_counter()-start
    state=[float(x) for x in out]
    ok=bool(solver.successful()) and abs(solver.t-c['t_end']) <= 16*math.ulp(c['t_end']) and all(math.isfinite(x) for x in state)
    reference=exact(c,c['t_end']); error=None
    if reference is not None and ok:
        error=max(abs(x-y) for x,y in zip(state,reference))
    status='SOLVER_FAILED' if not ok else 'UNVALIDATED_REFERENCE' if error is None else 'PASS_SCOPED' if error<=a.error_limit else 'ACCURACY_FAILED'
    print(canonical({'kind':'result','schema_version':1,'case':c['case_id'],'method':a.method,
      'parallel_layer':'serial','threads':1,'size':c['size'],'elapsed_seconds':seconds,
      'scientific_status':status,'speed_comparison_eligible':status=='PASS_SCOPED',
      'actual_error':error,'error_limit':a.error_limit,'solver_success':ok,
      'reference_kind':'analytic endpoint' if reference is not None else 'none',
      'final_state':state if all(math.isfinite(x) for x in state) else None,'final_time':float(solver.t),
      'counters':{'rhs':calls},'model_match':matched,'contract_sha256':digest(a.contract),
      'backend':'SciPy scipy.integrate.ode VODE (double-precision ODEPACK implementation)',
      'scipy_version':scipy.__version__,'vode_return_code':int(solver.get_return_code()),
      'timing_scope':'fresh problem/workspace + integration; Python callback cost included; imports/probe checks/reference/JSON excluded',
      'accuracy_scope':'maximum component absolute endpoint error; no continuous-time or missed-cycle guarantee'}))
if __name__=='__main__':main()
