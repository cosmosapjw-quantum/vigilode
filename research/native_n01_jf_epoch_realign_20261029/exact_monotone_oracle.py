#!/usr/bin/env python3
"""Independent exact rational oracle for the derivative-free Robertson implicit RK stage.

No derivatives/JVPs are evaluated by the root algorithm; derivative formula is used
only in independent a posteriori proof checks. This does not implement RODAS5P.
"""
from fractions import Fraction as Q
import json
import random
from pathlib import Path


def make_stage(y, k, tau):
    a0,b0,c0=y
    k1,k2,k3=k
    M=sum(y, Q(0))
    den=1+tau*k1
    def c(b):return c0+tau*k3*b*b
    def a(b):return (a0+tau*k2*b*c(b))/den
    def F(b):return a(b)+b+c(b)-M
    La=tau*k2*(c0+3*tau*k3*M*M)/den
    Lc=2*tau*k3*M
    return a,c,F,La,Lc,M


def bracket(F,M,N):
    lo,hi=Q(0),M
    assert F(lo)<=0 and F(hi)>=0
    for _ in range(N):
        mid=(lo+hi)/2
        if F(mid)<0:lo=mid
        elif F(mid)>0:hi=mid
        else:lo=hi=mid;break
    return lo,hi


def fixed_case():
    y=(Q(35,64),Q(27,128),Q(31,128))
    k=(Q(1,2),Q(1),Q(1,2))
    tau=Q(1,4)
    a,c,F,La,Lc,M=make_stage(y,k,tau)
    b=Q(1,4)
    assert F(b)==0
    assert (a(b),b,c(b))==(Q(1,2),Q(1,4),Q(1,4))
    return dict(y=[str(z) for z in y], k=[str(z) for z in k],tau=str(tau),root=[str(a(b)),str(b),str(c(b))], La=str(La),Lc=str(Lc))


def run():
    rng=random.Random(20261029)
    n_cases=384
    checks=0
    last_examples=[]
    max_width=Q(0)
    for i in range(n_cases):
        y=tuple(Q(rng.randrange(0,257),128) for _ in range(3))
        if sum(y)==0:y=(Q(1,128),Q(0),Q(0))
        k=tuple(Q(rng.randrange(0,513),32) for _ in range(3))
        tau=Q(rng.randrange(0,33),32)
        a,c,F,La,Lc,M=make_stage(y,k,tau)
        lo,hi=bracket(F,M,60)
        lo_ref,hi_ref=bracket(F,M,144)
        b=(lo+hi)/2
        f=abs(F(b))
        # The reference interval encloses the true algebraic root.
        assert lo<=lo_ref<=hi_ref<=hi
        checks+=1
        assert F(lo)<=0 and F(hi)>=0
        checks+=1
        assert 0<=lo<=hi<=M
        checks+=1
        assert 0<=a(lo)<=a(hi)<=M and 0<=c(lo)<=c(hi)<=M
        checks+=1
        # Bound on every physical root coordinate, with sub-interval endpoint slack.
        slack=hi_ref-lo_ref
        assert max(abs(b-lo_ref),abs(b-hi_ref)) <= f+slack
        checks+=1
        assert max(abs(a(b)-a(lo_ref)),abs(a(b)-a(hi_ref))) <= La*(f+slack)
        checks+=1
        assert max(abs(c(b)-c(lo_ref)),abs(c(b)-c(hi_ref))) <= Lc*(f+slack)
        checks+=1
        # The stage defect is physically evaluated on the original 3-vector.
        av,bv,cv=a(b),b,c(b)
        k1,k2,k3=k
        flux=(-k1*av+k2*bv*cv,k1*av-k2*bv*cv-k3*bv*bv,k3*bv*bv)
        residual=tuple(x-y_i-tau*fi for x,y_i,fi in zip((av,bv,cv),y,flux))
        assert residual[0]==0 and residual[2]==0 and residual[1]==F(b)
        checks+=1
        assert sum(flux)==0 and sum(residual)==F(b)
        checks+=1
        max_width=max(max_width,hi-lo)
        if i in (0,1,2,383):
            last_examples.append(dict(i=i,tau=str(tau),mass=str(M),width=str(hi-lo),scalar_residual=str(F(b)),defect_linf=float(max(map(abs,residual))),error_up=float(max(La,Q(1),Lc)*f)))
    res={"schema":"vigilode.n01.df.monotone.r5.v1","date":"2026-10-29","model":"Robertson-type nonnegative 3-species autonomous mass action","track":"DERIVATIVE_FREE_RHS_ONLY_AUX_DIAGONAL_IRK","not_original_rodas5p":True,"not_ivp":True,"model_generation":"Random(20261029), exact dyadic rationals","exact_fixed_case":fixed_case(),"random_cases":n_cases,"checks_passed":checks,"checks_failed":0,"bisection_steps":60,"reference_steps":144,"max_certified_bracket_width_float":float(max_width),"samples":last_examples,"native_subagent":"NOT_RUN","vigilode_core_dag":"7/23","rei_dag":"28/88","successor_admitted":"0/12","production":"HOLD"}
    root=Path(__file__).parent
    (root/'RESULTS.json').write_text(json.dumps(res,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(res,indent=2,ensure_ascii=False))

if __name__=='__main__':run()
