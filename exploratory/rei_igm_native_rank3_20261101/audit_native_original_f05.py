#!/usr/bin/env python3
"""Independent readback of native Rust original F05 physical-source receipts.

Only verifies provenance/scientific limits/benchmark receipts. ALL source/JVP/W
physics and timings are computed by native Rust, not this Python auditor.
"""
import argparse
import gzip
import hashlib
import json
import math
import statistics
from pathlib import Path

EXPECTED_SOURCE="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
EXPECTED_KERNEL="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
EXPECTED_ARRAY="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
EXPECTED_OWNER="fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
N=46080


def require(ok,name):
    if not ok:
        raise AssertionError(name)


def validate(meta,results,compressed_bytes):
    require(meta["schema"]=="VIGILODE_REAL_REI_ORIGINAL26_T160_RUST_PHOTON_PARTIAL_FIXTURE_V1",
            "NOT_ORIGINAL_F05_FIXTURE")
    require(meta["original_source_tar_sha256"]==EXPECTED_SOURCE,"SOURCE_TAR_DRIFT")
    require(meta["original_26event_kernel_sha256"]==EXPECTED_KERNEL,"KERNEL_DRIFT")
    require(meta["original_state_array_sha256"]==EXPECTED_ARRAY,"PARENT_ARRAY_DRIFT")
    require(meta["original_owner_head"]==EXPECTED_OWNER,"OWNER_HEAD_DRIFT")
    require(meta["accepted_tick"]=="160/131072","ACCEPTED_TICK_DRIFT")
    require(meta["original_cells"]==N,"NOT_REAL_46K_SOURCE")
    require(hashlib.sha256(compressed_bytes).hexdigest()==meta["gzip_archive_sha256"],
            "FIXTURE_GZIP_SHA_DRIFT")
    raw=gzip.decompress(compressed_bytes)
    require(len(raw)==meta["raw_binary_bytes"],"RAW_BINARY_SIZE_CHANGED")
    require(hashlib.sha256(raw).hexdigest()==meta["raw_binary_sha256"],"RAW_BINARY_SHA_DRIFT")
    require(meta["heii_positive_cells_in_extended_precision"]==N,
            "HEII_PHYSICAL_POSITIVE_SOURCE_LOST")
    require(meta["official_tick161"]=="NOT_ACCEPTED" and meta["canonical_delta"]==0,
            "SOURCE_FIXTURE_FALSE_PROMOTION")
    require(len(results)>=1,"NO_REAL_RUST_RUN")
    speeds=[]
    group=[]
    rank3=[]
    setups=[]
    factors=[]
    scientific=[]
    for r in results:
        require(r["schema"]=="VIGILODE_RUST_REI_ORIGINAL26_F05_T160_NATIVE_PHOTON_RANK3_V1",
                "NOT_NATIVE_RUST_PHYSICAL_RESULT")
        require(r["real_original_cells"]==N and r["source_photon_groups"]==4
                and r["neutral_moments"]==3 and r["photon_partial_nonlocal_rank_upper"]==3,
                "WRONG_ACTUAL_PHOTON_PHYSICS")
        require(r["source_fixture_raw_sha256"]==meta["raw_binary_sha256"],
                "RUST_SOURCE_IDENTITY_NOT_MATCHED")
        require(r["source_owner_interval_HI_HeI_HeII_inside"]==[N,N,N]
                and r["source_owner_heat_cells_inside"]==N
                and r["owner_interval_all_46080_included"] is True,
                "OWNER_SOURCE_INTERVAL_FAIL")
        require(r["photon_ledger_relative_residual"]<1e-12,"ORIGINAL_PHOTON_LEDGER_FAIL")
        require(r["source_direct_vs_rank3_jvp_max_relative"]<1e-10,
                "ORIGINAL_SOURCE_JVP_DIFFERENCE")
        require(r["source_finite_difference_vs_rank3_max_relative"]<1e-6,
                "ORIGINAL_PHOTON_JVP_NOT_FD")
        require(r["native_existing_LowRankBorderedPreconditioner_W_residual_linf"]<1e-8,
                "ORIGINAL_NATIVE_W_RESIDUAL_FAIL")
        require(r["native_f64_heii_rate_underflowed"] is True and r["no_claim_exact_zero_heii"],
                "HEII_UNDERFLOW_FALSE_ZERO")
        require(math.isfinite(r["native_heii_source_integrated_log10_nonzero"])
                and abs(r["native_heii_source_integrated_log10_nonzero"]
                        -meta["heii_total_extended_log10"])<1e-10,
                "HEII_EXTENDED_TRACE_NOT_PRESERVED")
        require(r["original_complete_26event_ode"]=="NOT_RUN"
                and r["original_14_ledgers"]=="NOT_RUN_IN_RUST",
                "FAKE_FULL_SOURCE_OWNER_SOLVER")
        require(r["tick161"]=="NOT_ACCEPTED" and r["production"]=="HOLD"
                and r["canonical_delta"]==0 and r["vigilode_official_dag"]=="7/23"
                and r["rei_central_dag"]=="28/88","FALSE_CANONICAL_PROMOTION")
        for k in ("native_groupwise_jvp_median_seconds",
                  "native_rank3_jvp_median_seconds",
                  "native_source_cache_setup_seconds",
                  "native_lowrank_W_factor_seconds"):
            require(math.isfinite(r[k]) and r[k]>0,"INVALID_NATIVE_BENCHMARK")
        a=r["native_groupwise_jvp_median_seconds"]
        b=r["native_rank3_jvp_median_seconds"]
        speed=a/b
        require(abs(speed-r["native_repeated_jvp_speedup_excluding_setup"])<1e-12,
                "JVP_SPEEDUP_FABRICATED")
        group.append(a);rank3.append(b);setups.append(r["native_source_cache_setup_seconds"])
        factors.append(r["native_lowrank_W_factor_seconds"]);speeds.append(speed)
        scientific.append({
            "photon_ledger_relative_residual":r["photon_ledger_relative_residual"],
            "source_direct_vs_rank3_jvp_max_relative":r["source_direct_vs_rank3_jvp_max_relative"],
            "source_finite_difference_vs_rank3_max_relative":r["source_finite_difference_vs_rank3_max_relative"],
            "native_existing_LowRankBorderedPreconditioner_W_residual_linf":
                r["native_existing_LowRankBorderedPreconditioner_W_residual_linf"]
        })
    first=scientific[0]
    require(all(x==first for x in scientific),"SOURCE_PHYSICS_RUN_REPRODUCIBILITY_CHANGED")
    gain=statistics.median(group)-statistics.median(rank3)
    break_even=max(1,math.ceil(statistics.median(setups)/gain)) if gain>0 else None
    return {
        "schema":"VIGILODE_RUST_REAL_F05_SOURCE_PARTIAL_INDEPENDENT_AUDIT_V1",
        "original_fixture_raw_sha256":meta["raw_binary_sha256"],
        "original_fixture_gzip_sha256":meta["gzip_archive_sha256"],
        "original_heii_source_positive_46080_cells":True,
        "original_heii_total_extended_decimal":meta["heii_total_extended_decimal"],
        "original_source_interval_inclusion":"ALL_THREE_SPECIES_AND_HEAT_46080_PASS",
        "all_native_results_source_scoped_pass":True,
        "native_release_process_repetitions":len(results),
        "native_jvp_per_process_calls":19,
        "native_repeated_jvp_speedups":speeds,
        "median_native_speedup_descriptive":statistics.median(speeds),
        "median_native_groupwise_seconds":statistics.median(group),
        "median_native_rank3_seconds":statistics.median(rank3),
        "median_native_cache_setup_seconds":statistics.median(setups),
        "median_native_W_factor_seconds":statistics.median(factors),
        "estimated_fixed_stage_jvp_break_even_calls":break_even,
        "observed_native_speedup_for_all_runs":all(s>1.0 for s in speeds),
        "observed_native_real_46k_repeated_JVP_not_full_IVP":True,
        "original_26event_full_ivp_speedup":"NOT_ESTABLISHED",
        "native_solve_W_residual":first["native_existing_LowRankBorderedPreconditioner_W_residual_linf"],
        "native_photon_jvp_relative_gap":first["source_direct_vs_rank3_jvp_max_relative"],
        "native_photon_JVP_vs_FD":first["source_finite_difference_vs_rank3_max_relative"],
        "native_original_14_time_map_ledgers":"NOT_RUN",
        "scientific_canonical_delta":0,
        "vigilode_dag":"7/23","rei_central_dag":"28/88",
        "production":"HOLD"
    }


def main():
    p=argparse.ArgumentParser()
    p.add_argument("--metadata",type=Path,required=True)
    p.add_argument("--fixture",type=Path,required=True)
    p.add_argument("--results",type=Path,nargs="+",required=True)
    p.add_argument("--out",type=Path,required=True)
    a=p.parse_args()
    v=validate(json.loads(a.metadata.read_text()),
         [json.loads(x.read_text()) for x in a.results],a.fixture.read_bytes())
    a.out.parent.mkdir(parents=True,exist_ok=True)
    a.out.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({"source_scoped":"PASS","native_processes":len(a.results),
          "native_median_speedup":v["median_native_speedup_descriptive"],
          "estimated_jvp_break_even_calls":v["estimated_fixed_stage_jvp_break_even_calls"]}))

if __name__=="__main__":
    main()
