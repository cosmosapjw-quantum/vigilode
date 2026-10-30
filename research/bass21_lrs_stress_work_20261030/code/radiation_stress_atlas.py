#!/usr/bin/env python3
"""Source-owned photon energy/stress response on original frozen BASS21 histories.

Bounded model: PR132 linear Thomson deltaI,Q on unchanged LRS Bianchi-I
62-state b, gamma_T/H, tau and J_l histories. No original or new IVPs.
All radiation stress values are in the original a^-4 I_ref energy-brightness units;
Q is NOT itself counted as additional energy-momentum.
"""
from __future__ import annotations
import argparse, io, csv, json, hashlib, pathlib, sys, zipfile, tempfile
import numpy as np
from numpy.polynomial.legendre import legvander
from scipy.integrate import cumulative_simpson

HERE=pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent/'source_code'))
from coupled_thomson_volterra import solve_case,gl_half,load_owner_archive,MU

PARENT_SHA='ff17ce0c5883de8363898e83ebcb8b30322d35402d0cdbe4c678437493f6103a'
ORIGINAL_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
LVALS=np.array([0,2,4,6,8,10],dtype=int)


def sha(path):
    with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def restore_source(parent_zip, temp_path):
    if sha(parent_zip)!=PARENT_SHA:raise ValueError('PARENT_PR132_SOURCE_SHA_MISMATCH')
    with zipfile.ZipFile(parent_zip) as z:
        if z.testzip() is not None:raise ValueError('PARENT_ZIP_CORRUPT')
        src=z.read('source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip')
    if hashlib.sha256(src).hexdigest()!=ORIGINAL_SHA:raise ValueError('ORIGINAL_BASS21_SOURCE_SHA_MISMATCH')
    path=pathlib.Path(temp_path)/'original_full25.zip';path.write_bytes(src)
    return path


def angular_moments(deltaI,mu,weights,lvals=LVALS):
    """J_l = int_0^1 deltaI P_l(mu) dmu, exactly the original BASS convention."""
    P=legvander(mu,int(max(lvals)))[:,lvals]
    return np.einsum('nm,ml,m->nl',deltaI,P,weights,optimize=True)


def stress_from_j0_j2(j0,j2):
    """Mixed tensor diag(-rho,P_perp,P_perp,P_parallel), source units."""
    perp=(j0-j2)/3
    para=(j0+2*j2)/3
    rho=j0
    trace=-rho+2*perp+para
    return perp,para,trace


def intensity_direct_stress(deltaI,mu,w):
    rho=deltaI@w
    parallel=(deltaI*mu**2)@w
    perpendicular=(deltaI*(1-mu**2))@w/2
    return rho,perpendicular,parallel


def evaluate_case(h,na=48,no=64,no_ref=96):
    mu,w=gl_half(no);mu2,w2=gl_half(no_ref)
    # Full parent two-field Volterra on original b,g,tau,J2, independent rays.
    sol=solve_case(h,nangular=na,mu_out=mu)
    sol2=solve_case(h,nangular=na,mu_out=mu2)
    J=angular_moments(sol['deltaI'],mu,w)
    J2=angular_moments(sol2['deltaI'],mu2,w2)
    gap=float(np.max(np.abs(J-J2)))
    j0,jquadr,j4=J[:,0],J[:,1],J[:,2]
    perp,para,trace=stress_from_j0_j2(j0,jquadr)
    drho,dp,dl=intensity_direct_stress(sol['deltaI'],mu,w)
    tensor_gap=max(float(np.max(np.abs(drho-j0))),float(np.max(np.abs(dp-perp))),float(np.max(np.abs(dl-para))))
    B=2*(sol['Q']*(1-mu*mu))@w
    x=h['x'];b=h['b'];g=h['g']
    bp=np.gradient(b,x,edge_order=2)
    shearpower=-2*bp*jquadr
    work=cumulative_simpson(shearpower,x=x,initial=0)
    work_gap=j0-work
    deriv_j0=np.gradient(j0,x,edge_order=2)
    deriv_j2=np.gradient(jquadr,x,edge_order=2)
    j2_shear=bp*(-8*j0/5-10*jquadr/7+36*j4/35)
    j2_collision=-g*(9*jquadr/10+3*B/40)
    j2_budget=deriv_j2-j2_shear-j2_collision
    # Parent response must be reproduced at the 17 original half-sky directions.
    sol_orig=solve_case(h,nangular=na,mu_out=MU)
    if np.any(sol_orig['fullI']<=0) or np.any(abs(sol_orig['Q'])>sol_orig['fullI']):
        raise ValueError('PARENT_POLARIZATION_POSITIVITY')
    if np.any(sol['fullI']<=0) or np.any(abs(sol['Q'])>sol['fullI']):
        raise ValueError('CONTINUUM_DIRECTION_POLARIZATION_POSITIVITY')
    if np.any(sol2['fullI']<=0) or np.any(abs(sol2['Q'])>sol2['fullI']):
        raise ValueError('INDEPENDENT_ANGLE_PHYSICALITY_FAIL')
    norm=max(float(np.max(np.abs(j0))),1e-30)
    return dict(J=J,B=B,perp=perp,para=para,work=work,work_gap=work_gap,
                j2_shear=j2_shear,j2_collision=j2_collision,j2_budget=j2_budget,
                geom_energy_power=shearpower,
                max_work_gap=float(np.max(np.abs(work_gap))),
                max_work_relative=float(np.max(np.abs(work_gap))/norm),
                max_j2_budget=float(np.max(np.abs(j2_budget))),
                max_tensor_gap=tensor_gap,max_angular_gap=gap,
                max_trace=float(np.max(np.abs(trace))),
                q_equator=sol_orig['Q'][: ,0],
                Q_over_I_max=float(np.max(np.abs(sol['Q']/sol['fullI']))),
                I_full_min=float(np.min(sol['fullI'])),
                operator_residual=float(sol['fixed_res']),
                source_sha256=h['source_sha256'])


def csv_write(path,rows):
    if not rows:return
    with open(path,'w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)


def run(original_source,out,na=48,no=64,noref=96):
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    histories=load_owner_archive(original_source);cases=sorted(histories);n=len(cases)
    if n!=50:raise ValueError('EXPECTED_50_ORIGINAL_SOURCES')
    moments=np.empty((n,193,6));B=np.empty((n,193));perp=np.empty_like(B);para=np.empty_like(B)
    work=np.empty_like(B);geometric=np.empty_like(B);j2_shear=np.empty_like(B)
    j2_collision=np.empty_like(B);j2_budget=np.empty_like(B);work_gap=np.empty_like(B)
    eqQ=np.empty_like(B);meta=[];rows=[]
    for i,case in enumerate(cases):
        h=histories[case];r=evaluate_case(h,na,no,noref)
        moments[i]=r['J'];B[i]=r['B'];perp[i]=r['perp'];para[i]=r['para']
        work[i]=r['work'];work_gap[i]=r['work_gap'];geometric[i]=r['geom_energy_power']
        j2_shear[i]=r['j2_shear'];j2_collision[i]=r['j2_collision'];j2_budget[i]=r['j2_budget']
        eqQ[i]=r['q_equator']
        rec={k:r[k] for k in ('max_work_gap','max_work_relative','max_j2_budget','max_tensor_gap','max_angular_gap','max_trace','Q_over_I_max','I_full_min','operator_residual','source_sha256')}
        rec['case']=case
        meta.append(rec)
        rows.append(dict(case=case,source_sha256=h['source_sha256'],deltaJ0_z2=float(moments[i,-1,0]),
          deltaJ2_z2=float(moments[i,-1,1]),deltaJ4_z2=float(moments[i,-1,2]),deltaJ6_z2=float(moments[i,-1,3]),
          deltaJ8_z2=float(moments[i,-1,4]),deltaJ10_z2=float(moments[i,-1,5]),
          deltaP_perp_z2=float(perp[i,-1]),deltaP_parallel_z2=float(para[i,-1]),
          delta_rho_work_integral_z2=float(work[i,-1]),absolute_work_defect_z2=float(abs(work_gap[i,-1])),
          max_relative_work_defect=rec['max_work_relative'],max_j2_budget_defect=rec['max_j2_budget']))
    paired=np.empty((25,193,6));pairrows=[];pairwork=np.empty((25,193));pidx=0
    for m in range(5):
      for k in range(5):
        A=cases.index(f's00_m{m:02d}_r{k:02d}');C=cases.index(f's04_m{m:02d}_r{k:02d}')
        paired[pidx]=moments[C]-moments[A]
        pairwork[pidx]=work[C]-work[A]
        pairrows.append(dict(pair=f'm{m:02d}_r{k:02d}',delta_deltaJ0_z2=float(paired[pidx,-1,0]),
            delta_deltaJ2_z2=float(paired[pidx,-1,1]),delta_deltaJ4_z2=float(paired[pidx,-1,2]),
            delta_deltaJ6_z2=float(paired[pidx,-1,3]),delta_deltaJ8_z2=float(paired[pidx,-1,4]),
            delta_deltaJ10_z2=float(paired[pidx,-1,5]),delta_delta_work_z2=float(pairwork[pidx,-1]),
            delta_delta_stress_parallel_z2=float(para[C,-1]-para[A,-1]),
            delta_delta_stress_perp_z2=float(perp[C,-1]-perp[A,-1])))
        pidx+=1
    # Each paired index belongs to the same original BASS source/rate, no cross-source interpolation.
    np.savez_compressed(out/'STRESS_50x193x6_SAME_OWNER.npz',cases=np.array(cases),x=histories[cases[0]]['x'],l_even=LVALS,
         deltaJ_even=moments,BQ=B,deltaP_perp=perp,deltaP_parallel=para,
         deltaJ0_geometric_work=work,energy_balance_gap=work_gap,geometric_power=geometric,
         quadrupole_shear=j2_shear,quadrupole_collision=j2_collision,quadrupole_local_budget_defect=j2_budget,
         equatorial_Q=eqQ,paired_deltaJ_even=paired,paired_delta_work=pairwork,
         pairs=np.array([f'm{m:02d}_r{k:02d}' for m in range(5) for k in range(5)]))
    csv_write(out/'ENDPOINT_50_STRESS.csv',rows)
    csv_write(out/'MATCHED_25_STRESS.csv',pairrows)
    csv_write(out/'FULL50_NUMERIC_AUDIT.csv',meta)
    summary=dict(schema='vigilode.bass21.source_owned_radiation_stress_work.v1',
        parent_sha256=PARENT_SHA,original_sha256=ORIGINAL_SHA,
        cases=n,epochs=193,even_l=list(map(int,LVALS)),new_deltaJ_rows=int(moments.size),
        paired_deltaJ_rows=int(paired.size),new_ivps=0,
        gl_source=na,gl_moments=no,gl_check=noref,
        mean_deltaJ0_end=float(np.mean(moments[:,-1,0])),mean_deltaJ2_end=float(np.mean(moments[:,-1,1])),
        mean_paired_deltaJ0_end=float(np.mean(paired[:,-1,0])),
        mean_paired_deltaJ2_end=float(np.mean(paired[:,-1,1])),
        min_deltaJ0_end=float(np.min(moments[:,-1,0])),max_deltaJ0_end=float(np.max(moments[:,-1,0])),
        min_deltaJ2_end=float(np.min(moments[:,-1,1])),max_deltaJ2_end=float(np.max(moments[:,-1,1])),
        mean_paired_delta_work_end=float(np.mean(pairwork[:,-1])),
        max_angular_moment_difference=float(max(r['max_angular_gap'] for r in meta)),
        max_energy_work_gap=float(max(r['max_work_gap'] for r in meta)),
        max_energy_work_gap_relative=float(max(r['max_work_relative'] for r in meta)),
        max_quadrupole_local_budget_defect=float(max(r['max_j2_budget'] for r in meta)),
        max_tensor_direct_integration_error=float(max(r['max_tensor_gap'] for r in meta)),
        max_trace_error=float(max(r['max_trace'] for r in meta)),
        max_parent_operator_residual=float(max(r['operator_residual'] for r in meta)),
        max_polarization_fraction=float(max(r['Q_over_I_max'] for r in meta)),
        min_full_intensity=float(min(r['I_full_min'] for r in meta)),
        source_identity_50_of_50=True,physical_claim='source-frozen polarized Thomson radiation stress tensor with anisotropic geometric work, not a solved Einstein feedback',
        limitations='no new IVP, no update of original source or electron/Einstein/chemistry background, no observed CMB E/B, no global BDF certificate, no independent-author referee',
        dag={'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor':'0/12','official_delta':0},
        production='HOLD',scope='auxiliary new model-derived stress/work dataset only')
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n',encoding='utf-8')
    return summary


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--parent-zip',type=pathlib.Path,required=True)
    ap.add_argument('--out',type=pathlib.Path,required=True)
    ap.add_argument('--nangular',type=int,default=48)
    ap.add_argument('--nmoment',type=int,default=64)
    ap.add_argument('--ncheck',type=int,default=96)
    args=ap.parse_args()
    with tempfile.TemporaryDirectory(prefix='bass21_stress_') as root:
        source=restore_source(args.parent_zip,root)
        print(json.dumps(run(source,args.out,args.nangular,args.nmoment,args.ncheck),indent=2))
if __name__=='__main__': main()
