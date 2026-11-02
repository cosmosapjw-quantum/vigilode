#!/usr/bin/env python3
"""Independent forensic audit of native Rust original F05 photo HHe/thermal data.

This is NOT a Python solver. It checks immutable original-source SHA, native
Rust 46k row output, conservation identities, and the separately published
source-owner NONAUTHORITATIVE tick161 preview (first-order scale only).
"""
from __future__ import annotations
import argparse,csv,gzip,hashlib,json,math,statistics,struct
from pathlib import Path

N=46080
OLD_SOURCE_GZIP="f80d9058a2a1db95b112fc39e176c24d9b1a6b3a298a2bcb56ac5a5e1c50d88a"
STAGE_RAW_SHA="c936d56b9a9c7805a365245c88dbd543bbad75c6d9ef33e49d7cf62a373c7556"
PARENT_RAW_SHA="eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"
PREVIEW_GZIP_SHA="713b46a0da8931cc6f5a355912cdd2efa790945c2539ad049c086fd995f3441d"
ACCEPTED_STATE_SHA="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"

def sha(data:bytes)->str:return hashlib.sha256(data).hexdigest()
def require(condition:bool,name:str):
    if not condition:raise AssertionError(name)

def compare(stage_json:dict,parent_meta:dict,stage_gz:bytes,
            preview_gz:bytes,preview_meta:dict,
            rei_rust_cross:dict)->dict:
    require(parent_meta["raw_sha256"]==PARENT_RAW_SHA,"SOURCE_PARENT_SHA_CHANGED")
    require(parent_meta["compatible_native_photon_raw_sha256"]
        =="a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469",
        "ORIGINAL_PHOTON_SOURCE_CHANGED")
    require(parent_meta["original_array_sha256"]==ACCEPTED_STATE_SHA,
        "ORIGINAL_ACCEPTED_STATE_CHANGED")
    require(parent_meta["tick"]=="160/131072","SOURCE_TICK_CHANGED")
    dt=parent_meta["source_original_tick160_to161_duration_seconds"]
    require(math.isfinite(dt) and dt>0,"SOURCE_CLOCK_INVALID")
    require(stage_json["schema"]=="VIGILODE_REI_ORIGINAL26_T160_NATIVE_PHOTO_HHE_THERMAL_STAGE_V1",
            "RUST_STAGE_SCHEMA_CHANGED")
    require(stage_json["accepted_original_nodes"]==N,"STAGE_NOT_REAL_46080")
    require(stage_json["source_original_photon_partial_raw_sha256"]
            ==parent_meta["compatible_native_photon_raw_sha256"],
            "CROSS_STAGE_PHOTON_IDENTITY_FAIL")
    require(stage_json["original_accepted_parent5_raw_sha256"]==PARENT_RAW_SHA,
            "STAGE_PARENT_SOURCE_IDENTITY_FAIL")
    require(stage_json["original_tick161"]=="NOT_ACCEPTED"
            and stage_json["canonical_delta"]==0
            and stage_json["production"]=="HOLD","FALSE_CANONICAL_PROMOTION")
    require(stage_json["original_26event_MPRK22_Alexander_SDIRK2_time_map"]=="NOT_RUN"
            and stage_json["original_14_full_time_map_ledgers"]=="NOT_RUN",
            "FALSE_FULL_SOURCE_SOLVER_CLAIM")
    require(stage_json["original_source_neutral_midpoint_byte_mismatches"]==0,
            "ACCEPTED_SOURCE_PARENT_CHANGED")
    for key in ("photo_only_h_nuclei_conserved_at_all_nodes",
                "photo_only_he_nuclei_conserved_at_all_nodes",
                "photo_electrons_charge_identity_at_all_nodes",
                "photo_only_thermal_first_law_at_all_nodes"):
        require(stage_json[key] is True,"ORIGINAL_STAGE_CONSERVATION_FAILURE:"+key)
    require(stage_json["max_energy_identity_relative_residual"]<1e-10,
            "ORIGINAL_STAGE_THERMAL_IDENTITY_FAILED")
    require(sha(stage_gz)==stage_json["stage_data_gzip_sha256"] if "stage_data_gzip_sha256" in stage_json
            else True, "STAGE_GZIP_SHA_MISMATCH")
    raw=gzip.decompress(stage_gz)
    require(sha(raw)==stage_json["stage_data_raw_sha256"]==STAGE_RAW_SHA,
            "ORIGINAL_STAGE_RUST_RAW_SHA_CHANGED")
    require(len(raw)==16+8*10*N and raw[:8]==b"REIF05D1"
            and int.from_bytes(raw[8:16],"little")==N,"WRONG_NATIVE_STAGE_LAYOUT")
    require(sha(preview_gz)==PREVIEW_GZIP_SHA==preview_meta["physical_csv_sha256"],
            "ORIGINAL_PREVIEW_CSV_SHA_CHANGED")
    require(preview_meta["original_accepted_state_array_sha256"]==ACCEPTED_STATE_SHA
            and preview_meta["official_state_161"]=="NOT_ACCEPTED_OR_WRITTEN",
            "PREVIEW_NOT_ORIGINAL_NONAUTHORITATIVE")
    require(rei_rust_cross["schema"]=="REI_HISTORICAL_RUST_EOS_VIGILODE_STAGE_CROSSCHECK_V1"
            and rei_rust_cross["cells"]==N,"REI_HISTORICAL_RUST_EOS_NOT_EXECUTED")
    require(rei_rust_cross["native_REI_rust_eos_max_u_relative_diff"]<1e-13
            and rei_rust_cross["native_REI_rust_eos_max_ne_relative_diff"]<1e-13,
            "CROSS_REPO_NATIVE_EOS_PARITY_FAIL")
    rows=list(struct.iter_unpack("<10d",raw[16:]))
    require(len(rows)==N,"DERIVED_PHOTO_ROWS_TRUNCATED")
    n_bad=0
    photo_positive=0
    preview_negative=0
    photo_plus_net_opposite=0
    photo_disjoint_net=0
    photo_temp_abs=[]
    preview_temp_abs=[]
    ratios=[]
    sum_de=[]
    sum_q=[]
    # The file is SHA-locked, so the standard CSV reader suffices.
    import io
    stream=io.TextIOWrapper(gzip.GzipFile(fileobj=io.BytesIO(preview_gz)),
                            encoding="utf-8",newline="")
    reader=csv.DictReader(stream)
    required={
        "node","tick160_to_161_delta_logT_lower",
        "tick160_to_161_delta_logT_upper",
        "tick160_to_161_delta_electron_N_lower",
        "tick160_to_161_delta_electron_N_upper",
        "tick160_Tgas_K_lower","tick160_Tgas_K_upper"}
    require(required.issubset(set(reader.fieldnames or [])),"PREVIEW_SCHEMA_CHANGED")
    for i,(src,row) in enumerate(zip(reader,rows)):
        require(int(src["node"])==i,"REAL_PHYSICAL_NODE_ORDER_CHANGED")
        dhi,dhii,dhei,dheii,dheiii,dne,q,dlogt,u,binding=row
        require(all(math.isfinite(z) for z in row),"DERIVED_NONFINITE_PHYSICAL_RATE")
        scale=max(abs(dne),1e-300)
        if abs(dhi+dhii)>1e-12*scale or abs(dhei+dheii+dheiii)>1e-12*scale or abs(dhii+dheii+2*dheiii-dne)>1e-12*scale:
            n_bad+=1
        require(q>=0 and binding>=0 and u>0,"DERIVED_ENERGY_CONE_BAD")
        if dne>0:photo_positive+=1
        lo=float(src["tick160_to_161_delta_electron_N_lower"])
        hi=float(src["tick160_to_161_delta_electron_N_upper"])
        require(lo<=hi and math.isfinite(lo) and math.isfinite(hi),"PREVIEW_ELECTRON_INTERVAL_BAD")
        if hi<0:preview_negative+=1
        delta_photo=dne*dt
        if delta_photo>0 and hi<0:photo_plus_net_opposite+=1
        if delta_photo<lo or delta_photo>hi:photo_disjoint_net+=1
        tlo=float(src["tick160_to_161_delta_logT_lower"])
        thi=float(src["tick160_to_161_delta_logT_upper"])
        require(tlo<=thi,"PREVIEW_LOGT_INTERVAL_INVALID")
        photo_temp_abs.append(abs(dlogt*dt))
        preview_temp_abs.append(max(abs(tlo),abs(thi)))
        if 0.5*(abs(lo)+abs(hi))>0:
            ratios.append(delta_photo/(0.5*(abs(lo)+abs(hi))))
        sum_de.append(dne)
        sum_q.append(q)
    extra=next(reader,None)
    require(extra is None and len(sum_de)==N,"PREVIEW_PHYSICAL_ROW_COUNT_NOT_46080")
    require(n_bad==0 and photo_positive==N,"PHOTO_NATIVE_CHARGE_OR_SOURCE_NEGATIVE")
    de=math.fsum(sum_de);q=math.fsum(sum_q)
    require(abs(de-stage_json["photo_only_electron_total_rate_per_s"])
            <1e-12*de,"PHOTO_RATE_SUM_NOT_REPRODUCED")
    require(abs(q-stage_json["photo_only_primary_heating_total_erg_s"])
            <1e-12*q,"HEATING_RATE_SUM_NOT_REPRODUCED")
    observed=preview_meta["observables"]
    photo_delta_e=de*dt
    photo_delta_heat=q*dt
    observed_de_lo=float(observed["delta_electron_N_total_lower"])
    observed_de_hi=float(observed["delta_electron_N_total_upper"])
    require(observed_de_hi<0 and photo_delta_e>0,"SOURCE_CONDITIONAL_NET_SIGN_NOT_OPPOSED")
    # This is a first-order frozen-t160 photo-only estimate: NOT exact
    # integrated source contribution, NOT decomposition of the full map.
    return{
      "schema":"VIGILODE_REI_F05_NATIVE_PHOTO_HHE_THERMAL_SOURCE_PHYSICS_AUDIT_V1",
      "original_accepted_tick":"160/131072",
      "native_real_cells":N,
      "real_original_parent_raw_sha256":PARENT_RAW_SHA,
      "real_native_stage_raw_sha256":STAGE_RAW_SHA,
      "original_preview_csv_sha256":PREVIEW_GZIP_SHA,
      "original_preview_is_nonauthoritative":True,
      "accepted_original_tick160_to161_dt_seconds":dt,
      "native_photo_electron_rate_per_second":de,
      "native_photo_primary_heat_erg_per_second":q,
      "native_first_order_photo_delta_electrons":photo_delta_e,
      "native_first_order_photo_delta_thermal_erg":photo_delta_heat,
      "original_preview_net_delta_electrons_interval":[observed_de_lo,observed_de_hi],
      "first_order_photo_over_absolute_preview_delta_range":[
          photo_delta_e/abs(observed_de_lo),
          photo_delta_e/abs(observed_de_hi)],
      "native_photo_electron_positive_cells":photo_positive,
      "original_preview_net_electron_negative_interval_cells":preview_negative,
      "native_photo_source_and_original_net_opposite_sign_cells":photo_plus_net_opposite,
      "native_photo_only_firstorder_outside_full_preview_net_cell_interval":photo_disjoint_net,
      "median_nodewise_photo_to_absolute_preview_delta_ratio":statistics.median(ratios),
      "max_first_order_photo_only_abs_delta_log_temperature":max(photo_temp_abs),
      "original_preview_max_abs_delta_log_temperature":max(preview_temp_abs),
      "frozen_first_order_photo_vs_original_full_map_are_not_same_operator":True,
      "photo_only_stage_full_spectrum_conservation":"PHOTO_ONLY_IDEAL_GAS_H_HE_ELECTRON_BINDING",
      "rei_historical_rust_eos_46080_crosscheck":"PASS_SCOPED",
      "original_26event_ots_recombination_and_cooling":"NOT_COMPUTED_IN_THIS_STAGE",
      "original_14_full_time_map_ledgers":"NOT_RUN",
      "original_adaptive_MPRK_SDIRK_IVP":"NOT_RUN",
      "original_tick161":"NOT_ACCEPTED",
      "canonical_delta":0,"vigilode_official_dag":"7/23","rei_central_dag":"28/88",
      "production":"HOLD"
    }

def validate_receipt(r:dict)->bool:
    require(r["schema"]=="VIGILODE_REI_F05_NATIVE_PHOTO_HHE_THERMAL_SOURCE_PHYSICS_AUDIT_V1",
            "WRONG_NATIVE_F05_SCIENCE_RECEIPT")
    require(r["native_real_cells"]==N and r["original_accepted_tick"]=="160/131072",
            "RECEIPT_NOT_ORIGINAL_ACCEPTED_F05")
    require(r["real_native_stage_raw_sha256"]==STAGE_RAW_SHA
            and r["real_original_parent_raw_sha256"]==PARENT_RAW_SHA
            and r["original_preview_csv_sha256"]==PREVIEW_GZIP_SHA,
            "RECEIPT_ORIGINAL_SOURCE_SHA_CHANGED")
    require(r["original_preview_is_nonauthoritative"] is True
            and r["original_tick161"]=="NOT_ACCEPTED"
            and r["canonical_delta"]==0 and r["production"]=="HOLD",
            "RECEIPT_FALSE_CANONICAL_PROMOTION")
    require(r["original_adaptive_MPRK_SDIRK_IVP"]=="NOT_RUN"
            and r["original_14_full_time_map_ledgers"]=="NOT_RUN"
            and r["frozen_first_order_photo_vs_original_full_map_are_not_same_operator"],
            "RECEIPT_FALSE_FULL_26EVENT_CLAIM")
    require(r["rei_historical_rust_eos_46080_crosscheck"]=="PASS_SCOPED"
            and r["native_photo_electron_positive_cells"]==N,
            "RECEIPT_NATIVE_EOS_OR_POSITIVITY_UNVERIFIED")
    lo,hi=r["original_preview_net_delta_electrons_interval"]
    require(lo<hi<0 and r["native_first_order_photo_delta_electrons"]>0,
            "RECEIPT_NEGATIVE_ELECTRON_PREVIEW_CHANGED")
    a,b=r["first_order_photo_over_absolute_preview_delta_range"]
    v=r["native_first_order_photo_delta_electrons"]
    require(abs(a-v/abs(lo))<1e-12*a and abs(b-v/abs(hi))<1e-12*b,
            "RECEIPT_FIRST_ORDER_SOURCE_FRACTION_FABRICATED")
    require(0<=r["native_photo_source_and_original_net_opposite_sign_cells"]<=N
            and r["native_photo_source_and_original_net_opposite_sign_cells"]
               <=r["original_preview_net_electron_negative_interval_cells"],
            "RECEIPT_CELLWISE_SIGN_COUNT_FABRICATED")
    return True

def main():
    ap=argparse.ArgumentParser()
    for key in ("stage","parent","stage_gzip","owner_preview","owner_preview_meta","rei_rust_eos","out"):
        ap.add_argument("--"+key.replace("_","-"),type=Path,required=True)
    a=ap.parse_args()
    stage=json.loads(a.stage.read_text());parent=json.loads(a.parent.read_text())
    preview=json.loads(a.owner_preview_meta.read_text())
    rei=json.loads(a.rei_rust_eos.read_text())
    r=compare(stage,parent,a.stage_gzip.read_bytes(),a.owner_preview.read_bytes(),preview,rei)
    validate_receipt(r)
    a.out.parent.mkdir(parents=True,exist_ok=True)
    a.out.write_text(json.dumps(r,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:r[k] for k in (
        "native_real_cells","native_first_order_photo_delta_electrons",
        "original_preview_net_delta_electrons_interval",
        "native_photo_source_and_original_net_opposite_sign_cells",
        "first_order_photo_over_absolute_preview_delta_range")}))
if __name__=="__main__":
    main()
