#!/usr/bin/env python3
"""BASS21 source-owned E2 collision crosswalk and higher spin-2 mode/shear atlas.

Only a spin-2 basis coefficient and a collision-level E2 normalization are derived.
No identification with BASS-native evolving polarization hierarchy, no new IVPs.
Source: frozen original 50 62-state BASS21 histories, 193 epochs.
"""
from __future__ import annotations
import argparse, csv, hashlib, json, math, pathlib, sys, zipfile
import numpy as np
from scipy.special import lpmv
from numpy.polynomial.legendre import leggauss
from scipy.integrate import cumulative_simpson

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'source_code'))
from coupled_thomson_volterra import solve_case,gl_half,load_owner_archive

PARENT_SHA='ffd7623ba2e2bd99233a1244ac7945470f6eeed348a63a4fd02f5967ad449b14'
SOURCE_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
L_EVEN=(2,4,6,8,10)
C2_TO_E2=12/(5*np.sqrt(6.)) # c2=A/3 for Q=A(1-mu^2)
BQ_TO_E2=3/(4*np.sqrt(6.))


def hash_file(path):
    with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def restore_owner(parent, dest):
    if hash_file(parent)!=PARENT_SHA:raise ValueError('PARENT_PR133_ARCHIVE_SHA_DRIFT')
    with zipfile.ZipFile(parent) as z:
        if z.testzip() is not None:raise ValueError('PARENT_PR133_ZIP_CRC_FAIL')
        inner=z.read('parent/VIGILODE_BASS21_COUPLED_THOMSON_20261030_FULL_SOURCE.zip')
    with zipfile.ZipFile(__import__('io').BytesIO(inner)) as z:
        raw=z.read('source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip')
    if hashlib.sha256(raw).hexdigest()!=SOURCE_SHA:raise ValueError('ORIGINAL_BASS21_SOURCE_DRIFT')
    dest=pathlib.Path(dest);dest.write_bytes(raw)
    return dest


def p2(mu):
    return .5*(3*mu**2-1)


def spin2_projection(Q,mu,w):
    """Q even full sky, positive spin-2 associated Legendre (m=2), half-sky."""
    P=np.stack([lpmv(2,l,mu) for l in L_EVEN],axis=-1)
    N=np.array([math.factorial(l+2)/((2*l+1)*math.factorial(l-2)) for l in L_EVEN])
    return np.einsum('tm,ml,m->tl',Q,P,w,optimize=True)/N


def B_and_C(Q,mu,w):
    B=2*(Q*(1-mu**2))@w
    C=2*(Q*(mu**2)*(1-mu**2))@w
    return B,C


def analyze(h,source_n=48,out_n=64,check_n=96):
    m,w=gl_half(out_n);m2,w2=gl_half(check_n)
    # One identical parent frozen Volterra equation, output on two independent meshes.
    # Passing 160 directions in a single solve avoids source-regeneration drift.
    sol=solve_case(h,nangular=source_n,mu_out=np.r_[m,m2])
    q1=sol['Q'][:,:out_n];q2=sol['Q'][:,out_n:]
    di1=sol['deltaI'][:,:out_n]
    c=spin2_projection(q1,m,w)
    cref=spin2_projection(q2,m2,w2)
    B,C=B_and_C(q1,m,w)
    B2,C2=B_and_C(q2,m2,w2)
    E=BQ_TO_E2*B
    j2=(di1*p2(m))@w
    g=h['g'];b=h['b'];x=h['x'];j2orig=h['photon_energy_moments'][:,2]
    bp=np.gradient(b,x,edge_order=2)
    # Correct source-owned *collision* E2 identity, up to sign and normalization.
    j2feedback=-g*(np.sqrt(6.)/10.)*E
    j2feedback_B=-g*(3./40.)*B
    e2collision=-g*((np.sqrt(6.)/10.)*(j2orig+j2)+(2./5.)*E)
    e2shear=bp*((10./7.)*E+(4*np.sqrt(6.)/7.)*c[:,1])
    # Exact C=B/7+32c4/21 identity holds for arbitrary even Q.
    Ccrosswalk=B/7.+(32./21.)*c[:,1]
    e2shear_direct=bp*BQ_TO_E2*(B+3*C)
    e2rate=e2shear+e2collision
    e2integral=cumulative_simpson(e2rate,x=x,initial=0)
    numerical_res=E-e2integral
    dev=np.gradient(E,x,edge_order=2)-e2rate
    # Independent angular spin2 result must reproduce j2feedback through bq.
    source_E2_no_c2_delta=-g*np.sqrt(6.)/10.*E
    # Original optical clock g nonnegative, and Q<=physical I.
    if (np.any(sol['fullI']<=0) or np.any(np.abs(sol['Q'])>sol['fullI'])
            or np.min(g)<0 or not np.all(np.isfinite(c))):
        raise ValueError('INVALID_PHOTON_COHERENCY_OR_CLOCK')
    if abs(E[0])>1e-16 or not np.allclose(sol['Q'][0],0,atol=0):
        raise ValueError('NONZERO_INITIAL_POLARIZATION')
    return dict(coeff=c,coeff_ref=cref,B=B,C=C,E2=E,j2=j2,
                j2feedback=j2feedback,j2feedback_B=j2feedback_B,
                E2_shear=e2shear,E2_scatter=e2collision,E2_work=e2integral,
                E2_work_gap=numerical_res,E2_local_budget=dev,
                fullQ=sol['Q'],max_spin_gap=float(np.max(np.abs(c-cref))),
                max_B_gap=float(np.max(np.abs(B-B2))),
                max_C_gap=float(np.max(np.abs(C-C2))),
                max_projection_E2_gap=float(np.max(np.abs(E-C2_TO_E2*c[:,0]))),
                max_C_crosswalk_gap=float(np.max(np.abs(C-Ccrosswalk))),
                max_rate_crosswalk_gap=float(np.max(np.abs(e2shear-e2shear_direct))),
                max_j2feedback_crosswalk=float(np.max(np.abs(j2feedback-j2feedback_B))),
                max_work_gap=float(np.max(np.abs(numerical_res))),
                max_local_budget=float(np.max(np.abs(dev))),
                fixed_res=sol['fixed_res'],max_QoverI=float(np.max(np.abs(sol['Q']/sol['fullI']))),
                positive_I=float(np.min(sol['fullI'])),source_sha=h['source_sha256'])


def csvwrite(path, rows):
    if not rows:return
    with open(path,'w',encoding='utf-8',newline='') as f:
        q=csv.DictWriter(f,fieldnames=rows[0].keys(),lineterminator='\n')
        q.writeheader();q.writerows(rows)


def run(owner,out,source_n=48,out_n=64,check_n=96):
    a=load_owner_archive(owner); cases=sorted(a)
    if len(cases)!=50:raise ValueError('NOT_ORIGINAL_50_CASES')
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    nt=len(a[cases[0]]['x']);assert nt==193
    coeff=np.empty((50,193,5));B=np.empty((50,193));C=np.empty_like(B);E=np.empty_like(B)
    j2feedback=np.empty_like(B);E2shear=np.empty_like(B);E2coll=np.empty_like(B);j2=np.empty_like(B)
    egap=np.empty_like(B);ebudget=np.empty_like(B);eint=np.empty_like(B)
    meta=[];endpoint=[]
    for i,key in enumerate(cases):
        h=a[key];r=analyze(h,source_n,out_n,check_n)
        coeff[i]=r['coeff'];B[i]=r['B'];C[i]=r['C'];E[i]=r['E2'];j2[i]=r['j2'];
        j2feedback[i]=r['j2feedback'];E2shear[i]=r['E2_shear'];E2coll[i]=r['E2_scatter'];
        egap[i]=r['E2_work_gap'];ebudget[i]=r['E2_local_budget'];eint[i]=r['E2_work'];
        record={k:r[k] for k in ('max_spin_gap','max_B_gap','max_C_gap','max_projection_E2_gap','max_C_crosswalk_gap','max_rate_crosswalk_gap','max_j2feedback_crosswalk','max_work_gap','max_local_budget','fixed_res','max_QoverI','positive_I','source_sha')};record['case']=key
        meta.append(record)
        endpoint.append({'case':key,'source_sha256':h['source_sha256'],'E2_final':float(E[i,-1]),'c4_final':float(coeff[i,-1,1]),
          'c6_final':float(coeff[i,-1,2]),'c8_final':float(coeff[i,-1,3]),'c10_final':float(coeff[i,-1,4]),
          'dJ2dx_pol_feedback_final':float(j2feedback[i,-1]),'E2_shear_rate_final':float(E2shear[i,-1]),
          'E2_collision_rate_final':float(E2coll[i,-1]),'B_Q_final':float(B[i,-1]),'E2_work_gap_final':float(egap[i,-1])})
    pairs=[];paired=np.empty((25,193,5));pairedE=np.empty((25,193));pairedJ2fb=np.empty_like(pairedE)
    for m in range(5):
      for k in range(5):
        p=5*m+k;ia=cases.index(f's00_m{m:02d}_r{k:02d}');ib=cases.index(f's04_m{m:02d}_r{k:02d}')
        paired[p]=coeff[ib]-coeff[ia];pairedE[p]=E[ib]-E[ia];pairedJ2fb[p]=j2feedback[ib]-j2feedback[ia]
        pairs.append(dict(pair=f'm{m:02d}_r{k:02d}',paired_E2_final=float(pairedE[p,-1]),paired_c4_final=float(paired[p,-1,1]),
          paired_E2_collision_on_J2_final=float(pairedJ2fb[p,-1])))
    np.savez_compressed(out/'BASS21_E2_SPIN2_50x193x5.npz',cases=np.asarray(cases),x=a[cases[0]]['x'],l_even=np.asarray(L_EVEN),
      spin2_c_even=coeff,B_Q=B,C_Q=C,E2_candidate=E,delta_j2=j2,j2_polarized_collision_feedback=j2feedback,
      E2_shear_rate=E2shear,E2_collision_rate=E2coll,E2_integrated_rate=eint,E2_work_gap=egap,
      E2_local_budget=ebudget,paired_spin2_coeff=paired,paired_E2=pairedE,paired_collision_feedback=pairedJ2fb,
      pairs=np.asarray([z['pair'] for z in pairs]))
    csvwrite(out/'ENDPOINT_50_E2_SPIN2.csv',endpoint)
    csvwrite(out/'PAIRED_25_E2_SPIN2.csv',pairs)
    csvwrite(out/'NUMERIC_AUDIT_50.csv',meta)
    summary=dict(schema='vigilode.bass21.e2_spin2_shear.v1',original_50=True,saved_epochs=193,source_owner='cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c',
      parent_archive_sha256=PARENT_SHA,original_source_sha256=SOURCE_SHA,spin2_l=list(L_EVEN),new_spin2_mode_entries=int(coeff.size),paired_mode_entries=int(paired.size),new_E2_entries=int(E.size),paired_E2_entries=int(pairedE.size),new_ivps=0,
      mean_E2_end=float(E[:,-1].mean()),mean_c4_end=float(coeff[:,-1,1].mean()),mean_paired_E2_end=float(pairedE[:,-1].mean()),
      mean_J2_polarized_collision_feedback_end=float(j2feedback[:,-1].mean()),mean_paired_J2_polarized_collision_feedback_end=float(pairedJ2fb[:,-1].mean()),
      s00_max_c4=float(np.max(np.abs(coeff[:25,:,1]))),s04_max_c4=float(np.max(np.abs(coeff[25:,:,1]))),
      max_projection_E2_gap=float(max(z['max_projection_E2_gap'] for z in meta)),max_C_crosswalk_gap=float(max(z['max_C_crosswalk_gap'] for z in meta)),
      max_j2feedback_crosswalk=float(max(z['max_j2feedback_crosswalk'] for z in meta)),max_rate_crosswalk_gap=float(max(z['max_rate_crosswalk_gap'] for z in meta)),
      max_spin2_angular_quadrature_gap=float(max(z['max_spin_gap'] for z in meta)),max_E2_rate_integral_gap=float(max(z['max_work_gap'] for z in meta)),
      max_E2_derivative_budget_gap=float(max(z['max_local_budget'] for z in meta)),max_parent_fixed_operator_residual=float(max(z['fixed_res'] for z in meta)),
      max_polarization_fraction=float(max(z['max_QoverI'] for z in meta)),
      physics='model-frame cold Thomson E2 collision crosswalk, spin2 mode and anisotropic shear transfer on source-frozen BASS21, not full E_l hierarchy',
      dag={'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor':'0/12','official_delta':0},production='HOLD',distinct_author_R6='NOT_RUN',distinct_author_C9='NOT_RUN',
      note='Numerical derivatives on original 193 saved epochs are diagnostic only; not BDF or full source-native evolving E_l certificate')
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,sort_keys=True,indent=2,allow_nan=False)+'\n',encoding='utf-8')
    return summary


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--source',required=True);ap.add_argument('--out',required=True)
    ap.add_argument('--source-angular',type=int,default=48);ap.add_argument('--angular',type=int,default=64);ap.add_argument('--angular-check',type=int,default=96)
    z=ap.parse_args();print(json.dumps(run(z.source,z.out,z.source_angular,z.angular,z.angular_check),sort_keys=True,indent=2))
if __name__=='__main__':main()
