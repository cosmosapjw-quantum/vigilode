#!/usr/bin/env python3
"""C5: generate native benchmark from exact frozen WP3 original Rust source."""
from pathlib import Path
import subprocess,hashlib

ROOT=Path(__file__).resolve().parents[2]
FROZEN=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
OUTPUT=ROOT/"crates/rodas5p-integrators/examples/vigilode_c5_source_owned_graph.rs"
LOCK="5dc8df34d314c0d8562c6f548a574625cef442ff"

def main():
    current=subprocess.check_output(["git","hash-object",str(FROZEN)],text=True).strip()
    assert current==LOCK,(current,LOCK)
    body=FROZEN.read_text()
    start=body.index("#[derive(Clone)]")
    end=body.index("fn run()")
    source=body[start:end]
    assert all(i in source for i in ("fn cases()", "fn rhs(", "fn jvp(", "fn problem("))
    pre=r'''
//! C5 native original WP3 full-IVP graph-Schur stage client vs protected baselines.
//! GENERATED from byte-locked original WP3 model RHS/JVP/data; opt-in ONLY.
use rodas5p_core::{LinearMethod,LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig,OdeProblem,OutputSamplingPlan,OutputSchedule,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_fast_observed_with_atol,
    source_owned_graph_schur::{CooperativeCubicOwner,GraphSchurTelemetry,source_owned_cooperative_cubic_problem},
};
use serde_json::{json,Value};
use std::{sync::{Arc,Mutex},time::Instant,hint::black_box};

'''
    rest=r'''
fn run()->Result<(),Box<dyn std::error::Error>> {
    let rtols=[1e-4_f64,1e-6,1e-8];
    for s in cases(){
        let h=0.1/s.steps as f64;
        let grid=(0..=s.steps).map(|i|h*i as f64).collect::<Vec<_>>();
        assert_eq!(grid.last().copied(),Some(0.1));
        let schedule=OutputSchedule::new(grid.clone())?;
        let ode=problem(&s)?.with_model_epoch(Arc::new(||1_u64));
        let feedback=if s.k.len()==3 {vec![0,1]} else {vec![0]};
        let owner=CooperativeCubicOwner{
            name:"wp3-original-cooperative-cubic".into(),epoch:1,
            k:s.k.clone(),g:s.g.clone(),c:s.c.clone(),feedback,
        };
        println!("{}",json!({
          "kind":"C5_FROZEN_ORIGINAL_SOURCE","source":s.name,
          "source_bits":source_bits(&s),"initial_bits":s.y0.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
          "original_wp3_git_blob":"5dc8df34d314c0d8562c6f548a574625cef442ff",
          "h_bits":h.to_bits(),"epoch":ode.model_epoch(),
          "source_ownership":"immutable original WP3 callback and analytic JVP",
          "clipped_schedule_contract":"C5 client vs fast direct; protected MF uses original dense interpolation"
        }));
        let linear=LinearSolverConfig{method:LinearMethod::Gmres,..LinearSolverConfig::default()};
        for rtol in rtols {
            let atol=rtol*1e-2;
            let adaptive=AdaptiveStepConfig{
                rtol,atol,initial_step:h,max_step:0.1,max_attempts:100000,
                ..AdaptiveStepConfig::default()
            };
            for method in ["source_owned_graph_schur","protected_fast_direct_W","protected_matrix_free_dense"]{
                let mut durations=Vec::new();
                let mut failures=Vec::new();
                let mut state_first:Option<Vec<Vec<u64>>>=None;
                let mut record_first:Option<Value>=None;
                let mut deterministic=true;
                for rep in 0..3 {
                    let shared=Arc::new(Mutex::new(GraphSchurTelemetry::default()));
                    let start=Instant::now();
                    let result:Result<(rodas5p_integrators::ObservedIntegrationResult,Value),String>=
                        match method {
                          "source_owned_graph_schur" => {
                            match source_owned_cooperative_cubic_problem(owner.clone(),Arc::clone(&shared)) {
                              Ok((owned_ode,client))=>
                                integrate_rodas5p_fast_client_observed_with_atol(
                                    &owned_ode,Box::new(client),(0.,0.1),&s.y0,&adaptive,&schedule,
                                    &vec![atol;s.y0.len()]
                                ).map(|(r,partial,_)|
                                    (r.observed,json!({"driver":"native_existing_fast_client",
                                                      "partial":partial.is_some(),
                                                      "owner_graph_telemetry":shared.lock().unwrap().clone()}))
                                ).map_err(|e|e.to_string()),
                              Err(e)=>{
                                // Explicit protected matrix-free full-run fallback on invalid graph.
                                integrate_sequential_matrix_free_adaptive_dense_observed(
                                  &ode,(0.,0.1),&s.y0,&linear,&adaptive,
                                  &OutputSamplingPlan::dense(schedule.clone())
                                ).map(|r|(r.observed,json!({
                                  "driver":"protected_matrix_free_after_source_refusal",
                                  "source_refusal":e.to_string(),
                                  "protected_diagnostics":r.diagnostics
                                }))).map_err(|inner|format!("owner refused ({e}); protected failed ({inner})"))
                              }
                            }
                          }
                          "protected_fast_direct_W" =>
                            integrate_rodas5p_fast_observed_with_atol(
                               &ode,(0.,0.1),&s.y0,&adaptive,&schedule,&vec![atol;s.y0.len()]
                            ).map(|(r,partial)|(r.observed,json!({
                              "driver":"original_fast_direct","partial":partial.is_some()
                            }))).map_err(|e|e.to_string()),
                          "protected_matrix_free_dense" =>
                            integrate_sequential_matrix_free_adaptive_dense_observed(
                               &ode,(0.,0.1),&s.y0,&linear,&adaptive,
                               &OutputSamplingPlan::dense(schedule.clone())
                            ).map(|r|(r.observed,json!({
                                "driver":"original_protected_matrix_free_dense",
                                "protected_diagnostics":r.diagnostics
                            }))).map_err(|e|e.to_string()),
                          _=>unreachable!()
                        };
                    let duration=start.elapsed().as_nanos();
                    durations.push(duration);
                    match result {
                      Ok((r,diagnostics))=>{
                        let state=r.y.iter().map(|v|v.iter().map(|x|x.to_bits()).collect::<Vec<_>>()).collect::<Vec<_>>();
                        if let Some(old)=&state_first {
                            if *old!=state {deterministic=false;}
                        } else {state_first=Some(state.clone());}
                        let tbits=r.t.iter().map(|x|x.to_bits()).collect::<Vec<_>>();
                        let complete=r.success && tbits==grid.iter().map(|x|x.to_bits()).collect::<Vec<_>>();
                        if record_first.is_none(){
                            record_first=Some(json!({
                                "native_success":r.success,"complete_original_grid":complete,
                                "time_bits":tbits,"state_bits":state,
                                "counters":r.counters,"internal_steps":r.internal_steps,
                                "output_clipped_steps":r.output_clipped_steps,
                                "diagnostics":diagnostics
                            }));
                        }
                        if !complete{failures.push(format!("rep={rep} incomplete original output grid"));}
                        black_box(r);
                      }
                      Err(e)=>{failures.push(format!("rep={rep} native error: {e}"));break;}
                    }
                }
                println!("{}",json!({
                    "kind":"C5_NATIVE_WHOLE_IVP","source":s.name,"method":method,
                    "rtol_bits":rtol.to_bits(),"atol_bits":atol.to_bits(),
                    "raw_wall_ns":durations,"errors":failures,"first":record_first,
                    "repeats_bit_deterministic":deterministic,
                    "source_protection":"original RODAS5P 8-stage controller kept",
                    "source_version":"immutable C5 owner and original WP3 bit identity",
                    "production":"HOLD"
                }));
            }
        }
    }
    Ok(())
}
fn main(){
    if let Err(e)=run(){
        println!("{}",json!({"kind":"C5_FATAL","error":e.to_string()}));
        std::process::exit(1);
    }
}
'''
    OUTPUT.write_text(pre+source+rest)
    subprocess.run(["rustfmt","--edition","2024",str(OUTPUT)],check=True)
    print("GENERATED",OUTPUT,"SOURCE_SHA256",hashlib.sha256(source.encode()).hexdigest(),
          "NEW_SHA256",hashlib.sha256(OUTPUT.read_bytes()).hexdigest())
if __name__=="__main__":main()
