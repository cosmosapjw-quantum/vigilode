#!/usr/bin/env python3
"""BASS21 exact LRS scalar-J / screen-spin2-c local generator on original 50x193.

NO new IVP: evaluates the moment ODE vector field on PREEXISTING
source-frozen Volterra deltaI/Q histories.  This is NOT original BASS-native E_l.
"""
from __future__ import annotations
import argparse, csv, hashlib, json, math, os, pathlib, sys
import numpy as np
from numpy.polynomial.legendre import leggauss,legvander
from scipy.special import lpmv
from scipy.integrate import cumulative_simpson

THIS=pathlib.Path(__file__).resolve()
ROOT=THIS.parents[1]
PARENT=ROOT.parent/'base'
for path in (PARENT/'source_code',PARENT/'code'):
    sys.path.insert(0,str(path))
from coupled_thomson_volterra import solve_case,load_owner_archive
from e2_spin2_atlas import BQ_TO_E2

SOURCE_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_SHA='d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16'
LJ=np.arange(0,15,2,dtype=int)  # 0..14, generator only through l=10
LC=np.arange(2,17,2,dtype=int)  # 2..16, generator only through l=12


def sha256(path):
    with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def project_field(field,mu,w,kind,lvals):
    if kind=='j':
        basis=legvander(mu,int(max(lvals)))[:,lvals]
        norm=np.ones(len(lvals))
    else:
        basis=np.stack([lpmv(2,int(l),mu) for l in lvals],axis=-1)
        norm=np.array([math.factorial(int(l)+2)/((2*l+1)*math.factorial(int(l)-2)) for l in lvals])
    return (field*w[None,:])@basis/norm[None,:]


def exact_angular_generator_matrices():
    """Source of authority is EXACT polynomial integration, not fitted arrays."""
    import sympy as s
    mu=s.symbols('mu')
    def poly(l,kind):
        p=s.legendre(int(l),mu)
        return s.Poly(p if kind=='j' else (1-mu**2)*s.diff(p,mu,2),mu)
    def integrate_poly(p):
        return sum(coef/(deg[0]+1) if deg[0]%2==0 else s.S.Zero for deg,coef in p.terms())
    def mat(L,kind):
        n=len(L)
        P={int(l):poly(l,kind) for l in L}
        B=s.Poly(1-3*mu**2,mu); D=s.Poly(3*mu*(1-mu**2),mu)
        A=s.zeros(n)
        for i,l in enumerate(L):
            pl=P[int(l)]
            # By parts: (1-3mu^2) P_l - 3 mu(1-mu^2) P_l'
            proj=B*pl-D*pl.diff()
            norm=(s.factorial(int(l)+2)/((2*int(l)+1)*s.factorial(int(l)-2))) if kind=='c' else s.Rational(1,1)
            for j,m in enumerate(L):
                w=(2*int(m)+1) if kind=='j' else s.Rational(1,1)
                A[i,j]=s.cancel(w*integrate_poly(proj*P[int(m)])/norm)
        return A
    AJ=mat(LJ,'j');AC=mat(LC,'c')
    # one-dimensional parity ensures even sector sufficient.
    for a in (AJ,AC):
        for i in range(a.rows):
            for j in range(a.cols):
                if abs(i-j)>1 and a[i,j]!=0:raise AssertionError('NONTRIDIAGONAL_SHEAR')
    assert [AJ[0,i] for i in range(3)]==[0,-2,0]
    assert [AC[0,i] for i in range(3)]==[s.Rational(10,7),s.Rational(10,7),0]
    return np.array(AJ.tolist(),dtype=float),np.array(AC.tolist(),dtype=float),{
        'intensity_even_l':LJ.tolist(), 'spin2_even_l':LC.tolist(),
        'A_J':[[str(AJ[i,j]) for j in range(AJ.cols)] for i in range(AJ.rows)],
        'A_c':[[str(AC[i,j]) for j in range(AC.cols)] for i in range(AC.rows)],
        'generator_rows_J':LJ[:6].tolist(),'generator_rows_c':LC[:6].tolist(),
        'operator':'d/dx=bprime*A - g*C, source-frozen coupled perturbation sector'
    }


def one_case(h,AJ,AC,na=48,n1=64,n2=96):
    mu1,w1=leggauss(n1);mu1=(mu1+1)*.5;w1=w1*.5
    mu2,w2=leggauss(n2);mu2=(mu2+1)*.5;w2=w2*.5
    sol=solve_case(h,nangular=na,mu_out=np.r_[mu1,mu2])
    di=sol['deltaI'][:,:n1];q=sol['Q'][:,:n1]
    di2=sol['deltaI'][:,n1:];q2=sol['Q'][:,n1:]
    J=project_field(di,mu1,w1,'j',LJ)
    C=project_field(q,mu1,w1,'c',LC)
    Jref=project_field(di2,mu2,w2,'j',LJ)
    Cref=project_field(q2,mu2,w2,'c',LC)
    bp=np.gradient(h['b'],h['x'],edge_order=2)
    g=h['g']
    j2orig=h['photon_energy_moments'][:,2]
    e2=12/(5*np.sqrt(6.))*C[:,0]
    # moment shear generators through l=10 (J12 is measured closure input)
    # and c through l=12 (c14 is measured closure input).
    shearJ=bp[:,None]*(J@AJ.T)[:,:6]
    shearC=bp[:,None]*(C@AC.T)[:,:6]
    collisionJ=-g[:,None]*J[:,:6]
    collisionJ[:,0]=0
    collisionJ[:,1]=-g*(.9*J[:,1]+np.sqrt(6)/10*e2)
    collisionC=-g[:,None]*C[:,:6]
    collisionC[:,0]=-g*((j2orig+J[:,1])/4 +.4*C[:,0])
    generatorJ=shearJ+collisionJ
    generatorC=shearC+collisionC
    dx=h['x']
    derivJ=np.gradient(J[:,:6],dx,axis=0,edge_order=2)
    derivC=np.gradient(C[:,:6],dx,axis=0,edge_order=2)
    # integrate only for error characterization; all actual outputs are source-evaluated local rate.
    dresJ=derivJ-generatorJ;dresC=derivC-generatorC
    j0int=cumulative_simpson(generatorJ[:,0],x=dx,initial=0)
    if np.any(sol['fullI']<=0) or np.any(abs(sol['Q'])>sol['fullI']):raise AssertionError('PHOTON_COHERENCY')
    if abs(J[0]).max()>1e-12 or abs(C[0]).max()>1e-12:raise AssertionError('SOURCE_INITIAL_RESPONSE_NOT_ZERO')
    return dict(J=J,C=C,Jref=Jref,Cref=Cref,generatorJ=generatorJ,generatorC=generatorC,
                shearJ=shearJ,shearC=shearC,collisionJ=collisionJ,collisionC=collisionC,
                residualJ=dresJ,residualC=dresC,bp=bp,j0_work_gap=J[:,0]-j0int,
                fixed_operator_res=float(sol['fixed_res']),positive_intensity=float(np.min(sol['fullI'])),
                max_polarization_ratio=float(np.max(np.abs(sol['Q']/sol['fullI']))))


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--source',required=True)
    ap.add_argument('--canonical-parent',required=True)
    ap.add_argument('--reference-j',required=True)
    ap.add_argument('--reference-c',required=True)
    ap.add_argument('--out',required=True)
    ap.add_argument('--limit',type=int,default=50)
    z=ap.parse_args()
    if sha256(z.source)!=SOURCE_SHA:raise RuntimeError('SOURCE_OWNER_SHA_DRIFT')
    if sha256(z.canonical_parent)!=PARENT_SHA:raise RuntimeError('PR134_PARENT_SHA_DRIFT')
    aj,ac,exact=exact_angular_generator_matrices()
    out=pathlib.Path(z.out);out.mkdir(parents=True,exist_ok=True)
    (out/'EXACT_GENERATOR.json').write_text(json.dumps(exact,indent=2,sort_keys=True)+'\n')
    with np.load(z.reference_j) as q:
        old_cases=q['cases'].copy(); old_j=q['deltaJ_even'].copy()
    with np.load(z.reference_c) as q:
        c_cases=q['cases'].copy(); old_c=q['spin2_c_even'].copy()
    histories=load_owner_archive(z.source)
    cases=sorted(histories)[:z.limit]
    assert len(cases)==z.limit and z.limit<=50
    assert np.array_equal(old_cases[:z.limit],cases) and np.array_equal(c_cases[:z.limit],cases)
    X=histories[cases[0]]['x'];n=len(X);assert n==193
    data={a:np.empty((z.limit,n,6)) for a in ('generatorJ','generatorC','shearJ','shearC','collisionJ','collisionC','residualJ','residualC')}
    jhigh=np.empty((z.limit,n,2));chigh=np.empty((z.limit,n,3))
    closureJ=np.empty((z.limit,n));closureC=np.empty((z.limit,n))
    meta=[];endpoint=[]
    max_j_reuse=0;max_c_reuse=0;max_gl_j=0;max_gl_c=0;max_j0work=0
    for ix,key in enumerate(cases):
        h=histories[key];r=one_case(h,aj,ac)
        max_j_reuse=max(max_j_reuse,float(np.max(abs(r['J'][:,:6]-old_j[ix]))))
        max_c_reuse=max(max_c_reuse,float(np.max(abs(r['C'][:,:5]-old_c[ix]))))
        max_gl_j=max(max_gl_j,float(np.max(abs(r['J']-r['Jref']))))
        max_gl_c=max(max_gl_c,float(np.max(abs(r['C']-r['Cref']))))
        max_j0work=max(max_j0work,float(np.max(abs(r['j0_work_gap']))))
        for k in data:data[k][ix]=r[k]
        jhigh[ix]=r['J'][:,6:8];chigh[ix]=r['C'][:,5:8]
        # Upper-neighbour contribution enters last generated row, retained measured response.
        closureJ[ix]=r['bp']*aj[5,6]*r['J'][:,6]
        closureC[ix]=r['bp']*ac[5,6]*r['C'][:,6]
        meta.append(dict(case=key,source_sha256=h['source_sha256'],fixed_operator_res=r['fixed_operator_res'],
                         positive_intensity=r['positive_intensity'],max_polarization_ratio=r['max_polarization_ratio'],
                         max_local_J_residual=float(abs(r['residualJ']).max()),max_local_c_residual=float(abs(r['residualC']).max())))
        endpoint.append(dict(case=key,source_sha256=h['source_sha256'],deltaJ12=float(r['J'][-1,6]),deltaJ14=float(r['J'][-1,7]),
                             spin2_c12=float(r['C'][-1,5]),spin2_c14=float(r['C'][-1,6]),spin2_c16=float(r['C'][-1,7]),
                             deltaJ10_from_J12_shear=float(closureJ[ix,-1]),c12_from_c14_shear=float(closureC[ix,-1]),
                             J2_pol_collision=float(r['collisionJ'][-1,1]),c2_total_collision=float(r['collisionC'][-1,0]),
                             max_abs_sampled_J_budget=float(abs(r['residualJ']).max()),max_abs_sampled_c_budget=float(abs(r['residualC']).max())))
        if (ix+1)%10==0:print('PHYSICS_SOURCE_CASES',ix+1,'/',z.limit,flush=True)
    if max_j_reuse>1e-11 or max_c_reuse>1e-11 or max_gl_j>1e-10 or max_gl_c>1e-10:
        raise AssertionError('PARENT_REPLAY_OR_ANGULAR_CROSSCHECK_FAIL')
    pair_index=[]
    if z.limit==50:
        index={name:i for i,name in enumerate(cases)}
        pair_index=[(index[f's00_m{m:02d}_r{k:02d}'],index[f's04_m{m:02d}_r{k:02d}']) for m in range(5) for k in range(5)]
    def emitcsv(path,rows):
        with open(path,'w',newline='',encoding='utf-8') as f:
            wr=csv.DictWriter(f,fieldnames=list(rows[0]));wr.writeheader();wr.writerows(rows)
    emitcsv(out/'ENDPOINT_50_NEW_JC.csv',endpoint)
    emitcsv(out/'AUDIT_50_JC.csv',meta)
    paired=[]
    for i,(a,b) in enumerate(pair_index):
        paired.append(dict(pair=f'm{i//5:02d}_r{i%5:02d}',delta_J12=float(jhigh[b,-1,0]-jhigh[a,-1,0]),
            delta_c12=float(chigh[b,-1,0]-chigh[a,-1,0]),delta_c14=float(chigh[b,-1,1]-chigh[a,-1,1]),
            delta_J2_pol_collision=float(data['collisionJ'][b,-1,1]-data['collisionJ'][a,-1,1]),
            delta_c2_collision=float(data['collisionC'][b,-1,0]-data['collisionC'][a,-1,0])))
    if paired:emitcsv(out/'PAIRED_25_NEW_JC.csv',paired)
    np.savez_compressed(out/'BASS21_50x193_JC_GENERATOR.npz',cases=np.asarray(cases),x=X,
        j_l=np.asarray(LJ),c_l=np.asarray(LC),deltaJ12_14=jhigh,spin2_c12_14_16=chigh,
        closure_J10_from_deltaJ12=closureJ,closure_c12_from_c14=closureC,**data)
    s={'schema':'vigilode.bass21.source_owned_jc_local_generator.v1','original_cases':len(cases),'saved_epochs':n,
       'source_sha256':SOURCE_SHA,'canonical_parent_sha256':PARENT_SHA,'preregistered_before_new_50_case_campaign':True,
       'new_ivps':0,'source_geometry_frozen':True,'source_J_orig_frozen':True,
       'new_high_modes':{'deltaJ12_14':int(jhigh.size),'c12_14_16':int(chigh.size)},
       'new_local_generator_values':int(sum(data[k].size for k in ('generatorJ','generatorC'))),
       'generator_per_channel_shape':list(data['generatorJ'].shape),
       'max_parent_deltaJ_replay_gap':max_j_reuse,'max_parent_c_replay_gap':max_c_reuse,
       'max_angular_GL64_96_J_gap':max_gl_j,'max_angular_GL64_96_c_gap':max_gl_c,
       'max_J0_integral_work_gap':max_j0work,
       'max_saved_grid_deltaJ_local_residual':float(np.max(abs(data['residualJ']))),
       'max_saved_grid_c_local_residual':float(np.max(abs(data['residualC']))),
       'max_c12':float(np.max(abs(chigh[:,:,0]))),'max_c14':float(np.max(abs(chigh[:,:,1]))),
       'max_deltaJ12':float(np.max(abs(jhigh[:,:,0]))),
       'final_mean_c12':float(np.mean(chigh[:,-1,0])),
       'final_mean_deltaJ12':float(np.mean(jhigh[:,-1,0])),
       'final_paired_mean_delta_c12':float(np.mean([chigh[b,-1,0]-chigh[a,-1,0] for a,b in pair_index])) if pair_index else None,
       'max_J10_upper_flux_from_J12':float(np.max(abs(closureJ))),
       'max_c12_upper_flux_from_c14':float(np.max(abs(closureC))),
       'source_case_hashes_50':{k:histories[k]['source_sha256'] for k in cases},
       'limitations':'local RHS generator on source-frozen linear-response Volterra fields, not full native evolving BASS E_l; higher c_l screen convention only; sampled derivatives are not global BDF accuracy',
       'dag':{'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','official_delta':0},
       'production':'HOLD','distinct_author_R6_C9':'NOT_RUN'}
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(s,indent=2,sort_keys=True,allow_nan=False)+'\n')
    print('DONE '+json.dumps({k:v for k,v in s.items() if k not in ('source_case_hashes_50','limitations')},sort_keys=True),flush=True)

if __name__=='__main__':main()
