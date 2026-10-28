#!/usr/bin/env python3
"""Joint Laguerre exp/phi1..4 same-input followup. No production import/mutation.
Analytic truncation bounds require symmetric negative operator. Quadrature and
roundoff are diagnosed, not certified. Decimal scalar formula is independent oracle.
"""
import json,math,hashlib,sys,platform
from pathlib import Path
from decimal import Decimal,localcontext
import numpy as np
import scipy
from numpy.polynomial.legendre import leggauss
from scipy.special import ive
OUT=Path(__file__).resolve().parent
TOL=1e-10

def plan(rho,h,cap):
    ps=[]
    for L in (1.,2.,4.,8.,16.,32.,64.):
        if L<=cap:
            beta=rho/L;a=h*beta;q=a/(1+a)
            m=max(0,math.ceil((math.log(TOL)-L/2)/math.log(q))-1)
            ps.append((m,beta,L,q))
    return min(ps)

def coefficients(m,a,nquad):
    u,w=leggauss(nquad);u=(u+1)/2;w=w/2
    z=a*u;q=z/(1+z);powers=np.ones_like(u)
    weights=np.stack([w*(1-u)**(k-1)/math.factorial(k-1)/(1+z) for k in range(1,5)])
    c=np.zeros((m+1,5));qm=a/(1+a)
    for n in range(m+1):
        c[n,0]=(1-qm)*qm**n
        c[n,1:]=weights@powers
        powers*=q
    return c

def joint(A,v,h,p,nquad,kahan=True):
    m,beta,L,q=p;coef=coefficients(m,h*beta,nquad);B=-A/beta
    prev=v.copy();total=coef[0,:,None]*prev[None,:];comp=np.zeros_like(total)
    abs_terms=np.abs(total)
    if m:
        curr=prev-B@prev
        for n in range(1,m+1):
            term=coef[n,:,None]*curr[None,:];abs_terms+=np.abs(term)
            if kahan:
                corrected=term-comp;new=total+corrected;comp=(new-total)-corrected;total=new
            else:total+=term
            if n<m:
                nxt=((2*n+1)*curr-B@curr-n*prev)/(n+1);prev,curr=curr,nxt
    return total,float(np.max(abs_terms)),coef

def scalar_phi(z):
    with localcontext() as c:
        c.prec=110;x=Decimal.from_float(float(z));out=[x.exp()]
        for k in range(1,5):out.append((out[-1]-Decimal(1)/Decimal(math.factorial(k-1)))/x if x else Decimal(1)/Decimal(math.factorial(k)))
        return np.array([float(t) for t in out])

def cheb(A,v,h,rho):
    r=h*rho/2
    for m in range(10000):
        s=math.asinh((m+1)/r);bound=2*math.exp(-s*(m+1)+r*(math.cosh(s)-1))
        if bound<=TOL:break
    X=np.eye(len(v))+2*A/rho;prev=v;out=ive(0,r)*prev
    if m:
        curr=X@v;out+=2*ive(1,r)*curr
        for n in range(1,m):
            nxt=2*X@curr-prev;out+=2*ive(n+1,r)*nxt;prev,curr=curr,nxt
    return out,m,bound

def run():
    A=-np.diag(np.geomspace(.1,100.,24));v=np.cos(np.arange(24)*.37)+.3;v/=np.linalg.norm(v)
    rows=[]
    for h in (.001,.01,.1,1.):
        ref=np.stack([scalar_phi(h*x) for x in np.diag(A)],axis=1)*v[None,:]
        for cap in (64.,16.):
            p=plan(100.,h,cap);x128,terms,c128=joint(A,v,h,p,128);x256,terms,c256=joint(A,v,h,p,256)
            raw,_,_=joint(A,v,h,p,256,kahan=False)
            errors=np.linalg.norm(x256-ref,axis=1);quad=np.linalg.norm(x256-x128,axis=1)
            tail=math.exp(p[2]/2)*p[3]**(p[0]+1)
            cb,cm,cbound=cheb(A,v,h,100.)
            rows.append(dict(h=h,h_rho=h*100,dimension=24,cap=cap,L=p[2],degree=p[0],JVPs=p[0],
                output_order=['exp','phi1','phi2','phi3','phi4'],error_L2=errors.tolist(),
                raw_summation_error_L2=np.linalg.norm(raw-ref,axis=1).tolist(),
                analytic_truncation_upper=[tail/math.factorial(k) for k in range(5)],
                quadrature_refinement_L2=quad.tolist(),all_outputs_meet_target=bool(np.all(errors<=TOL)),
                max_component_abs_term_sum=terms,epsilon_abs_term_sum=float(np.finfo(float).eps*terms),
                chebyshev_exp_JVPs=cm,chebyshev_exp_error=float(np.linalg.norm(cb-ref[0])),
                coefficient_nonnegative=bool(np.min(c256)>=0)))
    checks={
        'same_case_L64_failure_preserved':not rows[-2]['all_outputs_meet_target'],
        'same_case_cap16_all_outputs_pass':all(r['all_outputs_meet_target'] for r in rows if r['cap']==16),
        'coefficients_nonnegative':all(r['coefficient_nonnegative'] for r in rows),
        'joint_reuses_basis_five_outputs':all(r['JVPs']==r['degree'] for r in rows),
        'cap16_refinement_below_target':all(max(r['quadrature_refinement_L2'])<TOL for r in rows if r['cap']==16),
    }
    result=dict(schema='vigilode.reaudit.polynomial_joint.v1',source_commit='7708ef90554fc3986478d4602de6a01c7266b14f',
        evidence_status=['derived','numerically checked','implementation-verified'],
        claim_ceiling='standalone bounded prototype only; no production implementation, roundoff/quad certificate, nonnormal admission, or wall speedup',
        tolerance=TOL,runtime=dict(python=platform.python_version(),numpy=np.__version__,scipy=scipy.__version__),
        input_sha256=hashlib.sha256(A.tobytes()+v.tobytes()).hexdigest(),oracle='Decimal110 scalar exp and phi recurrence; same input v in every cap comparison',
        checks=checks,rows=rows)
    (OUT/'polynomial_joint_results.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps(result,indent=2));assert all(checks.values())
if __name__=='__main__':run()
