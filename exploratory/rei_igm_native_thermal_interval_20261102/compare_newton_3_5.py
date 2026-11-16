#!/usr/bin/env python3
"""Full 46,080×9 original REI F05 native root width and cost comparison.

This checks both complete physical source outputs, exact source and result
byte identity, and strict per-cell interval inclusion. Not whole-IVP speedup.
"""
import argparse,hashlib,json,math,struct
from pathlib import Path
N=46080
NR=9
LEN=24+NR*N*41
def sha(data):return hashlib.sha256(data).hexdigest()
def read(path,receipt,steps):
    raw=path.read_bytes()
    meta=json.loads(receipt.read_text())
    if len(raw)!=LEN or raw[:8]!=b"REIF05R2":
        raise ArithmeticError("ORIGINAL_NATIVE_THERMAL_RESULT_SCHEMA")
    if struct.unpack_from("<QQ",raw,8)!=(N,NR):
        raise ArithmeticError("ORIGINAL_NATIVE_THERMAL_RESULT_LENGTH")
    if sha(raw)!=meta["native_result_sha256"]:
        raise ArithmeticError("ORIGINAL_NATIVE_THERMAL_RESULT_SHA")
    if meta["newton_contraction_iterations"]!=steps:
        raise ArithmeticError("SOURCE_SCOPED_NEWTON_VARIANT_NOT_MATCHED")
    if meta["verified_root_cells"]!=N*NR:
        raise ArithmeticError("SOURCE_SCOPED_ROOTS_NOT_CERTIFIED")
    return raw,meta
def main(a):
    old,meta5=read(a.root5,a.receipt5,5)
    fast,meta3=read(a.root3,a.receipt3,3)
    if meta5["source_sha256"]!=meta3["source_sha256"]:
        raise ArithmeticError("ORIGINAL_REI_SOURCE_CONTEXT_CHANGED")
    summary=[]
    equal=0
    inverted=0
    maxratio=0.0
    for k in range(NR):
        strict=0;local_max=0;local_ratio=0.0
        for i in range(N):
            j=24+(k*N+i)*41
            if old[j]!=1 or fast[j]!=1:
                raise ArithmeticError("NATIVE_SOURCE_ROOT_UNCERTIFIED")
            a5,b5=struct.unpack_from("<2d",old,j+1)
            a3,b3=struct.unpack_from("<2d",fast,j+1)
            if not(a3<=a5<=b5<=b3):
                inverted+=1
                if inverted<4:print("INVALID_CONTRACTION",k,i,a3,a5,b5,b3)
                continue
            if a3==a5 and b3==b5:equal+=1
            else:strict+=1
            w5=b5-a5
            w3=b3-a3
            local_max=max(local_max,w3-w5)
            if w5>0:local_ratio=max(local_ratio,w3/w5)
        maxratio=max(maxratio,local_ratio)
        summary.append({
          "map":meta3["source_root_reports"][k]["map"],
          "stage":meta3["source_root_reports"][k]["root"],
          "source_cells":N,"three_step_tube_wider_in_cells":strict,
          "maximum_extra_lnT_interval_width":local_max,
          "max_3_over_5_lnT_width_ratio":local_ratio})
    out={
       "schema":"REI_F05_NATIVE_ORIGINAL_THERMAL_C8_NEWTON3_VS_5_V1",
       "source_sha256":meta5["source_sha256"],
       "root5_sha256":meta5["native_result_sha256"],
       "root3_sha256":meta3["native_result_sha256"],
       "contexts":N*NR,
       "five_root_contraction_passed":meta5["verified_root_cells"],
       "three_root_contraction_passed":meta3["verified_root_cells"],
       "strict_five_tube_subset_three_tube_violations":inverted,
       "equal_width_count":equal,
       "maximum_3_over_5_width_ratio":maxratio,
       "five_root_only_seconds":meta5["elapsed_seconds_native_root_only"],
       "three_root_only_seconds":meta3["elapsed_seconds_native_root_only"],
       "root_only_time_ratio_3_over_5":meta3["elapsed_seconds_native_root_only"]/meta5["elapsed_seconds_native_root_only"],
       "per_original_map_and_root":summary,
       "comparison":"REAL_46080_X9_SAME_SOURCE_ROOT_CONTRACT_ONLY",
       "whole_ivp_matched_error_speedup":"NOT_ESTABLISHED",
       "native_final_14_source_ledgers":"NOT_RUN",
       "official_tick161":"NOT_ACCEPTED","canonical_delta":0,"production":"HOLD"}
    a.out.write_text(json.dumps(out,indent=2,sort_keys=True)+"\n")
    print(json.dumps({k:out[k] for k in (
         "contexts","strict_five_tube_subset_three_tube_violations",
         "equal_width_count","maximum_3_over_5_width_ratio",
         "five_root_only_seconds","three_root_only_seconds",
         "root_only_time_ratio_3_over_5")},sort_keys=True))
    if inverted:raise ArithmeticError("CERTIFIED_THREE_VS_FIVE_INTERVAL_INCLUSION_FAILED")
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for a in ("root5","receipt5","root3","receipt3","out"):
        p.add_argument("--"+a,type=Path,required=True)
    main(p.parse_args())
