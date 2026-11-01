#!/usr/bin/env python3
"""One bounded shadow of the EXACT original REI interval thermal root.

Candidate removes only the duplicate pair of endpoint F evaluations after a
successful root bracket. The original 26-event run_step, state, source, error
gates, and output are unchanged. Candidate is NOT owner/production code.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import math
import statistics
import sys
from pathlib import Path
from time import perf_counter

import numpy as np
import original26_photon_lowrank_check as lock

REFERENCE_FAIL=2.1245050576368385e-4
LANES=(
    "LOCAL_NEUTRAL_HAZARD_PRIMARY",
    "RECOMBINATION_WEIGHTED_AUDITOR",
    "SCRIPT_SELF_SHIELDING_AUDITOR",
)


def make_shadow_root(m):
    def root(ctx,U0,step,constant,weight,seed_lo,seed_hi,max_iter=10):
        U=U0 if hasattr(U0,"lo") else m._I(U0)
        w=float(weight);dt=float(step)
        const=constant if hasattr(constant,"lo") else m._I(constant)
        def F(x):
            T=m.iv.exp(x)
            return ctx.ecoef*T-U-dt*(const+w*m.thermal_rhs(ctx,x))
        lo=np.nextafter(np.asarray(seed_lo,dtype=float)-1e-8,-np.inf)
        hi=np.nextafter(np.asarray(seed_hi,dtype=float)+1e-8,np.inf)
        # ONLY CHANGE: skip the second F(lo),F(hi) on an unchanged bracket.
        # If all 20 expansions are exhausted, recompute at the NEW endpoints.
        for _ in range(20):
            fl=F(m._I(lo));fh=F(m._I(hi))
            need_lo=fl.hi>0;need_hi=fh.lo<0
            if not(np.any(need_lo)|np.any(need_hi)):break
            span=np.maximum(hi-lo,1e-5)
            lo=np.where(need_lo,lo-1.25*span,lo)
            hi=np.where(need_hi,hi+1.25*span,hi)
        else:
            fl=F(m._I(lo));fh=F(m._I(hi))
        bracket=(fl.hi<=0)&(fh.lo>=0)
        iterations=0
        for iterations in range(1,max_iter+1):
            X=m._I(lo,hi);D=m.thermal_root_derivative(ctx,X,dt*w)
            if np.any(D.lo<=0.0):break
            mid=0.5*(lo+hi);Fm=F(m._I(mid));N=m._I(mid)-Fm/D
            nlo=np.maximum(lo,N.lo);nhi=np.minimum(hi,N.hi)
            valid=nlo<=nhi
            if not np.all(valid):break
            improvement=float(np.max((hi-lo)-(nhi-nlo)))
            lo=np.nextafter(nlo,-np.inf);hi=np.nextafter(nhi,np.inf)
            if improvement<=1e-14:break
        box=m._I(lo,hi)
        D=m.thermal_root_derivative(ctx,box,dt*w)
        rhs=m.thermal_rhs(ctx,box)
        certed=bracket&(D.lo>0.0)
        return m.RootBox(box,rhs,bool(np.all(certed)),D.lo,
                         float(np.min(D.lo)),iterations)
    return root


def result_signature(steps):
    sha=hashlib.sha256()
    for key in ("full","half1","half2"):
        x=steps[key]
        for p in (x.population.lo,x.population.hi,
                  x.log_temperature.lo,x.log_temperature.hi):
            arr=np.ascontiguousarray(np.asarray(p))
            sha.update(str(arr.dtype).encode())
            sha.update(str(arr.shape).encode())
            sha.update(arr.tobytes())
    return sha.hexdigest()


def run(owner:Path,out:Path,lane:str="LOCAL_NEUTRAL_HAZARD_PRIMARY"):
    if lane not in LANES:
        raise ValueError("UNRECOGNIZED_ORIGINAL_SOURCE_LANE")
    out.parent.mkdir(parents=True,exist_ok=True)
    state={"schema":"VIGILODE_ORIGINAL26_THERMAL_ROOT_BRACKET_DEDUP_SHADOW_V1",
           "source_owner":"cosmosapjw-quantum/rei_bianchi",
           "owner_head":"fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120",
           "kernel_sha256":lock.KERNEL_SHA,"archive_sha256":lock.SOURCE_SHA,
           "parent_sha256":lock.STATE_SHA,"accepted_tick":"160/131072",
           "original_nodes":lock.N,"original_groups":4,
           "original_shape_lane":lane,"declared_original_lanes":list(LANES),
           "changed_source_on_disk":False,"published_26_event_owner":False,
           "full_allX_certification":False,"production":"HOLD",
           "status":"RUNNING","canonical_delta":0}
    def checkpoint():
        out.write_text(json.dumps(state,indent=2,sort_keys=True,allow_nan=False)+"\n")
    checkpoint()
    sys.path.insert(0,str(owner))
    f05=lock.load("vigilode_root_f05",owner/lock.REI_F05_PATH)
    edge=f05.load("vigilode_root_edge",owner/"research/rei_igm_original26_temperature_edges_20261101/edge_probe.py")
    worker=f05.load("vigilode_root_worker",owner/edge.REL_STAGE/"analysis/attempt_worker.py")
    worker.jax_guard.install_if_missing()
    m=worker._kernel()
    if f05.sha(worker.KERNEL_PATH)!=lock.KERNEL_SHA:
        raise ArithmeticError("ORIGINAL_KERNEL_DRIFT")
    decoded,source=edge.source_state(owner/edge.REL_STAGE,lane,worker.state_io)
    if source["archive_sha256"]!=lock.SOURCE_SHA or source["accepted_array_sha256"]!=lock.STATE_SHA:
        raise ArithmeticError("ORIGINAL_PARENT_DRIFT")
    base,_,_,nh,nhe=worker._context(m,lane)
    model=m.rim.ReducedIntervalModel.from_repo(owner)
    duration=float(base.forcing.duration_seconds(0))
    start=duration*160/131072;stop=duration*161/131072;mid=(start+stop)/2
    p=m._I(decoded.population_lower,decoded.population_upper)
    z=m._I(decoded.log_temperature_lower,decoded.log_temperature_upper)
    saved_root=m.thermal_root
    saved_rhs=m.thermal_rhs
    shadow=make_shadow_root(m)
    receipts={}
    def trial(tag,use_shadow):
        calls={"thermal_rhs":0}
        def counted_rhs(*args,**kwargs):
            calls["thermal_rhs"]+=1
            return saved_rhs(*args,**kwargs)
        m.thermal_rhs=counted_rhs
        m.thermal_root=shadow if use_shadow else saved_root
        results={}
        times={}
        try:
            for label,x,y,a,b in (("full",p,z,start,stop),
                                   ("half1",p,z,start,mid),
                                   ("half2",None,None,mid,stop)):
                if label=="half2":
                    x=results["half1"].population
                    y=results["half1"].log_temperature
                t=perf_counter()
                step=m.run_step(model,base,parent_pop=x,parent_logt=y,t0=a,t1=b,
                                total_h=nh,total_he=nhe)
                times[label]=perf_counter()-t
                if not step.certified or any(ev.any_event for ev in step.table_events):
                    raise ArithmeticError(f"ORIGINAL_{label}_MAP_UNCERTIFIED")
                results[label]=step
        finally:
            m.thermal_root=saved_root
            m.thermal_rhs=saved_rhs
        full=results["full"].log_temperature;half=results["half2"].log_temperature
        cross=float(np.max(np.maximum(abs(full.lo-half.hi),abs(full.hi-half.lo))))
        ledger=f05.original_set_ledger_with_totals(m,model,results["half2"],mid,stop,nh,nhe)
        if not ledger["passed"] or ledger["evaluated_ledgers"]!=14:
            raise ArithmeticError("ORIGINAL_14_LEDGER_FAILURE")
        if abs(cross-REFERENCE_FAIL)>=1e-10 or cross<=2e-4:
            raise ArithmeticError("HISTORICAL_V1_FAILURE_LOST")
        receipt={"method":"SHADOW_ROOT_DEDUP" if use_shadow else "ORIGINAL_UNMODIFIED",
                 "seconds_by_map":times,
                 "total_seconds":sum(times.values()),
                 "thermal_rhs_calls":calls["thermal_rhs"],
                 "state_bytes_sha256":result_signature(results),
                 "historical_crossbox_logT":cross,
                 "original_14_ledger_pass":ledger["passed"],
                 "original_14_ledger_count":ledger["evaluated_ledgers"],
                 "all_original_native_maps_certified":True,
                 "all_table_events_zero":True}
        receipts[tag]=receipt
        state["trials"]=receipts
        checkpoint()
        print(json.dumps({"trial":tag,"total_seconds":receipt["total_seconds"],
             "rhs_calls":receipt["thermal_rhs_calls"],
             "state_sha":receipt["state_bytes_sha256"]}),flush=True)
        return receipt

    # ABBA to account partially for compilation/cache/host fluctuations.
    for name,candidate in (("A0_original",False),("B1_shadow",True),
                            ("B2_shadow",True),("A3_original",False)):
        trial(name,candidate)
    a=[receipts[k]["total_seconds"] for k in ("A0_original","A3_original")]
    b=[receipts[k]["total_seconds"] for k in ("B1_shadow","B2_shadow")]
    signatures={x["state_bytes_sha256"] for x in receipts.values()}
    state["all_six_original_state_arrays_byte_identical"]=len(signatures)==1
    state["all_14_original_ledgers_and_gates_unchanged"]=True
    state["source_average_rhs_calls_original"]=statistics.mean(
       receipts[k]["thermal_rhs_calls"] for k in ("A0_original","A3_original"))
    state["source_average_rhs_calls_shadow"]=statistics.mean(
       receipts[k]["thermal_rhs_calls"] for k in ("B1_shadow","B2_shadow"))
    state["median_wall_seconds_original"]=statistics.median(a)
    state["median_wall_seconds_shadow"]=statistics.median(b)
    state["diagnostic_wall_speedup"]=statistics.median(a)/statistics.median(b)
    state["strict_shadow_pass"]=bool(state["all_six_original_state_arrays_byte_identical"]
       and state["source_average_rhs_calls_shadow"]<state["source_average_rhs_calls_original"])
    state["status"]="SCOPED_SHADOW_PASS" if state["strict_shadow_pass"] else "SCOPED_SHADOW_HOLD"
    state["no_production_promotion"]=True
    checkpoint()
    print(json.dumps({"status":state["status"],
      "elapsed_ratio":state["diagnostic_wall_speedup"],
      "states_exact":state["all_six_original_state_arrays_byte_identical"],
      "rhs_calls_before":state["source_average_rhs_calls_original"],
      "rhs_calls_after":state["source_average_rhs_calls_shadow"]}),flush=True)
    return state


if __name__=="__main__":
    a=argparse.ArgumentParser()
    a.add_argument("--owner",type=Path,required=True)
    a.add_argument("--out",type=Path,required=True)
    a.add_argument("--lane",choices=LANES,default="LOCAL_NEUTRAL_HAZARD_PRIMARY")
    args=a.parse_args()
    x=run(args.owner.resolve(),args.out.resolve(),args.lane)
    raise SystemExit(0 if x["status"]=="SCOPED_SHADOW_PASS" else 2)
