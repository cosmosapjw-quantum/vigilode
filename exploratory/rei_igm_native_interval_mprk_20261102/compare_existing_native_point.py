#!/usr/bin/env python3
"""Independent immutable prior native point-map vs current original MPRK interval.

This point inclusion is a cross-implementation DIAGNOSTIC. It does not certify
the full thermochemical interval IVP or V2 official accepted tick161.
"""
import argparse,hashlib,json,struct
from pathlib import Path
N=46080
POINT_SHA="f7488ac29a51e28a2f12092947fe656eeefd4900ddd9ae0e5cf7c77e977a8f05"
PARENT_SHA="eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"
SOURCE_SHA="95b2cdbfeb5777c2b893910f1663c271d0ead673629180ece068bf3e0a6fb001"
def sha(b):return hashlib.sha256(b).hexdigest()
def run(parentfile,pointfile,intervalfile,sourcefile,outfile):
    parent=parentfile.read_bytes()
    points=pointfile.read_bytes()
    proof=intervalfile.read_bytes()
    source=sourcefile.read_bytes()
    if sha(parent)!=PARENT_SHA:raise AssertionError("SOURCE_PARENT_SHA_DRIFT")
    if sha(points)!=POINT_SHA:raise AssertionError("POINT_SHA_DRIFT")
    if sha(source)!=SOURCE_SHA:raise AssertionError("CORRECTOR_SOURCE_SHA_DRIFT")
    if not (parent[:8]==b"REIF05P2" and points[:8]==b"REIF05M1" and proof[:8]==b"REIF05I2"):raise AssertionError("SOURCE_LAYOUT_MAGIC_CHANGED")
    if not (len(proof)==16+3*N*10*8 and len(points)==16+3*N*6*8):raise AssertionError("STATE_LENGTH_CHANGED")
    if len(parent)!=24+12*N*8:raise AssertionError("SOURCE_PARENT_LENGTH_CHANGED")
    initial=struct.unpack_from("<"+str(10*N)+"d",parent,24)
    lower,upper=initial[:5*N],initial[5*N:]
    counts=[0,0,0];first=[]
    for m in range(3):
        for i in range(N):
            old=(tuple(0.5*(lower[5*i+j]+upper[5*i+j]) for j in range(5))
                 if m<2 else struct.unpack_from("<5d",points,16+48*(N+i)))
            result=struct.unpack_from("<5d",points,16+48*(m*N+i))
            envelope=struct.unpack_from("<10d",proof,16+80*(m*N+i))
            for j in range(5):
                delta=result[j]-old[j]
                if envelope[j]<=delta<=envelope[5+j]:counts[m]+=1
                elif len(first)<20:
                    first.append(dict(map=m,node=i,species=j,delta=delta,
                                      lower=envelope[j],upper=envelope[5+j]))
    record=dict(schema="REI_ORIGINAL_F05_PRIOR_RUST_POINT_VS_NATIVE_INTERVAL_V1",
        input_point_sha256=sha(points),source_sha256=sha(source),
        maps=["full","half1","half2"],node_count=N,
        point_increment_components_included=counts,
        all_point_included=(counts==[N*5]*3),first_missing=first,
        owner_14_interval_ledgers="NOT_RUN_BY_THIS_AUDIT",
        native_full_ivp_interval="NOT_ESTABLISHED",
        official_tick161="NOT_ACCEPTED",canonical_delta=0,production="HOLD")
    outfile.write_text(json.dumps(record,indent=2,sort_keys=True)+"\n")
    print(json.dumps({k:record[k] for k in ("point_increment_components_included",
                   "all_point_included","first_missing")},sort_keys=True))
    if not record["all_point_included"]:raise AssertionError("POINT_SHADOW_OUTSIDE_NATIVE_CORRECTOR")
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for a in ("parent","point","interval","source","out"):p.add_argument("--"+a,type=Path,required=True)
    args=p.parse_args()
    run(args.parent,args.point,args.interval,args.source,args.out)
