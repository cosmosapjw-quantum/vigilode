#!/usr/bin/env python3
"""Read-only SOURCE FIXTURE EXPORT for native Rust VigilODE, not a solver.

Original REI 26-event accepted tick160 source kernel is executed only to decode
its immutable source/state and original interval photo bounds. The native
photon/JVP operations are in Rust. Never serialize a tick161 accepted state.
"""
from __future__ import annotations
import argparse
import gzip
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

LOCKED_OWNER = "fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
ORIGINAL_KERNEL = "579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
ORIGINAL_SOURCE = "a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
ORIGINAL_STATE = "cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
NODES=46080
MAGIC=b"REIF05R1"


def binary(arr):
    a=np.ascontiguousarray(arr,dtype="<f8")
    if not np.all(np.isfinite(a)):
        raise ValueError("NONFINITE_SOURCE_FIXTURE")
    return a.tobytes()


def run(owner:Path, output:Path, metadata:Path):
    if subprocess.check_output(["git","-C",str(owner),"rev-parse","HEAD"],text=True).strip()!=LOCKED_OWNER:
        raise ArithmeticError("ORIGINAL_OWNER_HEAD_CHANGED")
    sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"rei_igm_solver_work_precision_20261101"))
    from original26_photon_group_fusion import original_source_inputs
    n,j,e,a,excess,ev,owner_boxes=original_source_inputs(owner,"LOCAL_NEUTRAL_HAZARD_PRIMARY")
    if n.shape!=(NODES,3) or np.shape(a)!=(4,3) or np.shape(excess)!=(3,4):
        raise ValueError("NOT_ORIGINAL_26EVENT_F05_SOURCE")
    if not (np.all(n>0) and np.all(np.asarray(j)>=0) and np.all(np.asarray(e)>=0)
            and np.all(np.asarray(a)>=0)):
        raise ValueError("ORIGINAL_INPUT_CONE_VIOLATION")
    a_ld=np.asarray(a,dtype=np.longdouble)
    den=np.asarray(e,dtype=np.longdouble)+a_ld@np.sum(n,dtype=np.longdouble,axis=0)
    if not np.all(den>0):
        raise ArithmeticError("ORIGINAL_PHOTON_DEN_NONPOSITIVE")
    hi_ll=np.zeros(len(n),dtype=np.longdouble)
    for g in range(4):
        hi_ll+=np.longdouble(j[g])*a_ld[g,2]*np.asarray(n[:,2],dtype=np.longdouble)/den[g]
    if not np.all(hi_ll>0):
        raise ArithmeticError("ORIGINAL_POSITIVE_HEII_TRACE_LOST")
    hi_ll_total=np.sum(hi_ll,dtype=np.longdouble)
    boxes=tuple(owner_boxes)
    if len(boxes)!=4:
        raise ArithmeticError("ORIGINAL_OWNER_INTERVAL_COUNT_NOT_FOUR")
    rate_low=np.column_stack([np.asarray(boxes[s].lo,dtype=np.float64) for s in range(3)])
    rate_high=np.column_stack([np.asarray(boxes[s].hi,dtype=np.float64) for s in range(3)])
    heat_low=np.asarray(boxes[3].lo,dtype=np.float64)
    heat_high=np.asarray(boxes[3].hi,dtype=np.float64)
    if np.any(rate_low>rate_high) or np.any(heat_low>heat_high):
        raise ArithmeticError("ORIGINAL_INTERVAL_BOX_INVALID")
    header=MAGIC+int(NODES).to_bytes(8,"little")
    raw=bytearray(header)
    for part in (j,e,a,excess,np.asarray([ev]),n,rate_low,rate_high,heat_low,heat_high):
        raw.extend(binary(part))
    raw=bytes(raw)
    expected_len=16+8*(4+4+12+12+1+NODES*(3+3+3+1+1))
    if len(raw)!=expected_len:
        raise AssertionError(("FIXTURE_BINARY_LENGTH",len(raw),expected_len))
    output.parent.mkdir(parents=True,exist_ok=True)
    metadata.parent.mkdir(parents=True,exist_ok=True)
    with output.open("wb") as stream:
        with gzip.GzipFile(fileobj=stream,mode="wb",filename="",mtime=0,compresslevel=6) as z:
            z.write(raw)
    output_sha=hashlib.sha256(output.read_bytes()).hexdigest()
    report={
      "schema":"VIGILODE_REAL_REI_ORIGINAL26_T160_RUST_PHOTON_PARTIAL_FIXTURE_V1",
      "source_kind":"ONLY_FROZEN_ACCEPTED_T160_SOURCE_STAGE_0_NO_TIME_INTEGRATION",
      "original_owner_head":LOCKED_OWNER,
      "original_26event_kernel_sha256":ORIGINAL_KERNEL,
      "original_source_tar_sha256":ORIGINAL_SOURCE,
      "original_state_array_sha256":ORIGINAL_STATE,
      "original_source_lane":"LOCAL_NEUTRAL_HAZARD_PRIMARY",
      "accepted_tick":"160/131072",
      "original_cells":NODES,
      "absorber_species":["H_I","He_I","He_II"],
      "photon_groups":4,
      "global_neutral_moments":3,
      "current_J":[float(x) for x in j],
      "external_subgrid":[float(x) for x in e],
      "alpha_f64":[[float(z) for z in row] for row in a],
      "excess_eV":[[float(z) for z in row] for row in excess],
      "ev_to_erg":float(ev),
      "f64_heii_channel_underflow_is_not_physical_zero":True,
      "heii_positive_cells_in_extended_precision":int(np.count_nonzero(hi_ll>0)),
      "heii_total_extended_decimal":str(hi_ll_total),
      "heii_total_extended_log10":float(np.log10(hi_ll_total)),
      "source_original_interval_checks_exported":["H_I","He_I","He_II","heat"],
      "binary_layout":"magic8_u64_N_then_le_f64_J4_e4_alpha4x3_excess3x4_ev1_nNx3_rate_loNx3_rate_hiNx3_heat_loN_heat_hiN",
      "raw_binary_bytes":len(raw),
      "raw_binary_sha256":hashlib.sha256(raw).hexdigest(),
      "gzip_archive_bytes":output.stat().st_size,
      "gzip_archive_sha256":output_sha,
      "canonical_delta":0,"production":"HOLD",
      "official_tick161":"NOT_ACCEPTED",
      "is_rust_solver":"FALSE_SOURCE_EXPORT_ONLY"
    }
    metadata.write_text(json.dumps(report,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:report[k] for k in ("original_cells","raw_binary_bytes",
                 "raw_binary_sha256","gzip_archive_sha256","heii_total_extended_decimal",
                 "heii_positive_cells_in_extended_precision")}))
    return report


if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--owner",type=Path,required=True)
    parser.add_argument("--out",type=Path,required=True)
    parser.add_argument("--metadata",type=Path,required=True)
    args=parser.parse_args()
    run(args.owner.resolve(),args.out.resolve(),args.metadata.resolve())
