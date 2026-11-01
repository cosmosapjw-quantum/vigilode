#!/usr/bin/env python3
"""Independent saved-receipt audit of actual original-26-event ABBA lanes.

Verifies physical/provenance scope and measured source work; no new solver
execution or acceptance of REI official tick161.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path

LANES={
  "PRIMARY":"LOCAL_NEUTRAL_HAZARD_PRIMARY",
  "RECOMB":"RECOMBINATION_WEIGHTED_AUDITOR",
  "SHIELD":"SCRIPT_SELF_SHIELDING_AUDITOR",
}
A=("A0_original","A3_original")
B=("B1_shadow","B2_shadow")
REF_STATE="fe260af02b4ad84ae4c90ed0066e42713ad056a7582d33b0708e5eb6d4b6c6cc"
SOURCE="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
PARENT="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
KERNEL="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
HISTORIC_FAIL=2.1245050576368385e-4


def require(ok:bool, name:str):
    if not ok:
        raise AssertionError(name)


def audit(data):
    require(set(data)==set(LANES), "MISSING_OR_EXTRA_REAL_LANES")
    outputs={}
    state_signatures=set()
    all_work=[]
    for code,expected_lane in LANES.items():
        r=data[code]
        require(r["original_shape_lane"]==expected_lane, "WRONG_OWNER_LANE")
        require(r["original_nodes"]==46080 and r["original_groups"]==4, "WRONG_ORIGINAL_PHYSICS_DIMENSION")
        for key,sha in (("archive_sha256",SOURCE),("parent_sha256",PARENT),("kernel_sha256",KERNEL)):
            require(r[key]==sha, "ORIGINAL_LOCK_FAIL_"+key)
        require(r["accepted_tick"]=="160/131072","ORIGINAL_PARENT_TICK_CHANGED")
        require(r["canonical_delta"]==0 and r["production"]=="HOLD", "FALSE_CANONICAL_PROMOTION")
        require(r["changed_source_on_disk"] is False, "OWNER_REPORTED_MUTATED")
        require(r["published_26_event_owner"] is False,"OWNER_SUBSTITUTED")
        require(r["status"]=="SCOPED_SHADOW_PASS" and r["strict_shadow_pass"], "ORIGINAL_SHADOW_SCOPE_FAIL")
        require(set(r["trials"])==set(A+B), "WRONG_ABBA_ARMS")
        require(r["all_six_original_state_arrays_byte_identical"], "INTRA_LANE_STATE_MISMATCH")
        require(r["all_14_original_ledgers_and_gates_unchanged"], "OWNER_LEDGER_FLAG_FALSE")
        require(r["source_average_rhs_calls_original"]==90, "BASELINE_CALLS_DRIFT")
        require(r["source_average_rhs_calls_shadow"]==72, "SHADOW_WORK_NOT_REDUCED")
        for arm,entry in r["trials"].items():
            require(entry["method"]==("ORIGINAL_UNMODIFIED" if arm in A else "SHADOW_ROOT_DEDUP"),"METHOD_MISCLASSIFIED")
            require(entry["state_bytes_sha256"]==REF_STATE, "SOURCE_ARRAYS_NOT_EXACT")
            state_signatures.add(entry["state_bytes_sha256"])
            require(entry["original_14_ledger_pass"] and entry["original_14_ledger_count"]==14,
                    "PHOTON_OTS_HHE_ENERGY_LEDGER_NOT_14_PASS")
            require(entry["all_original_native_maps_certified"] and entry["all_table_events_zero"],
                    "ONE_OF_FULL_HALF1_HALF2_NOT_CERTIFIED")
            require(abs(entry["historical_crossbox_logT"]-HISTORIC_FAIL)<1e-10
                    and entry["historical_crossbox_logT"]>2e-4,"HISTORIC_V1_GATE_FAILED_TO_FAIL")
            require(set(entry["seconds_by_map"])=={"full","half1","half2"},"INCOMPLETE_ORIGINAL_MAP")
            t=sum(float(s) for s in entry["seconds_by_map"].values())
            require(math.isfinite(t) and t>0
                    and abs(t-entry["total_seconds"])<1e-10,"INVALID_MEASURED_WALL")
            require(entry["thermal_rhs_calls"]==(90 if arm in A else 72),"WORK_CALLS_WRONG_ARM")
            all_work.append(entry["thermal_rhs_calls"])
        at=statistics.median(r["trials"][k]["total_seconds"] for k in A)
        bt=statistics.median(r["trials"][k]["total_seconds"] for k in B)
        ratio=at/bt
        require(abs(ratio-r["diagnostic_wall_speedup"])<1e-12, "WALL_SPEEDUP_MISSTATED")
        outputs[code]={"original_shape_lane":expected_lane,
                       "original_median_full_half_half_seconds":at,
                       "shadow_median_full_half_half_seconds":bt,
                       "diagnostic_wall_speedup":ratio,
                       "full_half_half_original_native_maps":12,
                       "source_owner_14_ledger_records":4,
                       "physical_owner_cells":46080,
                       "original_rhs_calls_per_arm":90,"shadow_rhs_calls_per_arm":72,
                       "all_output_bytes_equal_original":True}
    require(state_signatures=={REF_STATE},"CROSS_LANE_ARRAYS_NOT_SAME_SOURCE")
    return {
      "schema":"VIGILODE_ORIGINAL26_F05_THREE_LANE_ABBA_SHADOW_AUDIT_V1",
      "original_owner_source_sha256":SOURCE,
      "original_owner_state_sha256":PARENT,
      "original_owner_kernel_sha256":KERNEL,
      "actual_original_native_maps":36,
      "full_half_half_trials":12,
      "source_shape_lanes":3,
      "independent_astrophysical_realizations":1,
      "original_physical_source_cells":46080,
      "four_photon_groups":True,
      "original_raw_14_ledger_checks_per_arm":14,
      "all_native_14_ledger_arm_checks":12,
      "all_original_26event_output_bytes_identical":True,
      "shared_original_state_sha256":REF_STATE,
      "historical_v1_local_logt_upper":HISTORIC_FAIL,
      "unchanged_historical_v1_gate":2e-4,
      "historical_v1_failure_preserved":True,
      "all_source_scoped_lanes_passed":True,
      "all_lane_speed_ratios_above_one":all(outputs[k]["diagnostic_wall_speedup"]>1 for k in LANES),
      "source_specific_wall_speedup_by_lane":outputs,
      "median_of_lane_speedup_ratios_descriptive_only":
          statistics.median(outputs[k]["diagnostic_wall_speedup"] for k in LANES),
      "bench_scope":"three concurrent disjoint CPU-affinity groups; within each lane ABBA; fixed 46k source",
      "general_speedup_certification":"NOT_ESTABLISHED",
      "extreme_twenty_bracket_expansion_path":"NOT_TESTED",
      "full_continuum_error_certificate":"NOT_ESTABLISHED",
      "official_tick161":"NOT_ACCEPTED",
      "owner_v2_pr356":"VERSIONED_SOURCE_SCOPED_ADOPTED_ONLY",
      "native_subagent":"NOT_RUN_USER_REQUESTED_SERIAL_PARENT",
      "canonical_delta":0,"vigilode_dag":"7/23","vigilode_successor_dag":"0/12",
      "rei_central_dag":"28/88","rei_working_dag":"33/88","rei_fast_dag":"4/7",
      "production":"HOLD"
    }


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--inputs",type=Path,required=True)
    ap.add_argument("--out",type=Path,required=True)
    args=ap.parse_args()
    data={}
    for k in LANES:
        p=args.inputs/f"ORIGINAL26_ROOT_ABBA_{k}_3LANE.json"
        data[k]=json.loads(p.read_text())
    report=audit(data)
    args.out.parent.mkdir(parents=True,exist_ok=True)
    args.out.write_text(json.dumps(report,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({"status":"3LANE_SCOPED_PASS","arms":report["full_half_half_trials"],
                      "actual_original_maps":report["actual_original_native_maps"],
                      "median_speedup":report["median_of_lane_speedup_ratios_descriptive_only"],
                      "official_tick161":report["official_tick161"]}))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
