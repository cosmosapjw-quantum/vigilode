#!/usr/bin/env python3
"""Read-only owner F05 true 46,080-cell predictor/stage/final SDIRK root exporter.

This invokes ORIGINAL 26-event stage and source clock; it does not solve
the thermal IVP in Python on behalf of Rust. Root parameter intervals and
owner root references are transported verbatim with exact input hashes.
"""
from __future__ import annotations
import argparse,hashlib,importlib.util,json,struct,subprocess,sys
from pathlib import Path
import numpy as np

N=46080
REI_HEAD="f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa"
TAR_SHA="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
STATE_SHA="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
KERNEL_SHA="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
MAGIC=b"REIF05T2"
MAPS=("full","half1","half2")
STAGES=("predictor","stage","final")
TAGS=(b"FULL0000",b"HALF1000",b"HALF2000")
KTAGS=(b"PREDICT0",b"STAGE000",b"FINAL000")
COLUMNS=32

def sha(x):return hashlib.sha256(x).hexdigest()
def read_module(name,path):
    spec=importlib.util.spec_from_file_location(name,str(path))
    if not spec or not spec.loader:raise ImportError(path)
    m=importlib.util.module_from_spec(spec);sys.modules[name]=m;spec.loader.exec_module(m)
    return m
def as_iv(x, label, nonnegative=False):
    if hasattr(x,"lo") and hasattr(x,"hi"):
        lo=np.ascontiguousarray(x.lo,dtype="<f8")
        hi=np.ascontiguousarray(x.hi,dtype="<f8")
    else:
        lo=np.ascontiguousarray(x,dtype="<f8");hi=lo.copy()
    if lo.shape!=(N,) or hi.shape!=(N,) or not (
         np.all(np.isfinite(lo)) and np.all(np.isfinite(hi)) and np.all(lo<=hi)):
        raise ArithmeticError("SOURCE_THERMAL_INTERVAL_SHAPE_OR_FINITE_"+label)
    if nonnegative and np.any(lo < -np.nextafter(0.0,np.inf)):
        raise ArithmeticError("SOURCE_NEGATIVE_PHYSICAL_COEFFICIENT_"+label)
    return lo.copy(),hi.copy()
def capture(repo,out,receipt,owner_out):
    live=subprocess.check_output(["git","-C",str(repo),"rev-parse","HEAD"],text=True).strip()
    if live!=REI_HEAD:raise ValueError("FROZEN_OWNER_SOURCE_HEAD_DRIFT")
    script=repo/"research/rei_f05_common_parent_increments_20261101/increment_certificate.py"
    m=read_module("source_thermal_original_increment",script)
    orig_map=m.original_map;groups=[]
    def map_wrapper(kernel,model,base,label,pop,logt,t0,t1,nh,nhe):
        if label!=MAPS[len(groups)]:
            raise ArithmeticError("SOURCE_MAP_CLOCK_ORDER_CHANGED")
        saved=kernel.thermal_root
        calls=[]
        def root_wrapper(ctx,u0,step,constant,weight,seed_lo,seed_hi,**kwargs):
            q=saved(ctx,u0,step,constant,weight,seed_lo,seed_hi,**kwargs)
            calls.append((ctx,u0,float(step),constant,float(weight),q))
            return q
        kernel.thermal_root=root_wrapper
        try:
            result=orig_map(kernel,model,base,label,pop,logt,t0,t1,nh,nhe)
        finally:
            kernel.thermal_root=saved
        if not result[0].certified or len(calls)<3:
            raise ArithmeticError("ORIGINAL_SOURCE_ROOT_CALLS_NOT_CERTIFIED")
        # The unchanged owner map loops its stage/final root until self-inclusion;
        # predictor is first, accepted final stage/final are the last two calls.
        picked=(calls[0],calls[-2],calls[-1])
        if not all(q.certified for *_,q in picked):
            raise ArithmeticError("UNVERIFIED_ORIGINAL_SOURCE_ROOT")
        groups.append((label,picked,len(calls)))
        return result
    m.original_map=map_wrapper
    try:
        owner_out.mkdir(parents=True,exist_ok=True)
        result=m.run(repo,"LOCAL_NEUTRAL_HAZARD_PRIMARY",owner_out)
    finally:
        m.original_map=orig_map
    if not (len(groups)==3 and result.get("original_set_ledgers",{}).get("passed") is True
            and result["original_set_ledgers"]["evaluated_ledgers"]==14
            and result["original_native_table_event_pass"] is True
            and result["frozen_original_archive_sha256"]==TAR_SHA
            and result["original_state_array_sha256"]==STATE_SHA
            and result["original_26event_kernel_sha256"]==KERNEL_SHA
            and all(result["steps"][key]["status"]=="PASS" for key in MAPS)):
        raise ArithmeticError("FROZEN_ORIGINAL_OWNER_F05_MAPS_NOT_VALID")
    outbytes=bytearray(MAGIC+struct.pack("<QQ",N,9))
    descriptions=[]
    for map_i,(label,calls,total_calls) in enumerate(groups):
        for stage_i,(ctx,u0,dt,constant,weight,q) in enumerate(calls):
            if not(np.isfinite(dt) and dt>0 and np.isfinite(weight) and 0<weight<=1):
                raise ArithmeticError("ORIGINAL_ROOT_TIME_OR_WEIGHT_INVALID")
            fields=[]
            for name,v in (
               ("photoheat",ctx.photoheat),("expansion",ctx.expansion),
               ("ecoef",ctx.ecoef),("u0",u0),("constant",constant)):
                low,high=as_iv(v,name,nonnegative=name in ("photoheat","expansion","ecoef","u0"))
                if name in ("ecoef","u0") and np.any(low<=0):
                    raise ArithmeticError("ORIGINAL_THERMAL_POSITIVE_CONE_VIOLATION")
                fields.extend((low,high))
            for k,factor in enumerate(ctx.factors):
                fields.extend(as_iv(factor,f"factor{k}",nonnegative=True))
            fields.extend(as_iv(q.log_temperature,"owner_logt"))
            fields.extend(as_iv(q.rhs,"owner_rhs"))
            assert len(fields)==COLUMNS
            matrix=np.stack(fields,axis=1).astype("<f8",copy=False)
            outbytes.extend(TAGS[map_i]+KTAGS[stage_i]+struct.pack("<dd",dt,weight))
            outbytes.extend(matrix.tobytes(order="C"))
            descriptions.append({"map":label,"root":STAGES[stage_i],
                "original_root_calls_in_map":total_calls,
                "dt_seconds":dt,"weight":weight,
                "max_owner_lnT_width":float(np.max(q.log_temperature.hi-q.log_temperature.lo)),
                "min_owner_root_derivative":float(q.minimum_derivative),
                "original_nodes":N,"columns":COLUMNS})
    out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(outbytes)
    report={"schema":"REI_F05_ORIGINAL_46080_X3_THERMAL_SDIRK2_ROOT_CONTEXT_V1",
      "original_source_head":live,"original_source_tar_sha256":TAR_SHA,
      "original_accepted_state_sha256":STATE_SHA,"original_kernel_sha256":KERNEL_SHA,
      "source_binary_sha256":sha(outbytes),"source_binary_bytes":len(outbytes),
      "nodes_per_root":N,"root_count":9,"columns_per_cell":COLUMNS,
      "root_summaries":descriptions,"owner_14_final_set_ledgers":"14/14_PASS_OWNER_ONLY",
      "owner_table_event_count":0,"official_tick161":"NOT_ACCEPTED",
      "canonical_delta":0,"production":"HOLD"}
    receipt.write_text(json.dumps(report,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({"source_binary_sha256":report["source_binary_sha256"],
      "source_binary_bytes":len(outbytes),"roots":descriptions},sort_keys=True),flush=True)
if __name__=="__main__":
    parser=argparse.ArgumentParser()
    for name in ("owner","out","receipt","owner_out"):
        parser.add_argument("--"+name,type=Path,required=True)
    a=parser.parse_args()
    capture(a.owner.resolve(),a.out.resolve(),a.receipt.resolve(),a.owner_out.resolve())
