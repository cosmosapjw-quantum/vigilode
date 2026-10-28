use std::sync::Arc;
use rodas5p_core::{DenseMatrix,LinearMethod,LinearSolverConfig};
use rodas5p_integrators::{AdaptiveStepConfig,OdeProblem,OutputSchedule,OutputSamplingPlan,
    IntegrationMethod,DenseErrorControl,integrate_adaptive_dense_observed_with_dense_error_control};
fn main(){
    let p=OdeProblem::new("PR-no-exact-callback",1,
        Arc::new(|t,y,out| {let d=y[0]-t.sin();out[0]=-1e6*d+t.cos()+10.*d*d;Ok(())}),
        None,Some(Arc::new(|t,y| DenseMatrix::new(1,1,vec![-1e6+20.*(y[0]-t.sin())]))),
        Some(Arc::new(|t,y,v,out| {out[0]=(-1e6+20.*(y[0]-t.sin()))*v[0];Ok(())})),
        Some(Arc::new(|t,y,out| {out[0]=1e6*t.cos()-t.sin()-20.*(y[0]-t.sin())*t.cos();Ok(())})),
        false,None,None).unwrap();
    let l=LinearSolverConfig{method:LinearMethod::Gmres,..Default::default()};
    let a=AdaptiveStepConfig{atol:1e-11,rtol:1e-8,initial_step:1e-3,max_step:1.,max_attempts:10000,..Default::default()};
    let s=OutputSamplingPlan::dense(OutputSchedule::new((0..=100).map(|k|f64::from(k)/100.).collect()).unwrap());
    for mode in [DenseErrorControl::Off,DenseErrorControl::Enforce] {
        match integrate_adaptive_dense_observed_with_dense_error_control(&p,(0.,1.),&[0.],IntegrationMethod::Sequential,Some(&l),None,&a,&s,mode) {
            Err(e)=>println!("{}",serde_json::json!({"mode":mode,"error":e.to_string()})),
            Ok((r,d))=>{
                let max_abs=r.observed.t.iter().zip(&r.observed.y).map(|(&t,y)|(y[0]-t.sin()).abs()).fold(0.,f64::max);
                let max_scaled=r.observed.t.iter().zip(&r.observed.y).map(|(&t,y)|(y[0]-t.sin()).abs()/(a.atol+a.rtol*t.sin().abs())).fold(0.,f64::max);
                println!("{}",serde_json::json!({"mode":mode,"success":r.observed.success,"samples":r.observed.t.len(),"last_time":r.observed.t.last(),"max_abs_error":max_abs,"max_scaled_error":max_scaled,
                    "attempts":r.diagnostics.attempts,"accepted":r.diagnostics.accepted_macro_steps,"rejected":r.diagnostics.rejected_macro_steps,
                    "counters":r.observed.counters,"dense_estimate_count":d.samples.len(),"dense_estimate_unavailable":d.unavailable,
                    "max_accepted_dense_estimate":d.max_accepted_estimate(),"exact_callback_supplied":false}));
            }
        }
    }
}
