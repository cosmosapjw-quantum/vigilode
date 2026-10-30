#!/usr/bin/env python3
"""Immutable BASS21 50x193 exact Thomson eigenmodes and causal shear memory.

This program is *postprocessing of source-frozen BASS21 histories*. It executes
no new Einstein/Boltzmann IVP, no chemistry, thermal, and no self-consistent
source-native E_l hierarchy. It does re-evaluate original coupled Volterra
I/Q fields for the high-l J4 original-source moment, not numerical ODEs.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import json
import pathlib
import sys
from typing import Dict
import numpy as np
from numpy.polynomial.legendre import leggauss
from scipy.integrate import cumulative_trapezoid, cumulative_simpson
from scipy.special import lpmv

HERE=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(HERE/'source_code'))
from coupled_thomson_volterra import load_owner_archive, solve_case

OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_SHA='ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2'
C_EIG=np.array([[.9,.075],[.8,.4]],dtype=float)
LAMBDA_FAST=1.0
LAMBDA_SLOW=.3
DAG={'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor':'0/12','official_delta':0}


def digest(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def half_gl(n):
    mu,w=leggauss(n)
    return .5*(mu+1),.5*w


def source_modes(j2,bq):
    return 8*j2+bq,4*j2-3*bq


def reconstructed_jb(u,v):
    return (3*u+v)/28, (4*u-8*v)/28


def moment_4(I,mu,w):
    p4=(35*mu**4-30*mu**2+3)/8
    return I@(p4*w)


def c4_projection(Q,mu,w):
    p42=lpmv(2,4,mu)
    norm=40. # (l+2)! / ((2l+1)*(l-2)!) at l=4
    return Q@(p42*w)/norm


def projected_source(j0,j2,j4,bq,c4,bprime):
    jshear=bprime*(-8*j0/5-10*j2/7+36*j4/35)
    bshear=bprime*(10*bq/7+32*c4/7)
    return jshear,bshear,8*jshear+bshear,4*jshear-3*bshear


def optical_green(source,tau,x,lam,method='trapezoid'):
    """Driven collision-only Green response on *prescribed* physical sources.

    H(x)=e^-lam*tau(x) integral_0^x e^lam*tau(s) source(s) ds.
    This is NOT total field nor an IVP. Source/collision feedback is frozen.
    """
    f=np.exp(lam*tau)*source
    if method=='trapezoid':integ=cumulative_trapezoid(f,x=x,initial=0)
    elif method=='simpson':integ=cumulative_simpson(f,x=x,initial=0)
    else:raise ValueError('method')
    return np.exp(-lam*tau)*integ


def write_csv(p,rows):
    with open(p,'w',encoding='utf8',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n')
        writer.writeheader();writer.writerows(rows)


def execute(owner,parent,out, n_internal=48,n64=64,n96=96,limit=50):
    owner=pathlib.Path(owner);parent=pathlib.Path(parent);out=pathlib.Path(out)
    assert digest(owner)==OWNER_SHA,'OWNER_SOURCE_BYTE_DRIFT'
    assert digest(parent)==PARENT_SHA,'PARENT_SOURCE_BYTE_DRIFT'
    old=load_owner_archive(owner)
    cases=sorted(old)
    assert len(cases)==50 and 1<=limit<=50
    cases=cases[:limit]
    with np.load(HERE/'source'/'BASS21_RANK9_EHARM_50x193.npz',allow_pickle=False) as z:
        p_cases=z['cases']
        assert np.array_equal(np.array(cases),p_cases[:limit]),'CASE_ORDER_DRIFT'
        parent_data={k:z[k][:limit].copy() for k in ('J0_total','J2_total','BQ_total','c_l','M_anisotropy','j2_collision','BQ_collision')}
        x=z['x'].copy()
    assert len(x)==193
    assert all(np.array_equal(old[k]['x'],x) for k in cases)
    out.mkdir(parents=True,exist_ok=True)
    ng=len(x);shape=(limit,ng)
    names=('g','tau','bprime','J4_total','deltaJ4','U_fast','V_slow','C_U','C_V','S_U_shear','S_V_shear','S_J2_shear','S_BQ_shear','H_U_shear','H_V_shear','H_U_simpson','H_V_simpson','P_U_free','P_V_free','U_green_model','V_green_model','U_dynamic_defect','V_dynamic_defect','U_rate_defect','V_rate_defect','tau_derivative_defect','c4_reproject')
    data={k:np.zeros(shape,dtype=np.float64) for k in names}
    maxerrs={k:0.0 for k in ['parent_J0','parent_J2','parent_BQ','parent_c4','source_J4_GL','source_c4_GL','original_J4_reprojection','anisotropy_V','collision_fast','collision_slow','reconstruct_J2','reconstruct_BQ','original_J4_reprojection','green_integrator_fast','green_integrator_slow','positive_Q']}
    outrows=[]
    mu64,w64=half_gl(n64);mu96,w96=half_gl(n96)
    mu_all=np.r_[mu64,mu96]
    for i,case in enumerate(cases):
        h=old[case]
        assert np.min(h['g'])>=0 and np.all(np.diff(h['tau'])>=-1e-12),'UNPHYSICAL_THOMSON_CLOCK'
        result=solve_case(h,nangular=n_internal,mu_out=mu_all)
        if not np.array_equal(result['x'],x):raise AssertionError('SOURCE_CLOCK_MISMATCH')
        I64,Q64=result['fullI'][:,:n64],result['Q'][:,:n64]
        I96,Q96=result['fullI'][:,n64:],result['Q'][:,n64:]
        # Independently reproject J0/J2/BQ and c4 to establish source ownership.
        j0=I64@w64
        j2=I64@(w64*(3*mu64**2-1)/2)
        bq=2*Q64@(w64*(1-mu64**2))
        c4=c4_projection(Q64,mu64,w64)
        j4=moment_4(I64,mu64,w64)
        j496=moment_4(I96,mu96,w96)
        c496=c4_projection(Q96,mu96,w96)
        maxerrs['parent_J0']=max(maxerrs['parent_J0'],float(np.max(abs(j0-parent_data['J0_total'][i]))))
        maxerrs['parent_J2']=max(maxerrs['parent_J2'],float(np.max(abs(j2-parent_data['J2_total'][i]))))
        maxerrs['parent_BQ']=max(maxerrs['parent_BQ'],float(np.max(abs(bq-parent_data['BQ_total'][i]))))
        maxerrs['parent_c4']=max(maxerrs['parent_c4'],float(np.max(abs(c4-parent_data['c_l'][i,:,1]))))
        maxerrs['source_J4_GL']=max(maxerrs['source_J4_GL'],float(np.max(abs(j4-j496))))
        maxerrs['source_c4_GL']=max(maxerrs['source_c4_GL'],float(np.max(abs(c4-c496))))
        maxerrs['original_J4_reprojection']=max(maxerrs['original_J4_reprojection'],float(np.max(abs(moment_4(result['baseI'][:,:n64],mu64,w64)-h['photon_energy_moments'][:,4]))))
        sourceJ4=h['photon_energy_moments'][:,4]
        delta4=j4-sourceJ4
        # Own frozen source above source case: compare source original baseline J4 quadrature.
        # Original J4 is one component of the frozen 62-state Boltzmann hierarchy.
        data['J4_total'][i]=j4
        data['deltaJ4'][i]=delta4
        g=h['g'];tau=h['tau'];bp=np.gradient(h['b'],x,edge_order=2)
        u,v=source_modes(j2,bq)
        js,bs,su,sv=projected_source(j0,j2,j4,bq,c4,bp)
        cu=8*parent_data['j2_collision'][i]+parent_data['BQ_collision'][i]
        cv=4*parent_data['j2_collision'][i]-3*parent_data['BQ_collision'][i]
        hu=optical_green(su,tau,x,1,'trapezoid')
        hv=optical_green(sv,tau,x,.3,'trapezoid')
        hus=optical_green(su,tau,x,1,'simpson')
        hvs=optical_green(sv,tau,x,.3,'simpson')
        pu=np.exp(-tau)*u[0]
        pv=np.exp(-.3*tau)*v[0]
        gu=pu+hu;gv=pv+hv
        du=np.gradient(u,x,edge_order=2);dv=np.gradient(v,x,edge_order=2)
        maxerrs['collision_fast']=max(maxerrs['collision_fast'],float(np.max(abs(cu+g*u))))
        maxerrs['collision_slow']=max(maxerrs['collision_slow'],float(np.max(abs(cv+.3*g*v))))
        maxerrs['anisotropy_V']=max(maxerrs['anisotropy_V'],float(np.max(abs(parent_data['M_anisotropy'][i]+np.pi*v/2))))
        j2ret,bqret=reconstructed_jb(u,v)
        maxerrs['reconstruct_J2']=max(maxerrs['reconstruct_J2'],float(np.max(abs(j2-j2ret))))
        maxerrs['reconstruct_BQ']=max(maxerrs['reconstruct_BQ'],float(np.max(abs(bq-bqret))))
        maxerrs['green_integrator_fast']=max(maxerrs['green_integrator_fast'],float(np.max(abs(hu-hus))))
        maxerrs['green_integrator_slow']=max(maxerrs['green_integrator_slow'],float(np.max(abs(hv-hvs))))
        maxerrs['positive_Q']=max(maxerrs['positive_Q'],float(np.max(np.abs(Q64)/I64)))
        d={
            'g':g,'tau':tau,'bprime':bp,'U_fast':u,'V_slow':v,'C_U':cu,'C_V':cv,
            'S_U_shear':su,'S_V_shear':sv,'S_J2_shear':js,'S_BQ_shear':bs,
            'H_U_shear':hu,'H_V_shear':hv,'H_U_simpson':hus,'H_V_simpson':hvs,
            'P_U_free':pu,'P_V_free':pv,'U_green_model':gu,'V_green_model':gv,
            'U_dynamic_defect':u-gu,'V_dynamic_defect':v-gv,
            'U_rate_defect':du+g*u-su,'V_rate_defect':dv+.3*g*v-sv,
            'tau_derivative_defect':np.gradient(tau,x,edge_order=2)-g,
            'c4_reproject':c4,
        }
        for k,a in d.items():data[k][i]=a
        outrows.append(dict(case=case,source_sha256=h['source_sha256'],g_final=float(g[-1]),J0_final=float(j0[-1]),J2_final=float(j2[-1]),BQ_final=float(bq[-1]),c4_final=float(c4[-1]),M_anisotropy_final=float(parent_data['M_anisotropy'][i,-1]),U_final=float(u[-1]),V_final=float(v[-1]),
             J4_final=float(j4[-1]),deltaJ4_final=float(delta4[-1]),C_U_final=float(cu[-1]),C_V_final=float(cv[-1]),
             shear_U_final=float(su[-1]),shear_V_final=float(sv[-1]),H_U_final=float(hu[-1]),H_V_final=float(hv[-1]),
             U_model_defect_final=float((u-gu)[-1]),V_model_defect_final=float((v-gv)[-1]),
             free_U_transmission_final=float(np.exp(-tau[-1])),free_V_transmission_final=float(np.exp(-.3*tau[-1])),
             tau_final=float(tau[-1]),max_abs_V_rate_defect=float(np.max(np.abs(dv+.3*g*v-sv))),
             max_abs_V_dynamic_defect=float(np.max(np.abs(v-gv)))))
        if (i+1)%10==0:print(f'SOURCE_EIGENMODE_CASES={i+1}/{limit}',flush=True)
    prereg_ceil=1e-10
    for key in ['parent_J0','parent_J2','parent_BQ','parent_c4','source_J4_GL','source_c4_GL','original_J4_reprojection','anisotropy_V','collision_fast','collision_slow','reconstruct_J2','reconstruct_BQ']:
        if maxerrs[key]>prereg_ceil:raise AssertionError(('PRE_REGISTERED_EXACT_SCOPE_FAIL',key,maxerrs[key],prereg_ceil))
    if maxerrs['positive_Q']>1+1e-10:raise AssertionError('STOKES_POSITIVITY_FAIL')
    pairs=[];index={k:i for i,k in enumerate(cases)}
    if limit==50:
        for m in range(5):
            for r in range(5):
                a=index[f's00_m{m:02d}_r{r:02d}'];b=index[f's04_m{m:02d}_r{r:02d}']
                pairs.append(dict(pair=f'm{m:02d}_r{r:02d}',delta_U_final=float(data['U_fast'][b,-1]-data['U_fast'][a,-1]),
                      delta_V_final=float(data['V_slow'][b,-1]-data['V_slow'][a,-1]),
                      delta_H_U_final=float(data['H_U_shear'][b,-1]-data['H_U_shear'][a,-1]),
                      delta_H_V_final=float(data['H_V_shear'][b,-1]-data['H_V_shear'][a,-1]),
                      delta_U_initial=float(data['U_fast'][b,0]-data['U_fast'][a,0]),
                      delta_V_initial=float(data['V_slow'][b,0]-data['V_slow'][a,0]),
                      delta_P_U_final=float(data['P_U_free'][b,-1]-data['P_U_free'][a,-1]),
                      delta_P_V_final=float(data['P_V_free'][b,-1]-data['P_V_free'][a,-1]),
                      delta_S_V_final=float(data['S_V_shear'][b,-1]-data['S_V_shear'][a,-1]),
                      delta_model_V_defect_final=float(data['V_dynamic_defect'][b,-1]-data['V_dynamic_defect'][a,-1]),
                      delta_model_U_defect_final=float(data['U_dynamic_defect'][b,-1]-data['U_dynamic_defect'][a,-1])))
    bytime=[]
    for ix in range(ng):
        rec={'x':float(x[ix]),'mean_U':float(np.mean(data['U_fast'][:,ix])),'mean_V':float(np.mean(data['V_slow'][:,ix]))}
        for name in ('S_U_shear','S_V_shear','H_U_shear','H_V_shear','U_dynamic_defect','V_dynamic_defect'):
            rec['mean_'+name]=float(np.mean(data[name][:,ix]));rec['max_abs_'+name]=float(np.max(np.abs(data[name][:,ix])))
        bytime.append(rec)
    np.savez_compressed(out/'EIGENMODE_50x193_SOURCE_PHYSICS.npz',cases=np.array(cases),x=x,**data)
    write_csv(out/'EIGENMODE_CASE50_ENDPOINT.csv',outrows)
    write_csv(out/'EIGENMODE_PAIR25_ENDPOINT.csv',pairs if pairs else [outrows[0]])
    write_csv(out/'EIGENMODE_TIME193_MEANS.csv',bytime)
    summary={
        'schema':'vigilode.bass21.original_source_thomson_eigenmode_shear_memory.v1',
        'original_owner_raw_sha256':OWNER_SHA, 'pr141_raw_sha256':PARENT_SHA,
        'source_cases':limit,'saved_epochs':ng,'new_ivps':0,'dynamical_native_E_l':'NOT_RUN',
        'count_modal_U_V':2*limit*ng,'count_shear_source_U_V':2*limit*ng,
        'count_causal_memory_U_V':2*limit*ng,'count_new_total_J4_deltaJ4':2*limit*ng,
        'collision_lambda_exact':['1','3/10'],'max_falsifier_residuals':maxerrs,
        'max_local_dynamics_residual_U':float(np.max(np.abs(data['U_rate_defect']))),
        'max_local_dynamics_residual_V':float(np.max(np.abs(data['V_rate_defect']))),
        'max_integrated_dynamics_defect_U':float(np.max(np.abs(data['U_dynamic_defect']))),
        'max_integrated_dynamics_defect_V':float(np.max(np.abs(data['V_dynamic_defect']))),
        'max_tau_derivative_minus_g':float(np.max(np.abs(data['tau_derivative_defect']))),
        'max_abs_fast_U':float(np.max(np.abs(data['U_fast']))),
        'max_abs_slow_V':float(np.max(np.abs(data['V_slow']))),
        'max_abs_fast_causal_memory':float(np.max(np.abs(data['H_U_shear']))),
        'max_abs_slow_causal_memory':float(np.max(np.abs(data['H_V_shear']))),
        'mean_U_final':float(np.mean(data['U_fast'][:,-1])),'mean_V_final':float(np.mean(data['V_slow'][:,-1])),
        'mean_H_U_final':float(np.mean(data['H_U_shear'][:,-1])),'mean_H_V_final':float(np.mean(data['H_V_shear'][:,-1])),
        'min_fast_transmission':float(np.min(np.exp(-data['tau'][:,-1]))),
        'max_fast_transmission':float(np.max(np.exp(-data['tau'][:,-1]))),
        'min_slow_transmission':float(np.min(np.exp(-.3*data['tau'][:,-1]))),
        'max_slow_transmission':float(np.max(np.exp(-.3*data['tau'][:,-1]))),
        'mean_pair_delta_U_final':float(np.mean([p['delta_U_final'] for p in pairs])) if pairs else None,
        'mean_pair_delta_V_final':float(np.mean([p['delta_V_final'] for p in pairs])) if pairs else None,
        'mean_pair_delta_H_U_final':float(np.mean([p['delta_H_U_final'] for p in pairs])) if pairs else None,
        'mean_pair_delta_H_V_final':float(np.mean([p['delta_H_V_final'] for p in pairs])) if pairs else None,
        'max_abs_pair_initial_fast_difference':float(max(abs(p['delta_U_initial']) for p in pairs)) if pairs else None,
        'max_abs_pair_initial_slow_difference':float(max(abs(p['delta_V_initial']) for p in pairs)) if pairs else None,
        'max_abs_pair_green_defect_U':float(max(abs(p['delta_U_final']-p['delta_P_U_final']-p['delta_H_U_final']) for p in pairs)) if pairs else None,
        'max_abs_pair_green_defect_V':float(max(abs(p['delta_V_final']-p['delta_P_V_final']-p['delta_H_V_final']) for p in pairs)) if pairs else None,
        'source_sha256_by_case':{k:old[k]['source_sha256'] for k in cases},
        'dag':DAG,'production':'HOLD','distinct_author_R6_C9':'NOT_RUN',
        'physics_scope':'Exact local collision eigenmodes, shear source and collision-only Green memory evaluated on original frozen BASS21 photon Volterra sky, not actual new dynamic source-native E_l evolution.',
        'model_defect_scope':'Derivative and Duhamel defects are diagnostics of parent frozen/approximated dynamics and sampled grid, NOT rigorous temporal error or discovered violation of full kinetic theory.'
    }
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,ensure_ascii=False,sort_keys=True,indent=2,allow_nan=False)+'\n')
    alg={
      'source_collision_matrix':[ ['9/10','3/40'],['4/5','2/5'] ],'eigenvalues':['1','3/10'],
      'eigenmodes':{'U':'8 J2+BQ','V':'4 J2-3 BQ','J2':'(3U+V)/28','BQ':'(4U-8V)/28'},
      'collision_modes':{'C_U':'-g U','C_V':'-(3/10)g V'},
      'coherency_relation':'Mparallel-Mperp=-(pi/2)V',
      'shear_J2':'bprime*(-8 J0/5 - 10 J2/7 + 36 J4/35)',
      'shear_BQ':'bprime*(10 BQ/7 + 32 c4/7)',
      'causal_memory_fast':'exp(-tau) integral_0^x exp(tau(s)) S_U_shear(s) ds',
      'causal_memory_slow':'exp(-3tau/10) integral_0^x exp(3tau(s)/10) S_V_shear(s) ds',
      'time_coordinate':'x=ln(a/a_ref), tau(x) from original BASS owner, g=c sigma_T ne/H',
      'basis':'Stokes Q meridional-azimuthal; no unqualified full observer E_l'
    }
    (out/'EXACT_ALGEBRA.json').write_text(json.dumps(alg,sort_keys=True,indent=2,allow_nan=False)+'\n')
    print('FINAL_SUMMARY '+json.dumps({k:v for k,v in summary.items() if k not in ('source_sha256_by_case','max_falsifier_residuals')},sort_keys=True),flush=True)
    print('VALIDATION '+json.dumps(maxerrs,sort_keys=True),flush=True)
    return summary


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--owner',required=True);parser.add_argument('--parent',required=True)
    parser.add_argument('--out',required=True);parser.add_argument('--limit',type=int,default=50)
    args=parser.parse_args()
    execute(args.owner,args.parent,args.out,limit=args.limit)
