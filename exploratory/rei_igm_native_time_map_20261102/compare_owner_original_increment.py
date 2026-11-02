#!/usr/bin/env python3
"""Strict full 46,080-cell native point vs ORIGINAL owner interval increment check.

No tolerance or original gate changes; this is an *inclusion diagnostic*, never
proof that original owner uncertainty or all-X certified interval map has run in Rust.
"""
from __future__ import annotations
import argparse,csv,gzip,hashlib,json,math,struct
from pathlib import Path
N=46080
CHANNELS=("x_HII","x_HeII","x_HeIII","log_T")
MAPS={"full":"original_full","half2":"original_twohalf"}
def read_parent(path):
    b=path.read_bytes()
    if hashlib.sha256(b).hexdigest()!="eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064":
        raise ValueError("PARENT_DRIFT")
    y=struct.unpack_from("<"+str(12*N)+"d",b,24)
    return y[:5*N],y[5*N:10*N],y[10*N:11*N],y[11*N:12*N]
def read_shadow(path):
    b=path.read_bytes()
    if b[:8]!=b"REIF05M1" or len(b)!=16+3*N*6*8:raise ValueError("RUST_SHADOW_WRONG_LAYOUT")
    return [struct.unpack_from("<6d",b,16+48*(i+N*g))
            for g in range(3) for i in range(N)]
def compare(parent,native,owner_csv,out):
    plo,phi,zlo,zhi=read_parent(parent)
    arr=read_shadow(native)
    ok={k:{c:0 for c in CHANNELS} for k in MAPS}
    miss=[]
    with gzip.open(owner_csv,"rt",newline="") as fh:
        rows=csv.DictReader(fh)
        for i,row in enumerate(rows):
            if i>=N or int(row["node"])!=i:raise ValueError("OWNER_NODE_ORDER_DRIFT")
            pop0=[0.5*(plo[5*i+j]+phi[5*i+j]) for j in range(5)]
            ln0=0.5*(zlo[i]+zhi[i])
            nh=pop0[0]+pop0[1]
            nhe=sum(pop0[2:5])
            for name,prefix in MAPS.items():
                offset=0 if name=="full" else 2*N
                v=arr[offset+i]
                dx=dict(x_HII=(v[1]-pop0[1])/nh,
                        x_HeII=(v[3]-pop0[3])/nhe,
                        x_HeIII=(v[4]-pop0[4])/nhe,
                        log_T=v[5]-ln0)
                for c in CHANNELS:
                    lo=float(row[f"{c}_{prefix}_increment_lower"])
                    hi=float(row[f"{c}_{prefix}_increment_upper"])
                    if lo<=dx[c]<=hi:ok[name][c]+=1
                    elif len(miss)<15:miss.append(dict(node=i,map=name,channel=c,point=dx[c],lower=lo,upper=hi,delta_low=dx[c]-lo,delta_high=dx[c]-hi))
        if next(rows,None) is not None or i!=N-1:raise ValueError("OWNER_SOURCE_COUNT_DRIFT")
    r=dict(schema="INDEPENDENT_NATIVE_POINT_INSIDE_OWNER_ORIGINAL_INCREMENT_INTERVALS_V1",
        source_cells=N,owner_interval_csv_sha256=hashlib.sha256(owner_csv.read_bytes()).hexdigest(),
        native_point_inclusion_counts=ok,all_strict_included=all(v==N for m in ok.values() for v in m.values()),
        first_missing=miss,original_owner_interval_set_ledgers="OWNER_SEPARATELY_CERTIFIED",
        rust_full_interval_enclosure="NOT_ESTABLISHED",canonical_delta=0,production="HOLD",
        tick161="NOT_ACCEPTED")
    out.write_text(json.dumps(r,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:r[k] for k in ("native_point_inclusion_counts","all_strict_included","first_missing")},sort_keys=True))
    return r
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for name in ("parent","native","owner","out"):
        p.add_argument("--"+name,required=True,type=Path)
    a=p.parse_args()
    compare(a.parent,a.native,a.owner,a.out)
