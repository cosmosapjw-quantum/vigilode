use std::sync::Arc;
use rodas5p_core::{DenseMatrix,LinearMethod,LinearSolverConfig,WorkCounters};
use rodas5p_integrators::*;
use serde_json::{json,Value};
fn flow(v:f64)->OdeProblem {
 OdeProblem::new("constant-velocity",1,Arc::new(move|_,_,o:&mut[f64]|{o[0]=v;Ok(())}),None,Some(Arc::new(|_,_|DenseMatrix::from_rows(&[&[0.0]]))),Some(Arc::new(|_,_,_,o:&mut[f64]|{o[0]=0.0;Ok(())})),None,true,None,None).unwrap()
}
fn after(t:f64,n:u64)->f64 {if t>0.0 {f64::from_bits(t.to_bits()+n)} else {f64::from_bits(t.to_bits()-n)}}
fn emit(v:Value){println!("{}",v);}
fn strict_schedule(times:&[f64],t0:f64,tf:f64)->bool {
 if !t0.is_finite() || !tf.is_finite() || tf<t0 {return false;}
 if t0==tf {return times==[t0];}
 times.len()>=2 && times.first()==Some(&t0) && times.last()==Some(&tf)
   && times.iter().all(|t|t.is_finite() && *t>=t0 && *t<=tf)
   && times.windows(2).all(|w|w[0]<w[1])
}
fn observed(case:&str,method:&str,origin:f64,tf:f64,v:f64,r:Result<ObservedIntegrationResult,String>){
 match r {Ok(r)=>emit(json!({"case":case,"method":method,"origin":origin,"tf":tf,"velocity":v,"success":r.success,"steps":r.internal_steps,"times":r.t,"values":r.y,"message":r.message})),Err(e)=>emit(json!({"case":case,"method":method,"origin":origin,"tf":tf,"error":e}))}
}
fn fixed(case:&str,origin:f64,tf:f64,h:f64){
 let v=1.0/(tf-origin);let r=integrate_fixed(&flow(v),(origin,tf),&[0.0],h,IntegrationMethod::Sequential,None,None,1e-10,1e-9);
 match r {Ok(r)=>emit(json!({"case":case,"method":"fixed_rodas","origin":origin,"tf":tf,"velocity":v,"proposed_h":h,"success":r.success,"steps":r.attempts,"times":r.t,"values":r.y,"step_sizes":r.step_sizes,"embedded_errors":r.error_norms})),Err(e)=>emit(json!({"case":case,"method":"fixed_rodas","origin":origin,"tf":tf,"proposed_h":h,"error":e.to_string()}))}
}
fn repaired(origin:f64,tf:f64,nominal:f64){
 let v=1.0/(tf-origin);let problem=flow(v);let mut t=origin;let mut y=vec![0.0];let mut counter=WorkCounters::default();let mut ts=vec![t];let mut hs=vec![];let mut ys=vec![y.clone()];let mut fail=None;
 for _ in 0..100 {if t==tf {break;}let tn=(t+nominal).min(tf);let h=tn-t;
 if !(h>0.0 && t+h==tn && tn>t && tn<=tf){fail=Some("inconsistent represented clock");break;}
 match sequential_step(&problem,t,&y,h,&LinearSolverConfig::default(),None,1e-10,1e-9,true,&mut counter){Ok(r)=>{assert_eq!(r.t_new,tn);t=r.t_new;y=r.y_new;hs.push(h);ts.push(t);ys.push(y.clone());},Err(_)=>{fail=Some("native step error");break;}}
 }
 emit(json!({"case":"represented_h_candidate","method":"public_sequential_step_driver","origin":origin,"tf":tf,"velocity":v,"proposed_h":nominal,"success":t==tf && fail.is_none(),"failure":fail,"times":ts,"values":ys,"step_sizes":hs}));
}
fn main(){
 for origin in [1e12,-1e12] {let tf=after(origin,8);fixed("r2_original_short_span",origin,tf,(tf-origin)/2.0);fixed("documented_step_clock_mismatch",origin,tf,1e-4);repaired(origin,tf,1e-4);}
 fixed("r2_original_one_ulp_span",1.0,1.0_f64.next_up(),f64::EPSILON);
 fixed("r2_nonadvancing_step",1.0,1.0_f64.next_up(),f64::EPSILON/2.0);
 let origin=1e12_f64;let tf=after(origin,8);let v=1.0/(tf-origin);let output=OutputSchedule::new(vec![origin,tf]).unwrap();let sampling=OutputSamplingPlan::dense(output.clone());
 fixed("local_clock_control",0.0,tf-origin,1e-4);
 repaired(0.0,tf-origin,1e-4);
 let cfg=AdaptiveStepConfig{atol:1e-10,rtol:1e-9,initial_step:1e-4,min_step:1e-14,max_step:1e-4,max_attempts:100,..Default::default()};
 observed("documented_step_clock_mismatch","adaptive_rodas",origin,tf,v,integrate_adaptive_observed_with_config(&flow(v),(origin,tf),&[0.0],IntegrationMethod::Sequential,None,None,&cfg,&output).map(|r|r.observed).map_err(|e|e.to_string()));
 observed("documented_step_clock_mismatch","fixed_radau",origin,tf,v,integrate_radau_fixed_observed(&flow(v),(origin,tf),&[0.0],1e-4,&RadauConfig::default(),&output).map_err(|e|e.to_string()));
 observed("documented_step_clock_mismatch","fixed_bdf",origin,tf,v,integrate_bdf_fixed_observed(&flow(v),(origin,tf),&[0.0],1e-4,&BdfConfig::default(),&output).map_err(|e|e.to_string()));
 observed("documented_step_clock_mismatch","adaptive_fused_exp",origin,tf,v,integrate_pexprb54s4_fused_adaptive_observed(&flow(v),(origin,tf),&[0.0],&cfg,&output,FusedPhiKrylovConfig::default(),&ParallelExecution::sequential()).map(|r|r.observed).map_err(|e|e.to_string()));
 observed("documented_step_clock_mismatch","fixed_dense_rodas",origin,tf,v,integrate_fixed_dense_observed(&flow(v),(origin,tf),&[0.0],1e-4,IntegrationMethod::Sequential,None,None,1e-10,1e-9,&sampling).map_err(|e|e.to_string()));
 let tf=origin+1.0;let first=origin+0.25;let output=OutputSchedule::new(vec![origin,first,first.next_up(),tf]).unwrap();let sampling=OutputSamplingPlan::dense(output);
 let cfg=AdaptiveStepConfig{initial_step:0.25,max_step:0.25,..cfg};let linear=LinearSolverConfig{method:LinearMethod::Gmres,..Default::default()};
 observed("r2_original_dense_adjacent","matrixfree_dense_rodas",origin,tf,8192.0,integrate_sequential_matrix_free_adaptive_dense_observed(&flow(8192.0),(origin,tf),&[0.0],&linear,&cfg,&sampling).map(|r|r.observed).map_err(|e|e.to_string()));
 let output=OutputSchedule::new(vec![origin,origin.next_up(),tf]).unwrap();
 observed("r2_original_clipped_adjacent","adaptive_rodas",origin,tf,1.0,integrate_adaptive_observed_with_config(&flow(1.0),(origin,tf),&[0.0],IntegrationMethod::Sequential,None,None,&cfg,&output).map(|r|r.observed).map_err(|e|e.to_string()));
 // Endpoint alias admission: span endpoints are distinct represented numbers.
 let tf=after(origin,4);let span=tf-origin;let velocity=1.0/span;
 for (case,output) in [("single_point_uniform",OutputSchedule::uniform(origin,tf,1.0).unwrap()),("single_point_explicit",OutputSchedule::new(vec![tf]).unwrap()),("valid_two_point_control",OutputSchedule::new(vec![origin,tf]).unwrap())]{
  emit(json!({"case":"strict_schedule_candidate","input_case":case,"origin":origin,"tf":tf,"times":output.times(),"admitted":strict_schedule(output.times(),origin,tf)}));
  observed(case,"fixed_observed_rodas",origin,tf,velocity,integrate_fixed_observed(&flow(velocity),(origin,tf),&[0.0],span/2.0,IntegrationMethod::Sequential,None,None,1e-10,1e-9,&output).map_err(|e|e.to_string()));
  observed(case,"fixed_dense_rodas",origin,tf,velocity,integrate_fixed_dense_observed(&flow(velocity),(origin,tf),&[0.0],span/2.0,IntegrationMethod::Sequential,None,None,1e-10,1e-9,&OutputSamplingPlan::dense(output)).map_err(|e|e.to_string()));
 }
 let tf=origin+1.0;let output=OutputSchedule::new(vec![origin.next_up(),tf.next_up()]).unwrap();
 emit(json!({"case":"strict_schedule_candidate","input_case":"endpoint_alias_labels","origin":origin,"tf":tf,"times":output.times(),"admitted":strict_schedule(output.times(),origin,tf)}));
 observed("endpoint_alias_labels","fixed_observed_rodas",origin,tf,8192.0,integrate_fixed_observed(&flow(8192.0),(origin,tf),&[0.0],0.25,IntegrationMethod::Sequential,None,None,1e-10,1e-9,&output).map_err(|e|e.to_string()));
 fixed("long_run_acknowledged_microstep",0.0,10.0,0.01);
}
