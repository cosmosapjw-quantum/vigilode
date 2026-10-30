#!/usr/bin/env python3
"""Actual BASS21 original 50x193 fixed-screen E+ high-l and upper-boundary shear flux.

No new dynamical IVP: projected local generator evaluated on preexisting frozen
original BASS photon+Thomson Volterra histories, not source-native E_l evolution.
"""
from __future__ import annotations
import argparse,csv,hashlib,json,math,pathlib,sys
import numpy as np
import sympy as sp
from numpy.polynomial.legendre import leggauss,Legendre
from scipy.special import lpmv

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'source_code'))
from coupled_thomson_volterra import solve_case,load_owner_archive
LL=np.arange(2,25,2,dtype=int)
LGEN=LL[:-1]
CASE_N=50; EPOCH_N=193
OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_SHA='a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee'
DAG={'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','official_delta':0}

def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def half_gl(n):
    mu,w=leggauss(n)
    return .5*(mu+1),.5*w

def cl_norm(l):return math.factorial(int(l)+2)/((2*int(l)+1)*math.factorial(int(l)-2))
def k_ell(l):return math.sqrt(4*math.pi/(2*int(l)+1) * math.factorial(int(l)+2)/math.factorial(int(l)-2))

def matrices_exact():
    """Exact polynomial integration of the adjoint LRS spin2 shear operator.

    (A_c)_{l,m}=(1/N_l)int_0^1 P_m^2[(1-3mu^2)P_l^2-3mu(1-mu^2)(P_l^2)'] dmu.
    E_l=k_l c_l and A_E=diag(k) A_c diag(k)^-1.
    """
    m=sp.symbols('mu')
    basis={int(l):sp.Poly((1-m*m)*sp.diff(sp.legendre(int(l),m),m,2),m) for l in LL}
    B=sp.Poly(1-3*m*m,m);D=sp.Poly(3*m*(1-m*m),m)
    A=sp.zeros(len(LL)); exact=[]
    for i,l in enumerate(LL):
        pl=basis[int(l)]; adj=(B*pl-D*pl.diff())
        norm=sp.factorial(int(l)+2)/((2*int(l)+1)*sp.factorial(int(l)-2))
        for j,n in enumerate(LL):
            if abs(i-j)>1:continue
            prod=adj*basis[int(n)]
            integ=sum(co/sp.Integer(deg[0]+1) for deg,co in prod.terms() if deg[0]%2==0)
            A[i,j]=sp.cancel(integ/norm)
        exact.append({str(int(LL[j])):str(A[i,j]) for j in range(max(0,i-1),min(len(LL),i+2))})
    assert A[0,0]==sp.Rational(10,7) and A[0,1]==sp.Rational(10,7)
    assert A[1,0]==-sp.Rational(36,35) and A[1,2]==sp.Rational(504,143)
    assert A[7,8]==sp.Rational(3420,259)   # l16 to l18
    k=np.array([k_ell(l) for l in LL]); C=np.array(A.tolist(),float)
    E=k[:,None]*C/k[None,:]
    return C,E,{'schema':'vigilode.bass21.eharm_l24_exact_v1','l':LL.tolist(),
                'A_c_rows':exact,'k_plus':k.tolist(),
                'operator_form':'S_E_l=bprime*sum_m(k_l/k_m)A_c_lm E_m; S_c from adjoint spin2 shear projection',
                'collision_form':'C_E2=-g*(k2*J2_total/4+2E2/5); C_E_l=-g*E_l for even l>=4',
                'no_finite_closure':'For truncation L, boundary flux is bprime*(k_L/k_{L+2})A_c[L,L+2]*E_{L+2}',
                'screen':'plus-E_l0 axisymmetric transported screen, Q=I_theta-I_phi, U=0; B_l0=0 only here'}

def project(Q,mu,w):
    B=np.stack([lpmv(2,int(l),mu) for l in LL],axis=1)
    c=(Q*w[None,:])@B/np.array([cl_norm(l) for l in LL])[None,:]
    return c*np.array([k_ell(l) for l in LL])[None,:]

def project_derivative_independent(Q,mu,w):
    """Independent polynomial differentiation basis, not lpmv recursion."""
    B=np.stack([(1-mu**2)*Legendre.basis(int(l)).deriv(2)(mu) for l in LL],axis=1)
    c=(Q*w[None,:])@B/np.array([cl_norm(l) for l in LL])[None,:]
    return c*np.array([k_ell(l) for l in LL])[None,:]

def csvout(path,rows):
    with open(path,'w',newline='',encoding='utf8') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

def run(owner,parent,output,limit=50):
    assert sha(owner)==OWNER_SHA,'OWNER_ZIP_BYTE_MISMATCH'
    assert sha(parent)==PARENT_SHA,'PARENT_PR143_BYTE_MISMATCH'
    originals=load_owner_archive(owner)
    cases=sorted(originals)[:limit]
    assert len(originals)==50 and len(cases)==limit
    with np.load(ROOT/'source/PR141_EHARM_50x193.npz',allow_pickle=False) as z:
        assert np.array_equal(z['cases'][:limit],np.asarray(cases))
        refE=z['E_l0_plus'][:limit].copy()
        refc=z['c_l'][:limit].copy()
        refJ2=z['J2_total'][:limit].copy()
        x=z['x'].copy()
    assert len(x)==193
    C,A,coeff=matrices_exact()
    a1,w1=half_gl(64);a2,w2=half_gl(96)
    nu=len(x)
    E=np.zeros((limit,nu,len(LL)));E96=np.zeros_like(E)
    shear=np.zeros((limit,nu,len(LGEN)));collision=np.zeros_like(shear)
    flux=np.zeros((limit,nu,4));deriv_defect=np.zeros_like(shear)
    cases_end=[];maxgap=0.;maxbasis=0.;maxparent=0.;maxparentc=0.;maxJ2=0.;maxpositivity=0.;maxsource=0.
    gap_mode=np.zeros(len(LL));maxamplitude_s00=np.zeros(len(LL));maxamplitude_s04=np.zeros(len(LL))
    for i,case in enumerate(cases):
        h=originals[case]
        assert np.array_equal(x,h['x']),'TIME_GRID_IDENTITY_CHANGED'
        solution=solve_case(h,nangular=48,mu_out=np.r_[a1,a2])
        q=solution['Q'][:,:64];q96=solution['Q'][:,64:]
        I=solution['fullI'][:,:64]
        if np.any(I<=0) or np.any(np.abs(q)>I+1e-12):raise AssertionError('UNPHYSICAL_POLARIZATION')
        z=project(q,a1,w1);t=project(q96,a2,w2)
        zother=project_derivative_independent(q,a1,w1)
        E[i]=z;E96[i]=t
        gap_mode=np.maximum(gap_mode,np.max(abs(z-t),axis=0))
        maxgap=max(maxgap,float(np.max(abs(z-t))));maxbasis=max(maxbasis,float(np.max(abs(z-zother))))
        maxparent=max(maxparent,float(np.max(abs(z[:,:8]-refE[i]))))
        cnew=z[:,:8]/np.array([k_ell(l) for l in LL[:8]])
        maxparentc=max(maxparentc,float(np.max(abs(cnew-refc[i]))))
        maxsource=max(maxsource,float(np.max(abs(solution['fixed_res']))))
        m0=refJ2[i];m1=h['photon_energy_moments'][:,2]
        j2fromQ=solution['fullI'][:,:64]@(w1*(3*a1*a1-1)/2)
        maxJ2=max(maxJ2,float(np.max(abs(j2fromQ-m0))))
        bp=np.gradient(h['b'],x,edge_order=2);g=h['g']
        sh=bp[:,None]*(z@A.T)[:,:-1]
        co=-g[:,None]*z[:,:-1]
        co[:,0]=-g*(k_ell(2)*m0/4 +.4*z[:,0])
        shear[i]=sh;collision[i]=co
        deriv_defect[i]=np.gradient(z[:,:-1],x,axis=0,edge_order=2)-(sh+co)
        for idx,l in enumerate([16,18,20,22]):
            li=int(np.where(LL==l)[0][0])
            flux[i,:,idx]=bp*A[li,li+1]*z[:,li+1]
        maxpositivity=max(maxpositivity,float(np.max(np.abs(q)/I)))
        if case.startswith('s00'):maxamplitude_s00=np.maximum(maxamplitude_s00,np.max(np.abs(z),axis=0))
        else:maxamplitude_s04=np.maximum(maxamplitude_s04,np.max(np.abs(z),axis=0))
        cases_end.append({'case':case,'source_sha256':h['source_sha256'],'E20_final':float(z[-1,0]),
                          **{f'E{int(l)}0_plus_final':float(z[-1,j]) for j,l in enumerate(LL[8:],start=8)},
                          'flux_l16_final':float(flux[i,-1,0]),'flux_l18_final':float(flux[i,-1,1]),
                          'flux_l20_final':float(flux[i,-1,2]),'flux_l22_final':float(flux[i,-1,3])})
        if (i+1)%10==0:print('SOURCE_HIGH_E_CASES=%d/%d'%(i+1,limit),flush=True)
    assert maxparent<1e-10 and maxparentc<1e-10 and maxgap<1e-10 and maxbasis<1e-10 and maxJ2<1e-10
    output=pathlib.Path(output);output.mkdir(parents=True,exist_ok=True)
    total=shear+collision
    np.savez_compressed(output/'HIGH_E_HIERARCHY_50x193.npz',cases=np.asarray(cases),x=x,l=LL,
                        E_l0_plus=E,E_GL96=E96,shear_E=shear,collision_E=collision,
                        local_generator_E=total,boundary_flux_16_18_20_22=flux,
                        saved_grid_derivative_defect=deriv_defect)
    csvout(output/'ENDPOINT_50_HIGH_E.csv',cases_end)
    pairs=[]
    if limit==50:
        ix={v:i for i,v in enumerate(cases)}
        for m in range(5):
            for k in range(5):
                a=ix[f's00_m{m:02d}_r{k:02d}'];b=ix[f's04_m{m:02d}_r{k:02d}']
                pairs.append({'pair':f'm{m:02d}_r{k:02d}',
                              **{f'delta_E{int(l)}0_plus_final':float(E[b,-1,j]-E[a,-1,j]) for j,l in enumerate(LL[8:],start=8)},
                              'delta_flux_l16_final':float(flux[b,-1,0]-flux[a,-1,0])})
    else:pairs=[cases_end[0]]
    csvout(output/'MATCHED_25_HIGH_E.csv',pairs)
    means=[]
    for k,v in enumerate(x):
        means.append({'x':float(v),
                     **{f'mean_E{int(l)}0_plus':float(E[:,k,j].mean()) for j,l in enumerate(LL)},
                     **{f'mean_flux_l{int(l)}':float(flux[:,k,j].mean()) for j,l in enumerate([16,18,20,22])}})
    csvout(output/'TIME193_HIGH_E_MEANS.csv',means)
    (output/'EXACT_LRS_E_OPERATOR.json').write_text(json.dumps(coeff,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
    summary={'schema':'vigilode.bass21.frozen_screen_E_higher_rank_v1',
             'source_owner_commit_repository':'cosmosapjw-quantum/bass',
             'source_sha256':OWNER_SHA,'parent_pr143_sha256':PARENT_SHA,
             'original_cases':limit,'saved_epochs':nu,'new_ivps':0,
             'new_high_coefficients':limit*nu*4,
             'new_generator_values':limit*nu*11,
             'new_upper_flux_values':limit*nu*4,
             'highest_mode_l':24,'generator_max_l':22,
             'max_reuse_E_l2_16':maxparent,'max_reuse_c_l2_16':maxparentc,
             'max_GL64_96_gap':maxgap,'max_independent_basis_gap':maxbasis,
             'max_J2_reuse_gap':maxJ2,'max_source_fixed_residual':maxsource,
             'max_polarization_fraction':maxpositivity,
             'max_abs_E_per_mode':[float(np.max(np.abs(E[:,:,j]))) for j in range(len(LL))],
             'max_abs_flux':[float(np.max(np.abs(flux[:,:,j]))) for j in range(4)],
             'max_saved_grid_derivative_defect':float(np.max(abs(deriv_defect))),
             'max_gap_by_mode':gap_mode.tolist(),
             'max_abs_by_shear_family':{'s00':maxamplitude_s00.tolist(),'s04':maxamplitude_s04.tolist()},
             'resolved_max_vs_GL10_by_family':{'s00':(maxamplitude_s00>10*gap_mode).tolist(),
                                               's04':(maxamplitude_s04>10*gap_mode).tolist()},
             'pair_mean_delta_E18':float(np.mean([p['delta_E180_plus_final'] for p in pairs])) if limit==50 else None,
             'pair_mean_delta_flux_l16':float(np.mean([p['delta_flux_l16_final'] for p in pairs])) if limit==50 else None,
             'source_by_case_SHA256':{case:originals[case]['source_sha256'] for case in cases},
             'dag':DAG,'production':'HOLD','distinct_author_R6_C9':'NOT_RUN',
             'scope':'Physical source-frozen local spin2 E+ hierarchy and upper-boundary transport, NOT dynamically source-owned E_l full IVP, and not observed CMB E/B.'}
    (output/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,sort_keys=True)+'\n')
    print('FINAL_SOURCE_HIGH_E '+json.dumps({k:v for k,v in summary.items() if k not in ['source_by_case_SHA256','max_abs_E_per_mode','max_gap_by_mode','max_abs_by_shear_family','resolved_max_vs_GL10_by_family']},sort_keys=True),flush=True)
    return summary

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--owner',required=True);p.add_argument('--parent',required=True);p.add_argument('--out',required=True);p.add_argument('--limit',type=int,default=50)
    a=p.parse_args();run(a.owner,a.parent,a.out,a.limit)
