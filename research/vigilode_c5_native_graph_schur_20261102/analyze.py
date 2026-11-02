#!/usr/bin/env python3
"""C5 true Rust full-IVP original-source work/precision and guarded fallback audit."""
from pathlib import Path
import hashlib,json,math,statistics,struct,subprocess

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
RAW=HERE/"NATIVE.jsonl"
REF=ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/REFERENCE.json"
R7=ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/RESULTS.json"
WP3=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
ORIGINAL_WP3_BLOB="5dc8df34d314c0d8562c6f548a574625cef442ff"
ORIGINAL_R7_SHA="583e2fada6050a25dfcebd89b682edf45929a8da717b9a2385f628bed47094d4"
ROUTES=("source_owned_graph_schur","protected_fast_direct_W","protected_matrix_free_dense")
SOURCES=("asymmetric_2","asymmetric_2_refined","coupled_3_cubic","linear_exchange_control")
RTOLS=(1e-4,1e-6,1e-8)
def fl(bits):return struct.unpack(">d",struct.pack(">Q",int(bits)))[0]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def run():
    assert subprocess.check_output(["git","hash-object",str(WP3)],text=True).strip()==ORIGINAL_WP3_BLOB
    assert sha(R7)==ORIGINAL_R7_SHA
    reference=json.loads(REF.read_text())
    assert reference["all_reference_consensus_pass"]
    ref={row["case"]:row for row in reference["rows"]}
    assert set(ref)==set(SOURCES)
    raw=[json.loads(line) for line in RAW.read_text().splitlines() if line.strip()]
    sources={x["source"]:x for x in raw if x["kind"]=="C5_FROZEN_ORIGINAL_SOURCE"}
    native=[x for x in raw if x["kind"]=="C5_NATIVE_WHOLE_IVP"]
    assert len(sources)==4 and len(native)==36,len(native)
    r7original=json.loads((ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/NATIVE.jsonl").read_text().splitlines()[0])
    for s in SOURCES:
        assert sources[s]["source_bits"]==next(
            json.loads(row)["source_bits"] for row in
            (ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/NATIVE.jsonl").read_text().splitlines()
            if json.loads(row).get("kind")=="R7_ORIGINAL_SOURCE_CASE"
            and json.loads(row)["name"]==s)
        assert sources[s]["h_bits"]==ref[s]["original_step_h_bits"]
        assert sources[s]["epoch"]==1
    obs=[]
    for row in native:
        s=row["source"];method=row["method"];rtol=fl(row["rtol_bits"]);atol=fl(row["atol_bits"])
        assert s in SOURCES and method in ROUTES and rtol in RTOLS
        assert atol==rtol*.01
        durations=row["raw_wall_ns"];failures=row["errors"]
        assert not failures, (s,method,rtol,failures)
        assert len(durations)==3
        state=row["first"]
        assert state and state["native_success"] and state["complete_original_grid"]
        assert row["repeats_bit_deterministic"]
        assert state["time_bits"]==ref[s]["grid_time_bits"]
        ys=[[fl(v) for v in record] for record in state["state_bits"]]
        assert len(ys)==len(ref[s]["radau"]["output"])
        def err(refpath):
            return max(abs(a-b) for u,v in zip(ys,ref[s][refpath]["output"]) for a,b in zip(u,v))
        er,ed=err("radau"),err("dop853")
        wc=state["counters"]
        assert wc["accepted_steps"]==state["internal_steps"]
        out={
            "source":s,"physical_family":ref[s]["model_family"],"resolution_control":ref[s]["source_equivalent_resolution_control"],
            "method":method,"rtol":rtol,"atol":atol,"times":len(ys),
            "native_full_grid_success":True,"bitwise_replay":True,
            "independent_reference_max_error":max(er,ed),"error_radau":er,"error_dop853":ed,
            "reference_cross_max":ref[s]["cross_solver_state_linf"],
            "median_wall_ns":statistics.median(durations),"raw_wall_ns":durations,
            "work":wc,"internal_steps":state["internal_steps"],
            "output_clipped_steps":state["output_clipped_steps"],
            "diagnostics":state["diagnostics"]
        }
        if method=="source_owned_graph_schur":
            diag=state["diagnostics"]["owner_graph_telemetry"]
            assert diag["certified_stage_solves"]+diag["fallback_stage_solves"]==wc["linear_solves"],(s,rtol,diag,wc)
            assert diag["original_residual_checks"]>=diag["certified_stage_solves"]
            assert wc["jvp_calls"]==0 and wc["nonlinear_iterations"]==0
            assert diag["source_rhs_probes"]==diag["linearizations"]
            out["source_telemetry"]=diag
        obs.append(out)
    assert len({(r["source"],r["method"],r["rtol"]) for r in obs})==36
    paired=[]
    for source in SOURCES:
        for rt in RTOLS:
            fast=next(r for r in obs if r["source"]==source and r["method"]==ROUTES[1] and r["rtol"]==rt)
            own=next(r for r in obs if r["source"]==source and r["method"]==ROUTES[0] and r["rtol"]==rt)
            mf=next(r for r in obs if r["source"]==source and r["method"]==ROUTES[2] and r["rtol"]==rt)
            error_gap=abs(own["independent_reference_max_error"]-fast["independent_reference_max_error"])
            approx_matched=error_gap<=max(1e-11,.02*max(own["independent_reference_max_error"],fast["independent_reference_max_error"]))
            paired.append({
                "source":source,"physical_family":own["physical_family"],"resolution_control":own["resolution_control"],
                "rtol":rt,"native_fast_direct_vs_owned_same_output_driver":True,
                "achieved_error_close_enough_2pct_or_1e_minus11":approx_matched,
                "graph_over_fast_direct_wall_ratio":own["median_wall_ns"]/fast["median_wall_ns"],
                "graph_over_original_mf_dense_wall_ratio_DESCRIPTIVE_OUTPUT_DIFFERENT":own["median_wall_ns"]/mf["median_wall_ns"],
                "graph_error":own["independent_reference_max_error"],
                "fast_error":fast["independent_reference_max_error"],
                "mf_error":mf["independent_reference_max_error"],
                "fast_direct_and_owned_error_gap":error_gap,
                "graph_certified_stages":own["source_telemetry"]["certified_stage_solves"],
                "graph_direct_fallback_stages":own["source_telemetry"]["fallback_stage_solves"],
                "graph_source_rhs_probes":own["source_telemetry"]["source_rhs_probes"],
                "original_fast_direct_rhs_calls":fast["work"]["rhs_calls"]
            })
    assert len(paired)==12
    independent=[r for r in paired if not r["resolution_control"]]
    verdict="SLOWER_THAN_FAST_DIRECT_ON_ALL_ORIGINAL_CASES" if all(r["graph_over_fast_direct_wall_ratio"]>1 for r in paired) else "MIXED"
    result={
      "schema":"vigilode.c5.real_native_source_owned_graph_schur_full_ivp.v1",
      "source_identity":ORIGINAL_WP3_BLOB,
      "referenced_original_native_R7_SHA256":ORIGINAL_R7_SHA,
      "actual_native_conditions":len(obs),
      "native_repetitions_per_condition":3,
      "all_actual_native_full_grid_success":all(r["native_full_grid_success"] for r in obs),
      "all_bitwise_deterministic":all(r["bitwise_replay"] for r in obs),
      "source_families_physically_independent":3,"resolution_controls":1,
      "all_reference_consensus_pass":True,
      "identical_native_driver_comparison":"GRAPH_SCHUR_VS_FAST_DIRECT_SAME_OUTPUT_CLIPPING",
      "secondary_mf_dense_comparison":"SAME_OUTPUT_REQUEST_BUT_DIFFERENT_INTERPOLATION_AND_STEP_LANDING",
      "median_graph_over_fast_direct_wall_ratio_independent":statistics.median(r["graph_over_fast_direct_wall_ratio"] for r in independent),
      "median_graph_over_mf_dense_wall_ratio_DESCRIPTIVE_ONLY":statistics.median(r["graph_over_original_mf_dense_wall_ratio_DESCRIPTIVE_OUTPUT_DIFFERENT"] for r in independent),
      "same_error_ratio_gate_all_12":all(r["achieved_error_close_enough_2pct_or_1e_minus11"] for r in paired),
      "source_owned_solver_verdict":verdict,
      "native_new_method_production":"HOLD_NO_DEMONSTRATED_FAST_DIRECT_GAIN",
      "global_nonnormal_stiff_n_gt3_scope":"NOT_EVALUATED_YET",
      "parent_official_DAG":"7/23",
      "successor_admitted":"0/12","production":"HOLD",
      "original_native_source_and_full_output_rows":obs,
      "per_tolerance_paired_work_precision":paired,
      "provenance_sha256":{"NATIVE.jsonl":sha(RAW),"R7_REFERENCE.json":sha(REF)}
    }
    target=HERE/"RESULTS.json"
    target.write_text(json.dumps(result,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("C5_ACTUAL_NATIVE_ROWS",len(obs),"ORIGINAL_SOURCES",len(SOURCES),"ALL_SUCCESS",result["all_actual_native_full_grid_success"])
    print("MEDIAN_GRAPH_OVER_FAST_DIRECT",result["median_graph_over_fast_direct_wall_ratio_independent"])
    print("VERDICT",verdict,"SAME_ACHIEVED_ERROR_GATE",result["same_error_ratio_gate_all_12"])
    for x in paired:print("SOURCE",x["source"],"rtol",x["rtol"],"GRAPH/FAST",x["graph_over_fast_direct_wall_ratio"],"ERR",x["graph_error"],"FAST_ERR",x["fast_error"],"FALLBACK",x["graph_direct_fallback_stages"])
    print("RESULT_SHA256",sha(target))
    return result
if __name__=="__main__":run()
