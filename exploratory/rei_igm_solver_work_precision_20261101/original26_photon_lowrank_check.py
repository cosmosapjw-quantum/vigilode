#!/usr/bin/env python3
"""Source-exact real REI original 46k-cell photo-suboperator rank-three JVP check.

This is a conditional photon SOURCE evaluation (not a complete all-X D_h
certificate, not an alternative 26-event time integrator). The frozen original
26-event kernel and accepted state are imported read-only, never overwritten.
"""
from __future__ import annotations
import argparse, contextlib, hashlib, importlib.util, json, math, sys, time
from pathlib import Path
import numpy as np

REI_F05_PATH='research/rei_f05_common_parent_increments_20261101/increment_certificate.py'
SOURCE_SHA='a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b'
KERNEL_SHA='579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644'
STATE_SHA='cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01'
N=46080

def load(name,p):
    spec=importlib.util.spec_from_file_location(name,p)
    if spec is None or spec.loader is None:raise ImportError(p)
    mod=importlib.util.module_from_spec(spec);sys.modules[name]=mod;spec.loader.exec_module(mod)
    return mod

def midpoint(x):
    return .5*(np.asarray(x.lo,dtype=np.float64)+np.asarray(x.hi,dtype=np.float64))

def original_photons(n,j,e,alpha,exc,ev):
    """Exactly original reduced_interval_rhs._explicit_photo_fields' point algebra.
    Input n is [node, HI/HeI/HeII] and all source denominators are global.
    """
    totals=np.sum(n,axis=0)
    photons=np.zeros_like(n)
    heat=np.zeros(n.shape[0]); external=0.
    for g in range(4):
        den=e[g]+float(alpha[g]@totals)
        if not (math.isfinite(den) and den>0.):raise ArithmeticError('NONPOSITIVE_PHOTON_DENOMINATOR')
        fg=j[g]*(n*alpha[g][None,:])/den
        photons+=fg
        heat+=np.sum(fg*(exc[:,g][None,:]*ev),axis=1)
        external+=j[g]*e[g]/den
    return photons,heat,external

def lowrank_jvp(n,v,j,e,alpha):
    """f'(n)@v = diagonal_local(n)@v + U(n) (V^T v), rank(V) <= 3."""
    totals=np.sum(n,axis=0)
    moment_delta=np.sum(v,axis=0)
    out=np.zeros_like(n)
    for g in range(4):
        den=e[g]+float(alpha[g]@totals)
        fg=j[g]*(n*alpha[g][None,:])/den
        out+=j[g]*(v*alpha[g][None,:])/den-fg*(float(alpha[g]@moment_delta)/den)
    return out

def compute(owner:Path,lane:str,epsilon:float):
    if not (owner/'stages').is_dir():raise FileNotFoundError('original REI stages missing')
    sys.path.insert(0,str(owner))
    f05=load('rei_original26_f05_readonly',owner/REI_F05_PATH)
    edge=f05.load('rei_original26_edge_readonly',owner/'research/rei_igm_original26_temperature_edges_20261101/edge_probe.py')
    stage=owner/edge.REL_STAGE
    worker=f05.load('rei_original26_worker_readonly',stage/'analysis/attempt_worker.py')
    worker.jax_guard.install_if_missing()
    kernel=worker._kernel()
    if f05.sha(worker.KERNEL_PATH)!=KERNEL_SHA:raise ArithmeticError('ORIGINAL_26_EVENT_KERNEL_DRIFT')
    decoded,source=edge.source_state(stage,lane,worker.state_io)
    if source['archive_sha256']!=SOURCE_SHA or source['accepted_array_sha256']!=STATE_SHA:
        raise ArithmeticError('ORIGINAL_SOURCE_PARENT_DRIFT')
    _base,_,_,nh,nhe=worker._context(kernel,lane)
    if len(nh)!=N:raise ValueError('ORIGINAL_NODES_NOT_46080')
    original=kernel.rim.ReducedIntervalModel.from_repo(owner)
    duration=float(_base.forcing.duration_seconds(0));t160=duration*160/131072
    forcing=original.forcing_bounds(t160,t160)
    p=np.asarray((decoded.population_lower+decoded.population_upper)*.5,dtype=float)
    if p.shape!=(N,5) or np.any(~np.isfinite(p)) or np.any(p<=0.):
        raise ValueError('FROZEN_PARENT_INVALID')
    n=p[:,[0,2,3]].copy()
    z=float(midpoint(forcing.z));scale=kernel.rim.NH0_CM3*(1+z)**2*kernel.rim.MPC_CM
    j=np.array([float(midpoint(x)) for x in forcing.current]); e=np.array([float(midpoint(x)) for x in forcing.external_subgrid])
    alpha=np.zeros((4,3));exc=np.asarray(original.excess_eV)
    for g in range(4):
        alpha[g,0]=scale*(original.inputs.sigma_cm2[0,g] if original.inputs.owner_support[1,g] else 0.)/original.n_h_total
        alpha[g,1]=kernel.rim.YHE*scale*(original.inputs.sigma_cm2[1,g] if original.inputs.owner_support[2,g] else 0.)/original.n_he_total
        alpha[g,2]=kernel.rim.YHE*scale*(original.inputs.sigma_cm2[2,g] if original.inputs.owner_support[3,g] else 0.)/original.n_h_total
    if not(np.all(np.isfinite(alpha)) and np.all(alpha>=0.) and np.all(j>=0.) and np.all(e>=0.)):
        raise ValueError('INVALID_OWNER_PHOTON_SOURCES')
    # Exactly same original 46k-cell physical parent and original stage-0 current.
    current,heat,external=original_photons(n,j,e,alpha,exc,kernel.rim.EV_ERG)
    v=n*np.cos((np.arange(N)[:,None]+1)*np.array([0.101,0.203,0.307])[None,:])
    jv=lowrank_jvp(n,v,j,e,alpha)
    plus,_heat,_ext=original_photons(n+epsilon*v,j,e,alpha,exc,kernel.rim.EV_ERG)
    minus,_heat,_ext=original_photons(n-epsilon*v,j,e,alpha,exc,kernel.rim.EV_ERG)
    fd=(plus-minus)/(2*epsilon)
    scale_ref=max(float(np.max(np.abs(jv))),1e-300)
    max_rel=float(np.max(np.abs(jv-fd))/scale_ref)
    max_abs=float(np.max(np.abs(jv-fd)))
    # Independent original owner interval source-code inclusion, not just our algebra.
    pops=tuple(kernel._I(p[:,s],p[:,s]) for s in range(5))
    owner_hi,owner_hei,owner_heii,owner_heat=original._explicit_photo_fields(pops,forcing)
    owner_interval_inclusions=[]
    for s,box in enumerate((owner_hi,owner_hei,owner_heii)):
        lo=np.asarray(box.lo);hi=np.asarray(box.hi)
        inside=(lo<=current[:,s]) & (current[:,s]<=hi)
        owner_interval_inclusions.append(int(inside.sum()))
    heat_inside=(np.asarray(owner_heat.lo)<=heat)&(heat<=np.asarray(owner_heat.hi))
    emitted=float(np.sum(j));absorbed=float(np.sum(current));residual=emitted-absorbed-external
    photon_relative_residual=abs(residual)/max(abs(emitted),1e-300)
    return {
      'schema':'REI_ORIGINAL_26EVENT_KERNEL_46080_PHOTON_SOURCE_LOWRANK_JVP_V1',
      'frozen_original_archive_sha256':SOURCE_SHA,
      'original_26event_kernel_sha256':KERNEL_SHA,
      'original_accepted_array_sha256':STATE_SHA,
      'lane':lane,'parent_tick':'160/131072','nodes':N,'photon_groups':4,
      'shared_moments':3,'nonlocal_spatial_rank_upper':3,
      'source_stage':'original full26-event stage0 partial photo operator only',
      'parent_intervals_used':'midpoint diagnostic, not whole all-X certificate',
      'time_source':'original forcing_bounds(t160,t160)',
      'owner_interval_included_cell_species':owner_interval_inclusions,
      'owner_interval_included_heat_cells':int(heat_inside.sum()),
      'finite_difference_step':epsilon,
      'jvp_finite_difference_normalized_max_error':max_rel,
      'jvp_finite_difference_max_abs_error':max_abs,
      'photon_ledger_relative_residual':photon_relative_residual,
      'source_photoelectron_particles_per_s':absorbed,
      'source_primary_photoheat_erg_per_s':float(np.sum(heat)),
      'source_owner_comparison_pass':all(x==N for x in owner_interval_inclusions) and int(heat_inside.sum())==N and max_rel<1e-6 and photon_relative_residual<1e-12,
      'full26event_integration':'NOT_RUN',
      'full_dh_jacobian':'NOT_RUN',
      'allX_uniform_remainder':'NOT_RUN',
      'canonical_delta':0,'production':'HOLD',
    }

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--owner-root',type=Path,required=True)
    ap.add_argument('--lane',default='LOCAL_NEUTRAL_HAZARD_PRIMARY')
    ap.add_argument('--epsilon',type=float,default=0.0002)
    ap.add_argument('--out',type=Path,required=True);args=ap.parse_args()
    if not(0<args.epsilon<0.01):raise ValueError('INVALID_PREDECLARED_EPSILON')
    result=compute(args.owner_root,args.lane,args.epsilon)
    args.out.parent.mkdir(parents=True,exist_ok=True)
    args.out.write_text(json.dumps(result,indent=2,sort_keys=True,allow_nan=False)+'\n')
    print(json.dumps({k:result[k] for k in ['nodes','source_owner_comparison_pass','jvp_finite_difference_normalized_max_error','owner_interval_included_cell_species','owner_interval_included_heat_cells','photon_ledger_relative_residual']}))
    if not result['source_owner_comparison_pass']:raise SystemExit(2)
if __name__=='__main__':main()
