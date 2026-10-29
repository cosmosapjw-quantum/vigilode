#!/usr/bin/env python3
"""One process per SciPy arm. Raw integration status is independent of accuracy gates."""
from __future__ import annotations
import argparse
import json
import math
import sys
import time
from pathlib import Path

if __package__ in (None,''):
    sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from tools.component_benchmark.models import rhs,jac,jvp
from tools.component_benchmark.protocol import strict_load,validate_case

METHODS={'scipy-rk45':'RK45','scipy-dop853':'DOP853','scipy-radau':'Radau',
         'scipy-bdf':'BDF','scipy-lsoda':'LSODA',
         'vode-adams':'vode-adams','vode-bdf':'vode-bdf',
         'scipy-vode-adams':'vode-adams','scipy-vode-bdf':'vode-bdf'}


def _blank(status,reason=None):
    d={'status':status,'times':[],'states':[],'counters':None,
       'timing':{'solve_seconds':None,'total_seconds':None,'timing_authority':'diagnostic_only'},
       'allocations':None}
    if reason is not None: d['reason']=str(reason)
    return d


def solve(request):
    case=request.get('case'); sid=request.get('solver_id')
    if not isinstance(case,dict) or validate_case(case): return _blank('ENVIRONMENT_FAILURE','invalid CaseSpec')
    method=METHODS.get(sid)
    if method is None: return _blank('UNSUPPORTED_CAPABILITY','unknown SciPy solver_id')
    mass=case['mass_matrix']
    if method in ('Radau','BDF','LSODA','vode-bdf') and len(case['initial_state'])>512:
        return _blank('UNSUPPORTED_CAPABILITY','dense implicit Jacobian exceeds screening memory policy')
    if mass is not None:
        # Only the two invertible mass fixtures permit explicit M inverse.
        if case['model'] not in ('mass_invertible','mass_ill_conditioned'):
            return _blank('UNSUPPORTED_CAPABILITY','singular mass DAE unsupported by this SciPy arm')
    total_start=time.perf_counter()
    try:
        import numpy as np
        from scipy.integrate import solve_ivp,ode
        if mass is None:
            fun=lambda t,y: rhs(case,t,y)
            jf=lambda t,y: jac(case,t,y)
        else:
            matrix=np.asarray(mass,dtype=float)
            fun=lambda t,y: np.linalg.solve(matrix,np.asarray(rhs(case,t,y),dtype=float))
            jf=lambda t,y: np.linalg.solve(matrix,np.asarray(jac(case,t,y),dtype=float))
        t0,t1=case['t_span']; grid=case['output_times']
        def one_run():
            run_start=time.perf_counter()
            if method.startswith('vode'):
                integrator=ode(fun,jf).set_integrator('vode',method=method.split('-')[1],rtol=case['rtol'],atol=case['atol'])
                integrator.set_initial_value(case['initial_state'],t0)
                solve_start=time.perf_counter()
                current=[list(case['initial_state'])]
                for target in grid[1:]:
                    value=integrator.integrate(target)
                    if not integrator.successful() or not np.all(np.isfinite(value)):
                        raise ArithmeticError(f'VODE stopped at {integrator.t}')
                    current.append(np.asarray(value,dtype=float).tolist())
                solve_elapsed=time.perf_counter()-solve_start
                work={'rhs_evaluations':None,'jacobian_evaluations':None,'lu_factorizations':None,'accepted_steps':None,'rejected_steps':None}
            else:
                opts={'method':method,'t_eval':grid,'rtol':case['rtol'],'atol':case['atol']}
                if method in ('Radau','BDF','LSODA'): opts['jac']=jf
                solve_start=time.perf_counter()
                result=solve_ivp(fun,(t0,t1),case['initial_state'],**opts)
                solve_elapsed=time.perf_counter()-solve_start
                if not result.success or len(result.t)!=len(grid):
                    raise ArithmeticError(result.message)
                current=np.asarray(result.y.T,dtype=float).tolist()
                work={'rhs_evaluations':int(result.nfev),'jacobian_evaluations':int(result.njev) if result.njev is not None else None,
                      'lu_factorizations':int(result.nlu) if result.nlu is not None else None,
                      'accepted_steps':None,'rejected_steps':None}
            if not all(math.isfinite(x) for row in current for x in row):
                raise ArithmeticError('nonfinite state')
            return current,work,solve_elapsed,time.perf_counter()-run_start
        _,_,_,warmup=one_run()
        states,counters,elapsed,timed=one_run()
        if not all(math.isfinite(x) for row in states for x in row): return _blank('NUMERICAL_FAILURE','nonfinite state')
        return {'status':'OK','times':grid,'states':states,'counters':counters,
                'timing':{'solve_seconds':max(elapsed,sys.float_info.min),
                          'total_seconds':max(time.perf_counter()-total_start,sys.float_info.min),
                          'warmup_seconds':max(warmup,sys.float_info.min),
                          'timed_seconds':max(timed,sys.float_info.min),
                          'timing_authority':'diagnostic_only'},
                'allocations':None,'timing_scope':'warmed_solver_setup_integration_output_certificate',
                'measurement_mode':'timing','solver_id':sid,'mass_handling':'explicit_inverse' if mass is not None else 'none'}
    except (ImportError,OSError) as exc: return _blank('ENVIRONMENT_FAILURE',exc)
    except ArithmeticError as exc: return _blank('NUMERICAL_FAILURE',exc)
    except Exception as exc: return _blank('NUMERICAL_FAILURE',f'{type(exc).__name__}: {exc}')


def probe(request):
    case=request.get('case')
    if not isinstance(case,dict) or validate_case(case):
        return {'status':'ENVIRONMENT_FAILURE','reason':'invalid CaseSpec'}
    t0,t1=case['t_span'];y0=case['initial_state'];n=len(y0)
    if 'probes' in request:
        raw=request['probes']
        if not isinstance(raw,list) or not raw:
            return {'status':'ENVIRONMENT_FAILURE','reason':'invalid frozen probes'}
        points=[]
        for item in raw:
            if not isinstance(item,dict): return {'status':'ENVIRONMENT_FAILURE','reason':'invalid frozen probe'}
            t=item.get('time');y=item.get('state');v=item.get('direction')
            if not isinstance(t,(int,float)) or not math.isfinite(t) or not t0<=t<=t1 or not isinstance(y,list) or not isinstance(v,list) or len(y)!=n or len(v)!=n or any(not isinstance(z,(int,float)) or not math.isfinite(z) for z in y+v):
                return {'status':'ENVIRONMENT_FAILURE','reason':'invalid frozen probe values'}
            points.append((t,y,v))
    else:
        direction=[.5+.01*i for i in range(n)]
        points=[(t0,y0,direction),(t0+.37*(t1-t0),[v+.01*(i+1) for i,v in enumerate(y0)],direction)]
    try:
        rows=[]
        for t,y,direction in points:
            if n>512:
                indices=[0,n//2,n-1]
                columns=[]
                for index in indices:
                    basis=[0.0]*n;basis[index]=1.0
                    columns.append(jvp(case,t,y,basis))
                matrix=None
            else:
                indices=None;columns=None;matrix=jac(case,t,y)
            rows.append({'time':t,'state':y,'direction':direction,'rhs':rhs(case,t,y),
                         'jacobian':matrix,'jacobian_column_indices':indices,
                         'jacobian_columns':columns,'jvp':jvp(case,t,y,direction)})
        return {'status':'OK','case_id':case['case_id'],'probes':rows,
                'derivative_mode':'analytic_model_jacobian',
                'rhs_semantics':'original_f_with_algebraic_rows_for_DAE'}
    except Exception as exc:
        return {'status':'UNSUPPORTED_CAPABILITY','case_id':case['case_id'],'reason':str(exc)}


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--request',type=Path,required=True); ap.add_argument('--probe',action='store_true'); args=ap.parse_args()
    try:
        request=strict_load(args.request.read_text())
        result=probe(request) if args.probe else solve(request)
    except Exception as exc: result=_blank('ENVIRONMENT_FAILURE',f'{type(exc).__name__}: {exc}')
    print(json.dumps(result,allow_nan=False,separators=(',',':')))

if __name__=='__main__': main()
