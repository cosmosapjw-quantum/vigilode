use std::sync::Arc;
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, OutputSamplingPlan,
integrate_sequential_matrix_free_adaptive_dense_observed, prothero_robinson_problem};

fn record(label: &str, p: &OdeProblem, span: (f64,f64), y0: &[f64], linear: &LinearSolverConfig,
    adaptive: &AdaptiveStepConfig, exact: Option<fn(f64)->Vec<f64>>) {
    let grid = if span.0 == span.1 { vec![span.0] } else {
        (0..=100).map(|k| span.0 + (span.1-span.0)*f64::from(k)/100.).collect()
    };
    let sampling=OutputSamplingPlan::dense(OutputSchedule::new(grid).unwrap());
    match integrate_sequential_matrix_free_adaptive_dense_observed(p,span,y0,linear,adaptive,&sampling) {
        Err(e) => println!("{}",serde_json::json!({"case":label,"kind":"Err","error":e.to_string()})),
        Ok(r) => {
            let max_abs=exact.map(|f|r.observed.t.iter().zip(&r.observed.y).flat_map(|(&t,y)| {
                y.iter().zip(f(t)).map(|(&a,b)|(a-b).abs()).collect::<Vec<_>>()
            }).fold(0.,f64::max));
            let max_scaled=exact.map(|f|r.observed.t.iter().zip(&r.observed.y).flat_map(|(&t,y)| {
                y.iter().zip(f(t)).map(|(&a,b)|(a-b).abs()/(adaptive.atol+adaptive.rtol*b.abs())).collect::<Vec<_>>()
            }).fold(0.,f64::max));
            let endpoint_abs=exact.map(|f| {
                r.observed.y.last().unwrap().iter().zip(f(*r.observed.t.last().unwrap()))
                    .map(|(&a,b)|(a-b).abs()).fold(0.,f64::max)
            });
            println!("{}",serde_json::json!({"case":label,"kind":"Ok","success":r.observed.success,
                "message":r.observed.message,"last_time":r.observed.t.last(),"sample_count":r.observed.t.len(),
                "max_abs_error":max_abs,"max_scaled_error":max_scaled,"last_returned_abs_error":endpoint_abs,
                "atol":adaptive.atol,"rtol":adaptive.rtol,"attempts":r.diagnostics.attempts,
                "accepted":r.diagnostics.accepted_macro_steps,"rejected":r.diagnostics.rejected_macro_steps,
                "jacobian_builds":r.observed.counters.jacobian_builds,"jvp_vectors":r.observed.counters.jvp_vectors,
                "rhs_evaluations":r.observed.counters.rhs_evaluations,"nonfinite_failures":r.observed.counters.nonfinite_step_failures,
                "ledger_consistent":r.diagnostics.is_structurally_consistent()}));
        }
    }
}
fn main() {
    let l=LinearSolverConfig { method:LinearMethod::Gmres,..Default::default() };
    let a=AdaptiveStepConfig { atol:1e-11,rtol:1e-8,initial_step:1e-3,max_step:1.,max_attempts:10000,..Default::default() };
    let (p,y0)=prothero_robinson_problem(-1e6,10.,0.);
    let p=p.jvp_only_clone().unwrap();
    record("stiff_nonlinear_PR",&p,(0.,1.),&y0,&l,&a,Some(|t|vec![t.sin()]));
    let bad_rtol=AdaptiveStepConfig {rtol:f64::NAN,..a.clone()};
    record("adaptive_nan_rtol",&p,(0.,1.),&y0,&l,&bad_rtol,None);
    record("nan_initial",&p,(0.,1.),&[f64::NAN],&l,&a,None);
    let invalid_l=LinearSolverConfig {rtol:f64::INFINITY,..l.clone()};
    record("linear_inf_rtol",&p,(0.,1.),&y0,&invalid_l,&a,None);
    record("linear_inf_rtol_zero_span",&p,(0.,0.),&y0,&invalid_l,&a,None);
    let bad_rhs=OdeProblem::new("nonfinite-rhs",1,Arc::new(|_,_,out|{out[0]=f64::NAN;Ok(())}),None,None,
        Some(Arc::new(|_,_,v,out|{out[0]=-v[0];Ok(())})),None,true,None,None).unwrap();
    let limited=AdaptiveStepConfig {max_attempts:2,..a.clone()};
    record("nonfinite_rhs",&bad_rhs,(0.,1.),&[1.],&l,&limited,None);
    record("budget_one",&p,(0.,1.),&y0,&l,&AdaptiveStepConfig {max_attempts:1,..a},Some(|t|vec![t.sin()]));
}
