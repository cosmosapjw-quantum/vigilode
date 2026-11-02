#!/usr/bin/env python3
"""Read-only real REI F05 46k×3 original MPRK22(1) final corrector interval exporter.

Runs original source-owner full/half1/half2; original Python source/worker
is not modified.  Rust consumes its true 5-pop/25-flux/predictor-denominator
interval boxes; no synthetic/source-point atlas is permitted.
"""
from __future__ import annotations
import argparse,hashlib,importlib.util,json,struct,subprocess,sys
from pathlib import Path
import numpy as np

N=46080
ORIGINAL_HEAD="f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa"
TAR="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
STATE="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
KERNEL="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
MAGIC=b"REIF05C2"
METHODS=("full","half1","half2")
LABELS=(b"FULL0000",b"HALF1000",b"HALF2000")
COLUMNS=70

def sha(b:bytes):return hashlib.sha256(b).hexdigest()
def load(path,name):
    spec=importlib.util.spec_from_file_location(name,str(path))
    if spec is None or spec.loader is None:raise ImportError(path)
    m=importlib.util.module_from_spec(spec);sys.modules[name]=m
    spec.loader.exec_module(m);return m
def arr(x,shape,label,allow_negative=False):
    out=np.ascontiguousarray(x,dtype="<f8")
    if out.shape!=shape or not np.isfinite(out).all():
        raise ValueError("ORIGINAL_CORRECTOR_SHAPE_OR_NONFINITE_"+label)
    if allow_negative:
        if (out<np.nextafter(0.0,-np.inf)).any():
            raise ValueError("ORIGINAL_CORRECTOR_GENUINELY_NEGATIVE_FLUX_"+label)
    elif (out<=0).any():
        raise ValueError("ORIGINAL_CORRECTOR_NONPOSITIVE_"+label)
    return out
def extract(repo:Path,out:Path,receipt:Path,owner_out:Path):
    live=subprocess.check_output(["git","-C",str(repo),"rev-parse","HEAD"],text=True).strip()
    if live!=ORIGINAL_HEAD:raise ArithmeticError(f"SOURCE_REI_VERSION_DRIFT:{live}")
    src=repo/"research/rei_f05_common_parent_increments_20261101/increment_certificate.py"
    audit=repo/"research/rei_f05_mmatrix_residual_20261101/residual_certificate.py"
    m=load(src,"f05_current_original_increment")
    d=load(audit,"f05_current_original_residual")
    records=[]
    saved=m.krawczyk_increment
    def wrapped(kernel,parent,flux,den,dt,nh,nhe):
        def take(q,shape,label,allow_negative=False):
            return arr(q,shape,label,allow_negative).copy()
        p0=take(parent.lo,(N,5),"parent_lo")
        p1=take(parent.hi,(N,5),"parent_hi")
        d0=take(den.lo,(N,5),"denominator_lo")
        d1=take(den.hi,(N,5),"denominator_hi")
        f0=take(flux.lo,(N,5,5),"flux_lo",allow_negative=True)
        f1=take(flux.hi,(N,5,5),"flux_hi",allow_negative=True)
        if np.any(p1<p0) or np.any(d1<d0) or np.any(f1<f0):
            raise ArithmeticError("SOURCE_CORRECTOR_INTERVAL_INVERTED")
        if not (np.isfinite(dt) and float(dt)>0):raise ArithmeticError("SOURCE_DURATION_INVALID")
        records.append((p0,p1,f0,f1,d0,d1,float(dt)))
        # REUSE the OWNER's original independent 1-norm residual certifier,
        # rather than replacing its 26-event method or admission criteria.
        return d.structural_residual_increment(kernel,parent,flux,den,dt,nh,nhe,m)
    m.krawczyk_increment=wrapped
    owner_out.mkdir(parents=True,exist_ok=True)
    try:
        result=m.run(repo,"LOCAL_NEUTRAL_HAZARD_PRIMARY",owner_out)
    finally:
        m.krawczyk_increment=saved
    if not (result.get("original_set_ledgers",{}).get("passed") is True
        and result["original_set_ledgers"]["evaluated_ledgers"]==14
        and result.get("original_native_table_event_pass") is True
        and result["frozen_original_archive_sha256"]==TAR
        and result["original_state_array_sha256"]==STATE
        and result["original_26event_kernel_sha256"]==KERNEL
        and all(result["steps"][key]["status"]=="PASS" for key in METHODS)
        and len(records)==3):
        raise ArithmeticError("SOURCE_26EVENT_OWNER_FINAL_CORRECTOR_NOT_AUTHENTIC")
    # Source clock from PR183 original owner is exact; bind each step duration,
    # not a new Python interpretation of its time integration.
    times=[float(x[6]) for x in records]
    if not (0 < times[1] <= times[0] and abs(times[1]-times[2])<1e-3
            and abs(times[0]-(times[1]+times[2]))<1e-3):
        raise ArithmeticError("ORIGINAL_FULL_HALF_HALF_DURATION_DRIFT")
    binary=bytearray(MAGIC+struct.pack("<QQ",N,len(METHODS)))
    summaries=[]
    for name,tag,(p0,p1,f0,f1,d0,d1,dt) in zip(METHODS,LABELS,records):
        binary.extend(tag+struct.pack("<d",dt))
        fields=np.concatenate([p0,p1,f0.reshape(N,25),f1.reshape(N,25),d0,d1],axis=1)
        if fields.shape!=(N,COLUMNS):
            raise ValueError("MPRK_SOURCE_BINARY_COLUMN_DRIFT")
        binary.extend(fields.astype("<f8",copy=False).tobytes(order="C"))
        summaries.append(dict(map=name,source_duration_seconds=dt,nodes=N,
             min_population=float(p0.min()),min_denominator=float(d0.min()),
             flux_sources=25,nonpositive_flux_width=int(np.count_nonzero(f0<0.0)),
             stage_corrector_directed_entries=N*COLUMNS))
    out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(binary)
    r=dict(schema="ORIGINAL_REI_F05_46080_X3_MPRK_CORRECTOR_INTERVAL_SOURCE_V1",
        source_rei_git_head=live,owner_source_tar_sha256=TAR,
        owner_original_parent_sha256=STATE,owner_original_kernel_sha256=KERNEL,
        accepted_tick="160/131072",original_methods=list(METHODS),
        original_nodes=N,original_intervals_per_cell=COLUMNS,
        source_binary_bytes=len(binary),source_binary_sha256=sha(binary),
        owner_original_14_ledgers_pass=True,owner_original_table_events=0,
        original_source_corrector_recording="EXACT_ORIGINAL_MPRK22_FINAL_CORRECTOR_PER_MAP",
        map_summaries=summaries,official_tick161="NOT_ACCEPTED",
        production="HOLD",canonical_delta=0)
    receipt.write_text(json.dumps(r,sort_keys=True,indent=2,allow_nan=False)+"\n")
    print(json.dumps({k:r[k] for k in ("original_nodes","original_methods",
          "source_binary_bytes","source_binary_sha256","map_summaries")},sort_keys=True))
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for name in ("owner","out","receipt","owner_out"):p.add_argument("--"+name,required=True,type=Path)
    a=p.parse_args()
    extract(a.owner.resolve(),a.out.resolve(),a.receipt.resolve(),a.owner_out.resolve())
