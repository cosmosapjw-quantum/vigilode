#!/usr/bin/env python3
"""Read-only original REI 26-event full/half/half source-owner shadow profile.

No original solver, worker, source, acceptance or runtime mutation. The
profile is an exploratory performance map only; no solver optimization claim.
"""
from __future__ import annotations
import argparse
import cProfile
import hashlib
import importlib.util
import json
import math
import pstats
import sys
from pathlib import Path
from time import perf_counter

import numpy as np
import original26_photon_lowrank_check as locked

EXPECTED_HISTORICAL_LOGT=2.1245050576368385e-4
SOURCE_LANE="LOCAL_NEUTRAL_HAZARD_PRIMARY"


def profile(owner:Path, out:Path):
    out.mkdir(parents=True,exist_ok=True)
    def persist(state):
        (out/"ORIGINAL26_OWNER_MAP_PROFILE.json").write_text(
            json.dumps(state,allow_nan=False,indent=2,sort_keys=True,default=str)+"\n")
    state={"schema":"VIGILODE_READONLY_ORIGINAL26_F05_MAP_PROFILE_V1",
           "owner_head_expected":"fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120",
           "original_kernel_sha256":locked.KERNEL_SHA,
           "original_archive_sha256":locked.SOURCE_SHA,
           "accepted_array_sha256":locked.STATE_SHA,
           "accepted_parent_tick":"160/131072","nodes":locked.N,
           "lane":SOURCE_LANE,
           "status":"RUNNING", "source_owner_mutated":False,
           "original_owner_admission":"NOT_RUN","production":"HOLD",
           "canonical_delta":0,"map_timings":{}}
    persist(state)
    sys.path.insert(0,str(owner))
    f05=locked.load("vigilode_readonly_f05",owner/locked.REI_F05_PATH)
    edge=f05.load("vigilode_readonly_edge",owner/"research/rei_igm_original26_temperature_edges_20261101/edge_probe.py")
    worker=f05.load("vigilode_readonly_worker",owner/edge.REL_STAGE/"analysis/attempt_worker.py")
    worker.jax_guard.install_if_missing()
    modelkernel=worker._kernel()
    if f05.sha(worker.KERNEL_PATH)!=locked.KERNEL_SHA:
        raise ArithmeticError("ORIGINAL_KERNEL_IDENTITY_FAILED")
    decoded,source=edge.source_state(owner/edge.REL_STAGE,SOURCE_LANE,worker.state_io)
    if source["archive_sha256"]!=locked.SOURCE_SHA or source["accepted_array_sha256"]!=locked.STATE_SHA:
        raise ArithmeticError("ORIGINAL_ACCEPTED_PARENT_IDENTITY_FAILED")
    base,_a,_b,nh,nhe=worker._context(modelkernel,SOURCE_LANE)
    if len(nh)!=locked.N: raise ValueError("NOT_ORIGINAL_46080_CELLS")
    model=modelkernel.rim.ReducedIntervalModel.from_repo(owner)
    duration=float(base.forcing.duration_seconds(0))
    t0=duration*160/131072; t1=duration*161/131072
    tm=(t0+t1)/2
    P=modelkernel._I(decoded.population_lower,decoded.population_upper)
    Z=modelkernel._I(decoded.log_temperature_lower,decoded.log_temperature_upper)
    paths=(("full",P,Z,t0,t1),("half1",P,Z,t0,tm),("half2",None,None,tm,t1))
    results={}
    for name,initial_pop,initial_log,a,b in paths:
        if name=="half2":
            initial_pop=results["half1"].population
            initial_log=results["half1"].log_temperature
        pr=cProfile.Profile() if name=="full" else None
        start=perf_counter()
        if pr:pr.enable()
        try:
            result=modelkernel.run_step(model,base,parent_pop=initial_pop,
                        parent_logt=initial_log,t0=a,t1=b,total_h=nh,total_he=nhe)
        finally:
            if pr:pr.disable()
        elapsed=perf_counter()-start
        results[name]=result
        state["map_timings"][name]={"seconds":elapsed,
          "classification":result.classification,"native_certified":bool(result.certified),
          "table_events":int(sum(bool(x.any_event) for x in result.table_events))}
        if pr:
            ps=pstats.Stats(pr)
            top=sorted(ps.stats.items(),key=lambda kv:kv[1][3],reverse=True)[:25]
            state["profile_top25"]=[{"file":key[0].replace(str(owner),"<OWNER>"),
                "line":key[1],"func":key[2],"calls":int(value[1]),
                "total_self_seconds":float(value[2]),
                "total_cumulative_seconds":float(value[3])}
                for key,value in top]
        persist(state)
        print(json.dumps({"completed":name,**state["map_timings"][name]}),flush=True)
        if not result.certified or state["map_timings"][name]["table_events"]!=0:
            state["status"]="ORIGINAL_MAP_NOT_CERTIFIED";persist(state)
            return state
    f=results["full"];h=results["half2"]
    historic=np.maximum(np.abs(f.log_temperature.lo-h.log_temperature.hi),
                        np.abs(f.log_temperature.hi-h.log_temperature.lo))
    raw_cross=float(np.max(historic))
    state["historical_independent_output_logT_upper"]=raw_cross
    state["historical_independent_output_logT_gate"]=2e-4
    state["original_historical_failure_retained"]=bool(raw_cross>2e-4)
    state["historical_baseline_reproduced"]=bool(abs(raw_cross-EXPECTED_HISTORICAL_LOGT)<1e-10)
    # The authoritative 14 raw ledgers from the actual original half2 event.
    f05ledger=locked.load("vigilode_readonly_f05_ledger",owner/locked.REI_F05_PATH)
    ledger=f05ledger.original_set_ledger_with_totals(modelkernel,model,h,tm,t1,nh,nhe)
    state["original_14_ledger"]={"passed":bool(ledger["passed"]),
          "evaluated":int(ledger["evaluated_ledgers"]),
          "failed":ledger["failed_ledgers"]}
    state["all_three_original_maps_certified"]=all(x.certified for x in results.values())
    state["diagnostic_elapsed_total_seconds"]=sum(v["seconds"] for v in state["map_timings"].values())
    state["status"]="SCOPED_PASS" if (state["all_three_original_maps_certified"]
       and state["historical_baseline_reproduced"] and
       state["original_historical_failure_retained"] and
       ledger["passed"] and ledger["evaluated_ledgers"]==14) else "SCOPED_HOLD"
    persist(state)
    print(json.dumps({"status":state["status"],"timing":state["map_timings"],
           "historic_logT":raw_cross,"ledger":state["original_14_ledger"]}),flush=True)
    return state


if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("--owner",type=Path,required=True)
    p.add_argument("--out",type=Path,required=True)
    a=p.parse_args()
    x=profile(a.owner.resolve(),a.out.resolve())
    raise SystemExit(0 if x["status"]=="SCOPED_PASS" else 2)
