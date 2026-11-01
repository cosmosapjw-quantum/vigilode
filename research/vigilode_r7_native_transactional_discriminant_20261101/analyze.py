#!/usr/bin/env python3
"""R7 native full-IVP work-precision with independently generated dual reference.

No tolerance modification, no extrapolated production performance.
"""
from collections import Counter,defaultdict
from pathlib import Path
import hashlib
import json
import math
import statistics
import struct
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
NATIVE=HERE/"NATIVE.jsonl"
REFERENCE=HERE/"REFERENCE.json"
RESULT=HERE/"RESULTS.json"
ROUTES=("protected_sequential_RODAS5P","native_transactional_q1_q2")
RTOLS=(1e-4,1e-6,1e-8)
EXPECTED_SOURCE_BLOB="5dc8df34d314c0d8562c6f548a574625cef442ff"
PREREG_HEAD="703fc3d72f42f460cb7b33f72bdb7777816e6b41"


def val_float(u):
    return struct.unpack(">d",struct.pack(">Q",int(u)))[0]


def bit(x):
    return struct.unpack(">Q",struct.pack(">d",float(x)))[0]


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def interpolate_at_achieved_error(rows,target):
    """Piecewise log-linear work-precision interpolation, strictly in observed domain."""
    points=sorted((r["global_error_max"],r["median_wall_ns"]) for r in rows)
    assert len(points)>=2
    for (err0,time0),(err1,time1) in zip(points,points[1:]):
        if err0<=target<=err1 and 0<err0<err1 and 0<time0 and 0<time1:
            alpha=(math.log(target)-math.log(err0))/(math.log(err1)-math.log(err0))
            return math.exp(math.log(time0)+(math.log(time1)-math.log(time0))*alpha)
    return None


def main():
    import importlib.util
    p=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
    blob=subprocess.check_output(["git","hash-object",str(p)],text=True).strip()
    assert blob==EXPECTED_SOURCE_BLOB
    assert NATIVE.is_file() and REFERENCE.is_file()
    native=[json.loads(row) for row in NATIVE.read_text().splitlines() if row.strip()]
    ref=json.loads(REFERENCE.read_text())
    assert ref["all_reference_consensus_pass"]
    refs={row["case"]:row for row in ref["rows"]}
    assert set(refs)=={"asymmetric_2","asymmetric_2_refined","coupled_3_cubic","linear_exchange_control"}
    source_records={r["name"]:r for r in native if r.get("kind")=="R7_ORIGINAL_SOURCE_CASE"}
    assert set(source_records)==set(refs)
    # Independently read original frozen Rust source model via R5 source parser.
    p=HERE/"reference.py"
    spec=importlib.util.spec_from_file_location("r7_original_rust_source_ref",p)
    mod=importlib.util.module_from_spec(spec)
    sys.modules[spec.name]=mod
    spec.loader.exec_module(mod)
    model=mod.original_cases()
    for source,original in source_records.items():
        k,g,c,y0=model[source]
        expected=[bit(x) for x in k+g+y0]+[bit(x) for row in c for x in row]
        assert original["source_bits"]==expected,source
        assert original["y0_bits"]==[bit(x) for x in y0],source
        assert original["h_bits"]==refs[source]["original_step_h_bits"],source
        assert original["steps"]==refs[source]["original_steps"],source
        assert original["original_wp3_git_blob"]==EXPECTED_SOURCE_BLOB

    obs=[]
    for r in native:
        if r.get("kind")!="R7_NATIVE_TRACE":
            continue
        source=r["source"];route=r["method"]
        assert source in refs and route in ROUTES
        grid=refs[source]["grid_time_bits"]
        row={"source":source,"physics_family":refs[source]["model_family"],
             "resolution_control":bool(refs[source]["source_equivalent_resolution_control"]),
             "method":route,
             "rtol":val_float(r["rtol_bits"]),"atol":val_float(r["atol_bits"]),
             "all_original_output_points":len(grid),
             "reference_concordance":refs[source]["cross_solver_state_linf"],
             "repeat_status":r["repeat_status"],
             "repeat_wall_ns":r["repeat_wall_ns"],
             "median_wall_ns":statistics.median(r["repeat_wall_ns"]),
             "output_bit_deterministic":r["repeats_bit_deterministic"],
             "native_result_complete":False,
             "native_method_status":"NATIVE_ERROR",
            }
        assert row["rtol"] in RTOLS
        assert row["atol"]==row["rtol"]*1e-2
        first=r.get("first")
        if first is not None:
            assert len(r["repeat_wall_ns"])==len(r["repeat_status"])==3
            assert all(x["kind"]=="OK" and x["success"] and x["complete_grid"] for x in r["repeat_status"])
            assert row["output_bit_deterministic"]
            assert first["success"] and first["complete_grid"]
            assert first["requested_grid_bits"]==grid and first["actual_grid_bits"]==grid
            got=[[val_float(v) for v in state] for state in first["state_bits"]]
            assert len(got)==len(grid)
            rad=refs[source]["radau"]["output"]
            dop=refs[source]["dop853"]["output"]
            assert all(len(a)==len(b)==len(c) for a,b,c in zip(got,rad,dop))
            err_rad=max(abs(x-y) for a,b in zip(got,rad) for x,y in zip(a,b))
            err_dop=max(abs(x-y) for a,b in zip(got,dop) for x,y in zip(a,b))
            # Exact source dimensions, reference consensus, entire requested grid.
            row.update({
                "native_method_status":"SUCCESS_FULL_GRID",
                "native_result_complete":True,
                "global_error_max":max(err_rad,err_dop),
                "global_error_vs_Radau":err_rad,
                "global_error_vs_DOP853":err_dop,
                "endpoint_error_max":max(abs(x-y) for x,y in zip(got[-1],rad[-1])),
                "reported_work":first["work_counters"],
                "internal_accepted_steps":first["internal_steps"],
                "diagnostics":first["diagnostics"],
                "output_clipped_steps":first["output_clipped_steps"],
                "rejected_steps":first["work_counters"]["rejected_steps"],
                "jvp_vectors":first["work_counters"]["jvp_vectors"],
                "rhs_calls":first["work_counters"]["rhs_calls"],
                "linear_solves":first["work_counters"]["linear_solves"]
            })
            work=first["work_counters"]
            assert work["accepted_steps"]==first["internal_steps"]
            if route=="native_transactional_q1_q2":
                t=first["diagnostics"]["native_transactional"]
                assert t["accepted_q1_fast_steps"]+t["accepted_q2_escalated_steps"]+t["accepted_sequential_fallback_steps"]==first["internal_steps"],(source,row["rtol"],t)
                assert t["total_w_solve_vectors"]>=t["total_w_solve_batches"]
                row["native_q1_accepted"]=t["accepted_q1_fast_steps"]
                row["native_q2_accepted"]=t["accepted_q2_escalated_steps"]
                row["native_fallback_accepted"]=t["accepted_sequential_fallback_steps"]
                row["native_critical_depth_total"]=t["total_critical_path_depth"]
                row["native_w_solve_vectors"]=t["total_w_solve_vectors"]
            else:
                row["native_q1_accepted"]=None
                row["native_q2_accepted"]=None
                row["native_fallback_accepted"]=None
        obs.append(row)
    assert len(obs)==24
    assert len({(x["source"],x["method"],x["rtol"]) for x in obs})==24
    pairrows=[]
    by=defaultdict(list)
    for row in obs:
        by[(row["source"],row["method"])].append(row)
    for src in refs:
        P=by[(src,ROUTES[0])]
        T=by[(src,ROUTES[1])]
        assert len(P)==len(T)==3
        overlap_low=max(min(v["global_error_max"] for v in P),
                        min(v["global_error_max"] for v in T))
        overlap_high=min(max(v["global_error_max"] for v in P),
                         max(v["global_error_max"] for v in T))
        if 0<overlap_low<overlap_high:
            goal=math.sqrt(overlap_low*overlap_high)
            time_p=interpolate_at_achieved_error(P,goal)
            time_t=interpolate_at_achieved_error(T,goal)
            if time_p is not None and time_t is not None:
                match={"source":src,
                  "physics_family":refs[src]["model_family"],
                  "resolution_control":bool(refs[src]["source_equivalent_resolution_control"]),
                  "achieved_error_target_interpolated":goal,
                  "protected_interpolated_wall_ns":time_p,
                  "transactional_interpolated_wall_ns":time_t,
                  "protected_over_transactional_speed_ratio":time_p/time_t,
                  "interpolation":"piecewise log-linear across actual overlapping achieved errors; descriptive only",
                  "future_production_speed_claim":False}
                pairrows.append(match)
    assert len(pairrows)>0

    independent=[r for r in pairrows if not r["resolution_control"]]
    summary={
        "schema":"vigilode.r7.native_original_rodas_transactional_full_ivp.v1",
        "preregistration_commit":PREREG_HEAD,
        "original_frozen_wp3_git_blob":blob,
        "full_native_runs":len(obs),
        "three_repeats_each":True,
        "successful_full_native_grid_runs":sum(r["native_result_complete"] for r in obs),
        "distinct_physics_models":3,
        "resolution_control_models":1,
        "source_scoped_matched_error_interpolations":len(pairrows),
        "distinct_matched_error_pairs":len(independent),
        "median_matched_error_protected_over_transactional":statistics.median(
             r["protected_over_transactional_speed_ratio"] for r in independent),
        "no_native_q2_default_winner":all(
            r["protected_over_transactional_speed_ratio"]<1 for r in independent),
        "native_total_accepted_q1":sum(r["native_q1_accepted"] or 0 for r in obs),
        "native_total_accepted_q2":sum(r["native_q2_accepted"] or 0 for r in obs),
        "native_total_accepted_fallback":sum(r["native_fallback_accepted"] or 0 for r in obs),
        "reference_is_numerical_not_interval_certified":True,
        "stiffness_generalization":"NOT_ESTABLISHED_THREE_SMALL_CUBIC_MODELS_ONLY",
        "native_full_original_w_method":True,
        "fused_exponential_comparison_same_dense_grid":"NOT_RUN_CAPABILITY_DIFFERENCE",
        "official_dag":"7/23","successor_admitted":"0/12","production":"HOLD",
        "all_source_and_method_records":obs,
        "matched_error_interpolated_comparisons":pairrows,
        "input_sha256":{"NATIVE.jsonl":sha(NATIVE),"REFERENCE.json":sha(REFERENCE)},
    }
    RESULT.write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("RESULT_SHA256",sha(RESULT))
    print("NATIVE_ROWS",len(obs),"COMPLETE",summary["successful_full_native_grid_runs"])
    print("TOTAL_Q1_Q2_FALLBACK",summary["native_total_accepted_q1"],summary["native_total_accepted_q2"],summary["native_total_accepted_fallback"])
    for m in pairrows:
        print("MATCHED",m["source"],"error_target",m["achieved_error_target_interpolated"],
              "PROT_OVER_Q2_TIME_RATIO",m["protected_over_transactional_speed_ratio"])
    print("MEDIAN_INDEPENDENT_PROTECTED_OVER_Q2",
          summary["median_matched_error_protected_over_transactional"])


if __name__=="__main__":
    main()
