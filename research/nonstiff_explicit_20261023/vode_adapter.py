#!/usr/bin/env python3
"""Optional SciPy wrapper of the permitted Fortran VODE/DVODE comparator.

This is not scipy RK45. Python callback overhead is included; imports and
reference/JSON generation are excluded. No dependencies are installed here.
"""
import json, sys, time, platform, math
import numpy as np
import scipy
from scipy.integrate import ode

def main():
    req=json.load(sys.stdin);name=req['case'];n=req['dimension'];rtol=req['rtol'];atol=req['atol']
    end={'decay':4.,'driven':4.,'logistic':10.,'harmonic':12.,'rapid':2.,'damped':4.,
         'mixed-scales':4.,'nonnormal':4.,'large-vector':4.,'moderate-stiff':2.,'stiff-control':1.}[name]
    def initial():
        if name=='logistic':return np.array([.05+.01*(i%8) for i in range(n)])
        if name=='mixed-scales':return np.array([10.**(i%12-8) for i in range(n)])
        if name=='nonnormal':return np.array([float(i%2) for i in range(n)])
        if name in ('harmonic','rapid','damped'):return np.array([1.-float(i%2) for i in range(n)])
        return np.array([1./(1+i%8) for i in range(n)])
    def rates():
        if name=='logistic':return np.array([.25+.1*(i%8) for i in range(n)])
        if name=='large-vector':return np.array([.5+.05*(i%8) for i in range(n)])
        if name=='moderate-stiff':return np.array([[1.,30.,300.][i%3] for i in range(n)])
        if name=='stiff-control':return np.array([[1.,1000.,100000.][i%3] for i in range(n)])
        return np.array([[.25,1.,3.,.5][i%4] for i in range(n)])
    def frequencies():
        base,scale=(100.,5.) if name=='rapid' else (10.,1.) if name=='damped' else (1.,.25)
        return np.array([base+scale*(i%4) for i in range(n//2)])
    counts={'rhs_attempted':0,'rhs_completed':0}
    start=time.perf_counter();y0=initial();a=rates();w=frequencies();times=np.linspace(0,end,5)
    def rhs(t,y):
        counts['rhs_attempted']+=1
        if name in ('harmonic','rapid','damped'):
            dy=np.empty(n);d=3. if name=='damped' else 0.
            dy[0::2]=-d*y[0::2]-w*y[1::2];dy[1::2]=w*y[0::2]-d*y[1::2]
        elif name=='nonnormal':
            dy=np.empty(n);dy[0::2]=-y[0::2]+30*y[1::2];dy[1::2]=-2*y[1::2]
        elif name=='logistic':dy=a*y*(1-y)
        elif name=='driven':dy=-a*y+np.cos(t)+a*np.sin(t)
        else:dy=-a*y
        counts['rhs_completed']+=1;return dy
    method='bdf' if name in ('stiff-control','moderate-stiff') else 'adams'
    solver=ode(rhs).set_integrator('vode',method=method,rtol=rtol,atol=atol,nsteps=req.get('max_steps',100000),
                                    first_step=1e-3,with_jacobian=False)
    solver.set_initial_value(y0,0.)
    actual_times=[0.];states=[y0.copy()];success=True;message='success'
    try:
        for target in times[1:]:
            y=solver.integrate(float(target));actual_times.append(float(solver.t));states.append(y.copy())
            if not solver.successful() or not np.isfinite(y).all():
                success=False;message='VODE unsuccessful/nonfinite return';break
    except Exception as error:success=False;message=repr(error)
    elapsed=time.perf_counter()-start
    def exact(t):
        if name=='logistic':return 1/(1+(1/y0-1)*np.exp(-a*t))
        if name=='driven':return np.sin(t)+y0*np.exp(-a*t)
        if name in ('harmonic','rapid','damped'):
            y=np.empty(n);envelope=np.exp(-3*t) if name=='damped' else 1.
            y[0::2]=envelope*np.cos(w*t);y[1::2]=envelope*np.sin(w*t);return y
        if name=='nonnormal':
            y=np.empty(n);y[0::2]=30*(np.exp(-t)-np.exp(-2*t));y[1::2]=np.exp(-2*t);return y
        return y0*np.exp(-a*t)
    reference=np.array([exact(t) for t in actual_times]);states=np.array(states)
    valid=np.isfinite(states).all();error=np.abs(states-reference)
    scale=np.maximum(1.,np.maximum(np.abs(y0)[None,:],np.abs(reference)))
    record={**req,'schema':'vigilode.nonstiff-benchmark-run.v1','mode':'vode','end':end,
            'success':bool(success and valid and len(states)==5 and actual_times[-1]==end),
            'message':message,'elapsed_seconds':elapsed,'max_abs_error':float(error.max()) if valid else None,
            'max_normalized_error':float((error/scale).max()) if valid else None,
            'max_reference_scaled_wrms':float(np.sqrt(np.mean((error/(atol+rtol*np.abs(reference)))**2,axis=1)).max()) if valid else None,
            'requested_times':times.tolist(),'times':actual_times,'states':states.tolist(),'reference':reference.tolist(),
            'work':counts,'adapter':{'solver':'VODE/DVODE','method':method,'with_jacobian':False,
                                  'numpy_version':np.__version__,'scipy_version':scipy.__version__,'python':platform.python_version(),
                                  'callback_language':'Python/NumPy','output_policy':'VODE target-time output may interpolate an internal step'},
            'timing_scope':'problem/workspace construction and integration including Python callbacks; imports, reference and JSON excluded'}
    def finite_json(value):
        if isinstance(value,float) and not math.isfinite(value):return str(value)
        if isinstance(value,list):return [finite_json(x) for x in value]
        if isinstance(value,dict):return {k:finite_json(v) for k,v in value.items()}
        return value
    print(json.dumps(finite_json(record),allow_nan=False))

if __name__=='__main__':main()
