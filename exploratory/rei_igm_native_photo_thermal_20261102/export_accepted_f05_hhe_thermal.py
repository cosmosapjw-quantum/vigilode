#!/usr/bin/env python3
"""Read-only byte-locked original F05 accepted tick160 H/He thermal fixture.

NO Python solver or new physics: only extracts the frozen original accepted
population and logT intervals and verifies byte-exact HI,HeI,HeII midpoint
identity against the PREVIOUSLY PUBLISHED native Rust photon-source fixture.
"""
from __future__ import annotations
import argparse,gzip,hashlib,importlib.util,json,sys
from pathlib import Path
import numpy as np

OWNER_HEAD="fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
ORIGINAL_SOURCE_SHA="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
ORIGINAL_ARRAY_SHA="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
ORIGINAL_KERNEL_SHA="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
ORIGINAL_PHOTON_RAW_SHA="a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469"
N=46080
MAGIC=b"REIF05P2"

def digest(data:bytes)->str:
    return hashlib.sha256(data).hexdigest()

def module(name:str,path:Path):
    spec=importlib.util.spec_from_file_location(name,path)
    if spec is None or spec.loader is None: raise ImportError(path)
    m=importlib.util.module_from_spec(spec)
    sys.modules[name]=m
    spec.loader.exec_module(m)
    return m

def extract(owner:Path,source_gz:Path,out:Path,manifest:Path):
    import subprocess
    if subprocess.check_output(["git","-C",str(owner),"rev-parse","HEAD"],text=True).strip()!=OWNER_HEAD:
        raise RuntimeError("ORIGINAL_REI_SOURCE_OWNER_HEAD_DRIFT")
    raw_old=gzip.decompress(source_gz.read_bytes())
    if digest(raw_old)!=ORIGINAL_PHOTON_RAW_SHA:
        raise RuntimeError("ORIGINAL_NATIVE_RUST_PHOTON_SOURCE_CHANGED")
    if raw_old[:8]!=b"REIF05R1" or int.from_bytes(raw_old[8:16],"little")!=N:
        raise RuntimeError("PREVIOUS_ORIGINAL_PHOTON_FIXTURE_SHAPE_CHANGED")
    # [J4 e4 alpha(4x3) excess(3x4) ev1] 33 f64 scalar parameters.
    old_n=np.frombuffer(raw_old,dtype="<f8",offset=16+33*8,count=N*3).reshape(N,3)
    sys.path.insert(0,str(owner))
    f05=module("f05_native_accepted_parent_only",owner/"research/rei_f05_common_parent_increments_20261101/increment_certificate.py")
    edge=f05.load("f05_native_accepted_edge",owner/"research/rei_igm_original26_temperature_edges_20261101/edge_probe.py")
    worker=f05.load("f05_native_accepted_worker",owner/edge.REL_STAGE/"analysis/attempt_worker.py")
    worker.jax_guard.install_if_missing()
    m=worker._kernel()
    if f05.sha(worker.KERNEL_PATH)!=ORIGINAL_KERNEL_SHA:
        raise RuntimeError("ORIGINAL_26EVENT_KERNEL_CHANGED")
    decoded,receipt=edge.source_state(owner/edge.REL_STAGE,
                                      "LOCAL_NEUTRAL_HAZARD_PRIMARY",worker.state_io)
    base,_,_,nh,nhe=worker._context(m,"LOCAL_NEUTRAL_HAZARD_PRIMARY")
    tick_seconds=float(base.forcing.duration_seconds(0))/131072.0
    if not(np.isfinite(tick_seconds) and tick_seconds>0.0 and len(nh)==N and len(nhe)==N):
        raise RuntimeError("ORIGINAL_TICK_SECONDS_OR_CELL_CONTEXT_INVALID")
    if receipt["archive_sha256"]!=ORIGINAL_SOURCE_SHA or receipt["accepted_array_sha256"]!=ORIGINAL_ARRAY_SHA:
        raise RuntimeError("ACCEPTED_SOURCE_ARRAY_DRIFT")
    lo=np.asarray(decoded.population_lower,dtype=np.float64)
    hi=np.asarray(decoded.population_upper,dtype=np.float64)
    logtlo=np.asarray(decoded.log_temperature_lower,dtype=np.float64)
    logthi=np.asarray(decoded.log_temperature_upper,dtype=np.float64)
    if lo.shape!=(N,5) or hi.shape!=(N,5) or logtlo.shape!=(N,) or logthi.shape!=(N,):
        raise RuntimeError("ACCEPTED_PARENT_WRONG_SHAPE")
    if not(np.all(np.isfinite(lo)) and np.all(np.isfinite(hi)) and np.all(lo>0)
           and np.all(lo<=hi) and np.all(np.isfinite(logtlo))
           and np.all(np.isfinite(logthi)) and np.all(logtlo<=logthi)):
        raise RuntimeError("ACCEPTED_PARENT_INVALID_INTERVAL")
    middle=(lo+hi)*0.5
    check=np.ascontiguousarray(middle[:,[0,2,3]],dtype=np.float64)
    if check.tobytes()!=np.ascontiguousarray(old_n).tobytes():
        raise RuntimeError("REAL_ORIGINAL_NEUTRAL_MIDPOINT_NOT_IDENTICAL")
    kb=np.float64(m.rim.KB_ERG)
    if not np.isfinite(kb) or kb<=0:
        raise RuntimeError("ORIGINAL_KB_INVALID")
    raw=bytearray(MAGIC+N.to_bytes(8,"little"))
    raw.extend(np.asarray([kb],dtype="<f8").tobytes())
    for x in (lo,hi,logtlo,logthi):
        raw.extend(np.ascontiguousarray(x,dtype="<f8").tobytes())
    raw=bytes(raw)
    expected_bytes=24+8*12*N
    if len(raw)!=expected_bytes:
        raise RuntimeError("THERMAL_STAGE_FIXTURE_WRONG_BYTE_LENGTH")
    out.parent.mkdir(parents=True,exist_ok=True)
    with out.open("wb") as target:
        with gzip.GzipFile(fileobj=target,mode="wb",filename="",mtime=0,compresslevel=6) as z:
            z.write(raw)
    report={
      "schema":"REI_ORIGINAL_F05_ACCEPTED_T160_HHE_THERMAL_FIVEPOP_INTERVAL_V1",
      "owner_head":OWNER_HEAD,
      "original_source_tar_sha256":ORIGINAL_SOURCE_SHA,
      "original_array_sha256":ORIGINAL_ARRAY_SHA,
      "original_26event_kernel_sha256":ORIGINAL_KERNEL_SHA,
      "compatible_native_photon_raw_sha256":ORIGINAL_PHOTON_RAW_SHA,
      "cells":N,"population_order":["HI","HII","HeI","HeII","HeIII"],
      "population_unit":"original_cell_number_count",
      "thermal_unit":"Kelvin",
      "original_kb_erg_per_kelvin":float(kb),
      "tick":"160/131072",
      "source_original_tick160_to161_duration_seconds":tick_seconds,
      "description":"original accepted 5-pop intervals+logtemperature intervals only; NO owner solver step",
      "nHI_HeI_HeII_parent_midpoints_exact_match_existing_native_fixture":True,
      "format":"magic8 N_u64 kB_f64 pop_low_Nx5 pop_high_Nx5 logT_low_N logT_high_N all little-endian f64",
      "gzip_sha256":digest(out.read_bytes()),"raw_sha256":digest(raw),
      "raw_bytes":len(raw),"gz_bytes":out.stat().st_size,
      "parent_population_min":float(np.min(lo)),
      "parent_temperature_min_K":float(np.min(np.exp(logtlo))),
      "parent_temperature_max_K":float(np.max(np.exp(logthi))),
      "old_accepted_only":True,"tick161":"NOT_ACCEPTED","canonical_delta":0,
      "production":"HOLD"
    }
    manifest.write_text(json.dumps(report,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:report[k] for k in
        ("cells","original_kb_erg_per_kelvin","gzip_sha256","raw_sha256",
         "parent_temperature_min_K","parent_temperature_max_K")}))
    return report

if __name__=="__main__":
    ap=argparse.ArgumentParser()
    ap.add_argument("--owner",type=Path,required=True)
    ap.add_argument("--old",type=Path,required=True)
    ap.add_argument("--out",type=Path,required=True)
    ap.add_argument("--manifest",type=Path,required=True)
    args=ap.parse_args()
    extract(args.owner.resolve(),args.old.resolve(),args.out.resolve(),args.manifest.resolve())
