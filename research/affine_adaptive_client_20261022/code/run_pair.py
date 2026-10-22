#!/usr/bin/env python3
"""Exactly one bounded actual-owner pair plus one injected transactional check."""
from pathlib import Path
import argparse, json, sys, time, hashlib, traceback, resource
from types import SimpleNamespace
import numpy as np
from scipy.integrate._ivp.common import select_initial_step
from lifted_bdf import CNorm, LiftedBDF, NormFailure, CountedBDF
from affine_system import AffineSystem, KEEP, LIFT
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--baseline',type=Path,default=ROOT/'vendor/bass_compact');p.add_argument('--output',type=Path,default=ROOT/'evidence/pair_003');args=p.parse_args()
sys.path.insert(0,str(args.baseline/'code'))
from compact_system import CompactSystem, lift as lift62, restrict
from inherited_gates import evaluate
from compact_run import native_scalar_metadata
CONFIG=dict(Sigma_F=.6,matter_A=1.5561956189864556,radiation_A=-.4275708327913344,backend='prepared',opacity=True,_case_wall_seconds=25)
RTOL=2e-8; ATOL=restrict(np.r_[np.full(40,2e-11),np.full(21,2e-10),2e-11]);GRID=np.linspace(0,.01,9)

def counts(solver,system,wall):
    return dict(nfev=solver.nfev,njev=solver.njev,nlu=solver.nlu,owner_rhs_calls=system.nrhs,owner_jac_calls=system.njac,accepted_steps=solver.accepted_steps,lte_rejections=solver.lte_rejections,newton_rejections=solver.newton_rejections,norm_calls=dict(solver.norm_calls if hasattr(solver,'norm_calls') else solver.lifted_norm.calls),wall_seconds=wall)

def integrate(solver):
    y=np.empty((solver.n,len(GRID)));y[:,0]=solver.y;index=1
    while solver.status=='running':
        solver.step()
        if solver.status=='failed':raise RuntimeError('BDF step failed')
        dense=solver.dense_output()
        while index<len(GRID) and GRID[index]<=solver.t:
            y[:,index]=dense(GRID[index]);index+=1
    if index!=len(GRID) or solver.t!=GRID[-1]:raise RuntimeError('incomplete grid')
    return y

def injected_transaction(norm):
    class TinyAffine:
        def lift(self,u):
            d=np.zeros(33);d[[23,25,28]]=1
            return d+LIFT@u
    def poison(e,s,phase):
        if phase=='order_plus':s=s.copy();s[23]=0.
        return norm(e,s,phase)
    s=LiftedBDF(lambda t,u:np.zeros(30),0.,np.full(30,.01),.01,affine=TinyAffine(),full_atol=ATOL,lifted_norm=poison,first_step=.001,rtol=RTOL,atol=ATOL[KEEP],jac=np.zeros((30,30)))
    # Make the first attempt an order-adaptation opportunity. This is an
    # injected transaction test, not a scientific trajectory or norm oracle.
    s.n_equal_steps=s.order
    before={k:getattr(s,k).copy() if isinstance(getattr(s,k),np.ndarray) else getattr(s,k) for k in ('t','y','D','h_abs','J','LU','order','n_equal_steps')}
    try:s.step()
    except NormFailure as error:
        restored=all(np.array_equal(getattr(s,k),v,equal_nan=True) if isinstance(v,np.ndarray) else getattr(s,k)==v for k,v in before.items())
        return dict(status='PASS' if restored and s.accepted_steps==0 else 'FAIL',exception=str(error),accepted_steps=s.accepted_steps,all_accepted_state_restored=restored,injected_phase='order_plus',c_statuses=dict(norm.statuses))
    return dict(status='FAIL',error='invalid C norm was not raised')

args.output.mkdir(parents=True,exist_ok=False)
start=time.perf_counter();cpu0=time.process_time();receipt=dict(status='FAIL',config=CONFIG,acceptance='max sampled full33 endpoint-scaled WRMS <= 1 and inherited gates',rtol=RTOL,atol33=ATOL.tolist(),max_step=.025,samples=9,x_end=.01)
try:
    testnorm=CNorm(ROOT/'code/liblifted_error_norm.so');receipt['transactional_invalid_norm']=injected_transaction(testnorm)
    if receipt['transactional_invalid_norm']['status']!='PASS':raise RuntimeError('transactional check failed')
    full=CompactSystem(CONFIG);f0=full.rhs(0,full.y0)
    first=select_initial_step(full.rhs,0.,full.y0,.01,.025,f0,1.,1,RTOL,ATOL)
    receipt['common_first_step']=float(first);receipt['initial_selector_owner_rhs_calls']=full.nrhs
    wall=time.perf_counter();base=CountedBDF(full.rhs,0.,full.y0,.01,rtol=RTOL,atol=ATOL,max_step=.025,jac=full.jac,first_step=first);vbase=integrate(base)
    receipt['full33_counts']=counts(base,full,time.perf_counter()-wall)
    reduced_owner=CompactSystem(CONFIG);affine=AffineSystem(reduced_owner);norm=CNorm(ROOT/'code/liblifted_error_norm.so')
    wall=time.perf_counter();small=LiftedBDF(affine.rhs,0.,affine.y0,.01,rtol=RTOL,atol=ATOL[KEEP],max_step=.025,jac=affine.jac,first_step=first,affine=affine,full_atol=ATOL,lifted_norm=norm);u=integrate(small);vsmall=affine.lift(u)
    receipt['affine30_counts']=counts(small,reduced_owner,time.perf_counter()-wall);receipt['c_statuses']=dict(norm.statuses)
    error=vsmall-vbase;scale=ATOL[:,None]+RTOL*abs(vbase);wrms=np.sqrt(np.sum((error/scale)**2,axis=0)/33)
    # Pair discrepancy is genuinely 33-dimensional; it is diagnostic, not a
    # reduced controller error and thus is not passed through the tangent C API.
    db=evaluate(full,GRID,vbase,CONFIG);ds=evaluate(reduced_owner,GRID,vsmall,CONFIG)
    receipt.update(full33_diagnostics=db,affine30_diagnostics=ds,sampled_wrms=wrms.tolist(),max_sampled_wrms=float(max(wrms)),max_full33_abs_difference=float(np.max(abs(error))),max_species_scaled_difference=float(np.max(abs(lift62(vsmall)[40:61]-lift62(vbase)[40:61]))),final_signed_difference=(np.array(ds['final_signed_observables'])-db['final_signed_observables']).tolist(),final_tau_difference=ds['final_tau_T']-db['final_tau_T'])
    np.savez_compressed(args.output/'trajectory.npz',x=GRID,full33=vbase.T,affine30=u.T,affine_lifted33=vsmall.T,full62=lift62(vbase).T,affine_lifted62=lift62(vsmall).T,full33_wrms_difference=wrms,species_names=np.array(full.atomic.names),full21_Y=(lift62(vbase)[40:61].T*full.scale),affine21_Y=(lift62(vsmall)[40:61].T*full.scale))
    passed=max(wrms)<=1 and all(db['gates'].values()) and all(ds['gates'].values())
    receipt['status']='PASS_SCOPED' if passed else 'FAIL'
except Exception as error:
    receipt.update(error=str(error),traceback=traceback.format_exc())
receipt.update(wall_seconds=time.perf_counter()-start,cpu_seconds=time.process_time()-cpu0,physical_scope='Physical HOLD; conditional prescribed T/UV, test-fluid baryons, single-photoelectron yield; no speed claim')
receipt['outputs']={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in args.output.glob('*.npz')}
(args.output/'RESULT.json').write_text(json.dumps(native_scalar_metadata(receipt),indent=2,allow_nan=False)+'\n')
print(json.dumps({k:receipt[k] for k in ('status','wall_seconds','cpu_seconds','max_sampled_wrms','error') if k in receipt},indent=2));sys.exit(0 if receipt['status']=='PASS_SCOPED' else 1)
