use std::sync::Arc;
use rodas5p_core::{DenseMatrix,WorkCounters};
use rodas5p_integrators::*;
use serde_json::{json,Value};
fn emit(x:Value){println!("{}",x);}
fn flow(v:f64)->OdeProblem {
 OdeProblem::new("r4-constant-flow",1,Arc::new(move|_,_,o:&mut[f64]|{o[0]=v;Ok(())}),None,Some(Arc::new(|_,_|DenseMatrix::from_rows(&[&[0.0]]))),Some(Arc::new(|_,_,_,o:&mut[f64]|{o[0]=0.0;Ok(())})),None,true,None,None).unwrap()
}
fn quadratic(shift:f64,unit:f64)->OdeProblem {
 OdeProblem::new("r4-nonautonomous-quadratic",1,Arc::new(move|t,_,o:&mut[f64]|{o[0]=2.0*(t-shift)/(unit*unit);Ok(())}),None,Some(Arc::new(|_,_|DenseMatrix::from_rows(&[&[0.0]]))),Some(Arc::new(|_,_,_,o:&mut[f64]|{o[0]=0.0;Ok(())})),Some(Arc::new(move|_,_,o:&mut[f64]|{o[0]=2.0/(unit*unit);Ok(())})),false,None,None).unwrap()
}
fn after(t:f64,n:usize)->f64{(0..n).fold(t,|x,_|x.next_up())}
fn fixed(case:&str,p:&OdeProblem,t0:f64,tf:f64,h:f64,v:Option<f64>){
 match integrate_fixed(p,(t0,tf),&[0.0],h,IntegrationMethod::Sequential,None,None,1e-10,1e-9){
 Ok(r)=>emit(json!({"case":case,"method":"fixed_rodas","t0":t0,"tf":tf,"nominal_h":h,"velocity":v,"success":r.success,"times":r.t,"values":r.y,"steps":r.step_sizes,"message":r.message})),
 Err(e)=>emit(json!({"case":case,"method":"fixed_rodas","t0":t0,"tf":tf,"nominal_h":h,"error":e.to_string()}))}
}
fn obs(case:&str,method:&str,t0:f64,tf:f64,v:f64,r:Result<ObservedIntegrationResult,String>){
 match r {Ok(r)=>emit(json!({"case":case,"method":method,"t0":t0,"tf":tf,"velocity":v,"success":r.success,"times":r.t,"values":r.y,"internal_steps":r.internal_steps,"message":r.message})),Err(e)=>emit(json!({"case":case,"method":method,"t0":t0,"tf":tf,"error":e}))}
}
fn adapt(case:&str,method:&str,t0:f64,tf:f64,v:f64,h:f64,r:Result<AdaptiveObservedIntegrationResult,String>){
 match r {Ok(r)=>emit(json!({"case":case,"method":method,"t0":t0,"tf":tf,"velocity":v,"max_step":h,"success":r.observed.success,"times":r.observed.t,"values":r.observed.y,"accepted_h":r.diagnostics.accepted_step_sizes,"rejected_h":r.diagnostics.rejected_step_sizes,"error_norms":r.diagnostics.error_norms,"message":r.observed.message})),Err(e)=>emit(json!({"case":case,"method":method,"t0":t0,"tf":tf,"max_step":h,"error":e}))}
}
fn doubling_geometry(){
 let t0=1e12_f64;let tf=after(t0,3);let h=tf-t0;let mid=t0+0.5*h;let h1=mid-t0;let h2=tf-mid;let p=quadratic(t0,h);let config=RadauConfig{stages:RadauIiaStages::One,..Default::default()};let mut work=WorkCounters::default();
 let coarse=radau_step(&p,t0,&[0.0],h,&config,&mut work).unwrap();let first=radau_step(&p,t0,&[0.0],h1,&config,&mut work).unwrap();let fine=radau_step(&p,mid,&first.y_new,h2,&config,&mut work).unwrap();
 let estimate=step_doubling_wrms_error(&[0.0],&coarse.y_new,&fine.y_new,0.5,1e-12,1).unwrap();
 let q=(h1/h).powi(2)+(h2/h).powi(2);let candidate_error=(fine.y_new[0]-coarse.y_new[0]).abs()*q/(1.0-q);
 emit(json!({"case":"doubling_geometry_kernel","t0":t0,"tf":tf,"h1":h1,"h2":h2,"coarse":coarse.y_new[0],"fine":fine.y_new[0],"error_norm":estimate.error_norm,"estimator_error":estimate.error_vector[0],"candidate_abs_error":candidate_error,"atol":0.5,"rtol":1e-12}));
 let adaptive=AdaptiveStepConfig{atol:0.5,rtol:1e-12,initial_step:h,min_step:1e-20,max_step:h,max_attempts:100,..Default::default()};let out=OutputSchedule::new(vec![t0,tf]).unwrap();
 match integrate_radau_adaptive_observed(&p,(t0,tf),&[0.0],&config,&adaptive,&out){Ok(r)=>emit(json!({"case":"doubling_geometry_adaptive","method":"radau1","t0":t0,"tf":tf,"times":r.observed.t,"values":r.observed.y,"success":r.observed.success,"accepted_h":r.diagnostics.accepted_step_sizes,"error_norms":r.diagnostics.error_norms,"atol":0.5,"rtol":1e-12})),Err(e)=>emit(json!({"case":"doubling_geometry_adaptive","method":"radau1","error":e.to_string()}))}
}
fn bdf_boundary(power:i32){
 let b=2.0_f64.powi(power);let t0=b-8.0*(b-b.next_down());let tf=b+8.0*(b.next_up()-b);let h=0.675*(b.next_up()-b);let v=1.0/(tf-t0);let p=flow(v);
 match integrate_bdf_fixed(&p,(t0,tf),&[0.0],h,&BdfConfig::default()){
 Err(e)=>emit(json!({"case":"bdf_unit_boundary","power":power,"method":"fixed_bdf","error":e.to_string()})),
 Ok(r)=>{
  emit(json!({"case":"bdf_unit_boundary","power":power,"method":"fixed_bdf","t0":t0,"tf":tf,"nominal_h":h,"velocity":v,"times":r.t,"values":r.y,"orders":r.applied_orders}));
  let mut state=vec![0.0];let mut ys=vec![state.clone()];let mut history=BdfHistory::default();let mut work=WorkCounters::default();let mut error=None;
  for pair in r.t.windows(2){match bdf_step_variable(&p,pair[0],&state,pair[1]-pair[0],&BdfConfig::default(),&mut history,&mut work){Ok(s)=>{state=s.y_new;ys.push(state.clone());},Err(e)=>{error=Some(e.to_string());break;}}}
  emit(json!({"case":"bdf_unit_boundary","power":power,"method":"variable_bdf_candidate","t0":t0,"tf":tf,"velocity":v,"times":r.t,"values":ys,"error":error}));
 }
 }
}
fn main(){
 for t0 in [1e12,-1e12] {let tf=after(t0,8);let v=1.0/(tf-t0);let p=flow(v);let out=OutputSchedule::new(vec![t0,tf]).unwrap();fixed("R3_TIME01",&p,t0,tf,1e-4,Some(v));
 obs("R3_TIME01","fixed_bdf",t0,tf,v,integrate_bdf_fixed_observed(&p,(t0,tf),&[0.0],1e-4,&BdfConfig::default(),&out).map_err(|e|e.to_string()));
 obs("R3_TIME01","fixed_radau",t0,tf,v,integrate_radau_fixed_observed(&p,(t0,tf),&[0.0],1e-4,&RadauConfig::default(),&out).map_err(|e|e.to_string()));}
 let t0=1e12;let tf=after(t0,4);let v=1.0/(tf-t0);let p=flow(v);
 let uniform=OutputSchedule::uniform(t0,tf,1.0);emit(json!({"case":"R3_TIME02_uniform","admitted":uniform.is_ok(),"error":uniform.err().map(|e|e.to_string())}));
 for (name,times) in [("R3_TIME02_singleton",vec![tf]),("R3_TIME02_valid",vec![t0,tf])]{let out=OutputSchedule::new(times).unwrap();obs(name,"fixed_observed",t0,tf,v,integrate_fixed_observed(&p,(t0,tf),&[0.0],(tf-t0)/2.0,IntegrationMethod::Sequential,None,None,1e-10,1e-9,&out).map_err(|e|e.to_string()));}
 for power in [-40,0,10,40]{bdf_boundary(power);}
 let tf=after(t0,8);let v=1.0/(tf-t0);let p=flow(v);let ulp=t0.next_up()-t0;let out=OutputSchedule::new(vec![t0,tf]).unwrap();
 for multiplier in [0.49,0.75,1.0,1.5,2.0]{let h=multiplier*ulp;let cfg=AdaptiveStepConfig{atol:1e-10,rtol:1e-9,initial_step:h,min_step:1e-20,max_step:h,max_attempts:100,..Default::default()};adapt("cap_boundary","adaptive_rodas",t0,tf,v,h,integrate_adaptive_observed_with_config(&p,(t0,tf),&[0.0],IntegrationMethod::Sequential,None,None,&cfg,&out).map_err(|e|e.to_string()));}
 for multiplier in [1.0,2.0,3.0]{let h=multiplier*ulp;let cfg=AdaptiveStepConfig{atol:1e-10,rtol:1e-9,initial_step:h,min_step:1e-20,max_step:h,max_attempts:100,..Default::default()};
 adapt("split_clock","radau1",t0,tf,v,h,integrate_radau_adaptive_observed(&p,(t0,tf),&[0.0],&RadauConfig{stages:RadauIiaStages::One,..Default::default()},&cfg,&out).map_err(|e|e.to_string()));
 adapt("split_clock","bdf",t0,tf,v,h,integrate_bdf_adaptive_observed(&p,(t0,tf),&[0.0],&BdfConfig::default(),&cfg,&out).map_err(|e|e.to_string()));}
 for shift in [0.0,2.0_f64.powi(30)]{for unit in [1.0,2.0_f64.powi(10)]{fixed("nonautonomous_quadratic",&quadratic(shift,unit),shift,shift+unit,unit/64.0,None);}}
 for (t0,tf) in [(-0.7,0.3),(-1e-300,1e-300)]{let v=1.0/(tf-t0);fixed("cross_zero",&flow(v),t0,tf,(tf-t0)/10.0,Some(v));}
 doubling_geometry();
}
