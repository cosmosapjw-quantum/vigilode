#!/usr/bin/env python3
"""Export REAL ORIGINAL 46,080-cell F05 frozen t160 interval event oracle.

This Python is a READ-ONLY SOURCE OWNER DECODER only. It runs the original
event_box and thermal_rhs interval owner for original accepted tick160. New
physical rate calculations and numerical comparisons run in separate native Rust.
No alternate original time evolution, no new physics, no accepted tick161.
"""
from __future__ import annotations
import argparse,gzip,hashlib,importlib.util,json,subprocess,sys
from pathlib import Path
import numpy as np
N=46080
LOCK_OWNER="fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
LOCK_KERNEL="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
LOCK_SOURCE="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
LOCK_STATE="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
LOCK_PARENT_GZIP="751ad44b101cce78895804818dff5127ff6b905273527fa4afff45144c4c4b16"
LOCK_PHOTON_GZIP="f80d9058a2a1db95b112fc39e176c24d9b1a6b3a298a2bcb56ac5a5e1c50d88a"
MAGIC=b"REIF05E1"
EXTRAS=("resolved_heat","unresolved_energy","escaped_energy",
        "chemical_energy","photo_heat","photon_identity","thermal_rhs")


def digest(raw:bytes)->str:return hashlib.sha256(raw).hexdigest()

def broadcast_box(box):
    lo=np.asarray(box.lo,dtype=np.float64)
    hi=np.asarray(box.hi,dtype=np.float64)
    low=np.ascontiguousarray(np.broadcast_to(lo,(N,)),dtype=np.float64)
    high=np.ascontiguousarray(np.broadcast_to(hi,(N,)),dtype=np.float64)
    if not(np.all(np.isfinite(low)) and np.all(np.isfinite(high))
           and np.all(low<=high)):
        raise ArithmeticError("INVALID_ORIGINAL_NONPHOTO_SOURCE_INTERVAL")
    return low,high

def encoded(arr):
    a=np.ascontiguousarray(arr,dtype="<f8")
    if not np.all(np.isfinite(a)):
        raise ValueError("NONFINITE_ORIGINAL_EVENT_SOURCE")
    return a.tobytes()

def run(owner:Path,parent:Path,photon:Path,out:Path,metadata:Path):
    if subprocess.check_output(["git","-C",str(owner),"rev-parse","HEAD"],text=True).strip()!=LOCK_OWNER:
        raise ArithmeticError("ORIGINAL_REI_F05_OWNER_SHA_DRIFT")
    if digest(parent.read_bytes())!=LOCK_PARENT_GZIP or digest(photon.read_bytes())!=LOCK_PHOTON_GZIP:
        raise ArithmeticError("ORIGINAL_F05_RUST_PARENT_OR_PHOTON_DRIFT")
    sys.path.insert(0,str(owner))
    fpath=owner/"research/rei_f05_common_parent_increments_20261101/increment_certificate.py"
    spec=importlib.util.spec_from_file_location("original_f05_nonphoto_decompiler",fpath)
    m=importlib.util.module_from_spec(spec);sys.modules[spec.name]=m;spec.loader.exec_module(m)
    edge=m.load("original_f05_nonphoto_edge",owner/"research/rei_igm_original26_temperature_edges_20261101/edge_probe.py")
    worker=m.load("original_f05_nonphoto_worker",owner/edge.REL_STAGE/"analysis/attempt_worker.py")
    worker.jax_guard.install_if_missing()
    kernel=worker._kernel()
    if m.sha(worker.KERNEL_PATH)!=LOCK_KERNEL:
        raise ArithmeticError("ORIGINAL_F05_SOURCE_KERNEL_DRIFT")
    decoded,source=edge.source_state(owner/edge.REL_STAGE,
        "LOCAL_NEUTRAL_HAZARD_PRIMARY",worker.state_io)
    if source["archive_sha256"]!=LOCK_SOURCE or source["accepted_array_sha256"]!=LOCK_STATE:
        raise ArithmeticError("ORIGINAL_F05_ACCEPTED_PARENT_CHANGED")
    base,_a,_b,nH,nHe=worker._context(kernel,"LOCAL_NEUTRAL_HAZARD_PRIMARY")
    if len(nH)!=N or len(nHe)!=N:
        raise ArithmeticError("ORIGINAL_F05_NOT_46080")
    model=kernel.rim.ReducedIntervalModel.from_repo(owner)
    duration=float(base.forcing.duration_seconds(0))
    t=duration*(160/131072.0)
    pop=kernel._I(decoded.population_lower,decoded.population_upper)
    x=kernel._I(decoded.log_temperature_lower,decoded.log_temperature_upper)
    T=kernel.iv.exp(x)
    forcing=model.forcing_bounds(t,t)
    v=kernel._source_v(T)
    table_event=kernel.primitive.detect_table_events(decoded.log_temperature_lower,
                                                      decoded.log_temperature_upper)
    event=kernel.event_box(model,pop,T,t)
    thermal=kernel.thermal_context(pop,forcing.volume_cm3,event.photo_heat,
                                   forcing.hubble_s_inv)
    heat_rhs=kernel.thermal_rhs(thermal,x)
    event_map={
      "resolved_heat":event.resolved_heat,
      "unresolved_energy":event.unresolved_energy,
      "escaped_energy":event.escaped_energy,
      "chemical_energy":event.chemical_energy,
      "photo_heat":event.photo_heat,
      "photon_identity":event.photon_identity,
      "thermal_rhs":heat_rhs
    }
    flux_lo=np.ascontiguousarray(event.flux.lo,dtype="<f8")
    flux_hi=np.ascontiguousarray(event.flux.hi,dtype="<f8")
    if flux_lo.shape!=(N,5,5) or flux_hi.shape!=(N,5,5):
        raise ArithmeticError("ORIGINAL_F05_25_FLUX_SHAPE_FAILED")
    if not(np.all(np.isfinite(flux_lo)) and np.all(np.isfinite(flux_hi))
           and np.all(flux_lo<=flux_hi)):
        raise ArithmeticError("ORIGINAL_26EVENT_FLUX_BOX_NONFINITE_OR_REVERSED")
    extras_lo=np.column_stack([broadcast_box(event_map[k])[0] for k in EXTRAS])
    extras_hi=np.column_stack([broadcast_box(event_map[k])[1] for k in EXTRAS])
    vol_lo,vol_hi=broadcast_box(forcing.volume_cm3)
    hub_lo,hub_hi=broadcast_box(forcing.hubble_s_inv)
    v_lo,v_hi=broadcast_box(v)
    if not(np.all(vol_lo>0) and np.all(v_lo>=0) and np.all(v_hi<=1)
            and np.all(hub_lo>=0)):
        raise ArithmeticError("ORIGINAL_F05_FORCING_DOMAIN_FAILED")
    raw=bytearray(MAGIC+int(N).to_bytes(8,"little"))
    for part in (vol_lo,vol_hi,hub_lo,hub_hi,v_lo,v_hi,
                 flux_lo,flux_hi,extras_lo,extras_hi):
        raw.extend(encoded(part))
    raw=bytes(raw)
    expected=16+8*N*(6+25*2+7*2)
    if len(raw)!=expected:raise AssertionError(("BINARY_SIZE_ERROR",len(raw),expected))
    out.parent.mkdir(parents=True,exist_ok=True)
    with out.open("wb") as ff:
        with gzip.GzipFile(fileobj=ff,mode="wb",filename="",mtime=0,compresslevel=6) as z:z.write(raw)
    obj={
      "schema":"ORIGINAL_REI_F05_T160_NATIVE_RUST_NONPHOTO_EVENT_ENCLOSURE_V1",
      "original_owner_head":LOCK_OWNER,"original_source_tar_sha256":LOCK_SOURCE,
      "original_accepted_array_sha256":LOCK_STATE,"original_kernel_sha256":LOCK_KERNEL,
      "accepted_tick":"160/131072","source_time_seconds":t,"source_step_seconds":duration/131072,
      "n_real_cells":N,"parent_gzip_sha256":LOCK_PARENT_GZIP,
      "photon_gzip_sha256":LOCK_PHOTON_GZIP,
      "original_source_v_uncertainty":"point witness v_mid, f_mid=0.55; bounds exported",
      "extra_order":list(EXTRAS),
      "binary_format":"REIF05E1 u64N then le_f64 vol_lo,vol_hi,Hubble_lo,hi,v_lo,hi (N each); flux_lo,hi (N,5,5 each); energy_7_lo,hi (N,7 each)",
      "raw_sha256":digest(raw),"raw_bytes":len(raw),
      "gzip_sha256":digest(out.read_bytes()),"gzip_bytes":out.stat().st_size,
      "min_volume_cm3":float(np.min(vol_lo)),
      "max_volume_cm3":float(np.max(vol_hi)),
      "max_hubble_s_inv":float(np.max(hub_hi)),
      "original_flux_25_nonnegative_columns":int(np.sum(np.any(flux_hi>0,axis=0))),
      "owner_all_interval_rows_finite":True,
      "original_source_t160_table_event_any":bool(table_event.any_event),
      "original_source_t160_table_event_nodes":int(len(np.unique(table_event.node_indices))),
      "original_source_t160_table_min_logT_knot_distance":float(table_event.minimum_distance),
      "native_full_26event_ode":"NOT_RUN","original_14_final_ledgers":"NOT_RUN",
      "original_tick161":"NOT_ACCEPTED","production":"HOLD","canonical_delta":0
    }
    metadata.parent.mkdir(parents=True,exist_ok=True)
    metadata.write_text(json.dumps(obj,sort_keys=True,indent=2,allow_nan=False)+"\n")
    print(json.dumps({k:obj[k] for k in ("n_real_cells","source_time_seconds","min_volume_cm3",
        "max_volume_cm3","max_hubble_s_inv","original_flux_25_nonnegative_columns",
        "raw_sha256","gzip_sha256","raw_bytes","gzip_bytes")}))
    return obj

if __name__=="__main__":
    a=argparse.ArgumentParser()
    for k in ("owner","parent","photon","out","metadata"):
        a.add_argument("--"+k,type=Path,required=True)
    x=a.parse_args()
    run(x.owner.resolve(),x.parent.resolve(),x.photon.resolve(),x.out.resolve(),x.metadata.resolve())
