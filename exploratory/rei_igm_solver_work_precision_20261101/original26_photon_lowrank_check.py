#!/usr/bin/env python3
"""Source-exact real REI original 46k-cell photo-suboperator rank-three JVP check.

This is a conditional photon SOURCE evaluation (not a complete all-X D_h
certificate, not an alternative 26-event time integrator). The frozen original
26-event kernel and accepted state are imported read-only, never overwritten.
"""
from __future__ import annotations
import argparse, importlib.util, json, math, sys
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

def hybrid_source(n,j,e,alpha_extended,exc,ev):
    """Three precise global sums and four group coefficients; local arrays f64.

    Pure diagnostic implementation, NOT a zero-allocation runtime path.
    This path was motivated by an actual strict owner-enclosure violation in
    the initial all-f64 reordered-source experiment, which is retained.
    """
    ld=np.longdouble
    totals=np.sum(n.astype(ld),axis=0)
    photons=np.zeros_like(n);heat=np.zeros(n.shape[0]);external=0.
    for g in range(4):
        den=ld(e[g])+np.dot(alpha_extended[g],totals)
        if not(np.isfinite(den) and den>0):raise ArithmeticError('NONPOSITIVE_HYBRID_DENOMINATOR')
        for s in range(3):
            local_scalar=float(ld(j[g])*alpha_extended[g,s]/den)
            node=local_scalar*n[:,s]
            photons[:,s]+=node
            heat+=node*(exc[s,g]*ev)
        external+=float(ld(j[g])*ld(e[g])/den)
    return photons,heat,external

def hybrid_lowrank_jvp(n,v,j,e,alpha_extended):
    """Source-fixed exact derivative of the reduced real-arithmetic source law.
    Only three global reductions for nonlocal action; no 46k-by-46k Jacobian.
    """
    ld=np.longdouble
    totals=np.sum(n.astype(ld),axis=0)
    moment_delta=np.sum(v.astype(ld),axis=0)
    out=np.zeros_like(n)
    for g in range(4):
        den=ld(e[g])+np.dot(alpha_extended[g],totals)
        common=float(np.dot(alpha_extended[g],moment_delta)/den)
        for s in range(3):
            loc=float(ld(j[g])*alpha_extended[g,s]/den)
            out[:,s]+=loc*v[:,s]-loc*n[:,s]*common
    return out


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
    base,_,_,nh,nhe=worker._context(kernel,lane)
    if len(nh)!=N:raise ValueError('ORIGINAL_NODES_NOT_46080')
    original=kernel.rim.ReducedIntervalModel.from_repo(owner)
    duration=float(base.forcing.duration_seconds(0)); t160=duration*160/131072
    forcing=original.forcing_bounds(t160,t160)
    p=np.asarray((decoded.population_lower+decoded.population_upper)*0.5,dtype=float)
    if p.shape!=(N,5) or np.any(~np.isfinite(p)) or np.any(p<=0.):
        raise ValueError('FROZEN_PARENT_INVALID')
    n=p[:,[0,2,3]].copy()
    z=float(midpoint(forcing.z))
    j=np.array([float(midpoint(x)) for x in forcing.current])
    e=np.array([float(midpoint(x)) for x in forcing.external_subgrid])
    ld=np.longdouble
    scale=kernel.rim.NH0_CM3*(1.+z)**2*kernel.rim.MPC_CM
    scale_extended=ld(kernel.rim.NH0_CM3)*(ld(1)+ld(z))**2*ld(kernel.rim.MPC_CM)
    alpha=np.zeros((4,3));alpha_extended=np.zeros((4,3),dtype=ld)
    denominators=(original.n_h_total,original.n_he_total,original.n_h_total)
    source_yhe=(1.,kernel.rim.YHE,kernel.rim.YHE)
    for g in range(4):
        for s in range(3):
            sig=original.inputs.sigma_cm2[s,g] if original.inputs.owner_support[s+1,g] else 0.
            alpha[g,s]=float(source_yhe[s]*scale*sig/denominators[s])
            alpha_extended[g,s]=ld(source_yhe[s])*scale_extended*ld(sig)/ld(denominators[s])
    if not(np.all(np.isfinite(alpha)) and np.all(alpha>=0.) and np.all(j>=0.) and np.all(e>=0.)):
        raise ValueError('INVALID_OWNER_PHOTON_SOURCES')
    # Negative control: unguarded float64 algebraic coefficient coalescing.
    raw_current,raw_heat,_=original_photons(n,j,e,alpha,original.excess_eV,kernel.rim.EV_ERG)
    # Candidate: high-precision global moments/coefficient; f64 local 46,080 cells.
    current,heat,external=hybrid_source(n,j,e,alpha_extended,original.excess_eV,kernel.rim.EV_ERG)
    # Long-double trace-species diagnostic: a physically nonzero original
    # HeII source may lie below binary64's smallest subnormal. Never treat
    # such a floating-point zero as proof of an exact zero photo rate.
    source_totals_ld=np.sum(n.astype(ld),axis=0)
    heii_trace_ld=np.zeros(N,dtype=ld)
    for g in range(4):
        den_ld=ld(e[g])+np.dot(alpha_extended[g],source_totals_ld)
        heii_trace_ld+=ld(j[g])*alpha_extended[g,2]*n[:,2].astype(ld)/den_ld
    heii_nonzero_extended=int(np.count_nonzero(heii_trace_ld>0))
    heii_f64_lost=int(np.count_nonzero((heii_trace_ld>0)&(current[:,2]==0)))
    heii_integrated_ld=np.sum(heii_trace_ld,dtype=ld)
    v=n*np.cos((np.arange(N)[:,None]+1)*np.array([0.101,0.203,0.307])[None,:])
    jvp=hybrid_lowrank_jvp(n,v,j,e,alpha_extended)
    plus,_,_=hybrid_source(n+epsilon*v,j,e,alpha_extended,original.excess_eV,kernel.rim.EV_ERG)
    minus,_,_=hybrid_source(n-epsilon*v,j,e,alpha_extended,original.excess_eV,kernel.rim.EV_ERG)
    fd=(plus-minus)/(2*epsilon)
    jvp_scale=max(float(np.max(np.abs(jvp))),1e-300)
    max_rel=float(np.max(np.abs(jvp-fd))/jvp_scale)
    max_abs=float(np.max(np.abs(jvp-fd)))
    # Original unchanged owner interval source, including heat and all three species.
    pops=tuple(kernel._I(p[:,s],p[:,s]) for s in range(5))
    owner_hi,owner_hei,owner_heii,owner_heat=original._explicit_photo_fields(pops,forcing)
    included=[];raw_included=[]
    for s,box in enumerate((owner_hi,owner_hei,owner_heii)):
        lo=np.asarray(box.lo);hi=np.asarray(box.hi)
        included.append(int(((lo<=current[:,s])&(current[:,s]<=hi)).sum()))
        raw_included.append(int(((lo<=raw_current[:,s])&(raw_current[:,s]<=hi)).sum()))
    heat_inside=(np.asarray(owner_heat.lo)<=heat)&(heat<=np.asarray(owner_heat.hi))
    raw_heat_inside=(np.asarray(owner_heat.lo)<=raw_heat)&(raw_heat<=np.asarray(owner_heat.hi))
    emitted=float(np.sum(j));absorbed=float(np.sum(current))
    budget=abs(emitted-absorbed-external)/max(abs(emitted),1e-300)
    pass_gate=(all(x==N for x in included) and int(heat_inside.sum())==N
        and max_rel<1e-6 and budget<1e-12)
    return {
      'schema':'REI_ORIGINAL_26EVENT_KERNEL_46080_PHOTON_SOURCE_LOWRANK_JVP_V2',
      'original_owner_head':'fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120',
      'frozen_original_archive_sha256':SOURCE_SHA,
      'original_26event_kernel_sha256':KERNEL_SHA,
      'original_accepted_array_sha256':STATE_SHA,
      'lane':lane,'parent_tick':'160/131072','nodes':N,'photon_groups':4,
      'shared_moments':3,'nonlocal_spatial_rank_upper':3,
      'source_stage':'original 26-event stage-0 photon partial operator only',
      'parent_intervals_used':'midpoint diagnostic, not whole all-X certificate',
      'time_source':'original forcing_bounds(t160,t160)',
      'hybrid_owner_interval_included_cell_species':included,
      'hybrid_owner_interval_included_heat_cells':int(heat_inside.sum()),
      'original_unguarded_f64_interval_included_cell_species':raw_included,
      'original_unguarded_f64_interval_included_heat_cells':int(raw_heat_inside.sum()),
      'unguarded_fast_float_source_admitted':all(x==N for x in raw_included) and int(raw_heat_inside.sum())==N,
      'finite_difference_step':epsilon,
      'jvp_finite_difference_normalized_max_error':max_rel,
      'jvp_finite_difference_max_abs_error':max_abs,
      'photon_ledger_relative_residual':budget,
      'source_photoelectron_particles_per_s':absorbed,
      'source_primary_photoheat_erg_per_s':float(np.sum(heat)),
      'heii_trace_nonzero_extended_cells':heii_nonzero_extended,
      'heii_trace_underflowed_to_f64_zero_cells':heii_f64_lost,
      'heii_trace_integrated_rate_longdouble_decimal':str(heii_integrated_ld),
      'float64_underflow_is_not_physical_zero':True,
      'source_owner_comparison_pass':pass_gate,
      'first_failure_preserved_as':'ORIGINAL26_INITIAL_INTERVAL_FAIL.json',
      'full26event_integration':'NOT_RUN','full_dh_jacobian':'NOT_RUN',
      'allX_uniform_remainder':'NOT_RUN','canonical_delta':0,'production':'HOLD',
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
    print(json.dumps({k:result[k] for k in ['nodes','source_owner_comparison_pass','jvp_finite_difference_normalized_max_error','hybrid_owner_interval_included_cell_species','hybrid_owner_interval_included_heat_cells','photon_ledger_relative_residual']}))
    if not result['source_owner_comparison_pass']:raise SystemExit(2)
if __name__=='__main__':main()
