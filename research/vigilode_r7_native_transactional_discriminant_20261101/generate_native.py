#!/usr/bin/env python3
"""Regenerate a native side-by-side experiment FROM THE EXACT frozen WP3 model.

This generator intentionally preserves original RHS/JVP/IC model bytes by
extracting their Rust source fragment rather than replacing them with a model.
"""
from pathlib import Path
import hashlib
import subprocess

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
PARENT=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
OUTPUT=ROOT/"crates/rodas5p-integrators/examples/vigilode_r7_native_comparison.rs"
EXPECTED_BLOB="5dc8df34d314c0d8562c6f548a574625cef442ff"


def generate():
    actual=subprocess.check_output(["git","hash-object",str(PARENT)],text=True).strip()
    assert actual==EXPECTED_BLOB,(actual,EXPECTED_BLOB)
    source=PARENT.read_text()
    start=source.index("#[derive(Clone)]")
    end=source.index("fn run()")
    cases_and_callbacks=source[start:end]
    assert "fn cases()" in cases_and_callbacks and "fn rhs(" in cases_and_callbacks
    assert "fn jvp(" in cases_and_callbacks and "fn problem(" in cases_and_callbacks
    new=r'''//! R7 original-source-only full-IVP native transactional q1/q2 versus protected RODAS5P.
//! GENERATED from the exact Git-locked WP3 original case definitions.
//! No solver core, controller, source parameter or tolerance modification.
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSamplingPlan, OutputSchedule,
    TransactionalQ1Q2Config, integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_transactional_q1_q2_adaptive_dense_observed,
};
use serde_json::{json,Value};
use std::{hint::black_box, sync::Arc, time::Instant};

'''+cases_and_callbacks+r'''
fn run() -> Result<(),Box<dyn std::error::Error>> {
    // These are the original comparable WP3 tolerances and untouched formula.
    let rtols=[1e-4_f64,1e-6,1e-8];
    for s in cases() {
        let h=0.1_f64/s.steps as f64;
        assert_eq!(h*s.steps as f64,0.1_f64);
        let labels=(0..=s.steps).map(|i| h*i as f64).collect::<Vec<_>>();
        let schedule=OutputSchedule::new(labels.clone())?;
        let ode=problem(&s)?;
        let linear=LinearSolverConfig {
            method: LinearMethod::Gmres,
            ..LinearSolverConfig::default()
        };
        let transactional=TransactionalQ1Q2Config::default();
        println!("{}",json!({
            "kind":"R7_ORIGINAL_SOURCE_CASE",
            "name":s.name,
            "source_bits":source_bits(&s),
            "y0_bits":s.y0.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),
            "h_bits":h.to_bits(),"steps":s.steps,
            "horizon_bits":0.1_f64.to_bits(),
            "original_wp3_git_blob":"5dc8df34d314c0d8562c6f548a574625cef442ff",
            "transactional_config":transactional,
            "linear_config":linear,
            "output_plan":"same-native-dense-grid-original-WP3",
        }));
        for rtol in rtols {
            let atol=rtol*1e-2;
            let adaptive=AdaptiveStepConfig {
                rtol,atol,initial_step:h,max_step:0.1,
                max_attempts:100000,..AdaptiveStepConfig::default()
            };
            for method in ["protected_sequential_RODAS5P","native_transactional_q1_q2"] {
                let mut timings=Vec::new();
                let mut attempt_status=Vec::new();
                let mut first:Option<Value>=None;
                let mut first_bits:Option<Vec<Vec<u64>>>=None;
                let mut deterministic=true;
                for repetition in 0..3 {
                    let time=Instant::now();
                    let result=match method {
                        "protected_sequential_RODAS5P" =>
                            integrate_sequential_matrix_free_adaptive_dense_observed(
                                &ode,(0.0,0.1),&s.y0,&linear,&adaptive,
                                &OutputSamplingPlan::dense(schedule.clone())
                            ).map(|r|(r.observed,json!({"protected_diagnostics":r.diagnostics})))
                             .map_err(|e|e.to_string()),
                        "native_transactional_q1_q2" =>
                            integrate_transactional_q1_q2_adaptive_dense_observed(
                                &ode,(0.0,0.1),&s.y0,&transactional,&adaptive,
                                &OutputSamplingPlan::dense(schedule.clone())
                            ).map(|r|(r.observed,json!({
                                "native_transactional":r.transactional,
                                "adaptive_diagnostics":r.diagnostics
                            }))).map_err(|e|e.to_string()),
                        _=>unreachable!()
                    };
                    let elapsed=time.elapsed().as_nanos();
                    timings.push(elapsed);
                    match result {
                        Ok((r,diag))=>{
                            let state_bits=r.y.iter().map(|row|
                                row.iter().map(|x|x.to_bits()).collect::<Vec<_>>())
                                .collect::<Vec<_>>();
                            let time_bits=r.t.iter().map(|x|x.to_bits())
                                .collect::<Vec<u64>>();
                            let complete=r.success && r.t==labels && r.y.len()==labels.len();
                            attempt_status.push(json!({
                                "repetition":repetition,"kind":"OK",
                                "success":r.success,"complete_grid":complete,
                                "wall_ns":elapsed
                            }));
                            if let Some(ref prior)=first_bits {
                                if state_bits!=*prior {deterministic=false;}
                            } else {first_bits=Some(state_bits.clone());}
                            if first.is_none() {
                                first=Some(json!({
                                    "success":r.success,"complete_grid":complete,
                                    "message":r.message,
                                    "requested_grid_bits":labels.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),
                                    "actual_grid_bits":time_bits,"state_bits":state_bits,
                                    "work_counters":r.counters,
                                    "internal_steps":r.internal_steps,
                                    "output_clipped_steps":r.output_clipped_steps,
                                    "diagnostics":diag
                                }));
                            }
                            black_box(r);
                        }
                        Err(message)=>{
                            attempt_status.push(json!({
                                "repetition":repetition,"kind":"NATIVE_ERROR",
                                "error":message,"wall_ns":elapsed
                            }));
                            break;
                        }
                    }
                }
                println!("{}",json!({
                    "kind":"R7_NATIVE_TRACE",
                    "source":s.name,"method":method,
                    "rtol_bits":rtol.to_bits(),"atol_bits":atol.to_bits(),
                    "repeat_wall_ns":timings,"repeat_status":attempt_status,
                    "repeats_bit_deterministic":deterministic,
                    "first":first,
                    "scientific_scope":"actual_original_native_full_IVP_not_matched_achieved_error_yet",
                    "production":"HOLD",
                }));
            }
        }
    }
    Ok(())
}

fn main(){
    if let Err(e)=run(){
        println!("{}",json!({"kind":"R7_FATAL","error":e.to_string()}));
        std::process::exit(1);
    }
}
'''
    OUTPUT.write_text(new)
    subprocess.run(["rustfmt","--edition","2024",str(OUTPUT)],check=True)
    print("GENERATED_NATIVE_EXAMPLE",OUTPUT)
    print("FROZEN_ORIGINAL_WP3_BLOB",actual)
    print("EXTRACTED_SOURCE_RUST_SHA256",hashlib.sha256(cases_and_callbacks.encode()).hexdigest())
    print("GENERATED_SHA256",hashlib.sha256(OUTPUT.read_bytes()).hexdigest())


if __name__=="__main__":
    generate()
