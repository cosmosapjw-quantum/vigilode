#!/usr/bin/env python3
"""VigilODE standalone research: source-owned nonlinear RHS-only BE homotopy.

All certified inequalities use Python Fraction of represented binary64 source
coefficients and returned candidate bits. The predictor is intentionally f64
and cannot affect certificate soundness. The analytic solution is checked at
80-digit mpmath precision as a logically independent *numerical* cross-check.
No JVP, Jacobian, AD, Newton correction, or ODE library is used in this solver.
This does NOT assert derivative-free speed relative to actual production code.
"""
from __future__ import annotations
from dataclasses import dataclass
from fractions import Fraction as Q
import json
import math
import time
import mpmath as mp

mp.mp.dps = 80

def f(x): return Q.from_float(float(x))

def m(x): return mp.mpf(x.numerator) / mp.mpf(x.denominator)

@dataclass
class Plan:
    k: Q
    g: Q
    Y: Q
    T: Q
    h: Q
    a: Q
    b: Q
    D: list[Q]
    clock: list[Q]
    trunc: list[Q]
    residual_budget: Q
    epsilon: Q
    max_trunc_clock: Q

@dataclass
class Value:
    x: float
    mapped: float
    op_res: float
    R: Q


def plan(k,g,Y,T,N,epsilon):
    assert N>0 and N & (N-1)==0 and N <= 2**52
    assert all(math.isfinite(x) for x in (k,g,Y,T,epsilon))
    assert k>0 and g>=0 and Y>0 and T>0 and epsilon>0
    h_float = T / N
    assert h_float * N == T
    k,g,Y,T,h,epsilon=map(f,(k,g,Y,T,h_float,epsilon))
    assert h*N == T
    a=1+h*k;b=h*g
    M=Y;tr=Q(0);D=[];clock=[Q(0)];trunc=[Q(0)]
    P0=k*Y+g*Y**3
    maxTC=Q(0)
    for j in range(N):
        L=k+3*g*M*M;P=k*M+g*M**3
        dj=h*h*L*P/2
        tr=(tr+dj)/a
        label=f(float(h*(j+1)))
        t_exact=(j+1)*h
        displ=abs(label-t_exact)*P0
        maxTC=max(maxTC,tr+displ)
        D.append(dj);clock.append(displ);trunc.append(tr)
        M/=a
    if maxTC>=epsilon:raise ValueError(f'TRUNCATION_GATE max={float(maxTC):.9e}, requested={float(epsilon):.9e}')
    # Sum_{j=1}^i Rmax/a^j <= Rmax/(a-1); conservative for all i.
    rmax=(epsilon-maxTC)*(a-1)
    assert rmax>0
    return Plan(k,g,Y,T,h,a,b,D,clock,trunc,rmax,epsilon,maxTC)


def stage(p:Plan, source_y:float, fast:bool, cap=200):
    y=f(source_y)
    assert y>=0
    U_exact=y/p.a
    # upper round outward: used only to establish invariant domain
    U_float=float(U_exact)
    U_bound=U_float if f(U_float)>=U_exact else math.nextafter(U_float, math.inf)
    U=f(U_bound)
    q=3*p.b*U*U/p.a
    if q>=1: raise ValueError(f'SOURCE_NONCONTRACTION q={float(q)}')
    # exact source invariance of [0,U]: G(0)<=U, G(U)>=0
    if y/p.a > U or y/p.a - (p.b/p.a)*U**3 < 0:
        raise ValueError('SOURCE_INVARIANT_DOMAIN_FAILURE')
    # The transformed source map isolates stiff linear k algebraically.
    u_float=float(y/p.a); d_float=float(p.b/p.a)
    work={'rhs_attempted':0,'rhs_completed':0,'source_residual_evaluations':0,
          'speculative_trials':0,'accepted_acceleration':0,'fallbacks':0,
          'source_q':float(q)}
    def evaluate(x):
        if work['rhs_attempted']>=cap:raise ValueError('STAGE_WORK_CAP')
        if not (math.isfinite(x) and 0 <= x <= U_bound):
            raise ValueError('SOURCE_DOMAIN_REFUSAL')
        work['rhs_attempted']+=1
        mapped=u_float-d_float*x*x*x
        if not math.isfinite(mapped):raise ValueError('CALLBACK_OVERFLOW')
        # Source callback candidate clamped to its rigorously checked invariant domain.
        # This never replaces the original residual certificate.
        mapped=max(0.,min(U_bound,mapped))
        rootR=p.a*f(x)+p.b*f(x)**3-y
        work['source_residual_evaluations']+=1
        work['rhs_completed']+=1
        return Value(x,mapped,mapped-x,rootR)

    current=evaluate(0.0)
    old=None
    for iteration in range(cap):
        if abs(current.R)<=p.residual_budget:
            work['last_residual']=float(abs(current.R));work['iterations']=iteration
            return current.x,current.R,work
        selected=None
        baseline=None
        if not fast:
            baseline=evaluate(current.mapped)
            if abs(baseline.R)<=p.residual_budget:
                selected=baseline
        if selected is None and old is not None:
            dr=current.op_res-old.op_res
            if dr!=0 and math.isfinite(dr):
                gamma=current.op_res/dr
                proposal=current.mapped-gamma*(current.mapped-old.mapped)
                if math.isfinite(proposal) and 0<=proposal<=U_bound:
                    work['speculative_trials']+=1
                    accel=evaluate(proposal)
                    if fast:
                        if abs(accel.R)<=p.residual_budget or abs(accel.R)<=q*abs(current.R):
                            selected=accel;work['accepted_acceleration']+=1
                    else:
                        if abs(accel.R)<=abs(baseline.R):
                            selected=accel;work['accepted_acceleration']+=1
        if selected is None:
            if baseline is None:
                work['fallbacks']+=1
                baseline=evaluate(current.mapped)
            selected=baseline
        old,current=current,selected
    raise ValueError('STAGE_ITERATION_LIMIT')


def mp_exact(p:Plan, t:Q):
    k,g,Y=m(p.k),m(p.g),m(p.Y)
    e=mp.exp(-k*m(t))
    return Y*e/mp.sqrt(1+(g/k)*Y**2*(1-e*e))


def full_ivp(k,g,Y,T,N,eps,fast):
    p=plan(k,g,Y,T,N,eps)
    y=float(Y)
    E=Q(0)
    data=[]
    work={'rhs_attempted':0,'rhs_completed':0,'source_residual_evaluations':0,
          'speculative_trials':0,'accepted_acceleration':0,'fallbacks':0}
    for j in range(N):
        try:y_new,r,st=stage(p,y,fast)
        except Exception as e:raise RuntimeError(f'step={j}, last_state={y}, {e}') from e
        assert 0<=y_new<=y+1e-15
        E=(E+p.D[j]+abs(r))/p.a
        certified=E+p.clock[j+1]
        if certified>p.epsilon:raise AssertionError(f'PHYSICAL_ERROR_BOUND_FAIL step={j} {float(certified):.17e}')
        label=float(p.h*(j+1))
        error=abs(m(f(y_new))-mp_exact(p,f(label)))
        if error>m(certified):raise AssertionError(f'ANALYTIC_ORACLE_OUTSIDE_CERT step={j}: {error} > {m(certified)}')
        data.append({'step':j+1,'t_bits':label.hex(),'state_bits':y_new.hex(),
                     'actual_error_numeric':float(error),'exact_residual':float(abs(r)),
                     'certified_physical_error_upper':float(certified),'q':st['source_q'],
                     'rhs':st['rhs_attempted'],'accelerated':st['accepted_acceleration']})
        for key in work:work[key]+=st[key]
        y=y_new
    return p,data,work

COHORTS=[('stiff_fast',100.,1e6,.01,.1,64,.0015),
         ('relaxation',10.,1e5,.01,.3,32,.0009),
         ('scale_covariant',1000.,1e9,.001,.01,64,.00015),
         ('noncontractive_negative',100.,1e7,.01,.1,64,.01)]

def main():
    results=[]
    for label,k,g,Y,T,N,eps in COHORTS:
        arms=[]
        for fast in (False,True):
            try:
                start=time.perf_counter()
                p,points,work=full_ivp(k,g,Y,T,N,eps,fast)
                sec=time.perf_counter()-start
                arm={'fast_first':fast,'status':'PASS','total_runtime_python_seconds':sec,'work':work,
                     'max_physical_upper':max(z['certified_physical_error_upper'] for z in points),
                     'max_actual_error_numeric':max(z['actual_error_numeric'] for z in points),
                     'last_state_bits':points[-1]['state_bits'],'first_6_points':points[:6],
                     'trunc_max_exact_float':float(p.max_trunc_clock),'residual_budget_exact_float':float(p.residual_budget)}
            except Exception as exc:
                arm={'fast_first':fast,'status':'REJECTED','reason':str(exc)}
            arms.append(arm)
        results.append({'cohort':label,'inputs':{'k':k,'g':g,'Y':Y,'T':T,'N':N,'eps':eps},'arms':arms})
        print(label,[(a['fast_first'],a['status'],a.get('work',{}).get('rhs_attempted'),a.get('reason')) for a in arms],flush=True)
    import pathlib
    dest=pathlib.Path(__file__).parent/'RESULTS.json'
    dest.write_text(json.dumps({'model':'source-owned nonlinear BE positive scalar cubic',
                                'numerical_oracle_precision_digits':mp.mp.dps,
                                'claim':'exact rational original state/residual and exact rational global bound; mpmath separate numeric oracle, timings not authoritative',
                                'cohorts':results},indent=2)+'\n')
    assert all(a['status']=='PASS' for row in results[:3] for a in row['arms'])
    assert all(a['status']=='REJECTED' for a in results[-1]['arms'])
if __name__=='__main__':main()
