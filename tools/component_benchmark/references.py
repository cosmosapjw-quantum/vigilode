"""Analytic references and independent numerical refinement, never certificates."""
from __future__ import annotations
import math
from .models import rhs

ANALYTIC={'kepler','scalar_decay','logistic','pr_linear','pr_nonlinear','rotation','damping',
          'diagonal_stiff','jordan','nonnormal','periodic_heat','advection_diffusion',
          'sabr','mass_invertible','mass_ill_conditioned','dae_index1'}


def exact_state(case,t):
    import mpmath as mp
    mp.mp.dps=80
    p=case['parameters']; m=case['model']; q=mp.mpf; tt=q(t)
    if m in ('scalar_decay','sabr'): return [q(case['initial_state'][0])*mp.exp(q(p.get('rate',p.get('lambda')))*tt)]
    if m=='kepler':
        x0=q(case['initial_state'][0]); vy0=q(case['initial_state'][3]); mu=q(p['mu'])
        energy=vy0*vy0/2-mu/x0; a=-mu/(2*energy); e=1-x0/a
        n=mp.sqrt(mu/a**3); E=n*tt
        for _ in range(14): E-=(E-e*mp.sin(E)-n*tt)/(1-e*mp.cos(E))
        den=1-e*mp.cos(E); root=mp.sqrt(1-e*e)
        return [a*(mp.cos(E)-e),a*root*mp.sin(E),-a*n*mp.sin(E)/den,a*n*root*mp.cos(E)/den]
    if m=='logistic':
        r,K,y0=map(q,(p['rate'],p['capacity'],case['initial_state'][0]))
        return [K/(1+(K/y0-1)*mp.exp(-r*tt))]
    if m in ('pr_linear','pr_nonlinear'): return [mp.sin(tt)]
    if m=='rotation':
        w=q(p['omega']); x0,z0=map(q,case['initial_state']);return [x0*mp.cos(w*tt)-z0*mp.sin(w*tt),x0*mp.sin(w*tt)+z0*mp.cos(w*tt)]
    if m=='damping':
        w,z=q(p['omega']),q(p['zeta']);x0,v0=map(q,case['initial_state'])
        if z<1:
            d=w*mp.sqrt(1-z*z); a=mp.exp(-z*w*tt)
            x=a*(x0*mp.cos(d*tt)+(v0+z*w*x0)/d*mp.sin(d*tt))
            v=a*(-z*w*(x0*mp.cos(d*tt)+(v0+z*w*x0)/d*mp.sin(d*tt))-x0*d*mp.sin(d*tt)+(v0+z*w*x0)*mp.cos(d*tt))
        elif z==1:
            a=mp.exp(-w*tt); b=v0+w*x0; x=a*(x0+b*tt); v=a*(b-w*(x0+b*tt))
        else:
            d=w*mp.sqrt(z*z-1); a=mp.exp(-z*w*tt)
            x=a*(x0*mp.cosh(d*tt)+(v0+z*w*x0)/d*mp.sinh(d*tt))
            v=a*(-z*w*(x0*mp.cosh(d*tt)+(v0+z*w*x0)/d*mp.sinh(d*tt))+x0*d*mp.sinh(d*tt)+(v0+z*w*x0)*mp.cosh(d*tt))
        return [x,v]
    if m=='diagonal_stiff': return [q(y)*mp.exp(q(a)*tt) for y,a in zip(case['initial_state'],p['rates'])]
    if m=='jordan':
        e=mp.exp(q(p['lambda'])*tt);x0,y0=map(q,case['initial_state']);return [e*(x0+q(p['offdiag'])*tt*y0),e*y0]
    if m=='nonnormal':
        a,b,d=map(q,(p['a'],p['b'],p['d']));x0,y0=map(q,case['initial_state'])
        return [x0*mp.exp(a*tt)+b*y0*(mp.exp(d*tt)-mp.exp(a*tt))/(d-a),y0*mp.exp(d*tt)]
    if m in ('mass_invertible','mass_ill_conditioned'): return [q(v)*mp.exp(-tt) for v in case['initial_state']]
    if m=='dae_index1': return [mp.sin(tt),mp.sin(tt)+mp.cos(tt)]
    if m in ('periodic_heat','advection_diffusion'):
        n=p['cells']; D=q(p['diffusion']); vel=q(p.get('velocity',0))
        if tt==0: return [q(v) for v in case['initial_state']]
        angle=2*mp.pi/n
        real=D*n*n*(2*mp.cos(angle)-2)-vel*n*(1-mp.cos(angle))
        imag=-vel*n*mp.sin(angle)
        amp=q('0.1')*mp.exp(real*tt)
        return [1+amp*mp.sin(angle*i+imag*tt) for i in range(n)]
    raise NotImplementedError(m)


def analytic_reference(case):
    if case['model'] not in ANALYTIC: return {'status':'UNSUPPORTED_CAPABILITY','reason':'no analytic reference'}
    try:
        import mpmath as mp
        mp.mp.dps=80
        states=[[str(v) for v in exact_state(case,t)] for t in case['output_times']]
        # The stored periodic sine IC differs from the ideal sine mode by binary64
        # rounding. Positive diffusion and upwind advection give an l-infinity
        # contraction, so this initial projection error bounds every later time.
        projection=mp.mpf(0)
        if case['model'] in ('periodic_heat','advection_diffusion'):
            n=case['parameters']['cells']
            projection=max(abs(mp.mpf(v)-(1+mp.mpf('0.1')*mp.sin(2*mp.pi*i/n)))
                           for i,v in enumerate(case['initial_state']))
        errors=[]
        for row in states:
            errors.append([float(projection+2*abs(mp.mpf(v)-mp.mpf(float(v)))+4*mp.eps) for v in row])
        return {'status':'OK','times':case['output_times'],'states':[[float(v) for v in row] for row in states],
                'decimal_states':states,'uncertainty':errors,'uncertainty_abs':errors,'reference_converged':True,
                'verified_enclosure':False,'method':'mpmath_analytic_80dps',
                'initial_projection_bound':float(projection)}
    except (ImportError,OSError) as exc: return {'status':'ENVIRONMENT_FAILURE','reason':str(exc)}


def uncertainty_gate(reference,case,fraction=.05):
    if reference.get('status')!='OK': return False
    vals=reference['states']; errs=reference.get('uncertainty',reference.get('uncertainty_abs'))
    if errs is None or len(vals)!=len(errs): return False
    return all(math.isfinite(e) and e<=fraction*(a+case['rtol']*abs(v)) for row,erow in zip(vals,errs) for v,e,a in zip(row,erow,case['atol']))


def numerical_reference(case):
    if case['mass_matrix'] is not None:
        return {'status':'UNSUPPORTED_CAPABILITY','reason':'independent DAE reference unavailable'}
    levels=[]
    try:
        import numpy as np
        from scipy.integrate import solve_ivp
        from .models import jac
        grid=case['output_times'];span=case['t_span'];y0=case['initial_state']
        atol=[min(a*1e-4,1e-15) for a in case['atol']]
        arrays=[]
        for method,rtol in [('Radau',1e-10),('Radau',1e-11),('Radau',1e-12),('BDF',1e-12)]:
            sol=solve_ivp(lambda t,y:rhs(case,t,y),span,y0,method=method,t_eval=grid,
                          rtol=rtol,atol=atol,jac=lambda t,y:jac(case,t,y))
            state_array=np.asarray(sol.y.T,dtype=float)
            finite=bool(np.all(np.isfinite(state_array)))
            raw_states=[[float(v) if np.isfinite(v) else str(v) for v in row] for row in state_array]
            row={'method':method,'rtol':rtol,'atol':atol,'success':bool(sol.success),
                 'finite':finite,'message':str(sol.message),'nfev':int(sol.nfev),
                 'njev':int(sol.njev) if sol.njev is not None else None,
                 'nlu':int(sol.nlu) if sol.nlu is not None else None,
                 'times':np.asarray(sol.t,dtype=float).tolist(),'states':raw_states}
            levels.append(row)
            if not sol.success or len(sol.t)!=len(grid) or not finite:
                return {'status':'NUMERICAL_FAILURE',
                        'reason':method+': '+('nonfinite reference state' if not finite else sol.message),
                        'reference_converged':False,'levels':levels}
            arrays.append(state_array)
        coarse,medium,fine,independent=arrays
        budget=np.asarray(case['atol'])[None,:]+case['rtol']*np.abs(fine)
        eps=np.finfo(float).eps
        noise=8*eps*np.maximum(1,np.abs(fine))
        d_coarse=float(np.max(np.abs(medium-coarse)/budget))
        d_fine=float(np.max(np.abs(fine-medium)/budget))
        n=float(np.max(noise/budget))
        uncertainty=2*np.maximum(np.abs(fine-medium),np.abs(fine-independent))+noise
        u=float(np.max(uncertainty/budget))
        converged=bool(np.all(np.isfinite(fine)) and d_fine<=.5*d_coarse+n and u<=.05)
        return {'status':'OK','times':grid,'states':fine.tolist(),
                'uncertainty':uncertainty.tolist(),'uncertainty_abs':uncertainty.tolist(),
                'reference_converged':converged,'verified_enclosure':False,
                'method':'Radau 1e-10/1e-11/1e-12 plus BDF 1e-12',
                'levels':levels,'refinement':{'Dcoarse':d_coarse,'Dfine':d_fine,'N':n,
                              'uncertainty_budget_ratio':u,'contraction_gate':d_fine<=.5*d_coarse+n,
                              'uncertainty_gate':u<=.05}}
    except (ImportError,OSError) as exc:
        return {'status':'ENVIRONMENT_FAILURE','reason':str(exc),'levels':levels}
    except Exception as exc:
        return {'status':'NUMERICAL_FAILURE','reason':str(exc),'reference_converged':False,'levels':levels}


def build_reference(case):
    if case['reference_policy']=='expected_failure': return {'status':'UNSUPPORTED_CAPABILITY','reason':'inconsistent initial constraint is an expected failure control'}
    if case['model'] in ANALYTIC: return analytic_reference(case)
    return numerical_reference(case)
