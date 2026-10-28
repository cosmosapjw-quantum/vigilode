#!/usr/bin/env python3
"""Reproducible small Thomson low-rank checks; no full solver claims.
Scalar closure and physical real transverse coherency are tested separately.
"""
from __future__ import annotations
import os
for _key in ('OPENBLAS_NUM_THREADS','OMP_NUM_THREADS','MKL_NUM_THREADS'):
    os.environ[_key] = '1'
import json, math, platform, sys, time, hashlib, importlib.util, subprocess, argparse
from pathlib import Path
import numpy as np
import scipy
from scipy.linalg import expm
from numpy.polynomial.legendre import leggauss


def grid(nmu: int, nphi: int):
    if nmu < 3 or nphi < 5:
        raise ValueError('degree-four rule requires nmu >= 3 and nphi >= 5')
    mu, wm = leggauss(nmu)
    phi = 2*np.pi*np.arange(nphi)/nphi
    z, ph = np.meshgrid(mu, phi, indexing='ij')
    z, ph = z.ravel(), ph.ravel()
    rr = np.sqrt(1-z*z)
    n = np.column_stack((rr*np.cos(ph), rr*np.sin(ph), z))
    w = np.repeat(wm/(2*nphi), nphi)  # normalized dOmega/(4pi)
    et = np.column_stack((z*np.cos(ph), z*np.sin(ph), -rr))
    ep = np.column_stack((-np.sin(ph), np.cos(ph), np.zeros_like(z)))
    P = np.eye(3)[None,:,:] - np.einsum('ni,nj->nij',n,n)
    B = np.stack((np.einsum('ni,nj->nij',et,et), np.einsum('ni,nj->nij',ep,ep),
                  (np.einsum('ni,nj->nij',et,ep)+np.einsum('ni,nj->nij',ep,et))/np.sqrt(2)),axis=1)
    return n,w,P,B


def quadrupoles(n):
    x,y,z=n.T
    return np.column_stack((np.sqrt(5)*(3*z*z-1)/2,np.sqrt(15)*x*z,
                            np.sqrt(15)*y*z,np.sqrt(15)*(x*x-y*y)/2,np.sqrt(15)*x*y))


def scalar_scatter(y,w,Q):
    return np.dot(w,y)+0.1*(Q @ (Q.T @ (w*y)))


def scalar_exact(y,w,Q,tau):
    if not np.isfinite(tau) or tau<0: raise ValueError('tau must be finite nonnegative')
    p0=np.dot(w,y); p2=Q @ (Q.T @ (w*y))
    return np.exp(-tau)*y - np.expm1(-tau)*p0 + np.exp(-0.9*tau)*(-np.expm1(-0.1*tau))*p2


def lift(T,P):
    return 1.5*(P @ T @ P)


def polarized_scatter(F,w,P):
    return lift(np.einsum('n,nij->ij',w,F),P)


def polarized_exact(F,w,P,tau):
    """Exact real-symmetric transverse elastic Thomson collision subflow.
    Evaluates rate integrals tau=int n_e*sigma_T*c dt. It does not integrate
    streaming, moving-frame transport, or frequency redistribution.
    """
    if not np.isfinite(tau) or tau<0: raise ValueError('tau must be finite nonnegative')
    T=np.einsum('n,nij->ij',w,F)
    T0=np.trace(T)/3*np.eye(3)
    # Positive coefficients make the PSD argument explicit. c=a0-a2 is O(tau^2).
    a0=-np.expm1(-tau)
    a2=np.exp(-0.3*tau)*(-np.expm1(-0.7*tau))/0.7
    if tau < 1e-3:
        coeffs=[((-1)**k)*(0.3-0.3**k)/(0.7*math.factorial(k)) for k in range(2,13)]
        c=0.0
        for coefficient in reversed(coeffs): c=coefficient+tau*c
        c=tau*tau*c
    else:
        c=a0-a2
    return np.exp(-tau)*F+lift(a2*T+c*T0,P)


def lowrank_exact(y, U, V, tau):
    """Exact exp(tau*(U V.T - I)) action, without rank/invertibility assumption.
    Stable 2r x 2r block exponential avoids exp(tau) overflow at a Markov mode.
    U and V are N x r. Fixed-grid setup may cache B and its Schur/eigenbasis.
    """
    if not np.isfinite(tau) or tau < 0:
        raise ValueError('tau must be finite nonnegative')
    r = U.shape[1]
    B = V.T @ U
    augmented = np.block([[B-np.eye(r), np.eye(r)], [np.zeros((r,r)), -np.eye(r)]])
    K = expm(tau*augmented)[:r,r:]
    return np.exp(-tau)*y + U @ (K @ (V.T @ y))


def fvm_reference_checks(repo):
    source=repo/'research/bassrei_p06_full_sphere_scoped_20261022/FULL_SPHERE_FVM_REFERENCE.py'
    if not source.exists():
        return {'status':'not_run','reason':'optional pinned repository source unavailable','path':str(source)}
    spec=importlib.util.spec_from_file_location('spherical_fvm_reference', source)
    mod=importlib.util.module_from_spec(spec)
    previous=sys.dont_write_bytecode; sys.dont_write_bytecode=True
    try:
        sys.modules[spec.name]=mod
        spec.loader.exec_module(mod)
    finally:
        sys.dont_write_bytecode=previous
    cases=[]
    rules=[('irregular',[-1.,-0.7,-0.2,0.1,0.5,1.],[0.,0.4,1.7,3.,4.2,2*np.pi]),
           ('rank_deficient',[-1.,0.,1.],[0.,2*np.pi])]
    for name,mu,phi in rules:
        U0,V0=mod.thomson6_factors(mu,phi)
        U,V=np.asarray(U0).T,np.asarray(V0).T
        N=len(U); S=U@V.T; omega=U[:,0]
        yy=omega*(1.+np.arange(N)**2/max(1,N)**2)
        rhs=np.asarray(mod.thomson6_rhs(mu,phi,list(yy),1.))
        rhs_err=float(np.max(abs(rhs-(S@yy-yy))))
        for tau in [0.,1e-12,0.1,1.,20.,1000.]:
            fast=lowrank_exact(yy,U,V,tau)
            ref=expm(tau*(S-np.eye(N)))@yy
            p0=omega*sum(yy)
            # Deliberately wrong transplant: assigning continuum eigenvalue 0.1 to all discrete quadrupoles.
            naive=np.exp(-tau)*yy-np.expm1(-tau)*p0+np.exp(-0.9*tau)*(-np.expm1(-0.1*tau))/0.1*(S@yy-p0)
            cases.append({'grid':name,'n':N,'gain_rank':int(np.linalg.matrix_rank(S)),'tau':tau,'factor_vs_existing_rhs':rhs_err,
                          'gain_nonzero_eigenvalues':[float(z.real) for z in np.linalg.eigvals(S) if abs(z)>1e-12],
                          'max_abs_dense_difference':float(np.max(abs(fast-ref))),'mass_error':float(abs(sum(fast)-sum(yy))),
                          'minimum':float(fast.min()),'naive_continuum_shortcut_error':float(np.max(abs(naive-ref)))})
    return {'status':'pass_scoped' if all(c['max_abs_dense_difference']<2e-11 and c['factor_vs_existing_rhs']<1e-13 and c['mass_error']<2e-11 and c['minimum']>=-1e-13 for c in cases) else 'fail',
            'source':str(source),'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
            'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'cases':cases}


def local_coeff(F,B): return np.einsum('naij,nij->na',B,F)
def from_coeff(v,B): return np.einsum('na,naij->nij',v,B)
def entropic_distance(F,w,M):
    # Restrict eigenvalues to the physical two-plane. Full 3x3 adds a zero eigenvalue.
    ev=np.linalg.eigvalsh(F)[:,1:]
    ev=np.maximum(ev,0)
    g=M/2
    positive=ev>0
    out=-ev+g
    out[positive]+=ev[positive]*np.log(ev[positive]/g)
    return float(np.dot(w,out.sum(axis=1)))


def timer(fun, repetitions=30, samples=7):
    fun()
    vals=[]
    for _ in range(samples):
        t=time.perf_counter_ns()
        for __ in range(repetitions): fun()
        vals.append((time.perf_counter_ns()-t)*1e-9/repetitions)
    return {'median_seconds':float(np.median(vals)),'min_seconds':min(vals),'max_seconds':max(vals),'samples_seconds':vals,'repetitions':repetitions}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo',type=Path,default=Path(__file__).resolve().parents[2]/'vigilode-reaudit-1023',help='optional checkout for frozen FVM regression')
    args=parser.parse_args()
    start=time.perf_counter()
    rng=np.random.default_rng(20261023)
    n,w,P,B=grid(3,6); N=len(w); Q=quadrupoles(n)
    S=0.75*(1+(n@n.T)**2)*w[None,:]
    y=np.exp(3*n[:,0])+0.01
    eigscalar=np.linalg.eigvalsh(np.sqrt(w)[:,None]*S/np.sqrt(w)[None,:])
    scalar_checks={'n':N,'gram_defect':float(np.max(np.abs(Q.T@(w[:,None]*Q)-np.eye(5)))),'row_sum_defect':float(np.max(np.abs(S.sum(axis=1)-1))),'matvec_max_abs':float(np.max(np.abs(S@y-scalar_scatter(y,w,Q)))),'eigenvalues':eigscalar.tolist(),'cases':[]}
    for tau in [0.,1e-10,0.1,1.,20.,1000.]:
        fast=scalar_exact(y,w,Q,tau)
        ref=expm(tau*(S-np.eye(N)))@y
        scalar_checks['cases'].append({'tau':tau,'max_abs_dense_difference':float(np.max(np.abs(fast-ref))),'relative_dense_difference':float(np.max(np.abs(fast-ref))/max(1.,np.max(np.abs(ref)))),'mass_error':float(abs(w@fast-w@y)),'minimum':float(fast.min())})
    # Dense physical superoperator constructed from projected local tensor basis.
    A=rng.normal(size=(N,2,2)); H=A@A.transpose(0,2,1)+0.05*np.eye(2)
    coeff=np.column_stack((H[:,0,0],H[:,1,1],np.sqrt(2)*H[:,0,1]))
    F=from_coeff(coeff,B); M=float(np.dot(w,np.trace(F,axis1=1,axis2=2)))
    dense=np.empty((3*N,3*N))
    for j in range(3*N):
        cell, component=divmod(j,3)
        dense[:,j]=local_coeff(lift(w[cell]*B[cell,component],P),B).ravel()
    ww=np.repeat(w,3)
    sym=np.sqrt(ww)[:,None]*dense/np.sqrt(ww)[None,:]
    eig=np.linalg.eigvalsh(sym)
    target=np.r_[np.zeros(3*N-6),np.full(5,0.7),1.]
    map_T=np.einsum('n,nij->ij',w,lift(np.diag([1.,2.,3.]),P))
    polarized_checks={'n':N,'degrees_of_freedom':3*N,'independent_moments':6,'self_adjoint_defect':float(np.max(abs(sym-sym.T))),'eigenvalue_max_defect':float(np.max(abs(eig-target))),'integrated_map_max_defect':float(np.max(abs(map_T-(0.7*np.diag([1.,2.,3.])+0.6*np.eye(3))))),'matvec_max_abs':float(np.max(abs(dense@coeff.ravel()-local_coeff(polarized_scatter(F,w,P),B).ravel()))),'initial_entropy':entropic_distance(F,w,M),'cases':[]}
    for tau in [0.,1e-10,0.1,1.,20.,1000.]:
        fast=polarized_exact(F,w,P,tau)
        ref=from_coeff((expm(tau*(dense-np.eye(3*N)))@coeff.ravel()).reshape(N,3),B)
        err=float(np.max(abs(fast-ref)))
        eigf=np.linalg.eigvalsh(fast)
        polarized_checks['cases'].append({'tau':tau,'max_abs_dense_difference':err,'relative_dense_difference':err/max(1.,float(np.max(abs(ref)))),'trace_mass_error':float(abs(np.dot(w,np.trace(fast,axis1=1,axis2=2))-M)),'min_eigenvalue_full_3x3':float(eigf.min()),'minimum_physical_eigenvalue':float(eigf[:,1].min()),'transverse_defect':float(np.max(abs(np.einsum('nij,nj->ni',fast,n)))),'entropy':entropic_distance(fast,w,M)})
    # Basis-free construction retains generated polarization; scalar evolution does not.
    I=1+0.6*(3*n[:,2]**2-1)/2
    F_unpol=0.5*I[:,None,None]*P
    evolved=polarized_exact(F_unpol,w,P,1.)
    e=local_coeff(evolved,B)
    generated=np.sqrt((e[:,0]-e[:,1])**2+2*e[:,2]**2)
    scalar_projection_discrepancy=float(np.max(abs(np.trace(evolved,axis1=1,axis2=2)-scalar_exact(I,w,Q,1.))))
    polarized_checks['unpolarized_quadrupole_example']={'tau':1.,'max_generated_linear_polarization':float(generated.max()),'scalar_intensity_model_error':scalar_projection_discrepancy}
    # Actual PSD stress tests, including almost pure states and high anisotropy.
    stress=[]
    for nmu,nphi in [(3,6),(8,16),(16,32)]:
        nn,wwg,PP,BB=grid(nmu,nphi)
        for label,floor,contrast in [('mixed',0.1,1.),('near_pure_beam',1e-12,100.)]:
            AA=rng.normal(size=(len(wwg),2,1)); HH=AA@AA.transpose(0,2,1)+floor*np.eye(2)
            HH*=np.exp(contrast*(nn[:,2]-1))[:,None,None]
            cf=np.column_stack((HH[:,0,0],HH[:,1,1],np.sqrt(2)*HH[:,0,1]))
            FF=from_coeff(cf,BB); mass=float(wwg@np.trace(FF,axis1=1,axis2=2)); ent0=entropic_distance(FF,wwg,mass)
            for tau in [1e-12,0.1,5.,1000.]:
                out=polarized_exact(FF,wwg,PP,tau)
                stress.append({'n':len(wwg),'case':label,'tau':tau,'minimum_eigenvalue':float(np.linalg.eigvalsh(out).min()),'mass_error':float(abs(wwg@np.trace(out,axis1=1,axis2=2)-mass)),'entropy_increase':entropic_distance(out,wwg,mass)-ent0})
    # Matvec timings avoid expensive full trajectories; same double precision inputs.
    timings=[]
    for nmu,nphi in [(8,16),(16,32),(32,64)]:
        nn,wwg,PP,BB=grid(nmu,nphi); QQ=quadrupoles(nn); yy=np.exp(nn[:,0]+0.4*nn[:,2]); size=len(wwg)
        dd=0.75*(1+(nn@nn.T)**2)*wwg[None,:]
        d=timer(lambda: dd@yy, repetitions=20)
        f=timer(lambda: scalar_scatter(yy,wwg,QQ),repetitions=20)
        FF=0.5*yy[:,None,None]*PP
        p=timer(lambda: polarized_exact(FF,wwg,PP,10.),repetitions=20)
        timings.append({'n':size,'scalar_dense':d,'scalar_rank6':f,'polarized_exact_subflow':p,'scalar_median_speedup':d['median_seconds']/f['median_seconds'],'scalar_dense_storage_bytes':dd.nbytes,'scalar_rank6_basis_and_weights_bytes':QQ.nbytes+wwg.nbytes,'scalar_matvec_difference':float(np.max(abs(dd@yy-scalar_scatter(yy,wwg,QQ))))})
    checks=[scalar_checks['gram_defect']<1e-13,scalar_checks['row_sum_defect']<1e-13,scalar_checks['matvec_max_abs']<1e-12,polarized_checks['eigenvalue_max_defect']<1e-13,polarized_checks['integrated_map_max_defect']<1e-13,polarized_checks['matvec_max_abs']<1e-12]
    for r in scalar_checks['cases']+polarized_checks['cases']: checks.append(r['relative_dense_difference']<2e-12)
    checks.extend(r['entropy_increase']<1e-11 and r['minimum_eigenvalue']>-1e-11 and r['mass_error']<1e-11 for r in stress)
    fvm=fvm_reference_checks(args.repo)
    if fvm['status']!='not_run':
        checks.append(fvm['status']=='pass_scoped')
    result={'schema':'spherical-information.thomson-demo.v1','status':'pass_scoped' if all(checks) else 'fail','seed':20261023,'scope':'fixed rest-frame frequency shell; cold elastic Thomson; polarized real symmetric coherency V=0; no streaming or geometry coupling','scalar_scope':'explicit intensity-only depolarizing model, not repeated polarized Thomson','quadrature':'positive Gauss-Legendre mu times uniform phi; normalized dOmega/(4pi)','implementation':{'python':sys.version,'numpy':np.__version__,'scipy':scipy.__version__,'platform':platform.platform(),'blas_thread_limit':1,'source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},'scalar':scalar_checks,'polarized':polarized_checks,'psd_entropy_stress':stress,'existing_fvm_reference':fvm,'timing':timings,'elapsed_seconds':time.perf_counter()-start,'timing_interpretation':'seven process-local samples; fixed input and precision; matvec only; dense construction excluded; no whole-solver speed claim'}
    out=Path(__file__).resolve().parents[1]/'results'/'thomson_results.json'
    out.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'status':result['status'],'elapsed_seconds':result['elapsed_seconds'],'scalar_speedups':[r['scalar_median_speedup'] for r in timings],'polarized_eigenvalue_defect':polarized_checks['eigenvalue_max_defect'],'polarized_dense_max_difference':max(r['max_abs_dense_difference'] for r in polarized_checks['cases']),'output':str(out)},indent=2))
    if not all(checks): raise SystemExit(1)

if __name__=='__main__': main()
