#!/usr/bin/env python3
"""Original RODAS5P eight-stage raw-U tableau on N3H rank-2 two-rate source.
Independent Python dense reference, NOT a Rust native run and NOT adaptive acceptance.
"""
from pathlib import Path
import json, csv, math, sys
import numpy as np
import scipy.linalg
from scipy.integrate import solve_ivp

FIX=str(Path(__file__).resolve().parents[1]/'fixtures/rodas5p_coefficients_snapshot.json')
raw=json.loads(Path(FIX).read_text())
P=lambda s: float(s.split('/')[0])/float(s.split('/')[1]) if '/' in str(s) else float(s)
gamma=P(raw['gamma']); A=np.array([[P(x) for x in row] for row in raw['A']]); C=np.array([[P(x) for x in row] for row in raw['C']]); b=np.array([P(x) for x in raw['b_code']]);

def physical(n):
    j=np.arange(n);xi=2*j/(n-1)-1; w=np.full(n,1/n);phi=np.array([np.ones(n),np.where(j%2==0,1.,-1.)]);kap=np.where(j%2==0,.25,1000.)
    chi=np.array([.05,.02]);theta=.5*xi
    y0=np.zeros((n,2));y0[:,0]=.02+xi/500.;y0[:,1]=((j%3)-1)/200.
    def f(t,y):
        z=y.reshape(n,2);m=(phi*w).dot(z);mag=np.sum(m*m,axis=1)
        out=np.empty_like(z)
        out[:,0]=-kap*z[:,0]-theta*z[:,1] - np.einsum('a,aj,a->j',chi,phi,mag*m[:,1])
        out[:,1]=theta*z[:,0]-kap*z[:,1] + np.einsum('a,aj,a->j',chi,phi,mag*m[:,0])
        return out.ravel()
    def jac(t,y):
        z=y.reshape(n,2);m=(phi*w).dot(z);X=m[:,0];Y=m[:,1];d=2*n;J=np.zeros((d,d));
        for k in range(n):
            J[2*k,2*k]=-kap[k];J[2*k,2*k+1]=-theta[k]
            J[2*k+1,2*k]=theta[k];J[2*k+1,2*k+1]=-kap[k]
        for a in range(2):
            v=np.outer(chi[a]*phi[a],w*phi[a]);
            J[0::2,0::2]+=v*(-2*X[a]*Y[a]);J[0::2,1::2]+=v*(-X[a]**2-3*Y[a]**2)
            J[1::2,0::2]+=v*(3*X[a]**2+Y[a]**2);J[1::2,1::2]+=v*(2*X[a]*Y[a])
        return J
    energy=lambda z:float(np.sum(w*np.sum(z.reshape(n,2)**2,axis=1)))
    return y0.ravel(),f,jac,energy

def one(n,h):
    y,f,jac,energy=physical(n);N=len(y);J=jac(0,y);W=np.eye(N)-h*gamma*J;LU,piv=scipy.linalg.lu_factor(W)
    U=np.zeros((8,N));D=np.zeros_like(U);E=[]
    for i in range(8):
        ystage=y+np.einsum('j,jk->k',A[i,:i],U[:i]);mix=np.einsum('j,jk->k',C[i,:i],U[:i]);
        rhs=gamma*(mix+h*f(0,ystage));U[i]=scipy.linalg.lu_solve((LU,piv),rhs);
        D[i]=U[i]-h*gamma*(J@U[i])-gamma*mix-h*gamma*f(0,ystage)
        E.append(energy(ystage))
    new=y+b@U; rho=np.max(np.abs(D),axis=1);
    # Native coefficients.rs: btilde = last row Gamma, K=Gamma^{-1}U.
    # Therefore embedded error vector btilde*K equals raw U_7 in exact algebra.
    emb=U[-1]
    embedded_wrms=float(np.sqrt(np.mean((emb/((1e-10+1e-7*np.maximum(np.abs(y),np.abs(new)))))**2)))
    # sample mean first-column x0 etc
    ref=solve_ivp(f,(0,h),y,method='BDF',jac=jac,rtol=2e-12,atol=2e-14,max_step=min(h/10,0.001))
    delta=new-ref.y[:,-1]
    scale=(1e-10+1e-7*np.maximum(np.abs(y),np.abs(new)))
    return dict(n=n,h=h,max_original_raw_u_residual=float(np.max(rho)),per_stage_raw_u_residual=[float(x) for x in rho],initial_energy=energy(y),endpoint_energy=energy(new),maximum_stage_energy=max(E),max_endpoint_error=float(np.max(np.abs(delta))),endpoint_error_wrms=float(np.sqrt(np.mean((delta/scale)**2))),embedded_error_wrms=embedded_wrms, would_adaptively_accept=bool(embedded_wrms<=1.),bdf_success=bool(ref.success),bdf_nfev=int(ref.nfev),bdf_njev=int(ref.njev),bdf_lu=int(ref.nlu),direct_reference_solves=8,force_accept_diagnostic_only=True,first_output=[float(x) for x in new[:6]])

if __name__=='__main__':
    out=[]
    for n in (12,24):
        for h in (0.001,0.25):
            rec=one(n,h);out.append(rec)
            print(f"N4_RANK2_PY_RODAS8 n={n} h={h} rho={rec['max_original_raw_u_residual']:.3e} E0={rec['initial_energy']:.5e} E1={rec['endpoint_energy']:.5e} maxerr_vs_BDF={rec['max_endpoint_error']:.3e} wrms={rec['endpoint_error_wrms']:.3e} embedded={rec['embedded_error_wrms']:.3e} would_accept={rec['would_adaptively_accept']}")
    dest=Path(sys.argv[1] if len(sys.argv)>1 else 'evidence/RODAS8_PY_REFERENCE.json');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(out,indent=2)+'\n')
