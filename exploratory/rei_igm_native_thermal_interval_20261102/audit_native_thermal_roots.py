#!/usr/bin/env python3
"""Independent native original source thermochemical root interval audit.

Checks all 414,720 original predictor/stage/final REI F05 contexts and selected
high precision directly evaluated original nine cooling equations. Not a
certified global/nonlinear IVP or original V1 history admission.
"""
from __future__ import annotations
import argparse,hashlib,json,math,struct
from pathlib import Path
N=46080
R=9
COLS=32
SOURCE_SHA="73ed05ef73ebc2b098d70aa66970b9e6ceb2bcb567c13fa80b2efc4017ebce3a"
SAMPLES=(0,1,125,999,23040,46079)
def sha(b):return hashlib.sha256(b).hexdigest()
def fail(msg):raise ArithmeticError(msg)
def source_row(raw,k,i):
    base=24+k*(32+N*COLS*8)+32+i*COLS*8
    return struct.unpack_from("<32d",raw,base)
def native_row(raw,k,i):
    base=24+(k*N+i)*41
    return raw[base],*struct.unpack_from("<5d",raw,base+1)

def highprecision_residual(rec,x):
    """One physical source-admissible midpoint realization, 80-digit mpmath."""
    import mpmath as mp
    mp.mp.dps=80
    d=lambda i:(mp.mpf(rec[i])+mp.mpf(rec[i+1]))/2
    photo,expansion,ecoef,u0,const=(d(k) for k in (0,2,4,6,8))
    fac=[d(10+2*k) for k in range(9)]
    T=mp.exp(x)
    lh,le,le3=315614/T,570670/T,1263030/T
    rates=[
        mp.mpf('3.435e-30')*T*lh**mp.mpf('1.970')/
          (1+(lh/mp.mpf('2.250'))**mp.mpf('0.376'))**mp.mpf('3.720'),
        mp.mpf('1.380649e-16')*T*mp.mpf('1.26e-14')*le**mp.mpf('0.750'),
        8*mp.mpf('3.435e-30')*T*le3**mp.mpf('1.970')/
          (1+(le3/mp.mpf('2.250'))**mp.mpf('0.376'))**mp.mpf('3.720'),
        mp.mpf('7.5e-19')*mp.exp(-118348/T)/(1+mp.sqrt(T/1e5)),
        mp.mpf('5.54e-17')*T**mp.mpf('-0.397')*mp.exp(-473638/T)/(1+mp.sqrt(T/1e5)),
        mp.mpf('5.835e-11')*mp.sqrt(T)*mp.exp(-157804/T),
        mp.mpf('2.71e-11')*mp.sqrt(T)*mp.exp(-285331/T),
        mp.mpf('5.707e-12')*mp.sqrt(T)*mp.exp(-631495/T),
        mp.mpf('1.42e-27')*mp.sqrt(T)*(mp.mpf('1.1')+mp.mpf('0.34')*
              mp.exp(-(mp.mpf('5.5')-mp.log(T)/mp.log(10))**2/3))
    ]
    rhs=photo-sum(a*b for a,b in zip(fac,rates))-expansion*T
    return ecoef*T-u0-rec_dt[0]*(const+rec_dt[1]*rhs)

def audit(srcfile,rustfile,rustreceipt,owner_manifest,outfile):
    src=srcfile.read_bytes();raw=rustfile.read_bytes()
    report=json.loads(rustreceipt.read_text())
    origin=json.loads(owner_manifest.read_text())
    if sha(src)!=SOURCE_SHA:fail("ORIGINAL_THERMAL_SOURCE_SHA_MISMATCH")
    if src[:8]!=b"REIF05T2" or struct.unpack_from("<QQ",src,8)!=(N,R):
        fail("ORIGINAL_THERMAL_BINARY_SCHEMA")
    if len(src)!=24+R*(32+N*COLS*8):fail("ORIGINAL_THERMAL_SOURCE_LENGTH")
    if raw[:8]!=b"REIF05R2" or struct.unpack_from("<QQ",raw,8)!=(N,R):
        fail("NATIVE_THERMAL_BINARY_SCHEMA")
    if len(raw)!=24+R*N*41:fail("NATIVE_THERMAL_BINARY_LENGTH")
    if sha(raw)!=report["native_result_sha256"]:fail("NATIVE_THERMAL_OUTPUT_SHA_MISMATCH")
    if origin["owner_14_final_set_ledgers"]!="14/14_PASS_OWNER_ONLY":
        fail("MISSING_ORIGINAL_SOURCE_FINAL_LEDGER")
    if report["original_v1_tick161"]!="NOT_ACCEPTED":
        fail("FORGED_ORIGINAL_ACCEPTANCE")
    counts=[0]*R
    overlap=[0]*R
    rhs_overlap=[0]*R
    derivative_nonpositive=0
    invalid=0
    first=[]
    sampled=[]
    import mpmath as mp
    mp.mp.dps=80
    for k in range(R):
        expected=report["source_root_reports"][k]
        clock=struct.unpack_from("<dd",src,24+k*(32+N*COLS*8)+16)
        global rec_dt
        rec_dt=(mp.mpf(clock[0]),mp.mpf(clock[1]))
        for i in range(N):
            ref=source_row(src,k,i)
            status,l,h,dlo,rl,rh=native_row(raw,k,i)
            if status!=1:
                if len(first)<18:first.append({"map_idx":k,"cell":i,"status":status})
                continue
            if not all(math.isfinite(x) for x in (l,h,dlo,rl,rh)) or not (0<l<=h<=math.log(1e5) and dlo>0 and rl<=rh):
                invalid+=1
                if len(first)<18:first.append({"map_idx":k,"cell":i,"error":"NONFINITE_CONE"})
                continue
            counts[k]+=1
            if l<=ref[29] and h>=ref[28]:overlap[k]+=1
            elif len(first)<18:first.append({"map_idx":k,"cell":i,"error":"OWNER_ROOT_DISJOINT"})
            if rl<=ref[31] and rh>=ref[30]:rhs_overlap[k]+=1
            elif len(first)<18:first.append({"map_idx":k,"cell":i,"error":"OWNER_RHS_DISJOINT"})
            if i in SAMPLES:
                L=mp.mpf(l);H=mp.mpf(h)
                fl=highprecision_residual(ref,L)
                fh=highprecision_residual(ref,H)
                # A point chosen from all original interval parameters is
                # admissible, but finite precision midpoints can lose weak
                # correlations; each independent endpoint sign is diagnostic.
                point_pass=fl<=0<=fh
                sampled.append({"root_index":k,"cell":i,"midpoint_source_sign_crosses_native_root":bool(point_pass)})
                if not point_pass and len(first)<18:
                    first.append({"map_idx":k,"cell":i,"error":"HIGHPRECISION_POINT_NOT_BRACKETED"})
        if counts[k]!=expected["positive_verified_root_cells"]:
            fail("RUST_RECEIPT_COUNT_NOT_MATCHED")
    result={
        "schema":"REI_F05_NATIVE_SDIRK2_ROOT_INDEPENDENT_STDLIB_MP80_V1",
        "original_source_sha256":sha(src),"native_result_sha256":sha(raw),
        "original_root_context_count":N*R,"native_success_by_root":counts,
        "source_owner_root_overlap_by_root":overlap,
        "source_owner_rhs_overlap_by_root":rhs_overlap,
        "invalid_native_output_rows":invalid,
        "mpmath_80digit_midpoint_source_samples":sampled,
        "all_original_roots_verified_by_native":sum(counts)==N*R,
        "all_successful_root_intervals_overlap_owner":counts==overlap,
        "all_successful_rhs_intervals_overlap_owner":counts==rhs_overlap,
        "all_mp80_source_point_samples_bracketed":all(x["midpoint_source_sign_crosses_native_root"] for x in sampled),
        "first_discrepancies":first,
        "admission":"ORIGINAL_PARAMETRIC_SDIRK_ROOT_ONLY",
        "full_original_26event_IVP":"NOT_ESTABLISHED",
        "native_14_final_set_ledgers":"NOT_RUN",
        "official_tick161":"NOT_ACCEPTED","canonical_delta":0,"production":"HOLD"}
    outfile.write_text(json.dumps(result,sort_keys=True,indent=2)+"\n")
    print(json.dumps({k:result[k] for k in (
       "all_original_roots_verified_by_native",
       "native_success_by_root","source_owner_root_overlap_by_root",
       "source_owner_rhs_overlap_by_root","all_mp80_source_point_samples_bracketed",
       "first_discrepancies")},sort_keys=True))
    if not (result["all_original_roots_verified_by_native"]
            and result["all_successful_root_intervals_overlap_owner"]
            and result["all_successful_rhs_intervals_overlap_owner"]
            and result["all_mp80_source_point_samples_bracketed"]
            and result["invalid_native_output_rows"]==0):
        raise ArithmeticError("ORIGINAL_SOURCE_NATIVE_THERMAL_CERTIFICATION_FAILED")
    return result
if __name__=="__main__":
    ap=argparse.ArgumentParser()
    for key in ("source","rust","receipt","owner_manifest","out"):
        ap.add_argument("--"+key,type=Path,required=True)
    a=ap.parse_args()
    audit(a.source,a.rust,a.receipt,a.owner_manifest,a.out)
