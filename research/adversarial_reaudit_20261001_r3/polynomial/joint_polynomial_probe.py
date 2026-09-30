#!/usr/bin/env python3
"""R3 joint Chebyshev/Laguerre research. Exact-arithmetic tails != total certificate."""
import argparse, hashlib, json, math, platform, time
from decimal import Decimal, localcontext
from pathlib import Path
import numpy as np
import scipy
from scipy.special import ive
from numpy.polynomial.legendre import leggauss

SOURCE='cc2cd041737e7ff543624d1b59893a3b4397369f'
TRUNCATION_BUDGET=1e-12
OBSERVED_TARGET=1e-10

def scalar_phi(z):
    with localcontext() as c:
        c.prec=120
        x=z if isinstance(z,Decimal) else Decimal.from_float(float(z)); p=[x.exp()]
        for k in range(1,5):
            p.append((p[-1]-Decimal(1)/Decimal(math.factorial(k-1)))/x if x else Decimal(1)/Decimal(math.factorial(k)))
        return p

def reference(eigs,Q,W,h):
    # Exact Decimal products/sums of the supplied binary64 inputs, orthogonal Q known analytically.
    with localcontext() as c:
        c.prec=120
        n=len(eigs); q=[[Decimal.from_float(float(t)) for t in row] for row in Q]
        w=[[Decimal.from_float(float(t)) for t in row] for row in W]
        transformed=[[sum(q[j][i]*w[j][k] for j in range(n)) for k in range(5)] for i in range(n)]
        functions=[scalar_phi(Decimal.from_float(h)*Decimal.from_float(float(z))) for z in eigs]
        columns=[[sum(q[i][j]*functions[j][k]*transformed[j][k] for j in range(n)) for k in range(5)] for i in range(n)]
        fused=[sum(row) for row in columns]
        return np.array([[float(t) for t in row] for row in columns]), np.array([float(t) for t in fused])

def admissible(A,h,lam,rho):
    if h<0 or not 0<=lam<=rho or not np.array_equal(A,A.T):
        raise ValueError('OUTSIDE_DECLARED_SYMMETRIC_NONPOSITIVE_DOMAIN')
    # Spectral containment is a caller-supplied mathematical precondition, not inferred from this check.

def degree_cheb(h,lam,rho,weight_factor):
    b=h*(rho-lam)/2
    if b==0:return 0,0.
    for m in range(2048):
        r=m+1
        logbound=math.log(2)+math.hypot(b,r)-b-r*math.asinh(r/b)
        if logbound+math.log(weight_factor)<=math.log(TRUNCATION_BUDGET):return m,math.exp(logbound)
    raise RuntimeError('degree cap')

def degree_lag(h,rho,weight_factor):
    if h*rho==0:return 0,1.,0.,0.
    options=[]
    for L in (1.,2.,4.,8.,16.):
        beta=rho/L;a=h*beta;q=a/(1+a)
        m=max(0,math.ceil((math.log(TRUNCATION_BUDGET/weight_factor)-L/2)/math.log(q))-1)
        options.append((m,beta,L,q))
    return min(options)

def coefficients(kind,m,h,lam,rho,p,nquad):
    u,w=leggauss(nquad);u=(u+1)/2;w=w/2
    integ=np.stack([w*(1-u)**(k-1)/math.factorial(k-1) for k in range(1,5)])
    C=np.zeros((m+1,5))
    if kind=='chebyshev':
        b=h*(rho-lam)/2
        for n in range(m+1):
            factor=1 if n==0 else 2
            C[n,0]=factor*math.exp(-h*lam)*ive(n,b)
            C[n,1:]=integ@(factor*np.exp(-h*lam*u)*ive(n,b*u))
    else:
        a=h*p[1];qm=a/(1+a);qu=a*u/(1+a*u);power=np.ones_like(u)
        for n in range(m+1):
            C[n,0]=(1-qm)*qm**n
            C[n,1:]=integ@(power/(1+a*u))
            power*=qu
    return C

def action(kind,A,W,h,lam,rho,nquad=256):
    admissible(A,h,lam,rho)
    factor=sum(np.linalg.norm(W[:,k])/math.factorial(k) for k in range(5))
    if factor==0:return np.zeros_like(W),dict(degree=0,jvp_calls=0,jvp_vectors=0,truncation_upper_fused=0.)
    if h==0 or rho==0 or lam==rho:
        if lam==rho and not np.array_equal(A,-lam*np.eye(len(A))):raise ValueError('degenerate interval must be scalar matrix')
        C=np.array([float(t) for t in scalar_phi(-h*lam)])
        return W*C,dict(degree=0,jvp_calls=0,jvp_vectors=0,truncation_upper_fused=0.,direct_scalar_branch=True)
    if kind=='chebyshev':
        m,tail=degree_cheb(h,lam,rho,factor);p=None
        X=(A+(rho+lam)/2*np.eye(len(A)))/((rho-lam)/2)
    else:
        p=degree_lag(h,rho,factor);m=p[0];tail=math.exp(p[2]/2)*p[3]**(m+1);X=-A/p[1]
    C=coefficients(kind,m,h,lam,rho,p,nquad)
    prev=W.copy();total=prev*C[0];comp=np.zeros_like(total);abs_terms=np.abs(total)
    if m:
        curr=X@prev if kind=='chebyshev' else prev-X@prev
        for n in range(1,m+1):
            term=curr*C[n];abs_terms+=np.abs(term)
            corrected=term-comp;new=total+corrected;comp=(new-total)-corrected;total=new
            if n<m:
                nxt=2*X@curr-prev if kind=='chebyshev' else ((2*n+1)*curr-X@curr-n*prev)/(n+1)
                prev,curr=curr,nxt
    return total,dict(degree=m,jvp_calls=m,jvp_vectors=5*m,block_width=5,
        truncation_upper_fused=tail*factor,coefficient_min=float(C.min()),
        epsilon_abs_term_sum=float(np.finfo(float).eps*np.max(abs_terms)),
        normalization_cap=16 if kind=='laguerre' else None)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,default=Path(__file__).resolve().parent);a=parser.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    H=np.array([[1,1,1,1],[1,-1,1,-1],[1,1,-1,-1],[1,-1,-1,1]],dtype=float)/2
    families=[('diagonal24',-np.geomspace(.1,100.,24),np.eye(24),[.001,.01,.1,1.]),
              ('hadamard4',-np.array([.125,1.,8.,64.]),H,[1e-12,.01,.1,1.]),
              ('semidefinite4',-np.array([0.,.125,1.,8.]),H,[.1,1.])]
    rows=[];started=time.monotonic()
    for name,eigs,Q,hs in families:
        A=(Q*eigs)@Q.T;n=len(eigs)
        W=np.stack([np.cos(np.arange(n)*(.23+.11*k))+.2*(k+1) for k in range(5)],axis=1)
        W=W/np.linalg.norm(W,axis=0)*np.array([1.,-1.,.5,-.25,.125])
        for h in hs:
            refcols,reffused=reference(eigs,Q,W,h)
            for kind in ['laguerre','chebyshev']:
                Y,meta=action(kind,A,W,h,-float(max(eigs)),-float(min(eigs)))
                Y128,_=action(kind,A,W,h,-float(max(eigs)),-float(min(eigs)),128)
                rows.append(dict(family=name,h=h,h_rho=h*-float(min(eigs)),algorithm=kind,dimension=n,
                    input_sha256=hashlib.sha256(A.tobytes()+W.tobytes()).hexdigest(),
                    phi_column_errors=np.linalg.norm(Y-refcols,axis=0).tolist(),
                    fused_error_L2=float(np.linalg.norm(Y.sum(axis=1)-reffused)),
                    fused_quadrature_refinement=float(np.linalg.norm((Y-Y128).sum(axis=1))),
                    reference_fused_norm=float(np.linalg.norm(reffused)),
                    condition_proxy=float(sum(np.linalg.norm(refcols[:,k]) for k in range(5))/max(np.linalg.norm(reffused),1e-300)),**meta))
    edge=[]
    W=np.arange(20,dtype=float).reshape(4,5)/20
    for kind in ['laguerre','chebyshev']:
        for name,A,h,lam,rho in [('h_zero',-np.eye(4),0.,1.,1.),('A_zero',np.zeros((4,4)),1.,0.,0.),('scalar_matrix',-np.eye(4),1.,1.,1.)]:
            Y,meta=action(kind,A,W,h,lam,rho)
            with localcontext() as c:
                c.prec=100;z=Decimal.from_float(-h*lam)
                independent=[sum(z**j/Decimal(math.factorial(j+k)) for j in range(150)) if z else Decimal(1)/Decimal(math.factorial(k)) for k in range(5)]
            expected=W*np.array([float(t) for t in independent])
            edge.append(dict(case=name,algorithm=kind,error=float(np.max(np.abs(Y-expected))),**meta))
    rejects=[]
    for kind in ['laguerre','chebyshev']:
        try:action(kind,np.array([[-1.,100.],[0.,-1.]]),np.ones((2,5)),1.,1.,1.)
        except ValueError as e:rejects.append(dict(algorithm=kind,reason=str(e)))
    checks={'all_20_fused_actions_meet_observed_target':all(r['fused_error_L2']<=OBSERVED_TARGET for r in rows),
        'all_100_columns_meet_observed_target':all(max(r['phi_column_errors'])<=OBSERVED_TARGET for r in rows),
        'analytic_truncation_budgets_met':all(r['truncation_upper_fused']<=TRUNCATION_BUDGET*(1+1e-12) for r in rows),
        'edge_cases_match_independent_series':all(r['error']<=1e-15 for r in edge),
        'nonnormal_rejected_both':len(rejects)==2,
        'paired_inputs_identical':all(rows[i]['input_sha256']==rows[i+1]['input_sha256'] for i in range(0,len(rows),2))}
    result=dict(schema='vigilode.r3.joint_polynomial.v1',source_commit=SOURCE,
        research_status=['derived','numerically checked','implementation-verified'],
        operator_domain='Real symmetric nonpositive A with mathematically justified spectral enclosure [−rho,−lambda]; dense prototype symmetry check alone does not certify this enclosure.',
        action='sum(k=0..4) phi_k(h A) w_k; supplied w_k are distinct; scaled b_k conversion is out of scope',
        units='A,rho,lambda,beta: inverse time; h: time; hA, b=h(rho-lambda)/2: dimensionless',
        oracle='Decimal120 scalar phi, exact known orthogonal Q entries, independent sums for distinct columns and fused value',
        truncation_budget=TRUNCATION_BUDGET,observed_target=OBSERVED_TARGET,
        not_claimed=['rigorous total floating-point error','quadrature certificate','independent holdout optimization','wall-clock speedup','general nonnormal guarantee','production backend'],
        runtime={'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__},
        checks=checks,rows=rows,edge_cases=edge,domain_rejections=rejects,elapsed_seconds=time.monotonic()-started)
    (a.out/'RESULTS.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'checks':checks,'row_count':len(rows),'worst_fused_error':max(r['fused_error_L2'] for r in rows)},indent=2))
    if not all(checks.values()):raise SystemExit(1)

if __name__=='__main__':main()
