#!/usr/bin/env python3
"""Independent source-owner interval audit of actual native Rust nonphoto data.

Stdlib-only: no Python ODE/physics RHS evaluation. Consumes the SOURCE-EXACT
REI F05 46,080-cell original event interval oracle, native 40-column
full-event source point state, earlier immutable accepted 5-population source
and the original nonauthoritative tick161 owner preview.
"""
from __future__ import annotations
import argparse,csv,gzip,hashlib,io,json,math,struct
from pathlib import Path

N=46080
EVENT_SHA="28ea21ea9e5b4210b3e8971313af6aa6a8c4a07fad10c984bfd29424967ebf0b"
ORIGINAL_KERNEL="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
ORIGINAL_PARENT_SHA="eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"
ORIGINAL_NATIVE_SHA="79344b098d441710c61f099c3daeec8acd6eebb2417d31ddbe2e30455252f8ca"
PREVIEW_SHA="713b46a0da8931cc6f5a355912cdd2efa790945c2539ad049c086fd995f3441d"
EXTRAS=("OTS_heat","unresolved","escaped","chemical","photo_plus_OTS_heat","photon_identity","thermal_rhs")

def sha(b:bytes)->str:return hashlib.sha256(b).hexdigest()
def need(ok:bool,why:str):
    if not ok:raise AssertionError(why)

def verify(native:dict,oracle_meta:dict,parent_meta:dict,
           native_raw:bytes,event_gzip:bytes,parent_gzip:bytes,
           preview_gzip:bytes,preview_meta:dict)->dict:
    need(oracle_meta["original_kernel_sha256"]==ORIGINAL_KERNEL and
         oracle_meta["n_real_cells"]==N and
         oracle_meta["accepted_tick"]=="160/131072", "ORIGINAL_SOURCE_KERNEL_OR_TICK_DRIFT")
    need(oracle_meta["original_source_t160_table_event_any"] is False
         and oracle_meta["original_source_t160_table_event_nodes"]==0,
         "ORIGINAL_SOURCE_T160_TABLE_EVENT_NOT_CLEAR")
    need(oracle_meta["raw_sha256"]==EVENT_SHA and
         sha(event_gzip)==oracle_meta["gzip_sha256"],"ORIGINAL_EVENT_FIXTURE_GZIP_SHA_DRIFT")
    raw_evt=gzip.decompress(event_gzip)
    need(sha(raw_evt)==EVENT_SHA and len(raw_evt)==16+8*70*N
         and raw_evt[:8]==b"REIF05E1","ORIGINAL_EVENT_RAW_SHA_OR_SIZE_DRIFT")
    need(sha(native_raw)==ORIGINAL_NATIVE_SHA==
         native["derived_binary_sha256"] and len(native_raw)==16+8*40*N
         and native_raw[:8]==b"REIF05N1","NATIVE_RUST_DERIVED_SHA_OR_SIZE_DRIFT")
    need(sha(gzip.decompress(parent_gzip))==ORIGINAL_PARENT_SHA
         and parent_meta["raw_sha256"]==ORIGINAL_PARENT_SHA,"ORIGINAL_FIVEPOP_STATE_SHA_DRIFT")
    need(native["schema"]=="VIGILODE_REI_F05_REAL_46080_NATIVE_FULL_EVENT_SOURCE_POINT_V1"
         and native["actual_source_nodes"]==N,"ORIGINAL_NATIVE_SOURCE_SCHEMA_WRONG")
    need(native["all_owner_interval_checks_pass"] and
         native["actual_25_original_flux_transition_checks"]==[N]*25
         and native["actual_7_original_energy_component_checks"]==[N]*7,
         "NATIVE_REPORTED_INTERVAL_FAILURE")
    need(native["original_tick161"]=="NOT_ACCEPTED"
         and native["full_original_26event_continuum_ivp"]=="NOT_RUN"
         and native["original_14_final_time_map_ledgers"]=="NOT_RUN"
         and native["production"]=="HOLD" and native["canonical_delta"]==0,
         "FALSE_FULL_IVP_OR_TICK161_ADMISSION")
    need(preview_meta["official_state_161"]=="NOT_ACCEPTED_OR_WRITTEN"
         and sha(preview_gzip)==PREVIEW_SHA,"ORIGINAL_PREVIEW_OWNER_NOT_MATCHED")
    dt=parent_meta["source_original_tick160_to161_duration_seconds"]
    need(dt>0 and abs(dt-4871848749.16828)<1e-4,"ORIGINAL_TIME_SOURCE_DRIFT")
    evt=memoryview(raw_evt)[16:]
    source=memoryview(native_raw)[16:]
    state=memoryview(gzip.decompress(parent_gzip))[24:]
    def scalar(v:memoryview,k:int)->float:
        return struct.unpack_from("<d",v,8*k)[0]
    confirmed_flux=[0]*25
    confirmed_energy=[0]*7
    source_ne=[]
    source_thermal=[]
    source_temp_delta=[]
    preview_negative=0
    native_negative=0
    same_sign_negative=0
    preview_ne_inside=0
    preview_temp_inside=0
    max_nuclear_relative=0.0
    max_thermal_identity_relative=0.0
    max_energy_identity_relative=0.0
    thermal_rhs_sources=[]
    reader=csv.DictReader(io.TextIOWrapper(gzip.GzipFile(fileobj=io.BytesIO(preview_gzip)),encoding="utf-8"))
    required=("node","tick160_to_161_delta_electron_N_lower","tick160_to_161_delta_electron_N_upper",
              "tick160_to_161_delta_logT_lower","tick160_to_161_delta_logT_upper")
    need(all(x in (reader.fieldnames or []) for x in required),"ORIGINAL_PREVIEW_COLUMN_DRIFT")
    for i,reference in enumerate(reader):
        need(i<N and int(reference["node"])==i,"PREVIEW_NODE_ORDER_OR_LENGTH_CHANGED")
        row=struct.unpack_from("<40d",source,40*8*i)
        net=row[:5];extra=row[5:12];ne=row[12];cool=row[13];lnt=row[14];flux=row[15:40]
        need(all(math.isfinite(x) for x in row),"RUST_ORIGINAL_STAGE_NONFINITE")
        for k,x in enumerate(flux):
            low=scalar(evt,6*N+25*i+k)
            high=scalar(evt,31*N+25*i+k)
            if low<=x<=high:confirmed_flux[k]+=1
        for k,x in enumerate(extra):
            low=scalar(evt,56*N+7*i+k)
            high=scalar(evt,63*N+7*i+k)
            if low<=x<=high:confirmed_energy[k]+=1
        net_ref=[math.fsum(flux[5*s+j]-flux[5*j+s] for j in range(5)) for s in range(5)]
        scale=max(math.fsum(abs(x) for x in net),1e-300)
        max_nuclear_relative=max(max_nuclear_relative,
              abs(net[0]+net[1])/scale,
              abs(net[2]+net[3]+net[4])/scale,
              max(abs(net[k]-net_ref[k]) for k in range(5))/scale,
              abs(ne-net[1]-net[3]-2*net[4])/scale)
        escale=max(abs(extra[1]),abs(extra[3]),1e-300)
        max_energy_identity_relative=max(max_energy_identity_relative,
              abs(extra[0]+extra[1]+extra[2]+extra[3])/escale)
        pop=[(scalar(state,5*i+s)+scalar(state,5*N+5*i+s))*0.5 for s in range(5)]
        temp=math.exp((scalar(state,10*N+i)+scalar(state,11*N+i))*0.5)
        n_e=pop[1]+pop[3]+2*pop[4]
        npart=math.fsum(pop)+n_e
        u=1.5*1.380649e-16*temp*npart
        expected_lnt=extra[6]/u-ne/npart
        firstlaw_error=abs(extra[6]-(u*lnt+1.5*1.380649e-16*temp*ne))
        max_thermal_identity_relative=max(max_thermal_identity_relative,
            abs(lnt-expected_lnt)/max(abs(expected_lnt),abs(lnt),1e-300),
            firstlaw_error/max(abs(extra[6]),abs(u*lnt),1e-300))
        if ne<0:native_negative+=1
        a=float(reference["tick160_to_161_delta_electron_N_lower"])
        b=float(reference["tick160_to_161_delta_electron_N_upper"])
        p=a<=ne*dt<=b
        if p:preview_ne_inside+=1
        if b<0:preview_negative+=1
        if b<0 and ne<0:same_sign_negative+=1
        alo=float(reference["tick160_to_161_delta_logT_lower"])
        ahi=float(reference["tick160_to_161_delta_logT_upper"])
        if alo<=lnt*dt<=ahi:preview_temp_inside+=1
        source_ne.append(ne)
        source_thermal.append(extra[6])
        source_temp_delta.append(lnt*dt)
        thermal_rhs_sources.append(extra[6])
    need(len(source_ne)==N,"PREVIEW_ROWS_NOT_ORIGINAL_46080")
    need(confirmed_flux==[N]*25,"INDEPENDENT_ORIGINAL_FLUX_ENCLOSURE_FAILURE")
    need(confirmed_energy==[N]*7,"INDEPENDENT_ORIGINAL_ENERGY_ENCLOSURE_FAILURE")
    need(max_nuclear_relative<1e-10 and max_thermal_identity_relative<1e-10
         and max_energy_identity_relative<1e-10,"INDEPENDENT_PHYSICAL_INVARIANT_FAILURE")
    n_total=math.fsum(source_ne)
    thermal_total=math.fsum(source_thermal)
    need(abs(n_total-native["native_total_electron_rate_per_s"])
         <1e-12*abs(n_total),"INDEPENDENT_NATIVE_ELECTRON_TOTAL_MISMATCH")
    need(abs(thermal_total-native["native_thermal_rhs_total_erg_s"])
         <1e-12*abs(thermal_total),"INDEPENDENT_NATIVE_THERMAL_TOTAL_MISMATCH")
    predicted_ne=n_total*dt
    predicted_heat=thermal_total*dt
    o=preview_meta["observables"]
    lo=float(o["delta_electron_N_total_lower"])
    hi=float(o["delta_electron_N_total_upper"])
    need(lo<=predicted_ne<=hi,"FROZEN_RUST_ELECTRON_RATE_OUTSIDE_PREVIEW_NET_GLOBAL_INTERVAL")
    return {
        "schema":"VIGILODE_REI_F05_46080_NATIVE_NONPHOTO_INDEPENDENT_SOURCE_AUDIT_V1",
        "original_frozen_tick":"160/131072","actual_original_cells":N,
        "original_source_t160_table_event_nodes":0,
        "original_owner_kernel_sha256":ORIGINAL_KERNEL,
        "original_event_raw_sha256":EVENT_SHA,
        "native_original_nonphoto_raw_sha256":ORIGINAL_NATIVE_SHA,
        "source_point_selected_f":0.55,
        "f_physical_source_interval":[0.1,1.0],
        "original_25_transition_flux_components":25,
        "independent_flux_included_cells_by_component":confirmed_flux,
        "independent_energy_included_cells_by_component":confirmed_energy,
        "max_46k_nuclear_and_charge_residual_relative":max_nuclear_relative,
        "max_46k_thermal_firstlaw_residual_relative":max_thermal_identity_relative,
        "max_46k_ots_energy_partition_residual_relative":max_energy_identity_relative,
        "native_46k_net_electron_rate_s":n_total,
        "native_46k_net_thermal_source_erg_s":thermal_total,
        "original_step_seconds":dt,
        "native_frozen_first_order_delta_electrons":predicted_ne,
        "native_frozen_first_order_delta_U_erg":predicted_heat,
        "original_nonauthoritative_preview_net_delta_electrons_interval":[lo,hi],
        "native_predicted_firstorder_electron_change_inside_preview_global_interval":True,
        "native_frozen_point_negative_electron_source_cells":native_negative,
        "original_preview_definite_negative_delta_electron_cells":preview_negative,
        "both_native_point_rate_and_full_preview_negative_cells":same_sign_negative,
        "native_frozen_rate_firstorder_electron_inside_preview_interval_cells":preview_ne_inside,
        "native_frozen_rate_firstorder_logT_inside_preview_interval_cells":preview_temp_inside,
        "native_dlogT_firstorder_min":min(source_temp_delta),
        "native_dlogT_firstorder_max":max(source_temp_delta),
        "not_a_fully_integrated_ivp":True,
        "source_v_and_f_uncertain":True,
        "original_14_time_map_ledgers":"NOT_RUN",
        "original_tick161":"NOT_ACCEPTED","production":"HOLD","canonical_delta":0,
        "vigilode_dag":"7/23","rei_central_dag":"28/88"
    }

def validate_saved_receipt(r:dict)->bool:
    need(r["schema"]=="VIGILODE_REI_F05_46080_NATIVE_NONPHOTO_INDEPENDENT_SOURCE_AUDIT_V1",
         "WRONG_NATIVE_NONPHOTO_RECEIPT")
    need(r["actual_original_cells"]==N and r["original_frozen_tick"]=="160/131072"
         and r["original_source_t160_table_event_nodes"]==0,
         "RECEIPT_NOT_ORIGINAL_F05")
    need(r["original_owner_kernel_sha256"]==ORIGINAL_KERNEL
         and r["original_event_raw_sha256"]==EVENT_SHA
         and r["native_original_nonphoto_raw_sha256"]==ORIGINAL_NATIVE_SHA,
         "RECEIPT_SOURCE_IDENTITY_DRIFT")
    need(r["independent_flux_included_cells_by_component"]==[N]*25
         and r["independent_energy_included_cells_by_component"]==[N]*7,
         "RECEIPT_ORIGINAL_OWNER_INTERVAL_EXCLUSION")
    need(r["max_46k_nuclear_and_charge_residual_relative"]<1e-10
         and r["max_46k_thermal_firstlaw_residual_relative"]<1e-10
         and r["max_46k_ots_energy_partition_residual_relative"]<1e-10,
         "RECEIPT_PHYSICAL_LEDGER_FAILURE")
    need(r["native_frozen_rate_firstorder_electron_inside_preview_interval_cells"]==N
         and r["native_frozen_rate_firstorder_logT_inside_preview_interval_cells"]==N
         and r["native_predicted_firstorder_electron_change_inside_preview_global_interval"],
         "RECEIPT_PREVIEW_INCLUSION_CLAIM_NOT_OBSERVED")
    lo,hi=r["original_nonauthoritative_preview_net_delta_electrons_interval"]
    need(lo<r["native_frozen_first_order_delta_electrons"]<hi<0,
         "RECEIPT_SOURCE_FROZEN_PREDICTION_CHANGED")
    need(r["source_v_and_f_uncertain"] is True
         and r["not_a_fully_integrated_ivp"] is True
         and r["original_14_time_map_ledgers"]=="NOT_RUN",
         "RECEIPT_FAKE_COMPLETE_ORIGINAL_IVP")
    need(r["original_tick161"]=="NOT_ACCEPTED"
         and r["production"]=="HOLD"
         and r["canonical_delta"]==0
         and r["vigilode_dag"]=="7/23" and r["rei_central_dag"]=="28/88",
         "RECEIPT_FALSE_CANONICAL_PROMOTION")
    return True

def main():
    ap=argparse.ArgumentParser()
    for k in ("native_json","native_raw","event_meta","event_gzip",
              "parent_meta","parent_gzip","preview_meta","preview_gzip","out"):
        ap.add_argument("--"+k.replace("_","-"),required=True,type=Path)
    a=ap.parse_args()
    v=verify(json.loads(a.native_json.read_text()),
        json.loads(a.event_meta.read_text()),
        json.loads(a.parent_meta.read_text()),
        a.native_raw.read_bytes(),a.event_gzip.read_bytes(),a.parent_gzip.read_bytes(),
        a.preview_gzip.read_bytes(),json.loads(a.preview_meta.read_text()))
    validate_saved_receipt(v)
    a.out.parent.mkdir(parents=True,exist_ok=True)
    a.out.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:v[k] for k in (
        "native_46k_net_electron_rate_s","native_46k_net_thermal_source_erg_s",
        "native_frozen_first_order_delta_electrons",
        "native_predicted_firstorder_electron_change_inside_preview_global_interval",
        "native_frozen_rate_firstorder_electron_inside_preview_interval_cells",
        "native_frozen_rate_firstorder_logT_inside_preview_interval_cells"
    )}))
if __name__=="__main__":
    main()
